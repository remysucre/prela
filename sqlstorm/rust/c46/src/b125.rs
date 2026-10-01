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

// --- batch 125 --------------------------------------------------------------

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerName,
// COUNT(c.Id) AS TotalComments,
// AVG(v.BountyAmount) AS AverageBounty
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
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS TotalEdits,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// ps.CommentCount,
// ps.OwnerName,
// ps.TotalComments,
// COALESCE(pht.TotalEdits, 0) AS TotalEdits,
// pht.LastEditDate,
// ps.AverageBounty
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats pht ON ps.PostId = pht.PostId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q14172(db: &'static So) -> String {
    let hf = history_n_max(db);
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cv", &[]).and((&hf).opt()).drive(|p, (s, h)| v.push((p, s, h)));
    out(v, |&(p, _, _)| score_views(db, p), 100, |&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "answers", "comments", "owner"]);
        f.extend([V::I(s.cx), V::I(h.map_or(0, |h| h.0)), ots(h.map(|h| h.1)), stat_field(&s, "bounty_avg").unwrap()]);
        f
    })
}

// WITH PostSummary AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// U.Reputation AS OwnerReputation
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= '2023-01-01'
// ),
// VoteSummary AS (
// SELECT
// PostId,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(CASE WHEN V.VoteTypeId = 10 THEN 1 END) AS DeleteVotes
// FROM
// Votes V
// GROUP BY
// PostId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.FavoriteCount,
// PS.OwnerReputation,
// COALESCE(VS.UpVotes, 0) AS UpVotes,
// COALESCE(VS.DownVotes, 0) AS DownVotes,
// COALESCE(VS.DeleteVotes, 0) AS DeleteVotes
// FROM
// PostSummary PS
// LEFT JOIN
// VoteSummary VS ON PS.PostId = VS.PostId
// ORDER BY
// PS.Score DESC, PS.CreationDate DESC
// LIMIT 100;
fn q14173(db: &'static So) -> String {
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 10) as i64]);
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and((&vs).opt())).drive(|_, x| v.push(x));
    out(v, |&(p, _)| (score_desc(db, p), newest(db, p)), 100, |&(p, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "rep"]);
        f.extend(ints(&x.unwrap_or([0; 3])));
        f
    })
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// ),
// UserBadges AS (
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
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// COALESCE(b.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(b.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(b.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM
// UserPostCounts u
// LEFT JOIN
// UserBadges b ON u.UserId = b.UserId
// ORDER BY
// u.Reputation DESC,
// u.PostCount DESC;
fn q14182(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    user_posts_q(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&a[..3]));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// DATE_TRUNC('month', CreationDate) AS Month,
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniqueUsers,
// SUM(VoteCount) AS TotalVotes
// FROM
// Posts
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) AS VoteCounts ON Posts.Id = VoteCounts.PostId
// GROUP BY
// Month
// ),
// UserStats AS (
// SELECT
// DATE_TRUNC('month', CreationDate) AS Month,
// COUNT(*) AS TotalUsers,
// SUM(Reputation) AS TotalReputation
// FROM
// Users
// GROUP BY
// Month
// )
// SELECT
// ps.Month,
// ps.TotalPosts,
// ps.UniqueUsers,
// ps.TotalVotes,
// us.TotalUsers,
// us.TotalReputation
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.Month = us.Month
// ORDER BY
// ps.Month DESC;
fn q14184(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, .. } = &db.post;
    let pm = db.post.group_by(creation_date.map(trunc_month)).select(votes_per_post(db)).fold([0i64; 3], |a, x| [a[0] + 1, a[1] + (x > 0) as i64, a[2] + x]);
    let du = db.post.group_by(creation_date.map(trunc_month)).select(owner_user_id).count_distinct();
    let um = db.user.group_by((&db.user.creation_date).map(trunc_month)).select(&db.user.reputation).fold([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let mut v = Vec::new();
    (&pm).and((&du).opt()).and(&um).drive(|m, ((a, d), u)| v.push((m, a, d.unwrap_or(0), u)));
    rows(v.iter().map(|&(m, a, d, u)| row(vec![V::T(m), V::I(a[0]), V::I(d), nullable(a[2], a[1]), V::I(u[0]), V::I(u[1])])))
}

// WITH UserMetrics AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.Views,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.Reputation, u.Views
// ),
// PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score
// )
// SELECT
// um.UserId,
// um.Reputation,
// um.Views,
// um.PostCount,
// um.CommentCount AS UserCommentCount,
// um.UpVotes,
// um.DownVotes,
// pm.PostId,
// pm.Title,
// pm.ViewCount,
// pm.Score,
// pm.CommentCount AS PostCommentCount,
// pm.VoteCount AS PostVoteCount
// FROM
// UserMetrics um
// JOIN
// PostMetrics pm ON um.UserId = pm.PostId
// ORDER BY
// um.Reputation DESC,
// pm.Score DESC;
fn q14187(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and((&dc).opt()))))
        .drive(|_, y| v.push(y));
    rows(v.iter().map(|&(((p, c), x), (((u, a), dp), dc))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), user_col(db, u, "uviews"), V::I(dp.unwrap_or(0)), V::I(dc.unwrap_or(0)), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
        f.extend([V::I(c), V::I(x)]);
        row(f)
    }))
}

// WITH PostCounts AS (
// SELECT
// PostTypeId,
// COUNT(*) AS PostCount
// FROM
// Posts
// GROUP BY
// PostTypeId
// ),
// UserActivity AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS TotalPosts,
// SUM(ViewCount) AS TotalViews,
// SUM(Score) AS TotalScore
// FROM
// Posts
// WHERE
// OwnerUserId IS NOT NULL
// GROUP BY
// OwnerUserId
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
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(PC.PostCount, 0) AS TotalPostsByUser,
// COALESCE(UA.TotalViews, 0) AS TotalViewsByUser,
// COALESCE(UA.TotalScore, 0) AS TotalScoreByUser,
// COALESCE(BC.TotalBadges, 0) AS TotalBadgesByUser
// FROM
// Users U
// LEFT JOIN
// UserActivity UA ON U.Id = UA.OwnerUserId
// LEFT JOIN
// PostCounts PC ON U.Id = PC.PostTypeId
// LEFT JOIN
// BadgeCounts BC ON U.Id = BC.UserId
// ORDER BY
// TotalScoreByUser DESC,
// TotalPostsByUser DESC;
fn q14191(db: &'static So) -> String {
    let pc = db.post.group_by(&db.post.post_type_id).fold(0i64, |a, _| a + 1);
    let op = owner_posts(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&op).opt()).and((&db.user.origid).select(&pc).opt()).and(&bu)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, a), n), b)| {
        let a = a.unwrap_or([0; 6]);
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n.unwrap_or(0)), V::I(a[4]), V::I(a[5]), V::I(b)])
    }))
}

