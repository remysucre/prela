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

fn owner_posts(db: &'static So) -> Fold<Id<User>, [i64; 6]> {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(view_count.opt()).and(score))
        .fold([0i64; 6], |a, ((t, w), s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s])
}

// --- batch 115 --------------------------------------------------------------

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.Reputation,
// UA.PostCount,
// UA.CommentCount,
// UA.UpVotes,
// UA.DownVotes,
// UA.BadgeCount
// FROM UserActivity UA
// ORDER BY UA.Reputation DESC, UA.PostCount DESC
// LIMIT 100;
fn q10796(db: &'static So) -> String {
    let Vote { user, post, vote_type_id, .. } = &db.vote;
    let own: HashIdx<Id<Post>, Id<Vote>> = db.vote.with(user.and(post.select(&db.post.owner_user)).filt(|(a, b)| a == b)).select(post).inv().collect();
    let uf = g(db).select(posts_of(db).select(comments_of(db).opt().and((&own).select(vote_type_id).opt())).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (p, _)| {
        let t = p.and_then(|(_, t)| t);
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dc).opt()).and(&bu).drive(|u, (((a, p), c), b)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0), b)));
    out(v, |&(u, _, p, _, _)| (rep_desc(db, u), Reverse(p)), 100, |&(u, a, p, c, b)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(p), V::I(c), V::I(a[0]), V::I(a[1]), V::I(b)]
    })
}

// WITH PostStats AS (
// SELECT
// pt.Id AS PostTypeId,
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgPostScore,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Id, pt.Name
// ),
// UserStats AS (
// SELECT
// COUNT(DISTINCT u.Id) AS TotalUsers,
// SUM(u.Reputation) AS TotalReputation
// FROM
// Users u
// )
// SELECT
// ps.PostTypeId,
// ps.PostTypeName,
// ps.TotalPosts,
// ps.AvgPostScore,
// us.TotalUsers,
// us.TotalReputation,
// ps.AvgUserReputation
// FROM
// PostStats ps, UserStats us
// ORDER BY
// ps.PostTypeId;
fn q10798(db: &'static So) -> String {
    let f = by_key(db.post.iq(), &db.post.post_type, (&db.post.score).and((&db.post.owner_user).select(&db.user.reputation).opt()), [0i64; 4], |a, (s, r)| {
        [a[0] + 1, a[1] + s, a[2] + r.is_some() as i64, a[3] + r.unwrap_or(0)]
    });
    let tu = whole(db.user.iq()).select(Ident::<User>::new()).count_distinct();
    let rs = whole(db.user.iq()).select(&db.user.reputation).fold((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let one = rel(vec![()]).select((&tu).opt().and((&rs).opt()));
    let mut v = Vec::new();
    (&f).cross(&one).drive(|(t, _), (a, (tu, rs))| v.push((t, a, tu.unwrap_or(0), rs.unwrap_or((0, 0)))));
    rows(v.iter().map(|&(t, a, tu, (rn, rs))| {
        row(vec![V::I(db.post_type.origid.get(t).unwrap()), V::S(db.post_type.name.get(t).unwrap()), V::I(a[0]), avg(a[1], a[0]), V::I(tu), nullable(rs, rn), avg(a[3], a[2])])
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(uv.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(dv.DownVoteCount, 0) AS DownVoteCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// COALESCE(u.Reputation, 0) AS UserReputation
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS UpVoteCount
// FROM Votes
// WHERE VoteTypeId = 2
// GROUP BY PostId) uv ON p.Id = uv.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS DownVoteCount
// FROM Votes
// WHERE VoteTypeId = 3
// GROUP BY PostId) dv ON p.Id = dv.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId) b ON p.OwnerUserId = b.UserId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10799(db: &'static So) -> String {
    let bu = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    since(db, year_ago())
        .select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and(comments_per_post(db)).and((&db.post.owner_user_id).select(&bu).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| newest(db, p), 100, |&((((p, u), d), c), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(u), V::I(d), V::I(c), V::I(b.unwrap_or(0)), V::I(db.post.owner_user.get(p).map_or(0, |u| db.user.reputation.get(u).unwrap()))]);
        f
    })
}

// WITH RecentUsers AS (
// SELECT Id
// FROM Users
// WHERE LastAccessDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days'
// ),
// QuestionStats AS (
// SELECT
// COUNT(*) AS TotalQuestions,
// AVG(Score) AS AverageScore
// FROM Posts
// WHERE PostTypeId = 1
// )
// SELECT
// (SELECT TotalQuestions FROM QuestionStats) AS TotalQuestions,
// (SELECT AverageScore FROM QuestionStats) AS AverageScore,
// (SELECT COUNT(*) FROM RecentUsers) AS ActiveUsers
// FROM
// (SELECT 1) AS DUAL;
fn q10803(db: &'static So) -> String {
    let (n, s) = questions_only(db).select(&db.post.score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let act = count(db.user.with((&db.user.last_access_date).ge(ts(2024, 9, 24, 12, 34, 56))));
    row(vec![V::I(n), avg(s, n), V::I(act)])
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// COUNT(DISTINCT t.Id) AS TagCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN v.VoteTypeId = 10 THEN 1 ELSE 0 END) AS CloseVotes,
// SUM(CASE WHEN v.VoteTypeId = 11 THEN 1 ELSE 0 END) AS OpenVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.Id, u.DisplayName
// )
// SELECT
// rp.PostId,
// rp.Title,
// rp.PostCreationDate,
// rp.UserId,
// rp.UserDisplayName,
// rp.TagCount,
// rp.UpVotes,
// rp.DownVotes,
// rp.CloseVotes,
// rp.OpenVotes
// FROM
// RecentPosts rp
// ORDER BY
// rp.PostCreationDate DESC
// LIMIT 100;
fn q10847(db: &'static So) -> String {
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let pf = owned_since(db, ts(2024, 9, 1, 12, 34, 56))
        .group_by(Ident::<Post>::new())
        .select((&ex).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(10)) as i64, a[3] + (t == Some(11)) as i64]);
    let dt = per_post_distinct(db, &ex);
    let mut v = Vec::new();
    (&pf).and((&dt).opt()).drive(|p, (a, t)| v.push((p, a, t.unwrap_or(0))));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, a, t)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "uid", "owner"]);
        f.push(V::I(t));
        f.extend(ints(&a));
        f
    })
}

