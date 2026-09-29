use harness::prelude::*;
use std::cmp::Reverse;

fn since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    db.post.with((&db.post.creation_date).ge(d))
}

fn owned_since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.creation_date).ge(d))
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

fn sum_n<Q: Drive<R = i64>>(q: Q) -> (i64, i64) {
    q.fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x))
}

fn badges_distinct(db: &'static So) -> Fold<Id<Post>, i64> {
    per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)))
}

fn ud<R>(db: &'static So, w: UserWhere, r: R) -> Fold<Id<User>, i64>
where
    R: IntoQuery,
    R::Q: Probe<D = Id<User>>,
    ROf<R>: Ord,
{
    user_distinct(db, Ident::<User>::new(), w, r)
}

fn only_type(db: &'static So, t: i64) -> Compose<&'static HashIdx<Id<User>, Id<Post>>, Restrict<Ident<Post>, Filter<&'static Col<Post, i64>, impl Fn(i64) -> bool>>> {
    posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(t)))
}

fn rep_desc(db: &'static So, u: Id<User>) -> Reverse<i64> {
    Reverse(db.user.reputation.get(u).unwrap())
}

// --- users --------------------------------------------------------------------

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// PostCount DESC, u.Reputation DESC;
fn q13479(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, c) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(comments_of(db))));
    let v = users_stats_with(db, w, "cv", any_post, &[], &[&p, &c]);
    rows(v.iter().map(|(u, s, d)| {
        let mut f: Vec<V> = ["uid", "name", "rep"].iter().map(|c| user_col(db, *u, c)).collect();
        f.extend([V::I(d[0]), V::I(d[1]), V::I(s.bounty_sum), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(c.Id, 0)) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// u.Reputation DESC
// LIMIT 10;
fn q14183(db: &'static So) -> String {
    let w = UserWhere::RepGt(0);
    let (p, q, a) = (ud(db, w, posts_of(db)), ud(db, w, only_type(db, 1)), ud(db, w, only_type(db, 2)));
    let f = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt()).and(comments_of(db).select(&db.comment.origid).opt())).opt())
        .fold([0i64; 3], |a, r| match r {
            Some(((s, w), c)) => [a[0] + s, a[1] + w.unwrap_or(0), a[2] + c.unwrap_or(0)],
            None => a,
        });
    let mut v = Vec::new();
    f.and((&p).opt()).and((&q).opt()).and((&a).opt()).drive(|u, (((s, p), q), a)| v.push((u, s, [p, q, a].map(|x| x.unwrap_or(0)))));
    v.sort_by_key(|&(u, _, _)| rep_desc(db, u));
    rows(v.iter().take(10).map(|&(u, s, d)| {
        let mut f: Vec<V> = ["uid", "name", "rep"].iter().map(|c| user_col(db, u, c)).collect();
        f.extend(d.iter().map(|&x| V::I(x)));
        f.extend(s.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(COALESCE(c.Score, 0)) AS TotalCommentScore,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q12245(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, q, a, b) = (ud(db, w, posts_of(db)), ud(db, w, only_type(db, 1)), ud(db, w, only_type(db, 2)), ud(db, w, badges_of(db)));
    let v = users_stats_with(db, w, "cb", any_post, &[], &[&p, &q, &a, &b]);
    users_rows(db, v, |_, _, d| Reverse(d[0]), 10, &["uid", "name", "#d0", "#d1", "#d2", "score_sum", "views_sum", "cscore_sum0", "#d3"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN p.AcceptedAnswerId END) AS TotalAcceptedAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalPosts DESC, TotalScore DESC;
fn q11689(db: &'static So) -> String {
    let w = UserWhere::All;
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let (p, c, x) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(comments_of(db))), ud(db, w, posts_of(db).select(votes_of(db))));
    let acc = ud(db, w, posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2)).select(accepted_answer_id)));
    let v = users_stats_with(db, w, "cv", any_post, &[], &[&p, &c, &x, &acc]);
    users_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "rep", "#d0", "#d1", "#d2", "#d3", "score_sum", "views_sum"])
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT pc.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments pc ON p.Id = pc.PostId
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
// VoteCount
// FROM
// UserActivity
// ORDER BY
// PostCount DESC,
// CommentCount DESC,
// VoteCount DESC
// LIMIT 100;
fn q13887(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, c, x) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(comments_of(db))), ud(db, w, posts_of(db).select(votes_of(db))));
    let v = users_stats_with(db, w, "", any_post, &[], &[&p, &c, &x]);
    users_rows(db, v, |_, _, d| (Reverse(d[0]), Reverse(d[1]), Reverse(d[2])), 100, &["uid", "name", "#d0", "#d1", "#d2"])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT c.Id) AS CommentCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id
