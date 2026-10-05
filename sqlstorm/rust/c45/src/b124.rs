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

// --- batch 124 --------------------------------------------------------------

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(ba.BadgeCount, 0) AS BadgeCount
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
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) ba ON p.OwnerUserId = ba.UserId
// WHERE
// p.PostTypeId = 1
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// Score,
// AnswerCount,
// CommentCount,
// BadgeCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = ps.PostId) AS VoteCount
// FROM
// PostStats ps
// ORDER BY
// ViewCount DESC
// LIMIT 100;
fn q13850(db: &'static So) -> String {
    let bf = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.owner_user_id).select(&bf).opt()).and(votes_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| views_desc(db, p), 100, |&(((p, c), b), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.extend([V::I(c), V::I(b.unwrap_or(0)), V::I(x)]);
        f
    })
}

// WITH PostMetrics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// COALESCE(COUNT(C.Id), 0) AS CommentCount,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND (V.VoteTypeId = 8 OR V.VoteTypeId = 9)
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount
// ),
// UserMetrics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS TotalBadges,
// SUM(U.UpVotes) AS TotalUpVotes,
// SUM(U.DownVotes) AS TotalDownVotes,
// SUM(PM.Score) AS TotalPostScore
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// PostMetrics PM ON P.Id = PM.PostId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UM.UserId,
// UM.DisplayName,
// UM.TotalBadges,
// UM.TotalUpVotes,
// UM.TotalDownVotes,
// UM.TotalPostScore,
// PM.PostId,
// PM.Title,
// PM.CreationDate,
// PM.Score,
// PM.ViewCount,
// PM.AnswerCount,
// PM.CommentCount,
// PM.TotalBounty
// FROM
// UserMetrics UM
// JOIN
// PostMetrics PM ON UM.UserId = PM.PostId
// ORDER BY
// UM.TotalPostScore DESC, PM.Score DESC;
fn q13852(db: &'static So) -> String {
    let uid = uids(db);
    let pm = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[8, 9]);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt()).and(posts_of(db).select(&db.post.score).opt())).fold([0i64; 5], |a, (((u, d), b), s)| {
        [a[0] + b.is_some() as i64, a[1] + u, a[2] + d, a[3] + s.is_some() as i64, a[4] + s.unwrap_or(0)]
    });
    let mut v = Vec::new();
    db.post.with((&db.post.origid).select(&uid)).select(Ident::<Post>::new().and(&pm).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, s), (u, a))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3])];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]));
        f.extend([V::I(s.cx), V::I(s.bounty_sum)]);
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
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserVoteStats AS (
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
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(up.TotalPosts, 0) AS TotalPosts,
// COALESCE(up.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(up.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(up.TotalScore, 0) AS TotalScore,
// COALESCE(up.TotalViews, 0) AS TotalViews,
// COALESCE(uv.TotalVotes, 0) AS TotalVotes,
// COALESCE(uv.UpVotes, 0) AS UpVotes,
// COALESCE(uv.DownVotes, 0) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// UserPostStats up ON u.Id = up.UserId
// LEFT JOIN
// UserVoteStats uv ON u.Id = uv.UserId
// ORDER BY
// TotalPosts DESC, TotalScore DESC;
fn q13858(db: &'static So) -> String {
    let vs = vote_named(db);
    let mut v = Vec::new();
    upqa(db).and((&vs).opt()).drive(|u, (a, x)| v.push((u, a, x.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, a, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([V::I(a[5]), V::I(a[4])]);
        f.extend(ints(&x));
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
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.Reputation, u.DisplayName
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(COALESCE(b.Class, 0)) AS TotalBadges,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
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
// ps.OwnerReputation,
// ps.OwnerDisplayName,
// ps.TotalComments,
// us.TotalBadges,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerDisplayName = us.DisplayName
// ORDER BY
// ps.CreationDate DESC;
fn q13859(db: &'static So) -> String {
    let names = by_name(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.unwrap_or(0), a[1] + u, a[2] + d]);
    let c = per_post_distinct(db, comments_of(db));
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&c).opt()).and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(&us))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "rep", "owner"]);
        f.push(V::I(c.unwrap_or(0)));
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.Reputation
// ),
// UserVoteStats AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes v
// JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY v.UserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.CommentCount,
// u.BadgeCount,
// COALESCE(v.VoteCount, 0) AS TotalVotes,
// COALESCE(v.UpVotes, 0) AS UpVotes,
// COALESCE(v.DownVotes, 0) AS DownVotes
// FROM UserPostStats u
// LEFT JOIN UserVoteStats v ON u.UserId = v.UserId
// ORDER BY u.Reputation DESC
// LIMIT 100;
fn q13861(db: &'static So) -> String {
    let top: MatSet<Id<User>> = whole(user_base(db, UserWhere::All)).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc).filt(|(_, n)| n <= 100).map(|((u, _), _)| u).collect();
    let us = db.user.with(&top).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).opt()).opt().and(badges_of(db).opt())).fold([0i64; 3], |a, (p, b)| {
        [a[0] + p.is_some() as i64, a[1] + matches!(p, Some(Some(_))) as i64, a[2] + b.is_some() as i64]
    });
    let vs = vote_named(db);
    let mut v = Vec::new();
    (&us).and((&vs).opt()).drive(|u, (a, x)| v.push((u, a, x.unwrap_or([0; 3]))));
    out(v, |&(u, _, _)| rep_desc(db, u), 100, |&(u, a, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(ints(&x));
        f
    })
}

// SELECT
// pt.Name AS PostType,
// SUM(CASE WHEN p.Score IS NOT NULL THEN 1 ELSE 0 END) AS NumberOfPosts,
// AVG(p.ViewCount) AS AverageViewCount,
// AVG(COALESCE(votes.VoteCount, 0)) AS AverageVotes,
// AVG(COALESCE(badges.BadgeCount, 0)) AS AverageBadges,
// AVG(COALESCE(cmnt.CommentCount, 0)) AS AverageComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) votes ON p.Id = votes.PostId
// LEFT JOIN
// (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) badges ON p.OwnerUserId = badges.UserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) cmnt ON p.Id = cmnt.PostId
// GROUP BY
// pt.Name
// ORDER BY
// NumberOfPosts DESC;
fn q13867(db: &'static So) -> String {
    let bf = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let f = by_key(
        db.post.iq(),
        name(db),
        (&db.post.view_count).opt().and(votes_per_post(db)).and((&db.post.owner_user_id).select(&bf).opt()).and(comments_per_post(db)),
        [0i64; 6],
        |a, (((w, x), b), c)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + x, a[4] + b.unwrap_or(0), a[5] + c],
    );
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), avg(a[4], a[0]), avg(a[5], a[0])])))
}

