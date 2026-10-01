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

// --- batch 128 --------------------------------------------------------------

// WITH CommentedPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// c.UserId AS CommenterId,
// c.Text AS CommentText,
// c.CreationDate AS CommentDate
// FROM
// Posts p
// JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// AND c.CreationDate > p.CreationDate
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(c.Id) AS TotalComments
// FROM
// Users u
// JOIN
// Comments c ON u.Id = c.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// PostDetails AS (
// SELECT
// cp.PostId,
// cp.Title,
// cp.CommentText,
// cp.CommentDate,
// up.DisplayName AS CommenterName,
// up.Reputation AS CommenterReputation
// FROM
// CommentedPosts cp
// JOIN
// UserReputation up ON cp.CommenterId = up.UserId
// )
// SELECT
// pd.PostId,
// pd.Title,
// pd.CommentText,
// pd.CommentDate,
// pd.CommenterName,
// pd.CommenterReputation,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// PostDetails pd
// LEFT JOIN
// Votes v ON pd.PostId = v.PostId
// GROUP BY
// pd.PostId, pd.Title, pd.CommentText, pd.CommentDate, pd.CommenterName, pd.CommenterReputation
// ORDER BY
// pd.CommentDate DESC, pd.CommenterReputation DESC
// LIMIT 100;
fn q26513(db: &'static So) -> String {
    let Comment { post, text, creation_date, user, .. } = &db.comment;
    let key = post.and(text).and(creation_date).and(user.select(&db.user.display_name)).and(user.select(&db.user.reputation));
    let f = db
        .comment
        .with(post.select(&db.post.post_type_id).eq(1))
        .with(creation_date.and(post.select(&db.post.creation_date)).filt(|(c, p)| c > p))
        .with(user)
        .group_by(key)
        .select(post.select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |&((((_, cd), _), r), _)| (Reverse(cd), Reverse(r)), 100, |&(((((p, t), cd), n), r), a)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::S(t), V::T(cd), V::S(n), V::I(r)]);
        f.extend(ints(&a));
        f
    })
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
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(COALESCE(p.Score, 0)) AS AvgScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// UserPostMetrics AS (
// SELECT
// u.UserId,
// u.DisplayName,
// u.Reputation,
// u.BadgeCount,
// u.GoldBadges,
// u.SilverBadges,
// u.BronzeBadges,
// COALESCE(p.PostCount, 0) AS PostCount,
// COALESCE(p.QuestionCount, 0) AS QuestionCount,
// COALESCE(p.AnswerCount, 0) AS AnswerCount,
// COALESCE(p.AvgScore, 0) AS AvgScore,
// COALESCE(p.AvgViewCount, 0) AS AvgViewCount
// FROM UserStatistics u
// LEFT JOIN PostStatistics p ON u.UserId = p.UserId
// )
// SELECT
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
// AvgViewCount
// FROM UserPostMetrics
// ORDER BY Reputation DESC, BadgeCount DESC
// LIMIT 100;
fn q26637(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(u, b, _)| (rep_desc(db, u), Reverse(b[0])), 100, |&(u, b, p)| {
        let p0 = p.unwrap_or([0; 6]);
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend(ints(&p0[..3]));
        f.push(if p0[0] > 0 { avg(p0[3], p0[0]) } else { V::F(0.0) });
        f.push(if p0[4] > 0 { avg(p0[5], p0[4]) } else { V::F(0.0) });
        f
    })
}

// WITH KeywordOccurrences AS (
// SELECT
// P.Id AS PostId,
// COUNT(*) AS TagCount,
// SUM(CASE
// WHEN P.Title ILIKE '%SQL%' THEN 1
// ELSE 0 END) AS SQLTitleCount,
// SUM(CASE
// WHEN C.Text ILIKE '%SQL%' THEN 1
// ELSE 0 END) AS SQLCommentCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 YEAR'
// GROUP BY
// P.Id
// ),
// PostDetails AS (
// SELECT
// U.DisplayName,
// U.Reputation,
// P.Title,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// K.TagCount,
// K.SQLTitleCount,
// K.SQLCommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// JOIN
// KeywordOccurrences K ON P.Id = K.PostId
// WHERE
// P.PostTypeId = 1
// )
// SELECT
// DisplayName,
// Reputation,
// Title,
// ViewCount,
// AnswerCount,
// CommentCount,
// TagCount,
// SQLTitleCount,
// SQLCommentCount,
// CASE
// WHEN SQLTitleCount > 0 THEN 'Contains SQL in Title'
// ELSE 'No SQL in Title'
// END AS TitleStatus,
// CASE
// WHEN SQLCommentCount > 0 THEN 'Contains SQL in Comments'
// ELSE 'No SQL in Comments'
// END AS CommentStatus
// FROM
// PostDetails
// ORDER BY
// ViewCount DESC
// LIMIT 10;
fn q27244(db: &'static So) -> String {
    let sql = |s: Str| s.to_lowercase().contains("sql");
    let ko = since(db, date(2023, 10, 1))
        .group_by(Ident::<Post>::new())
        .select((&db.post.title).opt().and(comments_of(db).select(&db.comment.text).opt()))
        .fold([0i64; 3], move |a, (t, c)| [a[0] + 1, a[1] + t.map_or(false, sql) as i64, a[2] + c.map_or(false, sql) as i64]);
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(&ko)).drive(|_, x| v.push(x));
    out(v, |&(p, _)| views_desc(db, p), 10, |&(p, a)| {
        let mut f = post_fields(db, p, &["owner", "rep", "title", "views", "answers", "comments"]);
        f.extend(ints(&a));
        f.push(V::S(if a[1] > 0 { "Contains SQL in Title" } else { "No SQL in Title" }));
        f.push(V::S(if a[2] > 0 { "Contains SQL in Comments" } else { "No SQL in Comments" }));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// WHERE u.Reputation > 1000
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId IN (1, 2, 6, 7) THEN p.Score ELSE 0 END) AS TotalScore,
// AVG(p.ViewCount) AS AvgViews
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// CombinedStatistics AS (
// SELECT
// ur.UserId,
// ur.DisplayName,
// ur.Reputation,
// ur.BadgeCount,
// ps.TotalPosts,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.TotalScore,
// ps.AvgViews
// FROM UserReputation ur
// JOIN PostStatistics ps ON ur.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// AvgViews,
// CASE
// WHEN TotalPosts > 100 THEN 'Veteran'
// WHEN TotalPosts > 50 THEN 'Active'
// ELSE 'Newcomer'
// END AS UserRank
// FROM CombinedStatistics
// ORDER BY Reputation DESC, TotalScore DESC;
fn q27339(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if matches!(t, 1 | 2 | 6 | 7) { s } else { 0 }, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ps).and(&bu)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        f.extend(ints(&p[..4]));
        f.push(avg(p[5], p[4]));
        f.push(V::S(if p[0] > 100 {
            "Veteran"
        } else if p[0] > 50 {
            "Active"
        } else {
            "Newcomer"
        }));
        row(f)
    }))
}

// WITH QualifiedPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><'), 1) AS TagCount,
// u.DisplayName AS OwnerDisplayName,
// COALESCE((
// SELECT COUNT(*)
// FROM Posts AS a
// WHERE a.ParentId = p.Id
// ), 0) AS AnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1 AND
// p.Score > 0 AND
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(*) AS EditCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId IN (4, 5, 24)
// GROUP BY
// ph.PostId
// ),
// ClosedPosts AS (
// SELECT
// PostId,
// COUNT(*) AS CloseCount
// FROM
// PostHistory
// WHERE
// PostHistoryTypeId = 10
// GROUP BY
// PostId
// )
// SELECT
// qp.PostId,
// qp.Title,
// qp.OwnerDisplayName,
// qp.Score,
// qp.ViewCount,
// qp.TagCount,
// COALESCE(phe.EditCount, 0) AS EditCount,
// COALESCE(cp.CloseCount, 0) AS CloseCount
// FROM
// QualifiedPosts qp
// LEFT JOIN
// PostHistoryStats phe ON qp.PostId = phe.PostId
// LEFT JOIN
// ClosedPosts cp ON qp.PostId = cp.PostId
// ORDER BY
// qp.Score DESC, qp.ViewCount DESC, qp.CreationDate ASC;
fn q27582(db: &'static So) -> String {
    let ed = history_of_types(db, &[4, 5, 24]);
    let cl = history_of_types(db, &[10]);
    let mut v = Vec::new();
    owned_since(db, year_ago())
        .with((&db.post.post_type_id).eq(1))
        .with((&db.post.score).gt(0))
        .select(Ident::<Post>::new().and(&ed).and(&cl))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, e), c)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.extend([oint(db.post.tags_str.get(p).map(|s| split_n(s, "><"))), V::I(e), V::I(c)]);
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
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// PopularPosts AS (
// SELECT
// p.OwnerUserId,
// p.Id AS PostId,
// p.Score,
// p.ViewCount,
// p.Title,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId IN (1, 2)
// GROUP BY
// p.OwnerUserId, p.Id, p.Score, p.ViewCount, p.Title
// HAVING
// COUNT(c.Id) >= 5
// ),
// ActiveUsers AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// pp.PostId,
// pp.Score,
// pp.ViewCount,
// pp.Title
// FROM
// Users u
// JOIN
// UserBadgeCounts ub ON u.Id = ub.UserId
// JOIN
// PopularPosts pp ON u.Id = pp.OwnerUserId
// ORDER BY
// u.Reputation DESC,
// ub.BadgeCount DESC
// )
// SELECT
// au.DisplayName,
// au.Reputation,
// au.CreationDate,
// au.BadgeCount,
// au.GoldBadges,
// au.SilverBadges,
// au.BronzeBadges,
// au.Title,
// au.Score AS PostScore,
// au.ViewCount AS PostViewCount
// FROM
// ActiveUsers au
// WHERE
// au.BadgeCount > 0 AND
// au.Reputation > 100
// ORDER BY
// au.Reputation DESC,
// au.BadgeCount DESC;
fn q27651(db: &'static So) -> String {
    let ub = ubc(db);
    let pp = db.post.with((&db.post.post_type_id).in_v(vec![1, 2])).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |a, c| a + c.is_some() as i64);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100))
        .select(Ident::<User>::new().and((&ub).filt(|b: [i64; 4]| b[0] > 0)).and(posts_of(db).select(Ident::<Post>::new().with((&pp).filt(|c: i64| c >= 5)))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated")];
        f.extend(ints(&b));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
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
// MostActiveUsers AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(ba.BadgeCount, 0) AS BadgeCount,
// COALESCE(ma.PostCount, 0) AS TotalPosts,
// COALESCE(ma.Questions, 0) AS TotalQuestions,
// COALESCE(ma.Answers, 0) AS TotalAnswers
// FROM Users u
// LEFT JOIN UserBadges ba ON u.Id = ba.UserId
// LEFT JOIN MostActiveUsers ma ON u.Id = ma.OwnerUserId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.Reputation,
// ua.BadgeCount,
// ua.TotalPosts,
// ua.TotalQuestions,
// ua.TotalAnswers,
// CONCAT(ua.DisplayName, ' has earned ', ua.BadgeCount, ' badges and posted a total of ', ua.TotalPosts,
// ' entries (', ua.TotalQuestions, ' questions and ', ua.TotalAnswers, ' answers).') AS UserSummary
// FROM UserActivity ua
// ORDER BY ua.Reputation DESC, ua.BadgeCount DESC
// LIMIT 10;
fn q27684(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let pc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&pc).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 3]))));
    out(v, |&(u, b, _)| (rep_desc(db, u), Reverse(b)), 10, |&(u, b, p)| {
        let n = db.user.display_name.get(u).unwrap();
        let mut f = vec![user_col(db, u, "uid"), V::S(n), user_col(db, u, "rep"), V::I(b)];
        f.extend(ints(&p));
        f.push(V::S(leak(format!("{n} has earned {b} badges and posted a total of {} entries ({} questions and {} answers).", p[0], p[1], p[2]))));
        f
    })
}

