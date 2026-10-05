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

fn badges_by_uid(db: &'static So) -> HashIdx<i64, Id<Badge>> {
    (&db.badge.user_id).inv().collect()
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

// --- batch 114 --------------------------------------------------------------

// WITH UserVotes AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 1 THEN 1 ELSE 0 END) AS AcceptedVoteCount
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostDetails AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// ),
// VoteStatistics AS (
// SELECT
// PD.PostId,
// PD.Title,
// PD.CreationDate,
// PD.Score,
// PD.ViewCount,
// PD.AnswerCount,
// PD.CommentCount,
// PD.OwnerDisplayName,
// PD.OwnerReputation,
// UV.TotalVotes,
// UV.VoteCount,
// UV.AcceptedVoteCount
// FROM
// PostDetails PD
// LEFT JOIN
// UserVotes UV ON PD.OwnerDisplayName = UV.DisplayName
// )
// SELECT
// VS.Title,
// VS.CreationDate,
// VS.Score,
// VS.ViewCount,
// VS.AnswerCount,
// VS.CommentCount,
// VS.OwnerDisplayName,
// VS.OwnerReputation,
// VS.TotalVotes,
// VS.VoteCount,
// VS.AcceptedVoteCount
// FROM
// VoteStatistics VS
// ORDER BY
// VS.Score DESC, VS.ViewCount DESC
// LIMIT 100;
fn q10543(db: &'static So) -> String {
    let names = by_name(db);
    let uv = g(db).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + matches!(t, 2 | 3) as i64, a[2] + (t == 1) as i64],
        None => a,
    });
    let mut v = Vec::new();
    owned(db)
        .select(Ident::<Post>::new().and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(&uv).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&(p, _)| score_views(db, p), 100, |&(p, a)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "comments", "owner", "rep"]);
        f.extend((0..3).map(|i| oint(a.map(|a| a[i]))));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// AVG(LENGTH(c.Text)) AS AvgCommentLength
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
// COUNT(bp.Id) AS BadgesCount,
// SUM(u.Reputation) AS TotalReputation
// FROM
// Users u
// LEFT JOIN
// Badges bp ON u.Id = bp.UserId
// GROUP BY
// u.Id
// )
// SELECT
// ps.PostId,
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.AvgCommentLength,
// us.UserId,
// us.BadgesCount,
// us.TotalReputation
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.PostId;
fn q10547(db: &'static So) -> String {
    let uid = uids(db);
    let pf = db
        .post
        .with((&db.post.origid).select(&uid))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(&db.comment.text).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 5], |a, (c, t)| {
            [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64, a[4] + c.map_or(0, |s| s.chars().count() as i64)]
        });
    let us = g(db).select((&db.user.reputation).and(badges_of(db).opt())).fold((0i64, 0i64), |(n, r), (rep, b)| (n + b.is_some() as i64, r + rep));
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us))).drive(|p, (a, (u, b))| v.push((p, a, u, b)));
    rows(v.iter().map(|&(p, a, u, (n, r))| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), user_col(db, u, "uid"), V::I(n), V::I(r)]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis
// FROM
// Posts
// ),
// CommentStats AS (
// SELECT
// COUNT(*) AS TotalComments
// FROM
// Comments
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// SUM(CASE WHEN Reputation >= 1000 THEN 1 ELSE 0 END) AS InfluentialUsers
// FROM
// Users
// )
// SELECT
// (SELECT TotalPosts FROM PostStats) AS TotalPosts,
// (SELECT TotalQuestions FROM PostStats) AS TotalQuestions,
// (SELECT TotalAnswers FROM PostStats) AS TotalAnswers,
// (SELECT TotalTagWikis FROM PostStats) AS TotalTagWikis,
// (SELECT TotalComments FROM CommentStats) AS TotalComments,
// (SELECT TotalUsers FROM UserStats) AS TotalUsers,
// (SELECT InfluentialUsers FROM UserStats) AS InfluentialUsers;
fn q10549(db: &'static So) -> String {
    let p = db.post.select(&db.post.post_type_id).fold_flat([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64]);
    let (un, inf) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, a), r| (n + 1, a + (r >= 1000) as i64));
    row(vec![V::I(p[0]), nullable(p[1], p[0]), nullable(p[2], p[0]), nullable(p[3], p[0]), V::I(count(db.comment.iq())), V::I(un), nullable(inf, un)])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// p.CreationDate,
// p.ViewCount,
// COALESCE(a.AnswerCount, 0) AS TotalAnswers,
// COALESCE(c.CommentCount, 0) AS TotalComments,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// (
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
// LEFT JOIN
// (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.PostTypeId, p.CreationDate, p.ViewCount, a.AnswerCount, c.CommentCount, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10550(db: &'static So) -> String {
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, a), c), x)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id", "created", "views"]);
        f.extend([V::I(a), V::I(c)]);
        f.extend(post_fields(db, p, &["rep"]));
        f.push(V::I(x));
        f
    })
}

// WITH UserPosts AS (
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
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId,
// COALESCE(c.Count, 0) AS CommentCount,
// COALESCE(vs.TotalVotes, 0) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS Count FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId) vs ON p.Id = vs.PostId
// )
// SELECT
// up.DisplayName,
// up.TotalPosts,
// up.TotalQuestions,
// up.TotalAnswers,
// up.TotalViews,
// up.TotalScore,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.AcceptedAnswerId,
// ps.CommentCount,
// ps.TotalVotes
// FROM
// UserPosts up
// JOIN
// PostStats ps ON up.UserId = ps.AcceptedAnswerId
// ORDER BY
// up.TotalScore DESC, up.TotalPosts DESC;
fn q10552(db: &'static So) -> String {
    let uid = uids(db);
    let uf = upqa(db);
    let acc = (&db.post.accepted_answer_id).opt().map(|a: Option<i64>| a.unwrap_or(0));
    let mut v = Vec::new();
    db.post
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)).and(acc.select(&uid).select(Ident::<User>::new().and(&uf))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), x), (u, a))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), nullable(a[5], a[0])];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(db.post.accepted_answer_id.get(p).unwrap_or(0)), V::I(c), V::I(x)]);
        row(f)
    }))
}

// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS PostCount,
// AVG(p.ViewCount) AS AverageViewCount,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes
// FROM
// Posts p
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
// p.PostTypeId
// ORDER BY
// PostCount DESC;
fn q10553(db: &'static So) -> String {
    let f = by_key(
        db.post.iq(),
        &db.post.post_type_id,
        (&db.post.view_count).opt().and(comments_per_post(db)).and(votes_per_post(db)),
        [0i64; 5],
        |a, ((w, c), x)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + c, a[4] + x],
    );
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::I(k), V::I(a[0]), avg(a[2], a[1]), V::I(a[3]), V::I(a[4])])))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// COALESCE(V.UpVotes, 0) AS UpVotes,
// COALESCE(V.DownVotes, 0) AS DownVotes,
// COALESCE(V.VoteCount, 0) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(Id) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) V ON P.Id = V.PostId
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q10563(db: &'static So) -> String {
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, u), d), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(u), V::I(d), V::I(x)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// MAX(p.LastActivityDate) AS LastActivity,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// MAX(u.Reputation) AS Reputation
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
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.LastActivity,
// us.UserId,
// us.DisplayName AS OwnerDisplayName,
// us.PostCount,
// us.TotalUpVotes,
// us.TotalDownVotes,
// us.Reputation
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerUserId = us.UserId
// ORDER BY
// ps.ViewCount DESC,
// ps.LastActivity DESC
// LIMIT 100;
fn q10566(db: &'static So) -> String {
    let vt = || votes_of(db).select(&db.vote.vote_type_id).opt();
    let us = user_base(db, UserWhere::All).group_by(Ident::<User>::new()).select(posts_of(db).select(vt()).opt()).dense_fold(db.user.id.n, [0i64; 2], |a, p| {
        [a[0] + (p == Some(Some(2))) as i64, a[1] + (p == Some(Some(3))) as i64]
    });
    let ps = owned(db).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(vt())).dense_fold(db.post.id.n, [0i64; 4], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&ps)
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp)))
        .drive(|p, (s, ((u, a), d))| v.push((p, s, u, a, d)));
    out(v, |&(p, _, _, _, _)| (views_desc(db, p), Reverse(db.post.last_activity_date.get(p).unwrap())), 100, |&(p, s, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(s.map(V::I));
        f.extend(post_fields(db, p, &["activity"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a[0]), V::I(a[1]), user_col(db, u, "rep")]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN pt.Name = 'Wiki' THEN 1 ELSE 0 END) AS WikiCount,
// SUM(CASE WHEN pt.Name = 'TagWiki' THEN 1 ELSE 0 END) AS TagWikiCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserVoteStats AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// v.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.WikiCount,
// ups.TagWikiCount,
// uvs.VoteCount,
// uvs.UpVotes,
// uvs.DownVotes
// FROM
// UserPostStats ups
// LEFT JOIN
// UserVoteStats uvs ON ups.UserId = uvs.UserId
// ORDER BY
// ups.PostCount DESC, uvs.VoteCount DESC
// LIMIT 100;
fn q10569(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type).select(&db.post_type.name).opt()).opt()).fold([0i64; 5], |a, p| match p {
        Some(n) => [
            a[0] + 1,
            a[1] + (n == Some("Question")) as i64,
            a[2] + (n == Some("Answer")) as i64,
            a[3] + (n == Some("Wiki")) as i64,
            a[4] + (n == Some("TagWiki")) as i64,
        ],
        None => a,
    });
    let vs = vote_named(db);
    let mut v = Vec::new();
    (&uf).and((&vs).opt()).drive(|u, (a, x)| v.push((u, a, x)));
    out(v, |&(_, a, x)| (Reverse(a[0]), x.is_none(), Reverse(x.map(|x| x[0]))), 100, |&(u, a, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f
    })
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount
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
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate,
// COALESCE(UPC.PostCount, 0) AS TotalPosts,
// COALESCE(UPC.QuestionsCount, 0) AS TotalQuestions,
// COALESCE(UPC.AnswersCount, 0) AS TotalAnswers,
// COALESCE(UBC.BadgeCount, 0) AS TotalBadges,
// COALESCE(UBC.GoldBadgeCount, 0) AS GoldBadges,
// COALESCE(UBC.SilverBadgeCount, 0) AS SilverBadges,
// COALESCE(UBC.BronzeBadgeCount, 0) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// UserPostCounts UPC ON u.Id = UPC.UserId
// LEFT JOIN
// UserBadgeCounts UBC ON u.Id = UBC.UserId
// ORDER BY
// u.Reputation DESC
// LIMIT 100;
fn q10583(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(u, _, _)| rep_desc(db, u), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated"), user_col(db, u, "last_access")];
        f.extend(ints(&a[..3]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// COUNT(DISTINCT p.Id) AS PostCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AvgViewCount,
// AVG(p.Score) AS AvgScore,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.BadgeCount,
// ps.TotalPosts,
// ps.AvgViewCount,
// ps.AvgScore,
// ps.TotalComments,
// ps.QuestionCount,
// ps.AnswerCount,
// u.TotalViews,
// u.TotalScore,
// u.PostCount
// FROM
// UserStats u
// LEFT JOIN
// PostStats ps ON u.UserId = ps.OwnerUserId
// ORDER BY
// u.Reputation DESC,
// u.BadgeCount DESC,
// u.TotalScore DESC;
fn q10591(db: &'static So) -> String {
    let Post { view_count, score, post_type_id, .. } = &db.post;
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let ps = g(db).select(posts_of(db).select(view_count.opt().and(score).and(post_type_id).and(comments_of(db).opt()))).fold([0i64; 6], |a, (((w, s), t), _)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + (t == 1) as i64, a[5] + (t == 2) as i64]
    });
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&ps).opt()).and((&dc).opt()).drive(|u, (((a, d), p), c)| v.push((u, a, d.unwrap_or(0), p, c.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, d, p, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a.bx)];
        match p {
            Some(p) => f.extend([V::I(p[0]), avg(p[2], p[1]), avg(p[3], p[0]), V::I(c), V::I(p[4]), V::I(p[5])]),
            None => f.extend(nulls(6)),
        }
        f.extend([ustat_field(&a, "views_sum"), ustat_field(&a, "score_sum"), V::I(d)]);
        row(f)
    }))
}

// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10592(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_per_post(db)), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// SELECT
// p.Title AS PostTitle,
// u.DisplayName AS PostOwner,
// p.CreationDate AS PostCreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.ViewCount,
// (SELECT COUNT(*) FROM Posts p2 WHERE p2.ParentId = p.Id) AS AnswerCount
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
// p.Title, u.DisplayName, p.CreationDate, p.ViewCount, p.Id
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10597(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cv", &[]).and(answers_per_post(db)).drive(|p, (s, a)| v.push((p, s, a)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, s, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["views"]));
        f.push(V::I(a));
        f
    })
}

