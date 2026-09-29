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

fn split_n(s: Str, sep: &str) -> i64 {
    let n = s.chars().count();
    let inner: String = s.chars().skip(1).take(n.saturating_sub(2)).collect();
    inner.split(sep).count() as i64
}

fn ubc(db: &'static So) -> Fold<Id<User>, [i64; 4]> {
    g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    })
}

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

const Z: [i64; 13] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, i64::MIN, 0, 0];

fn pstat<Q: Drive<D = Id<Post>, R = Id<Post>>>(db: &'static So, base: Q) -> Fold<Id<User>, [i64; 13]> {
    let Post { post_type_id, score, view_count, answer_count, creation_date, closed_date, .. } = &db.post;
    base.with(&db.post.owner_user)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(score).and(view_count.opt()).and(answer_count.opt()).and(creation_date).and(closed_date.opt()))
        .fold(Z, |a: [i64; 13], (((((t, s), w), an), c), cl)| {
            [
                a[0] + 1,
                a[1] + (t == 1) as i64,
                a[2] + (t == 2) as i64,
                a[3] + s,
                a[4] + w.is_some() as i64,
                a[5] + w.unwrap_or(0),
                a[6] + an.is_some() as i64,
                a[7] + an.unwrap_or(0),
                a[8] + (t == 3) as i64,
                a[9] + matches!(t, 4 | 5) as i64,
                a[10].max(c),
                a[11] + cl.is_some() as i64,
                a[12] + (t == 4) as i64,
            ]
        })
}

fn pviews(p: [i64; 13]) -> V {
    nullable(p[5], p[4])
}

fn pscore_avg(p: [i64; 13]) -> V {
    avg(p[3], p[0])
}

fn pviews_avg(p: [i64; 13]) -> V {
    avg(p[5], p[4])
}

fn onull(p: Option<[i64; 13]>, f: impl Fn([i64; 13]) -> V) -> V {
    p.map_or(V::Null, f)
}

// --- batch 131 --------------------------------------------------------------

// WITH UserBadgeStats AS (
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
// PostActivity AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis,
// AVG(p.Score) AS AvgScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate,
// COALESCE(ubs.BadgeCount, 0) AS TotalBadges,
// COALESCE(ubs.GoldBadges, 0) AS GoldBadges,
// COALESCE(ubs.SilverBadges, 0) AS SilverBadges,
// COALESCE(ubs.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(pa.TotalPosts, 0) AS TotalPosts,
// COALESCE(pa.Questions, 0) AS TotalQuestions,
// COALESCE(pa.Answers, 0) AS TotalAnswers,
// COALESCE(pa.TagWikis, 0) AS TotalTagWikis,
// COALESCE(pa.AvgScore, 0) AS AvgPostScore
// FROM
// Users u
// LEFT JOIN
// UserBadgeStats ubs ON u.Id = ubs.UserId
// LEFT JOIN
// PostActivity pa ON u.Id = pa.OwnerUserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// u.Reputation DESC, u.DisplayName;
fn q9356(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and((&ps).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let q = p.unwrap_or(Z);
        let mut f: Vec<V> = ["uid", "name", "rep", "ucreated", "last_access"].iter().map(|c| user_col(db, u, c)).collect();
        f.extend(ints(&b));
        f.extend(ints(&[q[0], q[1], q[2], q[9]]));
        f.push(p.map_or(V::F(0.0), pscore_avg));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ), UserPostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ), CombinedStats AS (
// SELECT
// ubc.UserId,
// ubc.DisplayName,
// ubc.BadgeCount,
// ubc.GoldCount,
// ubc.SilverCount,
// ubc.BronzeCount,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.TotalScore,
// ups.AvgViewCount
// FROM
// UserBadgeCounts ubc
// JOIN
// UserPostStats ups ON ubc.UserId = ups.OwnerUserId
// )
// SELECT
// DisplayName,
// BadgeCount,
// GoldCount,
// SilverCount,
// BronzeCount,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore,
// AvgViewCount
// FROM
// CombinedStats
// WHERE
// BadgeCount > 0
// ORDER BY
// TotalScore DESC, BadgeCount DESC
// LIMIT 10;
fn q9412(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).filt(|b: [i64; 4]| b[0] > 0).and(&ps).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p[..4]));
        f.push(pviews_avg(p));
        f
    })
}

// WITH PopularPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerName,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId IN (1, 2) AND
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score, u.DisplayName
// HAVING
// COUNT(c.Id) > 5
// ),
// TopTags AS (
// SELECT
// t.TagName,
// COUNT(pt.Id) AS PostCount
// FROM
// Tags t
// JOIN
// Posts pt ON pt.Tags LIKE CONCAT('%', t.TagName, '%')
// WHERE
// pt.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// t.TagName
// ORDER BY
// PostCount DESC
// LIMIT 10
// )
// SELECT
// pp.PostId,
// pp.Title,
// pp.ViewCount,
// pp.Score,
// pp.OwnerName,
// pp.CommentCount,
// tt.TagName
// FROM
// PopularPosts pp
// JOIN
// PostLinks pl ON pp.PostId = pl.PostId
// JOIN
// TopTags tt ON pl.RelatedPostId IN (SELECT Id FROM Posts WHERE Tags LIKE CONCAT('%', tt.TagName, '%'))
// ORDER BY
// pp.Score DESC, pp.ViewCount DESC;
fn q9428(db: &'static So) -> String {
    let y = year_ago();
    let tm = tag_mentions(db);
    let recent = Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p).select(&db.post.creation_date).ge(y);
    let tc = (&tm).with(recent).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t)).fold(0i64, |a, _| a + 1);
    let top: MatSet<Id<Tag>> = whole(&tc).select(Same::new().and(&tc)).window(row_number, |(_, n): (Id<Tag>, i64)| n, desc).filt(|(_, n)| n <= 10).map(|((t, _), _)| t).collect();
    let tops = (&tm).with(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t).select(&top));
    let rel: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = tops.map(|(p, _)| p).inv().collect();
    let pp = since(db, y).with(&db.post.owner_user).with((&db.post.post_type_id).in_v(vec![1, 2])).with(comments_per_post(db).filt(|c: i64| c > 5));
    let mut v = Vec::new();
    pp.select(Ident::<Post>::new().and(comments_per_post(db)).and(links_of(db).select((&db.post_link.related_post).select(&rel)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), (_, t))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        f.extend([V::I(c), V::S(db.tag.tag_name.get(t).unwrap())]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
// COUNT(v.Id) AS TotalVotesCount
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostsCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// BadgeSummary AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgesCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// us.UpVotesCount,
// us.DownVotesCount,
// us.TotalVotesCount,
// ps.PostsCount,
// ps.TotalScore,
// ps.TotalViews,
// ps.AvgScore,
// ps.AvgViews,
// bs.BadgesCount,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges
// FROM
// Users u
// LEFT JOIN
// UserVoteSummary us ON u.Id = us.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// LEFT JOIN
// BadgeSummary bs ON u.Id = bs.UserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// u.Reputation DESC, us.UpVotesCount DESC
// LIMIT 100;
fn q9450(db: &'static So) -> String {
    let uv = user_votes(db);
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&uv).and((&ps).opt()).and((&bc).opt())).drive(|_, x| v.push(x));
    out(v, |&(((u, x), _), _)| (rep_desc(db, u), Reverse(x[1])), 100, |&(((u, x), p), b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(x[1]), V::I(x[2]), V::I(x[0])];
        f.extend([oint(p.map(|p| p[0])), oint(p.map(|p| p[3])), oint(p.map(|p| p[5])), onull(p, pscore_avg), onull(p, pviews_avg)]);
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f
    })
}

// WITH UserReputation AS (
// SELECT Id, Reputation, UpVotes, DownVotes,
// (UpVotes - DownVotes) AS NetVotes,
// CreationDate AS AccountAge
// FROM Users
// ),
// PostStats AS (
// SELECT OwnerUserId, COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(ViewCount) AS TotalViews,
// SUM(Score) AS TotalScore
// FROM Posts
// GROUP BY OwnerUserId
// ),
// BadgeCounts AS (
// SELECT UserId, COUNT(*) AS TotalBadges,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// )
// SELECT u.DisplayName, ur.Reputation, ur.NetVotes, ur.AccountAge,
// ps.TotalPosts, ps.Questions, ps.Answers, ps.TotalViews, ps.TotalScore,
// COALESCE(bc.TotalBadges, 0) AS TotalBadges,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges
// FROM UserReputation ur
// JOIN PostStats ps ON ur.Id = ps.OwnerUserId
// LEFT JOIN BadgeCounts bc ON ur.Id = bc.UserId
// JOIN Users u ON ur.Id = u.Id
// WHERE ur.Reputation > 1000
// ORDER BY ur.Reputation DESC, ps.TotalViews DESC
// LIMIT 50;
fn q9575(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ps).and((&bc).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, p), _)| (rep_desc(db, u), (p[4] == 0, Reverse(p[5]))), 50, |&((u, p), b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(db.user.up_votes.get(u).unwrap() - db.user.down_votes.get(u).unwrap()), user_col(db, u, "ucreated")];
        f.extend(ints(&p[..3]));
        f.extend([pviews(p), V::I(p[3])]);
        f.extend(ints(&b.unwrap_or([0; 4])));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// UserPerformances AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.TotalScore, 0) AS TotalScore