// WITH TagData AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Body,
// P.Tags,
// T.TagName,
// T.Count AS TagCount,
// U.Reputation AS UserReputation
// FROM
// Posts P
// JOIN
// Tags T ON POSITION(T.TagName IN P.Tags) > 0
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= DATE_TRUNC('year', cast('2024-10-01' as date))
// ),
// PostStats AS (
// SELECT
// COUNT(DISTINCT PostId) AS TotalPosts,
// AVG(LENGTH(Body)) AS AverageBodyLength,
// AVG(UserReputation) AS AverageUserReputation,
// SUM(TagCount) AS TotalTags
// FROM
// TagData
// )
// SELECT
// PS.TotalPosts,
// PS.AverageBodyLength,
// PS.AverageUserReputation,
// PS.TotalTags,
// (SELECT COUNT(*) FROM TagData WHERE array_length(string_to_array(Tags, '>'), 1) > 3) AS PostsWithComplexTags
// FROM
// PostStats PS;
fn q27709(db: &'static So) -> String {
    let tm = tag_mentions(db);
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&tm).map(|(p, _)| p).inv().collect();
    let base = || owned_since(db, date(2024, 1, 1));
    let pair = (&by_post)
        .select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t).select(&db.tag.count))
        .and(&db.post.body)
        .and((&db.post.owner_user).select(&db.user.reputation))
        .and(&db.post.tags_str);
    let a = base().select(pair).fold_flat([0i64; 5], |a, (((c, body), r), tags)| {
        [a[0] + 1, a[1] + body.chars().count() as i64, a[2] + r, a[3] + c, a[4] + (tags.split('>').count() > 3) as i64]
    });
    let dp = count(base().with(&by_post));
    row(vec![V::I(dp), avg(a[1], a[0]), avg(a[2], a[0]), nullable(a[3], a[0]), V::I(a[4])])
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
// UserPostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
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
// B.BadgeCount,
// B.GoldBadges,
// B.SilverBadges,
// B.BronzeBadges,
// P.PostCount,
// P.QuestionCount,
// P.AnswerCount,
// P.TotalViews,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate
// FROM
// Users U
// JOIN
// UserBadges B ON U.Id = B.UserId
// JOIN
// UserPostStats P ON U.Id = P.OwnerUserId
// )
// SELECT
// UA.DisplayName,
// UA.Reputation,
// UA.BadgeCount,
// UA.GoldBadges,
// UA.SilverBadges,
// UA.BronzeBadges,
// UA.PostCount,
// UA.QuestionCount,
// UA.AnswerCount,
// UA.TotalViews,
// CONCAT('User ', UA.DisplayName, ' has earned ', UA.BadgeCount, ' badges (',
// UA.GoldBadges, ' gold, ', UA.SilverBadges, ' silver, ', UA.BronzeBadges, ' bronze) and has created ',
// UA.PostCount, ' posts (', UA.QuestionCount, ' questions and ', UA.AnswerCount, ' answers) with a total of ',
// UA.TotalViews, ' views') AS UserSummary
// FROM
// UserActivity UA
// ORDER BY
// UA.Reputation DESC, UA.BadgeCount DESC
// LIMIT 10;
fn q27739(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 5], |a, (t, w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ub).and(&ps).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(u, b, _)| (rep_desc(db, u), Reverse(b[0])), 10, |&(u, b, p)| {
        let n = db.user.display_name.get(u).unwrap();
        let views = if p[3] > 0 { p[4].to_string() } else { String::new() };
        let mut f = vec![V::S(n), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend(ints(&p[..3]));
        f.push(nullable(p[4], p[3]));
        f.push(V::S(leak(format!(
            "User {n} has earned {} badges ({} gold, {} silver, {} bronze) and has created {} posts ({} questions and {} answers) with a total of {views} views",
            b[0], b[1], b[2], b[3], p[0], p[1], p[2]
        ))));
        f
    })
}

// WITH PostTagCounts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.OwnerUserId,
// ARRAY_LENGTH(STRING_TO_ARRAY(SUBSTRING(p.Tags, 2, LENGTH(p.Tags)-2), '><'), 1) AS TagCount
// FROM
// Posts p
// WHERE
// p.PostTypeId = 1
// ),
// PopularPosts AS (
// SELECT
// ptc.PostId,
// ptc.Title,
// ptc.CreationDate,
// ptc.OwnerUserId,
// ptc.TagCount,
// COUNT(v.Id) AS VoteCount
// FROM
// PostTagCounts ptc
// LEFT JOIN
// Votes v ON ptc.PostId = v.PostId AND v.VoteTypeId IN (2, 6)
// WHERE
// ptc.TagCount >= 3
// GROUP BY
// ptc.PostId, ptc.Title, ptc.CreationDate, ptc.OwnerUserId, ptc.TagCount
// ),
// TopUsers AS (
// SELECT
// u.Id,
// u.DisplayName,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// COUNT(p.Id) AS QuestionCount
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalScore DESC
// LIMIT 5
// )
// SELECT
// pp.Title,
// pp.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// pp.TagCount,
// pp.VoteCount,
// tu.TotalViews,
// tu.TotalScore,
// tu.QuestionCount
// FROM
// PopularPosts pp
// JOIN
// Users u ON pp.OwnerUserId = u.Id
// JOIN
// TopUsers tu ON pp.OwnerUserId = tu.Id
// ORDER BY
// pp.VoteCount DESC, pp.TagCount DESC;
fn q28007(db: &'static So) -> String {
    let tu = owned(db).with((&db.post.post_type_id).eq(1)).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&db.post.score)).fold([0i64; 4], |a, (w, s)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]
    });
    let top: MatSet<Id<User>> = whole(&tu).select(Same::new().and(&tu)).window(row_number, |(_, a): (Id<User>, [i64; 4])| a[3], desc).filt(|(_, n)| n <= 5).map(|((u, _), _)| u).collect();
    let vc = db.vote.with((&db.vote.vote_type_id).in_v(vec![2, 6])).select(&db.vote.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let pp = Ident::<Post>::new().with((&db.post.post_type_id).eq(1)).with((&db.post.tags_str).filt(|s: Str| split_n(s, "><") >= 3)).and(&vc);
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&tu).and(posts_of(db).select(pp))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, a), (p, x))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(split_n(db.post.tags_str.get(p).unwrap(), "><")), V::I(x), nullable(a[2], a[1]), V::I(a[3]), V::I(a[0])]);
        row(f)
    }))
}

