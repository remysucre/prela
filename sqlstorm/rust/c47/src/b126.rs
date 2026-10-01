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

// --- batch 126 --------------------------------------------------------------

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(v.BountyAmount) AS TotalBounty,
// AVG(p.Score) AS AverageScore,
// MAX(p.CreationDate) AS LastActivityDate,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.PostTypeId, p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.Reputation) AS TotalReputation,
// MAX(u.CreationDate) AS AccountCreationDate
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
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// ps.TotalBounty,
// ps.AverageScore,
// ps.LastActivityDate,
// us.UserId AS OwnerUserId,
// us.DisplayName AS OwnerDisplayName,
// us.BadgeCount,
// us.TotalReputation,
// us.AccountCreationDate
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerUserId = us.UserId
// ORDER BY
// ps.LastActivityDate DESC
// LIMIT 100;
fn q14466(db: &'static So) -> String {
    let us = g(db).select((&db.user.reputation).and(badges_of(db).opt())).fold([0i64; 2], |a, (r, b)| [a[0] + b.is_some() as i64, a[1] + r]);
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[])
        .and(votes_per_post(db))
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&us)))
        .drive(|p, ((s, x), (u, a))| v.push((p, s, x, u, a)));
    out(v, |&(p, ..)| newest(db, p), 100, |&(p, s, x, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id"]);
        f.extend([V::I(s.cx), V::I(x), stat_field(&s, "bounty_sum").unwrap(), V::F(db.post.score.get(p).unwrap() as f64)]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), user_col(db, u, "ucreated")]);
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
// AVG(P.Score) AS AverageScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswersPerQuestion
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserBadges AS (
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
// U.TotalQuestions,
// U.TotalAnswers,
// U.AverageScore,
// U.TotalViews,
// U.TotalAnswersPerQuestion,
// COALESCE(B.TotalBadges, 0) AS TotalBadges,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats U
// LEFT JOIN
// UserBadges B ON U.UserId = B.UserId
// ORDER BY
// U.TotalPosts DESC;
fn q14469(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, answer_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(answer_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some((((t, s), w), an)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5] + an.unwrap_or(0)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([avg(a[3], a[0]), V::I(a[4]), V::I(a[5])]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// pt.Name AS PostType,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// p.Id, pt.Name
// )
// SELECT
// PostId,
// PostType,
// VoteCount,
// CommentCount,
// UpVotes,
// DownVotes,
// BadgeCount
// FROM
// PostStats
// ORDER BY
// VoteCount DESC,
// CommentCount DESC;
fn q14476(db: &'static So) -> String {
    let by_uid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let mut v = Vec::new();
    since(db, month_ago())
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and((&db.post.owner_user_id).select(&by_uid).opt()))
        .fold([0i64; 5], |a, ((t, c), b)| [a[0] + t.is_some() as i64, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64, a[4] + b.is_some() as i64])
        .drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id", "type"]);
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserParticipation AS (
// SELECT
// u.Id AS UserId,
// AVG(u.Reputation) AS AverageReputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id
// )
// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// AVG(AverageReputation) AS OverallAverageReputation,
// SUM(PostCount) AS TotalPostsByUsers,
// SUM(VoteCount) AS TotalVotesByUsers
// FROM
// UserParticipation;
fn q14479(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dv = ud(db, UserWhere::All, votes_by(db));
    let t = db.user.select((&db.user.reputation).and((&dp).opt()).and((&dv).opt())).fold_flat([0i64; 4], |a, ((r, p), x)| [a[0] + 1, a[1] + r, a[2] + p.unwrap_or(0), a[3] + x.unwrap_or(0)]);
    row(vec![V::I(count(db.post.iq())), V::I(count(db.user.iq())), V::I(count(db.vote.iq())), avg(t[1], t[0]), V::I(t[2]), V::I(t[3])])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// AVG(u.Reputation) AS AvgReputation
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
// TotalVotes,
// AvgReputation
// FROM
// UserPostStats
// ORDER BY
// TotalVotes DESC, AvgReputation DESC
// LIMIT 100;
fn q14486(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let mut v = Vec::new();
    (&us).drive(|u, a| v.push((u, a)));
    out(v, |&(u, a)| (Reverse(a.vx), rep_desc(db, u)), 100, |&(u, a)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a.n), V::I(a.vx), V::F(db.user.reputation.get(u).unwrap() as f64)]
    })
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, u.Reputation
// )
// SELECT
// PM.PostId,
// PM.Title,
// PM.CreationDate,
// PM.ViewCount,
// PM.Score,
// PM.AnswerCount,
// PM.CommentCount,
// PM.OwnerReputation,
// PM.LastEditDate,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = PM.PostId) AS VoteCount
// FROM
// PostMetrics PM
// ORDER BY
// PM.ViewCount DESC
// LIMIT 100;
fn q14493(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "ch", &[]).and(votes_per_post(db)).drive(|p, (s, x)| v.push((p, s, x)));
    out(v, |&(p, _, _)| views_desc(db, p), 100, |&(p, s, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.push(V::I(s.cx));
        f.extend(post_fields(db, p, &["rep"]));
        f.extend([stat_field(&s, "hmax").unwrap(), V::I(x)]);
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// (SELECT COUNT(c.Id) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(b.Id) FROM Badges b WHERE b.UserId = u.Id) AS BadgeCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score,
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q14499(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new(), "v", &[])
        .and(comments_per_post(db))
        .and((&db.post.owner_user).select(&bu))
        .drive(|p, ((s, c), b)| v.push((p, s, c, b)));
    rows(v.iter().map(|&(p, s, c, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "uid", "owner", "rep"]);
        f.extend([V::I(s.vx), V::I(s.up), V::I(s.down), V::I(c), V::I(b)]);
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate,
// u.Views,
// u.UpVotes,
// u.DownVotes,
// up.PostCount,
// up.QuestionCount,
// up.AnswerCount
// FROM
// Users u
// LEFT JOIN
// UserPostCounts up ON u.Id = up.UserId
// )
// SELECT
// ua.UserId,
// ua.Reputation,
// ua.CreationDate,
// ua.LastAccessDate,
// ua.Views,
// ua.UpVotes,
// ua.DownVotes,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM
// UserActivity ua
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(Id) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) b ON ua.UserId = b.UserId
// ORDER BY
// ua.Reputation DESC,
// ua.PostCount DESC;
fn q14513(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    user_posts_q(db).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f: Vec<V> = ["uid", "rep", "ucreated", "last_access", "uviews", "uup", "udown"].iter().map(|c| user_col(db, u, c)).collect();
        f.extend(ints(&a[..3]));
        f.push(V::I(b));
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
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
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
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.UpVotes,
// ua.DownVotes,
// ua.BadgeCount,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.CloseCount,
// ps.ReopenCount
// FROM
// UserActivity ua
// LEFT JOIN
// PostStats ps ON ua.UserId = ps.PostId
// ORDER BY
// ua.PostCount DESC, ua.UpVotes DESC;
fn q14515(db: &'static So) -> String {
    let uid = uids(db);
    let pid = pids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "vb", any_post);
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "ch", &[]);
    let mut v = Vec::new();
    (&us).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&pf)).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "#up", "#down", "#bx"].iter().map(|c| ustat_field(&a, c)));
        match p {
            Some((p, s)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
                f.extend([V::I(s.cx), V::I(s.h10), V::I(s.h11)]);
            }
            None => f.extend(nulls(8)),
        }
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS TotalComments,
// COALESCE(answers.AnswerCount, 0) AS TotalAnswers,
// COALESCE(voteCounts.UpVotes, 0) AS TotalUpVotes,
// COALESCE(voteCounts.DownVotes, 0) AS TotalDownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount
// FROM Posts
// WHERE PostTypeId = 2
// GROUP BY ParentId) answers ON p.Id = answers.ParentId
// LEFT JOIN
// (SELECT PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes
// GROUP BY PostId) voteCounts ON p.Id = voteCounts.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14527(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.post_type_id).eq(1))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(typed_answers_per_post(db)).and((&pv).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, c), a), x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.extend([V::I(c), V::I(a), V::I(x[1]), V::I(x[2])]);
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
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts,
// AVG(p.Score) AS AvgScore,
// MAX(p.CreationDate) AS LastPostDate
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
// ups.TotalUpvotedPosts,
// ups.AvgScore,
// ups.LastPostDate,
// ubs.TotalBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY
// ups.TotalPosts DESC
// LIMIT 100;
fn q14528(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(creation_date)).opt()).fold([0, 0, 0, 0, 0, i64::MIN], |a: [i64; 6], p| match p {
        Some(((t, s), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + s, a[5].max(c)],
        None => a,
    });
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&uf).and((&bf).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.extend([avg(a[4], a[0]), if a[0] == 0 { V::Null } else { V::T(a[5]) }, oint(b)]);
        f
    })
}

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
// u.Reputation AS OwnerReputation,
// v.UpVoteCount,
// v.DownVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
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
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY
// p.CreationDate DESC;
fn q14529(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned_since(db, year_ago()).select(Ident::<Post>::new().and((&pv).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep"]);
        f.extend([oint(x.map(|x| x[1])), oint(x.map(|x| x[2]))]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
// AVG(P.Score) AS AverageScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// ),
// TopUsers AS (
// SELECT
// UserId,
// Reputation,
// PostCount,
// QuestionCount,
// AnswerCount,
// PositiveScoreCount,
// AverageScore
// FROM
// UserPostStats
// WHERE
// PostCount > 0
// ORDER BY
// Reputation DESC
// LIMIT 10
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// UPost.PostCount,
// UPost.QuestionCount,
// UPost.AnswerCount,
// UPost.PositiveScoreCount,
// UPost.AverageScore
// FROM
// TopUsers UPost
// JOIN
// Users U ON U.Id = UPost.UserId;
fn q14536(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + s],
        None => a,
    });
    let base = db.user.with((&uf).filt(|a: [i64; 5]| a[0] > 0));
    let top: MatSet<Id<User>> = whole(&base).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&uf)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, a)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a[..4]));
        f.push(avg(a[4], a[0]));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(V.BountyAmount) AS TotalBounty,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
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
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalComments,
// TotalBounty,
// TotalUpvotes,
// TotalDownvotes,
// (TotalUpvotes - TotalDownvotes) AS NetVotes
// FROM
// UserStats
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q14552(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&dc).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    out(v, |&(_, _, p, _)| Reverse(p), 10, |&(u, a, p, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(c)];
        f.extend(["bounty_sum", "#up", "#down", "#net"].iter().map(|c| ustat_field(&a, c)));
        f
    })
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.OwnerUserId,
// u.Reputation AS UserReputation,
// p.CreationDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ),
// PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalUsers,
// AVG(UserReputation) AS AverageUserReputation
// FROM
// RecentPosts
// ),
// PostTypeCounts AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(*) AS PostTypeCount
// FROM
// RecentPosts rp
// JOIN
// PostTypes pt ON rp.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// pc.TotalPosts,
// pc.TotalUsers,
// pc.AverageUserReputation,
// ptc.PostTypeName,
// ptc.PostTypeCount
// FROM
// PostCounts pc
// LEFT JOIN
// PostTypeCounts ptc ON true
// ORDER BY
// ptc.PostTypeName;
fn q14555(db: &'static So) -> String {
    let base = || owned_since(db, year_ago());
    let t = base().select((&db.post.owner_user).select(&db.user.reputation)).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let du = one(whole(base()).select(&db.post.owner_user_id).count_distinct());
    let f = by_key(base(), name(db), Ident::<Post>::new(), 0i64, |a, _| a + 1);
    let ptc: HashIdx<(), (Str, i64)> = whole(&f).select(Same::<Str>::new().and(&f)).collect();
    let mut v = Vec::new();
    rel(vec![()]).select((&ptc).opt()).drive(|_, x| v.push(x));
    rows(v.iter().map(|&x| {
        let mut r = vec![V::I(t[0]), V::I(du), avg(t[1], t[0])];
        r.extend(match x {
            Some((k, n)) => [V::S(k), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(r)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// AVG(COALESCE(c.CommentCount, 0)) AS AverageCommentCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14560(db: &'static So) -> String {
    let f = by_key(
        db.post.iq(),
        name(db),
        votes_of(db).select(&db.vote.vote_type_id).opt().and(&db.post.score).and((&db.post.view_count).opt()).and(comments_per_post(db)),
        [0i64; 7],
        |a, (((t, s), w), c)| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + c],
    );
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), avg(a[5], a[4]), avg(a[6], a[0])])))
}

