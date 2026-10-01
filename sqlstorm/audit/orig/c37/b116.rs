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

// --- batch 116 --------------------------------------------------------------

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(voteTypeVoteCount), 0) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS voteTypeVoteCount FROM Votes GROUP BY PostId) AS VoteSummary ON p.Id = VoteSummary.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalVotes DESC, PostCount DESC
// LIMIT 100;
fn q11113(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let uf = g(db).select(posts_of(db).select((&vf).opt().and(votes_of(db).opt())).opt()).fold([0i64; 2], |a, p| match p {
        Some((x, _)) => [a[0] + 1, a[1] + x.unwrap_or(0)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| (Reverse(a[1]), Reverse(a[0])), 100, |&(u, a)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1])])
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
// PostVoteStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.OwnerUserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalViews,
// ups.TotalScore,
// COALESCE(pvs.TotalVotes, 0) AS TotalVotes,
// COALESCE(pvs.UpVotes, 0) AS UpVotes,
// COALESCE(pvs.DownVotes, 0) AS DownVotes
// FROM
// UserPostStats ups
// LEFT JOIN
// PostVoteStats pvs ON ups.UserId = pvs.OwnerUserId
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC;
fn q11115(db: &'static So) -> String {
    let vf = post_votes(db);
    let mut v = Vec::new();
    upqa(db).and(posts_of(db).select((&vf).opt()).opt()).drive(|u, (a, x)| v.push((u, a, x.flatten().unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, a, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), nullable(a[5], a[0])]);
        f.extend(ints(&x));
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
// AVG(P.Score) AS AvgPostScore,
// SUM(P.ViewCount) AS TotalViews
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
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalQuestions,
// UPS.TotalAnswers,
// UPS.AvgPostScore,
// UPS.TotalViews,
// COALESCE(UBS.TotalBadges, 0) AS TotalBadges,
// COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats UPS
// LEFT JOIN
// UserBadgeStats UBS ON UPS.UserId = UBS.UserId
// ORDER BY
// UPS.TotalPosts DESC, UPS.AvgPostScore DESC;
fn q11118(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([avg(a[5], a[0]), nullable(a[4], a[3])]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id) AS VoteCount,
// (SELECT COUNT(*) FROM Posts A WHERE A.ParentId = P.Id) AS AnswerCount,
// P.OwnerUserId
// FROM
// Posts P
// WHERE
// P.PostTypeId = 1
// ),
// UserStats AS (
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
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.CommentCount,
// PS.VoteCount,
// PS.AnswerCount,
// US.UserId,
// US.DisplayName,
// US.BadgeCount
// FROM
// PostStats PS
// JOIN
// Users U ON PS.OwnerUserId = U.Id
// JOIN
// UserStats US ON US.UserId = U.Id
// ORDER BY
// PS.CreationDate DESC
// LIMIT 100;
fn q11122(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.post_type_id).eq(1))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)).and(answers_per_post(db)).and((&db.post.owner_user).select(&bu)))
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| newest(db, p), 100, |&((((p, c), x), a), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(x), V::I(a)]);
        f.extend(post_fields(db, p, &["uid", "owner"]));
        f.push(V::I(b));
        f
    })
}

// WITH PostVoteCounts AS (
// SELECT
// p.Id AS PostId,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id
// ),
// UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// )
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COALESCE(pvc.VoteCount, 0) AS VoteCount,
// COALESCE(pvc.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(pvc.DownVoteCount, 0) AS DownVoteCount,
// COALESCE(upc.PostCount, 0) AS OwnerPostCount,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN
// PostVoteCounts pvc ON p.Id = pvc.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// UserPostCounts upc ON u.Id = upc.UserId
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11131(db: &'static So) -> String {
    let ppu = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.post
        .select(Ident::<Post>::new().and(votes_per_post(db)).and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and((&db.post.owner_user).select(&ppu).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| newest(db, p), 100, |&((((p, x), u), d), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(x), V::I(u), V::I(d), V::I(n.unwrap_or(0))]);
        f.extend(post_fields(db, p, &["owner"]));
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
// AVG(p.ViewCount) AS AvgViewCount,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// AvgViewCount,
// TotalComments
// FROM
// UserPostStats
// WHERE
// TotalPosts > 0
// ORDER BY
// TotalScore DESC
// LIMIT 10;
fn q11135(db: &'static So) -> String {
    out(users_with_counts(db, "c", true), |r| Reverse(r.agg.score_sum), 10, |r| user_fields(r, "c", &["uid", "name", "#rows", "#q", "#a", "score_sum", "views_avg", "#c"]))
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount,
// MAX(CASE WHEN V.Id IS NOT NULL THEN 1 ELSE 0 END) AS HasVote,
// MAX(V.CreationDate) AS LastVoteDate
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.AnswerCount,
// PS.HasVote,
// PS.LastVoteDate,
// U.DisplayName AS AuthorDisplayName,
// U.Reputation AS AuthorReputation
// FROM
// PostStatistics PS
// JOIN
// Users U ON PS.PostId = U.Id
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q11148(db: &'static So) -> String {
    let uid = uids(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 10, 1)).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cva", &[])
        .and((&db.post.origid).select(&uid))
        .drive(|p, (s, u)| v.push((p, s, u)));
    rows(v.iter().map(|&(p, s, u)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(s.cx), V::I(s.ax), V::I((s.vx > 0) as i64), stat_field(&s, "vmax").unwrap(), user_col(db, u, "name"), user_col(db, u, "rep")]);
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostBenchmarking AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId
// ) v ON p.Id = v.PostId
// )
// SELECT
// upc.DisplayName,
// upc.PostCount,
// upc.TotalScore,
// upc.QuestionCount,
// upc.AnswerCount,
// pb.PostId,
// pb.Title,
// pb.CreationDate,
// pb.Score,
// pb.ViewCount,
// pb.CommentCount,
// pb.VoteCount
// FROM
// UserPostCounts upc
// JOIN
// PostBenchmarking pb ON upc.UserId = pb.OwnerUserId
// ORDER BY
// upc.TotalScore DESC,
// upc.PostCount DESC;
fn q11150(db: &'static So) -> String {
    let uf = upqa(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)).and((&db.post.owner_user).select(&uf))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), x), a)| {
        let mut f = vec![user_col(db, db.post.owner_user.get(p).unwrap(), "name"), V::I(a[0]), nullable(a[5], a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(x)]);
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
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount,
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ),
// Benchmark AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AvgViews,
// AVG(Score) AS AvgScore,
// AVG(AnswerCount) AS AvgAnswers,
// AVG(CommentCount) AS AvgComments,
// AVG(VoteCount) AS AvgVotes,
// AVG(OwnerReputation) AS AvgOwnerReputation
// FROM
// PostStats
// )
// SELECT
// TotalPosts,
// AvgViews,
// AvgScore,
// AvgAnswers,
// AvgComments,
// AvgVotes,
// AvgOwnerReputation
// FROM
// Benchmark;
fn q11152(db: &'static So) -> String {
    let Post { view_count, score, answer_count, .. } = &db.post;
    let a = since(db, year_ago())
        .select(view_count.opt().and(score).and(answer_count.opt()).and(comments_per_post(db)).and(votes_per_post(db)).and((&db.post.owner_user).select(&db.user.reputation).opt()))
        .fold_flat([0i64; 11], |a, (((((w, s), an), c), x), r)| {
            [
                a[0] + 1,
                a[1] + w.is_some() as i64,
                a[2] + w.unwrap_or(0),
                a[3] + s,
                a[4] + an.is_some() as i64,
                a[5] + an.unwrap_or(0),
                a[6] + c,
                a[7] + x,
                a[8] + r.is_some() as i64,
                a[9] + r.unwrap_or(0),
                0,
            ]
        });
    row(vec![V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), avg(a[5], a[4]), avg(a[6], a[0]), avg(a[7], a[0]), avg(a[9], a[8])])
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(v.BountyAmount) AS TotalBounty
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.AnswerCount) AS AvgAnswers
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.Id,
// u.DisplayName,
// us.Reputation,
// us.BadgeCount,
// us.TotalBounty,
// ps.PostCount,
// ps.TotalScore,
// ps.TotalViews,
// ps.AvgAnswers
// FROM
// Users u
// LEFT JOIN
// UserStats us ON u.Id = us.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// ORDER BY
// us.Reputation DESC,
// ps.TotalScore DESC
// LIMIT 100;
fn q11153(db: &'static So) -> String {
    let Post { score, view_count, answer_count, .. } = &db.post;
    let us = g(db).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 3], |a, (b, x)| {
        let x = x.flatten();
        [a[0] + b.is_some() as i64, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0)]
    });
    let pf = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(answer_count.opt())).fold([0i64; 6], |a, ((s, w), an)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0)]
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&us).and((&pf).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (rep_desc(db, u), p.is_none(), Reverse(p.map(|p| p[1]))), 100, |&((u, a), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), nullable(a[2], a[1])];
        match p {
            Some(p) => f.extend([V::I(p[0]), V::I(p[1]), nullable(p[3], p[2]), avg(p[5], p[4])]),
            None => f.extend(nulls(4)),
        }
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.ViewCount,
// P.Score,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount,
// COUNT(PH.Id) AS EditCount
// FROM Posts P
// LEFT JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN PostHistory PH ON P.Id = PH.PostId
// GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName, P.ViewCount, P.Score
// ),
// AveragePerformance AS (
// SELECT
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(VoteCount) AS AvgVoteCount,
// AVG(EditCount) AS AvgEditCount
// FROM PostStatistics
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.OwnerDisplayName,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.VoteCount,
// PS.EditCount,
// AP.AvgViewCount,
// AP.AvgScore,
// AP.AvgCommentCount,
// AP.AvgVoteCount,
// AP.AvgEditCount
// FROM PostStatistics PS
// CROSS JOIN AveragePerformance AP
// ORDER BY PS.Score DESC, PS.ViewCount DESC;
fn q11154(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let pf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cvh", &[]);
    let t = db.post.select(view_count.opt().and(score).and(&pf)).fold_flat([0i64; 7], |a, ((w, s), st)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + st.cx, a[5] + st.vx, a[6] + st.hx]
    });
    let mut v = Vec::new();
    (&pf).drive(|p, s| v.push((p, s)));
    rows(v.iter().map(|&(p, s)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "views", "score"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.hx), avg(t[2], t[1]), avg(t[3], t[0]), avg(t[4], t[0]), avg(t[5], t[0]), avg(t[6], t[0])]);
        row(f)
    }))
}