// FROM Users u
// LEFT JOIN UserBadgeCounts bc ON u.Id = bc.UserId
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// DisplayName,
// Reputation,
// BadgeCount,
// TotalPosts,
// QuestionCount,
// AnswerCount,
// TotalViews,
// TotalScore,
// (TotalScore / NULLIF(TotalPosts, 0)) AS AverageScorePerPost
// FROM UserPerformances
// WHERE Reputation > 1000
// ORDER BY TotalScore DESC, BadgeCount DESC
// LIMIT 50;
fn q9590(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b)), 50, |&(u, b, p)| {
        vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[5]), V::I(p[3]), ratio(p[3], p[0])]
    })
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
// SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
// AVG(P.Score) AS AverageScore,
// SUM(COALESCE(UPV.VoteCount, 0)) AS TotalUpVotes,
// SUM(COALESCE(DPV.VoteCount, 0)) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// WHERE VoteTypeId = 2
// GROUP BY PostId
// ) UPV ON P.Id = UPV.PostId
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// WHERE VoteTypeId = 3
// GROUP BY PostId
// ) DPV ON P.Id = DPV.PostId
// GROUP BY
// U.Id, U.DisplayName
// ), UserBadgeCounts AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// ), UserContributions AS (
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.PositivePosts,
// UPS.NegativePosts,
// UPS.AverageScore,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// UPS.TotalUpVotes,
// UPS.TotalDownVotes
// FROM
// UserPostStats UPS
// LEFT JOIN
// UserBadgeCounts UBC ON UPS.UserId = UBC.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// PositivePosts,
// NegativePosts,
// AverageScore,
// BadgeCount,
// TotalUpVotes,
// TotalDownVotes
// FROM
// UserContributions
// WHERE
// TotalPosts > 10
// ORDER BY
// AverageScore DESC, TotalPosts DESC;
fn q9675(db: &'static So) -> String {
    let Post { score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(score.and(votes_of_type(db, 2)).and(votes_of_type(db, 3))).opt()).fold([0i64; 6], |a, p| match p {
        Some(((s, u), d)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + s, a[4] + u, a[5] + d],
        None => a,
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 6]| a[0] > 10).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(b), V::I(a[4]), V::I(a[5])])
    }))
}

// SELECT
// u.DisplayName AS UserDisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(u.Reputation) AS AvgReputation,
// COUNT(b.Id) AS TotalBadges,
// (SELECT COUNT(*) FROM Votes v WHERE v.UserId = u.Id AND v.VoteTypeId = 2) AS UpvotesGiven,
// (SELECT COUNT(*) FROM Votes v WHERE v.UserId = u.Id AND v.VoteTypeId = 3) AS DownvotesGiven
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// HAVING
// COUNT(DISTINCT p.Id) > 10
// ORDER BY
// TotalPosts DESC
// LIMIT 20;
fn q9724(db: &'static So) -> String {
    let base = db.user.with((&db.user.creation_date).gt(year_ago()));
    let uf = base.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).opt())).fold([0i64; 3], |a, (t, b)| {
        [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.is_some() as i64]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let vt = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let mut v = Vec::new();
    (&uf).and((&dp).filt(|d: i64| d > 10)).and((&vt).opt()).drive(|u, ((a, d), x)| v.push((u, a, d, x.unwrap_or([0; 2]))));
    out(v, |&(_, _, d, _)| Reverse(d), 20, |&(u, a, d, x)| {
        vec![user_col(db, u, "name"), V::I(d), V::I(a[0]), V::I(a[1]), V::F(db.user.reputation.get(u).unwrap() as f64), V::I(a[2]), V::I(x[0]), V::I(x[1])]
    })
}

// WITH UserBadges AS (
// SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AvgViewCount,
// MAX(P.CreationDate) AS LastPostDate
// FROM Posts P
// WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY P.OwnerUserId
// ),
// ActiveUsers AS (
// SELECT
// U.Id,
// U.DisplayName,
// UB.BadgeCount,
// PS.PostCount,
// PS.TotalScore,
// PS.AvgViewCount,
// PS.LastPostDate
// FROM Users U
// JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// WHERE U.Reputation > 100
// )
// SELECT
// A.Id AS UserId,
// A.DisplayName,
// A.BadgeCount,
// COALESCE(A.PostCount, 0) AS PostCount,
// COALESCE(A.TotalScore, 0) AS TotalScore,
// COALESCE(A.AvgViewCount, 0) AS AvgViewCount,
// A.LastPostDate
// FROM ActiveUsers A
// ORDER BY A.BadgeCount DESC, A.TotalScore DESC
// LIMIT 20;
fn q9743(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((_, b), p)| (Reverse(b), (p.is_none(), Reverse(p.map(|p| p[3])))), 20, |&((u, b), p)| {
        let q = p.unwrap_or(Z);
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), V::I(q[0]), V::I(q[3]), if q[4] > 0 { pviews_avg(q) } else { V::F(0.0) }, ots(p.map(|p| p[10]))]
    })
}