// WITH PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS DistinctUsers,
// SUM(COALESCE(AnswerCount, 0)) AS TotalAnswers,
// SUM(COALESCE(CommentCount, 0)) AS TotalComments
// FROM
// Posts
// ),
// UserCounts AS (
// SELECT
// COUNT(*) AS TotalUsers,
// SUM(Reputation) AS TotalReputation
// FROM
// Users
// ),
// CommentCounts AS (
// SELECT
// COUNT(*) AS TotalComments
// FROM
// Comments
// )
// SELECT
// (SELECT TotalPosts FROM PostCounts) AS TotalPosts,
// (SELECT DistinctUsers FROM PostCounts) AS DistinctUsers,
// (SELECT TotalAnswers FROM PostCounts) AS TotalAnswers,
// (SELECT TotalComments FROM PostCounts) AS TotalCommentsInPosts,
// (SELECT TotalUsers FROM UserCounts) AS TotalUsers,
// (SELECT TotalReputation FROM UserCounts) AS TotalReputation,
// (SELECT TotalComments FROM CommentCounts) AS TotalCommentsInComments
fn q10848(db: &'static So) -> String {
    let a = db.post.select((&db.post.answer_count).opt().and(&db.post.comment_count)).fold_flat([0i64; 3], |a, (an, cc)| [a[0] + 1, a[1] + an.unwrap_or(0), a[2] + cc]);
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    row(vec![V::I(a[0]), V::I(owners), nullable(a[1], a[0]), nullable(a[2], a[0]), V::I(un), nullable(rs, un), V::I(count(db.comment.iq()))])
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostTypeName,
// AVG(p.Score) AS AverageScore,
// MAX(p.ViewCount) AS MaxViewCount,
// COUNT(DISTINCT u.Id) AS ActiveUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// u.Reputation > 1000
// GROUP BY
// pt.Name
// ),
// UserMetrics AS (
// SELECT
// COUNT(*) AS TotalUsers,
// COUNT(CASE WHEN Reputation > 1000 THEN 1 END) AS HighReputationUsers
// FROM
// Users
// )
// SELECT
// pm.PostTypeName,
// pm.AverageScore,
// pm.MaxViewCount,
// um.TotalUsers,
// um.HighReputationUsers
// FROM
// PostMetrics pm,
// UserMetrics um
// ORDER BY
// pm.AverageScore DESC;
fn q10852(db: &'static So) -> String {
    let base = owned(db).with((&db.post.owner_user).select(&db.user.reputation).gt(1000));
    let f = by_key(base, name(db), (&db.post.score).and((&db.post.view_count).opt()), [0i64, 0, 0, i64::MIN], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, w.map_or(a[3], |w| a[3].max(w))]
    });
    let um = whole(db.user.iq()).select(&db.user.reputation).fold((0i64, 0i64), |(n, h), r| (n + 1, h + (r > 1000) as i64));
    let mut v = Vec::new();
    let one = rel(vec![()]).select((&um).opt());
    (&f).cross(&one).drive(|(k, _), (a, u)| v.push((k, a, u.unwrap_or((0, 0)))));
    rows(v.iter().map(|&(k, a, (un, hi))| row(vec![V::S(k), avg(a[1], a[0]), omax(a[3], a[2]), V::I(un), V::I(hi)])))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// AVG(CASE WHEN p.PostTypeId = 1 THEN p.Score END) AS AvgQuestionScore,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS TotalBadges
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.AvgQuestionScore,
// ups.TotalAnswers,
// COALESCE(ubc.TotalBadges, 0) AS TotalBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeCounts ubc ON ups.UserId = ubc.UserId
// ORDER BY
// ups.AvgQuestionScore DESC,
// ups.TotalAnswers DESC;
fn q10863(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, s)) => [a[0] + (t == 1) as i64, a[1] + if t == 1 { s } else { 0 }, a[2] + (t == 2) as i64],
        None => a,
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), avg(a[1], a[0]), V::I(a[2]), V::I(b)])))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// AVG(P.Score) AS AvgPostScore,
// COUNT(P.Id) AS TotalPosts,
// COUNT(DISTINCT P.Tags) AS UniqueTagsUsed
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// PT.Name AS PostType,
// COUNT(C.Id) AS CommentCount,
// SUM(V.BountyAmount) AS TotalBounty,
// P.OwnerUserId
// FROM Posts P
// LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id, P.Title, P.CreationDate, PT.Name, P.OwnerUserId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.AvgPostScore,
// US.TotalPosts,
// US.UniqueTagsUsed,
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.PostType,
// PS.CommentCount,
// PS.TotalBounty
// FROM UserStats US
// JOIN PostStats PS ON US.UserId = PS.OwnerUserId
// ORDER BY US.TotalPosts DESC, US.AvgPostScore DESC;
fn q10867(db: &'static So) -> String {
    let us = g(db).select(posts_of(db).select(&db.post.score).opt()).fold((0i64, 0i64), |(n, s), x| match x {
        Some(x) => (n + 1, s + x),
        None => (n, s),
    });
    let dt = ud(db, UserWhere::All, posts_of(db).select(&db.post.tags_str));
    let pf = owned(db)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let mut v = Vec::new();
    (&pf).and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and((&dt).opt()))).drive(|p, (a, ((u, (n, s)), t))| v.push((p, a, u, n, s, t.unwrap_or(0))));
    rows(v.iter().map(|&(p, a, u, n, s, t)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), avg(s, n), V::I(n), V::I(t)];
        f.extend(post_fields(db, p, &["id", "title", "created", "type"]));
        f.extend([V::I(a[0]), nullable(a[2], a[1])]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName AS UserName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
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
// u.UserId,
// u.UserName,
// u.TotalPosts,
// u.Questions,
// u.Answers,
// u.UpVotes,
// u.DownVotes,
// COALESCE(ROUND((CAST(u.UpVotes AS FLOAT) / NULLIF(u.TotalPosts, 0)) * 100, 2), 0) AS UpvotePercentage
// FROM
// UserPostStats u
// ORDER BY
// u.TotalPosts DESC;
fn q10875(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let pct = if a[0] == 0 {
            0.0
        } else {
            let r = (a[3] as f32 / a[0] as f32) * 100f32;
            ((r as f64 * 100.0).round() / 100.0) as f32 as f64
        };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(V::F(pct));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// AnswerCount,
// QuestionCount,
// UpVotesCount,
// DownVotesCount,
// CommentCount
// FROM
// UserStats
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q10886(db: &'static So) -> String {
    out(users_with_counts(db, "cv", false), |r| Reverse(r.agg.n), 10, |r| user_fields(r, "cv", &["uid", "name", "rep", "#n", "#a", "#q", "#up", "#down", "#cx"]))
}

// WITH UserBadges AS (
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
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(pb.PostCount, 0) AS PostCount,
// COALESCE(pb.Questions, 0) AS Questions,
// COALESCE(pb.Answers, 0) AS Answers,
// COALESCE(pb.TotalViews, 0) AS TotalViews,
// COALESCE(pb.AverageScore, 0) AS AverageScore,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// PostStats pb ON u.Id = pb.OwnerUserId
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// ORDER BY
// TotalViews DESC
// LIMIT 100;
fn q10903(db: &'static So) -> String {
    let pf = owner_posts(db);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, p.unwrap_or([0; 6]), b.unwrap_or([0; 4]))));
    out(v, |&(_, p, _)| Reverse(p[4]), 100, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p[..3]));
        f.extend([V::I(p[4]), or0(p[5], p[0])]);
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
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
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.PostTypeId,
// COUNT(DISTINCT C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id, P.Title, P.PostTypeId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.PostCount,
// US.BadgeCount,
// PS.PostId,
// PS.Title,
// PS.PostTypeId,
// PS.CommentCount,
// PS.UpVoteCount,
// PS.DownVoteCount
// FROM UserStats US
// JOIN PostStats PS ON US.UserId = PS.PostId
// ORDER BY US.Reputation DESC, PS.UpVoteCount DESC;
fn q10910(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let c = per_post_distinct(db, comments_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&c).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&bu)))
        .drive(|p, ((s, c), ((u, d), b))| v.push((p, s, c.unwrap_or(0), u, d.unwrap_or(0), b)));
    rows(v.iter().map(|&(p, s, c, u, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d), V::I(b)];
        f.extend(post_fields(db, p, &["id", "title", "type_id"]));
        f.extend([V::I(c), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN v.voteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.voteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// upc.PostCount AS UserPostCount,
// upc.UpvoteCount,
// upc.DownvoteCount
// FROM Posts p
// LEFT JOIN UserPostCounts upc ON p.OwnerUserId = upc.UserId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.UserPostCount,
// ps.UpvoteCount,
// ps.DownvoteCount
// FROM PostStats ps
// WHERE ps.Score > 0
// ORDER BY ps.CreationDate DESC
// LIMIT 100;
fn q10912(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let mut v = Vec::new();
    db.post.with((&db.post.score).gt(0)).select(Ident::<Post>::new().and((&db.post.owner_user).select(&us).opt())).drive(|_, x| v.push(x));
    out(v, |&(p, _)| newest(db, p), 100, |&(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([oint(a.map(|a| a.n)), oint(a.map(|a| a.up)), oint(a.map(|a| a.down))]);
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(c.Id, 0)) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// TopPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// p.ViewCount,
// COALESCE(pc.comment_count, 0) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS comment_count
// FROM Comments
// GROUP BY PostId) pc ON p.Id = pc.PostId
// ORDER BY
// p.Score DESC
// LIMIT 10
// ),
// PostHistorySummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(ph.Id) AS TotalEdits,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// Posts p
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.TotalScore,
// ua.TotalComments,
// tp.PostId,
// tp.Title AS TopPostTitle,
// tp.CreationDate AS PostCreationDate,
// tp.Score AS PostScore,
// tp.OwnerDisplayName,
// tp.ViewCount,
// tp.CommentCount,
// phs.TotalEdits,
// phs.LastEditDate
// FROM
// UserActivity ua
// JOIN
// TopPosts tp ON ua.DisplayName = tp.OwnerDisplayName
// JOIN
// PostHistorySummary phs ON tp.PostId = phs.PostId
// ORDER BY
// ua.TotalScore DESC,
// tp.Score DESC;
fn q10916(db: &'static So) -> String {
    let Post { score, .. } = &db.post;
    let ua = g(db).select(posts_of(db).select(score.and(comments_of(db).select(&db.comment.origid).opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, c)) => [a[0] + 1, a[1] + s, a[2] + c.unwrap_or(0)],
        None => a,
    });
    let top: MatSet<Id<Post>> = whole(owned(db)).select(Ident::<Post>::new().and(score)).window(row_number, |(_, s)| s, desc).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let names = by_name(db);
    let hc = history_per_post(db);
    let hm = history_max_date(db);
    let mut v = Vec::new();
    (&top)
        .select(
            Ident::<Post>::new()
                .and(comments_per_post(db))
                .and(&hc)
                .and(&hm)
                .and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(Ident::<User>::new().and(&ua))),
        )
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((p, c), h), m), (u, a))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "owner", "views"]));
        f.extend([V::I(c), V::I(h), if h == 0 { V::Null } else { V::T(m) }]);
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
// SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS TotalWikis
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
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalWikis,
// ubs.TotalBadges,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY
// ups.TotalPosts DESC
// LIMIT 100;
fn q10924(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 4], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 3 | 4 | 5) as i64],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f
    })
}

