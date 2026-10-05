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

fn obadges(db: &'static So) -> HashIdx<i64, Id<Badge>> {
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

// --- batch 119 --------------------------------------------------------------

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
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// us.DisplayName AS OwnerDisplayName,
// us.BadgeCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q12142(db: &'static So) -> String {
    let uid = uids(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt())).fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, _, _)| newest(db, p), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "name")]);
        f.extend(ints(&a));
        f
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT c.Id) AS TotalComments,
// (SELECT COUNT(*) FROM Users) AS TotalUsers
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
fn q12143(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_of(db).opt()), [0i64; 2], |a, (s, _)| [a[0] + 1, a[1] + s]);
    let dc = db.post.group_by(name(db)).select(comments_of(db)).count_distinct();
    let tu = count(db.user.iq());
    let mut v = Vec::new();
    (&f).and((&dc).opt()).drive(|k, (a, c)| v.push((k, a, c.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, c)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(c), V::I(tu)])))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
// p.ViewCount,
// (SELECT COUNT(*) FROM Comments WHERE PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes WHERE PostId = p.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.ViewCount
// ORDER BY
// TotalVotes DESC, TotalComments DESC, p.ViewCount DESC
// LIMIT 100;
fn q12144(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and((&c).opt()).and((&x).opt()).and(comments_per_post(db)).and(votes_per_post(db)).drive(|p, ((((s, c), x), cc), vv)| {
        v.push((p, s, c.unwrap_or(0), x.unwrap_or(0), cc, vv))
    });
    out(v, |&(p, _, c, x, _, _)| (Reverse(x), Reverse(c), views_desc(db, p)), 100, |&(p, s, c, x, cc, vv)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(x), V::I(c), V::I(s.bounty_sum)]);
        f.extend(post_fields(db, p, &["views"]));
        f.extend([V::I(cc), V::I(vv)]);
        f
    })
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// P.Tags,
// PT.Name AS PostType,
// P.OwnerUserId
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.TotalComments,
// UA.TotalBounties,
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.FavoriteCount,
// PS.Tags,
// PS.PostType
// FROM
// UserActivity UA
// JOIN
// PostStatistics PS ON UA.UserId = PS.OwnerUserId
// ORDER BY
// UA.TotalPosts DESC, PS.ViewCount DESC;
fn q12145(db: &'static So) -> String {
    let ua = g(db).select(posts_of(db).select(comments_of(db).opt()).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold(0i64, |a, (_, b)| a + b.flatten().unwrap_or(0));
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&ua).and(&dp).and((&dc).opt())))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, (((u, b), d), c))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(c.unwrap_or(0)), V::I(b)];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "tags", "type"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostID,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// p.ViewCount AS Views,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT ph.Id) AS EditCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount
// ORDER BY
// Views DESC;
fn q12161(db: &'static So) -> String {
    let bi = obadges(db);
    let c = per_post_distinct(db, comments_of(db));
    let h = per_post_distinct(db, history_of(db));
    let b = per_post_distinct(db, (&db.post.owner_user_id).select(&bi));
    let l = per_post_distinct(db, links_of(db).select(&db.post_link.related_post_id));
    let f = since(db, year_ago())
        .group_by(Ident::<Post>::new())
        .select(
            votes_of(db)
                .select(&db.vote.vote_type_id)
                .opt()
                .and(comments_of(db).opt())
                .and(history_of(db).opt())
                .and((&db.post.owner_user_id).select(&bi).opt())
                .and(links_of(db).opt()),
        )
        .fold([0i64; 2], |a, ((((t, _), _), _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let mut v = Vec::new();
    (&f).and((&c).opt())
        .and((&h).opt())
        .and((&b).opt())
        .and((&l).opt())
        .drive(|p, ((((s, c), h), b), l)| v.push((p, s, [c, h, b, l].map(|x| x.unwrap_or(0)))));
    rows(v.iter().map(|&(p, s, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(ints(&s));
        f.extend(ints(&d));
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
// COUNT(b.Id) AS TotalBadges,
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
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalViews,
// ups.TotalScore,
// ubs.TotalBadges,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges
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
// TotalViews,
// TotalScore,
// COALESCE(TotalBadges, 0) AS TotalBadges,
// COALESCE(GoldBadges, 0) AS GoldBadges,
// COALESCE(SilverBadges, 0) AS SilverBadges,
// COALESCE(BronzeBadges, 0) AS BronzeBadges
// FROM
// CombinedStats
// ORDER BY
// TotalScore DESC, TotalPosts DESC;
fn q12163(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), nullable(a[5], a[0])]);
        f.extend(ints(&b));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Owner,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// (SELECT COUNT(*) FROM Posts WHERE AcceptedAnswerId = p.Id) AS AcceptedAnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12178(db: &'static So) -> String {
    let acc = (&db.post.accepted_answer).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    stats_fold(db, owned(db).with((&db.post.post_type_id).eq(1)), Ident::<Post>::new(), "cv", &[]).and(&acc).drive(|p, (s, a)| v.push((p, s, a)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, s, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(a)]);
        f
    })
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.OwnerUserId,
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
// p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.OwnerUserId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// rp.PostId,
// rp.Title,
// rp.CreationDate,
// ur.DisplayName AS OwnerDisplayName,
// ur.Reputation AS OwnerReputation,
// rp.CommentCount,
// rp.UpVoteCount,
// rp.DownVoteCount,
// ur.BadgeCount
// FROM
// RecentPosts rp
// JOIN
// UserReputation ur ON rp.OwnerUserId = ur.UserId
// ORDER BY
// rp.CreationDate DESC;
fn q12180(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let base = owned(db).with((&db.post.creation_date).gt(ts(2024, 9, 1, 12, 34, 56)));
    let mut v = Vec::new();
    stats_fold(db, base, Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(&bu)).drive(|p, (s, b)| v.push((p, s, b)));
    rows(v.iter().map(|&(p, s, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "rep"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(b)]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT B.Id) AS TotalBadges,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Tags,
// P.CreationDate,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// P.OwnerUserId
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.Tags, P.CreationDate, P.OwnerUserId
// )
// SELECT
// U.DisplayName,
// U.TotalBadges,
// U.TotalQuestions,
// U.TotalAnswers,
// U.TotalScore,
// U.TotalViews,
// P.Title,
// P.Tags,
// P.CreationDate,
// P.CommentCount,
// P.VoteCount
// FROM
// UserStats U
// JOIN
// PostStats P ON U.UserId = P.OwnerUserId
// ORDER BY
// U.TotalScore DESC, P.VoteCount DESC;
fn q12189(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let us = g(db).select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())).fold([0i64; 6], |a, (_, p)| match p {
        Some(((t, s), w)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + 1, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
        None => a,
    });
    let bu = badges_per_user(db);
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and((&db.post.owner_user).select((&us).and(&bu)))).drive(|_, y| v.push(y));
    rows(v.iter().map(|&(((p, c), x), (a, b))| {
        let mut f = vec![user_col(db, db.post.owner_user.get(p).unwrap(), "name"), V::I(b), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), nullable(a[5], a[4])];
        f.extend(post_fields(db, p, &["title", "tags", "created"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0))]);
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
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// p.OwnerUserId,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, u.Reputation
// ),
// PostTypeCounts AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(ps.PostId) AS TotalPosts,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.VoteCount) AS TotalVotes,
// AVG(ps.Score) AS AverageScore,
// AVG(ps.ViewCount) AS AverageViews,
// AVG(ps.OwnerReputation) AS AverageOwnerReputation
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostTypeName,
// TotalPosts,
// TotalComments,
// TotalVotes,
// AverageScore,
// AverageViews,
// AverageOwnerReputation
// FROM
// PostTypeCounts
// ORDER BY
// TotalPosts DESC;
fn q12206(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let pf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let f = by_key(db.post.iq(), name(db), (&pf).and(score).and(view_count.opt()).and((&db.post.owner_user).select(&db.user.reputation).opt()), [0i64; 8], |a, (((s, sc), w), r)| {
        [a[0] + 1, a[1] + s.cx, a[2] + s.vx, a[3] + sc, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + r.is_some() as i64, a[7] + r.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), avg(a[5], a[4]), avg(a[7], a[6])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.ViewCount) AS AvgViewCount,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// PostCount,
// AvgViewCount,
// TotalComments
// FROM
// PostStats
// ORDER BY
// PostCount DESC;
fn q12210(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.view_count).opt().and(comments_of(db).opt()), [0i64; 4], |a, (w, c)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + c.is_some() as i64]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), V::I(a[3])])))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
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
// ),
// UserVoteStats AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes v
// GROUP BY v.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalViews,
// ups.TotalScore,
// ubs.TotalBadges,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges,
// uvs.TotalVotes,
// uvs.UpVotes,
// uvs.DownVotes
// FROM UserPostStats ups
// LEFT JOIN UserBadgeStats ubs ON ups.UserId = ubs.UserId
// LEFT JOIN UserVoteStats uvs ON ups.UserId = uvs.UserId
// ORDER BY ups.TotalScore DESC
// LIMIT 100;
fn q12216(db: &'static So) -> String {
    let bc = badge_classes(db);
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).and((&vs).opt()).drive(|u, ((a, b), x)| v.push((u, a, b, x)));
    out(v, |&(_, a, _, _)| Reverse(a[5]), 100, |&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([V::I(a[4]), V::I(a[5])]);
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// pt.Name AS PostType,
// COALESCE(v.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(v.DownVoteCount, 0) AS DownVoteCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVoteCount
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
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12220(db: &'static So) -> String {
    let pv = post_votes(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&pv).opt()).and(comments_per_post(db)).and((&db.post.owner_user).select(&bu))).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, x), c), b)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "type"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(c), V::I(b)]);
        f
    })
}