// WITH UserPostActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// COUNT(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 END) AS VoteCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(*) AS TotalPosts
// FROM
// Users U
// LEFT JOIN
// Posts P ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON V.PostId = P.Id
// LEFT JOIN
// Comments C ON C.PostId = P.Id
// GROUP BY
// U.Id, U.DisplayName, P.Id, P.Title, P.CreationDate
// )
// SELECT
// U.DisplayName,
// U.Id AS UserId,
// COUNT(DISTINCT PA.PostId) AS PostsCreated,
// SUM(PA.VoteCount) AS TotalVotes,
// SUM(PA.CommentCount) AS TotalComments,
// MIN(PA.PostCreationDate) AS FirstPostDate,
// MAX(PA.PostCreationDate) AS LastPostDate
// FROM
// UserPostActivity PA
// JOIN
// Users U ON U.Id = PA.UserId
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// TotalVotes DESC, PostsCreated DESC;
fn q13869(db: &'static So) -> String {
    let uf = g(db)
        .select(posts_of(db).select((&db.post.creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt())
        .fold([0, 0, i64::MAX, i64::MIN], |a: [i64; 4], p| match p {
            Some(((cd, t), c)) => [a[0] + matches!(t, Some(2 | 3)) as i64, a[1] + c.is_some() as i64, a[2].min(cd), a[3].max(cd)],
            None => a,
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).drive(|u, (a, d)| v.push((u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, d)| {
        let t = |x: i64| if d == 0 { V::Null } else { V::T(x) };
        row(vec![user_col(db, u, "name"), user_col(db, u, "uid"), V::I(d), V::I(a[0]), V::I(a[1]), t(a[2]), t(a[3])])
    }))
}

// WITH PostStats AS (
// SELECT
// Posts.Id AS PostId,
// Posts.Title,
// Posts.PostTypeId,
// Posts.CreationDate,
// Posts.Score,
// Posts.ViewCount,
// COALESCE(Answers.Count, 0) AS AnswerCount,
// COALESCE(Comments.Count, 0) AS CommentCount,
// COALESCE(Votes.Count, 0) AS VoteCount
// FROM
// Posts
// LEFT JOIN (
// SELECT
// ParentId,
// COUNT(*) AS Count
// FROM
// Posts
// WHERE
// PostTypeId = 2
// GROUP BY
// ParentId
// ) AS Answers ON Posts.Id = Answers.ParentId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS Count
// FROM
// Comments
// GROUP BY
// PostId
// ) AS Comments ON Posts.Id = Comments.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS Count
// FROM
// Votes
// GROUP BY
// PostId
// ) AS Votes ON Posts.Id = Votes.PostId
// )
// SELECT
// PostStats.Title,
// PostStats.CreationDate,
// PostStats.Score,
// PostStats.ViewCount,
// PostStats.AnswerCount,
// PostStats.CommentCount,
// PostStats.VoteCount,
// PostHistoryType.Name AS LastEditType
// FROM
// PostStats
// LEFT JOIN
// PostHistory ON PostStats.PostId = PostHistory.PostId
// LEFT JOIN
// PostHistoryTypes AS PostHistoryType ON PostHistory.PostHistoryTypeId = PostHistoryType.Id
// WHERE
// PostStats.PostTypeId = 1
// ORDER BY
// PostStats.CreationDate DESC
// LIMIT 100;
fn q13878(db: &'static So) -> String {
    let mut v = Vec::new();
    questions_only(db)
        .select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and(votes_per_post(db)).and(history_of(db).select((&db.post_history.post_history_type).select(&db.post_history_type.name)).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| newest(db, p), 100, |&((((p, a), c), x), n)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(a), V::I(c), V::I(x), ostr(n)]);
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
// SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis,
// SUM(CASE WHEN P.PostTypeId = 2 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswersCount,
// AVG(P.Score) AS AverageScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.TotalPosts,
// U.TotalQuestions,
// U.TotalAnswers,
// U.TotalWikis,
// U.AcceptedAnswersCount,
// U.AverageScore,
// U.TotalViews,
// B.Name AS BadgeName,
// B.Class AS BadgeClass
// FROM
// UserPostStats U
// LEFT JOIN
// Badges B ON U.UserId = B.UserId
// ORDER BY
// U.TotalPosts DESC, U.AverageScore DESC;
fn q13892(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, accepted_answer_id, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(accepted_answer_id.opt())).opt()).fold([0i64; 7], |a, p| match p {
        Some((((t, s), w), ac)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + (t == 2 && ac.is_some()) as i64, a[5] + s, a[6] + w.unwrap_or(0)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).and(badges_of(db).select((&db.badge.name).and(&db.badge.class)).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..5]));
        f.extend([avg(a[5], a[0]), V::I(a[6]), ostr(b.map(|b| b.0)), oint(b.map(|b| b.1))]);
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
// p.FavoriteCount,
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentTotal,
// COUNT(v.Id) AS VoteTotal
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Reputation, u.DisplayName
// ),
// PostHistoryCount AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS HistoryCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
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
// ps.OwnerReputation,
// ps.OwnerDisplayName,
// phc.HistoryCount,
// phc.LastEditDate
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryCount phc ON ps.PostId = phc.PostId
// ORDER BY
// ps.CreationDate DESC;
fn q13907(db: &'static So) -> String {
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and((&hf).opt()).drive(|p, (_, h)| v.push((p, h)));
    rows(v.iter().map(|&(p, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "rep", "owner"]);
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// WITH PostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation
// FROM
// Users u
// )
// SELECT
// ur.UserId,
// ur.Reputation,
// pc.TotalPosts,
// pc.TotalQuestions,
// pc.TotalAnswers,
// pc.TotalWikis
// FROM
// UserReputation ur
// LEFT JOIN
// PostCounts pc ON ur.UserId = pc.OwnerUserId
// ORDER BY
// ur.Reputation DESC,
// pc.TotalPosts DESC;
fn q13911(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pc).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend((0..4).map(|i| oint(p.map(|p| p[i]))));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// AVG(COALESCE(v.BountyAmount, 0)) AS AvgBountyAmount,
// MAX(p.CreationDate) AS LastActivityDate,
// COUNT(DISTINCT ph.Id) AS HistoryCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// PostHistory ph ON ph.PostId = p.Id
// GROUP BY
// p.Id, p.Title
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// AVG(u.Reputation) AS AvgReputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.Views) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Badges b ON b.UserId = u.Id
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CommentCount,
// ps.VoteCount,
// ps.AvgBountyAmount,
// ps.LastActivityDate,
// ps.HistoryCount,
// us.UserId,
// us.DisplayName,
// us.AvgReputation,
// us.BadgeCount,
// us.TotalViews
// FROM
// PostStats ps
// JOIN
// Users u ON u.Id = ps.PostId
// JOIN
// UserStats us ON us.UserId = u.Id
// ORDER BY
// ps.LastActivityDate DESC;
fn q13912(db: &'static So) -> String {
    let uid = uids(db);
    let x = per_post_distinct(db, votes_of(db));
    let h = per_post_distinct(db, history_of(db));
    let us = g(db).select((&db.user.reputation).and(&db.user.views).and(badges_of(db).opt())).fold([0i64; 4], |a, ((r, w), b)| [a[0] + 1, a[1] + r, a[2] + b.is_some() as i64, a[3] + w]);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvh", &[])
        .and((&x).opt())
        .and((&h).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (((s, x), h), (u, a))| v.push((p, s, x.unwrap_or(0), h.unwrap_or(0), u, a)));
    rows(v.iter().map(|&(p, s, x, h, u, a)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s.cx), V::I(x), stat_field(&s, "bounty0_avg").unwrap()]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([V::I(h), user_col(db, u, "uid"), user_col(db, u, "name"), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AvgScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS TotalEdits,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TitleEdits,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (10) THEN 1 ELSE 0 END) AS PostClosedCount,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (11) THEN 1 ELSE 0 END) AS PostReopenedCount
// FROM
// PostHistory ph
// GROUP BY
// ph.UserId
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalTagWikis,
// ups.TotalViews,
// ups.AvgScore,
// phs.TotalEdits,
// phs.TitleEdits,
// phs.PostClosedCount,
// phs.PostReopenedCount
// FROM
// Users u
// LEFT JOIN
// UserPostStats ups ON u.Id = ups.UserId
// LEFT JOIN
// PostHistoryStats phs ON u.Id = phs.UserId
// ORDER BY
// u.Reputation DESC;
fn q13913(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)).opt()).fold([0i64; 7], |a, p| match p {
        Some(((t, w), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s],
        None => a,
    });
    let hf = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 4], |a, t| {
        [a[0] + 1, a[1] + matches!(t, 4 | 5) as i64, a[2] + (t == 10) as i64, a[3] + (t == 11) as i64]
    });
    let mut v = Vec::new();
    (&uf).and((&hf).opt()).drive(|u, (a, h)| v.push((u, a, h)));
    rows(v.iter().map(|&(u, a, h)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated")];
        f.extend(ints(&a[..4]));
        f.extend([nullable(a[5], a[4]), avg(a[6], a[0])]);
        f.extend((0..4).map(|i| oint(h.map(|h| h[i]))));
        row(f)
    }))
}