// WITH UserPostActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserCommentActivity AS (
// SELECT
// c.UserId,
// COUNT(c.Id) AS CommentCount
// FROM
// Comments c
// GROUP BY
// c.UserId
// ),
// UserActivity AS (
// SELECT
// up.UserId,
// up.DisplayName,
// up.PostCount,
// up.TotalViews,
// up.TotalScore,
// COALESCE(uca.CommentCount, 0) AS CommentCount
// FROM
// UserPostActivity up
// LEFT JOIN
// UserCommentActivity uca ON up.UserId = uca.UserId
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// TotalViews,
// TotalScore,
// CommentCount,
// (PostCount + CommentCount) AS TotalActivity
// FROM
// UserActivity
// ORDER BY
// TotalActivity DESC
// LIMIT 10;
fn q10600(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(view_count.opt().and(score)).opt()).fold([0i64; 3], |a, p| match p {
        Some((w, s)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s],
        None => a,
    });
    let cu = comments_per_user(db);
    let mut v = Vec::new();
    (&uf).and(&cu).drive(|u, (a, c)| v.push((u, a, c)));
    out(v, |&(_, a, c)| Reverse(a[0] + c), 10, |&(u, a, c)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c), V::I(a[0] + c)]
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// COUNT(DISTINCT ph.Id) AS HistoryCount,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.PostTypeId
// ),
// PerformanceBenchmark AS (
// SELECT
// PostTypeId,
// COUNT(*) AS PostCount,
// SUM(CommentCount) AS TotalComments,
// SUM(VoteCount) AS TotalVotes,
// SUM(BadgeCount) AS TotalBadges,
// SUM(HistoryCount) AS TotalHistories,
// AVG(AvgUserReputation) AS AvgUserReputation
// FROM
// PostStats
// GROUP BY
// PostTypeId
// )
// SELECT
// pt.Name AS PostType,
// pb.PostCount,
// pb.TotalComments,
// pb.TotalVotes,
// pb.TotalBadges,
// pb.TotalHistories,
// pb.AvgUserReputation
// FROM
// PerformanceBenchmark pb
// JOIN
// PostTypes pt ON pb.PostTypeId = pt.Id
// ORDER BY
// PostCount DESC;
fn q10605(db: &'static So) -> String {
    let base = || since(db, date(2023, 1, 1));
    let bid = badges_by_uid(db);
    let ob = || (&db.post.owner_user_id).select(&bid);
    let pf = base()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(ob().opt()).and(history_of(db).opt()))
        .fold(0i64, |n, (((c, _), _), _)| n + c.is_some() as i64);
    let x = per_post_distinct(db, votes_of(db));
    let b = per_post_distinct(db, ob());
    let h = per_post_distinct(db, history_of(db));
    let f = base()
        .group_by(&db.post.post_type)
        .select((&pf).and((&x).opt()).and((&b).opt()).and((&h).opt()).and((&db.post.owner_user).select(&db.user.reputation).opt()))
        .fold([0i64; 7], |a, ((((c, x), b), h), r)| {
            [a[0] + 1, a[1] + c, a[2] + x.unwrap_or(0), a[3] + b.unwrap_or(0), a[4] + h.unwrap_or(0), a[5] + r.is_some() as i64, a[6] + r.unwrap_or(0)]
        });
    let mut v = Vec::new();
    (&f).drive(|t, a| v.push((t, a)));
    rows(v.iter().map(|&(t, a)| {
        let mut f = vec![V::S(db.post_type.name.get(t).unwrap())];
        f.extend(ints(&a[..5]));
        f.push(avg(a[6], a[5]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.ViewCount) AS TotalPostViews,
// AVG(p.Score) AS AveragePostScore
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.Score AS PostScore,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.DisplayName AS PostOwner,
// us.Reputation AS OwnerReputation,
// us.BadgeCount,
// us.TotalPostViews,
// us.AveragePostScore
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q10610(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, _, _)| score_views(db, p), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")]);
        f.extend([V::I(a.bx), ustat_field(&a, "views_sum"), ustat_field(&a, "score_avg")]);
        f
    })
}

// WITH PostSummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// U.Reputation AS OwnerReputation,
// U.DisplayName AS OwnerDisplayName,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseVotes
// FROM
// Posts P
// LEFT JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN PostHistory PH ON P.Id = PH.PostId
// WHERE
// P.CreationDate >= '2022-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount, P.CommentCount, P.FavoriteCount, U.Reputation, U.DisplayName
// ),
// PostMetrics AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// AVG(AnswerCount) AS AvgAnswerCount,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(FavoriteCount) AS AvgFavoriteCount,
// SUM(UpVotes) AS TotalUpVotes,
// SUM(DownVotes) AS TotalDownVotes,
// SUM(CloseVotes) AS TotalCloseVotes
// FROM
// PostSummary
// )
// SELECT
// PM.TotalPosts,
// PM.AvgViewCount,
// PM.AvgScore,
// PM.AvgAnswerCount,
// PM.AvgCommentCount,
// PM.AvgFavoriteCount,
// PM.TotalUpVotes,
// PM.TotalDownVotes,
// PM.TotalCloseVotes
// FROM
// PostMetrics PM;
fn q10611(db: &'static So) -> String {
    let Post { view_count, score, answer_count, comment_count, favorite_count, .. } = &db.post;
    let base = || since(db, date(2022, 1, 1));
    let pf = stats_fold(db, base(), Ident::<Post>::new(), "vh", &[]);
    let a = base().select(view_count.opt().and(score).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()).and(&pf)).fold_flat([0i64; 12], |a, (((((w, s), an), cc), fc), st)| {
        [
            a[0] + 1,
            a[1] + w.is_some() as i64,
            a[2] + w.unwrap_or(0),
            a[3] + s,
            a[4] + an.is_some() as i64,
            a[5] + an.unwrap_or(0),
            a[6] + cc,
            a[7] + fc.is_some() as i64,
            a[8] + fc.unwrap_or(0),
            a[9] + st.up,
            a[10] + st.down,
            a[11] + st.h10,
        ]
    });
    row(vec![V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), avg(a[5], a[4]), avg(a[6], a[0]), avg(a[8], a[7]), nullable(a[9], a[0]), nullable(a[10], a[0]), nullable(a[11], a[0])])
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01' AND p.CreationDate < '2023-10-01'
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// PostCount,
// AverageScore,
// TotalVotes
// FROM
// PostMetrics
// ORDER BY
// PostCount DESC;
fn q10621(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(date(2023, 1, 1))).with((&db.post.creation_date).lt(date(2023, 10, 1)));
    let f = by_key(base, name(db), (&db.post.score).and(votes_of(db).opt()), [0i64; 3], |a, (s, x)| [a[0] + 1, a[1] + s, a[2] + x.is_some() as i64]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers
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
// COALESCE(UPS.PostCount, 0) AS PostCount,
// COALESCE(UPS.TotalScore, 0) AS TotalScore,
// COALESCE(UPS.TotalViews, 0) AS TotalViews,
// COALESCE(UPS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(UBS.BadgeCount, 0) AS BadgeCount,
// COALESCE(UBS.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(UBS.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(UBS.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM
// Users U
// LEFT JOIN
// UserPostStats UPS ON U.Id = UPS.UserId
// LEFT JOIN
// UserBadgeStats UBS ON U.Id = UBS.UserId
// ORDER BY
// COALESCE(UPS.TotalScore, 0) DESC
// LIMIT 100;
fn q10631(db: &'static So) -> String {
    let Post { score, view_count, answer_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(score.and(view_count.opt()).and(answer_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some(((s, w), an)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + an.unwrap_or(0)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| Reverse(a[1]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// U.Reputation AS OwnerReputation
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ),
// AggregatedStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AvgViews,
// AVG(Score) AS AvgScore,
// AVG(AnswerCount) AS AvgAnswers,
// AVG(CommentCount) AS AvgComments,
// AVG(OwnerReputation) AS AvgOwnerReputation
// FROM
// PostStatistics
// )
// SELECT
// *
// FROM
// AggregatedStats;
fn q10635(db: &'static So) -> String {
    let Post { view_count, score, answer_count, comment_count, .. } = &db.post;
    let a = owned_since(db, year_ago())
        .select(view_count.opt().and(score).and(answer_count.opt()).and(comment_count).and((&db.post.owner_user).select(&db.user.reputation)))
        .fold_flat([0i64; 8], |a, ((((w, s), an), cc), r)| {
            [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + cc, a[7] + r]
        });
    row(vec![V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), avg(a[5], a[4]), avg(a[6], a[0]), avg(a[7], a[0])])
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostsByUsers,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserStats AS (
// SELECT
// COUNT(Id) AS TotalUsers,
// SUM(Reputation) AS TotalReputation,
// AVG(Reputation) AS AverageReputation
// FROM
// Users
// ),
// CommentStats AS (
// SELECT
// COUNT(Id) AS TotalComments,
// AVG(Score) AS AverageCommentScore
// FROM
// Comments
// ),
// VoteStats AS (
// SELECT
// COUNT(Id) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.PostsByUsers,
// ps.PositiveScorePosts,
// ps.AverageScore,
// ps.TotalViews,
// us.TotalUsers,
// us.TotalReputation,
// us.AverageReputation,
// cs.TotalComments,
// cs.AverageCommentScore,
// vs.TotalVotes,
// vs.UpVotes,
// vs.DownVotes
// FROM
// PostStats ps,
// UserStats us,
// CommentStats cs,
// VoteStats vs
// ORDER BY
// ps.TotalPosts DESC;
fn q10640(db: &'static So) -> String {
    let Post { owner_user_id, score, view_count, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), owner_user_id.opt().and(score).and(view_count.opt()), [0i64; 6], |a, ((o, s), w)| {
        [a[0] + 1, a[1] + o.is_some() as i64, a[2] + (s > 0) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let us = whole(db.user.iq()).select(&db.user.reputation).fold((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let cs = whole(db.comment.iq()).select(&db.comment.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let vs = whole(db.vote.iq()).select(&db.vote.vote_type_id).fold((0i64, 0i64, 0i64), |(n, u, d), t| (n + 1, u + (t == 2) as i64, d + (t == 3) as i64));
    let mut v = Vec::new();
    let one = rel(vec![()]).select((&us).opt().and((&cs).opt()).and((&vs).opt()));
    (&f).cross(&one).drive(|(k, _), (a, ((u, c), x))| v.push((k, a, u.unwrap_or((0, 0)), c.unwrap_or((0, 0)), x.unwrap_or((0, 0, 0)))));
    rows(v.iter().map(|&(k, a, (un, rs), (cn, cs), (xn, xu, xd))| {
        row(vec![
            V::S(k),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            avg(a[3], a[0]),
            nullable(a[5], a[4]),
            V::I(un),
            nullable(rs, un),
            avg(rs, un),
            V::I(cn),
            avg(cs, cn),
            V::I(xn),
            nullable(xu, xn),
            nullable(xd, xn),
        ])
    }))
}

// WITH UserReputation AS (
// SELECT Id AS UserId, Reputation
// FROM Users
// ),
// PostCounts AS (
// SELECT OwnerUserId AS UserId, COUNT(Id) AS TotalPosts
// FROM Posts
// GROUP BY OwnerUserId
// ),
// VoteCounts AS (
// SELECT PostId, COUNT(Id) AS TotalVotes
// FROM Votes
// GROUP BY PostId
// ),
// TopPosts AS (
// SELECT p.Id AS PostId, p.Title, p.CreationDate, pc.TotalPosts, uc.Reputation, COALESCE(vc.TotalVotes, 0) AS TotalVotes
// FROM Posts p
// JOIN PostCounts pc ON p.OwnerUserId = pc.UserId
// JOIN UserReputation uc ON p.OwnerUserId = uc.UserId
// LEFT JOIN VoteCounts vc ON p.Id = vc.PostId
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
// ORDER BY TotalVotes DESC, p.CreationDate DESC
// LIMIT 100
// )
// SELECT *
// FROM TopPosts;
fn q10643(db: &'static So) -> String {
    let ppu = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned_since(db, month_ago()).select(Ident::<Post>::new().and((&db.post.owner_user).select(&ppu)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), x)| (Reverse(x), newest(db, p)), 100, |&((p, n), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["rep"]));
        f.push(V::I(x));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// WHERE
// p.CreationDate >= '2023-01-01' AND p.CreationDate < '2023-12-31'
// GROUP BY
// p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// ps.PostCount,
// ps.AverageScore
// FROM
// Users u
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// us.UserId,
// us.Reputation,
// COALESCE(us.PostCount, 0) AS TotalPosts,
// COALESCE(us.AverageScore, 0) AS AvgPostScore
// FROM
// UserStats us
// ORDER BY
// us.Reputation DESC;
fn q10646(db: &'static So) -> String {
    let pf = owned(db)
        .with((&db.post.creation_date).ge(date(2023, 1, 1)))
        .with((&db.post.creation_date).lt(date(2023, 12, 31)))
        .group_by(&db.post.owner_user)
        .select(&db.post.score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, a)| {
        let (n, s) = a.unwrap_or((0, 0));
        row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(n), or0(s, n)])
    }))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// PostHistoryStats AS (
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
// ps.PostType,
// ps.TotalPosts,
// ps.AverageScore,
// ps.TotalViews,
// COALESCE(AVG(pHS.RevisionCount), 0) AS AverageRevisions
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats pHS ON ps.PostType = (
// SELECT Name
// FROM PostTypes
// WHERE Id = (SELECT PostTypeId FROM Posts WHERE Id = pHS.PostId)
// )
// GROUP BY
// ps.PostType, ps.TotalPosts, ps.AverageScore, ps.TotalViews
// ORDER BY
// ps.TotalPosts DESC;
fn q10649(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), score.and(view_count.opt()), [0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let h = by_key(db.post.iq(), name(db), history_per_post(db), (0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mut v = Vec::new();
    (&f).and((&h).opt()).drive(|k, (a, h)| v.push((k, a, h)));
    rows(v.iter().map(|&(k, a, h)| {
        let (n, s) = h.unwrap_or((0, 0));
        row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), or0(s, n)])
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// pt.Name AS PostType,
// p.OwnerUserId
// FROM Posts p
// JOIN PostTypes pt ON p.PostTypeId = pt.Id
// ),
// VoteStats AS (
// SELECT
// v.PostId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes v
// JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY v.PostId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.Questions,
// ups.Answers,
// ups.TotalScore,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// ps.CommentCount,
// ps.PostType,
// vs.TotalVotes,
// vs.UpVotes,
// vs.DownVotes
// FROM UserPostStats ups
// JOIN PostStats ps ON ups.UserId = ps.OwnerUserId
// LEFT JOIN VoteStats vs ON ps.PostId = vs.PostId
// ORDER BY ups.TotalScore DESC, ups.TotalPosts DESC, ps.Score DESC;
fn q10652(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s],
        None => a,
    });
    let vs = db
        .vote
        .group_by(&db.vote.post)
        .select((&db.vote.vote_type).select(&db.vote_type.name))
        .fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&uf))).and((&vs).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, (u, a)), x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "type"]));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// Id AS UserId,
// Reputation,
// CreationDate,
// LastAccessDate,
// Views,
// UpVotes,
// DownVotes
// FROM
// Users
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.LastActivityDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.Tags,
// u.DisplayName AS OwnerDisplayName,
// p.OwnerUserId
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2022-01-01'
// ),
// VoteStatistics AS (
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
// us.UserId,
// us.Reputation,
// ps.PostId,
// ps.PostTypeId,
// ps.CreationDate,
// ps.LastActivityDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.Tags,
// ps.OwnerDisplayName,
// COALESCE(vs.UpVotes, 0) AS UpVotes,
// COALESCE(vs.DownVotes, 0) AS DownVotes
// FROM
// UserStatistics us
// JOIN
// PostStatistics ps ON us.UserId = ps.OwnerUserId
// LEFT JOIN
// VoteStatistics vs ON ps.PostId = vs.PostId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q10653(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, date(2022, 1, 1)).select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, u), d)| {
        let mut f = post_fields(db, p, &["uid", "rep", "id", "type_id", "created", "activity", "score", "views", "answers", "comments", "favorites", "tags", "owner"]);
        f.extend([V::I(u), V::I(d)]);
        f
    })
}

// WITH PostCounts AS (
// SELECT COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners,
// COUNT(DISTINCT Tags) AS UniqueTags
// FROM Posts
// ),
// UserCounts AS (
// SELECT COUNT(*) AS TotalUsers,
// SUM(CASE WHEN Reputation > 0 THEN 1 ELSE 0 END) AS ActiveUsers
// FROM Users
// ),
// CommentCounts AS (
// SELECT COUNT(*) AS TotalComments
// FROM Comments
// ),
// BadgeCounts AS (
// SELECT COUNT(*) AS TotalBadges
// FROM Badges
// ),
// VoteCounts AS (
// SELECT COUNT(*) AS TotalVotes
// FROM Votes
// )
// SELECT
// (SELECT TotalPosts FROM PostCounts) AS TotalPosts,
// (SELECT UniquePostOwners FROM PostCounts) AS UniquePostOwners,
// (SELECT UniqueTags FROM PostCounts) AS UniqueTags,
// (SELECT TotalUsers FROM UserCounts) AS TotalUsers,
// (SELECT ActiveUsers FROM UserCounts) AS ActiveUsers,
// (SELECT TotalComments FROM CommentCounts) AS TotalComments,
// (SELECT TotalBadges FROM BadgeCounts) AS TotalBadges,
// (SELECT TotalVotes FROM VoteCounts) AS TotalVotes;
fn q10654(db: &'static So) -> String {
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let tags = one(whole(db.post.iq()).select(&db.post.tags_str).count_distinct());
    let (un, act) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, a), r| (n + 1, a + (r > 0) as i64));
    row(vec![
        V::I(count(db.post.iq())),
        V::I(owners),
        V::I(tags),
        V::I(un),
        nullable(act, un),
        V::I(count(db.comment.iq())),
        V::I(count(db.badge.iq())),
        V::I(count(db.vote.iq())),
    ])
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 10 THEN 1 ELSE 0 END) AS DeletionVoteCount,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CreationDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate
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
// ps.PostId,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.DeletionVoteCount,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CreationDate,
// us.UserId,
// us.BadgeCount,
// us.TotalUpVotes,
// us.TotalDownVotes,
// us.TotalViews
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.Score DESC;
fn q10660(db: &'static So) -> String {
    let uid = uids(db);
    let User { up_votes, down_votes, views, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(views).and(badges_of(db).opt())).fold([0i64; 4], |a, (((u, d), w), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d, a[3] + w]);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), V::I(s.by_vt[10])]);
        f.extend(post_fields(db, p, &["score", "views", "answers", "created"]));
        f.push(user_col(db, u, "uid"));
        f.extend(ints(&a));
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
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount,
// AVG(p.CommentCount) AS AvgCommentCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// UserVoteStatistics AS (
// SELECT
// v.UserId AS UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM Votes v
// GROUP BY v.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalScore,
// ups.AvgViewCount,
// ups.AvgCommentCount,
// uvs.TotalVotes,
// uvs.Upvotes,
// uvs.Downvotes
// FROM UserPostStatistics ups
// LEFT JOIN UserVoteStatistics uvs ON ups.UserId = uvs.UserId
// ORDER BY ups.TotalPosts DESC, ups.TotalScore DESC;
fn q10661(db: &'static So) -> String {
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    (&uf).and((&vs).opt()).drive(|u, (a, x)| v.push((u, a, x)));
    rows(v.iter().map(|&(u, a, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "score_sum0", "views_avg", "cc_avg"].iter().map(|c| ustat_field(&a, c)));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.CommentCount) AS TotalComments,
// SUM(p.AnswerCount) AS TotalAnswers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostsCreated,
// SUM(v.BountyAmount) AS TotalBountyReceived
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
// ps.PostType,
// ps.TotalPosts,
// ps.AverageScore,
// ps.UniqueUsers,
// ps.TotalViews,
// ps.TotalComments,
// ps.TotalAnswers,
// us.UserId,
// us.DisplayName,
// us.PostsCreated,
// us.TotalBountyReceived
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.UniqueUsers = us.PostsCreated
// ORDER BY
// TotalPosts DESC;
fn q10667(db: &'static So) -> String {
    let Post { score, view_count, comment_count, answer_count, owner_user_id, .. } = &db.post;
    let pf = by_key(db.post.iq(), name(db), score.and(view_count.opt()).and(comment_count).and(answer_count.opt()), [0i64; 7], |a, (((s, w), cc), an)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + cc, a[5] + an.is_some() as i64, a[6] + an.unwrap_or(0)]
    });
    let owners = db.post.group_by(name(db)).select(owner_user_id).count_distinct();
    let uf = g(db).select(posts_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 3], |a, (p, b)| {
        let b = b.flatten();
        [a[0] + p.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
    });
    let by_posts: HashIdx<i64, Id<User>> = (&uf).map(|a: [i64; 3]| a[0]).inv().collect();
    let unique = || (&owners).opt().map(|o: Option<i64>| o.unwrap_or(0));
    let mut v = Vec::new();
    (&pf).and(unique()).and(unique().select(&by_posts).select(Ident::<User>::new().and(&uf))).drive(|k, ((a, o), (u, b))| v.push((k, a, o, u, b)));
    rows(v.iter().map(|&(k, a, o, u, b)| {
        row(vec![
            V::S(k),
            V::I(a[0]),
            avg(a[1], a[0]),
            V::I(o),
            nullable(a[3], a[2]),
            V::I(a[4]),
            nullable(a[6], a[5]),
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(b[0]),
            nullable(b[2], b[1]),
        ])
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COALESCE(v.UpVotesCount, 0) AS UpVotes,
// COALESCE(v.DownVotesCount, 0) AS DownVotes,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10675(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, u), d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "rep"]);
        f.extend([V::I(u), V::I(d)]);
        f.extend(post_fields(db, p, &["views", "answers", "comments", "favorites"]));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN v.VoteTypeId = 5 THEN 1 ELSE 0 END) AS Favorites,
// SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END) AS CloseVotes,
// SUM(CASE WHEN v.VoteTypeId = 7 THEN 1 ELSE 0 END) AS ReopenVotes,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalCloseReasons,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 ELSE 0 END) AS TotalDeleteUndeleteEvents
// FROM
// Posts p
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// PostHistory ph ON ph.PostId = p.Id
// GROUP BY
// p.Id, p.OwnerUserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(ps.PostId) AS TotalPosts,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.UpVotes) AS TotalUpVotes,
// SUM(ps.DownVotes) AS TotalDownVotes,
// SUM(ps.Favorites) AS TotalFavorites,
// SUM(ps.CloseVotes) AS TotalCloseVotes,
// SUM(ps.ReopenVotes) AS TotalReopenVotes,
// SUM(ps.TotalCloseReasons) AS TotalCloseReasons,
// SUM(ps.TotalDeleteUndeleteEvents) AS TotalDeleteUndeleteEvents
// FROM
// Users u
// LEFT JOIN
// PostStats ps ON ps.OwnerUserId = u.Id
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q10679(db: &'static So) -> String {
    let pf = owned(db)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 8], |a, ((c, t), h)| {
            [
                a[0] + c.is_some() as i64,
                a[1] + (t == Some(2)) as i64,
                a[2] + (t == Some(3)) as i64,
                a[3] + (t == Some(5)) as i64,
                a[4] + (t == Some(6)) as i64,
                a[5] + (t == Some(7)) as i64,
                a[6] + (h == Some(10)) as i64,
                a[7] + matches!(h, Some(12 | 13)) as i64,
            ]
        });
    let uf = g(db).select(posts_of(db).select(&pf).opt()).fold((0i64, [0i64; 8]), |(n, s), x| match x {
        Some(a) => (n + 1, std::array::from_fn(|i| s[i] + a[i])),
        None => (n, s),
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, (n, _))| Reverse(n), 100, |&(u, (n, s))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n)];
        f.extend(s.iter().map(|&x| nullable(x, n)));
        f
    })
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AveragePostScore,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
// FROM Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// SUM(Reputation) AS TotalReputation,
// AVG(Reputation) AS AverageUserReputation
// FROM Users
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// COUNT(DISTINCT PostId) AS UniqueVotedPosts
// FROM Votes
// ),
// CommentStats AS (
// SELECT
// COUNT(*) AS TotalComments,
// COUNT(DISTINCT PostId) AS UniqueCommentedPosts
// FROM Comments
// )
// SELECT
// p.TotalPosts,
// p.AveragePostScore,
// p.UniquePostOwners,
// p.TotalQuestions,
// p.TotalAnswers,
// u.TotalUsers,
// u.TotalReputation,
// u.AverageUserReputation,
// v.TotalVotes,
// v.UniqueVotedPosts,
// c.TotalComments,
// c.UniqueCommentedPosts
// FROM
// PostStats p,
// UserStats u,
// VoteStats v,
// CommentStats c;
fn q10705(db: &'static So) -> String {
    let p = db.post.select((&db.post.score).and(&db.post.post_type_id)).fold_flat([0i64; 4], |a, (s, t)| [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64]);
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let voted = one(whole(db.vote.iq()).select(&db.vote.post_id).count_distinct());
    let commented = one(whole(db.comment.iq()).select(&db.comment.post_id).count_distinct());
    row(vec![
        V::I(p[0]),
        avg(p[1], p[0]),
        V::I(owners),
        nullable(p[2], p[0]),
        nullable(p[3], p[0]),
        V::I(un),
        nullable(rs, un),
        avg(rs, un),
        V::I(count(db.vote.iq())),
        V::I(voted),
        V::I(count(db.comment.iq())),
        V::I(commented),
    ])
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
// COUNT(v.Id) AS VoteCount,
// u.Id AS OwnerUserId,
// u.Reputation AS OwnerReputation,
// u.CreationDate AS UserCreationDate,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.Id, u.Reputation, u.CreationDate, u.DisplayName
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(v.BountyAmount) AS TotalBounty
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
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
// ps.VoteCount,
// ps.OwnerUserId,
// ps.OwnerReputation,
// ps.UserCreationDate,
// ps.OwnerDisplayName,
// us.UserId,
// us.DisplayName AS UserDisplayName,
// us.BadgeCount,
// us.UpVotes,
// us.DownVotes,
// us.TotalBounty
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerUserId = us.UserId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q10707(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let us = g(db).select(badges_of(db).opt().and(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt())).fold([0i64; 5], |a, (b, x)| {
        let (t, bo) = x.map_or((None, None), |(t, bo)| (Some(t), bo));
        [a[0] + b.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + bo.is_some() as i64, a[4] + bo.unwrap_or(0)]
    });
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, _, _)| newest(db, p), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(s.cx), V::I(s.vx)]);
        f.extend(["uid", "rep", "ucreated", "name", "uid", "name"].iter().map(|c| user_col(db, u, c)));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3])]);
        f
    })
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
// ),
// AggregatedData AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.CreationDate,
// COALESCE(up.PostCount, 0) AS PostCount,
// COALESCE(up.QuestionCount, 0) AS QuestionCount,
// COALESCE(up.AnswerCount, 0) AS AnswerCount,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// UserPostCounts up ON u.Id = up.UserId
// LEFT JOIN
// UserBadgeCounts ub ON u.Id = ub.UserId
// )
// SELECT
// UserId,
// Reputation,
// CreationDate,
// PostCount,
// QuestionCount,
// AnswerCount,
// BadgeCount
// FROM
// AggregatedData
// ORDER BY
// Reputation DESC
// LIMIT 100;
fn q10710(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    upqa(db).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(u, _, _)| rep_desc(db, u), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), user_col(db, u, "ucreated")];
        f.extend(ints(&a[..3]));
        f.push(V::I(b));
        f
    })
}