// WITH UserPostInfo AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName AS UserName,
// p.Id AS PostId,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// COUNT(pc.Id) AS CommentCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments pc ON p.Id = pc.PostId
// GROUP BY
// u.Id, u.DisplayName, p.Id, p.Title, p.CreationDate
// ),
// UserBadgeInfo AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// up.UserId,
// up.UserName,
// up.PostId,
// up.PostTitle,
// up.PostCreationDate,
// up.CommentCount,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount
// FROM
// UserPostInfo up
// LEFT JOIN
// UserBadgeInfo ub ON up.UserId = ub.UserId
// ORDER BY
// up.UserId, up.PostCreationDate DESC;
fn q14581(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(posts_of(db).select(Ident::<Post>::new().and(comments_per_post(db))).opt()).and(&bu)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["id", "title", "created"]));
                f.push(V::I(c));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::I(0)]),
        }
        f.push(V::I(b));
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// UserReputation AS (
// SELECT
// Id AS UserId,
// Reputation
// FROM
// Users
// )
// SELECT
// ur.UserId,
// ur.Reputation,
// upc.PostCount,
// upc.QuestionCount,
// upc.AnswerCount,
// upc.WikiCount
// FROM
// UserReputation ur
// JOIN
// UserPostCounts upc ON ur.UserId = upc.UserId
// ORDER BY
// ur.Reputation DESC,
// upc.PostCount DESC
// LIMIT 100;
fn q14595(db: &'static So) -> String {
    let pc = g(db).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 4], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
        None => a,
    });
    let mut v = Vec::new();
    (&pc).drive(|u, a| v.push((u, a)));
    out(v, |&(u, a)| (rep_desc(db, u), Reverse(a[0])), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f
    })
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore
// FROM
// Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation,
// AVG(Views) AS AvgViews
// FROM
// Users
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// COUNT(DISTINCT PostId) AS UniqueVotedPosts,
// COUNT(DISTINCT UserId) AS UniqueVoters
// FROM
// Votes
// )
// SELECT
// ps.TotalPosts,
// ps.UniquePostOwners,
// ps.AvgViewCount,
// ps.AvgScore,
// us.TotalUsers,
// us.AvgReputation,
// us.AvgViews,
// vs.TotalVotes,
// vs.UniqueVotedPosts,
// vs.UniqueVoters
// FROM
// PostStats ps,
// UserStats us,
// VoteStats vs;
fn q14603(db: &'static So) -> String {
    let p = post_totals(db);
    let du = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let u = db.user.select((&db.user.reputation).and(&db.user.views)).fold_flat([0i64; 3], |a, (r, w)| [a[0] + 1, a[1] + r, a[2] + w]);
    let dvp = one(whole(db.vote.iq()).select(&db.vote.post_id).count_distinct());
    let dv = one(whole(db.vote.iq()).select(&db.vote.user_id).count_distinct());
    row(vec![V::I(p[0]), V::I(du), avg(p[3], p[2]), avg(p[1], p[0]), V::I(u[0]), avg(u[1], u[0]), avg(u[2], u[0]), V::I(count(db.vote.iq())), V::I(dvp), V::I(dv)])
}

