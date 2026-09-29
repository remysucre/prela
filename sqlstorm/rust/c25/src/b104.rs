use harness::prelude::*;
use std::cmp::Reverse;

fn questions(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.post_type_id).eq(1))
}

fn owned_since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.creation_date).ge(d))
}

fn score_then_newest(db: &'static So, p: Id<Post>) -> (Reverse<i64>, Reverse<i64>) {
    (Reverse(db.post.score.get(p).unwrap()), newest(db, p))
}

fn views_desc(db: &'static So, p: Id<Post>) -> (bool, Reverse<Option<i64>>) {
    let w = db.post.view_count.get(p);
    (w.is_none(), Reverse(w))
}

// GROUP BY a post and columns of one joined history row: the (post, history)
// rows are materialised, grouped by `hkey` of the history side, and each
// group folds its posts' comments x votes: [COUNT(c), COUNT(v), up, down, rows].
fn post_history_groups<Q, H, K>(db: &'static So, base: Q, hist: H, hkey: K) -> Vec<((Id<Post>, Option<ROf<K>>), [i64; 5])>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    H: IntoQuery,
    H::Q: Probe<D = Id<Post>, R = Id<PostHistory>>,
    K: IntoQuery,
    K::Q: Probe<D = Id<PostHistory>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = base.select(Ident::<Post>::new().and(hist.opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&hist_of).select(hkey).opt()))
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 5], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64, a[4] + 1])
        .drive(|k, a| v.push((k, a)));
    v
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
// u.Id, u.DisplayName, u.Reputation, u.CreationDate
// ORDER BY
// p.Score DESC, p.CreationDate DESC
// LIMIT 100;
fn q11330(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "cv", &[], &[]), |p, _| score_then_newest(db, p), 100, &["id", "title", "created", "score", "views", "answers", "comments", "uid", "owner", "rep", "ucreated", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// AVG(b.Class) AS AverageBadgeClass
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
// u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14510(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cvb", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "answers", "owner", "rep", "#cx", "#up", "#down", "bclass_avg"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation AS UserReputation,
// u.CreationDate AS UserCreationDate,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes,
// AVG(COALESCE(v.BountyAmount, 0)) AS AverageBountyAmount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.Id,
// u.DisplayName,
// u.Reputation,
// u.CreationDate
// ORDER BY
// p.ViewCount DESC, p.Score DESC
// LIMIT 100;
fn q14261(db: &'static So) -> String {
    stats_rows(db, stats_with(db, db.post.iq(), "cv", &[], &[]), |p, _| (views_desc(db, p), Reverse(db.post.score.get(p).unwrap())), 100, &["id", "title", "created", "views", "score", "answers", "comments", "uid", "owner", "rep", "ucreated", "#cx", "#vx", "bounty0_avg"])
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
// AVG(CASE WHEN bh.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseRate,
// AVG(CASE WHEN bh.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenRate
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory bh ON p.Id = bh.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q11105(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    stats_rows(db, stats_with(db, base, "cvh", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down", "h10_frac", "h11_frac"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// COUNT(c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// AVG(b.Class) AS AverageBadgeClass
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
// p.CreationDate >= '2021-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
// p.CommentCount, p.FavoriteCount, u.Id, u.DisplayName, u.Reputation,
// u.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q12486(db: &'static So) -> String {
    let votes = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, owned_since(db, date(2021, 1, 1)), "cvb", &[], &[&votes]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "uid", "owner", "rep", "ucreated", "#cx", "#d0", "bclass_avg"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AverageUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AverageDownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, u.Id, u.DisplayName, u.Reputation, u.CreationDate
// ORDER BY
// p.ViewCount DESC;
fn q10169(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cv", &[], &[]), |p, _| views_desc(db, p), 0, &["id", "title", "created", "views", "score", "answers", "uid", "owner", "rep", "ucreated", "#cx", "#vx", "up_frac", "down_frac"])
}

// SELECT
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// p.ViewCount AS PostViews,
// p.Score AS PostScore,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// AVG(CASE WHEN ph.PostHistoryTypeId IN (4, 5) THEN 1 ELSE NULL END) AS AverageEdits
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
// p.PostTypeId = 1
// GROUP BY
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName,
// p.Id
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11269(db: &'static So) -> String {
    stats_rows(db, stats_with(db, questions(db), "cvh", &[], &[]), |p, _| newest(db, p), 100, &["title", "created", "views", "score", "owner", "#cx", "#up", "#down", "h45_one"])
}

// SELECT
// p.Id AS PostID,
// p.Title AS PostTitle,
// pt.Name AS PostType,
// u.Id AS UserID,
// u.DisplayName AS UserName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.ViewCount AS ViewCount,
// p.Score AS Score,
// p.CreationDate AS PostCreationDate
// FROM
// Posts p
// INNER JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// INNER JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, pt.Name, u.Id, u.DisplayName, p.ViewCount, p.Score, p.CreationDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11070(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "type", "uid", "owner", "#cx", "#vx", "#up", "#down", "views", "score", "created"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.CreationDate,
// p.LastActivityDate,
// p.ViewCount,
// pp.Name AS PostTypeName,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostTypes pp ON p.PostTypeId = pp.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, u.DisplayName, pp.Name, p.CreationDate, p.LastActivityDate, p.ViewCount
// ORDER BY
// p.LastActivityDate DESC;
fn q11794(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cvh", &[], &[]), |_, _| 0, 0, &["id", "title", "owner", "#cx", "#up", "#down", "created", "activity", "views", "type", "hmax"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.Score,
// P.ViewCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
// T.TagName AS PostTag,
// PT.Name AS PostTypeName
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Tags T ON T.ExcerptPostId = P.Id
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName, U.Reputation, T.TagName, PT.Name
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q14061(db: &'static So) -> String {
    let tag: HashIdx<Id<Post>, Str> = (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).collect();
    let mut v = group_stats(db, owned(db), Ident::<Post>::new().and((&tag).opt()), "cv", &[]);
    v.sort_by_key(|&((p, _), _)| newest(db, p));
    rows(v.iter().take(100).map(|((p, tn), s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views", "owner", "rep", "#cx", "#up", "#down"]);
        f.push(ostr(*tn));
        f.extend(post_fields(db, *p, &["type"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation AS UserReputation,
// u.CreationDate AS UserCreationDate,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// COUNT(c.Id) AS CommentsCount,
// COUNT(b.Id) AS BadgesCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
// u.Id, u.DisplayName, u.Reputation, u.CreationDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13441(db: &'static So) -> String {
    stats_rows(db, stats_with(db, questions(db), "vcb", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "answers", "comments", "uid", "owner", "rep", "ucreated", "#up", "#cx", "#bx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.Score,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// u.LastAccessDate,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount,
// p.CommentCount, p.Score, u.Id, u.DisplayName, u.Reputation,
// u.CreationDate, u.LastAccessDate
// ORDER BY
// p.ViewCount DESC, p.CreationDate DESC
// LIMIT 100;
fn q14599(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cv", &[], &[]), |p, _| (views_desc(db, p), newest(db, p)), 100, &["id", "title", "created", "views", "answers", "comments", "score", "uid", "owner", "rep", "ucreated", "last_access", "#cx", "#vx", "#upj", "#downj"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS AuthorDisplayName,
// u.Reputation AS AuthorReputation,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(c.Id) AS CommentCount,
// AVG(uh.Reputation) AS AverageUserReputation
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users uh ON c.UserId = uh.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14588(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let mut v = Vec::new();
    base.group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).select((&db.comment.user).select(&db.user.reputation).opt()).opt()))
        .fold([0i64; 6], |a, (x, c)| {
            let r = c.flatten();
            [a[0] + x.is_some() as i64, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64, a[3] + c.is_some() as i64, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)]
        })
        .drive(|p, a| v.push((newest(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4])]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName,
// p.LastActivityDate,
// p.AcceptedAnswerId,
// ph.CreationDate AS PostHistoryDate,
// p.Body
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.Reputation, u.DisplayName,
// p.LastActivityDate, p.AcceptedAnswerId, ph.CreationDate, p.Body
// ORDER BY
// p.Score DESC;
fn q10936(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(ts(2023, 10, 1, 12, 34, 56)));
    let v = post_history_groups(db, base, history_of(db), &db.post_history.creation_date);
    rows(v.iter().map(|&((p, hd), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["rep", "owner", "activity", "accepted"]));
        f.push(ots(hd));
        f.extend(post_fields(db, p, &["body"]));
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
// ph.UserDisplayName AS LastEditorDisplayName,
// ph.CreationDate AS LastEditDate
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 4
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, ph.UserDisplayName, ph.CreationDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14003(db: &'static So) -> String {
    let PostHistory { post_history_type_id, user_display_name, creation_date, .. } = &db.post_history;
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let edits = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(4)));
    let mut v = post_history_groups(db, base, edits, user_display_name.opt().and(creation_date));
    v.sort_by_key(|&((p, h), _)| (newest(db, p), db.post.origid.get(p).unwrap(), h.is_none(), h.map(|h| (h.0.is_none(), h.0, h.1))));
    rows(v.iter().take(100).map(|&((p, h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[3]), ostr(h.and_then(|h| h.0)), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName AS UserName,
// U.Reputation,
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.ViewCount,
// P.Score AS PostScore,
// PH.CreationDate AS PostHistoryDate,
// PHT.Name AS PostHistoryType,
// COUNT(COALESCE(CM.Id, 0)) AS CommentCount,
// COUNT(COALESCE(V.Id, 0)) AS VoteCount
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// LEFT JOIN
// Comments CM ON P.Id = CM.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, PH.CreationDate, PHT.Name
// ORDER BY
// U.Reputation DESC, P.CreationDate DESC;
fn q10612(db: &'static So) -> String {
    let PostHistory { creation_date, post_history_type, .. } = &db.post_history;
    let v = post_history_groups(db, questions(db), history_of(db), creation_date.and(post_history_type.select(&db.post_history_type.name).opt()));
    rows(v.iter().map(|&((p, h), a)| {
        let mut f = post_fields(db, p, &["uid", "owner", "rep", "id", "title", "created", "views", "score"]);
        f.extend([ots(h.map(|h| h.0)), ostr(h.and_then(|h| h.1)), V::I(a[4]), V::I(a[4])]);
        row(f)
    }))
}

// SELECT
// P.Id AS PostID,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// U.Id AS UserID,
// U.DisplayName AS UserDisplayName,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// PT.Name AS PostType,
// PH.RevisionGUID
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount,
// U.Id, U.DisplayName, PT.Name, PH.RevisionGUID
// ORDER BY
// P.CreationDate DESC;
fn q13813(db: &'static So) -> String {
    let v = post_history_groups(db, owned(db), history_of(db), &db.post_history.revision_guid);
    rows(v.iter().map(|&((p, g), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "uid", "owner"]);
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["type"]));
        f.push(ostr(g));
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// U.Reputation AS OwnerReputation,
// COUNT(C.ID) AS CommentCount,
// COUNT(V.Id) AS VoteCount,
// PT.Name AS PostTypeName,
// P.LastActivityDate,
// PH.CreationDate AS LastEditDate,
// PH.Comment AS LastEditComment
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// WHERE
// P.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.Reputation,
// PT.Name, P.LastActivityDate, PH.CreationDate, PH.Comment
// ORDER BY
// P.LastActivityDate DESC
// LIMIT 100;
fn q12783(db: &'static So) -> String {
    let PostHistory { creation_date, comment, .. } = &db.post_history;
    let mut v = post_history_groups(db, owned_since(db, ts(2024, 9, 1, 12, 34, 56)), history_of(db), creation_date.and(comment.opt()));
    v.sort_by_key(|&((p, _), _)| Reverse(db.post.last_activity_date.get(p).unwrap()));
    rows(v.iter().take(100).map(|&((p, h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["type", "activity"]));
        f.extend([ots(h.map(|h| h.0)), ostr(h.and_then(|h| h.1))]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// ph.PostHistoryTypeId,
// ph.CreationDate AS HistoryCreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2020-01-01'
// GROUP BY
// p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, ph.PostHistoryTypeId, ph.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q12589(db: &'static So) -> String {
    let PostHistory { creation_date, post_history_type_id, .. } = &db.post_history;
    let v = post_history_groups(db, owned_since(db, date(2020, 1, 1)), history_of(db), post_history_type_id.and(creation_date));
    rows(v.iter().map(|&((p, h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views", "answers", "comments", "favorites"]);
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1)), V::I(a[0]), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1.0 ELSE 0.0 END) AS AverageUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1.0 ELSE 0.0 END) AS AverageDownVotes,
// ph.UserDisplayName AS LastEditedBy,
// ph.CreationDate AS LastEditDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
// u.DisplayName, u.Reputation, ph.UserDisplayName, ph.CreationDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12264(db: &'static So) -> String {
    let PostHistory { creation_date, user_display_name, .. } = &db.post_history;
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = questions(db).select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&hist_of).select(user_display_name.opt().and(creation_date)).opt()))
        .select((&post_of).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, x| [a[0] + x.is_some() as i64, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64, a[3] + 1])
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|&((p, h), _)| (newest(db, p), db.post.origid.get(p).unwrap(), h.is_none(), h.map(|h| (h.0.is_none(), h.0, h.1))));
    rows(v.iter().take(100).map(|&((p, h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep"]);
        f.extend([V::I(a[0]), V::F(a[1] as f64 / a[3] as f64), V::F(a[2] as f64 / a[3] as f64), ostr(h.and_then(|h| h.0)), ots(h.map(|h| h.1))]);
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
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// pt.Name AS PostType,
// ph.UserDisplayName AS LastEditor,
// ph.CreationDate AS LastEditDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.LastEditorUserId = ph.UserId AND p.LastEditDate = ph.CreationDate
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName,
// pt.Name, ph.UserDisplayName, ph.CreationDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12478(db: &'static So) -> String {
    let PostHistory { user, creation_date, user_display_name, .. } = &db.post_history;
    let by_user_time: HashIdx<(Id<User>, i64), Id<PostHistory>> = db.post_history.select(user.and(creation_date)).inv().collect();
    let edits = (&db.post.last_editor_user).and(&db.post.last_edit_date).select(&by_user_time);
    let mut v = post_history_groups(db, owned_since(db, date(2023, 1, 1)), edits, user_display_name.opt().and(creation_date));
    v.sort_by_key(|&((p, h), _)| (newest(db, p), db.post.origid.get(p).unwrap(), h.is_none(), h.map(|h| (h.0.is_none(), h.0, h.1))));
    rows(v.iter().take(100).map(|&((p, h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["type"]));
        f.extend([ostr(h.and_then(|h| h.0)), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// Posts per type (or a key like it), folding what each post is joined to.
fn fold_by<Q, K, R, S, F>(base: Q, key: K, joined: R, init: S, f: F) -> Vec<(ROf<K>, S)>
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
    let mut v = Vec::new();
    base.group_by(key).select(joined).fold(init, f).drive(|k, a| v.push((k, a)));
    v
}

fn type_name(db: &'static So) -> impl IntoQuery<Q: Probe<D = Id<Post>, R = Str>> {
    (&db.post.post_type).select(&db.post_type.name)
}

fn by_first<K, const N: usize>(mut v: Vec<(K, [i64; N])>) -> Vec<(K, [i64; N])> {
    v.sort_by_key(|x| Reverse(x.1[0]));
    v
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalQuestionsWithPositiveScore,
// AVG(u.Reputation) AS AverageUserReputation,
// AVG(p.ViewCount) AS AverageViewCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalPostsClosed,
// SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS TotalPostsReopened,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13126(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let v = fold_by(
        db.post.iq(),
        type_name(db),
        score.and(view_count.opt()).and(owner_user.select(&db.user.reputation).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()),
        [0i64; 10],
        |a, ((((s, w), r), h), x)| {
            [a[0] + 1, a[1] + (s > 0) as i64, a[2] + r.is_some() as i64, a[3] + r.unwrap_or(0), a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + (h == Some(10)) as i64, a[7] + (h == Some(11)) as i64, a[8] + (x == Some(2)) as i64, a[9] + (x == Some(3)) as i64]
        },
    );
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), avg(a[5], a[4]), V::I(a[6]), V::I(a[7]), V::I(a[8]), V::I(a[9])])))
}

// WITH BenchmarkData AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.Reputation AS OwnerReputation,
// u.CreationDate AS OwnerCreationDate,
// p.LastActivityDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// )
// SELECT
// PostTypeId,
// COUNT(PostId) AS TotalPosts,
// AVG(ViewCount) AS AvgViews,
// AVG(Score) AS AvgScore,
// AVG(AnswerCount) AS AvgAnswers,
// AVG(CommentCount) AS AvgComments,
// AVG(OwnerReputation) AS AvgOwnerReputation,
// MAX(LastActivityDate) AS LastActivity
// FROM
// BenchmarkData
// GROUP BY
// PostTypeId
// ORDER BY
// PostTypeId;
fn q13163(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, owner_user, last_activity_date, post_type_id, creation_date, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)));
    let mut v = fold_by(
        base,
        post_type_id,
        view_count.opt().and(score).and(answer_count.opt()).and(comment_count).and(owner_user.select(&db.user.reputation)).and(last_activity_date),
        [0, 0, 0, 0, 0, 0, 0, 0, i64::MIN],
        |a: [i64; 9], (((((w, s), an), c), r), la)| {
            [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c, a[7] + r, a[8].max(la)]
        },
    );
    v.sort_by_key(|x| x.0);
    rows(v.iter().map(|&(t, a)| row(vec![V::I(t), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), avg(a[5], a[4]), avg(a[6], a[0]), avg(a[7], a[0]), V::T(a[8])])))
}

// WITH Benchmark AS (
// SELECT
// PH.Id AS PostHistoryId,
// PT.Name AS PostType,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// U.DisplayName AS UserDisplayName,
// PH.CreationDate AS HistoryCreationDate,
// PH.Comment AS HistoryComment,
// PH.Text AS HistoryText
// FROM
// PostHistory PH
// JOIN
// Posts P ON PH.PostId = P.Id
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// JOIN
// Users U ON PH.UserId = U.Id
// WHERE
// PH.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
// )
// SELECT
// PostType,
// COUNT(*) AS TotalHistoryEntries,
// MIN(HistoryCreationDate) AS FirstEntryDate,
// MAX(HistoryCreationDate) AS LastEntryDate,
// AVG(EXTRACT(EPOCH FROM HistoryCreationDate - PostCreationDate)) AS AvgTimeToHistoryEntry
// FROM
// Benchmark
// GROUP BY
// PostType
// ORDER BY
// TotalHistoryEntries DESC;
fn q11444(db: &'static So) -> String {
    let PostHistory { post, user, creation_date, .. } = &db.post_history;
    let mut v = Vec::new();
    db.post_history
        .with(creation_date.ge(date(2024, 9, 1)).and(user))
        .group_by(post.select(type_name(db)))
        .select(creation_date.and(post.select(&db.post.creation_date)))
        .fold((0i64, i64::MAX, i64::MIN, 0i128), |(n, mn, mx, d), (h, c)| (n + 1, mn.min(h), mx.max(h), d + (h - c) as i128))
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|x| Reverse(x.1.0));
    rows(v.iter().map(|&(k, (n, mn, mx, d))| row(vec![V::S(k), V::I(n), V::T(mn), V::T(mx), V::F(d as f64 / n as f64 / 1e6)])))
}

// Types LEFT JOIN Votes: COUNT(p.Id), SUM(Score), non-null views and their
// sum, SUM(CASE WHEN v.Id IS NOT NULL ...), up, down, over the joined rows.
fn type_votes<Q, K>(db: &'static So, base: Q, key: K) -> Vec<(ROf<K>, [i64; 7])>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    K: IntoQuery,
    K::Q: Probe<D = Id<Post>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let Post { score, view_count, .. } = &db.post;
    fold_by(base, key, score.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()), [0i64; 7], |a, ((s, w), x)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x.is_some() as i64, a[5] + (x == Some(2)) as i64, a[6] + (x == Some(3)) as i64]
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS NumberOfPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// NumberOfPosts DESC;
fn q12303(db: &'static So) -> String {
    rows(by_first(type_votes(db, db.post.iq(), type_name(db))).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
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
fn q10937(db: &'static So) -> String {
    rows(by_first(type_votes(db, db.post.iq(), type_name(db))).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(AVG(p.ViewCount), 0) AS AvgViewCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
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
fn q11929(db: &'static So) -> String {
    rows(by_first(type_votes(db, db.post.iq(), type_name(db))).iter().map(|&(k, a)| {
        row(vec![V::S(k), V::I(a[0]), V::F(if a[2] == 0 { 0.0 } else { a[3] as f64 / a[2] as f64 }), V::I(a[5]), V::I(a[6])])
    }))
}

// SELECT
// pt.Name AS PostType,
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name, p.OwnerUserId
// ORDER BY
// pt.Name, PostCount DESC;
fn q14512(db: &'static So) -> String {
    let key = type_name(db).opt().and((&db.post.owner_user_id).opt());
    rows(type_votes(db, db.post.iq(), key).iter().map(|&((k, u), a)| row(vec![ostr(k), oint(u), V::I(a[0]), avg(a[1], a[0]), V::I(a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate BETWEEN '2023-01-01' AND '2023-12-31'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11200(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).between(date(2023, 1, 1), date(2023, 12, 31)));
    rows(by_first(type_votes(db, base, type_name(db).opt())).iter().map(|&(k, a)| row(vec![ostr(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[4])])))
}

// SELECT
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AveragePostScore,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.OwnerUserId
// ORDER BY
// TotalPosts DESC;
fn q12620(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(date(2023, 1, 1)));
    rows(type_votes(db, base, (&db.post.owner_user_id).opt()).iter().map(|&(u, a)| row(vec![V::I(a[0]), avg(a[1], a[0]), V::I(a[4]), oint(u)])))
}

// FROM PostTypes pt LEFT JOIN Posts p LEFT JOIN Votes v.
fn types_left_votes<K>(db: &'static So, key: K) -> Vec<(ROf<K>, [i64; 3])>
where
    K: IntoQuery,
    K::Q: Probe<D = Id<PostType>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let of: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let mut v = Vec::new();
    db.post_type
        .group_by(key)
        .select((&of).select((&db.post.score).and(votes_of(db).opt())).opt())
        .fold([0i64; 3], |a, r| match r {
            Some((s, x)) => [a[0] + 1, a[1] + s, a[2] + x.is_some() as i64],
            None => a,
        })
        .drive(|k, a| v.push((k, a)));
    v
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10581(db: &'static So) -> String {
    rows(by_first(types_left_votes(db, &db.post_type.name)).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// SELECT
// pt.Id AS PostTypeId,
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON pt.Id = p.PostTypeId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Id, pt.Name
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q14815(db: &'static So) -> String {
    rows(by_first(types_left_votes(db, (&db.post_type.origid).and(&db.post_type.name))).iter().take(100).map(|&((i, k), a)| row(vec![V::I(i), V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(COALESCE(p.Score, 0)) AS AverageScore,
// COUNT(v.Id) AS TotalVotes,
// COUNT(DISTINCT u.Id) AS UniqueUsers
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// Users u ON u.Id = p.OwnerUserId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12134(db: &'static So) -> String {
    let of: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let name = &db.post_type.name;
    let main = db.post_type.group_by(name).select((&of).select((&db.post.score).and(votes_of(db).opt())).opt()).fold([0i64; 4], |a, r| match r {
        Some((s, x)) => [a[0] + 1, a[1] + s, a[2] + x.is_some() as i64, a[3] + 1],
        None => [a[0], a[1], a[2], a[3] + 1],
    });
    let users = db.post_type.group_by(name).select((&of).select(&db.post.owner_user)).count_distinct();
    let mut v = Vec::new();
    main.and((&users).opt()).drive(|k, (a, u)| v.push((k, [a[0], a[1], a[2], a[3], u.unwrap_or(0)])));
    // AVG(COALESCE(p.Score, 0)) averages the NULL row of a type with no posts as 0
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[3]), V::I(a[2]), V::I(a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(CASE WHEN p.PostTypeId = 1 THEN COALESCE(p.AnswerCount, 0) ELSE NULL END) AS AvgAnswersPerQuestion,
// SUM(CASE WHEN v.VoteTypeId = 2 OR v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalVotes
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
fn q10668(db: &'static So) -> String {
    let Post { post_type_id, answer_count, .. } = &db.post;
    let v = fold_by(db.post.iq(), type_name(db), post_type_id.and(answer_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()), [0i64; 4], |a, ((t, an), x)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + if t == 1 { an.unwrap_or(0) } else { 0 }, a[3] + matches!(x, Some(2) | Some(3)) as i64]
    });
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), V::I(a[3])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14598(db: &'static So) -> String {
    let v = fold_by(db.post.iq(), type_name(db).opt(), (&db.post.score).and(comments_of(db).opt()), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c.is_some() as i64]);
    rows(by_first(v).iter().map(|&(k, a)| row(vec![ostr(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
// AVG(u.Reputation) AS AverageUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12168(db: &'static So) -> String {
    let v = fold_by(db.post.iq(), type_name(db), (&db.post.score).and(comments_of(db).opt()).and((&db.post.owner_user).select(&db.user.reputation).opt()), [0i64; 5], |a, ((s, c), r)| {
        [a[0] + 1, a[1] + s, a[2] + c.is_some() as i64, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0)]
    });
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), avg(a[4], a[3])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score ELSE NULL END) AS AverageScore,
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
// PostCount DESC;
fn q14753(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let main = db.post.group_by(type_name(db)).select(post_type_id.and(score).and(comments_of(db).opt())).fold([0i64; 3], |a, ((t, s), _)| {
        let x = if t == 1 || t == 2 { Some(s) } else { None };
        [a[0] + 1, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0)]
    });
    let cs = db.post.group_by(type_name(db)).select(comments_of(db)).count_distinct();
    let mut v = Vec::new();
    main.and((&cs).opt()).drive(|k, (a, c)| v.push((k, [a[0], a[1], a[2], c.unwrap_or(0)])));
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), V::I(a[3])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AvgViewCount,
// SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostsWithOwners,
// AVG(u.Reputation) AS AvgOwnerReputation
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
fn q11974(db: &'static So) -> String {
    let Post { view_count, owner_user_id, owner_user, .. } = &db.post;
    let v = fold_by(db.post.iq(), type_name(db), view_count.opt().and(owner_user_id.opt()).and(owner_user.select(&db.user.reputation).opt()), [0i64; 6], |a, ((w, o), r)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + o.is_some() as i64, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)]
    });
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), V::I(a[3]), avg(a[5], a[4])])))
}

// SELECT
// P.PostTypeId,
// COUNT(P.Id) AS TotalPosts,
// SUM(P.ViewCount) AS TotalViews,
// COUNT(DISTINCT V.UserId) AS UniqueVoters,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.PostTypeId
// ORDER BY
// TotalPosts DESC;
fn q12451(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let main = db.post.group_by(post_type_id).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 5], |a, (w, x)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64]
    });
    let voters = db.post.group_by(post_type_id).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let mut v = Vec::new();
    main.and((&voters).opt()).drive(|t, (a, u)| v.push((t, [a[0], a[1], a[2], a[3], a[4], u.unwrap_or(0)])));
    rows(by_first(v).iter().map(|&(t, a)| row(vec![V::I(t), V::I(a[0]), nullable(a[2], a[1]), V::I(a[5]), V::I(a[3]), V::I(a[4])])))
}

// SELECT
// DATE_TRUNC('month', p.CreationDate) AS Month,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// Month
// ORDER BY
// Month ASC;
fn q12061(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(ts(2023, 10, 1, 12, 34, 56)));
    let month = (&db.post.creation_date).map(trunc_month);
    let p = (&base).group_by(&month).select(Ident::<Post>::new()).count_distinct();
    let c = (&base).group_by(&month).select(comments_of(db)).count_distinct();
    let x = (&base).group_by(&month).select(votes_of(db)).count_distinct();
    let mut v = Vec::new();
    p.and((&c).opt()).and((&x).opt()).drive(|m, ((p, c), x)| v.push((m, p, c.unwrap_or(0), x.unwrap_or(0))));
    rows(v.iter().map(|&(m, p, c, x)| row(vec![V::T(m), V::I(p), V::I(c), V::I(x)])))
}

fn count<Q: Drive>(q: Q) -> i64 {
    q.fold_flat(0i64, |a, _| a + 1)
}

fn one(f: Fold<(), i64>) -> i64 {
    (&f).fold_flat(0i64, |a, x| a + x)
}

// SELECT
// (SELECT COUNT(DISTINCT OwnerUserId) FROM Posts WHERE PostTypeId = 1) AS TotalQuestions,
// (SELECT COUNT(DISTINCT OwnerUserId) FROM Posts WHERE PostTypeId = 2) AS TotalAnswers
// ;
fn q10189(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, .. } = &db.post;
    let q = one(whole(db.post.with(post_type_id.eq(1))).select(owner_user_id).count_distinct());
    let a = one(whole(db.post.with(post_type_id.eq(2))).select(owner_user_id).count_distinct());
    row(vec![V::I(q), V::I(a)])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT AVG(Reputation) FROM Users) AS AverageUserReputation,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT COUNT(DISTINCT PostId) FROM Votes) AS TotalVotedPosts;
fn q14308(db: &'static So) -> String {
    let (n, r) = (&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let voted = one(whole(db.vote.iq()).select(&db.vote.post_id).count_distinct());
    row(vec![V::I(count(db.post.iq())), V::I(count(db.comment.iq())), V::I(count(db.user.iq())), avg(r, n), V::I(count(db.badge.iq())), V::I(count(db.vote.iq())), V::I(voted)])
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(DISTINCT OwnerUserId) FROM Posts) AS UniqueUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q10771(db: &'static So) -> String {
    let comments = count(db.comment.iq());
    let users = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let v = fold_by(db.post.iq(), type_name(db), &db.post.score, [0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(comments), V::I(users)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViews,
// (SELECT COUNT(DISTINCT u.Id) FROM Users u) AS TotalUsers,
// (SELECT COUNT(b.Id) FROM Badges b) AS TotalBadges
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12946(db: &'static So) -> String {
    let users = one(whole(db.user.iq()).select(Ident::<User>::new()).count_distinct());
    let badges = count(db.badge.iq());
    rows(by_count(db).iter().map(|a| {
        let mut f = type_fields(a, &["name", "n", "score_avg", "views_avg"]);
        f.extend([V::I(users), V::I(badges)]);
        row(f)
    }))
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges,
// AVG(Score) AS AveragePostScore,
// AVG(ViewCount) AS AveragePostViewCount
// FROM
// Posts
// WHERE
// CreationDate >= '2023-01-01'
// AND PostTypeId = 1
// GROUP BY
// EXTRACT(YEAR FROM CreationDate)
// ORDER BY
// TotalPosts DESC;
fn q14973(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let (posts, users, badges) = (count(db.post.iq()), count(db.user.iq()), count(db.badge.iq()));
    let base = db.post.with(creation_date.ge(date(2023, 1, 1)).and(post_type_id.eq(1)));
    let v = fold_by(base, creation_date.map(year), score.and(view_count.opt()), [0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    rows(v.iter().map(|&(_, a)| row(vec![V::I(posts), V::I(users), V::I(badges), avg(a[1], a[0]), avg(a[3], a[2])])))
}

// Users LEFT JOIN Posts [LEFT JOIN Votes], folded per user or per name.
fn user_votes<K>(db: &'static So, key: K) -> Vec<(ROf<K>, [i64; 5])>
where
    K: IntoQuery,
    K::Q: Probe<D = Id<User>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let mut v = Vec::new();
    db.user
        .group_by(key)
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.creation_date).and((&db.vote.bounty_amount).opt())).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + x.is_some() as i64, a[4] + x.and_then(|x| x.1).unwrap_or(0)],
            None => a,
        })
        .drive(|k, a| v.push((k, a)));
    v
}

// SELECT u.Id, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// ORDER BY PostCount DESC
// LIMIT 10;
fn q19028(db: &'static So) -> String {
    rows(by_first(user_votes(db, Ident::<User>::new())).iter().take(10).map(|&(u, a)| row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(a[0]), V::I(a[3])])))
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties
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
fn q18157(db: &'static So) -> String {
    rows(by_first(user_votes(db, &db.user.display_name)).iter().take(10).map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[4])])))
}

// SELECT U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.DisplayName
// HAVING COUNT(P.Id) > 0
// ORDER BY PostCount DESC;
fn q17825(db: &'static So) -> String {
    let mut v = Vec::new();
    db.user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(votes_of(db).opt()).opt())
        .fold((0i64, 0i64), |(n, x), p| match p {
            Some(vi) => (n + 1, x + vi.is_some() as i64),
            None => (n, x),
        })
        .filt(|(n, _)| n > 0)
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, (n, x))| row(vec![V::S(k), V::I(n), V::I(x)])))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswers,
// AVG(U.Reputation) AS AverageReputation
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q10342(db: &'static So) -> String {
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select((&db.post.accepted_answer_id).opt()).opt()))
        .fold([0i64; 4], |a, (r, p)| [a[0] + p.is_some() as i64, a[1] + p.flatten().is_some() as i64, a[2] + r, a[3] + 1])
        .drive(|u, a| v.push((u, a)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(100).map(|&(u, a)| {
        row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(db.user.reputation.get(u).unwrap()), V::I(a[0]), V::I(a[1]), avg(a[2], a[3])])
    }))
}

// Users LEFT JOIN Posts GROUP BY u.DisplayName with COUNT(DISTINCT p.Id).
fn distinct_posts_by_name(db: &'static So, n: usize) -> String {
    let name = &db.user.display_name;
    let posts = db.user.group_by(name).select(posts_of(db)).count_distinct();
    let main = db.user.group_by(name).select((&db.user.reputation).and(posts_of(db).select(&db.post.post_type_id).opt())).fold([0i64; 4], |a, (r, t)| {
        [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + r, a[3] + 1]
    });
    let mut v = Vec::new();
    main.and((&posts).opt()).drive(|k, (a, d)| v.push((k, a, d.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.2));
    let n = if n == 0 { v.len() } else { n };
    rows(v.iter().take(n).map(|&(k, a, d)| row(vec![V::S(k), V::I(d), V::I(a[0]), V::I(a[1]), avg(a[2], a[3])])))
}

// SELECT
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
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
fn q15726(db: &'static So) -> String {
    distinct_posts_by_name(db, 10)
}

// SELECT
// Users.DisplayName,
// COUNT(DISTINCT Posts.Id) AS PostCount,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(Users.Reputation) AS AverageReputation
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.DisplayName
// ORDER BY
// PostCount DESC;
fn q15994(db: &'static So) -> String {
    distinct_posts_by_name(db, 0)
}

// SELECT
// Users.DisplayName,
// COUNT(DISTINCT Posts.Id) AS PostCount,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(Users.Reputation) AS AverageReputation
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q17599(db: &'static So) -> String {
    distinct_posts_by_name(db, 10)
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// MAX(p.CreationDate) AS MostRecentPostDate,
// MIN(p.CreationDate) AS FirstPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// u.Reputation DESC;
fn q11212(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let main = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt()).and(creation_date)).opt()).fold(
        [0, 0, 0, i64::MIN, i64::MAX, 0],
        |a: [i64; 6], p| match p {
            Some(((s, w), c)) => [a[0] + s, a[1] + w.unwrap_or(0), a[2] + 1, a[3].max(c), a[4].min(c), a[5]],
            None => a,
        },
    );
    let posts = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let mut v = Vec::new();
    main.and((&posts).opt()).drive(|u, (a, n)| v.push((u, a, n.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, n)| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(db.user.reputation.get(u).unwrap()),
            V::I(n),
            V::I(a[0]),
            V::I(a[1]),
            if a[2] == 0 { V::Null } else { V::T(a[3]) },
            if a[2] == 0 { V::Null } else { V::T(a[4]) },
        ])
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount
// FROM
// UserPostStats
// WHERE
// PostCount > 0
// ORDER BY
// Reputation DESC, PostCount DESC
// LIMIT 10;
fn q14926(db: &'static So) -> String {
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt())
        .fold(0i64, |n, p| n + p.is_some() as i64)
        .filt(|n| n > 0)
        .drive(|u, n| v.push((u, n)));
    v.sort_by_key(|&(u, n)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)));
    rows(v.iter().take(10).map(|&(u, n)| row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(db.user.reputation.get(u).unwrap()), V::I(n)])))
}