// SELECT
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// AVG(u.Reputation) AS AverageReputation,
// AVG(COALESCE(c.CommentCount, 0)) AS AverageCommentCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2020-01-01'
// AND p.CreationDate < '2023-10-01';
fn q13921(db: &'static So) -> String {
    let base = || db.post.with((&db.post.creation_date).ge(date(2020, 1, 1))).with((&db.post.creation_date).lt(date(2023, 10, 1)));
    let a = base().select((&db.post.owner_user).select(&db.user.reputation).opt().and(votes_of(db).opt()).and(comments_per_post(db))).fold_flat([0i64; 4], |a, ((r, _), c)| {
        [a[0] + r.is_some() as i64, a[1] + r.unwrap_or(0), a[2] + 1, a[3] + c]
    });
    let dp = one(whole(base()).select(Ident::<Post>::new()).count_distinct());
    let du = one(whole(base()).select(&db.post.owner_user).count_distinct());
    let dv = one(whole(base()).select(votes_of(db)).count_distinct());
    row(vec![V::I(dp), V::I(du), V::I(dv), avg(a[1], a[0]), avg(a[3], a[2])])
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
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
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// PostTypeStats AS (
// SELECT
// pt.Id AS PostTypeId,
// pt.Name,
// COUNT(p.Id) AS PostCount,
// AVG(ps.Score) AS AvgScore,
// AVG(ps.ViewCount) AS AvgViews,
// AVG(ps.CommentCount) AS AvgComments,
// AVG(ps.VoteCount) AS AvgVotes
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON pt.Id = p.PostTypeId
// LEFT JOIN
// PostStats ps ON p.Id = ps.PostId
// GROUP BY
// pt.Id, pt.Name
// )
// SELECT
// pts.Name AS PostType,
// pts.PostCount,
// pts.AvgScore,
// pts.AvgViews,
// pts.AvgComments,
// pts.AvgVotes,
// COALESCE(us.TotalUpVotes, 0) AS TotalUpVotes,
// COALESCE(us.TotalDownVotes, 0) AS TotalDownVotes,
// us.BadgeCount AS UserBadgeCount
// FROM
// PostTypeStats pts
// LEFT JOIN
// UserStats us ON us.UserId = (SELECT MIN(Id) FROM Users)
// ORDER BY
// pts.PostCount DESC;
fn q13923(db: &'static So) -> String {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let pf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let pt = db
        .post_type
        .group_by(Ident::<PostType>::new())
        .select((&of_type).select((&db.post.score).and((&db.post.view_count).opt()).and(&pf)).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((s, w), st)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + st.cx, a[5] + st.vx],
            None => a,
        });
    let umin = db.user.select(&db.user.origid).fold_flat(i64::MAX, |m, x| m.min(x));
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt())).fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let mu: HashIdx<(), Id<User>> = db.user.with((&db.user.origid).eq(umin)).map(|_| ()).inv().collect();
    let mut v = Vec::new();
    (&pt).and(Ident::<PostType>::new().map(|_| ()).select(&mu).select(&us).opt()).drive(|t, (a, ua)| v.push((t, a, ua)));
    rows(v.iter().map(|&(t, a, ua)| {
        row(vec![V::S(db.post_type.name.get(t).unwrap()), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), avg(a[4], a[0]), avg(a[5], a[0]), V::I(ua.map_or(0, |x| x[1])), V::I(ua.map_or(0, |x| x[2])), oint(ua.map(|x| x[0]))])
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
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(A.Id) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName
// )
// SELECT
// *,
// (UpVotes - DownVotes) AS NetVotes
// FROM
// PostStatistics
// ORDER BY
// Score DESC, ViewCount DESC
// LIMIT 100;
fn q13933(db: &'static So) -> String {
    out(stats_with(db, questions_only(db), "cav", &[], &[]), |&(p, _, _)| score_views(db, p), 100, |&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(s.cx), V::I(s.ax), V::I(s.up), V::I(s.down), V::I(s.up - s.down)]);
        f
    })
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount,
// COALESCE(AC.AnswerCount, 0) AS AnswerCount,
// COALESCE(C.CommentCount, 0) AS CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT
// ParentId,
// COUNT(*) AS AnswerCount
// FROM
// Posts
// WHERE
// PostTypeId = 2
// GROUP BY
// ParentId) AC ON P.Id = AC.ParentId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= '2022-01-01'
// ORDER BY
// P.Score DESC, P.ViewCount DESC;
fn q13938(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, date(2022, 1, 1)).select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, a), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.extend([V::I(a), V::I(c)]);
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
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserBadgeCount AS (
// SELECT
// UserId,
// COUNT(Id) AS TotalBadges
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalQuestions,
// UPS.TotalAnswers,
// UPS.TotalViews,
// UPS.AverageScore,
// COALESCE(UBC.TotalBadges, 0) AS TotalBadges
// FROM
// UserPostStats UPS
// LEFT JOIN
// UserBadgeCount UBC ON UPS.UserId = UBC.UserId
// ORDER BY
// UPS.TotalPosts DESC
// LIMIT 100;
fn q13940(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s],
        None => a,
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), avg(a[5], a[0]), V::I(b)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// Posts.Id AS PostId,
// Posts.Title,
// Posts.CreationDate,
// Posts.ViewCount,
// Posts.Score,
// Users.Reputation AS OwnerReputation,
// COUNT(DISTINCT Comments.Id) AS CommentCount,
// COUNT(DISTINCT Votes.Id) AS VoteCount
// FROM
// Posts
// LEFT JOIN
// Users ON Posts.OwnerUserId = Users.Id
// LEFT JOIN
// Comments ON Posts.Id = Comments.PostId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// GROUP BY
// Posts.Id, Posts.Title, Posts.CreationDate, Posts.ViewCount, Posts.Score, Users.Reputation
// ),
// PostHistoryStats AS (
// SELECT
// PostId,
// COUNT(*) AS HistoryCount,
// MAX(CreationDate) AS LastEdited
// FROM
// PostHistory
// GROUP BY
// PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.OwnerReputation,
// ps.CommentCount,
// ps.VoteCount,
// COALESCE(phs.HistoryCount, 0) AS PostHistoryCount,
// phs.LastEdited
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC;
fn q13945(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and((&hf).opt())).drive(|_, y| v.push(y));
    rows(v.iter().map(|&(((p, c), x), h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "rep"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0)), V::I(h.map_or(0, |h| h.0)), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// WITH UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.Reputation
// ),
// AveragePostAge AS (
// SELECT
// u.Id AS UserId,
// AVG(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate))/86400) AS AveragePostAgeInDays
// FROM Users u
// JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id
// )
// SELECT
// up.UserId,
// up.Reputation,
// up.TotalPosts,
// up.TotalQuestions,
// up.TotalAnswers,
// up.TotalUpVotes,
// up.TotalDownVotes,
// apa.AveragePostAgeInDays
// FROM UserPerformance up
// JOIN AveragePostAge apa ON up.UserId = apa.UserId
// ORDER BY up.Reputation DESC;
fn q13946(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let age = owned(db).group_by(&db.post.owner_user).select(&db.post.creation_date).fold((0i64, 0i128), |(n, s), c| (n + 1, s + (t0 - c) as i128));
    let mut v = Vec::new();
    (&us).and(&age).drive(|u, (a, g)| v.push((u, a, g)));
    rows(v.iter().map(|&(u, a, (n, s))| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a.n), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down), V::F(s as f64 / n as f64 / 1e6 / 86400.0)])
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
// P.CommentCount,
// U.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id) AS VoteCount,
// (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS TotalComments
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// ),
// PostHistoryCount AS (
// SELECT
// PH.PostId,
// COUNT(*) AS EditCount
// FROM
// PostHistory PH
// WHERE
// PH.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// PH.PostId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.OwnerDisplayName,
// PS.VoteCount,
// PS.TotalComments,
// COALESCE(PHC.EditCount, 0) AS EditCount
// FROM
// PostStats PS
// LEFT JOIN
// PostHistoryCount PHC ON PS.PostId = PHC.PostId
// ORDER BY
// PS.Score DESC,
// PS.ViewCount DESC
// LIMIT 100;
fn q13955(db: &'static So) -> String {
    let ed = history_of_types(db, &[4, 5, 6]);
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and(votes_per_post(db)).and(comments_per_post(db)).and(&ed)).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| score_views(db, p), 100, |&(((p, x), c), e)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::I(x), V::I(c), V::I(e)]);
        f
    })
}