// WITH PostTypeStats AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS UserPostCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// )
// SELECT
// pts.PostTypeName,
// pts.PostCount,
// pts.AverageScore,
// (SELECT COUNT(*) FROM UserPostCounts) AS TotalUsers
// FROM
// PostTypeStats pts
// ORDER BY
// pts.PostCount DESC;
fn q14606(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), &db.post.score, [0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let nu = count(db.user.iq());
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(nu)])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.Score IS NOT NULL THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount,
// MAX(p.Score) AS MaxPostScore
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
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.AnswerCount,
// us.BadgeCount,
// us.MaxPostScore,
// p.Title AS TopPostTitle
// FROM
// UserStats us
// LEFT JOIN
// Posts p ON us.UserId = p.OwnerUserId AND p.Score = us.MaxPostScore
// ORDER BY
// us.Reputation DESC, us.MaxPostScore DESC;
fn q14616(db: &'static So) -> String {
    let us = g(db)
        .select(posts_of(db).select(&db.post.score).opt().and(badges_of(db).opt()))
        .fold([0, 0, i64::MIN], |a: [i64; 3], (s, b)| [a[0] + s.is_some() as i64, a[1] + b.is_some() as i64, s.map_or(a[2], |s| a[2].max(s))]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&us)
        .and((&dp).opt())
        .and(posts_of(db).select(Ident::<Post>::new().and(&db.post.score)).opt())
        .filt(|((a, _), p): (([i64; 3], Option<i64>), Option<(Id<Post>, i64)>)| p.map_or(true, |(_, s)| s == a[2]))
        .drive(|u, ((a, d), p)| v.push((u, a, d.unwrap_or(0), p)));
    rows(v.iter().map(|&(u, a, d, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d), V::I(a[0]), V::I(a[1])];
        f.push(if a[0] == 0 { V::Null } else { V::I(a[2]) });
        f.push(p.map_or(V::Null, |(p, _)| post_fields(db, p, &["title"]).pop().unwrap()));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// COUNT(DISTINCT Posts.Id) AS PostCount,
// SUM(Posts.Score) AS TotalScore,
// SUM(Posts.ViewCount) AS TotalViewCount,
// SUM(Users.UpVotes) AS TotalUpVotes,
// SUM(Users.DownVotes) AS TotalDownVotes
// FROM Users
// LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY Users.Id, Users.DisplayName
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
// Posts.FavoriteCount,
// PostTypes.Name AS PostType
// FROM Posts
// JOIN PostTypes ON Posts.PostTypeId = PostTypes.Id
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.PostCount,
// U.TotalScore,
// U.TotalViewCount,
// U.TotalUpVotes,
// U.TotalDownVotes,
// P.PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// P.PostType
// FROM UserStats U
// LEFT JOIN PostStats P ON U.UserId = P.PostId
// ORDER BY U.TotalScore DESC, U.PostCount DESC;
fn q14620(db: &'static So) -> String {
    let pid = pids(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt())).opt())).fold([0i64; 6], |a, ((u, d), p)| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + u, a[5] + d],
        None => [a[0], a[1], a[2], a[3], a[4] + u, a[5] + d],
    });
    let mut v = Vec::new();
    (&us).and((&db.user.origid).select(&pid).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5])];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "type"])),
            None => f.extend(nulls(9)),
        }
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COALESCE(AnswerCount, 0) AS AnswerCount,
// COALESCE(CommentCount, 0) AS CommentCount,
// COALESCE(ViewCount, 0) AS ViewCount,
// COALESCE(Score, 0) AS Score,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.AnswerCount, p.CommentCount,
// p.ViewCount, p.Score, u.DisplayName, u.Reputation
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.AnswerCount,
// ps.CommentCount,
// ps.ViewCount,
// ps.Score,
// ps.OwnerDisplayName,
// ps.OwnerReputation,
// ps.Upvotes,
// ps.Downvotes,
// (ps.Upvotes - ps.Downvotes) AS NetVotes
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC
// LIMIT 100;
fn q14633(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&pv).opt())).drive(|_, x| v.push(x));
    let w0 = |p: Id<Post>| db.post.view_count.get(p).unwrap_or(0);
    out(v, |&(p, _)| (score_desc(db, p), Reverse(w0(p))), 100, |&(p, x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(db.post.answer_count.get(p).unwrap_or(0)), post_fields(db, p, &["comments"]).pop().unwrap(), V::I(w0(p))]);
        f.extend(post_fields(db, p, &["score", "owner", "rep"]));
        f.extend([V::I(x[1]), V::I(x[2]), V::I(x[1] - x[2])]);
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
// SUM(V.BountyAmount) AS TotalBounties,
// AVG(P.Score) AS AvgPostScore,
// MAX(P.CreationDate) AS MostRecentPost
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
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// US.UserId,
// US.Reputation,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.TotalBounties,
// US.AvgPostScore,
// US.MostRecentPost,
// BC.BadgeCount
// FROM
// UserStats US
// LEFT JOIN
// BadgeCounts BC ON US.UserId = BC.UserId
// ORDER BY
// US.Reputation DESC, US.PostCount DESC;
fn q14634(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bf).opt()).drive(|u, ((a, p), b)| v.push((u, a, p.unwrap_or(0), b)));
    rows(v.iter().map(|&(u, a, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(p)];
        f.extend(["#q", "#a", "bounty_sum", "score_avg", "created_max"].iter().map(|c| ustat_field(&a, c)));
        f.push(oint(b));
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// UserBadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// u.Id,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate,
// COALESCE(UPC.PostCount, 0) AS TotalPosts,
// COALESCE(UPC.QuestionCount, 0) AS TotalQuestions,
// COALESCE(UPC.AnswerCount, 0) AS TotalAnswers,
// COALESCE(UBC.BadgeCount, 0) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// UserPostCounts UPC ON u.Id = UPC.UserId
// LEFT JOIN
// UserBadgeCounts UBC ON u.Id = UBC.UserId
// ORDER BY
// u.Reputation DESC
// LIMIT 100;
fn q14635(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    user_posts_q(db).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(u, _, _)| rep_desc(db, u), 100, |&(u, a, b)| {
        let mut f: Vec<V> = ["uid", "name", "rep", "ucreated", "last_access"].iter().map(|c| user_col(db, u, c)).collect();
        f.extend(ints(&a[..3]));
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
// p.Score,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(v.DownVoteCount, 0) AS DownVoteCount
// FROM
// Posts p
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
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// Score,
// AnswerCount,
// CommentCount,
// UpVoteCount,
// DownVoteCount,
// (AnswerCount + CommentCount + UpVoteCount - DownVoteCount) AS EngagementScore
// FROM
// PostStats
// ORDER BY
// EngagementScore DESC
// LIMIT 100;
fn q14653(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and((&pv).opt())).drive(|_, ((( p, a), c), x)| {
        let x = x.unwrap_or([0; 3]);
        v.push((p, a, c, x[1], x[2], a + c + x[1] - x[2]));
    });
    out(v, |&(p, .., e)| (Reverse(e), db.post.origid.get(p).unwrap()), 100, |&(p, a, c, u, d, e)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(ints(&[a, c, u, d, e]));
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
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(ph.Id) AS HistoryCount
// FROM
// Posts p
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.Questions,
// ups.Answers,
// ups.TotalScore,
// ups.AvgViewCount,
// pH.HistoryCount
// FROM
// UserPostStats ups
// LEFT JOIN
// PostHistoryStats pH ON ups.UserId = pH.PostId
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC;
fn q14660(db: &'static So) -> String {
    let pid = pids(db);
    let mut v = Vec::new();
    upqa(db).and((&db.user.origid).select(&pid).select(history_per_post(db)).opt()).drive(|u, (a, h)| v.push((u, a, h)));
    rows(v.iter().map(|&(u, a, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[5], a[0]), avg(a[4], a[3]), oint(h)]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(p.AnswerCount), 0) AS TotalAnswersReceived,
// COALESCE(SUM(p.CommentCount), 0) AS TotalComments
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.CreationDate,
// u.DisplayName,
// u.LastAccessDate,
// ps.TotalPosts,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.TotalScore,
// ps.TotalViews,
// ps.TotalAnswersReceived,
// ps.TotalComments
// FROM
// Users u
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.CreationDate,
// us.LastAccessDate,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalScore,
// us.TotalViews,
// us.TotalAnswersReceived,
// us.TotalComments
// FROM
// UserStats us
// ORDER BY
// us.Reputation DESC, us.TotalPosts DESC;
fn q14661(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, answer_count, comment_count, .. } = &db.post;
    let pf = owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(score).and(view_count.opt()).and(answer_count.opt()).and(comment_count))
        .fold([0i64; 7], |a, ((((t, s), w), an), c)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5] + an.unwrap_or(0), a[6] + c]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, p)| {
        let mut f: Vec<V> = ["uid", "name", "rep", "ucreated", "last_access"].iter().map(|c| user_col(db, u, c)).collect();
        f.extend((0..7).map(|i| oint(p.map(|p| p[i]))));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// VoteStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UPS.TotalPosts, 0) AS TotalPosts,
// COALESCE(UPS.Questions, 0) AS Questions,
// COALESCE(UPS.Answers, 0) AS Answers,
// COALESCE(UPS.Wikis, 0) AS Wikis,
// COALESCE(UPS.TagWikis, 0) AS TagWikis,
// COALESCE(VS.TotalVotes, 0) AS TotalVotes,
// COALESCE(VS.UpVotes, 0) AS UpVotes,
// COALESCE(VS.DownVotes, 0) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// UserPostStats UPS ON U.Id = UPS.UserId
// LEFT JOIN
// VoteStats VS ON U.Id = VS.OwnerUserId
// ORDER BY
// TotalPosts DESC;
fn q14671(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 5], |a, t| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + matches!(t, 4 | 5) as i64]
    });
    let vs = owned(db).group_by(&db.post.owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pc).opt()).and((&vs).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p.unwrap_or([0; 5])));
        f.extend(ints(&x.unwrap_or([0; 3])));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// AVG(COALESCE(p.Score, 0)) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.AnswerCount) AS AverageAnswersPerQuestion
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.TotalAnswers,
// u.TotalQuestions,
// u.AverageScore,
// u.TotalViews,
// u.AverageAnswersPerQuestion
// FROM
// UserPostStats u
// ORDER BY
// u.TotalPosts DESC
// LIMIT 10;
fn q14674(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, answer_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(answer_count.opt())).opt()).fold([0i64; 8], |a, p| match p {
        Some((((t, s), w), an)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + an.is_some() as i64, a[7] + an.unwrap_or(0)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| Reverse(a[0]), 10, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([or0(a[3], a[0]), nullable(a[5], a[4]), avg(a[7], a[6])]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostsByUsers,
// SUM(CASE WHEN p.Score IS NOT NULL THEN 1 ELSE 0 END) AS ScoredPosts
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
// SUM(u.Reputation) AS TotalReputation
// FROM
// Users u
// ),
// VoteStats AS (
// SELECT
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.PostsByUsers,
// ps.ScoredPosts,
// us.TotalUsers,
// us.TotalReputation,
// vs.TotalVotes,
// vs.UpVotes,
// vs.DownVotes
// FROM
// PostStats ps,
// UserStats us,
// VoteStats vs
// ORDER BY
// ps.TotalPosts DESC;
fn q14676(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.owner_user_id).opt().and((&db.post.score).opt()), [0i64; 3], |a, (o, s)| {
        [a[0] + 1, a[1] + o.is_some() as i64, a[2] + s.is_some() as i64]
    });
    let u = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| {
        let mut f = vec![V::S(k)];
        f.extend(ints(&a));
        f.extend(ints(&u));
        f.extend(ints(&x));
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
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// MAX(p.CreationDate) AS LastPostDate
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
// ups.TotalScore,
// ups.TotalViews,
// ups.LastPostDate,
// COALESCE(ubs.TotalBadges, 0) AS TotalBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC;
fn q14682(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).opt()).fold([0, 0, 0, 0, 0, i64::MIN], |a: [i64; 6], p| match p {
        Some((((t, s), w), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5].max(c)],
        None => a,
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..5]));
        f.extend([if a[0] == 0 { V::Null } else { V::T(a[5]) }, V::I(b)]);
        row(f)
    }))
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// p.CreationDate,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// )
// SELECT
// pd.PostId,
// pd.Title,
// pd.ViewCount,
// pd.Score,
// pd.CommentCount,
// pd.VoteCount,
// pd.CreationDate,
// pd.LastEditDate,
// us.PostCount AS UserPostCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostDetails pd
// JOIN
// Users u ON pd.PostId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// pd.Score DESC, pd.ViewCount DESC
// LIMIT 100;
fn q14689(db: &'static So) -> String {
    let uid = uids(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(posts_of(db).opt())).fold([0i64; 3], |a, ((u, d), p)| [a[0] + p.is_some() as i64, a[1] + u, a[2] + d]);
    let hf = history_n_max(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)).and((&hf).opt()).and((&db.post.origid).select(&uid).select(&us)))
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| score_views(db, p), 100, |&((((p, c), x), h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(c), V::I(x)]);
        f.extend(post_fields(db, p, &["created"]));
        f.push(ots(h.map(|h| h.1)));
        f.extend(ints(&a));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.UpVotes,
