use harness::prelude::*;
use std::cmp::Reverse;

fn questions(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.post_type_id).eq(1))
}

fn all_questions(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    db.post.with((&db.post.post_type_id).eq(1))
}

fn owned_since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.creation_date).ge(d))
}

fn since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    db.post.with((&db.post.creation_date).ge(d))
}

fn score_then_newest(db: &'static So, p: Id<Post>) -> (Reverse<i64>, Reverse<i64>) {
    (Reverse(db.post.score.get(p).unwrap()), newest(db, p))
}

fn views_desc(db: &'static So, p: Id<Post>) -> (bool, Reverse<Option<i64>>) {
    let w = db.post.view_count.get(p);
    (w.is_none(), Reverse(w))
}

fn year_ago() -> i64 {
    ts(2023, 10, 1, 12, 34, 56)
}

fn month_ago() -> i64 {
    ts(2024, 9, 1, 12, 34, 56)
}

fn badges_distinct(db: &'static So) -> Fold<Id<Post>, i64> {
    per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)))
}

// One row per user: the joined-row fold beside up to four COUNT(DISTINCT ...).
fn users_with(db: &'static So, w: UserWhere, joins: &str, pf: fn(i64, i64) -> bool, d: &[&Fold<Id<User>, i64>]) -> Vec<(Id<User>, UStats, [i64; 4])> {
    let f = user_stats_fold(db, Ident::<User>::new(), w, joins, pf);
    let o = |x: Option<i64>| x.unwrap_or(0);
    let mut v = Vec::new();
    match d {
        [] => f.drive(|u, s| v.push((u, s, [0; 4]))),
        [a] => f.and(a.opt()).drive(|u, (s, x)| v.push((u, s, [o(x), 0, 0, 0]))),
        [a, b] => f.and(a.opt()).and(b.opt()).drive(|u, ((s, x), y)| v.push((u, s, [o(x), o(y), 0, 0]))),
        [a, b, c] => f.and(a.opt()).and(b.opt()).and(c.opt()).drive(|u, (((s, x), y), z)| v.push((u, s, [o(x), o(y), o(z), 0]))),
        [a, b, c, e] => f.and(a.opt()).and(b.opt()).and(c.opt()).and(e.opt()).drive(|u, ((((s, x), y), z), q)| v.push((u, s, [o(x), o(y), o(z), o(q)]))),
        _ => panic!("users_with: at most four distinct counts"),
    }
    v
}

fn user_rows<T: Ord>(db: &'static So, mut v: Vec<(Id<User>, UStats, [i64; 4])>, key: impl Fn(Id<User>, &UStats, &[i64; 4]) -> T, n: usize, cols: &[&str]) -> String {
    v.sort_by_key(|(u, s, d)| key(*u, s, d));
    let n = if n == 0 { v.len() } else { n };
    rows(v.iter().take(n).map(|(u, s, d)| {
        row(cols
            .iter()
            .map(|c| match *c {
                "#d0" => V::I(d[0]),
                "#d1" => V::I(d[1]),
                "#d2" => V::I(d[2]),
                "#d3" => V::I(d[3]),
                "uid" | "name" | "rep" | "ucreated" | "last_access" | "uviews" | "uup" | "udown" => user_col(db, *u, c),
                _ => ustat_field(s, c),
            })
            .collect())
    }))
}