// WITH UserReputation AS (
// SELECT Id AS UserId, Reputation, CreationDate
// FROM Users
// WHERE Reputation > 1000
// ),
// ActivePosts AS (
// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId
// FROM Posts p
// INNER JOIN UserReputation ur ON p.OwnerUserId = ur.UserId
// WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1
// ),
// TopTaggedPosts AS (
// SELECT ap.PostId, SUM(t.Count) AS TagCount
// FROM ActivePosts ap
// JOIN Tags t ON t.WikiPostId = ap.PostId
// GROUP BY ap.PostId
// ORDER BY TagCount DESC
// LIMIT 10
// ),
// PostComments AS (
// SELECT c.PostId, COUNT(c.Id) AS CommentCount
// FROM Comments c
// JOIN TopTaggedPosts ttp ON c.PostId = ttp.PostId
// GROUP BY c.PostId
// ),
// FinalResults AS (
// SELECT ap.PostId, ap.Title, ap.CreationDate, ap.Score, ap.ViewCount, pc.CommentCount, ap.OwnerUserId
// FROM ActivePosts ap
// LEFT JOIN PostComments pc ON ap.PostId = pc.PostId
// )
// SELECT
// fr.PostId,
// fr.Title,
// fr.CreationDate AS PostCreationDate,
// fr.Score,
// fr.ViewCount,
// COALESCE(fr.CommentCount, 0) AS TotalComments,
// ur.Reputation AS OwnerReputation
// FROM FinalResults fr
// JOIN UserReputation ur ON fr.OwnerUserId = ur.UserId
// ORDER BY fr.ViewCount DESC, fr.Score DESC
// LIMIT 20;
fn q9745(db: &'static So) -> String {
    let ap = || owned_since(db, year_ago()).with((&db.post.post_type_id).eq(1)).with((&db.post.owner_user).select(&db.user.reputation).gt(1000));
    let wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let tt = ap().group_by(Ident::<Post>::new()).select((&wiki).select(&db.tag.count)).fold(0i64, |a, c| a + c);
    let top: MatSet<Id<Post>> = whole(&tt).select(Same::new().and(&tt)).window(row_number, |(_, s): (Id<Post>, i64)| s, desc).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let pc = (&top).select(Ident::<Post>::new().and(comments_per_post(db))).map(|(_, c)| c);
    let pcf: Fold<Id<Post>, i64> = (&top).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |a, _| a + 1);
    let _ = pc;
    let mut v = Vec::new();
    ap().select(Ident::<Post>::new().and((&pcf).opt())).drive(|_, x| v.push(x));
    out(v, |&(p, _)| views_score(db, p), 20, |&(p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c.unwrap_or(0)), post_fields(db, p, &["rep"]).pop().unwrap()]);
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
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(P.Score) AS AverageScore,
// SUM(P.ViewCount) as TotalViews
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// EngagedUsers AS (
// SELECT
// U.Id,
// U.DisplayName,
// U.Reputation,
// UB.BadgeCount,
// PS.TotalPosts,
// PS.Questions,
// PS.Answers,
// PS.AverageScore,
// PS.TotalViews
// FROM Users U
// JOIN UserBadges UB ON U.Id = UB.UserId
// JOIN PostStats PS ON U.Id = PS.OwnerUserId
// WHERE U.Reputation > 1000
// )
// SELECT
// EU.DisplayName,
// EU.Reputation,
// EU.BadgeCount,
// EU.TotalPosts,
// EU.Questions,
// EU.Answers,
// EU.AverageScore,
// EU.TotalViews
// FROM EngagedUsers EU
// ORDER BY EU.Reputation DESC, EU.BadgeCount DESC
// LIMIT 10;
fn q9877(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and(&ps)).drive(|_, x| v.push(x));
    out(v, |&((u, b), _)| (rep_desc(db, u), Reverse(b)), 10, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        f.extend(ints(&p[..3]));
        f.extend([pscore_avg(p), pviews(p)]);
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViews,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// ActiveUsers AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalScore,
// us.AvgViews,
// us.LastPostDate
// FROM
// UserStatistics us
// WHERE
// us.TotalPosts > 5 AND
// us.TotalScore > 100
// ),
// RecentActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(c.Id) AS TotalComments,
// MAX(c.CreationDate) AS LastCommentDate
// FROM
// Users u
// LEFT JOIN
// Comments c ON u.Id = c.UserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// au.UserId,
// au.DisplayName,
// au.TotalPosts,
// au.TotalQuestions,
// au.TotalAnswers,
// au.TotalScore,
// au.AvgViews,
// ra.TotalComments,
// ra.LastCommentDate,
// au.LastPostDate
// FROM
// ActiveUsers au
// LEFT JOIN
// RecentActivity ra ON au.UserId = ra.UserId
// ORDER BY
// au.TotalScore DESC, au.LastPostDate DESC;
fn q9943(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let ra = g(db).select(comments_by(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 5 && p[3] > 100).and((&ra).opt()).drive(|u, (p, r)| v.push((u, p, r)));
    rows(v.iter().map(|&(u, p, r)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p[..4]));
        f.push(pviews_avg(p));
        f.extend([oint(r.map(|r| r.0)), ots(r.filter(|r| r.0 > 0).map(|r| r.1)), V::T(p[10])]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// MAX(p.CreationDate) AS LatestPostDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// u.Reputation > 1000
// GROUP BY
// p.OwnerUserId
// ),
// BadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ),
// FinalStats AS (
// SELECT
// ps.OwnerUserId,
// ps.QuestionCount,
// ps.AnswerCount,
// ps.TotalScore,
// ps.TotalViews,
// ps.LatestPostDate,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount
// FROM
// PostStats ps
// LEFT JOIN
// BadgeCounts bc ON ps.OwnerUserId = bc.UserId
// )
// SELECT
// u.DisplayName,
// fs.QuestionCount,
// fs.AnswerCount,
// fs.TotalScore,
// fs.TotalViews,
// fs.LatestPostDate,
// fs.BadgeCount
// FROM
// FinalStats fs
// JOIN
// Users u ON fs.OwnerUserId = u.Id
// ORDER BY
// fs.TotalScore DESC, fs.QuestionCount DESC
// LIMIT 10;
fn q9998(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.with((&db.post.owner_user).select(&db.user.reputation).gt(1000)));
    let mut v = Vec::new();
    (&ps).and(&bu).drive(|u, (p, b)| v.push((u, p, b)));
    out(v, |&(_, p, _)| (Reverse(p[3]), Reverse(p[1])), 10, |&(u, p, b)| vec![user_col(db, u, "name"), V::I(p[1]), V::I(p[2]), V::I(p[3]), pviews(p), V::T(p[10]), V::I(b)])
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
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT p.Tags) AS UniqueTags
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotes,
// us.DownVotes,
// bs.BadgeCount,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges,
// ps.TotalPosts,
// ps.AverageScore,
// ps.TotalViews,
// ps.UniqueTags
// FROM
// UserStats us
// LEFT JOIN
// BadgeStats bs ON us.UserId = bs.UserId
// LEFT JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// ORDER BY
// us.PostCount DESC, us.UpVotes DESC;
fn q14519(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let dt = owned(db).group_by(&db.post.owner_user).select(&db.post.tags_str).count_distinct();
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).and((&ps).opt()).and((&dt).opt()).drive(|u, ((((a, d), b), p), t)| v.push((u, a, d.unwrap_or(0), b, p, t)));
    rows(v.iter().map(|&(u, a, d, b, p, t)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down)];
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend([oint(p.map(|p| p[0])), onull(p, pscore_avg), onull(p, pviews), if p.is_some() { V::I(t.unwrap_or(0)) } else { V::Null }]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName AS UserName,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// COALESCE(SUM(P.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(P.Score), 0) AS TotalScore,
// COALESCE(AVG(P.ViewCount), 0) AS AverageViewsPerPost,
// COALESCE(AVG(P.Score), 0) AS AverageScorePerPost
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
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// UserActivity AS (
// SELECT
// PS.UserId,
// PS.UserName,
// PS.TotalPosts,
// PS.TotalQuestions,
// PS.TotalAnswers,
// PS.TotalViews,
// PS.TotalScore,
// PS.AverageViewsPerPost,
// PS.AverageScorePerPost,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges
// FROM
// UserPostStats PS
// JOIN
// UserBadges UB ON PS.UserId = UB.UserId
// )
// SELECT
// UserId,
// UserName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalViews,
// AverageViewsPerPost,
// TotalScore,
// AverageScorePerPost,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// UserActivity
// WHERE
// TotalPosts > 50
// ORDER BY
// TotalScore DESC,
// TotalViews DESC
// FETCH FIRST 10 ROWS ONLY;
fn q28321(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 50).and(&bc).drive(|u, (p, b)| v.push((u, p, b)));
    out(v, |&(_, p, _)| (Reverse(p[3]), Reverse(p[5])), 10, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[5]]));
        f.extend([if p[4] > 0 { pviews_avg(p) } else { V::F(0.0) }, V::I(p[3]), pscore_avg(p)]);
        f.extend(ints(&b[1..]));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AvgViewCount
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS Questions,
// COALESCE(PS.Answers, 0) AS Answers,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.AvgViewCount, 0.0) AS AvgViewCount
// FROM Users U
// LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// Questions,
// Answers,
// TotalScore,
// AvgViewCount
// FROM CombinedStats
// ORDER BY TotalScore DESC, TotalPosts DESC
// LIMIT 10;
fn q6767(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, _, p)| (Reverse(p.map_or(0, |p| p[3])), Reverse(p.map_or(0, |p| p[0]))), 10, |&(u, b, p)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&q[..4]));
        f.push(if q[4] > 0 { pviews_avg(q) } else { V::F(0.0) });
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// PostVoteStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.OwnerUserId
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(p.Score) AS AvgScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// ub.BadgeCount,
// ub.GoldCount,
// ub.SilverCount,
// ub.BronzeCount,
// ps.TotalPosts,
// ps.QuestionCount,
// ps.AnswerCount,
// pvs.TotalVotes,
// pvs.UpVotes,
// pvs.DownVotes,
// COALESCE(pvs.TotalVotes, 0) + COALESCE(ps.QuestionCount * 2, 0) AS EngagementScore
// FROM
// Users u
// LEFT JOIN
// UserBadgeCounts ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// LEFT JOIN
// PostVoteStats pvs ON u.Id = pvs.OwnerUserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// EngagementScore DESC,
// u.Reputation DESC
// LIMIT 10;
fn q5637(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let pvs = owned(db).group_by(&db.post.owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let eng = |p: Option<[i64; 13]>, x: Option<[i64; 3]>| x.map_or(0, |x| x[0]) + p.map_or(0, |p| p[1] * 2);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and((&ps).opt()).and((&pvs).opt())).drive(|_, x| v.push(x));
    out(v, |&(((u, _), p), x)| (Reverse(eng(p, x)), rep_desc(db, u)), 10, |&(((u, b), p), x)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f.push(V::I(eng(p, x)));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(v.Id) AS TotalVotes,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1.0 ELSE 0.0 END) AS AvgUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1.0 ELSE 0.0 END) AS AvgDownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
// p.FavoriteCount, p.CreationDate, u.DisplayName
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// COUNT(b.Id) AS TotalBadges
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
// ps.PostId,
// ps.Title,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.CreationDate,
// ps.OwnerDisplayName,
// ps.TotalVotes,
// ps.AvgUpVotes,
// ps.AvgDownVotes,
// us.UserId,
// us.DisplayName AS UserDisplayName,
// us.TotalPosts,
// us.TotalUpVotes,
// us.TotalDownVotes,
// us.TotalBadges
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerDisplayName = us.DisplayName
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q12810(db: &'static So) -> String {
    let names = by_name(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(posts_of(db).opt()).and(badges_of(db).opt())).fold([0i64; 4], |a, (((u, d), p), b)| {
        [a[0] + p.is_some() as i64, a[1] + u, a[2] + d, a[3] + b.is_some() as i64]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)), Ident::<Post>::new(), "v", &[])
        .and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, (s, ((u, a), d))| v.push((p, s, u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "comments", "favorites", "created", "owner"]);
        f.extend([V::I(s.vx), V::F(s.up as f64 / s.rows as f64), V::F(s.down as f64 / s.rows as f64)]);
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH PostVoteStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(v.Id) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id
// ),
// UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// PostDetails AS (
// SELECT
// p.Id,
// p.Title,
// p.CreationDate,
// p.OwnerUserId,
// ps.UpVotes,
// ps.DownVotes,
// ps.TotalVotes,
// COALESCE(ub.BadgeCount, 0) AS UserBadgeCount,
// COALESCE(ub.GoldBadges, 0) AS UserGoldBadges,
// COALESCE(ub.SilverBadges, 0) AS UserSilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS UserBronzeBadges
// FROM
// Posts p
// LEFT JOIN
// PostVoteStats ps ON p.Id = ps.PostId
// LEFT JOIN
// UserBadgeCounts ub ON p.OwnerUserId = ub.UserId
// )
// SELECT
// pd.Id AS PostId,
// pd.Title,
// pd.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// pd.UpVotes,
// pd.DownVotes,
// pd.TotalVotes,
// pd.UserBadgeCount,
// pd.UserGoldBadges,
// pd.UserSilverBadges,
// pd.UserBronzeBadges
// FROM
// PostDetails pd
// JOIN
// Users u ON pd.OwnerUserId = u.Id
// WHERE
// pd.TotalVotes > 5
// ORDER BY
// pd.TotalVotes DESC,
// pd.CreationDate ASC
// LIMIT 10;
fn q7560(db: &'static So) -> String {
    let ub = ubc(db);
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]
    });
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&pv).filt(|a: [i64; 3]| a[2] > 5)).and((&db.post.owner_user).select(&ub))).drive(|_, x| v.push(x));
    out(v, |&((p, a), _)| (Reverse(a[2]), db.post.creation_date.get(p).unwrap()), 10, |&((p, a), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(ints(&a));
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
// COUNT(DISTINCT C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
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
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.OwnerUserId,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// CASE
// WHEN P.PostTypeId = 1 THEN 'Question'
// WHEN P.PostTypeId = 2 THEN 'Answer'
// ELSE 'Other'
// END AS PostType,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.OwnerUserId, P.ViewCount, P.Score, P.AnswerCount, P.CommentCount, P.PostTypeId
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// U.PostCount,
// U.CommentCount,
// U.UpVoteCount,
// U.DownVoteCount,
// P.PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// P.PostType,
// P.UpVotes,
// P.DownVotes
// FROM
// UserStats U
// JOIN
// PostStats P ON U.UserId = P.OwnerUserId
// ORDER BY
// U.Reputation DESC, P.ViewCount DESC;
fn q10183(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db)
        .select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and((&dp).opt()).and((&dc).opt()))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), (((u, a), d), c))| {
        let x = x.unwrap_or([0; 3]);
        let t = db.post.post_type_id.get(p).unwrap();
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d.unwrap_or(0)), V::I(c.unwrap_or(0)), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments"]));
        f.extend([V::S(match t {
            1 => "Question",
            2 => "Answer",
            _ => "Other",
        })]);
        f.extend([V::I(x[1]), V::I(x[2])]);
        row(f)
    }))
}