// WITH PostScore AS (
// SELECT
// p.Id AS PostId,
// p.Score AS PostScore,
// COUNT(c.Id) AS CommentCount,
// u.Reputation AS UserReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Score, u.Reputation
// ),
// AverageStats AS (
// SELECT
// AVG(PostScore) AS AvgPostScore,
// SUM(CommentCount) AS TotalComments,
// AVG(UserReputation) AS AvgUserReputation
// FROM
// PostScore
// )
// SELECT
// *
// FROM
// AverageStats;
fn q10714(db: &'static So) -> String {
    let a = since(db, ts(2024, 9, 1, 12, 34, 56))
        .select((&db.post.score).and(comments_per_post(db)).and((&db.post.owner_user).select(&db.user.reputation).opt()))
        .fold_flat([0i64; 5], |a, ((s, c), r)| [a[0] + 1, a[1] + s, a[2] + c, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0)]);
    row(vec![avg(a[1], a[0]), nullable(a[2], a[0]), avg(a[4], a[3])])
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(Id) AS CommentCount
// FROM Comments
// GROUP BY PostId) AS c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10715(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_per_post(db)).and(&db.post.creation_date), [0i64, 0, 0, i64::MIN], |a, ((s, c), d)| {
        [a[0] + 1, a[1] + s, a[2] + c, a[3].max(d)]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::T(a[3])])))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// WHERE
