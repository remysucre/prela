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

// --- batch 117 --------------------------------------------------------------

// WITH PostVoteCounts AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS Upvotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS Downvotes
// FROM Votes
// GROUP BY PostId
// ),
// UserBadges AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// )
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.Score,
// P.ViewCount,
// COALESCE(U.DisplayName, 'Community User') AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// COALESCE(PVC.Upvotes, 0) AS UpvoteCount,
// COALESCE(PVC.Downvotes, 0) AS DownvoteCount,
// COALESCE(UB.BadgeCount, 0) AS OwnerBadgeCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// PostVoteCounts PVC ON P.Id = PVC.PostId
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// WHERE
// P.CreationDate >= '2023-01-01'
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q11395(db: &'static So) -> String {
    let vd = post_votes(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and((&vd).opt()).and((&db.post.owner_user).select(&bu).opt())).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, x), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(named_owner(db, p, "Community User"));
        f.extend(post_fields(db, p, &["rep"]));
        let x = x.unwrap_or([0; 3]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(b.unwrap_or(0))]);
        f
    })
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// CONCAT(u.DisplayName, ' (Reputation: ', u.Reputation, ')') AS OwnerInfo
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, u.Reputation
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// Score,
// CommentCount,
// VoteCount,
// OwnerInfo,
// ROUND((Score / NULLIF(ViewCount, 0)) * 100, 2) AS ScorePer100Views
// FROM
// PostSummary
// ORDER BY
// Score DESC, ViewCount DESC;
fn q11404(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and((&c).opt()).and((&x).opt())).drive(|_, y| v.push(y));
    rows(v.iter().map(|&((p, c), x)| {
        let u = db.post.owner_user.get(p).unwrap();
        let s = db.post.score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0))]);
        f.push(V::Owned(format!("{} (Reputation: {})", db.user.display_name.get(u).unwrap(), db.user.reputation.get(u).unwrap())));
        f.push(match db.post.view_count.get(p) {
            Some(w) if w != 0 => V::F(round2(s as f64 / w as f64 * 100.0)),
            _ => V::Null,
        });
        row(f)
    }))
}

// WITH PostPerformance AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostTypeName,
// PostCount,
// AverageScore,
// TotalVotes
// FROM
// PostPerformance
// ORDER BY
// PostCount DESC;
fn q11406(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(votes_of(db).opt()), [0i64; 3], |a, (s, x)| [a[0] + 1, a[1] + s, a[2] + x.is_some() as i64]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// COUNT(p.Id) AS PostCount,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViewCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostTypeCounts AS (
// SELECT
// OwnerUserId,
// COUNT(CASE WHEN PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN PostTypeId = 2 THEN 1 END) AS AnswerCount,
// COUNT(CASE WHEN PostTypeId = 4 THEN 1 END) AS TagWikiCount
// FROM
// Posts
// GROUP BY
// OwnerUserId
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// us.BadgeCount,
// us.PostCount,
// us.TotalViewCount,
// us.TotalScore,
// ptc.QuestionCount,
// ptc.AnswerCount,
// ptc.TagWikiCount
// FROM
// Users u
// JOIN
// UserStats us ON u.Id = us.UserId
// LEFT JOIN
// PostTypeCounts ptc ON u.Id = ptc.OwnerUserId
// ORDER BY
// us.TotalViewCount DESC,
// us.TotalScore DESC;
fn q11422(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let ptc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 4) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&us).and((&ptc).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, a), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a.bx), V::I(a.n), ustat_field(&a, "views_sum0"), ustat_field(&a, "score_sum0")];
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// U.DisplayName AS OwnerDisplayName,
// COUNT(C.ID) AS CommentCount,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName
// ),
// PostHistoryAnalysis AS (
// SELECT
// PH.PostId,
// COUNT(PH.Id) AS RevisionCount,
// MAX(PH.CreationDate) AS LastEdited
// FROM
// PostHistory PH
// GROUP BY
// PH.PostId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.OwnerDisplayName,
// PS.CommentCount,
// PS.TotalBounty,
// COALESCE(PHA.RevisionCount, 0) AS RevisionCount,
// PHA.LastEdited
// FROM
// PostStats PS
// LEFT JOIN
// PostHistoryAnalysis PHA ON PS.PostId = PHA.PostId
// ORDER BY
// PS.ViewCount DESC, PS.Score DESC
// LIMIT 100;
fn q11423(db: &'static So) -> String {
    let hc = history_per_post(db);
    let hm = history_max_date(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[8, 9]).and(&hc).and(&hm).drive(|p, ((s, h), m)| v.push((p, s, h, m)));
    out(v, |&(p, _, _, _)| views_score(db, p), 100, |&(p, s, h, m)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(s.cx), V::I(s.bounty_sum), V::I(h), if h == 0 { V::Null } else { V::T(m) }]);
        f
    })
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// u.DisplayName AS OwnerDisplayName,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// Tags t ON t.ExcerptPostId = p.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// ),
// VoteCounts AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS Upvotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS Downvotes
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
// pd.AnswerCount,
// pd.CommentCount,
// pd.FavoriteCount,
// pd.OwnerDisplayName,
// COALESCE(vc.Upvotes, 0) AS Upvotes,
// COALESCE(vc.Downvotes, 0) AS Downvotes,
// pd.TagName
// FROM
// PostDetails pd
// LEFT JOIN
// VoteCounts vc ON pd.PostId = vc.PostId
// ORDER BY
// pd.Score DESC,
// pd.ViewCount DESC
// LIMIT 100;
fn q11425(db: &'static So) -> String {
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let vd = post_votes(db);
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(&ex).and((&vd).opt())).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| score_views(db, p), 100, |&((p, t), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "owner"]);
        let x = x.unwrap_or([0; 3]);
        f.extend([V::I(x[1]), V::I(x[2]), V::S(db.tag.tag_name.get(t).unwrap())]);
        f
    })
}