// WITH UserReputation AS (
// SELECT
// Id AS UserId,
// Reputation,
// CreationDate,
// LastAccessDate,
// UpVotes,
// DownVotes,
// Views,
// (UpVotes - DownVotes) AS NetVotes
// FROM
// Users
// ),
// PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// p.CreationDate,
// p.ViewCount,
// COALESCE(a.AcceptedAnswerId, 0) AS AcceptedAnswerId,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT Id, AcceptedAnswerId FROM Posts) a ON p.Id = a.Id
// ),
// TagSummary AS (
// SELECT
// Id AS TagId,
// TagName,
// Count AS PostCount
// FROM
// Tags
// )
// SELECT
// u.UserId,
// u.Reputation,
// ps.PostId,
// ps.Title,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// ts.TagId,
// ts.TagName,
// ts.PostCount,
// ps.CreationDate
// FROM
// UserReputation u
// JOIN
// PostSummary ps ON u.UserId = ps.AcceptedAnswerId
// JOIN
// PostLinks pl ON ps.PostId = pl.PostId
// JOIN
// TagSummary ts ON pl.RelatedPostId = ts.TagId
// ORDER BY
// u.Reputation DESC, ps.ViewCount DESC;
fn q12225(db: &'static So) -> String {
    let uid = uids(db);
    let tid: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let acc = (&db.post.accepted_answer_id).opt().map(|a: Option<i64>| a.unwrap_or(0));
    let mut v = Vec::new();
    db.post
        .select(Ident::<Post>::new().and(acc.select(&uid)).and(comments_per_post(db)).and(votes_per_post(db)).and(links_of(db).select((&db.post_link.related_post_id).select(&tid))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((p, u), c), x), t)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(post_fields(db, p, &["id", "title", "views"]));
        f.extend([V::I(c), V::I(x), V::I(db.tag.origid.get(t).unwrap()), V::S(db.tag.tag_name.get(t).unwrap()), V::I(db.tag.count.get(t).unwrap())]);
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(NULLIF(p.AnswerCount, 0), 0) AS AnswerCount,
// COALESCE(NULLIF(p.CommentCount, 0), 0) AS CommentCount,
// COALESCE(NULLIF(p.FavoriteCount, 0), 0) AS FavoriteCount,
// u.Reputation AS UserReputation,
// u.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId IN (2, 3)) AS VoteCount,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS TotalComments,
// ph.CreationDate AS LastModifiedDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.CreationDate DESC;
fn q12249(db: &'static So) -> String {
    let v23 = db.vote.with((&db.vote.vote_type_id).in_v(vec![2, 3])).select(&db.vote.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(&v23).and(comments_per_post(db)).and(history_of(db).select(&db.post_history.creation_date).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, x), c), h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([
            V::I(db.post.answer_count.get(p).unwrap_or(0)),
            V::I(db.post.comment_count.get(p).unwrap()),
            V::I(db.post.favorite_count.get(p).unwrap_or(0)),
        ]);
        f.extend(post_fields(db, p, &["rep", "owner"]));
        f.extend([V::I(x), V::I(c), ots(h)]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COUNT(a.Id) AS TotalAnswers,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(p.ViewCount) AS TotalViews,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS TotalGoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS TotalSilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBronzeBadges
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalAnswers,
// TotalComments,
// TotalViews,
// TotalUpVotes,
// TotalDownVotes,
// TotalGoldBadges,
// TotalSilverBadges,
// TotalBronzeBadges
// FROM UserPostStats
// ORDER BY TotalPosts DESC
// LIMIT 10;
fn q12250(db: &'static So) -> String {
    let Post { view_count, accepted_answer, .. } = &db.post;
    let uf = g(db)
        .select(
            posts_of(db)
                .select(view_count.opt().and(accepted_answer.opt()).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
                .opt()
                .and(badges_of(db).select(&db.badge.class).opt()),
        )
        .fold([0i64; 9], |a, (p, b)| {
            let mut a = a;
            if let Some((((w, ac), _), t)) = p {
                a[0] += 1;
                a[1] += ac.is_some() as i64;
                a[2] += w.is_some() as i64;
                a[3] += w.unwrap_or(0);
                a[4] += (t == Some(2)) as i64;
                a[5] += (t == Some(3)) as i64;
            }
            if let Some(c) = b {
                if (1..4).contains(&c) {
                    a[5 + c as usize] += 1;
                }
            }
            a
        });
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&uf).and((&dc).opt()).drive(|u, (a, c)| v.push((u, a, c.unwrap_or(0))));
    out(v, |&(_, a, _)| Reverse(a[0]), 10, |&(u, a, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(c), nullable(a[3], a[2])];
        f.extend(ints(&a[4..]));
        f
    })
}

// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
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
// PostCount DESC;
fn q12257(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_per_post(db)), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT p.AcceptedAnswerId) AS AcceptedAnswers,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// MAX(u.Reputation) AS Reputation,
// SUM(COALESCE(v.UpVotes, 0)) AS TotalUpVotes,
// SUM(COALESCE(v.DownVotes, 0)) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId) v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q12260(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let pv = post_votes(db);
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and((&pv).opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, s), x)) => {
            let x = x.unwrap_or([0; 3]);
            [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + x[1], a[5] + x[2]]
        }
        None => a,
    });
    let da = ud(db, UserWhere::All, posts_of(db).select(&db.post.accepted_answer_id));
    let mut v = Vec::new();
    (&uf).and((&da).opt()).drive(|u, (a, d)| v.push((u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, d)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), avg(a[1], a[0]), V::I(d), V::I(a[2]), V::I(a[3]), user_col(db, u, "rep"), V::I(a[4]), V::I(a[5])])
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerDisplayName,
// COALESCE(badgeCount.TotalBadges, 0) AS TotalBadges,
// COALESCE(voteCount.TotalVotes, 0) AS TotalVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT UserId, COUNT(*) AS TotalBadges
// FROM Badges
// GROUP BY UserId) badgeCount ON badgeCount.UserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS TotalVotes
// FROM Votes
// GROUP BY PostId) voteCount ON voteCount.PostId = p.Id
// WHERE
// p.CreationDate >= '2022-01-01'
// ORDER BY
// p.Score DESC, p.CreationDate DESC
// LIMIT 100;
fn q12269(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned_since(db, date(2022, 1, 1)).select(Ident::<Post>::new().and((&db.post.owner_user).select(&bu)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| (score_desc(db, p), newest(db, p)), 100, |&((p, b), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::I(b), V::I(x)]);
        f
    })
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// pt.Name AS PostTypeName,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Id IS NOT NULL THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Id IS NOT NULL THEN 0 ELSE 1 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// JOIN PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// p.Id, p.Title, p.PostTypeId, pt.Name
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.PostTypeName,
// ps.TotalComments,
// ps.TotalVotes,
// ps.UpVotes,
// ps.DownVotes
// FROM
// PostSummary ps
// ORDER BY
// ps.TotalVotes DESC, ps.TotalComments DESC;
fn q12270(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    rows(stats_with(db, db.post.iq(), "cv", &[], &[&c, &x]).iter().map(|&(p, s, d)| {
        let mut f = post_fields(db, p, &["id", "title", "type"]);
        f.extend([V::I(d[0]), V::I(d[1]), V::I(s.vtj_n), V::I(s.rows - s.vtj_n)]);
        row(f)
    }))
}