// )
// SELECT
// u.DisplayName,
// ups.PostCount,
// ups.VoteCount,
// ups.CommentCount
// FROM
// UserPostStats ups
// JOIN
// Users u ON ups.UserId = u.Id
// ORDER BY
// ups.PostCount DESC,
// ups.VoteCount DESC,
// ups.CommentCount DESC
// LIMIT 100;
fn q12205(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, x, c) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(votes_of(db))), ud(db, w, posts_of(db).select(comments_of(db))));
    let v = users_stats_with(db, w, "", any_post, &[], &[&p, &x, &c]);
    users_rows(db, v, |_, _, d| (Reverse(d[0]), Reverse(d[1]), Reverse(d[2])), 100, &["name", "#d0", "#d1", "#d2"])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
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
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// ups.PostCount,
// ups.CommentCount,
// ups.VoteCount
// FROM
// Users u
// LEFT JOIN
// UserPostStats ups ON u.Id = ups.UserId
// ORDER BY
// ups.PostCount DESC, ups.CommentCount DESC, ups.VoteCount DESC;
fn q10395(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, c, x) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(comments_of(db))), ud(db, w, posts_of(db).select(votes_of(db))));
    let v = users_stats_with(db, w, "", any_post, &[], &[&p, &c, &x]);
    users_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "#d0", "#d1", "#d2"])
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(CASE WHEN V.UserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.CommentCount ELSE 0 END) AS TotalComments,
// AVG(P.Score) AS AvgPostScore,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// HAVING
// COUNT(DISTINCT P.Id) > 0
// ORDER BY
// TotalViews DESC;
fn q11065(db: &'static So) -> String {
    let w = UserWhere::All;
    let Post { view_count, post_type_id, answer_count, comment_count, score, creation_date, .. } = &db.post;
    let p = ud(db, w, posts_of(db));
    let f = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(post_type_id).and(answer_count.opt()).and(comment_count).and(score).and(creation_date).and(votes_of(db).select((&db.vote.user_id).opt()).opt())).opt())
        .fold([0, 0, 0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 9], r| match r {
            Some(((((((w, t), an), cc), s), c), x)) => {
                let ans = if t == 1 { an } else { Some(0) };
                [
                    a[0] + w.is_some() as i64,
                    a[1] + w.unwrap_or(0),
                    a[2] + x.flatten().is_some() as i64,
                    a[3] + ans.is_some() as i64,
                    a[4] + ans.unwrap_or(0),
                    a[5] + if t == 1 { cc } else { 0 },
                    a[6] + s,
                    a[7] + 1,
                    a[8].max(c),
                ]
            }
            None => a,
        });
    let mut v = Vec::new();
    f.and(&p).filt(|(_, p)| p > 0).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), nullable(a[1], a[0]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5]), avg(a[6], a[7]), V::T(a[8])])
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// U.UpVotes,
// U.DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, U.UpVotes, U.DownVotes
// ORDER BY
// PostCount DESC, U.Reputation DESC
// LIMIT 100;
fn q11821(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, q, a) = (ud(db, w, posts_of(db)), ud(db, w, only_type(db, 1)), ud(db, w, only_type(db, 2)));
    let v = users_stats_with(db, w, "", any_post, &[], &[&p, &q, &a]);
    users_rows(db, v, |u, _, d| (Reverse(d[0]), rep_desc(db, u)), 100, &["uid", "name", "rep", "ucreated", "last_access", "#d0", "#d1", "#d2", "score_sum0", "views_sum0", "uup", "udown"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(c.Score, 0)) AS TotalCommentsScore,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate
// ORDER BY
// TotalPosts DESC, TotalScore DESC
// LIMIT 100;
fn q10417(db: &'static So) -> String {
    let w = UserWhere::RepGt(0);
    let (p, q, a, c) = (ud(db, w, posts_of(db)), ud(db, w, only_type(db, 1)), ud(db, w, only_type(db, 2)), ud(db, w, posts_of(db).select(comments_of(db))));
    let v = users_stats_with(db, w, "c", any_post, &[], &[&p, &q, &a, &c]);
    users_rows(db, v, |_, s, d| (Reverse(d[0]), Reverse(s.score_sum)), 100, &["uid", "name", "rep", "ucreated", "last_access", "#d0", "#d1", "#d2", "score_sum0", "cscore_sum0", "#d3"])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePostCount,
// AVG(p.ViewCount) AS AvgViewCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// u.PositivePostCount,
// u.AvgViewCount
// FROM
// UserPostStats u
// ORDER BY
// u.Reputation DESC,
// u.PostCount DESC;
fn q10718(db: &'static So) -> String {
    let v = users_stats_with(db, UserWhere::All, "", any_post, &[], &[]);
    rows(v.iter().map(|(u, s, _)| {
        let mut f = vec![user_col(db, *u, "uid"), user_col(db, *u, "rep")];
        f.extend(["#n", "#q", "#a", "#pos", "views_avg"].iter().map(|c| ustat_field(s, c)));
        row(f)
    }))
}

// SELECT
// U.DisplayName AS UserName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// AVG(P.Score) AS AverageScore,
// AVG(P.ViewCount) AS AverageViews,
// COUNT(C.Id) AS TotalComments
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// TotalScore DESC, TotalPosts DESC;
fn q14874(db: &'static So) -> String {
    fn recent(_: i64, c: i64) -> bool {
        c >= ts(2023, 10, 1, 12, 34, 56)
    }
    let w = UserWhere::All;
    let recent_posts = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(year_ago())));
    let p = ud(db, w, &recent_posts);
    let f = user_stats_fold(db, Ident::<User>::new(), w, "c", recent).filt(|s| s.n > 0);
    let mut v = Vec::new();
    f.and((&p).opt()).drive(|u, (s, p)| v.push((u, s, [p.unwrap_or(0), 0, 0, 0])));
    users_rows(db, v, |_, s, d| (Reverse(s.score_sum), Reverse(d[0])), 0, &["name", "#d0", "#q", "#a", "views_sum", "score_sum", "score_avg", "views_avg", "#cx"])
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COUNT(DISTINCT B.Id) AS TotalBadges,
// COALESCE(MAX(C.CreationDate), '1970-01-01') AS LatestCommentDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate
// ORDER BY
// U.Reputation DESC
// LIMIT 100;
fn q14286(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, b) = (ud(db, w, posts_of(db)), ud(db, w, badges_of(db)));
    let v = users_stats_with(db, w, "bc", any_post, &[], &[&p, &b]);
    users_rows(db, v, |u, _, _| rep_desc(db, u), 100, &["uid", "name", "rep", "ucreated", "last_access", "#d0", "#q", "#a", "#d1", "cmax_or_epoch"])
}