// WITH UserPostActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
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
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalViews,
// UpVotes,
// DownVotes
// FROM
// UserPostActivity
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q11430(db: &'static So) -> String {
    out(users_with_counts(db, "v", false), |r| Reverse(r.agg.prows), 10, |r| user_fields(r, "v", &["uid", "name", "#rows", "#q", "#a", "views_sum0", "#up", "#down"]))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostInteractionStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
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
// P.Id, P.Title
// )
// SELECT
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.Questions,
// UPS.Answers,
// UPS.UpvotedPosts,
// PIS.PostId,
// PIS.Title,
// PIS.CommentCount,
// PIS.UpVotes,
// PIS.DownVotes
// FROM
// UserPostStats UPS
// JOIN
// PostInteractionStats PIS ON UPS.UserId = PIS.PostId
// WHERE
// UPS.TotalPosts > 0
// ORDER BY
// UPS.TotalPosts DESC, PIS.UpVotes DESC
// FETCH FIRST 50 ROWS ONLY;
fn q11449(db: &'static So) -> String {
    let uid = uids(db);
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64],
        None => a,
    });
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&uf).filt(|a: [i64; 4]| a[0] > 0))))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(_, s, _, a)| (Reverse(a[0]), Reverse(s.up)), 50, |&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f
    })
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// TopPosts AS (
// SELECT
// P.Id AS PostId,
// P.OwnerUserId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.CreationDate
// FROM Posts P
// WHERE P.PostTypeId = 1
// ORDER BY P.Score DESC
// LIMIT 10
// ),
// PostComments AS (
// SELECT
// C.PostId,
// COUNT(C.Id) AS CommentCount
// FROM Comments C
// GROUP BY C.PostId
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// U.PostCount,
// U.BadgeCount,
// TP.PostId,
// TP.Title,
// TP.Score,
// TP.ViewCount,
// COALESCE(PC.CommentCount, 0) AS CommentCount
// FROM UserReputation U
// JOIN TopPosts TP ON U.UserId = TP.OwnerUserId
// LEFT JOIN PostComments PC ON TP.PostId = PC.PostId
// ORDER BY U.Reputation DESC, TP.Score DESC;
fn q11465(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let top: MatSet<Id<Post>> = whole(questions_only(db))
        .select(Ident::<Post>::new().and(&db.post.score))
        .window(row_number, |(_, s)| s, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let mut v = Vec::new();
    (&top).select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.owner_user).select(Ident::<User>::new().and((&dp).opt()).and(&bu)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), ((u, d), b))| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d.unwrap_or(0)), V::I(b)];
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.LastActivityDate,
// p.AnswerCount,
// p.CommentCount,
// p.ViewCount,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.AnswerCount, p.CommentCount, p.ViewCount, u.Reputation
// ),
// PostHistorySummary AS (
// SELECT
// ph.PostId,
// ph.PostHistoryTypeId,
// COUNT(*) AS ChangeCount
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId, ph.PostHistoryTypeId
// )
// SELECT
// pa.PostId,
// pa.Title,
// pa.CreationDate,
// pa.LastActivityDate,
// pa.AnswerCount,
// pa.CommentCount,
// pa.ViewCount,
// pa.OwnerReputation,
// phs.ChangeCount AS TotalPostChanges,
// phs2.ChangeCount AS TotalCloseVotes
// FROM
// PostActivity pa
// LEFT JOIN
// PostHistorySummary phs ON pa.PostId = phs.PostId AND phs.PostHistoryTypeId IN (10, 11)
// LEFT JOIN
// PostHistorySummary phs2 ON pa.PostId = phs2.PostId AND phs2.PostHistoryTypeId = 12
// ORDER BY
// pa.ViewCount DESC;
fn q11466(db: &'static So) -> String {
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let key = || post.and(post_history_type_id);
    let cnt = db.post_history.group_by(key()).fold(0i64, |a, _| a + 1);
    let keys: MatSet<(Id<Post>, i64)> = db.post_history.select(key()).collect();
    let k1011: HashIdx<Id<Post>, (Id<Post>, i64)> = (&keys).filt(|(_, t)| t == 10 || t == 11).map(|(p, _)| p).inv().collect();
    let k12: HashIdx<Id<Post>, (Id<Post>, i64)> = (&keys).filt(|(_, t)| t == 12).map(|(p, _)| p).inv().collect();
    let mut v = Vec::new();
    since(db, date(2022, 1, 1)).select(Ident::<Post>::new().and((&k1011).select(&cnt).opt()).and((&k12).select(&cnt).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, a), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity", "answers", "comments", "views", "rep"]);
        f.extend([oint(a), oint(b)]);
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id
// ),
// BadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM Badges b
// GROUP BY b.UserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(uc.PostCount, 0) AS PostCount,
// COALESCE(uc.QuestionCount, 0) AS QuestionCount,
// COALESCE(uc.AnswerCount, 0) AS AnswerCount,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount
// FROM Users u
// LEFT JOIN UserPostCounts uc ON u.Id = uc.UserId
// LEFT JOIN BadgeCounts bc ON u.Id = bc.UserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// QuestionCount,
// AnswerCount,
// BadgeCount
// FROM UserStats
// ORDER BY Reputation DESC, PostCount DESC
// LIMIT 100;
fn q11468(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    upqa(db).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a[0])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a[..3]));
        f.push(V::I(b));
        f
    })
}

// WITH PostSummary AS (
// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore
// FROM
// Posts p
// GROUP BY
// p.PostTypeId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// AVG(u.Reputation) AS AvgReputation
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// )
// SELECT
// ps.PostTypeId,
// ps.PostCount,
// ps.AvgScore,
// ur.AvgReputation
// FROM
// PostSummary ps
// JOIN
// UserReputation ur ON ur.UserId IN (SELECT DISTINCT OwnerUserId FROM Posts WHERE PostTypeId = ps.PostTypeId)
// ORDER BY
// ps.PostCount DESC;
fn q11494(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let pf = by_key(db.post.iq(), post_type_id, score, (0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let pairs: MatSet<(i64, Id<User>)> = owned(db).select(post_type_id.and(owner_user)).collect();
    let mut v = Vec::new();
    (&pairs).select((&pairs).map(|(t, _)| t).select(&pf)).drive(|(t, u), a| v.push((t, u, a)));
    rows(v.iter().map(|&(t, u, (n, s))| row(vec![V::I(t), V::I(n), avg(s, n), V::F(db.user.reputation.get(u).unwrap() as f64)])))
}

// WITH Benchmarking AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.Reputation AS OwnerReputation,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation
// )
// SELECT
// *,
// (EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - CreationDate)) / 3600) AS HoursSinceCreation,
// (SELECT COUNT(*) FROM Posts WHERE AcceptedAnswerId = PostId) AS AcceptedAnswers
// FROM
// Benchmarking
// ORDER BY
// ViewCount DESC, Score DESC
// LIMIT 100;
fn q11495(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let hm = history_max_date(db);
    let acc = (&db.post.accepted_answer).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and(&hm).and(&acc)).drive(|_, y| v.push(y));
    out(v, |&((((p, _), _), _), _)| views_score(db, p), 100, |&((((p, c), x), m), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0)), if m == i64::MIN { V::Null } else { V::T(m) }]);
        f.extend([V::F(hours_to(t0, db.post.creation_date.get(p).unwrap()) / 3600.0), V::I(a)]);
        f
    })
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.Reputation AS OwnerReputation,
// COUNT(DISTINCT c.Id) AS CommentCountTotal,
// COUNT(DISTINCT v.Id) AS VoteCountTotal
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.Reputation
// ),
// PostHistories AS (
// SELECT
// ph.PostId,
// COUNT(*) AS EditCount,
// MAX(ph.CreationDate) AS LastEdited
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
// pm.OwnerReputation,
// ph.EditCount,
// ph.LastEdited
// FROM
// PostMetrics pm
// LEFT JOIN
// PostHistories ph ON pm.PostId = ph.PostId
// ORDER BY
// pm.ViewCount DESC
// LIMIT 100;
fn q11504(db: &'static So) -> String {
    let hc = history_per_post(db);
    let hm = history_max_date(db);
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and(&hc).and(&hm)).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| views_desc(db, p), 100, |&((p, h), m)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "rep"]);
        f.extend(if h == 0 { [V::Null, V::Null] } else { [V::I(h), V::T(m)] });
        f
    })
}

