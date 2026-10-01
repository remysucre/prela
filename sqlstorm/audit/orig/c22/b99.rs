use harness::prelude::*;
use std::cmp::Reverse;

fn cd(db: &'static So, p: Id<Post>) -> Reverse<i64> {
    Reverse(db.post.creation_date.get(p).unwrap())
}

fn score(db: &'static So, p: Id<Post>) -> Reverse<i64> {
    Reverse(db.post.score.get(p).unwrap())
}

fn views(db: &'static So, p: Id<Post>) -> (bool, Reverse<Option<i64>>) {
    let w = db.post.view_count.get(p);
    (w.is_none(), Reverse(w))
}

fn owned_since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.creation_date).ge(d))
}

// SELECT
// u.Reputation AS UserReputation,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// p.Score AS PostScore,
// p.ViewCount AS PostViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
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
// p.CreationDate > '2023-01-01'
// GROUP BY
// u.Reputation, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Id, p.Id
// ORDER BY
// PostScore DESC, PostViewCount DESC, UserReputation DESC
// LIMIT 100;
fn q12855(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).gt(date(2023, 1, 1)));
    stat_rows(db, post_stats(db, base, "cvb", &[]), |p, _| (score(db, p), views(db, p), Reverse(db.post.owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap()))), 100, &["rep", "title", "created", "score", "views", "#cx", "#vx", "bmax"])
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
// u.Reputation AS UserReputation,
// u.DisplayName AS UserDisplayName,
// COUNT(v.Id) AS VoteCount,
// AVG(v.BountyAmount) AS AverageBounty
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Reputation, u.DisplayName
// ORDER BY
// p.Score DESC, VoteCount DESC, p.CreationDate DESC
// LIMIT 100;
fn q13648(db: &'static So) -> String {
    stat_rows(db, post_stats(db, owned_since(db, date(2022, 1, 1)), "v", &[]), |p, s| (score(db, p), Reverse(s.vx), cd(db, p)), 100, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "rep", "owner", "#vx", "bounty_avg"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// u.LastAccessDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
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
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score,
// u.Id, u.DisplayName, u.Reputation,
// u.CreationDate, u.LastAccessDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13871(db: &'static So) -> String {
    stat_rows(db, post_stats(db, owned_since(db, date(2020, 1, 1)), "cv", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "views", "score", "uid", "owner", "rep", "ucreated", "last_access", "#cx", "#vx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// u.LastAccessDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.Score > 0
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount,
// u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate
// ORDER BY
// p.Score DESC, p.ViewCount DESC, p.CreationDate DESC
// LIMIT 100;
fn q10365(db: &'static So) -> String {
    let base = owned(db).with((&db.post.score).gt(0));
    stat_rows(db, post_stats(db, base, "cv", &[]), |p, _| (score(db, p), views(db, p), cd(db, p)), 100, &["id", "title", "created", "score", "views", "uid", "owner", "rep", "ucreated", "last_access", "#cx", "#vx"])
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
// AVG(v.BountyAmount) AS AverageBountyAmount,
// COUNT(b.Id) AS BadgeCount,
// COUNT(ph.Id) AS PostHistoryCount
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
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12360(db: &'static So) -> String {
    let base = owned(db).with((&db.post.post_type_id).eq(1));
    stat_rows(db, post_stats(db, base, "cvbh", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx", "bounty_avg", "#bx", "#hx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes,
// MAX(v.CreationDate) AS LastVoteDate,
// MAX(h.CreationDate) AS LastEditDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory h ON p.Id = h.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14099(db: &'static So) -> String {
    stat_rows(db, post_stats(db, owned(db), "cvh", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "owner", "rep", "#cx", "#vx", "vmax", "hmax"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.Id AS OwnerId,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes,
// AVG(v.BountyAmount) AS AverageBountyAmount,
// MAX(ph.CreationDate) AS LastPostHistoryChange
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
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13588(db: &'static So) -> String {
    stat_rows(db, post_stats(db, owned(db), "cvh", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "views", "score", "answers", "comments", "uid", "owner", "rep", "#cx", "#vx", "bounty_avg", "hmax"])
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
// COUNT(ph.Id) AS PostHistoryCount,
// t.TagName,
// pt.Name AS PostTypeName
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
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, t.TagName, pt.Name
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10706(db: &'static So) -> String {
    let tag: HashIdx<Id<Post>, Str> = (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).collect();
    let mut v = group_stats(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new().and((&tag).opt()), "cvh", &[]);
    v.sort_by_key(|&((p, _), _)| cd(db, p));
    rows(v.iter().take(100).map(|((p, tn), s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx", "#hx"]);
        f.push(ostr(*tn));
        f.extend(post_fields(db, *p, &["type"]));
        row(f)
    }))
}

// SELECT
// u.DisplayName AS UserName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// p.ViewCount AS PostViewCount,
// p.Score AS PostScore,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MIN(ph.CreationDate) AS FirstEditDate,
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
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// u.DisplayName, p.Title, p.CreationDate, p.ViewCount, p.Score
// ORDER BY
// PostScore DESC, PostViewCount DESC;
fn q11022(db: &'static So) -> String {
    let Post { title, creation_date, view_count, score, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(view_count.opt()).and(score);
    let mut v = group_stats(db, owned_since(db, date(2022, 1, 1)), key, "cvh", &[]);
    v.sort_by_key(|&(((((_, _), _), w), s), _)| (Reverse(s), w.is_none(), Reverse(w)));
    rows(v.iter().map(|&(((((dn, t), c), w), s), ref st)| {
        let mut f = vec![V::S(dn), ostr(t), V::T(c), oint(w), V::I(s)];
        f.extend(["#cx", "#vx", "hmin", "hmax"].iter().map(|x| stat_field(st, x).unwrap()));
        row(f)
    }))
}

// GROUP BY a post's columns and some of its history's: the (post, history)
// rows, one per history row or one with none, are the elements grouped.
type PostHist = MatSet<(Id<Post>, Option<Id<PostHistory>>)>;

fn post_hist<Q: Drive<D = Id<Post>, R = Id<Post>>>(db: &'static So, base: Q) -> PostHist {
    base.select(Ident::<Post>::new().and(history_of(db).opt())).collect()
}

// COUNT(c.Id), COUNT(v.Id) for each (post, history type, history date).
fn by_history_type_date(db: &'static So, j: &PostHist) -> Vec<((Id<Post>, Option<(i64, i64)>), (i64, i64))> {
    let post_of = j.map(|(p, _)| p);
    let hist_of = j.flat_map(|(_, h)| h);
    let PostHistory { post_history_type_id, creation_date, .. } = &db.post_history;
    let mut v = Vec::new();
    j.group_by((&post_of).and((&hist_of).select(post_history_type_id.and(creation_date)).opt()))
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).opt())))
        .fold((0i64, 0i64), |(c, x), (ci, vi)| (c + ci.is_some() as i64, x + vi.is_some() as i64))
        .drive(|k, a| v.push((k, a)));
    v
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
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// ph.PostHistoryTypeId,
// ph.CreationDate AS HistoryCreationDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
// u.Id, u.DisplayName, u.Reputation,
// ph.PostHistoryTypeId, ph.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q10892(db: &'static So) -> String {
    let j = post_hist(db, owned_since(db, date(2022, 1, 1)));
    rows(by_history_type_date(db, &j).iter().map(|&((p, h), (c, x))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "uid", "owner", "rep"]);
        f.extend([V::I(x), V::I(c), oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
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
// u.Reputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// t.TagName,
// ph.PostHistoryTypeId,
// ph.CreationDate AS HistoryCreationDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= DATE '2023-01-01' AND p.CreationDate < DATE '2024-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount,
// u.DisplayName, u.Reputation,
// t.TagName,
// ph.PostHistoryTypeId, ph.CreationDate
// ORDER BY
// p.Score DESC, p.ViewCount DESC;
fn q10819(db: &'static So) -> String {
    let tag: HashIdx<Id<Post>, Str> = (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).collect();
    let base = owned(db).with((&db.post.creation_date).ge(date(2023, 1, 1)).and((&db.post.creation_date).lt(date(2024, 1, 1))));
    let j = post_hist(db, base);
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let PostHistory { post_history_type_id, creation_date, .. } = &db.post_history;
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&post_of).select((&tag).opt())).and((&hist_of).select(post_history_type_id.and(creation_date)).opt()))
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).opt())))
        .fold((0i64, 0i64), |(c, x), (ci, vi)| (c + ci.is_some() as i64, x + vi.is_some() as i64))
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(((p, tn), h), (c, x))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep"]);
        f.extend([V::I(c), V::I(x), ostr(tn), oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
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
// p.FavoriteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// ph.PostHistoryTypeId,
// ph.CreationDate AS PostHistoryDate,
// COUNT(c.Id) AS CommentCountTotal,
// COUNT(v.Id) AS VoteCount
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
// p.CreationDate >= DATE '2020-01-01' AND p.CreationDate < DATE '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount,
// u.DisplayName, u.Reputation, ph.PostHistoryTypeId, ph.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q10350(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(date(2020, 1, 1)).and((&db.post.creation_date).lt(date(2023, 1, 1))));
    let j = post_hist(db, base);
    rows(by_history_type_date(db, &j).iter().map(|&((p, h), (c, x))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep"]);
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1)), V::I(c), V::I(x)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// PHT.Name AS PostHistoryType,
// COUNT(PH.Id) AS HistoryChangeCount
// FROM
// Posts p
// JOIN
// Users U ON p.OwnerUserId = U.Id
// LEFT JOIN
// PostHistory PH ON p.Id = PH.PostId
// LEFT JOIN
// PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
// U.DisplayName, U.Reputation, PHT.Name
// ORDER BY
// p.CreationDate DESC;
fn q14649(db: &'static So) -> String {
    let j = post_hist(db, owned_since(db, date(2023, 1, 1)));
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&hist_of).select((&db.post_history.post_history_type).select(&db.post_history_type.name)).opt()))
        .select((&hist_of).opt())
        .fold(0i64, |a, h| a + h.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    rows(v.iter().map(|&((p, t), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "rep"]);
        f.extend([ostr(t), V::I(n)]);
        row(f)
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate AS UserCreationDate,
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.Score,
// P.ViewCount,
// P.CommentCount,
// P.AnswerCount,
// PH.CreationDate AS PostHistoryDate,
// PHT.Name AS PostHistoryType,
// COUNT(CM.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount
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
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate,
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount,
// P.CommentCount, P.AnswerCount, PH.CreationDate, PHT.Name
// ORDER BY
// U.Reputation DESC, P.CreationDate DESC;
fn q14771(db: &'static So) -> String {
    let base = owned(db).with((&db.post.owner_user).select(&db.user.reputation).gt(1000));
    let j = post_hist(db, base);
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let PostHistory { post_history_type, creation_date, .. } = &db.post_history;
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&hist_of).select(creation_date.and(post_history_type.select(&db.post_history_type.name))).opt()))
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).opt())))
        .fold((0i64, 0i64), |(c, x), (ci, vi)| (c + ci.is_some() as i64, x + vi.is_some() as i64))
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((p, h), (c, x))| {
        let mut f = post_fields(db, p, &["uid", "owner", "rep", "ucreated", "id", "title", "created", "score", "views", "comments", "answers"]);
        f.extend([ots(h.map(|h| h.0)), ostr(h.map(|h| h.1)), V::I(c), V::I(x)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// ph.UserDisplayName AS LastEditorDisplayName,
// ph.CreationDate AS LastEditDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// pt.Name AS PostTypeName
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.LastEditorUserId = ph.UserId AND p.Id = ph.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score, u.DisplayName, ph.UserDisplayName, ph.CreationDate, pt.Name
// ORDER BY
// p.ViewCount DESC;
fn q13211(db: &'static So) -> String {
    let PostHistory { user, user_display_name, creation_date, .. } = &db.post_history;
    let by_editor = (&db.post.last_editor_user)
        .and(history_of(db).select(Ident::<PostHistory>::new().and(user)))
        .filt(|(e, (_, u))| e == u)
        .map(|(_, (h, _))| h);
    let j: PostHist = db.post.with((&db.post.creation_date).ge(date(2023, 1, 1))).select(Ident::<Post>::new().and(by_editor.opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&hist_of).select(user_display_name.opt().and(creation_date)).opt()))
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).opt())))
        .fold((0i64, 0i64), |(c, x), (ci, vi)| (c + ci.is_some() as i64, x + vi.is_some() as i64))
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((p, h), (c, x))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        f.extend([ostr(h.and_then(|h| h.0)), ots(h.map(|h| h.1)), V::I(c), V::I(x)]);
        f.extend(post_fields(db, p, &["type"]));
        row(f)
    }))
}

