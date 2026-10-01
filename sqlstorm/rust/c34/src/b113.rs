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

fn unit() -> VecRel<usize, ()> {
    rel(vec![()])
}

fn badges_by_uid(db: &'static So) -> HashIdx<i64, Id<Badge>> {
    (&db.badge.user_id).inv().collect()
}

fn cvb<Q: Drive<D = Id<Post>, R = Id<Post>>>(db: &'static So, base: Q, by_uid: &HashIdx<i64, Id<Badge>>) -> Fold<Id<Post>, [i64; 5]> {
    base.group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user_id).select(by_uid).opt()))
        .fold([0i64; 5], |s, ((c, t), b)| [s[0] + c.is_some() as i64, s[1] + t.is_some() as i64, s[2] + (t == Some(2)) as i64, s[3] + (t == Some(3)) as i64, s[4] + b.is_some() as i64])
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

// --- posts ------------------------------------------------------------------

// WITH PostVoteCounts AS (
// SELECT
// P.Id AS PostId,
// COUNT(V.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id
// ),
// UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// MAX(P.CreationDate) AS LastPostDate
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
// U.PostCount,
// U.TotalViews,
// U.TotalScore,
// U.LastPostDate,
// PVC.PostId,
// PVC.VoteCount,
// PVC.UpVotes,
// PVC.DownVotes
// FROM
// UserPostStats U
// LEFT JOIN
// PostVoteCounts PVC ON PVC.PostId = U.UserId
// ORDER BY
// U.TotalScore DESC, U.PostCount DESC
// LIMIT 100;
fn q10231(db: &'static So) -> String {
    let Post { view_count, score, creation_date, .. } = &db.post;
    let pid = pids(db);
    let pv = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "v", &[]);
    let uf = g(db).select(posts_of(db).select(view_count.opt().and(score).and(creation_date)).opt()).fold((0i64, 0i64, 0i64, i64::MIN), |(n, vs, ss, m), p| match p {
        Some(((w, s), cd)) => (n + 1, vs + w.unwrap_or(0), ss + s, m.max(cd)),
        None => (n, vs, ss, m),
    });
    let mut v = Vec::new();
    (&uf).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&pv)).opt()).drive(|u, (a, x)| v.push((u, a, x)));
    out(v, |&(_, (n, _, ss, _), _)| (Reverse(ss), Reverse(n)), 100, |&(u, (n, vs, ss, m), x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n), V::I(vs), V::I(ss), if n == 0 { V::Null } else { V::T(m) }];
        match x {
            Some((p, s)) => f.extend([post_fields(db, p, &["id"]).pop().unwrap(), V::I(s.vx), V::I(s.up), V::I(s.down)]),
            None => f.extend(nulls(4)),
        }
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(p.CreationDate) AS LastActivityDate,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// SUM(u.Reputation) AS TotalReputation,
// COUNT(DISTINCT p.Id) AS PostCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// )
// SELECT
// ps.PostId,
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// ps.LastActivityDate,
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.BadgeCount,
// us.TotalReputation,
// us.PostCount
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostTypeId = us.UserId
// ORDER BY
// ps.VoteCount DESC,
// ps.CommentCount DESC
// LIMIT 100;
fn q10246(db: &'static So) -> String {
    let uid = uids(db);
    let pf = stats_fold(db, db.post.with((&db.post.post_type_id).select(&uid)), Ident::<Post>::new(), "cv", &[]);
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&pf)
        .and((&db.post.post_type_id).select(&uid).select(Ident::<User>::new().and(&uf).and((&dp).opt())))
        .drive(|p, (s, ((u, us), d))| v.push((p, s, u, us, d.unwrap_or(0))));
    out(v, |&(_, s, _, _, _)| (Reverse(s.vx), Reverse(s.cx)), 100, |&(p, s, u, us, d)| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([V::I(s.cx), V::I(s.vx)]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([V::I(s.up), V::I(s.down), user_col(db, u, "uid"), V::I(us.bx), V::I(us.rep_sum), V::I(d)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.AnswerCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// MAX(v.CreationDate) AS LastVoteDate,
// MAX(ph.CreationDate) AS LastHistoryDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.AnswerCount, p.Score
// ),
// UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// MAX(b.Date) AS LastBadgeDate
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.ViewCount,
// ps.AnswerCount,
// ps.Score,
// ps.CommentCount,
// ps.LastVoteDate,
// ps.LastHistoryDate,
// ub.BadgeCount,
// ub.LastBadgeDate
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC
// LIMIT 100;
fn q10256(db: &'static So) -> String {
    let uid = uids(db);
    let base = db.post.with((&db.post.creation_date).gt(year_ago())).with((&db.post.origid).select(&uid));
    let pf = stats_fold(db, base, Ident::<Post>::new(), "cvh", &[]);
    let ub = g(db).select(badges_of(db).select(&db.badge.date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(&ub)).drive(|p, (s, b)| v.push((p, s, b)));
    out(v, |&(p, _, _)| score_views(db, p), 100, |&(p, s, (n, m))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "answers", "score"]);
        f.extend([V::I(s.cx), stat_field(&s, "vmax").unwrap(), stat_field(&s, "hmax").unwrap(), V::I(n), if n == 0 { V::Null } else { V::T(m) }]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// p.Id, p.PostTypeId
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
// ps.PostId,
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// us.PostCount AS UserPostCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostTypeId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.VoteCount DESC,
// ps.CommentCount DESC,
// us.PostCount DESC
// LIMIT 100;
fn q10258(db: &'static So) -> String {
    let uid = uids(db);
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let dp = ud(db, UserWhere::All, posts_of(db));
    let us = g(db).select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).opt())).fold((0i64, 0i64), |(u, d), ((a, b), _)| (u + a, d + b));
    let mut v = Vec::new();
    db.post
        .with((&db.post.post_type_id).select(&uid))
        .select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and((&db.post.post_type_id).select(&uid).select((&dp).opt().and(&us))))
        .drive(|_, (((p, c), x), (d, s))| v.push((p, c.unwrap_or(0), x.unwrap_or(0), d.unwrap_or(0), s)));
    out(v, |&(_, c, x, d, _)| (Reverse(x), Reverse(c), Reverse(d)), 100, |&(p, c, x, d, (su, sd))| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([V::I(c), V::I(x), V::I(d), V::I(su), V::I(sd)]);
        f
    })
}

// WITH PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1.0 ELSE 0.0 END) AS UpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1.0 ELSE 0.0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.Reputation
// )
// SELECT
// pa.PostId,
// pa.Title,
// pa.ViewCount,
// pa.Score,
// pa.AnswerCount,
// pa.CommentCount,
// pa.OwnerReputation,
// pa.VoteCount,
// pa.UpVotes,
// pa.DownVotes
// FROM
// PostActivity pa
// ORDER BY
// pa.Score DESC, pa.ViewCount DESC;
fn q10272(db: &'static So) -> String {
    rows(stats_with(db, since(db, year_ago()), "v", &[], &[]).iter().map(|&(p, s, _)| {
        row(stat_fields(db, p, &s, &["id", "title", "views", "score", "answers", "comments", "rep", "#vx", "up_frac", "down_frac"]))
    }))
}

// WITH PostDetails AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// U.DisplayName AS OwnerDisplayName,
// COUNT(C.ID) AS CommentCount,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName
// ),
// VotingDetails AS (
// SELECT
// V.PostId,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Votes V
// GROUP BY
// V.PostId
// )
// SELECT
// PD.PostId,
// PD.Title,
// PD.CreationDate,
// PD.Score,
// PD.ViewCount,
// PD.OwnerDisplayName,
// PD.CommentCount,
// VD.VoteCount,
// VD.UpVoteCount,
// VD.DownVoteCount,
// PD.BadgeCount
// FROM
// PostDetails PD
// LEFT JOIN
// VotingDetails VD ON PD.PostId = VD.PostId
// ORDER BY
// PD.Score DESC, PD.ViewCount DESC
// LIMIT 100;
fn q10276(db: &'static So) -> String {
    let bd = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    let vd = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cb", &[])
        .and((&bd).opt())
        .and((&db.post.origid).select(&vd).opt())
        .drive(|p, ((s, b), x)| v.push((p, s, b.unwrap_or(0), x)));
    out(v, |&(p, _, _, _)| score_views(db, p), 100, |&(p, s, b, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.push(V::I(s.cx));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f.push(V::I(b));
        f
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// AVG(vote_score.VoteCount) AS AverageVotes,
// AVG(c.CommentCount) AS AverageComments,
// MAX(p.CreationDate) AS LatestPostDate,
// MIN(p.CreationDate) AS EarliestPostDate
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId) AS vote_score ON p.Id = vote_score.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) AS c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10304(db: &'static So) -> String {
    let Post { score, creation_date, .. } = &db.post;
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let f = by_key(
        db.post.iq(),
        name(db),
        score.and(creation_date).and((&vf).opt()).and((&cf).opt()),
        [0i64, 0, 0, 0, 0, 0, i64::MIN, i64::MAX],
        |a, (((s, cd), x), c)| {
            [a[0] + 1, a[1] + s, a[2] + x.is_some() as i64, a[3] + x.unwrap_or(0), a[4] + c.is_some() as i64, a[5] + c.unwrap_or(0), a[6].max(cd), a[7].min(cd)]
        },
    );
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |x| Reverse(x.1[0]), 0, |&(k, a)| vec![V::S(k), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), avg(a[5], a[4]), V::T(a[6]), V::T(a[7])])
}