// p.CreationDate >= DATE('2024-10-01') - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC, p.Score DESC;
fn q10716(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    rows(stats_with(db, since(db, date(2024, 9, 1)), "cv", &[], &[&c, &x]).iter().map(|&(p, s, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(match db.post.owner_user.get(p) {
            Some(u) => V::S(db.user.display_name.get(u).unwrap()),
            None => V::S("Community"),
        });
        f.extend([V::I(d[0]), V::I(d[1]), V::I(s.upn), V::I(s.downn)]);
        row(f)
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(COALESCE(C.CommentCount, 0)) AS TotalComments,
// AVG(COALESCE(P.Score, 0)) AS AveragePostScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// TotalPosts DESC, AveragePostScore DESC
// FETCH FIRST 100 ROWS ONLY;
fn q10724(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db)
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comments_per_post(db)).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 9], |mut a, p| {
            a[0] += 1;
            if let Some(((((t, s), w), c), x)) = p {
                a[1] += 1;
                a[2] += (t == 1) as i64;
                a[3] += (t == 2) as i64;
                a[4] += (x == Some(2)) as i64;
                a[5] += (x == Some(3)) as i64;
                a[6] += c;
                a[7] += s;
                a[8] += w.unwrap_or(0);
            }
            a
        });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| (Reverse(a[1]), Reverse(fkey(a[7] as f64 / a[0] as f64))), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[1..7]));
        f.extend([V::F(a[7] as f64 / a[0] as f64), V::I(a[8])]);
        f
    })
}