// WITH PostVoteStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId IN (1, 2)
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersProvided
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
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.TotalViews,
// us.QuestionsAsked,
// us.AnswersProvided,
// pvs.PostId,
// pvs.Title,
// pvs.CreationDate,
// pvs.VoteCount,
// pvs.UpVotes,
// pvs.DownVotes
// FROM
// UserStats us
// JOIN
// Posts p ON us.UserId = p.OwnerUserId
// JOIN
// PostVoteStats pvs ON p.Id = pvs.PostId
// ORDER BY
// us.TotalViews DESC, pvs.VoteCount DESC;
fn q12278(db: &'static So) -> String {
    let Post { view_count, post_type_id, .. } = &db.post;
    let us = g(db).select(badges_of(db).opt().and(posts_of(db).select(view_count.opt().and(post_type_id)).opt())).fold([0i64; 5], |a, (b, p)| {
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
    stats_fold(db, owned(db).with(post_type_id.in_v(vec![1, 2])), Ident::<Post>::new(), "v", &[]).and((&db.post.owner_user).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4])];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(s.vx), V::I(s.up), V::I(s.down)]);
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
// U.TotalViews,
// U.AverageScore,
// COALESCE(B.TotalBadges, 0) AS TotalBadges,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats U
// LEFT JOIN
// UserBadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.TotalPosts DESC
// LIMIT 100;
fn q12280(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), avg(a[5], a[0])]);
        f.extend(ints(&b));
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// (SELECT COUNT(1) FROM Votes v2 WHERE v2.PostId = p.Id AND v2.VoteTypeId IN (6, 7)) AS VoteCount,
// COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadgeCount,
// COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadgeCount,
// COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadgeCount
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
// p.PostTypeId IN (1, 2)
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12292(db: &'static So) -> String {
    let v67 = db.vote.with((&db.vote.vote_type_id).in_v(vec![6, 7])).select(&db.vote.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.post_type_id).in_v(vec![1, 2])), Ident::<Post>::new(), "cvb", &[]).and(&v67).drive(|p, (s, x)| v.push((p, s, x)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, s, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(x), V::I(s.bcls[1]), V::I(s.bcls[2]), V::I(s.bcls[3])]);
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
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(u.UpVotes) AS TotalUps,
// SUM(u.DownVotes) AS TotalDowns
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
// pd.CreationDate,
// pd.ViewCount,
// pd.Score,
// pd.CommentCount,
// pd.VoteCount,
// ua.UserId,
// ua.PostCount,
// ua.TotalViews,
// ua.TotalUps,
// ua.TotalDowns
// FROM
// PostDetails pd
// JOIN
// UserActivity ua ON pd.PostId = ua.UserId
// ORDER BY
// pd.Score DESC, pd.ViewCount DESC
// LIMIT 100;
fn q12298(db: &'static So) -> String {
    let uid = uids(db);
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let User { up_votes, down_votes, .. } = &db.user;
    let ua = g(db).select(up_votes.and(down_votes).and(posts_of(db).select((&db.post.view_count).opt()).opt())).fold([0i64; 4], |a, ((u, d), w)| {
        let w = w.flatten();
        [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + u, a[3] + d]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&ua).and((&dp).opt()))))
        .drive(|_, y| v.push(y));
    out(v, |&(((p, _), _), _)| score_views(db, p), 100, |&(((p, c), x), ((u, a), d))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0)), user_col(db, u, "uid"), V::I(d.unwrap_or(0)), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
        f
    })
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(p.Score), 0) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON p.OwnerUserId = u.Id
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.Score,
// pd.ViewCount,
// pd.OwnerDisplayName,
// pd.CommentCount,
// pd.VoteCount,
// ua.BadgeCount,
// ua.TotalViews,
// ua.TotalScore
// FROM
// PostDetails pd
// JOIN
// UserActivity ua ON pd.OwnerDisplayName = ua.DisplayName
// ORDER BY
// pd.Score DESC, pd.CreationDate DESC;
fn q12313(db: &'static So) -> String {
    let names = by_name(db);
    let ua = g(db).select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)).opt())).fold([0i64; 3], |a, (b, p)| {
        let (w, s) = p.map_or((None, 0), |(w, s)| (w, s));
        [a[0] + b.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + s]
    });
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(&ua))
        .drive(|p, (s, a)| v.push((p, s, a)));
    rows(v.iter().map(|&(p, s, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(s.cx), V::I(s.vx)]);
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// COUNT(c.Id) AS CommentCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.BadgeCount,
// us.UpVoteCount,
// us.DownVoteCount,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount
// FROM UserStats us
// JOIN PostStats ps ON us.UserId = ps.PostId
// ORDER BY us.Reputation DESC, us.PostCount DESC, ps.Score DESC
// LIMIT 100;
fn q12323(db: &'static So) -> String {
    let uid = uids(db);
    let pid = pids(db);
    let us = user_counts_of(db, db.user.with((&db.user.origid).select(&pid)), "bv");
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and(&bu))))
        .drive(|_, x| v.push(x));
    out(v, |&((p, _), (((u, _), d), _))| (rep_desc(db, u), Reverse(d.unwrap_or(0)), score_desc(db, p)), 100, |&((p, c), (((u, a), d), b))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d.unwrap_or(0)), V::I(b), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]));
        f.push(V::I(c));
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
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount
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
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) b ON p.OwnerUserId = b.UserId
// WHERE
// p.PostTypeId = 1
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// Score,
// ViewCount,
// AnswerCount,
// CommentCount,
// VoteCount,
// BadgeCount,
// (Score / NULLIF(ViewCount, 0)) AS ScorePerView,
// (AnswerCount / NULLIF(ViewCount, 0)) AS AnswersPerView,
// (CommentCount / NULLIF(ViewCount, 0)) AS CommentsPerView,
// (BadgeCount / NULLIF(ViewCount, 0)) AS BadgesPerView
// FROM
// PostStats
// ORDER BY
// ViewCount DESC
// LIMIT 10;
fn q12329(db: &'static So) -> String {
    let bf = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    questions_only(db)
        .select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and(votes_per_post(db)).and((&db.post.owner_user_id).select(&bf).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| views_desc(db, p), 10, |&((((p, a), c), x), b)| {
        let b = b.unwrap_or(0);
        let w = db.post.view_count.get(p);
        let per = |n: i64| match w {
            Some(w) if w != 0 => V::F(n as f64 / w as f64),
            _ => V::Null,
        };
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a), V::I(c), V::I(x), V::I(b), per(db.post.score.get(p).unwrap()), per(a), per(c), per(b)]);
        f
    })
}