// U.DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.Reputation, U.UpVotes, U.DownVotes
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// COUNT(C.Id) AS CommentCount,
// SUM(V.BountyAmount) AS TotalBounty
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId
// ),
// FinalStats AS (
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.BadgeCount,
// PS.PostId,
// PS.PostTypeId,
// PS.CommentCount,
// PS.TotalBounty
// FROM
// UserStats U
// JOIN
// PostStats PS ON U.UserId = PS.PostId
// )
// SELECT
// F.UserId,
// F.Reputation,
// F.PostCount,
// F.BadgeCount,
// F.PostId,
// F.PostTypeId,
// F.CommentCount,
// F.TotalBounty,
// CASE
// WHEN F.PostTypeId = 1 THEN 'Question'
// WHEN F.PostTypeId = 2 THEN 'Answer'
// WHEN F.PostTypeId = 3 THEN 'Wiki'
// ELSE 'Other'
// END AS PostTypeName
// FROM
// FinalStats F
// ORDER BY
// F.Reputation DESC, F.PostCount DESC;
fn q14692(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&bu))).drive(|p, (s, ((u, d), b))| v.push((p, s, u, d.unwrap_or(0), b)));
    rows(v.iter().map(|&(p, s, u, d, b)| {
        let t = db.post.post_type_id.get(p).unwrap();
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d), V::I(b)];
        f.extend(post_fields(db, p, &["id", "type_id"]));
        f.extend([V::I(s.cx), stat_field(&s, "bounty_sum").unwrap()]);
        f.push(V::S(match t {
            1 => "Question",
            2 => "Answer",
            3 => "Wiki",
            _ => "Other",
        }));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(c.Score) AS TotalCommentScore,
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
// u.Id, u.DisplayName
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionsCount,
// us.AnswersCount,
// us.TotalCommentScore,
// us.UpVotes,
// us.DownVotes,
// COALESCE(FLOOR((CAST(us.UpVotes AS FLOAT) / NULLIF(us.DownVotes, 0)) * 100), 0) AS UpvoteDownvoteRatio
// FROM
// UserStats us
// ORDER BY
// us.PostCount DESC
// LIMIT 100;
fn q14693(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).drive(|u, (a, p)| v.push((u, a, p.unwrap_or(0))));
    out(v, |&(_, _, p)| Reverse(p), 100, |&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p)];
        f.extend(["#q", "#a", "cscore_sum", "#up", "#down"].iter().map(|c| ustat_field(&a, c)));
        f.push(V::F(if a.down == 0 { 0.0 } else { ((a.up as f32 / a.down as f32) * 100.0f32).floor() as f64 }));
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
// SUM(V.BountyAmount) AS TotalBounty,
// AVG(COALESCE(CAST(P.Score AS FLOAT) / NULLIF(P.ViewCount, 0), 0)) AS AvgScorePerView,
// MAX(P.CreationDate) AS LastActivityDate
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.DisplayName
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.PostCount,
// UA.QuestionCount,
// UA.AnswerCount,
// UA.TotalBounty,
// UA.AvgScorePerView,
// UA.LastActivityDate,
// U.Reputation
// FROM UserActivity UA
// JOIN Users U ON UA.UserId = U.Id
// ORDER BY UA.PostCount DESC
// LIMIT 100;
fn q14694(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let uf = g(db)
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(creation_date).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold(([0, 0, 0, 0, 0, 0, i64::MIN], 0.0f64), |(a, f): ([i64; 7], f64), p| match p {
            Some(((((t, s), w), c), b)) => {
                let b = b.flatten();
                let r = match w {
                    Some(w) if w != 0 => (s as f32 / w as f32) as f64,
                    _ => 0.0,
                };
                ([a[0] + 1, a[1] + 1, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0), a[6].max(c)], f + r)
            }
            None => ([a[0] + 1, a[1], a[2], a[3], a[4], a[5], a[6]], f),
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).drive(|u, ((a, f), p)| v.push((u, a, f, p.unwrap_or(0))));
    out(v, |&(.., p)| Reverse(p), 100, |&(u, a, f, p)| {
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(p),
            V::I(a[2]),
            V::I(a[3]),
            nullable(a[5], a[4]),
            V::F(f / a[0] as f64),
            if a[1] == 0 { V::Null } else { V::T(a[6]) },
            user_col(db, u, "rep"),
        ]
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.FavoriteCount) AS TotalFavorites
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
// ups.AvgScore,
// ups.TotalViews,
// ups.TotalFavorites,
// COALESCE(ub.TotalBadges, 0) AS TotalBadges,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadges ub ON ups.UserId = ub.UserId
// ORDER BY
// ups.TotalPosts DESC
// LIMIT 100;
fn q14700(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, favorite_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(favorite_count.opt())).opt()).fold([0i64; 8], |a, p| match p {
        Some((((t, s), w), fc)) => [
            a[0] + 1,
            a[1] + matches!(t, 1 | 2) as i64,
            a[2] + (t == 2) as i64,
            a[3] + s,
            a[4] + w.is_some() as i64,
            a[5] + w.unwrap_or(0),
            a[6] + fc.is_some() as i64,
            a[7] + fc.unwrap_or(0),
        ],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([avg(a[3], a[0]), nullable(a[5], a[4]), nullable(a[7], a[6])]);
        f.extend(ints(&b));
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
// p.AnswerCount,
// p.CommentCount,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
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
// ps.TotalUpVotes,
// ps.TotalDownVotes,
// us.UserId,
// us.DisplayName,
// us.TotalVotes,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges
// FROM
// PostStats ps
// LEFT JOIN
// Users u ON ps.PostId = u.Id
// LEFT JOIN
// UserStats us ON us.UserId = u.Id
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC;
fn q14713(db: &'static So) -> String {
    let uid = uids(db);
    let us = g(db).select(votes_by(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, (_, c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let dv = ud(db, UserWhere::All, votes_by(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[])
        .and(comments_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dv).opt())).opt())
        .drive(|p, ((s, c), u)| v.push((p, s, c, u)));
    rows(v.iter().map(|&(p, s, c, u)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments"]);
        f.extend([V::I(c), V::I(s.up), V::I(s.down)]);
        match u {
            Some(((u, a), d)) => {
                f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d.unwrap_or(0))]);
                f.extend(ints(&a));
            }
            None => f.extend(nulls(6)),
        }
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AverageQuestionScore
// FROM Posts
// WHERE PostTypeId = 1
// ),
// UserStats AS (
// SELECT
// COUNT(DISTINCT Id) AS TotalUsers,
// AVG(Reputation) AS AverageReputation
// FROM Users
// WHERE Id IN (SELECT OwnerUserId FROM Posts GROUP BY OwnerUserId HAVING COUNT(*) > 5)
// )
// SELECT
// PS.TotalPosts,
// PS.AverageQuestionScore,
// US.TotalUsers,
// US.AverageReputation
// FROM PostStats PS, UserStats US;
fn q14717(db: &'static So) -> String {
    let q = questions_only(db).select(&db.post.score).fold_flat([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let pc = db.post.group_by(&db.post.owner_user_id).fold(0i64, |a, _| a + 1);
    let u = db.user.with((&db.user.origid).select((&pc).filt(|n: i64| n > 5))).select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    row(vec![V::I(q[0]), avg(q[1], q[0]), V::I(u[0]), avg(u[1], u[0])])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PostsHighViews,
// AVG(p.Score) AS AvgScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostHistories AS (
// SELECT
// p.Id AS PostId,
// ph.PostHistoryTypeId,
// COUNT(ph.Id) AS HistoryCount
// FROM
// Posts p
// JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, ph.PostHistoryTypeId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.PositiveScorePosts,
// ups.PostsHighViews,
// ups.AvgScore,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.HistoryCount ELSE 0 END) AS CloseVotes,
// SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.HistoryCount ELSE 0 END) AS ReopenVotes,
// SUM(CASE WHEN ph.PostHistoryTypeId = 12 THEN ph.HistoryCount ELSE 0 END) AS DeletionVotes,
// SUM(CASE WHEN ph.PostHistoryTypeId = 13 THEN ph.HistoryCount ELSE 0 END) AS UndeletionVotes
// FROM
// UserPostStats ups
// LEFT JOIN
// PostHistories ph ON ups.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = ph.PostId)
// GROUP BY
// ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.PositiveScorePosts, ups.PostsHighViews, ups.AvgScore
// ORDER BY
// ups.TotalPosts DESC;
fn q14735(db: &'static So) -> String {
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let hc = db.post_history.group_by(post.and(post_history_type_id)).fold(0i64, |a, _| a + 1);
    let keys: MatSet<(Id<Post>, i64)> = db.post_history.select(post.and(post_history_type_id)).collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64)> = (&keys).map(|(p, _)| p).inv().collect();
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let w = UserWhere::RepGt(0);
    let uf = user_base(db, w).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, s), vw)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (vw.map_or(false, |x| x > 100)) as i64, a[5] + s],
        None => a,
    });
    let hs = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&by_post).select(Same::<(Id<Post>, i64)>::new().map(|(_, t)| t).and(&hc)).opt())
        .fold([0i64; 4], |mut a, x| {
            if let Some((t, n)) = x {
                if (10..14).contains(&t) {
                    a[(t - 10) as usize] += n;
                }
            }
            a
        });
    let mut v = Vec::new();
    (&uf).and(&hs).drive(|u, (a, h)| v.push((u, a, h)));
    rows(v.iter().map(|&(u, a, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..5]));
        f.push(avg(a[5], a[0]));
        f.extend(ints(&h));
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
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.FavoriteCount, 0)) AS TotalFavorites
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// UserBadgeStats AS (
// SELECT
// b.UserId,
// COUNT(*) AS TotalBadges,
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
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalScore,
// ups.TotalViews,
// ups.TotalFavorites,
// ubs.TotalBadges,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges
// FROM UserPostStats ups
// LEFT JOIN UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY ups.TotalScore DESC
// LIMIT 100;
fn q14751(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, favorite_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(favorite_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some((((t, s), w), fc)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5] + fc.unwrap_or(0)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| Reverse(a[3]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f
    })
}

