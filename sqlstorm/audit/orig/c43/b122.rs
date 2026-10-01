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

// --- batch 122 --------------------------------------------------------------

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPosts,
// AVG(p.ViewCount) AS AvgViewCount
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
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.UpvotedPosts,
// ups.DownvotedPosts,
// ups.AvgViewCount,
// COALESCE(ubc.TotalBadges, 0) AS TotalBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeCounts ubc ON ups.UserId = ubc.UserId
// ORDER BY
// ups.TotalPosts DESC;
fn q13213(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 7], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64, a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0)],
        None => a,
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..5]));
        f.extend([avg(a[6], a[5]), V::I(b)]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalUsers,
// COUNT(DISTINCT Id) FILTER (WHERE ParentId IS NOT NULL) AS TotalAnswers,
// COUNT(DISTINCT Id) FILTER (WHERE ParentId IS NULL) AS TotalQuestions
// FROM
// Posts
// ),
// UserStatistics AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation
// FROM
// Users
// ),
// CommentStatistics AS (
// SELECT
// COUNT(*) AS TotalComments,
// AVG(Score) AS AvgCommentScore
// FROM
// Comments
// ),
// RecentPosts AS (
// SELECT
// p.Id,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// ORDER BY
// p.CreationDate DESC
// LIMIT 10
// )
// SELECT
// ps.TotalPosts,
// ps.TotalUsers,
// ps.TotalAnswers,
// ps.TotalQuestions,
// us.TotalUsers AS UniqueUsers,
// us.AvgReputation,
// cs.TotalComments,
// cs.AvgCommentScore,
// rp.Id AS RecentPostId,
// rp.Title AS RecentPostTitle,
// rp.CreationDate AS RecentPostDate,
// rp.OwnerDisplayName AS RecentPostOwner
// FROM
// PostStatistics ps,
// UserStatistics us,
// CommentStatistics cs,
// RecentPosts rp;
fn q13216(db: &'static So) -> String {
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let p = db.post.select((&db.post.parent_id).opt()).fold_flat([0i64; 3], |a, pa| [a[0] + 1, a[1] + pa.is_some() as i64, a[2] + pa.is_none() as i64]);
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let (cn, cs) = db.comment.select(&db.comment.score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mut v = Vec::new();
    owned(db).drive(|_, p| v.push(p));
    out(v, |&p| newest(db, p), 10, |&q| {
        let mut f = vec![V::I(p[0]), V::I(owners), V::I(p[1]), V::I(p[2]), V::I(un), avg(rs, un), V::I(cn), avg(cs, cn)];
        f.extend(post_fields(db, q, &["id", "title", "created", "owner"]));
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
// AVG(p.ViewCount) AS AvgViewCount
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
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.TotalQuestions,
// u.TotalAnswers,
// u.TotalUpvotedPosts,
// u.AvgViewCount,
// COALESCE(b.TotalBadges, 0) AS TotalBadges,
// COALESCE(b.GoldBadges, 0) AS GoldBadges,
// COALESCE(b.SilverBadges, 0) AS SilverBadges,
// COALESCE(b.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats u
// LEFT JOIN
// UserBadgeStats b ON u.UserId = b.UserId
// ORDER BY
// u.TotalPosts DESC, u.AvgViewCount DESC;
fn q13218(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.push(avg(a[5], a[4]));
        f.extend(ints(&b));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.AnswerCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// ARRAY_AGG(t.TagName) AS Tags,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount
// FROM
// Posts p
// INNER JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id AND v.VoteTypeId IN (8, 9)
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score,
// u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13219(db: &'static So) -> String {
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let v89 = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![8, 9]))).select((&db.vote.bounty_amount).opt());
    let prod = || (&ex).select(&db.tag.tag_name).opt().and(comments_of(db).opt()).and(v89().opt());
    let f = owned(db).group_by(Ident::<Post>::new()).select(prod()).fold([0i64; 2], |a, ((_, c), b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let tags = owned(db).group_by(Ident::<Post>::new()).select(prod().map(|((t, _), _): ((Option<Str>, Option<Id<Comment>>), Option<Option<i64>>)| t)).buf_fold(|ts| &*Box::leak(ts.into_vec().into_boxed_slice()));
    let mut v = Vec::new();
    (&f).and(&tags).drive(|p, (a, t)| v.push((p, a, t)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, a, t)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers", "score", "owner", "rep"]);
        f.extend([V::L(t.iter().map(|&n| ostr(n)).collect()), V::I(a[0]), V::I(a[1])]);
        f
    })
}

// WITH PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis
// FROM
// Posts
// ),
// UserCounts AS (
// SELECT
// COUNT(*) AS TotalUsers,
// SUM(CASE WHEN Reputation > 1000 THEN 1 ELSE 0 END) AS ActiveUsers
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
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Votes
// )
// SELECT
// (SELECT TotalPosts FROM PostCounts) AS TotalPosts,
// (SELECT TotalQuestions FROM PostCounts) AS TotalQuestions,
// (SELECT TotalAnswers FROM PostCounts) AS TotalAnswers,
// (SELECT TotalTagWikis FROM PostCounts) AS TotalTagWikis,
// (SELECT TotalUsers FROM UserCounts) AS TotalUsers,
// (SELECT ActiveUsers FROM UserCounts) AS ActiveUsers,
// (SELECT TotalComments FROM CommentCounts) AS TotalComments,
// (SELECT TotalVotes FROM VoteCounts) AS TotalVotes,
// (SELECT TotalUpvotes FROM VoteCounts) AS TotalUpvotes,
// (SELECT TotalDownvotes FROM VoteCounts) AS TotalDownvotes;
fn q13223(db: &'static So) -> String {
    let p = db.post.select(&db.post.post_type_id).fold_flat([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64]);
    let (un, act) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, a), r| (n + 1, a + (r > 1000) as i64));
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut f = ints(&p);
    f.extend([V::I(un), V::I(act), V::I(count(db.comment.iq()))]);
    f.extend(ints(&x));
    row(f)
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score AS PostScore,
// p.ViewCount,
// u.Id AS UserId,
// u.DisplayName AS UserName,
// u.Reputation,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// COALESCE(v.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(v.DownVoteCount, 0) AS DownVoteCount,
// COALESCE(v.TotalVotes, 0) AS TotalVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId) b ON u.Id = b.UserId
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(*) AS TotalVotes
// FROM
// Votes
// GROUP BY
// PostId) v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2021-01-01'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13244(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned_since(db, date(2021, 1, 1)).select(Ident::<Post>::new().and((&db.post.owner_user).select(&bu)).and((&pv).opt())).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, b), x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "uid", "owner", "rep"]);
        f.extend([V::I(b), V::I(x[1]), V::I(x[2]), V::I(x[0])]);
        f
    })
}