// WITH UserVoteStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(v.Id) AS TotalVotes,
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
// p.Id, p.Title, p.Score, p.ViewCount
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.TotalVotes,
// u.UpVotes AS UserUpVotes,
// u.DownVotes AS UserDownVotes,
// p.PostId,
// p.Title AS PostTitle,
// p.Score AS PostScore,
// p.ViewCount AS PostViewCount,
// p.CommentCount,
// p.UpVotes AS PostUpVotes,
// p.DownVotes AS PostDownVotes
// FROM
// UserVoteStats u
// JOIN
// PostStats p ON u.UserId = p.PostId
// ORDER BY
// u.TotalVotes DESC, p.Score DESC;
fn q10934(db: &'static So) -> String {
    let uid = uids(db);
    let uv = user_votes(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uv)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// CommentCount,
// VoteCount,
// BadgeCount
// FROM
// UserActivity
// ORDER BY
// PostCount DESC, Reputation DESC
// LIMIT 100;
fn q10950(db: &'static So) -> String {
    let cols = ["uid", "name", "rep", "#rows", "#cx", "#vux", "#bx"];
    out(users_where(db, "cVb", false, UserWhere::RepGt(0)), |r| (Reverse(r.agg.prows), Reverse(r.rep)), 100, |r| user_fields(r, "cVb", &cols))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes vt ON p.Id = vt.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.PostTypeId, p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes vt ON p.Id = vt.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.PostTypeId,
// ps.CommentCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerUserId = us.UserId
// ORDER BY
// ps.CommentCount DESC, ps.UpVoteCount DESC
// LIMIT 100;
fn q10952(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, date(2022, 1, 1)), Ident::<Post>::new(), "cvb", &[])
        .and((&c).opt())
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp)))
        .drive(|p, ((s, c), ((u, a), d))| v.push((p, s, c.unwrap_or(0), u, a, d)));
    out(v, |&(_, s, c, _, _, _)| (Reverse(c), Reverse(s.up)), 100, |&(p, s, c, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id"]);
        f.extend([V::I(c), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.up), V::I(a.down)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.AnswerCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// p.Score,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// pt.Name AS PostType,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, u.DisplayName, pt.Name
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// AnswerCount,
// CommentCount,
// Score,
// OwnerDisplayName,
// PostType,
// UpVotes,
// DownVotes
// FROM
// PostStats
// ORDER BY
// Score DESC, ViewCount DESC
// LIMIT 100;
fn q10955(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    out(stats_with(db, db.post.iq(), "cv", &[], &[&c]), |&(p, _, _)| score_views(db, p), 100, |&(p, s, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers"]);
        f.push(V::I(d[0]));
        f.extend(post_fields(db, p, &["score"]));
        f.push(match db.post.owner_user.get(p) {
            Some(u) => V::S(db.user.display_name.get(u).unwrap()),
            None => V::S("Community User"),
        });
        f.extend(post_fields(db, p, &["type"]));
        f.extend([V::I(s.up), V::I(s.down)]);
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
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// COUNT(C.Id) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// P.Id, P.Title, P.Score, P.ViewCount, P.AnswerCount
// ),
// AggregateData AS (
// SELECT
// U.UserId,
// U.DisplayName,
// P.PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.CommentCount,
// U.TotalVotes,
// U.UpVotes,
// U.DownVotes
// FROM
// UserVoteStats U
// JOIN
// PostStats P ON U.UserId = P.PostId
// )
// SELECT
// A.UserId,
// A.DisplayName,
// A.PostId,
// A.Title,
// A.Score,
// A.ViewCount,
// A.CommentCount,
// A.TotalVotes,
// A.UpVotes,
// A.DownVotes
// FROM
// AggregateData A
// ORDER BY
// A.Score DESC, A.ViewCount DESC;
fn q10960(db: &'static So) -> String {
    let uid = uids(db);
    let uv = user_votes(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uv))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), (u, a))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.push(V::I(c));
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.Score,
// P.ViewCount,
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
// P.Id, P.PostTypeId, P.Score, P.ViewCount
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.BadgeCount,
// U.UpVotes AS UserUpVotes,
// U.DownVotes AS UserDownVotes,
// P.PostId,
// P.PostTypeId,
// P.Score,
// P.ViewCount,
// P.CommentCount,
// P.UpVotes AS PostUpVotes,
// P.DownVotes AS PostDownVotes
// FROM
// UserStats U
// JOIN
// PostStats P ON U.UserId = P.PostId
// ORDER BY
// U.Reputation DESC, P.ViewCount DESC
// FETCH FIRST 100 ROWS ONLY;
fn q10969(db: &'static So) -> String {
    let uid = uids(db);
    let pid = pids(db);
    let us = user_counts_of(db, db.user.with((&db.user.origid).select(&pid)), "bv");
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .dense_fold(db.post.id.n, [0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and(&bu)))
        .drive(|p, (s, (((u, a), d), b))| v.push((p, s, u, a, d.unwrap_or(0), b)));
    out(v, |&(p, _, u, _, _, _)| (rep_desc(db, u), views_desc(db, p)), 100, |&(p, s, u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d), V::I(b), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "type_id", "score", "views"]));
        f.extend(s.map(V::I));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AvgPostScore,
// AVG(ViewCount) AS AvgPostViewCount
// FROM
// Posts
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.TotalPosts,
// us.AverageScore,
// us.AverageViewCount,
// ps.TotalPosts AS OverallTotalPosts,
// ps.AvgPostScore AS OverallAvgPostScore,
// ps.AvgPostViewCount AS OverallAvgPostViewCount
// FROM
// UserStats us,
// PostStats ps
// ORDER BY
// us.TotalPosts DESC
// LIMIT 5;
fn q10971(db: &'static So) -> String {
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let Post { score, view_count, .. } = &db.post;
    let ps = whole(db.post.iq()).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    let one = rel(vec![()]).select((&ps).opt());
    (&uf).cross(&one).drive(|(u, _), (a, t)| v.push((u, a, t.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| Reverse(a.n), 5, |&(u, a, t)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a.n), ustat_field(&a, "score_avg"), ustat_field(&a, "views_avg"), V::I(t[0]), avg(t[1], t[0]), avg(t[3], t[2])]
    })
}

// WITH UserVoteSummary AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(v.Id) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostSummary AS (
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
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// BadgeSummary AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges,
// MAX(b.Class) AS HighestBadgeClass
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.UpVotes,
// u.DownVotes,
// u.TotalVotes,
// p.PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score AS PostScore,
// p.ViewCount AS PostViewCount,
// p.CommentCount AS PostCommentCount,
// b.TotalBadges AS UserBadgesCount,
// b.HighestBadgeClass AS UserHighestBadgeClass
// FROM
// UserVoteSummary u
// JOIN
// PostSummary p ON u.UserId = p.PostId
// LEFT JOIN
// BadgeSummary b ON u.UserId = b.UserId
// ORDER BY
// u.TotalVotes DESC,
// p.Score DESC
// LIMIT 100;
fn q10973(db: &'static So) -> String {
    let uid = uids(db);
    let uv = user_votes(db);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uv).and((&bs).opt()))))
        .drive(|_, x| v.push(x));
    out(v, |&((p, _), ((_, a), _))| (Reverse(a[0]), score_desc(db, p)), 100, |&((p, c), ((u, a), b))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[0])];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), oint(b.map(|b| b.0)), oint(b.map(|b| b.1))]);
        f
    })
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
// ), PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(DISTINCT C.Id) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.PostCount,
// U.CommentCount AS UserCommentCount,
// U.UpVotes,
// U.DownVotes,
// P.PostId,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// P.ViewCount AS PostViewCount,
// P.Score AS PostScore,
// P.CommentCount AS PostCommentCount
// FROM
// UserStats U
// JOIN
// PostStats P ON U.UserId = P.PostId
// ORDER BY
// U.Reputation DESC, P.ViewCount DESC
// LIMIT 100;
fn q10974(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and((&dc).opt()))))
        .drive(|_, ((p, c), (((u, a), d), e))| v.push((p, c, u, a, d.unwrap_or(0), e.unwrap_or(0))));
    out(v, |&(p, _, u, _, _, _)| (rep_desc(db, u), views_desc(db, p)), 100, |&(p, c, u, a, d, e)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d), V::I(e), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.push(V::I(c));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