// WITH PostCounts AS (
// SELECT
// PostTypeId,
// COUNT(*) AS TotalPosts
// FROM
// Posts
// GROUP BY
// PostTypeId
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
// UserStats AS (
// SELECT
// Id AS UserId,
// Reputation,
// COALESCE(BadgeCount, 0) AS BadgeCount
// FROM
// Users
// LEFT JOIN
// UserBadges ON Users.Id = UserBadges.UserId
// )
// SELECT
// p.PostTypeId,
// pc.TotalPosts,
// us.UserId,
// us.Reputation,
// us.BadgeCount
// FROM
// PostCounts pc
// JOIN
// Posts p ON pc.PostTypeId = p.PostTypeId
// JOIN
// UserStats us ON p.OwnerUserId = us.UserId
// ORDER BY
// pc.TotalPosts DESC, us.Reputation DESC;
fn q10725(db: &'static So) -> String {
    let pc = db.post.group_by(&db.post.post_type_id).fold(0i64, |a, _| a + 1);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.post_type_id).select(&pc)).and((&db.post.owner_user).select(&bu))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, n), b)| {
        let mut f = post_fields(db, p, &["type_id"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["uid", "rep"]));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// ),
// PostStatistics AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(ViewCount) AS TotalViews,
// SUM(Score) AS TotalScore
// FROM Posts
// GROUP BY OwnerUserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// PS.TotalPosts,
// PS.TotalQuestions,
// PS.TotalAnswers,
// PS.TotalViews,
// PS.TotalScore
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// ORDER BY U.Reputation DESC, PS.TotalScore DESC;
fn q10729(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let pf = owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(view_count.opt()).and(score))
        .fold([0i64; 6], |a, ((t, w), s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        match p {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5])]),
            None => f.extend(nulls(5)),
        }
        row(f)
    }))
}