// WITH UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
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
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// CommentCount,
// UpVotes,
// DownVotes,
// (PostCount + CommentCount) AS TotalEngagement,
// (UpVotes - DownVotes) AS EngagementScore
// FROM
// UserEngagement
// ORDER BY
// TotalEngagement DESC
// LIMIT 10;
fn q13251(db: &'static So) -> String {
    out(users_with_counts(db, "cv", false), |r| Reverse(r.agg.n + r.agg.c), 10, |r| {
        let a = r.agg;
        let mut f = user_fields(r, "cv", &["uid", "name", "#n", "#c", "#up", "#down"]);
        f.extend([V::I(a.n + a.c), V::I(a.up - a.down)]);
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COALESCE(NULLIF(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0), 0) AS UpVoteCount,
// COALESCE(NULLIF(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0), 0) AS DownVoteCount,
// COALESCE(NULLIF(MAX(b.Date), '1970-01-01'), NULL) AS LastBadgeDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON b.UserId = p.OwnerUserId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// *,
// (UpVoteCount - DownVoteCount) AS NetVoteCount
// FROM
// PostStatistics
// ORDER BY
// CreationDate DESC
// LIMIT 100;
fn q13259(db: &'static So) -> String {
    out(stats_with(db, db.post.iq(), "cvb", &[], &[]), |&(p, _, _)| newest(db, p), 100, |&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), stat_field(&s, "bmax").unwrap(), V::I(s.up - s.down)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.Score,
// p.CreationDate,
// p.LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.Score, p.CreationDate, p.LastActivityDate
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
// ps.CommentCount,
// ps.AnswerCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.Score,
// ps.CreationDate,
// ps.LastActivityDate,
// us.UserId,
// us.DisplayName AS UserDisplayName,
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
// ps.LastActivityDate DESC
// LIMIT 100;
fn q13268(db: &'static So) -> String {
    let uid = uids(db);
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, answers_of(db));
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt())).fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let mut v = Vec::new();
    stats_fold(db, questions_only(db).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvA", &[])
        .and((&c).opt())
        .and((&a).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (((s, c), a), (u, x))| v.push((p, s, c.unwrap_or(0), a.unwrap_or(0), u, x)));
    out(v, |&(p, ..)| Reverse(db.post.last_activity_date.get(p).unwrap()), 100, |&(p, s, c, a, u, x)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c), V::I(a), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["score", "created", "activity"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&x));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.CreationDate, p.Score, p.ViewCount
// ),
// UserStats AS (
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
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// us.UserId,
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
// ps.Score DESC, ps.ViewCount DESC;
fn q13272(db: &'static So) -> String {
    let uid = uids(db);
    let x = per_post_distinct(db, votes_of(db));
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt())).fold([0i64; 2], |a, ((u, d), _)| [a[0] + u, a[1] + d]);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&x).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&bu).and(&us)))
        .drive(|p, ((s, x), ((u, b), a))| v.push((p, s, x.unwrap_or(0), u, b, a)));
    rows(v.iter().map(|&(p, s, x, u, b, a)| {
        let mut f = post_fields(db, p, &["id", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(x), user_col(db, u, "uid"), V::I(b), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
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
// ),
// PostStats AS (
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
// )
// SELECT
// ue.UserId,
// ue.DisplayName,
// ue.TotalPosts,
// ue.TotalComments,
// ue.TotalUpVotes,
// ue.TotalDownVotes,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.VoteCount
// FROM
// UserEngagement ue
// JOIN
// PostStats ps ON ue.UserId = ps.PostId
// ORDER BY
// ue.TotalPosts DESC, ue.TotalUpVotes DESC;
fn q13276(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and((&dc).opt()))))
        .drive(|_, y| v.push(y));
    rows(v.iter().map(|&(((p, c), x), (((u, a), d), e))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d.unwrap_or(0)), V::I(e.unwrap_or(0)), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0))]);
        row(f)
    }))
}

// WITH Statistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.OwnerUserId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// s.TotalPosts,
// s.TotalQuestions,
// s.TotalAnswers,
// s.AverageScore,
// s.TotalViews,
// s.TotalComments
// FROM
// Users u
// LEFT JOIN
// Statistics s ON u.Id = s.OwnerUserId
// )
// SELECT
// UserId,
// Reputation,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// AverageScore,
// TotalViews,
// TotalComments
// FROM
// UserReputation
// ORDER BY
// Reputation DESC
// LIMIT 100;
fn q13283(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).opt())).fold([0i64; 7], |a, (((t, s), w), c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + c.is_some() as i64]
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt())).drive(|_, x| v.push(x));
    out(v, |&(u, _)| rep_desc(db, u), 100, |&(u, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        match p {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), nullable(a[5], a[4]), V::I(a[6])]),
            None => f.extend(nulls(6)),
        }
        f
    })
}