// SELECT
// u.DisplayName AS UserDisplayName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// p.Score AS PostScore,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// STRING_AGG(t.TagName, ',') AS Tags
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostLinks pl ON pl.PostId = p.Id
// LEFT JOIN
// Tags t ON t.Id = pl.RelatedPostId
// WHERE
// p.CreationDate >= '2020-01-01'
// GROUP BY
// u.DisplayName, p.Title, p.CreationDate, p.Score
// ORDER BY
// p.Score DESC, p.CreationDate DESC;
fn q11508(db: &'static So) -> String {
    let Post { owner_user, title, creation_date, score, .. } = &db.post;
    let tid: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let key = || owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(score);
    let base = || owned_since(db, date(2020, 1, 1));
    let links = || links_of(db).select((&db.post_link.related_post_id).select(&tid).opt()).opt();
    let f = base()
        .group_by(key())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(links()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tags = base()
        .group_by(key())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(links().map(|l: Option<Option<Id<Tag>>>| l.flatten())))
        .buf_fold(|ts| {
            let names: Vec<Str> = ts.iter().filter_map(|&(_, t)| t).map(|t| db.tag.tag_name.get(t).unwrap()).collect();
            if names.is_empty() { None } else { Some(leak_join(names, ",")) }
        });
    let mut v = Vec::new();
    (&f).and(&tags).drive(|k, (a, t)| v.push((k, a, t)));
    rows(v.iter().map(|&((((n, t), c), s), a, tg)| row(vec![V::S(n), ostr(t), V::T(c), V::I(s), V::I(a[0]), V::I(a[1]), V::I(a[2]), ostr(tg)])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// AVG(p.Score) AS AverageScore
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(ph.Id) AS HistoryCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM Posts p
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// GROUP BY p.Id
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVoteCount,
// us.DownVoteCount,
// us.AverageScore,
// phs.HistoryCount,
// phs.LastEditDate
// FROM UserStats us
// JOIN PostHistoryStats phs ON us.PostCount > 0 AND us.UserId = phs.PostId
// ORDER BY us.PostCount DESC;
fn q11520(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let hc = history_per_post(db);
    let hm = history_max_date(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(&hc).and(&hm).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).filt(|d: i64| d > 0)))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((_, h), m), ((u, a), d))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down), ustat_field(&a, "score_avg"), V::I(h)];
        f.push(if h == 0 { V::Null } else { V::T(m) });
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.Tags,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, p.Tags, u.DisplayName
// ),
// TagMetrics AS (
// SELECT
// t.Id AS TagId,
// t.TagName,
// SUM(pm.ViewCount) AS TotalViews,
// SUM(pm.Score) AS TotalScore,
// SUM(pm.AnswerCount) AS TotalAnswers,
// SUM(pm.CommentCount) AS TotalComments,
// SUM(pm.FavoriteCount) AS TotalFavorites
// FROM
// Tags t
// JOIN
// Posts p ON p.Tags LIKE '%' || t.TagName || '%'
// JOIN
// PostMetrics pm ON p.Id = pm.PostId
// GROUP BY
// t.Id, t.TagName
// )
// SELECT
// tm.TagId,
// tm.TagName,
// tm.TotalViews,
// tm.TotalScore,
// tm.TotalAnswers,
// tm.TotalComments,
// tm.TotalFavorites
// FROM
// TagMetrics tm
// ORDER BY
// tm.TotalViews DESC;
fn q11524(db: &'static So) -> String {
    let Post { view_count, score, answer_count, comment_count, favorite_count, .. } = &db.post;
    let m = tag_mentions(db);
    let f = (&m)
        .group_by((&m).map(|(_, t)| t))
        .select((&m).map(|(p, _)| p).select(view_count.opt().and(score).and(answer_count.opt()).and(comment_count).and(favorite_count.opt())))
        .fold([0i64; 8], |a, ((((w, s), an), cc), fc)| {
            [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + s, a[3] + an.is_some() as i64, a[4] + an.unwrap_or(0), a[5] + cc, a[6] + fc.is_some() as i64, a[7] + fc.unwrap_or(0)]
        });
    let mut v = Vec::new();
    (&f).drive(|t, a| v.push((t, a)));
    rows(v.iter().map(|&(t, a)| {
        row(vec![
            V::I(db.tag.origid.get(t).unwrap()),
            V::S(db.tag.tag_name.get(t).unwrap()),
            nullable(a[1], a[0]),
            V::I(a[2]),
            nullable(a[4], a[3]),
            V::I(a[5]),
            nullable(a[7], a[6]),
        ])
    }))
}

