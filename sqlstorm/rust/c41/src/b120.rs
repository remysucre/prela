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

// --- batch 120 --------------------------------------------------------------

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(v.BountyAmount) AS TotalBounties
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
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.AnswerCount,
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.TotalBounties
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC
// LIMIT 100;
fn q12509(db: &'static So) -> String {
    let uid = uids(db);
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, answers_of(db));
    let bu = badges_per_user(db);
    let ub = g(db).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 2], |a, (_, x)| {
        let x = x.flatten();
        [a[0] + x.is_some() as i64, a[1] + x.unwrap_or(0)]
    });
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and((&c).opt()).and((&a).opt()).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&bu).and(&ub))))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| score_views(db, p), 100, |&(((p, c), a), ((u, b), x))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0)), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), nullable(x[1], x[0])]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(u.Reputation) AS AvgUserReputation,
// MAX(p.CreationDate) AS MaxCreationDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.Id, p.PostTypeId
// ),
// TypeStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(ps.PostId) AS TotalPosts,
// AVG(ps.CommentCount) AS AvgComments,
// AVG(ps.VoteCount) AS AvgVotes,
// AVG(ps.AvgUserReputation) AS AvgUserReputation,
// MIN(ps.MaxCreationDate) AS EarliestPostDate,
// MAX(ps.MaxCreationDate) AS LatestPostDate
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// AvgComments,
// AvgVotes,
// AvgUserReputation,
// EarliestPostDate,
// LatestPostDate
// FROM
// TypeStats
// ORDER BY
// TotalPosts DESC;
fn q12523(db: &'static So) -> String {
    let pf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let f = by_key(db.post.iq(), name(db), (&pf).and((&db.post.owner_user).select(&db.user.reputation).opt()).and(&db.post.creation_date), [0, 0, 0, 0, 0, i64::MAX, i64::MIN], |a: [i64; 7], ((s, r), cd)| {
        [a[0] + 1, a[1] + s.cx, a[2] + s.vx, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0), a[5].min(cd), a[6].max(cd)]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[2], a[0]), avg(a[4], a[3]), V::T(a[5]), V::T(a[6])])))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore,