// WITH UserEngagement AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.UserId = U.Id
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// CommentCount,
// UpVotes,
// DownVotes,
// TotalViews,
// TotalScore,
// LastPostDate
// FROM
// UserEngagement
// ORDER BY
// PostCount DESC, TotalScore DESC
// LIMIT 10;
fn q14757(db: &'static So) -> String {
    let own = self_votes(db);
    let Post { view_count, score, creation_date, .. } = &db.post;
    let uf = g(db)
        .select(posts_of(db).select(view_count.opt().and(score).and(creation_date).and(comments_of(db).opt()).and((&own).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 7], p| match p {
            Some(((((w, s), c), _), t)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + (t == Some(2)) as i64, a[5] + (t == Some(3)) as i64, a[6].max(c)],
            None => a,
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dc).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    out(v, |&(_, a, p, _)| (Reverse(p), (a[0] == 0, Reverse(a[1]))), 10, |&(u, a, p, c)| {
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(p),
            V::I(c),
            V::I(a[4]),
            V::I(a[5]),
            nullable(a[3], a[2]),
            nullable(a[1], a[0]),
            if a[0] == 0 { V::Null } else { V::T(a[6]) },
        ]
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
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
// ups.TotalPosts DESC
// LIMIT 10;
fn q14759(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| Reverse(a[0]), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), nullable(a[5], a[0])]);
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
// p.AnswerCount,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalPostScore
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
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.VoteCount,
// ps.CommentCount,
// us.UserId,
// us.DisplayName AS Author,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges,
// us.PostCount,
// us.TotalPostScore
// FROM
// PostStats ps
// JOIN
// Users u ON ps.OwnerUserId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q14770(db: &'static So) -> String {
    let us = g(db).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(&db.post.score).opt())).fold([0i64; 5], |a, (c, s)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + s.is_some() as i64, a[4] + s.unwrap_or(0)]
    });
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(s.vx), V::I(s.cx), user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&a[..4]));
        f.push(nullable(a[4], a[3]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 1 THEN 1 ELSE 0 END), 0) AS AcceptedCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.UpVoteCount) AS TotalUpVotes,