// SUM(CASE WHEN p.CommentCount > 0 THEN 1 ELSE 0 END) AS PostsWithComments,
// AVG(p.ViewCount) AS AverageViewCount
// FROM Posts p
// GROUP BY p.PostTypeId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// AVG(u.Reputation) AS AverageReputation,
// SUM(u.Views) AS TotalViews
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// )
// SELECT
// p.PostTypeId,
// p.TotalPosts,
// p.PositiveScoreCount,
// p.PostsWithComments,
// p.AverageViewCount,
// u.TotalBadges,
// u.AverageReputation,
// u.TotalViews
// FROM PostStats p
// JOIN UserStats u ON u.UserId IN (SELECT OwnerUserId FROM Posts WHERE PostTypeId = p.PostTypeId)
// ORDER BY p.PostTypeId;
fn q10978(db: &'static So) -> String {
    let Post { score, comment_count, view_count, post_type_id, owner_user, .. } = &db.post;
    let pf = by_key(db.post.iq(), post_type_id, score.and(comment_count).and(view_count.opt()), [0i64; 5], |a, ((s, cc), w)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (cc > 0) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let us = g(db).select((&db.user.reputation).and(&db.user.views).and(badges_of(db).opt())).fold([0i64; 3], |a, ((r, w), _)| [a[0] + 1, a[1] + r, a[2] + w]);
    let bu = badges_per_user(db);
    let pairs: MatSet<(i64, Id<User>)> = owned(db).select(post_type_id.and(owner_user)).collect();
    let mut v = Vec::new();
    (&pairs)
        .select((&pairs).map(|(t, _)| t).select(&pf).and((&pairs).map(|(_, u)| u).select((&us).and(&bu))))
        .drive(|(t, _), (a, (s, b))| v.push((t, a, s, b)));
    rows(v.iter().map(|&(t, a, s, b)| row(vec![V::I(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), V::I(b), avg(s[1], s[0]), V::I(s[2])])))
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
// u.Id AS UserId,
// u.DisplayName,
// upc.PostCount,
// upc.QuestionCount,
// upc.AnswerCount,
// ub.BadgeCount,
// ub.GoldBadgeCount,
// ub.SilverBadgeCount,
// ub.BronzeBadgeCount
// FROM
// Users u
// LEFT JOIN
// UserPostCounts upc ON u.Id = upc.UserId
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// ORDER BY
// PostCount DESC, BadgeCount DESC
// LIMIT 100;
fn q10993(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, b)| (Reverse(a[0]), b.is_none(), Reverse(b.map(|b| b[0]))), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f
    })
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AvgScore,
// AVG(p.AnswerCount) AS AvgAnswersPerQuestion
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
// SUM(u.Reputation) AS TotalReputation,
// AVG(u.Reputation) AS AvgReputation,
// COUNT(DISTINCT CASE WHEN u.UpVotes > 0 THEN u.Id END) AS ActiveUsers,
// SUM(CASE WHEN u.LastAccessDate > cast('2024-10-01' as date) - INTERVAL '30 days' THEN 1 ELSE 0 END) AS RecentUsers
// FROM
// Users u
// ),
// CommentStats AS (
// SELECT
// COUNT(c.Id) AS TotalComments,
// AVG(LENGTH(c.Text)) AS AvgCommentLength,
// SUM(CASE WHEN c.Score > 0 THEN 1 ELSE 0 END) AS PositiveComments
// FROM
// Comments c
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.TotalViews,
// ps.AvgScore,
// ps.AvgAnswersPerQuestion,
// us.TotalUsers,
// us.TotalReputation,
// us.AvgReputation,
// us.ActiveUsers,
// us.RecentUsers,
// cs.TotalComments,
// cs.AvgCommentLength,
// cs.PositiveComments
// FROM
// PostStats ps,
// UserStats us,
// CommentStats cs
// ORDER BY
// ps.TotalPosts DESC;
fn q11002(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, answer_count, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), post_type_id.and(view_count.opt()).and(score).and(answer_count.opt()), [0i64; 8], |a, (((t, w), s), an)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s, a[6] + an.is_some() as i64, a[7] + an.unwrap_or(0)]
    });
    let User { reputation, up_votes, last_access_date, .. } = &db.user;
    let us = whole(db.user.iq()).select(reputation.and(last_access_date)).fold((0i64, 0i64, 0i64), |(n, s, c), (r, la)| (n + 1, s + r, c + (la > date(2024, 9, 1)) as i64));
    let act = whole(db.user.with(up_votes.gt(0))).select(Ident::<User>::new()).count_distinct();
    let cs = whole(db.comment.iq()).select((&db.comment.text).and(&db.comment.score)).fold((0i64, 0i64, 0i64), |(n, l, p), (t, s)| (n + 1, l + t.chars().count() as i64, p + (s > 0) as i64));
    let mut v = Vec::new();
    let one = rel(vec![()]).select((&us).opt().and((&act).opt()).and((&cs).opt()));
    (&f).cross(&one).drive(|(k, _), (a, ((u, active), c))| v.push((k, a, u.unwrap_or((0, 0, 0)), active.unwrap_or(0), c.unwrap_or((0, 0, 0)))));
    rows(v.iter().map(|&(k, a, (un, rs, rec), active, (cn, cl, cp))| {
        row(vec![
            V::S(k),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            nullable(a[4], a[3]),
            avg(a[5], a[0]),
            avg(a[7], a[6]),
            V::I(un),
            nullable(rs, un),
            avg(rs, un),
            V::I(active),
            nullable(rec, un),
            V::I(cn),
            avg(cl, cn),
            nullable(cp, cn),
        ])
    }))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// INNER JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// BadgeStats AS (