fn g(db: &'static So) -> GroupBy<Ident<User>, impl Drive<D = Id<User>, R = Id<User>>> {
    user_base(db, UserWhere::All).group_by(Ident::<User>::new())
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN v.VoteTypeId = 4 THEN 1 ELSE 0 END) AS TotalOffensiveVotes
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
fn q10026(db: &'static So) -> String {
    let p = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db));
    let c = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select(comments_of(db)));
    let x = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select(votes_of(db)));
    let v = users_with(db, UserWhere::All, "cv", any_post, &[&p, &c, &x]);
    user_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "#d0", "#d1", "#d2", "#up", "#down", "#v4"])
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// COUNT(DISTINCT C.Id) AS TotalComments,
// AVG(P.Score) AS AveragePostScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// PostCount DESC, TotalUpVotes DESC;
fn q11119(db: &'static So) -> String {
    fn recent(_: i64, c: i64) -> bool {
        c >= date(2023, 1, 1)
    }
    let recent_posts = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(date(2023, 1, 1))));
    let p = user_distinct(db, Ident::<User>::new(), UserWhere::All, &recent_posts);
    let c = user_distinct(db, Ident::<User>::new(), UserWhere::All, (&recent_posts).select(comments_of(db)));
    // JOIN Posts ... WHERE p.CreationDate: users with no such post drop out
    let f = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "vc", recent).filt(|s| s.n > 0);
    let mut out = Vec::new();
    f.and((&p).opt()).and((&c).opt()).drive(|u, ((s, p), c)| out.push((u, s, [p.unwrap_or(0), c.unwrap_or(0), 0, 0])));
    user_rows(db, out, |_, s, d| (Reverse(d[0]), Reverse(s.up)), 0, &["uid", "name", "#d0", "#up", "#down", "#d1", "score_avg", "views_sum"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT COALESCE(p.AcceptedAnswerId, 0)) AS AcceptedAnswers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// COUNT(DISTINCT c.Id) AS TotalComments,
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
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalPosts DESC, TotalScore DESC
// LIMIT 100;
fn q12435(db: &'static So) -> String {
    let p = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db));
    let acc = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select((&db.post.accepted_answer_id).opt()).opt().map(|a| a.flatten().unwrap_or(0)));
    let c = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select(comments_of(db)));
    let b = user_distinct(db, Ident::<User>::new(), UserWhere::All, badges_of(db));
    let v = users_with(db, UserWhere::All, "cb", any_post, &[&p, &acc, &c, &b]);
    user_rows(db, v, |_, s, d| (Reverse(d[0]), Reverse(s.score_sum)), 100, &["uid", "name", "rep", "#d0", "#d1", "score_sum0", "views_sum0", "#d2", "#d3"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// AVG(p.Score) AS AveragePostScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.AnswerCount) AS TotalAnswers
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
// TotalPosts DESC, TotalUpVotes DESC;
fn q11774(db: &'static So) -> String {
    let p = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db));
    let c = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select(comments_of(db)));
    let v = users_with(db, UserWhere::All, "cv", any_post, &[&p, &c]);
    user_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "#d0", "#d1", "#up", "#down", "score_avg", "views_sum", "answers_sum"])
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.AnswerCount) AS AvgAnswersPerQuestion,
// AVG(P.CommentCount) AS AvgCommentsPerPost,
// COUNT(DISTINCT B.Id) AS TotalBadges
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// TotalPosts DESC, TotalScore DESC;
fn q13217(db: &'static So) -> String {
    let b = user_distinct(db, Ident::<User>::new(), UserWhere::All, badges_of(db));
    let v = users_with(db, UserWhere::All, "b", any_post, &[&b]);
    user_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "rep", "#n", "#q", "#a", "score_sum", "views_sum", "answers_avg", "cc_avg", "#d0"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// u.Views,
// u.UpVotes,
// u.DownVotes,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(p.Score) AS AveragePostScore,
// AVG(COALESCE(CHAR_LENGTH(p.Body), 0)) AS AveragePostLength,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.Views, u.UpVotes, u.DownVotes
// ORDER BY
// u.Reputation DESC;
fn q12042(db: &'static So) -> String {
    let v = users_with(db, UserWhere::All, "", any_post, &[]);
    user_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "rep", "ucreated", "uviews", "uup", "udown", "#n", "#q", "#a", "score_avg", "len_avg0", "created_max"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// MAX(p.CreationDate) AS LastPostDate
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
// LIMIT 100;
fn q13900(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let p = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db));
    let q = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))));
    let a = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2))));
    let c = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select(comments_of(db)));
    let b = user_distinct(db, Ident::<User>::new(), UserWhere::All, badges_of(db));
    let f = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cb", any_post);
    let mut v = Vec::new();
    f.and((&p).opt()).and((&q).opt()).and((&a).opt()).and((&c).opt()).and((&b).opt()).drive(|u, (((((s, p), q), a), c), b)| v.push((u, s, [p, q, a, c, b].map(|x| x.unwrap_or(0)))));
    v.sort_by_key(|x| Reverse(x.2[0]));
    rows(v.iter().take(100).map(|&(u, ref s, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d[0]), V::I(d[1]), V::I(d[2])];
        f.extend([ustat_field(s, "score_sum"), ustat_field(s, "views_sum"), V::I(d[3]), V::I(d[4]), ustat_field(s, "created_max")]);
        row(f)
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
// SUM(CASE WHEN V.Id IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate
// ORDER BY
// PostCount DESC
// LIMIT 100;
fn q10255(db: &'static So) -> String {
    let v = users_with(db, UserWhere::All, "cv", any_post, &[]);
    user_rows(db, v, |_, s, _| Reverse(s.n), 100, &["uid", "name", "#n", "#q", "#a", "#cx", "#vx", "rep", "ucreated", "last_access"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount,
// AVG(u.Reputation) AS AverageReputation,
// MAX(u.CreationDate) AS LatestAccountCreation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, u.CreationDate
// ORDER BY
// PostCount DESC;
fn q10752(db: &'static So) -> String {
    let v = users_with(db, UserWhere::All, "cv", any_post, &[]);
    rows(v.iter().map(|(u, s, _)| {
        let mut f = vec![user_col(db, *u, "uid"), user_col(db, *u, "name")];
        f.extend(["#n", "#q", "#a", "#cx", "#vx", "rep_avg"].iter().map(|c| ustat_field(s, c)));
        f.push(user_col(db, *u, "ucreated"));
        row(f)
    }))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
// SUM(CASE WHEN V.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// AVG(P.ViewCount) AS AvgViewCount,
// AVG(P.Score) AS AvgScore,
// MAX(P.CreationDate) AS MostRecentPostDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q10448(db: &'static So) -> String {
    let v = users_with(db, UserWhere::All, "cv", any_post, &[]);
    user_rows(db, v, |_, s, _| Reverse(s.n), 100, &["name", "#n", "#q", "#a", "#cx", "#vx", "views_avg", "score_avg", "created_max"])
}

// SELECT
// u.DisplayName AS UserDisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalWikiPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT ph.Id) AS TotalPostHistoryEntries
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
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q11173(db: &'static So) -> String {
    let c = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select(comments_of(db)));
    let h = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select(history_of(db)));
    let v = users_with(db, UserWhere::All, "ch", any_post, &[&c, &h]);
    user_rows(db, v, |_, s, _| Reverse(s.n), 100, &["name", "#n", "#q", "#a", "#45", "score_avg", "views_avg", "#d0", "#d1"])
}

// SELECT
// u.DisplayName AS UserDisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// MAX(p.CreationDate) AS LastPostDate,
// MIN(p.CreationDate) AS FirstPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// FETCH FIRST 100 ROWS ONLY;
fn q11971(db: &'static So) -> String {
    let mut v = Vec::new();
    user_stats_fold(db, &db.user.display_name, UserWhere::All, "cv", any_post).drive(|k, s| v.push((k, s)));
    v.sort_by_key(|x| Reverse(x.1.n));
    rows(v.iter().take(100).map(|(k, s)| {
        let mut f = vec![V::S(k)];
        f.extend(["#n", "#q", "#a", "score_avg", "#cx", "#vx", "created_max", "created_min"].iter().map(|c| ustat_field(s, c)));
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsPosted,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersPosted,
// SUM(p.ViewCount) AS TotalViews,
// SUM(v.BountyAmount) AS TotalBountyEarned,
// AVG(p.Score) AS AveragePostScore,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.UserId = u.Id
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q13489(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, creation_date, .. } = &db.post;
    let own_votes: HashIdx<(Id<User>, Id<Post>), Id<Vote>> = db.vote.select((&db.vote.user).and(&db.vote.post)).inv().collect();
    let to_post = Same::<(Id<User>, Id<Post>)>::new().map(|(_, p)| p);
    let joined = (&to_post)
        .select(post_type_id.and(view_count.opt()).and(score).and(creation_date).and(comments_of(db).opt()))
        .and((&own_votes).select((&db.vote.bounty_amount).opt()).opt());
    let rows_ = g(db).select(Ident::<User>::new().and(posts_of(db)).select(joined).opt()).fold([0, 0, 0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 9], r| match r {
        Some((((((t, w), s), c), _), b)) => {
            let b = b.flatten();
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0), a[6] + s, a[7] + 1, a[8].max(c)]
        }
        None => a,
    });
    let p = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db));
    let c = user_distinct(db, Ident::<User>::new(), UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    rows_.and((&p).opt()).and((&c).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.2));
    rows(v.iter().take(100).map(|&(u, a, p, c)| {
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(p),
            V::I(c),
            V::I(a[0]),
            V::I(a[1]),
            nullable(a[3], a[2]),
            nullable(a[5], a[4]),
            avg(a[6], a[7]),
            if a[7] == 0 { V::Null } else { V::T(a[8]) },
        ])
    }))
}