// WITH UserVoteSummary AS (
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
// PostSummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// COUNT(CM.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Comments CM ON P.Id = CM.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.TotalVotes,
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.CommentCount,
// PS.UpVotes AS PostUpVotes,
// PS.DownVotes AS PostDownVotes
// FROM
// UserVoteSummary U
// JOIN
// PostSummary PS ON U.UserId = PS.PostId
// ORDER BY
// U.TotalVotes DESC, PS.CommentCount DESC;
fn q12330(db: &'static So) -> String {
    let uid = uids(db);
    let uv = user_votes(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uv)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0])];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
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
// UserReputations AS (
// SELECT
// Reputation,
// COUNT(*) AS UserCount
// FROM
// Users
// GROUP BY
// Reputation
// ),
// AveragePostViewCount AS (
// SELECT
// AVG(ViewCount) AS AvgViewCount
// FROM
// Posts
// ),
// PopularTags AS (
// SELECT
// TagName,
// SUM(Count) AS TotalCount
// FROM
// Tags
// GROUP BY
// TagName
// ORDER BY
// TotalCount DESC
// LIMIT 5
// )
// SELECT
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges,
// (SELECT AVG(PostCount) FROM PostCounts) AS AvgPostsPerType,
// (SELECT AVG(UserCount) FROM UserReputations) AS AvgUsersPerReputation,
// (SELECT AvgViewCount FROM AveragePostViewCount) AS AvgViewCount,
// (SELECT STRING_AGG(TagName, ', ' ORDER BY TotalCount DESC) FROM PopularTags) AS TopTags
// ;
// (uses rewrites/12331.sql: the STRING_AGG is ordered by TotalCount DESC)
fn q12331(db: &'static So) -> String {
    let pc = db.post.group_by(&db.post.post_type_id).fold(0i64, |a, _| a + 1);
    let rc = db.user.group_by(&db.user.reputation).fold(0i64, |a, _| a + 1);
    let mean = |f: &Fold<i64, i64>| f.fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let (pn, ps) = mean(&pc);
    let (rn, rs) = mean(&rc);
    let t = post_totals(db);
    let tags = db.tag.group_by(&db.tag.tag_name).select(&db.tag.count).fold(0i64, |a, c| a + c);
    let top = whole(&tags)
        .select(Same::new().and(&tags))
        .window(row_number, |(_, c)| c, desc)
        .filt(|(_, n)| n <= 5)
        .map(|((k, c), _)| (c, k))
        .buf_fold(|v| {
            let mut w = v.to_vec();
            w.sort_by_key(|&(c, _)| Reverse(c));
            leak_join(w.iter().map(|&(_, k)| k), ", ")
        });
    row(vec![
        V::I(count(db.user.iq())),
        V::I(count(db.post.iq())),
        V::I(count(db.comment.iq())),
        V::I(count(db.vote.iq())),
        V::I(count(db.badge.iq())),
        avg(ps, pn),
        avg(rs, rn),
        avg(t[3], t[2]),
        ostr((&top).fold_flat(None, |_, s| Some(s))),
    ])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
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
// ups.UpVotes,
// ups.DownVotes,
// COALESCE(ubs.TotalBadges, 0) AS TotalBadges,
// COALESCE(ubs.GoldBadges, 0) AS GoldBadges,
// COALESCE(ubs.SilverBadges, 0) AS SilverBadges,
// COALESCE(ubs.BronzeBadges, 0) AS BronzeBadges
// FROM UserPostStats ups
// LEFT JOIN UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY ups.TotalPosts DESC;
fn q12332(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
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
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(b.Class), 0) AS BadgeCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Badges b ON b.UserId = p.OwnerUserId
// GROUP BY p.Id, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// COUNT(PostId) AS TotalPosts,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// SUM(CommentCount) AS TotalComments,
// SUM(VoteCount) AS TotalVotes,
// AVG(BadgeCount) AS AvgBadgeCount
// FROM PostStats
// WHERE PostTypeId IN (1, 2)
// AND CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year';
fn q12337(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let base = || since(db, year_ago()).with((&db.post.post_type_id).in_v(vec![1, 2]));
    let bi = obadges(db);
    let pf = base()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and((&db.post.owner_user_id).select(&bi).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, ((c, x), b)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + b.unwrap_or(0)]);
    let a = base().select(view_count.opt().and(score).and(&pf)).fold_flat([0i64; 7], |a, ((w, s), st)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + st[0], a[5] + st[1], a[6] + st[2]]
    });
    row(vec![V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), nullable(a[4], a[0]), nullable(a[5], a[0]), avg(a[6], a[0])])
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.CreationDate,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes
// FROM
// Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
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
// GROUP BY
// u.Id, u.Reputation, u.CreationDate
// )
// SELECT
// UserId,
// Reputation,
// CreationDate,
// PostCount,
// AnswerCount,
// QuestionCount,
// TotalComments,
// TotalVotes
// FROM
// UserStats
// ORDER BY
// Reputation DESC;
fn q12339(db: &'static So) -> String {
    let mut v = Vec::new();
    uqacx(db).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), user_col(db, u, "ucreated"), V::I(a[0]), V::I(a[2]), V::I(a[1]), V::I(a[3]), V::I(a[4])])))
}