// WITH PostTagCounts AS (
// SELECT
// p.Id AS PostId,
// COUNT(DISTINCT t.TagName) AS TagCount,
// SUM(l.Count) AS TagUsageCount
// FROM
// Posts p
// JOIN
// Tags t ON p.Tags LIKE CONCAT('%<', t.TagName, '>%')
// LEFT JOIN
// Tags l ON t.Id = l.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id
// ),
// AnswerStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(pa.Id) AS AnswerCount,
// AVG(pa.Score) AS AvgAnswerScore
// FROM
// Posts p
// LEFT JOIN
// Posts pa ON p.Id = pa.ParentId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id
// ),
// PostHistoryAnalytics AS (
// SELECT
// ph.PostId,
// COUNT(*) AS EditCount,
// MAX(CASE WHEN ph.PostHistoryTypeId IN (4, 5) THEN ph.CreationDate END) AS LastEditDate,
// MAX(CASE WHEN ph.PostHistoryTypeId IN (10) THEN ph.CreationDate END) AS ClosedDate
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// pt.TagCount,
// pt.TagUsageCount,
// asn.AnswerCount,
// asn.AvgAnswerScore,
// ph.EditCount,
// ph.LastEditDate,
// ph.ClosedDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostTagCounts pt ON p.Id = pt.PostId
// LEFT JOIN
// AnswerStats asn ON p.Id = asn.PostId
// LEFT JOIN
// PostHistoryAnalytics ph ON p.Id = ph.PostId
// WHERE
// p.Score > 10
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q28010(db: &'static So) -> String {
    let by_name: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pairs: MatSet<(Id<Post>, Id<Tag>)> = questions_only(db).select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&by_name))).collect();
    let post_of = || Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p);
    let tag_of = || Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t);
    let usage = (&pairs).group_by(post_of()).select(tag_of().select(&db.tag.count)).fold(0i64, |a, c| a + c);
    let dn = (&pairs).group_by(post_of()).select(tag_of().select(&db.tag.tag_name)).count_distinct();
    let an = questions_only(db).group_by(Ident::<Post>::new()).select(children_of(db).select(&db.post.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let ph = db
        .post_history
        .group_by(&db.post_history.post)
        .select((&db.post_history.post_history_type_id).and(&db.post_history.creation_date))
        .fold([0, i64::MIN, i64::MIN], |a: [i64; 3], (t, d)| [a[0] + 1, if matches!(t, 4 | 5) { a[1].max(d) } else { a[1] }, if t == 10 { a[2].max(d) } else { a[2] }]);
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.score).gt(10))
        .select(Ident::<Post>::new().and((&dn).opt()).and((&usage).opt()).and((&an).opt()).and((&ph).opt()))
        .drive(|_, x| v.push(x));
    let t = |x: i64| if x == i64::MIN { V::Null } else { V::T(x) };
    out(v, |&((((p, _), _), _), _)| newest(db, p), 100, |&((((p, d), u), a), h)| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend([oint(d), oint(u), oint(a.map(|a| a[0])), a.map_or(V::Null, |a| avg(a[1], a[0]))]);
        match h {
            Some(h) => f.extend([V::I(h[0]), t(h[1]), t(h[2])]),
            None => f.extend(nulls(3)),
        }
        f
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName AS UserName,
// COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// SUM(u.Views) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostSummary AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ub.UserName,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.TotalScore,
// ub.TotalUpVotes,
// ub.TotalDownVotes,
// ub.TotalViews,
// COALESCE(ROUND((CAST(ps.TotalScore AS DECIMAL) / NULLIF(ps.TotalPosts, 0)), 2), 0) AS AverageScorePerPost
// FROM
// UserBadges ub
// JOIN
// PostSummary ps ON ub.UserId = ps.OwnerUserId
// ORDER BY
// ub.TotalViews DESC, ub.GoldBadges DESC, ps.TotalScore DESC
// FETCH FIRST 10 ROWS ONLY;
fn q28165(db: &'static So) -> String {
    let User { up_votes, down_votes, views, .. } = &db.user;
    let ub = g(db).select(up_votes.and(down_votes).and(views).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 6], |a, (((u, d), w), c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + u, a[4] + d, a[5] + w]
    });
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let mut v = Vec::new();
    (&ub).and(&ps).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[5]), Reverse(b[0]), Reverse(p[3])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b[..3]));
        f.extend(ints(&p));
        f.extend(ints(&b[3..]));
        f.push(V::F(round2(p[3] as f64 / p[0] as f64)));
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
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostInformation AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.CreationDate,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN v.VoteTypeId = 1 THEN 1 ELSE 0 END) AS AcceptedByOriginatorVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.Body, p.CreationDate, p.OwnerUserId
// ),
// CombinedData AS (
// SELECT
// pi.PostId,
// pi.Title,
// pi.Body,
// pi.CreationDate,
// pi.CommentCount,
// pi.UpVotes,
// pi.DownVotes,
// pi.AcceptedByOriginatorVotes,
// ubc.UserId,
// ubc.DisplayName,
// ubc.BadgeCount,
// ubc.GoldCount,
// ubc.SilverCount,
// ubc.BronzeCount
// FROM
// PostInformation pi
// JOIN
// UserBadgeCounts ubc ON pi.OwnerUserId = ubc.UserId
// )
// SELECT
// ubc.UserId,
// ubc.DisplayName,
// cd.PostId,
// cd.Title,
// cd.CreationDate,
// cd.CommentCount,
// cd.UpVotes,
// cd.DownVotes,
// cd.AcceptedByOriginatorVotes,
// ubc.BadgeCount,
// ubc.GoldCount,
// ubc.SilverCount,
// ubc.BronzeCount
// FROM
// CombinedData cd
// JOIN
// UserBadgeCounts ubc ON cd.UserId = ubc.UserId
// WHERE
// ubc.BadgeCount > 0
// ORDER BY
// ubc.BadgeCount DESC, cd.UpVotes DESC, cd.CommentCount DESC;
fn q28173(db: &'static So) -> String {
    let ub = ubc(db);
    let mut v = Vec::new();
    stats_fold(db, owned(db).with((&db.post.post_type_id).eq(1)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and((&ub).filt(|b: [i64; 4]| b[0] > 0))))
        .drive(|p, (s, (u, b))| v.push((p, s, u, b)));
    rows(v.iter().map(|&(p, s, u, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend(ints(&[s.cx, s.up, s.down, s.by_vt[1]]));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS TotalBadges,
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
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(BC.TotalBadges, 0) AS BadgeCount,
// COALESCE(PS.TotalPosts, 0) AS PostCount,
// COALESCE(PS.TotalQuestions, 0) AS QuestionCount,
// COALESCE(PS.TotalAnswers, 0) AS AnswerCount,
// COALESCE(PS.TotalViews, 0) AS ViewCount,
// COALESCE(PS.TotalScore, 0) AS Score
// FROM Users U
// LEFT JOIN UserBadgeCounts BC ON U.Id = BC.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UP.UserId,
// UP.DisplayName,
// UP.BadgeCount,
// UP.PostCount,
// UP.QuestionCount,
// UP.AnswerCount,
// UP.ViewCount,
// UP.Score
// FROM UserPerformance UP
// WHERE UP.BadgeCount > 0 OR UP.PostCount > 0
// ORDER BY UP.BadgeCount DESC, UP.ViewCount DESC, UP.Score DESC
// LIMIT 10;
fn q28604(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 5], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s]
    });
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()))
        .filt(|((_, b), p): ((Id<User>, i64), Option<[i64; 5]>)| b > 0 || p.map_or(0, |p| p[0]) > 0)
        .drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 5]))));
    out(v, |&(_, b, p)| (Reverse(b), Reverse(p[3]), Reverse(p[4])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b)];
        f.extend(ints(&p));
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
// RecentPostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// ubc.UserId,
// ubc.DisplayName,
// ubc.BadgeCount,
// rp.PostCount,
// rp.QuestionCount,
// rp.AnswerCount,
// rp.LastPostDate
// FROM
// UserBadgeCounts ubc
// JOIN
// RecentPostStats rp ON ubc.UserId = rp.OwnerUserId
// WHERE
// ubc.BadgeCount > 0
// ORDER BY
// ubc.BadgeCount DESC,
// rp.PostCount DESC
// LIMIT 10
// )
// SELECT
// tu.DisplayName,
// tu.BadgeCount,
// tu.PostCount,
// tu.QuestionCount,
// tu.AnswerCount,
// EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - tu.LastPostDate)) / 3600 AS HoursSinceLastPost
// FROM
// TopUsers tu
// ORDER BY
// tu.BadgeCount DESC,
// tu.PostCount DESC;
fn q28646(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let bu = badges_per_user(db);
    let rp = owned_since(db, year_ago()).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.creation_date)).fold([0, 0, 0, i64::MIN], |a: [i64; 4], (t, c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(c)]
    });
    let mut v = Vec::new();
    (&rp).and((&bu).filt(|b: i64| b > 0)).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, b)| (Reverse(b), Reverse(a[0])), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name"), V::I(b)];
        f.extend(ints(&a[..3]));
        f.push(V::F(hours_to(t0, a[3]) / 3600.0));
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
// PopularPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title
// HAVING
// COUNT(c.Id) > 5 AND SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) > SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END)
// )
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.TotalBadges,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// pp.PostId,
// pp.Title,
// pp.CommentCount,
// pp.UpVoteCount,
// pp.DownVoteCount
// FROM
// UserBadges ub
// JOIN
// Posts p ON p.OwnerUserId = ub.UserId
// JOIN
// PopularPosts pp ON pp.PostId = p.Id
// ORDER BY
// ub.TotalBadges DESC, pp.UpVoteCount DESC
// FETCH FIRST 10 ROWS ONLY;
fn q2885(db: &'static So) -> String {
    let ub = ubc(db);
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, month_ago()), Ident::<Post>::new(), "cv", &[])
        .filt(|s: Stats| s.cx > 5 && s.up > s.down)
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&ub)))
        .drive(|p, (s, (u, b))| v.push((p, s, u, b)));
    out(v, |&(_, s, _, b)| (Reverse(b[0]), Reverse(s.up)), 10, |&(p, s, u, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend(ints(&[s.cx, s.up, s.down]));
        f
    })
}

// WITH TagCounts AS (
// SELECT
// TagName,
// COUNT(*) AS PostCount
// FROM Tags
// GROUP BY TagName
// ),
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// pt.Name AS PostType,
// u.DisplayName AS Author,
// p.AnswerCount,
// COALESCE(pc.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// COALESCE(l.LinkedScore, 0) AS LinkedScore
// FROM Posts p
// JOIN PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) pc ON p.Id = pc.PostId
// LEFT JOIN (
// SELECT PostId, SUM(CASE WHEN LinkTypeId = 1 THEN 1 ELSE 0 END) AS LinkedScore
// FROM PostLinks
// GROUP BY PostId
// ) l ON p.Id = l.PostId
// LEFT JOIN (
// SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// ) b ON u.Id = b.UserId
// ),
// FilteredPosts AS (
// SELECT
// pd.*,
// tc.PostCount
// FROM PostDetails pd
// JOIN TagCounts tc ON pd.Title LIKE '%' || tc.TagName || '%'
// )
// SELECT
// fp.PostId,
// fp.Title,
// fp.CreationDate,
// fp.ViewCount,
// fp.PostType,
// fp.Author,
// fp.AnswerCount,
// fp.CommentCount,
// fp.BadgeCount,
// fp.LinkedScore,
// fp.PostCount
// FROM FilteredPosts fp
// WHERE fp.PostCount > 10
// ORDER BY fp.ViewCount DESC, fp.CreationDate DESC
// LIMIT 100;
fn q28879(db: &'static So) -> String {
    let tc = db.tag.group_by(&db.tag.tag_name).fold(0i64, |a, _| a + 1);
    let titles: MatSet<Str> = db.post.select(&db.post.title).collect();
    let m: HashIdx<Str, Id<Tag>> = (&titles).select_where((&db.tag.tag_name).inv(), |t: Str, n: Str| t.contains(n)).collect();
    let linked = db.post_link.group_by(&db.post_link.post).select(&db.post_link.link_type_id).fold(0i64, |a, t| a + (t == 1) as i64);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.post
        .select(
            Ident::<Post>::new()
                .and((&db.post.title).select(&m).select(&db.tag.tag_name).select((&tc).filt(|n: i64| n > 10)))
                .and(comments_per_post(db))
                .and((&linked).opt())
                .and((&db.post.owner_user).select(&bu).opt()),
        )
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| (views_desc(db, p), newest(db, p)), 100, |&((((p, n), c), l), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "type", "owner", "answers"]);
        f.extend([V::I(c), V::I(b.unwrap_or(0)), V::I(l.unwrap_or(0)), V::I(n)]);
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldCount,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverCount,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeCount,
// COUNT(B.Id) AS TotalBadges
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
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// ActiveUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// UB.GoldCount,
// UB.SilverCount,
// UB.BronzeCount,
// PS.TotalPosts,
// PS.TotalQuestions,
// PS.TotalAnswers,
// PS.TotalViews,
// PS.AverageScore
// FROM
// Users U
// JOIN
// UserBadgeCounts UB ON U.Id = UB.UserId
// JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// PS.TotalPosts DESC,
// U.Reputation DESC
// LIMIT 10
// )
// SELECT
// AU.DisplayName,
// AU.TotalPosts,
// AU.TotalQuestions,
// AU.TotalAnswers,
// AU.TotalViews,
// AU.AverageScore,
// AU.GoldCount,
// AU.SilverCount,
// AU.BronzeCount
// FROM
// ActiveUsers AU
// ORDER BY
// AU.TotalPosts DESC;
fn q29086(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and(&ps)).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (Reverse(p[0]), rep_desc(db, u)), 10, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&p[..3]));
        f.extend([nullable(p[4], p[3]), avg(p[5], p[0])]);
        f.extend(ints(&b[1..]));
        f
    })
}