// WITH UserPostSummary AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedQuestions,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadges AS (
// SELECT
// UserId,
// COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges
// GROUP BY
// UserId
// ),
// UserVoteSummary AS (
// SELECT
// UserId,
// COUNT(CASE WHEN VoteTypeId IN (2, 3) THEN 1 END) AS VoteCount,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived
// FROM
// Votes
// GROUP BY
// UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.AcceptedQuestions,
// ups.TotalScore,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// uvs.VoteCount,
// uvs.UpVotesReceived,
// uvs.DownVotesReceived
// FROM
// UserPostSummary ups
// LEFT JOIN
// UserBadges ub ON ups.UserId = ub.UserId
// LEFT JOIN
// UserVoteSummary uvs ON ups.UserId = uvs.UserId
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC;
fn q6478(db: &'static So) -> String {
    let Post { post_type_id, score, accepted_answer_id, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(accepted_answer_id.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, s), ac)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && ac.is_some()) as i64, a[4] + s],
        None => a,
    });
    let bc = badge_classes(db);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + matches!(t, 2 | 3) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).and((&uv).opt()).drive(|u, ((a, b), x)| v.push((u, a, b, x)));
    rows(v.iter().map(|&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend((1..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(UBC.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBC.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBC.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS Questions,
// COALESCE(PS.Answers, 0) AS Answers,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.TotalScore, 0) AS TotalScore
// FROM Users U
// LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// Questions,
// Answers,
// TotalViews,
// TotalScore
// FROM CombinedStats
// WHERE TotalPosts > 10
// ORDER BY TotalScore DESC, BadgeCount DESC;
fn q7283(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 13]| p[0] > 10)).drive(|u, (b, p)| v.push((u, b, p)));
    rows(v.iter().map(|&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[5], p[3]]));
        row(f)
    }))
}

// WITH PostVoteSummary AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END) AS CloseVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate
// ),
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.Score,
// p.OwnerUserId,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.LastActivityDate,
// p.Tags,
// p.AcceptedAnswerId,
// COALESCE(vs.VoteCount, 0) AS VoteCount,
// COALESCE(vs.UpVotes, 0) AS UpVotes,
// COALESCE(vs.DownVotes, 0) AS DownVotes,
// COALESCE(vs.CloseVotes, 0) AS CloseVotes,
// p.PostTypeId
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostVoteSummary vs ON p.Id = vs.PostId
// )
// SELECT
// pd.PostId,
// pd.Title,
// pd.ViewCount,
// pd.AnswerCount,
// pd.CommentCount,
// pd.Score,
// pd.OwnerDisplayName,
// pd.CreationDate,
// pd.LastActivityDate,
// pd.Tags,
// pd.VoteCount,
// pd.UpVotes,
// pd.DownVotes,
// pd.CloseVotes,
// CASE
// WHEN pd.PostTypeId = 1 THEN 'Question'
// WHEN pd.PostTypeId = 2 THEN 'Answer'
// WHEN pd.PostTypeId = 3 THEN 'Wiki'
// ELSE 'Other'
// END AS PostType
// FROM
// PostDetails pd
// ORDER BY
// pd.CreationDate DESC
// LIMIT 100;
fn q14398(db: &'static So) -> String {
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (t == 6) as i64]);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&pv).opt())).drive(|_, x| v.push(x));
    out(v, |&(p, _)| newest(db, p), 100, |&(p, x)| {
        let t = db.post.post_type_id.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "views", "answers", "comments", "score", "owner", "created", "activity", "tags"]);
        f.extend(ints(&x.unwrap_or([0; 4])));
        f.push(V::S(match t {
            1 => "Question",
            2 => "Answer",
            3 => "Wiki",
            _ => "Other",
        }));
        f
    })
}

// WITH UserBadgeStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// AggregatedStats AS (
// SELECT
// ubs.UserId,
// ubs.DisplayName,
// ubs.Reputation,
// ubs.BadgeCount,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges,
// ps.PostCount,
// ps.QuestionCount,
// ps.AnswerCount,
// ps.TotalViews,
// ps.TotalScore
// FROM
// UserBadgeStats ubs
// LEFT JOIN
// PostStats ps ON ubs.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// COALESCE(PostCount, 0) AS PostCount,
// COALESCE(QuestionCount, 0) AS QuestionCount,
// COALESCE(AnswerCount, 0) AS AnswerCount,
// COALESCE(TotalViews, 0) AS TotalViews,
// COALESCE(TotalScore, 0) AS TotalScore
// FROM
// AggregatedStats
// ORDER BY
// Reputation DESC, BadgeCount DESC
// LIMIT 100;
fn q8387(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(u, b, _)| (rep_desc(db, u), Reverse(b[0])), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[5], p[3]]));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// AggregateData AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ub.TotalBadges, 0) AS TotalBadges,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(ps.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.TotalViews, 0) AS TotalViews
// FROM Users u
// LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// a.UserId,
// a.DisplayName,
// a.TotalBadges,
// a.GoldBadges,
// a.SilverBadges,
// a.BronzeBadges,
// a.TotalPosts,
// a.TotalQuestions,
// a.TotalAnswers,
// a.TotalScore,
// a.TotalViews
// FROM AggregateData a
// ORDER BY a.TotalScore DESC, a.TotalPosts DESC, a.DisplayName;
fn q5231(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or(Z))));
    rows(v.iter().map(|&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[3], p[5]]));
        row(f)
    }))
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score
// ),
// UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COALESCE(UP.TotalVotes, 0) AS TotalVotes,
// COALESCE(UP.Upvotes, 0) AS Upvotes,
// COALESCE(UP.Downvotes, 0) AS Downvotes,
// PS.CommentCount,
// PS.VoteCount
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// UserVoteStats UP ON U.Id = UP.UserId
// JOIN
// PostStats PS ON P.Id = PS.PostId
// )
// SELECT
// UserId,
// DisplayName,
// COUNT(PostId) AS TotalPosts,
// SUM(ViewCount) AS TotalPostViews,
// AVG(Score) AS AvgPostScore,
// SUM(CommentCount) AS TotalComments,
// SUM(VoteCount) AS TotalVotes
// FROM
// UserPostStats
// GROUP BY
// UserId,
// DisplayName
// ORDER BY
// TotalPosts DESC,
// TotalPostViews DESC;
fn q14583(db: &'static So) -> String {
    let pf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let f = owned(db).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&db.post.score).and(&pf)).fold([0i64; 6], |a, ((w, s), st)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + st.cx, a[5] + st.vx]
    });
    let mut v = Vec::new();
    (&f).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), V::I(a[4]), V::I(a[5])])))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// ),
// UserEngagement AS (
// SELECT
// ups.UserId,
// ups.TotalPosts,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.TotalViews,
// ups.UpVotes,
// ups.DownVotes,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM UserPostStats ups
// LEFT JOIN UserBadges ub ON ups.UserId = ub.UserId
// )
// SELECT
// u.DisplayName,
// ue.TotalPosts,
// ue.QuestionCount,
// ue.AnswerCount,
// ue.TotalViews,
// ue.UpVotes,
// ue.DownVotes,
// ue.BadgeCount,
// ue.GoldBadges,
// ue.SilverBadges,
// ue.BronzeBadges
// FROM Users u
// JOIN UserEngagement ue ON u.Id = ue.UserId
// WHERE ue.TotalPosts > 10
// ORDER BY ue.UpVotes DESC, ue.TotalViews DESC;
fn q6305(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).filt(|a: UStats| a.n > 10).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[a.n, a.q, a.a]));
        f.extend([ustat_field(&a, "views_sum"), V::I(a.up), V::I(a.down)]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// u.Id AS OwnerUserId,
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// ),
// VoteCounts AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(CASE WHEN VoteTypeId = 6 OR VoteTypeId = 10 THEN 1 END) AS CloseVotes
// FROM
// Votes
// GROUP BY
// PostId
// ),
// CommentCounts AS (
// SELECT
// PostId,
// COUNT(*) AS TotalComments
// FROM
// Comments
// GROUP BY
// PostId
// ),
// FinalResults AS (
// SELECT
// pa.PostId,
// pa.Title,
// pa.CreationDate,
// pa.Score,
// pa.ViewCount,
// pa.AnswerCount,
// pa.CommentCount,
// pa.FavoriteCount,
// pa.OwnerUserId,
// pa.OwnerReputation,
// pa.OwnerDisplayName,
// COALESCE(vc.UpVotes, 0) AS UpVotes,
// COALESCE(vc.DownVotes, 0) AS DownVotes,
// COALESCE(vc.CloseVotes, 0) AS CloseVotes,
// COALESCE(cc.TotalComments, 0) AS TotalComments
// FROM
// PostActivity pa
// LEFT JOIN
// VoteCounts vc ON pa.PostId = vc.PostId
// LEFT JOIN
// CommentCounts cc ON pa.PostId = cc.PostId
// )
// SELECT
// *,
// (UpVotes - DownVotes) AS VoteBalance,
// (Score + UpVotes - DownVotes + TotalComments) AS EngagementScore
// FROM
// FinalResults
// ORDER BY
// EngagementScore DESC
// LIMIT 100;
fn q10946(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + matches!(t, 6 | 10) as i64]);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&vc).opt()).and(comments_per_post(db))).drive(|_, ((p, x), c)| {
        let x = x.unwrap_or([0; 3]);
        v.push((p, x, c, db.post.score.get(p).unwrap() + x[0] - x[1] + c));
    });
    out(v, |&(.., e)| Reverse(e), 100, |&(p, x, c, e)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "uid", "rep", "owner"]);
        f.extend(ints(&x));
        f.extend([V::I(c), V::I(x[0] - x[1]), V::I(e)]);
        f
    })
}