// WITH UserPostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS PostCount,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers
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
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.CreationDate,
// u.LastAccessDate,
// u.Views,
// p.PostCount,
// p.Questions,
// p.Answers,
// r.Reputation
// FROM
// UserPostCounts p
// JOIN
// UserReputation r ON p.OwnerUserId = r.UserId
// JOIN
// Users u ON u.Id = p.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// CreationDate,
// LastAccessDate,
// Views,
// PostCount,
// Questions,
// Answers
// FROM
// UserActivity
// ORDER BY
// Reputation DESC,
// PostCount DESC;
fn q13959(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let mut v = Vec::new();
    (&pc).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let mut f: Vec<V> = ["uid", "name", "rep", "ucreated", "last_access", "uviews"].iter().map(|c| user_col(db, u, c)).collect();
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
// SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
// SUM(P.ViewCount) AS TotalViews,
// SUM(V.BountyAmount) AS TotalBounty
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
// TotalPosts,
// TotalComments,
// PositivePosts,
// NegativePosts,
// TotalViews,
// TotalBounty
// FROM
// UserActivity
// ORDER BY
// TotalPosts DESC, TotalViews DESC;
fn q13964(db: &'static So) -> String {
    let own = self_votes(db);
    let uf = g(db)
        .select(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt()).and(comments_of(db).opt()).and((&own).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((((s, w), _), b)) => {
                let b = b.flatten();
                [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0)]
            }
            None => a,
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dc).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, p, c)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(c), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), nullable(a[5], a[4])])))
}

// WITH Metrics AS (
// SELECT
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM Posts p
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// )
// SELECT
// TotalPosts,
// TotalUsers,
// TotalComments,
// TotalVotes,
// (SELECT COUNT(*) FROM PostHistory) AS TotalPostHistoryRecords,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges,
// (SELECT COUNT(*) FROM Tags) AS TotalTags,
// (SELECT COUNT(*) FROM PostLinks) AS TotalPostLinks
// FROM Metrics;
fn q13966(db: &'static So) -> String {
    let dp = one(whole(db.post.iq()).select(Ident::<Post>::new()).count_distinct());
    let du = one(whole(db.post.iq()).select(&db.post.owner_user).count_distinct());
    let dc = one(whole(db.post.iq()).select(comments_of(db)).count_distinct());
    let dv = one(whole(db.post.iq()).select(votes_of(db)).count_distinct());
    row(vec![V::I(dp), V::I(du), V::I(dc), V::I(dv), V::I(count(db.post_history.iq())), V::I(count(db.badge.iq())), V::I(count(db.tag.iq())), V::I(count(db.post_link.iq()))])
}

// WITH UserPosts AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// COUNT(Posts.Id) AS PostCount,
// SUM(COALESCE(Posts.Score, 0)) AS TotalScore,
// SUM(COALESCE(Posts.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(Posts.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(Posts.AnswerCount, 0)) AS TotalAnswers
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.Id,
// Users.DisplayName
// ),
// UserBadges AS (
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
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(P.PostCount, 0) AS PostCount,
// COALESCE(P.TotalScore, 0) AS TotalScore,
// COALESCE(P.TotalViews, 0) AS TotalViews,
// COALESCE(P.TotalComments, 0) AS TotalComments,
// COALESCE(P.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(B.BadgeCount, 0) AS BadgeCount,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// UserPosts P ON U.Id = P.UserId
// LEFT JOIN
// UserBadges B ON U.Id = B.UserId
// ORDER BY
// TotalScore DESC,
// PostCount DESC
// LIMIT 100;
fn q13970(db: &'static So) -> String {
    let Post { score, view_count, comment_count, answer_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(score.and(view_count.opt()).and(comment_count).and(answer_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((((s, w), cc), an)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + cc, a[4] + an.unwrap_or(0)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| (Reverse(a[1]), Reverse(a[0])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f
    })
}

// WITH UserPosts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
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
// U.PostCount,
// U.TotalViews,
// U.TotalScore,
// U.QuestionCount,
// U.AnswerCount,
// COALESCE(B.BadgeCount, 0) AS BadgeCount,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPosts U
// LEFT JOIN
// UserBadges B ON U.UserId = B.UserId
// ORDER BY
// U.TotalScore DESC, U.PostCount DESC
// LIMIT 100;
fn q13975(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| (Reverse(a[5]), Reverse(a[0])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[4]), V::I(a[5]), V::I(a[1]), V::I(a[2])];
        f.extend(ints(&b));
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// u.DisplayName AS OwnerDisplayName,
// pt.Name AS PostTypeName,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id
// WHERE
// p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, pt.Name, t.TagName
// ORDER BY
// p.Score DESC, p.CreationDate DESC
// LIMIT 100;
fn q13977(db: &'static So) -> String {
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let key = Ident::<Post>::new().and((&ex).select(&db.tag.tag_name).opt());
    let base = || owned_since(db, year_ago());
    let f = base()
        .group_by(key)
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user).select(badges_of(db)).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |&((p, _), _)| (score_desc(db, p), newest(db, p)), 100, |&((p, t), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["owner", "type"]));
        f.extend([V::I((&b).get(p).unwrap_or(0)), ostr(t)]);
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(COALESCE(V.VoteCount, 0)) AS TotalVotes,
// SUM(COALESCE(C.CommentCount, 0)) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) V ON P.Id = V.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.Reputation
// )
// SELECT
// UserId,
// Reputation,
// PostCount,
// BadgeCount,
// TotalVotes,
// TotalComments
// FROM
// UserStats
// WHERE
// PostCount > 0
// ORDER BY
// Reputation DESC, TotalVotes DESC
// LIMIT 100;
fn q13979(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let uf = g(db).select(posts_of(db).select(votes_per_post(db).and(comments_per_post(db))).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (p, _)| match p {
        Some((x, c)) => [a[0] + x, a[1] + c],
        None => a,
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&uf).and((&dp).filt(|d: i64| d > 0)).and(&bu).drive(|u, ((a, d), b)| v.push((u, a, d, b)));
    out(v, |&(u, a, _, _)| (rep_desc(db, u), Reverse(a[0])), 100, |&(u, a, d, b)| vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d), V::I(b), V::I(a[0]), V::I(a[1])])
}

// WITH UserPostCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(P.Id) AS PostCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id
// ),
// TopUsers AS (
// SELECT
// UserId,
// PostCount
// FROM
// UserPostCounts
// ORDER BY
// PostCount DESC
// LIMIT 10
// ),
// PostVoteCounts AS (
// SELECT
// P.Id AS PostId,
// COUNT(V.Id) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id
// )
// SELECT
// U.DisplayName,
// UPC.PostCount,
// PVC.VoteCount
// FROM
// Users U
// JOIN
// TopUsers UPC ON U.Id = UPC.UserId
// LEFT JOIN
// PostVoteCounts PVC ON UPC.UserId = PVC.PostId
// ORDER BY
// UPC.PostCount DESC;
fn q13984(db: &'static So) -> String {
    let pid = pids(db);
    let pc = g(db).select(posts_of(db).opt()).fold(0i64, |a, p| a + p.is_some() as i64);
    let top: MatSet<Id<User>> = whole(&pc).select(Same::new().and(&pc)).window(row_number, |(_, n)| n, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&pc).and((&db.user.origid).select(&pid).select(votes_per_post(db)).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, n), x)| row(vec![user_col(db, u, "name"), V::I(n), oint(x)])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM Users AS u
// LEFT JOIN Badges AS b ON u.Id = b.UserId
// LEFT JOIN Votes AS v ON u.Id = v.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount
// FROM Posts AS p
// GROUP BY p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.UpVotes,
// us.DownVotes,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount
// FROM UserStats AS us
// LEFT JOIN PostStats AS ps ON us.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// UpVotes,
// DownVotes,
// PostCount,
// TotalScore,
// TotalViews,
// QuestionCount,
// AnswerCount
// FROM CombinedStats
// ORDER BY TotalScore DESC
// LIMIT 100;
fn q13990(db: &'static So) -> String {
    let us = g(db).select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (b, t)| [a[0] + b.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let Post { score, view_count, post_type_id, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(post_type_id)).fold([0i64; 5], |a, ((s, w), t)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + (t == 1) as i64, a[4] + (t == 2) as i64]);
    let mut v = Vec::new();
    (&us).and((&pf).opt()).drive(|u, (a, p)| v.push((u, a, p.unwrap_or([0; 5]))));
    out(v, |&(_, _, p)| Reverse(p[1]), 100, |&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&p));
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
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.DisplayName
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.PostCount,
// UA.QuestionCount,
// UA.AnswerCount,
// UA.CommentCount,
// UA.UpVotes,
// UA.DownVotes,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate
// FROM UserActivity UA
// JOIN Users U ON UA.UserId = U.Id
// ORDER BY UA.PostCount DESC
// LIMIT 100;
fn q14000(db: &'static So) -> String {
    out(users_with_counts(db, "cv", false), |r| Reverse(r.agg.n), 100, |r| user_fields(r, "cv", &["uid", "name", "#n", "#q", "#a", "#cx", "#up", "#down", "rep", "ucreated", "last_access"]))
}

// WITH UserPostDetails AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.Score AS PostScore,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, P.Id, P.Title, P.CreationDate, P.Score
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// COUNT(DISTINCT PostId) AS TotalPosts,
// SUM(PostScore) AS TotalPostScore,
// SUM(CommentCount) AS TotalComments,
// SUM(VoteCount) AS TotalVotes,
// SUM(BadgeCount) AS TotalBadges
// FROM
// UserPostDetails
// GROUP BY
// UserId, DisplayName, Reputation
// ORDER BY
// TotalPosts DESC, TotalPostScore DESC
// LIMIT 100;
fn q14002(db: &'static So) -> String {
    let w = UserWhere::RepGt(1000);
    let bu = badges_per_user(db);
    let uf = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold(0i64, |n, (p, _)| n + p.map_or(0, |(c, _)| c.is_some() as i64));
    let pf = user_base(db, w).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.score).and(votes_per_post(db))).opt().and(&bu)).fold([0i64; 4], |a, (p, b)| match p {
        Some((s, x)) => [a[0] + 1, a[1] + s, a[2] + x, a[3] + b],
        None => [a[0], a[1], a[2], a[3] + b],
    });
    let mut v = Vec::new();
    (&pf).and(&uf).drive(|u, (a, c)| v.push((u, a, c)));
    out(v, |&(_, a, _)| (Reverse(a[0]), a[0] == 0, Reverse(a[1])), 100, |&(u, a, c)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), nullable(a[1], a[0]), V::I(c), V::I(a[2]), V::I(a[3])]
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(A.Id) AS AnswerCount,
// SUM(V.BountyAmount) AS TotalBounty
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON A.ParentId = P.Id AND A.PostTypeId = 2
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
// GROUP BY
// P.Id, P.PostTypeId, P.Title, P.CreationDate, P.Score, P.ViewCount
// )
// SELECT
// PST.PostId,
// PST.Title,
// PST.CreationDate,
// PST.PostTypeId,
// PST.Score,
// PST.ViewCount,
// PST.CommentCount,
// PST.AnswerCount,
// PST.TotalBounty,
// U.Reputation,
// U.DisplayName AS OwnerDisplayName
// FROM
// PostStats PST
// JOIN
// Users U ON PST.PostTypeId = 1 AND PST.PostId = U.AccountId
// ORDER BY
// PST.Score DESC,
// PST.ViewCount DESC
// LIMIT 100;
fn q14004(db: &'static So) -> String {
    let acc: HashIdx<i64, Id<User>> = (&db.user.account_id).inv().collect();
    let mut v = Vec::new();
    stats_fold(db, questions_only(db).with((&db.post.origid).select(&acc)), Ident::<Post>::new(), "cAv", &[8, 9])
        .and((&db.post.origid).select(&acc))
        .drive(|p, (s, u)| v.push((p, s, u)));
    out(v, |&(p, _, _)| score_views(db, p), 100, |&(p, s, u)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "type_id", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.ax), stat_field(&s, "bounty_sum").unwrap(), user_col(db, u, "rep"), user_col(db, u, "name")]);
        f
    })
}