// WITH PostVoteCounts AS (
// SELECT
// P.PostTypeId,
// COUNT(V.Id) AS VoteCount,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.PostTypeId
// ),
// PostTypeDetails AS (
// SELECT
// PT.Name AS PostTypeName,
// PVC.VoteCount,
// PVC.UpVoteCount,
// PVC.DownVoteCount
// FROM
// PostTypes PT
// JOIN
// PostVoteCounts PVC ON PT.Id = PVC.PostTypeId
// )
// SELECT
// PTD.PostTypeName,
// PTD.VoteCount,
// PTD.UpVoteCount,
// PTD.DownVoteCount
// FROM
// PostTypeDetails PTD
// ORDER BY
// PTD.VoteCount DESC;
fn q10308(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), &db.post.post_type, "v", &[]).drive(|t, s| v.push((t, s)));
    out(v, |x| Reverse(x.1.vx), 0, |&(t, s)| vec![V::S(db.post_type.name.get(t).unwrap()), V::I(s.vx), V::I(s.up), V::I(s.down)])
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
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
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// )
// SELECT
// pd.PostId,
// pd.Title,
// pd.PostCreationDate,
// pd.ViewCount,
// pd.Score,
// pd.AnswerCount,
// pd.CommentCount,
// pd.VoteCount,
// pd.OwnerDisplayName,
// pd.OwnerReputation
// FROM
// PostDetails pd
// ORDER BY
// pd.Score DESC,
// pd.ViewCount DESC
// LIMIT 100;
fn q10313(db: &'static So) -> String {
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| score_views(db, p), 100, |&((p, c), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.extend([V::I(c), V::I(x)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
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
// u.Reputation AS OwnerReputation,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AvgUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AvgDownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.Reputation
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
// ps.TotalComments,
// ps.TotalVotes,
// ps.AvgUpVotes,
// ps.AvgDownVotes
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q10315(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(
        db,
        stats_with(db, db.post.iq(), "cv", &[], &[&c, &x]),
        |_, _| 0,
        0,
        &["id", "title", "created", "score", "views", "answers", "comments", "rep", "#d0", "#d1", "up_frac", "down_frac"],
    )
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId
// ),
// UserReputation AS (
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
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// ur.Reputation,
// ur.BadgeCount
// FROM
// PostStats ps
// JOIN
// Users u ON ps.OwnerUserId = u.Id
// JOIN
// UserReputation ur ON u.Id = ur.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// FETCH FIRST 100 ROWS ONLY;
fn q10318(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(badges_per_user(db))).drive(|p, (s, b)| v.push((p, s, b)));
    out(v, |&(p, _, _)| (score_views(db, p), db.post.origid.get(p).unwrap()), 100, |&(p, s, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx)]);
        f.extend(post_fields(db, p, &["rep"]));
        f.push(V::I(b));
        f
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(COALESCE(pc.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(v.UpVotes, 0)) AS TotalUpVotes,
// SUM(COALESCE(v.DownVotes, 0)) AS TotalDownVotes,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) pc ON p.Id = pc.PostId
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
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10334(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_key(
        db.post.iq(),
        name(db),
        score.and(view_count.opt()).and(comments_per_post(db)).and(votes_of_type(db, 2)).and(votes_of_type(db, 3)),
        [0i64; 7],
        |a, ((((s, w), c), u), d)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c, a[5] + u, a[6] + d],
    );
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |x| Reverse(x.1[0]), 0, |&(k, a)| vec![V::S(k), V::I(a[0]), V::I(a[4]), V::I(a[5]), V::I(a[6]), avg(a[1], a[0]), avg(a[3], a[2])])
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.CreationDate,
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate, U.Id, U.Reputation
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.Score,
// PS.ViewCount,
// PS.CreationDate,
// PS.UserId,
// PS.Reputation,
// US.PostCount,
// US.GoldBadges,
// US.SilverBadges,
// US.BronzeBadges,
// PS.CommentCount,
// PS.VoteCount,
// PS.Upvotes,
// PS.Downvotes
// FROM
// PostStats PS
// JOIN
// UserStats US ON PS.UserId = US.UserId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q10338(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[])
        .and((&c).opt())
        .and((&x).opt())
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&uf).and((&dp).opt())))
        .drive(|p, (((s, c), x), ((u, us), d))| v.push((p, s, c.unwrap_or(0), x.unwrap_or(0), u, us, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, c, x, u, us, d)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend([user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d)]);
        f.extend(["#gold", "#silver", "#bronze"].iter().map(|k| ustat_field(&us, k)));
        f.extend([V::I(c), V::I(x), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
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
// p.Id, p.PostTypeId, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// COUNT(ph.Id) AS HistoryCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// PostHistory ph ON u.Id = ph.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.DisplayName AS AuthorDisplayName,
// us.Reputation AS AuthorReputation,
// us.BadgeCount AS AuthorBadgeCount,
// us.HistoryCount AS AuthorHistoryCount
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostTypeId = 1 AND ps.PostId = us.UserId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC
// LIMIT 100;
fn q10364(db: &'static So) -> String {
    let uid = uids(db);
    let qid: HashIdx<i64, Id<Post>> = questions_only(db).select(&db.post.origid).inv().collect();
    let by_editor: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let us = db
        .user
        .with((&db.user.origid).select(&qid))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and((&by_editor).opt()))
        .fold((0i64, 0i64), |(b, h), (bi, hi)| (b + bi.is_some() as i64, h + hi.is_some() as i64));
    let pf = stats_fold(db, questions_only(db).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, _, _)| views_score(db, p), 100, |&(p, s, u, (b, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(h)]);
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 8 THEN 1 ELSE 0 END) AS BountyStartCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.OwnerUserId,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 8 THEN 1 ELSE 0 END) AS BountyStartCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.OwnerUserId, p.CreationDate
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.UpVoteCount AS TotalUpVotes,
// us.DownVoteCount AS TotalDownVotes,
// us.BountyStartCount AS TotalBountyStarts,
// ps.PostId,
// ps.Title AS PostTitle,
// ps.CommentCount AS TotalComments,
// ps.UpVoteCount AS PostUpVotes,
// ps.DownVoteCount AS PostDownVotes,
// ps.BountyStartCount AS PostBountyStarts
// FROM
// UserStatistics us
// JOIN
// PostStatistics ps ON us.UserId = ps.OwnerUserId
// ORDER BY
// us.BadgeCount DESC,
// ps.CommentCount DESC
// LIMIT 100;
fn q10370(db: &'static So) -> String {
    let bv = by_voter(db);
    let us = g(db).select(badges_of(db).opt().and((&bv).select(&db.vote.vote_type_id).opt())).fold([0i64; 4], |a, (b, t)| {
        [a[0] + b.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (t == Some(8)) as i64]
    });
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(_, s, _, a)| (Reverse(a[0]), Reverse(s.cx)), 100, |&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(a.iter().map(|&x| V::I(x)));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(s.by_vt[8])]);
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.PostTypeId
// )
// SELECT
// pst.PostId,
// pst.Title,
// pst.PostTypeId,
// pst.CommentCount,
// pst.VoteCount,
// pst.BadgeCount,
// pst.UpVoteCount,
// pst.DownVoteCount,
// u.Reputation,
// u.CreationDate AS UserCreationDate
// FROM
// PostStatistics pst
// JOIN
// Users u ON pst.PostId = u.Id
// ORDER BY
// pst.VoteCount DESC, pst.CommentCount DESC;
fn q10378(db: &'static So) -> String {
    let uid = uids(db);
    let x = per_post_distinct(db, votes_of(db));
    let bu = badges_by_uid(db);
    let b = per_post_distinct(db, (&db.post.owner_user_id).select(&bu));
    let mut v = Vec::new();
    cvb(db, since(db, date(2023, 1, 1)).with((&db.post.origid).select(&uid)), &bu)
        .and((&x).opt())
        .and((&b).opt())
        .and((&db.post.origid).select(&uid))
        .drive(|p, (((s, x), b), u)| v.push((p, s, x.unwrap_or(0), b.unwrap_or(0), u)));
    rows(v.iter().map(|&(p, s, x, b, u)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id"]);
        f.extend([V::I(s[0]), V::I(x), V::I(b), V::I(s[2]), V::I(s[3]), user_col(db, u, "rep"), user_col(db, u, "ucreated")]);
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
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// EXISTS (SELECT 1 FROM PostHistory ph WHERE ph.PostId = p.Id AND ph.PostHistoryTypeId = 10) AS IsClosed
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.Score DESC;
fn q10391(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, children_of(db));
    let closed = db
        .post_history
        .with((&db.post_history.post_history_type_id).eq(10))
        .select(&db.post_history.post)
        .inv()
        .dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2022, 1, 1)), Ident::<Post>::new(), "cva", &[])
        .and((&c).opt())
        .and((&a).opt())
        .and(closed)
        .drive(|p, (((s, c), a), h)| v.push((p, s, c.unwrap_or(0), a.unwrap_or(0), h)));
    rows(v.iter().map(|&(p, s, c, a, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(a), V::I(s.up), V::I(s.down), V::B(h > 0)]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation
// FROM Posts P
// JOIN Users U ON P.OwnerUserId = U.Id
// ),
// UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostsCreated,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCreated,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCreated
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName
// ),
// VotingSummary AS (
// SELECT
// V.UserId,
// COUNT(V.Id) AS VotesCount,
// SUM(CASE WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes V
// JOIN VoteTypes VT ON V.VoteTypeId = VT.Id
// GROUP BY V.UserId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.AnswerCount,
// PS.CommentCount,
// PS.OwnerDisplayName,
// PS.OwnerReputation,
// UA.PostsCreated,
// UA.QuestionsCreated,
// UA.AnswersCreated,
// VS.VotesCount,
// VS.UpVotes,
// VS.DownVotes
// FROM PostStatistics PS
// LEFT JOIN UserActivity UA ON PS.OwnerDisplayName = UA.DisplayName
// LEFT JOIN VotingSummary VS ON UA.UserId = VS.UserId
// ORDER BY PS.CreationDate DESC;
fn q10406(db: &'static So) -> String {
    let ua = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let vs = vote_named(db);
    let names = by_name(db);
    let mut v = Vec::new();
    owned(db)
        .select(Ident::<Post>::new().and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(Ident::<User>::new().and(&ua).and((&vs).opt())).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "rep"]);
        match x {
            Some(((_, a), s)) => {
                f.extend(["#n", "#q", "#a"].iter().map(|c| ustat_field(&a, c)));
                f.extend((0..3).map(|i| oint(s.map(|s| s[i]))));
            }
            None => f.extend(nulls(6)),
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
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// MAX(p.LastActivityDate) AS LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
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
// ps.LastActivityDate
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q10418(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, answers_of(db));
    out(stats_with(db, since(db, year_ago()), "cvA", &[], &[&c, &a]), |&(p, _, _)| (score_views(db, p), db.post.origid.get(p).unwrap()), 100, |&(p, s, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(d[0]), V::I(d[1]), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["activity"]));
        f
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoredPosts,
// SUM(CASE WHEN p.ViewCount > 1000 THEN 1 ELSE 0 END) AS HighViewCountPosts,
// AVG(COALESCE(EXTRACT(EPOCH FROM p.LastActivityDate - p.CreationDate), 0)) AS AvgTimeToActivity,
// COUNT(DISTINCT p.OwnerUserId) AS UniquePostOwners,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(v.BountyAmount) AS TotalBountySpent
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10428(db: &'static So) -> String {
    let Post { score, view_count, last_activity_date, creation_date, owner_user_id, .. } = &db.post;
    let f = by_key(
        db.post.iq(),
        name(db),
        score.and(view_count.opt()).and(last_activity_date).and(creation_date).and(comments_of(db).opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()),
        ([0i64; 5], 0i128),
        |(a, d), (((((s, w), la), cd), _), b)| {
            let b = b.flatten();
            (
                [a[0] + 1, a[1] + (s > 0) as i64, a[2] + w.is_some_and(|w| w > 1000) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)],
                d + (la - cd) as i128,
            )
        },
    );
    let owners = db.post.group_by(name(db)).select(owner_user_id).count_distinct();
    let dc = db.post.group_by(name(db)).select(comments_of(db)).count_distinct();
    let mut v = Vec::new();
    (&f).and((&owners).opt()).and((&dc).opt()).drive(|k, ((a, o), c)| v.push((k, a, o.unwrap_or(0), c.unwrap_or(0))));
    out(v, |x| Reverse(x.1.0[0]), 0, |&(k, (a, d), o, c)| {
        vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(d as f64 / a[0] as f64 / 1e6), V::I(o), V::I(c), nullable(a[4], a[3])]
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// SUM(COALESCE(v.UpVotes, 0)) AS TotalUpVotes,
// SUM(COALESCE(v.DownVotes, 0)) AS TotalDownVotes,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// AverageScore DESC;
fn q10433(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_key(
        db.post.iq(),
        name(db),
        score.and(view_count.opt()).and(comments_per_post(db)).and(votes_of_type(db, 2)).and(votes_of_type(db, 3)),
        [0i64; 6],
        |a, ((((s, w), c), u), d)| [a[0] + 1, a[1] + u, a[2] + d, a[3] + c, a[4] + w.unwrap_or(0), a[5] + s],
    );
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[5], a[0])])))
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
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
// UserDetails AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(b.Class) AS TotalBadges,
// COUNT(p.Id) AS TotalPosts
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
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.ViewCount,
// pd.CommentCount,
// pd.VoteCount,
// ud.DisplayName AS OwnerDisplayName,
// ud.TotalBadges,
// ud.TotalPosts
// FROM
// PostDetails pd
// JOIN
// UserDetails ud ON pd.PostId = ud.UserId
// ORDER BY
// pd.ViewCount DESC
// LIMIT 10;
fn q10437(db: &'static So) -> String {
    let uid = uids(db);
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uf)))
        .drive(|p, (s, (u, us))| v.push((p, s, u, us)));
    out(v, |&(p, _, _, _)| views_desc(db, p), 10, |&(p, s, u, us)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), user_col(db, u, "name"), ustat_field(&us, "bclass_sum"), ustat_field(&us, "#n")]);
        f
    })
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// AVG(p.ViewCount) AS AvgViewCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// TotalComments,
// AvgViewCount,
// ROUND((TotalComments::decimal / NULLIF(TotalPosts, 0)), 2) AS AvgCommentsPerPost
// FROM
// PostMetrics
// ORDER BY
// TotalPosts DESC;
fn q10438(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.view_count).opt().and(comments_of(db).opt()), [0i64; 3], |a, (w, _)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]
    });
    let dc = db.post.group_by(name(db)).select(comments_of(db)).count_distinct();
    let mut v = Vec::new();
    (&f).and((&dc).opt()).drive(|k, (a, c)| v.push((k, a, c.unwrap_or(0))));
    out(v, |x| Reverse(x.1[0]), 0, |&(k, a, c)| vec![V::S(k), V::I(a[0]), V::I(c), avg(a[2], a[1]), V::F(round2(c as f64 / a[0] as f64))])
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.ViewCount,
// P.Score,
// P.CreationDate,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT A.Id) AS AnswerCount,
// MAX(V.CreationDate) AS LastVoteDate
// FROM
// Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
// LEFT JOIN Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title, P.ViewCount, P.Score, P.CreationDate
// ),
// UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(U.UpVotes) AS TotalUpVotes,
// SUM(U.DownVotes) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.AnswerCount,
// US.UserId,
// US.DisplayName,
// US.PostCount,
// US.TotalUpVotes,
// US.TotalDownVotes,
// PS.LastVoteDate
// FROM
// PostStatistics PS
// JOIN
// UserStatistics US ON PS.PostId = US.PostCount
// ORDER BY
// PS.ViewCount DESC
// LIMIT 100;
fn q10440(db: &'static So) -> String {
    let base = || since(db, date(2023, 10, 1));
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, answers_of(db));
    let us = g(db).select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).opt())).fold((0i64, 0i64, 0i64), |(n, u, d), ((a, b), p)| {
        (n + p.is_some() as i64, u + a, d + b)
    });
    let by_count: HashIdx<i64, Id<User>> = (&us).map(|(n, _, _)| n).inv().collect();
    let mut v = Vec::new();
    stats_fold(db, base(), Ident::<Post>::new(), "v", &[])
        .and((&c).opt())
        .and((&a).opt())
        .and((&db.post.origid).select(&by_count).select(Ident::<User>::new().and(&us)))
        .drive(|p, (((s, c), a), u)| v.push((p, s, c.unwrap_or(0), a.unwrap_or(0), u)));
    out(v, |&(p, _, _, _, _)| views_desc(db, p), 100, |&(p, s, c, a, (u, (n, su, sd)))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(c), V::I(a), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n), V::I(su), V::I(sd), stat_field(&s, "vmax").unwrap()]);
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
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Score,
// p.ViewCount,
// p.CommentCount,
// p.CreationDate,
// pt.Name AS PostType,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// ),
// VoteCounts AS (
// SELECT
// p.Id AS PostId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id
// )
// SELECT
// up.UserId,
// u.DisplayName,
// up.PostCount,
// up.QuestionCount,
// up.AnswerCount,
// ps.PostId,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.CreationDate,
// ps.PostType,
// ps.OwnerDisplayName,
// vc.UpVoteCount,
// vc.DownVoteCount
// FROM
// UserPostCounts up
// JOIN
// Users u ON up.UserId = u.Id
// JOIN
// PostStats ps ON ps.OwnerDisplayName = u.DisplayName
// JOIN
// VoteCounts vc ON vc.PostId = ps.PostId
// ORDER BY
// up.PostCount DESC, up.QuestionCount DESC, up.AnswerCount DESC;
fn q10450(db: &'static So) -> String {
    let ua = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let names = by_name(db);
    let mut v = Vec::new();
    (&ua)
        .and((&db.user.display_name).select(&names).select(posts_of(db)).select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3))))
        .drive(|u, (a, ((p, x), y))| v.push((u, a, p, x, y)));
    rows(v.iter().map(|&(u, a, p, x, y)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a"].iter().map(|c| ustat_field(&a, c)));
        f.extend(post_fields(db, p, &["id", "score", "views", "comments", "created", "type", "owner"]));
        f.extend([V::I(x), V::I(y)]);
        row(f)
    }))
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.OwnerUserId,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.LastActivityDate,
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// ),
// VoteStats AS (
// SELECT
// v.PostId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount
// FROM
// Votes v
// GROUP BY
// v.PostId
// ),
// CommentCounts AS (
// SELECT
// c.PostId,
// COUNT(c.Id) AS TotalComments
// FROM
// Comments c
// GROUP BY
// c.PostId
// )
// SELECT
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.Score,
// pd.ViewCount,
// pd.OwnerReputation,
// pd.OwnerDisplayName,
// COALESCE(vs.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(vs.DownVoteCount, 0) AS DownVoteCount,
// COALESCE(cc.TotalComments, 0) AS TotalComments,
// pd.AnswerCount,
// pd.CommentCount,
// pd.FavoriteCount,
// pd.LastActivityDate
// FROM
// PostDetails pd
// LEFT JOIN
// VoteStats vs ON pd.PostId = vs.PostId
// LEFT JOIN
// CommentCounts cc ON pd.PostId = cc.PostId
// ORDER BY
// pd.CreationDate DESC;
fn q10467(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1))
        .select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and(comments_per_post(db)))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, u), d), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep", "owner"]);
        f.extend([V::I(u), V::I(d), V::I(c)]);
        f.extend(post_fields(db, p, &["answers", "comments", "favorites", "activity"]));
        row(f)
    }))
}