// WITH UserPosts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserBadges AS (
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
// UP.UserId,
// UP.DisplayName,
// UP.TotalPosts,
// UP.TotalQuestions,
// UP.TotalAnswers,
// UP.TotalScore,
// UP.TotalViews,
// COALESCE(UB.TotalBadges, 0) AS TotalBadges,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPosts UP
// LEFT JOIN
// UserBadges UB ON UP.UserId = UB.UserId
// ORDER BY
// UP.TotalScore DESC;
fn q14192(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[5], a[0]), nullable(a[4], a[3])]);
        f.extend(ints(&b));
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
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount,
// COUNT(CASE WHEN b.Id IS NOT NULL THEN 1 END) AS TotalBadges
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
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes,
// p.CreationDate,
// p.LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.CreationDate, p.LastActivityDate
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalTagWikis,
// ups.TotalScore,
// ups.AvgViewCount,
// ups.TotalBadges,
// pe.PostId,
// pe.TotalComments,
// pe.TotalVotes,
// pe.CreationDate,
// pe.LastActivityDate
// FROM
// UserPostStatistics ups
// JOIN
// PostEngagement pe ON ups.UserId = pe.PostId
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC;
fn q14193(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "#45", "score_sum", "views_avg", "#bx"].iter().map(|c| ustat_field(&a, c)));
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(s.cx), V::I(s.vx)]);
        f.extend(post_fields(db, p, &["created", "activity"]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// COALESCE(PH.EditHistoryCount, 0) AS EditCount,
// COALESCE(C.CommentCount, 0) AS CommentCount,
// COALESCE(V.UpvoteCount, 0) AS UpvoteCount,
// COALESCE(V.DownvoteCount, 0) AS DownvoteCount
// FROM
// Posts P
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS EditHistoryCount
// FROM
// PostHistory
// GROUP BY
// PostId
// ) PH ON P.Id = PH.PostId
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
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) V ON P.Id = V.PostId
// )
// SELECT
// PS.PostId,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.EditCount,
// PS.CommentCount,
// PS.UpvoteCount,
// PS.DownvoteCount,
// U.Id AS UserId,
// U.DisplayName AS UserDisplayName,
// U.Reputation
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostId = U.Id
// ORDER BY
// PS.CreationDate DESC
// LIMIT 100;
fn q14200(db: &'static So) -> String {
    let uid = uids(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(history_per_post(db)).and(comments_per_post(db)).and((&pv).opt()).and((&db.post.origid).select(&uid)))
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| newest(db, p), 100, |&((((p, h), c), x), u)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "created", "score", "views", "answers"]);
        f.extend([V::I(h), V::I(c), V::I(x[1]), V::I(x[2]), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")]);
        f
    })
}