// SELECT
// COUNT(DISTINCT b.UserId) AS UniqueUsersWithBadges
// FROM
// Badges b
// )
// SELECT
// ps.PostType,
// ps.PostCount,
// ps.AverageScore,
// bs.UniqueUsersWithBadges
// FROM
// PostStats ps,
// BadgeStats bs
// ORDER BY
// ps.PostType;
fn q11008(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), &db.post.score, (0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let holders = whole(db.badge.iq()).select(&db.badge.user_id).count_distinct();
    let mut v = Vec::new();
    let one = rel(vec![()]).select((&holders).opt());
    (&f).cross(&one).drive(|(k, _), (a, h)| v.push((k, a, h.unwrap_or(0))));
    rows(v.iter().map(|&(k, (n, s), holders)| row(vec![V::S(k), V::I(n), avg(s, n), V::I(holders)])))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerName,
// u.Reputation AS OwnerReputation,
// COALESCE(ph.RevisionCount, 0) AS PostHistoryCount,
// COALESCE(c.CommentCount, 0) AS TotalComments
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS RevisionCount
// FROM PostHistory
// GROUP BY PostId) ph ON p.Id = ph.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11012(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(history_per_post(db)).and(comments_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, h), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep"]);
        f.extend([V::I(h), V::I(c)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// AVG(c.CommentCount) AS AvgComments
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(Id) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY
// p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id,
// u.DisplayName,
// u.Reputation,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.AvgScore, 0) AS AvgScore,
// COALESCE(ps.AvgComments, 0) AS AvgComments
// FROM
// Users u
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.AvgScore,
// us.AvgComments
// FROM
// UserStats us
// ORDER BY
// us.Reputation DESC;
fn q11024(db: &'static So) -> String {
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&cf).opt())).fold([0i64; 4], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c.is_some() as i64, a[3] + c.unwrap_or(0)]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, a)| {
        let a = a.unwrap_or([0; 4]);
        row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), or0(a[1], a[0]), or0(a[3], a[2])])
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// PT.Name AS PostType,
// U.DisplayName AS OwnerDisplayName,
// COALESCE(V.UpVotesCount, 0) AS UpVotesCount,
// COALESCE(V.DownVotesCount, 0) AS DownVotesCount,
// COALESCE(B.BadgeCount, 0) AS BadgeCount
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
// FROM
// Votes
// GROUP BY
// PostId) V ON P.Id = V.PostId
// LEFT JOIN
// (SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId) B ON U.Id = B.UserId
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q11026(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and((&db.post.owner_user).select(&bu).opt())).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, u), d), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "type", "owner"]);
        f.extend([V::I(u), V::I(d), V::I(b.unwrap_or(0))]);
        f
    })
}