// PostStats's COUNT(c.Id) is never selected; its GROUP BY keeps one row per post.
// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCountTotal
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName, u.Reputation
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
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
// ps.Score,
// ps.AnswerCount,
// ps.CommentCount,
// ps.OwnerDisplayName,
// ps.OwnerReputation,
// us.BadgeCount,
// us.TotalBounty
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerDisplayName = us.DisplayName
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q10469(db: &'static So) -> String {
    let bv = by_voter(db);
    let us = g(db).select(badges_of(db).opt().and((&bv).select((&db.vote.bounty_amount).opt()).opt())).fold((0i64, 0i64), |(b, s), (bi, x)| {
        (b + bi.is_some() as i64, s + x.flatten().unwrap_or(0))
    });
    let names = by_name(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(&us))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, (b, s))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "rep"]);
        f.extend([V::I(b), V::I(s)]);
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes,
// COUNT(DISTINCT u.Id) AS TotalUsers
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(Id) AS VoteCount FROM Votes GROUP BY PostId) v ON v.PostId = p.Id
// LEFT JOIN
// Users u ON u.Id = p.OwnerUserId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10470(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let f = db
        .post_type
        .group_by(&db.post_type.name)
        .select((&of_type).select((&db.post.score).and((&vf).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((s, x)) => [a[0] + 1, a[1] + s, a[2] + x.unwrap_or(0)],
            None => a,
        });
    let du = db.post_type.group_by(&db.post_type.name).select((&of_type).select(&db.post.owner_user)).count_distinct();
    let mut v = Vec::new();
    (&f).and((&du).opt()).drive(|k, (a, d)| v.push((k, a, d.unwrap_or(0))));
    out(v, |x| Reverse(x.1[0]), 0, |&(k, a, d)| vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(d)])
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.OwnerUserId,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
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
// ),
// CommentStats AS (
// SELECT
// c.PostId,
// COUNT(c.Id) AS TotalComments
// FROM
// Comments c
// GROUP BY
// c.PostId
// )
// SELECT
// ps.PostId,
// ps.PostTypeId,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.OwnerReputation,
// COALESCE(vs.UpVotes, 0) AS UpVotes,
// COALESCE(vs.DownVotes, 0) AS DownVotes,
// COALESCE(cs.TotalComments, 0) AS TotalComments
// FROM
// PostStats ps
// LEFT JOIN
// VoteStats vs ON ps.PostId = vs.PostId
// LEFT JOIN
// CommentStats cs ON ps.PostId = cs.PostId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q10481(db: &'static So) -> String {
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and(comments_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, u), d), c)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "views", "score", "answers", "comments", "favorites", "rep"]);
        f.extend([V::I(u), V::I(d), V::I(c)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(b.Id) AS BadgeCount,
// MAX(p.CreationDate) AS LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(p.ViewCount) AS TotalViews,
// AVG(u.Reputation) AS AverageReputation,
// COUNT(b.Id) AS BadgeCount
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
// ps.CommentCount,
// ps.VoteCount,
// ps.BadgeCount AS PostBadges,
// ps.LastActivityDate,
// us.UserId,
// us.DisplayName,
// us.TotalViews,
// us.AverageReputation,
// us.BadgeCount AS UserBadges
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.LastActivityDate DESC
// LIMIT 100;
fn q10490(db: &'static So) -> String {
    let uid = uids(db);
    let x = per_post_distinct(db, votes_of(db));
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    cvb(db, since(db, year_ago()).with((&db.post.origid).select(&uid)), &badges_by_uid(db))
        .and((&x).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uf)))
        .drive(|p, ((s, x), (u, us))| v.push((p, s, x.unwrap_or(0), u, us)));
    out(v, |&(p, _, _, _, _)| newest(db, p), 100, |&(p, s, x, u, us)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s[0]), V::I(x), V::I(s[4])]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(["views_sum", "rep_avg", "#bx"].iter().map(|c| ustat_field(&us, c)));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.UserId) AS VoteCount,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON v.UserId = u.Id