// WITH UserBadgeStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// COUNT(B.Id) AS TotalBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserPostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.DisplayName,
// COALESCE(UPS.TotalPosts, 0) AS TotalPosts,
// COALESCE(UPS.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(UPS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(UBS.TotalBadges, 0) AS TotalBadges
// FROM
// Users U
// LEFT JOIN
// UserPostStats UPS ON U.Id = UPS.OwnerUserId
// LEFT JOIN
// UserBadgeStats UBS ON U.Id = UBS.UserId
// )
// SELECT
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalBadges,
// (TotalPosts * 1.0) / NULLIF(TotalAnswers, 0) AS PostsAnswerRatio
// FROM
// CombinedStats
// WHERE
// (TotalPosts > 5 OR TotalQuestions > 5)
// ORDER BY
// TotalPosts DESC,
// PostsAnswerRatio DESC
// FETCH FIRST 10 ROWS ONLY;
fn q1468(db: &'static So) -> String {
    let ub = g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64, a[3] + 1],
        None => a,
    });
    let ps = pstat(db, db.post.iq());
    let r = |p: [i64; 13]| if p[2] == 0 { None } else { Some(p[0] as f64 / p[2] as f64) };
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or(Z))));
    let v: Vec<_> = drain(rel(v).filt(|(_, _, p)| p[0] > 5 || p[1] > 5)).into_iter().map(|x| x.1).collect();
    out(v, |&(_, _, p)| (Reverse(p[0]), (r(p).is_none(), Reverse(r(p).map(fkey)))), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&p[..3]));
        f.extend(ints(&b));
        f.push(ofloat(r(p)));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
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
// ur.GoldBadges,
// ur.SilverBadges,
// ur.BronzeBadges,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.Questions, 0) AS Questions,
// COALESCE(ps.Answers, 0) AS Answers,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.TotalScore, 0) AS TotalScore
// FROM
// UserReputation ur
// LEFT JOIN
// PostStatistics ps ON ur.UserId = ps.OwnerUserId
// )
// SELECT
// u.DisplayName,
// ups.Reputation,
// ups.BadgeCount,
// ups.GoldBadges,
// ups.SilverBadges,
// ups.BronzeBadges,
// ups.PostCount,
// ups.Questions,
// ups.Answers,
// ups.TotalViews,
// ups.TotalScore
// FROM
// Users u
// JOIN
// UserPostStats ups ON u.Id = ups.UserId
// WHERE
// ups.PostCount > 0
// ORDER BY
// ups.TotalScore DESC, ups.Reputation DESC
// LIMIT 10;
fn q6686(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 13]| p[0] > 0)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(u, _, p)| (Reverse(p[3]), rep_desc(db, u)), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[5], p[3]]));
        f
    })
}

// WITH PostEngagement AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(v.UpVotes, 0) AS UpVotes,
// COALESCE(v.DownVotes, 0) AS DownVotes,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// u.Reputation AS UserReputation
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
// ParentId AS PostId,
// COUNT(*) AS AnswerCount
// FROM
// Posts
// WHERE
// PostTypeId = 2
// GROUP BY
// ParentId
// ) a ON p.Id = a.PostId
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
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) b ON p.OwnerUserId = b.UserId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// AND p.CreationDate < '2023-10-01'
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// CommentCount,
// AnswerCount,
// UpVotes,
// DownVotes,
// UserReputation,
// BadgeCount
// FROM
// PostEngagement
// ORDER BY
// ViewCount DESC;
fn q11675(db: &'static So) -> String {
    let pv = post_votes(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    since(db, date(2023, 1, 1))
        .with((&db.post.creation_date).lt(date(2023, 10, 1)))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(typed_answers_per_post(db)).and((&pv).opt()).and((&db.post.owner_user).select(&bu).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((p, c), a), x), b)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(c), V::I(a), V::I(x[1]), V::I(x[2])]);
        f.extend(post_fields(db, p, &["rep"]));
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 10 THEN 1 ELSE 0 END), 0) AS TotalDeletions,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 11 THEN 1 ELSE 0 END), 0) AS TotalUndeletions
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// EngagementInsights AS (
// SELECT
// ue.UserId,
// ue.DisplayName,
// ue.Reputation,
// ue.TotalUpvotes,
// ue.TotalDownvotes,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.AvgScore,
// ps.TotalViews
// FROM UserEngagement ue
// LEFT JOIN PostStatistics ps ON ue.UserId = ps.OwnerUserId
// )
// SELECT
// ei.DisplayName,
// ei.Reputation,
// ei.TotalUpvotes,
// ei.TotalDownvotes,
// ei.TotalQuestions,
// ei.TotalAnswers,
// ei.AvgScore,
// ei.TotalViews,
// CASE
// WHEN ei.TotalAnswers > ei.TotalQuestions THEN 'High Answer Rate'
// WHEN ei.TotalUpvotes > ei.TotalDownvotes THEN 'Positive Engagement'
// ELSE 'Needs Improvement'
// END AS EngagementCategory
// FROM EngagementInsights ei
// WHERE ei.Reputation > 100
// ORDER BY ei.Reputation DESC, ei.TotalUpvotes DESC;
fn q6598(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and((&uv).opt()).and((&ps).opt())).drive(|_, ((u, x), p)| v.push((u, x.unwrap_or([0; 2]), p)));
    rows(v.iter().map(|&(u, x, p)| {
        let cat = if p.map_or(false, |p| p[2] > p[1]) {
            "High Answer Rate"
        } else if x[0] > x[1] {
            "Positive Engagement"
        } else {
            "Needs Improvement"
        };
        row(vec![
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(x[0]),
            V::I(x[1]),
            oint(p.map(|p| p[1])),
            oint(p.map(|p| p[2])),
            onull(p, pscore_avg),
            onull(p, pviews),
            V::S(cat),
        ])
    }))
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikiCount,
// SUM(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPostCount
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// VoteStats AS (
// SELECT
// V.UserId,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes V
// GROUP BY
// V.UserId
// )
// SELECT
// U.DisplayName,
// UB.BadgeCount,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// PS.PostCount,
// PS.QuestionCount,
// PS.AnswerCount,
// PS.TagWikiCount,
// PS.ClosedPostCount,
// VS.VoteCount,
// VS.UpVotes,
// VS.DownVotes
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// LEFT JOIN
// VoteStats VS ON U.Id = VS.UserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// UB.BadgeCount DESC, PS.PostCount DESC, VS.VoteCount DESC
// LIMIT 100;
fn q7264(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and((&ps).opt()).and((&vs).opt())).drive(|_, x| v.push(x));
    out(v, |&(((_, b), p), x)| (Reverse(b[0]), (p.is_none(), Reverse(p.map(|p| p[0]))), (x.is_none(), Reverse(x.map(|x| x[0])))), 100, |&(((u, b), p), x)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend([0, 1, 2, 9, 11].iter().map(|&i| oint(p.map(|p| p[i]))));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AvgScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(US.PostCount, 0) AS PostCount,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount
// FROM Users U
// LEFT JOIN PostStatistics US ON U.Id = US.OwnerUserId
// LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId
// )
// SELECT
// UP.DisplayName,
// UP.PostCount,
// UP.BadgeCount,
// COALESCE(UPB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UPB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UPB.BronzeBadges, 0) AS BronzeBadges,
// CASE
// WHEN UP.PostCount > 50 THEN 'Prolific'
// WHEN UP.PostCount BETWEEN 20 AND 50 THEN 'Active'
// WHEN UP.PostCount BETWEEN 1 AND 19 THEN 'Newcomer'
// ELSE 'No Posts'
// END AS UserStatus,
// (SELECT COUNT(*) FROM Comments C WHERE C.UserId = UP.UserId) AS TotalComments
// FROM UserPerformance UP
// LEFT JOIN UserBadgeCounts UPB ON UP.UserId = UPB.UserId
// ORDER BY UP.PostCount DESC, UP.BadgeCount DESC
// LIMIT 10;
fn q489(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let cu = comments_per_user(db);
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).and(&cu).drive(|u, ((b, p), c)| v.push((u, b, p.map_or(0, |p| p[0]), c)));
    out(v, |&(_, b, n, _)| (Reverse(n), Reverse(b[0])), 10, |&(u, b, n, c)| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(b[0])];
        f.extend(ints(&b[1..]));
        f.push(V::S(if n > 50 {
            "Prolific"
        } else if (20..=50).contains(&n) {
            "Active"
        } else if (1..=19).contains(&n) {
            "Newcomer"
        } else {
            "No Posts"
        }));
        f.push(V::I(c));
        f
    })
}

// WITH UserBadgeStats AS (
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
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(p.Score) AS AvgScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ubs.BadgeCount, 0) AS BadgeCount,
// COALESCE(ubs.GoldBadges, 0) AS GoldBadges,
// COALESCE(ubs.SilverBadges, 0) AS SilverBadges,
// COALESCE(ubs.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.AvgScore, 0) AS AvgScore
// FROM
// Users u
// LEFT JOIN
// UserBadgeStats ubs ON u.Id = ubs.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// up.UserId,
// up.DisplayName,
// up.BadgeCount,
// up.GoldBadges,
// up.SilverBadges,
// up.BronzeBadges,
// up.PostCount,
// up.QuestionCount,
// up.AnswerCount,
// up.AvgScore
// FROM
// UserPerformance up
// WHERE
// up.PostCount > 0
// ORDER BY
// up.AvgScore DESC, up.BadgeCount DESC
// LIMIT 10;
fn q5669(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let av = |p: [i64; 13]| p[3] as f64 / p[0] as f64;
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 13]| p[0] > 0)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(fkey(av(p))), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p[..3]));
        f.push(V::F(av(p)));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// TopPosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// COALESCE(b.GoldBadges, 0) AS GoldBadges,