// WITH PostAnalytics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(b.Id) AS BadgeCount,
// MAX(ph.CreationDate) AS LastHistoryUpdate
// FROM Posts p
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ),
// VoteAnalytics AS (
// SELECT
// v.PostId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS Downvotes
// FROM Votes v
// INNER JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY v.PostId
// )
// SELECT
// pa.PostId,
// pa.Title,
// pa.CreationDate,
// pa.ViewCount,
// pa.Score,
// pa.OwnerDisplayName,
// pa.CommentCount,
// pa.BadgeCount,
// pa.LastHistoryUpdate,
// COALESCE(va.VoteCount, 0) AS TotalVotes,
// COALESCE(va.Upvotes, 0) AS Upvotes,
// COALESCE(va.Downvotes, 0) AS Downvotes
// FROM PostAnalytics pa
// LEFT JOIN VoteAnalytics va ON pa.PostId = va.PostId
// ORDER BY pa.CreationDate DESC
// LIMIT 100;
fn q12350(db: &'static So) -> String {
    let va = db.vote.group_by(&db.vote.post).select((&db.vote.vote_type).select(&db.vote_type.name)).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cbh", &[]).and((&va).opt()).drive(|p, (s, x)| v.push((p, s, x)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, s, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(s.cx), V::I(s.bx), stat_field(&s, "hmax").unwrap()]);
        f.extend(ints(&x.unwrap_or([0; 3])));
        f
    })
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
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
// PostAnalytics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// pt.Name AS PostTypeName,
// p.OwnerUserId
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// )
// SELECT
// tu.UserId,
// pa.PostId,
// pa.Title,
// pa.CreationDate,
// pa.Score,
// pa.ViewCount,
// pa.OwnerDisplayName,
// pa.PostTypeName
// FROM
// TopUsers tu
// LEFT JOIN
// PostAnalytics pa ON tu.UserId = pa.OwnerUserId
// ORDER BY
// tu.PostCount DESC,
// pa.Score DESC;
fn q12358(db: &'static So) -> String {
    let pc = g(db).select(posts_of(db).opt()).fold(0i64, |a, p| a + p.is_some() as i64);
    let top: MatSet<Id<User>> = whole(&pc).select(Same::new().and(&pc)).window(row_number, |(_, n)| n, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(posts_of(db).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, p)| {
        let mut f = vec![user_col(db, u, "uid")];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "type"])),
            None => f.extend(nulls(7)),
        }
        row(f)
    }))
}

// WITH PostActivities AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS TotalChanges,
// COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseVotes,
// COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenVotes,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteUndeleteVotes,
// MAX(ph.CreationDate) AS LastActivityDate
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(pa.TotalChanges, 0) AS TotalChanges,
// COALESCE(pa.CloseVotes, 0) AS CloseVotes,
// COALESCE(pa.ReopenVotes, 0) AS ReopenVotes,
// COALESCE(pa.DeleteUndeleteVotes, 0) AS DeleteUndeleteVotes,
// pa.LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// PostActivities pa ON p.Id = pa.PostId
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// Score,
// TotalChanges,
// CloseVotes,
// ReopenVotes,
// DeleteUndeleteVotes,
// LastActivityDate
// FROM
// PostStatistics
// ORDER BY
// Score DESC,
// TotalChanges DESC;
fn q12363(db: &'static So) -> String {
    let pa = db.post_history.group_by(&db.post_history.post).select((&db.post_history.post_history_type_id).and(&db.post_history.creation_date)).fold([0, 0, 0, 0, i64::MIN], |a: [i64; 5], (t, d)| {
        [a[0] + 1, a[1] + (t == 10) as i64, a[2] + (t == 11) as i64, a[3] + matches!(t, 12 | 13) as i64, a[4].max(d)]
    });
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&pa).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        let b = a.unwrap_or([0; 5]);
        f.extend(ints(&b[..4]));
        f.push(match a {
            Some(a) => V::T(a[4]),
            None => V::Null,
        });
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// P.LastEditDate,
// PH.PostHistoryTypeId,
// COUNT(PH.Id) AS HistoryCount
// FROM Posts P
// JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN PostHistory PH ON P.Id = PH.PostId
// GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.CreationDate, U.DisplayName, U.Reputation, P.LastEditDate, PH.PostHistoryTypeId
// ),
// VoteStats AS (
// SELECT
// V.PostId,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN V.VoteTypeId = 1 THEN 1 ELSE 0 END) AS AcceptedCount
// FROM Votes V
// GROUP BY V.PostId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.CreationDate,
// PS.OwnerDisplayName,
// PS.OwnerReputation,
// PS.LastEditDate,
// PS.HistoryCount,
// COALESCE(VS.UpVotes, 0) AS UpVotes,
// COALESCE(VS.DownVotes, 0) AS DownVotes,
// COALESCE(VS.AcceptedCount, 0) AS AcceptedCount
// FROM PostStats PS
// LEFT JOIN VoteStats VS ON PS.PostId = VS.PostId
// ORDER BY PS.CreationDate DESC
// LIMIT 100;
fn q12370(db: &'static So) -> String {
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 1) as i64]);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let key = || post.and(post_history_type_id);
    let cnt = db.post_history.group_by(key()).fold(0i64, |a, _| a + 1);
    let keys: MatSet<(Id<Post>, i64)> = db.post_history.select(key()).collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64)> = (&keys).map(|(p, _)| p).inv().collect();
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&by_post).select(Same::<(Id<Post>, i64)>::new().and(&cnt)).opt()).and((&pv).opt())).drive(|_, x| v.push(x));
    let ty = |h: Option<((Id<Post>, i64), i64)>| h.map(|((_, t), _)| t);
    out(v, |&((p, h), _)| (newest(db, p), ty(h).is_none(), ty(h)), 100, |&((p, h), x)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "comments", "created", "owner", "rep", "edited"]);
        f.push(V::I(h.map_or(0, |(_, n)| n)));
        f.extend(ints(&x.unwrap_or([0; 3])));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.CreationDate,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.Reputation, u.CreationDate, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// p.CreationDate,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, p.OwnerUserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// ps.PostId,