// GROUP BY
// p.Id, pt.Name
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(*) AS HistoryCount,
// COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
// COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.PostType,
// ps.CommentCount,
// ps.VoteCount,
// ps.AvgUserReputation,
// phs.HistoryCount,
// phs.CloseCount,
// phs.ReopenCount
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.VoteCount DESC, ps.CommentCount DESC;
fn q10491(db: &'static So) -> String {
    let pf = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.user).select(&db.user.reputation).opt()).opt()))
        .fold((0i64, 0i64, 0i64), |(c, n, s), (ci, r)| {
            let r = r.flatten();
            (c + ci.is_some() as i64, n + r.is_some() as i64, s + r.unwrap_or(0))
        });
    let dv = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    let hf = history_types(db);
    let mut v = Vec::new();
    (&pf).and((&dv).opt()).and((&hf).opt()).drive(|p, ((a, d), h)| v.push((p, a, d.unwrap_or(0), h)));
    rows(v.iter().map(|&(p, (c, n, s), d, h)| {
        let mut f = post_fields(db, p, &["id", "type"]);
        f.extend([V::I(c), V::I(d), avg(s, n)]);
        f.extend((0..3).map(|i| oint(h.map(|h| h[i]))));
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
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COUNT(b.Id) AS BadgeCount
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.BadgeCount
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q10493(db: &'static So) -> String {
    let mut v = Vec::new();
    cvb(db, questions_only(db), &badges_by_uid(db)).drive(|p, s| v.push((p, s)));
    out(v, |&(p, _)| score_views(db, p), 100, |&(p, s)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s[0]), V::I(s[2]), V::I(s[3]), V::I(s[4])]);
        f
    })
}