// MAX(p.CreationDate) AS LastPostDate
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
// ups.LastPostDate,
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
// LIMIT 100;
fn q12525(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(creation_date)).opt()).fold([0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 7], p| match p {
        Some((((t, w), s), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s, a[6].max(c)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), avg(a[5], a[0]), if a[0] == 0 { V::Null } else { V::T(a[6]) }]);
        f.extend(ints(&b));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// MAX(ph.CreationDate) AS LastEditDate
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
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.CommentCount,
// PS.UpVoteCount,
// PS.DownVoteCount,
// PS.BadgeCount,
// PS.LastEditDate
// FROM
// PostStats PS
// ORDER BY
// PS.CreationDate DESC;
fn q12529(db: &'static So) -> String {
    let by_uid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let base = || since(db, date(2023, 1, 1));
    let f = base()
        .group_by(Ident::<Post>::new())
        .select(
            comments_of(db)
                .opt()
                .and(votes_of(db).select(&db.vote.vote_type_id).opt())
                .and((&db.post.owner_user_id).select(&by_uid).opt())
                .and(history_of(db).select(&db.post_history.creation_date).opt()),
        )
        .fold([0, 0, 0, 0, i64::MIN], |a: [i64; 5], (((c, t), _), h)| {
            [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + h.is_some() as i64, h.map_or(a[4], |d| a[4].max(d))]
        });
    let b = base().group_by(Ident::<Post>::new()).select((&db.post.owner_user_id).select(&by_uid)).count_distinct();
    let mut v = Vec::new();
    (&f).and((&b).opt()).drive(|p, (a, b)| v.push((p, a, b.unwrap_or(0))));
    rows(v.iter().map(|&(p, a, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b), if a[3] == 0 { V::Null } else { V::T(a[4]) }]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// PT.Name AS PostType,
// COUNT(P.Id) AS PostCount,
// AVG(P.Score) AS AverageScore,
// AVG(P.ViewCount) AS AverageViewCount,
// SUM(CASE WHEN V.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// AVG(U.Reputation) AS AverageUserReputation
// FROM
// Posts P
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// GROUP BY
// PT.Name
// )
// SELECT
// PostType,
// PostCount,
// AverageScore,
// AverageViewCount,
// TotalVotes,
// AverageUserReputation
// FROM
// PostStatistics
// ORDER BY
// PostCount DESC;
fn q12533(db: &'static So) -> String {
    let f = by_key(
        db.post.iq(),
        name(db),
        (&db.post.score).and((&db.post.view_count).opt()).and((&db.post.owner_user).select(&db.user.reputation).opt()).and(votes_of(db).opt()),
        [0i64; 8],
        |a, (((s, w), r), x)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x.is_some() as i64, a[5] + r.is_some() as i64, a[6] + r.unwrap_or(0), 0],
    );
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), avg(a[6], a[5])])))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS TotalDownvotedPosts
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ), UserBadgeStats AS (
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
// ups.TotalDownvotedPosts,
// COALESCE(ubs.TotalBadges, 0) AS TotalBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY
// ups.TotalPosts DESC
// FETCH FIRST 100 ROWS ONLY;
fn q12546(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64],
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
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.Views,
// PC.TotalPosts,
// PC.TotalQuestions,
// PC.TotalAnswers
// FROM
// Users U
// LEFT JOIN
// PostCounts PC ON U.Id = PC.OwnerUserId
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
// )
// SELECT
// US.UserId,
// US.Reputation,
// US.Views,
// COALESCE(BC.TotalBadges, 0) AS TotalBadges,
// COALESCE(BC.GoldBadges, 0) AS GoldBadges,
// COALESCE(BC.SilverBadges, 0) AS SilverBadges,
// COALESCE(BC.BronzeBadges, 0) AS BronzeBadges,
// US.TotalPosts,
// US.TotalQuestions,
// US.TotalAnswers
// FROM
// UserStats US
// LEFT JOIN
// BadgeCounts BC ON US.UserId = BC.UserId
// ORDER BY
// US.Reputation DESC, US.Views DESC;
fn q12555(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pc).opt()).and((&bc).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), user_col(db, u, "uviews")];
        f.extend(ints(&b.unwrap_or([0; 4])));
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS TotalGoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS TotalSilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBronzeBadges
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
// UserId,
// DisplayName,
// TotalPosts,
// TotalComments,
// TotalUpVotes,
// TotalDownVotes,
// TotalGoldBadges,
// TotalSilverBadges,
// TotalBronzeBadges
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q12556(db: &'static So) -> String {
    let uf = g(db)
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let mut a = a;
            if let Some((_, t)) = p {
                a[0] += 1;
                a[1] += (t == Some(2)) as i64;
                a[2] += (t == Some(3)) as i64;
            }
            if let Some(c) = b {
                if (1..4).contains(&c) {
                    a[2 + c as usize] += 1;
                }
            }
            a
        });
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&uf).and((&dc).opt()).drive(|u, (a, c)| v.push((u, a, c.unwrap_or(0))));
    out(v, |&(_, a, _)| Reverse(a[0]), 10, |&(u, a, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(c)];
        f.extend(ints(&a[1..]));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalPostScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViewCount,
// AVG(COALESCE(p.Score, 0)) AS AvgPostScore,
// AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// UserBadgeStats AS (
// SELECT
// UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// UserId
// )
// SELECT
// u.DisplayName,
// ups.PostCount,
// ups.TotalPostScore,
// ups.TotalViewCount,
// ups.AvgPostScore,
// ups.AvgViewCount,
// ubs.BadgeCount,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges
// FROM
// Users u
// LEFT JOIN
// UserPostStats ups ON u.Id = ups.UserId
// LEFT JOIN
// UserBadgeStats ubs ON u.Id = ubs.UserId
// ORDER BY
// ups.TotalPostScore DESC, ups.PostCount DESC;
fn q12557(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + 1, a[2] + s, a[3] + w.unwrap_or(0)],
        None => [a[0], a[1] + 1, a[2], a[3]],
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[2]), V::I(a[3]), avg(a[2], a[1]), avg(a[3], a[1])];
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
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
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments GROUP BY PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT ParentId, COUNT(*) AS AnswerCount
// FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId
// ) a ON p.Id = a.ParentId
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// WHERE p.PostTypeId = 1
// ),
// HistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(*) AS EditCount,
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
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.AnswerCount,
// ps.OwnerDisplayName,
// ps.OwnerReputation,
// hs.EditCount,
// hs.LastEditDate
// FROM
// PostStats ps
// LEFT JOIN
// HistoryStats hs ON ps.PostId = hs.PostId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q12560(db: &'static So) -> String {
    let es = db
        .post_history
        .with((&db.post_history.post_history_type_id).in_v(vec![4, 5, 6]))
        .group_by(&db.post_history.post)
        .select(&db.post_history.creation_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and(comments_per_post(db)).and(typed_answers_per_post(db)).and((&es).opt())).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| score_views(db, p), 100, |&(((p, c), a), e)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend([oint(e.map(|e| e.0)), ots(e.map(|e| e.1))]);
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
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
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
// TotalQuestions,
// TotalAnswers,
// TotalViews,
// TotalScore
// FROM
// UserPostStats
// WHERE
// TotalPosts > 0
// ORDER BY
// TotalScore DESC
// LIMIT 10
// )
// SELECT
// tu.DisplayName,
// tu.TotalPosts,
// tu.TotalQuestions,
// tu.TotalAnswers,
// tu.TotalViews,
// tu.TotalScore,
// COUNT(b.Id) AS TotalBadges
// FROM
// TopUsers tu
// LEFT JOIN
// Badges b ON tu.UserId = b.UserId
// GROUP BY
// tu.UserId, tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalViews, tu.TotalScore
// ORDER BY
// tu.TotalScore DESC;
fn q12575(db: &'static So) -> String {
    let uf = upqa(db);
    let pos = (&uf).filt(|a: [i64; 6]| a[0] > 0);
    let top: MatSet<Id<User>> = whole(&pos).select(Same::new().and(&uf)).window(row_number, |(_, a)| a[5], desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let mut v = Vec::new();
    (&top).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64).and(&uf).drive(|u, (b, a)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), V::I(a[5]), V::I(b)]);
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
// P.AcceptedAnswerId,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT PV.UserId) AS VoteCount,
// AVG(CASE WHEN PV.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// AVG(CASE WHEN PV.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes PV ON P.Id = PV.PostId
// WHERE
// P.CreationDate >= '2020-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.AcceptedAnswerId
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// Score,
// AcceptedAnswerId,
// CommentCount,
// VoteCount,
// UpVotes,
// DownVotes
// FROM
// PostStats
// ORDER BY
// Score DESC, ViewCount DESC;
fn q12594(db: &'static So) -> String {
    let dv = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2020, 1, 1)), Ident::<Post>::new(), "cv", &[]).and((&dv).opt()).drive(|p, (s, x)| v.push((p, s, x.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "accepted"]);
        f.extend([V::I(s.cx), V::I(x), stat_field(&s, "up_frac").unwrap(), stat_field(&s, "down_frac").unwrap()]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
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
// ups.PostCount,
// ups.TotalScore,
// ups.TotalViews,
// ups.TotalUpVotes,
// ups.TotalDownVotes,
// ubs.TotalBadges,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY
// ups.TotalScore DESC, ups.PostCount DESC
// LIMIT 100;
fn q12595(db: &'static So) -> String {
    let uf = g(db)
        .select(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((s, w), t)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + (t == Some(2)) as i64, a[4] + (t == Some(3)) as i64],
            None => a,
        });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| (Reverse(a[1]), Reverse(a[0])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f
    })
}