// WITH PostSummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// U.Reputation AS OwnerReputation,
// U.DisplayName AS OwnerDisplayName,
// COUNT(V.Id) AS VoteCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= DATE '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, U.Reputation, U.DisplayName
// ),
// TopPosts AS (
// SELECT
// PostId, Title, Score, ViewCount, AnswerCount, CommentCount, VoteCount, OwnerReputation, OwnerDisplayName
// FROM
// PostSummary
// ORDER BY
// Score DESC, ViewCount DESC
// LIMIT 10
// )
// SELECT
// TP.PostId,
// TP.Title,
// TP.Score,
// TP.ViewCount,
// TP.AnswerCount,
// TP.CommentCount,
// TP.VoteCount,
// TP.OwnerReputation,
// TP.OwnerDisplayName
// FROM
// TopPosts TP
// ORDER BY
// TP.Score DESC;
fn q14007(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(votes_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(p, _)| score_views(db, p), 10, |&(p, x)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "comments"]);
        f.push(V::I(x));
        f.extend(post_fields(db, p, &["rep", "owner"]));
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// Posts.Id AS PostId,
// Posts.Title,
// Posts.CreationDate,
// Posts.ViewCount,
// Posts.Score,
// Users.Reputation AS OwnerReputation,
// COUNT(Comments.Id) AS CommentCount,
// COUNT(Votes.Id) AS VoteCount,
// Posts.OwnerUserId  -- Added OwnerUserId to the group by clause
// FROM
// Posts
// LEFT JOIN
// Users ON Posts.OwnerUserId = Users.Id
// LEFT JOIN
// Comments ON Posts.Id = Comments.PostId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// GROUP BY
// Posts.Id, Posts.Title, Posts.CreationDate, Posts.ViewCount, Posts.Score, Users.Reputation, Posts.OwnerUserId
// ),
// BadgeStatistics AS (
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
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.OwnerReputation,
// PS.CommentCount,
// PS.VoteCount,
// COALESCE(BS.BadgeCount, 0) AS OwnerBadgeCount
// FROM
// PostStatistics PS
// LEFT JOIN
// BadgeStatistics BS ON PS.OwnerUserId = BS.UserId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC
// LIMIT 100;
fn q14015(db: &'static So) -> String {
    let bu = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user_id).select(&bu).opt()).drive(|p, (s, b)| v.push((p, s, b)));
    out(v, |&(p, _, _)| score_views(db, p), 100, |&(p, s, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "rep"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(b.unwrap_or(0))]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// U.Reputation AS OwnerReputation,
// P.CreationDate,
// P.LastActivityDate,
// P.ViewCount,
// P.Score,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId
// GROUP BY
// P.Id, P.PostTypeId, U.Reputation, P.CreationDate, P.LastActivityDate, P.ViewCount, P.Score
// ),
// PostHistoryStats AS (
// SELECT
// PH.PostId,
// MAX(PH.CreationDate) AS LastEditedDate,
// COUNT(PH.Id) AS EditCount
// FROM
// PostHistory PH
// GROUP BY
// PH.PostId
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.OwnerReputation,
// PS.CreationDate,
// PS.LastActivityDate,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.AnswerCount,
// PHS.LastEditedDate,
// PHS.EditCount
// FROM
// PostStats PS
// LEFT JOIN
// PostHistoryStats PHS ON PS.PostId = PHS.PostId
// ORDER BY
// PS.LastActivityDate DESC
// LIMIT 100;
fn q14019(db: &'static So) -> String {
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "ca", &[]).and((&hf).opt()).drive(|p, (s, h)| v.push((p, s, h)));
    out(v, |&(p, _, _)| (Reverse(db.post.last_activity_date.get(p).unwrap()), db.post.origid.get(p).unwrap()), 100, |&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "type_id", "rep", "created", "activity", "views", "score"]);
        f.extend([V::I(s.cx), V::I(s.ax), ots(h.map(|h| h.1)), oint(h.map(|h| h.0))]);
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.CreationDate,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(COALESCE(P.Score, 0)) AS TotalScore
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.Reputation, U.CreationDate, U.DisplayName
// ),
// PostActivity AS (
// SELECT
// P.Id AS PostId,
// P.OwnerUserId,
// P.PostTypeId,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id, P.OwnerUserId, P.PostTypeId
// ),
// PostHistoryStats AS (
// SELECT
// PH.PostId,
// COUNT(PH.Id) AS EditCount,
// COUNT(DISTINCT PH.UserId) AS UniqueEditors
// FROM PostHistory PH
// GROUP BY PH.PostId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.BadgeCount,
// U.TotalViews,
// U.TotalPosts,
// U.TotalScore,
// PA.CommentCount,
// PA.VoteCount,
// COALESCE(PH.EditCount, 0) AS EditCount,
// COALESCE(PH.UniqueEditors, 0) AS UniqueEditors
// FROM UserStats U
// LEFT JOIN PostActivity PA ON U.UserId = PA.OwnerUserId
// LEFT JOIN PostHistoryStats PH ON PA.PostId = PH.PostId
// ORDER BY U.Reputation DESC, U.TotalScore DESC;
fn q14021(db: &'static So) -> String {
    let us = g(db).select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)).opt())).fold([0i64; 3], |a, (b, p)| {
        let (w, s) = p.map_or((0, 0), |(w, s)| (w.unwrap_or(0), s));
        [a[0] + b.is_some() as i64, a[1] + w, a[2] + s]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pa = stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]);
    let hc = history_per_post(db);
    let de = per_post_distinct(db, history_of(db).select(&db.post_history.user_id));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(posts_of(db).select((&pa).and(&hc).and((&de).opt())).opt()).drive(|u, ((a, d), p)| v.push((u, a, d.unwrap_or(0), p)));
    rows(v.iter().map(|&(u, a, d, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), V::I(a[1]), V::I(d), V::I(a[2])];
        match p {
            Some(((s, h), e)) => f.extend([V::I(s.cx), V::I(s.vx), V::I(h), V::I(e.unwrap_or(0))]),
            None => f.extend([V::Null, V::Null, V::I(0), V::I(0)]),
        }
        row(f)
    }))
}