// SELECT
// u.DisplayName AS UserDisplayName,
// p.Title,
// p.CreationDate AS PostCreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(voteCount) AS TotalVotes,
// p.ViewCount,
// p.Score,
// CASE
// WHEN p.PostTypeId = 1 THEN 'Question'
// WHEN p.PostTypeId = 2 THEN 'Answer'
// ELSE 'Other'
// END AS PostType
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS voteCount FROM Votes GROUP BY PostId) v ON v.PostId = p.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// u.DisplayName, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId
// ORDER BY
// p.CreationDate DESC;
fn q10528(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let Post { owner_user, title, creation_date, view_count, score, post_type_id, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(view_count.opt()).and(score).and(post_type_id);
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1))
        .group_by(key)
        .select(comments_of(db).opt().and((&vf).opt()))
        .fold((0i64, 0i64, 0i64), |(c, n, s), (ci, x)| (c + ci.is_some() as i64, n + x.is_some() as i64, s + x.unwrap_or(0)))
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(((((( u, t), cd), w), s), ty), (c, n, x))| {
        row(vec![
            V::S(u),
            ostr(t),
            V::T(cd),
            V::I(c),
            nullable(x, n),
            oint(w),
            V::I(s),
            V::S(match ty {
                1 => "Question",
                2 => "Answer",
                _ => "Other",
            }),
        ])
    }))
}

// --- users ------------------------------------------------------------------

