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

fn badges_by_uid(db: &'static So) -> HashIdx<i64, Id<Badge>> {
    (&db.badge.user_id).inv().collect()
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

// --- batch 123 --------------------------------------------------------------

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(p.ViewCount) AS AvgViewCount,
// AVG(p.Score) AS AvgScore
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
// ups.Questions,
// ups.Answers,
// ups.AvgViewCount,
// ups.AvgScore,
// bs.TotalBadges,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// BadgeStats bs ON ups.UserId = bs.UserId
// ORDER BY
// ups.TotalPosts DESC
// LIMIT 100;
fn q13411(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([avg(a[5], a[4]), avg(a[3], a[0])]);
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
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
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id) AS VoteCount,
// (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = p.Id) AS HistoryCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
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
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13496(db: &'static So) -> String {
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and(votes_per_post(db)).and(history_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| newest(db, p), 100, |&((((p, a), c), x), h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a), V::I(c), V::I(x), V::I(h)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// MAX(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVote,
// MAX(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVote,
// MAX(p.CreationDate) AS LastActivityDate
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
// COUNT(b.Id) AS BadgeCount,
// SUM(u.Views) AS TotalViews,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// PostTypeBreakdown AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.VoteCount) AS TotalVotes
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON pt.Id = p.PostTypeId
// LEFT JOIN
// PostStats ps ON p.Id = ps.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// u.UserId,
// u.BadgeCount,
// u.TotalViews,
// u.TotalUpVotes,
// u.TotalDownVotes,
// pt.PostType,
// pt.TotalPosts,
// pt.TotalComments,
// pt.TotalVotes
// FROM
// UserStats u
// JOIN
// PostTypeBreakdown pt ON u.UserId = pt.TotalPosts
// ORDER BY
// u.TotalViews DESC,
// pt.TotalPosts DESC;
fn q13504(db: &'static So) -> String {
    let uid = uids(db);
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let x = per_post_distinct(db, votes_of(db));
    let pf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let User { views, up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(views.and(up_votes).and(down_votes).and(badges_of(db).opt())).fold([0i64; 4], |a, (((w, u), d), b)| [a[0] + b.is_some() as i64, a[1] + w, a[2] + u, a[3] + d]);
    let pt = db.post_type.group_by(&db.post_type.name).select((&of_type).select((&pf).and((&x).opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, x)) => [a[0] + 1, a[1] + s.cx, a[2] + x.unwrap_or(0)],
        None => a,
    });
    let mut v = Vec::new();
    (&pt).and((&pt).map(|a: [i64; 3]| a[0]).select(&uid).select(Ident::<User>::new().and(&us))).drive(|t, (a, (u, s))| v.push((t, a, u, s)));
    rows(v.iter().map(|&(t, a, u, s)| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(ints(&s));
        f.extend([V::S(t), V::I(a[0]), nullable(a[1], a[0]), nullable(a[2], a[0])]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName
// ),
// UserBadgeStats AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS TotalBadges,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.Questions,
// UPS.Answers,
// UPS.TotalScore,
// UPS.TotalViews,
// COALESCE(UBS.TotalBadges, 0) AS TotalBadges,
// COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges
// FROM UserPostStats UPS
// LEFT JOIN UserBadgeStats UBS ON UPS.UserId = UBS.UserId
// ORDER BY UPS.TotalPosts DESC, UPS.TotalScore DESC;
fn q13508(db: &'static So) -> String {
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

// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.ViewCount) AS AverageViews,
// AVG(p.Score) AS AverageScore,
// MIN(p.CreationDate) AS EarliestPost,
// MAX(p.CreationDate) AS LatestPost
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
// GROUP BY
// p.PostTypeId
// ORDER BY
// TotalPosts DESC;
fn q13513(db: &'static So) -> String {
    let Post { accepted_answer_id, score, view_count, creation_date, .. } = &db.post;
    let f = by_key(owned(db), &db.post.post_type_id, accepted_answer_id.opt().and(score).and(view_count.opt()).and(creation_date), [0, 0, 0, 0, 0, i64::MAX, i64::MIN], |a: [i64; 7], (((ac, s), w), c)| {
        [a[0] + 1, a[1] + ac.is_some() as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5].min(c), a[6].max(c)]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::I(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), avg(a[4], a[3]), avg(a[2], a[0]), V::T(a[5]), V::T(a[6])])))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers,
// SUM(COALESCE(p.CommentCount, 0)) AS TotalComments,
// AVG(u.Reputation) AS AverageReputation
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
// p.LastActivityDate,
// EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) / 60 AS ActivityDurationMinutes,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.OwnerUserId
// FROM
// Posts p
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.TotalScore,
// ups.TotalViews,
// ups.TotalAnswers,
// ups.TotalComments,
// ups.AverageReputation,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.LastActivityDate,
// ps.ActivityDurationMinutes,
// ps.Score AS PostScore,
// ps.ViewCount AS PostViewCount,
// ps.AnswerCount AS PostAnswerCount,
// ps.CommentCount AS PostCommentCount,
// ps.FavoriteCount AS PostFavoriteCount
// FROM
// UserPostStats ups
// JOIN
// PostStats ps ON ups.UserId = ps.OwnerUserId
// ORDER BY
// ups.PostCount DESC, ups.TotalScore DESC;
fn q13517(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count)).opt()).fold([0i64; 5], |a, p| match p {
        Some((((s, w), an), cc)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + an.unwrap_or(0), a[4] + cc],
        None => a,
    });
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&uf)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, (u, a))| {
        let (cd, la) = (db.post.creation_date.get(p).unwrap(), db.post.last_activity_date.get(p).unwrap());
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f.extend(post_fields(db, p, &["id", "title", "created", "activity"]));
        f.push(V::F(hours_to(la, cd) / 60.0));
        f.extend(post_fields(db, p, &["score", "views", "answers", "comments", "favorites"]));
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
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
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
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.TotalQuestions,
// u.TotalAnswers,
// u.TotalViews,
// u.TotalScore,
// COALESCE(b.TotalBadges, 0) AS TotalBadges
// FROM
// UserPostStats u
// LEFT JOIN
// UserBadgeStats b ON u.UserId = b.UserId
// ORDER BY
// u.TotalScore DESC, u.TotalPosts DESC;
fn q13527(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    upqa(db).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), nullable(a[5], a[0]), V::I(b)]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// AVG(COALESCE(P.ViewCount, 0)) AS AvgViewCount,
// MAX(P.CreationDate) AS LastPostDate
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
// UPS.UserId,
// UPS.DisplayName,
// UPS.PostCount,
// UPS.QuestionCount,
// UPS.AnswerCount,
// UPS.TotalScore,
// UPS.AvgViewCount,
// UPS.LastPostDate,
// COALESCE(UBS.BadgeCount, 0) AS BadgeCount,
// COALESCE(UBS.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(UBS.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(UBS.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM
// UserPostStats UPS
// LEFT JOIN
// UserBadgeStats UBS ON UPS.UserId = UBS.UserId
// ORDER BY
// UPS.TotalScore DESC
// LIMIT 10;
fn q13536(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).opt()).fold([0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 7], p| match p {
        Some((((t, s), w), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + 1, a[5] + w.unwrap_or(0), a[6].max(c)],
        None => [a[0], a[1], a[2], a[3], a[4] + 1, a[5], a[6]],
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| Reverse(a[3]), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.extend([avg(a[5], a[4]), if a[0] == 0 { V::Null } else { V::T(a[6]) }]);
        f.extend(ints(&b));
        f
    })
}

// WITH PostVoteCounts AS (
// SELECT
// p.Id AS PostId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(CASE WHEN v.VoteTypeId = 6 THEN 1 END) AS CloseVotes,
// COUNT(CASE WHEN v.VoteTypeId = 7 THEN 1 END) AS ReopenVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id
// ), PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COALESCE(pvc.UpVotes, 0) AS UpVotes,
// COALESCE(pvc.DownVotes, 0) AS DownVotes,
// COALESCE(pvc.CloseVotes, 0) AS CloseVotes,
// COALESCE(pvc.ReopenVotes, 0) AS ReopenVotes,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount
// FROM
// Posts p
// LEFT JOIN
// PostVoteCounts pvc ON p.Id = pvc.PostId
// )
// SELECT
// pm.PostId,
// pm.Title,
// pm.CreationDate,
// pm.ViewCount,
// pm.UpVotes,
// pm.DownVotes,
// pm.CloseVotes,
// pm.ReopenVotes,
// pm.AnswerCount,
// pm.CommentCount,
// pm.FavoriteCount
// FROM
// PostMetrics pm
// ORDER BY
// pm.ViewCount DESC, pm.UpVotes DESC;
fn q13541(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "v", &[]).drive(|p, s| v.push((p, s)));
    rows(v.iter().map(|&(p, s)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(s.up), V::I(s.down), V::I(s.by_vt[6]), V::I(s.by_vt[7])]);
        f.extend(post_fields(db, p, &["answers", "comments", "favorites"]));
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
// U.Reputation AS OwnerReputation,
// U.DisplayName AS OwnerDisplayName
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// ),
// VoteStats AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId
// ),
// PostEngagement AS (
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.OwnerReputation,
// PS.OwnerDisplayName,
// COALESCE(VS.UpVotes, 0) AS UpVotes,
// COALESCE(VS.DownVotes, 0) AS DownVotes
// FROM
// PostStats PS
// LEFT JOIN
// VoteStats VS ON PS.PostId = VS.PostId
// )
// SELECT
// PostTypeId,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// AVG(UpVotes) AS AvgUpVotes,
// AVG(DownVotes) AS AvgDownVotes,
// AVG(AnswerCount) AS AvgAnswerCount,
// AVG(CommentCount) AS AvgCommentCount,
// COUNT(*) AS TotalPosts
// FROM
// PostEngagement
// GROUP BY
// PostTypeId
// ORDER BY
// PostTypeId;
fn q13542(db: &'static So) -> String {
    let Post { view_count, score, answer_count, comment_count, .. } = &db.post;
    let pv = post_votes(db);
    let f = by_key(owned(db), &db.post.post_type_id, view_count.opt().and(score).and((&pv).opt()).and(answer_count.opt()).and(comment_count), [0i64; 9], |a, ((((w, s), x), an), cc)| {
        let x = x.unwrap_or([0; 3]);
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + x[1], a[5] + x[2], a[6] + an.is_some() as i64, a[7] + an.unwrap_or(0), a[8] + cc]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::I(k), avg(a[2], a[1]), avg(a[3], a[0]), avg(a[4], a[0]), avg(a[5], a[0]), avg(a[7], a[6]), avg(a[8], a[0]), V::I(a[0])])))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount
// ),
// UserPostActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostsCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount
// FROM
// Users u
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
// ps.FavoriteCount,
// ps.VoteCount,
// upa.UserId,
// upa.DisplayName AS UserDisplayName,
// upa.PostsCount,
// upa.TotalViews,
// upa.TotalScore,
// upa.QuestionsCount,
// upa.AnswersCount
// FROM
// PostStatistics ps
// JOIN
// UserPostActivity upa ON ps.PostId = upa.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q13543(db: &'static So) -> String {
    let uid = uids(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + (t == 1) as i64, a[5] + (t == 2) as i64],
        None => a,
    });
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "vc", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uf)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "favorites"]);
        f.extend([V::I(s.vx), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), nullable(a[3], a[0]), V::I(a[4]), V::I(a[5])]);
        row(f)
    }))
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
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
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName
// )
// SELECT
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.ViewCount,
// pd.OwnerDisplayName,
// pd.CommentCount,
// pd.UpVotes,
// pd.DownVotes,
// (pd.UpVotes - pd.DownVotes) AS NetVotes
// FROM
// PostDetails pd
// ORDER BY
// pd.ViewCount DESC
// LIMIT 100;
fn q13561(db: &'static So) -> String {
    out(stats_with(db, questions_only(db), "cv", &[], &[]), |&(p, _, _)| views_desc(db, p), 100, |&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(s.up - s.down)]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS Questions,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS Answers,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END), 0) AS Wikis,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 7 THEN 1 ELSE 0 END), 0) AS WikiPlaceholders,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 4 THEN 1 ELSE 0 END), 0) AS TagWikis
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostEngagement AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
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
// p.Id, p.Title, p.ViewCount, p.Score
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.Questions,
// ups.Answers,
// ups.Wikis,
// ups.WikiPlaceholders,
// ups.TagWikis,
// pe.PostId,
// pe.Title,
// pe.ViewCount,
// pe.Score,
// pe.CommentCount,
// pe.UpVotes,
// pe.DownVotes
// FROM
// UserPostStats ups
// LEFT JOIN
// PostEngagement pe ON ups.UserId = pe.PostId
// ORDER BY
// ups.TotalPosts DESC, pe.ViewCount DESC;
fn q13575(db: &'static So) -> String {
    let pid = pids(db);
    let uf = g(db).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 6], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + (t == 7) as i64, a[5] + (t == 4) as i64],
        None => a,
    });
    let pe = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&uf).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&pe)).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        match p {
            Some((p, s)) => {
                f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
                f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
            }
            None => f.extend(nulls(7)),
        }
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
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts,
// AVG(P.Score) AS AvgScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// PH.UserId,
// COUNT(PH.Id) AS TotalChanges,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (2, 4, 5) THEN 1 ELSE 0 END) AS Edits,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS Closures
// FROM
// PostHistory PH
// GROUP BY
// PH.UserId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.Questions,
// UPS.Answers,
// UPS.UpvotedPosts,
// UPS.AvgScore,
// PHS.TotalChanges,
// PHS.Edits,
// PHS.Closures
// FROM
// UserPostStats UPS
// LEFT JOIN
// PostHistoryStats PHS ON UPS.UserId = PHS.UserId
// ORDER BY
// UPS.TotalPosts DESC;
fn q13579(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + s],
        None => a,
    });
    let hf = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| {
        [a[0] + 1, a[1] + matches!(t, 2 | 4 | 5) as i64, a[2] + matches!(t, 10 | 11) as i64]
    });
    let mut v = Vec::new();
    (&uf).and((&hf).opt()).drive(|u, (a, h)| v.push((u, a, h)));
    rows(v.iter().map(|&(u, a, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.push(avg(a[4], a[0]));
        f.extend((0..3).map(|i| oint(h.map(|h| h[i]))));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
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
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(v.BountyAmount) AS TotalBountyAmount
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
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.TotalBountyAmount
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStats us ON us.UserId = u.Id
// ORDER BY
// ps.ViewCount DESC, ps.VoteCount DESC;
fn q13585(db: &'static So) -> String {
    let uid = uids(db);
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let bu = badges_per_user(db);
    let ub = g(db).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 2], |a, (_, b)| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]
    });
    let mut v = Vec::new();
    since(db, date(2023, 1, 1))
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&bu).and(&ub))))
        .drive(|_, y| v.push(y));
    rows(v.iter().map(|&(((p, c), x), ((u, b), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0)), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), nullable(a[1], a[0])]);
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
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount,
// AVG(p.CommentCount) AS AvgCommentCount
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
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.TotalScore,
// ups.AvgViewCount,
// ups.AvgCommentCount,
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
fn q13593(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, comment_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comment_count)).opt()).fold([0i64; 7], |a, p| match p {
        Some((((t, s), w), cc)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + cc],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[3], a[0]), avg(a[5], a[4]), avg(a[6], a[0])]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// AVG(vote_counts.VoteCount) AS AvgVotesPerPost,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount
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
// ) AS vote_counts ON p.Id = vote_counts.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// COALESCE(AvgVotesPerPost, 0) AS AvgVotesPerPost,
// QuestionCount,
// AnswerCount,
// WikiCount,
// (PostCount * 100.0 / NULLIF((SELECT COUNT(*) FROM Posts), 0)) AS PostPercentage
// FROM
// UserPostStats
// ORDER BY
// PostCount DESC;
fn q13594(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and((&vf).opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0), a[3] + (t == 1) as i64, a[4] + (t == 2) as i64, a[5] + (t == 3) as i64],
        None => a,
    });
    let tp = count(db.post.iq());
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), or0(a[2], a[1]), V::I(a[3]), V::I(a[4]), V::I(a[5]), ratio(a[0] * 100, tp)])
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.AnswerCount) AS TotalAnswers,
// SUM(p.FavoriteCount) AS TotalFavorites
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
// p.Id AS PostId,
// p.OwnerUserId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.OwnerUserId, p.PostTypeId
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.BadgeCount,
// us.TotalViews,
// ps.PostId,
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// us.TotalAnswers,
// us.TotalFavorites
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// ORDER BY
// us.Reputation DESC, ps.VoteCount DESC;
fn q13604(db: &'static So) -> String {
    let Post { view_count, answer_count, favorite_count, .. } = &db.post;
    let us = g(db).select(badges_of(db).opt().and(posts_of(db).select(view_count.opt().and(answer_count.opt()).and(favorite_count.opt())).opt())).fold([0i64; 7], |a, (b, p)| {
        let mut a = a;
        a[0] += b.is_some() as i64;
        if let Some(((w, an), fc)) = p {
            a[1] += w.is_some() as i64;
            a[2] += w.unwrap_or(0);
            a[3] += an.is_some() as i64;
            a[4] += an.unwrap_or(0);
            a[5] += fc.is_some() as i64;
            a[6] += fc.unwrap_or(0);
        }
        a
    });
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a[0]), nullable(a[2], a[1])];
        f.extend(post_fields(db, p, &["id", "type_id"]));
        f.extend([V::I(s.cx), V::I(s.vx), nullable(a[4], a[3]), nullable(a[6], a[5])]);
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// p.PostTypeId,
// COUNT(*) AS TotalPosts,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueAuthors,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers
// FROM
// Posts p
// GROUP BY
// p.PostTypeId
// ),
// UserMetrics AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(u.Reputation) AS AverageReputation,
// SUM(u.Views) AS TotalProfileViews
// FROM
// Users u
// ),
// VoteMetrics AS (
// SELECT
// COUNT(*) AS TotalVotes,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS TotalUpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS TotalDownVotes
// FROM
// Votes v
// )
// SELECT
// pm.PostTypeId,
// pm.TotalPosts,
// pm.TotalViews,
// pm.TotalScore,
// pm.AverageScore,
// pm.UniqueAuthors,
// pm.TotalQuestions,
// pm.TotalAnswers,
// um.TotalUsers,
// um.AverageReputation,
// um.TotalProfileViews,
// vm.TotalVotes,
// vm.TotalUpVotes,
// vm.TotalDownVotes
// FROM
// PostMetrics pm,
// UserMetrics um,
// VoteMetrics vm
// ORDER BY
// pm.PostTypeId;
fn q13608(db: &'static So) -> String {
    let f = by_key(db.post.iq(), &db.post.post_type_id, (&db.post.view_count).opt().and(&db.post.score).and(&db.post.post_type_id), [0i64; 6], |a, ((w, s), t)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + (t == 1) as i64, a[5] + (t == 2) as i64]
    });
    let owners = db.post.group_by(&db.post.post_type_id).select(&db.post.owner_user_id).count_distinct();
    let (un, rs, vs) = db.user.select((&db.user.reputation).and(&db.user.views)).fold_flat((0i64, 0i64, 0i64), |(n, s, v), (r, w)| (n + 1, s + r, v + w));
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    (&f).and((&owners).opt()).drive(|k, (a, o)| v.push((k, a, o.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, o)| {
        let mut f = vec![V::I(k), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), avg(a[3], a[0]), V::I(o), V::I(a[4]), V::I(a[5]), V::I(un), avg(rs, un), V::I(vs)];
        f.extend(ints(&x));
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
// UserBadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// UserStats AS (
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalUpVotes,
// ups.TotalDownVotes,
// COALESCE(ubs.TotalBadges, 0) AS TotalBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalUpVotes,
// TotalDownVotes,
// TotalBadges
// FROM
// UserStats
// ORDER BY
// TotalPosts DESC;
fn q13613(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64],
        None => a,
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH PostInteractionCounts AS (
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
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ),
// PostSummary AS (
// SELECT
// p.Id,
// p.Title,
// p.CreationDate,
// COALESCE(ic.CommentCount, 0) AS CommentCount,
// COALESCE(ic.VoteCount, 0) AS VoteCount,
// COALESCE(ic.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(ic.DownVoteCount, 0) AS DownVoteCount,
// p.Score,
// p.ViewCount
// FROM
// Posts p
// LEFT JOIN
// PostInteractionCounts ic ON p.Id = ic.PostId
// )
// SELECT
// ps.Title,
// ps.CreationDate,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.Score,
// ps.ViewCount
// FROM
// PostSummary ps
// ORDER BY
// ps.CreationDate DESC;
fn q13614(db: &'static So) -> String {
    let ic = stats_fold(db, since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&ic).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, s)| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(match s {
            Some(s) => [V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down)],
            None => [V::I(0), V::I(0), V::I(0), V::I(0)],
        });
        f.extend(post_fields(db, p, &["score", "views"]));
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// Id AS UserId,
// Reputation,
// CreationDate,
// LastAccessDate,
// Views,
// UpVotes,
// DownVotes
// FROM Users
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.OwnerUserId,
// u.DisplayName AS OwnerDisplayName,
// MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS CloseDate,
// MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenDate
// FROM Posts p
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount,
// p.AnswerCount, p.CommentCount, p.FavoriteCount, p.OwnerUserId, u.DisplayName
// )
// SELECT
// ur.UserId,
// ur.Reputation,
// ps.PostId,
// ps.PostTypeId,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.CloseDate,
// ps.ReopenDate
// FROM UserReputation ur
// JOIN PostStats ps ON ur.UserId = ps.OwnerUserId
// WHERE ur.Reputation > 1000
// ORDER BY ps.Score DESC, ps.CreationDate DESC;
fn q13625(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, owned(db).with((&db.post.owner_user).select(&db.user.reputation).gt(1000)), Ident::<Post>::new(), "h", &[]).drive(|p, s| v.push((p, s)));
    rows(v.iter().map(|&(p, s)| {
        let mut f = post_fields(db, p, &["uid", "rep", "id", "type_id", "created", "score", "views", "answers", "comments", "favorites"]);
        f.extend(["h10max", "h11max"].iter().map(|c| stat_field(&s, c).unwrap()));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore
// FROM
// Posts p
// JOIN PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserStats AS (
// SELECT
// COUNT(DISTINCT u.Id) AS UserCount,
// AVG(u.Reputation) AS AvgReputation,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
// FROM
// Users u
// )
// SELECT
// ps.PostTypeName,
// ps.PostCount,
// ps.AvgScore,
// us.UserCount,
// us.AvgReputation,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostStats ps,
// UserStats us
// ORDER BY
// ps.PostCount DESC;
fn q13641(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), &db.post.score, (0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let u = db.user.select((&db.user.reputation).and(&db.user.up_votes).and(&db.user.down_votes)).fold_flat([0i64; 4], |a, ((r, x), y)| [a[0] + 1, a[1] + r, a[2] + x, a[3] + y]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, (n, s))| row(vec![V::S(k), V::I(n), avg(s, n), V::I(u[0]), avg(u[1], u[0]), V::I(u[2]), V::I(u[3])])))
}

// WITH UserReputation AS (
// SELECT
// Id AS UserId,
// Reputation,
// (SELECT COUNT(*) FROM Posts WHERE OwnerUserId = Users.Id) AS PostCount,
// (SELECT COUNT(*) FROM Comments WHERE UserId = Users.Id) AS CommentCount
// FROM Users
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.CreationDate,
// P.OwnerUserId,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate, P.OwnerUserId
// ),
// VotesPerPost AS (
// SELECT
// PostId,
// COUNT(*) FILTER (WHERE VoteTypeId = 2) AS UpVotes,
// COUNT(*) FILTER (WHERE VoteTypeId = 3) AS DownVotes
// FROM Votes
// GROUP BY PostId
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.CommentCount,
// PS.PostId,
// PS.Title,
// PS.Score,
// PS.ViewCount,
// PS.CreationDate,
// V.UpVotes,
// V.DownVotes
// FROM UserReputation U
// JOIN PostStats PS ON U.UserId = PS.OwnerUserId
// JOIN VotesPerPost V ON PS.PostId = V.PostId
// ORDER BY U.Reputation DESC, PS.Score DESC;
fn q13643(db: &'static So) -> String {
    let pc = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let cu = comments_per_user(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).with(votes_of(db)).select(Ident::<Post>::new().and(&pv).and((&db.post.owner_user).select((&pc).and(&cu)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), (n, c))| {
        let mut f = post_fields(db, p, &["uid", "rep"]);
        f.extend([V::I(n), V::I(c)]);
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "created"]));
        f.extend([V::I(x[1]), V::I(x[2])]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.CreationDate,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.CreationDate, p.Score
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.ViewCount) AS TotalPostViews,
// SUM(ps.ViewCount) AS TotalPostScore
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostStats ps ON p.Id = ps.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.BadgeCount,
// us.TotalPostViews,
// us.TotalPostScore,
// ps.Title AS PostTitle,
// ps.ViewCount AS PostViews,
// ps.Score AS PostScore,
// ps.CommentCount
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.TotalPostViews DESC,
// ps.Score DESC;
fn q13646(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a.bx), ustat_field(&a, "views_sum"), ustat_field(&a, "views_sum")];
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.push(V::I(s.cx));
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
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
// u.Id, u.Reputation
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// us.PostCount,
// us.CommentCount,
// us.UpVotes,
// us.DownVotes,
// us.Reputation
// FROM
// Users u
// JOIN
// UserStatistics us ON u.Id = us.UserId
// ORDER BY
// us.Reputation DESC,
// us.PostCount DESC
// LIMIT 100;
fn q13653(db: &'static So) -> String {
    out(users_with_counts(db, "cv", false), |r| (Reverse(r.rep), Reverse(r.agg.n)), 100, |r| user_fields(r, "cv", &["uid", "name", "#n", "#c", "#up", "#down", "rep"]))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// u.Id AS OwnerUserId,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.VoteCount,
// us.UpVotes,
// us.DownVotes,
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.Score,
// pd.ViewCount,
// pd.AnswerCount,
// pd.CommentCount,
// pd.FavoriteCount,
// pd.OwnerUserId,
// pd.OwnerDisplayName
// FROM
// UserStats us
// LEFT JOIN
// PostDetails pd ON us.UserId = pd.OwnerUserId
// ORDER BY
// us.Reputation DESC, us.PostCount DESC;
fn q13661(db: &'static So) -> String {
    let us = g(db).select(posts_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let vu = votes_per_user(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&vu).and(posts_of(db).opt()).drive(|u, (((a, d), x), p)| v.push((u, a, d.unwrap_or(0), x, p)));
    rows(v.iter().map(|&(u, a, d, x, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d), V::I(x), V::I(a[0]), V::I(a[1])];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "uid", "owner"])),
            None => f.extend(nulls(10)),
        }
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(b.Date) AS LastBadgeDate,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId
// ),
// UserMetrics AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS QuestionsCount,
// COUNT(DISTINCT b.Id) AS BadgesCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.Reputation
// )
// SELECT
// pm.PostId,
// pm.Title,
// pm.CreationDate,
// pm.ViewCount,
// pm.Score,
// pm.CommentCount,
// pm.VoteCount,
// um.UserId,
// um.Reputation,
// um.QuestionsCount,
// um.BadgesCount,
// pm.LastBadgeDate
// FROM
// PostMetrics pm
// JOIN
// Users u ON pm.OwnerUserId = u.Id
// JOIN
// UserMetrics um ON u.Id = um.UserId
// ORDER BY
// pm.Score DESC, pm.ViewCount DESC
// LIMIT 100;
fn q13662(db: &'static So) -> String {
    let qs = posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(1)));
    let dq = ud(db, UserWhere::All, &qs);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, owned(db).with((&db.post.post_type_id).eq(1)), Ident::<Post>::new(), "cvb", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and((&dq).opt()).and(&bu)))
        .drive(|p, (s, ((u, q), b))| v.push((p, s, u, q.unwrap_or(0), b)));
    out(v, |&(p, ..)| score_views(db, p), 100, |&(p, s, u, q, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(s.cx), V::I(s.vx), user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(q), V::I(b), stat_field(&s, "bmax").unwrap()]);
        f
    })
}