// WITH TagStatistics AS (
// SELECT
// t.TagName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScoreCount,
// AVG(p.ViewCount) AS AverageViews,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Tags t
// JOIN
// Posts p ON p.Tags LIKE CONCAT('%<', t.TagName, '>%')
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// t.Id, t.TagName
// ),
// HighScoringTags AS (
// SELECT
// ts.TagName,
// ts.PostCount,
// ts.PositiveScoreCount,
// ts.NegativeScoreCount,
// ts.AverageViews,
// ts.AverageReputation
// FROM
// TagStatistics ts
// WHERE
// ts.AverageReputation > 1000 AND
// ts.PositiveScoreCount > ts.NegativeScoreCount
// )
// SELECT
// ht.TagName,
// ht.PostCount,
// ht.PositiveScoreCount,
// ht.NegativeScoreCount,
// ht.AverageViews,
// ht.AverageReputation
// FROM
// HighScoringTags ht
// ORDER BY
// ht.AverageViews DESC,
// ht.PositiveScoreCount DESC;
fn q29321(db: &'static So) -> String {
    let by_name: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pairs: MatSet<(Id<Tag>, Id<Post>)> = since(db, year_ago()).select((&db.post.tags_str).flat_map(tag_list).select(&by_name).and(Ident::<Post>::new())).collect();
    let Post { score, view_count, owner_user, .. } = &db.post;
    let f = (&pairs)
        .group_by(Same::<(Id<Tag>, Id<Post>)>::new().map(|(t, _)| t))
        .select(Same::<(Id<Tag>, Id<Post>)>::new().map(|(_, p)| p).select(score.and(view_count.opt()).and(owner_user.select(&db.user.reputation).opt())))
        .fold([0i64; 7], |a, ((s, w), r)| {
            [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + r.is_some() as i64, a[6] + r.unwrap_or(0)]
        });
    let mut v = Vec::new();
    (&f).filt(|a: [i64; 7]| a[5] > 0 && a[6] as f64 / a[5] as f64 > 1000.0 && a[1] > a[2]).drive(|t, a| v.push((t, a)));
    rows(v.iter().map(|&(t, a)| row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), avg(a[6], a[5])])))
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS NumberOfBadges,
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
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// UserPostSummary AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.NumberOfBadges, 0) AS BadgeCount,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.TotalScore, 0) AS TotalScore
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalViews,
// TotalScore,
// (BadgeCount * 0.2 + TotalPosts * 0.5 + TotalViews * 0.1 + TotalScore * 0.2) AS BenchmarkScore
// FROM UserPostSummary
// WHERE TotalPosts > 0
// ORDER BY BenchmarkScore DESC
// LIMIT 10;
fn q29487(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s]
    });
    let tenths = |b: i64, p: [i64; 5]| 2 * b + 5 * p[0] + p[3] + 2 * p[4];
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 5]| p[0] > 0).and(&bu).drive(|u, (p, b)| v.push((u, p, b)));
    out(v, |&(_, p, b)| Reverse(tenths(b, p)), 10, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b)];
        f.extend(ints(&p));
        f.push(V::F(tenths(b, p) as f64 / 10.0));
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
// COUNT(P.Id) AS PostCount,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(BC.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.QuestionCount, 0) AS QuestionCount,
// COALESCE(PS.AnswerCount, 0) AS AnswerCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts BC ON U.Id = BC.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UP.UserId,
// UP.DisplayName,
// UP.BadgeCount,
// UP.QuestionCount,
// UP.AnswerCount,
// UP.TotalScore,
// U.Reputation,
// U.CreationDate,
// U.Views,
// U.UpVotes,
// U.DownVotes
// FROM
// UserPerformance UP
// JOIN
// Users U ON UP.UserId = U.Id
// WHERE
// UP.BadgeCount > 0 OR UP.QuestionCount > 0
// ORDER BY
// UP.BadgeCount DESC, UP.TotalScore DESC
// LIMIT 50;
fn q29631(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 3], |a, (t, s)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s]);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()))
        .filt(|((_, b), p): ((Id<User>, i64), Option<[i64; 3]>)| b > 0 || p.map_or(0, |p| p[0]) > 0)
        .drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 3]))));
    out(v, |&(_, b, p)| (Reverse(b), Reverse(p[2])), 50, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b)];
        f.extend(ints(&p));
        f.extend(["rep", "ucreated", "uviews", "uup", "udown"].iter().map(|c| user_col(db, u, c)));
        f
    })
}

// WITH UserBadgeCount AS (
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
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPostBadgeMetrics AS (
// SELECT
// UBC.UserId,
// UBC.DisplayName,
// PS.TotalPosts,
// PS.TotalQuestions,
// PS.TotalAnswers,
// PS.TotalViews,
// PS.AverageScore,
// UBC.BadgeCount,
// UBC.GoldBadges,
// UBC.SilverBadges,
// UBC.BronzeBadges
// FROM
// UserBadgeCount UBC
// JOIN
// PostStatistics PS ON UBC.UserId = PS.OwnerUserId
// )
// SELECT
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalViews,
// AverageScore,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// UserPostBadgeMetrics
// WHERE
// TotalPosts > 0
// ORDER BY
// BadgeCount DESC, TotalViews DESC
// LIMIT 50;
fn q29726(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]
    });
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 6]| p[0] > 0)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[0]), (p[3] == 0, Reverse(p[4]))), 50, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&p[..3]));
        f.extend([nullable(p[4], p[3]), avg(p[5], p[0])]);
        f.extend(ints(&b));
        f
    })
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.Tags,
// p.CreationDate,
// u.DisplayName AS AuthorDisplayName,
// u.Reputation AS AuthorReputation,
// ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><'), 1) AS TagCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(v.UpVoteCount, 0) AS UpVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN (
// SELECT
// ParentId,
// COUNT(*) AS AnswerCount
// FROM
// Posts
// WHERE
// PostTypeId = 2
// GROUP BY
// ParentId
// ) a ON p.Id = a.ParentId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS UpVoteCount
// FROM
// Votes
// WHERE
// VoteTypeId = 2
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// ),
// ClosedPosts AS (
// SELECT
// ph.PostId,
// COUNT(*) AS CloseEvents
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId = 10
// GROUP BY
// ph.PostId
// ),
// EnhancedPostDetails AS (
// SELECT
// pd.PostId,
// pd.Title,
// pd.Body,
// pd.Tags,
// pd.CreationDate,
// pd.AuthorDisplayName,
// pd.AuthorReputation,
// pd.TagCount,
// pd.AnswerCount,
// pd.UpVoteCount,
// COALESCE(cp.CloseEvents, 0) AS CloseEventCount
// FROM
// PostDetails pd
// LEFT JOIN
// ClosedPosts cp ON pd.PostId = cp.PostId
// )
// SELECT
// epd.PostId,
// epd.Title,
// epd.Body,
// epd.Tags,
// epd.CreationDate,
// epd.AuthorDisplayName,
// epd.AuthorReputation,
// epd.TagCount,
// epd.AnswerCount,
// epd.UpVoteCount,
// epd.CloseEventCount
// FROM
// EnhancedPostDetails epd
// WHERE
// epd.TagCount > 5
// ORDER BY
// epd.UpVoteCount DESC,
// epd.CreationDate ASC
// LIMIT 10;
fn q29842(db: &'static So) -> String {
    let ce = history_of_types(db, &[10]);
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.tags_str).filt(|s: Str| split_n(s, "><") > 5))
        .select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(votes_of_type(db, 2)).and(&ce))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), u), _)| (Reverse(u), db.post.creation_date.get(p).unwrap()), 10, |&(((p, a), u), c)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "owner", "rep"]);
        f.extend([V::I(split_n(db.post.tags_str.get(p).unwrap(), "><")), V::I(a), V::I(u), V::I(c)]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// AVG(COALESCE(p.Score, 0)) AS AvgPostScore
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// RecentActivity AS (
// SELECT
// UserId,
// MAX(CreationDate) AS LastActiveDate
// FROM Comments
// GROUP BY UserId
// ),
// UserBadges AS (
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
// ups.Reputation,
// ups.TotalPosts,
// ups.QuestionsCount,
// ups.AnswersCount,
// ups.AvgPostScore,
// ra.LastActiveDate,
// COALESCE(ub.TotalBadges, 0) AS TotalBadges,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM UserPostStats ups
// LEFT JOIN RecentActivity ra ON ups.UserId = ra.UserId
// LEFT JOIN UserBadges ub ON ups.UserId = ub.UserId
// WHERE ups.Reputation > 100
// ORDER BY ups.TotalPosts DESC, ups.Reputation DESC
// FETCH FIRST 10 ROWS ONLY;
fn q30841(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let uf = user_base(db, UserWhere::RepGt(100)).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s],
        None => a,
    });
    let lc = db.comment.group_by(&db.comment.user).select(&db.comment.creation_date).fold(i64::MIN, |a, d| a.max(d));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&lc).opt()).and((&bc).opt()).drive(|u, ((a, l), b)| v.push((u, a, l, b.unwrap_or([0; 4]))));
    out(v, |&(u, a, _, _)| (Reverse(a[0]), rep_desc(db, u)), 10, |&(u, a, l, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a[..3]));
        f.extend([or0(a[3], a[0]), ots(l)]);
        f.extend(ints(&b));
        f
    })
}