// WITH UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(b.BadgeCount, 0)) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
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
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// TotalVotes,
// TotalComments,
// TotalBadges
// FROM
// UserEngagement
// ORDER BY
// Reputation DESC, PostCount DESC
// FETCH FIRST 100 ROWS ONLY;
fn q10734(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let uf = g(db).select(posts_of(db).select(votes_per_post(db).and(comments_per_post(db))).opt().and(&bu)).fold([0i64; 4], |a, (p, b)| match p {
        Some((x, c)) => [a[0] + 1, a[1] + x, a[2] + c, a[3] + b],
        None => [a[0], a[1], a[2], a[3] + b],
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(u, a)| (rep_desc(db, u), Reverse(a[0])), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f
    })
}

// WITH PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalPostOwners
// FROM
// Posts
// ),
// UserCounts AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation
// FROM
// Users
// ),
// VoteCounts AS (
// SELECT
// COUNT(*) AS TotalVotes,
// COUNT(DISTINCT PostId) AS VotedPosts,
// AVG(VoteTypeId) AS AverageVoteType
// FROM
// Votes
// )
// SELECT
// pc.TotalPosts,
// pc.TotalPostOwners,
// uc.TotalUsers,
// uc.AverageReputation,
// vc.TotalVotes,
// vc.VotedPosts,
// vc.AverageVoteType
// FROM
// PostCounts pc,
// UserCounts uc,
// VoteCounts vc;
fn q10735(db: &'static So) -> String {
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let (xn, xs) = db.vote.select(&db.vote.vote_type_id).fold_flat((0i64, 0i64), |(n, s), t| (n + 1, s + t));
    let voted = one(whole(db.vote.iq()).select(&db.vote.post_id).count_distinct());
    row(vec![V::I(count(db.post.iq())), V::I(owners), V::I(un), avg(rs, un), V::I(xn), V::I(voted), avg(xs, xn)])
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(v.VoteCount) AS TotalVotes,
// AVG(p.Score) AS AverageScore,
// (SELECT COUNT(DISTINCT u.Id) FROM Users u) AS TotalUsers,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId) v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10743(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let f = by_key(
        db.post.iq(),
        name(db),
        (&db.post.score).and((&vf).opt()).and((&db.post.owner_user).select(&db.user.reputation).opt()),
        [0i64; 6],
        |a, ((s, x), r)| [a[0] + 1, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0), a[3] + s, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)],
    );
    let tu = whole(db.user.iq()).select(Ident::<User>::new()).count_distinct();
    let one = rel(vec![()]).select((&tu).opt());
    let mut v = Vec::new();
    (&f).cross(&one).drive(|(k, _), (a, tu)| v.push((k, a, tu.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, tu)| row(vec![V::S(k), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), V::I(tu), avg(a[5], a[4])])))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// UpVotes,