// WITH PostAggregates AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// MAX(PH.CreationDate) AS LastEditDate
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// WHERE
// P.CreationDate >= '2020-01-01'
// GROUP BY
// P.Id, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.FavoriteCount
// ),
// UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.Reputation
// FROM
// Users U
// )
// SELECT
// P.PostId,
// U.UserId,
// U.Reputation,
// P.PostTypeId,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// P.UpVotes,
// P.DownVotes,
// P.LastEditDate
// FROM
// PostAggregates P
// JOIN
// UserReputation U ON P.PostTypeId = U.UserId
// ORDER BY
// P.ViewCount DESC, P.Score DESC;
fn q11530(db: &'static So) -> String {
    let uid = uids(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2020, 1, 1)).with((&db.post.post_type_id).select(&uid)), Ident::<Post>::new(), "vh", &[])
        .and((&db.post.post_type_id).select(&uid))
        .drive(|p, (s, u)| v.push((p, s, u)));
    rows(v.iter().map(|&(p, s, u)| {
        let mut f = vec![post_fields(db, p, &["id"]).pop().unwrap(), user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(post_fields(db, p, &["type_id", "created", "score", "views", "answers", "comments", "favorites"]));
        f.extend([V::I(s.up), V::I(s.down), stat_field(&s, "hmax").unwrap()]);
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
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT ph.Id) AS EditCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotes,
// us.DownVotes,
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.Score,
// pd.ViewCount,
// pd.CommentCount,
// pd.EditCount
// FROM
// UserStats us
// JOIN
// PostDetails pd ON us.UserId = pd.OwnerUserId
// ORDER BY
// us.UserId DESC, pd.ViewCount DESC
// LIMIT 100;
fn q11532(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pd = owned(db).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).opt())).fold(0i64, |a, (c, _)| a + c.is_some() as i64);
    let dh = history_per_post(db);
    let mut v = Vec::new();
    (&pd).and(&dh).and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp))).drive(|p, ((c, h), ((u, a), d))| v.push((p, c, h, u, a, d)));
    out(v, |&(p, _, _, u, _, _)| (Reverse(db.user.origid.get(u).unwrap()), views_desc(db, p)), 100, |&(p, c, h, u, a, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(h)]);
        f
    })
}

// WITH UserPostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(Id) AS PostCount,
// SUM(ViewCount) AS TotalViews,
// SUM(Score) AS TotalScore
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// UserBadges AS (
// SELECT
// UserId,
// COUNT(Id) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ),
// UserVoteCounts AS (
// SELECT
// UserId,
// COUNT(Id) AS VoteCount
// FROM
// Votes
// GROUP BY
// UserId
// ),
// UserComments AS (
// SELECT
// UserId,
// COUNT(Id) AS CommentCount
// FROM
// Comments
// GROUP BY
// UserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UPC.PostCount, 0) AS PostCount,
// COALESCE(UPC.TotalViews, 0) AS TotalViews,
// COALESCE(UPC.TotalScore, 0) AS TotalScore,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UVC.VoteCount, 0) AS VoteCount,
// COALESCE(UC.CommentCount, 0) AS CommentCount
// FROM
// Users U
// LEFT JOIN
// UserPostCounts UPC ON U.Id = UPC.OwnerUserId
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// UserVoteCounts UVC ON U.Id = UVC.UserId
// LEFT JOIN
// UserComments UC ON U.Id = UC.UserId
// ORDER BY
// U.Reputation DESC;
fn q11539(db: &'static So) -> String {
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&db.post.score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let bu = badges_per_user(db);
    let vu = votes_per_user(db);
    let cu = comments_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt()).and(&bu).and(&vu).and(&cu)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((u, p), b), x), c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p.unwrap_or([0; 3])));
        f.extend([V::I(b), V::I(x), V::I(c)]);
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
// P.AnswerCount,
// COUNT(C.Id) AS CommentCount,
// U.DisplayName AS OwnerDisplayName
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, U.DisplayName
// ),
// VoteStats AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(CASE WHEN VoteTypeId = 1 THEN 1 END) AS AcceptedVotes
// FROM
// Votes
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
// VS.UpVotes,
// VS.DownVotes,
// VS.AcceptedVotes,
// PS.OwnerDisplayName
// FROM
// PostStats PS
// LEFT JOIN
// VoteStats VS ON PS.PostId = VS.PostId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC
// LIMIT 100;
fn q11551(db: &'static So) -> String {
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 1) as i64]);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and(comments_per_post(db)).and((&vs).opt())).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| score_views(db, p), 100, |&((p, c), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(c));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f.extend(post_fields(db, p, &["owner"]));
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
// AVG(p.Score) AS AveragePostScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.CommentCount, 0)) AS TotalComments
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
// COUNT(b.Id) AS BadgeCount
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
// ups.AveragePostScore,
// ups.TotalViews,
// ups.TotalComments,
// COALESCE(ub.BadgeCount, 0) AS TotalBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadges ub ON ups.UserId = ub.UserId
// ORDER BY
// ups.PostCount DESC;
fn q11556(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, comment_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comment_count)).opt()).fold([0i64; 6], |a, p| match p {
        Some((((t, s), w), cc)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5] + cc],
        None => a,
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([avg(a[3], a[0]), V::I(a[4]), V::I(a[5]), V::I(b)]);
        row(f)
    }))
}

// SELECT
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS OwnerDisplayName,
// v.VoteCount AS TotalVotes,
// c.CommentCount AS TotalComments,
// COALESCE(ph.EditedCount, 0) AS TotalEdits,
// COALESCE(b.BadgeCount, 0) AS TotalBadges
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId) v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS EditedCount
// FROM PostHistory
// WHERE PostHistoryTypeId IN (4, 5, 6)
// GROUP BY PostId) ph ON p.Id = ph.PostId
// LEFT JOIN
// (SELECT U.Id, COUNT(b.Id) AS BadgeCount
// FROM Badges b
// JOIN Users U ON b.UserId = U.Id
// GROUP BY U.Id) b ON u.Id = b.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11561(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let ed = history_of_types(db, &[4, 5, 6]);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.post_type_id).eq(1))
        .select(Ident::<Post>::new().and((&vf).opt()).and((&cf).opt()).and(&ed).and((&db.post.owner_user).select(&bu)))
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| newest(db, p), 100, |&((((p, x), c), e), b)| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend([oint(x), oint(c), V::I(e), V::I(b)]);
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q11564(db: &'static So) -> String {
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "cvb", &[]).and((&b).opt()).drive(|p, (s, b)| v.push((p, s, b.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), V::I(b)]);
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
// COALESCE(COUNT(A.Id), 0) AS AnswerCount,
// COALESCE(SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(COUNT(C.Id), 0) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// Posts A ON P.Id = A.ParentId AND P.PostTypeId = 1
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount
// ),
// AverageMetrics AS (
// SELECT
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// AVG(AnswerCount) AS AvgAnswerCount,
// AVG(CommentCount) AS AvgCommentCount,
// SUM(UpVotes) AS TotalUpVotes,
// SUM(DownVotes) AS TotalDownVotes
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
// PM.UpVotes,
// PM.DownVotes,
// AM.AvgViewCount,
// AM.AvgScore,
// AM.AvgAnswerCount,
// AM.AvgCommentCount,
// AM.TotalUpVotes,
// AM.TotalDownVotes
// FROM
// PostMetrics PM,
// AverageMetrics AM
// ORDER BY
// PM.ViewCount DESC;
fn q11566(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let base = || since(db, date(2023, 1, 1));
    let kids = Ident::<Post>::new().with((&db.post.post_type_id).eq(1)).select(children_of(db));
    let pf = base()
        .group_by(Ident::<Post>::new())
        .select(kids.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()))
        .fold([0i64; 4], |a, ((k, t), c)| [a[0] + k.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + c.is_some() as i64]);
    let t = base().select(view_count.opt().and(score).and(&pf)).fold_flat([0i64; 8], |t, ((w, s), a)| {
        [t[0] + 1, t[1] + w.is_some() as i64, t[2] + w.unwrap_or(0), t[3] + s, t[4] + a[0], t[5] + a[3], t[6] + a[1], t[7] + a[2]]
    });
    let mut v = Vec::new();
    (&pf).drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(t[2], t[1]), avg(t[3], t[0]), avg(t[4], t[0]), avg(t[5], t[0]), nullable(t[6], t[0]), nullable(t[7], t[0])]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis
// FROM
// Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation,
// MAX(CreationDate) AS MostRecentUserCreation
// FROM
// Users
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// COUNT(DISTINCT UserId) AS UniqueVoters,
// COUNT(DISTINCT PostId) AS UniqueVotedPosts
// FROM
// Votes
// )
// SELECT
// p.TotalPosts,
// p.UniquePostOwners,
// p.TotalQuestions,
// p.TotalAnswers,
// p.TotalTagWikis,
// u.TotalUsers,
// u.AvgReputation,
// u.MostRecentUserCreation,
// v.TotalVotes,
// v.UniqueVoters,
// v.UniqueVotedPosts
// FROM
// PostStats p,
// UserStats u,
// VoteStats v;
fn q11567(db: &'static So) -> String {
    let p = db.post.select(&db.post.post_type_id).fold_flat([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64]);
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let (un, rs, mc) = db.user.select((&db.user.reputation).and(&db.user.creation_date)).fold_flat((0i64, 0i64, i64::MIN), |(n, s, m), (r, c)| (n + 1, s + r, m.max(c)));
    let voters = one(whole(db.vote.iq()).select(&db.vote.user_id).count_distinct());
    let voted = one(whole(db.vote.iq()).select(&db.vote.post_id).count_distinct());
    row(vec![V::I(p[0]), V::I(owners), V::I(p[1]), V::I(p[2]), V::I(p[3]), V::I(un), avg(rs, un), V::T(mc), V::I(count(db.vote.iq())), V::I(voters), V::I(voted)])
}