// WITH RecentVotes AS (
// SELECT
// P.Id AS PostId,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(*) AS TotalVotes
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// P.Id
// ),
// UserBadges AS (
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
// PostEditHistory AS (
// SELECT
// Ph.PostId,
// COUNT(*) AS EditCount,
// MAX(Ph.CreationDate) AS LastEditDate
// FROM
// PostHistory Ph
// WHERE
// Ph.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// Ph.PostId
// )
// SELECT
// P.Title,
// P.Body,
// R.UpVotes,
// R.DownVotes,
// R.TotalVotes,
// U.DisplayName AS PostOwner,
// U.Reputation AS OwnerReputation,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// PE.EditCount,
// PE.LastEditDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// JOIN
// RecentVotes R ON P.Id = R.PostId
// JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostEditHistory PE ON P.Id = PE.PostId
// WHERE
// P.PostTypeId = 1
// AND U.Reputation > 100
// AND R.TotalVotes > 0
// AND (R.UpVotes - R.DownVotes) > 10
// ORDER BY
// R.TotalVotes DESC,
// U.Reputation DESC
// LIMIT 50;
fn q31194(db: &'static So) -> String {
    let rv = since(db, month_ago()).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let ub = ubc(db);
    let eh = db
        .post_history
        .with((&db.post_history.post_history_type_id).in_v(vec![4, 5, 6]))
        .group_by(&db.post_history.post)
        .select(&db.post_history.creation_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.post_type_id).eq(1))
        .with((&db.post.owner_user).select(&db.user.reputation).gt(100))
        .select(Ident::<Post>::new().and((&rv).filt(|a: [i64; 3]| a[0] > 0 && a[1] - a[2] > 10)).and((&db.post.owner_user).select(&ub)).and((&eh).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, a), _), _)| (Reverse(a[0]), rep_desc(db, db.post.owner_user.get(p).unwrap())), 50, |&(((p, a), b), e)| {
        let mut f = post_fields(db, p, &["title", "body"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[0])]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend(ints(&b[1..]));
        f.extend([oint(e.map(|e| e.0)), ots(e.map(|e| e.1))]);
        f
    })
}

// WITH RecursivePostCounts AS (
// SELECT
// P.Id AS PostId,
// COUNT(C.Id) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON C.PostId = P.Id
// GROUP BY
// P.Id
// ),
// CTEUserBadges AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON B.UserId = U.Id
// GROUP BY
// U.Id, U.Reputation
// ),
// MaxPostViewCount AS (
// SELECT
// MAX(ViewCount) AS MaxViews
// FROM
// Posts
// ),
// FilteredPosts AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.ViewCount,
// RPC.CommentCount,
// CUB.UserId,
// CUB.Reputation,
// CUB.BadgeCount
// FROM
// Posts P
// JOIN
// RecursivePostCounts RPC ON RPC.PostId = P.Id
// JOIN
// CTEUserBadges CUB ON CUB.UserId = P.OwnerUserId
// WHERE
// P.ViewCount > (SELECT MaxViews FROM MaxPostViewCount) * 0.5
// ),
// FinalResults AS (
// SELECT
// FP.PostId,
// FP.Title,
// FP.ViewCount,
// FP.CommentCount,
// FP.UserId,
// FP.Reputation,
// FP.BadgeCount,
// CASE
// WHEN FP.Reputation >= 1000 THEN 'High Reputation'
// WHEN FP.Reputation BETWEEN 500 AND 999 THEN 'Medium Reputation'
// ELSE 'Low Reputation'
// END AS ReputationCategory,
// CASE
// WHEN FP.BadgeCount > 5 THEN 'Experienced User'
// ELSE 'Novice User'
// END AS UserStatus
// FROM
// FilteredPosts FP
// )
// SELECT
// F.PostId,
// F.Title,
// F.ViewCount,
// F.CommentCount,
// F.Reputation,
// F.ReputationCategory,
// F.UserStatus
// FROM
// FinalResults F
// WHERE
// F.CommentCount > 10
// ORDER BY
// F.ViewCount DESC;
fn q31614(db: &'static So) -> String {
    let maxv = db.post.select(&db.post.view_count).fold_flat(i64::MIN, |a, w| a.max(w));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.view_count).filt(move |w: i64| w as f64 > maxv as f64 * 0.5))
        .select(Ident::<Post>::new().and(comments_per_post(db).filt(|c: i64| c > 10)).and((&db.post.owner_user).select(&bu)))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), b)| {
        let r = db.user.reputation.get(db.post.owner_user.get(p).unwrap()).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c), V::I(r)]);
        f.push(V::S(if r >= 1000 {
            "High Reputation"
        } else if (500..=999).contains(&r) {
            "Medium Reputation"
        } else {
            "Low Reputation"
        }));
        f.push(V::S(if b > 5 { "Experienced User" } else { "Novice User" }));
        row(f)
    }))
}