// DownVotes,
// TotalComments,
// TotalBadges
// FROM UserActivity
// ORDER BY TotalPosts DESC
// LIMIT 100;
fn q10748(db: &'static So) -> String {
    let cols = ["uid", "name", "#rows", "#q", "#a", "#up", "#down", "#cx", "#bx"];
    out(users_with_counts(db, "cvb", false), |r| Reverse(r.agg.prows), 100, |r| user_fields(r, "cvb", &cols))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
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
// u.UserId,
// u.Reputation,
// u.TotalPosts,
// u.Questions,
// u.Answers,
// u.TotalViews,
// u.TotalScore,
// COALESCE(b.TotalBadges, 0) AS TotalBadges,
// COALESCE(b.GoldBadges, 0) AS GoldBadges,
// COALESCE(b.SilverBadges, 0) AS SilverBadges,
// COALESCE(b.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats u
// LEFT JOIN
// UserBadgeStats b ON u.UserId = b.UserId
// ORDER BY
// u.TotalScore DESC, u.Reputation DESC;
fn q10751(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), V::I(a[5])]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// p.Score,
// p.ViewCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// EXTRACT(EPOCH FROM (CURRENT_TIMESTAMP - p.CreationDate)) AS PostAgeInSeconds
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate > CURRENT_DATE - INTERVAL '30 days'
// GROUP BY
// p.Id, p.PostTypeId, p.Score, p.ViewCount, p.CreationDate
// )
// SELECT
// pt.Name AS PostType,
// COUNT(ps.PostId) AS TotalPosts,
// AVG(ps.CommentCount) AS AvgCommentsPerPost,
// AVG(ps.VoteCount) AS AvgVotesPerPost,
// AVG(ps.Score) AS AvgScore,
// AVG(ps.ViewCount) AS AvgViewCount,
// AVG(ps.PostAgeInSeconds) AS AvgPostAgeInSeconds
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10756(db: &'static So) -> String {
    let now = utc_to_ny(now_utc());
    let base = || db.post.with((&db.post.creation_date).gt(add_days(current_date(), -30)));
    let bid = badges_by_uid(db);
    let pf = base()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and((&db.post.owner_user_id).select(&bid).opt()))
        .fold((0i64, 0i64), |(c, v), ((ci, vi), _)| (c + ci.is_some() as i64, v + vi.is_some() as i64));
    let f = base()
        .group_by(name(db))
        .select((&pf).and(&db.post.score).and((&db.post.view_count).opt()).and(&db.post.creation_date))
        .fold(([0i64; 6], (0f64, 0f64)), |(a, age), ((((c, x), sc), w), cd)| {
            let us = tz_sub(now, cd);
            let e = (us.div_euclid(86_400_000_000)) as f64 * 86400.0 + us.rem_euclid(86_400_000_000) as f64 / 1e6;
            ([a[0] + 1, a[1] + c, a[2] + x, a[3] + sc, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)], kahan(age, e))
        });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, (a, age))| {
        row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[2], a[0]), avg(a[3], a[0]), avg(a[5], a[4]), fmean(age, a[0])])
    }))
}

// WITH RankedPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.OwnerUserId,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation
// FROM
// Users u
// )
// SELECT
// rp.PostId,
// rp.Title,
// rp.CreationDate,
// ur.Reputation,
// rp.Score,
// rp.ViewCount,
// rp.CommentCount,
// rp.UpVoteCount,
// rp.DownVoteCount
// FROM
// RankedPosts rp
// LEFT JOIN
// UserReputation ur ON rp.OwnerUserId = ur.UserId
// ORDER BY
// rp.ViewCount DESC;
fn q10757(db: &'static So) -> String {
    stats_rows(db, stats_with(db, since(db, year_ago()), "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "rep", "score", "views", "#cx", "#up", "#down"])
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.PostCount,
// U.CommentCount,
// U.UpVotes,
// U.DownVotes,
// (U.UpVotes - U.DownVotes) AS NetVotes
// FROM
// UserStats U
// WHERE
// U.PostCount > 0
// ORDER BY
// U.Reputation DESC,
// NetVotes DESC;
fn q10768(db: &'static So) -> String {
    let (np, nc) = (ud(db, UserWhere::All, posts_of(db)), ud(db, UserWhere::All, posts_of(db).select(comments_of(db))));
    let uv = g(db).select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 2], |a, p| match p {
        Some((_, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
        None => a,
    });
    let mut v = Vec::new();
    (&uv).and((&np).filt(|n| n > 0)).and((&nc).opt()).drive(|u, ((a, n), c)| v.push((u, a, n, c.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, n, c)| {
        let mut f: Vec<V> = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect();
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS PostCount,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// ActiveUsers AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// up.PostCount,
// up.QuestionCount,
// up.AnswerCount
// FROM
// Users u
// JOIN
// UserPostCounts up ON u.Id = up.OwnerUserId
// WHERE
// u.Reputation > 0
// )
// SELECT
// au.UserId,
// au.DisplayName,
// au.Reputation,
// au.PostCount,
// au.QuestionCount,
// au.AnswerCount,
// COUNT(c.Id) AS CommentCount,
// SUM(v.BountyAmount) AS TotalBountyAmount
// FROM
// ActiveUsers au
// LEFT JOIN
// Comments c ON c.UserId = au.UserId
// LEFT JOIN
// Votes v ON v.UserId = au.UserId
// GROUP BY
// au.UserId, au.DisplayName, au.Reputation, au.PostCount, au.QuestionCount, au.AnswerCount
// ORDER BY
// au.Reputation DESC, au.PostCount DESC;
fn q10785(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let uf = user_base(db, UserWhere::RepGt(0))
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let mut v = Vec::new();
    (&uf).and(&pc).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&p));
        f.extend([V::I(a[0]), nullable(a[2], a[1])]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
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
// ),
// FinalStats AS (
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
// )
// SELECT
// *
// FROM
// FinalStats
// ORDER BY
// TotalScore DESC,
// TotalPosts DESC
// LIMIT 100;
fn q10791(db: &'static So) -> String {
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

// WITH UserVoteStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// TopPosts AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount
// FROM
// Posts P
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.Score DESC, P.ViewCount DESC
// LIMIT 10
// )
// SELECT
// U.DisplayName,
// U.TotalVotes,
// U.UpVotes,
// U.DownVotes,
// T.Title AS TopPostTitle,
// T.Score AS TopPostScore,
// T.ViewCount AS TopPostViews,
// T.AnswerCount AS TopPostAnswers,
// T.CommentCount AS TopPostComments
// FROM
// UserVoteStatistics U
// JOIN
// TopPosts T ON T.PostId IN (SELECT P.Id FROM Posts P WHERE P.OwnerUserId = U.UserId)
// ORDER BY
// U.TotalVotes DESC
// LIMIT 10;
fn q10794(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let top: MatSet<Id<Post>> = whole(questions_only(db))
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(_, k)| k, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let uv = user_votes(db);
    let mut v = Vec::new();
    (&top).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&uv)))).drive(|_, x| v.push(x));
    out(v, |&(_, (_, a))| Reverse(a[0]), 10, |&(p, (u, a))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["title", "score", "views", "answers", "comments"]));
        f
    })
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10543", q10543),
    ("10547", q10547),
    ("10549", q10549),
    ("10550", q10550),
    ("10552", q10552),
    ("10553", q10553),
    ("10563", q10563),
    ("10566", q10566),
    ("10569", q10569),
    ("10583", q10583),
    ("10591", q10591),
    ("10592", q10592),
    ("10597", q10597),
    ("10600", q10600),
    ("10605", q10605),
    ("10610", q10610),
    ("10611", q10611),
    ("10621", q10621),
    ("10631", q10631),
    ("10635", q10635),
    ("10640", q10640),
    ("10643", q10643),
    ("10646", q10646),
    ("10649", q10649),
    ("10652", q10652),
    ("10653", q10653),
    ("10654", q10654),
    ("10660", q10660),
    ("10661", q10661),
    ("10667", q10667),
    ("10675", q10675),
    ("10679", q10679),
    ("10705", q10705),
    ("10707", q10707),
    ("10710", q10710),
    ("10714", q10714),
    ("10715", q10715),
    ("10716", q10716),
    ("10724", q10724),
    ("10725", q10725),
    ("10729", q10729),
    ("10734", q10734),
    ("10735", q10735),
    ("10743", q10743),
    ("10748", q10748),
    ("10751", q10751),
    ("10756", q10756),
    ("10757", q10757),
    ("10768", q10768),
    ("10785", q10785),
    ("10791", q10791),
    ("10794", q10794),
];