// SUM(ps.DownVoteCount) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.PostId
// GROUP BY
// u.Id, u.Reputation
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.BadgeCount,
// us.TotalComments,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// UserStats us
// ORDER BY
// us.Reputation DESC, us.BadgeCount DESC;
fn q14772(db: &'static So) -> String {
    let uid = uids(db);
    let pid = pids(db);
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[]);
    let uf = g(db).select(badges_of(db).opt().and((&db.user.origid).select(&pid).select(&pf).opt())).fold([0i64; 5], |a, (b, s)| match s {
        Some(s) => [a[0] + b.is_some() as i64, a[1] + 1, a[2] + s.cx, a[3] + s.up, a[4] + s.down],
        None => [a[0] + b.is_some() as i64, a[1], a[2], a[3], a[4]],
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a[0]), nullable(a[2], a[1]), nullable(a[3], a[1]), nullable(a[4], a[1])])))
}

// WITH UserBadges AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ),
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.CreationDate AS PostCreationDate,
// u.Reputation,
// ub.BadgeCount,
// p.PostTypeId,
// p.AcceptedAnswerId,
// p.AnswerCount,
// p.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// )
// SELECT
// p.PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.PostCreationDate,
// p.Reputation,
// p.BadgeCount,
// p.PostTypeId,
// p.AcceptedAnswerId,
// p.AnswerCount,
// p.CommentCount
// FROM
// PostDetails p
// ORDER BY
// p.Score DESC, p.ViewCount DESC
// LIMIT 100;
fn q14776(db: &'static So) -> String {
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned_since(db, year_ago()).select(Ident::<Post>::new().and((&db.post.owner_user).select(&bf).opt())).drive(|_, x| v.push(x));
    out(v, |&(p, _)| (score_views(db, p), db.post.origid.get(p).unwrap()), 100, |&(p, b)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "rep"]);
        f.push(oint(b));
        f.extend(post_fields(db, p, &["type_id", "accepted", "answers", "comments"]));
        f
    })
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users U
// LEFT JOIN Votes V ON U.Id = V.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.OwnerUserId,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// COUNT(C.Id) AS CommentCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY P.Id, P.Title, P.OwnerUserId, P.Score, P.ViewCount, P.AnswerCount
// ),
// PostVoteStats AS (
// SELECT
// P.Id AS PostId,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS PostUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS PostDownVotes
// FROM Posts P
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id
// )
// SELECT
// U.DisplayName,
// U.TotalVotes,
// U.UpVotes,
// U.DownVotes,
// P.PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// PV.PostUpVotes,
// PV.PostDownVotes
// FROM UserVoteStats U
// JOIN PostStats P ON U.UserId = P.OwnerUserId
// JOIN PostVoteStats PV ON P.PostId = PV.PostId
// ORDER BY U.TotalVotes DESC, P.Score DESC;
fn q14781(db: &'static So) -> String {
    let uv = user_votes(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db)
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&pv).opt()).and((&db.post.owner_user).select(Ident::<User>::new().and(&uv))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), x), (u, a))| {
        let x = x.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "answers"]));
        f.extend([V::I(c), V::I(x[1]), V::I(x[2])]);
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT OwnerUserId, COUNT(*) AS TotalPosts
// FROM Posts
// GROUP BY OwnerUserId
// ),
// UserVoteCounts AS (
// SELECT UserId, COUNT(*) AS TotalVotes
// FROM Votes
// GROUP BY UserId
// ),
// UserBadgeCounts AS (
// SELECT UserId, COUNT(*) AS TotalBadges
// FROM Badges
// GROUP BY UserId
// )
// SELECT
// Users.DisplayName,
// Users.Reputation,
// COALESCE(UserPostCounts.TotalPosts, 0) AS TotalPosts,
// COALESCE(UserVoteCounts.TotalVotes, 0) AS TotalVotes,
// COALESCE(UserBadgeCounts.TotalBadges, 0) AS TotalBadges
// FROM Users
// LEFT JOIN UserPostCounts ON Users.Id = UserPostCounts.OwnerUserId
// LEFT JOIN UserVoteCounts ON Users.Id = UserVoteCounts.UserId
// LEFT JOIN UserBadgeCounts ON Users.Id = UserBadgeCounts.UserId
// ORDER BY Users.Reputation DESC;
fn q14784(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).fold(0i64, |a, _| a + 1);
    let vc = votes_per_user(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pc).opt()).and(&vc).and(&bu)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, p), x), b)| row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(p.unwrap_or(0)), V::I(x), V::I(b)])))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostsCreated,