// COALESCE(b.SilverBadges, 0) AS SilverBadges,
// COALESCE(b.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(p.PostCount, 0) AS PostCount,
// COALESCE(p.QuestionCount, 0) AS QuestionCount,
// COALESCE(p.AnswerCount, 0) AS AnswerCount,
// COALESCE(p.PositiveScoreCount, 0) AS PositiveScoreCount
// FROM
// Users u
// LEFT JOIN
// UserBadges b ON u.Id = b.UserId
// LEFT JOIN
// TopPosts p ON u.Id = p.OwnerUserId
// )
// SELECT
// u.Id,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// u.BadgeCount,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// u.PositiveScoreCount
// FROM
// UserStats u
// WHERE
// u.Reputation > 1000
// ORDER BY
// u.Reputation DESC,
// u.BadgeCount DESC
// LIMIT 50;
fn q5961(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let pos = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, s| a + (s > 0) as i64);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt()).and((&pos).opt())).drive(|_, x| v.push(x));
    out(v, |&(((u, b), _), _)| (rep_desc(db, u), Reverse(b)), 50, |&(((u, b), p), s)| {
        let q = p.unwrap_or(Z);
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated"), V::I(b), V::I(q[0]), V::I(q[1]), V::I(q[2]), V::I(s.unwrap_or(0))]
    })
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserBadges AS (
// SELECT
// B.UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 ELSE NULL END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 ELSE NULL END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 ELSE NULL END) AS BronzeBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// UserStats AS (
// SELECT
// UR.UserId,
// UR.DisplayName,
// UR.UpVotes,
// UR.DownVotes,
// UR.QuestionCount,
// UR.AnswerCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserReputation UR
// LEFT JOIN
// UserBadges UB ON UR.UserId = UB.UserId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.UpVotes,
// U.DownVotes,
// U.QuestionCount,
// U.AnswerCount,
// U.GoldBadges,
// U.SilverBadges,
// U.BronzeBadges,
// (U.UpVotes - U.DownVotes) AS NetVotes,
// (U.QuestionCount + U.AnswerCount) AS TotalPosts,
// (U.GoldBadges + U.SilverBadges + U.BronzeBadges) AS TotalBadges
// FROM
// UserStats U
// WHERE
// (U.UpVotes - U.DownVotes) > 0
// ORDER BY
// TotalPosts DESC, NetVotes DESC
// LIMIT 10;
fn q5748(db: &'static So) -> String {
    let uf = g(db).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(&db.post.post_type_id).opt())).fold([0i64; 4], |a, (v, t)| {
        [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (t == Some(1)) as i64, a[3] + (t == Some(2)) as i64]
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 4]| a[0] - a[1] > 0).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| (Reverse(a[2] + a[3]), Reverse(a[0] - a[1])), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b[1..]));
        f.extend([V::I(a[0] - a[1]), V::I(a[2] + a[3]), V::I(b[1] + b[2] + b[3])]);
        f
    })
}

// WITH UserPostActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
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
// ),
// PostPerformance AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.TotalQuestions,
// ua.TotalAnswers,
// ua.TotalScore,
// ua.TotalViews,
// pp.PostId,
// pp.Title,
// pp.CreationDate,
// pp.Score,
// pp.ViewCount,
// pp.CommentCount,
// pp.UpVotes AS PostUpVotes,
// pp.DownVotes AS PostDownVotes
// FROM
// UserPostActivity ua
// JOIN
// PostPerformance pp ON ua.UserId = pp.PostId
// ORDER BY
// ua.TotalScore DESC, pp.Score DESC;
fn q11166(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&pf).and(comments_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()))).drive(|p, ((s, c), ((u, a), d))| v.push((p, s, c, u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, c, u, a, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.q), V::I(a.a), ustat_field(&a, "score_sum"), ustat_field(&a, "views_sum")];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ), PostDetails AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ), CombinedData AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(UB.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(UB.BronzeBadgeCount, 0) AS BronzeBadgeCount,
// COALESCE(PD.QuestionCount, 0) AS QuestionCount,
// COALESCE(PD.AnswerCount, 0) AS AnswerCount,
// COALESCE(PD.TotalScore, 0) AS TotalScore,
// COALESCE(PD.TotalViews, 0) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostDetails PD ON U.Id = PD.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadgeCount,
// SilverBadgeCount,
// BronzeBadgeCount,
// QuestionCount,
// AnswerCount,
// TotalScore,
// TotalViews
// FROM
// CombinedData
// ORDER BY
// TotalScore DESC, BadgeCount DESC, DisplayName ASC
// FETCH FIRST 100 ROWS ONLY;
fn q9598(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(u, b, p)| (Reverse(p[3]), Reverse(b[0]), db.user.display_name.get(u).unwrap()), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[1], p[2], p[3], p[5]]));
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPosts,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties,
// COALESCE(SUM(b.Class), 0) AS TotalBadges
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
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS ClosedCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenedCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate
// )
// SELECT
// ua.DisplayName,
// ua.PostCount,
// ua.UpvotedPosts,
// ua.DownvotedPosts,
// ua.TotalBounties,
// ua.TotalBadges,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Upvotes,
// ps.Downvotes,
// ps.CommentCount,
// ps.ClosedCount,
// ps.ReopenedCount
// FROM
// UserActivity ua
// JOIN
// PostStatistics ps ON ua.UserId = ps.PostId
// WHERE
// ua.PostCount > 0
// ORDER BY
// ua.TotalBadges DESC,
// ps.Upvotes DESC
// LIMIT 100;
fn q9611(db: &'static So) -> String {
    let uid = uids(db);
    let Post { score, .. } = &db.post;
    let uf = g(db)
        .select(posts_of(db).select(score.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, c)| match p {
            Some((s, b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + b.unwrap_or(0), a[4] + c.unwrap_or(0)]
            }
            None => [a[0], a[1], a[2], a[3], a[4] + c.unwrap_or(0)],
        });
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvh", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&uf).filt(|a: [i64; 5]| a[0] > 0)))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(_, s, _, a)| (Reverse(a[4]), Reverse(s.up)), 100, |&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend(ints(&[s.up, s.down, s.cx, s.h10, s.h11]));
        f
    })
}

// WITH UserBadges AS (
// SELECT U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.Score) AS TotalScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// UserEngagement AS (
// SELECT U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.Views,
// COALESCE(PostStats.TotalPosts, 0) AS TotalPosts,
// COALESCE(PostStats.Questions, 0) AS Questions,
// COALESCE(PostStats.Answers, 0) AS Answers,
// COALESCE(PostStats.TotalScore, 0) AS TotalScore,
// COALESCE(UserBadges.BadgeCount, 0) AS BadgeCount,
// COALESCE(UserBadges.GoldBadges, 0) AS GoldBadges,
// COALESCE(UserBadges.SilverBadges, 0) AS SilverBadges,
// COALESCE(UserBadges.BronzeBadges, 0) AS BronzeBadges
// FROM Users U
// LEFT JOIN PostStatistics PostStats ON U.Id = PostStats.OwnerUserId
// LEFT JOIN UserBadges UserBadges ON U.Id = UserBadges.UserId
// )
// SELECT UserId, DisplayName, Reputation, Views, TotalPosts, Questions, Answers, TotalScore,
// BadgeCount, GoldBadges, SilverBadges, BronzeBadges
// FROM UserEngagement
// WHERE Reputation > 1000
// ORDER BY TotalScore DESC, BadgeCount DESC
// LIMIT 100;
fn q5223(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "uviews")];
        f.extend(ints(&p[..4]));
        f.extend(ints(&b));
        f
    })
}

