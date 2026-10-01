use harness::prelude::*;
use std::cmp::Reverse;

fn names(db: &'static So, w: UserWhere, by: fn(&NameAgg) -> i64, n: usize, cols: &[&str]) -> String {
    let mut v = user_groups(db, &db.user.display_name, w);
    v.sort_by_key(|(_, a)| Reverse(by(a)));
    let n = if n == 0 { v.len() } else { n };
    rows(v.iter().take(n).map(|(k, a)| {
        let mut f = vec![V::S(k)];
        f.extend(cols.iter().map(|c| name_field(a, c)));
        row(f)
    }))
}

fn posts(a: &NameAgg) -> i64 {
    a.n
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q19337(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a"])
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS NumberOfPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS NumberOfQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS NumberOfAnswers
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// NumberOfPosts DESC
// LIMIT 10;
fn q17173(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a"])
}

// SELECT
// Users.DisplayName,
// COUNT(Posts.Id) AS TotalPosts,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q16535(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a"])
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q16023(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a", "rep_avg"])
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q17871(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a", "rep_avg"])
}

// SELECT u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(u.Reputation) AS AverageReputation
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.DisplayName
// ORDER BY PostCount DESC
// LIMIT 10;
fn q19629(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a", "rep_avg"])
}

// SELECT
// u.DisplayName AS UserDisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q16513(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a", "rep_avg"])
}

// SELECT
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(P.Score) AS AverageScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q15225(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a", "score_avg"])
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId IN (5, 4) THEN 1 ELSE 0 END) AS TagWikis
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q18739(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a", "#45"])
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q16141(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a", "#45"])
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q16297(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a", "#3"])
}

// SELECT
// Users.DisplayName,
// COUNT(Posts.Id) AS NumberOfPosts,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS NumberOfQuestions,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS NumberOfAnswers,
// SUM(Posts.Score) AS TotalScore
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.DisplayName
// ORDER BY
// NumberOfPosts DESC
// LIMIT 10;
fn q17296(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a", "score_sum"])
}

// SELECT
// Users.DisplayName,
// COUNT(Posts.Id) AS NumberOfPosts,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS NumberOfQuestions,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS NumberOfAnswers,
// SUM(Posts.ViewCount) AS TotalViews
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.DisplayName
// ORDER BY
// NumberOfPosts DESC
// LIMIT 10;
fn q17866(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#q", "#a", "views_sum"])
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q16502(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#qn", "#an"])
}

// SELECT
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN PT.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN PT.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// GROUP BY
// U.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q19025(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#an", "#qn"])
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q18959(db: &'static So) -> String {
    names(db, UserWhere::All, posts, 10, &["#n", "#qn", "#an"])
}

// SELECT
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts,
// SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPosts
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q15120(db: &'static So) -> String {
    let mut v = user_groups(db, Ident::<User>::new(), UserWhere::All);
    v.sort_by_key(|(_, a)| Reverse(a.n));
    rows(v.iter().take(10).map(|(u, a)| {
        let mut f = vec![V::S(db.user.display_name.get(*u).unwrap())];
        f.extend(["#n", "#pos", "#neg"].iter().map(|c| name_field(a, c)));
        row(f)
    }))
}

// AVG(EXTRACT(EPOCH FROM ...)) averages doubles, and DuckDB's printed answer
// moves with SET threads for 859 of the users, so the query has no single
// answer. rewrites/12732.sql takes the exact-integer mean instead, as 5603's does.
//
// SELECT
// U.Id AS UserId,
// U.DisplayName AS UserName,
// COUNT(P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// AVG(EXTRACT(EPOCH FROM P.CreationDate)) AS AveragePostCreationDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// PostCount DESC;
fn q12732(db: &'static So) -> String {
    let v = user_groups(db, Ident::<User>::new(), UserWhere::All);
    rows(v.iter().map(|(u, a)| {
        let mut f = vec![V::I(db.user.origid.get(*u).unwrap()), V::S(db.user.display_name.get(*u).unwrap())];
        f.extend(["#n", "views_sum", "score_sum", "created_avg"].iter().map(|c| name_field(a, c)));
        row(f)
    }))
}

// SELECT
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName,
// u.Reputation
// ORDER BY
// u.Reputation DESC
// LIMIT 10;
fn q15330(db: &'static So) -> String {
    let mut v = user_groups(db, (&db.user.display_name).and(&db.user.reputation), UserWhere::All);
    v.sort_by_key(|((_, r), _)| Reverse(*r));
    rows(v.iter().take(10).map(|((k, r), a)| {
        let mut f = vec![V::S(k), V::I(*r)];
        f.extend(["#n", "#q", "#a"].iter().map(|c| name_field(a, c)));
        row(f)
    }))
}

// Users LEFT JOIN Posts LEFT JOIN Votes GROUP BY u.DisplayName.
fn names_votes(db: &'static So, w: UserWhere, n: usize) -> String {
    let mut v = Vec::new();
    user_base(db, w)
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold((0i64, 0i64, 0i64), |(n, u, d), p| match p {
            Some(t) => (n + 1, u + (t == Some(2)) as i64, d + (t == Some(3)) as i64),
            None => (n, u, d),
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|x| Reverse(x.1.0));
    let n = if n == 0 { v.len() } else { n };
    rows(v.iter().take(n).map(|&(k, (n, u, d))| row(vec![V::S(k), V::I(n), V::I(u), V::I(d)])))
}

// SELECT
// Users.DisplayName,
// COUNT(Posts.Id) AS PostCount,
// SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// GROUP BY
// Users.DisplayName
// ORDER BY
// PostCount DESC;
fn q15128(db: &'static So) -> String {
    names_votes(db, UserWhere::All, 0)
}

// SELECT
// Users.DisplayName,
// COUNT(Posts.Id) AS NumberOfPosts,
// SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// GROUP BY
// Users.DisplayName
// ORDER BY
// NumberOfPosts DESC;
fn q15806(db: &'static So) -> String {
    names_votes(db, UserWhere::All, 0)
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation > 100
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q18746(db: &'static So) -> String {
    names_votes(db, UserWhere::RepGt(100), 10)
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// AVG(V.VoteTypeId) AS AverageVotesPerPost
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// U.Reputation DESC;
fn q12018(db: &'static So) -> String {
    let votes = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold((0i64, 0i64), |(n, s), t| match t.flatten() {
            Some(t) => (n + 1, s + t),
            None => (n, s),
        });
    let posted = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let mut v = Vec::new();
    votes.and((&posted).opt()).drive(|u, ((n, s), c)| v.push((u, n, s, c.unwrap_or(0))));
    rows(v.iter().map(|&(u, n, s, c)| {
        row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(db.user.reputation.get(u).unwrap()), V::I(c), avg(s, n)])
    }))
}

// The newest questions with correlated counts of their children.
fn newest_questions<R>(db: &'static So, counts: R, cols: &[&str]) -> String
where
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>>,
    ROf<R>: Into<Vec<i64>>,
{
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(counts).drive(|p, n| v.push((Reverse(db.post.creation_date.get(p).unwrap()), p, n.into())));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(10).map(|(_, p, n)| {
        let mut f = post_fields(db, *p, cols);
        f.extend(n.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15152(db: &'static So) -> String {
    newest_questions(db, comments_per_post(db).map(|n| [n]), &["id", "title", "owner", "created", "score", "views"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16443(db: &'static So) -> String {
    newest_questions(db, comments_per_post(db).map(|n| [n]), &["id", "title", "owner", "created", "views", "score"])
}

// SELECT
// p.Id AS PostID,
// p.Title,
// u.DisplayName AS Owner,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q18312(db: &'static So) -> String {
    newest_questions(db, comments_per_post(db).map(|n| [n]), &["id", "title", "owner", "created", "views", "score"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Owner,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17164(db: &'static So) -> String {
    newest_questions(db, comments_per_post(db).and(votes_per_post(db)).map(|(c, v)| [c, v]), &["id", "title", "created", "owner"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// WHERE p.PostTypeId = 1
// ORDER BY p.CreationDate DESC
// LIMIT 10;
fn q16325(db: &'static So) -> String {
    newest_questions(db, comments_per_post(db).and(votes_of_type(db, 2)).map(|(c, v)| [c, v]), &["id", "title", "created", "owner"])
}

// SELECT
// p.Title,
// p.CreationDate,
// u.Reputation,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Title, p.CreationDate, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT
// 100;
fn q11375(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(ts(2024, 9, 1, 12, 34, 56)));
    let mut v = group_stats(db, base, title.opt().and(creation_date).and(owner_user.select(&db.user.reputation)), "v", &[]);
    v.sort_by_key(|&(((_, c), _), _)| Reverse(c));
    rows(v.iter().take(100).map(|&(((t, c), r), ref s)| row(vec![ostr(t), V::T(c), V::I(r), V::I(s.vx)])))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// AVG(P.Score) AS AvgScore,
// SUM(P.AnswerCount) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// P.PostTypeId = 1
// AND B.Id IS NOT NULL
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// TotalViews DESC
// LIMIT 10;
fn q10698(db: &'static So) -> String {
    let Post { score, answer_count, view_count, owner_user, post_type_id, .. } = &db.post;
    let mut v = Vec::new();
    owned(db)
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(score.and(answer_count.opt()).and(view_count.opt()).and(owner_user.select(badges_of(db))))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, an, as_, vn, vs), (((x, a), w), _)| {
            (n + 1, s + x, an + a.is_some() as i64, as_ + a.unwrap_or(0), vn + w.is_some() as i64, vs + w.unwrap_or(0))
        })
        .drive(|u, a| v.push((u, a)));
    v.sort_by_key(|&(_, (_, _, _, _, vn, vs))| (vn == 0, Reverse(vs)));
    rows(v.iter().take(10).map(|&(u, (n, s, an, as_, vn, vs))| {
        row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), avg(s, n), nullable(as_, an), nullable(vs, vn)])
    }))
}

// Types, with whatever they are joined to folded over the joined rows.
fn by_type<Q, R, S, F>(db: &'static So, base: Q, joined: R, init: S, f: F) -> Fold<Str, S>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>>,
    S: Copy,
    F: Fn(S, ROf<R>) -> S,
{
    base.group_by((&db.post.post_type).select(&db.post_type.name)).select(joined).fold(init, f)
}

fn listed<S: Copy>(f: Fold<Str, S>) -> Vec<(Str, S)> {
    let mut v = Vec::new();
    f.drive(|k, a| v.push((k, a)));
    v
}

// A type's fold beside a COUNT(DISTINCT ...) over the same group, which has
// no entry where every row's value was NULL.
fn with_distinct<S: Copy>(f: Fold<Str, S>, d: Fold<Str, i64>) -> Vec<(Str, (S, i64))> {
    let mut v = Vec::new();
    f.and((&d).opt()).drive(|k, (a, n)| v.push((k, (a, n.unwrap_or(0)))));
    v
}

fn distinct_by_type<Q, R>(db: &'static So, base: Q, r: R) -> Fold<Str, i64>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>>,
    ROf<R>: Ord,
{
    base.group_by((&db.post.post_type).select(&db.post_type.name)).select(r).count_distinct()
}

fn by_count<S: Copy>(mut v: Vec<(Str, S)>, n: impl Fn(&S) -> i64) -> Vec<(Str, S)> {
    v.sort_by_key(|x| Reverse(n(&x.1)));
    v
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViewCount,
// COUNT(DISTINCT v.UserId) AS TotalVoters
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10434(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let v = by_type(db, db.post.iq(), score.and(view_count.opt()).and(votes_of(db).opt()), [0i64; 4], |a, ((s, w), _)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let voters = distinct_by_type(db, db.post.iq(), votes_of(db).select(&db.vote.user_id));
    rows(by_count(with_distinct(v, voters), |a| a.0[0]).iter().map(|&(k, (a, u))| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// MAX(p.ViewCount) AS MaxViews,
// COUNT(DISTINCT c.Id) AS TotalComments
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
fn q14454(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let v = by_type(db, db.post.iq(), score.and(view_count.opt()).and(comments_of(db).opt()), [0, 0, 0, i64::MIN], |a: [i64; 4], ((s, w), _)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, w.map_or(a[3], |x| a[3].max(x))]
    });
    let cs = distinct_by_type(db, db.post.iq(), comments_of(db));
    rows(by_count(with_distinct(v, cs), |a| a.0[0]).iter().map(|&(k, (a, c))| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), omax(a[3], a[2]), V::I(c)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AverageViewCount,
// SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13101(db: &'static So) -> String {
    let Post { view_count, .. } = &db.post;
    let v = by_type(db, db.post.iq(), view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()), [0i64; 4], |a, (w, t)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + matches!(t, Some(2) | Some(3)) as i64]
    });
    rows(by_count(listed(v), |a| a[0]).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), V::I(a[3])])))
}

// Types LEFT JOIN Users: COUNT, SUM(Score), SUM(ViewCount), SUM/AVG(u.Reputation), COUNT(DISTINCT u.Id).
fn types_users(db: &'static So) -> Vec<(Str, [i64; 6], i64)> {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let v = by_type(db, db.post.iq(), score.and(view_count.opt()).and(owner_user.select(&db.user.reputation).opt()), [0i64; 6], |a, ((s, w), r)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)]
    });
    let users = distinct_by_type(db, db.post.iq(), owner_user);
    by_count(with_distinct(v, users), |a| a.0[0]).into_iter().map(|(k, (a, u))| (k, a, u)).collect()
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(u.Reputation) AS TotalUserReputation,
// COUNT(DISTINCT u.Id) AS UniqueUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q12932(db: &'static So) -> String {
    rows(types_users(db).iter().map(|&(k, a, u)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[5], a[4]), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(u.Reputation) AS AverageUserReputation,
// COUNT(DISTINCT u.Id) AS TotalUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12921(db: &'static So) -> String {
    rows(types_users(db).iter().map(|&(k, a, u)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[5], a[4]), V::I(u)])))
}

// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(u.Reputation) AS AverageUserReputation,
// COUNT(DISTINCT u.Id) AS UniqueUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14412(db: &'static So) -> String {
    rows(types_users(db).iter().map(|&(k, a, u)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[5], a[4]), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(u.Reputation) AS AverageUserReputation,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q10651(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let v = by_type(db, owned(db), owner_user.select(&db.user.reputation).and(owner_user.select(badges_of(db)).opt()), [0i64; 2], |a, (r, _)| [a[0] + 1, a[1] + r]);
    let bs = distinct_by_type(db, owned(db), owner_user.select(badges_of(db)));
    rows(by_count(with_distinct(v, bs), |a| a.0[0]).iter().map(|&(k, (a, b))| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(b)])))
}

fn count<Q: Drive>(q: Q) -> i64 {
    q.fold_flat(0i64, |a, _| a + 1)
}

fn mean<Q: Drive<R = i64>>(q: Q) -> V {
    let (n, s) = q.fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    avg(s, n)
}

fn type_score_views(db: &'static So, users: i64, badges: i64) -> String {
    rows(by_count(type_aggs(db).iter().map(|a| (a.name, *a)).collect(), |a| a.n).iter().map(|(_, a)| {
        let mut f = type_fields(a, &["name", "n", "score_avg", "views_avg"]);
        f.extend([V::I(users), V::I(badges)]);
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10247(db: &'static So) -> String {
    type_score_views(db, count(db.user.iq()), count(db.badge.iq()))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q13874(db: &'static So) -> String {
    type_score_views(db, count(db.user.iq()), count(db.badge.iq()))
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges,
// (SELECT COUNT(*) FROM Tags) AS TotalTags,
// (SELECT COUNT(*) FROM PostHistory) AS TotalPostHistory
fn q13465(db: &'static So) -> String {
    row(vec![
        V::I(count(db.post.iq())),
        V::I(count(db.user.iq())),
        V::I(count(db.vote.iq())),
        V::I(count(db.comment.iq())),
        V::I(count(db.badge.iq())),
        V::I(count(db.tag.iq())),
        V::I(count(db.post_history.iq())),
    ])
}

// SELECT
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT AVG(CAST(Reputation AS FLOAT)) FROM Users) AS AverageReputation,
// (SELECT AVG(CAST(Score AS FLOAT)) FROM Posts) AS AveragePostScore
// FROM
// (SELECT 1) AS dummy;
fn q11587(db: &'static So) -> String {
    row(vec![V::I(count(db.user.iq())), V::I(count(db.post.iq())), V::I(count(db.comment.iq())), V::I(count(db.vote.iq())), mean(&db.user.reputation), mean(&db.post.score)])
}

// SELECT
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT AVG(ViewCount) FROM Posts) AS AveragePostViews,
// (SELECT AVG(Score) FROM Posts) AS AveragePostScore,
// (SELECT AVG(Reputation) FROM Users) AS AverageUserReputation
fn q12650(db: &'static So) -> String {
    let views = db.post.select(&db.post.view_count);
    row(vec![
        V::I(count(db.user.iq())),
        V::I(count(db.post.iq())),
        V::I(count(db.vote.iq())),
        V::I(count(db.comment.iq())),
        mean(views),
        mean(&db.post.score),
        mean(&db.user.reputation),
    ])
}

// SELECT
// PT.Name AS PostType,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT V.Id) AS TotalVotes,
// COUNT(DISTINCT C.Id) AS TotalComments
// FROM
// PostTypes PT
// LEFT JOIN
// Posts P ON P.PostTypeId = PT.Id
// LEFT JOIN
// Votes V ON V.PostId = P.Id
// LEFT JOIN
// Comments C ON C.PostId = P.Id
// GROUP BY
// PT.Id, PT.Name
// ORDER BY
// TotalPosts DESC;
fn q14429(db: &'static So) -> String {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let posts = db.post_type.group_by(Ident::<PostType>::new()).select(&of_type).count_distinct();
    let votes = db.post_type.group_by(Ident::<PostType>::new()).select((&of_type).select(votes_of(db))).count_distinct();
    let comments = db.post_type.group_by(Ident::<PostType>::new()).select((&of_type).select(comments_of(db))).count_distinct();
    let mut v = Vec::new();
    db.post_type.select((&posts).opt().and((&votes).opt()).and((&comments).opt())).drive(|t, ((p, x), c)| v.push((t, p.unwrap_or(0), x.unwrap_or(0), c.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1));
    rows(v.iter().map(|&(t, p, x, c)| row(vec![V::S(db.post_type.name.get(t).unwrap()), V::I(p), V::I(x), V::I(c)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(u.Reputation), 0) AS TotalUserReputation,
// COUNT(ph.Id) AS PostHistoryCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q14477(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let v = by_type(db, db.post.iq(), owner_user.select(&db.user.reputation).opt().and(history_of(db).opt()), [0i64; 3], |a, (r, h)| {
        [a[0] + 1, a[1] + r.unwrap_or(0), a[2] + h.is_some() as i64]
    });
    rows(by_count(listed(v), |a| a[0]).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12118(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(ts(2024, 9, 1, 12, 34, 56)));
    let v = by_type(db, base, (&db.post.score).and(comments_of(db).opt()), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c.is_some() as i64]);
    rows(by_count(listed(v), |a| a[0]).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13007(db: &'static So) -> String {
    let Post { score, view_count, owner_user_id, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2023, 1, 1)));
    let v = by_type(db, &base, score.and(view_count.opt()), [0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let users = distinct_by_type(db, &base, owner_user_id);
    rows(by_count(with_distinct(v, users), |a| a.0[0]).iter().map(|&(k, (a, u))| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(u)])))
}

// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q13105(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(date(2023, 1, 1)));
    let v = by_type(db, &base, (&db.post.owner_user).select(&db.user.reputation).and(comments_of(db).opt()), [0i64; 2], |a, (r, _)| [a[0] + 1, a[1] + r]);
    let cs = distinct_by_type(db, &base, comments_of(db));
    rows(by_count(with_distinct(v, cs), |a| a.0[0]).iter().map(|&(k, (a, c))| row(vec![V::S(k), V::I(a[0]), V::I(c), avg(a[1], a[0])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT u.Id) AS UserCount,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// PostType;
fn q10149(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let v = by_type(db, db.post.iq(), score.and(view_count.opt()).and(comments_of(db).opt()), [0i64; 5], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c.is_some() as i64]
    });
    let users = distinct_by_type(db, db.post.iq(), owner_user);
    rows(with_distinct(v, users).iter().map(|&(k, (a, u))| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(u), V::I(a[4])])))
}

// SELECT
// PT.Name AS PostTypeName,
// COUNT(P.Id) AS TotalPosts,
// AVG(P.Score) AS AverageScore,
// COUNT(DISTINCT U.Id) AS TotalUsers,
// COUNT(DISTINCT T.TagName) AS TotalTags
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Tags T ON P.Tags LIKE '%' || T.TagName || '%'
// GROUP BY
// PT.Name
// ORDER BY
// TotalPosts DESC;
fn q10587(db: &'static So) -> String {
    let mentions = tag_mentions(db);
    let tags_of: HashIdx<Id<Post>, Id<Tag>> = (&mentions).map(|(p, _)| p).inv().select((&mentions).map(|(_, t)| t)).collect();
    let Post { score, owner_user, .. } = &db.post;
    let v = by_type(db, db.post.iq(), score.and((&tags_of).opt()), [0i64; 2], |a, (s, _)| [a[0] + 1, a[1] + s]);
    let users = distinct_by_type(db, db.post.iq(), owner_user);
    let tags = distinct_by_type(db, db.post.iq(), (&tags_of).select(&db.tag.tag_name));
    let mut out = Vec::new();
    v.and((&users).opt()).and((&tags).opt()).drive(|k, ((a, u), n)| out.push((k, a, u.unwrap_or(0), n.unwrap_or(0))));
    out.sort_by_key(|x| Reverse(x.1[0]));
    rows(out.iter().map(|&(k, a, u, n)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(u), V::I(n)])))
}

// SELECT
// COUNT(DISTINCT p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AverageViewsPerPost,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// COUNT(DISTINCT t.Id) AS TotalTags
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id OR t.WikiPostId = p.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// AND p.PostTypeId = 1;
fn q13635(db: &'static So) -> String {
    let Post { view_count, score, owner_user, post_type_id, creation_date, .. } = &db.post;
    let pairs: MatSet<(Id<Post>, Id<Tag>)> =
        db.tag.select((&db.tag.excerpt_post).and(Ident::<Tag>::new())).union(db.tag.select((&db.tag.wiki_post).and(Ident::<Tag>::new()))).collect();
    let tags_of: HashIdx<Id<Post>, Id<Tag>> = (&pairs).map(|(p, _)| p).inv().select((&pairs).map(|(_, t)| t)).collect();
    let base = owned(db).with(creation_date.ge(date(2023, 1, 1)).and(post_type_id.eq(1)));
    let (vn, vs, n, s) = (&base).select(view_count.opt().and(score).and((&tags_of).opt())).fold_flat((0i64, 0i64, 0i64, 0i64), |(vn, vs, n, s), ((w, x), _)| {
        (vn + w.is_some() as i64, vs + w.unwrap_or(0), n + 1, s + x)
    });
    let distinct = |f: Fold<(), i64>| (&f).fold_flat(0i64, |a, x| a + x);
    let posts = distinct(whole(&base).select(Ident::<Post>::new()).count_distinct());
    let users = distinct(whole(&base).select(owner_user).count_distinct());
    let tags = distinct(whole(&base).select(&tags_of).count_distinct());
    row(vec![V::I(posts), avg(vs, vn), avg(s, n), V::I(users), V::I(tags)])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("19337", q19337),
    ("17173", q17173),
    ("16535", q16535),
    ("16023", q16023),
    ("17871", q17871),
    ("19629", q19629),
    ("16513", q16513),
    ("15225", q15225),
    ("18739", q18739),
    ("16141", q16141),
    ("16297", q16297),
    ("17296", q17296),
    ("17866", q17866),
    ("16502", q16502),
    ("19025", q19025),
    ("18959", q18959),
    ("15120", q15120),
    ("12732", q12732),
    ("15330", q15330),
    ("15128", q15128),
    ("15806", q15806),
    ("18746", q18746),
    ("12018", q12018),
    ("15152", q15152),
    ("16443", q16443),
    ("18312", q18312),
    ("17164", q17164),
    ("16325", q16325),
    ("11375", q11375),
    ("10698", q10698),
    ("10434", q10434),
    ("14454", q14454),
    ("13101", q13101),
    ("12932", q12932),
    ("12921", q12921),
    ("14412", q14412),
    ("10651", q10651),
    ("10247", q10247),
    ("13874", q13874),
    ("13465", q13465),
    ("11587", q11587),
    ("12650", q12650),
    ("14429", q14429),
    ("14477", q14477),
    ("12118", q12118),
    ("13007", q13007),
    ("13105", q13105),
    ("10149", q10149),
    ("10587", q10587),
    ("13635", q13635),
];