// WITH UserVoteCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM Users U
// LEFT JOIN Votes V ON U.Id = V.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.CreationDate,
// P.AnswerCount,
// P.CommentCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// COUNT(C.ID) AS TotalComments
// FROM Posts P
// LEFT JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate, P.AnswerCount, P.CommentCount, U.DisplayName, U.Reputation
// ),
// BenchmarkStats AS (
// SELECT
// P.PostId,
// P.Title,
// P.OwnerDisplayName,
// P.OwnerReputation,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.TotalComments,
// UV.TotalVotes,
// UV.Upvotes,
// UV.Downvotes
// FROM PostStatistics P
// LEFT JOIN UserVoteCounts UV ON P.OwnerDisplayName = UV.DisplayName
// )
// SELECT
// PostId,
// Title,
// OwnerDisplayName,
// OwnerReputation,
// Score,
// ViewCount,
// AnswerCount,
// CommentCount,
// TotalComments,
// TotalVotes,
// Upvotes,
// Downvotes
// FROM BenchmarkStats
// ORDER BY Score DESC, ViewCount DESC
// LIMIT 100;
fn q11029(db: &'static So) -> String {
    let names = by_name(db);
    let uv = user_votes(db);
    let mut v = Vec::new();
    db.post
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(&uv).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| score_views(db, p), 100, |&((p, c), a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "rep", "score", "views", "answers", "comments"]);
        f.push(V::I(c));
        f.extend((0..3).map(|i| oint(a.map(|a| a[i]))));
        f
    })
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
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11034(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and((&db.post.view_count).opt()).and(comments_per_post(db)), [0i64; 5], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// (SELECT COUNT(*) FROM Users) AS TotalUsers
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
fn q11040(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_per_post(db)), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c]);
    let tu = whole(db.user.iq()).fold(0i64, |n, _| n + 1);
    let mut v = Vec::new();
    let one = rel(vec![()]).select((&tu).opt());
    (&f).cross(&one).drive(|(k, _), (a, tu)| v.push((k, a, tu.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, tu)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(tu)])))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// AVG(p.Score) AS AverageScore,