// WITH UserStats AS (
// SELECT
// Id AS UserId,
// DisplayName,
// Reputation,
// CreationDate,
// UpVotes,
// DownVotes,
// Views,
// CAST((UpVotes - DownVotes) AS INT) AS NetVotes
// FROM
// Users
// ),
// PostStats AS (
// SELECT
// Id AS PostId,
// OwnerUserId,
// Score,
// ViewCount,
// CommentCount,
// AnswerCount,
// CreationDate,
// LastActivityDate,
// Title,
// (SELECT COUNT(*) FROM Comments WHERE PostId = Posts.Id) AS TotalComments,
// (SELECT COUNT(*) FROM Votes WHERE PostId = Posts.Id) AS TotalVotes
// FROM
// Posts
// WHERE
// CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// ),
// UserPostStats AS (
// SELECT
// U.UserId,
// U.DisplayName,
// COUNT(P.PostId) AS PostCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.ViewCount) AS AverageViewsPerPost
// FROM
// UserStats U
// JOIN
// PostStats P ON U.UserId = P.OwnerUserId
// GROUP BY
// U.UserId, U.DisplayName
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.PostCount,
// UPS.TotalScore,
// UPS.TotalViews,
// UPS.AverageViewsPerPost,
// U.Reputation,
// U.CreationDate
// FROM
// UserPostStats UPS
// JOIN
// Users U ON UPS.UserId = U.Id
// ORDER BY
// UPS.TotalScore DESC,
// UPS.TotalViews DESC
// LIMIT 100;
fn q10254(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    owned_since(db, month_ago())
        .group_by(&db.post.owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)])
        .drive(|u, a| v.push((u, a)));
    out(v, |&(u, a)| (Reverse(a[1]), a[2] == 0, Reverse(a[3]), db.user.origid.get(u).unwrap()), 100, |&(u, a)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[3], a[2]), user_col(db, u, "rep"), user_col(db, u, "ucreated")]
    })
}

// WITH PostCounts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// CommentCounts AS (
// SELECT
// c.UserId,
// COUNT(c.Id) AS TotalComments
// FROM
// Comments c
// GROUP BY
// c.UserId
// )
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COALESCE(pc.TotalPosts, 0) AS TotalPosts,
// COALESCE(pc.AverageScore, 0) AS AverageScore,
// COALESCE(cc.TotalComments, 0) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// PostCounts pc ON u.Id = pc.OwnerUserId
// LEFT JOIN
// CommentCounts cc ON u.Id = cc.UserId
// ORDER BY
// u.Reputation DESC;
fn q10283(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pc).opt()).and(comments_per_user(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), c)| {
        let (n, s) = p.unwrap_or((0, 0));
        row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(n), or0(s, n), V::I(c)])
    }))
}

// WITH UserPostActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostVoteStatistics AS (
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
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.Questions,
// u.Answers,
// u.LastPostDate,
// COALESCE(pvs.TotalVotes, 0) AS TotalVotes,
// COALESCE(pvs.UpVotes, 0) AS UpVotes,
// COALESCE(pvs.DownVotes, 0) AS DownVotes
// FROM
// UserPostActivity u
// LEFT JOIN
// PostVoteStatistics pvs ON u.UserId = pvs.PostId
// ORDER BY
// u.TotalPosts DESC
// LIMIT 100;
fn q10297(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let pid = pids(db);
    let pv = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "v", &[]);
    let uf = g(db).select(posts_of(db).select(post_type_id.and(creation_date)).opt()).fold((0i64, 0i64, 0i64, i64::MIN), |(n, q, a, m), p| match p {
        Some((t, cd)) => (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, m.max(cd)),
        None => (n, q, a, m),
    });
    let mut v = Vec::new();
    (&uf).and((&db.user.origid).select(&pid).select(&pv).opt()).drive(|u, (a, s)| v.push((u, a, s)));
    out(v, |&(_, (n, _, _, _), _)| Reverse(n), 100, |&(u, (n, q, a, m), s)| {
        let o = |f: fn(&Stats) -> i64| V::I(s.as_ref().map_or(0, f));
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(n),
            V::I(q),
            V::I(a),
            if n == 0 { V::Null } else { V::T(m) },
            o(|s| s.vx),
            o(|s| s.up),
            o(|s| s.down),
        ]
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// PostActivityStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes,
// COUNT(DISTINCT h.UserId) AS TotalEdits
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory h ON p.Id = h.PostId
// GROUP BY
// p.Id
// )
// SELECT
// ups.UserId,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalScore,
// pas.TotalComments,
// pas.TotalVotes,
// pas.TotalEdits
// FROM
// UserPostStats ups
// LEFT JOIN
// PostActivityStats pas ON ups.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pas.PostId LIMIT 1)
// ORDER BY
// ups.TotalPosts DESC, ups.TotalScore DESC;
fn q10301(db: &'static So) -> String {
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let pas = stats_fold(db, owned(db), Ident::<Post>::new(), "cvh", &[]);
    let ed = owned(db).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.user_id)).count_distinct();
    let mut v = Vec::new();
    (&uf).and(posts_of(db).select((&pas).and((&ed).opt())).opt()).drive(|u, (a, x)| v.push((u, a, x)));
    rows(v.iter().map(|&(u, a, x)| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(["#n", "#q", "#a", "score_sum"].iter().map(|c| ustat_field(&a, c)));
        match x {
            Some((s, e)) => f.extend([V::I(s.cx), V::I(s.vx), V::I(e.unwrap_or(0))]),
            None => f.extend(nulls(3)),
        }
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// MAX(p.CreationDate) AS MostRecentPost,
// AVG(p.Score) AS AveragePostScore,
// SUM(c.CommentCount) AS TotalComments,
// SUM(v.VoteCount) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q10303(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    g(db)
        .select(posts_of(db).select(post_type_id.and(creation_date).and(score).and((&cf).opt()).and((&vf).opt())).opt())
        .fold([0i64, 0, 0, i64::MIN, 0, 0, 0, 0, 0], |a, p| match p {
            Some(((((t, cd), s), c), x)) => [
                a[0] + 1,
                a[1] + (t == 1) as i64,
                a[2] + (t == 2) as i64,
                a[3].max(cd),
                a[4] + s,
                a[5] + c.is_some() as i64,
                a[6] + c.unwrap_or(0),
                a[7] + x.is_some() as i64,
                a[8] + x.unwrap_or(0),
            ],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            if a[0] == 0 { V::Null } else { V::T(a[3]) },
            avg(a[4], a[0]),
            nullable(a[6], a[5]),
            nullable(a[8], a[7]),
        ])
    }))
}

// WITH UserPostStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserBadgeStatistics AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS TotalBadges,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
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
// UPS.TotalScore,
// UPS.TotalViews,
// COALESCE(UBS.TotalBadges, 0) AS TotalBadges,
// COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStatistics UPS
// LEFT JOIN
// UserBadgeStatistics UBS ON UPS.UserId = UBS.UserId
// ORDER BY
// UPS.TotalScore DESC;
fn q10314(db: &'static So) -> String {
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let bf = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bf).opt()).drive(|u, (s, b)| v.push((u, s, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, s, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "score_sum", "views_sum"].iter().map(|c| ustat_field(&s, c)));
        f.extend(b.iter().map(|&x| V::I(x)));
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
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON V.PostId = P.Id AND V.VoteTypeId IN (8, 9)
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
// UPS.TotalBounty,
// COALESCE(UBS.TotalBadges, 0) AS TotalBadges,
// COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats UPS
// LEFT JOIN
// UserBadgeStats UBS ON UPS.UserId = UBS.UserId
// ORDER BY
// UPS.TotalPosts DESC
// LIMIT 100;
fn q10325(db: &'static So) -> String {
    let uf = user_stats_fold_v(db, Ident::<User>::new(), UserWhere::All, "v", any_post, &[8, 9]);
    let bf = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bf).opt()).drive(|u, (s, b)| v.push((u, s, b.unwrap_or([0; 4]))));
    out(v, |(_, s, _)| Reverse(s.n), 100, |&(u, s, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "bounty_sum0"].iter().map(|c| ustat_field(&s, c)));
        f.extend(b.iter().map(|&x| V::I(x)));
        f
    })
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties,
// SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// AVG(COALESCE(p.Score, 0)) AS AveragePostScore,
// AVG(COALESCE(c.Score, 0)) AS AverageCommentScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, u.CreationDate;
fn q10372(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    users_rows(
        db,
        users_stats_with(db, UserWhere::All, "cv", any_post, &[], &[&dp, &dc]),
        |_, _, _| 0,
        0,
        &["uid", "name", "rep", "ucreated", "#d0", "#d1", "#q", "#a", "bounty_sum0", "#vx", "score_avg_rows", "cscore_avg_rows"],
    )
}