// WITH UserPostCount AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadgeCount AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// upc.UserId,
// upc.DisplayName,
// upc.PostCount,
// upc.QuestionCount,
// upc.AnswerCount,
// COALESCE(ubc.BadgeCount, 0) AS BadgeCount,
// u.Reputation,
// u.CreationDate
// FROM
// UserPostCount upc
// JOIN
// Users u ON upc.UserId = u.Id
// LEFT JOIN
// UserBadgeCount ubc ON upc.UserId = ubc.UserId
// ORDER BY
// upc.PostCount DESC,
// u.Reputation DESC
// FETCH FIRST 100 ROWS ONLY;
fn q11165(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    upqa(db).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(u, a, _)| (Reverse(a[0]), rep_desc(db, u)), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([V::I(b), user_col(db, u, "rep"), user_col(db, u, "ucreated")]);
        f
    })
}

// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// COALESCE(SUM(c.CommentCount), 0) AS TotalComments,
// COALESCE(SUM(v.VoteCount), 0) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.Reputation, u.DisplayName
// ORDER BY
// u.Reputation DESC;
fn q11181(db: &'static So) -> String {
    let mut v = Vec::new();
    uqacx(db).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), user_col(db, u, "name")];
        f.extend(ints(&a));
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(AVG(vote_count), 0) AS AvgVotes,
// COALESCE(AVG(comment_count), 0) AS AvgComments,
// COALESCE(AVG(answer_count), 0) AS AvgAnswers
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON p.PostTypeId = pt.Id
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS vote_count
// FROM Votes
// GROUP BY PostId
// ) v ON v.PostId = p.Id
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS comment_count
// FROM Comments
// GROUP BY PostId
// ) c ON c.PostId = p.Id
// LEFT JOIN (
// SELECT ParentId AS PostId, COUNT(*) AS answer_count
// FROM Posts
// WHERE PostTypeId = 2
// GROUP BY ParentId
// ) a ON a.PostId = p.Id
// GROUP BY
// pt.Name
// ORDER BY
// pt.Name;
fn q11187(db: &'static So) -> String {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let af = db.post.with((&db.post.post_type_id).eq(2)).group_by(&db.post.parent).fold(0i64, |a, _| a + 1);
    let f = db
        .post_type
        .group_by(&db.post_type.name)
        .select((&of_type).select((&vf).opt().and((&cf).opt()).and((&af).opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((x, c), an)) => [
                a[0] + 1,
                a[1] + x.is_some() as i64,
                a[2] + x.unwrap_or(0),
                a[3] + c.is_some() as i64,
                a[4] + c.unwrap_or(0),
                a[5] + an.is_some() as i64,
                a[6] + an.unwrap_or(0),
            ],
            None => a,
        });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), or0(a[2], a[1]), or0(a[4], a[3]), or0(a[6], a[5])])))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// u.Reputation,
// u.DisplayName
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
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId IN (1, 2)
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.Reputation,
// ps.DisplayName
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC
// LIMIT 100;
fn q11190(db: &'static So) -> String {
    let mut v = Vec::new();
    db.post.with((&db.post.post_type_id).in_v(vec![1, 2])).select(Ident::<Post>::new().and(comments_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(p, _)| score_views(db, p), 100, |&(p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["rep", "owner"]));
        f
    })
}

