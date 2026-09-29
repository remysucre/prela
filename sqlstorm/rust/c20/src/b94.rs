use harness::prelude::*;

// Five more one-row-per-post queries and one that groups by a value tuple.
// All six count Votes by type, so `#up`/`#down` are SUM(CASE WHEN
// v.VoteTypeId = 2 / 3 THEN 1 ELSE 0 END) with the other children's fan-out
// folded in.

fn posts(db: &'static So, outer: bool, joins: &str) -> Vec<(Id<Post>, Agg)> {
    posts_with_counts(db, false, outer, joins)
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName, u.Reputation ORDER BY p.CreationDate DESC
fn q12190(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, false, "v", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 0,
        &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep", "#vx", "#up", "#down"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate AS PostCreationDate, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, p.Score
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2020-01-01' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, u.Reputation, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, p.Score ORDER BY p.CreationDate DESC
fn q12283(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, false, "v", PostWhere::CreatedGe(date(2020, 1, 1))), "created", 0,
        &["id", "title", "created", "owner", "rep", "#vx", "#up", "#down", "views", "answers", "comments", "favorites", "score"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate AS PostCreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Id AS UserId, u.DisplayName AS UserDisplayName, u.Reputation AS UserReputation, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2022-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Id, u.DisplayName, u.Reputation ORDER BY p.CreationDate DESC
fn q14083(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, false, "v", PostWhere::CreatedGe(date(2022, 1, 1))), "created", 0,
        &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner_id", "owner", "rep", "#vx", "#up", "#down"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, p.AnswerCount, p.FavoriteCount, pt.Name AS PostTypeName
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, p.AnswerCount, p.FavoriteCount, pt.Name ORDER BY p.CreationDate DESC LIMIT 100
fn q11949(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100,
        &["id", "title", "created", "views", "score", "owner", "#cx", "#up", "#down", "answers", "favorites", "type"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, p.AnswerCount, p.FavoriteCount
// FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.AnswerCount, p.FavoriteCount ORDER BY p.Score DESC, p.CreationDate DESC LIMIT 100
fn q10839(db: &'static So) -> String {
    render_posts(db, posts(db, true, "cv"), "score,created", 100,
        &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "#up", "#down", "answers", "favorites"])
}

// The only one here whose GROUP BY leaves out p.Id: two posts that agree on
// owner, title, date, score, views, answers, favorites and last activity are
// one row, and their comments and votes are pooled.
//
// SELECT u.DisplayName AS UserDisplayName, p.Title AS PostTitle, p.CreationDate AS PostCreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, p.Score AS PostScore, p.ViewCount AS PostViewCount, p.AnswerCount AS TotalAnswers, p.FavoriteCount AS TotalFavorites, p.LastActivityDate AS LastActivityDate
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2023-01-01' GROUP BY u.DisplayName, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.FavoriteCount, p.LastActivityDate ORDER BY PostScore DESC, PostCreationDate DESC LIMIT 100
fn q13408(db: &'static So) -> String {
    let Post { answer_count, creation_date, favorite_count, last_activity_date, owner_user, score, title, view_count, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(score)
        .and(view_count.opt())
        .and(answer_count.opt())
        .and(favorite_count.opt())
        .and(last_activity_date);
    let mut v = Vec::new();
    db.post
        .with(creation_date.ge(date(2023, 1, 1)))
        .group_by(key)
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64, 0i64), |(c, u2, d2), (ci, vt)| {
            (c + ci.is_some() as i64, u2 + (vt == Some(2)) as i64, d2 + (vt == Some(3)) as i64)
        })
        .drive(|(((((((dn, t), cd), s), w), ac), fc), la), (c, u2, d2)| {
            v.push((dn, t, cd, s, w, ac, fc, la, c, u2, d2))
        });
    v.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| b.2.cmp(&a.2)));
    rows(v.iter().take(100).map(|&(dn, t, cd, s, w, ac, fc, la, c, u2, d2)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), V::I(c), V::I(u2), V::I(d2), V::I(s), oint(w), oint(ac), oint(fc), V::T(la)])
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10839", q10839),
    ("11949", q11949),
    ("12190", q12190),
    ("12283", q12283),
    ("13408", q13408),
    ("14083", q14083),
];
