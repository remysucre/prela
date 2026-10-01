use harness::prelude::*;

// Twelve more of the one-row-per-post shape. Every GROUP BY here names p.Id,
// so the group is a single post and the only thing the children contribute is
// their counts:
//
//   SELECT <post and owner columns>, <counts over c | v | b | ph>
//   FROM Posts p [LEFT] JOIN Users u ON p.OwnerUserId = u.Id
//        [LEFT JOIN Comments c ON p.Id = c.PostId]
//        [LEFT JOIN Votes v ON p.Id = v.PostId]
//        [LEFT JOIN Badges b ON u.Id = b.UserId]
//        [LEFT JOIN PostHistory ph ON p.Id = ph.PostId]
//   [WHERE p.CreationDate >= ...] GROUP BY <them> ORDER BY ... [LIMIT n]
//
// `#cx` is COUNT(c.Id), which the other children multiply; `#c` is
// COUNT(DISTINCT c.Id), which they do not. The join set passed to `posts`
// says which multipliers are in play.

fn posts(db: &'static So, only_q: bool, outer: bool, joins: &str) -> Vec<(Id<Post>, Agg)> {
    posts_with_counts(db, only_q, outer, joins)
}

// SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, COUNT(Cm.Id) AS CommentCount
// FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments Cm ON P.Id = Cm.PostId GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName ORDER BY P.CreationDate DESC LIMIT 10
fn q18895(db: &'static So) -> String {
    render_posts(db, posts(db, false, false, "c"), "created", 10, &["id", "title", "created", "owner", "#cx"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Reputation AS OwnerReputation, COUNT(v.Id) AS TotalVotes
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Reputation ORDER BY p.CreationDate DESC LIMIT 100
fn q11136(db: &'static So) -> String {
    render_posts(db, posts(db, false, false, "v"), "created", 100,
        &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "rep", "#vx"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, p.AnswerCount, p.FavoriteCount
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, p.AnswerCount, p.FavoriteCount ORDER BY p.ViewCount DESC
fn q10287(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "views", 0,
        &["id", "title", "created", "views", "score", "owner", "#cx", "#vx", "answers", "favorites"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COUNT(v.Id) AS VoteCount
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName, u.Reputation ORDER BY p.CreationDate DESC LIMIT 100
fn q12814(db: &'static So) -> String {
    render_posts(db, posts(db, true, false, "v"), "created", 100,
        &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "owner", "rep", "#vx"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COUNT(v.Id) AS TotalVotes, COUNT(c.Id) AS TotalComments
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName, u.Reputation ORDER BY p.CreationDate DESC LIMIT 100
fn q14356(db: &'static So) -> String {
    render_posts(db, posts(db, false, false, "cv"), "created", 100,
        &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep", "#vx", "#cx"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate AS PostCreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Id AS UserId, u.DisplayName AS OwnerDisplayName, u.Reputation, COUNT(c.Id) AS TotalComments, COUNT(v.Id) AS TotalVotes
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Id, u.DisplayName, u.Reputation ORDER BY p.CreationDate DESC LIMIT 100
fn q14844(db: &'static So) -> String {
    render_posts(db, posts(db, false, false, "cv"), "created", 100,
        &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner_id", "owner", "rep", "#cx", "#vx"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, MAX(ph.CreationDate) AS LastEditDate
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName ORDER BY p.CreationDate DESC LIMIT 100
fn q11586(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, false, "cvh", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100,
        &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "owner", "#cx", "#vx", "#hmax"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COUNT(cm.Id) AS CommentCountPerPost, COUNT(v.Id) AS VoteCountPerPost, COUNT(b.Id) AS BadgeCountPerUser
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments cm ON p.Id = cm.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE p.CreationDate >= DATE '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName, u.Reputation ORDER BY p.CreationDate DESC LIMIT 100
fn q13617(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100,
        &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep", "#cx", "#vx", "#bx"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(DISTINCT v.Id) AS TotalVotes, COUNT(DISTINCT c.Id) AS TotalComments, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, p.Score, p.ViewCount, p.Tags, p.AnswerCount, p.FavoriteCount
// FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, u.Reputation, p.Score, p.ViewCount, p.Tags, p.AnswerCount, p.FavoriteCount ORDER BY p.CreationDate DESC LIMIT 100
fn q13996(db: &'static So) -> String {
    render_posts(db, posts(db, false, true, "cv"), "created", 100,
        &["id", "title", "created", "#v", "#c", "owner", "rep", "score", "views", "tags", "answers", "favorites"])
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS AuthorDisplayName, COUNT(c.Id) AS CommentCount, p.AnswerCount, p.FavoriteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.AnswerCount, p.FavoriteCount ORDER BY p.CreationDate DESC LIMIT 100
fn q13745(db: &'static So) -> String {
    render_posts(db, posts(db, false, false, "cv"), "created", 100,
        &["id", "title", "created", "score", "views", "owner", "#cx", "answers", "favorites", "#up", "#down"])
}

// SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, p.CreationDate, u.Id AS UserId, u.DisplayName AS Author, u.Reputation, COUNT(DISTINCT v.Id) AS VoteCount, COUNT(DISTINCT c.Id) AS CommentCount
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= '2020-01-01' GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, p.CreationDate, u.Id, u.DisplayName, u.Reputation ORDER BY p.CreationDate DESC
fn q10764(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2020, 1, 1))), "created", 0,
        &["id", "title", "views", "score", "answers", "comments", "favorites", "created", "owner_id", "owner", "rep", "#v", "#c"])
}

// SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, p.CreationDate AS PostCreationDate, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, u.DisplayName, u.Reputation, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount ORDER BY p.CreationDate DESC LIMIT 100
fn q10444(db: &'static So) -> String {
    render_posts(db, posts(db, false, false, "v"), "created", 100,
        &["id", "title", "owner", "rep", "created", "#vx", "#up", "#down", "views", "answers", "comments", "favorites"])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10287", q10287),
    ("10444", q10444),
    ("10764", q10764),
    ("11136", q11136),
    ("11586", q11586),
    ("12814", q12814),
    ("13617", q13617),
    ("13745", q13745),
    ("13996", q13996),
    ("14356", q14356),
    ("14844", q14844),
    ("18895", q18895),
];