// SELECT
// u.DisplayName AS UserName,
// p.Title AS PostTitle,
// ph.CreationDate AS HistoryCreationDate,
// ph.Comment AS HistoryComment,
// ph2.UserDisplayName AS EditorName,
// ph2.CreationDate AS EditCreationDate,
// COUNT(v.Id) AS TotalVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostHistory ph ON p.Id = ph.PostId
// JOIN
// PostHistory ph2 ON ph.PostId = ph2.PostId AND ph2.PostHistoryTypeId IN (4, 5, 6)
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// u.DisplayName, p.Title, ph.CreationDate, ph.Comment, ph2.UserDisplayName, ph2.CreationDate
// ORDER BY
// TotalVotes DESC, ph.CreationDate DESC;
fn q11319(db: &'static So) -> String {
    let PostHistory { post_history_type_id, creation_date, comment, user_display_name, .. } = &db.post_history;
    let edits = history_of(db).select(Ident::<PostHistory>::new().and(post_history_type_id.in_v(vec![4, 5, 6])).map(|(h, _)| h));
    let j: MatSet<(Id<Post>, Id<PostHistory>, Id<PostHistory>)> = owned_since(db, date(2023, 1, 1))
        .select(Ident::<Post>::new().and(history_of(db)).and(edits))
        .map(|((p, h), h2)| (p, h, h2))
        .collect();
    let post_of = (&j).map(|(p, _, _)| p);
    let key = (&post_of)
        .select((&db.post.owner_user).select(&db.user.display_name).and((&db.post.title).opt()))
        .and((&j).map(|(_, h, _)| h).select(creation_date.and(comment.opt())))
        .and((&j).map(|(_, _, h)| h).select(user_display_name.opt().and(creation_date)));
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(votes_of(db).opt()))
        .fold(0i64, |a, x| a + x.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    rows(v.iter().map(|&((((dn, t), (hd, hc)), (en, ed)), n)| row(vec![V::S(dn), ostr(t), V::T(hd), ostr(hc), ostr(en), V::T(ed), V::I(n)])))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.Body,
// P.CreationDate AS PostCreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// U.Id AS UserId,
// U.DisplayName AS UserDisplayName,
// U.Reputation AS UserReputation,
// T.TagName,
// COUNT(C.Id) AS CommentCountPerPost
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// JOIN
// Tags T ON P.Tags LIKE CONCAT('%,', T.TagName, ',%')
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.Body, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount, P.CommentCount, P.FavoriteCount,
// U.Id, U.DisplayName, U.Reputation,
// T.TagName
// ORDER BY
// P.CreationDate DESC
// LIMIT
// 100;
fn q14214(db: &'static So) -> String {
    let tagged = (&db.post.tags_str).select_where((&db.tag.tag_name).inv(), |s: Str, n: Str| s.contains(&format!(",{n},")));
    let groups: MatSet<(Id<Post>, Id<Tag>)> = owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(tagged)).collect();
    let post_of = (&groups).map(|(p, _)| p);
    let mut out = Vec::new();
    (&groups).group_by(Same::new())
        .select((&post_of).select(comments_of(db).opt()))
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|(p, t), n| out.push((cd(db, p), p, t, n)));
    out.sort_by_key(|x| x.0);
    rows(out.iter().take(100).map(|&(_, p, t, n)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "views", "score", "answers", "comments", "favorites", "uid", "owner", "rep"]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(n)]);
        row(f)
    }))
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
// (SELECT COUNT(*) FROM Votes) AS TotalVotes;
fn q13558(db: &'static So) -> String {
    row(vec![V::I(count(db.post.iq())), V::I(count(db.user.iq())), V::I(count(db.vote.iq()))])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS Total_Posts,
// (SELECT COUNT(*) FROM Users) AS Total_Users,
// (SELECT COUNT(*) FROM Votes) AS Total_Votes,
// AVG(Reputation) AS Average_User_Reputation
// FROM
// Users;
fn q11490(db: &'static So) -> String {
    row(vec![V::I(count(db.post.iq())), V::I(count(db.user.iq())), V::I(count(db.vote.iq())), mean(&db.user.reputation)])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1) AS AvgQuestionScore,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT MAX(Reputation) FROM Users) AS HighestReputation