// ps.Title,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.VoteCount,
// us.CreationDate
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// ORDER BY
// us.Reputation DESC, ps.ViewCount DESC;
fn q12381(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and((&db.post.owner_user).select(&dp))).drive(|_, y| v.push(y));
    out(v, |&(((p, _), _), _)| (rep_desc(db, db.post.owner_user.get(p).unwrap()), views_desc(db, p)), 0, |&(((p, c), x), d)| {
        let u = db.post.owner_user.get(p).unwrap();
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d)];
        f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0)), user_col(db, u, "ucreated")]);
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.OwnerUserId,
// P.Score,
// P.ViewCount,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.OwnerUserId, P.Score, P.ViewCount
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// UB.BadgeCount,
// PS.PostId,
// PS.PostTypeId,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.VoteCount
// FROM
// Users U
// JOIN
// UserBadges UB ON U.Id = UB.UserId
// JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// ORDER BY
// U.Reputation DESC,
// PS.Score DESC;
fn q12389(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(&bu)).drive(|p, (s, b)| v.push((p, s, b)));
    rows(v.iter().map(|&(p, s, b)| {
        let u = db.post.owner_user.get(p).unwrap();
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        f.extend(post_fields(db, p, &["id", "type_id", "score", "views"]));
        f.extend([V::I(s.cx), V::I(s.vx)]);
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
// P.AnswerCount,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// U.Reputation AS OwnerReputation
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount, U.Reputation
// ),
// PostHistoryStats AS (
// SELECT
// PH.PostId,
// MAX(PH.CreationDate) AS LastEditDate,
// COUNT(PH.Id) AS EditCount,
// SUM(CASE WHEN PHT.Name = 'Edit Title' THEN 1 ELSE 0 END) AS TitleEdits,
// SUM(CASE WHEN PHT.Name = 'Edit Body' THEN 1 ELSE 0 END) AS BodyEdits
// FROM
// PostHistory PH
// JOIN
// PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// GROUP BY
// PH.PostId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.AnswerCount,
// PS.CommentCount,
// PS.OwnerReputation,
// PHS.LastEditDate,
// PHS.EditCount,
// PHS.TitleEdits,
// PHS.BodyEdits,
// PS.UpVotes,
// PS.DownVotes
// FROM
// PostStats PS
// LEFT JOIN
// PostHistoryStats PHS ON PS.PostId = PHS.PostId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC
// FETCH FIRST 100 ROWS ONLY;
fn q12393(db: &'static So) -> String {
    let es = edit_names(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and((&es).opt()).drive(|p, (s, e)| v.push((p, s, e)));
    out(v, |&(p, _, _)| score_views(db, p), 100, |&(p, s, e)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.push(V::I(s.cx));
        f.extend(post_fields(db, p, &["rep"]));
        match e {
            Some(e) => f.extend([V::T(e[3]), V::I(e[0]), V::I(e[2]), V::I(e[1])]),
            None => f.extend(nulls(4)),
        }
        f.extend([V::I(s.up), V::I(s.down)]);
        f
    })
}

// WITH PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalUniqueUsers
// FROM
// Posts
// ),
// UserVoteCounts AS (
// SELECT
// UserId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// UserId
// ),
// AvgVoteCount AS (
// SELECT
// AVG(VoteCount) AS AverageVotesPerUser
// FROM
// UserVoteCounts
// )
// SELECT
// pc.TotalPosts,
// pc.TotalUniqueUsers,
// av.AverageVotesPerUser
// FROM
// PostCounts pc,
// AvgVoteCount av;
fn q12396(db: &'static So) -> String {
    let pc = whole(db.post.iq()).fold(0i64, |n, _| n + 1);
    let owners = whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct();
    let vc = db.vote.group_by((&db.vote.user_id).opt()).fold(0i64, |a, _| a + 1);
    let av = whole(&vc).select(&vc).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mut out = Vec::new();
    rel(vec![()]).select((&pc).opt().and((&owners).opt()).and((&av).opt())).drive(|_, ((p, o), a)| {
        let (n, s) = a.unwrap_or((0, 0));
        out.push(row(vec![V::I(p.unwrap_or(0)), V::I(o.unwrap_or(0)), avg(s, n)]))
    });
    rows(out)
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
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS EditCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// ),
// VoteStats AS (
// SELECT
// v.PostId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(v.Id) AS TotalVotes
// FROM
// Votes v
// GROUP BY
// v.PostId
// )
// SELECT
// ps.PostId,
// ps.CreationDate,
// ps.PostTypeId,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.OwnerReputation,
// phs.EditCount,
// phs.LastEditDate,
// vs.UpVotes,
// vs.DownVotes,
// vs.TotalVotes
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// LEFT JOIN
// VoteStats vs ON ps.PostId = vs.PostId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q12411(db: &'static So) -> String {
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&hf).opt()).and((&pv).opt())).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, h), x)| {
        let mut f = post_fields(db, p, &["id", "created", "type_id", "score", "views", "answers", "comments", "favorites", "rep"]);
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1)), oint(x.map(|x| x[1])), oint(x.map(|x| x[2])), oint(x.map(|x| x[0]))]);
        f
    })
}