// WITH PostsAggregated AS (
// SELECT
// pst.OwnerUserId,
// COUNT(pst.Id) AS PostCount,
// SUM(pst.ViewCount) AS TotalViews,
// SUM(pst.Score) AS TotalScore,
// AVG(pst.Score) AS AvgScore
// FROM
// Posts pst
// WHERE
// pst.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// pst.OwnerUserId
// ),
// UserMetrics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(pa.PostCount, 0) AS PostCount,
// COALESCE(pa.TotalViews, 0) AS TotalViews,
// COALESCE(pa.TotalScore, 0) AS TotalScore,
// COALESCE(pa.AvgScore, 0) AS AvgScore,
// u.Reputation,
// u.CreationDate
// FROM
// Users u
// LEFT JOIN
// PostsAggregated pa ON u.Id = pa.OwnerUserId
// )
// SELECT
// um.UserId,
// um.DisplayName,
// um.PostCount,
// um.TotalViews,
// um.TotalScore,
// um.AvgScore,
// um.Reputation,
// um.CreationDate
// FROM
// UserMetrics um
// ORDER BY
// um.TotalViews DESC,
// um.TotalScore DESC;
fn q14030(db: &'static So) -> String {
    let pf = owned_since(db, year_ago()).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&db.post.score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, p)| {
        let p = p.unwrap_or([0; 3]);
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), or0(p[2], p[0]), user_col(db, u, "rep"), user_col(db, u, "ucreated")])
    }))
}

// WITH UserPosts AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// p.CreationDate AS PostCreationDate,
// p.LastActivityDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.Title,
// u.Reputation AS UserReputation
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ),
// PostInteractions AS (
// SELECT
// p.Id AS PostId,
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
// p.Id
// )
// SELECT
// up.PostId,
// up.Title,
// up.UserReputation,
// up.PostCreationDate,
// up.LastActivityDate,
// pi.CommentCount,
// pi.UpVotes,
// pi.DownVotes,
// up.Score,
// up.ViewCount,
// up.AnswerCount,
// up.FavoriteCount
// FROM
// UserPosts up
// JOIN
// PostInteractions pi ON up.PostId = pi.PostId
// ORDER BY
// up.LastActivityDate DESC
// LIMIT 100;
fn q14057(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, year_ago()), Ident::<Post>::new(), "cv", &[]).drive(|p, s| v.push((p, s)));
    out(v, |&(p, _)| Reverse(db.post.last_activity_date.get(p).unwrap()), 100, |&(p, s)| {
        let mut f = post_fields(db, p, &["id", "title", "rep", "created", "activity"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["score", "views", "answers", "favorites"]));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
// p.CreationDate,
// p.LastActivityDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.ViewCount, p.Score, p.AnswerCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// AVG(u.Reputation) AS AverageReputation,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
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
// ps.CommentCount,
// ps.UpvoteCount,
// ps.DownvoteCount,
// ps.CreationDate,
// ps.LastActivityDate,
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.AverageReputation,
// us.TotalViews,
// us.TotalScore
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.Score DESC;
fn q14068(db: &'static So) -> String {
    let uid = uids(db);
    let us = g(db).select((&db.user.reputation).and(badges_of(db).opt()).and(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)).opt())).fold([0i64; 7], |a, ((r, b), p)| {
        let mut a = [a[0] + 1, a[1] + r, a[2] + b.is_some() as i64, a[3], a[4], a[5], a[6]];
        if let Some((w, s)) = p {
            a[3] += w.is_some() as i64;
            a[4] += w.unwrap_or(0);
            a[5] += 1;
            a[6] += s;
        }
        a
    });
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["created", "activity", "views", "score", "answers"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[2]), avg(a[1], a[0]), nullable(a[4], a[3]), nullable(a[6], a[5])]);
        row(f)
    }))
}

// WITH BenchmarkData AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS Author,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVoteCount,
// COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVoteCount,
// P.CreationDate,
// P.LastActivityDate,
// P.Score,
// P.ViewCount,
// P.Body
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, U.DisplayName, P.CreationDate, P.LastActivityDate, P.Score, P.ViewCount, P.Body
// )
// SELECT
// PostId,
// Title,
// Author,
// CommentCount,
// UpVoteCount,
// DownVoteCount,
// CreationDate,
// LastActivityDate,
// Score,
// ViewCount,
// Body,
// (EXTRACT(EPOCH FROM (LastActivityDate - CreationDate)) / 60) AS TimeToActivityMinutes,
// (ViewCount / NULLIF(CommentCount, 0)) AS ViewPerComment,
// (Score / NULLIF(ViewCount, 0)) AS ScorePerView
// FROM
// BenchmarkData
// ORDER BY
// Score DESC, ViewCount DESC;
fn q14072(db: &'static So) -> String {
    rows(stats_with(db, questions_only(db), "cv", &[], &[]).iter().map(|&(p, s, _)| {
        let (cd, la) = (db.post.creation_date.get(p).unwrap(), db.post.last_activity_date.get(p).unwrap());
        let w = db.post.view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["created", "activity", "score", "views", "body"]));
        f.push(V::F(hours_to(la, cd) / 60.0));
        f.push(match w {
            Some(w) if s.cx != 0 => V::F(w as f64 / s.cx as f64),
            _ => V::Null,
        });
        f.push(match w {
            Some(w) if w != 0 => V::F(db.post.score.get(p).unwrap() as f64 / w as f64),
            _ => V::Null,
        });
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
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.CreationDate,
// COUNT(c.Id) AS Comments,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.UpVotes,
// ua.DownVotes,
// ua.CommentCount,
// ua.LastPostDate,
// ps.PostId,
// ps.Title AS PostTitle,
// ps.Score AS PostScore,
// ps.ViewCount AS PostViewCount,
// ps.CreationDate AS PostCreationDate,
// ps.Comments AS PostCommentCount
// FROM
// UserActivity ua
// LEFT JOIN
// PostStatistics ps ON ua.UserId = ps.OwnerUserId
// ORDER BY
// ua.PostCount DESC, ua.LastPostDate DESC;
fn q14076(db: &'static So) -> String {
    let uf = g(db)
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt())
        .fold([0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 7], p| match p {
            Some((((t, cd), x), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64, a[5] + c.is_some() as i64, a[6].max(cd)],
            None => a,
        });
    let mut v = Vec::new();
    (&uf).and(posts_of(db).select(Ident::<Post>::new().and(comments_per_post(db))).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..6]));
        f.push(if a[0] == 0 { V::Null } else { V::T(a[6]) });
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["id", "title", "score", "views", "created"]));
                f.push(V::I(c));
            }
            None => f.extend(nulls(6)),
        }
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT v.UserId) AS UniqueVoters,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// SUM(u.Views) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// )
// SELECT
// p.PostId,
// p.PostTypeId,
// p.CreationDate,
// p.CommentCount,
// p.UpVotes,
// p.DownVotes,
// p.UniqueVoters,
// p.AverageReputation,
// u.UserId,
// u.BadgeCount,
// u.TotalUpVotes,
// u.TotalDownVotes,
// u.TotalViews
// FROM
// PostStats p
// JOIN
// UserStats u ON p.PostId = u.UserId
// ORDER BY
// p.CommentCount DESC,
// p.UpVotes DESC;
fn q14095(db: &'static So) -> String {
    let uid = uids(db);
    let dv = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    let User { up_votes, down_votes, views, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(views).and(badges_of(db).opt())).fold([0i64; 4], |a, (((u, d), w), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d, a[3] + w]);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&dv).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, ((s, x), (u, a))| v.push((p, s, x.unwrap_or(0), u, a)));
    rows(v.iter().map(|&(p, s, x, u, a)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(x), stat_fields(db, p, &s, &["rep_avg"]).pop().unwrap(), user_col(db, u, "uid")]);
        f.extend(ints(&a));
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT c.Id) AS TotalComments,
// (SELECT COUNT(DISTINCT u.Id) FROM Users u) AS TotalUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14098(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_of(db).opt()), [0i64; 2], |a, (s, _)| [a[0] + 1, a[1] + s]);
    let dc = db.post.group_by(name(db)).select(comments_of(db)).count_distinct();
    let tu = one(whole(db.user.iq()).select(Ident::<User>::new()).count_distinct());
    let mut v = Vec::new();
    (&f).and((&dc).opt()).drive(|k, (a, c)| v.push((k, a, c.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, c)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(c), V::I(tu)])))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COALESCE(SUM(P.Score), 0) AS TotalScore,