// SELECT
// p.Id AS PostID,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(vote_counts.UpVotes, 0) AS UpVotes,
// COALESCE(vote_counts.DownVotes, 0) AS DownVotes,
// COALESCE(comment_counts.CommentCount, 0) AS CommentCount,
// COALESCE(user_stats.UserReputation, 0) AS UserReputation,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
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
// ) AS vote_counts ON p.Id = vote_counts.PostId
// LEFT JOIN
// (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) AS comment_counts ON p.Id = comment_counts.PostId
// LEFT JOIN
// (
// SELECT
// u.Id,
// SUM(u.Reputation) AS UserReputation
// FROM
// Users u
// GROUP BY
// u.Id
// ) AS user_stats ON u.Id = user_stats.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13285(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    since(db, year_ago()).select(Ident::<Post>::new().and((&pv).opt()).and(comments_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, x), c)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(c), V::I(db.post.owner_user.get(p).map_or(0, |u| db.user.reputation.get(u).unwrap()))]);
        f.extend(post_fields(db, p, &["owner"]));
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
// COALESCE(u.Reputation, 0) AS OwnerReputation,
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
// p.CreationDate BETWEEN '2022-01-01' AND '2023-12-31'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, u.Reputation
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
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// ps.OwnerReputation,
// ps.CommentCount,
// ps.VoteCount,
// us.DisplayName AS OwnerDisplayName,
// us.BadgeCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostStats ps
// LEFT JOIN
// UserStats us ON ps.OwnerReputation = us.UserId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q13286(db: &'static So) -> String {
    let uid = uids(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt())).fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let base = db.post.with((&db.post.creation_date).between(date(2022, 1, 1), date(2023, 12, 31)));
    let rep0 = || (&db.post.owner_user).select(&db.user.reputation).opt().map(|r: Option<i64>| r.unwrap_or(0));
    let mut v = Vec::new();
    stats_fold(db, base, Ident::<Post>::new(), "cv", &[]).and(rep0().select(&uid).select(Ident::<User>::new().and(&us)).opt()).drive(|p, (s, x)| v.push((p, s, x)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, s, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.extend([V::I(db.post.owner_user.get(p).map_or(0, |u| db.user.reputation.get(u).unwrap())), V::I(s.cx), V::I(s.vx)]);
        match x {
            Some((u, a)) => {
                f.push(user_col(db, u, "name"));
                f.extend(ints(&a));
            }
            None => f.extend(nulls(4)),
        }
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ),
// UserStatistics AS (
// SELECT
// AVG(u.Reputation) AS AverageReputation,
// COUNT(b.Id) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// )
// SELECT
// ps.PostTypeName,
// ps.TotalPosts,
// ps.AverageScore,
// ps.TotalComments,
// us.AverageReputation,
// us.TotalBadges
// FROM
// PostStatistics ps,
// UserStatistics us
// ORDER BY
// ps.TotalPosts DESC;
fn q13295(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_of(db).opt()), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c.is_some() as i64]);
    let (rn, rs, bn) = db.user.select((&db.user.reputation).and(badges_of(db).opt())).fold_flat((0i64, 0i64, 0i64), |(n, s, b), (r, x)| (n + 1, s + r, b + x.is_some() as i64));
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), avg(rs, rn), V::I(bn)])))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(V.BountyAmount) AS TotalBounties,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.OwnerUserId
// FROM Posts P
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.PostCount,
// U.TotalBounties,
// U.UpVotes,
// U.DownVotes,
// COUNT(DISTINCT PS.PostId) AS ActivePostCount,
// AVG(PS.Score) AS AveragePostScore,
// SUM(PS.ViewCount) AS TotalPostViews
// FROM UserStats U
// LEFT JOIN PostStats PS ON U.UserId = PS.OwnerUserId
// GROUP BY U.UserId, U.DisplayName, U.PostCount, U.TotalBounties, U.UpVotes, U.DownVotes
// ORDER BY U.UserId;
fn q13296(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let ps = g(db).select(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
        None => a,
    });
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&ps).drive(|u, ((a, d), p)| v.push((u, a, d.unwrap_or(0), p)));
    rows(v.iter().map(|&(u, a, d, p)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), ustat_field(&a, "bounty_sum"), V::I(a.up), V::I(a.down), V::I(p[0]), avg(p[1], p[0]), nullable(p[3], p[2])])
    }))
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
// COUNT(Id) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13303(db: &'static So) -> String {
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
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AvgScorePerPost,
// AVG(P.ViewCount) AS AvgViewsPerPost
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
// UPS.Questions,
// UPS.Answers,
// UPS.TotalScore,
// UPS.TotalViews,
// UPS.AvgScorePerPost,
// UPS.AvgViewsPerPost,
// UBS.TotalBadges,
// UBS.GoldBadges,
// UBS.SilverBadges,
// UBS.BronzeBadges
// FROM
// UserPostStats UPS
// LEFT JOIN
// UserBadgeStats UBS ON UPS.UserId = UBS.UserId
// ORDER BY
// UPS.TotalScore DESC, UPS.TotalPosts DESC;
fn q13308(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[3], a[0]), nullable(a[5], a[4]), avg(a[3], a[0]), avg(a[5], a[4])]);
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        row(f)
    }))
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
// p.OwnerUserId,
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
// p.Id, p.OwnerUserId
// ),
// FinalStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// us.TotalVotes,
// us.UpVotes AS UserUpVotes,
// us.DownVotes AS UserDownVotes,
// p.PostId,
// p.CommentCount,
// p.UpVotes AS PostUpVotes,
// p.DownVotes AS PostDownVotes
// FROM
// UserVoteStats us
// JOIN
// Users u ON us.UserId = u.Id
// JOIN
// PostStats p ON p.OwnerUserId = u.Id
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// TotalVotes,
// UserUpVotes,
// UserDownVotes,
// PostId,
// CommentCount,
// PostUpVotes,
// PostDownVotes
// FROM
// FinalStats
// ORDER BY
// Reputation DESC, TotalVotes DESC;
fn q13312(db: &'static So) -> String {
    let uv = user_votes(db);
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(Ident::<User>::new().and(&uv))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId IN (2) THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 12 THEN 1 ELSE 0 END), 0) AS SpamCount,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// AVG(LENGTH(p.Body)) AS AvgBodyLength
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.PostTypeId
// ),
// PostTypeSummary AS (
// SELECT
// pt.Name AS PostType,
// COUNT(ps.PostId) AS PostCount,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.UpVoteCount) AS TotalUpVotes,
// SUM(ps.DownVoteCount) AS TotalDownVotes,
// SUM(ps.SpamCount) AS TotalSpam,
// SUM(ps.TotalViews) AS TotalViews,
// AVG(ps.AvgBodyLength) AS AverageBodyLength
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// PostCount,
// TotalComments,
// TotalUpVotes,
// TotalDownVotes,
// TotalSpam,
// TotalViews,
// AverageBodyLength
// FROM
// PostTypeSummary
// ORDER BY
// PostCount DESC;
fn q13321(db: &'static So) -> String {
    let Post { view_count, body, .. } = &db.post;
    let base = || since(db, date(2023, 1, 1));
    let pf = stats_fold(db, base(), Ident::<Post>::new(), "cv", &[]);
    let f = by_key(base(), name(db), (&pf).and(view_count.opt()).and(body), [0i64; 7], |a, ((s, w), b)| {
        [a[0] + 1, a[1] + s.cx, a[2] + s.up, a[3] + s.down, a[4] + s.by_vt[12], a[5] + w.unwrap_or(0) * s.rows, a[6] + b.chars().count() as i64]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| {
        let mut f = vec![V::S(k)];
        f.extend(ints(&a[..6]));
        f.push(avg(a[6], a[0]));
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// P.CreationDate,
// U.Reputation
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON P.OwnerUserId = B.UserId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= DATE '2022-01-01'
// GROUP BY
// P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate, U.Reputation
// )
// SELECT
// PostId,
// Title,
// Score,
// ViewCount,
// CommentCount,
// VoteCount,
// BadgeCount,
// CreationDate,
// Reputation,
// CASE
// WHEN Score > 10 THEN 'High Engagement'
// WHEN Score BETWEEN 1 AND 10 THEN 'Moderate Engagement'
// ELSE 'Low Engagement'
// END AS EngagementLevel
// FROM
// PostStatistics
// ORDER BY
// Score DESC, ViewCount DESC;
fn q13327(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    rows(stats_with(db, since(db, date(2022, 1, 1)), "cvb", &[], &[&x, &b]).iter().map(|&(p, s, d)| {
        let sc = db.post.score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(s.cx), V::I(d[0]), V::I(d[1])]);
        f.extend(post_fields(db, p, &["created", "rep"]));
        f.push(V::S(if sc > 10 { "High Engagement" } else if (1..=10).contains(&sc) { "Moderate Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
// FROM Posts
// ),
// CommentCounts AS (
// SELECT
// COUNT(*) AS TotalComments
// FROM Comments
// ),
// UserCounts AS (
// SELECT
// COUNT(*) AS TotalUsers,
// SUM(CASE WHEN Reputation > 0 THEN 1 ELSE 0 END) AS ActiveUsers
// FROM Users
// )
// SELECT
// p.TotalPosts,
// p.TotalQuestions,
// p.TotalAnswers,
// c.TotalComments,
// u.TotalUsers,
// u.ActiveUsers
// FROM PostCounts p, CommentCounts c, UserCounts u;
fn q13334(db: &'static So) -> String {
    let p = db.post.select(&db.post.post_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let (un, act) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, a), r| (n + 1, a + (r > 0) as i64));
    let mut f = ints(&p);
    f.extend([V::I(count(db.comment.iq())), V::I(un), V::I(act)]);
    row(f)
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// CASE
// WHEN p.PostTypeId = 1 THEN 'Question'
// WHEN p.PostTypeId = 2 THEN 'Answer'
// ELSE 'Other'
// END AS PostType
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
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId
// ) v ON p.Id = v.PostId
// LEFT JOIN (
// SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// ) b ON p.OwnerUserId = b.UserId
// WHERE p.CreationDate >= '2023-01-01'
// )
// SELECT
// PostType,
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// AVG(AnswerCount) AS AvgAnswerCount,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(VoteCount) AS AvgVoteCount,
// AVG(BadgeCount) AS AvgBadgeCount
// FROM
// PostMetrics
// GROUP BY
// PostType
// ORDER BY
// TotalPosts DESC;
fn q13341(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let bu = badges_per_user(db);
    let label = |t: i64| -> Str {
        match t {
            1 => "Question",
            2 => "Answer",
            _ => "Other",
        }
    };
    let f = by_key(
        since(db, date(2023, 1, 1)),
        (&db.post.post_type_id).map(label),
        view_count.opt().and(score).and(typed_answers_per_post(db)).and(comments_per_post(db)).and(votes_per_post(db)).and((&db.post.owner_user).select(&bu).opt()),
        [0i64; 8],
        |a, (((((w, s), an), c), x), b)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + an, a[5] + c, a[6] + x, a[7] + b.unwrap_or(0)],
    );
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), avg(a[4], a[0]), avg(a[5], a[0]), avg(a[6], a[0]), avg(a[7], a[0])])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis,
// SUM(CASE WHEN p.PostTypeId = 6 THEN 1 ELSE 0 END) AS ModeratorNominations,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// ),
// BadgeStats AS (
// SELECT
// UserId,
// COUNT(*) AS TotalBadges,
// COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.TotalPosts,
// us.Questions,
// us.Answers,
// us.Wikis,
// us.TagWikis,
// us.ModeratorNominations,
// us.TotalViews,
// us.TotalScore,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats us
// LEFT JOIN
// BadgeStats bs ON us.UserId = bs.UserId
// ORDER BY
// us.Reputation DESC, us.TotalPosts DESC;
fn q13346(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 9], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + matches!(t, 4 | 5) as i64, a[5] + (t == 6) as i64, a[6] + w.is_some() as i64, a[7] + w.unwrap_or(0), a[8] + s]
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&a[..6]));
        f.extend([nullable(a[7], a[6]), V::I(a[8])]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
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
// TotalQuestions,
// TotalAnswers,
// TotalUpVotes,
// TotalDownVotes,
// TotalGoldBadges,
// TotalSilverBadges,
// TotalBronzeBadges
// FROM
// UserStats
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q13350(db: &'static So) -> String {
    let uf = g(db)
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let t = p.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (b == Some(1)) as i64, a[3] + (b == Some(2)) as i64, a[4] + (b == Some(3)) as i64]
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dq = ud(db, UserWhere::All, posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(1))));
    let da = ud(db, UserWhere::All, posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(2))));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dq).opt()).and((&da).opt()).drive(|u, (((a, p), q), x)| v.push((u, a, [p, q, x].map(|y| y.unwrap_or(0)))));
    out(v, |&(_, _, d)| Reverse(d[0]), 10, |&(u, a, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&d));
        f.extend(ints(&a));
        f
    })
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// COALESCE(VoteCounts.UpVotes, 0) AS UpVoteCount,
// COALESCE(VoteCounts.DownVotes, 0) AS DownVoteCount,
// COALESCE(BadgeCounts.BadgeCount, 0) AS UserBadgeCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId) VoteCounts ON P.Id = VoteCounts.PostId
// LEFT JOIN
// (SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId) BadgeCounts ON U.Id = BadgeCounts.UserId
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q13351(db: &'static So) -> String {
    let pv = post_votes(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(&bu))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, x), b)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "rep"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(b)]);
        f
    })
}

