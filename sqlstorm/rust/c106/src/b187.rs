use harness::prelude::*;
use std::cmp::Reverse;

fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|(a, _)| a).inv().map(|(_, b): (A, B)| b).collect()
}

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
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(score).and(accepted_by.opt())).window(row_number, |((p, s), _)| (Reverse(s), p), asc);
    let rp: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|(((p, _), _), _)| p).collect();
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
    let v = drain((&qc).filt(|n| n > 0).and(&ua).and((&rp).select(Ident::<Post>::new().and((&cr).opt())).opt()));
    let v = top_n(v, |&(u, ((_, b), r))| {
        let w = r.and_then(|(p, _)| view_count.get(p));
        (Reverse(b), w.is_none(), Reverse(w), u)
    }, 5);
    rows(v.into_iter().map(|(u, ((n, b), r))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(n), V::I(b)]);
        match r {
            Some((p, c)) => {
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
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    type U = ((Id<User>, i64), i64);
    let tu: MatSet<U> = (&w).filt(|(_, k)| k <= 10).collect();
    let uk: HashIdx<Id<User>, U> = (&tu).map(|x: U| x.0 .0).inv().collect();
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let pd = (&tu)
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
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let pcount = db.tag.group_by(&db.tag.tag_name).select(&by_tag).fold(0i64, |n, _| n + 1);
    let pt: MatSet<Str> = (&pcount).filt(|n| n > 10).inv().collect();
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
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1)).and(score.gt(0))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
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
    let w = whole(db.post.with(post_type_id.eq(1))).select(Ident::<Post>::new().and(score).and(view_count.opt())).window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
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
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    type U = ((Id<User>, i64), i64);
    let ur: MatSet<U> = (&w).collect();
    let uidx: HashIdx<i64, U> = (&ur).map(|x: U| x.0 .0).select(&db.user.origid).inv().collect();
    let mvp = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let v = drain(db.post.with(post_type_id.eq(1)).select(accepted_answer.select(Ident::<Post>::new().and(&mvp)).opt().and(accepted_answer_id.select(&uidx))));
    rows(v.into_iter().map(|(p, (m, ((u, rep), k)))| {
        let t = title.get(p);
        let mut f = vec![ostr(t), ostr(t), ostr(accepted_answer.get(p).and_then(|a| title.get(a)))];
        f.extend(match m {
            Some((a, n)) => [ostr(title.get(a)), V::I(n)],
            None => [V::Null, V::Null],
        });
        f.extend([user_col(db, u, "name"), V::I(rep), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// ClosedPostReasons AS (SELECT ph.PostId, COUNT(*) AS CloseCount, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph
//     JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyReceived, RANK() OVER (ORDER BY SUM(COALESCE(v.BountyAmount, 0)) DESC) AS UserRank
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// FinalPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.AnswerCount, COALESCE(cpr.CloseCount, 0) AS CloseCount, COALESCE(cpr.CloseReasons, 'No Close Reasons') AS CloseReasons,
//        tu.DisplayName AS TopUser, tu.TotalBountyReceived
//     FROM RankedPosts rp LEFT JOIN ClosedPostReasons cpr ON rp.PostId = cpr.PostId LEFT JOIN TopUsers tu ON rp.Score > 0 AND tu.UserRank <= 10 WHERE rp.rn = 1)
// SELECT fp.PostId, fp.Title, fp.ViewCount, fp.Score, fp.AnswerCount, fp.CloseCount, fp.CloseReasons, COALESCE(fp.TopUser, 'No Top User') AS TopUser, fp.TotalBountyReceived
// FROM FinalPosts fp ORDER BY fp.Score DESC, fp.ViewCount DESC LIMIT 50;
//
// `ON rp.Score > 0 AND tu.UserRank <= 10` names no join column: posts with a positive score cross the top users, the rest get the NULL row.
// rn breaks CreationDate ties by post id (the SQL leaves them open). STRING_AGG is given in history id order.
fn q2238(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cpr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|it| {
            let mut n: Vec<(Id<PostHistory>, Str)> = it.into_iter().collect();
            n.sort();
            let s: Vec<Str> = n.iter().map(|x| x.1).collect();
            (n.len() as i64, &*Box::leak(s.join(", ").into_boxed_str()))
        });
    let tb = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let tw = whole(&tb).select(Ident::<User>::new().and(&tb)).window(rank, |(_, b)| Reverse(b), asc);
    let tu: HashIdx<(), (Id<User>, i64)> = (&tw).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let v = drain((&rp).select(Ident::<Post>::new().with(score.gt(0)).map(|_: Id<Post>| ()).select(&tu).opt().and((&cpr).opt())));
    let v = top_n(v, |&(p, (u, _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, u)
    }, 50);
    rows(v.into_iter().map(|(p, (u, c))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "answers"]);
        let (n, s) = c.unwrap_or((0, "No Close Reasons"));
        f.extend([V::I(n), V::S(s)]);
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "name"), V::I(b)],
            None => [V::S("No Top User"), V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts FROM Posts p GROUP BY p.OwnerUserId),
// TopUsers AS (SELECT u.Id, u.DisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        ps.TotalPosts, ps.Questions, ps.Answers, ps.PopularPosts, ROW_NUMBER() OVER (ORDER BY ps.TotalPosts DESC) AS Rank
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.PopularPosts, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges,
//        CASE WHEN tu.TotalPosts = 0 THEN 'No Posts' WHEN tu.TotalPosts > 100 THEN 'Super Contributor' ELSE 'Regular Contributor' END AS ContributorType,
//        STRING_AGG(p.Title, ', ') AS PopularPostTitles
// FROM TopUsers tu LEFT JOIN Posts p ON p.OwnerUserId = tu.Id AND p.ViewCount > 100
// GROUP BY tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.PopularPosts, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges
// HAVING COUNT(p.Id) = 0 ORDER BY tu.PopularPosts DESC;
//
// Rank is never read. The groups are keyed by the name and the counts, not the user, so users alike in all of them merge.
fn q23632(db: &'static So) -> String {
    let Post { post_type_id, view_count, title, .. } = &db.post;
    let ps = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt()))).fold([0i64; 4], |a, (t, w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.map_or(false, |w| w > 100) as i64]
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    type K = (Str, [i64; 7]);
    type R = (Id<User>, K);
    let tu: MatSet<R> = db
        .user
        .select(Ident::<User>::new().and(&db.user.display_name).and(&ps).and((&ub).opt()))
        .map(|(((u, n), a), b): (((Id<User>, Str), [i64; 4]), Option<[i64; 3]>)| {
            let b = b.unwrap_or([0; 3]);
            (u, (n, [a[0], a[1], a[2], a[3], b[0], b[1], b[2]]))
        })
        .collect();
    let popular = posts_of(db).select(Ident::<Post>::new().with(view_count.gt(100))).select(Ident::<Post>::new().and(title.opt()));
    let g = (&tu).group_by(Same::<R>::new().map(|x: R| x.1)).select(Same::<R>::new().map(|x: R| x.0).select(popular.opt())).buf_fold(|it| {
        let v: Vec<Option<(Id<Post>, Option<Str>)>> = it.into_iter().collect();
        let n = v.iter().filter(|x| x.is_some()).count() as i64;
        let mut t: Vec<(Id<Post>, Str)> = v.into_iter().flatten().filter_map(|(p, t)| t.map(|t| (p, t))).collect();
        t.sort();
        let s: Vec<Str> = t.into_iter().map(|x| x.1).collect();
        (n, if s.is_empty() { None } else { Some(&*Box::leak(s.join(", ").into_boxed_str())) })
    });
    let v = drain((&g).filt(|(n, _)| n == 0));
    rows(v.into_iter().map(|((name, a), (_, s))| {
        let mut f = vec![V::S(name)];
        f.extend(a.iter().map(|&x| V::I(x)));
        f.push(V::S(if a[0] == 0 { "No Posts" } else if a[0] > 100 { "Super Contributor" } else { "Regular Contributor" }));
        f.push(ostr(s));
        row(f)
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title AS PostTitle, p.ParentId, 0 AS Level FROM Posts p WHERE p.ParentId IS NULL
//     UNION ALL SELECT p.Id AS PostId, p.Title AS PostTitle, p.ParentId, ph.Level + 1 FROM Posts p INNER JOIN PostHierarchy ph ON p.ParentId = ph.PostId),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, COUNT(DISTINCT c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBounty, COUNT(DISTINCT v.Id) AS VoteCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY SUM(v.BountyAmount) DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopPosts AS (SELECT pa.PostId, pa.Title, pa.CommentCount, pa.VoteCount, pa.TotalBounty FROM PostActivity pa WHERE pa.Rank <= 10),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COALESCE(SUM(pb.TotalBounty), 0) AS BountiesAwarded, COALESCE(SUM(ub.BadgeCount), 0) AS TotalBadges,
//        MAX(p.CreationDate) AS LastPostDate
// FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN TopPosts pb ON p.Id = pb.PostId LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE u.Reputation > 1000 GROUP BY u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5 ORDER BY TotalPosts DESC, BountiesAwarded DESC;
//
// PostHierarchy is never read. Rank partitions by owner, so only the posts of users above 1000 reputation are ranked; its ties carry equal bounties.
// SUM(v.BountyAmount) runs over the comment x vote rows.
fn q30141(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let ps: MatSet<Id<Post>> = users().select(posts_of(db)).collect();
    let pa = (&ps)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold(None::<i64>, |s, (_, b)| match b.flatten() {
            Some(b) => Some(s.unwrap_or(0) + b),
            None => s,
        });
    let w = (&ps).group_by(owner_user).select(Ident::<Post>::new().and(&pa)).window(row_number, |(p, b)| (b.is_none(), Reverse(b), p), asc);
    let tp: MatSet<(Id<Post>, Option<i64>)> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let pb = by_first(&tp);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let g = users()
        .group_by(&db.user.display_name)
        .select((&ub).and(posts_of(db).select(creation_date.and((&pb).opt())).opt()))
        .fold([0i64, 0, 0, i64::MIN], |a, (b, p)| match p {
            Some((d, t)) => [a[0] + 1, a[1] + t.flatten().unwrap_or(0), a[2] + b, a[3].max(d)],
            None => [a[0], a[1], a[2] + b, a[3]],
        });
    let v = drain((&g).filt(|a| a[0] > 5));
    rows(v.into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), tmax(a[3])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, p.Score, COALESCE(COUNT(DISTINCT c.Id), 0) AS TotalComments,
//        ARRAY_AGG(DISTINCT t.TagName) AS Tags, RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Tags t ON t.ExcerptPostId = p.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, p.Score),
// CloseReasons AS (SELECT ph.PostId, STRING_AGG(DISTINCT crt.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes crt ON CAST(ph.Comment AS int) = crt.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation FROM Users u WHERE u.Reputation > 500)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.TotalComments, rp.Tags, cr.CloseReasons, ur.DisplayName AS MostActiveUser,
//        ur.Reputation AS UserReputation
// FROM RankedPosts rp JOIN CloseReasons cr ON rp.PostId = cr.PostId
// JOIN (SELECT p.OwnerUserId, COUNT(*) AS PostCount FROM Posts p WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId ORDER BY PostCount DESC LIMIT 1) AS ActiveUser
//     ON ActiveUser.OwnerUserId = cr.PostId
// JOIN UserReputation ur ON ur.UserId = ActiveUser.OwnerUserId WHERE rp.Rank <= 10 ORDER BY rp.Rank;
//
// Rank reads only base columns, so the ten posts are picked first. `ActiveUser.OwnerUserId = cr.PostId` joins a user id to a post id, so it goes
// through the raw ids. The NULL owner is a group of its own. ARRAY_AGG(DISTINCT) and STRING_AGG(DISTINCT) are sorted.
fn q25963(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user_id, origid, .. } = &db.post;
    let w = whole(db.post.with(post_type_id.eq(1))).select(Ident::<Post>::new().and(score).and(creation_date)).window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    type P = (Id<Post>, i64);
    let rv: MatSet<P> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), k)| (p, k)).collect();
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tc = (&rv).map(|x: P| x.0).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tg = (&rv).map(|x: P| x.0).group_by(Ident::<Post>::new()).select(excerpt.select(&db.tag.tag_name).opt()).buf_fold(|it| {
        let mut t: Vec<Option<Str>> = it.into_iter().collect();
        t.sort_by_key(|x| (x.is_none(), *x));
        t.dedup();
        &*Box::leak(t.into_boxed_slice())
    });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)).buf_fold(|it| {
        let mut n: Vec<Str> = it.into_iter().collect();
        n.sort();
        n.dedup();
        &*Box::leak(n.join(", ").into_boxed_str())
    });
    let pc = db.post.with(post_type_id.eq(1)).select(owner_user_id.opt()).group_by(Same::<Option<i64>>::new()).select(Same::<Option<i64>>::new()).fold(0i64, |n, _| n + 1);
    let au = top_n(drain(&pc), |&(o, n)| (Reverse(n), o), 1);
    let av = rel(au.into_iter().map(|x| x.0).collect());
    let aidx: HashIdx<Option<i64>, Option<i64>> = (&av).map(|x: Option<i64>| x).inv().select(&av).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ur = Ident::<User>::new().with((&db.user.reputation).gt(500));
    let v = drain((&rv).select(Same::<P>::new().and(Same::<P>::new().map(|x: P| x.0).select((&tc).and(&tg).and(&cr).and(origid.map(|x: i64| Some(x)).select(&aidx).flat_map(|o: Option<i64>| o).select(&uidx).select(ur))))));
    rows(v.into_iter().map(|(_, ((p, _), (((n, t), c), u)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers", "comments"]);
        f.push(V::I(n));
        f.push(V::L(t.iter().map(|x| ostr(*x)).collect()));
        f.push(V::S(c));
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, 1 AS Level FROM Users u WHERE u.Reputation > 1000
//     UNION ALL SELECT u.Id, u.DisplayName, u.Reputation, ur.Level + 1 FROM Users u JOIN UserReputationCTE ur ON u.Id = ur.UserId WHERE u.Reputation > ur.Reputation),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND p.ViewCount > 500),
// UserVotes AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS Downvotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(DISTINCT ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate, MAX(ph.UserId) AS LastEditorId FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.PostId)
// SELECT u.DisplayName AS UserName, u.Reputation, p.Title AS PostTitle, p.Score AS PostScore, ph.EditCount, ph.LastEditDate, uv.VoteCount, uv.Upvotes, uv.Downvotes
// FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId JOIN TopPosts tp ON p.Id = tp.PostId LEFT JOIN PostHistorySummary ph ON p.Id = ph.PostId LEFT JOIN UserVotes uv ON u.Id = uv.UserId
// WHERE u.Reputation > 1000 AND ph.EditCount > 0 AND EXISTS (SELECT 1 FROM UserReputationCTE ur WHERE ur.UserId = u.Id) ORDER BY u.Reputation DESC, p.Score DESC;
//
// The recursive step joins a user to itself and asks for a reputation above its own, so it never adds a row: the CTE is its base case. rn is never read.
fn q34669(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, .. } = &db.post;
    let ur = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vtype_name(db))).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain(
        db.post
            .with(creation_date.ge(add_years(current_date(), -1)).and(view_count.gt(500)))
            .select(owner_user.select(ur.and((&uv).opt())).and((&ph).filt(|(n, _)| n > 0))),
    );
    rows(v.into_iter().map(|(p, ((u, a), (n, d)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(n), V::T(d)]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS RankByView, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.Tags, rp.ViewCount, rp.Score, rp.CreationDate, u.DisplayName AS AuthorName, COALESCE(badge_count.BadgeCount, 0) AS BadgeCount,
//        COALESCE(comment_count.CommentCount, 0) AS CommentCount, rp.RankByScore, rp.RankByView
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) badge_count ON u.Id = badge_count.UserId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) comment_count ON rp.PostId = comment_count.PostId)
// SELECT pd.PostId, pd.Title, pd.Tags, pd.ViewCount, pd.Score, pd.CreationDate, pd.AuthorName, pd.BadgeCount, pd.CommentCount, STRING_AGG(pt.Name, ', ') AS PostTypeNames
// FROM PostDetails pd JOIN PostTypes pt ON pd.PostId = pt.Id WHERE pd.RankByScore <= 5 OR pd.RankByView <= 5
// GROUP BY pd.PostId, pd.Title, pd.Tags, pd.ViewCount, pd.Score, pd.CreationDate, pd.AuthorName, pd.BadgeCount, pd.CommentCount ORDER BY pd.Score DESC, pd.ViewCount DESC;
//
// `pd.PostId = pt.Id` joins a post id to a post type id, so it goes through the raw ids; a post joins one type at most, so STRING_AGG has one name.
// The ranks break ties by post id (the SQL leaves them open).
fn q26124(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, view_count, owner_user, origid, .. } = &db.post;
    type R = ((Id<Post>, i64), Option<i64>);
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), _)| (Reverse(s), p), asc);
    let w = (&w).window(row_number, |(((p, _), v), _): (R, i64)| (v.is_none(), Reverse(v), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|((_, a), b)| a <= 5 || b <= 5).map(|((((p, _), _), _), _)| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let ptidx: HashIdx<i64, Str> = (&db.post_type.origid).inv().select(&db.post_type.name).collect();
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and((&bc).opt())).and((&cc).opt()).and(origid.select(&ptidx))));
    rows(v.into_iter().map(|(p, (((u, b), c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "views", "score", "created"]);
        f.extend([user_col(db, u, "name"), V::I(b.unwrap_or(0)), V::I(c.unwrap_or(0)), V::S(n)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, CASE WHEN Reputation >= 10000 THEN 'Platinum' WHEN Reputation >= 1000 THEN 'Gold' WHEN Reputation >= 100 THEN 'Silver' ELSE 'Bronze' END AS Badge FROM Users),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, COUNT(ph.Id) AS CloseReasonCount, STRING_AGG(DISTINCT crt.Name, ', ') AS CloseReasons
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10 JOIN CloseReasonTypes crt ON CAST(ph.Comment AS INTEGER) = crt.Id
//     WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title),
// TopCloseReasons AS (SELECT CloseReasons, RANK() OVER (ORDER BY COUNT(*) DESC) AS ReasonRank FROM (SELECT CloseReasons FROM ClosedPosts) AS RankedReasons GROUP BY CloseReasons),
// PostWithMostComments AS (SELECT PostId, COUNT(*) AS TotalComments FROM Comments GROUP BY PostId ORDER BY TotalComments DESC LIMIT 1),
// UserVotesSummary AS (SELECT v.UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v GROUP BY v.UserId)
// SELECT u.DisplayName, u.CreationDate, ur.Badge, cp.Title AS ClosedPostTitle, cp.CloseReasonCount, cp.CloseReasons, p.TotalComments AS CommentsOnMostCommentedPost,
//        uvs.TotalVotes, uvs.UpVotes, uvs.DownVotes
// FROM Users u LEFT JOIN UserReputation ur ON u.Id = ur.Id LEFT JOIN ClosedPosts cp ON cp.PostId IN (SELECT PostId FROM PostWithMostComments)
// LEFT JOIN PostWithMostComments p ON p.PostId = cp.PostId LEFT JOIN UserVotesSummary uvs ON u.Id = uvs.UserId
// WHERE u.Reputation > 100 ORDER BY u.Reputation DESC;
//
// TopCloseReasons is never read. `ON cp.PostId IN (...)` names only ClosedPosts, so every user crosses the closed rows of the most commented post
// (or the NULL row). STRING_AGG(DISTINCT) is sorted.
fn q1882(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let pw = rel(top_n(drain(&cc), |&(p, n)| (Reverse(n), p), 1));
    type W = (Id<Post>, i64);
    let pwidx: HashIdx<Id<Post>, W> = (&pw).map(|x: W| x.0).inv().select(&pw).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post
        .with((&db.post.post_type_id).is_in([1, 2]))
        .with(&pwidx)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|it| {
            let mut n: Vec<Str> = it.into_iter().collect();
            let c = n.len() as i64;
            n.sort();
            n.dedup();
            (c, &*Box::leak(n.join(", ").into_boxed_str()))
        });
    let cpv: HashIdx<(), ((Id<Post>, (i64, Str)), Option<W>)> = whole(&cp).select(Ident::<Post>::new().and(&cp).and((&pwidx).opt())).collect();
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select((&uvs).opt().and(Ident::<User>::new().map(|_: Id<User>| ()).select(&cpv).opt())));
    rows(v.into_iter().map(|(u, (a, c))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "ucreated"]);
        f.push(V::S(if rep >= 10000 { "Platinum" } else if rep >= 1000 { "Gold" } else if rep >= 100 { "Silver" } else { "Bronze" }));
        match c {
            Some(((p, (n, s)), w)) => {
                f.extend(post_fields(db, p, &["title"]));
                f.extend([V::I(n), V::S(s), oint(w.map(|w| w.1))]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS (SELECT u.Id, u.DisplayName, u.Reputation, 1 AS Level FROM Users u WHERE u.Reputation > 1000
//     UNION ALL SELECT u.Id, u.DisplayName, u.Reputation, ur.Level + 1 FROM Users u INNER JOIN UserReputationCTE ur ON u.Reputation > ur.Reputation WHERE ur.Level < 5),
// PostsWithVoteCount AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.Id, p.Title, p.CreationDate),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, COUNT(*) AS CloseCount, STRING_AGG(DISTINCT c.Name, ', ') AS CloseReasons FROM PostHistory ph
//     INNER JOIN CloseReasonTypes c ON CAST(ph.Comment AS INTEGER) = c.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.CreationDate),
// TopPosts AS (SELECT pw.PostId, pw.Title, pw.CreationDate, pw.VoteCount, pw.UpVotes, pw.DownVotes, COALESCE(cp.CloseCount, 0) AS CloseCount,
//        COALESCE(cp.CloseReasons, 'No reasons') AS CloseReasons, u.DisplayName AS OwnerDisplayName
//     FROM PostsWithVoteCount pw LEFT JOIN ClosedPosts cp ON pw.PostId = cp.PostId INNER JOIN Posts p ON pw.PostId = p.Id INNER JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE u.Reputation > 5000)
// SELECT tp.*, CASE WHEN tp.VoteCount > 50 THEN 'Hot' WHEN tp.VoteCount BETWEEN 20 AND 50 THEN 'Trending' ELSE 'New' END AS PostStatus
// FROM TopPosts tp ORDER BY tp.VoteCount DESC, tp.CreationDate ASC LIMIT 10;
//
// UserReputationCTE is never read. STRING_AGG(DISTINCT) is sorted.
fn q30204(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let pw = db
        .post
        .with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(5000))))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| match t {
            Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(hd))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|it| {
            let mut n: Vec<Str> = it.into_iter().collect();
            let c = n.len() as i64;
            n.sort();
            n.dedup();
            (c, &*Box::leak(n.join(", ").into_boxed_str()))
        });
    type C = ((Id<Post>, i64), (i64, Str));
    let cv = rel(drain(&cp));
    let cidx: HashIdx<Id<Post>, C> = (&cv).map(|x: C| x.0 .0).inv().select(&cv).collect();
    let v = drain((&pw).and((&cidx).opt()).and(owner_user));
    let v = top_n(v, |&(p, ((a, c), _))| (Reverse(a[0]), creation_date.get(p).unwrap(), p, c.map(|c| c.0 .1)), 10);
    rows(v.into_iter().map(|(p, ((a, c), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        let (n, s) = c.map_or((0, "No reasons"), |c| c.1);
        f.extend([V::I(n), V::S(s), user_col(db, u, "name")]);
        f.push(V::S(if a[0] > 50 { "Hot" } else if a[0] >= 20 { "Trending" } else { "New" }));
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT Id, Reputation, 1 AS Level FROM Users WHERE Reputation > 1000
//     UNION ALL SELECT u.Id, u.Reputation, ur.Level + 1 FROM Users u INNER JOIN UserReputation ur ON u.Reputation > ur.Reputation AND ur.Level < 10),
// PostStats AS (SELECT p.Id AS PostId, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, MAX(p.CreationDate) AS LastActivity
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id),
// BadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY UserId),
// FinalResult AS (SELECT u.DisplayName, u.Reputation, COALESCE(bc.BadgeCount, 0) AS RecentBadgeCount, ps.CommentCount, ps.UpVotes, ps.DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY ps.LastActivity DESC) AS ActivityRank
//     FROM Users u LEFT JOIN BadgeCounts bc ON u.Id = bc.UserId LEFT JOIN PostStats ps ON u.Id = ps.CommentCount)
// SELECT fr.DisplayName, fr.Reputation, fr.RecentBadgeCount, fr.CommentCount, fr.UpVotes, fr.DownVotes,
//        CASE WHEN fr.Reputation > 5000 THEN 'Highly Reputable' WHEN fr.Reputation BETWEEN 2000 AND 5000 THEN 'Moderately Reputable' ELSE 'New Contributor' END AS ReputationCategory
// FROM FinalResult fr WHERE fr.ActivityRank = 1 ORDER BY fr.Reputation DESC LIMIT 50;
//
// UserReputation is never read. `u.Id = ps.CommentCount` joins a user id to a count, so it goes through the raw id. Each user keeps exactly one
// ActivityRank = 1 row, so the 50 users are picked first; ActivityRank breaks LastActivity ties by post id (the SQL leaves them open).
fn q30512(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ps = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    type P = (Id<Post>, ([i64; 3], i64));
    let pv: MatSet<P> = db.post.select(Ident::<Post>::new().and((&ps).and(creation_date))).collect();
    let pidx: HashIdx<i64, P> = (&pv).map(|x: P| x.1 .0[0]).inv().collect();
    let bc = db.badge.with((&db.badge.date).ge(add_years(t0, -1))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let tu: MatSet<Id<User>> = rel(top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 50).into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let w = (&tu)
        .group_by(Ident::<User>::new())
        .select(Ident::<User>::new().and((&bc).opt()).and((&db.user.origid).select(&pidx).opt()))
        .window(row_number, |(_, p)| p.map(|(p, (_, d))| (Reverse(d), p)), asc);
    let v = drain((&w).filt(|(_, r)| r == 1));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 50);
    rows(v.into_iter().map(|(_, (((u, b), p), _))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b.unwrap_or(0)));
        f.extend(match p {
            Some((_, (a, _))) => [V::I(a[0]), V::I(a[1]), V::I(a[2])],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if rep > 5000 { "Highly Reputable" } else if rep >= 2000 { "Moderately Reputable" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RECURSIVE UserPostActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(p.ViewCount) AS TotalViews,
//        SUM(p.Score) AS TotalScore, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY SUM(p.Score) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// RecentPostHistory AS (SELECT ph.UserId, ph.PostId, ph.CreationDate, p.Title, p.PostTypeId, php.Name AS HistoryType,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS RecentAction
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id JOIN PostHistoryTypes php ON ph.PostHistoryTypeId = php.Id
//     WHERE php.Name IN ('Post Closed', 'Post Reopened', 'Edit Title', 'Edit Body') AND ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// AggregatedPostLinks AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS TotalLinks, STRING_AGG(CONCAT(pl.RelatedPostId, ': ', l.Name), ', ') AS RelatedPosts
//     FROM PostLinks pl JOIN LinkTypes l ON pl.LinkTypeId = l.Id GROUP BY pl.PostId)
// SELECT upr.UserId, upr.DisplayName, upr.TotalPosts, upr.TotalComments, upr.TotalViews, upr.TotalScore, upr.Rank, rph.Title, rph.HistoryType, rph.CreationDate AS LastActionDate,
//        apl.TotalLinks, apl.RelatedPosts
// FROM UserPostActivity upr LEFT JOIN RecentPostHistory rph ON upr.UserId = rph.UserId LEFT JOIN AggregatedPostLinks apl ON rph.PostId = apl.PostId
// WHERE upr.Rank <= 10 ORDER BY upr.TotalScore DESC, rph.CreationDate DESC LIMIT 100;
//
// No CTE refers to itself. Rank partitions by the user, so it is always 1; RecentAction is never read. SUM(p.ViewCount) and SUM(p.Score) run over
// the post x comment rows. STRING_AGG is given in link id order.
fn q31568(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let upa = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(score).and(comments_of(db).opt())).opt())
        .fold((0i64, None::<i64>, None::<i64>), |(n, w, s), p| match p {
            Some(((v, sc), c)) => (n + c.is_some() as i64, match v { Some(v) => Some(w.unwrap_or(0) + v), None => w }, Some(s.unwrap_or(0) + sc)),
            None => (n, w, s),
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { creation_date: hd, user, .. } = &db.post_history;
    let rph: HashIdx<Id<User>, Id<PostHistory>> = db
        .post_history
        .with(htype_name(db).filt(|n: Str| ["Post Closed", "Post Reopened", "Edit Title", "Edit Body"].contains(&n)))
        .with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .select(user)
        .inv()
        .collect();
    let PostLink { post, related_post_id, link_type, .. } = &db.post_link;
    let apl = db.post_link.group_by(post).select(Ident::<PostLink>::new().and(related_post_id).and(link_type.select(&db.link_type.name))).buf_fold(|it| {
        let mut v: Vec<((Id<PostLink>, i64), Str)> = it.into_iter().collect();
        v.sort();
        let s: Vec<String> = v.iter().map(|((_, r), n)| format!("{r}: {n}")).collect();
        (v.len() as i64, &*Box::leak(s.join(", ").into_boxed_str()))
    });
    let hp = &db.post_history.post;
    let v = drain((&upa).and(&pc).and(rph.select(Ident::<PostHistory>::new().and(hp.select(&apl).opt())).opt()));
    let v = top_n(v, |&(u, ((a, _), h))| {
        let d = h.map(|(h, _)| hd.get(h).unwrap());
        (a.2.is_none(), Reverse(a.2), d.is_none(), Reverse(d), u, h.map(|x| x.0))
    }, 100);
    rows(v.into_iter().map(|(u, (((n, w, s), pc), h))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(pc), V::I(n), oint(w), oint(s), V::I(1)]);
        match h {
            Some((h, l)) => {
                f.extend(post_fields(db, hp.get(h).unwrap(), &["title"]));
                f.extend([V::S(htype_name(db).get(h).unwrap()), V::T(hd.get(h).unwrap())]);
                f.extend(match l {
                    Some((k, s)) => [V::I(k), V::S(s)],
                    None => [V::Null, V::Null],
                });
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT Id, Reputation, CreationDate, DisplayName, 1 AS Level FROM Users WHERE Reputation > 1000
//     UNION ALL SELECT u.Id, u.Reputation, u.CreationDate, u.DisplayName, ur.Level + 1 FROM Users u INNER JOIN UserReputation ur ON u.Reputation > ur.Reputation WHERE ur.Level < 5),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Tags, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Tags),
// HighScoringPosts AS (SELECT rp.*, (UpVotes - DownVotes) AS NetScore, ROW_NUMBER() OVER (ORDER BY (UpVotes - DownVotes) DESC) AS Rank FROM RecentPosts rp WHERE (UpVotes - DownVotes) > 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, pt.Name AS PostType, COALESCE(c.CommentCount, 0) AS TotalComments, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON p.OwnerUserId = b.UserId)
// SELECT dp.Title, dp.CreationDate, dp.PostType, dp.TotalComments, dp.BadgeCount, hs.UpVotes, hs.DownVotes, hs.NetScore
// FROM PostDetails dp JOIN HighScoringPosts hs ON dp.PostId = hs.PostId WHERE dp.BadgeCount > 0 ORDER BY hs.NetScore DESC LIMIT 10;
//
// UserReputation is never read, nor is Rank.
fn q33789(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(current_date(), -30)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&rp).filt(|a| a[0] - a[1] > 10).and(owner_user_id.select(&bc).filt(|n| n > 0)).and((&cc).opt()));
    let v = top_n(v, |&(p, ((a, _), _))| (Reverse(a[0] - a[1]), p), 10);
    rows(v.into_iter().map(|(p, ((a, b), c))| {
        let mut f = post_fields(db, p, &["title", "created", "type"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS ScoreRank
//     FROM Posts p WHERE p.PostTypeId IN (1, 2)),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, CASE WHEN rp.Score IS NULL THEN 'No Score' WHEN rp.Score < 0 THEN 'Under review' ELSE 'Popular Post' END AS PostCategory,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 2) AS UpVotes, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 3) AS DownVotes,
//        (SELECT STRING_AGG(c.Text, '; ') FROM Comments c WHERE c.PostId = rp.PostId) AS CommentSummaries
//     FROM RankedPosts rp WHERE rp.ScoreRank <= 5),
// FinalReport AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.Score, pd.PostCategory, pd.UpVotes, pd.DownVotes, COALESCE(pd.CommentSummaries, 'No comments') AS CommentSummaries,
//        COALESCE((SELECT STRING_AGG(pht.Name, ', ') FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
//                  WHERE ph.PostId = pd.PostId AND ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'), 'No recent edits') AS RecentEdits
//     FROM PostDetails pd)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.ViewCount, fr.Score, fr.PostCategory, fr.UpVotes, fr.DownVotes, fr.CommentSummaries, fr.RecentEdits
// FROM FinalReport fr WHERE fr.SCORE IS NOT NULL AND fr.PostCategory = 'Popular Post' ORDER BY fr.ViewCount DESC, fr.CreationDate DESC LIMIT 50;
//
// ScoreRank reads only base columns, so the posts are picked first. The STRING_AGGs are given in comment and history id order.
fn q24283(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.is_in([1, 2])).group_by(post_type_id).select(Ident::<Post>::new().and(score).and(creation_date)).window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().and(&db.comment.text))).buf_fold(|it| {
        let mut v: Vec<(Id<Comment>, Str)> = it.into_iter().collect();
        v.sort();
        let s: Vec<Str> = v.into_iter().map(|x| x.1).collect();
        &*Box::leak(s.join("; ").into_boxed_str())
    });
    let recent = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let re = (&tp).group_by(Ident::<Post>::new()).select(recent.select(Ident::<PostHistory>::new().and(htype_name(db)))).buf_fold(|it| {
        let mut v: Vec<(Id<PostHistory>, Str)> = it.into_iter().collect();
        v.sort();
        let s: Vec<Str> = v.into_iter().map(|x| x.1).collect();
        &*Box::leak(s.join(", ").into_boxed_str())
    });
    let v = drain(db.post.with(score.ge(0)).select(Ident::<Post>::new().with(&tp)).select((&vc).and((&cs).opt()).and((&re).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()), p)
    }, 50);
    rows(v.into_iter().map(|(p, ((a, c), e))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::S("Popular Post"), V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("No comments")), V::S(e.unwrap_or("No recent edits"))]);
        row(f)
    }))
}

/// `ph.Comment = CAST(cr.Id AS VARCHAR)`: the close reason names keyed by the text of their id.
fn reason_by_text(db: &'static So) -> HashIdx<Str, Str> {
    (&db.close_reason_type.origid).map(|i| &*Box::leak(i.to_string().into_boxed_str())).inv().select(&db.close_reason_type.name).collect()
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// PostAnalytics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId
//     GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplayName),
// ClosedPostDetails AS (SELECT ph.PostId, STRING_AGG(CASE WHEN ph.PostHistoryTypeId = 10 THEN cr.Name END, ', ') AS CloseReasons, COUNT(*) AS CloseCount, MIN(ph.CreationDate) AS FirstCloseDate
//     FROM PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment = CAST(cr.Id AS VARCHAR) WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT pa.Title, pa.OwnerDisplayName, pa.CreationDate, pa.ViewCount, pa.Score, pa.CommentCount, pa.Upvotes, pa.Downvotes, COALESCE(cpd.CloseCount, 0) AS CloseCount,
//        COALESCE(cpd.CloseReasons, 'Not Closed') AS CloseReasons,
//        CASE WHEN pa.Score > 10 THEN 'High Score' WHEN pa.Score BETWEEN 5 AND 10 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostAnalytics pa LEFT JOIN ClosedPostDetails cpd ON pa.PostId = cpd.PostId WHERE pa.ViewCount > (SELECT AVG(ViewCount) FROM Posts)
// ORDER BY pa.CreationDate DESC LIMIT 100;
//
// The WHERE and the order read only base columns, so the 100 questions are picked first. OwnerRank is never read. STRING_AGG is given in history id order.
fn q2784(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, owner_user, score, .. } = &db.post;
    let (s, n) = view_count.fold_flat((0i128, 0i64), |(s, n), w| (s + w as i128, n + 1));
    let a = s as f64 / n as f64;
    let v = drain(db.post.with(post_type_id.eq(1)).select(view_count.filt(move |w: i64| w as f64 > a)));
    let top = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let rt = reason_by_text(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cpd = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(Ident::<PostHistory>::new().and(comment.select(&rt))).buf_fold(|it| {
        let mut v: Vec<(Id<PostHistory>, Str)> = it.into_iter().collect();
        v.sort();
        let s: Vec<Str> = v.iter().map(|x| x.1).collect();
        (v.len() as i64, &*Box::leak(s.join(", ").into_boxed_str()))
    });
    let v = drain((&pa).and((&cpd).opt()));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["title"]);
        f.push(owner_user.get(p).map_or(V::S("Community User"), |u| user_col(db, u, "name")));
        f.extend(post_fields(db, p, &["created", "views", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        let (k, s) = c.unwrap_or((0, "Not Closed"));
        f.extend([V::I(k), V::S(s)]);
        let sc = score.get(p).unwrap();
        f.push(V::S(if sc > 10 { "High Score" } else if sc >= 5 { "Moderate Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.OwnerUserId, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank, DENSE_RANK() OVER (ORDER BY p.CreationDate DESC) AS RecentRank
//     FROM Posts p WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(p.Id) AS PostCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 50
//     GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, STRING_AGG(pt.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id
//     WHERE pt.Name IN ('Post Closed', 'Post Reopened') GROUP BY ph.PostId, ph.CreationDate),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeType FROM Badges b GROUP BY b.UserId)
// SELECT ru.PostId, ru.Title, ru.CreationDate, ru.ViewCount, ru.Score AS PostScore, tu.DisplayName AS Owner, tu.TotalScore, tu.PostCount, cb.CloseReasons, ub.BadgeCount,
//        CASE WHEN ub.HighestBadgeType = 1 THEN 'Gold' WHEN ub.HighestBadgeType = 2 THEN 'Silver' WHEN ub.HighestBadgeType = 3 THEN 'Bronze' ELSE 'None' END AS HighestBadge
// FROM RankedPosts ru JOIN TopUsers tu ON ru.OwnerUserId = tu.UserId LEFT JOIN ClosedPosts cb ON ru.PostId = cb.PostId LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId
// WHERE ru.Rank <= 5 AND (ru.Score >= 10 OR ru.ViewCount > 100) ORDER BY ru.ViewCount DESC, tu.TotalScore DESC, cb.CreationDate DESC LIMIT 100;
//
// Rank breaks ties by post id (the SQL leaves them open); RecentRank is never read. STRING_AGG is given in history id order.
fn q23338(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let tu = db.user.with((&db.user.reputation).gt(50)).group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(htype_name(db).filt(|n: Str| n == "Post Closed" || n == "Post Reopened"))
        .group_by(post.and(hd))
        .select(Ident::<PostHistory>::new().and(htype_name(db)))
        .buf_fold(|it| {
            let mut v: Vec<(Id<PostHistory>, Str)> = it.into_iter().collect();
            v.sort();
            let s: Vec<Str> = v.iter().map(|x| x.1).collect();
            &*Box::leak(s.join(", ").into_boxed_str())
        });
    type C = ((Id<Post>, i64), Str);
    let cv = rel(drain(&cp));
    let cidx: HashIdx<Id<Post>, C> = (&cv).map(|x: C| x.0 .0).inv().select(&cv).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let v = drain(
        (&rp)
            .with(score.ge(10).or(view_count.gt(100)))
            .select(owner_user.select(Ident::<User>::new().and(&tu).and((&ub).opt())).and((&cidx).opt())),
    );
    let v = top_n(v, |&(p, (((_, (s, _)), _), c))| {
        let w = view_count.get(p);
        let d = c.map(|c| c.0 .1);
        (w.is_none(), Reverse(w), Reverse(s), d.is_none(), Reverse(d), p)
    }, 100);
    rows(v.into_iter().map(|(p, (((u, (s, n)), b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([user_col(db, u, "name"), V::I(s), V::I(n), ostr(c.map(|c| c.1)), oint(b.map(|b| b.0))]);
        f.push(V::S(match b.map(|b| b.1) {
            Some(1) => "Gold",
            Some(2) => "Silver",
            Some(3) => "Bronze",
            _ => "None",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT PostId, Title, Score, ViewCount, AnswerCount, OwnerDisplayName FROM RankedPosts WHERE PostRank <= 5),
// VotesSummary AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostsWithVoteCounts AS (SELECT t.PostId, t.Title, t.Score, t.ViewCount, t.AnswerCount, t.OwnerDisplayName, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes
//     FROM TopRankedPosts t LEFT JOIN VotesSummary v ON t.PostId = v.PostId),
// CombinedResults AS (SELECT p.PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.OwnerDisplayName, p.UpVotes, p.DownVotes, 'Top Posts' AS Category FROM PostsWithVoteCounts p
//     UNION ALL SELECT NULL AS PostId, NULL AS Title, NULL AS Score, NULL AS ViewCount, NULL AS AnswerCount, NULL AS OwnerDisplayName, SUM(v.UpVotes) AS UpVotes, SUM(v.DownVotes) AS DownVotes,
//        'Summary' AS Category FROM VotesSummary v)
// SELECT PostId, Title, Score, ViewCount, AnswerCount, OwnerDisplayName, UpVotes, DownVotes, Category FROM CombinedResults ORDER BY Category DESC, Score DESC NULLS LAST LIMIT 20;
//
// The summary sums every PostId group, so it is the vote counts over all of Votes.
fn q3949(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score).and(creation_date)).window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain(&vs), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 20);
    let mut out: Vec<String> = v
        .iter()
        .map(|&(p, a)| {
            let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers"]);
            f.push(user_col(db, owner_user.get(p).unwrap(), "name"));
            f.extend([V::I(a[0]), V::I(a[1]), V::S("Top Posts")]);
            row(f)
        })
        .collect();
    if out.len() < 20 {
        let s = (&db.vote.vote_type_id).fold_flat([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
        out.push(row(vec![V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::I(s[0]), V::I(s[1]), V::S("Summary")]));
    }
    rows(out)
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        AVG(P.Score) AS AverageScore, SUM(P.ViewCount) AS TotalViews, COALESCE(MAX(P.LastActivityDate), '1970-01-01') AS LastActivity,
//        COALESCE(MAX(P.CreationDate), '1970-01-01') AS FirstPost FROM Posts P GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT U.DisplayName, UB.TotalBadges, UB.GoldBadges, PS.QuestionCount, PS.AnswerCount, PS.AverageScore, PS.TotalViews, PS.LastActivity, PS.FirstPost,
//        (SELECT COUNT(*) FROM Comments C WHERE C.UserId = U.Id) AS TotalComments
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId),
// TopUsers AS (SELECT *, ROW_NUMBER() OVER (ORDER BY COALESCE(TotalViews, 0) DESC, AverageScore DESC) AS Rank FROM CombinedStats)
// SELECT DisplayName, TotalBadges, GoldBadges, QuestionCount, AnswerCount, AverageScore, TotalViews, LastActivity, FirstPost, Rank FROM TopUsers WHERE Rank <= 10
// UNION ALL
// SELECT 'Average Statistics' AS DisplayName, AVG(TotalBadges) AS TotalBadges, AVG(GoldBadges) AS GoldBadges, AVG(QuestionCount) AS QuestionCount, AVG(AnswerCount) AS AnswerCount,
//        AVG(AverageScore) AS AverageScore, AVG(TotalViews) AS TotalViews, MAX(LastActivity) AS LastActivity, MIN(FirstPost) AS FirstPost, NULL AS Rank
// FROM CombinedStats WHERE TotalBadges IS NOT NULL;
//
// TotalComments is never read. The UNION makes the count columns DOUBLE. TotalBadges is a COUNT, never NULL, so the averages run over every user.
fn q20560(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, last_activity_date, creation_date, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 2], |a, c| [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64]);
    let ps = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(last_activity_date).and(creation_date)))
        .fold([0i64, 0, 0, 0, 0, 0, i64::MIN, i64::MIN], |a, ((((t, s), w), l), d)| {
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + 1, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6].max(l), a[7].max(d)]
        });
    type S = ([i64; 2], Option<[i64; 8]>);
    let cs = (&ub).and((&ps).opt());
    let tv = |a: Option<[i64; 8]>| a.and_then(|a| if a[4] > 0 { Some(a[5]) } else { None });
    let avs = |a: Option<[i64; 8]>| a.map(|a| a[2] as f64 / a[3] as f64);
    let w = whole(&cs).select(Ident::<User>::new().and(&cs)).window(row_number, |(u, (_, a))| (Reverse(tv(a).unwrap_or(0)), avs(a).is_none(), Reverse(avs(a).map(fkey)), u), asc);
    let mut out: Vec<String> = drain((&w).filt(|(_, k)| k <= 10))
        .into_iter()
        .map(|(_, ((u, (b, a)), k))| {
            let mut f = vec![user_col(db, u, "name"), V::F(b[0] as f64), V::F(b[1] as f64)];
            match a {
                Some(a) => f.extend([V::F(a[0] as f64), V::F(a[1] as f64), V::F(a[2] as f64 / a[3] as f64), tv(Some(a)).map_or(V::Null, |x| V::F(x as f64)), V::T(a[6]), V::T(a[7])]),
                None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]),
            }
            f.push(V::I(k));
            row(f)
        })
        .collect();
    let t = (&cs).fold_flat((0i64, [0i64; 2], 0i64, [0i128; 2], (0f64, 0f64), 0i64, 0i128, 0i64, i64::MIN, i64::MAX), |mut s, (b, a): S| {
        s.0 += 1;
        s.1[0] += b[0];
        s.1[1] += b[1];
        if let Some(a) = a {
            s.2 += 1;
            s.3[0] += a[0] as i128;
            s.3[1] += a[1] as i128;
            s.4 = kahan(s.4, a[2] as f64 / a[3] as f64);
            if a[4] > 0 {
                s.5 += 1;
                s.6 += a[5] as i128;
            }
            s.8 = s.8.max(a[6]);
            s.9 = s.9.min(a[7]);
        }
        s
    });
    let fa = |x: i128, n: i64| if n == 0 { V::Null } else { V::F(x as f64 / n as f64) };
    out.push(row(vec![
        V::S("Average Statistics"),
        fa(t.1[0] as i128, t.0),
        fa(t.1[1] as i128, t.0),
        fa(t.3[0], t.2),
        fa(t.3[1], t.2),
        fmean(t.4, t.2),
        fa(t.6, t.5),
        tmax(t.8),
        tmin(t.9),
        V::Null,
    ]));
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.PostTypeId, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RN
//     FROM Posts p WHERE p.ViewCount IS NOT NULL),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// ClosedPostDetails AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosed, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph
//     JOIN CloseReasonTypes cr ON ph.Comment = CAST(cr.Id AS VARCHAR) WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// CombinedData AS (SELECT rp.PostId, rp.Title, u.DisplayName, u.Reputation, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, cp.LastClosed, cp.CloseReasons, COUNT(c.Id) AS CommentCount
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN Comments c ON c.PostId = rp.PostId
//     LEFT JOIN ClosedPostDetails cp ON cp.PostId = rp.PostId WHERE rp.RN <= 5
//     GROUP BY rp.PostId, rp.Title, u.DisplayName, u.Reputation, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, cp.LastClosed, cp.CloseReasons)
// SELECT cd.*, CASE WHEN cd.CloseReasons IS NOT NULL THEN CONCAT('Closed due to ', cd.CloseReasons) ELSE 'Open' END AS ClosureStatus,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = cd.PostId AND v.VoteTypeId = 2) AS Upvotes, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = cd.PostId AND v.VoteTypeId = 3) AS Downvotes
// FROM CombinedData cd ORDER BY cd.Reputation DESC, cd.LastClosed DESC NULLS LAST;
//
// RN breaks ViewCount ties by post id (the SQL leaves them open). STRING_AGG is given in history id order.
fn q23137(db: &'static So) -> String {
    let Post { post_type_id, view_count, owner_user, .. } = &db.post;
    let w = db.post.with(view_count).group_by(post_type_id).select(Ident::<Post>::new().and(view_count)).window(row_number, |(p, w)| (Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let rt = reason_by_text(db);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(Ident::<PostHistory>::new().and(hd).and(comment.select(&rt))).buf_fold(|it| {
        let mut v: Vec<((Id<PostHistory>, i64), Str)> = it.into_iter().collect();
        v.sort();
        let m = v.iter().map(|x| x.0 .1).max().unwrap();
        let s: Vec<Str> = v.iter().map(|x| x.1).collect();
        (m, &*Box::leak(s.join(", ").into_boxed_str()))
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and((&ub).opt())).and((&cp).opt()).and(&cc).and(&vc)));
    rows(v.into_iter().map(|(p, ((((u, b), c), n), a))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(match b {
            Some(b) => [V::I(b[0]), V::I(b[1]), V::I(b[2])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some((d, s)) => [V::T(d), V::S(s)],
            None => [V::Null, V::Null],
        });
        f.push(V::I(n));
        f.push(match c {
            Some((_, s)) => V::Owned(format!("Closed due to {s}")),
            None => V::S("Open"),
        });
        f.extend([V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.PostTypeId, P.Score,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate ASC) AS RankPerType FROM Posts P WHERE P.Score IS NOT NULL),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// CloseReasonStats AS (SELECT PH.PostId, COUNT(*) AS CloseReasonCount, STRING_AGG(CASE WHEN PH.Comment IS NOT NULL THEN PH.Comment ELSE 'Unspecified' END, ', ') AS CloseReasons
//     FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId),
// AppendedData AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.OwnerUserId, CB.CloseReasonCount, CB.CloseReasons, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges,
//        CASE WHEN RP.PostTypeId = 1 AND RP.RankPerType <= 5 THEN 'Top Question' WHEN RP.PostTypeId = 2 AND RP.Score > 10 THEN 'Popular Answer' ELSE 'Other' END AS PostCategory
//     FROM RankedPosts RP LEFT JOIN CloseReasonStats CB ON RP.PostId = CB.PostId LEFT JOIN UserBadges UB ON RP.OwnerUserId = UB.UserId)
// SELECT AD.Title, AD.CreationDate, AD.CloseReasonCount, AD.CloseReasons, AD.BadgeCount, AD.GoldBadges, AD.SilverBadges, AD.BronzeBadges, AD.PostCategory,
//        CASE WHEN AD.CloseReasonCount IS NULL THEN 'No Close Reason Available' ELSE 'Has Close Reason: ' || AD.CloseReasons END AS CloseStatus
// FROM AppendedData AD WHERE AD.BadgeCount > 0 ORDER BY AD.CreationDate DESC LIMIT 100;
//
// STRING_AGG is given in history id order.
fn q23940(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(score).and(creation_date)).window(rank, |((_, s), d)| (Reverse(s), d), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).with(post_type_id.eq(1)).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain(db.post.select(owner_user.select(&ub).filt(|a| a[0] > 0)));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let tp = rel(v);
    type T = (Id<Post>, [i64; 4]);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cb = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(Ident::<PostHistory>::new().and(comment.opt())).buf_fold(|it| {
        let mut v: Vec<(Id<PostHistory>, Option<Str>)> = it.into_iter().collect();
        v.sort();
        let s: Vec<Str> = v.iter().map(|x| x.1.unwrap_or("Unspecified")).collect();
        (v.len() as i64, &*Box::leak(s.join(", ").into_boxed_str()))
    });
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select((&cb).opt().and(Ident::<Post>::new().with(&top).opt())))));
    rows(v.into_iter().map(|(_, ((p, b), (c, t)))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(match c {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::Null, V::Null],
        });
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        f.push(V::S(if t.is_some() {
            "Top Question"
        } else if post_type_id.get(p).unwrap() == 2 && score.get(p).unwrap() > 10 {
            "Popular Answer"
        } else {
            "Other"
        }));
        f.push(match c {
            Some((_, s)) => V::Owned(format!("Has Close Reason: {s}")),
            None => V::S("No Close Reason Available"),
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, COALESCE(ph.Comment, 'No Comments') AS PostHistoryComment, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY p.ViewCount DESC) AS ViewRank
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id LEFT JOIN PostHistory ph ON ph.PostId = p.Id AND ph.PostHistoryTypeId IN (10, 11)
//     GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, ph.Comment),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.ViewCount, pd.CreationDate, pd.PostHistoryComment, pd.CommentCount, pd.UpVotes, pd.DownVotes,
//        CASE WHEN pd.CommentCount > 0 THEN ROUND(COALESCE(pd.UpVotes, 0) / NULLIF(pd.CommentCount, 0), 2) ELSE 0 END AS UpVoteToCommentRatio,
//        RANK() OVER (ORDER BY pd.UpVotes DESC) AS TopRank FROM PostDetails pd WHERE pd.ViewRank <= 100),
// UserTopPosts AS (SELECT ur.UserId, ur.Reputation, tp.Title, tp.UpVotes, tp.CommentCount, tp.UpVoteToCommentRatio FROM UserReputation ur JOIN Posts p ON p.OwnerUserId = ur.UserId
//     JOIN TopPosts tp ON tp.PostId = p.Id)
// SELECT utp.UserId, u.DisplayName, utp.Reputation, ARRAY_AGG(utp.Title) AS PostTitles, SUM(utp.UpVotes) AS TotalUpVotes, SUM(utp.CommentCount) AS TotalComments,
//        AVG(utp.UpVoteToCommentRatio) AS AvgUpVoteToCommentRatio
// FROM UserTopPosts utp JOIN Users u ON u.Id = utp.UserId GROUP BY utp.UserId, u.DisplayName, utp.Reputation HAVING AVG(utp.UpVoteToCommentRatio) > 1.5
// ORDER BY TotalUpVotes DESC LIMIT 10;
//
// PostDetails groups by the post and the close comment, so its rows are built over posts x history (10, 11) and grouped by (post, comment).
// ARRAY_AGG is given in post id order.
fn q23040(db: &'static So) -> String {
    let Post { view_count, owner_user, .. } = &db.post;
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let ph = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11])));
    type J = (Id<Post>, Option<Id<PostHistory>>);
    let jv = rel(drain(db.post.select(ph.opt())));
    let pd = (&jv)
        .group_by(Same::<J>::new().map(|(p, h): J| (p, h.and_then(|h| comment.get(h)))))
        .select(Same::<J>::new().map(|x: J| x.0).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    type K = (Id<Post>, Option<Str>);
    type G = (K, [i64; 3]);
    let w = whole(&pd).select(Same::<K>::new().and(&pd).and(Same::<K>::new().map(|x: K| x.0).select(view_count.opt()))).window(rank, |(_, w)| (w.is_none(), Reverse(w)), asc);
    let tp: MatSet<G> = (&w).filt(|(_, k)| k <= 100).map(|(x, _)| x.0).collect();
    let g = (&tp)
        .group_by(Same::<G>::new().map(|x: G| x.0 .0).select(owner_user))
        .select(Same::<G>::new())
        .buf_fold(|it| {
            let mut v: Vec<G> = it.into_iter().collect();
            v.sort();
            let ratio = |a: [i64; 3]| if a[0] > 0 { (a[1] as f64 / a[0] as f64 * 100.0).round() / 100.0 } else { 0.0 };
            let up: i64 = v.iter().map(|x| x.1[1]).sum();
            let cc: i64 = v.iter().map(|x| x.1[0]).sum();
            let s: f64 = v.iter().map(|x| ratio(x.1)).fold(0.0, |s, x| s + x);
            let titles: Vec<Id<Post>> = v.iter().map(|x| x.0 .0).collect();
            (up, cc, s / v.len() as f64, &*Box::leak(titles.into_boxed_slice()))
        });
    let v = top_n(drain((&g).filt(|(_, _, a, _)| a > 1.5)), |&(u, (s, _, _, _))| (Reverse(s), u), 10);
    rows(v.into_iter().map(|(u, (s, c, a, t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::L(t.iter().map(|&p| title(db, p)).collect()));
        f.extend([V::I(s), V::I(c), V::F(a)]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, 1 AS Level FROM Users U WHERE U.Reputation > 10000
//     UNION ALL SELECT U.Id, U.DisplayName, U.Reputation, U.CreationDate, UR.Level + 1 FROM Users U JOIN UserReputation UR ON U.Reputation > UR.Reputation),
// PostStats AS (SELECT P.Id AS PostId, P.OwnerUserId, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount, COUNT(CASE WHEN Ph.Comment IS NOT NULL THEN 1 END) AS EditCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory Ph ON P.Id = Ph.PostId GROUP BY P.Id, P.OwnerUserId),
// FilteredPosts AS (SELECT PS.PostId, PS.OwnerUserId, PS.TotalBounty, PS.CommentCount, PS.VoteCount, PS.EditCount, U.Reputation AS OwnerReputation
//     FROM PostStats PS JOIN Users U ON PS.OwnerUserId = U.Id WHERE PS.CommentCount > 5 AND PS.VoteCount > 10),
// RankedPosts AS (SELECT FP.PostId, FP.OwnerUserId, FP.TotalBounty, FP.CommentCount, FP.VoteCount, FP.EditCount, FP.OwnerReputation,
//        RANK() OVER (PARTITION BY FP.OwnerUserId ORDER BY FP.VoteCount DESC, FP.CommentCount DESC) AS Rank FROM FilteredPosts FP)
// SELECT RP.PostId, U.DisplayName AS OwnerName, RP.TotalBounty, RP.CommentCount, RP.VoteCount, RP.EditCount, RP.Rank, COALESCE(T.TagName, 'No Tag') AS TopTag
// FROM RankedPosts RP JOIN Users U ON RP.OwnerUserId = U.Id
// LEFT JOIN (SELECT TL.PostId, T.TagName, ROW_NUMBER() OVER (PARTITION BY TL.PostId ORDER BY T.Count DESC) AS TagRank FROM PostLinks TL JOIN Tags T ON TL.RelatedPostId = T.Id) T
//     ON RP.PostId = T.PostId AND T.TagRank = 1
// WHERE RP.Rank = 1 ORDER BY RP.OwnerReputation DESC, RP.CommentCount DESC;
//
// UserReputation is never read. The counts run over the vote x comment x history rows. `TL.RelatedPostId = T.Id` joins a post id to a tag id, so
// it goes through the raw ids; TagRank breaks Count ties by tag id (the SQL leaves them open).
fn q33590(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt().and(comments_of(db).opt()).and(history_of(db).select((&db.post_history.comment).opt()).opt()))
        .fold([0i64; 4], |a, ((v, c), h)| [a[0] + v.flatten().unwrap_or(0), a[1] + c.is_some() as i64, a[2] + v.is_some() as i64, a[3] + h.flatten().is_some() as i64]);
    let fp = (&ps).filt(|a| a[1] > 5 && a[2] > 10);
    let w = db.post.with(&fp).group_by(owner_user).select(Ident::<Post>::new().and(&fp)).window(rank, |(_, a)| (Reverse(a[2]), Reverse(a[1])), asc);
    let tidx: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let PostLink { post, related_post_id, .. } = &db.post_link;
    let tw = db.post_link.group_by(post).select(related_post_id.select(&tidx).select(Ident::<Tag>::new().and(&db.tag.count))).window(row_number, |(t, c)| (Reverse(c), t), asc);
    let tt: HashIdx<Id<Post>, Str> = (&tw).filt(|(_, r)| r == 1).map(|((t, _), _)| t).select(&db.tag.tag_name).collect();
    let v = drain((&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).select(Ident::<Post>::new().and(&fp).and((&tt).opt())));
    rows(v.into_iter().map(|(u, ((p, a), t))| {
        let mut f = post_fields(db, p, &["id"]);
        f.push(user_col(db, u, "name"));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(1)]);
        f.push(V::S(t.unwrap_or("No Tag")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, COALESCE(p.ViewCount, 0) AS ViewCount, COALESCE(p.AnswerCount, 0) AS AnswerCount, COALESCE(u.Reputation, 0) AS UserReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, RANK() OVER (ORDER BY COALESCE(p.Score, 0) DESC, p.CreationDate ASC) AS ScoreRank,
//        SUBSTRING(p.Body, 1, 100) AS ShortBody
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, ViewCount, AnswerCount, UserReputation, rn, ScoreRank, ShortBody FROM RankedPosts WHERE rn = 1 AND ScoreRank <= 10),
// PostHistoryDetails AS (SELECT ph.PostId, STRING_AGG(CONCAT('Type: ', pht.Name, ' by: ', ph.UserDisplayName, ' on: ', ph.CreationDate), '; ') AS HistoryInfo, COUNT(*) AS HistoryCount
//     FROM PostHistory ph INNER JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId),
// CommentStats AS (SELECT PostId, COUNT(*) AS CommentCount, AVG(LENGTH(Text)) AS AverageCommentLength FROM Comments GROUP BY PostId),
// FinalReport AS (SELECT t.PostId, t.Title, t.ViewCount, t.AnswerCount, t.UserReputation, COALESCE(c.CommentCount, 0) AS CommentCount,
//        COALESCE(c.AverageCommentLength, 0) AS AverageCommentLength, COALESCE(h.HistoryCount, 0) AS HistoryCount, h.HistoryInfo
//     FROM TopPosts t LEFT JOIN CommentStats c ON t.PostId = c.PostId LEFT JOIN PostHistoryDetails h ON t.PostId = h.PostId)
// SELECT PostId, Title, ViewCount, AnswerCount, UserReputation, CommentCount, AverageCommentLength, HistoryCount, HistoryInfo FROM FinalReport WHERE ViewCount > 10
// ORDER BY AnswerCount DESC, ViewCount DESC LIMIT 50;
//
// Both ranks read only base columns, so the posts are picked first; rn breaks CreationDate ties by post id. STRING_AGG is given in history id order.
fn q24577(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, owner_user_id, view_count, answer_count, .. } = &db.post;
    let base = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let sw = whole(base()).select(Ident::<Post>::new().and(score).and(creation_date)).window(rank, |((_, s), d)| (Reverse(s), d), asc);
    let sr: MatSet<Id<Post>> = (&sw).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let rw = base().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rn1: MatSet<Id<Post>> = (&rw).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let tp: MatSet<Id<Post>> = (&sr).with(&rn1).with(view_count.gt(10)).collect();
    let Comment { text, .. } = &db.comment;
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(text)).fold((0i64, 0i64), |(n, l), t| (n + 1, l + t.chars().count() as i64));
    let PostHistory { user_display_name, creation_date: hd, .. } = &db.post_history;
    let hs = (&tp)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().and(htype_name(db)).and(user_display_name.opt()).and(hd)))
        .buf_fold(|it| {
            let mut v: Vec<(((Id<PostHistory>, Str), Option<Str>), i64)> = it.into_iter().collect();
            v.sort();
            let s: Vec<String> = v.iter().map(|&(((_, n), u), d)| format!("Type: {n} by: {} on: {}", u.unwrap_or(""), ts_text(d))).collect();
            (v.len() as i64, &*Box::leak(s.join("; ").into_boxed_str()))
        });
    let v = drain((&tp).select((&cs).opt().and((&hs).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(answer_count.get(p).unwrap_or(0)), Reverse(view_count.get(p).unwrap_or(0)), p), 50);
    rows(v.into_iter().map(|(p, (c, h))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(view_count.get(p).unwrap_or(0)), V::I(answer_count.get(p).unwrap_or(0))]);
        f.push(V::I(owner_user.get(p).map_or(0, |u| db.user.reputation.get(u).unwrap())));
        f.extend(match c {
            Some((n, l)) => [V::I(n), V::F(l as f64 / n as f64)],
            None => [V::I(0), V::F(0.0)],
        });
        f.extend(match h {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, p.PostTypeId,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS CreationRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopQuestions AS (SELECT rp.PostId, rp.Title, u.DisplayName AS Owner, rp.Score, rp.ViewCount, rp.CreationDate FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id
//     WHERE rp.PostTypeId = 1 AND rp.Rank <= 5),
// TopAnswers AS (SELECT rp.PostId, rp.Title, u.DisplayName AS Owner, rp.Score, rp.ViewCount, rp.CreationDate FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id
//     WHERE rp.PostTypeId = 2 AND rp.Rank <= 5),
// CombinedTopPosts AS (SELECT 'Question' AS PostType, tq.PostId, tq.Title, tq.Owner, tq.Score, tq.ViewCount, tq.CreationDate FROM TopQuestions tq
//     UNION ALL SELECT 'Answer' AS PostType, ta.PostId, ta.Title, ta.Owner, ta.Score, ta.ViewCount, ta.CreationDate FROM TopAnswers ta),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// FinalPosts AS (SELECT ct.*, COALESCE(pc.CommentCount, 0) AS TotalComments FROM CombinedTopPosts ct LEFT JOIN PostComments pc ON ct.PostId = pc.PostId)
// SELECT fp.PostType, fp.PostId, fp.Title, fp.Owner, fp.Score, fp.ViewCount, fp.CreationDate, fp.TotalComments,
//        CASE WHEN fp.TotalComments = 0 THEN 'No comments' ELSE CONCAT(fp.TotalComments, ' comments') END AS CommentStatus
// FROM FinalPosts fp ORDER BY fp.Score DESC, fp.CreationDate DESC;
//
// Rank breaks ties by post id (the SQL leaves them open); CreationRank is never read.
fn q32948(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).with(post_type_id.is_in([1, 2])).select(post_type_id.and(owner_user).and((&cc).opt())));
    rows(v.into_iter().map(|(p, ((t, u), c))| {
        let c = c.unwrap_or(0);
        let mut f = vec![V::S(if t == 1 { "Question" } else { "Answer" })];
        f.extend(post_fields(db, p, &["id", "title"]));
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["score", "views", "created"]));
        f.push(V::I(c));
        f.push(if c == 0 { V::S("No comments") } else { V::Owned(format!("{c} comments")) });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.Body,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryInfo AS (SELECT p.Id AS PostId, MAX(ph.CreationDate) AS LastEdited, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id),
// VoteSummary AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v INNER JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// FinalResult AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ub.BadgeCount, ub.BadgeNames, ph.LastEdited, ph.CloseReopenCount,
//        COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes
//     FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostHistoryInfo ph ON rp.PostId = ph.PostId LEFT JOIN VoteSummary vs ON rp.PostId = vs.PostId
//     WHERE rp.Rank = 1 AND rp.ViewCount > 100)
// SELECT fr.Title, fr.CreationDate, fr.Score, fr.ViewCount, fr.BadgeCount, fr.BadgeNames, fr.LastEdited, fr.CloseReopenCount,
//        CASE WHEN fr.UpVotes > fr.DownVotes THEN 'Positive' WHEN fr.UpVotes < fr.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM FinalResult fr WHERE EXISTS (SELECT 1 FROM Posts sub WHERE sub.AcceptedAnswerId = fr.PostId) ORDER BY fr.Score DESC, fr.ViewCount DESC;
//
// Rank breaks CreationDate ties by post id (the SQL leaves them open). STRING_AGG is given in badge id order.
fn q21366(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, owner_user, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let accepted_by: HashIdx<Id<Post>, Id<Post>> = (&db.post.accepted_answer).inv().collect();
    let fr: MatSet<Id<Post>> = (&rp).with(view_count.gt(100)).with(&accepted_by).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(Ident::<Badge>::new().and(&db.badge.name)).opt()).buf_fold(|it| {
        let mut v: Vec<(Id<Badge>, Str)> = it.into_iter().flatten().collect();
        v.sort();
        let s: Vec<Str> = v.iter().map(|x| x.1).collect();
        (v.len() as i64, if s.is_empty() { None } else { Some(&*Box::leak(s.join(", ").into_boxed_str())) })
    });
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = (&fr).group_by(Ident::<Post>::new()).select(history_of(db).select(hd.and(post_history_type_id)).opt()).fold((i64::MIN, 0i64), |(m, n), h| match h {
        Some((d, t)) => (m.max(d), n + (t == 10 || t == 11) as i64),
        None => (m, n),
    });
    let vs = (&fr).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let v = drain((&fr).select(owner_user.select(&ub).opt().and(&ph).and((&vs).opt())));
    rows(v.into_iter().map(|(p, ((b, (m, n)), a))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(match b {
            Some((k, s)) => [V::I(k), ostr(s)],
            None => [V::Null, V::Null],
        });
        f.extend([tmax(m), V::I(n)]);
        let a = a.unwrap_or([0, 0]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.Score > 0),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(p.ViewCount) AS AverageViewCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, u.DisplayName, ua.VoteCount AS UserVoteCount, ua.UpVotes, ua.DownVotes, ua.AverageViewCount
//     FROM RankedPosts rp LEFT JOIN UserActivity ua ON ua.UserId = (SELECT Id FROM Users ORDER BY Reputation DESC LIMIT 1)
//     JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId LIMIT 1) WHERE rp.PostRank = 1)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.DisplayName AS Owner, COALESCE(tp.UserVoteCount, 0) AS TotalVotes, COALESCE(tp.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(tp.DownVotes, 0) AS TotalDownVotes,
//        CASE WHEN tp.AverageViewCount IS NULL THEN 'No views available' ELSE 'Average views - ' || ROUND(tp.AverageViewCount, 2)::text END AS AverageViewText
// FROM TopPosts tp
// UNION ALL
// SELECT NULL AS PostId, 'Aggregate Statistics' AS Title, NULL AS CreationDate, NULL AS ViewCount, NULL AS Owner, SUM(UserVoteCount) AS TotalVotes, SUM(UpVotes) AS TotalUpVotes,
//        SUM(DownVotes) AS TotalDownVotes, NULL AS AverageViewText FROM TopPosts GROUP BY NULL
// ORDER BY TotalVotes DESC NULLS LAST;
//
// The `ORDER BY Reputation DESC LIMIT 1` subquery is the top user (a tie there would be ambiguous); `Id = rp.PostId LIMIT 1` looks a post up by its
// primary key, so it is the post's owner. PostRank breaks CreationDate ties by post id (the SQL leaves them open).
fn q3898(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let w = db.post.with(score.gt(0)).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let best: MatSet<Id<User>> = rel(top_n(drain(&db.user.reputation), |&(_, r)| Reverse(r), 1).into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Vote { vote_type_id, post, .. } = &db.vote;
    let ua = (&best)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id.and(post.select(&db.post.view_count).opt())).opt())
        .fold([0i64; 5], |a, v| match v {
            Some((t, w)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)],
            None => a,
        });
    let uav: HashIdx<(), [i64; 5]> = whole(&best).select(&ua).collect();
    type T = ((Id<Post>, Id<User>), Option<[i64; 5]>);
    let tv: MatSet<T> = (&rp).select(Ident::<Post>::new().and(owner_user).and(Ident::<Post>::new().map(|_: Id<Post>| ()).select(&uav).opt())).collect();
    let agg = whole(&tv).fold([0i64, 0, 0, 0], |s, (_, a): T| match a {
        Some(a) => [s[0] + 1, s[1] + a[0], s[2] + a[1], s[3] + a[2]],
        None => s,
    });
    let mut out: Vec<String> = drain(&tv)
        .into_iter()
        .map(|(_, ((p, u), a))| {
            let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
            f.push(user_col(db, u, "name"));
            let a = a.unwrap_or([0; 5]);
            f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
            f.push(if a[3] == 0 { V::S("No views available") } else { V::Owned(format!("Average views - {}", (a[4] as f64 / a[3] as f64 * 100.0).round() / 100.0)) });
            row(f)
        })
        .collect();
    out.extend(drain(&agg).into_iter().map(|(_, s)| {
        let n = |x: i64| if s[0] == 0 { V::Null } else { V::I(x) };
        row(vec![V::Null, V::S("Aggregate Statistics"), V::Null, V::Null, V::Null, n(s[1]), n(s[2]), n(s[3]), V::Null])
    }));
    rows(out)
}

// WITH TagStatistics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        AVG(u.Reputation) AS AverageUserReputation, STRING_AGG(DISTINCT p.Title, '; ') AS PostTitles
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN Users u ON u.Id = p.OwnerUserId GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, AcceptedAnswers, AverageUserReputation, PostTitles, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM TagStatistics),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, ph.CreationDate AS ClosedDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10)
// SELECT tt.TagName, tt.PostCount, tt.AcceptedAnswers, tt.AverageUserReputation, closed.PostId, closed.Title, closed.ClosedDate
// FROM TopTags tt LEFT JOIN ClosedPosts closed ON tt.TagName LIKE '%' || closed.Title || '%' WHERE tt.Rank <= 10 ORDER BY tt.PostCount DESC, tt.TagName;
//
// PostTitles is never read. The title LIKE goes through the distinct titles of closed posts, each title a pattern (its `%` and `_` stay wildcards).
fn q29808(db: &'static So) -> String {
    let Post { owner_user, accepted_answer_id, title, .. } = &db.post;
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select(Ident::<Post>::new().with(accepted_answer_id).opt().and(owner_user.select(&db.user.reputation))))
        .fold([0i64; 3], |a, (c, r)| [a[0] + 1, a[1] + c.is_some() as i64, a[2] + r]);
    let pc = db.tag.group_by(&db.tag.tag_name).select((&by_tag).map(|(p, _)| p).select(Ident::<Post>::new().with(owner_user))).count_distinct();
    type T = (Str, (i64, [i64; 3]));
    let w = whole(&pc).select(Same::<Str>::new().and((&pc).and(&ts_))).window(rank, |(_, (n, _))| Reverse(n), asc);
    let tt: MatSet<T> = (&w).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let names: MatSet<Str> = (&tt).map(|x: T| x.0).collect();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let by_title: HashIdx<Str, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(post.select(title)).inv().collect();
    let hit: HashIdx<Str, Id<PostHistory>> = (&names).select_where(&by_title, |n: Str, t: Str| like(n, &format!("%{t}%"))).collect();
    let v = drain((&tt).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select(&hit).opt())));
    rows(v.into_iter().map(|(_, ((n, (c, a)), h))| {
        let mut f = vec![V::S(n), V::I(c), V::I(a[1]), V::F(a[2] as f64 / a[0] as f64)];
        match h {
            Some(h) => {
                let p = post.get(h).unwrap();
                f.extend(post_fields(db, p, &["id", "title"]));
                f.push(V::T(db.post_history.creation_date.get(h).unwrap()));
            }
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, t.TagName
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN UNNEST(string_to_array(p.Tags, '><')) AS t(TagName) ON TRUE WHERE p.CreationDate >= DATE('2024-10-01') - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, t.TagName),
// RankedPosts AS (SELECT pd.*, RANK() OVER (PARTITION BY pd.TagName ORDER BY pd.Score DESC, pd.ViewCount DESC) AS TagRank FROM PostDetails pd)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Author, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.TagName
// FROM RankedPosts rp WHERE rp.TagRank <= 5 ORDER BY rp.TagName, rp.TagRank;
//
// The split pieces keep the outer `<` and `>`. A post without Tags keeps one NULL TagName row, and those rows rank as their own partition.
fn q5609(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, score, view_count, .. } = &db.post;
    type J = (Id<Post>, Option<Str>);
    let jv = rel(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30))).with(owner_user).select(tags_str.flat_map(|t: Str| t.split("><")).opt())));
    let pd = (&jv)
        .group_by(Same::<J>::new())
        .select(Same::<J>::new().map(|x: J| x.0).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = whole(&pd)
        .group_by(Same::<J>::new().map(|x: J| x.1))
        .select(Same::<J>::new().and(&pd).and(Same::<J>::new().map(|x: J| x.0).select(score.and(view_count.opt()))))
        .window(rank, |(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let v = drain((&w).filt(|(_, k)| k <= 5));
    rows(v.into_iter().map(|(_, ((((p, t), a), _), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), ostr(t)]);
        row(f)
    }))
}

// WITH ComputedTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagStatistics AS (SELECT Tag, COUNT(*) AS PostCount, COUNT(DISTINCT pt.OwnerUserId) AS UserCount, AVG(pt.Score) AS AverageScore FROM ComputedTags ct JOIN Posts pt ON ct.PostId = pt.Id
//     GROUP BY Tag),
// TopTags AS (SELECT Tag, PostCount, UserCount, AverageScore, ROW_NUMBER() OVER (ORDER BY PostCount DESC, AverageScore DESC) AS Rank FROM TagStatistics)
// SELECT t.Tag, t.PostCount, t.UserCount, t.AverageScore, COALESCE(b.BadgeName, 'No Badge') AS BadgeName, COUNT(DISTINCT b.UserId) AS BadgeWinners
// FROM TopTags t LEFT JOIN (SELECT b.Name AS BadgeName, b.UserId FROM Badges b WHERE b.Class = 1 OR b.Class = 2) b ON t.UserCount >= 10
// WHERE t.Rank <= 10 GROUP BY t.Tag, t.PostCount, t.UserCount, t.AverageScore, b.BadgeName ORDER BY t.PostCount DESC, t.AverageScore DESC;
//
// `ON t.UserCount >= 10` names only TopTags, so those tags cross every gold and silver badge and the rest get the NULL row.
fn q27790(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user_id, score, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let ts_ = qs().group_by(tags_str.flat_map(tag_list)).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let uc = qs().group_by(tags_str.flat_map(tag_list)).select(owner_user_id).count_distinct();
    type T = (Str, ((i64, i64), Option<i64>));
    let top: MatSet<T> = rel(top_n(drain((&ts_).and((&uc).opt())), |&(t, ((n, s), _))| (Reverse(n), Reverse(fkey(s as f64 / n as f64)), t), 10)).collect();
    let Badge { class, name, user_id, .. } = &db.badge;
    let bn = db.badge.with(class.eq(1).or(class.eq(2))).group_by(name).select(user_id).count_distinct();
    let bs: HashIdx<(), (Str, i64)> = whole(&bn).select(Same::<Str>::new().and(&bn)).collect();
    let wide = Same::<T>::new().filt(|(_, (_, u)): T| u.unwrap_or(0) >= 10).map(|_: T| ()).select(&bs).opt();
    let v = drain((&top).select(Same::<T>::new().and(wide)));
    rows(v.into_iter().map(|(_, ((t, ((n, s), u)), b))| row(vec![V::S(t), V::I(n), V::I(u.unwrap_or(0)), V::F(s as f64 / n as f64), V::S(b.map_or("No Badge", |b| b.0)), V::I(b.map_or(0, |b| b.1))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore,
//        STRING_AGG(t.TagName, ', ') AS TagsList
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%' WHERE p.CreationDate > CURRENT_DATE - INTERVAL '1 year' AND
//     p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// UserVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.RankScore, COALESCE(rp.TagsList, '') AS TagsList, COALESCE(uv.UpVotes, 0) AS UpVotes,
//        COALESCE(uv.DownVotes, 0) AS DownVotes
// FROM RankedPosts rp LEFT JOIN UserVotes uv ON rp.PostId = uv.PostId WHERE rp.RankScore <= 10 ORDER BY rp.RankScore, rp.ViewCount DESC;
//
// RankScore reads only base columns, so the posts are picked first. COUNT(c.Id) and STRING_AGG run over the comment x tag rows; STRING_AGG is given
// in (comment, tag) id order.
fn q7442(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let w = whole(db.post.with(creation_date.gt(add_years(current_date(), -1)).and(post_type_id.eq(1))))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    type P = (Id<Post>, i64);
    let rv: MatSet<P> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), k)| (p, k)).collect();
    let lt = tag_mentions(db);
    let tags_of: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&lt).map(|(p, _)| p).inv().collect();
    let rp = (&rv)
        .map(|x: P| x.0)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&tags_of).map(|(_, t)| t).select(Ident::<Tag>::new().and(&db.tag.tag_name)).opt()))
        .buf_fold(|it| {
            let mut v: Vec<(Option<Id<Comment>>, Option<(Id<Tag>, Str)>)> = it.into_iter().collect();
            v.sort();
            let n = v.iter().filter(|x| x.0.is_some()).count() as i64;
            let s: Vec<Str> = v.iter().filter_map(|x| x.1).map(|t| t.1).collect();
            (n, &*Box::leak(s.join(", ").into_boxed_str()))
        });
    let uv = (&rv).map(|x: P| x.0).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&rv).select(Same::<P>::new().and(Same::<P>::new().map(|x: P| x.0).select((&rp).and((&uv).opt())))));
    rows(v.into_iter().map(|(_, ((p, k), ((n, s), u)))| {
        let u = u.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(n), V::I(k), V::S(s), V::I(u[0]), V::I(u[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// MostCommentedPosts AS (SELECT PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId),
// PostsWithTags AS (SELECT p.Id AS PostId, p.Title, t.TagName, COALESCE(cm.CommentCount, 0) as TotalComments
//     FROM Posts p LEFT JOIN LATERAL UNNEST(STRING_TO_ARRAY(p.Tags, ',')) AS TagArray(Tag) ON TRUE LEFT JOIN Tags t ON t.TagName = TRIM(TagArray.Tag)
//     LEFT JOIN MostCommentedPosts cm ON p.Id = cm.PostId)
// SELECT wp.PostId, wp.Title, wp.TagName, wp.TotalComments, rp.OwnerName FROM PostsWithTags wp JOIN RankedPosts rp ON wp.PostId = rp.PostId
// WHERE rp.PostRank = 1 AND wp.TotalComments > 5 ORDER BY wp.TotalComments DESC, rp.CreationDate DESC LIMIT 10;
//
// PostRank breaks CreationDate ties by post id (the SQL leaves them open).
fn q33285(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cm = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let names: HashIdx<Str, Str> = (&db.tag.tag_name).inv().select(&db.tag.tag_name).collect();
    let tags = tags_str.flat_map(|t: Str| t.split(',').collect::<Vec<_>>()).select(Same::<Str>::new().map(|x: Str| x.trim()).select(&names).opt()).opt();
    let v = drain((&cm).filt(|n| n > 5).and(tags).and(owner_user));
    let v = top_n(v, |&(p, ((n, t), _))| (Reverse(n), Reverse(creation_date.get(p).unwrap()), p, t), 10);
    rows(v.into_iter().map(|(p, ((n, t), u))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([ostr(t.flatten()), V::I(n), user_col(db, u, "name")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COALESCE(r.BadgeCount, 0) AS BadgeCount
//     FROM Posts p LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) r ON p.OwnerUserId = r.UserId),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate, BadgeCount FROM RankedPosts WHERE Rank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(CASE WHEN c.UserId IS NOT NULL THEN 1 END) AS CommentCount, STRING_AGG(c.Text, ' | ') AS CommentTexts FROM Comments c GROUP BY c.PostId)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.BadgeCount, COALESCE(pc.CommentCount, 0) AS TotalComments, PC.CommentTexts
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId WHERE tp.BadgeCount > 0 OR tp.ViewCount > 100
// ORDER BY tp.Score DESC, tp.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// Rank breaks Score ties by post id (the SQL leaves them open). STRING_AGG is given in comment id order.
fn q1136(db: &'static So) -> String {
    let Post { score, post_type_id, owner_user_id, view_count, creation_date, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let Comment { user_id, text, .. } = &db.comment;
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().and(user_id.opt()).and(text))).buf_fold(|it| {
        let mut v: Vec<((Id<Comment>, Option<i64>), Str)> = it.into_iter().collect();
        v.sort();
        let n = v.iter().filter(|x| x.0 .1.is_some()).count() as i64;
        let s: Vec<Str> = v.iter().map(|x| x.1).collect();
        (n, &*Box::leak(s.join(" | ").into_boxed_str()))
    });
    type J = (Id<Post>, ((Option<i64>, Option<i64>), Option<(i64, Str)>));
    let v = drain(
        (&tp)
            .select(Ident::<Post>::new().and(owner_user_id.select(&bc).opt().and(view_count.opt()).and((&pc).opt())))
            .filt(|(_, ((b, w), _)): J| b.unwrap_or(0) > 0 || w.map_or(false, |w| w > 100)),
    );
    let v = top_n(v, |&(_, (p, _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(_, (p, ((b, _), c)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.push(V::I(b.unwrap_or(0)));
        f.extend(match c {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.ViewCount > 1000 THEN 1 ELSE 0 END) AS HighViewCountPosts,
//        STRING_AGG(DISTINCT p.Title, ', ') AS TopPostTitles, COUNT(DISTINCT c.Id) AS TotalComments, SUM(v.BountyAmount) AS TotalBountyAmount
//     FROM Tags AS t JOIN Posts AS p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Comments AS c ON c.PostId = p.Id LEFT JOIN Votes AS v ON v.PostId = p.Id AND v.VoteTypeId IN (8, 9)
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, HighViewCountPosts, TotalComments, TotalBountyAmount, ROW_NUMBER() OVER (ORDER BY PostCount DESC, HighViewCountPosts DESC) AS Rank FROM TagStats)
// SELECT tt.TagName, tt.PostCount, tt.HighViewCountPosts, tt.TotalComments, tt.TotalBountyAmount, tt.Rank,
//        CASE WHEN tt.PostCount > 100 THEN 'High Activity' WHEN tt.PostCount BETWEEN 51 AND 100 THEN 'Moderate Activity' ELSE 'Low Activity' END AS ActivityLevel
// FROM TopTags AS tt WHERE tt.Rank <= 10 ORDER BY tt.Rank;
//
// TopPostTitles is never read. HighViewCountPosts and the bounty sum run over the post x comment x vote rows.
fn q29234(db: &'static So) -> String {
    let Post { creation_date, view_count, .. } = &db.post;
    let lt = tag_mentions(db);
    let recent = || Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1)));
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let posts = || (&by_tag).map(|(p, _)| p).select(recent());
    let a = db
        .tag
        .group_by(&db.tag.tag_name)
        .select(posts().select(view_count.opt().and(comments_of(db).opt()).and(bv.opt())))
        .fold((0i64, None::<i64>), |(h, s), ((w, _), b)| {
            (h + w.map_or(false, |w| w > 1000) as i64, match b.flatten() { Some(b) => Some(s.unwrap_or(0) + b), None => s })
        });
    let pc = db.tag.group_by(&db.tag.tag_name).select(posts()).count_distinct();
    let cc = db.tag.group_by(&db.tag.tag_name).select(posts().select(comments_of(db))).count_distinct();
    let w = whole(&pc).select(Same::<Str>::new().and((&pc).and(&a).and((&cc).opt()))).window(row_number, |(t, ((n, (h, _)), _))| (Reverse(n), Reverse(h), t), asc);
    let v = drain((&w).filt(|(_, k)| k <= 10));
    rows(v.into_iter().map(|(_, ((t, ((n, (h, b)), c)), k))| {
        row(vec![
            V::S(t),
            V::I(n),
            V::I(h),
            V::I(c.unwrap_or(0)),
            oint(b),
            V::I(k),
            V::S(if n > 100 { "High Activity" } else if n >= 51 { "Moderate Activity" } else { "Low Activity" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1),
// UserDetails AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TaggedPosts AS (SELECT rp.PostId, unnest(string_to_array(rp.Tags, '>')) AS Tag FROM RankedPosts rp)
// SELECT rp.OwnerUserId AS UserId, ud.DisplayName, ud.Reputation, COUNT(tp.Tag) AS TagCount, SUM(CASE WHEN rp.Rank = 1 THEN 1 ELSE 0 END) AS LatestQuestionCount,
//        COUNT(DISTINCT rp.PostId) AS TotalQuestions, SUM(rp.ViewCount) AS TotalViews, SUM(rp.Score) AS TotalScore, MAX(rp.CreationDate) AS LastActivityDate
// FROM RankedPosts rp JOIN UserDetails ud ON rp.OwnerUserId = ud.UserId LEFT JOIN TaggedPosts tp ON rp.PostId = tp.PostId
// GROUP BY rp.OwnerUserId, ud.DisplayName, ud.Reputation ORDER BY TotalScore DESC, TotalViews DESC LIMIT 10;
//
// The sums run over the post x tag-piece rows (the split on '>' leaves a trailing empty piece). Rank breaks CreationDate ties by post id.
fn q27075(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, view_count, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let qs = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let pieces = tags_str.flat_map(|t: Str| t.split('>').collect::<Vec<_>>()).opt();
    let g = db
        .user
        .group_by(Ident::<User>::new())
        .select(qs.select(Ident::<Post>::new().with(&first).opt().and(view_count.opt()).and(score).and(creation_date).and(pieces)))
        .fold((0i64, 0i64, None::<i64>, 0i64, i64::MIN), |(n, l, w, s, m), ((((f, v), sc), d), t)| {
            (n + t.is_some() as i64, l + f.is_some() as i64, match v { Some(v) => Some(w.unwrap_or(0) + v), None => w }, s + sc, m.max(d))
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)))).fold(0i64, |n, _| n + 1);
    let v = drain((&g).and(&pc));
    let v = top_n(v, |&(u, ((_, _, w, s, _), _))| (Reverse(s), w.is_none(), Reverse(w), u), 10);
    rows(v.into_iter().map(|(u, ((n, l, w, s, m), q))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(l), V::I(q), oint(w), V::I(s), V::T(m)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS Ranking FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, P.Score, COUNT(C.Id) AS CommentCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY P.Id, P.OwnerUserId, P.Title, P.CreationDate, P.Score),
// TopUsers AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation FROM UserReputation UR WHERE UR.Ranking <= 10)
// SELECT T.DisplayName AS TopUser, RP.Title AS RecentPostTitle, RP.Score AS PostScore, RP.CommentCount,
//        COALESCE((SELECT STRING_AGG(DISTINCT C.Text, ', ') FROM Comments C WHERE C.PostId = RP.PostId), 'No comments') AS RecentComments
// FROM RecentPosts RP JOIN TopUsers T ON RP.OwnerUserId = T.UserId LEFT JOIN Votes V ON RP.PostId = V.PostId AND V.VoteTypeId = 2
// WHERE RP.Score > 10 ORDER BY RP.CreationDate DESC LIMIT 100;
//
// STRING_AGG(DISTINCT) is sorted.
fn q435(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(dense_rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let rp: MatSet<Id<Post>> = db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(score.gt(10))).with(owner_user.select(&tu)).collect();
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ct = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.text)).buf_fold(|it| {
        let mut v: Vec<Str> = it.into_iter().collect();
        v.sort();
        v.dedup();
        &*Box::leak(v.join(", ").into_boxed_str())
    });
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let v = drain((&cc).and((&ct).opt()).and(up.opt()).and(owner_user));
    let v = top_n(v, |&(p, ((_, x), _))| (Reverse(creation_date.get(p).unwrap()), p, x), 100);
    rows(v.into_iter().map(|(p, (((c, t), _), u))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), V::S(t.unwrap_or("No comments"))]);
        row(f)
    }))
}

/// SQL `s LIKE '%' || x || '%'`; when `x` holds no wildcard this is a plain substring test.
fn like_sub(s: &str, x: &str) -> bool {
    if x.contains('%') || x.contains('_') { like(s, &format!("%{x}%")) } else { s.contains(x) }
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes, MAX(v.CreationDate) AS LastVoteDate
//     FROM Votes v GROUP BY v.PostId),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(DISTINCT crt.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes crt ON ph.Comment = CAST(crt.Id AS varchar)
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(rv.Upvotes, 0) AS Upvotes, COALESCE(rv.Downvotes, 0) AS Downvotes, COALESCE(cp.CloseReasons, 'Not Closed') AS CloseReasons
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.Id = rv.PostId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId WHERE rp.Rank = 1 AND rp.ViewCount > 100
// ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 10;
//
// The NULL owner is a partition of its own. STRING_AGG(DISTINCT) is sorted.
fn q4249(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user_id, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(score.gt(0))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let rt = reason_by_text(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(comment.select(&rt)).buf_fold(|it| {
        let mut v: Vec<Str> = it.into_iter().collect();
        v.sort();
        v.dedup();
        &*Box::leak(v.join(", ").into_boxed_str())
    });
    let fr: MatSet<Id<Post>> = (&rp).with(view_count.gt(100)).collect();
    let rv = (&fr).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&fr).select((&rv).opt().and((&cp).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("Not Closed"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.LastActivityDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) AS RN FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// FilteredTags AS (SELECT PostId, UNNEST(string_to_array(substring(Tags, 2, length(Tags) - 2), '><')) AS Tag FROM RankedPosts WHERE RN = 1),
// TagCounts AS (SELECT Tag, COUNT(*) AS TagCount FROM FilteredTags GROUP BY Tag),
// MostPopularTags AS (SELECT Tag, TagCount, RANK() OVER (ORDER BY TagCount DESC) AS Rank FROM TagCounts WHERE TagCount > 0)
// SELECT pt.Name AS PostType, mpt.Tag, mpt.TagCount, COUNT(DISTINCT p.id) FILTER (WHERE p.AcceptedAnswerId IS NOT NULL) AS CountAcceptedAnswers, COUNT(DISTINCT c.id) AS CommentCount
// FROM MostPopularTags mpt JOIN Posts p ON p.Tags LIKE '%' || mpt.Tag || '%' JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON c.PostId = p.Id
// GROUP BY pt.Name, mpt.Tag, mpt.TagCount ORDER BY mpt.TagCount DESC, pt.Name;
//
// RN breaks LastActivityDate ties by post id (the SQL leaves them open). The LIKE goes through the distinct Tags strings.
fn q29817(db: &'static So) -> String {
    let Post { post_type_id, owner_user, last_activity_date, tags_str, accepted_answer_id, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(last_activity_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let tc = (&first).group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    type T = (Str, i64);
    let tv: MatSet<T> = whole(&tc).select(Same::<Str>::new().and(&tc)).collect();
    let tk: HashIdx<Str, T> = (&tv).map(|x: T| x.0).inv().collect();
    let strs: MatSet<Str> = db.post.select(tags_str).collect();
    let hit: HashIdx<Str, T> = (&strs).select_where(&tk, |s: Str, t: Str| like_sub(s, t)).collect();
    type K = (Str, T);
    let xs: MatSet<(Id<Post>, K)> = db.post.select(Ident::<Post>::new().and(ptype_name(db).and(tags_str.select(&hit)))).collect();
    type X = (Id<Post>, K);
    let keys: MatSet<K> = (&xs).map(|x: X| x.1).collect();
    let ca = (&xs).group_by(Same::<X>::new().map(|x: X| x.1)).select(Same::<X>::new().map(|x: X| x.0).select(Ident::<Post>::new().with(accepted_answer_id))).count_distinct();
    let cc = (&xs).group_by(Same::<X>::new().map(|x: X| x.1)).select(Same::<X>::new().map(|x: X| x.0).select(comments_of(db))).count_distinct();
    rows(drain((&keys).and((&ca).opt()).and((&cc).opt())).into_iter().map(|(_, (((n, (t, k)), a), c))| row(vec![V::S(n), V::S(t), V::I(k), V::I(a.unwrap_or(0)), V::I(c.unwrap_or(0))])))
}

// WITH PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagStatistics AS (SELECT pt.Tag, COUNT(pt.PostId) AS PostCount, COUNT(DISTINCT ph.UserId) AS EditorCount, AVG(u.Reputation) AS AvgReputation
//     FROM PostTags pt LEFT JOIN PostHistory ph ON ph.PostId = pt.PostId AND ph.PostHistoryTypeId IN (4, 5, 6) LEFT JOIN Users u ON u.Id = ph.UserId GROUP BY pt.Tag),
// RankedTags AS (SELECT Tag, PostCount, EditorCount, AvgReputation, RANK() OVER (ORDER BY PostCount DESC) AS RankByPostCount, RANK() OVER (ORDER BY EditorCount DESC) AS RankByEditorCount,
//        RANK() OVER (ORDER BY AvgReputation DESC) AS RankByAvgReputation FROM TagStatistics)
// SELECT rt.Tag, rt.PostCount, rt.EditorCount, rt.AvgReputation,
//        CASE WHEN rt.RankByPostCount <= 5 THEN 'Top 5 by Posts' WHEN rt.RankByEditorCount <= 5 THEN 'Top 5 by Editors' WHEN rt.RankByAvgReputation <= 5 THEN 'Top 5 by Reputation' ELSE 'Other' END AS Category
// FROM RankedTags rt ORDER BY rt.RankByPostCount, rt.RankByEditorCount, rt.RankByAvgReputation;
//
// COUNT(pt.PostId) and AVG(u.Reputation) run over the post x edit rows.
fn q26922(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let PostHistory { post_history_type_id, user_id, user, .. } = &db.post_history;
    let qs = || db.post.with(post_type_id.eq(1));
    let edits = || history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6])));
    let ts_ = qs().group_by(tags_str.flat_map(tag_list)).select(edits().select(user.select(&db.user.reputation).opt()).opt()).fold([0i64; 3], |a, r| match r.flatten() {
        Some(r) => [a[0] + 1, a[1] + 1, a[2] + r],
        None => [a[0] + 1, a[1], a[2]],
    });
    let ec = qs().group_by(tags_str.flat_map(tag_list)).select(edits().select(user_id)).count_distinct();
    type R = ((Str, [i64; 3]), Option<i64>);
    let ar = |a: [i64; 3]| if a[1] == 0 { None } else { Some(fkey(a[2] as f64 / a[1] as f64)) };
    let w = whole(&ts_).select(Same::<Str>::new().and(&ts_).and((&ec).opt())).window(rank, |((_, a), _)| Reverse(a[0]), asc);
    let w = (&w).window(rank, |((_, e), _): (R, i64)| Reverse(e.unwrap_or(0)), asc);
    let w = (&w).window(rank, |((((_, a), _), _), _): ((R, i64), i64)| (ar(a).is_none(), Reverse(ar(a))), asc);
    rows(drain(&w).into_iter().map(|(_, (((((t, a), e), r1), r2), r3))| {
        row(vec![
            V::S(t),
            V::I(a[0]),
            V::I(e.unwrap_or(0)),
            if a[1] == 0 { V::Null } else { V::F(a[2] as f64 / a[1] as f64) },
            V::S(if r1 <= 5 { "Top 5 by Posts" } else if r2 <= 5 { "Top 5 by Editors" } else if r3 <= 5 { "Top 5 by Reputation" } else { "Other" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.DisplayName),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, '; ') AS Comments FROM Comments c GROUP BY c.PostId)
// SELECT rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, us.DisplayName, us.BadgeCount, us.TotalUpvotes, us.TotalDownvotes, pc.CommentCount, pc.Comments
// FROM RankedPosts rp LEFT JOIN UserStats us ON rp.Id = us.UserId LEFT JOIN PostComments pc ON rp.Id = pc.PostId WHERE rp.Rank <= 5
// ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10;
//
// `rp.Id = us.UserId` joins a post id to a user id, so it goes through the raw ids; only the users so named are aggregated. Rank breaks Score ties
// by post id (the SQL leaves them open). STRING_AGG is given in comment id order.
fn q3041(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let us: MatSet<Id<User>> = (&tp).select(origid.select(&uidx)).collect();
    let st = (&us)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (b, t)| [a[0] + b.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().and(&db.comment.text))).buf_fold(|it| {
        let mut v: Vec<(Id<Comment>, Str)> = it.into_iter().collect();
        v.sort();
        let s: Vec<Str> = v.iter().map(|x| x.1).collect();
        (v.len() as i64, &*Box::leak(s.join("; ").into_boxed_str()))
    });
    let v = drain((&tp).select(origid.select(&uidx).select(Ident::<User>::new().and(&st)).opt().and((&pc).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (u, c))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(match u {
            Some((u, a)) => [user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, OwnerDisplayName, CommentCount, VoteCount FROM RankedPosts WHERE PostRank <= 10)
// SELECT tp.Title, tp.OwnerDisplayName, tp.Score, tp.CommentCount, tp.VoteCount, COALESCE(ARRAY_AGG(DISTINCT t.TagName), ARRAY[]::TEXT[]) AS Tags
// FROM TopPosts tp LEFT JOIN Posts p ON tp.PostId = p.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.Id
// GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.CommentCount, tp.VoteCount, tp.CreationDate ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// PostRank reads only base columns, so the posts are picked first; it breaks ties by post id. ARRAY_AGG(DISTINCT) is sorted, a NULL element last.
fn q6817(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vu = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let cv = (&cc).and((&vu).opt()).map(|(c, n): (i64, Option<i64>)| (c, n.unwrap_or(0)));
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&tp).group_by(Ident::<Post>::new()).select(excerpt.select(&db.tag.tag_name).opt()).buf_fold(|it| {
        let mut t: Vec<Option<Str>> = it.into_iter().collect();
        t.sort_by_key(|x| (x.is_none(), *x));
        t.dedup();
        &*Box::leak(t.into_boxed_slice())
    });
    let v = drain((&cv).and(&tg));
    rows(v.into_iter().map(|(p, ((c, n), t))| {
        let mut f = post_fields(db, p, &["title", "owner", "score"]);
        f.extend([V::I(c), V::I(n), V::L(t.iter().map(|x| ostr(*x)).collect())]);
        row(f)
    }))
}

// Rewritten (rewrites/9533.sql): the tag STRING_AGG ordered by TagName.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.PostRank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpVotes, tp.DownVotes,
//        COALESCE((SELECT STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) FROM Posts p JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%' WHERE p.Id = tp.PostId), 'No Tags') AS Tags
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// PostRank reads only base columns, so the posts are picked first; it breaks CreationDate ties by post id.
fn q9533(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let lt = tag_mentions(db);
    let tags_of: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&lt).map(|(p, _)| p).inv().collect();
    let tg = (&tp).group_by(Ident::<Post>::new()).select((&tags_of).map(|(_, t)| t).select(&db.tag.tag_name)).buf_fold(|it| {
        let mut v: Vec<Str> = it.into_iter().collect();
        v.sort();
        &*Box::leak(v.join(", ").into_boxed_str())
    });
    let v = drain((&rp).and((&tg).opt()));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t.unwrap_or("No Tags"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, p.PostTypeId),
// CloseReasonDetails AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, COALESCE(cr.CloseReasons, 'No Closures') AS CloseReasons
// FROM RankedPosts rp LEFT JOIN CloseReasonDetails cr ON rp.PostId = cr.PostId WHERE rp.RankScore <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// RankScore reads only base columns, so the posts are picked first. STRING_AGG is given in history id order.
fn q2088(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|it| {
            let mut v: Vec<(Id<PostHistory>, Str)> = it.into_iter().collect();
            v.sort();
            let s: Vec<Str> = v.iter().map(|x| x.1).collect();
            &*Box::leak(s.join(", ").into_boxed_str())
        });
    let v = drain((&rp).and((&cr).opt()));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(c.unwrap_or("No Closures"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.PostRank <= 5),
// PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(p.Tags, '><')) AS Tag FROM Posts p)
// SELECT trp.Title, trp.CreationDate, trp.OwnerDisplayName, trp.Score, trp.ViewCount, pt.Tag, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM TopRankedPosts trp LEFT JOIN Comments c ON c.PostId = trp.PostId LEFT JOIN Votes v ON v.PostId = trp.PostId LEFT JOIN PostTags pt ON pt.PostId = trp.PostId
// GROUP BY trp.PostId, trp.Title, trp.CreationDate, trp.OwnerDisplayName, trp.Score, trp.ViewCount, pt.Tag ORDER BY trp.Score DESC;
//
// PostRank reads only base columns, so the posts are picked first; it breaks ties by post id. The rows are grouped by (post, tag piece).
fn q8815(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let g = (&tp)
        .group_by(Ident::<Post>::new().and(tags_str.flat_map(|t: Str| t.split("><")).opt()))
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&g).into_iter().map(|((p, t), a)| {
        let mut f = post_fields(db, p, &["title", "created", "owner", "score", "views"]);
        f.extend([ostr(t), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH PostTagCounts AS (SELECT p.Id AS PostId, COUNT(DISTINCT t.TagName) AS UniqueTagCount, SUM(LENGTH(p.Body) - LENGTH(REPLACE(p.Body, ' ', ''))) + 1 AS WordCount
//     FROM Posts p LEFT JOIN unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '> <')) AS t(TagName) ON TRUE WHERE p.PostTypeId = 1 GROUP BY p.Id),
// HighTagWordCountPosts AS (SELECT ptc.PostId, ptc.UniqueTagCount, ptc.WordCount, p.Title, p.Score, u.DisplayName AS OwnerDisplayName FROM PostTagCounts ptc
//     JOIN Posts p ON ptc.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id WHERE ptc.UniqueTagCount > 5 AND ptc.WordCount > 100),
// MostCommentedPosts AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId HAVING COUNT(*) >= 5),
// FinalBenchmarkResults AS (SELECT h.Title, h.OwnerDisplayName, h.UniqueTagCount, h.WordCount, COALESCE(mc.CommentCount, 0) AS TotalComments FROM HighTagWordCountPosts h
//     LEFT JOIN MostCommentedPosts mc ON h.PostId = mc.PostId)
// SELECT *, RANK() OVER (ORDER BY TotalComments DESC, UniqueTagCount DESC) AS PostRank FROM FinalBenchmarkResults ORDER BY PostRank LIMIT 10;
//
// The word count is summed over the tag-piece rows. LENGTH counts characters.
fn q25154(db: &'static So) -> String {
    let Post { post_type_id, tags_str, body, owner_user, .. } = &db.post;
    let pieces = || tags_str.flat_map(|t: Str| {
        let mut c = t.chars();
        let a = c.next().map_or(0, |x| x.len_utf8());
        let b = c.next_back().map_or(0, |x| x.len_utf8());
        t[a..t.len() - b].split("> <").collect::<Vec<_>>()
    });
    let qs = || db.post.with(post_type_id.eq(1));
    let wc = qs().group_by(Ident::<Post>::new()).select(pieces().opt().and(body)).fold(0i64, |n, (_, b)| n + (b.chars().count() - b.chars().filter(|&c| c != ' ').count()) as i64);
    let tc = qs().group_by(Ident::<Post>::new()).select(pieces()).count_distinct();
    let ptc = (&wc).and((&tc).opt()).map(|(w, u): (i64, Option<i64>)| (u.unwrap_or(0), w + 1));
    let mc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let fb = (&ptc).filt(|(u, w)| u > 5 && w > 100).and(owner_user).and((&mc).filt(|n| n >= 5).opt());
    let w = whole(&fb).select(Ident::<Post>::new().and(&fb)).window(rank, |(_, (((u, _), _), c))| (Reverse(c.unwrap_or(0)), Reverse(u)), asc);
    let r = top_n(drain(&w).into_iter().map(|x| x.1).collect(), |&((p, _), k)| (k, p), 10);
    rows(r.into_iter().map(|((p, (((u, w), o), c)), k)| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([user_col(db, o, "name"), V::I(u), V::I(w), V::I(c.unwrap_or(0)), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplayName, rp.Tags FROM RankedPosts rp WHERE rp.PostRank <= 5),
// TagsExploded AS (SELECT tp.PostId, unnest(string_to_array(tp.Tags, '><')) AS Tag FROM TopPosts tp),
// TagCount AS (SELECT Tag, COUNT(*) AS PostCount FROM TagsExploded GROUP BY Tag)
// SELECT t.Tag, t.PostCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, (SELECT COUNT(*) FROM Comments c WHERE c.PostId IN (SELECT PostId FROM TopPosts)) AS TotalComments
// FROM TagCount t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.Tag || '%' LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
// GROUP BY t.Tag, t.PostCount ORDER BY t.PostCount DESC;
//
// PostRank breaks CreationDate ties by post id (the SQL leaves them open). The LIKE goes through the distinct Tags strings; TotalComments is a
// separate query.
fn q28425(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1)))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let tc = (&tp).group_by(tags_str.flat_map(|t: Str| t.split("><"))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    type T = (Str, i64);
    let tv: MatSet<T> = whole(&tc).select(Same::<Str>::new().and(&tc)).collect();
    let tidx: HashIdx<Str, T> = (&tv).map(|x: T| x.0).inv().collect();
    let strs: MatSet<Str> = db.post.select(tags_str).collect();
    let hit: HashIdx<Str, Str> = (&strs).select_where(&tidx, |s: Str, t: Str| like_sub(s, t)).map(|x: T| x.0).collect();
    let by_tag: HashIdx<Str, Id<Post>> = db.post.select(tags_str.select(&hit)).inv().collect();
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let b = whole(&tc).group_by(Same::<Str>::new()).select(by_tag.select(bv.opt()).opt()).fold(0i64, |n, x| n + x.flatten().flatten().unwrap_or(0));
    let total = count((&tp).select(comments_of(db)));
    let v = drain((&tc).and(&b));
    rows(v.into_iter().map(|(t, (n, s))| row(vec![V::S(t), V::I(n), V::I(s), V::I(total)])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000),
// PostVoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// ClosedPostHistory AS (SELECT ph.PostId, STRING_AGG(DISTINCT cht.Name, ', ') AS CloseReasons, COUNT(*) AS CloseCount FROM PostHistory ph
//     INNER JOIN CloseReasonTypes cht ON CAST(ph.Comment AS INTEGER) = cht.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.Title, rp.CreationDate, rp.Score, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes, COALESCE(cph.CloseReasons, 'No Close Reasons') AS CloseReasons,
//        COALESCE(cph.CloseCount, 0) AS CloseCount
// FROM RankedPosts rp LEFT JOIN PostVoteCounts pvc ON rp.Id = pvc.PostId LEFT JOIN ClosedPostHistory cph ON rp.Id = cph.PostId WHERE rp.PostRank = 1
// ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 10;
//
// PostRank breaks Score ties by post id (the SQL leaves them open). STRING_AGG(DISTINCT) is sorted.
fn q373(db: &'static So) -> String {
    let Post { score, creation_date, owner_user, .. } = &db.post;
    let w = db.post.with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let first = top_n(drain((&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p)), |&(_, p)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.1).collect()).collect();
    let pvc = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cph = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|it| {
            let mut v: Vec<Str> = it.into_iter().collect();
            let n = v.len() as i64;
            v.sort();
            v.dedup();
            (n, &*Box::leak(v.join(", ").into_boxed_str()))
        });
    let v = drain((&rp).select((&pvc).opt().and((&cph).opt())));
    rows(v.into_iter().map(|(p, (a, c))| {
        let a = a.unwrap_or([0, 0]);
        let (n, s) = c.unwrap_or((0, "No Close Reasons"));
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(s), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(AVG(v.BountyAmount), 0) AS AverageBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, ' | ') AS AllComments FROM Comments c GROUP BY c.PostId)
// SELECT up.DisplayName, up.Reputation, rp.PostId, rp.Title, rp.CreationDate, rp.Score, COALESCE(pc.CommentCount, 0) AS TotalComments, pc.AllComments, ur.AverageBounty,
//        CASE WHEN rp.PostRank = 1 THEN 'Latest Post' ELSE 'Previous Post' END AS PostStatus
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id LEFT JOIN PostComments pc ON rp.PostId = pc.PostId JOIN UserReputation ur ON up.Id = ur.UserId
// WHERE ur.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY rp.CreationDate DESC LIMIT 50;
//
// PostRank breaks CreationDate ties by post id (the SQL leaves them open). The average reputation is a separate query. STRING_AGG is given in comment id order.
fn q20495(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    type R = ((Id<Post>, i64), i64);
    let rv: MatSet<R> = (&w).collect();
    let (s, n) = (&db.user.reputation).fold_flat((0i128, 0i64), |(s, n), r| (s + r as i128, n + 1));
    let avg_rep = s as f64 / n as f64;
    let good = Ident::<User>::new().with((&db.user.reputation).filt(move |r: i64| r as f64 > avg_rep));
    type Q = (R, Id<User>);
    let rp: MatSet<Q> = (&rv).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(owner_user.select(good)))).collect();
    let top = top_n(drain(&rp), |&(_, (((p, _), _), _))| (Reverse(creation_date.get(p).unwrap()), p), 50);
    let tv = rel(top.into_iter().map(|x| x.1).collect());
    let posts: MatSet<Id<Post>> = (&tv).map(|x: Q| x.0 .0 .0).collect();
    let users: MatSet<Id<User>> = (&tv).map(|x: Q| x.1).collect();
    let pc = (&posts).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().and(&db.comment.text))).buf_fold(|it| {
        let mut v: Vec<(Id<Comment>, Str)> = it.into_iter().collect();
        v.sort();
        let s: Vec<Str> = v.iter().map(|x| x.1).collect();
        (v.len() as i64, &*Box::leak(s.join(" | ").into_boxed_str()))
    });
    let ab = (&users).group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold((0i64, 0i64), |(n, s), b| match b.flatten() {
        Some(b) => (n + 1, s + b),
        None => (n, s),
    });
    let v = drain((&tv).select(Same::<Q>::new().and(Same::<Q>::new().map(|x: Q| x.0 .0 .0).select(&pc).opt()).and(Same::<Q>::new().map(|x: Q| x.1).select(&ab))));
    rows(v.into_iter().map(|(_, (((((p, _), k), u), c), (n, s)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend(match c {
            Some((m, t)) => [V::I(m), V::S(t)],
            None => [V::I(0), V::Null],
        });
        f.push(V::F(if n == 0 { 0.0 } else { s as f64 / n as f64 }));
        f.push(V::S(if k == 1 { "Latest Post" } else { "Previous Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentPosts AS (SELECT Id, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE PostRank <= 3),
// TopBadgers AS (SELECT U.DisplayName, COUNT(B.Id) AS BadgeCount FROM Users U JOIN Badges B ON U.Id = B.UserId GROUP BY U.DisplayName HAVING COUNT(B.Id) > 5)
// SELECT RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, TB.BadgeCount,
//        CASE WHEN RP.Score > 100 THEN 'High Score' WHEN RP.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory,
//        COALESCE((SELECT STRING_AGG(T.TagName, ', ') FROM Tags T WHERE T.ExcerptPostId = RP.Id), 'No Tags') AS TagsUsed
// FROM RecentPosts RP LEFT JOIN TopBadgers TB ON RP.OwnerDisplayName = TB.DisplayName ORDER BY RP.CreationDate DESC;
//
// PostRank breaks CreationDate ties by post id (the SQL leaves them open). TopBadgers groups by the name, so namesakes pool their badges.
// STRING_AGG is given in tag id order.
fn q3133(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let tb = db.user.group_by(&db.user.display_name).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&rp).group_by(Ident::<Post>::new()).select(excerpt.select(Ident::<Tag>::new().and(&db.tag.tag_name))).buf_fold(|it| {
        let mut v: Vec<(Id<Tag>, Str)> = it.into_iter().collect();
        v.sort();
        let s: Vec<Str> = v.iter().map(|x| x.1).collect();
        &*Box::leak(s.join(", ").into_boxed_str())
    });
    let v = drain((&rp).select(owner_user.select(&db.user.display_name).select((&tb).filt(|n| n > 5)).opt().and((&tg).opt())));
    rows(v.into_iter().map(|(p, (b, t))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([oint(b), V::S(if s > 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" }), V::S(t.unwrap_or("No Tags"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, p.Tags, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year')),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, STRING_AGG(DISTINCT ctr.Name, ', ') AS Reasons FROM PostHistory ph JOIN CloseReasonTypes ctr ON CAST(ph.Comment AS int) = ctr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.Title, rp.Score, ur.DisplayName AS PostOwner, ur.Reputation AS OwnerReputation, ur.TotalPosts, ur.PositivePosts, cp.CloseCount, cp.Reasons AS CloseReasons
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.ScoreRank <= 5
// ORDER BY rp.CreationDate DESC LIMIT 50;
//
// ScoreRank reads only base columns, so the posts are picked first. STRING_AGG(DISTINCT) is sorted.
fn q1105(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ur = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + (s > 0) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)).buf_fold(|it| {
        let mut v: Vec<Str> = it.into_iter().collect();
        let n = v.len() as i64;
        v.sort();
        v.dedup();
        (n, &*Box::leak(v.join(", ").into_boxed_str()))
    });
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ur)).and((&cp).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((u, a), c))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match c {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyEarned, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostDetails AS (SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.ViewCount, us.UserId, us.Reputation, us.TotalBountyEarned, us.BadgeCount
//     FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.Rank <= 3)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.Reputation, pd.TotalBountyEarned, pd.BadgeCount,
//        (SELECT STRING_AGG(t.TagName, ', ') FROM Tags t JOIN Posts p ON t.ExcerptPostId = p.Id WHERE p.Id = pd.PostId) AS Tags
// FROM PostDetails pd ORDER BY pd.ViewCount DESC LIMIT 10;
//
// Rank and the order read only base columns, so the ten posts are picked first and only their owners are aggregated; Rank breaks Score ties by
// post id (the SQL leaves them open). The bounty sum runs over the vote x badge rows. STRING_AGG is given in tag id order.
fn q3065(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let top = top_n(drain((&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p)).into_iter().map(|(u, p)| (p, u)).collect(), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let bs = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()))
        .fold(0i64, |n, (b, _)| n + b.flatten().unwrap_or(0));
    let bc = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&tp).group_by(Ident::<Post>::new()).select(excerpt.select(Ident::<Tag>::new().and(&db.tag.tag_name))).buf_fold(|it| {
        let mut v: Vec<(Id<Tag>, Str)> = it.into_iter().collect();
        v.sort();
        let s: Vec<Str> = v.iter().map(|x| x.1).collect();
        &*Box::leak(s.join(", ").into_boxed_str())
    });
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&bs).and(&bc)).and((&tg).opt())));
    rows(v.into_iter().map(|(p, (((u, b), c), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([user_col(db, u, "rep"), V::I(b), V::I(c), ostr(t)]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, P.ViewCount, P.AcceptedAnswerId,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn FROM Posts P WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserVoteStatistics AS (SELECT U.Id AS UserId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id),
// TopTags AS (SELECT T.TagName, COUNT(P.Id) AS PostCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName ORDER BY PostCount DESC LIMIT 5)
// SELECT RP.PostId, RP.Title, RP.CreationDate, U.DisplayName AS OwnerDisplayName, U.Reputation, UVS.TotalVotes, UVS.UpVotes, UVS.DownVotes,
//        (SELECT STRING_AGG(TT.TagName, ', ') FROM TopTags TT) AS TopTags
// FROM RecentPosts RP JOIN Users U ON RP.OwnerUserId = U.Id LEFT JOIN UserVoteStatistics UVS ON U.Id = UVS.UserId WHERE RP.rn = 1 AND RP.AcceptedAnswerId IS NOT NULL
// ORDER BY RP.Score DESC LIMIT 10;
//
// rn breaks CreationDate ties by post id (the SQL leaves them open). The top-tag list is a separate query, joined in PostCount order.
fn q3302(db: &'static So) -> String {
    let Post { creation_date, owner_user, accepted_answer_id, score, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let rp = drain((&rp).with(accepted_answer_id).select(owner_user));
    let rp = top_n(rp, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    let tv = rel(rp);
    type T = (Id<Post>, Id<User>);
    let users: MatSet<Id<User>> = (&tv).map(|x: T| x.1).collect();
    let uvs = (&users).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let pcount = db.tag.group_by(&db.tag.tag_name).select(&by_tag).fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&pcount), |&(t, n)| (Reverse(n), t), 5);
    let names = tt.iter().map(|x| x.0).collect::<Vec<Str>>().join(", ");
    let v = drain((&tv).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.1).select(&uvs))));
    rows(v.into_iter().map(|(_, ((p, u), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::Owned(names.clone())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostsWithCloseReasons AS (SELECT ph.PostId, STRING_AGG(ct.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes ct ON ph.Comment = CAST(ct.Id AS TEXT)
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT up.DisplayName AS UserDisplayName, up.Reputation, rp.Title, rp.Score, ub.BadgeCount, ub.GoldBadgeCount, ub.SilverBadgeCount, ub.BronzeBadgeCount,
//        COALESCE(pc.CloseReasons, 'No close reasons') AS CloseReasons
// FROM Users up JOIN RankedPosts rp ON up.Id = rp.OwnerUserId LEFT JOIN UserBadges ub ON up.Id = ub.UserId LEFT JOIN PostsWithCloseReasons pc ON rp.Id = pc.PostId
// WHERE rp.PostRank = 1 AND up.Reputation > 1000 ORDER BY ub.BadgeCount DESC, rp.Score DESC LIMIT 10;
//
// STRING_AGG is given in history id order.
fn q2283(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(score).and(owner_user)).window(rank, |((_, s), _)| Reverse(s), asc);
    type F = (Id<Post>, Id<User>);
    let fv: MatSet<F> = (&w).filt(|(_, k)| k == 1).map(|(((p, _), u), _)| (p, u)).with(Same::<F>::new().map(|x: F| x.1).with((&db.user.reputation).gt(1000))).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let rt = reason_by_text(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let pc = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(Ident::<PostHistory>::new().and(comment.select(&rt))).buf_fold(|it| {
        let mut v: Vec<(Id<PostHistory>, Str)> = it.into_iter().collect();
        v.sort();
        let s: Vec<Str> = v.iter().map(|x| x.1).collect();
        &*Box::leak(s.join(", ").into_boxed_str())
    });
    let v = drain((&fv).select(Same::<F>::new().and(Same::<F>::new().map(|x: F| x.1).select(&ub)).and(Same::<F>::new().map(|x: F| x.0).select(&pc).opt())));
    let v = top_n(v, |&(_, (((p, u), a), _))| (Reverse(a[0]), Reverse(score.get(p).unwrap()), p, u), 10);
    rows(v.into_iter().map(|(_, (((p, u), a), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::S(c.unwrap_or("No close reasons"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) as Rank,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.UpVoteCount, tp.DownVoteCount,
//        CASE WHEN tp.UpVoteCount > tp.DownVoteCount THEN 'Positive' WHEN tp.UpVoteCount < tp.DownVoteCount THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment,
//        COALESCE((SELECT STRING_AGG(t.TagName, ', ') FROM Tags t JOIN Posts p ON t.ExcerptPostId = p.Id WHERE p.Id = tp.PostId), 'No Tags') AS Tags
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11) ORDER BY tp.Score DESC LIMIT 10;
//
// Rank breaks Score ties by post id (the SQL leaves them open). The WHERE on ph makes the history join inner. STRING_AGG is given in tag id order.
fn q1458(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(ptype_name(db)).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&tp).group_by(Ident::<Post>::new()).select(excerpt.select(Ident::<Tag>::new().and(&db.tag.tag_name))).buf_fold(|it| {
        let mut v: Vec<(Id<Tag>, Str)> = it.into_iter().collect();
        v.sort();
        let s: Vec<Str> = v.iter().map(|x| x.1).collect();
        &*Box::leak(s.join(", ").into_boxed_str())
    });
    let ph = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let v = drain((&vc).and((&tg).opt()).and(ph));
    let v = top_n(v, |&(p, (_, h))| (Reverse(score.get(p).unwrap()), p, h), 10);
    rows(v.into_iter().map(|(p, ((a, t), _))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        f.push(V::S(t.unwrap_or("No Tags")));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, p.PostTypeId,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'),
// PostVoteCounts AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// ClosedPostReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes,
//        COALESCE(cpr.CloseReasons, 'No reasons') AS CloseReasons
// FROM RecentPosts rp LEFT JOIN PostVoteCounts pvc ON rp.PostId = pvc.PostId LEFT JOIN ClosedPostReasons cpr ON rp.PostId = cpr.PostId WHERE rp.rn <= 5 ORDER BY rp.CreationDate DESC;
//
// rn breaks CreationDate ties by post id (the SQL leaves them open). STRING_AGG is given in history id order.
fn q3481(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let pvc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cpr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|it| {
            let mut v: Vec<(Id<PostHistory>, Str)> = it.into_iter().collect();
            v.sort();
            let s: Vec<Str> = v.iter().map(|x| x.1).collect();
            &*Box::leak(s.join(", ").into_boxed_str())
        });
    let v = drain((&tp).select((&pvc).opt().and((&cpr).opt())));
    rows(v.into_iter().map(|(p, (a, c))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("No reasons"))]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(DISTINCT P.Id) AS TotalPosts, AVG(COALESCE(P.Score, 0)) AS AvgPostScore
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON P.OwnerUserId = U.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalVotes, Upvotes, Downvotes, TotalPosts, AvgPostScore, RANK() OVER (ORDER BY TotalVotes DESC) AS UserRank FROM UserVoteStats)
// SELECT T.DisplayName, T.TotalVotes, T.Upvotes, T.Downvotes, T.TotalPosts, T.AvgPostScore,
//        COALESCE(CASE WHEN T.UserRank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END, 'Unknown') AS ContributorType
// FROM TopUsers T WHERE T.TotalVotes > 5
// UNION ALL
// SELECT U.DisplayName, 0 AS TotalVotes, 0 AS Upvotes, 0 AS Downvotes, 0 AS TotalPosts, 0 AS AvgPostScore, 'Inactive Contributor' AS ContributorType
// FROM Users U WHERE NOT EXISTS (SELECT 1 FROM Votes V WHERE V.UserId = U.Id) AND U.LastAccessDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years'
// ORDER BY TotalVotes DESC NULLS LAST;
//
// The counts and the average run over the vote x post rows.
fn q570(db: &'static So) -> String {
    let uvs = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(&db.post.score).opt()))
        .fold([0i64; 5], |a, (t, s)| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + s.unwrap_or(0), a[4] + 1]);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&uvs).select(Ident::<User>::new().and(&uvs).and(&pc)).window(rank, |((_, a), _)| Reverse(a[0]), asc);
    let v = drain((&w).filt(|(((_, a), _), _)| a[0] > 5));
    let mut out: Vec<String> = v
        .into_iter()
        .map(|(_, (((u, a), n), k))| {
            row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), V::F(a[3] as f64 / a[4] as f64), V::S(if k <= 10 { "Top Contributor" } else { "Regular Contributor" })])
        })
        .collect();
    let inactive = drain(db.user.minus(votes_by(db)).with((&db.user.last_access_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -2))));
    out.extend(inactive.into_iter().map(|(u, _)| row(vec![user_col(db, u, "name"), V::I(0), V::I(0), V::I(0), V::I(0), V::F(0.0), V::S("Inactive Contributor")])));
    rows(out)
}

// WITH UserScores AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.UpVotes, u.DownVotes, (u.UpVotes - u.DownVotes) AS NetVotes FROM Users u WHERE u.Reputation > 1000),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.Score AS PostScore, p.ViewCount, p.AnswerCount, p.CreationDate, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserWithTopPosts AS (SELECT us.UserId, us.DisplayName, us.Reputation, tp.PostId, tp.Title, tp.PostScore, tp.ViewCount, tp.AnswerCount, tp.CreationDate
//     FROM UserScores us JOIN TopPosts tp ON us.UserId = tp.OwnerUserId WHERE tp.PostRank <= 3)
// SELECT tp.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(b.Class, 0)) AS TotalBadges, AVG(tp.PostScore) AS AveragePostScore, SUM(tp.ViewCount) AS TotalViews,
//        STRING_AGG(DISTINCT tp.Title, '; ') AS TopPostTitles
// FROM UserWithTopPosts tp LEFT JOIN Posts p ON tp.PostId = p.Id LEFT JOIN Badges b ON tp.UserId = b.UserId GROUP BY tp.DisplayName
// ORDER BY TotalPosts DESC, AveragePostScore DESC LIMIT 10;
//
// PostRank breaks Score ties by post id (the SQL leaves them open). The sums run over the post x badge rows, grouped by the name, so namesakes pool.
// STRING_AGG(DISTINCT) is sorted.
fn q7058(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, view_count, title, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(score).and(owner_user)).window(row_number, |((p, s), _)| (Reverse(s), p), asc);
    type T = (Id<Post>, Id<User>);
    let uw: MatSet<T> = (&w).filt(|(_, r)| r <= 3).map(|(((p, _), u), _)| (p, u)).with(Same::<T>::new().map(|x: T| x.1).with((&db.user.reputation).gt(1000))).collect();
    let name = || Same::<T>::new().map(|x: T| x.1).select(&db.user.display_name);
    let g = (&uw)
        .group_by(name())
        .select(Same::<T>::new().map(|x: T| x.0).select(score.and(view_count.opt()).and(title.opt())).and(Same::<T>::new().map(|x: T| x.1).select(badges_of(db).select(&db.badge.class).opt())))
        .buf_fold(|it| {
            let v: Vec<(((i64, Option<i64>), Option<Str>), Option<i64>)> = it.into_iter().collect();
            let n = v.len() as i64;
            let b: i64 = v.iter().map(|x| x.1.unwrap_or(0)).sum();
            let s: i64 = v.iter().map(|x| x.0 .0 .0).sum();
            let w: Vec<i64> = v.iter().filter_map(|x| x.0 .0 .1).collect();
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.0 .1).collect();
            t.sort();
            t.dedup();
            let ts_ = if t.is_empty() { None } else { Some(&*Box::leak(t.join("; ").into_boxed_str())) };
            (b, s as f64 / n as f64, if w.is_empty() { None } else { Some(w.iter().sum::<i64>()) }, ts_)
        });
    let pc = (&uw).group_by(name()).select(Same::<T>::new().map(|x: T| x.0)).count_distinct();
    let v = top_n(drain((&pc).and(&g)), |&(nm, (ps, (_, a, _, _)))| (Reverse(ps), Reverse(fkey(a)), nm), 10);
    rows(v.into_iter().map(|(nm, (ps, (b, a, w, t)))| row(vec![V::S(nm), V::I(ps), V::I(b), V::F(a), oint(w), ostr(t)])))
}

// WITH PostTagCounts AS (SELECT p.Id AS PostId, p.Title, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagCounts AS (SELECT Tag, COUNT(PostId) AS PostCount FROM PostTagCounts GROUP BY Tag),
// PopularTags AS (SELECT Tag, PostCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM TagCounts WHERE PostCount >= 10),
// UsersEngaged AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ue.QuestionCount, ue.UpVotes, ue.DownVotes FROM Users u JOIN UsersEngaged ue ON u.Id = ue.OwnerUserId)
// SELECT upr.DisplayName, upr.Reputation, upr.QuestionCount, upr.UpVotes, upr.DownVotes, pt.Tag AS PopularTag
// FROM UserReputation upr JOIN PopularTags pt ON upr.QuestionCount >= 5 ORDER BY upr.Reputation DESC, upr.QuestionCount DESC;
//
// `ON upr.QuestionCount >= 5` names only UserReputation, so those users cross every popular tag. Rank is never read.
fn q27917(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let tc = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let ue = db.user.group_by(Ident::<User>::new()).select(qs().select(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let qc = db.user.group_by(Ident::<User>::new()).select(qs()).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&qc).filt(|n| n >= 5).and(&ue).cross((&tc).filt(|n| n >= 10)).drive(|(u, t), ((n, a), _)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::S(t)]);
        out.push(row(f));
    });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(au.UpVotes, 0) AS UpVotes, COALESCE(au.DownVotes, 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Users au ON p.OwnerUserId = au.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS ClosedDate, STRING_AGG(ct.Name, ', ') AS CloseReasons FROM PostHistory ph
//     JOIN CloseReasonTypes ct ON CAST(ph.Comment AS INTEGER) = ct.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// PostSummary AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.UpVotes, rp.DownVotes, cp.ClosedDate, cp.CloseReasons, RANK() OVER (ORDER BY rp.Score DESC) AS OverallRank
//     FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.UpVotes, ps.DownVotes, ps.ClosedDate, ps.CloseReasons, ps.OverallRank
// FROM PostSummary ps WHERE ps.ClosedDate IS NOT NULL ORDER BY ps.OverallRank, ps.Score DESC;
//
// UpVotes and DownVotes are the owner's Users columns. Rank is never read. STRING_AGG is given in history id order.
fn q8595(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let w = whole(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(hd).and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|it| {
            let mut v: Vec<((Id<PostHistory>, i64), Str)> = it.into_iter().collect();
            v.sort();
            let m = v.iter().map(|x| x.0 .1).max().unwrap();
            let s: Vec<Str> = v.iter().map(|x| x.1).collect();
            (m, &*Box::leak(s.join(", ").into_boxed_str()))
        });
    type R = ((Id<Post>, i64), i64);
    let rv: MatSet<R> = (&w).collect();
    let v = drain((&rv).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(&cp))));
    rows(v.into_iter().map(|(_, (((p, _), k), (d, s)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        match owner_user.get(p) {
            Some(u) => f.extend(ucols(db, u, &["uup", "udown"])),
            None => f.extend([V::I(0), V::I(0)]),
        }
        f.extend([V::T(d), V::S(s), V::I(k)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, COUNT(C.Id) AS CommentCount, STRING_AGG(DISTINCT T.TagName, ', ') AS Tags
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN LATERAL unnest(string_to_array(P.Tags, ',')) AS Tag(Tag) ON TRUE
//     LEFT JOIN Tags T ON TRIM(BOTH ' ' FROM Tag) = T.TagName WHERE P.CreationDate > DATE '2024-10-01' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.ViewCount, P.Score
//     ORDER BY P.ViewCount DESC LIMIT 10)
// SELECT UBC.DisplayName, UBC.BadgeCount, UBC.GoldBadges, UBC.SilverBadges, UBC.BronzeBadges, PP.Title AS PopularPostTitle, PP.ViewCount AS PopularPostViewCount,
//        PP.Score AS PopularPostScore, PP.CommentCount AS PopularPostCommentCount, PP.Tags AS PopularPostTags
// FROM UserBadgeCounts UBC JOIN PopularPosts PP ON UBC.UserId IN (SELECT DISTINCT P.OwnerUserId FROM Posts P WHERE P.Title = PP.Title)
// ORDER BY UBC.BadgeCount DESC, PP.ViewCount DESC;
//
// The order reads only ViewCount, so the ten posts are picked first. The IN subquery is a semi-join onto the owners of posts with the same title.
// STRING_AGG(DISTINCT) is sorted.
fn q28975(db: &'static So) -> String {
    let Post { creation_date, view_count, tags_str, title, owner_user, .. } = &db.post;
    let top = top_n(drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).select(view_count.opt())), |&(p, w)| (w.is_none(), Reverse(w), p), 10);
    let pp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let names: HashIdx<Str, Str> = (&db.tag.tag_name).inv().select(&db.tag.tag_name).collect();
    let tags = tags_str.flat_map(|t: Str| t.split(',').collect::<Vec<_>>()).select(Same::<Str>::new().map(|x: Str| x.trim_matches(' ')).select(&names).opt()).opt();
    let agg = (&pp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(tags)).buf_fold(|it| {
        let v: Vec<(Option<Id<Comment>>, Option<Option<Str>>)> = it.into_iter().collect();
        let n = v.iter().filter(|x| x.0.is_some()).count() as i64;
        let mut t: Vec<Str> = v.iter().filter_map(|x| x.1.flatten()).collect();
        t.sort();
        t.dedup();
        (n, if t.is_empty() { None } else { Some(&*Box::leak(t.join(", ").into_boxed_str())) })
    });
    let by_title: HashIdx<Str, Id<User>> = db.post.with(owner_user).select(title).inv().select(owner_user).collect();
    let pairs: MatSet<(Id<Post>, Id<User>)> = (&pp).select(Ident::<Post>::new().and(title.select(&by_title))).collect();
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    type P = (Id<Post>, Id<User>);
    let v = drain((&pairs).select(Same::<P>::new().and(Same::<P>::new().map(|x: P| x.1).select(&ubc)).and(Same::<P>::new().map(|x: P| x.0).select(&agg))));
    rows(v.into_iter().map(|(_, (((p, u), b), (n, t)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])];
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(n), ostr(t)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS PostCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// ClosedPosts AS (SELECT P.Id AS PostId, P.Title, PH.CreationDate AS ClosedDate, CTR.Name AS CloseReason FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId
//     JOIN CloseReasonTypes CTR ON CAST(PH.Comment AS int) = CTR.Id WHERE PH.PostHistoryTypeId = 10),
// TopUsers AS (SELECT UserId, RANK() OVER (ORDER BY SUM(UpVotes) DESC) AS Rank FROM UserVoteStats GROUP BY UserId)
// SELECT U.DisplayName, U.UpVotes, U.DownVotes, COALESCE(COUNT(DISTINCT CP.PostId), 0) AS ClosedPostCount,
//        COALESCE(STRING_AGG(CONCAT(CP.Title, ' (Closed on: ', CAST(CP.ClosedDate AS date), ' - Reason: ', CP.CloseReason, ')'), '; '), 'No closed posts') AS ClosedPostDetails
// FROM UserVoteStats U LEFT JOIN ClosedPosts CP ON U.UserId = CP.PostId WHERE U.UpVotes - U.DownVotes > 10 GROUP BY U.DisplayName, U.UpVotes, U.DownVotes
// ORDER BY U.UpVotes DESC LIMIT 10;
//
// TopUsers is never read. `U.UserId = CP.PostId` joins a user id to a post id, so it goes through the raw ids. The groups are keyed by the name
// and the vote counts, so users alike in all three merge. STRING_AGG is given in history id order; CONCAT skips NULLs, so a user with no
// closed post still adds ' (Closed on:  - Reason: )'.
fn q3377(db: &'static So) -> String {
    let uvs = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type K = (Str, [i64; 2]);
    type U = (Id<User>, K);
    let uv: MatSet<U> = db.user.select(Ident::<User>::new().and((&db.user.display_name).and((&uvs).filt(|a| a[0] - a[1] > 10)))).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = || {
        history_of(db)
            .select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)))
            .select(Ident::<PostHistory>::new().and(hd).and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
    };
    let post_of = || Same::<U>::new().map(|x: U| x.0).select(&db.user.origid).select(&pidx);
    let g = (&uv)
        .group_by(Same::<U>::new().map(|x: U| x.1))
        .select(post_of().select((&db.post.title).opt().and(cp())).opt())
        .buf_fold(|it| {
            let mut v: Vec<Option<(Option<Str>, ((Id<PostHistory>, i64), Str))>> = it.into_iter().collect();
            v.sort_by_key(|x| x.map(|x| x.1 .0 .0));
            let s: Vec<String> = v
                .iter()
                .map(|x| match *x {
                    Some((t, ((_, d), r))) => format!("{} (Closed on: {} - Reason: {})", t.unwrap_or(""), fmt_date(d), r),
                    None => " (Closed on:  - Reason: )".to_string(),
                })
                .collect();
            &*Box::leak(s.join("; ").into_boxed_str())
        });
    let n = (&uv).group_by(Same::<U>::new().map(|x: U| x.1)).select(post_of().select(Ident::<Post>::new().with(cp()))).count_distinct();
    let v = top_n(drain((&g).and((&n).opt())), |&((nm, a), _)| (Reverse(a[0]), nm, a[1]), 10);
    rows(v.into_iter().map(|((nm, a), (s, n))| row(vec![V::S(nm), V::I(a[0]), V::I(a[1]), V::I(n.unwrap_or(0)), V::S(s)])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.ViewCount, p.CreationDate, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT up.DisplayName, up.Reputation, COALESCE(rp.Title, 'No Questions') AS MostViewedQuestion, COALESCE(rp.ViewCount, 0) AS ViewCounts,
//        COALESCE(cp.CloseReasons, 'Not Closed') AS PostClosedReasons, COALESCE(ur.TotalBounties, 0) AS TotalBounties
// FROM Users up LEFT JOIN RankedPosts rp ON up.Id = rp.OwnerUserId AND rp.Rank = 1 LEFT JOIN UserReputation ur ON up.Id = ur.UserId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
// WHERE up.Reputation > 500 ORDER BY up.Reputation DESC, ViewCounts DESC LIMIT 50;
//
// Every user yields at least one row, so the users ranked within the first 50 by reputation are picked first. STRING_AGG(DISTINCT) is sorted.
fn q3918(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, view_count, .. } = &db.post;
    let uw = whole(db.user.with((&db.user.reputation).gt(500))).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&uw).filt(|(_, k)| k <= 50).map(|((u, _), _)| u).collect();
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .with(owner_user.select(&tu))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(dense_rank, |(_, w)| (w.is_none(), Reverse(w)), asc);
    let rp: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let ur = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)).buf_fold(|it| {
        let mut v: Vec<Str> = it.into_iter().collect();
        v.sort();
        v.dedup();
        &*Box::leak(v.join(", ").into_boxed_str())
    });
    let v = drain((&tu).select((&rp).select(Ident::<Post>::new().and((&cp).opt())).opt().and(&ur)));
    let v = top_n(v, |&(u, (r, _))| {
        let w = r.and_then(|(p, _)| view_count.get(p)).unwrap_or(0);
        (Reverse(db.user.reputation.get(u).unwrap()), Reverse(w), u, r.map(|x| x.0))
    }, 50);
    rows(v.into_iter().map(|(u, (r, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        match r {
            Some((p, c)) => {
                f.push(db.post.title.get(p).map_or(V::S("No Questions"), V::S));
                f.push(V::I(view_count.get(p).unwrap_or(0)));
                f.push(V::S(c.unwrap_or("Not Closed")));
            }
            None => f.extend([V::S("No Questions"), V::I(0), V::S("Not Closed")]),
        }
        f.push(V::I(b));
        row(f)
    }))
}

// WITH UserTagCounts AS (SELECT U.Id AS UserId, U.DisplayName, T.TagName, COUNT(*) AS TagCount FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId
//     JOIN LATERAL (SELECT UNNEST(string_to_array(substring(P.Tags, 2, length(P.Tags)-2), '><')) AS TagName) T ON TRUE WHERE P.PostTypeId = 1 GROUP BY U.Id, U.DisplayName, T.TagName),
// RankedTags AS (SELECT UserId, DisplayName, TagName, TagCount, RANK() OVER (PARTITION BY UserId ORDER BY TagCount DESC) AS TagRank FROM UserTagCounts),
// TopUserTags AS (SELECT UserId, DisplayName, TagName FROM RankedTags WHERE TagRank = 1),
// UserBadges AS (SELECT U.Id AS UserId, B.Name AS BadgeName, B.Class, B.Date FROM Users U JOIN Badges B ON U.Id = B.UserId WHERE B.Class = 1)
// SELECT TUT.DisplayName AS TopUser, TUT.TagName AS FavoriteTag, COUNT(UB.BadgeName) AS GoldBadges,
//        SUM(CASE WHEN Post.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 1 ELSE 0 END) AS RecentPosts
// FROM TopUserTags TUT LEFT JOIN Posts Post ON TUT.UserId = Post.OwnerUserId LEFT JOIN UserBadges UB ON TUT.UserId = UB.UserId
// GROUP BY TUT.DisplayName, TUT.TagName ORDER BY GoldBadges DESC, RecentPosts DESC LIMIT 10;
//
// The counts run over the post x gold badge rows of each top (user, tag) row, grouped by (name, tag), so namesakes pool.
fn q27036(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, creation_date, .. } = &db.post;
    type C = (Id<User>, Str);
    let utc = db.post.with(post_type_id.eq(1)).group_by(owner_user.and(tags_str.flat_map(tag_list))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let w = whole(&utc).group_by(Same::<C>::new().map(|x: C| x.0)).select(Same::<C>::new().and(&utc)).window(rank, |(_, n)| Reverse(n), asc);
    let tut: MatSet<C> = (&w).filt(|(_, k)| k == 1).map(|((c, _), _)| c).collect();
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let g = (&tut)
        .group_by(Same::<C>::new().map(|x: C| x.0).select(&db.user.display_name).and(Same::<C>::new().map(|x: C| x.1)))
        .select(Same::<C>::new().map(|x: C| x.0).select(posts_of(db).select(creation_date).opt().and(gold.opt())))
        .fold([0i64; 2], |a, (d, b)| [a[0] + b.is_some() as i64, a[1] + d.map_or(false, |d| d >= add_days(t0, -30)) as i64]);
    let v = top_n(drain(&g), |&(k, a)| (Reverse(a[0]), Reverse(a[1]), k), 10);
    rows(v.into_iter().map(|((n, t), a)| row(vec![V::S(n), V::S(t), V::I(a[0]), V::I(a[1])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.Rank <= 100)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, COUNT(DISTINCT b.Id) AS BadgeCount,
//        STRING_AGG(pt.Name, ', ') AS PostType
// FROM TopPosts tp LEFT JOIN Posts p ON tp.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount, u.DisplayName ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the posts are picked first. STRING_AGG repeats the type name once per owner-badge row.
fn q9630(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let w = whole(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 100).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db)).count_distinct();
    let bc = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db))).count_distinct();
    let bt = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt().and(ptype_name(db))).buf_fold(|it| {
        let s: Vec<Str> = it.into_iter().map(|x| x.1).collect();
        &*Box::leak(s.join(", ").into_boxed_str())
    });
    let v = drain((&cc).and((&vc).opt()).and((&bc).opt()).and(&bt));
    rows(v.into_iter().map(|(p, (((c, n), b), t))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n.unwrap_or(0))]);
        f.push(owner_user.get(p).map_or(V::S("Community User"), |u| user_col(db, u, "name")));
        f.extend([V::I(b.unwrap_or(0)), V::S(t)]);
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
    ("2238", q2238),
    ("23632", q23632),
    ("30141", q30141),
    ("25963", q25963),
    ("34669", q34669),
    ("26124", q26124),
    ("1882", q1882),
    ("30204", q30204),
    ("30512", q30512),
    ("31568", q31568),
    ("33789", q33789),
    ("24283", q24283),
    ("2784", q2784),
    ("23338", q23338),
    ("3949", q3949),
    ("20560", q20560),
    ("23137", q23137),
    ("23940", q23940),
    ("23040", q23040),
    ("33590", q33590),
    ("24577", q24577),
    ("32948", q32948),
    ("21366", q21366),
    ("3898", q3898),
    ("29808", q29808),
    ("5609", q5609),
    ("27790", q27790),
    ("7442", q7442),
    ("33285", q33285),
    ("1136", q1136),
    ("29234", q29234),
    ("27075", q27075),
    ("435", q435),
    ("4249", q4249),
    ("29817", q29817),
    ("26922", q26922),
    ("3041", q3041),
    ("6817", q6817),
    ("9533", q9533),
    ("2088", q2088),
    ("8815", q8815),
    ("25154", q25154),
    ("28425", q28425),
    ("373", q373),
    ("20495", q20495),
    ("3133", q3133),
    ("1105", q1105),
    ("3065", q3065),
    ("3302", q3302),
    ("2283", q2283),
    ("1458", q1458),
    ("3481", q3481),
    ("570", q570),
    ("7058", q7058),
    ("27917", q27917),
    ("8595", q8595),
    ("28975", q28975),
    ("3377", q3377),
    ("3918", q3918),
    ("27036", q27036),
    ("9630", q9630),
];