// WITH UserBadgeCounts AS (
// SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// ),
// UserPostCounts AS (
// SELECT OwnerUserId, COUNT(*) AS PostCount
// FROM Posts
// GROUP BY OwnerUserId
// ),
// UserVoteCounts AS (
// SELECT UserId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY UserId
// ),
// UserCommentCounts AS (
// SELECT UserId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY UserId
// ),
// UserStats AS (
// SELECT u.Id AS UserId,
// u.DisplayName,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(up.PostCount, 0) AS PostCount,
// COALESCE(uv.VoteCount, 0) AS VoteCount,
// COALESCE(uc.CommentCount, 0) AS CommentCount
// FROM Users u
// LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId
// LEFT JOIN UserPostCounts up ON u.Id = up.OwnerUserId
// LEFT JOIN UserVoteCounts uv ON u.Id = uv.UserId
// LEFT JOIN UserCommentCounts uc ON u.Id = uc.UserId
// )
// SELECT UserId,
// DisplayName,
// BadgeCount,
// PostCount,
// VoteCount,
// CommentCount,
// (BadgeCount + PostCount + VoteCount + CommentCount) AS TotalActivity
// FROM UserStats
// ORDER BY TotalActivity DESC
// LIMIT 100;
fn q10398(db: &'static So) -> String {
    let pc = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(badges_per_user(db)).and(pc).and(votes_per_user(db)).and(comments_per_user(db)))
        .drive(|_, ((((u, b), p), x), c)| v.push((u, [b, p, x, c])));
    out(v, |(_, a)| Reverse(a.iter().sum::<i64>()), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(a.iter().map(|&x| V::I(x)));
        f.push(V::I(a.iter().sum()));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// AVG(P.Score) AS AverageScore,
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.PostCount,
// UPS.AverageScore,
// UPS.TotalComments
// FROM
// UserPostStats UPS
// ORDER BY
// UPS.PostCount DESC,
// UPS.AverageScore DESC
// LIMIT 100;
fn q10408(db: &'static So) -> String {
    let mut v = Vec::new();
    user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post).drive(|u, s| v.push((u, s)));
    let mean = |s: &UStats| if s.n == 0 { None } else { Some(s.score_sum as f64 / s.n as f64) };
    v.sort_by(|(_, a), (_, b)| {
        b.n.cmp(&a.n).then_with(|| match (mean(a), mean(b)) {
            (Some(x), Some(y)) => y.partial_cmp(&x).unwrap(),
            (x, y) => x.is_none().cmp(&y.is_none()),
        })
    });
    rows(v.iter().take(100).map(|(u, s)| {
        let mut f = vec![user_col(db, *u, "uid"), user_col(db, *u, "name")];
        f.extend(["#n", "score_avg", "#cx"].iter().map(|c| ustat_field(s, c)));
        row(f)
    }))
}

