use harness::prelude::*;
use std::cmp::Reverse;

fn cd(db: &'static So, p: Id<Post>) -> Reverse<i64> {
    Reverse(db.post.creation_date.get(p).unwrap())
}

fn questions(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.post_type_id).eq(1))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COALESCE(p.Score, 0) AS Score,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19669(db: &'static So) -> String {
    stat_rows(db, post_stats(db, questions(db), "c", &[]), |p, _| cd(db, p), 10, &["id", "title", "created", "owner", "score", "#cx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// COALESCE(p.Score, 0) AS PostScore,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16166(db: &'static So) -> String {
    stat_rows(db, post_stats(db, questions(db), "c", &[]), |p, _| cd(db, p), 10, &["id", "title", "owner", "created", "score", "#cx"])
}

// The newest questions with correlated counts of their children.
fn newest<R, const N: usize>(db: &'static So, counts: R, cols: &[&str]) -> String
where
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>, R = [i64; N]>,
{
    let mut v = Vec::new();
    questions(db).select(counts).drive(|p, n| v.push((cd(db, p), p, n)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(10).map(|(_, p, n)| {
        let mut f = post_fields(db, *p, cols);
        f.extend(n.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// SELECT
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15234(db: &'static So) -> String {
    newest(db, comments_per_post(db).and(votes_of_type(db, 2)).map(|(c, u)| [c, u]), &["title", "created", "owner"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Posts p2 WHERE p2.ParentId = p.Id) AS AnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15619(db: &'static So) -> String {
    newest(db, comments_per_post(db).and(answers_per_post(db)).map(|(c, a)| [c, a]), &["id", "title", "owner", "created", "views", "score"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT
// 10;
fn q19010(db: &'static So) -> String {
    newest(db, comments_per_post(db).and(votes_of_type(db, 2)).map(|(c, u)| [c, u]), &["id", "title", "created", "score", "owner"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16241(db: &'static So) -> String {
    newest(db, comments_per_post(db).and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).map(|((c, u), d)| [c, u, d]), &["id", "title", "created", "owner"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17974(db: &'static So) -> String {
    newest(db, comments_per_post(db).and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).map(|((c, u), d)| [c, u, d]), &["id", "title", "created", "score", "owner"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19106(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    stat_rows(db, post_stats(db, base, "cv", &[]), |p, _| cd(db, p), 10, &["id", "title", "created", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q15032(db: &'static So) -> String {
    let v = post_stats(db, questions(db), "cv", &[8]);
    rows(v.iter().map(|(p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "owner", "#cx"]);
        f.push(V::I(s.bounty_sum));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastEdit
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11004(db: &'static So) -> String {
    let votes = db.post.group_by(Ident::<Post>::new()).select(votes_of(db)).count_distinct();
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cvh", &[]).and((&votes).opt()).drive(|p, (s, x)| v.push((cd(db, p), p, s, x.unwrap_or(0))));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, ref s, x)| {
        let mut f = stat_fields(db, p, s, &["id", "title", "created", "views", "#cx"]);
        f.extend([V::I(x), stat_field(s, "hmax").unwrap()]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT ph.Id) AS PostHistoryCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score
// ORDER BY
// VoteCount DESC,
// CommentCount DESC,
// PostHistoryCount DESC
// LIMIT 100;
fn q11613(db: &'static So) -> String {
    let hist = db.post.group_by(Ident::<Post>::new()).select(history_of(db)).count_distinct();
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cvh", &[]).and((&hist).opt()).drive(|p, (s, h)| v.push((p, s, h.unwrap_or(0))));
    v.sort_by_key(|&(_, ref s, h)| (Reverse(s.vx), Reverse(s.cx), Reverse(h)));
    rows(v.iter().take(100).map(|&(p, ref s, h)| {
        let mut f = stat_fields(db, p, s, &["id", "title", "created", "score", "#vx", "#cx"]);
        f.push(V::I(h));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS AuthorDisplayName,
// COUNT(c.Id) AS CommentCount,
// AVG(EXTRACT(EPOCH FROM (c.CreationDate - p.CreationDate))) AS AvgCommentTime
// FROM
// Posts p
// INNER JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11035(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let mut v = Vec::new();
    owned(db)
        .with(creation_date.ge(date(2023, 1, 1)))
        .group_by(Ident::<Post>::new())
        .select(creation_date.and(comments_of(db).select(&db.comment.creation_date).opt()))
        .fold((0i64, 0.0f64), |(n, s), (pc, c)| match c {
            Some(c) => (n + 1, s + (c - pc) as f64 / 1e6),
            None => (n, s),
        })
        .drive(|p, a| v.push((cd(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, (n, s))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(n), if n == 0 { V::Null } else { V::F(s / n as f64) }]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// v.VoteTypeId,
// v.CreationDate AS VoteCreationDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// ORDER BY
// p.CreationDate DESC;
fn q13566(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.creation_date).ge(ts(2024, 9, 1, 12, 34, 56)))
        .select(votes_of(db).select((&db.vote.vote_type_id).and(&db.vote.creation_date)).opt())
        .drive(|p, x| v.push((p, x)));
    rows(v.iter().map(|&(p, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep"]);
        f.extend([oint(x.map(|x| x.0)), ots(x.map(|x| x.1))]);
        row(f)
    }))
}

// SELECT
// U.Reputation AS UserReputation,
// P.Score AS PostScore,
// P.ViewCount AS PostViewCount,
// P.CreationDate AS PostCreationDate,
// COUNT(C.Id) AS CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// U.Reputation, P.Score, P.ViewCount, P.CreationDate
// ORDER BY
// UserReputation DESC, PostScore DESC;
fn q13724(db: &'static So) -> String {
    let Post { score, view_count, creation_date, owner_user, .. } = &db.post;
    let base = owned(db).with(creation_date.gt(ts(2023, 10, 1, 12, 34, 56)));
    let key = owner_user.select(&db.user.reputation).and(score).and(view_count.opt()).and(creation_date);
    let v = group_stats(db, base, key, "c", &[]);
    rows(v.iter().map(|&((((r, s), w), c), ref st)| row(vec![V::I(r), V::I(s), oint(w), V::T(c), V::I(st.cx)])))
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS Owner,
// COUNT(a.Id) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Posts a ON a.ParentId = p.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17339(db: &'static So) -> String {
    tuple_rows(by_name_title_date(db, true, "av", PostWhere::All), "created", 10, &["title", "created", "owner", "#ax", "#up", "#down"])
}

fn names(db: &'static So, w: UserWhere, cols: &[&str]) -> String {
    let mut v = user_groups(db, &db.user.display_name, w);
    v.sort_by_key(|(_, a)| Reverse(a.n));
    rows(v.iter().take(10).map(|(k, a)| {
        let mut f = vec![V::S(k)];
        f.extend(cols.iter().map(|c| name_field(a, c)));
        row(f)
    }))
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 10 THEN 1 ELSE 0 END) AS TotalClosedPosts
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q16453(db: &'static So) -> String {
    names(db, UserWhere::All, &["#n", "#q", "#a", "#10"])
}

// SELECT
// Users.DisplayName,
// COUNT(Posts.Id) AS PostCount,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN Posts.ViewCount > 1000 THEN 1 ELSE 0 END) AS PopularPosts
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q16589(db: &'static So) -> String {
    names(db, UserWhere::All, &["#n", "#q", "#a", "#popular"])
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(p.Id) AS TotalPosts,
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
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q14293(db: &'static So) -> String {
    names(db, UserWhere::All, &["#n", "#q", "#a", "score_sum", "views_avg", "created_max"])
}

// Users grouped by their own row, LEFT JOIN Posts.
fn users(db: &'static So) -> Vec<(Id<User>, NameAgg)> {
    user_groups(db, Ident::<User>::new(), UserWhere::All)
}

fn user_row(db: &'static So, u: Id<User>, a: &NameAgg, ucols: &[&str], cols: &[&str]) -> String {
    let mut f: Vec<V> = ucols
        .iter()
        .map(|c| match *c {
            "uid" => V::I(db.user.origid.get(u).unwrap()),
            "name" => V::S(db.user.display_name.get(u).unwrap()),
            "rep" => V::I(db.user.reputation.get(u).unwrap()),
            _ => unreachable!(),
        })
        .collect();
    f.extend(cols.iter().map(|c| name_field(a, c)));
    row(f)
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikiCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q18701(db: &'static So) -> String {
    let mut v = users(db);
    v.sort_by_key(|(_, a)| Reverse(a.n));
    rows(v.iter().take(10).map(|(u, a)| user_row(db, *u, a, &["uid", "name"], &["#n", "#q", "#a", "#45"])))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
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
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalScore DESC, Reputation DESC;
fn q12640(db: &'static So) -> String {
    rows(users(db).iter().map(|(u, a)| user_row(db, *u, a, &["uid", "name", "rep"], &["#n", "#q", "#a", "views_sum", "score_sum"])))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// Reputation DESC, PostCount DESC
// LIMIT 100;
fn q10708(db: &'static So) -> String {
    let mut v = users(db);
    v.sort_by_key(|(u, a)| (Reverse(db.user.reputation.get(*u).unwrap()), Reverse(a.n)));
    rows(v.iter().take(100).map(|(u, a)| user_row(db, *u, a, &["uid", "name", "rep"], &["#n", "#q", "#a", "#3"])))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q12247(db: &'static So) -> String {
    rows(users(db).iter().map(|(u, a)| user_row(db, *u, a, &["uid", "name"], &["#n", "#q", "#a", "#pos", "views_sum", "score_avg"])))
}

// SELECT
// Users.DisplayName,
// COUNT(Posts.Id) AS PostCount,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// GROUP BY
// Users.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q19394(db: &'static So) -> String {
    let mut v = user_vote_groups(db, &db.user.display_name, UserWhere::All, &[]);
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(10).map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])])))
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q19620(db: &'static So) -> String {
    let mut v = user_vote_groups(db, &db.user.display_name, UserWhere::All, &[]);
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(10).map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])])))
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(v.BountyAmount) AS TotalBounty
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q19520(db: &'static So) -> String {
    let mut v = user_vote_groups(db, &db.user.display_name, UserWhere::RepGt(1000), &[]);
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(10).map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[8], a[7])])))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AveragePostScore,
// SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// SUM(CASE WHEN v.VoteTypeId = 5 THEN 1 ELSE 0 END) AS TotalFavorites
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q11143(db: &'static So) -> String {
    let v = user_vote_groups(db, Ident::<User>::new(), UserWhere::All, &[]);
    rows(v.iter().map(|&(u, a)| {
        row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(a[0]), avg(a[9], a[0]), V::I(a[5]), V::I(a[4]), V::I(a[6])])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalQuestions,
// COUNT(DISTINCT a.Id) AS TotalAcceptedAnswers,
// SUM(p.AnswerCount) AS TotalAnswersReceived
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1
// LEFT JOIN
// Posts a ON p.AcceptedAnswerId = a.Id
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalQuestions DESC
// LIMIT 10;
fn q13338(db: &'static So) -> String {
    let Post { post_type_id, answer_count, accepted_answer, owner_user, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1).and(owner_user.select(&db.user.reputation).gt(0)));
    let qs = (&base).group_by(owner_user).select(Ident::<Post>::new()).count_distinct();
    let acc = (&base).group_by(owner_user).select(accepted_answer).count_distinct();
    let answers = (&base).group_by(owner_user).select(answer_count.opt().and(accepted_answer.opt())).fold((0i64, 0i64), |(n, s), (a, _)| {
        (n + a.is_some() as i64, s + a.unwrap_or(0))
    });
    let mut v = Vec::new();
    qs.and((&acc).opt()).and(&answers).drive(|u, ((q, a), (n, s))| v.push((u, q, a.unwrap_or(0), n, s)));
    v.sort_by_key(|x| Reverse(x.1));
    rows(v.iter().take(10).map(|&(u, q, a, n, s)| {
        row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(db.user.reputation.get(u).unwrap()), V::I(q), V::I(a), nullable(s, n)])
    }))
}

// SELECT
// COUNT(P.Id) AS TotalPosts,
// SUM(P.ViewCount) AS TotalViewCount,
// SUM(P.Score) AS TotalScore,
// AVG(U.Reputation) AS AverageUserReputation,
// (SELECT
// B.Name
// FROM
// Badges B
// WHERE
// B.UserId = U.Id
// ORDER BY
// B.Date DESC, B.Id
// LIMIT 1) AS RecentBadge
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// GROUP BY
// U.Id
// ORDER BY
// TotalScore DESC
// LIMIT 10;
fn q13037(db: &'static So) -> String {
    let Post { view_count, score, owner_user, .. } = &db.post;
    let latest = db
        .badge
        .group_by(&db.badge.user)
        .select(Ident::<Badge>::new().and(&db.badge.date))
        .window(row_number, |(b, d)| (d, Reverse(db.badge.origid.get(b).unwrap())), desc)
        .filt(|(_, n)| n == 1)
        .map(|((b, _), _)| b);
    let mut v = Vec::new();
    owned(db)
        .group_by(owner_user)
        .select(view_count.opt().and(score).and(owner_user.select(&db.user.reputation)))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, vn, vs, s, r), ((w, x), rep)| (n + 1, vn + w.is_some() as i64, vs + w.unwrap_or(0), s + x, r + rep))
        .and(latest.opt())
        .drive(|u, a| v.push((u, a)));
    v.sort_by_key(|x| Reverse(x.1.0.3));
    rows(v.iter().take(10).map(|&(_, ((n, vn, vs, s, r), b))| {
        row(vec![V::I(n), nullable(vs, vn), V::I(s), avg(r, n), ostr(b.map(|b| db.badge.name.get(b).unwrap()))])
    }))
}

// Posts per type with a fold over whatever each post is joined to.
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

fn distinct_by_type<Q, R>(db: &'static So, base: Q, r: R) -> Fold<Str, i64>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>>,
    ROf<R>: Ord,
{
    base.group_by((&db.post.post_type).select(&db.post_type.name)).select(r).count_distinct()
}

fn listed<S: Copy>(f: Fold<Str, S>) -> Vec<(Str, S)> {
    let mut v = Vec::new();
    f.drive(|k, a| v.push((k, a)));
    v.sort_by_key(|(k, _)| *k);
    v
}

fn by_first<const N: usize>(mut v: Vec<(Str, [i64; N])>) -> Vec<(Str, [i64; N])> {
    v.sort_by_key(|x| Reverse(x.1[0]));
    v
}

fn opt0(x: Option<i64>) -> i64 {
    x.unwrap_or(0)
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViewCount,
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
// ORDER BY
// TotalPosts DESC;
fn q14087(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_type(db, db.post.iq(), score.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt()), [0i64; 6], |a, (((s, w), _), _)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), 0, 0]
    });
    let c = distinct_by_type(db, db.post.iq(), comments_of(db));
    let x = distinct_by_type(db, db.post.iq(), votes_of(db));
    let mut v = Vec::new();
    f.and((&c).opt()).and((&x).opt()).drive(|k, ((a, c), x)| v.push((k, [a[0], a[1], a[2], a[3], opt0(c), opt0(x)])));
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), V::I(a[5])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13225(db: &'static So) -> String {
    types_users_badges(db)
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViewCount,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q14039(db: &'static So) -> String {
    types_users_badges(db)
}

// Types LEFT JOIN Users LEFT JOIN Badges.
fn types_users_badges(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let f = by_type(db, db.post.iq(), score.and(view_count.opt()).and(owner_user.select(badges_of(db)).opt()), [0i64; 4], |a, ((s, w), _)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let u = distinct_by_type(db, db.post.iq(), owner_user);
    let b = distinct_by_type(db, db.post.iq(), owner_user.select(badges_of(db)));
    let mut v = Vec::new();
    f.and((&u).opt()).and((&b).opt()).drive(|k, ((a, u), b)| v.push((k, [a[0], a[1], a[2], a[3], opt0(u), opt0(b)])));
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
// COUNT(DISTINCT ph.PostHistoryTypeId) AS UniquePostHistoryTypes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12941(db: &'static So) -> String {
    let Post { score, view_count, owner_user_id, .. } = &db.post;
    let f = by_type(db, db.post.iq(), score.and(view_count.opt()).and(history_of(db).opt()), [0i64; 4], |a, ((s, w), _)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let u = distinct_by_type(db, db.post.iq(), owner_user_id);
    let h = distinct_by_type(db, db.post.iq(), history_of(db).select(&db.post_history.post_history_type_id));
    let mut v = Vec::new();
    f.and((&u).opt()).and((&h).opt()).drive(|k, ((a, u), h)| v.push((k, [a[0], a[1], a[2], a[3], opt0(u), opt0(h)])));
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), V::I(a[5])])))
}

// FROM PostTypes pt LEFT JOIN Posts p ...: every type, with or without posts.
fn of_type(db: &'static So) -> HashIdx<Id<PostType>, Id<Post>> {
    (&db.post.post_type).inv().collect()
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON pt.Id = p.PostTypeId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10860(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let of = of_type(db);
    let name = &db.post_type.name;
    let f = db.post_type.group_by(name).select((&of).select(score.and(view_count.opt()).and(votes_of(db).opt()).and(comments_of(db).opt())).opt()).fold(
        [0i64; 4],
        |a, r| match r {
            Some((((s, w), _), _)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
            None => a,
        },
    );
    let x = db.post_type.group_by(name).select((&of).select(votes_of(db))).count_distinct();
    let c = db.post_type.group_by(name).select((&of).select(comments_of(db))).count_distinct();
    let mut v = Vec::new();
    f.and((&x).opt()).and((&c).opt()).drive(|k, ((a, x), c)| v.push((k, [a[0], a[1], a[2], a[3], opt0(x), opt0(c)])));
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// GROUP BY
// pt.Id, pt.Name
// ORDER BY
// pt.Id;
fn q13544(db: &'static So) -> String {
    let of = of_type(db);
    let t = Ident::<PostType>::new();
    let p = db.post_type.group_by(&t).select(&of).count_distinct();
    let u = db.post_type.group_by(&t).select((&of).select(&db.post.owner_user)).count_distinct();
    let c = db.post_type.group_by(&t).select((&of).select(comments_of(db))).count_distinct();
    let x = db.post_type.group_by(&t).select((&of).select(votes_of(db))).count_distinct();
    let mut v = Vec::new();
    db.post_type.select((&p).opt().and((&u).opt()).and((&c).opt()).and((&x).opt())).drive(|t, (((p, u), c), x)| v.push((db.post_type.origid.get(t).unwrap(), t, [opt0(p), opt0(u), opt0(c), opt0(x)])));
    v.sort_by_key(|x| x.0);
    rows(v.iter().map(|&(_, t, a)| row(vec![V::S(db.post_type.name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// COUNT(c.Id) AS TotalComments
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13237(db: &'static So) -> String {
    let of = of_type(db);
    let f = db.post_type.group_by(&db.post_type.name).select((&of).select((&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt()).fold(
        [0i64; 5],
        |a, r| match r {
            Some(((s, t), c)) => [a[0] + 1, a[1] + s, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64, a[4] + c.is_some() as i64],
            None => a,
        },
    );
    rows(by_first(listed(f)).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// COUNT(DISTINCT u.Id) AS UserCount,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q13352(db: &'static So) -> String {
    let Post { score, view_count, owner_user, post_type, .. } = &db.post;
    let name = post_type.select(&db.post_type.name).opt();
    let f = db.post.group_by(&name).select(score.and(view_count.opt()).and(votes_of(db).opt()).and(comments_of(db).opt())).fold([0i64; 6], |a, (((s, w), x), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x.is_some() as i64, a[5] + c.is_some() as i64]
    });
    let u = db.post.group_by(&name).select(owner_user).count_distinct();
    let mut v = Vec::new();
    f.and((&u).opt()).drive(|k, (a, u)| v.push((k, a, opt0(u))));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a, u)| row(vec![ostr(k), V::I(a[0]), V::I(u), V::I(a[4]), V::I(a[5]), avg(a[1], a[0]), avg(a[3], a[2])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueAuthors,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// MAX(p.LastActivityDate) AS MostRecentActivity
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14458(db: &'static So) -> String {
    let Post { score, view_count, owner_user_id, accepted_answer_id, last_activity_date, .. } = &db.post;
    let f = by_type(db, db.post.iq(), score.and(view_count.opt()).and(accepted_answer_id.opt()).and(last_activity_date), [0, 0, 0, 0, 0, i64::MIN], |a: [i64; 6], (((s, w), ac), la)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + ac.is_some() as i64, a[5].max(la)]
    });
    let u = distinct_by_type(db, db.post.iq(), owner_user_id);
    let mut v = Vec::new();
    f.and((&u).opt()).drive(|k, (a, u)| v.push((k, a, opt0(u))));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a, u)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(u), V::I(a[4]), V::T(a[5])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViews
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// PostCount,
// COALESCE(AvgScore, 0) AS AvgScore,
// COALESCE(AvgViews, 0) AS AvgViews
// FROM
// PostStats
// ORDER BY
// PostCount DESC;
fn q14533(db: &'static So) -> String {
    coalesced_averages(db)
}

// WITH PostStatistics AS (
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
// )
// SELECT
// PostType,
// PostCount,
// COALESCE(AverageScore, 0) AS AverageScore,
// COALESCE(AverageViewCount, 0) AS AverageViewCount
// FROM
// PostStatistics
// ORDER BY
// PostCount DESC;
fn q10220(db: &'static So) -> String {
    coalesced_averages(db)
}

// COALESCE(AVG(..), 0): the Answer type has no ViewCount at all.
fn coalesced_averages(db: &'static So) -> String {
    let or0 = |s: i64, n: i64| V::F(if n == 0 { 0.0 } else { s as f64 / n as f64 });
    rows(by_count(db).iter().map(|a| row(vec![V::S(a.name), V::I(a.n), or0(a.score_sum, a.n), or0(a.views_sum, a.views_n)])))
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.PostTypeId
// )
// SELECT
// pt.Name AS PostType,
// ps.TotalPosts,
// ps.AvgScore
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// ORDER BY
// ps.TotalPosts DESC;
fn q12927(db: &'static So) -> String {
    let mut v = Vec::new();
    db.post
        .with((&db.post.creation_date).ge(ts(2023, 10, 1, 12, 34, 56)))
        .group_by(&db.post.post_type)
        .select(&db.post.score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x))
        .drive(|t, a| v.push((db.post_type.name.get(t).unwrap(), a)));
    v.sort_by_key(|x| Reverse(x.1.0));
    rows(v.iter().map(|&(k, (n, s))| row(vec![V::S(k), V::I(n), avg(s, n)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViewCount,
// AVG(EXTRACT(EPOCH FROM (COALESCE(p.LastActivityDate, '2024-10-01 12:34:56'::timestamp) - p.CreationDate))) AS AverageTimeToActivity,
// COUNT(DISTINCT u.Id) AS UniqueUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14136(db: &'static So) -> String {
    let Post { score, view_count, owner_user, creation_date, last_activity_date, .. } = &db.post;
    let f = owned(db).group_by((&db.post.post_type).select(&db.post_type.name)).select(score.and(view_count.opt()).and(last_activity_date).and(creation_date)).fold(
        (0i64, 0i64, 0i64, 0i64, 0i128),
        |(n, s, vn, vs, d), (((x, w), la), c)| (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0), d + (la - c) as i128),
    );
    let u = owned(db).group_by((&db.post.post_type).select(&db.post_type.name)).select(owner_user).count_distinct();
    let mut v = Vec::new();
    f.and(&u).drive(|k, (a, u)| v.push((k, a, u)));
    v.sort_by_key(|x| Reverse(x.1.0));
    rows(v.iter().map(|&(k, (n, s, vn, vs, d), u)| row(vec![V::S(k), V::I(n), avg(s, n), nullable(vs, vn), V::F(d as f64 / n as f64 / 1e6), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// u.Reputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name, u.Reputation
// ORDER BY
// PostCount DESC, u.Reputation DESC
// LIMIT 100;
fn q11178(db: &'static So) -> String {
    let Post { post_type, owner_user, .. } = &db.post;
    let key = post_type.select(&db.post_type.name).and(owner_user.select(&db.user.reputation).opt());
    let mut v = Vec::new();
    db.post
        .group_by(key)
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold((0i64, 0i64, 0i64), |(n, u, d), t| (n + 1, u + (t == Some(2)) as i64, d + (t == Some(3)) as i64))
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|&((_, r), (n, _, _))| (Reverse(n), r.is_none(), Reverse(r)));
    rows(v.iter().take(100).map(|&((k, r), (n, u, d))| row(vec![V::S(k), V::I(n), V::I(u), V::I(d), oint(r)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(COALESCE(p.Score, 0)) AS AverageScore,
// SUM(COALESCE(v.UserId, 0)) AS TotalVotes,
// SUM(COALESCE(c.Id, 0)) AS TotalComments,
// AVG(u.Reputation) AS AverageUserReputation
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
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11550(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let f = by_type(
        db,
        db.post.iq(),
        score.and(votes_of(db).select((&db.vote.user_id).opt()).opt()).and(comments_of(db).select(&db.comment.origid).opt()).and(owner_user.select(&db.user.reputation).opt()),
        [0i64; 6],
        |a, (((s, vu), c), r)| {
            [a[0] + 1, a[1] + s, a[2] + vu.flatten().unwrap_or(0), a[3] + c.unwrap_or(0), a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)]
        },
    );
    rows(by_first(listed(f)).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4])])))
}

// SELECT
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// AVG(u.Reputation) AS AverageReputation,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
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
// GROUP BY
// u.Reputation
// ORDER BY
// TotalPosts DESC;
fn q11370(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2023, 1, 1)));
    let key = owner_user.select(&db.user.reputation).opt();
    let f = (&base).group_by(&key).select(owner_user.select((&db.user.reputation).and(&db.user.up_votes).and(&db.user.down_votes)).opt().and(comments_of(db).opt()).and(votes_of(db).opt())).fold(
        [0i64; 4],
        |a, ((u, _), _)| match u {
            Some(((r, up), dn)) => [a[0] + 1, a[1] + r, a[2] + up, a[3] + dn],
            None => a,
        },
    );
    let p = (&base).group_by(&key).select(Ident::<Post>::new()).count_distinct();
    let c = (&base).group_by(&key).select(comments_of(db)).count_distinct();
    let x = (&base).group_by(&key).select(votes_of(db)).count_distinct();
    let mut v = Vec::new();
    f.and(&p).and((&c).opt()).and((&x).opt()).drive(|_, (((a, p), c), x)| v.push((p, opt0(c), opt0(x), a)));
    rows(v.iter().map(|&(p, c, x, a)| row(vec![V::I(p), V::I(c), V::I(x), avg(a[1], a[0]), nullable(a[2], a[0]), nullable(a[3], a[0])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// AVG(p.Score) AS AverageScore,
// AVG(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate)) / 60) AS AverageResponseTimeInMinutes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// pt.Name
// ORDER BY
// CommentCount DESC, AverageScore DESC
// LIMIT 10;
fn q10995(db: &'static So) -> String {
    let Post { score, creation_date, .. } = &db.post;
    let now = ts(2024, 10, 1, 12, 34, 56);
    let base = db.post.with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)));
    let f = by_type(db, base, score.and(creation_date).and(comments_of(db).opt()), (0i64, 0i64, 0i64, 0i128), |(n, c, s, d), ((x, pc), ci)| {
        (n + 1, c + ci.is_some() as i64, s + x, d + (now - pc) as i128)
    });
    let mut v = Vec::new();
    f.drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.1.1.cmp(&a.1.1).then((b.1.2 as f64 / b.1.0 as f64).total_cmp(&(a.1.2 as f64 / a.1.0 as f64))));
    rows(v.iter().take(10).map(|&(k, (n, c, s, d))| row(vec![V::S(k), V::I(c), avg(s, n), V::F(d as f64 / n as f64 / 1e6 / 60.0)])))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// pt.Name AS PostType
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// u.Id, u.DisplayName, pt.Name
// ORDER BY
// u.Id;
fn q14046(db: &'static So) -> String {
    let j: MatSet<(Id<User>, Option<Id<Post>>)> = db.user.select(Ident::<User>::new().and(posts_of(db).opt())).collect();
    let post_of = (&j).flat_map(|(_, p)| p);
    let key = (&j).map(|(u, _)| u).and((&post_of).select((&db.post.post_type).select(&db.post_type.name)).opt());
    let p = (&j).group_by(&key).select(&post_of).count_distinct();
    let c = (&j).group_by(&key).select((&post_of).select(comments_of(db))).count_distinct();
    let x = (&j).group_by(&key).select((&post_of).select(votes_of(db))).count_distinct();
    let groups: MatSet<(Id<User>, Option<Str>)> = (&j).select(&key).collect();
    let mut v = Vec::new();
    (&groups).select((&p).opt().and((&c).opt()).and((&x).opt())).drive(|(u, t), ((p, c), x)| v.push((u, t, opt0(p), opt0(c), opt0(x))));
    rows(v.iter().map(|&(u, t, p, c, x)| row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(p), V::I(c), V::I(x), ostr(t)])))
}

// SELECT
// COUNT(DISTINCT p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgPostScore,
// AVG(p.ViewCount) AS AvgViewCount,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// COUNT(DISTINCT u.Id) AS TotalUsers
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON b.UserId = p.OwnerUserId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2021-01-01'
// AND p.PostTypeId = 1
fn q11578(db: &'static So) -> String {
    let Post { score, view_count, owner_user, creation_date, post_type_id, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2021, 1, 1)).and(post_type_id.eq(1)));
    let (n, s, vn, vs) = (&base).select(score.and(view_count.opt()).and(votes_of(db).opt()).and(owner_user.select(badges_of(db)).opt())).fold_flat(
        (0i64, 0i64, 0i64, 0i64),
        |(n, s, vn, vs), (((x, w), _), _)| (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0)),
    );
    let one = |f: Fold<(), i64>| (&f).fold_flat(0i64, |a, x| a + x);
    let p = one(whole(&base).select(Ident::<Post>::new()).count_distinct());
    let x = one(whole(&base).select(votes_of(db)).count_distinct());
    let b = one(whole(&base).select(owner_user.select(badges_of(db))).count_distinct());
    let u = one(whole(&base).select(owner_user).count_distinct());
    row(vec![V::I(p), avg(s, n), avg(vs, vn), V::I(x), V::I(b), V::I(u)])
}

fn count<Q: Drive>(q: Q) -> i64 {
    q.fold_flat(0i64, |a, _| a + 1)
}

fn mean<Q: Drive<R = i64>>(q: Q) -> V {
    let (n, s) = q.fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    avg(s, n)
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// AVG(Score) AS AveragePostScore,
// AVG(ViewCount) AS AveragePostViewCount,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM PostHistory) AS TotalPostHistoryEntries
// FROM
// Posts;
fn q14569(db: &'static So) -> String {
    row(vec![
        V::I(count(db.post.iq())),
        V::I(count(db.user.iq())),
        V::I(count(db.vote.iq())),
        mean(&db.post.score),
        mean(db.post.select(&db.post.view_count)),
        V::I(count(db.badge.iq())),
        V::I(count(db.comment.iq())),
        V::I(count(db.post_history.iq())),
    ])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS Total_Posts,
// (SELECT COUNT(*) FROM Users) AS Total_Users,
// (SELECT COUNT(*) FROM Comments) AS Total_Comments,
// (SELECT COUNT(*) FROM Votes) AS Total_Votes,
// (SELECT COUNT(*) FROM Badges) AS Total_Badges,
// (SELECT COUNT(*) FROM Tags) AS Total_Tags,
// (SELECT COUNT(*) FROM PostHistory) AS Total_PostHistories,
// (SELECT COUNT(*) FROM PostLinks) AS Total_PostLinks
fn q14938(db: &'static So) -> String {
    row(vec![
        V::I(count(db.post.iq())),
        V::I(count(db.user.iq())),
        V::I(count(db.comment.iq())),
        V::I(count(db.vote.iq())),
        V::I(count(db.badge.iq())),
        V::I(count(db.tag.iq())),
        V::I(count(db.post_history.iq())),
        V::I(count(db.post_link.iq())),
    ])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT AVG(Score) FROM Posts) AS AvgPostScore,
// (SELECT AVG(Score) FROM Comments) AS AvgCommentScore,
// (SELECT SUM(UpVotes) FROM Users) AS TotalUpVotes,
// (SELECT SUM(DownVotes) FROM Users) AS TotalDownVotes,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges
// ;
fn q12915(db: &'static So) -> String {
    row(vec![
        V::I(count(db.post.iq())),
        V::I(count(db.comment.iq())),
        V::I(count(db.user.iq())),
        mean(&db.post.score),
        mean(&db.comment.score),
        nullable((&db.user.up_votes).fold_flat(0i64, |a, x| a + x), count(db.user.iq())),
        nullable((&db.user.down_votes).fold_flat(0i64, |a, x| a + x), count(db.user.iq())),
        V::I(count(db.badge.iq())),
    ])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("19669", q19669),
    ("16166", q16166),
    ("15234", q15234),
    ("15619", q15619),
    ("19010", q19010),
    ("16241", q16241),
    ("17974", q17974),
    ("19106", q19106),
    ("15032", q15032),
    ("11004", q11004),
    ("11613", q11613),
    ("11035", q11035),
    ("13566", q13566),
    ("13724", q13724),
    ("17339", q17339),
    ("16453", q16453),
    ("16589", q16589),
    ("14293", q14293),
    ("18701", q18701),
    ("12640", q12640),
    ("10708", q10708),
    ("12247", q12247),
    ("19394", q19394),
    ("19620", q19620),
    ("19520", q19520),
    ("11143", q11143),
    ("13338", q13338),
    ("13037", q13037),
    ("14087", q14087),
    ("13225", q13225),
    ("14039", q14039),
    ("12941", q12941),
    ("10860", q10860),
    ("13544", q13544),
    ("13237", q13237),
    ("13352", q13352),
    ("14458", q14458),
    ("14533", q14533),
    ("10220", q10220),
    ("12927", q12927),
    ("14136", q14136),
    ("11178", q11178),
    ("11550", q11550),
    ("11370", q11370),
    ("10995", q10995),
    ("14046", q14046),
    ("11578", q11578),
    ("14569", q14569),
    ("14938", q14938),
    ("12915", q12915),
];