// SELECT
// u.Id AS UserId,
// u.Reputation,
// COALESCE(p.PostCount, 0) AS PostCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount
// FROM
// Users u
// LEFT JOIN
// (SELECT OwnerUserId, COUNT(*) AS PostCount
// FROM Posts
// GROUP BY OwnerUserId) p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT OwnerUserId, COUNT(*) AS AnswerCount
// FROM Posts
// WHERE PostTypeId = 2
// GROUP BY OwnerUserId) a ON u.Id = a.OwnerUserId
// LEFT JOIN
// (SELECT UserId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY UserId) c ON u.Id = c.UserId
// ORDER BY
// u.Reputation DESC;
fn q11571(db: &'static So) -> String {
    let pc = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let ac = db.post.with((&db.post.post_type_id).eq(2)).select(&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let cu = comments_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&pc).and(&ac).and(&cu)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, p), a), c)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(p), V::I(a), V::I(c)])))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.CreationDate,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.CreationDate
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(c.Id, 0)) AS TotalComments,
// SUM(COALESCE(v.Id, 0)) AS TotalVotes
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
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.TotalComments,
// PS.TotalVotes,
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.TotalScore,
// UA.TotalComments AS UserTotalComments,
// UA.TotalVotes AS UserTotalVotes
// FROM
// PostStats PS
// JOIN
// UserActivity UA ON PS.PostId = UA.TotalPosts
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q11593(db: &'static So) -> String {
    let ua = g(db)
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).select(&db.comment.origid).opt()).and(votes_of(db).select(&db.vote.origid).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((s, c), x)) => [a[0] + 1, a[1] + s, a[2] + c.unwrap_or(0), a[3] + x.unwrap_or(0)],
            None => a,
        });
    let by_n: HashIdx<i64, Id<User>> = (&ua).map(|a: [i64; 4]| a[0]).inv().collect();
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&by_n)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&by_n).select(Ident::<User>::new().and(&ua)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "comments"]);
        f.extend([V::I(s.cx), V::I(s.vx), user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserPosts AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(c.CommentCount, 0)) AS CommentCount,
// SUM(COALESCE(v.VoteCount, 0)) AS VoteCount
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
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// GROUP BY
// u.Id
// ),
// BenchmarkResults AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// up.PostCount,
// up.CommentCount,
// up.VoteCount
// FROM
// Users u
// JOIN
// UserPosts up ON u.Id = up.UserId
// )
// SELECT
// Reputation,
// COUNT(UserId) AS UserCount,
// SUM(PostCount) AS TotalPosts,
// SUM(CommentCount) AS TotalComments,
// SUM(VoteCount) AS TotalVotes
// FROM
// BenchmarkResults
// GROUP BY
// Reputation
// ORDER BY
// Reputation DESC;
fn q11610(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select(comments_per_post(db).and(votes_per_post(db))).opt()).fold([0i64; 3], |a, p| match p {
        Some((c, x)) => [a[0] + 1, a[1] + c, a[2] + x],
        None => a,
    });
    let f = db.user.group_by(&db.user.reputation).select(&uf).fold([0i64; 4], |a, u| [a[0] + 1, a[1] + u[0], a[2] + u[1], a[3] + u[2]]);
    let mut v = Vec::new();
    (&f).drive(|r, a| v.push((r, a)));
    rows(v.iter().map(|&(r, a)| {
        let mut f = vec![V::I(r)];
        f.extend(ints(&a));
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
// p.OwnerUserId,
// COALESCE(v.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(v.DownVoteCount, 0) AS DownVoteCount,
// COALESCE(c.CommentCount, 0) AS CommentCount
// FROM
// Posts p
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
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.CommentCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// PostSummary ps
// JOIN
// Users u ON ps.OwnerUserId = u.Id
// ORDER BY
// ps.CreationDate DESC;
fn q11623(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and(comments_per_post(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, u), d), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(u), V::I(d), V::I(c)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
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
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount,
// SUM(V.BountyAmount) AS TotalBounty,
// AVG(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AvgUpVotes,
// AVG(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AvgDownVotes
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Posts A ON P.Id = A.ParentId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.AnswerCount,
// PS.TotalBounty,
// PS.AvgUpVotes,
// PS.AvgDownVotes
// FROM PostStats PS
// ORDER BY PS.ViewCount DESC, PS.Score DESC
// LIMIT 100;
fn q11626(db: &'static So) -> String {
    out(stats_with(db, db.post.iq(), "cva", &[], &[]), |&(p, _, _)| views_score(db, p), 100, |&(p, s, _)| {
        stat_fields(db, p, &s, &["id", "title", "created", "score", "views", "#cx", "#ax", "bounty_sum", "up_frac", "down_frac"])
    })
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END, 0)) AS QuestionCount,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AnswerCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS UpVotesCount,
// SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS DownVotesCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
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
// )
// SELECT
// US.DisplayName,
// US.PostCount,
// US.TotalScore,
// US.QuestionCount,
// US.AnswerCount,
// US.TotalViews,
// US.UpVotesCount,
// US.DownVotesCount,
// COALESCE(BS.BadgeCount, 0) AS BadgeCount,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStatistics US
// LEFT JOIN
// BadgeStatistics BS ON US.UserId = BS.UserId
// ORDER BY
// US.TotalScore DESC
// FETCH FIRST 100 ROWS ONLY;
fn q11635(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db)
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some((((t, s), w), x)) => [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + w.unwrap_or(0), a[5] + (x == Some(2)) as i64, a[6] + (x == Some(3)) as i64],
            None => a,
        });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| Reverse(a[1]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.AnswerCount,
// PS.UpVotes,
// PS.DownVotes,
// US.UserId,
// US.DisplayName AS UserDisplayName,
// US.Reputation,
// US.BadgeCount,
// US.TotalViews,
// US.TotalScore
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostId = U.AccountId
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.ViewCount DESC, PS.Score DESC
// LIMIT 100;
fn q11640(db: &'static So) -> String {
    let acc: HashIdx<i64, Id<User>> = (&db.user.account_id).inv().collect();
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    stats_fold(db, questions_only(db).with((&db.post.origid).select(&acc)), Ident::<Post>::new(), "cvA", &[])
        .and((&db.post.origid).select(&acc).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, _, _)| views_score(db, p), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(s.cx), V::I(s.ax), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")]);
        f.extend([V::I(a.bx), ustat_field(&a, "views_sum"), ustat_field(&a, "score_sum")]);
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
// COUNT(DISTINCT v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
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
// ps.BadgeCount
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q11647(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, db.post.iq(), "cvb", &[], &[&c, &x]), |p, _| score_views(db, p), 100, &["id", "title", "created", "views", "score", "#d0", "#d1", "#up", "#down", "#bx"])
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts,
// COUNT(DISTINCT b.Id) AS BadgeCount
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
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// CASE
// WHEN p.PostTypeId = 1 THEN 'Question'
// WHEN p.PostTypeId = 2 THEN 'Answer'
// ELSE 'Other'
// END AS PostType
// FROM
// Posts p
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.PositiveScorePosts,
// u.NegativeScorePosts,
// u.BadgeCount,
// p.PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.PostType
// FROM
// UserStats u
// JOIN
// PostStats p ON u.UserId = p.PostId
// ORDER BY
// u.Reputation DESC, p.ViewCount DESC;
fn q11654(db: &'static So) -> String {
    let uid = uids(db);
    let uf = g(db).select(posts_of(db).select(&db.post.score).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (s, _)| [a[0] + (s > Some(0)) as i64, a[1] + matches!(s, Some(x) if x < 0) as i64]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uf).and((&dp).opt()).and(&bu))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, (((u, a), d), b))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(b)];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "answers", "comments", "favorites"]));
        f.push(V::S(match db.post.post_type_id.get(p).unwrap() {
            1 => "Question",
            2 => "Answer",
            _ => "Other",
        }));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// COUNT(c.Id) AS TotalComments,