// WITH PostSummary AS (
// SELECT
// PostTypeId,
// COUNT(*) AS TotalPosts,
// SUM(ViewCount) AS TotalViews,
// SUM(Score) AS TotalScore
// FROM
// Posts
// GROUP BY
// PostTypeId
// )
// , UserReputation AS (
// SELECT
// p.OwnerUserId,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.OwnerUserId
// )
// , CommentSummary AS (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// )
// SELECT
// p.PostTypeId,
// ps.TotalPosts,
// ps.TotalViews,
// ps.TotalScore,
// ur.AverageReputation,
// cs.CommentCount
// FROM
// PostSummary ps
// LEFT JOIN
// Posts p ON ps.PostTypeId = p.PostTypeId
// LEFT JOIN
// UserReputation ur ON p.OwnerUserId = ur.OwnerUserId
// LEFT JOIN
// CommentSummary cs ON p.Id = cs.PostId
// ORDER BY
// ps.TotalPosts DESC;
fn q14208(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let ps = db.post.group_by(&db.post.post_type_id).select(view_count.opt().and(score)).fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let cs = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&db.post.post_type_id).select(&ps)).and((&db.post.owner_user).select(&db.user.reputation).opt()).and((&cs).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, a), r), c)| {
        let mut f = post_fields(db, p, &["type_id"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), ofloat(r.map(|r| r as f64)), oint(c)]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// AVG(P.Score) AS AverageScore,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) / NULLIF(COUNT(P.Id), 0) AS ScorePerPost
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id
// ),
// UserBadgeStats AS (
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
// U.Id,
// U.DisplayName,
// UPS.PostCount,
// UPS.Questions,
// UPS.Answers,
// UPS.Wikis,
// UPS.AverageScore,
// UPS.TotalViews,
// UBS.BadgeCount,
// UBS.GoldBadges,
// UBS.SilverBadges,
// UBS.BronzeBadges,
// UPS.ScorePerPost
// FROM
// Users U
// LEFT JOIN
// UserPostStats UPS ON U.Id = UPS.UserId
// LEFT JOIN
// UserBadgeStats UBS ON U.Id = UBS.UserId
// ORDER BY
// UPS.PostCount DESC, UPS.AverageScore DESC;
fn q14209(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)).opt()).fold([0i64; 7], |a, p| match p {
        Some(((t, w), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.extend([avg(a[6], a[0]), nullable(a[5], a[4])]);
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.push(ratio(a[6], a[0]));
        row(f)
    }))
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS TotalCommentCount,
// AVG(vote.VoteTypeId) AS AvgVoteType
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes vote ON p.Id = vote.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Reputation
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(*) AS RevisionCount,
// MAX(ph.CreationDate) AS LastEditDate,
// MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS ClosedPosts,
// MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenedPosts
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.PostTypeId,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.TotalCommentCount,
// ps.FavoriteCount,
// ps.OwnerReputation,
// pht.RevisionCount,
// pht.LastEditDate,
// pht.ClosedPosts,
// pht.ReopenedPosts
// FROM
// PostSummary ps
// LEFT JOIN
// PostHistoryStats pht ON ps.PostId = pht.PostId
// ORDER BY
// ps.CreationDate DESC
// FETCH FIRST 100 ROWS ONLY;
fn q14211(db: &'static So) -> String {
    let hf = db
        .post_history
        .group_by(&db.post_history.post)
        .select((&db.post_history.creation_date).and(&db.post_history.post_history_type_id))
        .fold((0i64, i64::MIN, 0i64, 0i64), |(n, m, a, b), (d, t)| (n + 1, m.max(d), a.max((t == 10) as i64), b.max((t == 11) as i64)));
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and((&hf).opt()).drive(|p, (s, h)| v.push((p, s, h)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views", "answers"]);
        f.push(V::I(s.cx));
        f.extend(post_fields(db, p, &["favorites", "rep"]));
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1)), oint(h.map(|h| h.2)), oint(h.map(|h| h.3))]);
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
// SUM(CASE WHEN V.VoteTypeId = 10 THEN 1 ELSE 0 END) AS Deletions
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// LEFT JOIN Votes V ON U.Id = V.UserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// PS.PostCount,
// PS.QuestionCount,
// PS.AnswerCount,
// PS.TotalViews,
// PS.TotalScore,
// US.BadgeCount,
// US.UpVotes,
// US.DownVotes,
// US.Deletions
// FROM UserStats US
// JOIN PostStats PS ON US.UserId = PS.OwnerUserId
// JOIN Users U ON US.UserId = U.Id
// ORDER BY U.Reputation DESC;
fn q14242(db: &'static So) -> String {
    let us = g(db).select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (_, t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(10)) as i64]
    });
    let bu = badges_per_user(db);
    let op = owner_posts(db);
    let mut v = Vec::new();
    (&us).and(&op).and(&bu).drive(|u, ((a, p), b)| v.push((u, a, p, b)));
    rows(v.iter().map(|&(u, a, p, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&p[..3]));
        f.extend([nullable(p[4], p[3]), V::I(p[5]), V::I(b)]);
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount,
// AVG(COALESCE(p.AcceptedAnswerId, 0)) AS AcceptedAnswerRatio
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS EditCount,
// MAX(ph.CreationDate) AS LastEdited
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// ),
// FinalStats AS (
// SELECT
// up.UserId,
// up.PostCount,
// up.TotalScore,
// up.AvgViewCount,
// ph.EditCount,
// ph.LastEdited
// FROM
// UserPostStats up
// LEFT JOIN
// PostHistoryStats ph ON up.UserId = ph.PostId
// )
// SELECT
// UserId,
// PostCount,
// TotalScore,
// AvgViewCount,
// EditCount,
// LastEdited
// FROM
// FinalStats
// ORDER BY
// TotalScore DESC, PostCount DESC;
fn q14250(db: &'static So) -> String {
    let pid = pids(db);
    let hf = history_n_max(db);
    let uf = g(db).select(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).and((&db.user.origid).select(&pid).select(&hf).opt()).drive(|u, (a, h)| v.push((u, a, h)));
    rows(v.iter().map(|&(u, a, h)| row(vec![user_col(db, u, "uid"), V::I(a[0]), V::I(a[1]), or0(a[2], a[0]), oint(h.map(|h| h.0)), ots(h.map(|h| h.1))])))
}

// WITH PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners
// FROM
// Posts
// ),
// VoteCounts AS (
// SELECT
// COUNT(*) AS TotalVotes,
// COUNT(DISTINCT UserId) AS UniqueVoters
// FROM
// Votes
// ),
// CommentCounts AS (
// SELECT
// COUNT(*) AS TotalComments,
// COUNT(DISTINCT UserId) AS UniqueCommenters
// FROM
// Comments
// ),
// UserCounts AS (
// SELECT
// COUNT(*) AS TotalUsers,
// SUM(CASE WHEN Reputation > 0 THEN 1 ELSE 0 END) AS ActiveUsers
// FROM
// Users
// )
// SELECT
// pc.TotalPosts,
// pc.UniquePostOwners,
// vc.TotalVotes,
// vc.UniqueVoters,
// cc.TotalComments,
// cc.UniqueCommenters,
// uc.TotalUsers,
// uc.ActiveUsers
// FROM
// PostCounts pc,
// VoteCounts vc,
// CommentCounts cc,
// UserCounts uc;
fn q14251(db: &'static So) -> String {
    let du = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let dv = one(whole(db.vote.iq()).select(&db.vote.user_id).count_distinct());
    let dc = one(whole(db.comment.iq()).select(&db.comment.user_id).count_distinct());
    let act = count(db.user.with((&db.user.reputation).gt(0)));
    row(vec![V::I(count(db.post.iq())), V::I(du), V::I(count(db.vote.iq())), V::I(dv), V::I(count(db.comment.iq())), V::I(dc), V::I(count(db.user.iq())), V::I(act)])
}

// WITH PostStatistics AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners,
// SUM(CASE WHEN OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostsWithUsers,
// AVG(ViewCount) AS AverageViews,
// AVG(Score) AS AverageScore,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
// FROM
// Posts
// ),
// UserStatistics AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation,
// MAX(Reputation) AS MaxReputation,
// MIN(Reputation) AS MinReputation
// FROM
// Users
// ),
// VoteStatistics AS (
// SELECT
// COUNT(*) AS TotalVotes,
// COUNT(DISTINCT UserId) AS UniqueVoters,
// AVG(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS AverageUpvotes,
// AVG(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS AverageDownvotes
// FROM
// Votes
// )
// SELECT
// ps.TotalPosts,
// ps.UniquePostOwners,
// ps.PostsWithUsers,
// ps.AverageViews,
// ps.AverageScore,
// ps.TotalQuestions,
// ps.TotalAnswers,
// us.TotalUsers,
// us.AverageReputation,
// us.MaxReputation,
// us.MinReputation,
// vs.TotalVotes,
// vs.UniqueVoters,
// vs.AverageUpvotes,
// vs.AverageDownvotes
// FROM
// PostStatistics ps,
// UserStatistics us,
// VoteStatistics vs;
fn q14256(db: &'static So) -> String {
    let Post { view_count, score, post_type_id, owner_user_id, .. } = &db.post;
    let p = db.post.select(view_count.opt().and(score).and(post_type_id).and(owner_user_id.opt())).fold_flat([0i64; 7], |a, (((w, s), t), o)| {
        [a[0] + 1, a[1] + o.is_some() as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s, a[5] + (t == 1) as i64, a[6] + (t == 2) as i64]
    });
    let du = one(whole(db.post.iq()).select(owner_user_id).count_distinct());
    let u = db.user.select(&db.user.reputation).fold_flat([0, 0, i64::MIN, i64::MAX], |a: [i64; 4], r| [a[0] + 1, a[1] + r, a[2].max(r), a[3].min(r)]);
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let dv = one(whole(db.vote.iq()).select(&db.vote.user_id).count_distinct());
    row(vec![
        V::I(p[0]),
        V::I(du),
        V::I(p[1]),
        avg(p[3], p[2]),
        avg(p[4], p[0]),
        V::I(p[5]),
        V::I(p[6]),
        V::I(u[0]),
        avg(u[1], u[0]),
        omax(u[2], u[0]),
        omax(u[3], u[0]),
        V::I(x[0]),
        V::I(dv),
        avg(x[1], x[0]),
        avg(x[2], x[0]),
    ])
}

// WITH UserPostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(Id) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// up.TotalPosts,
// up.QuestionCount,
// up.AnswerCount
// FROM
// Users u
// LEFT JOIN UserPostCounts up ON u.Id = up.OwnerUserId
// ),
// PostStatistics AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViews
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.TotalPosts,
// u.QuestionCount,
// u.AnswerCount,
// ps.PostType,
// ps.PostCount,
// ps.AverageScore,
// ps.AverageViews
// FROM
// UserReputation u
// JOIN
// PostStatistics ps ON u.QuestionCount > 0 AND ps.PostType = 'Question'
// ORDER BY
// u.Reputation DESC, ps.PostCount DESC;
fn q14265(db: &'static So) -> String {
    let f = by_key(db.post.with(name(db).eq("Question")), name(db), (&db.post.score).and((&db.post.view_count).opt()), [0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let op = owner_posts(db);
    let mut v = Vec::new();
    (&op).filt(|a: [i64; 6]| a[1] > 0).cross(&f).drive(|(u, k), (a, q)| v.push((u, a, k, q)));
    rows(v.iter().map(|&(u, a, k, q)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&a[..3]));
        f.extend([V::S(k), V::I(q[0]), avg(q[1], q[0]), avg(q[3], q[2])]);
        row(f)
    }))
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// UserSummary AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.AnswerCount,
// us.UserId,
// us.BadgeCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostSummary ps
// JOIN
// UserSummary us ON ps.PostId = us.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q14266(db: &'static So) -> String {
    let uid = uids(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt())).fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let an = per_post_distinct(db, children_of(db));
    let mut v = Vec::new();
    questions_only(db)
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&an).opt()).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us))))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| score_views(db, p), 100, |&(((p, c), a), (u, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a.unwrap_or(0)), user_col(db, u, "uid")]);
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
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
// p.Score,
// pt.Name AS PostTypeName,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, pt.Name
// )
// SELECT
// us.UserId,
// us.TotalPosts,
// us.Questions,
// us.Answers,
// us.TotalViews,
// us.TotalScore,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.PostTypeName,
// ps.TotalComments
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.TotalScore DESC, us.TotalPosts DESC;
fn q14277(db: &'static So) -> String {
    let uid = uids(db);
    let up = upqa(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&up))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), (u, a))| {
        let mut f = vec![user_col(db, u, "uid"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), nullable(a[5], a[0])];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "type"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.CreationDate,
// p.LastActivityDate,
// u.Reputation AS UserReputation,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, u.Reputation
// ),
// VoteSummary AS (
// SELECT
// p.Id AS PostId,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.Score,
// ps.ViewCount,
// ps.CreationDate,
// ps.LastActivityDate,
// ps.UserReputation,
// ps.CommentCount,
// vs.VoteCount
// FROM
// PostSummary ps
// LEFT JOIN
// VoteSummary vs ON ps.PostId = vs.PostId
// ORDER BY
// ps.LastActivityDate DESC;
fn q14279(db: &'static So) -> String {
    let mut v = Vec::new();
    since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), x)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "activity", "rep"]);
        f.extend([V::I(c), V::I(x)]);
        row(f)
    }))
}