// WITH TagCounts AS (
// SELECT
// Tags.TagName,
// COUNT(*) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS ClosedPostCount
// FROM
// Posts P
// JOIN
// Tags ON P.Tags LIKE '%' || Tags.TagName || '%'
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// Tags.TagName
// ),
// UserVotes AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostMetrics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// COALESCE(PC.PostCount, 0) AS TotalPosts,
// COALESCE(UC.UpVoteCount, 0) AS TotalUpVotes,
// COALESCE(UC.DownVoteCount, 0) AS TotalDownVotes,
// COALESCE(TC.ClosedPostCount, 0) AS TotalClosedPosts
// FROM
// Posts P
// LEFT JOIN
// (SELECT
// P.Id,
// COUNT(P.Id) AS PostCount
// FROM
// Posts P
// WHERE
// P.PostTypeId IN (1, 2)
// GROUP BY
// P.Id) PC ON P.Id = PC.Id
// LEFT JOIN
// UserVotes UC ON P.OwnerUserId = UC.UserId
// LEFT JOIN
// TagCounts TC ON P.Tags LIKE '%' || TC.TagName || '%'
// )
// SELECT
// Title,
// TotalPosts,
// TotalUpVotes,
// TotalDownVotes,
// TotalClosedPosts
// FROM
// PostMetrics
// WHERE
// TotalPosts > 1
// ORDER BY
// TotalPosts DESC, TotalUpVotes DESC, TotalDownVotes ASC
// LIMIT 10;
fn q28027(db: &'static So) -> String {
    let tm = tag_mentions(db);
    let tc = (&tm)
        .group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t))
        .select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p).select((&db.post.post_type_id).and(history_of(db).select(&db.post_history.post_history_type_id).opt())))
        .fold([0i64; 4], |a, (t, h)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (h == Some(10)) as i64]);
    let pc = db.post.with((&db.post.post_type_id).in_v(vec![1, 2])).group_by(Ident::<Post>::new()).fold(0i64, |a, _| a + 1);
    let uv = user_votes(db);
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&tm).map(|(p, _)| p).inv().collect();
    let mut v = Vec::new();
    db.post
        .select(
            Ident::<Post>::new()
                .and((&pc).filt(|n: i64| n > 1))
                .and((&db.post.owner_user).select(&uv).opt())
                .and((&by_post).select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t).select(&tc)).opt()),
        )
        .drive(|_, x| v.push(x));
    out(v, |&(((_, n), x), _)| (Reverse(n), Reverse(x.map_or(0, |x| x[1])), x.map_or(0, |x| x[2])), 10, |&(((p, n), x), c)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(n), V::I(x[1]), V::I(x[2]), V::I(c.map_or(0, |c| c[3]))]);
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(BC.GoldBadges, 0) AS GoldBadges,
// COALESCE(BC.SilverBadges, 0) AS SilverBadges,
// COALESCE(BC.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS Questions,
// COALESCE(PS.Answers, 0) AS Answers,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AverageScore, 0) AS AverageScore
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts BC ON U.Id = BC.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UP.UserId,
// UP.DisplayName,
// UP.TotalPosts,
// UP.Questions,
// UP.Answers,
// UP.TotalViews,
// UP.AverageScore,
// (UP.GoldBadges * 3 + UP.SilverBadges * 2 + UP.BronzeBadges * 1) AS BadgeScore
// FROM
// UserPerformance UP
// WHERE
// (UP.TotalPosts > 10 OR UP.GoldBadges > 0)
// ORDER BY
// BadgeScore DESC,
// UP.TotalViews DESC;
fn q3073(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    rows(drain(rel(v).filt(|(_, b, p)| p.map_or(0, |p| p[0]) > 10 || b[1] > 0)).into_iter().map(|(_, (u, b, p))| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[q[0], q[1], q[2], q[5]]));
        f.extend([p.map_or(V::F(0.0), pscore_avg), V::I(b[1] * 3 + b[2] * 2 + b[3])]);
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AverageViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserBenchmarks AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS TotalBadges,
// COALESCE(PS.PostCount, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS TotalQuestions,
// COALESCE(PS.Answers, 0) AS TotalAnswers,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.AverageViews, 0) AS AverageViews
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalBadges,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// AverageViews,
// CASE
// WHEN TotalBadges >= 5 AND TotalPosts >= 10 THEN 'Active Contributor'
// WHEN TotalBadges >= 3 AND TotalPosts < 10 THEN 'Emerging Contributor'
// ELSE 'New User'
// END AS UserType
// FROM
// UserBenchmarks
// ORDER BY
// TotalScore DESC, TotalPosts DESC;
fn q26245(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let q = p.unwrap_or(Z);
        let t = if b >= 5 && q[0] >= 10 {
            "Active Contributor"
        } else if b >= 3 && q[0] < 10 {
            "Emerging Contributor"
        } else {
            "New User"
        };
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), V::I(q[0]), V::I(q[1]), V::I(q[2]), V::I(q[3]), if q[4] > 0 { pviews_avg(q) } else { V::F(0.0) }, V::S(t)])
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(c.Score) AS TotalCommentScore,
// AVG(p.Score) AS AveragePostScore
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
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.CreationDate,
// p.LastActivityDate,
// pt.Name AS PostType
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// ),
// UserPostStats AS (
// SELECT
// u.UserId,
// u.DisplayName,
// ps.PostId,
// ps.Title,
// ps.PostType,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount
// FROM
// UserActivity u
// JOIN
// Posts p ON u.UserId = p.OwnerUserId
// JOIN
// PostStatistics ps ON p.Id = ps.PostId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.TotalComments,
// ua.TotalUpVotes,
// ua.TotalDownVotes,
// ua.TotalCommentScore,
// ua.AveragePostScore,
// ups.Title AS PostTitle,
// ups.PostType,
// ups.ViewCount,
// ups.AnswerCount,
// ups.CommentCount,
// ups.FavoriteCount
// FROM
// UserActivity ua
// LEFT JOIN
// UserPostStats ups ON ua.UserId = ups.UserId
// ORDER BY
// ua.TotalPosts DESC;
fn q11079(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&dc).opt()).and(posts_of(db).opt()).drive(|u, (((a, d), c), p)| v.push((u, a, d.unwrap_or(0), c.unwrap_or(0), p)));
    rows(v.iter().map(|&(u, a, d, c, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(c), V::I(a.up), V::I(a.down), ustat_field(&a, "cscore_sum"), ustat_field(&a, "score_avg")];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "type", "views", "answers", "comments", "favorites"])),
            None => f.extend(nulls(6)),
        }
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViewCount,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// AVG(CASE WHEN p.CommentCount IS NOT NULL THEN p.CommentCount ELSE 0 END) AS AvgCommentCount,
// AVG(CASE WHEN p.FavoriteCount IS NOT NULL THEN p.FavoriteCount ELSE 0 END) AS AvgFavoriteCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserStatistics AS (
// SELECT
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(b.Class) AS TotalBadgeClass,
// AVG(p.ViewCount) AS UserAvgViewCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName, u.Reputation
// ),
// CommentStatistics AS (
// SELECT
// COUNT(c.Id) AS TotalComments,
// AVG(c.Score) AS AvgCommentScore
// FROM
// Comments c
// )
// SELECT
// ps.PostType,
// ps.PostCount,
// ps.AvgScore,
// ps.AvgViewCount,
// ps.AcceptedAnswers,
// ps.AvgCommentCount,
// ps.AvgFavoriteCount,
// us.DisplayName AS TopUser,
// us.Reputation AS TopUserReputation,
// us.BadgeCount AS TopUserBadgeCount,
// us.TotalBadgeClass AS TopUserTotalBadgeClass,
// us.UserAvgViewCount AS TopUserAvgViewCount,
// cs.TotalComments,
// cs.AvgCommentScore
// FROM
// PostStatistics ps
// CROSS JOIN
// (SELECT u.DisplayName, u.Reputation, u.BadgeCount, u.TotalBadgeClass, u.UserAvgViewCount
// FROM UserStatistics u
// ORDER BY u.Reputation DESC
// LIMIT 1) us
// CROSS JOIN
// CommentStatistics cs;
fn q14270(db: &'static So) -> String {
    let Post { score, view_count, accepted_answer_id, comment_count, favorite_count, .. } = &db.post;
    let pf = by_key(db.post.iq(), name(db), score.and(view_count.opt()).and(accepted_answer_id.opt()).and(comment_count).and(favorite_count.opt()), [0i64; 7], |a, ((((s, w), ac), c), fv)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + ac.is_some() as i64, a[5] + c, a[6] + fv.unwrap_or(0)]
    });
    let User { display_name, reputation, .. } = &db.user;
    let us = db.user.group_by(display_name.and(reputation)).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select((&db.post.view_count).opt()).opt())).fold([0i64; 4], |a, (b, w)| {
        let w = w.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let top: MatSet<(Str, i64)> = whole(&us).select(Same::<(Str, i64)>::new()).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 1).map(|(k, _)| k).collect();
    let t = rel(drain((&top).select(Same::<(Str, i64)>::new().and(&us))));
    let c = db.comment.select(&db.comment.score).fold_flat([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = drain((&pf).cross(&t));
    rows(v.into_iter().map(|((k, _), (a, (_, ((n, r), b))))| {
        {
            row(vec![
                V::S(k),
                V::I(a[0]),
                avg(a[1], a[0]),
                avg(a[3], a[2]),
                V::I(a[4]),
                avg(a[5], a[0]),
                avg(a[6], a[0]),
                V::S(n),
                V::I(r),
                V::I(b[0]),
                nullable(b[1], b[0]),
                avg(b[3], b[2]),
                V::I(c[0]),
                avg(c[1], c[0]),
            ])
        }
    }))
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
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
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.TotalViews,
// ps.AverageScore
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// up.UserId,
// up.DisplayName,
// COALESCE(up.BadgeCount, 0) AS TotalBadges,
// COALESCE(up.GoldBadges, 0) AS GoldBadges,
// COALESCE(up.SilverBadges, 0) AS SilverBadges,
// COALESCE(up.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(up.TotalPosts, 0) AS TotalPosts,
// COALESCE(up.Questions, 0) AS TotalQuestions,
// COALESCE(up.Answers, 0) AS TotalAnswers,
// COALESCE(up.TotalViews, 0) AS TotalViews,
// COALESCE(up.AverageScore, 0) AS AverageScore
// FROM
// UserPerformance up
// ORDER BY
// TotalViews DESC,
// AverageScore DESC
// LIMIT 100;
fn q9874(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    let av = |p: Option<[i64; 13]>| p.map_or(0.0, |p| p[3] as f64 / p[0] as f64);
    out(v, |&(_, _, p)| (Reverse(p.map_or(0, |p| p[5])), Reverse(fkey(av(p)))), 100, |&(u, b, p)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[q[0], q[1], q[2], q[5]]));
        f.push(V::F(av(p)));
        f
    })
}

// WITH UserBadgeStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
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
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ubs.BadgeCount, 0) AS BadgeCount,
// COALESCE(ubs.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(ubs.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(ubs.BronzeBadgeCount, 0) AS BronzeBadgeCount,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// UserBadgeStats ubs ON u.Id = ubs.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadgeCount,
// SilverBadgeCount,
// BronzeBadgeCount,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore
// FROM
// UserPerformance
// WHERE
// TotalScore > 20 OR BadgeCount >= 5
// ORDER BY
// TotalScore DESC, BadgeCount DESC
// LIMIT 10;
fn q8310(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or(Z))));
    let v: Vec<_> = drain(rel(v).filt(|(_, b, p)| p[3] > 20 || b[0] >= 5)).into_iter().map(|x| x.1).collect();
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p[..4]));
        f
    })
}