// WITH UserVoteCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId IN (2, 8) THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COALESCE(V.VoteCount, 0) AS UserVoteCount,
// COALESCE(V.UpVotes, 0) AS UserUpVotes,
// COALESCE(V.DownVotes, 0) AS UserDownVotes,
// COALESCE(C.CommentsCount, 0) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// UserVoteCounts V ON P.OwnerUserId = V.UserId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(Id) AS CommentsCount
// FROM
// Comments
// GROUP BY
// PostId
// ) C ON P.Id = C.PostId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.UserVoteCount,
// PS.UserUpVotes,
// PS.UserDownVotes,
// PS.CommentCount
// FROM
// PostStats PS
// ORDER BY
// PS.Score DESC,
// PS.ViewCount DESC
// LIMIT 100;
fn q13357(db: &'static So) -> String {
    let uv = g(db).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + matches!(t, 2 | 8) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&db.post.owner_user).select(&uv).opt()).and(comments_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| score_views(db, p), 100, |&((p, x), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(ints(&x.unwrap_or([0; 3])));
        f.push(V::I(c));
        f
    })
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COALESCE(V.VoteCount, 0) AS VoteCount,
// COALESCE(C.CommentCount, 0) AS CommentCount
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
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
// ORDER BY
// U.Reputation DESC,
// P.CreationDate DESC;
fn q13358(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(votes_per_post(db)).and(comments_per_post(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), c)| {
        let mut f = post_fields(db, p, &["uid", "owner", "rep", "id", "title", "created", "score", "views"]);
        f.extend([V::I(x), V::I(c)]);
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
// PostComments AS (
// SELECT
// p.Id AS PostId,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalUpVotes,
// ups.TotalDownVotes,
// COALESCE(pc.CommentCount, 0) AS CommentCount
// FROM
// UserPostStats ups
// LEFT JOIN
// PostComments pc ON ups.UserId = pc.PostId
// ORDER BY
// ups.DisplayName;
fn q13371(db: &'static So) -> String {
    let pid = pids(db);
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).and((&db.user.origid).select(&pid).select(comments_per_post(db)).opt()).drive(|u, (a, c)| v.push((u, a, c)));
    rows(v.iter().map(|&(u, a, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(V::I(c.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(c.Id, 0)) AS CommentCount
// FROM
// Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id
// )
// SELECT
// us.UserId,
// us.BadgeCount,
// us.PostCount,
// us.TotalViews,
// us.TotalScore,
// us.CommentCount
// FROM
// UserStats us
// ORDER BY
// us.TotalScore DESC,
// us.TotalViews DESC
// LIMIT 100;
fn q13390(db: &'static So) -> String {
    let uf = g(db)
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score).and(comments_of(db).select(&db.comment.origid).opt())).opt()))
        .fold([0i64; 4], |a, (b, p)| {
            let (w, s, c) = p.map_or((0, 0, 0), |((w, s), c)| (w.unwrap_or(0), s, c.unwrap_or(0)));
            [a[0] + b.is_some() as i64, a[1] + w, a[2] + s, a[3] + c]
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).drive(|u, (a, d)| v.push((u, a, d.unwrap_or(0))));
    out(v, |&(_, a, _)| (Reverse(a[2]), Reverse(a[1])), 100, |&(u, a, d)| vec![user_col(db, u, "uid"), V::I(a[0]), V::I(d), V::I(a[1]), V::I(a[2]), V::I(a[3])])
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
// GROUP BY p.Id, p.Title, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN p.Id IS NOT NULL THEN 1 ELSE 0 END) AS PostCount,
// SUM(b.Class) AS TotalBadgeClass,
// AVG(u.Reputation) AS AvgReputation
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
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
// us.DisplayName AS OwnerDisplayName,
// us.PostCount,
// us.TotalBadgeClass,
// us.AvgReputation
// FROM PostStats ps
// JOIN UserStats us ON ps.PostId = us.UserId
// ORDER BY ps.CreationDate DESC
// LIMIT 100;
fn q13407(db: &'static So) -> String {
    let uid = uids(db);
    let us = g(db).select((&db.user.reputation).and(posts_of(db).opt()).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 5], |a, ((r, p), b)| {
        [a[0] + p.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0), a[3] + r, a[4] + 1]
    });
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvb", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, ..)| newest(db, p), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[4])]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(CASE WHEN PostTypeId = 1 THEN Score ELSE NULL END) AS AvgQuestionScore
// FROM
// Posts
// ),
// AcceptedAnswers AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS AcceptedAnswerCount
// FROM
// Posts
// WHERE
// AcceptedAnswerId IS NOT NULL
// GROUP BY
// OwnerUserId
// )
// SELECT
// PS.TotalPosts,
// PS.AvgQuestionScore,
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// AA.AcceptedAnswerCount
// FROM
// PostStats PS
// CROSS JOIN
// Users U
// JOIN
// AcceptedAnswers AA ON U.Id = AA.OwnerUserId
// ORDER BY
// AA.AcceptedAnswerCount DESC
// LIMIT 5;
fn q13412(db: &'static So) -> String {
    let (n, s, q) = db.post.select((&db.post.post_type_id).and(&db.post.score)).fold_flat((0i64, 0i64, 0i64), |(n, s, q), (t, x)| (n + 1, s + if t == 1 { x } else { 0 }, q + (t == 1) as i64));
    let aa = owned(db).with(&db.post.accepted_answer_id).group_by(&db.post.owner_user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&aa).drive(|u, c| v.push((u, c)));
    out(v, |&(_, c)| Reverse(c), 5, |&(u, c)| vec![V::I(n), avg(s, q), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(c)])
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
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Votes
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation
// FROM
// Users
// ),
// BadgeStats AS (
// SELECT
// COUNT(*) AS TotalBadges
// FROM
// Badges
// )
// SELECT
// (SELECT TotalPosts FROM PostStats) AS TotalPosts,
// (SELECT TotalQuestions FROM PostStats) AS TotalQuestions,
// (SELECT TotalAnswers FROM PostStats) AS TotalAnswers,
// (SELECT TotalTagWikis FROM PostStats) AS TotalTagWikis,
// (SELECT TotalComments FROM CommentStats) AS TotalComments,
// (SELECT TotalVotes FROM VoteStats) AS TotalVotes,
// (SELECT TotalUpVotes FROM VoteStats) AS TotalUpVotes,
// (SELECT TotalDownVotes FROM VoteStats) AS TotalDownVotes,
// (SELECT TotalUsers FROM UserStats) AS TotalUsers,
// (SELECT AvgReputation FROM UserStats) AS AvgReputation,
// (SELECT TotalBadges FROM BadgeStats) AS TotalBadges;
fn q13414(db: &'static So) -> String {
    let p = db.post.select(&db.post.post_type_id).fold_flat([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64]);
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 2 | 3) as i64, a[2] + (t == 3) as i64]);
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let mut f = ints(&p);
    f.push(V::I(count(db.comment.iq())));
    f.extend(ints(&x));
    f.extend([V::I(un), avg(rs, un), V::I(count(db.badge.iq()))]);
    row(f)
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(uc.UpVoteCount, 0) AS UpVotes,
// COALESCE(dc.DownVoteCount, 0) AS DownVotes,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// p.OwnerUserId,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS UpVoteCount
// FROM
// Votes
// WHERE
// VoteTypeId = 2
// GROUP BY
// PostId
// ) uc ON p.Id = uc.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS DownVoteCount
// FROM
// Votes
// WHERE
// VoteTypeId = 3
// GROUP BY
// PostId
// ) dc ON p.Id = dc.PostId
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
// ParentId,
// COUNT(*) AS AnswerCount
// FROM
// Posts
// WHERE
// PostTypeId = 2
// GROUP BY
// ParentId
// ) a ON p.Id = a.ParentId
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// )
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AvgViewCount,
// SUM(Score) AS TotalScore,
// SUM(UpVotes) AS TotalUpVotes,
// SUM(DownVotes) AS TotalDownVotes,
// SUM(CommentCount) AS TotalComments,
// SUM(AnswerCount) AS TotalAnswers
// FROM
// PostStats;
fn q13415(db: &'static So) -> String {
    let a = db
        .post
        .select((&db.post.view_count).opt().and(&db.post.score).and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and(comments_per_post(db)).and(typed_answers_per_post(db)))
        .fold_flat([0i64; 8], |a, (((((w, s), u), d), c), an)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + u, a[5] + d, a[6] + c, a[7] + an]);
    let mut f = vec![V::I(a[0]), avg(a[2], a[1])];
    f.extend(ints(&a[3..]));
    row(f)
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
// UserVoteCounts AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// INNER JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// v.UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(upc.PostCount, 0) AS PostCount,
// COALESCE(upc.QuestionCount, 0) AS QuestionCount,
// COALESCE(upc.AnswerCount, 0) AS AnswerCount,
// COALESCE(uvc.VoteCount, 0) AS VoteCount,
// COALESCE(uvc.UpVotes, 0) AS UpVotes,
// COALESCE(uvc.DownVotes, 0) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// UserPostCounts upc ON u.Id = upc.UserId
// LEFT JOIN
// UserVoteCounts uvc ON u.Id = uvc.UserId
// ORDER BY
// u.Reputation DESC
// LIMIT 100;
fn q13423(db: &'static So) -> String {
    let vs = vote_named(db);
    let mut v = Vec::new();
    upqa(db).and((&vs).opt()).drive(|u, (a, x)| v.push((u, a, x.unwrap_or([0; 3]))));
    out(v, |&(u, _, _)| rep_desc(db, u), 100, |&(u, a, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend(ints(&x));
        f
    })
}

// WITH Benchmark AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT ph.Id) AS PostHistoryCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// AverageScore,
// AverageViewCount,
// CommentCount,
// PostHistoryCount
// FROM
// Benchmark
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q13424(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).opt()).and(history_of(db).opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((((t, s), w), _), _)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
        None => a,
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let dh = ud(db, UserWhere::All, posts_of(db).select(history_of(db)));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dc).opt()).and((&dh).opt()).drive(|u, (((a, p), c), h)| v.push((u, a, [p, c, h].map(|x| x.unwrap_or(0)))));
    out(v, |&(_, _, d)| Reverse(d[0]), 10, |&(u, a, d)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), avg(a[5], a[4]), V::I(d[1]), V::I(d[2])]
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
// u.Reputation AS OwnerReputation,
// COUNT(b.Id) AS BadgeCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON b.UserId = u.Id
// WHERE
// p.CreationDate >= DATE '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.Reputation
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS EditCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.OwnerReputation,
// ps.BadgeCount,
// phs.EditCount,
// phs.CloseCount
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.ViewCount DESC;
fn q13430(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + (t == 10) as i64]);
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and((&db.post.owner_user).select(&bu)).and((&hf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, b), h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "rep"]);
        f.extend([V::I(b), oint(h.map(|h| h[0])), oint(h.map(|h| h[1]))]);
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
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// ARRAY_AGG(DISTINCT t.TagName) AS Tags
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13433(db: &'static So) -> String {
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, children_of(db));
    let names: MatSet<(Id<Post>, Option<Str>)> = questions_only(db).select(Ident::<Post>::new().and((&ex).select(&db.tag.tag_name).opt())).collect();
    let lists = (&names).gather_by((&names).map(|(p, _)| p));
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and((&c).opt()).and((&a).opt()).and(&lists)).drive(|_, (((p, c), a), l)| v.push((p, c.unwrap_or(0), a.unwrap_or(0), l.iter().map(|&(_, n)| n).collect::<Vec<_>>())));
    out(v, |(p, ..)| newest(db, *p), 100, |(p, c, a, l)| {
        let mut f = post_fields(db, *p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(*c), V::I(*a), V::L(l.iter().map(|&n| ostr(n)).collect())]);
        f
    })
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS AuthorName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COALESCE(MAX(b.Date), DATE '1900-01-01') AS LastBadgeDate
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
// p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// )
// SELECT
// rp.PostId,
// rp.Title,
// rp.CreationDate,
// rp.Score,
// rp.ViewCount,
// rp.AuthorName,
// rp.CommentCount,
// rp.VoteCount,
// rp.LastBadgeDate,
// (SELECT COUNT(*) FROM Comments WHERE PostId = rp.PostId) AS TotalComments,
// (SELECT AVG(Score) FROM Posts WHERE OwnerUserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId)) AS AvgUserScore
// FROM
// RecentPosts rp
// ORDER BY
// rp.ViewCount DESC
// LIMIT 50;
fn q13434(db: &'static So) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as i64;
    let base = || db.post.with((&db.post.creation_date).ge(add_days(now, -30)));
    let pf = stats_fold(db, base(), Ident::<Post>::new(), "cvb", &[]);
    let us = g(db).select(posts_of(db).select(&db.post.score).opt()).fold((0i64, 0i64), |(n, s), x| match x {
        Some(x) => (n + 1, s + x),
        None => (n, s),
    });
    let mut v = Vec::new();
    (&pf).and(comments_per_post(db)).and((&db.post.owner_user).select(&us).opt()).drive(|p, ((s, c), a)| v.push((p, s, c, a)));
    out(v, |&(p, ..)| views_desc(db, p), 50, |&(p, s, c, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::T(if s.bx == 0 { date(1900, 1, 1) } else { s.bmax }), V::I(c), a.map_or(V::Null, |(n, s)| avg(s, n))]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.FavoriteCount,
// u.Reputation AS OwnerReputation
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY p.Id, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.FavoriteCount, u.Reputation
// ),
// AverageStats AS (
// SELECT
// AVG(Score) AS AvgScore,
// AVG(ViewCount) AS AvgViewCount,
// AVG(OwnerReputation) AS AvgOwnerReputation,
// AVG(AnswerCount) AS AvgAnswerCount,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(FavoriteCount) AS AvgFavoriteCount,
// SUM(UpVotes) AS TotalUpVotes,
// SUM(DownVotes) AS TotalDownVotes
// FROM PostStats
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
// ps.OwnerReputation,
// av.AvgScore,
// av.AvgViewCount,
// av.AvgOwnerReputation,
// av.AvgAnswerCount,
// av.AvgCommentCount,
// av.AvgFavoriteCount,
// av.TotalUpVotes,
// av.TotalDownVotes
// FROM PostStats ps
// CROSS JOIN AverageStats av
// ORDER BY ps.CreationDate DESC
// LIMIT 100;
fn q13437(db: &'static So) -> String {
    let Post { score, view_count, answer_count, favorite_count, .. } = &db.post;
    let c = per_post_distinct(db, comments_of(db));
    let pf = stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]);
    let t = owned(db)
        .select(score.and(view_count.opt()).and((&db.post.owner_user).select(&db.user.reputation)).and(answer_count.opt()).and((&c).opt()).and(favorite_count.opt()).and(&pf))
        .fold_flat([0i64; 12], |a, ((((((s, w), r), an), c), fc), st)| {
            [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + r, a[5] + an.is_some() as i64, a[6] + an.unwrap_or(0), a[7] + c.unwrap_or(0), a[8] + fc.is_some() as i64, a[9] + fc.unwrap_or(0), a[10] + st.up, a[11] + st.down]
        });
    let mut v = Vec::new();
    (&pf).and((&c).opt()).drive(|p, (_, c)| v.push((p, c.unwrap_or(0))));
    out(v, |&(p, _)| newest(db, p), 100, |&(p, c)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views", "answers"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["favorites", "rep"]));
        f.extend([avg(t[1], t[0]), avg(t[3], t[2]), avg(t[4], t[0]), avg(t[6], t[5]), avg(t[7], t[0]), avg(t[9], t[8]), V::I(t[10]), V::I(t[11])]);
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
// PostSummary AS (
// SELECT
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// ),
// UserReputation AS (
// SELECT
// AVG(Reputation) AS AvgReputation
// FROM
// Users
// )
// SELECT
// ps.TotalPosts,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.TotalComments,
// ur.AvgReputation,
// COUNT(upc.UserId) AS TotalUsers,
// SUM(upc.PostCount) AS TotalPostsByUsers,
// AVG(upc.PostCount) AS AvgPostsPerUser
// FROM
// PostSummary ps,
// UserReputation ur,
// UserPostCounts upc
// GROUP BY
// ps.TotalPosts,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.TotalComments,
// ur.AvgReputation;
fn q13443(db: &'static So) -> String {
    let ps = db.post.select((&db.post.post_type_id).and(comments_of(db).opt())).fold_flat([0i64; 4], |a, (t, c)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64]);
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let pc = g(db).select(posts_of(db).opt()).fold(0i64, |a, p| a + p.is_some() as i64);
    let (n, s) = (&pc).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mut f = ints(&ps);
    f.extend([avg(rs, un), V::I(n), V::I(s), avg(s, n)]);
    row(f)
}

// WITH PostSummary AS (
// SELECT
// p.OwnerUserId,
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(SUM(c.Score), 0) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.OwnerUserId, pt.Name
// ),
// UserSummary AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(SUM(ps.TotalPosts), 0) AS PostsCount,
// COALESCE(SUM(ps.TotalComments), 0) AS CommentsCount
// FROM
// Users u
// LEFT JOIN
// PostSummary ps ON u.Id = ps.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostsCount,
// us.CommentsCount,
// (us.PostsCount + us.CommentsCount) AS TotalInteractions
// FROM
// UserSummary us
// ORDER BY
// TotalInteractions DESC;
fn q13445(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select(comments_of(db).select(&db.comment.score).opt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(c) => [a[0] + 1, a[1] + c.unwrap_or(0)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[0] + a[1])])))
}