// COALESCE(SUM(P.ViewCount), 0) AS TotalViews
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
// U.UserId,
// U.DisplayName,
// U.TotalPosts,
// U.TotalQuestions,
// U.TotalAnswers,
// U.TotalScore,
// U.TotalViews,
// COALESCE(B.TotalBadges, 0) AS TotalBadges,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats U
// LEFT JOIN
// UserBadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.TotalScore DESC,
// U.TotalPosts DESC
// LIMIT 100;
fn q14101(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| (Reverse(a[5]), Reverse(a[0])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([V::I(a[5]), V::I(a[4])]);
        f.extend(ints(&b));
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// AVG(p.Score) AS AverageScore,
// MAX(p.CreationDate) AS LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title
// ),
// UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
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
// ps.CommentCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.AverageScore,
// ps.LastActivityDate,
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostStatistics ps
// JOIN
// UserStatistics us ON ps.PostId = us.UserId
// ORDER BY
// ps.LastActivityDate DESC,
// ps.AverageScore DESC
// LIMIT 100;
fn q14106(db: &'static So) -> String {
    let uid = uids(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt())).fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, ..)| (newest(db, p), score_desc(db, p)), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::F(db.post.score.get(p).unwrap() as f64)]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&a));
        f
    })
}

// WITH UserPosts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName AS UserDisplayName,
// P.Id AS PostId,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// P.Score AS PostScore,
// P.ViewCount AS PostViewCount,
// P.AnswerCount AS PostAnswerCount,
// P.CommentCount AS PostCommentCount,
// COALESCE(C.VoteCount, 0) AS VoteCount
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
// ) C ON P.Id = C.PostId
// )
// SELECT
// UP.UserId,
// UP.UserDisplayName,
// COUNT(UP.PostId) AS TotalPosts,
// SUM(UP.PostScore) AS TotalPostScore,
// SUM(UP.PostViewCount) AS TotalPostViews,
// SUM(UP.VoteCount) AS TotalVotes,
// MAX(UP.PostCreationDate) AS LastPostDate
// FROM
// UserPosts UP
// GROUP BY
// UP.UserId, UP.UserDisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q14109(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(score.and(view_count.opt()).and(votes_per_post(db)).and(creation_date)).opt()).fold([0, 0, 0, 0, 0, i64::MIN], |a: [i64; 6], p| match p {
        Some((((s, w), x), cd)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x, a[5].max(cd)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| Reverse(a[0]), 10, |&(u, a)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), if a[0] == 0 { V::Null } else { V::T(a[5]) }]
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q14110(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_per_post(db)), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE
// WHEN P.PostTypeId = 1 THEN 1
// ELSE 0
// END) AS Questions,
// SUM(CASE
// WHEN P.PostTypeId = 2 THEN 1
// ELSE 0
// END) AS Answers,
// SUM(CASE
// WHEN P.PostTypeId = 3 THEN 1
// ELSE 0
// END) AS Wikis,
// COALESCE(SUM(P.Score), 0) AS TotalScore
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
// U.UserId,
// U.DisplayName,
// U.TotalPosts,
// U.Questions,
// U.Answers,
// U.Wikis,
// U.TotalScore,
// COALESCE(B.TotalBadges, 0) AS TotalBadges,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats U
// LEFT JOIN
// UserBadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.TotalScore DESC, U.TotalPosts DESC
// LIMIT 100;
fn q14118(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + s],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| (Reverse(a[4]), Reverse(a[0])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
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
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(ph.HistoryCount, 0) AS HistoryCount
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS HistoryCount FROM PostHistory GROUP BY PostId) ph ON p.Id = ph.PostId
// )
// SELECT
// u.DisplayName,
// u.TotalPosts,
// u.TotalQuestions,
// u.TotalAnswers,
// u.TotalUpVotes,
// u.TotalDownVotes,
// p.PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.CommentCount,
// p.HistoryCount
// FROM
// UserPostStats u
// INNER JOIN
// PostStats p ON u.UserId = p.OwnerUserId
// ORDER BY
// u.TotalPosts DESC, p.ViewCount DESC;
fn q14120(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64],
        None => a,
    });
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(comments_per_post(db)).and(history_per_post(db)).and((&db.post.owner_user).select(&uf))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), h), a)| {
        let mut f = vec![user_col(db, db.post.owner_user.get(p).unwrap(), "name")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(c), V::I(h)]);
        row(f)
    }))
}