// WITH Benchmark AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COALESCE(b.UserId, -1) AS BadgeOwnerId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// Badges b ON b.UserId = p.OwnerUserId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, b.UserId
// )
// SELECT
// AVG(Score) AS AvgScore,
// AVG(ViewCount) AS AvgViewCount,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(UpVotes) AS AvgUpVotes,
// AVG(DownVotes) AS AvgDownVotes,
// COUNT(DISTINCT PostId) AS PostCount,
// COUNT(DISTINCT BadgeOwnerId) AS UniqueBadgeOwners
// FROM
// Benchmark;
fn q11205(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let base = || since(db, year_ago());
    let pf = stats_fold(db, base(), Ident::<Post>::new(), "cvb", &[]);
    let a = base().select(score.and(view_count.opt()).and(&pf)).fold_flat([0i64; 7], |a, ((s, w), st)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + st.cx, a[5] + st.up, a[6] + st.down]
    });
    let posts = one(whole(base()).select(Ident::<Post>::new()).count_distinct());
    let owners = one(whole(base()).select((&db.post.owner_user).select(badges_of(db)).select(&db.badge.user_id).opt().map(|x: Option<i64>| x.unwrap_or(-1))).count_distinct());
    row(vec![avg(a[1], a[0]), avg(a[3], a[2]), avg(a[4], a[0]), avg(a[5], a[0]), avg(a[6], a[0]), V::I(posts), V::I(owners)])
}

// WITH UserVoteStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(c.Id) AS TotalComments,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id
// ),
// AggregatedData AS (
// SELECT
// u.Reputation,
// u.CreationDate,
// MAX(ps.TotalComments) AS MaxComments,
// SUM(ps.QuestionCount) AS TotalQuestions,
// SUM(ps.AnswerCount) AS TotalAnswers,
// SUM(uvs.TotalVotes) AS TotalUserVotes,
// SUM(uvs.UpVotes) AS TotalUserUpVotes,
// SUM(uvs.DownVotes) AS TotalUserDownVotes
// FROM
// Users u
// LEFT JOIN
// UserVoteStats uvs ON u.Id = uvs.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.PostId
// GROUP BY
// u.Reputation, u.CreationDate
// )
// SELECT
// Reputation,
// CreationDate,
// MaxComments,
// TotalQuestions,
// TotalAnswers,
// TotalUserVotes,
// TotalUserUpVotes,
// TotalUserDownVotes
// FROM
// AggregatedData
// ORDER BY
// Reputation DESC
// LIMIT 100;
fn q11207(db: &'static So) -> String {
    let pid = pids(db);
    let uv = user_votes(db);
    let ps = db.post.group_by(Ident::<Post>::new()).select((&db.post.post_type_id).and(comments_of(db).opt())).fold([0i64; 3], |a, (t, c)| {
        [a[0] + c.is_some() as i64, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]
    });
    let f = db
        .user
        .group_by((&db.user.reputation).and(&db.user.creation_date))
        .select((&uv).and((&db.user.origid).select(&pid).select(&ps).opt()))
        .fold([i64::MIN, 0, 0, 0, 0, 0, 0], |a, (x, p)| {
            let mut a = a;
            if let Some(p) = p {
                a[0] = a[0].max(p[0]);
                a[1] += p[1];
                a[2] += p[2];
                a[3] += 1;
            }
            a[4] += x[0];
            a[5] += x[1];
            a[6] += x[2];
            a
        });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |&((r, _), _)| Reverse(r), 100, |&((r, c), a)| {
        vec![V::I(r), V::T(c), omax(a[0], a[3]), nullable(a[1], a[3]), nullable(a[2], a[3]), V::I(a[4]), V::I(a[5]), V::I(a[6])]
    })
}

// WITH PostStatistics AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalCommentCount,
// SUM(v.VoteCount) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT PostId, VoteTypeId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId, VoteTypeId
// ) v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// PostCount,
// AverageScore,
// AverageViewCount,
// TotalCommentCount,
// TotalVotes
// FROM
// PostStatistics
// ORDER BY
// PostCount DESC;
fn q11215(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let key = || (&db.vote.post).and(&db.vote.vote_type_id);
    let vt = db.vote.group_by(key()).fold(0i64, |a, _| a + 1);
    let keys: MatSet<(Id<Post>, i64)> = db.vote.select(key()).collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64)> = (&keys).map(|(p, _)| p).inv().collect();
    let f = by_key(db.post.iq(), name(db), score.and(view_count.opt()).and(comments_per_post(db)).and((&by_post).select(&vt).opt()), [0i64; 7], |a, (((s, w), c), x)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c, a[5] + x.is_some() as i64, a[6] + x.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), nullable(a[6], a[5])])))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// AnswerCount,
// BadgeCount
// FROM
// UserStats
// ORDER BY
// Reputation DESC
// LIMIT 100;
fn q11218(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let da = ud(db, UserWhere::All, posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(2))));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&dp).opt()).and((&da).opt()).and(&bu)).drive(|_, (((u, p), a), b)| v.push((u, p.unwrap_or(0), a.unwrap_or(0), b)));
    out(v, |&(u, _, _, _)| rep_desc(db, u), 100, |&(u, p, a, b)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(p), V::I(a), V::I(b)])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// BadgeCount AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// PostHistoryCount AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS TotalPostHistories
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
// ups.TotalViews,
// ups.TotalScore,
// COALESCE(bc.TotalBadges, 0) AS TotalBadges,
// COALESCE(phc.TotalPostHistories, 0) AS TotalPostHistories
// FROM
// UserPostStats ups
// LEFT JOIN
// BadgeCount bc ON ups.UserId = bc.UserId
// LEFT JOIN
// PostHistoryCount phc ON ups.UserId = phc.UserId
// ORDER BY
// ups.TotalPosts DESC,
// ups.TotalScore DESC
// LIMIT 100;
fn q11220(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let hu = (&db.post_history.user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    upqa(db).and(&bu).and(&hu).drive(|u, ((a, b), h)| v.push((u, a, b, h)));
    out(v, |&(_, a, _, _)| (Reverse(a[0]), a[0] == 0, Reverse(a[5])), 100, |&(u, a, b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), nullable(a[5], a[0]), V::I(b), V::I(h)]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(p.FavoriteCount), 0) AS TotalFavorites
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.Reputation,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalScore,
// ups.TotalViews,
// ups.TotalFavorites,
// (SELECT COUNT(*) FROM Badges b WHERE b.UserId = ups.UserId) AS BadgeCount
// FROM
// UserPostStats ups
// ORDER BY
// ups.Reputation DESC,
// ups.TotalScore DESC
// FETCH FIRST 100 ROWS ONLY;
fn q11221(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, favorite_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(favorite_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some((((t, s), w), fc)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5] + fc.unwrap_or(0)],
        None => a,
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(u, a, _)| (rep_desc(db, u), a[0] == 0, Reverse(a[3])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[3], a[0]), V::I(a[4]), V::I(a[5]), V::I(b)]);
        f
    })
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COALESCE(COUNT(c.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount
// ),
// UserMetrics AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.ViewCount) AS TotalPostViews,
// SUM(pm.ViewCount) AS PostMetricsViews
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostMetrics pm ON p.Id = pm.PostId
// GROUP BY
// u.Id, u.Reputation, u.DisplayName
// )
// SELECT
// um.UserId,
// um.DisplayName,
// um.Reputation,
// um.BadgeCount,
// um.TotalPostViews,
// pm.PostId,
// pm.Title,
// pm.CreationDate,
// pm.ViewCount,
// pm.CommentCount,
// pm.UpVotes,
// pm.DownVotes
// FROM
// UserMetrics um
// JOIN
// PostMetrics pm ON um.UserId = pm.PostId
// ORDER BY
// um.Reputation DESC, pm.ViewCount DESC
// LIMIT 100;
fn q11224(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, u, _)| (rep_desc(db, u), views_desc(db, p)), 100, |&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a.bx), ustat_field(&a, "views_sum")];
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f
    })
}