// WITH UserVotes AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM Users U
// LEFT JOIN Votes V ON U.Id = V.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COALESCE(CA.Id, -1) AS AcceptedAnswerId,
// COUNT(C.Id) AS CommentCount,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN Posts CA ON P.AcceptedAnswerId = CA.Id
// WHERE P.PostTypeId = 1
// GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, CA.Id
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.AcceptedAnswerId,
// PS.CommentCount,
// PS.UpVoteCount,
// PS.DownVoteCount,
// UV.TotalVotes,
// UV.TotalUpVotes,
// UV.TotalDownVotes
// FROM PostStats PS
// JOIN UserVotes UV ON PS.AcceptedAnswerId = UV.UserId
// ORDER BY PS.Score DESC, PS.ViewCount DESC
// LIMIT 100;
fn q12597(db: &'static So) -> String {
    let uid = uids(db);
    let uv = user_votes(db);
    let acc = || (&db.post.accepted_answer).select(&db.post.origid).opt().map(|a: Option<i64>| a.unwrap_or(-1));
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cv", &[]).and(acc().and(acc().select(&uid).select(&uv))).drive(|p, (s, (a, x))| v.push((p, s, a, x)));
    out(v, |&(p, _, _, _)| score_views(db, p), 100, |&(p, s, a, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a), V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f.extend(ints(&x));
        f
    })
}

// WITH PostVoteCounts AS (
// SELECT
// p.Id AS PostId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id
// ),
// UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostsCount,
// SUM(COALESCE(ph.VoteCount, 0)) AS TotalVotes,
// SUM(COALESCE(ph.UpVotes, 0)) AS TotalUpVotes,
// SUM(COALESCE(ph.DownVotes, 0)) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostVoteCounts ph ON p.Id = ph.PostId
// GROUP BY
// u.Id
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate,
// ups.PostsCount,
// ups.TotalVotes,
// ups.TotalUpVotes,
// ups.TotalDownVotes
// FROM
// Users u
// JOIN
// UserPostStats ups ON u.Id = ups.UserId
// ORDER BY
// ups.TotalVotes DESC, ups.PostsCount DESC;
fn q12600(db: &'static So) -> String {
    let pv = post_votes(db);
    let uf = g(db).select(posts_of(db).select((&pv).opt()).opt()).fold([0i64; 4], |a, p| match p {
        Some(x) => {
            let x = x.unwrap_or([0; 3]);
            [a[0] + 1, a[1] + x[0], a[2] + x[1], a[3] + x[2]]
        }
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let mut f: Vec<V> = ["name", "rep", "ucreated", "last_access"].iter().map(|c| user_col(db, u, c)).collect();
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// COUNT(C.Id) AS TotalComments,
// SUM(COALESCE(P.Score, 0)) AS TotalPostScore,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalComments,
// TotalPostScore,
// TotalUpvotes,
// TotalDownvotes,
// (TotalUpvotes - TotalDownvotes) AS NetVotes
// FROM UserPostStats
// ORDER BY NetVotes DESC
// LIMIT 10;
fn q12606(db: &'static So) -> String {
    out(users_with_counts(db, "cv", false), |r| Reverse(r.agg.up - r.agg.down), 10, |r| {
        let mut f = user_fields(r, "cv", &["uid", "name", "#rows", "#cx", "score_sum0", "#up", "#down"]);
        f.push(V::I(r.agg.up - r.agg.down));
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COALESCE(c.CommentCount, 0) AS TotalComments,
// COALESCE(v.VoteCount, 0) AS TotalVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId) v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12613(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, date(2024, 9, 1)).select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, c), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep"]);
        f.extend([V::I(c), V::I(x)]);
        f
    })
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
// P.ClosedDate,
// U.Reputation AS OwnerReputation,
// P.OwnerUserId
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// ),
// VoteStats AS (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// COUNT(*) AS TotalVotes
// FROM
// Votes
// GROUP BY
// PostId
// ),
// CommentStats AS (
// SELECT
// PostId,
// COUNT(*) AS TotalComments
// FROM
// Comments
// GROUP BY
// PostId
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
// PS.ClosedDate,
// PS.OwnerReputation,
// COALESCE(VS.Upvotes, 0) AS Upvotes,
// COALESCE(VS.Downvotes, 0) AS Downvotes,
// COALESCE(VS.TotalVotes, 0) AS TotalVotes,
// COALESCE(CS.TotalComments, 0) AS TotalComments
// FROM
// PostStats PS
// LEFT JOIN
// VoteStats VS ON PS.PostId = VS.PostId
// LEFT JOIN
// CommentStats CS ON PS.PostId = CS.PostId
// ORDER BY
// PS.CreationDate DESC;
fn q12621(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&pv).opt()).and(comments_per_post(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), c)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views", "answers", "comments", "favorites", "closed", "rep"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(x[0]), V::I(c)]);
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// AVG(COALESCE(p.ViewCount, 0)) AS AverageViewsPerPost,
// AVG(COALESCE(p.Score, 0)) AS AverageScorePerPost,
// COUNT(DISTINCT CASE WHEN bh.UserId IS NOT NULL THEN bh.UserId END) AS UniqueEditors,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// PostHistory bh ON bh.PostId = p.Id
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12638(db: &'static So) -> String {
    let base = || since(db, year_ago());
    let f = by_key(
        base(),
        name(db),
        (&db.post.view_count).opt().and(&db.post.score).and(history_of(db).opt()).and(comments_of(db).opt()),
        [0i64; 3],
        |a, (((w, s), _), _)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s],
    );
    let ed = base().group_by(name(db)).select(history_of(db).select(&db.post_history.user_id)).count_distinct();
    let dc = base().group_by(name(db)).select(comments_of(db)).count_distinct();
    let mut v = Vec::new();
    (&f).and((&ed).opt()).and((&dc).opt()).drive(|k, ((a, e), c)| v.push((k, a, e.unwrap_or(0), c.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, e, c)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[1], a[0]), avg(a[2], a[0]), V::I(e), V::I(c)])))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.Reputation