// WITH PostData AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.ClosedDate,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// bt.Name AS BadgeName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges bt ON u.Id = bt.UserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
// ),
// VoteData AS (
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
// pd.PostId,
// pd.PostTypeId,
// pd.CreationDate,
// pd.Score,
// pd.ViewCount,
// pd.AnswerCount,
// pd.CommentCount,
// pd.FavoriteCount,
// pd.ClosedDate,
// pd.Reputation,
// pd.UserCreationDate,
// vd.UpVotes,
// vd.DownVotes,
// pd.BadgeName
// FROM
// PostData pd
// LEFT JOIN
// VoteData vd ON pd.PostId = vd.PostId
// ORDER BY
// pd.CreationDate DESC;
fn q14130(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned_since(db, month_ago()).select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(badges_of(db).select(&db.badge.name)).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), b)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views", "answers", "comments", "favorites", "closed", "rep"]);
        f.extend(post_fields(db, p, &["ucreated"]));
        f.extend([oint(x.map(|x| x[1])), oint(x.map(|x| x[2])), ostr(b)]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN v.VoteTypeId = 5 THEN 1 ELSE 0 END) AS TotalFavorites
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.UserId = u.Id
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalComments,
// TotalUpVotes,
// TotalDownVotes,
// TotalFavorites
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC, TotalUpVotes DESC
// LIMIT 100;
fn q14135(db: &'static So) -> String {
    let own = self_votes(db);
    let uf = g(db).select(posts_of(db).select(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 3], |a, p| {
        let t = p.and_then(|(_, t)| t);
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(5)) as i64]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dc).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    out(v, |&(_, a, p, _)| (Reverse(p), Reverse(a[0])), 100, |&(u, a, p, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(c)];
        f.extend(ints(&a));
        f
    })
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalPostOwners,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// AVG(ViewCount) AS AvgViews,
// AVG(Score) AS AvgScore,
// AVG(AnswerCount) AS AvgAnswersPerQuestion
// FROM
// Posts
// ), UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation,
// SUM(CASE WHEN Views IS NOT NULL THEN Views ELSE 0 END) AS TotalViews,
// SUM(UpVotes) AS TotalUpVotes,
// SUM(DownVotes) AS TotalDownVotes
// FROM
// Users
// ), VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN VoteTypeId = 6 THEN 1 ELSE 0 END) AS TotalCloseVotes,
// SUM(CASE WHEN VoteTypeId = 11 THEN 1 ELSE 0 END) AS TotalUndeletionVotes
// FROM
// Votes
// )
// SELECT
// p.TotalPosts,
// p.TotalPostOwners,
// p.TotalQuestions,
// p.TotalAnswers,
// p.TotalTagWikis,
// p.AvgViews,
// p.AvgScore,
// p.AvgAnswersPerQuestion,
// u.TotalUsers,
// u.AvgReputation,
// u.TotalViews AS UserTotalViews,
// u.TotalUpVotes,
// u.TotalDownVotes,
// v.TotalVotes,
// v.TotalUpVotes AS VoteTotalUpVotes,
// v.TotalDownVotes AS VoteTotalDownVotes,
// v.TotalCloseVotes,
// v.TotalUndeletionVotes
// FROM
// PostStats p,
// UserStats u,
// VoteStats v;
fn q14137(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, answer_count, .. } = &db.post;
    let p = db.post.select(post_type_id.and(view_count.opt()).and(score).and(answer_count.opt())).fold_flat([0i64; 9], |a, (((t, w), s), an)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s, a[7] + an.is_some() as i64, a[8] + an.unwrap_or(0)]
    });
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let User { reputation, views, up_votes, down_votes, .. } = &db.user;
    let u = db.user.select(reputation.and(views).and(up_votes).and(down_votes)).fold_flat([0i64; 5], |a, (((r, w), x), y)| [a[0] + 1, a[1] + r, a[2] + w, a[3] + x, a[4] + y]);
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 5], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (t == 6) as i64, a[4] + (t == 11) as i64]);
    let mut f = vec![V::I(p[0]), V::I(owners), nullable(p[1], p[0]), nullable(p[2], p[0]), nullable(p[3], p[0]), avg(p[5], p[4]), avg(p[6], p[0]), avg(p[8], p[7]), V::I(u[0]), avg(u[1], u[0]), nullable(u[2], u[0]), nullable(u[3], u[0]), nullable(u[4], u[0])];
    f.extend([V::I(x[0]), nullable(x[1], x[0]), nullable(x[2], x[0]), nullable(x[3], x[0]), nullable(x[4], x[0])]);
    row(f)
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViews,
// MAX(p.LastActivityDate) AS LastActivity
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// BadgeStats AS (
// SELECT
// UserId,
// COUNT(Id) AS BadgeCount,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
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
// ups.TotalScore,
// ups.AvgViews,
// ups.LastActivity,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// BadgeStats bs ON ups.UserId = bs.UserId
// ORDER BY
// ups.TotalScore DESC;
fn q14146(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, last_activity_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(last_activity_date)).opt()).fold([0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 7], p| match p {
        Some((((t, s), w), la)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6].max(la)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[3], a[0]), avg(a[5], a[4]), if a[0] == 0 { V::Null } else { V::T(a[6]) }]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.CommentCount) AS TotalComments,
// SUM(p.FavoriteCount) AS TotalFavorites,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.DisplayName
// FROM
// Users u
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// ps.TotalPosts,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.AverageScore,
// ps.TotalViews,
// ps.TotalComments,
// ps.TotalFavorites,
// ps.LastPostDate
// FROM
// UserReputation u
// LEFT JOIN
// PostStats ps ON u.UserId = ps.OwnerUserId
// ORDER BY
// u.Reputation DESC;
fn q14156(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, comment_count, favorite_count, creation_date, .. } = &db.post;
    let pf = owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(score).and(view_count.opt()).and(comment_count).and(favorite_count.opt()).and(creation_date))
        .fold([0, 0, 0, 0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 10], (((((t, s), w), cc), fc), cd)| {
            [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + cc, a[7] + fc.is_some() as i64, a[8] + fc.unwrap_or(0), a[9].max(cd)]
        });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        match p {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), nullable(a[5], a[4]), V::I(a[6]), nullable(a[8], a[7]), V::T(a[9])]),
            None => f.extend(nulls(8)),
        }
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastEditDate,
// p.CreationDate
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// GROUP BY p.Id, p.Title, p.PostTypeId, p.CreationDate
// ),
// UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// ps.LastEditDate,
// ps.CreationDate AS PostCreationDate,
// us.UserId,
// us.DisplayName AS OwnerDisplayName,
// us.BadgeCount,
// us.TotalViews,
// us.TotalScore
// FROM PostStatistics ps
// JOIN UserStatistics us ON ps.PostId = us.UserId
// ORDER BY ps.PostId;
fn q14165(db: &'static So) -> String {
    let uid = uids(db);
    let us = g(db).select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)).opt())).fold([0i64; 5], |a, (b, p)| {
        let mut a = a;
        a[0] += b.is_some() as i64;
        if let Some((w, s)) = p {
            a[1] += w.is_some() as i64;
            a[2] += w.unwrap_or(0);
            a[3] += 1;
            a[4] += s;
        }
        a
    });
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvh", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id"]);
        f.extend([V::I(s.cx), V::I(s.vx), stat_field(&s, "hmax").unwrap()]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), nullable(a[4], a[3])]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// AVG(p.Score) AS AveragePostScore,
// SUM(p.ViewCount) AS TotalViewCount,
// SUM(p.CommentCount) AS TotalCommentCount
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
// u.TotalQuestions,
// u.TotalAnswers,
// u.AveragePostScore,
// u.TotalViewCount,
// u.TotalCommentCount
// FROM
// UserPostStats u
// ORDER BY
// u.TotalPosts DESC
// LIMIT 100;
fn q14168(db: &'static So) -> String {
    let Post { score, view_count, comment_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(score.and(view_count.opt()).and(comment_count)).opt()).fold([0i64; 5], |a, p| match p {
        Some(((s, w), cc)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + cc],
        None => a,
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dq = ud(db, UserWhere::All, posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(1))));
    let da = ud(db, UserWhere::All, posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(2))));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dq).opt()).and((&da).opt()).drive(|u, (((a, p), q), x)| v.push((u, a, [p, q, x].map(|y| y.unwrap_or(0)))));
    out(v, |&(_, _, d)| Reverse(d[0]), 100, |&(u, a, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&d));
        f.extend([avg(a[1], a[0]), nullable(a[3], a[2]), nullable(a[4], a[0])]);
        f
    })
}

pub static ENTRIES: &[harness::Entry] = &[
    ("13850", q13850),
    ("13852", q13852),
    ("13858", q13858),
    ("13859", q13859),
    ("13861", q13861),
    ("13867", q13867),
    ("13869", q13869),
    ("13878", q13878),
    ("13892", q13892),
    ("13907", q13907),
    ("13911", q13911),
    ("13912", q13912),
    ("13913", q13913),
    ("13921", q13921),
    ("13923", q13923),
    ("13933", q13933),
    ("13938", q13938),
    ("13940", q13940),
    ("13945", q13945),
    ("13946", q13946),
    ("13955", q13955),
    ("13959", q13959),
    ("13964", q13964),
    ("13966", q13966),
    ("13970", q13970),
    ("13975", q13975),
    ("13977", q13977),
    ("13979", q13979),
    ("13984", q13984),
    ("13990", q13990),
    ("14000", q14000),
    ("14002", q14002),
    ("14004", q14004),
    ("14007", q14007),
    ("14015", q14015),
    ("14019", q14019),
    ("14021", q14021),
    ("14030", q14030),
    ("14057", q14057),
    ("14068", q14068),
    ("14072", q14072),
    ("14076", q14076),
    ("14095", q14095),
    ("14098", q14098),
    ("14101", q14101),
    ("14106", q14106),
    ("14109", q14109),
    ("14110", q14110),
    ("14118", q14118),
    ("14120", q14120),
    ("14130", q14130),
    ("14135", q14135),
    ("14137", q14137),
    ("14146", q14146),
    ("14156", q14156),
    ("14165", q14165),
    ("14168", q14168),
];