// WITH PostVoteStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.PostTypeId,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.PostTypeId
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
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
// U.Id, U.DisplayName
// )
// SELECT
// PVS.PostId,
// PVS.Title,
// PVS.PostTypeId,
// PVS.VoteCount,
// PVS.UpVoteCount,
// PVS.DownVoteCount,
// US.UserId,
// US.DisplayName,
// US.BadgeCount,
// US.TotalViews,
// US.TotalScore
// FROM
// PostVoteStats PVS
// JOIN
// Users U ON PVS.PostId = U.AccountId
// LEFT JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PVS.VoteCount DESC, US.TotalScore DESC;
fn q11225(db: &'static So) -> String {
    let acc: HashIdx<i64, Id<User>> = (&db.user.account_id).inv().collect();
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&acc)), Ident::<Post>::new(), "v", &[])
        .and((&db.post.origid).select(&acc).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id"]);
        f.extend([V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a.bx), ustat_field(&a, "views_sum"), ustat_field(&a, "score_sum")]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
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
// p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(v.BountyAmount) AS TotalBounty
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.ViewCount,
// ps.Score,
// ps.CreationDate,
// ps.CommentCount,
// ps.VoteCount,
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.BadgeCount,
// us.TotalBounty
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.ViewCount DESC,
// ps.Score DESC
// LIMIT 100;
fn q11226(db: &'static So) -> String {
    let uid = uids(db);
    let x = per_post_distinct(db, votes_of(db));
    let us = g(db).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 3], |a, (b, x)| {
        let x = x.flatten();
        [a[0] + b.is_some() as i64, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0)]
    });
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&x).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, ((s, x), (u, a))| v.push((p, s, x.unwrap_or(0), u, a)));
    out(v, |&(p, _, _, _, _)| views_score(db, p), 100, |&(p, s, x, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "created"]);
        f.extend([V::I(s.cx), V::I(x), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), nullable(a[2], a[1])]);
        f
    })
}

// WITH PostEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
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
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalComments,
// TotalVotes,
// (TotalPosts + TotalComments + TotalVotes) AS EngagementScore
// FROM
// PostEngagement
// ORDER BY
// EngagementScore DESC
// LIMIT 10;
fn q11240(db: &'static So) -> String {
    let mut v = Vec::new();
    uqacx(db).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| Reverse(a[0] + a[3] + a[4]), 10, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(V::I(a[0] + a[3] + a[4]));
        f
    })
}