// ),
// UserStats AS (
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
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.OwnerReputation,
// ps.CommentCount,
// us.DisplayName AS OwnerDisplayName,
// us.BadgeCount
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerReputation = us.UserId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q12641(db: &'static So) -> String {
    let uid = uids(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db)
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.owner_user).select(&db.user.reputation).select(&uid).select(Ident::<User>::new().and(&bu))))
        .drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, c), (u, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "rep"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(b)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.PostTypeId,
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AverageViewCount,
// AVG(Score) AS AverageScore,
// AVG(AnswerCount) AS AverageAnswerCount,
// AVG(CommentCount) AS AverageCommentCount
// FROM Posts P
// WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY P.PostTypeId
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS TotalBadges,
// SUM(U.UpVotes) AS TotalUpVotes,
// SUM(U.DownVotes) AS TotalDownVotes,
// AVG(U.Reputation) AS AverageReputation
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id
// )
// SELECT
// PST.PostTypeId,
// PST.TotalPosts,
// PST.AverageViewCount,
// PST.AverageScore,
// PST.AverageAnswerCount,
// PST.AverageCommentCount,
// US.TotalBadges,
// US.TotalUpVotes,
// US.TotalDownVotes,
// US.AverageReputation
// FROM PostStats PST
// JOIN UserStats US ON US.UserId = (SELECT MIN(Id) FROM Users)
// ORDER BY PST.PostTypeId;
fn q12643(db: &'static So) -> String {
    let Post { view_count, score, answer_count, comment_count, .. } = &db.post;
    let f = by_key(since(db, year_ago()), &db.post.post_type_id, view_count.opt().and(score).and(answer_count.opt()).and(comment_count), [0i64; 7], |a, (((w, s), an), cc)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + cc]
    });
    let umin = db.user.select(&db.user.origid).fold_flat(i64::MAX, |m, x| m.min(x));
    let us = g(db).select((&db.user.up_votes).and(&db.user.down_votes).and(&db.user.reputation).and(badges_of(db).opt())).fold([0i64; 5], |a, (((u, d), r), b)| {
        [a[0] + b.is_some() as i64, a[1] + u, a[2] + d, a[3] + r, a[4] + 1]
    });
    let one_u: HashIdx<Id<User>, [i64; 5]> = db.user.with((&db.user.origid).eq(umin)).select(&us).collect();
    let mut v = Vec::new();
    (&f).cross(&one_u).drive(|(k, _), (a, ua)| v.push((k, a, ua)));
    rows(v.iter().map(|&(k, a, ua)| row(vec![V::I(k), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), avg(a[5], a[4]), avg(a[6], a[0]), V::I(ua[0]), V::I(ua[1]), V::I(ua[2]), avg(ua[3], ua[4])])))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(P.Id) AS TotalPosts,
// COUNT(C.Id) AS TotalComments,
// COALESCE(SUM(VoteCount), 0) AS TotalVotes,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(Id) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId) V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// TotalVotes DESC, TotalPosts DESC;
fn q12646(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let uf = g(db).select(posts_of(db).select((&db.post.view_count).opt().and(comments_of(db).opt()).and((&vf).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((w, c), x)) => [a[0] + 1, a[1] + c.is_some() as i64, a[2] + x.unwrap_or(0), a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3])])))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.Score ELSE 0 END) AS QuestionsScore,
// SUM(CASE WHEN P.PostTypeId = 2 THEN P.Score ELSE 0 END) AS AnswersScore,
// AVG(U.Reputation) AS AvgReputation
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostDetails AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(CM.Id) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// Comments CM ON P.Id = CM.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.PostCount,
// US.QuestionsCount,
// US.AnswersCount,
// US.QuestionsScore,
// US.AnswersScore,
// US.AvgReputation,
// PD.PostId,
// PD.Title,
// PD.CreationDate,
// PD.ViewCount,
// PD.Score,
// PD.CommentCount
// FROM
// UserStats US
// JOIN
// PostDetails PD ON US.UserId = PD.PostId
// ORDER BY
// US.AvgReputation DESC, PD.ViewCount DESC
// LIMIT 100;
fn q12647(db: &'static So) -> String {
    let uid = uids(db);
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }, a[4] + if t == 2 { s } else { 0 }],
        None => a,
    });
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uf))))
        .drive(|_, x| v.push(x));
    out(v, |&((p, _), (u, _))| (rep_desc(db, u), views_desc(db, p)), 100, |&((p, c), (u, a))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.push(V::I(c));
        f
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON c.PostId = p.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId) v ON v.PostId = p.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12653(db: &'static So) -> String {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let f = db
        .post_type
        .group_by(&db.post_type.name)
        .select((&of_type).select((&db.post.score).and(comments_per_post(db)).and(votes_per_post(db))).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((s, c), x)) => [a[0] + 1, a[1] + s, a[2] + c, a[3] + x],
            None => a,
        });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])])))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(a.Id) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.OwnerUserId
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Posts a ON p.Id = a.ParentId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.Score) AS TotalPostScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.AnswerCount,
// ps.UpVotes,
// ps.DownVotes,
// us.DisplayName AS PostOwner,
// us.BadgeCount,
// us.TotalPostScore,
// us.TotalViews
// FROM PostStats ps
// JOIN UserStats us ON ps.OwnerUserId = us.UserId
// ORDER BY ps.CreationDate DESC
// LIMIT 100;
fn q12659(db: &'static So) -> String {
    let top: MatSet<Id<Post>> = whole(owned(db)).select(Ident::<Post>::new().and(&db.post.creation_date)).window(rank, |(_, d)| d, desc).filt(|(_, r)| r <= 100).map(|((p, _), _)| p).collect();
    let need: MatSet<Id<User>> = db.post.with(&top).select(&db.post.owner_user).collect();
    let us = db
        .user
        .with(&need)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt())).opt()))
        .fold([0i64; 5], |a, (b, p)| {
            let (s, w) = p.map_or((None, None), |(s, w)| (Some(s), w));
            [a[0] + b.is_some() as i64, a[1] + s.is_some() as i64, a[2] + s.unwrap_or(0), a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
        });
    let mut v = Vec::new();
    stats_fold(db, db.post.with(&top), Ident::<Post>::new(), "cav", &[]).and((&db.post.owner_user).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, _, _)| newest(db, p), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.ax), V::I(s.up), V::I(s.down), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), nullable(a[4], a[3])]);
        f
    })
}