// WITH RECURSIVE UserPostCount AS (
// SELECT
// U.Id AS UserId,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id
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
// ),
// TopUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(UPC.PostCount, 0) AS PostCount,
// COALESCE(UPC.TotalScore, 0) AS TotalScore,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.HighestBadgeClass, 0) AS HighestBadgeClass
// FROM
// Users U
// LEFT JOIN
// UserPostCount UPC ON U.Id = UPC.UserId
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// U.Reputation DESC
// LIMIT 10
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(C.Id) AS CommentCount,
// SUM(V.BountyAmount) AS TotalBounties
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
// GROUP BY
// P.OwnerUserId
// ),
// FinalStatistics AS (
// SELECT
// TU.UserId,
// TU.DisplayName,
// TU.Reputation,
// TU.PostCount,
// TU.TotalScore,
// TU.BadgeCount,
// TU.HighestBadgeClass,
// COALESCE(PS.CommentCount, 0) AS TotalComments,
// COALESCE(PS.TotalBounties, 0) AS TotalBounties
// FROM
// TopUsers TU
// LEFT JOIN
// PostStatistics PS ON TU.UserId = PS.OwnerUserId
// )
// SELECT
// F.UserId,
// F.DisplayName,
// F.Reputation,
// F.PostCount,
// F.TotalScore,
// F.BadgeCount,
// F.HighestBadgeClass,
// F.TotalComments,
// F.TotalBounties
// FROM
// FinalStatistics F
// WHERE
// F.TotalBounties > 0 OR F.TotalComments > 10
// ORDER BY
// F.Reputation DESC,
// F.TotalScore DESC;
fn q33104(db: &'static So) -> String {
    let upc = g(db).select(posts_of(db).select(&db.post.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 2], |a, c| [a[0] + 1, a[1].max(c)]);
    let base = user_base(db, UserWhere::RepGt(1000));
    let top: MatSet<Id<User>> = whole(&base).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let v8 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ps = owned(db).group_by(&db.post.owner_user).select(comments_of(db).opt().and(v8.opt())).fold([0i64; 3], |a, (c, b)| {
        let b = b.flatten();
        [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&upc).and((&ub).opt()).and((&ps).opt())).drive(|_, x| v.push(x));
    rows(v.iter().filter_map(|&(((u, a), b), p)| {
        let b = b.unwrap_or([0; 2]);
        let p = p.unwrap_or([0; 3]);
        let tb = if p[1] > 0 { p[2] } else { 0 };
        if !(tb > 0 || p[0] > 10) {
            return None;
        }
        Some(row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), V::I(a[1]), V::I(b[0]), V::I(b[1]), V::I(p[0]), V::I(tb)]))
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS Questions,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS Answers,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadgeStats AS (
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
// ups.Questions,
// ups.Answers,
// ups.TotalUpvotes,
// ups.TotalDownvotes,
// COALESCE(ubs.TotalBadges, 0) AS TotalBadges,
// COALESCE(ubs.GoldBadges, 0) AS GoldBadges,
// COALESCE(ubs.SilverBadges, 0) AS SilverBadges,
// COALESCE(ubs.BronzeBadges, 0) AS BronzeBadges,
// CASE
// WHEN ups.TotalUpvotes > ups.TotalDownvotes THEN 'Positive'
// WHEN ups.TotalUpvotes < ups.TotalDownvotes THEN 'Negative'
// ELSE 'Neutral'
// END AS VoteSentiment
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// WHERE
// ups.Questions > 5 OR COALESCE(ubs.TotalBadges, 0) > 3
// ORDER BY
// ups.TotalUpvotes DESC,
// ups.DisplayName;
fn q3335(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us)
        .and((&bc).opt())
        .filt(|(a, b): (UStats, Option<[i64; 4]>)| a.q > 5 || b.map_or(0, |b| b[0]) > 3)
        .drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[a.q, a.a, a.up, a.down]));
        f.extend(ints(&b));
        f.push(V::S(if a.up > a.down {
            "Positive"
        } else if a.up < a.down {
            "Negative"
        } else {
            "Neutral"
        }));
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS BadgeCount
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id
// ),
// RecentPostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS Answers,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AvgViews
// FROM Posts P
// WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY P.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// U.Id,
// U.DisplayName,
// U.Reputation,
// B.BadgeCount,
// R.TotalPosts,
// R.Questions,
// R.Answers,
// R.TotalScore,
// R.AvgViews,
// R.OwnerUserId
// FROM Users U
// JOIN UserBadgeCounts B ON U.Id = B.UserId
// JOIN RecentPostStats R ON U.Id = R.OwnerUserId
// ORDER BY U.Reputation DESC
// LIMIT 10
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// U.BadgeCount,
// R.TotalPosts,
// R.Questions,
// R.Answers,
// R.TotalScore,
// R.AvgViews
// FROM TopUsers U
// JOIN RecentPostStats R ON U.Id = R.OwnerUserId
// LEFT JOIN PostHistory PH ON PH.UserId = U.Id AND PH.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// WHERE R.TotalPosts > 0
// ORDER BY U.BadgeCount DESC, R.TotalScore DESC;
fn q33426(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let rp = owned_since(db, month_ago()).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let top: MatSet<Id<User>> = whole(&rp).select(Same::<Id<User>>::new().and(&db.user.reputation)).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let ph: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with((&db.post_history.creation_date).ge(month_ago())).select(&db.post_history.user).inv().collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&bu).and(&rp).and((&ph).opt())).drive(|_, (((u, b), a), _)| v.push((u, b, a)));
    rows(v.iter().map(|&(u, b, a)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        f.extend(ints(&a[..4]));
        f.push(avg(a[5], a[4]));
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
// AVG(P.Score) AS AvgScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// U.Id,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// PU.TotalPosts,
// PU.Questions,
// PU.Answers,
// PU.AvgScore,
// PU.TotalViews
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PU ON U.Id = PU.OwnerUserId
// WHERE
// U.Reputation > 1000
// )
// SELECT
// T.DisplayName,
// T.TotalPosts,
// T.Questions,
// T.Answers,
// T.BadgeCount,
// T.AvgScore,
// T.TotalViews
// FROM
// TopUsers T
// ORDER BY
// T.BadgeCount DESC, T.TotalPosts DESC
// LIMIT 10;
fn q3453(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((_, b), p)| (Reverse(b), (p.is_none(), Reverse(p.map(|p| p[0])))), 10, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        f.push(V::I(b));
        f.push(p.map_or(V::Null, |p| avg(p[3], p[0])));
        f.push(p.map_or(V::Null, |p| nullable(p[5], p[4])));
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
// ), PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM Posts P
// GROUP BY P.OwnerUserId
// ), RichStats AS (
// SELECT
// UB.DisplayName,
// COALESCE(PS.QuestionCount, 0) AS Questions,
// COALESCE(PS.AnswerCount, 0) AS Answers,
// COALESCE(PS.TotalScore, 0) AS Score,
// COALESCE(PS.TotalViews, 0) AS Views,
// UB.BadgeCount,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges
// FROM UserBadges UB
// LEFT JOIN PostStats PS ON UB.UserId = PS.OwnerUserId
// )
// SELECT
// R.DisplayName,
// R.Questions,
// R.Answers,
// R.Score,
// R.Views,
// R.BadgeCount,
// R.GoldBadges,
// R.SilverBadges,
// R.BronzeBadges,
// R.BadgeCount / NULLIF(R.Questions, 0) AS BadgePerQuestion,
// R.BadgeCount / NULLIF(R.Answers, 0) AS BadgePerAnswer
// FROM RichStats R
// WHERE R.BadgeCount > 0
// ORDER BY R.Score DESC, R.DisplayName ASC;
fn q3972(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 4], |a, ((t, s), w)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&bc).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&p));
        f.extend(ints(&b));
        f.extend([ratio(b[0], p[0]), ratio(b[0], p[1])]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// COALESCE(AVG(p.Score), 0) AS AvgScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// FinalStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ubc.BadgeCount, 0) AS BadgeCount,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.Questions, 0) AS Questions,
// COALESCE(ps.Answers, 0) AS Answers,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.AvgScore, 0) AS AvgScore
// FROM Users u
// LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// TotalPosts,
// Questions,
// Answers,
// TotalViews,
// AvgScore,
// CASE
// WHEN TotalPosts > 50 THEN 'Active Contributor'
// WHEN TotalPosts BETWEEN 20 AND 50 THEN 'Moderate Contributor'
// ELSE 'New Contributor'
// END AS ContributorLevel
// FROM FinalStats
// WHERE TotalViews > 1000
// ORDER BY TotalPosts DESC, AvgScore DESC
// LIMIT 10;
fn q4625(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 5], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s]
    });
    let avg0 = |p: [i64; 5]| if p[0] > 0 { p[4] as f64 / p[0] as f64 } else { 0.0 };
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()))
        .filt(|(_, p): ((Id<User>, i64), Option<[i64; 5]>)| p.map_or(0, |p| p[3]) > 1000)
        .drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 5]))));
    out(v, |&(_, _, p)| (Reverse(p[0]), Reverse(fkey(avg0(p)))), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b)];
        f.extend(ints(&p[..4]));
        f.push(V::F(avg0(p)));
        f.push(V::S(if p[0] > 50 {
            "Active Contributor"
        } else if (20..=50).contains(&p[0]) {
            "Moderate Contributor"
        } else {
            "New Contributor"
        }));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldCount,
// COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverCount,
// COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeCount
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
// AVG(P.ViewCount) AS AvgViewCount
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// CloseReasonCount AS (
// SELECT
// PH.UserId,
// COUNT(*) AS CloseReasonCount
// FROM
// PostHistory PH
// WHERE
// PH.PostHistoryTypeId = 10
// GROUP BY
// PH.UserId
// )
// SELECT
// U.DisplayName,
// COALESCE(UBC.GoldCount, 0) AS GoldBadges,
// COALESCE(UBC.SilverCount, 0) AS SilverBadges,
// COALESCE(UBC.BronzeCount, 0) AS BronzeBadges,
// COALESCE(PS.PostCount, 0) AS NumberOfPosts,
// COALESCE(PS.TotalScore, 0) AS TotalPostScore,
// COALESCE(PS.AvgViewCount, 0) AS AvgPostViewCount,
// COALESCE(CRC.CloseReasonCount, 0) AS CloseReasonTotal
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// LEFT JOIN
// CloseReasonCount CRC ON U.Id = CRC.UserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// U.Reputation DESC
// LIMIT 50;
fn q4845(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let cr = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()).and((&cr).opt())).drive(|_, x| v.push(x));
    out(v, |&(((u, _), _), _)| rep_desc(db, u), 50, |&(((u, b), p), c)| {
        let b = b.unwrap_or([0; 4]);
        let p = p.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b[1..]));
        f.extend([V::I(p[0]), V::I(p[1]), if p[2] > 0 { avg(p[3], p[2]) } else { V::F(0.0) }, V::I(c.unwrap_or(0))]);
        f
    })
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(v.BountyAmount) AS TotalBounties
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// TopPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// HAVING
// COUNT(c.Id) > 0 AND p.Score > 10
// )
// SELECT
// ur.DisplayName AS UserName,
// ur.Reputation,
// ur.PostCount,
// ur.TotalBounties,
// tp.Title AS TopPostTitle,
// tp.Score AS PostScore,
// tp.ViewCount AS PostViewCount,
// tp.CommentCount AS RelatedCommentCount
// FROM
// UserReputation ur
// JOIN
// TopPosts tp ON ur.UserId = tp.PostId
// ORDER BY
// ur.Reputation DESC,
// tp.Score DESC
// LIMIT 10;
fn q5004(db: &'static So) -> String {
    let uid = uids(db);
    let w = UserWhere::RepGt(1000);
    let dp = ud(db, w, posts_of(db));
    let v8 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let bf = user_base(db, w).group_by(Ident::<User>::new()).select(posts_of(db).select(v8.opt()).opt()).fold([0i64; 2], |a, p| match p.flatten().flatten() {
        Some(b) => [a[0] + 1, a[1] + b],
        None => a,
    });
    let mut v = Vec::new();
    questions_only(db)
        .with((&db.post.score).gt(10))
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db).filt(|c: i64| c > 0)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&bf).and((&dp).opt()))))
        .drive(|_, x| v.push(x));
    out(v, |&((p, _), ((u, _), _))| (rep_desc(db, u), score_desc(db, p)), 10, |&((p, c), ((u, b), d))| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d.unwrap_or(0)), nullable(b[1], b[0])];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(V::I(c));
        f
    })
}

// WITH UserReputation AS (
// SELECT Id, Reputation, CreationDate, DisplayName, LastAccessDate
// FROM Users
// WHERE Reputation > 1000
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// MAX(ph.CreationDate) AS LastEdited
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// WHERE p.PostTypeId = 1
// GROUP BY p.Id, p.Title, p.Score, p.ViewCount
// ),
// BadgeSummary AS (
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
// ur.DisplayName,
// ur.Reputation,
// ps.Title,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// bs.TotalBadges,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges,
// ps.LastEdited
// FROM UserReputation ur
// JOIN Posts p ON ur.Id = p.OwnerUserId
// JOIN PostStatistics ps ON p.Id = ps.PostId
// LEFT JOIN BadgeSummary bs ON ur.Id = bs.UserId
// WHERE ur.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// ORDER BY ur.Reputation DESC, ps.ViewCount DESC
// LIMIT 50;
fn q5020(db: &'static So) -> String {
    let bc = badge_classes(db);
    let y = year_ago();
    let base = owned(db)
        .with((&db.post.post_type_id).eq(1))
        .with((&db.post.owner_user).select(&db.user.reputation).gt(1000))
        .with((&db.post.owner_user).select(&db.user.creation_date).lt(y));
    let mut v = Vec::new();
    stats_fold(db, base, Ident::<Post>::new(), "cvh", &[]).and((&db.post.owner_user).select(Ident::<User>::new().and((&bc).opt()))).drive(|p, (s, (u, b))| v.push((p, s, u, b)));
    out(v, |&(p, _, u, _)| (rep_desc(db, u), views_desc(db, p)), 50, |&(p, s, u, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend(ints(&[s.cx, s.vx, s.up, s.down]));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.push(stat_field(&s, "hmax").unwrap());
        f
    })
}