// AVG(v.VoteTypeId) AS AverageVoteType
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(b.Id, 0)) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// )
// SELECT
// ps.PostId,
// ps.PostTypeId,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.TotalComments,
// ps.AverageVoteType,
// us.UserId,
// us.TotalPosts,
// us.TotalScore,
// us.TotalBadges
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.CreationDate DESC;
fn q11655(db: &'static So) -> String {
    let uid = uids(db);
    let ub = g(db).select(posts_of(db).select(&db.post.score).opt().and(badges_of(db).select(&db.badge.origid).opt())).fold([0i64; 2], |a, (s, b)| [a[0] + s.unwrap_or(0), a[1] + b.unwrap_or(0)]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&ub).and((&dp).opt())))
        .drive(|p, (s, ((u, a), d))| v.push((p, s, u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views", "answers", "comments", "favorites"]);
        f.extend([V::I(s.cx), stat_field(&s, "vt_avg").unwrap(), user_col(db, u, "uid"), V::I(d), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(u.Reputation) AS AverageUserReputation,
// COUNT(DISTINCT u.Id) AS UniqueUsers,
// COUNT(CASE WHEN p.LastActivityDate >= cast('2024-10-01' as date) - INTERVAL '30 days' THEN p.Id END) AS RecentActivityCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11676(db: &'static So) -> String {
    let Post { view_count, score, last_activity_date, .. } = &db.post;
    let recent = date(2024, 9, 1);
    let f = by_key(db.post.iq(), name(db), view_count.opt().and(score).and((&db.post.owner_user).select(&db.user.reputation).opt()).and(last_activity_date), [0i64; 6], |a, (((w, s), r), la)| {
        [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0), a[5] + (la >= recent) as i64]
    });
    let du = db.post.group_by(name(db)).select(&db.post.owner_user).count_distinct();
    let mut v = Vec::new();
    (&f).and((&du).opt()).drive(|k, (a, d)| v.push((k, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), V::I(d), V::I(a[5])])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// AVG(u.Reputation) AS AvgReputation
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
// us.TotalScore,
// us.TotalViews,
// us.AvgReputation,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats us
// LEFT JOIN
// BadgeStats bs ON us.UserId = bs.UserId
// ORDER BY
// us.PostCount DESC;
fn q11677(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f.extend(ints(&b));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COALESCE(pv.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(pv.DownVoteCount, 0) AS DownVoteCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(a.AcceptedAnswerId, 0) AS AcceptedAnswerId,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// DATE_PART('epoch', cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate) AS AgeInSeconds
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
// PostId) pv ON p.Id = pv.PostId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT DISTINCT
// Id AS AcceptedAnswerId
// FROM
// Posts
// WHERE
// PostTypeId = 2) a ON p.AcceptedAnswerId = a.AcceptedAnswerId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC;
fn q11678(db: &'static So) -> String {
    let pv = post_votes(db);
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let acc = (&db.post.accepted_answer).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(2)));
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and((&pv).opt()).and(comments_per_post(db)).and(acc.opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, x), c), a)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(c), V::I(a.map_or(0, |a| db.post.origid.get(a).unwrap()))]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.push(V::F(hours_to(t0, db.post.creation_date.get(p).unwrap())));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// u.Reputation AS OwnerReputation,
// COUNT(CASE WHEN c.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN v.Id IS NOT NULL THEN 1 END) AS VoteCount,
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
// GROUP BY
// p.Id, p.PostTypeId, u.Reputation
// )
// SELECT
// pst.PostId,
// pst.PostTypeId,
// pst.OwnerReputation,
// pst.CommentCount,
// pst.VoteCount,
// pst.UpVoteCount,
// pst.DownVoteCount,
// pt.Name AS PostTypeName
// FROM
// PostStats pst
// JOIN
// PostTypes pt ON pst.PostTypeId = pt.Id
// ORDER BY
// pst.VoteCount DESC,
// pst.CommentCount DESC
// LIMIT 100;
fn q11687(db: &'static So) -> String {
    out(stats_with(db, db.post.iq(), "cv", &[], &[]), |&(_, s, _)| (Reverse(s.vx), Reverse(s.cx)), 100, |&(p, s, _)| {
        stat_fields(db, p, &s, &["id", "type_id", "rep", "#cx", "#vx", "#up", "#down", "type"])
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.OwnerUserId,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount,
// COALESCE(COUNT(DISTINCT C.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.OwnerUserId, U.DisplayName, P.Score, P.ViewCount
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.OwnerDisplayName,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.UpVotes,
// PS.DownVotes
// FROM
// PostStats PS
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC
// LIMIT 100;
fn q11693(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    stats_rows(db, stats_with(db, db.post.iq(), "cv", &[], &[&c]), |p, _| score_views(db, p), 100, &["id", "title", "created", "owner", "score", "views", "#d0", "#up", "#down"])
}

// WITH PostStats AS (
// SELECT
// P.PostTypeId,
// COUNT(P.Id) AS TotalPosts,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore,
// SUM(P.AnswerCount) AS TotalAnswers
// FROM
// Posts P
// GROUP BY
// P.PostTypeId
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS TotalBadges,
// SUM(U.Reputation) AS TotalReputation,
// AVG(U.Views) AS AverageViews
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id
// ),
// VoteStats AS (
// SELECT
// P.Id AS PostId,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id
// )
// SELECT
// PS.PostTypeId,
// PS.TotalPosts,
// PS.TotalViews,
// PS.AverageScore,
// PS.TotalAnswers,
// US.UserId,
// US.TotalBadges,
// US.TotalReputation,
// US.AverageViews,
// VS.TotalVotes,
// VS.UpVotes,
// VS.DownVotes
// FROM
// PostStats PS
// JOIN
// UserStats US ON US.UserId = (SELECT MIN(Id) FROM Users)
// JOIN
// VoteStats VS ON VS.PostId = (SELECT MIN(Id) FROM Posts)
// ORDER BY
// PS.TotalPosts DESC;
fn q11701(db: &'static So) -> String {
    let Post { view_count, score, answer_count, .. } = &db.post;
    let f = by_key(db.post.iq(), &db.post.post_type_id, view_count.opt().and(score).and(answer_count.opt()), [0i64; 6], |a, ((w, s), an)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0)]
    });
    let umin = db.user.select(&db.user.origid).fold_flat(i64::MAX, |m, x| m.min(x));
    let pmin = db.post.select(&db.post.origid).fold_flat(i64::MAX, |m, x| m.min(x));
    let us = g(db).select((&db.user.reputation).and(&db.user.views).and(badges_of(db).opt())).fold([0i64; 3], |a, ((r, w), b)| [a[0] + b.is_some() as i64, a[1] + r, w]);
    let vs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let one_u = rel(drain(db.user.with((&db.user.origid).eq(umin)).select(&us)));
    let one_p = rel(drain(db.post.with((&db.post.origid).eq(pmin)).select(&vs)));
    let mut v = Vec::new();
    (&f).cross(&one_u).cross(&one_p).drive(|((k, _), _), ((a, (_, ua)), (_, pa))| v.push((k, a, ua, pa)));
    rows(v.iter().map(|&(k, a, ua, pa)| {
        row(vec![
            V::I(k),
            V::I(a[0]),
            nullable(a[2], a[1]),
            avg(a[3], a[0]),
            nullable(a[5], a[4]),
            V::I(umin),
            V::I(ua[0]),
            V::I(ua[1]),
            V::F(ua[2] as f64),
            V::I(pa[0]),
            V::I(pa[1]),
            V::I(pa[2]),
        ])
    }))
}

// WITH PostCounts AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN 1 ELSE 0 END) AS TotalViews,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// TotalViews,
// TotalComments,
// TotalVotes
// FROM
// PostCounts
// ORDER BY
// TotalPosts DESC;
fn q11706(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.view_count).opt().and(comments_of(db).opt()).and(votes_of(db).opt()), [0i64; 4], |a, ((w, c), x)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + c.is_some() as i64, a[3] + x.is_some() as i64]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| {
        let mut f = vec![V::S(k)];
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserPostCount AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS PostCount,
// SUM(ViewCount) AS TotalViews
// FROM
// Posts
// WHERE
// CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// OwnerUserId
// ),
// UserBadgeCount AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ),
// UserVoteCount AS (
// SELECT
// UserId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// UserId
// )
// SELECT
// U.DisplayName,
// COALESCE(UPC.PostCount, 0) AS PostCount,
// COALESCE(UPC.TotalViews, 0) AS TotalViews,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(UVC.VoteCount, 0) AS VoteCount,
// U.Reputation,
// U.CreationDate
// FROM
// Users U
// LEFT JOIN
// UserPostCount UPC ON U.Id = UPC.OwnerUserId
// LEFT JOIN
// UserBadgeCount UBC ON U.Id = UBC.UserId
// LEFT JOIN
// UserVoteCount UVC ON U.Id = UVC.UserId
// ORDER BY
// U.Reputation DESC
// LIMIT 100;
fn q11714(db: &'static So) -> String {
    let pf = owned_since(db, year_ago()).group_by(&db.post.owner_user).select((&db.post.view_count).opt()).fold([0i64; 2], |a, w| [a[0] + 1, a[1] + w.unwrap_or(0)]);
    let bu = badges_per_user(db);
    let vu = votes_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt()).and(&bu).and(&vu)).drive(|_, x| v.push(x));
    out(v, |&(((u, _), _), _)| rep_desc(db, u), 100, |&(((u, p), b), x)| {
        let p = p.unwrap_or([0; 2]);
        vec![user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(b), V::I(x), user_col(db, u, "rep"), user_col(db, u, "ucreated")]
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id, P.PostTypeId, P.CreationDate, P.ViewCount, P.Score
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// COUNT(DISTINCT P.Id) AS PostsCount,
// SUM(P.ViewCount) AS TotalViewCount
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.Reputation
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
// US.TotalViewCount
// FROM PostStats PS
// JOIN UserStats US ON PS.PostId = US.UserId
// ORDER BY PS.CreationDate DESC
// LIMIT 100;
fn q11720(db: &'static So) -> String {
    let uid = uids(db);
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let us = g(db).select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt()).opt())).fold([0i64; 2], |a, (_, w)| {
        let w = w.flatten();
        [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0)]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and(&bu))))
        .drive(|_, y| v.push(y));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, c), x), (((u, a), d), b))| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "views", "score"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0)), user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b), V::I(d.unwrap_or(0)), nullable(a[1], a[0])]);
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.UserId) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.BadgeCount,
// us.UpVotesCount,
// us.DownVotesCount,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.VoteCount
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.Reputation DESC,
// ps.ViewCount DESC
// LIMIT 100;
fn q11724(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "bv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let dv = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&dv).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and(&bu)))
        .drive(|p, ((s, x), (((u, a), d), b))| v.push((p, s, x.unwrap_or(0), u, a, d.unwrap_or(0), b)));
    out(v, |&(p, _, _, u, _, _, _)| (rep_desc(db, u), views_desc(db, p)), 100, |&(p, s, x, u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d), V::I(b), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(s.cx), V::I(x)]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// AVG(EXTRACT(EPOCH FROM P.CreationDate)) AS AvgPostAge
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostFeatureStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// C.CommentCount,
// PH.RevisionCount,
// P.OwnerUserId
// FROM
// Posts P
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS RevisionCount FROM PostHistory GROUP BY PostId) PH ON P.Id = PH.PostId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.TotalViews,
// U.TotalScore,
// U.AvgPostAge,
// PFS.PostId,
// PFS.Title,
// PFS.CreationDate,
// PFS.ViewCount,
// PFS.Score,
// PFS.CommentCount,
// PFS.RevisionCount
// FROM
// UserPostStats U
// JOIN
// PostFeatureStats PFS ON U.UserId = PFS.OwnerUserId
// ORDER BY
// U.PostCount DESC, U.TotalScore DESC;
fn q11727(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(creation_date)).opt()).fold(([0i64; 6], 0i128), |(a, e), p| match p {
        Some((((t, w), s), c)) => ([a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s], e + c as i128),
        None => (a, e),
    });
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let hf = db.post_history.group_by(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&cf).opt()).and((&hf).opt()).and((&db.post.owner_user).select(Ident::<User>::new().and(&uf)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), h), (u, (a, e)))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), V::I(a[5]), V::F(e as f64 / a[0] as f64 / 1e6)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([oint(c), oint(h)]);
        row(f)
    }))
}