// WITH UserVoteStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(ps.Id) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts ps ON p.Id = ps.ParentId AND ps.PostTypeId = 2
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate
// )
// SELECT
// u.DisplayName,
// u.VoteCount,
// u.UpVotes,
// u.DownVotes,
// p.PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.CommentCount,
// p.AnswerCount
// FROM
// UserVoteStats u
// JOIN
// PostStats p ON u.UserId = p.PostId
// ORDER BY
// u.VoteCount DESC, p.Score DESC;
fn q12660(db: &'static So) -> String {
    let uid = uids(db);
    let uv = user_votes(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cA", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uv)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(s.cx), V::I(s.ax)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount,
// COUNT(DISTINCT ph.Id) AS EditHistoryCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, u.DisplayName, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, c.CommentCount, v.VoteCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12668(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)).and(history_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, c), x), h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "views", "score", "answers"]);
        f.extend([V::I(c), V::I(x), V::I(h)]);
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
// p.CommentCount,
// p.FavoriteCount,
// COUNT(c.Id) AS CommentTotal,
// COUNT(DISTINCT v.Id) AS VoteTotal
// FROM
// Posts p
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.Score) AS UserPostScore
// FROM
// Users u
// LEFT JOIN
// Badges b ON b.UserId = u.Id
// LEFT JOIN
// Posts p ON p.OwnerUserId = u.Id
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.CommentTotal,
// ps.VoteTotal,
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.BadgeCount,
// us.UserPostScore
// FROM
// PostStats ps
// JOIN
// Posts p ON ps.PostId = p.Id
// JOIN
// UserStats us ON us.UserId = p.OwnerUserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q12675(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    let us = g(db).select(badges_of(db).opt().and(posts_of(db).select(&db.post.score).opt())).fold([0i64; 3], |a, (b, s)| [a[0] + b.is_some() as i64, a[1] + s.is_some() as i64, a[2] + s.unwrap_or(0)]);
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&x).opt()).and((&db.post.owner_user).select(Ident::<User>::new().and(&us))).drive(|p, ((s, x), (u, a))| v.push((p, s, x.unwrap_or(0), u, a)));
    rows(v.iter().map(|&(p, s, x, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites"]);
        f.extend([V::I(s.cx), V::I(x), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), nullable(a[2], a[1])]);
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
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// AVG(CASE WHEN p.OwnerUserId IS NOT NULL THEN p.Score ELSE NULL END) AS AvgPostScore,
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
// u.Id, u.DisplayName
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.AvgPostScore,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges,
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
// us.AvgPostScore DESC,
// ps.Score DESC;
fn q12678(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), ustat_field(&a, "score_avg")];
        f.extend(["#gold", "#silver", "#bronze"].iter().map(|c| ustat_field(&a, c)));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(s.cx), V::I(s.vx)]);
        row(f)
    }))
}

// WITH Benchmarking AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COALESCE(a.AcceptedAnswerId, -1) AS AcceptedAnswerId,
// a.Score AS AcceptedAnswerScore
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1
// WHERE
// p.CreationDate >= '2023-01-01'
// AND p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score, p.ViewCount, a.AcceptedAnswerId, a.Score
// )
// SELECT
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// AVG(CommentCount) AS AvgCommentCount,
// COUNT(*) AS TotalQuestions,
// COUNT(CASE WHEN AcceptedAnswerId != -1 THEN 1 END) AS QuestionsWithAcceptedAnswer
// FROM
// Benchmarking;
fn q12680(db: &'static So) -> String {
    type K = (Id<Post>, Option<(Option<i64>, i64)>);
    let Post { view_count, score, .. } = &db.post;
    let key = Ident::<Post>::new().and(children_of(db).select((&db.post.accepted_answer_id).opt().and(score)).opt());
    let f = since(db, date(2023, 1, 1)).with((&db.post.post_type_id).eq(1)).group_by(key).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let post = || Same::<K>::new().map(|(p, _): K| p);
    let has_acc = Same::<K>::new().map(|(_, a): K| a.is_some_and(|(x, _)| x.is_some()));
    let a = (&f).and(post().select(view_count.opt().and(score))).and(has_acc).fold_flat([0i64; 6], |a, ((c, (w, s)), h)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + c, a[5] + h as i64]
    });
    row(vec![avg(a[2], a[1]), avg(a[3], a[0]), avg(a[4], a[0]), V::I(a[0]), V::I(a[5])])
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount,
// p.OwnerUserId  -- Added OwnerUserId to GROUP BY clause
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId  -- Group by all selected columns
// ),
// UserMetrics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(b.Class) AS TotalBadgeCount,
// AVG(u.Reputation) AS AverageReputation,
// COUNT(DISTINCT p.Id) AS PostCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName  -- Group by all selected columns
// )
// SELECT
// pm.PostId,
// pm.Title,
// pm.CreationDate,
// pm.Score,
// pm.ViewCount,
// pm.CommentCount,
// pm.VoteCount,
// pm.TotalBountyAmount,
// um.UserId,
// um.DisplayName,
// um.TotalBadgeCount,
// um.AverageReputation,
// um.PostCount
// FROM
// PostMetrics pm
// JOIN
// UserMetrics um ON pm.OwnerUserId = um.UserId
// ORDER BY
// pm.Score DESC, pm.ViewCount DESC;
fn q12688(db: &'static So) -> String {
    let us = g(db).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold([0i64; 2], |a, (b, _)| [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp)))
        .drive(|p, (s, ((u, a), d))| v.push((p, s, u, a, d)));
    rows(v.iter().map(|&(p, s, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.bounty_sum), user_col(db, u, "uid"), user_col(db, u, "name"), nullable(a[1], a[0])]);
        f.extend([V::F(db.user.reputation.get(u).unwrap() as f64), V::I(d)]);
        row(f)
    }))
}