fn q12437(db: &'static So) -> String {
    let max_rep = (&db.user.reputation).fold_flat(i64::MIN, |a, r| a.max(r));
    row(vec![
        V::I(count(db.post.iq())),
        mean(db.post.with((&db.post.post_type_id).eq(1)).select(&db.post.score)),
        V::I(count(db.user.iq())),
        V::I(max_rep),
    ])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// AVG(Score) AS AveragePostScore
// FROM
// Posts;
fn q10689(db: &'static So) -> String {
    row(vec![V::I(count(db.post.iq())), V::I(count(db.user.iq())), V::I(count(db.comment.iq())), V::I(count(db.vote.iq())), mean(&db.post.score)])
}

// SELECT
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// AVG(Reputation) AS AverageReputation
// FROM Users;
fn q13584(db: &'static So) -> String {
    row(vec![V::I(count(db.user.iq())), V::I(count(db.post.iq())), V::I(count(db.comment.iq())), V::I(count(db.vote.iq())), mean(&db.user.reputation)])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT AVG(VoteCount) FROM
// (SELECT COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId) AS PostVoteCounts) AS AverageVotesPerPost
fn q14893(db: &'static So) -> String {
    let per_post = db.vote.group_by(&db.vote.post_id).fold(0i64, |a, _| a + 1);
    row(vec![V::I(count(db.post.iq())), V::I(count(db.user.iq())), mean((&per_post).map(|n| n))])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT AVG(Reputation) FROM Users) AS AverageReputation,
// (SELECT MAX(CreationDate) FROM Posts) AS LatestPostCreationDate
// ;
fn q10797(db: &'static So) -> String {
    let latest = (&db.post.creation_date).fold_flat(i64::MIN, |a, d| a.max(d));
    row(vec![V::I(count(db.post.iq())), V::I(count(db.user.iq())), V::I(count(db.vote.iq())), mean(&db.user.reputation), V::T(latest)])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT COUNT(*) FROM Tags) AS TotalTags,
// (SELECT COUNT(*) FROM PostHistory) AS TotalPostHistoryEntries;
fn q12002(db: &'static So) -> String {
    row(vec![
        V::I(count(db.post.iq())),
        V::I(count(db.user.iq())),
        V::I(count(db.comment.iq())),
        V::I(count(db.vote.iq())),
        V::I(count(db.tag.iq())),
        V::I(count(db.post_history.iq())),
    ])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// AVG(Score) AS AveragePostScore,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges
// FROM
// Posts
// WHERE
// CreationDate >= '2023-01-01'
// AND ViewCount > 100;
fn q11102(db: &'static So) -> String {
    let recent = db.post.with((&db.post.creation_date).ge(date(2023, 1, 1)).and((&db.post.view_count).gt(100)));
    row(vec![V::I(count(db.post.iq())), V::I(count(db.user.iq())), V::I(count(db.comment.iq())), mean(recent.select(&db.post.score)), V::I(count(db.badge.iq()))])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// AVG(ViewCount) AS AveragePostViews,
// AVG((SELECT COUNT(*) FROM Votes WHERE Votes.PostId = Posts.Id)) AS AverageVotesPerPost
// FROM Posts;
fn q10826(db: &'static So) -> String {
    let (vn, vs) = db.post.select((&db.post.view_count).opt()).fold_flat((0i64, 0i64), |(n, s), w| (n + w.is_some() as i64, s + w.unwrap_or(0)));
    let vpp = votes_per_post(db);
    row(vec![V::I(count(db.post.iq())), V::I(count(db.comment.iq())), V::I(count(db.user.iq())), avg(vs, vn), mean(db.post.select(&vpp))])
}

// SELECT
// COUNT(P.Id) AS TotalPosts,
// AVG(P.Score) AS AveragePostScore,
// COUNT(DISTINCT V.UserId) AS TotalUniqueVoters
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= '2020-01-01'
// AND
// P.PostTypeId = 1;
fn q12347(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2020, 1, 1)).and(post_type_id.eq(1)));
    let (n, s) = (&base).select(score.and(votes_of(db).opt())).fold_flat((0i64, 0i64), |(n, s), (x, _)| (n + 1, s + x));
    let voters = whole(&base).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let u = (&voters).fold_flat(0i64, |a, x| a + x);
    row(vec![V::I(n), avg(s, n), V::I(u)])
}

// SELECT
// COUNT(P.Id) AS TotalPosts,
// AVG(P.ViewCount) AS AverageViewCount,
// COUNT(C.Id) AS TotalComments
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year';
fn q14879(db: &'static So) -> String {
    let (n, vn, vs, c) = db
        .post
        .with((&db.post.creation_date).ge(ts(2023, 10, 1, 12, 34, 56)))
        .select((&db.post.view_count).opt().and(comments_of(db).opt()))
        .fold_flat((0i64, 0i64, 0i64, 0i64), |(n, vn, vs, c), (w, ci)| (n + 1, vn + w.is_some() as i64, vs + w.unwrap_or(0), c + ci.is_some() as i64));
    row(vec![V::I(n), avg(vs, vn), V::I(c)])
}

// The newest questions with (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id).
fn newest_with_comments(db: &'static So, cols: &[&str]) -> String {
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(&cc).drive(|p, n| v.push((cd(db, p), p, n)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(10).map(|&(_, p, n)| {
        let mut f = post_fields(db, p, cols);
        f.push(V::I(n));
        row(f)
    }))
}

// SELECT
// p.Title,
// u.DisplayName AS Owner,
// p.CreationDate,
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
fn q15417(db: &'static So) -> String {
    newest_with_comments(db, &["title", "owner", "created", "score"])
}

// SELECT
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
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
fn q18042(db: &'static So) -> String {
    newest_with_comments(db, &["title", "owner", "created", "score"])
}

// SELECT p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Author,
// p.Score,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// WHERE p.PostTypeId = 1
// ORDER BY p.CreationDate DESC
// LIMIT 10;
fn q15303(db: &'static So) -> String {
    newest_with_comments(db, &["id", "title", "created", "owner", "score"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
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
fn q17946(db: &'static So) -> String {
    newest_with_comments(db, &["id", "title", "owner", "created", "score"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
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
fn q18054(db: &'static So) -> String {
    newest_with_comments(db, &["id", "title", "created", "owner", "score"])
}

// SELECT *,
// (SELECT COUNT(*) FROM Votes WHERE PostId = p.Id) AS VoteCount,
// (SELECT COUNT(*) FROM Comments WHERE PostId = p.Id) AS CommentCount
// FROM Posts p
// WHERE CreationDate >= '2023-01-01'
// ORDER BY CreationDate DESC
// LIMIT 10;
fn q17196(db: &'static So) -> String {
    let vpp = votes_per_post(db);
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post.with((&db.post.creation_date).ge(date(2023, 1, 1))).select((&vpp).and(&cc)).drive(|p, a| v.push((cd(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(10).map(|&(_, p, (x, c))| {
        let mut f = post_fields(db, p, POST_STAR);
        f.extend([V::I(x), V::I(c)]);
        row(f)
    }))
}

// Types with COUNT(DISTINCT u.Id) over `LEFT JOIN Users u`.
fn type_users(db: &'static So) -> Vec<(Str, i64, i64, i64)> {
    let Post { post_type, score, owner_user, .. } = &db.post;
    let name = post_type.select(&db.post_type.name);
    let main = db.post.group_by(&name).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let uniq = db.post.group_by(&name).select(owner_user).count_distinct();
    let mut v = Vec::new();
    main.and((&uniq).opt()).drive(|k, ((n, s), u)| v.push((k, n, s, u.unwrap_or(0))));
    v
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// AVG(p.Score) AS AverageScore
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
fn q11229(db: &'static So) -> String {
    let mut v = type_users(db);
    v.sort_by_key(|x| Reverse(x.1));
    rows(v.iter().map(|&(k, n, s, u)| row(vec![V::S(k), V::I(n), V::I(u), avg(s, n)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// AVG(p.Score) AS AveragePostScore
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
fn q14297(db: &'static So) -> String {
    let mut v = type_users(db);
    v.sort_by_key(|x| Reverse(x.1));
    rows(v.iter().map(|&(k, n, s, u)| row(vec![V::S(k), V::I(n), V::I(u), avg(s, n)])))
}

// SELECT
// pt.Name AS PostType,
// AVG(p.Score) AS AverageScore,
// COUNT(p.Id) AS TotalPosts,
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
// AverageScore DESC;
fn q13497(db: &'static So) -> String {
    let v = type_users(db);
    rows(v.iter().map(|&(k, n, s, u)| row(vec![V::S(k), avg(s, n), V::I(n), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(AVG(p.Score), 0) AS AverageScore,
// COUNT(c.Id) AS TotalComments
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
fn q12894(db: &'static So) -> String {
    let Post { post_type, score, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(comments_of(db).opt()))
        .fold((0i64, 0i64, 0i64), |(n, s, c), (x, ci)| (n + 1, s + x, c + ci.is_some() as i64))
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, (n, s, c))| row(vec![V::S(k), V::I(n), avg(s, n), V::I(c)])))
}

// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT b.UserId) AS TotalUsersWithBadges
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12462(db: &'static So) -> String {
    let Post { post_type, score, owner_user, .. } = &db.post;
    let name = post_type.select(&db.post_type.name);
    let main = db.post.group_by(&name).select(score.and(owner_user.select(badges_of(db)).opt())).fold((0i64, 0i64), |(n, s), (x, _)| (n + 1, s + x));
    let holders = db.post.group_by(&name).select(owner_user.select(badges_of(db)).select(&db.badge.user_id)).count_distinct();
    let mut v = Vec::new();
    main.and((&holders).opt()).drive(|k, ((n, s), u)| v.push((k, n, s, u.unwrap_or(0))));
    rows(v.iter().map(|&(k, n, s, u)| row(vec![V::S(k), V::I(n), avg(s, n), V::I(u)])))
}

// Users LEFT JOIN Posts GROUP BY u.DisplayName: COUNT(p.Id), the questions,
// the answers, SUM(COALESCE(p.Score, 0)), SUM(COALESCE(p.ViewCount, 0)).
fn by_name(db: &'static So) -> Vec<(Str, [i64; 5])> {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0)],
            None => a,
        })
        .drive(|k, a| v.push((k, a)));
    v
}

fn top_names(db: &'static So, by: usize, cols: &[usize]) -> String {
    let mut v = by_name(db);
    v.sort_by_key(|x| Reverse(x.1[by]));
    rows(v.iter().take(10).map(|(k, a)| {
        let mut f = vec![V::S(k)];
        f.extend(cols.iter().map(|&i| V::I(a[i])));
        row(f)
    }))
}

// SELECT
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.DisplayName
// ORDER BY
// TotalScore DESC
// LIMIT 10;
fn q15988(db: &'static So) -> String {
    top_names(db, 3, &[0, 3])
}

// SELECT
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q15825(db: &'static So) -> String {
    top_names(db, 0, &[0, 4])
}

// SELECT
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q15528(db: &'static So) -> String {
    top_names(db, 0, &[0, 3])
}

// SELECT u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.DisplayName
// ORDER BY PostCount DESC
// LIMIT 10;
fn q15077(db: &'static So) -> String {
    top_names(db, 0, &[0, 2])
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q15045(db: &'static So) -> String {
    top_names(db, 0, &[0, 1, 2])
}

// SELECT
// U.DisplayName AS UserName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q15266(db: &'static So) -> String {
    top_names(db, 0, &[0, 1, 2])
}

// SELECT
// u.DisplayName AS UserDisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q16874(db: &'static So) -> String {
    top_names(db, 0, &[0, 1, 2])
}

// SELECT
// Users.DisplayName,
// COUNT(Posts.Id) AS PostCount,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q17953(db: &'static So) -> String {
    top_names(db, 0, &[0, 1, 2])
}

// SELECT u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.DisplayName
// ORDER BY PostCount DESC
// LIMIT 10;
fn q16943(db: &'static So) -> String {
    let mut v = Vec::new();
    db.user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold((0i64, 0i64, 0i64), |(n, u, d), p| match p {
            Some(t) => (n + 1, u + (t == Some(2)) as i64, d + (t == Some(3)) as i64),
            None => (n, u, d),
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|x| Reverse(x.1.0));
    rows(v.iter().take(10).map(|&(k, (n, u, d))| row(vec![V::S(k), V::I(n), V::I(u), V::I(d)])))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// COALESCE(SUM(P.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(P.Score), 0) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// TotalScore DESC, PostCount DESC;
fn q11602(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt())).opt())
        .fold((0i64, 0i64, 0i64), |(n, w, s), p| match p {
            Some((x, vw)) => (n + 1, w + vw.unwrap_or(0), s + x),
            None => (n, w, s),
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, (n, w, s))| row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(n), V::I(w), V::I(s)])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("12855", q12855),
    ("13648", q13648),
    ("13871", q13871),
    ("10365", q10365),
    ("12360", q12360),
    ("14099", q14099),
    ("13588", q13588),
    ("10706", q10706),
    ("11022", q11022),
    ("10892", q10892),
    ("10819", q10819),
    ("10350", q10350),
    ("14649", q14649),
    ("14771", q14771),
    ("13211", q13211),
    ("11319", q11319),
    ("14214", q14214),
    ("13558", q13558),
    ("11490", q11490),
    ("12437", q12437),
    ("10689", q10689),
    ("13584", q13584),
    ("14893", q14893),
    ("10797", q10797),
    ("12002", q12002),
    ("11102", q11102),
    ("10826", q10826),
    ("12347", q12347),
    ("14879", q14879),
    ("15417", q15417),
    ("18042", q18042),
    ("15303", q15303),
    ("17946", q17946),
    ("18054", q18054),
    ("17196", q17196),
    ("11229", q11229),
    ("14297", q14297),
    ("13497", q13497),
    ("12894", q12894),
    ("12462", q12462),
    ("15988", q15988),
    ("15825", q15825),
    ("15528", q15528),
    ("15077", q15077),
    ("15045", q15045),
    ("15266", q15266),
    ("16874", q16874),
    ("17953", q17953),
    ("16943", q16943),
    ("11602", q11602),
];