// WITH UserBadges AS (
// SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount
// FROM Posts P
// WHERE P.CreationDate >= '2023-01-01'
// GROUP BY P.OwnerUserId
// ),
// UserActivity AS (
// SELECT U.Id, U.DisplayName, U.Reputation, UB.BadgeCount, PS.PostCount, PS.TotalScore, PS.AvgViewCount
// FROM Users U
// JOIN UserBadges UB ON U.Id = UB.UserId
// JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT U.DisplayName, U.Reputation, U.BadgeCount, U.PostCount, U.TotalScore, U.AvgViewCount
// FROM UserActivity U
// WHERE U.Reputation > 1000
// ORDER BY U.TotalScore DESC, U.BadgeCount DESC
// LIMIT 10;
fn q5032(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = owned_since(db, date(2023, 1, 1)).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and(&ps)).drive(|_, x| v.push(x));
    out(v, |&((_, b), p)| (Reverse(p[1]), Reverse(b)), 10, |&((u, b), p)| vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(p[0]), V::I(p[1]), avg(p[3], p[2])])
}

// WITH UserReputation AS (
// SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.Reputation
// ),
// PostActivity AS (
// SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
// FROM Posts p
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.OwnerUserId
// ),
// ActiveUsers AS (
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ur.BadgeCount, pa.PostCount, pa.TotalScore, pa.AvgViewCount
// FROM Users u
// JOIN UserReputation ur ON u.Id = ur.UserId
// JOIN PostActivity pa ON u.Id = pa.OwnerUserId
// WHERE u.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' AND u.Reputation > 1000
// )
// SELECT au.UserId, au.DisplayName, au.Reputation, au.BadgeCount, au.PostCount, au.TotalScore, au.AvgViewCount,
// CASE
// WHEN au.Reputation > 5000 THEN 'Expert'
// WHEN au.Reputation > 1000 THEN 'Active'
// ELSE 'Newcomer'
// END AS UserLevel
// FROM ActiveUsers au
// ORDER BY au.Reputation DESC, au.PostCount DESC
// LIMIT 10;
fn q5090(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let pa = owned_since(db, year_ago()).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).with((&db.user.last_access_date).ge(month_ago())).select(Ident::<User>::new().and(&bu).and(&pa)).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (rep_desc(db, u), Reverse(p[0])), 10, |&((u, b), p)| {
        let r = db.user.reputation.get(u).unwrap();
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(r),
            V::I(b),
            V::I(p[0]),
            V::I(p[1]),
            avg(p[3], p[2]),
            V::S(if r > 5000 {
                "Expert"
            } else if r > 1000 {
                "Active"
            } else {
                "Newcomer"
            }),
        ]
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
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// UserBadges
// WHERE
// BadgeCount > 0
// ORDER BY
// BadgeCount DESC
// LIMIT 10
// ),
// UserPosts AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// COUNT(CASE WHEN P.LastActivityDate >= cast('2024-10-01' as date) - INTERVAL '30 days' THEN 1 END) AS RecentActivity
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// TU.UserId,
// TU.DisplayName,
// TU.BadgeCount,
// TU.GoldBadges,
// TU.SilverBadges,
// TU.BronzeBadges,
// UP.TotalPosts,
// UP.Questions,
// UP.Answers,
// UP.RecentActivity
// FROM
// TopUsers TU
// JOIN
// UserPosts UP ON TU.UserId = UP.OwnerUserId
// ORDER BY
// TU.BadgeCount DESC, UP.TotalPosts DESC;
fn q5112(db: &'static So) -> String {
    let bc = badge_classes(db);
    let t0 = date(2024, 9, 1);
    let up = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.last_activity_date)).fold([0i64; 4], move |a, (t, la)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (la >= t0) as i64]
    });
    let top: MatSet<Id<User>> = whole(&bc).select(Same::new().and(&bc)).window(row_number, |(_, a): (Id<User>, [i64; 4])| a[0], desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&bc).and(&up)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p));
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
// PopularPosts AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// p.Title,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.OwnerUserId, p.Title, p.Score, p.ViewCount
// HAVING
// p.Score > 5 AND p.ViewCount > 100
// ),
// ActiveUsers AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ub.DisplayName AS UserName,
// ub.BadgeCount,
// pp.Title AS PopularPost,
// pp.Score AS PostScore,
// pp.ViewCount,
// au.PostCount AS RecentPostCount
// FROM
// UserBadges ub
// JOIN
// PopularPosts pp ON ub.UserId = pp.OwnerUserId
// JOIN
// ActiveUsers au ON ub.UserId = au.UserId
// ORDER BY
// ub.BadgeCount DESC, pp.Score DESC, au.PostCount DESC
// LIMIT 10;
fn q5115(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let au = owned_since(db, month_ago()).group_by(&db.post.owner_user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned_since(db, year_ago())
        .with((&db.post.score).gt(5))
        .with((&db.post.view_count).gt(100))
        .select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&bu).and(&au))))
        .drive(|_, x| v.push(x));
    out(v, |&(p, ((_, b), n))| (Reverse(b), score_desc(db, p), Reverse(n)), 10, |&(p, ((u, b), n))| {
        let mut f = vec![user_col(db, u, "name"), V::I(b)];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(V::I(n));
        f
    })
}

// WITH UserBadgeSummary AS (
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
// PostSummary AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// CombinedSummary AS (
// SELECT
// ubs.UserId,
// ubs.DisplayName,
// ubs.BadgeCount,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges,
// ps.QuestionCount,
// ps.AnswerCount,
// ps.TotalScore,
// ps.TotalViews
// FROM UserBadgeSummary ubs
// LEFT JOIN PostSummary ps ON ubs.UserId = ps.OwnerUserId
// )
// SELECT
// c.DisplayName,
// c.BadgeCount,
// c.GoldBadges,
// c.SilverBadges,
// c.BronzeBadges,
// COALESCE(c.QuestionCount, 0) AS QuestionCount,
// COALESCE(c.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.TotalScore, 0) AS TotalScore,
// COALESCE(c.TotalViews, 0) AS TotalViews
// FROM CombinedSummary c
// WHERE c.BadgeCount > 0
// ORDER BY c.BadgeCount DESC, c.TotalScore DESC
// LIMIT 10;
fn q5130(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 4], |a, ((t, s), w)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&bc).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or([0; 4]))));
    out(v, |&(_, b, p)| (Reverse(b[0]), Reverse(p[2])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p));
        f
    })
}

// WITH UserVoteCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY u.Id
// ),
// PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// GROUP BY p.Id, p.OwnerUserId
// ),
// PostDetails AS (
// SELECT
// p.Title,
// p.Score,
// pa.CommentCount,
// pa.VoteCount,
// pa.CloseCount,
// pa.ReopenCount,
// u.DisplayName AS OwnerName,
// u.Reputation AS OwnerReputation,
// p.OwnerUserId
// FROM Posts p
// JOIN PostActivity pa ON p.Id = pa.PostId
// JOIN Users u ON p.OwnerUserId = u.Id
// )
// SELECT
// pd.*,
// uvc.TotalVotes,
// uvc.UpVotes,
// uvc.DownVotes
// FROM PostDetails pd
// JOIN UserVoteCounts uvc ON pd.OwnerUserId = uvc.UserId
// WHERE pd.Score > 10
// ORDER BY pd.Score DESC, pd.CommentCount DESC, uvc.TotalVotes DESC
// LIMIT 100;
fn q5138(db: &'static So) -> String {
    let uvc = g(db).select(votes_by(db).select((&db.vote.vote_type).select(&db.vote_type.name)).opt()).fold([0i64; 3], |a, n| match n {
        Some(n) => [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64],
        None => a,
    });
    let mut v = Vec::new();
    stats_fold(db, owned(db).with((&db.post.score).gt(10)), Ident::<Post>::new(), "cvh", &[])
        .and(votes_per_post(db))
        .and((&db.post.owner_user).select(&uvc))
        .drive(|p, ((s, x), a)| v.push((p, s, x, a)));
    out(v, |&(p, s, _, a)| (score_desc(db, p), Reverse(s.cx), Reverse(a[0])), 100, |&(p, s, x, a)| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend(ints(&[s.cx, x, s.h10, s.h11]));
        f.extend(post_fields(db, p, &["owner", "rep", "owner_id"]));
        f.extend(ints(&a));
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
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AverageViews,
// SUM(COALESCE(v.UpVotes, 0)) AS TotalUpVotes,
// SUM(COALESCE(v.DownVotes, 0)) AS TotalDownVotes
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes
// GROUP BY PostId) v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ), UserBadges AS (
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
// ups.TotalScore,
// ups.AverageViews,
// ub.TotalBadges,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadges ub ON ups.UserId = ub.UserId
// WHERE
// ups.TotalPosts > 10 AND
// ups.TotalScore > 100
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC;
fn q5157(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 6]| p[0] > 10 && p[3] > 100).and((&bc).opt()).drive(|u, (p, b)| v.push((u, p, b)));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p[..4]));
        f.push(avg(p[5], p[4]));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        row(f)
    }))
}