// SELECT
// u.Id AS UserId,
// u.Reputation,
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// u.Id, u.Reputation, p.Id, p.Title, p.CreationDate, p.ViewCount
// ORDER BY
// u.Reputation DESC, p.CreationDate DESC
// LIMIT 100;
fn q10873(db: &'static So) -> String {
    let base = owned(db).with((&db.post.owner_user).select(&db.user.creation_date).ge(year_ago()));
    stats_rows(db, stats_with(db, base, "cv", &[], &[]), |p, _| (Reverse(db.post.owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap())), newest(db, p)), 100, &["uid", "rep", "id", "title", "created", "views", "#cx", "#up", "#down"])
}

// SELECT
// P.Id AS PostID,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// U.DisplayName AS OwnerDisplayName,
// COUNT(C.Id) AS CommentCount,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBountyAmount,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q13149(db: &'static So) -> String {
    let b = badges_distinct(db);
    let v = stats_with(db, db.post.iq(), "cvb", &[8], &[&b]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, d)| (newest(db, p), p, s, d)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s, d)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score", "owner", "#cx"]);
        f.extend([V::I(s.bounty_sum), V::I(d[0])]);
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
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13967(db: &'static So) -> String {
    let b = badges_distinct(db);
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cvb", &[], &[&b]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.UserId) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN vt.Name = 'AcceptedByOriginator' THEN 1 ELSE 0 END) AS AcceptedVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q13958(db: &'static So) -> String {
    let voters = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    stats_rows(db, stats_with(db, since(db, date(2023, 1, 1)), "cv", &[], &[&voters]), |_, _| 0, 0, &["id", "title", "created", "#cx", "#d0", "#upn", "#downn", "#accn"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT ph.UserId) AS EditCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// PostHistory ph ON ph.PostId = p.Id
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11272(db: &'static So) -> String {
    let editors = per_post_distinct(db, history_of(db).select(&db.post_history.user_id));
    stats_rows(db, stats_with(db, since(db, month_ago()), "vch", &[], &[&editors]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "owner", "#vx", "#cx", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10271(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ORDER BY
// p.Score DESC, p.CreationDate DESC
// LIMIT 100;
fn q11535(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, answers_of(db));
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, all_questions(db), "cAv", &[], &[&c, &a, &x]), |p, _| score_then_newest(db, p), 100, &["id", "title", "created", "views", "score", "#d0", "#d1", "#d2", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// u.DisplayName AS UserDisplayName,
// u.Reputation AS UserReputation,
// COUNT(DISTINCT b.Id) AS BadgeCount
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
// p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score,
// u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q12074(db: &'static So) -> String {
    let b = badges_distinct(db);
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cvb", &[], &[&b]), |_, _| 0, 0, &["id", "title", "created", "views", "score", "#cx", "#vx", "owner", "rep", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// SUM(v.BountyAmount) AS TotalBountyAmount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11461(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, answers_of(db));
    stats_rows(db, stats_with(db, since(db, date(2023, 1, 1)), "cAv", &[], &[&c, &a]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "score", "#d0", "#d1", "bounty_sum", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT bh.Id) AS HistoryCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory bh ON p.Id = bh.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.ViewCount DESC, p.Score DESC
// LIMIT 100;
fn q12129(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    stats_rows(db, stats_with(db, owned(db), "cvh", &[], &[&h]), |p, _| (views_desc(db, p), Reverse(db.post.score.get(p).unwrap())), 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#up", "#down", "#d0"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.PostId IS NOT NULL THEN 1 END) AS VoteCount,
// COUNT(CASE WHEN PH.PostId IS NOT NULL THEN 1 END) AS HistoryCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, U.DisplayName, P.CreationDate, P.Score, P.ViewCount
// ORDER BY
// P.CreationDate DESC;
fn q14875(db: &'static So) -> String {
    stats_rows(db, stats_with(db, all_questions(db), "cvh", &[], &[]), |_, _| 0, 0, &["id", "title", "owner", "created", "score", "views", "#cx", "#vx", "#hx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q12628(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, questions(db), "cv", &[], &[&c, &x]), |_, _| 0, 0, &["id", "title", "created", "views", "score", "owner", "#d0", "#d1", "#upn", "#downn"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month')
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12226(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, month_ago()), "vc", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "owner", "#vx", "#up", "#down", "#cx"])
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
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
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
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10568(db: &'static So) -> String {
    stats_rows(db, stats_with(db, all_questions(db), "cvb", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "owner", "#cx", "#up", "#down", "#bx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= DATE '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.Score DESC, p.CreationDate DESC
// LIMIT 100;
fn q14762(db: &'static So) -> String {
    let v = stats_with(db, since(db, date(2023, 1, 1)), "cv", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (score_then_newest(db, p), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views"]);
        f.push(V::S(db.post.owner_user.get(*p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend(["#cx", "#vx", "#up", "#down"].iter().map(|c| stat_field(s, c).unwrap()));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount,
// p.ViewCount,
// p.Score
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ORDER BY
// p.CreationDate DESC;
fn q11450(db: &'static So) -> String {
    let b = badges_distinct(db);
    let r = per_post_distinct(db, links_of(db).select(&db.post_link.related_post_id));
    stats_rows(db, stats_with(db, all_questions(db), "cvbl", &[], &[&b, &r]), |_, _| 0, 0, &["id", "title", "created", "#cx", "#up", "#down", "#d0", "#d1", "views", "score"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// COUNT(DISTINCT pl.RelatedPostId) AS RelatedLinkCount
// FROM
// Posts p
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN PostLinks pl ON p.Id = pl.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10447(db: &'static So) -> String {
    let b = badges_distinct(db);
    let r = per_post_distinct(db, links_of(db).select(&db.post_link.related_post_id));
    stats_rows(db, stats_with(db, db.post.iq(), "cvbl", &[], &[&b, &r]), |p, _| newest(db, p), 100, &["id", "title", "created", "owner", "#cx", "#up", "#down", "#d0", "#d1"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// p.AnswerCount,
// p.FavoriteCount,
// CASE
// WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Yes'
// ELSE 'No'
// END AS HasAcceptedAnswer
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2020-01-01'
// GROUP BY
// p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.FavoriteCount, p.AcceptedAnswerId
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12116(db: &'static So) -> String {
    let v = stats_with(db, owned_since(db, date(2020, 1, 1)), "cv", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (newest(db, p), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "owner", "created", "score", "views", "#cx", "#vx", "answers", "favorites"]);
        f.push(V::S(if db.post.accepted_answer_id.get(*p).is_some() { "Yes" } else { "No" }));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AverageUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AverageDownVotes,
// COUNT(DISTINCT b.Id) AS BadgeCount
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
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName
// ORDER BY
// p.ViewCount DESC;
fn q12119(db: &'static So) -> String {
    let b = badges_distinct(db);
    stats_rows(db, stats_with(db, owned_since(db, date(2022, 1, 1)), "cvb", &[], &[&b]), |p, _| views_desc(db, p), 0, &["id", "title", "created", "views", "owner", "#cx", "#vx", "up_frac", "down_frac", "#d0"])
}

// SELECT
// u.DisplayName AS UserName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COALESCE(SUM(b.Class), 0) AS TotalBadges,
// p.LastActivityDate AS LastActivityDate
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
// p.PostTypeId = 1
// GROUP BY
// u.DisplayName, p.Title, p.CreationDate, p.LastActivityDate, u.Id, p.Id
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13705(db: &'static So) -> String {
    let v = stats_with(db, questions(db), "cvb", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (newest(db, p), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["owner", "title", "created", "#cx", "#up", "#down"]);
        f.push(V::I(s.bclass));
        f.extend(post_fields(db, *p, &["activity"]));
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
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// pt.Name AS PostTypeName
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= CURRENT_DATE - INTERVAL '6 months'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, pt.Name
// ORDER BY
// p.CreationDate DESC;
fn q10982(db: &'static So) -> String {
    stats_rows(db, stats_with(db, since(db, add_months(current_date(), -6)), "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down", "type"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(a.Id) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
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
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.Score DESC, p.ViewCount DESC
// LIMIT 100;
fn q12258(db: &'static So) -> String {
    let v = stats_with(db, all_questions(db), "cav", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| ((Reverse(db.post.score.get(p).unwrap()), views_desc(db, p)), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score"]);
        f.push(V::S(db.post.owner_user.get(*p).map_or("Community", |u| db.user.display_name.get(u).unwrap())));
        f.extend(["#cx", "#ax", "#up", "#down"].iter().map(|c| stat_field(s, c).unwrap()));
        row(f)
    }))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// P.ViewCount AS PostViewCount,
// P.Score AS PostScore,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount,
// SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
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
// P.PostTypeId IN (1, 2)
// GROUP BY
// U.DisplayName, P.Title, P.CreationDate, P.ViewCount, P.Score, U.Id, P.Id
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q13965(db: &'static So) -> String {
    let base = owned(db).with((&db.post.post_type_id).in_v(vec![1, 2]));
    stats_rows(db, stats_with(db, base, "cvb", &[], &[]), |p, _| newest(db, p), 100, &["owner", "title", "created", "views", "score", "#cx", "#vx", "#bx"])
}

// WITH PerformanceBenchmark AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// U.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Users U ON p.OwnerUserId = U.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, U.DisplayName
// )
// SELECT
// *,
// (CommentCount + VoteCount) AS EngagementScore
// FROM
// PerformanceBenchmark
// ORDER BY
// EngagementScore DESC
// LIMIT 10;
fn q13896(db: &'static So) -> String {
    let v = stats_with(db, db.post.iq(), "cv", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (Reverse(s.cx + s.vx), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(10).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx"]);
        f.push(V::I(s.cx + s.vx));
        row(f)
    }))
}

// The accepted-answer join is part of the group key.
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COALESCE(a.Body, 'No Accepted Answer') AS AcceptedAnswerBody
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Posts a ON p.AcceptedAnswerId = a.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, a.Body
// ORDER BY
// p.ViewCount DESC
// LIMIT 10;
fn q10606(db: &'static So) -> String {
    let key = Ident::<Post>::new().and((&db.post.accepted_answer).select(&db.post.body).opt());
    let mut v = group_stats(db, all_questions(db), key, "cv", &[]);
    v.sort_by_key(|&((p, _), _)| views_desc(db, p));
    rows(v.iter().take(10).map(|((p, b), s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "#cx", "#up", "#down"]);
        f.push(V::S(b.unwrap_or("No Accepted Answer")));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(a.AcceptedAnswerId, 0) AS AcceptedAnswerId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT ph.Id) AS PostHistoryCount
// FROM
// Posts p
// LEFT JOIN
// Posts a ON p.Id = a.AcceptedAnswerId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, a.AcceptedAnswerId
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14281(db: &'static So) -> String {
    let accepting: HashIdx<Id<Post>, Id<Post>> = (&db.post.accepted_answer).inv().collect();
    let key = Ident::<Post>::new().and((&accepting).select(&db.post.accepted_answer_id).opt());
    let s = stats_fold(db, all_questions(db), &key, "cvh", &[]);
    let x = all_questions(db).group_by(&key).select(votes_of(db)).count_distinct();
    let h = all_questions(db).group_by(&key).select(history_of(db)).count_distinct();
    let mut v = Vec::new();
    s.and((&x).opt()).and((&h).opt()).drive(|(p, a), ((s, x), h)| v.push((newest(db, p), p, a, s, x.unwrap_or(0), h.unwrap_or(0))));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, a, ref s, x, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a.unwrap_or(0)), V::I(s.cx), V::I(x), V::I(h)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COALESCE(a.AcceptedAnswerId, 0) AS AcceptedAnswerId
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Posts a ON p.Id = a.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, a.AcceptedAnswerId
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13506(db: &'static So) -> String {
    let key = Ident::<Post>::new().and(Ident::<Post>::new().select(&db.post.accepted_answer_id).opt());
    let mut v = group_stats(db, all_questions(db), key, "cv", &[]);
    v.sort_by_key(|&((p, _), _)| newest(db, p));
    rows(v.iter().take(100).map(|((p, a), s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "owner", "#cx", "#up", "#down"]);
        f.push(V::I(a.unwrap_or(0)));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COALESCE(a.Id, 0) AS AcceptedAnswerId
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Posts a ON p.AcceptedAnswerId = a.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation, a.Id
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11666(db: &'static So) -> String {
    let key = Ident::<Post>::new().and((&db.post.accepted_answer).select(&db.post.origid).opt());
    let mut v = group_stats(db, all_questions(db), key, "cv", &[]);
    v.sort_by_key(|&((p, _), _)| newest(db, p));
    rows(v.iter().take(100).map(|((p, a), s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views", "rep", "#cx", "#up", "#down"]);
        f.push(V::I(a.unwrap_or(0)));
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
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Tags t ON p.Id = t.ExcerptPostId OR p.Id = t.WikiPostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, t.TagName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14792(db: &'static So) -> String {
    let pairs: MatSet<(Id<Post>, Id<Tag>)> =
        db.tag.select((&db.tag.excerpt_post).and(Ident::<Tag>::new())).union(db.tag.select((&db.tag.wiki_post).and(Ident::<Tag>::new()))).collect();
    let tags_of: HashIdx<Id<Post>, Id<Tag>> = (&pairs).map(|(p, _)| p).inv().select((&pairs).map(|(_, t)| t)).collect();
    let key = Ident::<Post>::new().and((&tags_of).select(&db.tag.tag_name).opt());
    let mut v = group_stats(db, questions(db), key, "cv", &[]);
    v.sort_by_key(|&((p, t), _)| (newest(db, p), db.post.origid.get(p).unwrap(), t.is_none(), t));
    rows(v.iter().take(100).map(|((p, t), s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "owner", "#cx", "#vx", "#up", "#down"]);
        f.push(ostr(*t));
        row(f)
    }))
}

// SELECT
// p.Title AS PostTitle,
// u.DisplayName AS Author,
// p.CreationDate AS PostCreationDate,
// COUNT(c.Id) AS CommentCount,
// MAX(v.CreationDate) AS LastVoteDate,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 DAY'
// GROUP BY
// p.Title, u.DisplayName, p.CreationDate
// ORDER BY
// PostCreationDate DESC;
fn q12636(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let key = title.opt().and(owner_user.select(&db.user.display_name)).and(creation_date);
    let v = group_stats(db, owned_since(db, month_ago()), key, "cv", &[]);
    rows(v.iter().map(|&(((t, dn), c), ref s)| {
        let mut f = vec![ostr(t), V::S(dn), V::T(c)];
        f.extend(["#cx", "vmax", "#upn", "#downn"].iter().map(|x| stat_field(s, x).unwrap()));
        row(f)
    }))
}

// SELECT
// u.DisplayName AS UserName,
// p.Title AS PostTitle,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// MAX(b.Date) AS LastBadgeDate
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// AND p.PostTypeId = 1
// AND p.Score > 0
// GROUP BY
// u.DisplayName,
// p.Title
// ORDER BY
// UpVotes DESC,
// CommentCount DESC
// LIMIT 10;
fn q7551(db: &'static So) -> String {
    let Post { title, owner_user, post_type_id, score, creation_date, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(year_ago()).and(post_type_id.eq(1)).and(score.gt(0)));
    let key = owner_user.select(&db.user.display_name).and(title.opt());
    let mut v = group_stats(db, base, key, "cvb", &[]);
    v.sort_by_key(|&(_, ref s)| (Reverse(s.up), Reverse(s.cx)));
    rows(v.iter().take(10).map(|&((dn, t), ref s)| {
        let mut f = vec![V::S(dn), ostr(t)];
        f.extend(["#cx", "#up", "#down", "bmax"].iter().map(|x| stat_field(s, x).unwrap()));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVotes,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVotes,
// (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = p.Id) AS HistoryCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12349(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, month_ago()).select(votes_of_type(db, 2).and(votes_of_type(db, 3)).and(history_per_post(db))).drive(|p, a| v.push((newest(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, ((u, d), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "uid", "owner", "rep", "score", "views", "answers", "comments", "favorites"]);
        f.extend([V::I(u), V::I(d), V::I(h)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
// COALESCE((SELECT COUNT(*) FROM Posts a WHERE a.ParentId = p.Id), 0) AS AnswerCount,
// COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVoteCount,
// COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVoteCount,
// u.Reputation AS UserReputation,
// u.DisplayName AS UserDisplayName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13375(db: &'static So) -> String {
    let mut v = Vec::new();
    questions(db)
        .select(comments_per_post(db).and(answers_per_post(db)).and(votes_of_type(db, 2)).and(votes_of_type(db, 3)))
        .drive(|p, a| v.push((newest(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, (((c, a), u), d))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(a), V::I(u), V::I(d)]);
        f.extend(post_fields(db, p, &["rep", "owner"]));
        row(f)
    }))
}

// WITH Benchmark AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastHistoryDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// )
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AvgViewCount,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(Score) AS AvgScore,
// MAX(LastHistoryDate) AS LastHistoryUpdate
// FROM
// Benchmark;
fn q13307(db: &'static So) -> String {
    let per_post = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cvh", &[]);
    let Post { view_count, score, .. } = &db.post;
    let (n, vn, vs, c, s, h) = (&per_post).and(view_count.opt()).and(score).fold_flat((0i64, 0i64, 0i64, 0i64, 0i64, i64::MIN), |(n, vn, vs, c, sc, h), ((st, w), x)| {
        (n + 1, vn + w.is_some() as i64, vs + w.unwrap_or(0), c + st.cx, sc + x, if st.hx == 0 { h } else { h.max(st.hmax) })
    });
    row(vec![V::I(n), avg(vs, vn), avg(c, n), avg(s, n), if h == i64::MIN { V::Null } else { V::T(h) }])
}

// Posts grouped by `key`, folding what each post is joined to.
fn fold_by<Q, K, R, S, F>(base: Q, key: K, joined: R, init: S, f: F) -> Fold<ROf<K>, S>
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

fn by_first<K: Copy + Eq + std::hash::Hash, const N: usize>(f: &Fold<K, [i64; N]>) -> Vec<(K, [i64; N])> {
    let mut v = Vec::new();
    f.drive(|k, a| v.push((k, a)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    v
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 END), 0) AS AcceptedAnswers,
// COALESCE(SUM(p.ViewCount) FILTER (WHERE p.ViewCount IS NOT NULL), 0) AS TotalViews,
// COALESCE(AVG(p.Score), 0) AS AverageScore,
// COALESCE(MAX(p.CreationDate), '1970-01-01') AS MostRecentPostDate,
// COALESCE(MIN(p.CreationDate), '1970-01-01') AS EarliestPostDate,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueOwners
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11834(db: &'static So) -> String {
    let Post { accepted_answer_id, view_count, score, creation_date, owner_user_id, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db), accepted_answer_id.opt().and(view_count.opt()).and(score).and(creation_date), [0, 0, 0, 0, i64::MIN, i64::MAX], |a: [i64; 6], (((ac, w), s), c)| {
        [a[0] + 1, a[1] + ac.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4].max(c), a[5].min(c)]
    });
    let u = db.post.group_by(name(db)).select(owner_user_id).count_distinct();
    let mut v = Vec::new();
    f.and((&u).opt()).drive(|k, (a, u)| v.push((k, a, u.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a, u)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::T(a[4]), V::T(a[5]), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore,
// SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN vt.VoteTypeId = 10 THEN 1 ELSE 0 END) AS TotalCloseVotes,
// SUM(CASE WHEN vt.VoteTypeId = 11 THEN 1 ELSE 0 END) AS TotalReopenVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes vt ON p.Id = vt.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q12471(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(year_ago()));
    let f = fold_by(base, name(db), (&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt()), [0i64; 6], |a, (s, x)| {
        [a[0] + 1, a[1] + s, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64, a[4] + (x == Some(10)) as i64, a[5] + (x == Some(11)) as i64]
    });
    rows(by_first(&f).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[5])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// AVG(p.ViewCount) AS AvgViewsPerPost,
// SUM(p.AnswerCount) AS TotalAnswers,
// SUM(p.CommentCount) AS TotalComments,
// SUM(CASE WHEN p.FavoriteCount > 0 THEN 1 ELSE 0 END) AS FavoritePosts,
// COUNT(DISTINCT u.Id) AS UniqueUsers,
// SUM(CASE WHEN u.Reputation > 1000 THEN 1 ELSE 0 END) AS HighReputationUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2020-01-01'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13133(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, favorite_count, owner_user, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2020, 1, 1)));
    let f = fold_by(
        &base,
        name(db),
        score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()).and(owner_user.select(&db.user.reputation).opt()),
        [0i64; 9],
        |a, (((((s, w), an), c), fv), r)| {
            [a[0] + 1, a[1] + (s > 0) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c, a[7] + fv.is_some_and(|x| x > 0) as i64, a[8] + r.is_some_and(|x| x > 1000) as i64]
        },
    );
    let u = (&base).group_by(name(db)).select(owner_user).count_distinct();
    let mut v = Vec::new();
    f.and((&u).opt()).drive(|k, (a, u)| v.push((k, a, u.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a, u)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), nullable(a[5], a[4]), V::I(a[6]), V::I(a[7]), V::I(u), V::I(a[8])])))
}

// SELECT
// PT.Name AS PostType,
// COUNT(P.Id) AS TotalPosts,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// COUNT(DISTINCT C.Id) AS TotalComments
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// PT.Name
// ORDER BY
// TotalPosts DESC;
fn q14596(db: &'static So) -> String {
    let Post { view_count, score, post_type_id, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db), view_count.opt().and(score).and(post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()), [0i64; 8], |a, ((((w, s), t), x), _)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + (t == 1) as i64, a[5] + (t == 2) as i64, a[6] + (x == Some(2)) as i64, a[7] + (x == Some(3)) as i64]
    });
    let c = db.post.group_by(name(db)).select(comments_of(db)).count_distinct();
    let mut v = Vec::new();
    f.and((&c).opt()).drive(|k, (a, c)| v.push((k, a, c.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a, c)| row(vec![V::S(k), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), V::I(a[4]), V::I(a[5]), V::I(a[6]), V::I(a[7]), V::I(c)])))
}

// SELECT
// PT.Name AS PostType,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore,
// AVG(P.AnswerCount) AS AverageAnswers,
// AVG(P.CommentCount) AS AverageComments,
// AVG(P.FavoriteCount) AS AverageFavorites,
// U.Reputation AS UserReputation,
// COUNT(DISTINCT U.Id) AS ActiveUsers
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// PT.Name, U.Reputation
// ORDER BY
// TotalPosts DESC;
fn q10604(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, favorite_count, owner_user, creation_date, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(year_ago()));
    let key = name(db).and(owner_user.select(&db.user.reputation));
    let f = fold_by(&base, &key, score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()), [0i64; 8], |a, ((((s, w), an), c), fv)| {
        [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + an.is_some() as i64, a[4] + an.unwrap_or(0), a[5] + c, a[6] + fv.is_some() as i64, a[7] + fv.unwrap_or(0)]
    });
    let u = (&base).group_by(&key).select(owner_user).count_distinct();
    let mut v = Vec::new();
    f.and(&u).drive(|(k, r), (a, u)| v.push((k, r, a, u)));
    rows(v.iter().map(|&(k, r, a, u)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), avg(a[5], a[0]), avg(a[7], a[6]), V::I(r), V::I(u)])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10026", q10026),
    ("11119", q11119),
    ("12435", q12435),
    ("11774", q11774),
    ("13217", q13217),
    ("12042", q12042),
    ("13900", q13900),
    ("10255", q10255),
    ("10752", q10752),
    ("10448", q10448),
    ("11173", q11173),
    ("11971", q11971),
    ("13489", q13489),
    ("10873", q10873),
    ("13149", q13149),
    ("13967", q13967),
    ("13958", q13958),
    ("11272", q11272),
    ("10271", q10271),
    ("11535", q11535),
    ("12074", q12074),
    ("11461", q11461),
    ("12129", q12129),
    ("14875", q14875),
    ("12628", q12628),
    ("12226", q12226),
    ("10568", q10568),
    ("14762", q14762),
    ("11450", q11450),
    ("10447", q10447),
    ("12116", q12116),
    ("12119", q12119),
    ("13705", q13705),
    ("10982", q10982),
    ("12258", q12258),
    ("13965", q13965),
    ("13896", q13896),
    ("10606", q10606),
    ("14281", q14281),
    ("13506", q13506),
    ("11666", q11666),
    ("14792", q14792),
    ("12636", q12636),
    ("7551", q7551),
    ("12349", q12349),
    ("13375", q13375),
    ("13307", q13307),
    ("11834", q11834),
    ("12471", q12471),
    ("13133", q13133),
    ("14596", q14596),
    ("10604", q10604),
];