// WITH UserPostActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// COUNT(c.Id) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostHistorySummary AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS TotalPostHistories,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalPostClosures,
// SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS TotalPostReopenings
// FROM
// PostHistory ph
// GROUP BY
// ph.UserId
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.TotalQuestions,
// u.TotalAnswers,
// u.TotalScore,
// u.TotalViews,
// u.TotalComments,
// pht.TotalPostHistories,
// pht.TotalPostClosures,
// pht.TotalPostReopenings
// FROM
// UserPostActivity u
// LEFT JOIN
// PostHistorySummary pht ON u.UserId = pht.UserId
// ORDER BY
// u.TotalScore DESC, u.TotalPosts DESC;
fn q14298(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post);
    let hf = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 10) as i64, a[2] + (t == 11) as i64]);
    let mut v = Vec::new();
    (&us).and((&hf).opt()).drive(|u, (a, h)| v.push((u, a, h)));
    rows(v.iter().map(|&(u, a, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "score_sum0", "views_sum0", "#cx"].iter().map(|c| ustat_field(&a, c)));
        f.extend((0..3).map(|i| oint(h.map(|h| h[i]))));
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// COALESCE(PH.EditCount, 0) AS EditCount,
// COALESCE(C.Count, 0) AS CommentCount,
// COALESCE(V.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(V.DownVoteCount, 0) AS DownVoteCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
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
// ) PH ON PH.PostId = P.Id
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS Count
// FROM
// Comments
// GROUP BY
// PostId
// ) C ON C.PostId = P.Id
// LEFT JOIN (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) V ON V.PostId = P.Id
// )
// SELECT
// *
// FROM
// PostMetrics
// ORDER BY
// CreationDate DESC
// LIMIT 100;
fn q14299(db: &'static So) -> String {
    let ed = history_of_types(db, &[4, 5, 6]);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(&ed).and(comments_per_post(db)).and((&pv).opt())).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, e), c), x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(e), V::I(c), V::I(x[1]), V::I(x[2])]);
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
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// UBad.BadgeCount,
// UBad.GoldBadges,
// UBad.SilverBadges,
// UBad.BronzeBadges,
// PStats.TotalPosts,
// PStats.Questions,
// PStats.Answers,
// PStats.TagWikis
// FROM
// Users U
// JOIN
// UserBadges UBad ON U.Id = UBad.UserId
// JOIN
// PostStats PStats ON U.Id = PStats.OwnerUserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// U.Reputation DESC
// LIMIT 100;
fn q14300(db: &'static So) -> String {
    let bc = badge_classes(db);
    let pc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&bc).opt()).and(&pc)).drive(|_, x| v.push(x));
    out(v, |&((u, _), _)| rep_desc(db, u), 100, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b.unwrap_or([0; 4])));
        f.extend(ints(&p));
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// ARRAY_AGG(t.TagName) AS Tags,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14303(db: &'static So) -> String {
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let prod = || (&ex).select(&db.tag.tag_name).opt().and(votes_of(db).opt());
    let base = || owned_since(db, date(2023, 1, 1));
    let f = base().group_by(Ident::<Post>::new()).select(prod()).fold(0i64, |a, (_, x)| a + x.is_some() as i64);
    let tags = base().group_by(Ident::<Post>::new()).select(prod().map(|(t, _): (Option<Str>, Option<Id<Vote>>)| t)).buf_fold(|ts| &*Box::leak(ts.into_vec().into_boxed_slice()));
    let mut v = Vec::new();
    (&f).and(&tags).drive(|p, (a, t)| v.push((p, a, t)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, a, t)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep"]);
        f.extend([V::L(t.iter().map(|&n| ostr(n)).collect()), V::I(a)]);
        f
    })
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.CreationDate, p.Score, p.ViewCount
// ),
// UserSummary AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// AVG(u.Reputation) AS AverageReputation,
// SUM(p.ViewCount) AS TotalViews
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
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.AverageReputation,
// us.TotalViews
// FROM
// PostSummary ps
// JOIN
// UserSummary us ON ps.PostId = us.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q14304(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(["#bx", "rep_avg", "views_sum"].iter().map(|c| ustat_field(&a, c)));
        row(f)
    }))
}

// WITH Benchmark AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadgeCount,
// COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadgeCount,
// COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadgeCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
// WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 MONTH'
// GROUP BY p.Id, p.Title, p.CreationDate
// )
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(VoteCount) AS AvgVoteCount,
// AVG(BadgeCount) AS AvgBadgeCount,
// AVG(GoldBadgeCount) AS AvgGoldBadgeCount,
// AVG(SilverBadgeCount) AS AvgSilverBadgeCount,
// AVG(BronzeBadgeCount) AS AvgBronzeBadgeCount
// FROM Benchmark;
fn q14305(db: &'static So) -> String {
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let bu = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let cls = since(db, month_ago())
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and((&db.post.owner_user_id).select(&bidx).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (_, b)| [a[0] + (b == Some(1)) as i64, a[1] + (b == Some(2)) as i64, a[2] + (b == Some(3)) as i64]);
    let t = (&cls)
        .and(comments_per_post(db))
        .and(votes_per_post(db))
        .and((&db.post.owner_user_id).select(&bu).opt())
        .fold_flat([0i64; 7], |a, (((s, c), x), b)| [a[0] + 1, a[1] + c, a[2] + x, a[3] + b.unwrap_or(0), a[4] + s[0], a[5] + s[1], a[6] + s[2]]);
    let mut f = vec![V::I(t[0])];
    f.extend((1..7).map(|i| avg(t[i], t[0])));
    row(f)
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// COALESCE(v.UpVotes, 0) AS UpVotes,
// COALESCE(v.DownVotes, 0) AS DownVotes,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// p.ViewCount,
// p.Score
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId) v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(Id) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT
// UserId,
// COUNT(Id) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId) b ON u.Id = b.UserId
// WHERE
// p.CreationDate BETWEEN '2023-01-01' AND '2023-12-31'
// ORDER BY
// p.CreationDate DESC;
fn q14307(db: &'static So) -> String {
    let pv = post_votes(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.creation_date).ge(date(2023, 1, 1)))
        .with((&db.post.creation_date).le(date(2023, 12, 31)))
        .select(Ident::<Post>::new().and((&pv).opt()).and(comments_per_post(db)).and((&db.post.owner_user).select(&bu)))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, x), c), b)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "uid", "owner"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(c), V::I(b)]);
        f.extend(post_fields(db, p, &["views", "score"]));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// BadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges,
// COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.Questions,
// ups.Answers,
// ups.TotalScore,
// ups.TotalViews,
// COALESCE(bc.TotalBadges, 0) AS TotalBadges,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges
// FROM UserPostStats ups
// LEFT JOIN BadgeCounts bc ON ups.UserId = bc.UserId
// ORDER BY ups.TotalPosts DESC;
fn q14309(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[5], a[0]), nullable(a[4], a[3])]);
        f.extend(ints(&b));
        row(f)
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
// SUM(p.Score) AS TotalScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT ph.Id) AS EditHistoryCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.Questions,
// us.Answers,
// us.TotalScore,
// us.UpVotes,
// us.DownVotes,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score AS PostScore,
// ps.ViewCount,
// ps.CommentCount,
// ps.EditHistoryCount
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.Reputation DESC,
// ps.Score DESC;
fn q14313(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "ch", &[]);
    let mut v = Vec::new();
    (&pf)
        .and(history_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, ((s, h), ((u, a), d))| v.push((p, s, h, u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, h, u, a, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d)];
        f.extend(["#q", "#a", "score_sum", "#up", "#down"].iter().map(|c| ustat_field(&a, c)));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(s.cx), V::I(h)]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AvgViews,
// AVG(p.Score) AS AvgScore,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers
// FROM
// Posts p
// INNER JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation
// FROM
// Users
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.AvgViews,
// ps.AvgScore,
// ps.UniqueUsers,
// us.TotalUsers,
// us.AvgReputation
// FROM
// PostStats ps,
// UserStats us
// ORDER BY
// ps.TotalPosts DESC;
fn q14316(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.view_count).opt().and(&db.post.score), [0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let du = db.post.group_by(name(db)).select(&db.post.owner_user_id).count_distinct();
    let u = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let mut v = Vec::new();
    (&f).and((&du).opt()).drive(|k, (a, d)| v.push((k, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), V::I(d), V::I(u[0]), avg(u[1], u[0])])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(v.BountyAmount) AS TotalBounty,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(c.Id) AS CommentCount,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.TotalBounty,
// ps.PostId,
// ps.CommentCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.AnswerCount
// FROM
// UserStats us
// LEFT JOIN PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.BadgeCount DESC, ps.CommentCount DESC;
fn q14318(db: &'static So) -> String {
    let uid = uids(db);
    let pid = pids(db);
    let us = g(db).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt())).fold([0i64; 3], |a, (b, v)| {
        let bo = v.and_then(|v| v.1);
        [a[0] + b.is_some() as i64, a[1] + bo.is_some() as i64, a[2] + bo.unwrap_or(0)]
    });
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&us).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&pf)).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1])];
        match p {
            Some((p, s)) => {
                f.extend(post_fields(db, p, &["id"]));
                f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(if db.post.post_type_id.get(p).unwrap() == 1 { s.rows } else { 0 })]);
            }
            None => f.extend(nulls(5)),
        }
        row(f)
    }))
}

// WITH UserVotes AS (
// SELECT
// u.Id AS UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId IN (2, 4) THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY u.Id
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// COUNT(c.Id) AS TotalComments,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY p.Id, p.OwnerUserId
// )
// SELECT
// u.DisplayName AS UserName,
// u.Reputation,
// uv.TotalVotes,
// uv.UpVotes,
// uv.DownVotes,
// ps.PostId,
// ps.TotalComments,
// ps.TotalAnswers,
// ps.TotalUpVotes,
// ps.TotalDownVotes
// FROM Users u
// JOIN UserVotes uv ON u.Id = uv.UserId
// JOIN PostStatistics ps ON ps.OwnerUserId = u.Id
// ORDER BY u.Reputation DESC, uv.TotalVotes DESC;
fn q14321(db: &'static So) -> String {
    let uv = g(db).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + matches!(t, 2 | 4) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(Ident::<User>::new().and(&uv))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(s.cx), V::I((db.post.post_type_id.get(p).unwrap() == 2) as i64), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(VoteTypeCounts.UpVotes) AS TotalUpVotes,
// SUM(VoteTypeCounts.DownVotes) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// (
// SELECT
// V.UserId,
// P.Id AS PostId,
// SUM(CASE WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes V
// JOIN
// VoteTypes VT ON V.VoteTypeId = VT.Id
// JOIN
// Posts P ON V.PostId = P.Id
// GROUP BY
// V.UserId, P.Id
// ) VoteTypeCounts ON U.Id = VoteTypeCounts.UserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalComments,
// TotalUpVotes,
// TotalDownVotes
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC, TotalUpVotes DESC;
fn q14323(db: &'static So) -> String {
    let Vote { user, post, vote_type, .. } = &db.vote;
    let vtc = db.vote.group_by(user.and(post)).select(vote_type.select(&db.vote_type.name)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let keys: MatSet<(Id<User>, Id<Post>)> = db.vote.select(user.and(post)).collect();
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&keys).map(|(u, _)| u).inv().collect();
    let uf = g(db).select(posts_of(db).select(comments_of(db).opt()).opt().and((&by_user).select(&vtc).opt())).fold([0i64; 3], |a, (_, x)| match x {
        Some(x) => [a[0] + 1, a[1] + x[0], a[2] + x[1]],
        None => a,
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dc).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, p, c)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(c), nullable(a[1], a[0]), nullable(a[2], a[0])])))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// U.DisplayName AS Author,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// MAX(P.LastActivityDate) AS LastActivity
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// GROUP BY
// P.Id, P.PostTypeId, P.CreationDate, U.DisplayName
// ),
// TagStats AS (
// SELECT
// T.TagName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
// FROM
// Tags T
// LEFT JOIN
// Posts P ON P.Tags LIKE '%' || T.TagName || '%'
// GROUP BY
// T.TagName
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.CreationDate,
// PS.Author,
// PS.CommentCount,
// PS.VoteCount,
// PS.UpVotes,
// PS.DownVotes,
// PS.LastActivity,
// TS.TagName,
// TS.PostCount,
// TS.TotalViews
// FROM
// PostStats PS
// LEFT JOIN
// TagStats TS ON PS.PostId = TS.PostCount
// ORDER BY
// PS.VoteCount DESC, PS.CreationDate DESC
// LIMIT 100;
fn q14327(db: &'static So) -> String {
    let tm = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&tm).map(|(_, t)| t).inv().collect();
    let tf = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p).select((&db.post.view_count).opt())).opt())
        .fold([0i64; 2], |a, w| match w {
            Some(w) => [a[0] + 1, a[1] + w.unwrap_or(0)],
            None => a,
        });
    let by_count: HashIdx<i64, Str> = (&tf).map(|a| a[0]).inv().collect();
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and((&db.post.origid).select(&by_count).select(Same::<Str>::new().and(&tf)).opt()).drive(|p, (s, t)| v.push((p, s, t)));
    out(v, |&(p, s, _)| (Reverse(s.vx), newest(db, p)), 100, |&(p, s, t)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "owner"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["activity"]));
        match t {
            Some((n, a)) => f.extend([V::S(n), V::I(a[0]), V::I(a[1])]),
            None => f.extend(nulls(3)),
        }
        f
    })
}