// WITH UserPostingStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS Wikis,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
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
// ),
// UserVoteStats AS (
// SELECT
// V.UserId,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS Upvotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS Downvotes
// FROM Votes V
// GROUP BY V.UserId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.Questions,
// UPS.Answers,
// UPS.Wikis,
// UPS.TotalViews,
// UPS.TotalScore,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(UVS.Upvotes, 0) AS Upvotes,
// COALESCE(UVS.Downvotes, 0) AS Downvotes
// FROM UserPostingStats UPS
// LEFT JOIN UserBadges UB ON UPS.UserId = UB.UserId
// LEFT JOIN UserVoteStats UVS ON UPS.UserId = UVS.UserId
// ORDER BY UPS.TotalPosts DESC, UPS.TotalScore DESC
// LIMIT 50;
fn q5168(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 7], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 3 | 4 | 5) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s],
        None => a,
    });
    let bc = badge_classes(db);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).and((&uv).opt()).drive(|u, ((a, b), x)| v.push((u, a, b.unwrap_or([0; 4]), x.unwrap_or([0; 2]))));
    out(v, |&(_, a, _, _)| (Reverse(a[0]), (a[0] == 0, Reverse(a[6]))), 50, |&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.extend([nullable(a[5], a[4]), nullable(a[6], a[0])]);
        f.extend(ints(&b[1..]));
        f.extend(ints(&x));
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
// COUNT(CASE WHEN P.PostTypeId = 3 THEN 1 END) AS WikiCount,
// SUM(P.Score) AS TotalScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// UBadge.BadgeCount,
// UBadge.GoldBadges,
// UBadge.SilverBadges,
// UBadge.BronzeBadges,
// PStat.QuestionCount,
// PStat.AnswerCount,
// PStat.WikiCount,
// PStat.TotalScore
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UBadge ON U.Id = UBadge.UserId
// LEFT JOIN
// PostStatistics PStat ON U.Id = PStat.OwnerUserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// U.Reputation DESC,
// UBadge.BadgeCount DESC;
fn q5201(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 4], |a, (t, s)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + s]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and((&ps).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend((0..4).map(|i| oint(p.map(|p| p[i]))));
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount,
// AVG(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - U.CreationDate)) / 86400) AS AccountAgeDays
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// TopPosts AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.Score DESC
// LIMIT 10
// ),
// PostEngagement AS (
// SELECT
// P.Id AS PostId,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id
// )
// SELECT
// UR.DisplayName,
// UR.Reputation,
// UR.BadgeCount,
// UR.AccountAgeDays,
// TP.Title,
// TP.Score AS PostScore,
// TP.ViewCount AS PostViewCount,
// PE.CommentCount,
// PE.UpvoteCount,
// PE.DownvoteCount
// FROM
// UserReputation UR
// JOIN
// TopPosts TP ON UR.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = TP.PostId)
// JOIN
// PostEngagement PE ON TP.PostId = PE.PostId
// ORDER BY
// UR.Reputation DESC, TP.Score DESC;
fn q5232(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let base = owned(db).with((&db.post.post_type_id).eq(1));
    let top: MatSet<Id<Post>> = whole(&base).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(_, s)| s, desc).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let pe = stats_fold(db, questions_only(db), Ident::<Post>::new(), "cv", &[]);
    let ur = g(db).select((&db.user.creation_date).and(badges_of(db).opt())).fold((0i64, 0i64, 0.0f64), move |(r, b, s), (c, x)| (r + 1, b + x.is_some() as i64, s + hours_to(t0, c) / 86400.0));
    let mut v = Vec::new();
    (&top).select(Ident::<Post>::new().and(&pe).and((&db.post.owner_user).select(Ident::<User>::new().and(&ur)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, s), (u, (r, b, a)))| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::F(a / r as f64)];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend(ints(&[s.cx, s.up, s.down]));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
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
// COUNT(b.Id) AS BadgeCount
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
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// (ua.UpVotes - ua.DownVotes) AS NetVotes
// FROM
// UserActivity ua
// LEFT JOIN
// BadgeCounts bc ON ua.UserId = bc.UserId
// )
// SELECT
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.BadgeCount,
// us.NetVotes
// FROM
// UserSummary us
// WHERE
// us.PostCount > 5 AND
// us.BadgeCount > 0
// ORDER BY
// us.NetVotes DESC,
// us.BadgeCount DESC;
fn q5262(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&us).and((&bu).filt(|b: i64| b > 0)).filt(|(a, _): (UStats, i64)| a.n > 5).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| row(vec![user_col(db, u, "name"), V::I(a.n), V::I(a.q), V::I(a.a), V::I(b), V::I(a.up - a.down)])))
}

// WITH UserDetails AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// u.UpVotes,
// u.DownVotes,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.UpVotes, u.DownVotes
// ),
// PostMetrics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// UserPostDetails AS (
// SELECT
// ud.UserId,
// ud.DisplayName,
// ud.Reputation,
// pm.PostCount,
// pm.Questions,
// pm.Answers,
// pm.TotalViews,
// pm.AverageScore,
// ud.GoldBadges,
// ud.SilverBadges,
// ud.BronzeBadges
// FROM UserDetails ud
// LEFT JOIN PostMetrics pm ON ud.UserId = pm.OwnerUserId
// )
// SELECT
// upd.DisplayName,
// upd.Reputation,
// upd.PostCount,
// upd.Questions,
// upd.Answers,
// upd.TotalViews,
// upd.AverageScore,
// upd.GoldBadges,
// upd.SilverBadges,
// upd.BronzeBadges
// FROM UserPostDetails upd
// WHERE upd.PostCount > 10
// ORDER BY upd.Reputation DESC, upd.TotalViews DESC
// LIMIT 100;
fn q5305(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 6]| p[0] > 10)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(u, _, p)| (rep_desc(db, u), (p[4] == 0, Reverse(p[5]))), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&p[..3]));
        f.extend([nullable(p[5], p[4]), avg(p[3], p[0])]);
        f.extend(ints(&b[1..]));
        f
    })
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
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.AnswerCount) AS AvgAnswers,
// AVG(p.CommentCount) AS AvgComments
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// ActiveUsers AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.AvgAnswers, 0) AS AvgAnswers,
// COALESCE(ps.AvgComments, 0) AS AvgComments
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// WHERE
// u.Reputation > 100
// )
// SELECT
// au.UserId,
// au.DisplayName,
// au.BadgeCount,
// au.PostCount,
// au.TotalScore,
// au.TotalViews,
// au.AvgAnswers,
// au.AvgComments
// FROM
// ActiveUsers au
// ORDER BY
// au.TotalScore DESC,
// au.BadgeCount DESC
// LIMIT 10;
fn q5390(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { score, view_count, answer_count, comment_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count)).fold([0i64; 6], |a, (((s, w), an), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + an.is_some() as i64, a[4] + an.unwrap_or(0), a[5] + c]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 6]))));
    out(v, |&(_, b, p)| (Reverse(p[1]), Reverse(b)), 10, |&(u, b, p)| {
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(b),
            V::I(p[0]),
            V::I(p[1]),
            V::I(p[2]),
            if p[3] > 0 { avg(p[4], p[3]) } else { V::F(0.0) },
            if p[0] > 0 { avg(p[5], p[0]) } else { V::F(0.0) },
        ]
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
// SUM(CASE WHEN p.ViewCount > 1000 THEN 1 ELSE 0 END) AS PopularPosts,
// MAX(p.CreationDate) AS LastPostDate
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
// us.PopularPosts,
// b.TotalBadges,
// b.GoldBadges,
// b.SilverBadges,
// b.BronzeBadges,
// us.LastPostDate
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts b ON us.UserId = b.UserId
// ORDER BY
// us.Reputation DESC, us.TotalPosts DESC
// FETCH FIRST 100 ROWS ONLY;
fn q5399(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(creation_date)).opt()).fold([0, 0, 0, 0, i64::MIN], |a: [i64; 5], p| match p {
        Some(((t, w), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.map_or(false, |w| w > 1000) as i64, a[4].max(c)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a[0])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a[..4]));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.push(if a[0] == 0 { V::Null } else { V::T(a[4]) });
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT UserId, COUNT(Id) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// ),
// PostScoreStatistics AS (
// SELECT OwnerUserId,
// AVG(Score) AS AvgScore,
// SUM(ViewCount) AS TotalViews,
// COUNT(*) AS PostCount
// FROM Posts
// WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY OwnerUserId
// ),
// UserReputation AS (
// SELECT U.Id AS UserId,
// U.Reputation,
// U.DisplayName,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(PSS.AvgScore, 0) AS AvgPostScore,
// COALESCE(PSS.TotalViews, 0) AS TotalPostViews,
// COALESCE(PSS.PostCount, 0) AS TotalPosts
// FROM Users U
// LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN PostScoreStatistics PSS ON U.Id = PSS.OwnerUserId
// )
// SELECT U.DisplayName,
// U.Reputation,
// U.BadgeCount,
// U.AvgPostScore,
// U.TotalPostViews,
// U.TotalPosts
// FROM UserReputation U
// WHERE U.Reputation > 1000
// ORDER BY U.AvgPostScore DESC, U.TotalPosts DESC
// LIMIT 20;
fn q5407(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let avg0 = |p: [i64; 3]| if p[0] > 0 { p[1] as f64 / p[0] as f64 } else { 0.0 };
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 3]))));
    out(v, |&(_, _, p)| (Reverse(fkey(avg0(p))), Reverse(p[0])), 20, |&(u, b, p)| {
        vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::F(avg0(p)), V::I(p[2]), V::I(p[0])]
    })
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
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM
// Posts P
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// U.Id,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.TotalScore, 0) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UA.DisplayName,
// UA.BadgeCount,
// UA.PostCount,
// UA.TotalViews,
// UA.TotalScore
// FROM
// UserActivity UA
// WHERE
// UA.BadgeCount > 0 OR UA.PostCount > 0
// ORDER BY
// UA.TotalScore DESC, UA.TotalViews DESC
// LIMIT 10;
fn q5417(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&db.post.score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()))
        .filt(|((_, b), p): ((Id<User>, i64), Option<[i64; 3]>)| b > 0 || p.map_or(0, |p| p[0]) > 0)
        .drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 3]))));
    out(v, |&(_, _, p)| (Reverse(p[2]), Reverse(p[1])), 10, |&(u, b, p)| vec![user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2])])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("26513", q26513),
    ("26637", q26637),
    ("27244", q27244),
    ("27339", q27339),
    ("27582", q27582),
    ("27651", q27651),
    ("27684", q27684),
    ("27709", q27709),
    ("27739", q27739),
    ("28007", q28007),
    ("28010", q28010),
    ("28165", q28165),
    ("28173", q28173),
    ("28604", q28604),
    ("28646", q28646),
    ("2885", q2885),
    ("28879", q28879),
    ("29086", q29086),
    ("29321", q29321),
    ("29487", q29487),
    ("29631", q29631),
    ("29726", q29726),
    ("29842", q29842),
    ("30841", q30841),
    ("31194", q31194),
    ("31614", q31614),
    ("33104", q33104),
    ("3335", q3335),
    ("33426", q33426),
    ("3453", q3453),
    ("3972", q3972),
    ("4625", q4625),
    ("4845", q4845),
    ("5004", q5004),
    ("5020", q5020),
    ("5032", q5032),
    ("5090", q5090),
    ("5112", q5112),
    ("5115", q5115),
    ("5130", q5130),
    ("5138", q5138),
    ("5157", q5157),
    ("5168", q5168),
    ("5201", q5201),
    ("5232", q5232),
    ("5262", q5262),
    ("5305", q5305),
    ("5390", q5390),
    ("5399", q5399),
    ("5407", q5407),
    ("5417", q5417),
];