// WITH PostEngagement AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.ViewCount,
// P.Score,
// P.CommentCount,
// P.AnswerCount,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// COUNT(V.Id) AS TotalVotes,
// MAX(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// MAX(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// P.PostTypeId
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title, P.ViewCount, P.Score, P.CommentCount, P.AnswerCount, P.CreationDate,
// U.DisplayName, U.Reputation, P.PostTypeId
// ),
// PostTypes AS (
// SELECT
// PT.Id AS PostTypeId,
// PT.Name AS PostTypeName
// FROM
// PostTypes PT
// )
// SELECT
// PE.PostId,
// PE.Title,
// PT.PostTypeName,
// PE.ViewCount,
// PE.Score,
// PE.CommentCount,
// PE.AnswerCount,
// PE.CreationDate,
// PE.OwnerDisplayName,
// PE.OwnerReputation,
// PE.TotalVotes,
// PE.TotalUpvotes,
// PE.TotalDownvotes
// FROM
// PostEngagement PE
// JOIN
// PostTypes PT ON PE.PostTypeId = PT.PostTypeId
// ORDER BY
// PE.ViewCount DESC;
fn q11242(db: &'static So) -> String {
    let pf = since(db, year_ago()).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1].max((t == Some(2)) as i64), a[2].max((t == Some(3)) as i64)]
    });
    let mut v = Vec::new();
    (&pf).drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "type", "views", "score", "comments", "answers", "created", "owner", "rep"]);
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// MAX(p.CreationDate) AS LastActivityDate,
// p.OwnerUserId
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate >= '2023-01-01'
// GROUP BY p.Id, p.Title, p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE u.CreationDate >= '2023-01-01'
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.LastActivityDate,
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM PostStats ps
// JOIN UserStats us ON us.UserId = ps.OwnerUserId
// ORDER BY ps.LastActivityDate DESC, ps.VoteCount DESC;
fn q11243(db: &'static So) -> String {
    let w = UserWhere::CreatedGe(date(2023, 1, 1));
    let us = user_stats_fold(db, Ident::<User>::new(), w, "v", any_post);
    let dp = ud(db, w, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, (s, ((u, a), d))| v.push((p, s, u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.up), V::I(a.down)]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id
// ),
// BadgeCount AS (
// SELECT
// UserId,
// COUNT(Id) AS BadgeTotal
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// u.DisplayName,
// us.PostCount,
// us.CommentCount,
// us.UpVotes,
// us.DownVotes,
// us.QuestionCount,
// us.AnswerCount,
// COALESCE(bc.BadgeTotal, 0) AS TotalBadges
// FROM
// Users u
// JOIN
// UserPostStats us ON u.Id = us.UserId
// LEFT JOIN
// BadgeCount bc ON u.Id = bc.UserId
// ORDER BY
// us.PostCount DESC, us.UpVotes DESC
// LIMIT 50;
fn q11245(db: &'static So) -> String {
    out(users_with_counts(db, "cv", false), |r| (Reverse(r.agg.prows), Reverse(r.agg.up)), 50, |r| user_fields(r, "cv", &["name", "#rows", "#cx", "#up", "#down", "#q", "#a", "#b"]))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AveragePostScore,
// COUNT(CASE WHEN PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// COUNT(CASE WHEN PostTypeId IN (4, 5) THEN 1 END) AS TotalTagWikis
// FROM
// Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation
// FROM
// Users
// ),
// CommentStats AS (
// SELECT
// COUNT(*) AS TotalComments
// FROM
// Comments
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS TotalUpvotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS TotalDownvotes
// FROM
// Votes
// )
// SELECT
// PS.TotalPosts,
// PS.AveragePostScore,
// PS.TotalQuestions,
// PS.TotalAnswers,
// PS.TotalTagWikis,
// US.TotalUsers,
// US.AverageReputation,
// CS.TotalComments,
// VS.TotalVotes,
// VS.TotalUpvotes,
// VS.TotalDownvotes
// FROM
// PostStats PS,
// UserStats US,
// CommentStats CS,
// VoteStats VS;
fn q11246(db: &'static So) -> String {
    let p = db.post.select((&db.post.score).and(&db.post.post_type_id)).fold_flat([0i64; 5], |a, (s, t)| {
        [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + matches!(t, 4 | 5) as i64]
    });
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let (xn, xu, xd) = db.vote.select(&db.vote.vote_type_id).fold_flat((0i64, 0i64, 0i64), |(n, u, d), t| (n + 1, u + (t == 2) as i64, d + (t == 3) as i64));
    row(vec![V::I(p[0]), avg(p[1], p[0]), V::I(p[2]), V::I(p[3]), V::I(p[4]), V::I(un), avg(rs, un), V::I(count(db.comment.iq())), V::I(xn), V::I(xu), V::I(xd)])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(a.AcceptedAnswerCount, 0) AS AcceptedAnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount,
// u.Reputation AS UserReputation,
// u.Location AS UserLocation,
// u.CreationDate AS UserCreationDate
// FROM
// Posts p
// LEFT JOIN
// (SELECT
// ParentId,
// COUNT(*) AS AcceptedAnswerCount
// FROM Posts
// WHERE PostTypeId = 2 AND AcceptedAnswerId IS NOT NULL
// GROUP BY ParentId) a ON p.Id = a.ParentId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId) v ON p.Id = v.PostId
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11267(db: &'static So) -> String {
    let aa = db.post.with((&db.post.post_type_id).eq(2)).with(&db.post.accepted_answer_id).group_by(&db.post.parent).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and((&aa).opt()).and(comments_per_post(db)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, a), c), x)| {
        let u = db.post.owner_user.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a.unwrap_or(0)), V::I(c), V::I(x), user_col(db, u, "rep"), ostr(db.user.location.get(u)), user_col(db, u, "ucreated")]);
        f
    })
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.Reputation
// ),
// PostDetails AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// P.OwnerUserId,
// PT.Name AS PostType
// FROM Posts P
// JOIN PostTypes PT ON P.PostTypeId = PT.Id
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.TotalScore,
// U.TotalViews,
// PD.Title,
// PD.CreationDate,
// PD.ViewCount,
// PD.Score,
// PD.AnswerCount,
// PD.CommentCount,
// PD.FavoriteCount,
// PD.PostType
// FROM UserReputation U
// JOIN PostDetails PD ON U.UserId = PD.OwnerUserId
// ORDER BY U.Reputation DESC, PD.CreationDate DESC
// LIMIT 100;
fn q11268(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)],
        None => a,
    });
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&uf)))).drive(|_, x| v.push(x));
    out(v, |&(p, (u, _))| (rep_desc(db, u), newest(db, p)), 100, |&(p, (u, a))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["title", "created", "views", "score", "answers", "comments", "favorites", "type"]));
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
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AvgViewCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// P.Id AS PostId,
// COUNT(PH.Id) AS TotalHistoryRecords,
// MAX(PH.CreationDate) AS LastEditedDate
// FROM
// Posts P
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalQuestions,
// UPS.TotalAnswers,
// UPS.TotalScore,
// UPS.AvgViewCount,
// PHS.TotalHistoryRecords,
// PHS.LastEditedDate
// FROM
// UserPostStats UPS
// LEFT JOIN
// PostHistoryStats PHS ON UPS.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = PHS.PostId LIMIT 1)
// ORDER BY
// UPS.TotalScore DESC,
// UPS.TotalPosts DESC;
fn q11270(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
        None => a,
    });
    let hc = history_per_post(db);
    let hm = history_max_date(db);
    let mut v = Vec::new();
    (&uf).and(posts_of(db).select((&hc).and(&hm)).opt()).drive(|u, (a, h)| v.push((u, a, h)));
    rows(v.iter().map(|&(u, a, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[3], a[0]), avg(a[5], a[4])]);
        match h {
            Some((n, m)) => f.extend([V::I(n), if n == 0 { V::Null } else { V::T(m) }]),
            None => f.extend(nulls(2)),
        }
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// ARRAY_AGG(DISTINCT t.TagName) AS Tags,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Tags t ON t.WikiPostId = p.Id OR t.ExcerptPostId = p.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11274(db: &'static So) -> String {
    let Tag { wiki_post, excerpt_post, .. } = &db.tag;
    let tp: HashIdx<Id<Post>, Id<Tag>> = db.tag.select(wiki_post.opt().and(excerpt_post.opt())).flat_map(|(w, e)| [w, e.filter(|&e| Some(e) != w)].into_iter().flatten()).inv().collect();
    let pf = questions_only(db)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and((&tp).opt()))
        .fold([0i64; 2], |a, ((c, x), _)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let names: MatSet<(Id<Post>, Option<Str>)> = questions_only(db).select(Ident::<Post>::new().and((&tp).select(&db.tag.tag_name).opt())).collect();
    let lists = (&names).gather_by((&names).map(|(p, _)| p));
    let mut v = Vec::new();
    (&pf).and(&lists).drive(|p, (a, l)| v.push((p, a, l.iter().map(|&(_, n)| n).collect::<Vec<_>>())));
    out(v, |(p, _, _)| newest(db, *p), 100, |(p, a, l)| {
        let mut f = post_fields(db, *p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::L(l.iter().map(|&n| ostr(n)).collect())]);
        f.extend(post_fields(db, *p, &["owner", "rep"]));
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.PostTypeId,
// COUNT(P.Id) AS PostCount,
// AVG(P.Score) AS AvgScore,
// AVG(P.ViewCount) AS AvgViewCount
// FROM
// Posts P
// GROUP BY
// P.PostTypeId
// ),
// UserStats AS (
// SELECT
// P.PostTypeId,
// U.Id AS UserId,
// U.Reputation
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.OwnerUserId IS NOT NULL
// )
// SELECT
// P.Name AS PostTypeName,
// PS.PostCount,
// PS.AvgScore,
// PS.AvgViewCount,
// MAX(U.Reputation) AS HighestUserReputation
// FROM
// PostStats PS
// JOIN
// PostTypes P ON PS.PostTypeId = P.Id
// LEFT JOIN
// UserStats U ON PS.PostTypeId = U.PostTypeId
// GROUP BY
// P.Name, PS.PostCount, PS.AvgScore, PS.AvgViewCount
// ORDER BY
// PS.PostCount DESC;
fn q11280(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_key(db.post.iq(), &db.post.post_type, score.and(view_count.opt()), [0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let mr = by_key(owned(db), &db.post.post_type, (&db.post.owner_user).select(&db.user.reputation), i64::MIN, |m, r| m.max(r));
    let mut v = Vec::new();
    (&f).and((&mr).opt()).drive(|t, (a, m)| v.push((t, a, m)));
    rows(v.iter().map(|&(t, a, m)| row(vec![V::S(db.post_type.name.get(t).unwrap()), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), oint(m)])))
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// LEFT JOIN (
// SELECT ParentId, COUNT(*) AS AnswerCount
// FROM Posts
// WHERE PostTypeId = 2
// GROUP BY ParentId
// ) a ON p.Id = a.ParentId
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// WHERE p.PostTypeId = 1
// ),
// VoteDetails AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Votes
// GROUP BY PostId
// )
// SELECT
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.ViewCount,
// pd.Score,
// pd.AnswerCount,
// pd.CommentCount,
// pd.OwnerDisplayName,
// pd.OwnerReputation,
// vd.UpVotes,
// vd.DownVotes
// FROM
// PostDetails pd
// LEFT JOIN VoteDetails vd ON pd.PostId = vd.PostId
// ORDER BY
// pd.ViewCount DESC, pd.Score DESC
// LIMIT 100;
fn q11282(db: &'static So) -> String {
    let vd = post_votes(db);
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and((&vd).opt())).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| views_score(db, p), 100, |&(((p, a), c), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a), V::I(c)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend([oint(x.map(|x| x[1])), oint(x.map(|x| x[2]))]);
        f
    })
}

// WITH UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// COUNT(c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes
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
// *,
// (TotalViews + TotalScore + TotalComments + TotalVotes) AS EngagementScore
// FROM
// UserEngagement
// ORDER BY
// EngagementScore DESC;
fn q11284(db: &'static So) -> String {
    rows(users_with_counts(db, "cv", false).iter().map(|r| {
        let a = r.agg;
        let mut f = user_fields(r, "cv", &["uid", "name", "#rows", "views_sum0", "score_sum0", "#cx", "#v"]);
        f.push(V::I(a.views_sum + a.score_sum + a.cx + a.v));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// (SELECT COUNT(*) FROM Posts AS a WHERE a.AcceptedAnswerId = p.Id) AS AcceptedAnswerCount
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
// p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11285(db: &'static So) -> String {
    let acc = (&db.post.accepted_answer).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    stats_fold(db, owned(db).with((&db.post.post_type_id).eq(1)), Ident::<Post>::new(), "cv", &[]).and(&acc).drive(|p, (s, a)| v.push((p, s, a)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, s, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.push(V::I(a));
        f
    })
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
// FROM Posts p
// JOIN PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY pt.Name
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.TotalScore,
// ps.TotalViews,
// ps.TotalUpvotes,
// ps.TotalDownvotes,
// ROUND(ps.TotalScore * 1.0 / NULLIF(ps.TotalPosts, 0), 2) AS AvgScorePerPost,
// ROUND(ps.TotalViews * 1.0 / NULLIF(ps.TotalPosts, 0), 2) AS AvgViewsPerPost
// FROM PostStats ps
// ORDER BY ps.TotalPosts DESC;
fn q11315(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), score.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()), [0i64; 5], |a, ((s, w), t)| {
        [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + (t == Some(2)) as i64, a[4] + (t == Some(3)) as i64]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| {
        let mut r = vec![V::S(k)];
        r.extend(ints(&a));
        r.extend([V::F(round2(a[1] as f64 / a[0] as f64)), V::F(round2(a[2] as f64 / a[0] as f64))]);
        row(r)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName
// ),
// TopUsers AS (
// SELECT * FROM UserPostStats
// ORDER BY TotalScore DESC
// LIMIT 10
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.PostCount,
// U.TotalScore,
// U.TotalViews,
// U.QuestionCount,
// U.AnswerCount,
// B.Name AS BadgeName,
// B.Class AS BadgeClass
// FROM TopUsers U
// LEFT JOIN Badges B ON U.UserId = B.UserId
// ORDER BY U.TotalScore DESC;
fn q11322(db: &'static So) -> String {
    let Post { score, view_count, post_type_id, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(score.and(view_count.opt()).and(post_type_id)).opt()).fold([0i64; 5], |a, p| match p {
        Some(((s, w), t)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + (t == 1) as i64, a[4] + (t == 2) as i64],
        None => a,
    });
    let top: MatSet<Id<User>> = whole(&uf).select(Same::new().and(&uf)).window(row_number, |(_, a)| a[1], desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&uf).and(badges_of(db).select((&db.badge.name).and(&db.badge.class)).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, a), b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend([ostr(b.map(|b| b.0)), oint(b.map(|b| b.1))]);
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// QuestionCount,
// AnswerCount,
// WikiCount,
// Upvotes,
// Downvotes,
// CASE
// WHEN PostCount > 0 THEN
// (Upvotes - Downvotes) / PostCount
// ELSE 0
// END AS VoteRatio,
// QuestionCount::float / NULLIF(PostCount, 0) AS QuestionRatio,
// AnswerCount::float / NULLIF(PostCount, 0) AS AnswerRatio,
// WikiCount::float / NULLIF(PostCount, 0) AS WikiRatio
// FROM
// UserReputation
// ORDER BY
// Reputation DESC, PostCount DESC
// LIMIT 100;
fn q11326(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).drive(|u, (a, d)| v.push((u, a, d.unwrap_or(0))));
    out(v, |&(u, _, d)| (rep_desc(db, u), Reverse(d)), 100, |&(u, a, d)| {
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(d),
            V::I(a.q),
            V::I(a.a),
            V::I(a.t3),
            V::I(a.up),
            V::I(a.down),
            if d > 0 { V::F((a.up - a.down) as f64 / d as f64) } else { V::F(0.0) },
            ratio32(a.q, d),
            ratio32(a.a, d),
            ratio32(a.t3, d),
        ]
    })
}

// WITH PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalPostOwners,
// COUNT(DISTINCT Id) FILTER (WHERE PostTypeId = 1) AS TotalQuestions,
// COUNT(DISTINCT Id) FILTER (WHERE PostTypeId = 2) AS TotalAnswers,
// COUNT(DISTINCT Id) FILTER (WHERE PostTypeId IN (4, 5)) AS TotalTagWikis
// FROM
// Posts
// ),
// UserCounts AS (
// SELECT
// COUNT(*) AS TotalUsers,
// SUM(Reputation) AS TotalReputation,
// AVG(Reputation) AS AvgReputation
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
// )
// SELECT
// p.TotalPosts,
// p.TotalPostOwners,
// p.TotalQuestions,
// p.TotalAnswers,
// p.TotalTagWikis,
// u.TotalUsers,
// u.TotalReputation,
// u.AvgReputation,
// c.TotalComments,
// v.TotalVotes
// FROM
// PostCounts p,
// UserCounts u,
// CommentCounts c,
// VoteCounts v;
fn q11333(db: &'static So) -> String {
    let p = db.post.select(&db.post.post_type_id).fold_flat([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64]);
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    row(vec![V::I(p[0]), V::I(owners), V::I(p[1]), V::I(p[2]), V::I(p[3]), V::I(un), V::I(rs), avg(rs, un), V::I(count(db.comment.iq())), V::I(count(db.vote.iq()))])
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount
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
// PostCount
// FROM
// UserPostCounts
// ORDER BY
// PostCount DESC
// LIMIT 10
// ),
// PostScores AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.Score IS NOT NULL
// )
// SELECT
// tu.DisplayName AS TopUser,
// tu.PostCount,
// ps.PostId,
// ps.Title,
// ps.Score
// FROM
// TopUsers tu
// JOIN
// PostScores ps ON ps.OwnerDisplayName = tu.DisplayName
// ORDER BY
// tu.PostCount DESC, ps.Score DESC;
fn q11342(db: &'static So) -> String {
    let pc = g(db).select(posts_of(db).opt()).fold(0i64, |a, p| a + p.is_some() as i64);
    let top: MatSet<Id<User>> = whole(&pc).select(Same::new().and(&pc)).window(row_number, |(_, n)| n, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let names = by_name(db);
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&pc).and((&db.user.display_name).select(&names).select(posts_of(db)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, n), p)| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(post_fields(db, p, &["id", "title", "score"]));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// SUM(v.BountyAmount) AS TotalBountyAmount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// COALESCE(SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswers
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// u.Id,
// u.DisplayName,
// ua.TotalPosts,
// ua.TotalComments,
// ua.TotalBadges,
// ua.TotalBountyAmount,
// ua.TotalUpvotes,
// ua.TotalDownvotes,
// ua.AcceptedAnswers
// FROM
// Users u
// JOIN
// UserActivity ua ON u.Id = ua.UserId
// ORDER BY
// ua.TotalPosts DESC
// LIMIT 10;
fn q11346(db: &'static So) -> String {
    out(users_with_counts(db, "cvb", false), |r| Reverse(r.agg.n), 10, |r| {
        user_fields(r, "cvb", &["uid", "name", "#n", "#c", "#b", "bounty_sum", "#up", "#down", "#acc"])
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
// UserReputationScores AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COALESCE(up.PostCount, 0) AS PostCount,
// COALESCE(up.QuestionCount, 0) AS QuestionCount,
// COALESCE(up.AnswerCount, 0) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// UserPostCounts up ON u.Id = up.UserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// (u.Reputation / NULLIF(u.PostCount, 0)) AS ReputationPerPost,
// (u.Reputation / NULLIF(u.QuestionCount, 0)) AS ReputationPerQuestion,
// (u.Reputation / NULLIF(u.AnswerCount, 0)) AS ReputationPerAnswer
// FROM
// UserReputationScores u
// ORDER BY
// u.Reputation DESC
// LIMIT 10;
fn q11351(db: &'static So) -> String {
    let mut v = Vec::new();
    upqa(db).drive(|u, a| v.push((u, a)));
    out(v, |&(u, _)| rep_desc(db, u), 10, |&(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        vec![user_col(db, u, "uid"), V::I(r), V::I(a[0]), V::I(a[1]), V::I(a[2]), ratio(r, a[0]), ratio(r, a[1]), ratio(r, a[2])]
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostVoteStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.PositiveScorePosts,
// ups.LastPostDate,
// pvs.TotalVotes,
// pvs.UpVotes,
// pvs.DownVotes
// FROM
// UserPostStats ups
// LEFT JOIN
// PostVoteStats pvs ON ups.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pvs.PostId LIMIT 1)
// ORDER BY
// ups.TotalPosts DESC;
fn q11352(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(creation_date)).opt()).fold([0, 0, 0, 0, i64::MIN], |a: [i64; 5], p| match p {
        Some(((t, s), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4].max(c)],
        None => a,
    });
    let vf = post_votes(db);
    let mut v = Vec::new();
    (&uf).and(posts_of(db).select((&vf).opt()).opt()).drive(|u, (a, x)| v.push((u, a, x)));
    rows(v.iter().map(|&(u, a, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.push(if a[0] == 0 { V::Null } else { V::T(a[4]) });
        match x {
            Some(x) => f.extend(ints(&x.unwrap_or([0; 3]))),
            None => f.extend(nulls(3)),
        }
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// p.AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(pl.LinksCount, 0) AS LinksCount,
// COALESCE(ph.HistoryCount, 0) AS HistoryCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS LinksCount FROM PostLinks GROUP BY PostId) pl ON p.Id = pl.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS HistoryCount FROM PostHistory GROUP BY PostId) ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// ORDER BY
// p.Score DESC,
// p.CreationDate DESC
// LIMIT 100;
fn q11357(db: &'static So) -> String {
    let lc = (&db.post_link.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned_since(db, date(2023, 10, 1)).select(Ident::<Post>::new().and(comments_per_post(db)).and(&lc).and(history_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| (score_desc(db, p), newest(db, p)), 100, |&(((p, c), l), h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep", "answers"]);
        f.extend([V::I(c), V::I(l), V::I(h)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// ),
// VoteStats AS (
// SELECT
// v.PostId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(CASE WHEN v.VoteTypeId = 1 THEN 1 END) AS AcceptedVotes
// FROM
// Votes v
// GROUP BY
// v.PostId
// ),
// CombinedStats AS (
// SELECT
// ps.PostId,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.OwnerDisplayName,
// COALESCE(vs.UpVotes, 0) AS UpVotes,
// COALESCE(vs.DownVotes, 0) AS DownVotes,
// COALESCE(vs.AcceptedVotes, 0) AS AcceptedVotes
// FROM
// PostStats ps
// LEFT JOIN
// VoteStats vs ON ps.PostId = vs.PostId
// )
// SELECT
// *,
// (ViewCount * 1.0 / NULLIF(AnswerCount, 0)) AS ViewsPerAnswer,
// (UpVotes * 1.0 / NULLIF(CommentCount, 0)) AS UpVotesPerComment
// FROM
// CombinedStats
// ORDER BY
// Score DESC
// LIMIT 100;
fn q11358(db: &'static So) -> String {
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 1) as i64]);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&vs).opt())).drive(|_, x| v.push(x));
    out(v, |&(p, _)| score_desc(db, p), 100, |&(p, x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "created", "score", "views", "answers", "comments", "favorites"]);
        f.push(match db.post.owner_user.get(p) {
            Some(u) => V::S(db.user.display_name.get(u).unwrap()),
            None => V::S("Community"),
        });
        f.extend(ints(&x));
        let an = db.post.answer_count.get(p).unwrap_or(0);
        f.push(match db.post.view_count.get(p) {
            Some(w) if an != 0 => V::F(w as f64 / an as f64),
            _ => V::Null,
        });
        f.push(ratio(x[0], db.post.comment_count.get(p).unwrap()));
        f
    })
}

// WITH UserVoteStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY u.Id, u.DisplayName
// ),
// PostActivityStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY p.Id, p.Title
// ),
// FinalStats AS (
// SELECT
// u.UserId,
// u.DisplayName,
// p.PostId,
// p.Title,
// us.TotalVotes AS UserTotalVotes,
// ps.TotalVotes AS PostTotalVotes,
// ps.TotalComments,
// ps.UpVotes AS PostUpVotes,
// ps.DownVotes AS PostDownVotes
// FROM UserVoteStats u
// JOIN PostActivityStats p ON u.UserId = p.PostId
// JOIN UserVoteStats us ON u.UserId = us.UserId
// JOIN PostActivityStats ps ON p.PostId = ps.PostId
// )
// SELECT
// fs.DisplayName,
// fs.Title,
// fs.UserTotalVotes,
// fs.PostTotalVotes,
// fs.TotalComments,
// fs.PostUpVotes,
// fs.PostDownVotes
// FROM FinalStats fs
// ORDER BY fs.UserTotalVotes DESC, fs.PostTotalVotes DESC;
fn q11363(db: &'static So) -> String {
    let uid = uids(db);
    let uv = vote_named(db);
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&c).opt())
        .and((&x).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&uv).opt())))
        .drive(|p, (((s, c), x), (u, a))| v.push((p, s, c.unwrap_or(0), x.unwrap_or(0), u, a.map_or(0, |a| a[0]))));
    rows(v.iter().map(|&(p, s, c, x, u, n)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(n), V::I(x), V::I(c), V::I(s.upn), V::I(s.downn)]);
        row(f)
    }))
}

// WITH BenchmarkResults AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(CAST(ViewCount AS FLOAT)) AS AverageViews,
// AVG(CAST(Score AS FLOAT)) AS AverageScore,
// COUNT(DISTINCT OwnerUserId) AS UniqueUsers,
// COUNT(DISTINCT CASE WHEN PostTypeId = 1 THEN Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN PostTypeId = 2 THEN Id END) AS TotalAnswers
// FROM
// Posts
// )
// SELECT
// *,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes
// FROM
// BenchmarkResults;
fn q11366(db: &'static So) -> String {
    let t = post_totals(db);
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let p = db.post.select(&db.post.post_type_id).fold_flat([0i64; 2], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64]);
    row(vec![V::I(t[0]), avg(t[3], t[2]), avg(t[1], t[0]), V::I(owners), V::I(p[0]), V::I(p[1]), V::I(count(db.comment.iq())), V::I(count(db.vote.iq()))])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id) AS VoteCount,
// (SELECT COUNT(*) FROM Badges b WHERE b.UserId = p.OwnerUserId) AS BadgeCount,
// pt.Name AS PostType,
// COALESCE(ph.CreationDate, (SELECT MAX(ph2.CreationDate) FROM PostHistory ph2 WHERE ph2.PostId = p.Id)) AS LastEditedDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11373(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let hm = history_max_date(db);
    let mut v = Vec::new();
    owned_since(db, year_ago())
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)).and((&db.post.owner_user).select(&bu)).and(&hm).and(history_of(db).opt()))
        .drive(|_, x| v.push(x));
    let hid = |h: Option<Id<PostHistory>>| h.map(|h| db.post_history.origid.get(h).unwrap());
    out(v, |&(((((p, _), _), _), _), h)| (newest(db, p), hid(h).is_none(), hid(h)), 100, |&(((((p, c), x), b), m), h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(x), V::I(b)]);
        f.extend(post_fields(db, p, &["type"]));
        f.push(match h {
            Some(h) => V::T(db.post_history.creation_date.get(h).unwrap()),
            None if m != i64::MIN => V::T(m),
            None => V::Null,
        });
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
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// pt.Name AS PostType,
// u.DisplayName AS OwnerName,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS VoteCount,
// p.OwnerUserId
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount,
// p.CommentCount, p.FavoriteCount, pt.Name, u.DisplayName, u.Reputation, p.OwnerUserId
// ),
// BadgeSummary AS (
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
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.PostType,
// ps.OwnerName,
// ps.OwnerReputation,
// COALESCE(bs.BadgeCount, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
// ps.VoteCount
// FROM
// PostSummary ps
// LEFT JOIN
// BadgeSummary bs ON ps.OwnerUserId = bs.UserId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC;
fn q11376(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(votes_per_post(db)).and((&db.post.owner_user).select(&bc).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "type", "owner", "rep"]);
        f.extend(ints(&b.unwrap_or([0; 4])));
        f.push(V::I(x));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// SUM(CASE WHEN p.PostTypeId = 1 THEN p.AnswerCount ELSE 0 END) AS AnswerCount
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
// p.Tags,
// pt.Name AS PostType,
// p.OwnerUserId
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.Upvotes,
// us.Downvotes,
// us.AnswerCount,
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.Score,
// pd.ViewCount,
// pd.Tags,
// pd.PostType
// FROM
// UserStats us
// JOIN
// PostDetails pd ON us.UserId = pd.OwnerUserId
// ORDER BY
// us.PostCount DESC, us.Upvotes DESC;
fn q11384(db: &'static So) -> String {
    let Post { post_type_id, answer_count, .. } = &db.post;
    let us = g(db).select(posts_of(db).select(post_type_id.and(answer_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some(((t, an), x)) => {
            let c = if t == 1 { an } else { Some(0) };
            [a[0] + (x == Some(2)) as i64, a[1] + (x == Some(3)) as i64, a[2] + c.unwrap_or(0), a[3] + c.is_some() as i64]
        }
        None => a,
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, ((u, a), d))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a[0]), V::I(a[1]), nullable(a[2], a[3])];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "tags", "type"]));
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
// U.Reputation AS OwnerReputation,
// COUNT(C.Id) AS CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, U.Reputation
// ),
// VoteStats AS (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// PS.OwnerReputation,
// PS.CommentCount,
// COALESCE(VS.UpVotes, 0) AS UpVotes,
// COALESCE(VS.DownVotes, 0) AS DownVotes
// FROM
// PostStats PS
// LEFT JOIN
// VoteStats VS ON PS.PostId = VS.PostId
// ORDER BY
// PS.CreationDate DESC
// FETCH FIRST 100 ROWS ONLY;
fn q11392(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_of_type(db, 2)).and(votes_of_type(db, 3))).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, c), u), d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "rep"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
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
// u.Id, u.DisplayName, u.Reputation
// ),
// AvgPostStats AS (
// SELECT
// AVG(PostCount) AS AvgPosts,
// AVG(QuestionCount) AS AvgQuestions,
// AVG(AnswerCount) AS AvgAnswers
// FROM
// UserStats
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotes,
// us.DownVotes,
// aps.AvgPosts,
// aps.AvgQuestions,
// aps.AvgAnswers
// FROM
// UserStats us, AvgPostStats aps
// ORDER BY
// us.Reputation DESC, us.PostCount DESC;
fn q11393(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let t = (&us).and((&dp).opt()).fold_flat([0i64; 4], |t, (a, d)| [t[0] + 1, t[1] + d.unwrap_or(0), t[2] + a.q, t[3] + a.a]);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).drive(|u, (a, d)| v.push((u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, d)| {
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(d),
            V::I(a.q),
            V::I(a.a),
            V::I(a.up),
            V::I(a.down),
            avg(t[1], t[0]),
            avg(t[2], t[0]),
            avg(t[3], t[0]),
        ])
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("11113", q11113),
    ("11115", q11115),
    ("11118", q11118),
    ("11122", q11122),
    ("11131", q11131),
    ("11135", q11135),
    ("11148", q11148),
    ("11150", q11150),
    ("11152", q11152),
    ("11153", q11153),
    ("11154", q11154),
    ("11165", q11165),
    ("11181", q11181),
    ("11187", q11187),
    ("11190", q11190),
    ("11205", q11205),
    ("11207", q11207),
    ("11215", q11215),
    ("11218", q11218),
    ("11220", q11220),
    ("11221", q11221),
    ("11224", q11224),
    ("11225", q11225),
    ("11226", q11226),
    ("11240", q11240),
    ("11242", q11242),
    ("11243", q11243),
    ("11245", q11245),
    ("11246", q11246),
    ("11267", q11267),
    ("11268", q11268),
    ("11270", q11270),
    ("11274", q11274),
    ("11280", q11280),
    ("11282", q11282),
    ("11284", q11284),
    ("11285", q11285),
    ("11315", q11315),
    ("11322", q11322),
    ("11326", q11326),
    ("11333", q11333),
    ("11342", q11342),
    ("11346", q11346),
    ("11351", q11351),
    ("11352", q11352),
    ("11357", q11357),
    ("11358", q11358),
    ("11363", q11363),
    ("11366", q11366),
    ("11373", q11373),
    ("11376", q11376),
    ("11384", q11384),
    ("11392", q11392),
    ("11393", q11393),
];