// WITH UserPostStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore
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
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(bb.BadgeCount, 0) AS BadgeCount,
// COALESCE(bb.GoldBadges, 0) AS GoldBadges,
// COALESCE(bb.SilverBadges, 0) AS SilverBadges,
// COALESCE(bb.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// UserPostStatistics ps ON u.Id = ps.UserId
// LEFT JOIN
// UserBadges bb ON u.Id = bb.UserId
// ORDER BY
// TotalScore DESC, PostCount DESC;
fn q10423(db: &'static So) -> String {
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let bf = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bf).opt()).drive(|u, (s, b)| v.push((u, s, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, s, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "score_sum0"].iter().map(|c| ustat_field(&s, c)));
        f.extend(b.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.Reputation
// ),
// AverageReputation AS (
// SELECT AVG(Reputation) AS AvgReputation FROM Users
// ),
// TopUsers AS (
// SELECT
// UserId,
// Reputation,
// BadgeCount,
// QuestionCount,
// AnswerCount,
// UpVoteCount,
// DownVoteCount,
// (Reputation - (SELECT AvgReputation FROM AverageReputation)) AS ReputationDifference
// FROM UserStats
// WHERE Reputation > (SELECT AvgReputation FROM AverageReputation)
// ORDER BY Reputation DESC
// LIMIT 10
// )
// SELECT
// us.DisplayName,
// tu.Reputation,
// tu.BadgeCount,
// tu.QuestionCount,
// tu.AnswerCount,
// tu.UpVoteCount,
// tu.DownVoteCount,
// tu.ReputationDifference
// FROM TopUsers tu
// JOIN Users us ON tu.UserId = us.Id;
fn q10432(db: &'static So) -> String {
    let users: HashIdx<(), Id<User>> = whole(db.user.iq()).collect();
    let mean = (&unit()).select((&users).select(&db.user.reputation).opt()).fold((0i64, 0i64), |(n, s), r| (n + r.is_some() as i64, s + r.unwrap_or(0)));
    let mean = (&mean).filt(|(n, _)| n > 0);
    let above = db.user.select(Ident::<User>::new().and(&db.user.reputation)).cross(&mean).filt(|((_, r), (n, s))| r * n > s);
    let top: MatSet<Id<User>> = whole(&above)
        .select(&above)
        .window(row_number, |((_, r), _)| r, desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let mut v = Vec::new();
    (&top)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 5], |a, (b, p)| {
            let (t, x) = p.map_or((None, None), |(t, x)| (Some(t), x));
            [a[0] + b.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64]
        })
        .cross(&mean)
        .drive(|(u, _), x| v.push((u, x)));
    rows(v.iter().map(|&(u, (a, (n, s)))| {
        let mean = s as f64 / n as f64;
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(rep)];
        f.extend(a.iter().map(|&x| V::I(x)));
        f.push(V::F(rep as f64 - mean));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id as UserId,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// AVG(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN p.Score END) AS AvgAcceptedAnswerScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// )
// SELECT
// UserId,
// Reputation,
// TotalPosts,
// COALESCE(AvgAcceptedAnswerScore, 0) AS AvgAcceptedAnswerScore
// FROM
// UserPostStats
// ORDER BY
// Reputation DESC, TotalPosts DESC
// LIMIT 100;
fn q10443(db: &'static So) -> String {
    let Post { post_type_id, score, accepted_answer_id, .. } = &db.post;
    let mut v = Vec::new();
    g(db)
        .select(posts_of(db).select(post_type_id.and(score).and(accepted_answer_id.opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some(((t, s), acc)) => {
                let hit = t == 2 && acc.is_some();
                [a[0] + 1, a[1] + hit as i64, a[2] + if hit { s } else { 0 }]
            }
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    out(v, |&(u, a)| (rep_desc(db, u), Reverse(a[0])), 100, |&(u, a)| vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a[0]), or0(a[2], a[1])])
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId IN (3, 4, 5, 6, 7, 8) THEN 1 ELSE 0 END) AS WikiPostCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.WikiPostCount,
// us.UpVotesCount,
// us.DownVotesCount,
// us.CommentCount,
// us.BadgeCount,
// (us.UpVotesCount - us.DownVotesCount) AS NetVotes
// FROM
// UserStats us
// ORDER BY
// NetVotes DESC
// LIMIT 10;
fn q10496(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    users_rows(
        db,
        users_stats_with(db, UserWhere::All, "cvb", any_post, &[], &[&dp]),
        |_, s, _| Reverse(s.up - s.down),
        10,
        &["uid", "name", "#d0", "#q", "#a", "#3to8", "#up", "#down", "#cx", "#bx", "#net"],
    )
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// COALESCE(SUM(V.VoteCount), 0) AS TotalVotes,
// COALESCE(SUM(V.UpVotes), 0) AS TotalUpVotes,
// COALESCE(SUM(V.DownVotes), 0) AS TotalDownVotes,
// COALESCE(AVG(P.ViewCount), 0) AS AvgViewCount,
// COALESCE(AVG(P.Score), 0) AS AvgScore,
// COALESCE(AVG(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE NULL END), 0) AS AvgAcceptedAnswers
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS VoteCount,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId) V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q10497(db: &'static So) -> String {
    let Post { view_count, score, post_type_id, .. } = &db.post;
    let vf = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    g(db)
        .select(posts_of(db).select(view_count.opt().and(score).and(post_type_id).and((&vf).opt())).opt())
        .fold([0i64; 8], |a, p| match p {
            Some((((w, s), t), x)) => {
                let x = x.unwrap_or([0; 3]);
                [a[0] + 1, a[1] + x[0], a[2] + x[1], a[3] + x[2], a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s, a[7] + (t == 1) as i64]
            }
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), or0(a[5], a[4]), or0(a[6], a[0]), or0(a[7], a[7])])
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalUpvotes,
// us.TotalDownvotes,
// COALESCE(SUM(CASE WHEN ph.Comment IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalPostEdits
// FROM
// UserStats us
// LEFT JOIN
// PostHistory ph ON us.UserId = ph.UserId
// GROUP BY
// us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.TotalQuestions, us.TotalAnswers, us.TotalUpvotes, us.TotalDownvotes
// ORDER BY
// us.Reputation DESC;
fn q10501(db: &'static So) -> String {
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(0), "v", any_post);
    let dp = ud(db, UserWhere::RepGt(0), posts_of(db));
    let he = db.post_history.group_by(&db.post_history.user).select((&db.post_history.comment).opt()).fold(0i64, |a, c| a + c.is_some() as i64);
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&he).opt()).drive(|u, ((s, d), h)| v.push((u, s, d.unwrap_or(0), h.unwrap_or(0))));
    rows(v.iter().map(|&(u, s, d, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d)];
        f.extend(["#q", "#a", "#up", "#down"].iter().map(|c| ustat_field(&s, c)));
        f.push(V::I(h));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(v.BountyAmount) AS TotalBountyAmount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// u.UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(up.PostCount, 0) AS PostCount,
// COALESCE(up.QuestionCount, 0) AS QuestionCount,
// COALESCE(up.AnswerCount, 0) AS AnswerCount,
// COALESCE(up.TotalScore, 0) AS TotalScore,
// COALESCE(up.AvgViewCount, 0) AS AvgViewCount,
// u.BadgeCount,
// u.TotalBountyAmount
// FROM UserStats u
// LEFT JOIN PostStats up ON u.UserId = up.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore,
// AvgViewCount,
// BadgeCount,
// TotalBountyAmount
// FROM CombinedStats
// ORDER BY Reputation DESC, TotalScore DESC;
fn q10509(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let bv = by_voter(db);
    let us = g(db).select(badges_of(db).opt().and((&bv).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 3], |a, (b, x)| {
        let x = x.flatten();
        [a[0] + b.is_some() as i64, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0)]
    });
    let ps = owned(db).group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&us).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or([0; 6]))));
    rows(v.iter().map(|&(u, b, p)| {
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(p[0]),
            V::I(p[1]),
            V::I(p[2]),
            V::I(p[3]),
            or0(p[5], p[4]),
            V::I(b[0]),
            nullable(b[2], b[1]),
        ])
    }))
}

// --- one-row totals -----------------------------------------------------------

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
// SUM(CASE WHEN Reputation > 0 THEN 1 ELSE 0 END) AS ActiveUsers
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
// p.UniquePostOwners,
// u.TotalUsers,
// u.ActiveUsers,
// c.TotalComments,
// v.TotalVotes
// FROM
// PostCounts p,
// UserCounts u,
// CommentCounts c,
// VoteCounts v;
fn q10478(db: &'static So) -> String {
    let posts: HashIdx<(), Id<Post>> = whole(db.post.iq()).collect();
    let users: HashIdx<(), Id<User>> = whole(db.user.iq()).collect();
    let comments: HashIdx<(), Id<Comment>> = whole(db.comment.iq()).collect();
    let votes: HashIdx<(), Id<Vote>> = whole(db.vote.iq()).collect();
    let pn = (&unit()).select((&posts).opt()).fold(0i64, |a, p| a + p.is_some() as i64);
    let owners = (&unit()).select((&posts).select(&db.post.owner_user_id).opt()).buf_fold(distinct_some);
    let us = (&unit()).select((&users).select(&db.user.reputation).opt()).fold((0i64, 0i64), |(n, a), r| (n + r.is_some() as i64, a + r.is_some_and(|r| r > 0) as i64));
    let cn = (&unit()).select((&comments).opt()).fold(0i64, |a, c| a + c.is_some() as i64);
    let vn = (&unit()).select((&votes).opt()).fold(0i64, |a, v| a + v.is_some() as i64);
    let mut out = Vec::new();
    (&pn).and(&owners).and(&us).and(&cn).and(&vn).drive(|_, x| out.push(x));
    rows(out.iter().map(|&((((pn, owners), (un, act)), cn), vn)| row(vec![V::I(pn), V::I(owners), V::I(un), nullable(act, un), V::I(cn), V::I(vn)])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10231", q10231),
    ("10246", q10246),
    ("10256", q10256),
    ("10258", q10258),
    ("10272", q10272),
    ("10276", q10276),
    ("10304", q10304),
    ("10308", q10308),
    ("10313", q10313),
    ("10315", q10315),
    ("10318", q10318),
    ("10334", q10334),
    ("10338", q10338),
    ("10364", q10364),
    ("10370", q10370),
    ("10378", q10378),
    ("10391", q10391),
    ("10406", q10406),
    ("10418", q10418),
    ("10428", q10428),
    ("10433", q10433),
    ("10437", q10437),
    ("10438", q10438),
    ("10440", q10440),
    ("10450", q10450),
    ("10467", q10467),
    ("10469", q10469),
    ("10470", q10470),
    ("10481", q10481),
    ("10490", q10490),
    ("10491", q10491),
    ("10493", q10493),
    ("10528", q10528),
    ("10254", q10254),
    ("10283", q10283),
    ("10297", q10297),
    ("10301", q10301),
    ("10303", q10303),
    ("10314", q10314),
    ("10325", q10325),
    ("10372", q10372),
    ("10398", q10398),
    ("10408", q10408),
    ("10423", q10423),
    ("10432", q10432),
    ("10443", q10443),
    ("10496", q10496),
    ("10497", q10497),
    ("10501", q10501),
    ("10509", q10509),
    ("10478", q10478),
];