// WITH UserVoteSummary AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(a.Id) AS AnswerCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Posts a ON p.Id = a.ParentId
// GROUP BY p.Id, p.Title, p.Score, p.ViewCount
// ),
// CombinedSummary AS (
// SELECT
// uvs.UserId,
// uvs.DisplayName,
// ps.PostId,
// ps.Title,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.AnswerCount,
// uvs.TotalVotes,
// uvs.UpVotes,
// uvs.DownVotes
// FROM UserVoteSummary uvs
// JOIN PostSummary ps ON uvs.UserId = ps.PostId
// )
// SELECT
// UserId,
// DisplayName,
// PostId,
// Title,
// Score,
// ViewCount,
// CommentCount,
// AnswerCount,
// TotalVotes,
// UpVotes,
// DownVotes
// FROM CombinedSummary
// ORDER BY Score DESC, ViewCount DESC
// LIMIT 100;
fn q14329(db: &'static So) -> String {
    let uid = uids(db);
    let uv = user_votes(db);
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "ca", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uv))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, _, _)| score_views(db, p), 100, |&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(s.cx), V::I(s.ax)]);
        f.extend(ints(&a));
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// U.Id AS UserId,
// U.DisplayName AS UserDisplayName,
// U.Reputation,
// U.CreationDate AS UserCreationDate,
// COUNT(V.Id) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// U.Id,
// U.DisplayName,
// U.Reputation,
// U.CreationDate
// ),
// BadgeStats AS (
// SELECT
// UserId,
// COUNT(Id) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate AS PostCreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.FavoriteCount,
// PS.UserId,
// PS.UserDisplayName,
// PS.Reputation,
// PS.UserCreationDate,
// COALESCE(BS.BadgeCount, 0) AS BadgeCount,
// PS.VoteCount
// FROM
// PostStats PS
// LEFT JOIN
// BadgeStats BS ON PS.UserId = BS.UserId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q14331(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and(votes_per_post(db)).and((&db.post.owner_user).select(&bu).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "uid", "owner", "rep", "ucreated"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(x)]);
        row(f)
    }))
}

// WITH PostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// UserReputation AS (
// SELECT
// Id AS UserId,
// Reputation
// FROM
// Users
// ),
// PostStats AS (
// SELECT
// u.UserId,
// u.Reputation,
// pc.TotalPosts,
// pc.TotalQuestions,
// pc.TotalAnswers
// FROM
// PostCounts pc
// JOIN
// UserReputation u ON pc.OwnerUserId = u.UserId
// )
// SELECT
// UserId,
// Reputation,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// (CAST(TotalQuestions AS FLOAT) / NULLIF(TotalPosts, 0)) * 100 AS QuestionPercentage,
// (CAST(TotalAnswers AS FLOAT) / NULLIF(TotalPosts, 0)) * 100 AS AnswerPercentage
// FROM
// PostStats
// ORDER BY
// Reputation DESC;
fn q14336(db: &'static So) -> String {
    let op = owner_posts(db);
    let pct = |x: i64, n: i64| V::F(((x as f32 / n as f32) * 100.0f32) as f64);
    let mut v = Vec::new();
    (&op).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a[0]), V::I(a[1]), V::I(a[2]), pct(a[1], a[0]), pct(a[2], a[0])])))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.Score,
// p.ViewCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.OwnerUserId, p.Score, p.ViewCount, p.PostTypeId
// ), UserReputation AS (
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
// ps.PostTypeId,
// ps.OwnerUserId,
// ur.Reputation,
// ur.BadgeCount,
// ps.CommentCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.Score,
// ps.ViewCount
// FROM
// PostStats ps
// JOIN
// UserReputation ur ON ps.OwnerUserId = ur.UserId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC;
fn q14337(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(&bu)).drive(|p, (s, b)| v.push((p, s, b)));
    rows(v.iter().map(|&(p, s, b)| {
        let mut f = post_fields(db, p, &["id", "type_id", "owner_id", "rep"]);
        f.extend([V::I(b), V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["score", "views"]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalPostOwners,
// AVG(ViewCount) AS AverageViews,
// AVG(Score) AS AverageScore
// FROM
// Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation,
// AVG(Views) AS AverageViewsPerUser
// FROM
// Users
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// AVG(BountyAmount) AS AverageBountyAmount
// FROM
// Votes
// ),
// CommentStats AS (
// SELECT
// COUNT(*) AS TotalComments,
// AVG(Score) AS AverageCommentScore
// FROM
// Comments
// )
// SELECT
// (SELECT TotalPosts FROM PostStats) AS TotalPosts,
// (SELECT TotalPostOwners FROM PostStats) AS TotalPostOwners,
// (SELECT AverageViews FROM PostStats) AS AveragePostViews,
// (SELECT AverageScore FROM PostStats) AS AveragePostScore,
// (SELECT TotalUsers FROM UserStats) AS TotalUsers,
// (SELECT AverageReputation FROM UserStats) AS AverageUserReputation,
// (SELECT AverageViewsPerUser FROM UserStats) AS AverageViewsPerUser,
// (SELECT TotalVotes FROM VoteStats) AS TotalVotes,
// (SELECT AverageBountyAmount FROM VoteStats) AS AverageBountyAmount,
// (SELECT TotalComments FROM CommentStats) AS TotalComments,
// (SELECT AverageCommentScore FROM CommentStats) AS AverageCommentScore;
fn q14344(db: &'static So) -> String {
    let p = post_totals(db);
    let du = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let u = db.user.select((&db.user.reputation).and(&db.user.views)).fold_flat([0i64; 3], |a, (r, w)| [a[0] + 1, a[1] + r, a[2] + w]);
    let x = db.vote.select((&db.vote.bounty_amount).opt()).fold_flat([0i64; 3], |a, b| [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]);
    let c = db.comment.select(&db.comment.score).fold_flat([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    row(vec![V::I(p[0]), V::I(du), avg(p[3], p[2]), avg(p[1], p[0]), V::I(u[0]), avg(u[1], u[0]), avg(u[2], u[0]), V::I(x[0]), avg(x[2], x[1]), V::I(c[0]), avg(c[1], c[0])])
}

// WITH PostStatistics AS (
// SELECT
// Posts.Id AS PostId,
// Posts.PostTypeId,
// COUNT(Votes.Id) AS VoteCount,
// COUNT(Comments.Id) AS CommentCount,
// MAX(Posts.CreationDate) AS LastActivityDate
// FROM
// Posts
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// LEFT JOIN
// Comments ON Posts.Id = Comments.PostId
// WHERE
// Posts.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// Posts.Id, Posts.PostTypeId
// ),
// UserStatistics AS (
// SELECT
// Users.Id AS UserId,
// AVG(Users.Reputation) AS AvgReputation,
// SUM(CASE WHEN Badges.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Badges.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Badges.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users
// LEFT JOIN
// Badges ON Users.Id = Badges.UserId
// GROUP BY
// Users.Id
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.VoteCount,
// PS.CommentCount,
// PS.LastActivityDate,
// US.UserId,
// US.AvgReputation,
// US.GoldBadges,
// US.SilverBadges,
// US.BronzeBadges
// FROM
// PostStatistics PS
// JOIN
// Users U ON PS.PostTypeId = U.Id
// JOIN
// UserStatistics US ON US.UserId = U.Id
// ORDER BY
// PS.LastActivityDate DESC;
fn q14346(db: &'static So) -> String {
    let uid = uids(db);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()).with((&db.post.post_type_id).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.post_type_id).select(&uid).select(Ident::<User>::new().and((&bc).opt())))
        .drive(|p, (s, (u, b))| v.push((p, s, u, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(p, s, u, b)| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([V::I(s.vx), V::I(s.cx)]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([user_col(db, u, "uid"), V::F(db.user.reputation.get(u).unwrap() as f64)]);
        f.extend(ints(&b[1..]));
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
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(V.BountyAmount) AS TotalBountyAmount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.AnswerCount,
// US.DisplayName AS UserDisplayName,
// US.BadgeCount,
// US.TotalBountyAmount
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostId = U.AccountId
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q14360(db: &'static So) -> String {
    let by_acct: HashIdx<i64, Id<User>> = (&db.user.account_id).inv().collect();
    let us = g(db).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 3], |a, (b, v)| {
        let bo = v.flatten();
        [a[0] + b.is_some() as i64, a[1] + bo.is_some() as i64, a[2] + bo.unwrap_or(0)]
    });
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&by_acct)), Ident::<Post>::new(), "cA", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&by_acct).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.ax), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1])]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalPostOwners,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalClosedPosts
// FROM
// Posts
// ),
// UserStatistics AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation,
// MIN(CreationDate) AS EarliestUserCreation,
// MAX(CreationDate) AS LatestUserCreation
// FROM
// Users
// ),
// VoteStatistics AS (
// SELECT
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Votes
// ),
// CommentStatistics AS (
// SELECT
// COUNT(*) AS TotalComments,
// AVG(Score) AS AverageCommentScore
// FROM
// Comments
// )
// SELECT
// ps.TotalPosts,
// ps.TotalPostOwners,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.TotalClosedPosts,
// us.TotalUsers,
// us.AverageReputation,
// us.EarliestUserCreation,
// us.LatestUserCreation,
// vs.TotalVotes,
// vs.TotalUpVotes,
// vs.TotalDownVotes,
// cs.TotalComments,
// cs.AverageCommentScore
// FROM
// PostStatistics ps,
// UserStatistics us,
// VoteStatistics vs,
// CommentStatistics cs;
fn q14372(db: &'static So) -> String {
    let Post { post_type_id, closed_date, .. } = &db.post;
    let p = db.post.select(post_type_id.and(closed_date.opt())).fold_flat([0i64; 4], |a, (t, c)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64]);
    let du = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let u = db.user.select((&db.user.reputation).and(&db.user.creation_date)).fold_flat([0, 0, i64::MAX, i64::MIN], |a: [i64; 4], (r, c)| [a[0] + 1, a[1] + r, a[2].min(c), a[3].max(c)]);
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let c = db.comment.select(&db.comment.score).fold_flat([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    row(vec![
        V::I(p[0]),
        V::I(du),
        V::I(p[1]),
        V::I(p[2]),
        V::I(p[3]),
        V::I(u[0]),
        avg(u[1], u[0]),
        if u[0] == 0 { V::Null } else { V::T(u[2]) },
        if u[0] == 0 { V::Null } else { V::T(u[3]) },
        V::I(x[0]),
        V::I(x[1]),
        V::I(x[2]),
        V::I(c[0]),
        avg(c[1], c[0]),
    ])
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AvgPostScore,
// SUM(ViewCount) AS TotalViews
// FROM
// Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgUserReputation
// FROM
// Users
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Votes
// )
// SELECT
// p.TotalPosts,
// p.AvgPostScore,
// p.TotalViews,
// u.TotalUsers,
// u.AvgUserReputation,
// v.TotalVotes,
// v.TotalUpVotes,
// v.TotalDownVotes
// FROM
// PostStats p,
// UserStats u,
// VoteStats v;
fn q14376(db: &'static So) -> String {
    let p = post_totals(db);
    let u = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    row(vec![V::I(p[0]), avg(p[1], p[0]), nullable(p[3], p[2]), V::I(u[0]), avg(u[1], u[0]), V::I(x[0]), V::I(x[1]), V::I(x[2])])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS AuthorDisplayName,
// u.Reputation AS AuthorReputation,
// COALESCE(votes.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(votes.DownVoteCount, 0) AS DownVoteCount,
// COALESCE(comments.CommentCount, 0) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Votes
// GROUP BY
// PostId) votes ON p.Id = votes.PostId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) comments ON p.Id = comments.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14386(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned_since(db, month_ago()).select(Ident::<Post>::new().and((&pv).opt()).and(comments_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, x), c)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "rep"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(c)]);
        f
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(a.Id) AS TotalAnswers,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// (SELECT
// PostId, COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT
// PostId, COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId) v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14388(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), answers_of(db).opt().and(comments_per_post(db)).and(votes_per_post(db)).and(&db.post.score), [0i64; 5], |a, (((an, c), x), s)| {
        [a[0] + 1, a[1] + an.is_some() as i64, a[2] + c, a[3] + x, a[4] + s]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0])])))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastHistoryDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ),
// UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 1 ELSE 0 END) AS RecentPostCount
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
// ps.Title,
// ps.CreationDate,
// ps.CommentCount,
// ps.VoteCount,
// ps.LastHistoryDate,
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.RecentPostCount
// FROM
// PostStatistics ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStatistics us ON u.Id = us.UserId
// ORDER BY
// ps.VoteCount DESC, ps.CommentCount DESC, ps.LastHistoryDate DESC
// LIMIT 100;
fn q14401(db: &'static So) -> String {
    let uid = uids(db);
    let y = year_ago();
    let us = g(db).select(badges_of(db).opt().and(posts_of(db).select(&db.post.creation_date).opt())).fold([0i64; 2], move |a, (b, c)| [a[0] + b.is_some() as i64, a[1] + c.map_or(false, |c| c >= y) as i64]);
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvh", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(_, s, _, _)| (Reverse(s.vx), Reverse(s.cx), (s.hx == 0, Reverse(s.hmax))), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), stat_field(&s, "hmax").unwrap(), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1])]);
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
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ), UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN p.OwnerUserId = u.Id THEN 1 ELSE 0 END) AS PostCount,
// SUM(CASE WHEN v.UserId = u.Id THEN 1 ELSE 0 END) AS UserVoteCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.AnswerCount,
// ps.VoteCount,
// us.DisplayName AS PostOwner,
// us.PostCount AS OwnerPostCount,
// us.UserVoteCount AS OwnerVoteCount
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q14406(db: &'static So) -> String {
    let uid = uids(db);
    let us = g(db).select(posts_of(db).opt().and(votes_by(db).opt())).fold([0i64; 2], |a, (p, v)| [a[0] + p.is_some() as i64, a[1] + v.is_some() as i64]);
    let an = per_post_distinct(db, answers_of(db));
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&an).opt()).and(votes_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((p, c), a), x), (u, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(a.unwrap_or(0)), V::I(x), user_col(db, u, "name"), V::I(n[0]), V::I(n[1])]);
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
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// U.Reputation AS OwnerReputation,
// U.DisplayName AS OwnerDisplayName
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// ),
// VoteStats AS (
// SELECT
// V.PostId,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes V
// GROUP BY
// V.PostId
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
// PS.PostId,
// PS.PostTypeId,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.FavoriteCount,
// PS.OwnerReputation,
// PS.OwnerDisplayName,
// COALESCE(VS.VoteCount, 0) AS VoteCount,
// COALESCE(VS.UpVotes, 0) AS UpVotes,
// COALESCE(VS.DownVotes, 0) AS DownVotes,
// COALESCE(BS.BadgeCount, 0) AS OwnerBadgeCount
// FROM
// PostStats PS
// LEFT JOIN
// VoteStats VS ON PS.PostId = VS.PostId
// LEFT JOIN
// BadgeStats BS ON PS.OwnerReputation = BS.UserId
// ORDER BY
// PS.CreationDate DESC;
fn q14407(db: &'static So) -> String {
    let bf = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(&db.user.reputation).select(&bf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), b)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views", "answers", "comments", "favorites", "rep", "owner"]);
        f.extend(ints(&x.unwrap_or([0; 3])));
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(P.Id) AS PostCount,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id
// ),
// UserPerformance AS (
// SELECT
// UserId,
// PostCount,
// CommentCount,
// VoteCount,
// (PostCount + CommentCount + VoteCount) AS TotalEngagement
// FROM
// PostStats
// )
// SELECT
// UserId,
// PostCount,
// CommentCount,
// VoteCount,
// TotalEngagement
// FROM
// UserPerformance
// ORDER BY
// TotalEngagement DESC;
fn q14416(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let mut v = Vec::new();
    (&us).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), V::I(a.n), V::I(a.cx), V::I(a.vx), V::I(a.n + a.cx + a.vx)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswers,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// AVG(u.Reputation) AS AvgUserReputation,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14418(db: &'static So) -> String {
    let Post { accepted_answer_id, view_count, score, owner_user, .. } = &db.post;
    let f = by_key(
        db.post.iq(),
        name(db),
        comments_of(db).opt().and(votes_of(db).opt()).and(accepted_answer_id.opt()).and(view_count.opt()).and(score).and(owner_user.select(&db.user.reputation).opt()),
        [0i64; 6],
        |a, (((((_, _), ac), w), s), r)| [a[0] + 1, a[1] + ac.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)],
    );
    let dc = db.post.group_by(name(db)).select(comments_of(db)).count_distinct();
    let dv = db.post.group_by(name(db)).select(votes_of(db)).count_distinct();
    let mut v = Vec::new();
    (&f).and((&dc).opt()).and((&dv).opt()).drive(|k, ((a, c), x)| v.push((k, a, c.unwrap_or(0), x.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, c, x)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4]), V::I(c), V::I(x)])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.Views) AS TotalViews,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// AVG(u.Reputation) AS AvgReputation
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.ViewCount) AS TotalPostViews,
// AVG(p.Score) AS AvgPostScore,
// COUNT(c.Id) AS TotalComments
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY p.OwnerUserId
// ),
// FinalStats AS (
// SELECT
// us.UserId,
// us.BadgeCount,
// us.TotalViews,
// us.TotalUpVotes,
// us.TotalDownVotes,
// us.AvgReputation,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalPostViews, 0) AS PostViewCount,
// COALESCE(ps.AvgPostScore, 0) AS AvgPostScore,
// COALESCE(ps.TotalComments, 0) AS TotalComments
// FROM UserStats us
// LEFT JOIN PostStats ps ON us.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// BadgeCount,
// TotalViews,
// TotalUpVotes,
// TotalDownVotes,
// AvgReputation,
// PostCount,
// PostViewCount,
// AvgPostScore,
// TotalComments
// FROM FinalStats
// ORDER BY UserId;
fn q14432(db: &'static So) -> String {
    let User { views, up_votes, down_votes, reputation, .. } = &db.user;
    let us = g(db).select(views.and(up_votes).and(down_votes).and(reputation).and(badges_of(db).opt())).fold([0i64; 6], |a, ((((w, u), d), r), b)| {
        [a[0] + 1, a[1] + b.is_some() as i64, a[2] + w, a[3] + u, a[4] + d, a[5] + r]
    });
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&db.post.score).and(comments_of(db).opt())).fold([0i64; 4], |a, ((w, s), c)| {
        [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + c.is_some() as i64]
    });
    let mut v = Vec::new();
    (&us).and((&pf).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[5], a[0])];
        match p {
            Some(p) => f.extend([V::I(p[0]), V::I(p[1]), avg(p[2], p[0]), V::I(p[3])]),
            None => f.extend([V::I(0), V::I(0), V::F(0.0), V::I(0)]),
        }
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// QuestionCount,
// AnswerCount,
// LastPostDate
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC
// LIMIT 10
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// T.TotalPosts,
// T.QuestionCount,
// T.AnswerCount,
// T.LastPostDate
// FROM
// Users U
// JOIN
// TopUsers T ON U.Id = T.UserId;
fn q14443(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(&db.post.creation_date)).opt()).fold([0, 0, 0, i64::MIN], |a: [i64; 4], p| match p {
        Some((t, c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(c)],
        None => a,
    });
    let top: MatSet<Id<User>> = whole(&uf).select(Same::new().and(&uf)).window(row_number, |(_, a): (Id<User>, [i64; 4])| a[0], desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&uf)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[0] == 0 { V::Null } else { V::T(a[3]) }])))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostID,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COALESCE(SUM(CASE WHEN b.Date IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount,
// COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount
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
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// ps.PostID,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.BadgeCount,
// ps.RelatedPostCount
// FROM
// PostStatistics ps
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// FETCH FIRST 100 ROWS ONLY;
fn q14456(db: &'static So) -> String {
    let rel = per_post_distinct(db, links_of(db).select(&db.post_link.related_post_id));
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let pf = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user_id).select(&bidx).opt()).and(links_of(db).opt()))
        .fold([0i64; 4], |a, (((c, t), b), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + b.is_some() as i64]);
    let mut v = Vec::new();
    (&pf).and(votes_per_post(db)).and((&rel).opt()).drive(|p, ((s, x), r)| v.push((p, s, x, r.unwrap_or(0))));
    out(v, |&(p, _, _, _)| score_views(db, p), 100, |&(p, s, x, r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s[0]), V::I(x), V::I(s[1]), V::I(s[2]), V::I(s[3]), V::I(r)]);
        f
    })
}