// WITH UserPostCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// TopPosts AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.OwnerUserId
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.Score DESC
// LIMIT 10
// )
// SELECT
// UPC.UserId,
// UPC.DisplayName,
// UPC.PostCount,
// TP.PostId,
// TP.Title,
// TP.Score,
// TP.ViewCount,
// TP.CreationDate,
// TP.OwnerDisplayName
// FROM
// UserPostCounts UPC
// LEFT JOIN
// TopPosts TP ON UPC.UserId = TP.OwnerUserId;
fn q12414(db: &'static So) -> String {
    let pc = g(db).select(posts_of(db).opt()).fold(0i64, |a, p| a + p.is_some() as i64);
    let top: MatSet<Id<Post>> = whole(owned(db).with((&db.post.post_type_id).eq(1)))
        .select(Ident::<Post>::new().and(&db.post.score))
        .window(row_number, |(_, s)| s, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let tp: HashIdx<Id<User>, Id<Post>> = (&top).select(&db.post.owner_user).inv().collect();
    let mut v = Vec::new();
    (&pc).and((&tp).opt()).drive(|u, (n, p)| v.push((u, n, p)));
    rows(v.iter().map(|&(u, n, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n)];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "score", "views", "created", "owner"])),
            None => f.extend(nulls(6)),
        }
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
// SUM(v.BountyAmount) AS TotalBounty,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.Reputation
// ),
// BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// u.TotalBounty,
// u.UpVotes,
// u.DownVotes,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// COALESCE(b.GoldBadges, 0) AS GoldBadges,
// COALESCE(b.SilverBadges, 0) AS SilverBadges,
// COALESCE(b.BronzeBadges, 0) AS BronzeBadges
// FROM UserStats u
// LEFT JOIN BadgeStats b ON u.UserId = b.UserId
// ORDER BY u.Reputation DESC;
fn q12436(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a.n), V::I(a.q), V::I(a.a), ustat_field(&a, "bounty_sum"), V::I(a.up), V::I(a.down)];
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserReputation AS (
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
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPostStats AS (
// SELECT
// ur.UserId,
// ur.Reputation,
// ur.BadgeCount,
// ps.PostCount,
// ps.TotalScore,
// ps.TotalViews
// FROM
// UserReputation ur
// LEFT JOIN
// PostStats ps ON ur.UserId = ps.OwnerUserId
// )
// SELECT
// ups.UserId,
// ups.Reputation,
// ups.BadgeCount,
// COALESCE(ups.PostCount, 0) AS PostCount,
// COALESCE(ups.TotalScore, 0) AS TotalScore,
// COALESCE(ups.TotalViews, 0) AS TotalViews
// FROM
// UserPostStats ups
// ORDER BY
// ups.Reputation DESC,
// ups.TotalScore DESC;
fn q12440(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b)];
        f.extend(ints(&p.unwrap_or([0; 3])));
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
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
fn q12441(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and((&db.post.view_count).opt()).and(comments_per_post(db)), [0i64; 5], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4])])))
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT ph.Id) AS HistoryCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// pm.PostId,
// pm.Title,
// pm.CreationDate,
// pm.Score,
// pm.ViewCount,
// pm.CommentCount,
// pm.VoteCount,
// pm.HistoryCount,
// CASE
// WHEN pm.VoteCount > 0 THEN 'Has Votes'
// ELSE 'No Votes'
// END AS VoteStatus
// FROM
// PostMetrics pm
// ORDER BY
// pm.Score DESC,
// pm.ViewCount DESC
// LIMIT 100;
fn q12455(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    let h = per_post_distinct(db, history_of(db));
    out(stats_with(db, db.post.iq(), "cvh", &[], &[&x, &h]), |&(p, _, _)| score_views(db, p), 100, |&(p, s, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(d[0]), V::I(d[1]), V::S(if d[0] > 0 { "Has Votes" } else { "No Votes" })]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers
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
// ups.TotalScore,
// ups.TotalQuestions,
// ups.TotalAnswers,
// COALESCE(ubs.TotalBadges, 0) AS TotalBadges,
// COALESCE(ubs.GoldBadges, 0) AS GoldBadges,
// COALESCE(ubs.SilverBadges, 0) AS SilverBadges,
// COALESCE(ubs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC
// FETCH FIRST 100 ROWS ONLY;
fn q12470(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| (Reverse(a[5]), Reverse(a[0])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[5]), V::I(a[1]), V::I(a[2])];
        f.extend(ints(&b));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(p.CreationDate) AS LatestActivity,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// AVG(u.Reputation) AS AvgReputation,
// COUNT(b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// )
// SELECT
// ps.PostId,
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// ps.LatestActivity,
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.AvgReputation,
// us.BadgeCount
// FROM
// PostStats ps
// JOIN
// Users u ON ps.OwnerUserId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.LatestActivity DESC;
fn q12477(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(&bu)).drive(|p, (s, b)| v.push((p, s, b)));
    rows(v.iter().map(|&(p, s, b)| {
        let u = db.post.owner_user.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([V::I(s.cx), V::I(s.vx)]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([V::I(s.up), V::I(s.down), user_col(db, u, "uid"), V::F(db.user.reputation.get(u).unwrap() as f64), V::I(b)]);
        row(f)
    }))
}

// WITH ActivePosts AS (
// SELECT
// p.Id AS PostId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.Title,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ),
// PostStatistics AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AvgScore,
// AVG(ViewCount) AS AvgViewCount,
// AVG(AnswerCount) AS AvgAnswerCount,
// AVG(CommentCount) AS AvgCommentCount,
// SUM(OwnerReputation) AS TotalOwnerReputation
// FROM
// ActivePosts
// )
// SELECT
// ps.TotalPosts,
// ps.AvgScore,
// ps.AvgViewCount,
// ps.AvgAnswerCount,
// ps.AvgCommentCount,
// ps.TotalOwnerReputation
// FROM
// PostStatistics ps
fn q12482(db: &'static So) -> String {
    let Post { view_count, score, answer_count, comment_count, .. } = &db.post;
    let a = owned_since(db, year_ago())
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and((&db.post.owner_user).select(&db.user.reputation)))
        .fold_flat([0i64; 8], |a, ((((s, w), an), cc), r)| {
            [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + cc, a[7] + r]
        });
    row(vec![V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0]), nullable(a[7], a[0])])
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
// FROM
// Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.AcceptedAnswerId,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.AcceptedAnswerId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.BadgeCount,
// u.TotalBounties,
// u.TotalViews,
// p.PostId,
// p.PostTypeId,
// p.AcceptedAnswerId,
// p.CommentCount,
// p.VoteCount
// FROM
// UserStats u
// JOIN
// PostStats p ON u.UserId = p.AcceptedAnswerId
// ORDER BY
// u.Reputation DESC, u.PostCount DESC;
fn q12491(db: &'static So) -> String {
    let uid = uids(db);
    let us = g(db)
        .select(posts_of(db).select((&db.post.view_count).opt()).opt().and(badges_of(db).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, ((w, _), b)| [a[0] + b.flatten().unwrap_or(0), a[1] + w.flatten().unwrap_or(0)]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    db.post
        .with((&db.post.accepted_answer_id).select(&uid))
        .select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and((&db.post.accepted_answer_id).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and(&bu))))
        .drive(|_, y| v.push(y));
    rows(v.iter().map(|&(((p, c), x), (((u, a), d), b))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d.unwrap_or(0)), V::I(b), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["id", "type_id", "accepted"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0))]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalAuthors,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgPostScore
// FROM Posts
// ),
// CommentStats AS (
// SELECT
// COUNT(*) AS TotalComments,
// AVG(Score) AS AvgCommentScore
// FROM Comments
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation
// FROM Users
// )
// SELECT
// PS.TotalPosts,
// PS.TotalAuthors,
// PS.AvgViewCount,
// PS.AvgPostScore,
// CS.TotalComments,
// CS.AvgCommentScore,
// US.TotalUsers,
// US.AvgReputation
// FROM PostStats PS, CommentStats CS, UserStats US;
fn q12500(db: &'static So) -> String {
    let Post { score, view_count, owner_user_id, .. } = &db.post;
    let ps = whole(db.post.iq()).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let ow = whole(db.post.iq()).select(owner_user_id).count_distinct();
    let cs = whole(db.comment.iq()).select(&db.comment.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let us = whole(db.user.iq()).select(&db.user.reputation).fold((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let mut out = Vec::new();
    rel(vec![()]).select((&ps).opt().and((&ow).opt()).and((&cs).opt()).and((&us).opt())).drive(|_, (((t, o), c), u)| {
        let (t, (cn, cs), (un, rs)) = (t.unwrap_or([0; 4]), c.unwrap_or((0, 0)), u.unwrap_or((0, 0)));
        out.push(row(vec![V::I(t[0]), V::I(o.unwrap_or(0)), avg(t[3], t[2]), avg(t[1], t[0]), V::I(cn), avg(cs, cn), V::I(un), avg(rs, un)]))
    });
    rows(out)
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.Reputation, U.CreationDate, U.LastAccessDate
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// COUNT(DISTINCT C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT PH.Id) AS EditCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.BadgeCount,
// U.PostCount,
// U.UpVotesReceived,
// U.DownVotesReceived,
// P.PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.CommentCount,
// P.UpVotes,
// P.DownVotes,
// P.EditCount
// FROM
// UserStats U
// JOIN
// PostStats P ON U.UserId = P.PostId
// ORDER BY
// U.Reputation DESC, P.ViewCount DESC
// LIMIT 100;
fn q12506(db: &'static So) -> String {
    let uid = uids(db);
    let pid = pids(db);
    let us = user_counts_of(db, db.user.with((&db.user.origid).select(&pid)), "bv");
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let c = per_post_distinct(db, comments_of(db));
    let h = per_post_distinct(db, history_of(db));
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).opt()))
        .dense_fold(db.post.id.n, [0i64; 2], |a, ((_, t), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64])
        .and((&c).opt())
        .and((&h).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and(&bu)))
        .drive(|p, (((s, c), h), (((u, a), d), b))| v.push((p, s, c.unwrap_or(0), h.unwrap_or(0), u, a, d.unwrap_or(0), b)));
    out(v, |&(p, _, _, _, u, _, _, _)| (rep_desc(db, u), views_desc(db, p)), 100, |&(p, s, c, h, u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b), V::I(d), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend([V::I(c), V::I(s[0]), V::I(s[1]), V::I(h)]);
        f
    })
}

#[derive(Clone, Copy, Default)]
struct UC {
    n: i64,
    q: i64,
    a: i64,
    t38: i64,
    cx: i64,
    up: i64,
    down: i64,
    bx: i64,
    gold: i64,
    silver: i64,
    bronze: i64,
    views_sum: i64,
}

fn user_counts(db: &'static So, w: UserWhere, joins: &str) -> DenseFold<Id<User>, UC> {
    user_counts_of(db, user_base(db, w), joins)
}

fn user_counts_of<Q: Drive<D = Id<User>, R = Id<User>>>(db: &'static So, users: Q, joins: &str) -> DenseFold<Id<User>, UC> {
    let (c, v, b) = (joins.contains('c'), joins.contains('v'), joins.contains('b'));
    let post = (&db.post.post_type_id)
        .and((&db.post.view_count).opt())
        .and(comments_of_if(db, c).opt())
        .and(votes_of_if(db, v).select(&db.vote.vote_type_id).opt());
    users
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post).opt().and(badges_of_if(db, b).select(&db.badge.class).opt()))
        .dense_fold(db.user.id.n, UC::default(), |mut a, (p, b)| {
            if let Some((((t, w), c), v)) = p {
                a.n += 1;
                a.q += (t == 1) as i64;
                a.a += (t == 2) as i64;
                a.t38 += (3..=8).contains(&t) as i64;
                a.views_sum += w.unwrap_or(0);
                a.cx += c.is_some() as i64;
                a.up += (v == Some(2)) as i64;
                a.down += (v == Some(3)) as i64;
            }
            if let Some(cls) = b {
                a.bx += 1;
                a.gold += (cls == 1) as i64;
                a.silver += (cls == 2) as i64;
                a.bronze += (cls == 3) as i64;
            }
            a
        })
}

pub static ENTRIES: &[harness::Entry] = &[
    ("12142", q12142),
    ("12143", q12143),
    ("12144", q12144),
    ("12145", q12145),
    ("12161", q12161),
    ("12163", q12163),
    ("12178", q12178),
    ("12180", q12180),
    ("12189", q12189),
    ("12206", q12206),
    ("12210", q12210),
    ("12216", q12216),
    ("12220", q12220),
    ("12225", q12225),
    ("12249", q12249),
    ("12250", q12250),
    ("12257", q12257),
    ("12260", q12260),
    ("12269", q12269),
    ("12270", q12270),
    ("12278", q12278),
    ("12280", q12280),
    ("12292", q12292),
    ("12298", q12298),
    ("12313", q12313),
    ("12323", q12323),
    ("12329", q12329),
    ("12330", q12330),
    ("12331", q12331),
    ("12332", q12332),
    ("12337", q12337),
    ("12339", q12339),
    ("12350", q12350),
    ("12358", q12358),
    ("12363", q12363),
    ("12370", q12370),
    ("12381", q12381),
    ("12389", q12389),
    ("12393", q12393),
    ("12396", q12396),
    ("12411", q12411),
    ("12414", q12414),
    ("12436", q12436),
    ("12440", q12440),
    ("12441", q12441),
    ("12455", q12455),
    ("12470", q12470),
    ("12477", q12477),
    ("12482", q12482),
    ("12491", q12491),
    ("12500", q12500),
    ("12506", q12506),
];