// WITH PostsStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score,
// p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS EditCount
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
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.OwnerDisplayName,
// COALESCE(phs.EditCount, 0) AS TotalEdits,
// ps.TotalComments,
// ps.TotalVotes,
// (ps.Score / NULLIF(ps.ViewCount, 0)) AS ScorePerView
// FROM
// PostsStats ps
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC
// LIMIT 100;
fn q11730(db: &'static So) -> String {
    let ed = history_of_types(db, &[4, 5, 6, 24]);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and(&ed).drive(|p, (s, e)| v.push((p, s, e)));
    out(v, |&(p, _, _)| views_score(db, p), 100, |&(p, s, e)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "favorites"]);
        f.push(named_owner(db, p, "Community User"));
        f.extend([V::I(e), V::I(s.cx), V::I(s.vx)]);
        f.push(match db.post.view_count.get(p) {
            Some(w) if w != 0 => V::F(db.post.score.get(p).unwrap() as f64 / w as f64),
            _ => V::Null,
        });
        f
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// SUM(c.CommentCount) AS TotalComments,
// COUNT(DISTINCT p.OwnerUserId) AS UniquePostOwners
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11743(db: &'static So) -> String {
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and((&db.post.view_count).opt()).and((&cf).opt()), [0i64; 6], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c.is_some() as i64, a[5] + c.unwrap_or(0)]
    });
    let owners = db.post.group_by(name(db)).select(&db.post.owner_user_id).count_distinct();
    let mut v = Vec::new();
    (&f).and((&owners).opt()).drive(|k, (a, o)| v.push((k, a, o.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, o)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), nullable(a[5], a[4]), V::I(o)])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("11395", q11395),
    ("11404", q11404),
    ("11406", q11406),
    ("11422", q11422),
    ("11423", q11423),
    ("11425", q11425),
    ("11430", q11430),
    ("11449", q11449),
    ("11465", q11465),
    ("11466", q11466),
    ("11468", q11468),
    ("11494", q11494),
    ("11495", q11495),
    ("11504", q11504),
    ("11508", q11508),
    ("11520", q11520),
    ("11524", q11524),
    ("11530", q11530),
    ("11532", q11532),
    ("11539", q11539),
    ("11551", q11551),
    ("11556", q11556),
    ("11561", q11561),
    ("11564", q11564),
    ("11566", q11566),
    ("11567", q11567),
    ("11571", q11571),
    ("11593", q11593),
    ("11610", q11610),
    ("11623", q11623),
    ("11626", q11626),
    ("11635", q11635),
    ("11640", q11640),
    ("11647", q11647),
    ("11654", q11654),
    ("11655", q11655),
    ("11676", q11676),
    ("11677", q11677),
    ("11678", q11678),
    ("11687", q11687),
    ("11693", q11693),
    ("11701", q11701),
    ("11706", q11706),
    ("11714", q11714),
    ("11720", q11720),
    ("11724", q11724),
    ("11727", q11727),
    ("11730", q11730),
    ("11743", q11743),
];