// SELECT
// u.DisplayName AS UserName,
// p.Title AS PostTitle,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount
// FROM Posts
// WHERE PostTypeId = 2
// GROUP BY ParentId) a ON p.Id = a.ParentId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId) b ON u.Id = b.UserId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14465(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.post_type_id).eq(1))
        .select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and((&db.post.owner_user).select(&bu)))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, a), c), b)| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "views", "score"]);
        f.extend([V::I(a), V::I(c), V::I(b)]);
        f
    })
}

pub static ENTRIES: &[harness::Entry] = &[
    ("14172", q14172),
    ("14173", q14173),
    ("14182", q14182),
    ("14184", q14184),
    ("14187", q14187),
    ("14191", q14191),
    ("14192", q14192),
    ("14193", q14193),
    ("14200", q14200),
    ("14208", q14208),
    ("14209", q14209),
    ("14211", q14211),
    ("14242", q14242),
    ("14250", q14250),
    ("14251", q14251),
    ("14256", q14256),
    ("14265", q14265),
    ("14266", q14266),
    ("14277", q14277),
    ("14279", q14279),
    ("14298", q14298),
    ("14299", q14299),
    ("14300", q14300),
    ("14303", q14303),
    ("14304", q14304),
    ("14305", q14305),
    ("14307", q14307),
    ("14309", q14309),
    ("14313", q14313),
    ("14316", q14316),
    ("14318", q14318),
    ("14321", q14321),
    ("14323", q14323),
    ("14327", q14327),
    ("14329", q14329),
    ("14331", q14331),
    ("14336", q14336),
    ("14337", q14337),
    ("14344", q14344),
    ("14346", q14346),
    ("14360", q14360),
    ("14372", q14372),
    ("14376", q14376),
    ("14386", q14386),
    ("14388", q14388),
    ("14401", q14401),
    ("14406", q14406),
    ("14407", q14407),
    ("14416", q14416),
    ("14418", q14418),
    ("14432", q14432),
    ("14443", q14443),
    ("14456", q14456),
    ("14465", q14465),
];