// WITH UserVoteCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN (
// SELECT
// ParentId,
// COUNT(Id) AS AnswerCount
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
// COUNT(Id) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(Id) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) b ON p.OwnerUserId = b.UserId
// )
// SELECT
// u.DisplayName,
// SUM(uv.VoteCount) AS TotalVotes,
// SUM(uv.Upvotes) AS TotalUpvotes,
// SUM(uv.Downvotes) AS TotalDownvotes,
// COUNT(ps.PostId) AS TotalPosts,
// SUM(ps.ViewCount) AS TotalViews,
// SUM(ps.AnswerCount) AS TotalAnswers,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.BadgeCount) AS TotalBadges
// FROM
// UserVoteCounts uv
// JOIN
// Users u ON uv.UserId = u.Id
// JOIN
// PostStatistics ps ON u.Id = ps.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalVotes DESC
// LIMIT 10;
fn q12629(db: &'static So) -> String {
    let uv = user_votes(db);
    let bu = badges_per_user(db);
    let f = owned(db)
        .group_by((&db.post.owner_user).select(&db.user.display_name))
        .select((&db.post.owner_user).select(&uv).and((&db.post.view_count).opt()).and(typed_answers_per_post(db)).and(comments_per_post(db)).and((&db.post.owner_user).select(&bu)))
        .fold([0i64; 10], |a, ((((x, w), an), c), b)| {
            [a[0] + x[0], a[1] + x[1], a[2] + x[2], a[3] + 1, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + an, a[7] + c, a[8] + b, 0]
        });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |&(_, a)| Reverse(a[0]), 10, |&(k, a)| vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), V::I(a[6]), V::I(a[7]), V::I(a[8])])
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
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(pb.TotalPosts, 0) AS TotalPosts,
// COALESCE(pb.Questions, 0) AS Questions,
// COALESCE(pb.Answers, 0) AS Answers,
// COALESCE(pb.TotalScore, 0) AS TotalScore,
// COALESCE(pb.TotalViews, 0) AS TotalViews,
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
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// TotalScore,
// TotalViews,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// CombinedStats
// WHERE
// TotalPosts > 0
// ORDER BY
// TotalScore DESC,
// BadgeCount DESC
// LIMIT 10;
fn q8345(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 13]| p[0] > 0)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[3], p[5]]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.Questions, 0) AS Questions,
// COALESCE(PS.Answers, 0) AS Answers,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AverageScore, 0) AS AverageScore,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// Questions,
// Answers,
// TotalViews,
// AverageScore,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// UserActivity
// WHERE
// BadgeCount > 0
// ORDER BY
// BadgeCount DESC,
// TotalViews DESC
// LIMIT 10;
fn q6970(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).filt(|b: [i64; 4]| b[0] > 0).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[0]), Reverse(p.map_or(0, |p| p[5]))), 10, |&(u, b, p)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[q[0], q[1], q[2], q[5]]));
        f.push(p.map_or(V::F(0.0), pscore_avg));
        f.extend(ints(&b));
        f
    })
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// AVG(U.Reputation) AS AvgReputation,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// PostCount,
// AnswerCount,
// QuestionCount,
// UpVoteCount,
// DownVoteCount,
// AvgReputation,
// LastPostDate
// FROM
// UserActivity
// WHERE
// PostCount > 0
// ORDER BY
// UpVoteCount DESC
// LIMIT 10
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// AVG(P.Score) AS AvgScore,
// COUNT(DISTINCT C.Id) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// TU.DisplayName,
// TU.PostCount,
// TU.QuestionCount,
// TU.AnswerCount,
// TU.UpVoteCount,
// TU.DownVoteCount,
// TU.AvgReputation,
// PS.TotalQuestions,
// PS.TotalAnswers,
// PS.AvgScore,
// PS.CommentCount,
// TU.LastPostDate
// FROM
// TopUsers TU
// JOIN
// PostStatistics PS ON TU.UserId = PS.OwnerUserId
// ORDER BY
// TU.UpVoteCount DESC,
// PS.AvgScore DESC;
fn q6757(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let base = db.user.with((&us).filt(|a: UStats| a.n > 0));
    let top: MatSet<Id<User>> = whole(&base).select(Ident::<User>::new().and(&us)).window(row_number, |(_, a): (Id<User>, UStats)| a.up, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score).and(comments_of(db).opt())).fold([0i64; 4], |a, ((t, s), _)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]
    });
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&us).and(&ps).and((&dc).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, a), p), c)| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a.n),
            V::I(a.q),
            V::I(a.a),
            V::I(a.up),
            V::I(a.down),
            V::F(db.user.reputation.get(u).unwrap() as f64),
            V::I(p[1]),
            V::I(p[2]),
            avg(p[3], p[0]),
            V::I(c.unwrap_or(0)),
            ustat_field(&a, "created_max"),
        ])
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS TotalBadges,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AvgViewCount
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UBC.TotalBadges, 0) AS TotalBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS Questions,
// COALESCE(PS.Answers, 0) AS Answers,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.AvgViewCount, 0) AS AvgViewCount
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UPS.DisplayName,
// UPS.TotalBadges,
// UPS.TotalPosts,
// UPS.Questions,
// UPS.Answers,
// UPS.TotalScore,
// UPS.AvgViewCount,
// CASE
// WHEN UPS.TotalPosts > 50 THEN 'High Activity'
// WHEN UPS.TotalPosts > 20 THEN 'Moderate Activity'
// ELSE 'Low Activity'
// END AS ActivityLevel
// FROM
// UserPostStats UPS
// WHERE
// UPS.TotalBadges > 0
// ORDER BY
// UPS.TotalScore DESC,
// UPS.TotalPosts DESC
// LIMIT 10;
fn q1823(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&bu).filt(|b: i64| b > 0)).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, p)));
    out(v, |&(_, _, p)| (Reverse(p.map_or(0, |p| p[3])), Reverse(p.map_or(0, |p| p[0]))), 10, |&(u, b, p)| {
        let q = p.unwrap_or(Z);
        vec![
            user_col(db, u, "name"),
            V::I(b),
            V::I(q[0]),
            V::I(q[1]),
            V::I(q[2]),
            V::I(q[3]),
            if q[4] > 0 { pviews_avg(q) } else { V::F(0.0) },
            V::S(if q[0] > 50 {
                "High Activity"
            } else if q[0] > 20 {
                "Moderate Activity"
            } else {
                "Low Activity"
            }),
        ]
    })
}

// WITH UserBadgeCounts AS (
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
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(pc.TotalPosts, 0) AS TotalPosts,
// COALESCE(pc.Questions, 0) AS Questions,
// COALESCE(pc.Answers, 0) AS Answers,
// COALESCE(pc.TotalViews, 0) AS TotalViews,
// COALESCE(pc.AverageScore, 0.0) AS AverageScore,
// COALESCE(bc.BadgeCount, 0) AS TotalBadges,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// PostStats pc ON u.Id = pc.OwnerUserId
// LEFT JOIN
// UserBadgeCounts bc ON u.Id = bc.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// TotalViews,
// AverageScore,
// TotalBadges,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// CombinedStats
// WHERE
// TotalPosts > 0 OR TotalBadges > 0
// ORDER BY
// TotalPosts DESC, TotalBadges DESC;
fn q28091(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    rows(drain(rel(v).filt(|(_, b, p)| p.map_or(0, |p| p[0]) > 0 || b[0] > 0)).into_iter().map(|(_, (u, b, p))| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[q[0], q[1], q[2], q[5]]));
        f.push(p.map_or(V::F(0.0), pscore_avg));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.DisplayName,
// U.Reputation,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.QuestionCount, 0) AS QuestionCount,
// COALESCE(PS.AnswerCount, 0) AS AnswerCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews
// FROM Users U
// LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// DisplayName,
// Reputation,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore,
// TotalViews
// FROM CombinedStats
// ORDER BY Reputation DESC, TotalScore DESC, BadgeCount DESC
// LIMIT 10;
fn q9433(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, answer_count, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(answer_count.opt()).and(score).and(view_count.opt())).fold([0i64; 5], |a, (((t, an), s), w)| {
        [a[0] + 1, a[1] + if t == 1 { an.unwrap_or(0) } else { 0 }, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or([0; 5]))));
    out(v, |&(u, b, p)| (rep_desc(db, u), Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend(ints(&p));
        f
    })
}

pub static ENTRIES: &[harness::Entry] = &[
    ("9356", q9356),
    ("9412", q9412),
    ("9428", q9428),
    ("9450", q9450),
    ("9575", q9575),
    ("9590", q9590),
    ("9675", q9675),
    ("9724", q9724),
    ("9743", q9743),
    ("9745", q9745),
    ("9877", q9877),
    ("9943", q9943),
    ("9998", q9998),
    ("14519", q14519),
    ("28321", q28321),
    ("6767", q6767),
    ("5637", q5637),
    ("12810", q12810),
    ("7560", q7560),
    ("10183", q10183),
    ("6478", q6478),
    ("7283", q7283),
    ("14398", q14398),
    ("8387", q8387),
    ("5231", q5231),
    ("14583", q14583),
    ("6305", q6305),
    ("10946", q10946),
    ("1468", q1468),
    ("6686", q6686),
    ("11675", q11675),
    ("6598", q6598),
    ("7264", q7264),
    ("489", q489),
    ("5669", q5669),
    ("5961", q5961),
    ("5748", q5748),
    ("11166", q11166),
    ("9598", q9598),
    ("9611", q9611),
    ("5223", q5223),
    ("28027", q28027),
    ("3073", q3073),
    ("26245", q26245),
    ("11079", q11079),
    ("14270", q14270),
    ("9874", q9874),
    ("8310", q8310),
    ("12629", q12629),
    ("8345", q8345),
    ("6970", q6970),
    ("6757", q6757),
    ("1823", q1823),
    ("28091", q28091),
    ("9433", q9433),
];