// WITH PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners
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
// CommentCounts AS (
// SELECT
// COUNT(*) AS TotalComments
// FROM
// Comments
// ),
// VoteCounts AS (
// SELECT
// COUNT(*) AS TotalVotes
// FROM
// Votes
// ),
// BadgeCounts AS (
// SELECT
// COUNT(*) AS TotalBadges
// FROM
// Badges
// )
// SELECT
// p.TotalPosts,
// p.UniquePostOwners,
// u.TotalUsers,
// u.AverageReputation,
// c.TotalComments,
// v.TotalVotes,
// b.TotalBadges
// FROM
// PostCounts p,
// UserCounts u,
// CommentCounts c,
// VoteCounts v,
// BadgeCounts b;
fn q12698(db: &'static So) -> String {
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    row(vec![V::I(count(db.post.iq())), V::I(owners), V::I(un), avg(rs, un), V::I(count(db.comment.iq())), V::I(count(db.vote.iq())), V::I(count(db.badge.iq()))])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId) v ON p.Id = v.PostId
// GROUP BY
// u.Id,
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 100;
fn q12704(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(creation_date).and(comments_per_post(db)).and(votes_per_post(db))).opt()).fold([0, 0, 0, 0, 0, i64::MIN], |a: [i64; 6], p| match p {
        Some((((t, cd), c), x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c, a[4] + x, a[5].max(cd)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| Reverse(a[0]), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..5]));
        f.push(if a[0] == 0 { V::Null } else { V::T(a[5]) });
        f
    })
}

