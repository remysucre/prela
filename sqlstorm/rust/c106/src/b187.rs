use harness::prelude::*;
use std::cmp::Reverse;

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(a.AcceptedAnswerId, 0) AS AcceptedAnswerId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.AcceptedAnswerId WHERE p.PostTypeId = 1),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, MAX(u.CreationDate) AS LastActive
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName),
// CloseReasonCounts AS (SELECT ph.PostId, COUNT(*) AS CloseReasonCount, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons
//     FROM PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment = cr.Id::TEXT WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT up.DisplayName, up.QuestionCount, up.TotalBounty, rp.Title, rp.CreationDate AS PostCreationDate, rp.ViewCount, COALESCE(cr.CloseReasonCount, 0) AS CloseReasonCount,
//        COALESCE(cr.CloseReasons, 'No reasons') AS CloseReasons
// FROM UserActivity up LEFT JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId AND rp.PostRank = 1 LEFT JOIN CloseReasonCounts cr ON rp.PostId = cr.PostId
// WHERE up.QuestionCount > 0 ORDER BY up.TotalBounty DESC, rp.ViewCount DESC LIMIT 5;
//
// PostRank breaks Score ties by post id (the SQL leaves them open). STRING_AGG(DISTINCT) is given in name order.
fn q20434(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, view_count, .. } = &db.post;
    let accepted_by: HashIdx<Id<Post>, Id<Post>> = (&db.post.accepted_answer).inv().collect();
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.and(accepted_by.opt())));
    let top = top_per(v, |&(_, (u, _))| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tv = rel(top.into_iter().map(|(p, (u, _))| (u, p)).collect());
    type T = (Id<User>, Id<Post>);
    let rp: HashIdx<Id<User>, T> = (&tv).map(|x: T| x.0).inv().select(&tv).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bv = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt());
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(bv.opt()).opt()).fold(0i64, |n, b| n + b.flatten().flatten().unwrap_or(0));
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let by_text: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i| &*Box::leak(i.to_string().into_boxed_str())).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(comment.select(&by_text)).buf_fold(|it| {
        let mut n: Vec<Str> = it.into_iter().collect();
        let c = n.len() as i64;
        n.sort();
        n.dedup();
        (c, &*Box::leak(n.join(", ").into_boxed_str()))
    });
    let v = drain((&qc).filt(|n| n > 0).and(&ua).and((&rp).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.1).select(&cr).opt())).opt()));
    let v = top_n(v, |&(u, ((_, b), r))| {
        let w = r.and_then(|(t, _)| view_count.get(t.1));
        (Reverse(b), w.is_none(), Reverse(w), u)
    }, 5);
    rows(v.into_iter().map(|(u, ((n, b), r))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(n), V::I(b)]);
        match r {
            Some(((_, p), c)) => {
                f.extend(post_fields(db, p, &["title", "created", "views"]));
                let (k, s) = c.unwrap_or((0, "No reasons"));
                f.extend([V::I(k), V::S(s)]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::I(0), V::S("No reasons")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerName, p.CreationDate, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) as RowNum
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, u.DisplayName, p.Title, p.Body, p.Tags, p.CreationDate),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.OwnerName, rp.CreationDate, rp.CommentCount, rp.Upvotes, rp.Downvotes, (rp.Upvotes - rp.Downvotes) AS NetVotes
//     FROM RankedPosts rp WHERE rp.RowNum = 1),
// TaggedPosts AS (SELECT fp.PostId, fp.Title, fp.Body, fp.Tags, fp.OwnerName, fp.CreationDate, fp.CommentCount, fp.Upvotes, fp.Downvotes, fp.NetVotes,
//        unnest(string_to_array(fp.Tags, ',')) AS TagArray FROM FilteredPosts fp)
// SELECT tp.TagArray AS Tag, COUNT(tp.PostId) AS PostCount, AVG(tp.Upvotes) AS AvgUpvotes, AVG(tp.Downvotes) AS AvgDownvotes, AVG(tp.CommentCount) AS AvgComments,
//        AVG(tp.NetVotes) AS AvgNetVotes
// FROM TaggedPosts tp GROUP BY tp.TagArray ORDER BY PostCount DESC LIMIT 10;
//
// RowNum partitions by the post itself, so it is always 1.
fn q28201(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    type R = (Id<Post>, ([i64; 3], Str));
    let tp = rel(drain((&rp).and(tags_str.flat_map(|t: Str| t.split(',')))));
    let g = (&tp).group_by(Same::<R>::new().map(|x: R| x.1 .1)).select(Same::<R>::new()).fold([0i64; 4], |s, (_, (a, _)): R| [s[0] + 1, s[1] + a[1], s[2] + a[2], s[3] + a[0]]);
    let v = top_n(drain(&g), |&(t, s)| (Reverse(s[0]), t), 10);
    rows(v.into_iter().map(|(t, s)| row(vec![V::S(t), V::I(s[0]), avg(s[1], s[0]), avg(s[2], s[0]), avg(s[3], s[0]), avg(s[1] - s[2], s[0])])))
}

// WITH UserReputation AS (SELECT Id, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetScore
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.ViewCount, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount, STRING_AGG(COALESCE(ct.Name, 'Unknown'), ', ') AS CloseReasons
//     FROM PostHistory ph LEFT JOIN CloseReasonTypes ct ON CAST(ph.Comment AS INT) = ct.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PopularPosts AS (SELECT pd.PostId, pd.Title, pd.OwnerDisplayName, pd.ViewCount, pd.NetScore, cp.CloseCount, cp.CloseReasons,
//        ROW_NUMBER() OVER (ORDER BY pd.NetScore DESC, pd.ViewCount DESC) AS PopularityRank
//     FROM PostDetails pd LEFT JOIN ClosedPosts cp ON pd.PostId = cp.PostId WHERE pd.NetScore > 0)
// SELECT up.Reputation, up.ReputationRank, pp.Title, pp.OwnerDisplayName, pp.ViewCount, pp.NetScore, pp.CloseCount, pp.CloseReasons
// FROM UserReputation up JOIN PopularPosts pp ON up.Id = (SELECT OwnerUserId FROM Posts WHERE Id = pp.PostId)
// WHERE up.ReputationRank <= 10 ORDER BY up.ReputationRank, pp.NetScore DESC;
//
// The subquery looks a post up by its primary key, so it is the post's owner. The ten ranked users are picked first and only their questions are voted.
// STRING_AGG is given in history id order (the SQL leaves it open).
fn q1803(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    type U = ((Id<User>, i64), i64);
    let tu = rel(drain(rel(r).filt(|(_, k): U| k <= 10)).into_iter().map(|x| x.1).collect());
    let uk: HashIdx<Id<User>, U> = (&tu).map(|x: U| x.0 .0).inv().select(&tu).collect();
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let pd = (&uk)
        .map(|x: U| x.0 .0)
        .select(qs())
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt()))
        .buf_fold(|it| {
            let mut n: Vec<(Id<PostHistory>, Option<Str>)> = it.into_iter().collect();
            n.sort();
            let s: Vec<Str> = n.iter().map(|x| x.1.unwrap_or("Unknown")).collect();
            (n.len() as i64, &*Box::leak(s.join(", ").into_boxed_str()))
        });
    let v = drain((&pd).filt(|n| n > 0).and((&db.post.owner_user).select(&uk)).and((&cp).opt()));
    rows(v.into_iter().map(|(p, ((n, ((u, r), k)), c))| {
        let mut f = vec![V::I(r), V::I(k)];
        f.extend(post_fields(db, p, &["title"]));
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["views"]));
        f.push(V::I(n));
        f.extend(match c {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

/// SQL `s LIKE pat`: `%` matches any run of characters, `_` exactly one; no escape character.
fn like(s: &str, pat: &str) -> bool {
    let (s, p): (Vec<char>, Vec<char>) = (s.chars().collect(), pat.chars().collect());
    let (mut i, mut j, mut star, mut mark) = (0, 0, usize::MAX, 0);
    while i < s.len() {
        if j < p.len() && (p[j] == '_' || (p[j] != '%' && p[j] == s[i])) {
            i += 1;
            j += 1;
        } else if j < p.len() && p[j] == '%' {
            star = j;
            mark = i;
            j += 1;
        } else if star != usize::MAX {
            j = star + 1;
            mark += 1;
            i = mark;
        } else {
            return false;
        }
    }
    while j < p.len() && p[j] == '%' {
        j += 1;
    }
    j == p.len()
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT b.Id) AS BadgeCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.ViewCount DESC) AS ViewRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, u.DisplayName),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.BadgeCount,
//        CASE WHEN rp.ViewCount > 100 THEN 'High Traffic' WHEN rp.ViewCount > 50 THEN 'Medium Traffic' ELSE 'Low Traffic' END AS TrafficCategory
//     FROM RankedPosts rp WHERE rp.CommentCount > 5)
// SELECT fp.PostId, fp.Title, fp.Body, fp.Tags, fp.CreationDate, fp.ViewCount, fp.OwnerDisplayName, fp.CommentCount, fp.BadgeCount, fp.TrafficCategory,
//        STRING_AGG(t.TagName, ', ') AS AssociatedTags
// FROM FilteredPosts fp LEFT JOIN Tags t ON t.Count > 3 AND POSITION(fp.Tags IN t.TagName) > 0
// GROUP BY fp.PostId, fp.Title, fp.Body, fp.Tags, fp.CreationDate, fp.ViewCount, fp.OwnerDisplayName, fp.CommentCount, fp.BadgeCount, fp.TrafficCategory
// ORDER BY fp.ViewCount DESC, fp.TrafficCategory DESC;
//
// COUNT(c.Id) counts comment x badge rows. The POSITION join goes through the distinct Tags strings; STRING_AGG is given in tag name order.
fn q27631(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, view_count, .. } = &db.post;
    let cc = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(owner_user.select(badges_of(db)).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let fp = rel(drain((&cc).filt(|n| n > 5)));
    type F = (Id<Post>, i64);
    let strs: MatSet<Str> = (&fp).map(|x: F| x.0).select(tags_str).collect();
    let tn: HashIdx<Str, Str> = db.tag.with((&db.tag.count).gt(3)).select(&db.tag.tag_name).inv().select(&db.tag.tag_name).collect();
    let hit: HashIdx<Str, Str> = (&strs).select_where(&tn, |s: Str, n: Str| n.contains(s)).collect();
    let at = (&fp).map(|x: F| x.0).group_by(Ident::<Post>::new()).select(tags_str.select(&hit)).buf_fold(|it| {
        let mut n: Vec<Str> = it.into_iter().collect();
        n.sort();
        &*Box::leak(n.join(", ").into_boxed_str())
    });
    let v = drain((&fp).select(Same::<F>::new().and(Same::<F>::new().map(|x: F| x.0).select(owner_user.select(Ident::<User>::new().and(&bc)).opt().and((&at).opt())))));
    rows(v.into_iter().map(|(_, ((p, c), (u, t)))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "views"]);
        f.push(u.map_or(V::Null, |(u, _)| user_col(db, u, "name")));
        f.extend([V::I(c), V::I(u.map_or(0, |x| x.1))]);
        let w = view_count.get(p).unwrap_or(0);
        f.push(V::S(if w > 100 { "High Traffic" } else if w > 50 { "Medium Traffic" } else { "Low Traffic" }));
        f.push(ostr(t));
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT Id, Reputation, CreationDate, DisplayName, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS Author, p.Score, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.ViewCount, p.Score),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(p.Id) > 10),
// PostHistoryStats AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEditedDate, STRING_AGG(DISTINCT pp.Name, ', ') AS PostHistoryTypes
//     FROM PostHistory ph JOIN PostHistoryTypes pp ON pp.Id = ph.PostHistoryTypeId GROUP BY ph.PostId),
// TopPosts AS (SELECT rp.*, pt.PostHistoryTypes, r.Id AS UserId FROM RecentPosts rp JOIN PostHistoryStats pt ON pt.PostId = rp.PostId JOIN Votes v ON v.PostId = rp.PostId
//     JOIN Users r ON r.Id = v.UserId WHERE v.VoteTypeId = 2)
// SELECT up.DisplayName AS UserName, up.Reputation, tp.Title, tp.ViewCount, tp.CommentCount, tp.Score, pts.LastEditedDate, pts.PostHistoryTypes, tt.TagName
// FROM UserReputation up JOIN TopPosts tp ON tp.Author = up.DisplayName JOIN PopularTags tt ON tp.Title LIKE '%' || tt.TagName || '%'
// JOIN PostHistoryStats pts ON pts.PostId = tp.PostId WHERE up.Rank <= 10 ORDER BY up.Reputation DESC, tp.Score DESC LIMIT 10;
//
// No CTE refers to itself. Rank breaks Reputation ties by user id (the SQL leaves them open). STRING_AGG(DISTINCT) is given in name order.
fn q30695(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, score, .. } = &db.post;
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(top);
    type U = (Id<User>, i64);
    let by_name: HashIdx<Str, U> = (&tu).map(|x: U| x.0).select(&db.user.display_name).inv().select(&tu).collect();
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let rp: MatSet<Id<Post>> = recent().with(owner_user.select(&db.user.display_name).select(&by_name)).collect();
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(hd.and(htype_name(db))).buf_fold(|it| {
        let v: Vec<(i64, Str)> = it.into_iter().collect();
        let m = v.iter().map(|x| x.0).max().unwrap();
        let mut n: Vec<Str> = v.into_iter().map(|x| x.1).collect();
        n.sort();
        n.dedup();
        (m, &*Box::leak(n.join(", ").into_boxed_str()))
    });
    let up2 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)).with(&db.vote.user));
    let ts_ = tag_stats(db);
    let pt: HashIdx<Str, Str> = db.tag.with((&ts_).filt(|a| a[0] > 10)).select(&db.tag.tag_name).inv().select(&db.tag.tag_name).collect();
    let titles: MatSet<Str> = (&rp).select(title).collect();
    let hit: HashIdx<Str, Str> = (&titles).select_where(&pt, |t: Str, n: Str| like(t, &format!("%{n}%"))).collect();
    let v = drain(
        (&rp).select(
            Ident::<Post>::new()
                .and(&cc)
                .and(&phs)
                .and(up2)
                .and(owner_user.select(&db.user.display_name).select(&by_name))
                .and(title.select(&hit))
                .and(&phs),
        ),
    );
    let v = top_n(v, |&(_, ((((((p, _), _), vt), (u, r)), n), _))| (Reverse(r), Reverse(score.get(p).unwrap()), u, p, vt, n), 10);
    rows(v.into_iter().map(|(_, ((((((p, c), _), _), (u, _)), n), (d, h)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(c)]);
        f.extend(post_fields(db, p, &["score"]));
        f.extend([V::T(d), V::S(h), V::S(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND p.Score > 0),
// UserEngagement AS (SELECT u.Id AS UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ue.UserId, ue.TotalVotes, ue.UpVotes, ue.DownVotes, ue.TotalBadges,
//        CASE WHEN ue.TotalVotes IS NULL THEN 'No votes' WHEN ue.UpVotes > ue.DownVotes THEN 'More Upvotes' ELSE 'More Downvotes' END AS VoteSummary
// FROM RankedPosts rp INNER JOIN UserEngagement ue ON ue.UserId = rp.PostId WHERE rp.Rank <= 5
// UNION ALL
// SELECT NULL, 'No Active Posts', NULL, NULL, NULL, ue.UserId, ue.TotalVotes, ue.UpVotes, ue.DownVotes, ue.TotalBadges,
//        CASE WHEN ue.TotalVotes IS NULL THEN 'No votes' WHEN ue.UpVotes > ue.DownVotes THEN 'More Upvotes' ELSE 'More Downvotes' END AS VoteSummary
// FROM UserEngagement ue WHERE NOT EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = ue.UserId)
// ORDER BY PostId DESC, UserId DESC;
//
// `ue.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. Rank breaks Score ties by post id (the SQL leaves them open).
fn q4440(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (t, b)| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + b.is_some() as i64]);
    let summary = |a: [i64; 4]| V::S(if a[1] > a[2] { "More Upvotes" } else { "More Downvotes" });
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1)).and(score.gt(0))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let a = drain((&tp).select((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ue))));
    let b = drain(db.user.minus(posts_of(db)).select(&ue));
    let mut out: Vec<String> = a
        .into_iter()
        .map(|(p, (u, e))| {
            let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
            f.push(user_col(db, u, "uid"));
            f.extend([V::I(e[0]), V::I(e[1]), V::I(e[2]), V::I(e[3]), summary(e)]);
            row(f)
        })
        .collect();
    out.extend(b.into_iter().map(|(u, e)| {
        let mut f = vec![V::Null, V::S("No Active Posts"), V::Null, V::Null, V::Null, user_col(db, u, "uid")];
        f.extend([V::I(e[0]), V::I(e[1]), V::I(e[2]), V::I(e[3]), summary(e)]);
        row(f)
    }));
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, ARRAY_AGG(DISTINCT t.TagName) AS Tags,
//        RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Tags t ON t.ExcerptPostId = p.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.Tags, rp.Rank FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostHistoryInfo AS (SELECT ph.PostId, ph.UserDisplayName, ph.CreationDate AS ChangeDate, ph.PostHistoryTypeId, pht.Name AS ChangeType, ph.Comment AS CloseReason
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.PostId IN (SELECT PostId FROM TopRankedPosts) AND ph.PostHistoryTypeId IN (10, 11))
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.CommentCount, trp.Tags,
//        ARRAY_AGG(DISTINCT phi.UserDisplayName || ' - ' || phi.ChangeType || ' on ' || CAST(phi.ChangeDate AS DATE) || COALESCE(' (Reason: ' || phi.CloseReason || ')', '')) AS History
// FROM TopRankedPosts trp LEFT JOIN PostHistoryInfo phi ON trp.PostId = phi.PostId
// GROUP BY trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.CommentCount, trp.Tags ORDER BY trp.Score DESC, trp.ViewCount DESC;
//
// Rank reads only base columns, so the ten posts are picked first. ARRAY_AGG(DISTINCT) is sorted, with a NULL element last.
fn q28773(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let r = ranked(drain(db.post.with(post_type_id.eq(1)).select(score.and(view_count.opt()))), |&(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), false);
    type R = ((Id<Post>, (i64, Option<i64>)), i64);
    let tp: MatSet<Id<Post>> = rel(r).filt(|(_, k): R| k <= 10).map(|x: R| x.0 .0).collect();
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let agg = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(excerpt.select(&db.tag.tag_name).opt())).buf_fold(|it| {
        let v: Vec<(Option<Id<Comment>>, Option<Str>)> = it.into_iter().collect();
        let n = v.iter().filter(|x| x.0.is_some()).count() as i64;
        let mut t: Vec<Option<Str>> = v.into_iter().map(|x| x.1).collect();
        t.sort_by_key(|x| (x.is_none(), *x));
        t.dedup();
        (n, &*Box::leak(t.into_boxed_slice()))
    });
    let PostHistory { post_history_type_id, user_display_name, creation_date: hd, comment, .. } = &db.post_history;
    let phi = history_of(db)
        .select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11])))
        .select(user_display_name.opt().and(htype_name(db)).and(hd).and(comment.opt()));
    let hist = (&tp).group_by(Ident::<Post>::new()).select(phi.opt()).buf_fold(|it| {
        let mut t: Vec<Option<String>> = it
            .into_iter()
            .map(|h| {
                h.and_then(|(((u, n), d), c)| {
                    u.map(|u| format!("{u} - {n} on {}{}", fmt_date(d), c.map_or(String::new(), |c| format!(" (Reason: {c})"))))
                })
            })
            .collect();
        t.sort_by(|a, b| (a.is_none(), a).cmp(&(b.is_none(), b)));
        t.dedup();
        &*Box::leak(t.into_boxed_slice())
    });
    let v = drain((&agg).and(&hist));
    rows(v.into_iter().map(|(p, ((n, t), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(n));
        f.push(V::L(t.iter().map(|x| ostr(*x)).collect()));
        f.push(V::L(h.iter().map(|x| x.as_ref().map_or(V::Null, |s| V::Owned(s.clone()))).collect()));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.ParentId, 1 AS PostLevel FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p2.Id, p2.Title, p2.ParentId, ph.PostLevel + 1 FROM Posts p2 INNER JOIN PostHierarchy ph ON p2.ParentId = ph.PostId),
// MostVotedPosts AS (SELECT p.Id, p.Title, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END), 0) AS NetVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// ClosestPosts AS (SELECT p.Id, p.Title, p.AcceptedAnswerId, ph.PostLevel, ph.Title AS ParentPostTitle FROM Posts p LEFT JOIN PostHierarchy ph ON p.Id = ph.PostId WHERE ph.PostLevel = 1)
// SELECT cp.Title AS QuestionTitle, cp.ParentPostTitle AS ParentOfQuestion,
//        CASE WHEN cp.AcceptedAnswerId IS NOT NULL THEN (SELECT Title FROM Posts WHERE Id = cp.AcceptedAnswerId) ELSE 'No accepted answer' END AS AcceptedAnswerTitle,
//        mvp.Title AS MostVotedAnswer, mvp.NetVotes, ur.DisplayName, ur.Reputation, ur.ReputationRank
// FROM ClosestPosts cp LEFT JOIN MostVotedPosts mvp ON mvp.Id = cp.AcceptedAnswerId JOIN UserReputation ur ON ur.UserId = cp.AcceptedAnswerId
// ORDER BY cp.Title, ur.Reputation DESC;
//
// Only PostLevel = 1 is read, which is the base case: the questions themselves, each its own ParentPostTitle. `ur.UserId = cp.AcceptedAnswerId`
// joins a user id to a post id, so it goes through the raw ids.
fn q34933(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, accepted_answer_id, title, .. } = &db.post;
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    type U = ((Id<User>, i64), i64);
    let ur = rel(r);
    let uidx: HashIdx<i64, U> = (&ur).map(|x: U| x.0 .0).select(&db.user.origid).inv().select(&ur).collect();
    let mvp = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let v = drain(db.post.with(post_type_id.eq(1)).select(accepted_answer.select(Ident::<Post>::new().and(&mvp)).opt().and(accepted_answer_id.select(&uidx))));
    rows(v.into_iter().map(|(p, (m, ((u, rep), k)))| {
        let t = title.get(p);
        let mut f = vec![ostr(t), ostr(t), ostr(m.and_then(|(a, _)| title.get(a)))];
        f.extend(match m {
            Some((a, n)) => [ostr(title.get(a)), V::I(n)],
            None => [V::Null, V::Null],
        });
        f.extend([user_col(db, u, "name"), V::I(rep), V::I(k)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("20434", q20434),
    ("28201", q28201),
    ("1803", q1803),
    ("27631", q27631),
    ("30695", q30695),
    ("4440", q4440),
    ("28773", q28773),
    ("34933", q34933),
];