// WITH UsersStats AS (
// SELECT
// Id,
// Reputation,
// CreationDate,
// Views,
// UpVotes,
// DownVotes,
// (UpVotes - DownVotes) AS NetVotes
// FROM
// Users
// ),
// PostsStats AS (
// SELECT
// p.Id,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.OwnerUserId,
// u.Reputation AS OwnerReputation,
// u.Views AS OwnerViews,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// )
// SELECT
// us.Id AS UserId,
// us.Reputation,
// ps.Id AS PostId,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.OwnerReputation,
// ps.OwnerViews,
// ps.TotalComments,
// ps.CreationDate AS PostCreationDate
// FROM
// UsersStats us
// JOIN
// PostsStats ps ON us.Id = ps.OwnerUserId
// ORDER BY
// us.Reputation DESC, ps.Score DESC;
fn q13667(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(comments_per_post(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, c)| {
        let u = db.post.owner_user.get(p).unwrap();
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(post_fields(db, p, &["id", "score", "views", "answers", "comments", "favorites", "rep"]));
        f.extend([user_col(db, u, "uviews"), V::I(c)]);
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 4 THEN 1 ELSE 0 END) AS OffensiveVoteCount,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT p.Tags) AS TagCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.OwnerUserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(ps.PostId) AS TotalPosts,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.UpVoteCount) AS TotalUpVotes,
// SUM(ps.DownVoteCount) AS TotalDownVotes,
// SUM(ps.OffensiveVoteCount) AS TotalOffensiveVotes,
// AVG(ps.AverageScore) AS AveragePostScore,
// SUM(ps.TagCount) AS TotalTagsUsed
// FROM
// Users u
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalPosts DESC;
fn q13683(db: &'static So) -> String {
    let pf = stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]);
    let uf = g(db).select(posts_of(db).select((&pf).and(&db.post.score).and((&db.post.tags_str).opt())).opt()).fold([0i64; 8], |a, p| match p {
        Some(((s, sc), t)) => [a[0] + 1, a[1] + s.cx, a[2] + s.up, a[3] + s.down, a[4] + s.by_vt[4], a[5] + sc, a[6] + t.is_some() as i64, 0],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0])];
        f.extend((1..5).map(|i| nullable(a[i], a[0])));
        f.extend([avg(a[5], a[0]), nullable(a[6], a[0])]);
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
// SUM(COALESCE(c.Score, 0)) AS TotalCommentScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(COALESCE(b.Class, 0)) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ua.DisplayName,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.TotalCommentScore,
// ua.TotalUpVotes,
// ua.TotalDownVotes,
// ua.TotalBadges,
// r.Reputation
// FROM
// UserActivity ua
// JOIN
// Users r ON ua.UserId = r.Id
// ORDER BY
// ua.TotalUpVotes DESC,
// ua.PostCount DESC
// LIMIT 100;
fn q13689(db: &'static So) -> String {
    let uf = g(db)
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).select(&db.comment.score).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let mut a = a;
            if let Some(((t, c), x)) = p {
                a[0] += 1;
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += c.unwrap_or(0);
                a[4] += (x == Some(2)) as i64;
                a[5] += (x == Some(3)) as i64;
            }
            a[6] += b.unwrap_or(0);
            a
        });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| (Reverse(a[4]), Reverse(a[0])), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(user_col(db, u, "rep"));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