// WITH UserPostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS PostCount,
// SUM(ViewCount) AS TotalViews,
// SUM(Score) AS TotalScore
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// UserBadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ),
// UserMetrics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UPC.PostCount, 0) AS PostCount,
// COALESCE(UPC.TotalViews, 0) AS TotalViews,
// COALESCE(UPC.TotalScore, 0) AS TotalScore,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// UserPostCounts UPC ON U.Id = UPC.OwnerUserId
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// TotalViews,
// TotalScore,
// BadgeCount
// FROM
// UserMetrics
// ORDER BY
// PostCount DESC,
// TotalScore DESC;
fn q12711(db: &'static So) -> String {
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&db.post.score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt()).and(&bu)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p.unwrap_or([0; 3])));
        f.push(V::I(b));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COUNT(DISTINCT ph.Id) AS PostHistoryCount
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
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.Score DESC, p.ViewCount DESC
// LIMIT 100;
fn q12717(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, year_ago()), Ident::<Post>::new(), "cvh", &[]).and((&h).opt()).drive(|p, (s, h)| v.push((p, s, h.unwrap_or(0))));
    out(v, |&(p, _, _)| (score_views(db, p), db.post.origid.get(p).unwrap()), 100, |&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(h)]);
        f
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
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
fn q12718(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_per_post(db)), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// WITH PostEngagement AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// COALESCE(COUNT(C.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// Score,
// AnswerCount,
// CommentCount,
// Upvotes,
// Downvotes,
// (ViewCount + Score + AnswerCount + CommentCount + Upvotes - Downvotes) AS EngagementScore
// FROM
// PostEngagement
// ORDER BY
// EngagementScore DESC
// FETCH FIRST 10 ROWS ONLY;
fn q12719(db: &'static So) -> String {
    let eng = |p: Id<Post>, s: &Stats| -> Option<i64> {
        Some(db.post.view_count.get(p)? + db.post.score.get(p).unwrap() + db.post.answer_count.get(p)? + s.cx + s.up - s.down)
    };
    out(stats_with(db, questions_only(db), "cv", &[], &[]), |&(p, s, _)| (eng(p, &s).is_none(), Reverse(eng(p, &s))), 10, |&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), oint(eng(p, &s))]);
        f
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount
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
// COUNT(p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 1) AS QuestionCount,
// COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 2) AS AnswerCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// ub.BadgeCount,
// ps.PostCount,
// ps.TotalScore,
// ps.TotalViews,
// ps.QuestionCount,
// ps.AnswerCount
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// COALESCE(BadgeCount, 0) AS BadgeCount,
// COALESCE(PostCount, 0) AS PostCount,
// COALESCE(TotalScore, 0) AS TotalScore,
// COALESCE(TotalViews, 0) AS TotalViews,
// COALESCE(QuestionCount, 0) AS QuestionCount,
// COALESCE(AnswerCount, 0) AS AnswerCount
// FROM
// UserPerformance
// ORDER BY
// Reputation DESC,
// TotalScore DESC;
fn q12720(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let pf = owned(db)
        .group_by(&db.post.owner_user)
        .select((&db.post.score).and((&db.post.view_count).opt()).and(&db.post.post_type_id))
        .fold([0i64; 5], |a, ((s, w), t)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + (t == 1) as i64, a[4] + (t == 2) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        f.extend(ints(&p.unwrap_or([0; 5])));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.CreationDate,
// U.Views,
// U.UpVotes,
// U.DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.Reputation, U.CreationDate, U.Views, U.UpVotes, U.DownVotes
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// AVG(P.AnswerCount) AS AvgAnswerCount,
// AVG(P.CommentCount) AS AvgCommentCount
// FROM Posts P
// GROUP BY P.OwnerUserId
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.Views,
// U.UpVotes,
// U.DownVotes,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.AvgAnswerCount, 0) AS AvgAnswerCount,
// COALESCE(PS.AvgCommentCount, 0) AS AvgCommentCount,
// U.BadgeCount
// FROM UserStats U
// LEFT JOIN PostStats PS ON U.UserId = PS.OwnerUserId
// ORDER BY U.Reputation DESC;
fn q12724(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let Post { view_count, score, answer_count, comment_count, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(view_count.opt().and(score).and(answer_count.opt()).and(comment_count)).fold([0i64; 6], |a, (((w, s), an), cc)| {
        [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + an.is_some() as i64, a[4] + an.unwrap_or(0), a[5] + cc]
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&dp).opt()).and(&bu).and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, _), b), p)| {
        let mut f: Vec<V> = ["uid", "rep", "uviews", "uup", "udown"].iter().map(|c| user_col(db, u, c)).collect();
        let p = p.unwrap_or([0; 6]);
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), or0(p[4], p[3]), or0(p[5], p[0]), V::I(b)]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// SUM(p.AnswerCount) AS TotalAnswerCount
// FROM Posts p
// GROUP BY p.OwnerUserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// us.BadgeCount,
// us.TotalUpVotes,
// us.TotalDownVotes,
// us.TotalViews,
// us.AverageScore,
// ps.TotalPosts,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.TotalAnswerCount
// FROM Users u
// LEFT JOIN UserStats us ON u.Id = us.UserId
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// ORDER BY u.Reputation DESC
// LIMIT 100;
fn q12742(db: &'static So) -> String {
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt()).and(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)).opt())).fold([0i64; 7], |a, (((u, d), b), p)| {
        let mut a = [a[0] + b.is_some() as i64, a[1] + u, a[2] + d, a[3], a[4], a[5], a[6]];
        if let Some((w, s)) = p {
            a[3] += w.is_some() as i64;
            a[4] += w.unwrap_or(0);
            a[5] += 1;
            a[6] += s;
        }
        a
    });
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and((&db.post.answer_count).opt())).fold([0i64; 5], |a, (t, an)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + an.is_some() as i64, a[4] + an.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&us).and((&pf).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    out(v, |&(u, _, _)| rep_desc(db, u), 100, |&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), avg(a[6], a[5])];
        match p {
            Some(p) => f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[4], p[3])]),
            None => f.extend(nulls(4)),
        }
        f
    })
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score AS PostScore,
// p.ViewCount,
// COALESCE(vote_counts.UpVotes, 0) AS UpVotes,
// COALESCE(vote_counts.DownVotes, 0) AS DownVotes,
// COALESCE(vote_counts.TotalVotes, 0) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(*) AS TotalVotes
// FROM
// Votes
// GROUP BY
// PostId) AS vote_counts ON p.Id = vote_counts.PostId
// WHERE
// p.Id IS NOT NULL
// ORDER BY
// u.Reputation DESC, p.CreationDate DESC;
fn q12754(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&pv).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["uid", "owner", "rep", "id", "title", "created", "score", "views"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(x[0])]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN v.VoteTypeId = 4 THEN 1 ELSE 0 END) AS OffensiveVotes,
// MAX(p.CreationDate) AS LastActivityDate
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY p.Id, p.PostTypeId
// ),
// UserStatistics AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// SUM(u.Views) AS TotalViews
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// )
// SELECT
// ps.PostId,
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.OffensiveVotes,
// ps.LastActivityDate,
// us.UserId,
// us.BadgeCount,
// us.TotalUpVotes,
// us.TotalDownVotes,
// us.TotalViews
// FROM PostStatistics ps
// JOIN UserStatistics us ON ps.PostTypeId = us.UserId
// ORDER BY ps.LastActivityDate DESC;
fn q12755(db: &'static So) -> String {
    let uid = uids(db);
    let User { up_votes, down_votes, views, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(views).and(badges_of(db).opt())).fold([0i64; 4], |a, (((u, d), w), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d, a[3] + w]);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.post_type_id).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.post_type_id).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), V::I(s.by_vt[4])]);
        f.extend(post_fields(db, p, &["created"]));
        f.push(user_col(db, u, "uid"));
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// SUM(c.CommentCount) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// QuestionCount,
// AnswerCount,
// TotalViews,
// TotalScore,
// TotalComments
// FROM
// UserEngagement
// ORDER BY
// QuestionCount DESC, TotalScore DESC;
fn q12761(db: &'static So) -> String {
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let uf = owned(db)
        .with((&db.post.post_type_id).eq(1))
        .group_by(&db.post.owner_user)
        .select((&db.post.post_type_id).and((&db.post.view_count).opt()).and(&db.post.score).and((&cf).opt()))
        .fold([0i64; 7], |a, (((t, w), s), c)| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s, a[5] + c.is_some() as i64, a[6] + c.unwrap_or(0)]);
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4]), nullable(a[6], a[5])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12768(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_per_post(db)), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// WITH PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount
// FROM Posts p
// LEFT JOIN (
// SELECT
// ParentId,
// COUNT(*) AS AnswerCount
// FROM Posts
// WHERE PostTypeId = 2
// GROUP BY ParentId
// ) a ON p.Id = a.ParentId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId
// ) v ON p.Id = v.PostId
// WHERE p.PostTypeId = 1
// )
// SELECT
// pa.PostId,
// pa.Title,
// pa.CreationDate,
// pa.ViewCount,
// pa.AnswerCount,
// pa.CommentCount,
// pa.VoteCount,
// (pa.ViewCount + pa.AnswerCount + pa.CommentCount + pa.VoteCount) AS TotalEngagement
// FROM PostActivity pa
// ORDER BY TotalEngagement DESC
// LIMIT 10;
fn q12769(db: &'static So) -> String {
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    let tot = |p: Id<Post>, a: i64, c: i64, x: i64| db.post.view_count.get(p).map(|w| w + a + c + x);
    out(v, |&(((p, a), c), x)| (tot(p, a, c, x).is_none(), Reverse(tot(p, a, c, x))), 10, |&(((p, a), c), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(a), V::I(c), V::I(x), oint(tot(p, a, c, x))]);
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Posts p2 WHERE p2.ParentId = p.Id) AS AnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q12777(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    let c = per_post_distinct(db, comments_of(db));
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and((&x).opt()).and((&c).opt()).and(answers_per_post(db))).drive(|_, y| v.push(y));
    rows(v.iter().map(|&(((p, x), c), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.extend([V::I(x.unwrap_or(0)), V::I(c.unwrap_or(0)), V::I(a)]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
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
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId
// ),
// VoteStats AS (
// SELECT
// v.PostId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Votes v
// GROUP BY
// v.PostId
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.BadgeCount,
// us.QuestionCount,
// us.AnswerCount,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// vs.UpVotes,
// vs.DownVotes
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// LEFT JOIN
// VoteStats vs ON ps.PostId = vs.PostId
// ORDER BY
// us.Reputation DESC, ps.Score DESC;
fn q12780(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(comments_per_post(db)).and((&pv).opt()).and((&db.post.owner_user).select(Ident::<User>::new().and(&us)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), x), (u, a))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a.bx), V::I(a.q), V::I(a.a)];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), oint(x.map(|x| x[1])), oint(x.map(|x| x[2]))]);
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount,
// COALESCE(A.AnswerCount, 0) AS AnswerCount,
// COALESCE(C.CommentCount, 0) AS CommentCount,
// COALESCE(F.FavoriteCount, 0) AS FavoriteCount
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
// ParentId) A ON P.Id = A.ParentId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS FavoriteCount
// FROM
// Votes
// WHERE
// VoteTypeId = 5
// GROUP BY
// PostId) F ON P.Id = F.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.Score DESC
// LIMIT 100;
fn q12790(db: &'static So) -> String {
    let fav = votes_of_type(db, 5);
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and(&fav)).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| score_desc(db, p), 100, |&(((p, a), c), f5)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.extend([V::I(a), V::I(c), V::I(f5)]);
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
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AvgScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AvgScore, 0) AS AvgScore
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// ORDER BY
// U.Reputation DESC
// LIMIT 100;
fn q12794(db: &'static So) -> String {
    let bc = badge_classes(db);
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&bc).opt()).and((&pf).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), _)| rep_desc(db, u), 100, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b.unwrap_or([0; 4])));
        let p = p.unwrap_or([0; 3]);
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), or0(p[1], p[0])]);
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
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// p.AnswerCount,
// (SELECT COUNT(*) FROM PostLinks pl WHERE pl.PostId = p.Id) AS RelatedPostCount,
// (SELECT COUNT(*) FROM PostHistory h WHERE h.PostId = p.Id) AS RevisionCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, p.AnswerCount
// ORDER BY
// p.CreationDate DESC;
fn q12799(db: &'static So) -> String {
    let lc = (&db.post_link.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cv", &[]).and(&lc).and(history_per_post(db)).drive(|p, ((s, l), h)| v.push((p, s, l, h)));
    rows(v.iter().map(|&(p, s, l, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["answers"]));
        f.extend([V::I(l), V::I(h)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("12509", q12509),
    ("12523", q12523),
    ("12525", q12525),
    ("12529", q12529),
    ("12533", q12533),
    ("12546", q12546),
    ("12555", q12555),
    ("12556", q12556),
    ("12557", q12557),
    ("12560", q12560),
    ("12575", q12575),
    ("12594", q12594),
    ("12595", q12595),
    ("12597", q12597),
    ("12600", q12600),
    ("12606", q12606),
    ("12613", q12613),
    ("12621", q12621),
    ("12638", q12638),
    ("12641", q12641),
    ("12643", q12643),
    ("12646", q12646),
    ("12647", q12647),
    ("12653", q12653),
    ("12659", q12659),
    ("12660", q12660),
    ("12668", q12668),
    ("12675", q12675),
    ("12678", q12678),
    ("12680", q12680),
    ("12688", q12688),
    ("12698", q12698),
    ("12704", q12704),
    ("12711", q12711),
    ("12717", q12717),
    ("12718", q12718),
    ("12719", q12719),
    ("12720", q12720),
    ("12724", q12724),
    ("12742", q12742),
    ("12754", q12754),
    ("12755", q12755),
    ("12761", q12761),
    ("12768", q12768),
    ("12769", q12769),
    ("12777", q12777),
    ("12780", q12780),
    ("12790", q12790),
    ("12794", q12794),
    ("12799", q12799),
];