// COUNT(DISTINCT B.Id) AS BadgesEarned
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// U.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
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
// PS.VoteCount,
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.PostsCreated,
// US.BadgesEarned
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostId = U.Id
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.Score DESC,
// PS.ViewCount DESC
// LIMIT 100;
fn q14790(db: &'static So) -> String {
    let uid = uids(db);
    let y = year_ago();
    let w = UserWhere::CreatedGe(y);
    let dp = ud(db, w, posts_of(db));
    let dbg = ud(db, w, badges_of(db));
    let mut v = Vec::new();
    since(db, y)
        .with((&db.post.origid).select(&uid).select((&db.user.creation_date).ge(y)))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and((&dbg).opt()))))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| score_views(db, p), 100, |&(((p, c), x), ((u, d), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(x), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d.unwrap_or(0)), V::I(b.unwrap_or(0))]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(*) AS TotalPosts,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViews,
// MAX(p.CreationDate) AS LatestPostDate
// FROM
// Posts p
// GROUP BY
// p.PostTypeId
// ), UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostsCreated,
// SUM(p.Score) AS ScoreFromPosts,
// SUM(p.ViewCount) AS ViewsFromPosts
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// )
// SELECT
// pth.Id AS PostTypeId,
// pth.Name AS PostTypeName,
// ps.TotalPosts,
// ps.TotalScore,
// ps.TotalViews,
// ps.UniqueUsers,
// ps.AvgScore,
// ps.AvgViews,
// ps.LatestPostDate,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(DISTINCT PostId) FROM PostLinks) AS TotalLinks,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes
// FROM
// PostTypes pth
// LEFT JOIN
// PostStats ps ON pth.Id = ps.PostTypeId
// ORDER BY
// TotalPosts DESC;
fn q14793(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let ps = db.post.group_by(&db.post.post_type).select(score.and(view_count.opt()).and(creation_date)).fold([0, 0, 0, 0, i64::MIN], |a: [i64; 5], ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4].max(c)]
    });
    let du = db.post.group_by(&db.post.post_type).select(&db.post.owner_user_id).count_distinct();
    let nu = count(db.user.iq());
    let dl = one(whole(db.post_link.iq()).select(&db.post_link.post_id).count_distinct());
    let nv = count(db.vote.iq());
    let mut v = Vec::new();
    db.post_type.select(Ident::<PostType>::new().and((&ps).opt()).and((&du).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((t, a), d)| {
        let mut f = vec![V::I(db.post_type.origid.get(t).unwrap()), V::S(db.post_type.name.get(t).unwrap())];
        match a {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(d.unwrap_or(0)), avg(a[1], a[0]), avg(a[3], a[2]), V::T(a[4])]),
            None => f.extend(nulls(7)),
        }
        f.extend([V::I(nu), V::I(dl), V::I(nv)]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(v.BountyAmount) AS TotalBounty
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
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.TotalBounty,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts bc ON us.UserId = bc.UserId
// ORDER BY
// us.Reputation DESC, us.PostCount DESC
// LIMIT 100;
fn q14799(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a.n)), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(["#n", "#q", "#a", "bounty_sum"].iter().map(|c| ustat_field(&a, c)));
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers
// FROM
// Users
// ),
// PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AvgPostScore
// FROM
// Posts
// ),
// CommentStats AS (
// SELECT
// COUNT(*) AS TotalComments
// FROM
// Comments
// ),
// VoteStats AS (
// SELECT
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// COUNT(*) AS TotalVotes
// FROM
// Votes
// )
// SELECT
// (SELECT TotalUsers FROM UserStats) AS TotalUsers,
// (SELECT TotalPosts FROM PostStats) AS TotalPosts,
// (SELECT AvgPostScore FROM PostStats) AS AvgPostScore,
// (SELECT TotalComments FROM CommentStats) AS TotalComments,
// (SELECT TotalUpVotes FROM VoteStats) AS TotalUpVotes,
// (SELECT TotalDownVotes FROM VoteStats) AS TotalDownVotes,
// (SELECT TotalVotes FROM VoteStats) AS TotalVotes;
fn q14807(db: &'static So) -> String {
    let p = post_totals(db);
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    row(vec![V::I(count(db.user.iq())), V::I(p[0]), avg(p[1], p[0]), V::I(count(db.comment.iq())), V::I(x[1]), V::I(x[2]), V::I(x[0])])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// UserBadgeStats AS (
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
// ups.TotalScore,
// ups.AvgViewCount,
// ubs.TotalBadges,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges
// FROM UserPostStats ups
// LEFT JOIN UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY ups.TotalPosts DESC
// LIMIT 100;
fn q14809(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[5], a[0]), avg(a[4], a[3])]);
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f
    })
}