// COALESCE(SUM(p.CommentCount), 0) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// TotalScore,
// TotalViews,
// QuestionCount,
// AnswerCount,
// TotalComments
// FROM
// UserPostStats
// ORDER BY
// TotalScore DESC, PostCount DESC;
fn q13694(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, comment_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comment_count)).opt()).fold([0i64; 6], |a, p| match p {
        Some((((t, s), w), cc)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + (t == 1) as i64, a[4] + (t == 2) as i64, a[5] + cc],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
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
// UserCommentCounts AS (
// SELECT
// c.UserId,
// COUNT(c.Id) AS CommentCount
// FROM
// Comments c
// GROUP BY
// c.UserId
// ),
// UserVoteCounts AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS VoteCount
// FROM
// Votes v
// GROUP BY
// v.UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(p.PostCount, 0) AS TotalPosts,
// COALESCE(p.QuestionCount, 0) AS TotalQuestions,
// COALESCE(p.AnswerCount, 0) AS TotalAnswers,
// COALESCE(c.CommentCount, 0) AS TotalComments,
// COALESCE(v.VoteCount, 0) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// UserPostCounts p ON u.Id = p.UserId
// LEFT JOIN
// UserCommentCounts c ON u.Id = c.UserId
// LEFT JOIN
// UserVoteCounts v ON u.Id = v.UserId
// ORDER BY
// UserId;
fn q13702(db: &'static So) -> String {
    let cu = comments_per_user(db);
    let vu = votes_per_user(db);
    let mut v = Vec::new();
    upqa(db).and(&cu).and(&vu).drive(|u, ((a, c), x)| v.push((u, a, c, x)));
    rows(v.iter().map(|&(u, a, c, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([V::I(c), V::I(x)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// COALESCE(vote_counts.UpVotes, 0) AS UpVotes,
// COALESCE(vote_counts.DownVotes, 0) AS DownVotes,
// COALESCE(comment_counts.CommentCount, 0) AS CommentCount,
// p.CreationDate,
// p.LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId
// ) vote_counts ON p.Id = vote_counts.PostId
// LEFT JOIN
// (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) comment_counts ON p.Id = comment_counts.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.Score DESC, p.ViewCount DESC
// LIMIT 100;
fn q13714(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and((&pv).opt()).and(comments_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| score_views(db, p), 100, |&((p, x), c)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(c)]);
        f.extend(post_fields(db, p, &["created", "activity"]));
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(v.VoteCount, 0) AS TotalVotes,
// COALESCE(c.CommentCount, 0) AS TotalComments,
// p.AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId) v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY
// p.CreationDate DESC;
fn q13722(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, year_ago()).select(Ident::<Post>::new().and(votes_per_post(db)).and(comments_per_post(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(x), V::I(c)]);
        f.extend(post_fields(db, p, &["answers", "owner", "rep"]));
        row(f)
    }))
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// LEFT JOIN
// Posts P ON V.PostId = P.Id
// WHERE
// U.Reputation > 100
// GROUP BY
// U.Id
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate
// )
// SELECT
// U.DisplayName AS UserDisplayName,
// U.Reputation,
// U.CreationDate AS UserCreationDate,
// UVote.UpVotes,
// UVote.DownVotes,
// PStats.PostId,
// PStats.Title,
// PStats.CreationDate AS PostCreationDate,
// PStats.CommentCount,
// PStats.VoteCount,
// PStats.UpVoteCount,
// PStats.DownVoteCount
// FROM
// UserVoteStats UVote
// JOIN
// Users U ON UVote.UserId = U.Id
// JOIN
// PostStats PStats ON U.Id = PStats.PostId
// ORDER BY
// U.Reputation DESC,
// PStats.VoteCount DESC;
fn q13731(db: &'static So) -> String {
    let uid = uids(db);
    let w = UserWhere::RepGt(100);
    let uv = user_base(db, w).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let dp = ud(db, w, votes_by(db).select(&db.vote.post));
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&c).opt())
        .and((&x).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uv).and((&dp).opt())))
        .drive(|p, (((s, c), x), ((u, a), _))| v.push((p, s, c.unwrap_or(0), x.unwrap_or(0), u, a)));
    rows(v.iter().map(|&(p, s, c, x, u, a)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated"), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(c), V::I(x), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// WITH PostVoteStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.Score,
// P.ViewCount,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.Score, P.ViewCount
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount,
// SUM(P.ViewCount) AS TotalPostViews,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
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
// PVS.PostId,
// PVS.PostTypeId,
// PVS.Score,
// PVS.ViewCount,
// PVS.VoteCount,
// PVS.UpVotes,
// PVS.DownVotes,
// US.UserId,
// US.Reputation,
// US.BadgeCount,
// US.TotalPostViews,
// US.QuestionCount,
// US.AnswerCount
// FROM
// PostVoteStats PVS
// JOIN
// UserStats US ON PVS.PostId = US.UserId
// ORDER BY
// PVS.Score DESC, PVS.ViewCount DESC
// LIMIT 100;
fn q13732(db: &'static So) -> String {
    let uid = uids(db);
    let us = g(db).select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt().and(&db.post.post_type_id)).opt())).fold([0i64; 5], |a, (b, p)| {
        let mut a = a;
        a[0] += b.is_some() as i64;
        if let Some((w, t)) = p {
            a[1] += w.is_some() as i64;
            a[2] += w.unwrap_or(0);
            a[3] += (t == 1) as i64;
            a[4] += (t == 2) as i64;
        }
        a
    });
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "v", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, ..)| score_views(db, p), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "type_id", "score", "views"]);
        f.extend([V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4])]);
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(v.UpVotes, 0) AS UpVotes,
// COALESCE(v.DownVotes, 0) AS DownVotes,
// u.Reputation AS UserReputation,
// u.DisplayName AS UserDisplayName
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
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13733(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(typed_answers_per_post(db)).and((&pv).opt())).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, a), x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a), V::I(x[1]), V::I(x[2])]);
        f.extend(post_fields(db, p, &["rep", "owner"]));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// AVG(P.AnswerCount) AS AvgAnswersPerQuestion
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName
// ),
// UserBadgeStats AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS TotalBadges,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalQuestions,
// UPS.TotalAnswers,
// UPS.TotalViews,
// UPS.TotalScore,
// UPS.AvgAnswersPerQuestion,
// UBS.TotalBadges,
// UBS.GoldBadges,
// UBS.SilverBadges,
// UBS.BronzeBadges
// FROM UserPostStats UPS
// LEFT JOIN UserBadgeStats UBS ON UPS.UserId = UBS.UserId
// ORDER BY UPS.TotalPosts DESC;
fn q13738(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, answer_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(answer_count.opt())).opt()).fold([0i64; 8], |a, p| match p {
        Some((((t, s), w), an)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s, a[6] + an.is_some() as i64, a[7] + an.unwrap_or(0)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), nullable(a[5], a[0]), avg(a[7], a[6])]);
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
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
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
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
// UserId,
// COUNT(Id) AS BadgeCount,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ups.PostCount, 0) AS TotalPosts,
// COALESCE(ups.QuestionCount, 0) AS TotalQuestions,
// COALESCE(ups.AnswerCount, 0) AS TotalAnswers,
// COALESCE(ups.UpVoteCount, 0) AS TotalUpVotes,
// COALESCE(ups.DownVoteCount, 0) AS TotalDownVotes,
// COALESCE(ubs.BadgeCount, 0) AS TotalBadges,
// COALESCE(ubs.GoldBadgeCount, 0) AS TotalGoldBadges,
// COALESCE(ubs.SilverBadgeCount, 0) AS TotalSilverBadges,
// COALESCE(ubs.BronzeBadgeCount, 0) AS TotalBronzeBadges
// FROM
// Users u
// LEFT JOIN
// UserPostStats ups ON u.Id = ups.UserId
// LEFT JOIN
// UserBadgeStats ubs ON u.Id = ubs.UserId
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q13743(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// MAX(p.CreationDate) AS LastPostDate,
// AVG(COALESCE(c.Score, 0)) AS AverageCommentScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// pt.Name AS PostTypeName,
// p.OwnerUserId
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.LastPostDate,
// us.AverageCommentScore,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// ps.CommentCount,
// ps.PostTypeName
// FROM
// UserStatistics us
// LEFT JOIN
// PostStatistics ps ON us.UserId = ps.OwnerUserId
// ORDER BY
// us.Reputation DESC,
// ps.ViewCount DESC;
fn q13763(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(creation_date).and(comments_of(db).select(&db.comment.score).opt())).opt()).fold([0, 0, 0, i64::MIN, 0, 0], |a: [i64; 6], p| match p {
        Some(((t, cd), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(cd), a[4] + c.unwrap_or(0), a[5] + 1],
        None => [a[0], a[1], a[2], a[3], a[4], a[5] + 1],
    });
    let mut v = Vec::new();
    (&uf).and(posts_of(db).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[0] == 0 { V::Null } else { V::T(a[3]) }, avg(a[4], a[5])];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "type"])),
            None => f.extend(nulls(8)),
        }
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
// COALESCE(UPT.Upvotes, 0) AS Upvotes,
// COALESCE(DWT.Downvotes, 0) AS Downvotes,
// COALESCE(CT.ClosedCount, 0) AS ClosedCount,
// COALESCE(ED.EditCount, 0) AS EditCount
// FROM
// Posts p
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS Upvotes
// FROM
// Votes
// WHERE
// VoteTypeId = 2
// GROUP BY
// PostId
// ) UPT ON p.Id = UPT.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS Downvotes
// FROM
// Votes
// WHERE
// VoteTypeId = 3
// GROUP BY
// PostId
// ) DWT ON p.Id = DWT.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS ClosedCount
// FROM
// PostHistory
// WHERE
// PostHistoryTypeId = 10
// GROUP BY
// PostId
// ) CT ON p.Id = CT.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS EditCount
// FROM
// PostHistory
// WHERE
// PostHistoryTypeId IN (4, 5, 6, 24)
// GROUP BY
// PostId
// ) ED ON p.Id = ED.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.Upvotes,
// ps.Downvotes,
// ps.ClosedCount,
// ps.EditCount,
// (ps.Score + ps.Upvotes - ps.Downvotes) AS NetScore
// FROM
// PostStats ps
// ORDER BY
// NetScore DESC
// LIMIT 100;
fn q13764(db: &'static So) -> String {
    let cl = history_of_types(db, &[10]);
    let ed = history_of_types(db, &[4, 5, 6, 24]);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and(&cl).and(&ed)).drive(|_, x| v.push(x));
    let net = |p: Id<Post>, u: i64, d: i64| db.post.score.get(p).unwrap() + u - d;
    out(v, |&((((p, u), d), _), _)| Reverse(net(p, u, d)), 100, |&((((p, u), d), c), e)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([V::I(u), V::I(d), V::I(c), V::I(e), V::I(net(p, u, d))]);
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(V.BountyAmount) AS TotalBountyAmount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.Reputation
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(P.Score) AS TotalPostScore,
// AVG(P.ViewCount) AS AverageViewCount
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// US.BadgeCount,
// US.TotalBountyAmount,
// PS.PostCount,
// PS.TotalPostScore,
// PS.AverageViewCount
// FROM
// Users U
// LEFT JOIN
// UserStatistics US ON U.Id = US.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// ORDER BY
// U.Reputation DESC;
fn q13772(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ub = g(db).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 2], |a, (_, b)| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]
    });
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    (&ub).and(&bu).and((&pf).opt()).drive(|u, ((a, b), p)| v.push((u, a, b, p)));
    rows(v.iter().map(|&(u, a, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), nullable(a[1], a[0])];
        match p {
            Some(p) => f.extend([V::I(p[0]), V::I(p[1]), avg(p[3], p[2])]),
            None => f.extend(nulls(3)),
        }
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners,
// COUNT(*) FILTER (WHERE PostTypeId = 1) AS TotalQuestions,
// COUNT(*) FILTER (WHERE PostTypeId = 2) AS TotalAnswers,
// SUM(ViewCount) AS TotalViews,
// SUM(Score) AS TotalScore
// FROM
// Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// SUM(Reputation) AS TotalReputation,
// COUNT(*) FILTER (WHERE Reputation > 1000) AS UsersWithHighReputation
// FROM
// Users
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN VoteTypeId = 6 THEN 1 ELSE 0 END) AS TotalCloseVotes
// FROM
// Votes
// )
// SELECT
// ps.TotalPosts,
// ps.UniquePostOwners,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.TotalViews,
// ps.TotalScore,
// us.TotalUsers,
// us.TotalReputation,
// us.UsersWithHighReputation,
// vs.TotalVotes,
// vs.TotalUpVotes,
// vs.TotalDownVotes,
// vs.TotalCloseVotes
// FROM
// PostStats ps,
// UserStats us,
// VoteStats vs;
fn q13773(db: &'static So) -> String {
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let p = db.post.select((&db.post.post_type_id).and((&db.post.view_count).opt()).and(&db.post.score)).fold_flat([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]
    });
    let u = db.user.select(&db.user.reputation).fold_flat([0i64; 3], |a, r| [a[0] + 1, a[1] + r, a[2] + (r > 1000) as i64]);
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (t == 6) as i64]);
    let mut f = vec![V::I(p[0]), V::I(owners), V::I(p[1]), V::I(p[2]), nullable(p[4], p[3]), nullable(p[5], p[0])];
    f.extend([V::I(u[0]), nullable(u[1], u[0]), V::I(u[2])]);
    f.extend([V::I(x[0]), nullable(x[1], x[0]), nullable(x[2], x[0]), nullable(x[3], x[0])]);
    row(f)
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// COUNT(b.Id) AS BadgeCount,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalPostViews
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
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.TotalPostViews
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.CreationDate DESC;
fn q13776(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a.bx), ustat_field(&a, "views_sum0")]);
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id
// ),
// UserVoteStats AS (
// SELECT
// V.UserId,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes V
// GROUP BY
// V.UserId
// ),
// UserBadgeCounts AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UPC.PostCount, 0) AS TotalPosts,
// COALESCE(UPC.Questions, 0) AS TotalQuestions,
// COALESCE(UPC.Answers, 0) AS TotalAnswers,
// COALESCE(UPC.Wikis, 0) AS TotalWikis,
// COALESCE(UVC.VoteCount, 0) AS TotalVotes,
// COALESCE(UVC.UpVotes, 0) AS TotalUpVotes,
// COALESCE(UVC.DownVotes, 0) AS TotalDownVotes,
// COALESCE(UBC.BadgeCount, 0) AS TotalBadges,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate
// FROM
// Users U
// LEFT JOIN
// UserPostCounts UPC ON U.Id = UPC.UserId
// LEFT JOIN
// UserVoteStats UVC ON U.Id = UVC.UserId
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// ORDER BY
// TotalPosts DESC, Reputation DESC
// LIMIT 100;
fn q13778(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 4], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
        None => a,
    });
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and((&uv).opt()).and(&bu).drive(|u, ((a, x), b)| v.push((u, a, x.unwrap_or([0; 3]), b)));
    out(v, |&(u, a, _, _)| (Reverse(a[0]), rep_desc(db, u)), 100, |&(u, a, x, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&x));
        f.extend([V::I(b), user_col(db, u, "rep"), user_col(db, u, "ucreated"), user_col(db, u, "last_access")]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// COALESCE(SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// p.Id, p.Title, p.CreationDate
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.BadgeCount
// FROM
// PostStats ps
// ORDER BY
// ps.CreationDate DESC
// FETCH FIRST 100 ROWS ONLY;
fn q13780(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let by_uid = badges_by_uid(db);
    let pf = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user_id).select(&by_uid).opt()))
        .fold([0i64; 3], |a, ((_, t), b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]);
    let mut v = Vec::new();
    (&pf).and((&c).opt()).and((&x).opt()).drive(|p, ((s, c), x)| v.push((p, s, c.unwrap_or(0), x.unwrap_or(0))));
    out(v, |&(p, ..)| newest(db, p), 100, |&(p, s, c, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(x), V::I(s[0]), V::I(s[1]), V::I(s[2])]);
        f
    })
}