// SELECT
// u.DisplayName AS UserName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
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
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalPosts DESC, TotalUpVotes DESC
// LIMIT 100;
fn q13603(db: &'static So) -> String {
    let w = UserWhere::RepGt(0);
    let (p, c) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(comments_of(db))));
    let v = users_stats_with(db, w, "cvb", any_post, &[], &[&p, &c]);
    users_rows(db, v, |_, s, d| (Reverse(d[0]), Reverse(s.up)), 100, &["name", "rep", "#d0", "#d1", "#up", "#down", "#bx"])
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
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
// U.Reputation > 0
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// TotalPosts DESC, U.Reputation DESC
// LIMIT 100;
fn q12149(db: &'static So) -> String {
    let w = UserWhere::RepGt(0);
    let (p, c) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(comments_of(db))));
    let v = users_stats_with(db, w, "cvb", any_post, &[], &[&p, &c]);
    users_rows(db, v, |u, _, d| (Reverse(d[0]), rep_desc(db, u)), 100, &["uid", "name", "rep", "#d0", "#d1", "#up", "#down", "#bx"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalClosedPosts,
// SUM(v.BountyAmount) AS TotalBounties,
// AVG(u.Reputation) AS AvgReputation,
// MAX(p.CreationDate) AS LastPostDate
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
// ORDER BY
// TotalPosts DESC;
fn q14991(db: &'static So) -> String {
    let w = UserWhere::All;
    let Post { post_type_id, closed_date, creation_date, .. } = &db.post;
    let (p, c) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(comments_of(db))));
    let f = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(post_type_id.and(closed_date.opt()).and(creation_date).and(comments_of(db).opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt()))
        .fold([0, 0, 0, 0, 0, 0, 0, i64::MIN, 0], |a: [i64; 9], (r, p)| {
            let mut a = a;
            a[5] += r;
            a[6] += 1;
            if let Some(((((t, cl), c), _), b)) = p {
                let b = b.flatten();
                a[0] += (t == 1) as i64;
                a[1] += (t == 2) as i64;
                a[2] += cl.is_some() as i64;
                a[3] += b.is_some() as i64;
                a[4] += b.unwrap_or(0);
                a[7] = a[7].max(c);
                a[8] += 1;
            }
            a
        });
    let mut v = Vec::new();
    f.and((&p).opt()).and((&c).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, p, c)| {
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(p),
            V::I(c),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            nullable(a[4], a[3]),
            avg(a[5], a[6]),
            if a[8] == 0 { V::Null } else { V::T(a[7]) },
        ])
    }))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// U.Reputation,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// AVG(COALESCE(LENGTH(P.Body), 0)) AS AveragePostLength,
// COUNT(C.Id) AS TotalComments,
// COUNT(B.Id) AS TotalBadges
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// U.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// U.DisplayName, U.Reputation
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q13098(db: &'static So) -> String {
    let w = UserWhere::CreatedLt(year_ago());
    let key = (&db.user.display_name).and(&db.user.reputation);
    let mut v = Vec::new();
    user_stats_fold(db, &key, w, "cb", any_post).drive(|k, s| v.push((k, s)));
    v.sort_by_key(|x| Reverse(x.1.n));
    rows(v.iter().take(100).map(|&((dn, r), ref s)| {
        let mut f = vec![V::S(dn), V::I(r)];
        f.extend(["#n", "#q", "#a", "views_sum", "score_sum", "len_avg0", "#cx", "#bx"].iter().map(|c| ustat_field(s, c)));
        row(f)
    }))
}

// A float AVG whose printed digits move with DuckDB's SET threads, so the
// rewrite takes the exact-integer mean, as 5603's does.
//
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(COALESCE(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)), 0)) AS AvgPostActivityDuration,
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
// ORDER BY
// TotalPosts DESC;
fn q10023(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, last_activity_date, creation_date, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(last_activity_date).and(creation_date)).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold(([0i64; 10], 0i128), |(a, d), (p, b)| {
            let mut a = a;
            let mut d = d;
            a[6] += 1;
            if let Some(((((t, s), w), la), c)) = p {
                a[0] += 1;
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += s;
                a[4] += w.is_some() as i64;
                a[5] += w.unwrap_or(0);
                d += (la - c) as i128;
            }
            if let Some(k) = b {
                a[7] += (k == 1) as i64;
                a[8] += (k == 2) as i64;
                a[9] += (k == 3) as i64;
            }
            (a, d)
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, (a, d))| {
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            nullable(a[3], a[0]),
            nullable(a[5], a[4]),
            V::F(d as f64 / a[6] as f64 / 1e6),
            V::I(a[7]),
            V::I(a[8]),
            V::I(a[9]),
        ])
    }))
}