// p.CreationDate
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.Id, p.Title, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CommentCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.AverageScore,
// us.UserId AS PostOwnerId,
// us.DisplayName AS PostOwnerName,
// us.BadgeCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM PostStats ps
// JOIN Users u ON ps.PostId = u.Id
// JOIN UserStats us ON u.Id = us.UserId
// ORDER BY ps.AverageScore DESC
// LIMIT 100;
fn q11053(db: &'static So) -> String {
    let uid = uids(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt())).fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, _, _)| score_desc(db, p), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::F(db.post.score.get(p).unwrap() as f64), user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&a));
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
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT bh.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// Badges bh ON bh.UserId = p.OwnerUserId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostsCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// SUM(u.Views) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Posts p ON p.OwnerUserId = u.Id
// WHERE
// u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
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
// ps.VoteCount,
// us.UserId,
// us.DisplayName,
// us.PostsCount,
// us.TotalUpVotes,
// us.TotalDownVotes,
// us.TotalViews
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC
// LIMIT 100;
fn q11068(db: &'static So) -> String {
    let uid = uids(db);
    let x = per_post_distinct(db, votes_of(db));
    let User { up_votes, down_votes, views, .. } = &db.user;
    let w = UserWhere::CreatedGe(year_ago());
    let us = user_base(db, w).group_by(Ident::<User>::new()).select(up_votes.and(down_votes).and(views).and(posts_of(db).opt())).fold([0i64; 3], |a, (((u, d), v), _)| [a[0] + u, a[1] + d, a[2] + v]);
    let dp = ud(db, w, posts_of(db));
    let mut v = Vec::new();
    let bid = badges_by_uid(db);
    since(db, year_ago())
        .with((&db.post.origid).select(&uid))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and((&db.post.owner_user_id).select(&bid).opt()))
        .fold(0i64, |n, ((c, _), _)| n + c.is_some() as i64)
        .and((&x).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, ((s, x), ((u, a), d))| v.push((p, s, x.unwrap_or(0), u, a, d.unwrap_or(0))));
    out(v, |&(p, _, _, _, _, _)| views_score(db, p), 100, |&(p, s, x, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(s), V::I(x), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d)]);
        f.extend(ints(&a));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// AVG(u.Reputation) AS AvgReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.UserId) AS UniqueVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotes,