// WITH PostStats AS (
// SELECT
// Posts.Id AS PostId,
// Posts.PostTypeId,
// Posts.CreationDate,
// COUNT(Comments.Id) AS CommentCount,
// SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts
// LEFT JOIN
// Comments ON Comments.PostId = Posts.Id
// LEFT JOIN
// Votes ON Votes.PostId = Posts.Id
// GROUP BY
// Posts.Id, Posts.PostTypeId, Posts.CreationDate
// ),
// UserStats AS (
// SELECT
// Users.Id AS UserId,
// Users.Reputation,
// COUNT(DISTINCT Posts.Id) AS PostCount,
// COUNT(DISTINCT Badges.Id) AS BadgeCount
// FROM
// Users
// LEFT JOIN
// Posts ON Posts.OwnerUserId = Users.Id
// LEFT JOIN
// Badges ON Badges.UserId = Users.Id
// GROUP BY
// Users.Id, Users.Reputation
// )
// SELECT
// p.PostId,
// p.PostTypeId,
// p.CreationDate,
// p.CommentCount,
// p.UpVoteCount,
// p.DownVoteCount,
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.BadgeCount
// FROM
// PostStats p
// JOIN
// UserStats u ON p.PostTypeId = u.UserId
// ORDER BY
// p.CreationDate DESC;
fn q13448(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.post_type_id).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.post_type_id).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&bu)))
        .drive(|p, (s, ((u, d), b))| v.push((p, s, u, d.unwrap_or(0), b)));
    rows(v.iter().map(|&(p, s, u, d, b)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d), V::I(b)]);
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// SUM(c.CommentCount) AS TotalComments,
// MAX(p.LastActivityDate) AS LastActive,
// SUM(b.Class) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// PostCount DESC
// LIMIT 100;
fn q13461(db: &'static So) -> String {
    let Post { post_type_id, score, last_activity_date, .. } = &db.post;
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let w = UserWhere::RepGt(0);
    let uf = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(last_activity_date).and((&cf).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0, 0, 0, 0, 0, 0, i64::MIN, 0, 0], |a: [i64; 9], (p, b)| {
            let mut a = a;
            if let Some((((t, s), la), c)) = p {
                a[0] += 1;
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += s;
                a[4] += c.is_some() as i64;
                a[5] += c.unwrap_or(0);
                a[6] = a[6].max(la);
            }
            if let Some(c) = b {
                a[7] += 1;
                a[8] += c;
            }
            a
        });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| Reverse(a[0]), 100, |&(u, a)| {
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            nullable(a[3], a[0]),
            nullable(a[5], a[4]),
            if a[0] == 0 { V::Null } else { V::T(a[6]) },
            nullable(a[8], a[7]),
        ]
    })
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN p.Id IS NOT NULL THEN 1 ELSE 0 END) AS PostCount,
// AVG(u.Reputation) AS AvgReputation
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.UpVotes,
// us.DownVotes,
// us.PostCount,
// us.AvgReputation
// FROM
// UserStatistics us
// ORDER BY
// us.PostCount DESC,
// us.AvgReputation DESC
// LIMIT 100;
fn q13464(db: &'static So) -> String {
    let uf = g(db).select((&db.user.reputation).and(badges_of(db).opt()).and(votes_by(db).select(&db.vote.vote_type_id).opt()).and(posts_of(db).opt())).fold([0i64; 6], |a, (((r, b), t), p)| {
        [a[0] + b.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + p.is_some() as i64, a[4] + r, a[5] + 1]
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(u, a)| (Reverse(a[3]), rep_desc(db, u)), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.push(avg(a[4], a[5]));
        f
    })
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners,
// SUM(COALESCE(ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(Score, 0)) AS TotalScore
// FROM
// Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// SUM(Reputation) AS TotalReputation
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
// ),
// BadgeStats AS (
// SELECT
// COUNT(*) AS TotalBadges,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS TotalGoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS TotalSilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS TotalBronzeBadges
// FROM
// Badges
// ),
// RecentPost AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS Author,
// P.CreationDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// ORDER BY
// P.CreationDate DESC
// LIMIT 1
// )
// SELECT
// PS.TotalPosts,
// PS.UniquePostOwners,
// PS.TotalViews,
// PS.TotalScore,
// US.TotalUsers,
// US.TotalReputation,
// VS.TotalVotes,
// VS.TotalUpVotes,
// VS.TotalDownVotes,
// BS.TotalBadges,
// BS.TotalGoldBadges,
// BS.TotalSilverBadges,
// BS.TotalBronzeBadges,
// RP.PostId,
// RP.Title AS RecentPostTitle,
// RP.Author AS RecentAuthor,
// RP.CreationDate AS RecentPostDate
// FROM
// PostStats PS,
// UserStats US,
// VoteStats VS,
// BadgeStats BS,
// RecentPost RP;
fn q13466(db: &'static So) -> String {
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let t = post_totals(db);
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let b = db.badge.select(&db.badge.class).fold_flat([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let mut v = Vec::new();
    owned(db).drive(|_, p| v.push(p));
    out(v, |&p| newest(db, p), 1, |&p| {
        let mut f = vec![V::I(t[0]), V::I(owners), V::I(t[3]), V::I(t[1]), V::I(un), V::I(rs)];
        f.extend(ints(&x));
        f.extend(ints(&b));
        f.extend(post_fields(db, p, &["id", "title", "owner", "created"]));
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT bh.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges bh ON p.OwnerUserId = bh.UserId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id
// ),
// UserStatistics AS (
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
// ps.PostId,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// us.UserId,
// us.PostCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostStatistics ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStatistics us ON u.Id = us.UserId
// ORDER BY
// ps.CommentCount DESC, ps.VoteCount DESC;
fn q13471(db: &'static So) -> String {
    let uid = uids(db);
    let x = per_post_distinct(db, votes_of(db));
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(posts_of(db).opt())).fold([0i64; 2], |a, ((u, d), _)| [a[0] + u, a[1] + d]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvb", &[])
        .and((&x).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, ((s, x), ((u, a), d))| v.push((p, s, x.unwrap_or(0), u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, x, u, a, d)| {
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(s.cx), V::I(x), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), V::I(d), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH PostCounts AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT v.UserId) AS TotalVotes,
// COUNT(DISTINCT u.Id) AS TotalUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// TotalVotes,
// TotalUsers,
// TotalVotes * 1.0 / NULLIF(TotalPosts, 0) AS VotesPerPost,
// TotalUsers * 1.0 / NULLIF(TotalPosts, 0) AS UsersPerPost
// FROM
// PostCounts
// ORDER BY
// TotalPosts DESC;
fn q13475(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), votes_of(db).opt(), 0i64, |a, _| a + 1);
    let dv = db.post.group_by(name(db)).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let du = db.post.group_by(name(db)).select(&db.post.owner_user).count_distinct();
    let mut v = Vec::new();
    (&f).and((&dv).opt()).and((&du).opt()).drive(|k, ((n, x), u)| v.push((k, n, x.unwrap_or(0), u.unwrap_or(0))));
    rows(v.iter().map(|&(k, n, x, u)| row(vec![V::S(k), V::I(n), V::I(x), V::I(u), ratio(x, n), ratio(u, n)])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM Posts p
// LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON p.OwnerUserId = b.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.TotalVotes,
// us.UpVotes,
// us.DownVotes,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.BadgeCount
// FROM UserStats us
// LEFT JOIN PostStats ps ON us.UserId = ps.PostId
// ORDER BY us.Reputation DESC, us.TotalVotes DESC
// LIMIT 100;
fn q13480(db: &'static So) -> String {
    let pid = pids(db);
    let uf = g(db).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 4], |a, p| match p {
        Some(t) => [a[0] + 1, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
        None => a,
    });
    let bu = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&uf).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.owner_user).select(&bu).opt())).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a[1])), 100, |&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        match p {
            Some(((p, c), b)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
                f.extend([V::I(c), V::I(b.unwrap_or(0))]);
            }
            None => f.extend(nulls(7)),
        }
        f
    })
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AvgPostScore,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners
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
// AVG(Reputation) AS AvgUserReputation
// FROM Users
// )
// SELECT
// p.TotalPosts,
// p.AvgPostScore,
// p.UniquePostOwners,
// c.TotalComments,
// c.AvgCommentScore,
// u.TotalUsers,
// u.AvgUserReputation
// FROM
// PostStats p,
// CommentStats c,
// UserStats u;
fn q13481(db: &'static So) -> String {
    let t = post_totals(db);
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let (cn, cs) = db.comment.select(&db.comment.score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    row(vec![V::I(t[0]), avg(t[1], t[0]), V::I(owners), V::I(cn), avg(cs, cn), V::I(un), avg(rs, un)])
}

// WITH UserPostCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.Score) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id
// ),
// UserBadges AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// FinalMetrics AS (
// SELECT
// UPC.UserId,
// UPC.PostCount,
// UPC.Questions,
// UPC.Answers,
// UPC.TotalScore,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount
// FROM
// UserPostCounts UPC
// LEFT JOIN
// UserBadges UB ON UPC.UserId = UB.UserId
// )
// SELECT
// UserId,
// PostCount,
// Questions,
// Answers,
// TotalScore,
// BadgeCount,
// (PostCount + BadgeCount) AS TotalMetrics
// FROM
// FinalMetrics
// ORDER BY
// TotalMetrics DESC;
fn q13492(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    upqa(db).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[5], a[0]), V::I(b), V::I(a[0] + b)]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(P.Score) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// AVG(P.ViewCount) AS AvgViewsPerPost,
// COUNT(COALESCE(C.Id, NULL)) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.Reputation
// )
// SELECT
// UserId,
// Reputation,
// TotalPosts,
// Questions,
// Answers,
// TotalScore,
// TotalViews,
// AvgViewsPerPost,
// TotalComments
// FROM
// UserPostStats
// ORDER BY
// Reputation DESC, TotalPosts DESC;
fn q13494(db: &'static So) -> String {
    rows(users_with_counts(db, "c", false).iter().map(|r| row(user_fields(r, "c", &["uid", "rep", "#rows", "#q", "#a", "score_sum", "views_sum0", "views_avg", "#cx"]))))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("13213", q13213),
    ("13216", q13216),
    ("13218", q13218),
    ("13219", q13219),
    ("13223", q13223),
    ("13244", q13244),
    ("13251", q13251),
    ("13259", q13259),
    ("13268", q13268),
    ("13272", q13272),
    ("13276", q13276),
    ("13283", q13283),
    ("13285", q13285),
    ("13286", q13286),
    ("13295", q13295),
    ("13296", q13296),
    ("13303", q13303),
    ("13308", q13308),
    ("13312", q13312),
    ("13321", q13321),
    ("13327", q13327),
    ("13334", q13334),
    ("13341", q13341),
    ("13346", q13346),
    ("13350", q13350),
    ("13351", q13351),
    ("13357", q13357),
    ("13358", q13358),
    ("13371", q13371),
    ("13390", q13390),
    ("13407", q13407),
    ("13412", q13412),
    ("13414", q13414),
    ("13415", q13415),
    ("13423", q13423),
    ("13424", q13424),
    ("13430", q13430),
    ("13433", q13433),
    ("13434", q13434),
    ("13437", q13437),
    ("13443", q13443),
    ("13445", q13445),
    ("13448", q13448),
    ("13461", q13461),
    ("13464", q13464),
    ("13466", q13466),
    ("13471", q13471),
    ("13475", q13475),
    ("13480", q13480),
    ("13481", q13481),
    ("13492", q13492),
    ("13494", q13494),
];