// A float AVG whose printed digits move with DuckDB's SET threads, so the
// rewrite takes the exact-integer mean, as 5603's does.
//
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgPostDurationInSeconds
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q13043(db: &'static So) -> String {
    let w = UserWhere::RepGt(1000);
    let (p, q, a) = (ud(db, w, posts_of(db)), ud(db, w, only_type(db, 1)), ud(db, w, only_type(db, 2)));
    let Post { score, last_activity_date, creation_date, .. } = &db.post;
    let f = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(last_activity_date).and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold(([0i64; 4], 0i128), |(a, d), r| match r {
            Some((((s, la), c), x)) => ([a[0] + (x == Some(2)) as i64, a[1] + (x == Some(3)) as i64, a[2] + (s > 0) as i64, a[3] + 1], d + (la - c) as i128),
            None => (a, d),
        });
    let mut v = Vec::new();
    f.and((&p).opt()).and((&q).opt()).and((&a).opt()).drive(|u, ((((a, d), p), q), qa)| v.push((u, a, d, [p, q, qa].map(|x| x.unwrap_or(0)))));
    rows(v.iter().map(|&(u, a, d, n)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(n.iter().map(|&x| V::I(x)));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[3] == 0 { V::Null } else { V::F(d as f64 / a[3] as f64 / 1e6) }]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// PT.Name AS PostType,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers,
// SUM(COALESCE(P.CommentCount, 0)) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON P.OwnerUserId = U.Id
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// GROUP BY
// U.Id, U.DisplayName, PT.Name
// )
// SELECT
// UserId,
// DisplayName,
// PostType,
// PostCount,
// TotalScore,
// TotalViews,
// TotalAnswers,
// TotalComments
// FROM
// UserPostStats
// ORDER BY
// TotalScore DESC, PostCount DESC;
fn q10994(db: &'static So) -> String {
    let Post { post_type, score, view_count, answer_count, comment_count, .. } = &db.post;
    let j: MatSet<(Id<User>, Option<Id<Post>>)> = db.user.select(Ident::<User>::new().and(posts_of(db).opt())).collect();
    let post_of = (&j).flat_map(|(_, p)| p);
    let key = (&j).map(|(u, _)| u).and((&post_of).select(post_type.select(&db.post_type.name)).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((((s, w), an), cc)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + an.unwrap_or(0), a[4] + cc],
            None => a,
        })
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((u, t), a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), ostr(t)];
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// --- posts ------------------------------------------------------------------

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// c.CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (
// SELECT
// PostId, COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, u.DisplayName, p.Title, p.CreationDate, p.Score, p.ViewCount, c.CommentCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19314(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post_id).fold(0i64, |a, _| a + 1);
    let key = Ident::<Post>::new().and((&db.post.origid).select((&cc).opt()));
    let mut v = group_stats(db, owned(db).with((&db.post.post_type_id).eq(1)), key, "v", &[]);
    v.sort_by_key(|&((p, _), _)| newest(db, p));
    rows(v.iter().take(10).map(|&((p, c), ref s)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([oint(c), V::I(s.vx)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COUNT(DISTINCT b.Id) AS BadgeCount
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ORDER BY
// p.Score DESC, p.ViewCount DESC;
fn q11784(db: &'static So) -> String {
    let b = badges_distinct(db);
    stats_rows(db, stats_with(db, since(db, date(2023, 1, 1)), "cvb", &[], &[&b]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "#cx", "#up", "#down", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COUNT(b.Id) AS BadgeCount,
// (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = p.Id) AS HistoryCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q12503(db: &'static So) -> String {
    let hpp = history_per_post(db);
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cvb", &[]).and(&hpp).drive(|p, (s, h)| v.push((p, s, h)));
    rows(v.iter().map(|&(p, ref s, h)| {
        let mut f = stat_fields(db, p, s, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "#bx"]);
        f.push(V::I(h));
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// COUNT(DISTINCT BH.Id) AS HistoryChangeCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostHistory BH ON P.Id = BH.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate > DATE '2020-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, U.DisplayName, U.Reputation
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q13659(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    let h = per_post_distinct(db, history_of(db));
    let base = db.post.with((&db.post.creation_date).gt(date(2020, 1, 1)));
    stats_rows(db, stats_with(db, base, "cvh", &[], &[&x, &h]), |p, _| newest(db, p), 100, &["id", "title", "created", "#cx", "#d0", "#d1", "owner", "rep"])
}

// WITH PostAnalytics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// COUNT(C.ID) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// CommentCount,
// UpVotes,
// DownVotes
// FROM
// PostAnalytics
// ORDER BY
// CreationDate DESC;
fn q11869(db: &'static So) -> String {
    stats_rows(db, stats_with(db, since(db, date(2023, 1, 1)), "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "#cx", "#up", "#down"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// PT.Name AS PostType,
// U.Id AS UserId,
// U.DisplayName AS UserDisplayName,
// U.Reputation AS UserReputation,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, PT.Name, U.Id, U.DisplayName, U.Reputation
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q12111(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, owned(db), "cv", &[], &[&x]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "type", "uid", "owner", "rep", "#cx", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// CASE
// WHEN p.AcceptedAnswerId IS NOT NULL THEN 1
// ELSE 0
// END AS HasAcceptedAnswer
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AcceptedAnswerId
// ORDER BY
// p.Score DESC, p.ViewCount DESC
// LIMIT 100;
fn q13366(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let v = stats_with(db, base, "cv", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| ((Reverse(db.post.score.get(p).unwrap()), db.post.view_count.get(p).is_none(), Reverse(db.post.view_count.get(p))), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score", "#cx", "#up", "#down"]);
        f.push(V::I(db.post.accepted_answer_id.get(*p).is_some() as i64));
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.UserId) AS UniqueVoterCount,
// AVG(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS AverageUpVotes,
// AVG(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS AverageDownVotes,
// U.Reputation AS OwnerReputation
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= '2021-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.Reputation
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q13632(db: &'static So) -> String {
    let voters = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    let v = stats_with(db, since(db, date(2021, 1, 1)), "cv", &[], &[&voters]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, d)| (newest(db, p), p, s, d)).collect();
    v.sort_by_key(|x| x.0);
    let one = |n: i64| if n == 0 { V::Null } else { V::F(1.0) };
    rows(v.iter().take(100).map(|(_, p, s, d)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views", "#cx"]);
        f.extend([V::I(d[0]), one(s.up), one(s.down)]);
        f.extend(post_fields(db, *p, &["rep"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostID,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.Id AS UserID,
// u.DisplayName AS UserDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.Id, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14065(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cv", &[], &[&c, &x]), |p, _| newest(db, p), 100, &["id", "title", "created", "uid", "owner", "#d0", "#d1", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.LastActivityDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// AVG(u.Reputation) AS AverageOwnerReputation,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.Score
// ORDER BY
// p.Score DESC, p.CreationDate DESC
// LIMIT 100;
fn q13365(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    let base = db.post.with((&db.post.creation_date).gt(year_ago()));
    stats_rows(db, stats_with(db, base, "cv", &[], &[&x]), |p, _| (Reverse(db.post.score.get(p).unwrap()), newest(db, p)), 100, &["id", "title", "created", "activity", "#cx", "#d0", "rep_avg", "#down", "#up"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13232(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cvb", &[], &[&c, &x]), |p, _| newest(db, p), 100, &["id", "title", "created", "owner", "#d0", "#d1", "#up", "#down", "#bx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT ph.Id) AS EditCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// Posts p
// INNER JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q12515(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cvh", &[], &[&h]), |_, _| 0, 0, &["id", "title", "created", "owner", "#cx", "#up", "#down", "#d0", "hmax"])
}

// WITH Benchmark AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount,
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
// PostId,
// Title,
// CreationDate,
// CommentCount,
// VoteCount,
// UpVoteCount,
// DownVoteCount,
// (CommentCount + VoteCount) AS EngagementScore
// FROM
// Benchmark
// ORDER BY
// EngagementScore DESC
// LIMIT 10;
fn q14328(db: &'static So) -> String {
    let v = stats_with(db, db.post.iq(), "cv", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (Reverse(s.cx + s.vx), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(10).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "#cx", "#vx", "#up", "#down"]);
        f.push(V::I(s.cx + s.vx));
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
// COUNT(DISTINCT b.Id) AS BadgeCount,
// p.ViewCount,
// u.Reputation AS UserReputation
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation
// ORDER BY
// p.Score DESC, p.CreationDate DESC;
fn q13174(db: &'static So) -> String {
    let b = badges_distinct(db);
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cvb", &[], &[&b]), |_, _| 0, 0, &["id", "title", "created", "score", "#cx", "#up", "#down", "#d0", "views", "rep"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.UserId END) AS UpVoteCount,
// COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.UserId END) AS DownVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score,
// u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q14319(db: &'static So) -> String {
    let t = |x: i64| per_post_distinct(db, votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(x))).select(&db.vote.user_id));
    let (u, d) = (t(2), t(3));
    stats_rows(db, stats_with(db, owned_since(db, month_ago()), "cv", &[], &[&u, &d]), |_, _| 0, 0, &["id", "title", "created", "views", "score", "owner", "rep", "#cx", "#d0", "#d1"])
}

// SELECT p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// pt.Name AS PostTypeName,
// COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
// LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
// WHERE p.CreationDate >= '2022-01-01'
// GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, pt.Name, p.AcceptedAnswerId
// ORDER BY p.CreationDate DESC;
fn q11011(db: &'static So) -> String {
    let b = badges_distinct(db);
    let v = stats_with(db, since(db, date(2022, 1, 1)), "cvb", &[], &[&b]);
    rows(v.iter().map(|(p, s, d)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score", "#cx", "#up", "#down"]);
        f.extend([V::I(d[0])]);
        f.extend(post_fields(db, *p, &["type"]));
        f.push(V::I(db.post.accepted_answer_id.get(*p).unwrap_or(-1)));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT ph.Id) AS EditHistoryCount,
// p.ViewCount
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
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.ViewCount
// ORDER BY
// p.CreationDate DESC;
fn q11253(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cvh", &[], &[&h]), |_, _| 0, 0, &["id", "title", "created", "owner", "#cx", "#vx", "#up", "#down", "#d0", "views"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
// COUNT(b.Id) AS BadgeCount,
// u.Reputation AS OwnerReputation,
// DATE_TRUNC('day', p.CreationDate) AS CreationDate
// FROM Posts p
// JOIN PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN Comments c ON c.PostId = p.Id
// LEFT JOIN Votes v ON v.PostId = p.Id
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Badges b ON b.UserId = u.Id
// WHERE p.CreationDate >= '2023-01-01'
// GROUP BY p.Id, p.Title, pt.Name, u.Reputation, p.CreationDate
// ORDER BY p.CreationDate DESC;
fn q12685(db: &'static So) -> String {
    let v = stats_with(db, since(db, date(2023, 1, 1)), "cvb", &[], &[]);
    rows(v.iter().map(|(p, s, _)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "type", "#cx", "#up", "#down"]);
        f.extend([V::I(s.bounty_sum), V::I(s.bx)]);
        f.extend(post_fields(db, *p, &["rep"]));
        f.push(V::T(trunc_day(db.post.creation_date.get(*p).unwrap())));
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.UserId) AS VoteCount,
// COUNT(DISTINCT P2.Id) AS RelatedPostCount,
// MAX(CASE WHEN PH.PostHistoryTypeId = 10 THEN PH.CreationDate END) AS LastClosedDate,
// MAX(CASE WHEN PH.PostHistoryTypeId = 11 THEN PH.CreationDate END) AS LastReopenedDate
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostLinks PL ON P.Id = PL.PostId
// LEFT JOIN
// Posts P2 ON PL.RelatedPostId = P2.Id
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount
// ORDER BY
// P.ViewCount DESC;
fn q12366(db: &'static So) -> String {
    let voters = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    let related = per_post_distinct(db, links_of(db).select(&db.post_link.related_post));
    let v = stats_with(db, db.post.iq(), "cvlh", &[], &[&voters, &related]);
    rows(v.iter().map(|(p, s, d)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "#cx"]);
        f.extend([V::I(d[0]), V::I(d[1]), stat_field(s, "h10max").unwrap(), stat_field(s, "h11max").unwrap()]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// u.DisplayName AS UserName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore,
// SUM(p.CommentCount) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name, u.DisplayName
// )
// SELECT
// PostType,
// UserName,
// TotalPosts,
// TotalUpvotedPosts,
// TotalViews,
// AverageScore,
// TotalComments
// FROM
// PostStats
// ORDER BY
// TotalPosts DESC, AverageScore DESC;
fn q12480(db: &'static So) -> String {
    let Post { post_type, owner_user, score, view_count, comment_count, .. } = &db.post;
    let key = post_type.select(&db.post_type.name).and(owner_user.select(&db.user.display_name).opt());
    let mut v = Vec::new();
    db.post
        .group_by(key)
        .select(score.and(view_count.opt()).and(comment_count))
        .fold([0i64; 6], |a, ((s, w), c)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s, a[5] + c])
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((k, dn), a)| row(vec![V::S(k), ostr(dn), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[4], a[0]), V::I(a[5])])))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// P.Score AS PostScore,
// P.ViewCount AS PostViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// MAX(P.LastActivityDate) AS LastActivityDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// U.DisplayName, P.Title, P.CreationDate, P.Score, P.ViewCount
// ORDER BY
// PostScore DESC, PostViewCount DESC;
fn q11732(db: &'static So) -> String {
    let Post { title, creation_date, score, view_count, owner_user, .. } = &db.post;
    let base = owned_since(db, date(2023, 1, 1));
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(score).and(view_count.opt());
    let s = stats_fold(db, &base, &key, "cvb", &[]);
    let x = (&base).group_by(&key).select(votes_of(db)).count_distinct();
    let b = (&base).group_by(&key).select(owner_user.select(badges_of(db))).count_distinct();
    let la = (&base).group_by(&key).select(&db.post.last_activity_date).fold(i64::MIN, |a, d| a.max(d));
    let mut v = Vec::new();
    s.and((&x).opt()).and((&b).opt()).and(&la).drive(|k, (((s, x), b), la)| v.push((k, s, x.unwrap_or(0), b.unwrap_or(0), la)));
    rows(v.iter().map(|&(((((dn, t), c), sc), w), ref s, x, b, la)| row(vec![V::S(dn), ostr(t), V::T(c), V::I(sc), oint(w), V::I(s.cx), V::I(x), V::I(b), V::T(la)])))
}

// WITH QuestionStats AS (
// SELECT
// p.Id AS QuestionId,
// p.Score AS QuestionScore,
// p.ViewCount AS QuestionViews,
// COUNT(a.Id) AS AnswerCount,
// COALESCE(AVG(a.Score), 0) AS AverageAnswerScore,
// COALESCE(AVG(a.ViewCount), 0) AS AverageAnswerViews
// FROM
// Posts p
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Score, p.ViewCount
// )
// SELECT
// Q.QuestionId,
// Q.QuestionScore,
// Q.QuestionViews,
// Q.AnswerCount,
// Q.AverageAnswerScore,
// Q.AverageAnswerViews
// FROM
// QuestionStats Q
// ORDER BY
// Q.QuestionId;
fn q14066(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).select(score.and(view_count.opt())).opt())
        .fold([0i64; 4], |a, r| match r {
            Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
            None => a,
        })
        .drive(|p, a| v.push((p, a)));
    let or0 = |s: i64, n: i64| V::F(if n == 0 { 0.0 } else { s as f64 / n as f64 });
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id", "score", "views"]);
        f.extend([V::I(a[0]), or0(a[1], a[0]), or0(a[3], a[2])]);
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
// COUNT(v.Id) AS VoteCount,
// (SELECT COUNT(*) FROM Posts ap WHERE ap.AcceptedAnswerId = p.Id) AS AcceptedAnswerCount,
// (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = p.Id) AS HistoryRecordCount
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
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10824(db: &'static So) -> String {
    let accepting = (&db.post.accepted_answer).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let hpp = history_per_post(db);
    let mut v = Vec::new();
    stats_fold(db, owned(db).with((&db.post.post_type_id).eq(1)), Ident::<Post>::new(), "cv", &[]).and(&accepting).and(&hpp).drive(|p, ((s, a), h)| v.push((newest(db, p), p, s, a, h)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, ref s, a, h)| {
        let mut f = stat_fields(db, p, s, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx"]);
        f.extend([V::I(a), V::I(h)]);
        row(f)
    }))
}

// --- whole-table aggregates ------------------------------------------------

// SELECT
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// AVG(p.Score) AS AveragePostScore,
// AVG(c.Score) AS AverageCommentScore,
// AVG(CASE WHEN p.ViewCount IS NULL THEN 0 ELSE p.ViewCount END) AS AveragePostViewCount,
// AVG(CASE WHEN p.AnswerCount IS NULL THEN 0 ELSE p.AnswerCount END) AS AverageAnswersPerQuestion,
// AVG(u.Reputation) AS AverageUserReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// AND p.PostTypeId IN (1, 2);
fn q14930(db: &'static So) -> String {
    let Post { score, view_count, answer_count, owner_user, post_type_id, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2023, 1, 1)).and(post_type_id.in_v(vec![1, 2])));
    let a = (&base)
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(owner_user.select(&db.user.reputation).opt()).and(comments_of(db).select(&db.comment.score).opt()).and(votes_of(db).opt()))
        .fold_flat([0i64; 8], |a, (((((s, w), an), r), c), _)| {
            [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + an.unwrap_or(0), a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0), a[6] + c.is_some() as i64, a[7] + c.unwrap_or(0)]
        });
    let one = |f: Fold<(), i64>| (&f).fold_flat(0i64, |a, x| a + x);
    let p = one(whole(&base).select(Ident::<Post>::new()).count_distinct());
    let c = one(whole(&base).select(comments_of(db)).count_distinct());
    let x = one(whole(&base).select(votes_of(db)).count_distinct());
    row(vec![V::I(p), V::I(c), V::I(x), avg(a[1], a[0]), avg(a[7], a[6]), avg(a[2], a[0]), avg(a[3], a[0]), avg(a[5], a[4])])
}

// SELECT
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// SUM(p.Score) AS TotalPostScore,
// AVG(u.Reputation) AS AverageUserReputation,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= CURRENT_DATE - INTERVAL '6 months'
// GROUP BY
// u.Reputation
// ORDER BY
// TotalPosts DESC;
fn q14543(db: &'static So) -> String {
    let Post { owner_user, score, post_type_id, creation_date, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(add_months(current_date(), -6)));
    let key = owner_user.select(&db.user.reputation);
    let f = (&base).group_by(&key).select(score.and(post_type_id).and(owner_user.select(&db.user.reputation)).and(owner_user.select(badges_of(db)).opt()).and(comments_of(db).opt())).fold([0i64; 5], |a, ((((s, t), rep), _), _)| {
        [a[0] + s, a[1] + 1, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + rep]
    });
    let p = dby(&base, &key, Ident::<Post>::new());
    let u = dby(&base, &key, owner_user);
    let b = dby(&base, &key, owner_user.select(badges_of(db)));
    let c = dby(&base, &key, comments_of(db));
    let mut v = Vec::new();
    f.and(&p).and(&u).and((&b).opt()).and((&c).opt()).drive(|_, ((((a, p), u), b), c)| v.push((a, p, u, b.unwrap_or(0), c.unwrap_or(0))));
    rows(v.iter().map(|&(a, p, u, b, c)| row(vec![V::I(p), V::I(u), V::I(a[0]), avg(a[4], a[1]), V::I(b), V::I(a[2]), V::I(a[3]), V::I(c)])))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AverageViews
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
// COUNT(*) AS TotalVotes
// FROM
// Votes
// )
// SELECT
// p.TotalPosts,
// p.AverageViews,
// u.TotalUsers,
// u.AverageReputation,
// c.TotalComments,
// v.TotalVotes
// FROM
// PostStats p,
// UserStats u,
// CommentStats c,
// VoteStats v;
fn q11935(db: &'static So) -> String {
    let (vn, vs) = sum_n(db.post.select(&db.post.view_count));
    let (rn, rs) = sum_n(&db.user.reputation);
    row(vec![V::I(count(db.post.iq())), avg(vs, vn), V::I(rn), avg(rs, rn), V::I(count(db.comment.iq())), V::I(count(db.vote.iq()))])
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AveragePostScore,
// AVG(ViewCount) AS AverageViewCount
// FROM
// Posts
// ),
// CommentStats AS (
// SELECT
// COUNT(*) AS TotalComments,
// AVG(Score) AS AverageCommentScore
// FROM
// Comments
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageUserReputation
// FROM
// Users
// )
// SELECT
// PS.TotalPosts,
// PS.AveragePostScore,
// PS.AverageViewCount,
// CS.TotalComments,
// CS.AverageCommentScore,
// US.TotalUsers,
// US.AverageUserReputation
// FROM
// PostStats PS,
// CommentStats CS,
// UserStats US;
fn q11990(db: &'static So) -> String {
    let (pn, ps) = sum_n(&db.post.score);
    let (vn, vs) = sum_n(db.post.select(&db.post.view_count));
    let (cn, cs) = sum_n(&db.comment.score);
    let (rn, rs) = sum_n(&db.user.reputation);
    row(vec![V::I(pn), avg(ps, pn), avg(vs, vn), V::I(cn), avg(cs, cn), V::I(rn), avg(rs, rn)])
}

// Post types beside one-row user aggregates (a CTE cross join with a single row).
fn types_with_users(db: &'static So, cols: &[&str], extra: impl Fn() -> Vec<V>) -> Vec<String> {
    by_count(db)
        .iter()
        .map(|a| {
            let mut f = type_fields(a, cols);
            f.extend(extra());
            row(f)
        })
        .collect()
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserStats AS (
// SELECT
// COUNT(u.Id) AS UserCount,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Users u
// )
// SELECT
// ps.PostType,
// ps.PostCount,
// ps.AverageScore,
// ps.AverageViewCount,
// us.UserCount,
// us.AverageReputation
// FROM
// PostStats ps,
// UserStats us
// ORDER BY
// ps.PostType;
fn q12870(db: &'static So) -> String {
    let (rn, rs) = sum_n(&db.user.reputation);
    rows(types_with_users(db, &["name", "n", "score_avg", "views_avg"], || vec![V::I(rn), avg(rs, rn)]))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AvgViewCount,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation,
// MAX(Reputation) AS MaxReputation,
// MIN(Reputation) AS MinReputation
// FROM
// Users
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.AvgViewCount,
// ps.TotalScore,
// us.TotalUsers,
// us.AvgReputation,
// us.MaxReputation,
// us.MinReputation
// FROM
// PostStats ps, UserStats us
// ORDER BY
// ps.TotalPosts DESC;
fn q12461(db: &'static So) -> String {
    let (rn, rs) = sum_n(&db.user.reputation);
    let mx = (&db.user.reputation).fold_flat(i64::MIN, |a, x| a.max(x));
    let mn = (&db.user.reputation).fold_flat(i64::MAX, |a, x| a.min(x));
    rows(types_with_users(db, &["name", "n", "views_avg", "score_sum"], || vec![V::I(rn), avg(rs, rn), V::I(mx), V::I(mn)]))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViews
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
// AVG(u.Reputation) AS AverageReputation,
// SUM(u.Views) AS TotalViews
// FROM
// Users u
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.AverageScore,
// ps.AverageViews,
// us.TotalUsers,
// us.AverageReputation,
// us.TotalViews
// FROM
// PostStats ps,
// UserStats us
// ORDER BY
// ps.TotalPosts DESC;
fn q12999(db: &'static So) -> String {
    let (rn, rs) = sum_n(&db.user.reputation);
    let (_, views) = sum_n(&db.user.views);
    rows(types_with_users(db, &["name", "n", "score_avg", "views_avg"], || vec![V::I(rn), avg(rs, rn), V::I(views)]))
}

// --- post types ---------------------------------------------------------------

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

fn dby<Q, K, R>(base: Q, key: K, r: R) -> Fold<ROf<K>, i64>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    K: IntoQuery,
    K::Q: Probe<D = Id<Post>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>>,
    ROf<R>: Ord,
{
    base.group_by(key).select(r).count_distinct()
}

fn with_d<K: Copy + Eq + std::hash::Hash, const N: usize>(f: &Fold<K, [i64; N]>, d: &[&Fold<K, i64>]) -> Vec<(K, [i64; N], [i64; 2])> {
    let mut v = Vec::new();
    let o = |x: Option<i64>| x.unwrap_or(0);
    match d {
        [] => f.drive(|k, a| v.push((k, a, [0; 2]))),
        [x] => f.and(x.opt()).drive(|k, (a, x)| v.push((k, a, [o(x), 0]))),
        [x, y] => f.and(x.opt()).and(y.opt()).drive(|k, ((a, x), y)| v.push((k, a, [o(x), o(y)]))),
        _ => panic!(),
    }
    v.sort_by_key(|x| Reverse(x.1[0]));
    v
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViewCount,
// AVG(u.Reputation) AS AverageUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// PostCount,
// AverageScore,
// TotalViewCount,
// AverageUserReputation
// FROM
// PostStats
// ORDER BY
// PostCount DESC;
fn q13026(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let f = by_key(since(db, year_ago()), name(db), score.and(view_count.opt()).and(owner_user.select(&db.user.reputation).opt()), [0i64; 6], |a, ((s, w), r)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)]
    });
    rows(with_d(&f, &[]).iter().map(|&(k, a, _)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), avg(a[5], a[4])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// PostCount,
// AverageScore,
// AverageViewCount,
// TotalComments,
// TotalVotes
// FROM
// PostStats
// ORDER BY
// PostCount DESC;
fn q13462(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), score.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt()), [0i64; 4], |a, (((s, w), _), _)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let c = dby(db.post.iq(), name(db), comments_of(db));
    let x = dby(db.post.iq(), name(db), votes_of(db));
    rows(with_d(&f, &[&c, &x]).iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(d[0]), V::I(d[1])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// pt.Name
// )
// SELECT
// PostTypeName,
// TotalPosts,
// COALESCE(AverageScore, 0) AS AverageScore,
// COALESCE(AverageViewCount, 0) AS AverageViewCount,
// UniqueUsers
// FROM
// PostStats
// ORDER BY
// TotalPosts DESC;
fn q14619(db: &'static So) -> String {
    let Post { score, view_count, owner_user_id, .. } = &db.post;
    let base = since(db, year_ago());
    let f = by_key(&base, name(db), score.and(view_count.opt()), [0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let u = dby(&base, name(db), owner_user_id);
    let or0 = |s: i64, n: i64| V::F(if n == 0 { 0.0 } else { s as f64 / n as f64 });
    rows(with_d(&f, &[&u]).iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), or0(a[1], a[0]), or0(a[3], a[2]), V::I(d[0])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalQuestionsWithUpvotes,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// AVG(COALESCE(p.Score, 0)) AS AverageScore,
// AVG(COALESCE(p.ViewCount, 0)) AS AverageViewCount
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
fn q14223(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db).opt(), score.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt()), [0i64; 6], |a, (((s, w), c), x)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + w.unwrap_or(0), a[3] + c.is_some() as i64, a[4] + x.is_some() as i64, a[5] + s]
    });
    rows(with_d(&f, &[]).iter().map(|&(k, a, _)| row(vec![ostr(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[5], a[0]), avg(a[2], a[0])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// COUNT(DISTINCT u.Id) AS TotalContributors,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14511(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let base = since(db, year_ago());
    let f = by_key(&base, name(db), score.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()), [0i64; 6], |a, (((s, w), x), _)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + (x == Some(2)) as i64, a[5] + (x == Some(3)) as i64]
    });
    let u = dby(&base, name(db), owner_user);
    let c = dby(&base, name(db), comments_of(db));
    rows(with_d(&f, &[&u, &c]).iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), V::I(a[4]), V::I(a[5]), V::I(d[0]), V::I(d[1])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(u.Reputation) AS AverageUserReputation,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// pt.Name, u.Reputation
// ORDER BY
// TotalPosts DESC, AverageUserReputation DESC;
fn q9214(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let base = since(db, year_ago());
    let key = name(db).and(owner_user.select(&db.user.reputation).opt());
    let f = by_key(&base, &key, owner_user.select(&db.user.reputation).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt()), [0i64; 4], |a, (((r, x), _), _)| {
        [a[0] + 1, a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0), a[3] + (x == Some(2)) as i64]
    });
    let c = dby(&base, &key, comments_of(db));
    let b = dby(&base, &key, owner_user.select(badges_of(db)));
    rows(with_d(&f, &[&c, &b]).iter().map(|&((k, _), a, d)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), V::I(a[3]), V::I(d[0]), V::I(d[1])])))
}

// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS TotalPosts,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// AVG(COALESCE(p.ViewCount, 0)) AS AverageViewsPerPost,
// AVG(COALESCE(p.Score, 0)) AS AverageScorePerPost,
// COUNT(c.Id) AS TotalComments,
// SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyAmount,
// COUNT(DISTINCT b.Id) AS TotalBadgesAwarded
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.PostTypeId
// ORDER BY
// p.PostTypeId;
fn q14226(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let base = since(db, year_ago());
    let f = by_key(
        &base,
        post_type_id,
        score.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).select((&db.vote.vote_type_id).in_v(vec![8, 9]).map(|_| ()).and((&db.vote.bounty_amount).opt())).opt()).and(owner_user.select(badges_of(db)).opt()),
        [0i64; 5],
        |a, ((((s, w), c), x), _)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + c.is_some() as i64, a[4] + x.and_then(|x| x.1).unwrap_or(0)],
    );
    let b = dby(&base, post_type_id, owner_user.select(badges_of(db)));
    let mut v = with_d(&f, &[&b]);
    v.sort_by_key(|x| x.0);
    rows(v.iter().map(|&(t, a, d)| row(vec![V::I(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(a[1] as f64 / a[0] as f64), V::F(a[2] as f64 / a[0] as f64), V::I(a[3]), V::I(a[4]), V::I(d[0])])))
}

// A float AVG whose printed digits move with DuckDB's SET threads, so the
// rewrite takes the exact-integer mean, as 5603's does.
//
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(COALESCE(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)), 0)) AS AvgPostAgeInSeconds,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AvgScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalPostsClosed
// FROM
// Posts p
// INNER JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11058(db: &'static So) -> String {
    let Post { score, view_count, last_activity_date, creation_date, .. } = &db.post;
    let f = by_key(
        db.post.iq(),
        name(db),
        score.and(view_count.opt()).and(last_activity_date).and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()),
        ([0i64; 7], 0i128),
        |(a, d), (((((s, w), la), c), x), h)| {
            ([a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + (x == Some(2)) as i64, a[5] + (x == Some(3)) as i64, a[6] + (h == Some(10)) as i64], d + (la - c) as i128)
        },
    );
    let mut v = Vec::new();
    f.drive(|k, a| v.push((k, a)));
    v.sort_by_key(|x| Reverse(x.1.0[0]));
    rows(v.iter().map(|&(k, (a, d))| row(vec![V::S(k), V::I(a[0]), V::F(d as f64 / a[0] as f64 / 1e6), nullable(a[3], a[2]), avg(a[1], a[0]), V::I(a[4]), V::I(a[5]), V::I(a[6])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(p.Score) AS AverageScore,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) / 3600) AS AverageHoursToActivity,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11045(db: &'static So) -> String {
    let Post { score, view_count, last_activity_date, creation_date, .. } = &db.post;
    let base = since(db, month_ago());
    let f = by_key(&base, name(db), score.and(view_count.opt()).and(last_activity_date).and(creation_date).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()), ([0i64; 6], 0i128), |(a, d), (((((s, w), la), c), _), x)| {
        ([a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + (x == Some(2)) as i64, a[5] + (x == Some(3)) as i64], d + (la - c) as i128)
    });
    let c = dby(&base, name(db), comments_of(db));
    let mut v = Vec::new();
    f.and((&c).opt()).drive(|k, ((a, d), c)| v.push((k, a, d, c.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a, d, c)| row(vec![V::S(k), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), avg(a[3], a[0]), V::F(d as f64 / a[0] as f64 / 1e6 / 3600.0), V::I(c), V::I(a[4]), V::I(a[5])])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("13479", q13479),
    ("14183", q14183),
    ("12245", q12245),
    ("11689", q11689),
    ("13887", q13887),
    ("12205", q12205),
    ("10395", q10395),
    ("11065", q11065),
    ("11821", q11821),
    ("10417", q10417),
    ("10718", q10718),
    ("14874", q14874),
    ("14286", q14286),
    ("13603", q13603),
    ("12149", q12149),
    ("14991", q14991),
    ("13098", q13098),
    ("10023", q10023),
    ("13043", q13043),
    ("10994", q10994),
    ("19314", q19314),
    ("11784", q11784),
    ("12503", q12503),
    ("13659", q13659),
    ("11869", q11869),
    ("12111", q12111),
    ("13366", q13366),
    ("13632", q13632),
    ("14065", q14065),
    ("13365", q13365),
    ("13232", q13232),
    ("12515", q12515),
    ("14328", q14328),
    ("13174", q13174),
    ("14319", q14319),
    ("11011", q11011),
    ("11253", q11253),
    ("12685", q12685),
    ("12366", q12366),
    ("12480", q12480),
    ("11732", q11732),
    ("14066", q14066),
    ("10824", q10824),
    ("14930", q14930),
    ("14543", q14543),
    ("11935", q11935),
    ("11990", q11990),
    ("12870", q12870),
    ("12461", q12461),
    ("12999", q12999),
    ("13026", q13026),
    ("13462", q13462),
    ("14619", q14619),
    ("14223", q14223),
    ("14511", q14511),
    ("9214", q9214),
    ("14226", q14226),
    ("11058", q11058),
    ("11045", q11045),
];