// us.DownVotes,
// us.AvgReputation,
// pm.PostId,
// pm.Title,
// pm.CreationDate,
// pm.Score,
// pm.ViewCount,
// pm.CommentCount,
// pm.UniqueVotes
// FROM
// UserStats us
// JOIN
// PostMetrics pm ON us.UserId = pm.PostId
// ORDER BY
// us.AvgReputation DESC, pm.Score DESC;
fn q11074(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dv = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&dv).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, ((s, x), ((u, a), d))| v.push((p, s, x.unwrap_or(0), u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, x, u, a, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down), ustat_field(&a, "rep_avg")];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(s.cx), V::I(x)]);
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
// PostInteractions AS (
// SELECT
// p.Id AS PostId,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes
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
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalScore,
// ups.TotalViews,
// COALESCE(pi.TotalComments, 0) AS TotalComments,
// COALESCE(pi.TotalVotes, 0) AS TotalVotes
// FROM
// UserPostStats ups
// LEFT JOIN
// PostInteractions pi ON ups.UserId = pi.PostId
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC;
fn q11075(db: &'static So) -> String {
    let pid = pids(db);
    let pi = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    upqa(db).and((&db.user.origid).select(&pid).select(&pi).opt()).drive(|u, (a, s)| v.push((u, a, s)));
    rows(v.iter().map(|&(u, a, s)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[5], a[0]), nullable(a[4], a[3]), V::I(s.map_or(0, |s| s.cx)), V::I(s.map_or(0, |s| s.vx))]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
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
// u.DisplayName,
// u.PostCount,
// u.QuestionsCount,
// u.AnswersCount,
// u.TotalScore,
// u.TotalViews,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// COALESCE(b.GoldBadges, 0) AS GoldBadges,
// COALESCE(b.SilverBadges, 0) AS SilverBadges,
// COALESCE(b.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats u
// LEFT JOIN
// UserBadgeStats b ON u.UserId = b.UserId
// ORDER BY
// u.TotalScore DESC,
// u.PostCount DESC;
fn q11082(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[5], a[0]), nullable(a[4], a[3])]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// U.Id AS OwnerUserId,
// U.DisplayName AS OwnerDisplayName,
// COUNT(C.Id) AS TotalComments,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON C.PostId = P.Id
// LEFT JOIN
// Votes V ON V.PostId = P.Id
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, U.Id, U.DisplayName
// ),
// UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUserUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalUserDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON V.UserId = U.Id
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.PostCreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.TotalComments,
// PS.TotalUpVotes,
// PS.TotalDownVotes,
// US.UserId AS OwnerUserId,
// US.DisplayName AS OwnerDisplayName,
// US.TotalPosts AS UserTotalPosts,
// US.TotalUserUpVotes,
// US.TotalUserDownVotes
// FROM
// PostStatistics PS
// JOIN
// UserStatistics US ON PS.OwnerUserId = US.UserId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q11090(db: &'static So) -> String {
    let us = g(db).select(posts_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (p, t)| {
        [a[0] + p.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let mut v = Vec::new();
    stats_fold(db, owned(db).with((&db.post.post_type_id).eq(1)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// P.OwnerUserId,
// P.PostTypeId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// COUNT(C) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.OwnerUserId, P.PostTypeId
// ),
// UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.DisplayName,
// U.CreationDate
// FROM
// Users U
// )
// SELECT
// UR.UserId,
// UR.DisplayName,
// UR.Reputation,
// PS.QuestionCount,
// PS.AnswerCount,
// PS.TotalScore,
// PS.TotalViews,
// PS.CommentCount,
// PS.VoteCount,
// UR.CreationDate
// FROM
// UserReputation UR
// LEFT JOIN
// PostStatistics PS ON UR.UserId = PS.OwnerUserId
// WHERE
// UR.Reputation > 0
// ORDER BY
// UR.Reputation DESC;
fn q11091(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let key = || owner_user.and(post_type_id);
    let f = owned(db).group_by(key()).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt())).fold([0i64; 6], |a, ((((t, s), w), _), _)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + 1]
    });
    let dv = owned(db).group_by(key()).select(votes_of(db)).count_distinct();
    let keys: MatSet<(Id<User>, i64)> = owned(db).select(key()).collect();
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&keys).map(|(u, _)| u).inv().collect();
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(0))
        .select(Ident::<User>::new().and((&by_user).select((&f).and((&dv).opt())).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, x)| {
        let mut r = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        match x {
            Some((a, d)) => r.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5]), V::I(d.unwrap_or(0))]),
            None => r.extend(nulls(6)),
        }
        r.push(user_col(db, u, "ucreated"));
        row(r)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// COUNT(v.Id) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11092(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_per_post(db)).and(votes_of(db).opt()), [0i64; 4], |a, ((s, c), x)| {
        [a[0] + 1, a[1] + s, a[2] + c, a[3] + x.is_some() as i64]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])])))
}

// WITH UserPosts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// AVG(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS AvgViewsPerPost
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// PostHistoryAggregates AS (
// SELECT
// ph.PostId,
// COUNT(*) AS HistoryCount,
// MAX(ph.CreationDate) AS LastModified
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// ),
// PostsWithHistory AS (
// SELECT
// p.*,
// pha.HistoryCount,
// pha.LastModified
// FROM
// Posts p
// LEFT JOIN
// PostHistoryAggregates pha ON p.Id = pha.PostId
// )
// SELECT
// u.DisplayName,
// up.PostCount,
// up.TotalScore,
// up.TotalViews,
// up.AvgViewsPerPost,
// p.Title,
// p.HistoryCount,
// p.LastModified,
// p.CreationDate AS PostCreationDate
// FROM
// UserPosts up
// JOIN
// Users u ON up.UserId = u.Id
// JOIN
// PostsWithHistory p ON u.Id = p.OwnerUserId
// ORDER BY
// up.TotalScore DESC,
// up.PostCount DESC;
fn q11096(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let up = g(db).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)],
        None => a,
    });
    let hc = history_per_post(db);
    let hm = history_max_date(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(&hc).and(&hm).and((&db.post.owner_user).select(Ident::<User>::new().and(&up)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, h), m), (u, a))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(a[2] as f64 / a[0] as f64)];
        f.extend(post_fields(db, p, &["title"]));
        f.extend(if h == 0 { [V::Null, V::Null] } else { [V::I(h), V::T(m)] });
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
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
    ("10796", q10796),
    ("10798", q10798),
    ("10799", q10799),
    ("10803", q10803),
    ("10847", q10847),
    ("10848", q10848),
    ("10852", q10852),
    ("10863", q10863),
    ("10867", q10867),
    ("10875", q10875),
    ("10886", q10886),
    ("10903", q10903),
    ("10910", q10910),
    ("10912", q10912),
    ("10916", q10916),
    ("10924", q10924),
    ("10934", q10934),
    ("10950", q10950),
    ("10952", q10952),
    ("10955", q10955),
    ("10960", q10960),
    ("10969", q10969),
    ("10971", q10971),
    ("10973", q10973),
    ("10974", q10974),
    ("10978", q10978),
    ("10993", q10993),
    ("11002", q11002),
    ("11008", q11008),
    ("11012", q11012),
    ("11024", q11024),
    ("11026", q11026),
    ("11029", q11029),
    ("11034", q11034),
    ("11040", q11040),
    ("11053", q11053),
    ("11068", q11068),
    ("11074", q11074),
    ("11075", q11075),
    ("11082", q11082),
    ("11090", q11090),
    ("11091", q11091),
    ("11092", q11092),
    ("11096", q11096),
];