// SELECT
// p.Title,
// u.DisplayName AS Author,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE( (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19301(db: &'static So) -> String {
    let mut v = Vec::new();
    questions(db).select(comments_per_post(db)).drive(|p, n| v.push((newest(db, p), p, n)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(10).map(|&(_, p, n)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.push(V::I(n));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("11330", q11330),
    ("14510", q14510),
    ("14261", q14261),
    ("11105", q11105),
    ("12486", q12486),
    ("10169", q10169),
    ("11269", q11269),
    ("11070", q11070),
    ("11794", q11794),
    ("14061", q14061),
    ("13441", q13441),
    ("14599", q14599),
    ("14588", q14588),
    ("10936", q10936),
    ("14003", q14003),
    ("10612", q10612),
    ("13813", q13813),
    ("12783", q12783),
    ("12589", q12589),
    ("12264", q12264),
    ("12478", q12478),
    ("13126", q13126),
    ("13163", q13163),
    ("11444", q11444),
    ("12303", q12303),
    ("10937", q10937),
    ("11929", q11929),
    ("14512", q14512),
    ("11200", q11200),
    ("12620", q12620),
    ("10581", q10581),
    ("14815", q14815),
    ("12134", q12134),
    ("10668", q10668),
    ("14598", q14598),
    ("12168", q12168),
    ("14753", q14753),
    ("11974", q11974),
    ("12451", q12451),
    ("12061", q12061),
    ("10189", q10189),
    ("14308", q14308),
    ("10771", q10771),
    ("12946", q12946),
    ("14973", q14973),
    ("19028", q19028),
    ("18157", q18157),
    ("17825", q17825),
    ("10342", q10342),
    ("15726", q15726),
    ("15994", q15994),
    ("17599", q17599),
    ("11212", q11212),
    ("14926", q14926),
    ("19301", q19301),
];