pub static ENTRIES: &[harness::Entry] = &[
    ("14466", q14466),
    ("14469", q14469),
    ("14476", q14476),
    ("14479", q14479),
    ("14486", q14486),
    ("14493", q14493),
    ("14499", q14499),
    ("14513", q14513),
    ("14515", q14515),
    ("14527", q14527),
    ("14528", q14528),
    ("14529", q14529),
    ("14536", q14536),
    ("14552", q14552),
    ("14555", q14555),
    ("14560", q14560),
    ("14581", q14581),
    ("14595", q14595),
    ("14603", q14603),
    ("14606", q14606),
    ("14616", q14616),
    ("14620", q14620),
    ("14633", q14633),
    ("14634", q14634),
    ("14635", q14635),
    ("14653", q14653),
    ("14660", q14660),
    ("14661", q14661),
    ("14671", q14671),
    ("14674", q14674),
    ("14676", q14676),
    ("14682", q14682),
    ("14689", q14689),
    ("14692", q14692),
    ("14693", q14693),
    ("14694", q14694),
    ("14700", q14700),
    ("14713", q14713),
    ("14717", q14717),
    ("14735", q14735),
    ("14751", q14751),
    ("14757", q14757),
    ("14759", q14759),
    ("14770", q14770),
    ("14772", q14772),
    ("14776", q14776),
    ("14781", q14781),
    ("14784", q14784),
    ("14790", q14790),
    ("14793", q14793),
    ("14799", q14799),
    ("14807", q14807),
    ("14809", q14809),
];