// WITH PostEngagement AS (
// SELECT
// p.Id AS PostId,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// u.Reputation AS UserReputation,
// u.DisplayName AS UserDisplayName
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.Id, u.Reputation, u.DisplayName
// )
// SELECT
// PostId,
// PostTitle,
// PostCreationDate,
// CommentCount,
// VoteCount,
// UpVoteCount,
// DownVoteCount,
// BadgeCount,
// UserReputation,
// UserDisplayName
// FROM
// PostEngagement
// ORDER BY
// VoteCount DESC, CommentCount DESC;
fn q13784(db: &'static So) -> String {
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    rows(stats_with(db, since(db, month_ago()), "cvb", &[], &[&b]).iter().map(|&(p, s, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), V::I(d[0])]);
        f.extend(post_fields(db, p, &["rep", "owner"]));
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViewCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id
// ),
// BadgeCounts AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// UserMetrics AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COALESCE(PC.PostCount, 0) AS PostCount,
// COALESCE(BC.BadgeCount, 0) AS BadgeCount,
// PC.TotalScore,
// PC.TotalViewCount
// FROM
// Users U
// LEFT JOIN
// UserPostCounts PC ON U.Id = PC.UserId
// LEFT JOIN
// BadgeCounts BC ON U.Id = BC.UserId
// )
// SELECT
// UserId,
// Reputation,
// PostCount,
// BadgeCount,
// TotalScore,
// TotalViewCount
// FROM
// UserMetrics
// ORDER BY
// Reputation DESC, PostCount DESC;
fn q13789(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let uf = g(db).select(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a[0]), V::I(b), V::I(a[1]), V::I(a[2])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(Id) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13794(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and((&db.post.view_count).opt()).and(comments_per_post(db)), [0i64; 5], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4])])))
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Users U ON p.OwnerUserId = U.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, U.DisplayName
// ),
// PostHistorySummary AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS EditCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.OwnerDisplayName,
// ps.CommentCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// COALESCE(phs.EditCount, 0) AS EditCount,
// phs.LastEditDate
// FROM
// PostSummary ps
// LEFT JOIN
// PostHistorySummary phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.CreationDate DESC;
fn q13799(db: &'static So) -> String {
    let es = db
        .post_history
        .with((&db.post_history.post_history_type_id).in_v(vec![4, 5, 6]))
        .group_by(&db.post_history.post)
        .select(&db.post_history.creation_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cv", &[]).and((&es).opt()).drive(|p, (s, e)| v.push((p, s, e)));
    rows(v.iter().map(|&(p, s, e)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(e.map_or(0, |e| e.0)), ots(e.map(|e| e.1))]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(c.CommentCount), 0) AS TotalComments,
// COALESCE(SUM(v.VoteCount), 0) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN (
// SELECT PostId, COUNT(Id) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT PostId, COUNT(Id) AS VoteCount
// FROM Votes
// GROUP BY PostId
// ) v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostTypeName,
// PostCount,
// TotalComments,
// TotalVotes
// FROM
// PostStats
// ORDER BY
// PostCount DESC;
fn q13801(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), comments_per_post(db).and(votes_per_post(db)), [0i64; 3], |a, (c, x)| [a[0] + 1, a[1] + c, a[2] + x]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2])])))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COALESCE(vote_counts.UpVotes, 0) AS UpVoteCount,
// COALESCE(vote_counts.DownVotes, 0) AS DownVoteCount,
// COALESCE(c_counts.CommentCount, 0) AS CommentCount,
// COALESCE(badge_counts.BadgeCount, 0) AS UserBadgeCount
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
// PostId) vote_counts ON p.Id = vote_counts.PostId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(Id) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c_counts ON p.Id = c_counts.PostId
// LEFT JOIN
// (SELECT
// UserId,
// COUNT(Id) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId) badge_counts ON u.Id = badge_counts.UserId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13811(db: &'static So) -> String {
    let pv = post_votes(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and((&pv).opt()).and(comments_per_post(db)).and((&db.post.owner_user).select(&bu))).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, x), c), b)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "rep"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(c), V::I(b)]);
        f
    })
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.CommentCount,
// COALESCE(VoteCounts.UpVotes, 0) AS UpVotes,
// COALESCE(VoteCounts.DownVotes, 0) AS DownVotes,
// COALESCE(CommentCounts.TotalComments, 0) AS TotalComments,
// U.Reputation AS OwnerReputation,
// U.DisplayName AS OwnerDisplayName,
// P.PostTypeId,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseVotes,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenVotes
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId
// ) VoteCounts ON P.Id = VoteCounts.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS TotalComments
// FROM
// Comments
// GROUP BY
// PostId
// ) CommentCounts ON P.Id = CommentCounts.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.CommentCount,
// U.Reputation, U.DisplayName, P.PostTypeId,
// VoteCounts.UpVotes, VoteCounts.DownVotes, CommentCounts.TotalComments
// ORDER BY
// P.CreationDate DESC;
fn q13832(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "h", &[]).and((&pv).opt()).and(comments_per_post(db)).drive(|p, ((s, x), c)| v.push((p, s, x, c)));
    rows(v.iter().map(|&(p, s, x, c)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "comments"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(c)]);
        f.extend(post_fields(db, p, &["rep", "owner", "type_id"]));
        f.extend([V::I(s.h10), V::I(s.h11)]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(DISTINCT p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ),
// TopPosts AS (
// SELECT
// p.Id,
// p.Title,
// p.ViewCount,
// pt.Name AS PostType
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// ORDER BY
// p.ViewCount DESC
// LIMIT 5
// )
// SELECT
// ps.PostType,
// ps.PostCount,
// ps.AvgScore,
// ps.CommentCount,
// tp.Title AS TopPostTitle,
// tp.ViewCount AS TopPostViewCount
// FROM
// PostStats ps
// LEFT JOIN
// TopPosts tp ON ps.PostType = tp.PostType;
fn q13835(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_of(db).opt()), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c.is_some() as i64]);
    let np = db.post.group_by(name(db)).fold(0i64, |a, _| a + 1);
    let top: MatSet<Id<Post>> = whole(db.post.iq()).select(Ident::<Post>::new().and((&db.post.view_count).opt())).window(row_number, |(_, w)| w, desc).filt(|(_, n)| n <= 5).map(|((p, _), _)| p).collect();
    let tp: HashIdx<Str, Id<Post>> = (&top).select(name(db)).inv().collect();
    let mut v = Vec::new();
    (&f).and(&np).and((&tp).opt()).drive(|k, ((a, n), p)| v.push((k, a, n, p)));
    rows(v.iter().map(|&(k, a, n, p)| {
        let mut f = vec![V::S(k), V::I(n), avg(a[1], a[0]), V::I(a[2])];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "views"])),
            None => f.extend(nulls(2)),
        }
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("13411", q13411),
    ("13496", q13496),
    ("13504", q13504),
    ("13508", q13508),
    ("13513", q13513),
    ("13517", q13517),
    ("13527", q13527),
    ("13536", q13536),
    ("13541", q13541),
    ("13542", q13542),
    ("13543", q13543),
    ("13561", q13561),
    ("13575", q13575),
    ("13579", q13579),
    ("13585", q13585),
    ("13593", q13593),
    ("13594", q13594),
    ("13604", q13604),
    ("13608", q13608),
    ("13613", q13613),
    ("13614", q13614),
    ("13625", q13625),
    ("13641", q13641),
    ("13643", q13643),
    ("13646", q13646),
    ("13653", q13653),
    ("13661", q13661),
    ("13662", q13662),
    ("13667", q13667),
    ("13683", q13683),
    ("13689", q13689),
    ("13694", q13694),
    ("13702", q13702),
    ("13714", q13714),
    ("13722", q13722),
    ("13731", q13731),
    ("13732", q13732),
    ("13733", q13733),
    ("13738", q13738),
    ("13743", q13743),
    ("13763", q13763),
    ("13764", q13764),
    ("13772", q13772),
    ("13773", q13773),
    ("13776", q13776),
    ("13778", q13778),
    ("13780", q13780),
    ("13784", q13784),
    ("13789", q13789),
    ("13794", q13794),
    ("13799", q13799),
    ("13801", q13801),
    ("13811", q13811),
    ("13832", q13832),
    ("13835", q13835),
];
