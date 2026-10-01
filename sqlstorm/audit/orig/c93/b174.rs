use harness::prelude::*;
use std::cmp::Reverse;

/// ORDER BY `sql` LIMIT `n`, with `tie` breaking what the SQL leaves open; warns when the SQL's own key ties across the cut.
fn top_k<X, K: Ord, T: Ord>(mut v: Vec<X>, sql: impl Fn(&X) -> K, tie: impl Fn(&X) -> T, n: usize) -> Vec<X> {
    v.sort_by(|a, b| sql(a).cmp(&sql(b)).then_with(|| tie(a).cmp(&tie(b))));
    if n > 0 && n < v.len() && sql(&v[n - 1]) == sql(&v[n]) {
        eprintln!("tie at the LIMIT cut");
    }
    v.truncate(n);
    v
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank, COALESCE(NULLIF(AboutMe, ''), 'No information provided') AS UserInfo
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, ViewCount, OwnerUserId FROM RankedPosts WHERE Rank <= 10),
// PostCommentCounts AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId),
// PostVotes AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS VoteNet FROM Votes GROUP BY PostId),
// FinalResult AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, COALESCE(pcc.CommentCount, 0) AS CommentCount, COALESCE(pv.VoteNet, 0) AS NetVotes,
//        u.DisplayName AS OwnerDisplayName,
//        COALESCE(CASE WHEN tp.ViewCount > 1000 THEN 'Hot' WHEN tp.ViewCount BETWEEN 500 AND 1000 THEN 'Trending' ELSE 'New' END, 'Unknown') AS Popularity
//     FROM TopPosts tp JOIN Users u ON tp.OwnerUserId = u.Id LEFT JOIN PostCommentCounts pcc ON tp.PostId = pcc.PostId LEFT JOIN PostVotes pv ON tp.PostId = pv.PostId)
// SELECT f.PostId, f.Title, f.Score, f.ViewCount, f.CommentCount, f.NetVotes, f.OwnerDisplayName, f.Popularity,
//        CASE WHEN f.Score > 100 THEN 'Highly Rated' WHEN f.ViewCount < 50 THEN 'Needs Attention' ELSE 'Average Performance' END AS PerformanceLabel
// FROM FinalResult f ORDER BY f.Score DESC, f.ViewCount DESC;
fn q23058(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(ts(2023, 10, 1, 0, 0, 0)).and(post_type_id.eq(1))).with(owner_user).select(score));
    let top = top_n(v, |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let nv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + match t {
        Some(2) => 1,
        Some(3) => -1,
        _ => 0,
    });
    rows(drain((&cc).and(&nv)).into_iter().map(|(p, (c, n))| {
        let s = score.get(p).unwrap();
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(n), post_fields(db, p, &["owner"]).remove(0)]);
        f.push(V::S(match w {
            Some(w) if w > 1000 => "Hot",
            Some(w) if (500..=1000).contains(&w) => "Trending",
            _ => "New",
        }));
        f.push(V::S(if s > 100 { "Highly Rated" } else if w.map_or(false, |w| w < 50) { "Needs Attention" } else { "Average Performance" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.Reputation,
//        ROW_NUMBER() OVER (PARTITION BY CASE WHEN u.Reputation >= 1000 THEN 'High' WHEN u.Reputation >= 100 THEN 'Medium' ELSE 'Low' END ORDER BY u.Reputation DESC) AS Rank FROM Users u),
// RecentActivePosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 DAYS') GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.ViewCount),
// PostVoteStatistics AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(v.Id) AS TotalVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// ClosedPostDetails AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, p.Title, t.TagName FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id JOIN Tags t ON t.ExcerptPostId = p.Id
//     WHERE ph.PostHistoryTypeId = 10)
// SELECT u.DisplayName, ur.Reputation, rp.PostId, rp.Title, rp.ViewCount, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes, rp.CommentCount,
//        CASE WHEN rp.CommentCount > 0 THEN 'Commented' ELSE 'No Comments' END AS CommentStatus,
//        ROW_NUMBER() OVER (PARTITION BY ur.Rank ORDER BY rp.ViewCount DESC) AS MostViewedRank
// FROM UserReputation ur JOIN Users u ON ur.UserId = u.Id JOIN RecentActivePosts rp ON u.Id = rp.OwnerUserId LEFT JOIN PostVoteStatistics pvs ON rp.PostId = pvs.PostId
// WHERE ur.Reputation > (SELECT AVG(Reputation) FROM Users) AND EXISTS (SELECT 1 FROM ClosedPostDetails cp WHERE cp.PostId = rp.PostId)
// ORDER BY ur.Rank DESC, rp.ViewCount DESC, u.DisplayName;
//
// Both ROW_NUMBERs leave ties open (equal reputations, equal view counts); the port breaks them by id. The answer is empty on this data, so that is untested.
fn q20378(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let (sum, n) = db.user.select(reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let tier = |r: i64| if r >= 1000 { 0 } else if r >= 100 { 1 } else { 2 };
    let ur = ranked(drain(reputation), |&(u, r)| (tier(r), Reverse(r), u), false);
    let ur = per_group(ur, |&(_, r)| tier(r));
    let ur = rel(ur.into_iter().map(|((u, _), k)| (u, k)).collect());
    let urk: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let above = move || Ident::<User>::new().with(reputation.filt(move |r| r * n > sum));
    let excerpt: MatSet<Id<Post>> = db.tag.select(&db.tag.excerpt_post).collect();
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select((&db.post_history.post).select(Ident::<Post>::new().with(&excerpt))).collect();
    let rp = || db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(&closed).with(owner_user.select(above()));
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(&vs).and(owner_user.select(&urk)));
    let v = ranked(v, |&(p, (_, (_, k)))| {
        let w = view_count.get(p);
        (k, w.is_none(), Reverse(w), p)
    }, false);
    let v = per_group(v, |&(_, (_, (_, k)))| k);
    rows(v.into_iter().map(|((p, ((c, a), (u, _))), r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if c > 0 { "Commented" } else { "No Comments" }), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.PostTypeId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVoteCount,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, COALESCE((SELECT SUM(b.Class) FROM Badges b WHERE b.UserId = p.OwnerUserId), 0) AS TotalBadgeLevel
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AggregatedData AS (SELECT rp.OwnerUserId, COUNT(*) AS TotalPosts, SUM(rp.UpVoteCount) AS TotalUpVotes, SUM(rp.DownVoteCount) AS TotalDownVotes, SUM(rp.CommentCount) AS TotalComments,
//        AVG(rp.TotalBadgeLevel) AS AvgBadgeLevel FROM RankedPosts rp WHERE rp.Rank <= 5 GROUP BY rp.OwnerUserId),
// FilteredUsers AS (SELECT u.Id, u.DisplayName, au.OwnerUserId, au.TotalPosts, au.TotalUpVotes, au.TotalDownVotes, au.TotalComments, au.AvgBadgeLevel
//     FROM Users u LEFT JOIN AggregatedData au ON u.Id = au.OwnerUserId WHERE u.Reputation > 1000 OR (u.Reputation IS NULL AND EXISTS (SELECT 1 FROM Badges WHERE UserId = u.Id AND Class = 1)))
// SELECT fu.DisplayName, fy.TotalPosts, fy.TotalUpVotes, fy.TotalDownVotes, fy.TotalComments, fy.AvgBadgeLevel,
//        COALESCE(NULLIF(fu.TotalUpVotes, 0), 1) AS SafeUpVotes, COALESCE(NULLIF(fu.TotalDownVotes, 0), 1) AS SafeDownVotes
// FROM FilteredUsers fu LEFT OUTER JOIN AggregatedData fy ON fu.OwnerUserId = fy.OwnerUserId
// WHERE fy.TotalPosts > 3 AND (fy.TotalUpVotes - fy.TotalDownVotes) > 10 ORDER BY fy.TotalComments DESC, fy.TotalUpVotes DESC LIMIT 100;
//
// Reputation is never NULL, so the OR branch never applies. The ownerless posts' group cannot meet a user, so they are left out of the ranking.
fn q22775(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bl = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    let agg = (&tp).group_by(owner_user).select((&pv).and(&pc).and(owner_user.select(&bl))).fold([0i64; 5], |a, ((v, c), b)| [a[0] + 1, a[1] + v[0], a[2] + v[1], a[3] + c, a[4] + b]);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&agg).filt(|a| a[0] > 3 && a[1] - a[2] > 10)));
    let v = top_n(v, |&(u, a)| (Reverse(a[3]), Reverse(a[1]), u), 100);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0])]);
        f.extend([V::I(if a[1] == 0 { 1 } else { a[1] }), V::I(if a[2] == 0 { 1 } else { a[2] })]);
        row(f)
    }))
}

// WITH RecursivePostStats AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.ViewCount, P.Score, P.AcceptedAnswerId,
//        RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS UserPostRank FROM Posts P WHERE P.PostTypeId = 1),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS QuestionCount FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId
//     WHERE P.PostTypeId = 1 GROUP BY U.Id, U.DisplayName, U.Reputation HAVING COUNT(DISTINCT P.Id) > 5),
// UserWithHighestReputation AS (SELECT UserId, DisplayName, Reputation FROM TopUsers ORDER BY Reputation DESC LIMIT 1),
// PostInteractions AS (SELECT P.Id AS PostId, COUNT(DISTINCT C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.OwnerUserId = (SELECT UserId FROM UserWithHighestReputation) GROUP BY P.Id),
// FinalReport AS (SELECT PS.PostId, PS.Title, U.DisplayName AS OwnerDisplayName, PS.ViewCount, PS.Score, PI.CommentCount, PI.UpVoteCount, PI.DownVoteCount
//     FROM RecursivePostStats PS JOIN Users U ON PS.OwnerUserId = U.Id LEFT JOIN PostInteractions PI ON PS.PostId = PI.PostId WHERE PS.UserPostRank = 1)
// SELECT FR.PostId, FR.Title, FR.OwnerDisplayName, FR.ViewCount, FR.Score, COALESCE(FR.CommentCount, 0) AS CommentCount, COALESCE(FR.UpVoteCount, 0) AS UpVoteCount,
//        COALESCE(FR.DownVoteCount, 0) AS DownVoteCount
// FROM FinalReport FR ORDER BY FR.Score DESC, FR.ViewCount DESC;
fn q31431(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let qc = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tu = top_n(drain((&qc).filt(|n| n > 5)), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 1);
    let u0 = tu[0].0;
    let mine = || db.post.with(owner_user.filt(move |u| u == u0));
    let pc = mine().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = mine()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&first).select(Ident::<Post>::new().and((&pc).and(&pv).opt()))).into_iter().map(|(_, (p, x))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views", "score"]);
        let (c, a) = x.unwrap_or((0, [0, 0]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.PostRank = 1),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostWithVotes AS (SELECT tp.PostId, tp.Title, tp.CreationDate, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes,
//        CASE WHEN COALESCE(pvc.UpVotes, 0) > COALESCE(pvc.DownVotes, 0) THEN 'Positive' WHEN COALESCE(pvc.UpVotes, 0) < COALESCE(pvc.DownVotes, 0) THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
//     FROM TopPosts tp LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId),
// PostsWithComments AS (SELECT pwv.PostId, pwv.Title, pwv.CreationDate, pwv.UpVotes, pwv.DownVotes, pwv.VoteSentiment, COUNT(c.Id) AS CommentCount
//     FROM PostWithVotes pwv LEFT JOIN Comments c ON pwv.PostId = c.PostId GROUP BY pwv.PostId, pwv.Title, pwv.CreationDate, pwv.UpVotes, pwv.DownVotes, pwv.VoteSentiment)
// SELECT pwc.PostId, pwc.Title, pwc.CreationDate, pwc.UpVotes, pwc.DownVotes, pwc.VoteSentiment, pwc.CommentCount,
//        CASE WHEN pwc.CommentCount > 10 THEN 'Hot' WHEN pwc.CommentCount > 0 THEN 'Active' ELSE 'Silent' END AS ActivityLevel
// FROM PostsWithComments pwc WHERE pwc.UpVotes IS NOT NULL ORDER BY pwc.UpVotes DESC, pwc.CreationDate DESC;
fn q4163(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&pv).and(&pc)).into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }), V::I(c)]);
        f.push(V::S(if c > 10 { "Hot" } else if c > 0 { "Active" } else { "Silent" }));
        row(f)
    }))
}

// WITH RankedTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS TagRank FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// TopUserActivity AS (SELECT u.DisplayName, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, RANK() OVER (ORDER BY SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id GROUP BY u.DisplayName),
// RecentPostEdits AS (SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS EditRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6, 24)),
// TagEngagement AS (SELECT rt.TagName, COUNT(DISTINCT p.Id) AS EngagedPosts, SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TagUpvoteCount
//     FROM RankedTags rt JOIN Posts p ON p.Tags LIKE '%' || rt.TagName || '%' LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE rt.TagRank <= 10 GROUP BY rt.TagName)
// SELECT u.DisplayName, ua.AnswerCount, ua.UpvoteCount, ua.DownvoteCount, te.TagName, te.EngagedPosts, te.CommentCount, te.TagUpvoteCount
// FROM TopUserActivity ua JOIN TagEngagement te ON te.TagName IN (SELECT TagName FROM RankedTags WHERE TagRank <= 10) JOIN Users u ON u.DisplayName = ua.DisplayName
// WHERE ua.UserRank <= 5 ORDER BY ua.AnswerCount DESC, te.EngagedPosts DESC;
//
// RecentPostEdits is never referenced. The ON of TagEngagement names only te, so the users and the tags are crossed.
fn q27027(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let rt = ranked(drain((&tag_stats(db)).filt(|a| a[0] > 0)), |&(_, a)| Reverse(a[0]), false);
    let tt: MatSet<Id<Tag>> = rel(rt.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|t| t).collect();
    let te = (&tt)
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let ep = (&tt).group_by(Ident::<Tag>::new()).select((&by_tag).map(|(p, _)| p)).fold(0i64, |n, _| n + 1);
    let te = rel(drain((&te).and(&ep)));
    let ov = own_votes(db);
    let ua = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(post_type_id.and((&ov).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (v == Some(2)) as i64, a[2] + (v == Some(3)) as i64],
            None => a,
        });
    let ur = ranked(drain(&ua), |&(_, a)| Reverse(a[0]), false);
    let ur = rel(ur.into_iter().take_while(|x| x.1 <= 5).map(|x| x.0).collect());
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    type R = (Str, [i64; 3]);
    let uu = (&ur).select(Same::<R>::new().and(Same::<R>::new().map(|(n, _): R| n).select(&by_name)));
    let mut v = Vec::new();
    uu.cross(&te).drive(|_, (((_, a), u), (t, (e, n)))| v.push((u, a, t, e, n)));
    rows(v.into_iter().map(|(u, a, t, e, n)| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(db.tag.tag_name.get(t).unwrap()), V::I(n), V::I(e[0]), V::I(e[1])])
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, CASE WHEN Reputation >= 1000 THEN 'High' WHEN Reputation >= 100 THEN 'Medium' ELSE 'Low' END AS ReputationCategory FROM Users),
// TopQuestions AS (SELECT P.Id AS QuestionId, P.Title, P.CreationDate, P.Score, COUNT(A.Id) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY P.Score DESC) AS Rank
//     FROM Posts P LEFT JOIN Posts A ON P.Id = A.ParentId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// UserVotingStats AS (SELECT U.Id AS UserId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 8 THEN V.BountyAmount ELSE 0 END), 0) AS TotalBounties FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id),
// QuestionCloseStats AS (SELECT PH.PostId, COUNT(*) AS CloseCount, MAX(PH.CreationDate) AS LastCloseDate FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId)
// SELECT U.Id AS UserId, U.DisplayName, UReputation.ReputationCategory, TQ.QuestionId, TQ.Title AS QuestionTitle, TQ.CreationDate AS QuestionDate, TQ.Score AS QuestionScore,
//        TQ.AnswerCount, UVS.TotalUpvotes, UVS.TotalDownvotes, UVS.TotalBounties, QCS.CloseCount, QCS.LastCloseDate
// FROM Users U JOIN UserReputation UReputation ON U.Id = UReputation.Id LEFT JOIN TopQuestions TQ ON TQ.Rank <= 5 AND U.Id = TQ.QuestionId
// LEFT JOIN UserVotingStats UVS ON U.Id = UVS.UserId LEFT JOIN QuestionCloseStats QCS ON TQ.QuestionId = QCS.PostId
// WHERE UReputation.ReputationCategory = 'High' OR (TQ.Score > 10 AND TQ.QuestionId IS NOT NULL) ORDER BY U.DisplayName ASC, TQ.Score DESC;
//
// `U.Id = TQ.QuestionId` joins a user id to a post id, so it goes through the raw ids. Partitioned by its own id, every question has Rank 1.
fn q22740(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let qidx: HashIdx<i64, Id<Post>> = db.post.with(post_type_id.eq(1)).select(&db.post.origid).inv().collect();
    let ac = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + if t == 8 { b.unwrap_or(0) } else { 0 }],
        None => a,
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let qcs = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let tq = Ident::<Post>::new().and(&ac).and((&qcs).opt());
    let base = db
        .user
        .select(Ident::<User>::new().and((&db.user.origid).select(&qidx).select(tq).opt()))
        .filt(|(u, q): (Id<User>, Option<((Id<Post>, i64), Option<(i64, i64)>)>)| db.user.reputation.get(u).unwrap() >= 1000 || q.map_or(false, |((p, _), _)| score.get(p).unwrap() > 10));
    rows(drain(base.select(Same::<(Id<User>, Option<((Id<Post>, i64), Option<(i64, i64)>)>)>::new().and(Same::<(Id<User>, Option<((Id<Post>, i64), Option<(i64, i64)>)>)>::new().map(|(u, _)| u).select(&uvs))))
        .into_iter()
        .map(|(_, ((u, q), a))| {
            let r = db.user.reputation.get(u).unwrap();
            let mut f = ucols(db, u, &["uid", "name"]);
            f.push(V::S(if r >= 1000 { "High" } else if r >= 100 { "Medium" } else { "Low" }));
            match q {
                Some(((p, n), c)) => {
                    f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
                    f.push(V::I(n));
                    f.extend(a.map(V::I));
                    f.extend(match c {
                        Some((n, d)) => [V::I(n), V::T(d)],
                        None => [V::Null, V::Null],
                    });
                }
                None => {
                    f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]);
                    f.extend(a.map(V::I));
                    f.extend([V::Null, V::Null]);
                }
            }
            row(f)
        }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation IS NOT NULL),
// PostsWithCounts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.OwnerUserId, p.PostTypeId),
// UserBadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId),
// FrequentPostTypes AS (SELECT PostTypeId, COUNT(*) AS PostTypeFrequency FROM Posts GROUP BY PostTypeId)
// SELECT u.UserId, u.DisplayName, u.Reputation, ub.BadgeCount, COUNT(pwc.PostId) AS TotalPosts, COALESCE(FPT.PostTypeFrequency, 0) AS FrequentPostTypeFrequency,
//        SUM(pwc.CommentCount) AS TotalComments, SUM(pwc.UpVoteCount) AS TotalUpVotes, SUM(pwc.DownVoteCount) AS TotalDownVotes,
//        CASE WHEN ub.HighestBadgeClass IS NOT NULL THEN (CASE WHEN ub.HighestBadgeClass = 1 THEN 'Gold' WHEN ub.HighestBadgeClass = 2 THEN 'Silver' ELSE 'Bronze' END) ELSE 'No Badge' END AS HighestBadge
// FROM RankedUsers u LEFT JOIN UserBadgeCounts ub ON u.UserId = ub.UserId LEFT JOIN PostsWithCounts pwc ON u.UserId = pwc.OwnerUserId LEFT JOIN FrequentPostTypes FPT ON pwc.PostTypeId = FPT.PostTypeId
// WHERE u.ReputationRank < 101 GROUP BY u.UserId, u.DisplayName, u.Reputation, ub.BadgeCount, ub.HighestBadgeClass, FPT.PostTypeFrequency
// ORDER BY u.Reputation DESC, TotalPosts DESC LIMIT 10 OFFSET 0;
//
// The GROUP BY names a column of the joined post type, so the (user, post) rows are materialised and grouped by (user, frequency).
fn q22214(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let rr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(rr.into_iter().take_while(|x| x.1 < 101).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let pwc = (&tu)
        .select(posts_of(db))
        .group_by(Same::<Id<Post>>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let freq = db.post.group_by(post_type_id).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let j: MatSet<(Id<User>, Option<Id<Post>>)> = (&tu).select(Ident::<User>::new().and(posts_of(db).opt())).collect();
    let u_of = (&j).map(|(u, _)| u);
    let p_of = (&j).flat_map(|(_, p)| p);
    let g = (&j).group_by((&u_of).and((&p_of).select(post_type_id).select(&freq).opt())).select((&p_of).select(&pwc).opt()).fold([0i64; 4], |a, x| match x {
        Some(c) => [a[0] + 1, a[1] + c[0], a[2] + c[1], a[3] + c[2]],
        None => a,
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let v = drain((&g).select(Same::<[i64; 4]>::new()));
    let v = drain(rel(v).select(Same::<((Id<User>, Option<i64>), [i64; 4])>::new().and(Same::<((Id<User>, Option<i64>), [i64; 4])>::new().map(|((u, _), _)| u).select(&ub).opt())));
    let v = top_n(v, |&(_, (((u, fr), a), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), u, fr), 10);
    rows(v.into_iter().map(|(_, (((u, fr), a), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(b.map_or(V::Null, |b| V::I(b.0)));
        f.extend([V::I(a[0]), V::I(fr.unwrap_or(0)), nullable(a[1], a[0]), nullable(a[2], a[0]), nullable(a[3], a[0])]);
        f.push(V::S(match b.map(|b| b.1) {
            Some(1) => "Gold",
            Some(2) => "Silver",
            Some(_) => "Bronze",
            None => "No Badge",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankScore FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// ClosedPosts AS (SELECT p.Id AS ClosedPostId, ph.CreationDate AS ClosedDate, ph.UserDisplayName AS CloserDisplayName FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(v.BountyAmount) AS TotalBounties FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5),
// MostActive AS (SELECT ua.DisplayName, ua.TotalPosts, ua.TotalBounties, RANK() OVER (ORDER BY ua.TotalPosts DESC) AS UserRank FROM ActiveUsers ua),
// QuestionStats AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score AS QuestionScore, rp.ViewCount, COALESCE(cp.ClosedDate, '1970-01-01') AS ClosedDate,
//        COALESCE(cp.CloserDisplayName, 'Not Closed') AS CloserDisplayName FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.ClosedPostId)
// SELECT qs.Title, qs.OwnerDisplayName, qs.QuestionScore, qs.ViewCount, qs.ClosedDate, qs.CloserDisplayName, ma.DisplayName AS MostActiveUser, ma.TotalPosts, ma.TotalBounties
// FROM QuestionStats qs JOIN MostActive ma ON ma.UserRank <= 10 ORDER BY qs.QuestionScore DESC, qs.ViewCount DESC LIMIT 20;
//
// The ON names only ma, so the questions and the most active users are crossed.
fn q33962(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let rp = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user);
    let PostHistory { post_history_type_id, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)));
    let qs = rel(drain(rp.select(Ident::<Post>::new().and(closes.opt()))).into_iter().map(|x| x.1).collect());
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tb = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 2], |a, b| match b.flatten() {
        Some(b) => [a[0] + 1, a[1] + b],
        None => a,
    });
    let ma = ranked(drain((&tp).filt(|n| n > 5).and(&tb)), |&(_, (n, _))| Reverse(n), false);
    let ma = rel(ma.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let mut v = Vec::new();
    (&qs).cross(&ma).drive(|_, ((p, h), (u, (n, b)))| v.push((p, h, u, n, b)));
    let v = top_n(v, |&(p, h, u, _, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, h, u)
    }, 20);
    rows(v.into_iter().map(|(p, h, u, n, b)| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.push(V::T(h.map_or(0, |h| db.post_history.creation_date.get(h).unwrap())));
        f.push(V::S(h.and_then(|h| db.post_history.user_display_name.get(h)).unwrap_or("Not Closed")));
        f.extend([user_col(db, u, "name"), V::I(n), nullable(b[1], b[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND p.Score > 0),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// Closures AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostComments AS (SELECT c.PostId, COUNT(*) AS CommentCount FROM Comments c GROUP BY c.PostId),
// HighPerformers AS (SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, u.DisplayName AS OwnerName, us.TotalUpVotes, us.TotalDownVotes,
//        COALESCE(cl.CloseCount, 0) AS TotalClosures, COALESCE(pc.CommentCount, 0) AS TotalComments
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserStats us ON u.Id = us.UserId LEFT JOIN Closures cl ON rp.Id = cl.PostId LEFT JOIN PostComments pc ON rp.Id = pc.PostId
//     WHERE rp.RankByScore <= 5)
// SELECT h.PostId, h.Title, h.CreationDate, h.Score, h.ViewCount, h.OwnerName, h.TotalUpVotes, h.TotalDownVotes, h.TotalClosures, h.TotalComments,
//        (CASE WHEN h.TotalUpVotes > h.TotalDownVotes THEN 'Positive' WHEN h.TotalDownVotes > h.TotalUpVotes THEN 'Negative' ELSE 'Neutral' END) AS VoteSentiment
// FROM HighPerformers h ORDER BY h.Score DESC, h.ViewCount DESC;
fn q30067(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1)).and(score.gt(0))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cl = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cl).and(&pc).and(owner_user.select(&us))).into_iter().map(|(p, ((c, n), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(n), V::S(if a[0] > a[1] { "Positive" } else if a[1] > a[0] { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.Score, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, COALESCE(B.GoldBadges, 0) + COALESCE(B.SilverBadges, 0) + COALESCE(B.BronzeBadges, 0) AS TotalBadges
//     FROM Users U LEFT JOIN UserBadges B ON U.Id = B.UserId WHERE U.Reputation > (SELECT AVG(Reputation) FROM Users)),
// PostVoteSummary AS (SELECT P.Id, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id)
// SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.TotalBadges, P.Title, P.CreationDate, COALESCE(S.UpVotes, 0) AS TotalUpVotes, COALESCE(S.DownVotes, 0) AS TotalDownVotes,
//        R.GoldBadges, R.SilverBadges, R.BronzeBadges, CASE WHEN R.GoldBadges > 0 THEN 'Gold Member' WHEN R.SilverBadges > 0 THEN 'Silver Member' ELSE 'Regular Member' END AS MembershipStatus
// FROM TopUsers U LEFT JOIN UserBadges R ON U.Id = R.UserId LEFT JOIN RecentPosts P ON U.Id = P.OwnerUserId AND P.PostRank = 1 LEFT JOIN PostVoteSummary S ON P.PostId = S.Id
// WHERE (U.Reputation BETWEEN 100 AND 1000 OR U.TotalBadges > 5) ORDER BY U.Reputation DESC, TotalUpVotes DESC LIMIT 100;
//
// A tie on CreationDate inside PostRank goes to the larger post id.
fn q21241(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let Post { creation_date, owner_user, .. } = &db.post;
    let (sum, n) = db.user.select(reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let rp = top_per(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let first: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let pvs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let tu = db.user.with(reputation.filt(move |r| r * n > sum)).select(Ident::<User>::new().and(&ub)).filt(|(u, b): (Id<User>, [i64; 3])| {
        let r = db.user.reputation.get(u).unwrap();
        (100..=1000).contains(&r) || b[0] + b[1] + b[2] > 5
    });
    let v = drain(tu.select(Same::<(Id<User>, [i64; 3])>::new().and(Same::<(Id<User>, [i64; 3])>::new().map(|(u, _)| u).select((&first).map(|(_, p)| p).select(Ident::<Post>::new().and((&pvs).opt())).opt()))));
    let v = top_n(v, |&(_, ((u, _), p))| (Reverse(reputation.get(u).unwrap()), Reverse(p.map_or(0, |(_, s)| s.map_or(0, |s| s[0]))), u), 100);
    rows(v.into_iter().map(|(_, ((u, b), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(b[0] + b[1] + b[2]));
        match p {
            Some((p, s)) => {
                f.extend(post_fields(db, p, &["title", "created"]));
                let s = s.unwrap_or([0, 0]);
                f.extend([V::I(s[0]), V::I(s[1])]);
            }
            None => f.extend([V::Null, V::Null, V::I(0), V::I(0)]),
        }
        f.extend(b.map(V::I));
        f.push(V::S(if b[0] > 0 { "Gold Member" } else if b[1] > 0 { "Silver Member" } else { "Regular Member" }));
        row(f)
    }))
}

// WITH RECURSIVE UserVoteCounts AS (SELECT U.Id AS UserId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(*) AS TotalVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id),
// RecentPostHistory AS (SELECT PH.PostId, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
//        COUNT(CASE WHEN PH.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteUndeleteCount, MAX(PH.CreationDate) AS LastActionDate FROM PostHistory PH GROUP BY PH.PostId),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, PS.CloseCount, PS.ReopenCount, PS.DeleteUndeleteCount, ROW_NUMBER() OVER (ORDER BY P.LastActivityDate DESC) AS RecentRanking
//     FROM Posts P LEFT JOIN RecentPostHistory PS ON P.Id = PS.PostId WHERE P.Score > 0 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RankedPosts AS (SELECT PS.PostId, PS.Title, PS.Score, PS.ViewCount, PS.CloseCount, PS.ReopenCount, PS.DeleteUndeleteCount, RANK() OVER (ORDER BY PS.CloseCount DESC, PS.ReopenCount ASC) AS CloseRank FROM PostStats PS)
// SELECT U.DisplayName, U.Reputation, U.LastAccessDate, RP.Title, RP.Score, RP.ViewCount, RP.CloseCount, RP.ReopenCount, RP.DeleteUndeleteCount, UVC.TotalVotes,
//        CASE WHEN UVC.UpVotes > UVC.DownVotes THEN 'Positive' ELSE 'Negative' END AS VoteSentiment
// FROM Users U JOIN UserVoteCounts UVC ON U.Id = UVC.UserId JOIN RankedPosts RP ON RP.CloseRank <= 10
// WHERE U.Reputation > (SELECT AVG(Reputation) FROM Users WHERE Reputation IS NOT NULL) ORDER BY U.Reputation DESC, RP.CloseCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. The ON names only RP, so the users and the ranked posts are crossed.
fn q34916(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let Post { score, creation_date, .. } = &db.post;
    let (sum, n) = db.user.select(reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let uvc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + 1]);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let rph = db.post_history.group_by(post).select(post_history_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + matches!(t, 12 | 13) as i64]);
    let ps = drain(db.post.with(score.gt(0).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select((&rph).opt()));
    let ps = ranked(ps, |&(_, a)| (a.is_none(), Reverse(a.map(|a| a[0])), a.is_none(), a.map(|a| a[1])), false);
    let rp = rel(ps.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let us = drain(db.user.with(reputation.filt(move |r| r * n > sum)).select(&uvc));
    let mut v = Vec::new();
    rel(us).cross(&rp).drive(|_, ((u, a), (p, h))| v.push((u, a, p, h)));
    rows(v.into_iter().map(|(u, a, p, h)| {
        let mut f = ucols(db, u, &["name", "rep", "last_access"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend(match h {
            Some(h) => h.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::I(a[2]), V::S(if a[0] > a[1] { "Positive" } else { "Negative" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, AVG(p.ViewCount) AS AvgViews,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) DESC) AS PostRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, PositivePosts, NegativePosts, AvgViews FROM UserStats WHERE PostRank <= 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(c.CommentCount, 0) AS Comments, COALESCE(vote.VoteCount, 0) AS UpVotes, COALESCE(closed.ClosedPostCount, 0) AS ClosedCount
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) vote ON p.Id = vote.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS ClosedPostCount FROM PostHistory WHERE PostHistoryTypeId = 10 GROUP BY PostId) closed ON p.Id = closed.PostId)
// SELECT tu.DisplayName, tu.Reputation, pd.Title, pd.CreationDate, pd.Comments, pd.UpVotes, pd.ClosedCount, CASE WHEN pd.ClosedCount > 0 THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM TopUsers tu JOIN PostDetails pd ON tu.UserId = pd.PostId ORDER BY tu.Reputation DESC, pd.UpVotes DESC;
//
// Partitioned by its own id, every user has PostRank 1, so TopUsers is every user. `tu.UserId = pd.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q4615(db: &'static So) -> String {
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let mp: MatSet<Id<Post>> = db.user.select((&db.user.origid).select(&pidx)).collect();
    let cc = (&mp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let up = (&mp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cl = (&mp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain(db.user.select((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&cc).and(&up).and(&cl))));
    rows(v.into_iter().map(|(u, (((p, c), n), k))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(c), V::I(n), V::I(k), V::S(if k > 0 { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId
//     LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// UserRankings AS (SELECT UserId, DisplayName, QuestionCount, TotalUpVotes, TotalDownVotes, GoldBadges, SilverBadges, BronzeBadges, CommentCount, RANK() OVER (ORDER BY QuestionCount DESC) AS QuestionRank,
//        RANK() OVER (ORDER BY TotalUpVotes DESC) AS VoteRank, RANK() OVER (ORDER BY (TotalUpVotes - TotalDownVotes) DESC) AS ScoreRank FROM UserMetrics)
// SELECT UM.DisplayName, UM.QuestionCount, UM.TotalUpVotes, UM.TotalDownVotes, UM.CommentCount, CASE WHEN QR.QuestionRank <= 10 THEN 'Top 10 by Questions' ELSE 'Others' END AS QuestionRankCategory,
//        CASE WHEN VR.VoteRank <= 10 THEN 'Top 10 by Votes' ELSE 'Others' END AS VoteRankCategory, CASE WHEN SR.ScoreRank <= 10 THEN 'Top 10 by Score' ELSE 'Others' END AS ScoreRankCategory
// FROM UserMetrics UM JOIN UserRankings QR ON UM.UserId = QR.UserId JOIN UserRankings VR ON UM.UserId = VR.UserId JOIN UserRankings SR ON UM.UserId = SR.UserId
// WHERE UM.QuestionCount > 0 OR UM.TotalUpVotes > 0 ORDER BY UM.QuestionCount DESC, UM.TotalUpVotes DESC;
fn q25827(db: &'static So) -> String {
    let asked = || posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(1)));
    let um = db
        .user
        .group_by(Ident::<User>::new())
        .select(asked().select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| {
            let t = p.and_then(|(t, _)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let qc = db.user.group_by(Ident::<User>::new()).select(asked().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(asked().select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&um).and(&qc).and(&cc));
    let v = ranked(v, |&(_, ((_, q), _))| Reverse(q), false);
    let v = ranked(v, |&((_, ((a, _), _)), _)| Reverse(a[0]), false);
    let v = ranked(v, |&(((_, ((a, _), _)), _), _)| Reverse(a[0] - a[1]), false);
    let v = drain(rel(v).filt(|((((_, ((a, q), _)), _), _), _)| q > 0 || a[0] > 0));
    rows(v.into_iter().map(|(_, ((((u, ((a, q), c)), qr), vr), sr))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(q),
            V::I(a[0]),
            V::I(a[1]),
            V::I(c),
            V::S(if qr <= 10 { "Top 10 by Questions" } else { "Others" }),
            V::S(if vr <= 10 { "Top 10 by Votes" } else { "Others" }),
            V::S(if sr <= 10 { "Top 10 by Score" } else { "Others" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS RankPerUser,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPostsByUser FROM Posts p WHERE p.PostTypeId = 1),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// PostAnalytics AS (SELECT rp.Id, rp.Title, rp.ViewCount, rp.RankPerUser, rp.TotalPostsByUser, COALESCE(rv.TotalVotes, 0) AS TotalVotes, COALESCE(rv.UpVotes, 0) AS UpVotes,
//        COALESCE(rv.DownVotes, 0) AS DownVotes, CASE WHEN rp.RankPerUser = 1 THEN 'Most Viewed' WHEN rp.ViewCount > 100 THEN 'Popular' ELSE 'Regular' END AS PostCategory
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.Id = rv.PostId WHERE rp.TotalPostsByUser > 2),
// PostHistoryAnalysis AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT pa.Title, pa.ViewCount, pa.TotalVotes, pa.UpVotes, pa.DownVotes, pha.EditCount, pha.LastEditDate, pa.PostCategory,
//        CASE WHEN pa.PostCategory = 'Most Viewed' THEN 'Featured' WHEN pa.UpVotes > pa.DownVotes THEN 'Positive' ELSE 'Needs Attention' END AS PostStatus
// FROM PostAnalytics pa LEFT JOIN PostHistoryAnalysis pha ON pa.Id = pha.PostId
// WHERE (pa.UpVotes IS NOT NULL OR pa.DownVotes IS NOT NULL) AND pa.Title IS NOT NULL AND pa.ViewCount IS NOT NULL ORDER BY pa.ViewCount DESC LIMIT 50;
//
// The ownerless questions are one partition of their own. A tie on ViewCount inside RankPerUser goes to the smaller post id.
fn q21604(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, title, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let tot = qs().group_by(owner_user.opt()).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let first = top_per(drain(qs().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cand = || qs().with(owner_user.opt().select((&tot).filt(|n| n > 2))).with(title).with(view_count);
    let rv = cand().group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 3], |a, n| match n {
        Some(n) => [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64],
        None => a,
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&rv).and((&pha).opt()).and(Ident::<Post>::new().with(&first).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(view_count.get(p)), p), 50);
    rows(v.into_iter().map(|(p, ((a, h), m))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        let cat = if m.is_some() { "Most Viewed" } else if view_count.get(p).unwrap() > 100 { "Popular" } else { "Regular" };
        f.push(V::S(cat));
        f.push(V::S(if cat == "Most Viewed" { "Featured" } else if a[1] > a[2] { "Positive" } else { "Needs Attention" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, AVG(P.Score) AS AvgPostScore, SUM(COALESCE(P.ViewCount, 0)) AS TotalViewCount, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
//        COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount FROM Posts P GROUP BY P.OwnerUserId),
// RecentVotes AS (SELECT V.UserId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId IN (2, 8) THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Votes V WHERE V.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY V.UserId),
// CombinedStats AS (SELECT UB.UserId, UB.DisplayName, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(RV.VoteCount, 0) AS VoteCount, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges,
//        PS.AvgPostScore, PS.TotalViewCount, PS.QuestionCount, PS.AnswerCount FROM UserBadges UB LEFT JOIN PostStats PS ON UB.UserId = PS.OwnerUserId LEFT JOIN RecentVotes RV ON UB.UserId = RV.UserId),
// RankedStats AS (SELECT *, ROW_NUMBER() OVER (ORDER BY PostCount DESC, VoteCount DESC, BadgeCount DESC) AS Rank FROM CombinedStats)
// SELECT UserId, DisplayName, PostCount, VoteCount, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, AvgPostScore, TotalViewCount, QuestionCount, AnswerCount, Rank
// FROM RankedStats WHERE (PostCount + VoteCount) > 0 AND (BadgeCount > 0 OR TotalViewCount > 100) ORDER BY Rank LIMIT 10;
//
// A tie inside the ROW_NUMBER goes to the smaller user id.
fn q21530(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, score, view_count, post_type_id, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt()).and(post_type_id)).fold([0i64; 5], |a, ((s, w), t)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + (t == 1) as i64, a[4] + (t == 2) as i64]);
    let rv = db.vote.with((&db.vote.creation_date).ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&ub).and((&ps).opt()).and((&rv).opt()));
    let v = ranked(v, |&(u, ((b, p), r))| (Reverse(p.map_or(0, |p| p[0])), Reverse(r.unwrap_or(0)), Reverse(b[0]), u), false);
    let v = drain(rel(v).filt(|((_, ((b, p), r)), _)| p.map_or(0, |p| p[0]) + r.unwrap_or(0) > 0 && (b[0] > 0 || p.map_or(false, |p| p[2] > 100))));
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&(_, k)| k, 10);
    rows(v.into_iter().map(|((u, ((b, p), r)), k)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p.map_or(0, |p| p[0])), V::I(r.unwrap_or(0))]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(p) => [avg(p[1], p[0]), V::I(p[2]), V::I(p[3]), V::I(p[4])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(k));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, U.Views, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY U.CreationDate DESC) AS RecentView,
//        LAG(U.Reputation, 1, 0) OVER (PARTITION BY U.Id ORDER BY U.CreationDate DESC) AS PreviousReputation FROM Users U WHERE U.Reputation > 100),
// ActivePosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.PostTypeId, P.Title, P.CreationDate, COALESCE(P.AcceptedAnswerId, -1) AS AcceptedAnswerId,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownVotes FROM Posts P WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostHistoryDetails AS (SELECT H.PostId, H.UserId, PH.Name AS HistoryType, COUNT(*) AS RevisionCount, MAX(H.CreationDate) AS LastRevisionDate
//     FROM PostHistory H JOIN PostHistoryTypes PH ON H.PostHistoryTypeId = PH.Id GROUP BY H.PostId, H.UserId, PH.Name)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.LastAccessDate, S.PostId, S.Title as PostTitle, S.CommentCount, S.UpVotes, S.DownVotes, COALESCE(PH.HistoryType, 'No Modifications') AS LastHistoryType,
//        PH.RevisionCount, PH.LastRevisionDate,
//        CASE WHEN (U.Reputation - U.PreviousReputation) > 0 THEN 'Increased' WHEN (U.Reputation - U.PreviousReputation) < 0 THEN 'Decreased' ELSE 'No Change' END as ReputationChange
// FROM UserReputation U LEFT JOIN ActivePosts S ON U.UserId = S.OwnerUserId LEFT JOIN PostHistoryDetails PH ON S.PostId = PH.PostId
// WHERE (PH.RevisionCount > 5 OR S.CommentCount > 0) ORDER BY U.Reputation DESC, S.CreationDate DESC LIMIT 100;
//
// Partitioned by its own id, each user is one row, so LAG gives the default 0. The tie at the cut between history groups of one post is broken on (user, type name).
fn q20323(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let PostHistory { post, user, .. } = &db.post_history;
    let phd = db.post_history.group_by(post.and(user.opt()).and(htype_name(db))).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pv = rel(drain(&phd));
    type K = ((Id<Post>, Option<Id<User>>), Str);
    let by_post: HashIdx<Id<Post>, (K, (i64, i64))> = (&pv).map(|(((p, _), _), _)| p).inv().select(&pv).collect();
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let ap: MatSet<Id<Post>> = db.user.with((&db.user.reputation).gt(100)).select(recent).collect();
    let cc = (&ap).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&ap).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let s = (&cc).and(&vc).and((&by_post).opt()).filt(|((c, _), h): ((i64, [i64; 2]), Option<(K, (i64, i64))>)| h.map_or(false, |(_, (n, _))| n > 5) || c > 0);
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select(posts_of(db).select(Ident::<Post>::new().and(s))));
    let v = top_n(v, |&(u, (p, (_, h)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), u, p, h.map(|((k, n), _)| (k.1, n))), 100);
    rows(v.into_iter().map(|(u, (p, ((c, a), h)))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep", "last_access"]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(((_, name), (n, d))) => [V::S(name), V::I(n), V::T(d)],
            None => [V::S("No Modifications"), V::Null, V::Null],
        });
        let d = rep - 0;
        f.push(V::S(if d > 0 { "Increased" } else if d < 0 { "Decreased" } else { "No Change" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COALESCE(b.BadgeCount, 0) AS BadgeCount, COALESCE(v.VoteCount, 0) AS VoteCount,
//        COALESCE(c.CommentCount, 0) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY u.Reputation DESC) AS UserRank
//     FROM Users u LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId LEFT JOIN (SELECT UserId, COUNT(*) AS VoteCount FROM Votes GROUP BY UserId) v ON u.Id = v.UserId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS CommentCount FROM Comments GROUP BY UserId) c ON u.Id = c.UserId),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.Score, p.ViewCount, p.OwnerUserId, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RecentPostRank FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months'),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosed FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT us.DisplayName, us.Reputation, us.BadgeCount, pd.Title AS RecentPostTitle, pd.Score AS PostScore, pd.ViewCount AS PostViewCount, COUNT(DISTINCT cp.PostId) AS ClosedPostCount
// FROM UserStats us JOIN Posts p ON us.UserId = p.OwnerUserId JOIN PostDetails pd ON p.Id = pd.PostId LEFT JOIN ClosedPosts cp ON p.Id = cp.PostId
// WHERE us.Reputation > (SELECT AVG(Reputation) FROM Users) AND pd.RecentPostRank <= 3 GROUP BY us.UserId, us.DisplayName, us.Reputation, us.BadgeCount, pd.Title, pd.Score, pd.ViewCount
// HAVING COUNT(DISTINCT cp.PostId) >= 1 ORDER BY us.Reputation DESC, us.BadgeCount DESC LIMIT 10;
//
// A tie on CreationDate inside RecentPostRank goes to the smaller post id; the answer is empty, so that is untested.
fn q21818(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let Post { creation_date, post_type_id, owner_user, title, score, view_count, .. } = &db.post;
    let (sum, n) = db.user.select(reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let top = top_per(drain(db.post.with(creation_date.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let pd: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).select(&db.post_history.post).collect();
    let above = Ident::<User>::new().with(reputation.filt(move |r| r * n > sum));
    let g = (&pd)
        .with(owner_user.select(above))
        .group_by(owner_user.and(title.opt()).and(score).and(view_count.opt()))
        .select(Ident::<Post>::new().with(&closed).opt())
        .fold(0i64, |k, c| k + c.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |k, b| k + b.is_some() as i64);
    let v = drain((&g).filt(|k| k >= 1));
    let v = drain(rel(v).select(Same::<((((Id<User>, Option<Str>), i64), Option<i64>), i64)>::new().and(Same::<((((Id<User>, Option<Str>), i64), Option<i64>), i64)>::new().map(|((((u, _), _), _), _)| u).select(&bc))));
    let v = top_n(v, |&(_, (((((u, t), s), w), _), b))| (Reverse(reputation.get(u).unwrap()), Reverse(b), u, t, s, w), 10);
    rows(v.into_iter().map(|(_, (((((u, t), s), w), k), b))| row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), ostr(t), V::I(s), oint(w), V::I(k)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpvoteCount
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerUserId, rp.CommentCount, rp.UpvoteCount, p.Tags, u.DisplayName AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Posts p2 WHERE p2.ParentId = rp.PostId AND p2.PostTypeId = 2) AS AnswerCount
//     FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN Posts p ON p.Id = rp.PostId WHERE rp.PostRank = 1 AND (rp.CommentCount > 5 OR rp.UpvoteCount > 10)),
// AggregatedData AS (SELECT fp.OwnerDisplayName, SUM(fp.CommentCount) AS TotalComments, SUM(fp.UpvoteCount) AS TotalUpvotes, COUNT(fp.PostId) AS TotalPosts, AVG(fp.AnswerCount) AS AverageAnswers
//     FROM FilteredPosts fp GROUP BY fp.OwnerDisplayName),
// ClosedPosts AS (SELECT ph.UserId, ph.Comment, ph.CreationDate, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId, ph.Comment, ph.CreationDate)
// SELECT ad.OwnerDisplayName, ad.TotalComments, ad.TotalUpvotes, ad.TotalPosts, ad.AverageAnswers, COALESCE(cp.CloseCount, 0) AS TotalClosedPosts,
//        CASE WHEN ad.TotalPosts > 50 THEN 'Active User' ELSE 'New User' END AS UserType
// FROM AggregatedData ad LEFT JOIN ClosedPosts cp ON ad.OwnerDisplayName = (SELECT DisplayName FROM Users u WHERE u.Id = cp.UserId) ORDER BY ad.TotalUpvotes DESC, ad.TotalComments DESC;
//
// The ownerless posts rank as one partition and group under a NULL OwnerDisplayName, which meets no ClosedPosts row.
fn q21773(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let first = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&first).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uc = (&first).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let ac = (&first).group_by(Ident::<Post>::new()).select(children_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2))).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let fp: MatSet<Id<Post>> = (&first).with((&cc).and(&uc).filt(|(c, u)| c > 5 || u > 10)).collect();
    let ad = (&fp).group_by(owner_user.select(&db.user.display_name).opt()).select((&cc).and(&uc).and(&ac)).fold([0i64; 4], |a, ((c, u), n)| [a[0] + c, a[1] + u, a[2] + 1, a[3] + n]);
    let PostHistory { user, comment, creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let cpf = db.post_history.with(post_history_type_id.eq(10)).group_by(user.opt().and(comment.opt()).and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cpv = rel(drain(&cpf));
    let byname: HashIdx<Str, usize> = (&cpv).flat_map(|(((u, _), _), _): (((Option<Id<User>>, Option<Str>), i64), i64)| u).select(&db.user.display_name).inv().collect();
    type R = (Option<Str>, [i64; 4]);
    let v = drain(rel(drain(&ad)).select(Same::<R>::new().and(Same::<R>::new().flat_map(|(n, _): R| n).select(&byname).select(&cpv).map(|(_, n)| n).opt())));
    rows(v.into_iter().map(|(_, ((n, a), k))| row(vec![ostr(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[2]), V::I(k.unwrap_or(0)), V::S(if a[2] > 50 { "Active User" } else { "New User" })])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RowNum
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount FROM RankedPosts WHERE RowNum <= 5),
// UserPostInteractions AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// UserBadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, up.VoteCount, up.UpVotes, up.DownVotes,
//        ROW_NUMBER() OVER (ORDER BY up.VoteCount DESC, COALESCE(ub.BadgeCount, 0) DESC) AS UserRank
//     FROM Users u LEFT JOIN UserPostInteractions up ON u.Id = up.UserId LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId WHERE u.Reputation > 1000)
// SELECT tp.Title AS PostTitle, tp.CreationDate AS PostDate, tp.Score AS PostScore, tp.CommentCount AS TotalComments, tu.DisplayName AS UserName, tu.BadgeCount AS UserBadges,
//        tu.VoteCount AS TotalVotes, tu.UpVotes, tu.DownVotes
// FROM TopPosts tp JOIN TopUsers tu ON EXISTS (SELECT 1 FROM Posts p WHERE p.Id = tp.PostId AND p.OwnerUserId = tu.UserId) ORDER BY tp.Score DESC, tu.VoteCount DESC;
//
// UserRank is never read. A tie on Score inside RowNum goes to the smaller post id; the answer is empty on this data, so that is untested.
fn q34292(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let up = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tu = Ident::<User>::new().with((&db.user.reputation).gt(1000)).and(&up).and(&bc);
    rows(drain((&cc).and(owner_user.select(tu))).into_iter().map(|(p, (c, ((u, a), b)))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank,
//        COUNT(*) OVER (PARTITION BY p.PostTypeId) AS TotalPosts, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVotes FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, (CASE WHEN u.Reputation >= 1000 THEN 'High' WHEN u.Reputation BETWEEN 500 AND 999 THEN 'Medium' ELSE 'Low' END) AS ReputationCategory
//     FROM Users u WHERE u.LastAccessDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// PostVoteCounts AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId IN (2, 4) THEN 1 ELSE 0 END) AS UsefulVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS HarmfulVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// CombinedData AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.ScoreRank, rp.TotalPosts, rp.UpVotes, rp.DownVotes, ur.ReputationCategory, pvc.UsefulVotes, pvc.HarmfulVotes
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.PostId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) JOIN PostVoteCounts pvc ON rp.PostId = pvc.PostId)
// SELECT cd.PostId, cd.Title, cd.ViewCount, cd.Score, cd.ScoreRank, cd.TotalPosts, cd.UpVotes, cd.DownVotes, cd.ReputationCategory, cd.UsefulVotes, cd.HarmfulVotes
// FROM CombinedData cd WHERE cd.UsefulVotes IS NOT NULL AND (cd.ReputationCategory <> 'Low' OR cd.HarmfulVotes = 0) ORDER BY cd.Score DESC, cd.ViewCount ASC LIMIT 50;
//
// The ON compares a post's id with its own owner's id and names no column of ur, so the posts where the two are equal are crossed with UserReputation.
// A tie on Score inside ScoreRank goes to the smaller post id; the answer is empty on this data, so that is untested.
fn q20180(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, origid, owner_user_id, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1)));
    let tot = rp().group_by(post_type_id).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let rk = ranked(drain(rp().select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false);
    let rk = per_group(rk, |&(_, t)| t);
    let rk = rel(rk.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let same = rp().with(origid.and(owner_user_id).filt(|(a, b)| a == b));
    let pv = same.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 4], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(2 | 4)) as i64, a[3] + (t == Some(3)) as i64]
    });
    let cd = rel(drain((&pv).and((&rank).map(|(_, r)| r)).and(post_type_id.select(&tot))));
    let ur = rel(drain(db.user.with((&db.user.last_access_date).ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30))).select(&db.user.reputation)));
    let cat = |r: i64| if r >= 1000 { "High" } else if (500..=999).contains(&r) { "Medium" } else { "Low" };
    let mut v = Vec::new();
    (&cd).cross(&ur).drive(|_, ((p, ((a, r), n)), (_, rep))| v.push((p, a, r, n, rep)));
    let v = drain(rel(v).filt(move |(_, a, _, _, rep)| cat(rep) != "Low" || a[3] == 0));
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&(p, _, _, _, rep)| (Reverse(score.get(p).unwrap()), view_count.get(p).is_none(), view_count.get(p), p, rep), 50);
    rows(v.into_iter().map(|(p, a, r, n, rep)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(r), V::I(n), V::I(a[0]), V::I(a[1]), V::S(cat(rep)), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(ABS(vote_count.UpVotes - vote_count.DownVotes), 0) AS VoteBalance,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank, DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS CreationRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v INNER JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId) vote_count ON p.Id = vote_count.PostId WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'),
// RecentUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, RANK() OVER (ORDER BY u.CreationDate DESC) AS RecentRank FROM Users u WHERE u.CreationDate >= DATE '2024-10-01' - INTERVAL '6 months'),
// PostHistoryDetails AS (SELECT ph.PostId, MIN(CASE WHEN pht.Name = 'Edit Body' THEN ph.CreationDate END) AS FirstEdit, COUNT(CASE WHEN pht.Name = 'Post Closed' THEN 1 END) AS CloseCount,
//        COUNT(CASE WHEN pht.Name = 'Post Reopened' THEN 1 END) AS ReopenCount FROM PostHistory ph INNER JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT p.Title, p.CreationDate, p.Score, p.VoteBalance, ru.DisplayName AS RecentUser, ru.Reputation AS UserReputation, phd.FirstEdit, phd.CloseCount, phd.ReopenCount
// FROM RankedPosts p LEFT JOIN RecentUsers ru ON ru.UserId = p.OwnerUserId LEFT JOIN PostHistoryDetails phd ON phd.PostId = p.PostId
// WHERE p.Rank <= 5 AND (phd.CloseCount > 0 OR phd.ReopenCount > 0) ORDER BY COALESCE(p.Rank, 0), p.Score DESC, phd.FirstEdit DESC LIMIT 100;
//
// A tie on Score inside Rank goes to the smaller post id; the answer is empty on this data, so that is untested.
fn q24461(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let rk = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false);
    let rk = per_group(rk, |&(_, t)| t);
    let rp = rel(rk.into_iter().filter(|x| x.1 <= 5).map(|((p, _), r)| (p, r)).collect());
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let ru = Ident::<User>::new().with((&db.user.creation_date).ge(add_months(ts(2024, 10, 1, 0, 0, 0), -6)));
    let phd = db.post_history.group_by(&db.post_history.post).select(htype_name(db).and(&db.post_history.creation_date)).fold((i64::MAX, 0i64, 0i64), |(m, c, r), (n, d)| {
        (if n == "Edit Body" { m.min(d) } else { m }, c + (n == "Post Closed") as i64, r + (n == "Post Reopened") as i64)
    });
    type R = (Id<Post>, i64);
    let v = drain((&rp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&phd).filt(|(_, c, r)| c > 0 || r > 0).and((&vc).opt()).and(owner_user.select(ru).opt())))));
    let v = top_n(v, |&(_, ((p, r), ((h, _), _)))| (r, Reverse(score.get(p).unwrap()), h.0 == i64::MAX, Reverse(h.0), p), 100);
    rows(v.into_iter().map(|(_, ((p, _), ((h, b), u)))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.push(V::I(b.map_or(0, |b| (b[0] - b[1]).abs())));
        f.extend(match u {
            Some(u) => ucols(db, u, &["name", "rep"]),
            None => vec![V::Null, V::Null],
        });
        f.extend([tmin(h.0), V::I(h.1), V::I(h.2)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
//        COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS PostRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews FROM UserPostStats WHERE TotalPosts > 0 ORDER BY TotalPosts DESC LIMIT 10),
// PostEngagement AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COUNT(C) AS CommentCount, SUM(V.BountyAmount) AS TotalBounty FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
//     LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate),
// PostDetails AS (SELECT PE.PostId, PE.Title, PE.CreationDate, PE.CommentCount, PE.TotalBounty, COALESCE(PE.CommentCount, 0) AS ActualCommentCount, COALESCE(LT.Name, 'No Link') AS LinkType
//     FROM PostEngagement PE LEFT JOIN PostLinks PL ON PE.PostId = PL.PostId LEFT JOIN LinkTypes LT ON PL.LinkTypeId = LT.Id),
// FinalOutput AS (SELECT TU.UserId, TU.DisplayName, P.Title AS PostTitle, P.CreationDate, P.CommentCount, P.TotalBounty, P.LinkType,
//        CASE WHEN P.CommentCount > 10 THEN 'Popular' ELSE 'Moderate' END AS EngagementLevel FROM TopUsers TU LEFT JOIN PostDetails P ON TU.UserId = P.PostId)
// SELECT UserId, DisplayName, PostTitle, CreationDate, CommentCount, TotalBounty, LinkType, EngagementLevel FROM FinalOutput
// WHERE EngagementLevel = 'Popular' OR (EngagementLevel = 'Moderate' AND TotalBounty > 0) ORDER BY CreationDate DESC;
//
// `TU.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids. COUNT(C) counts every joined row (translation-failures 11).
// CreationDate is compared with CURRENT_TIMESTAMP as an instant in the session zone.
fn q22416(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tu = rel(top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10));
    let since = now_utc() - 30 * DAY_US;
    let pidx: HashIdx<i64, Id<Post>> = db.post.with(creation_date.filt(move |d| ny_to_utc(d) >= since)).select(&db.post.origid).inv().collect();
    let pe = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 3], |a, (_, b)| match b.flatten() {
        Some(b) => [a[0] + 1, a[1] + 1, a[2] + b],
        None => [a[0] + 1, a[1], a[2]],
    });
    let lt = links_of(db).select((&db.post_link.link_type).select(&db.link_type.name));
    type R = (Id<User>, i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pe).and(lt.opt()))))));
    let v = drain(rel(v).filt(|(_, (_, ((_, a), _)))| a[0] > 10 || (a[1] > 0 && a[2] > 0)));
    rows(v.into_iter().map(|(_, (_, ((u, _), ((p, a), l))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::S(l.unwrap_or("No Link")), V::S(if a[0] > 10 { "Popular" } else { "Moderate" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, LastAccessDate, CASE WHEN Reputation IS NULL THEN 'Unknown Reputation' WHEN Reputation < 100 THEN 'Low Reputation'
//        WHEN Reputation BETWEEN 100 AND 1000 THEN 'Medium Reputation' ELSE 'High Reputation' END AS ReputationCategory FROM Users),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.PostTypeId, COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS TotalComments,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes, P.OwnerUserId,
//        RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY COUNT(V.Id) DESC) AS UserPostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.PostTypeId, P.AcceptedAnswerId, P.OwnerUserId),
// PopularPosts AS (SELECT PS.PostId, PS.Title, PS.UpVotes, PS.DownVotes, PS.TotalComments, PS.CreationDate, U.DisplayName AS OwnerDisplayName, U.Reputation, PS.UserPostRank
//     FROM PostStats PS JOIN UserReputation U ON PS.OwnerUserId = U.Id WHERE U.Reputation IS NOT NULL)
// SELECT PP.PostId, PP.Title, PP.OwnerDisplayName, PP.UpVotes, PP.DownVotes, PP.TotalComments,
//        CASE WHEN PP.UserPostRank = 1 THEN 'Top Contributor' WHEN PP.UserPostRank BETWEEN 2 AND 5 THEN 'Notable Contributor' ELSE 'Regular Contributor' END AS ContributorStatus,
//        DATE_PART('day', TIMESTAMP '2024-10-01 12:34:56' - PP.CreationDate) AS DaysSincePosted,
//        CASE WHEN PP.UpVotes - PP.DownVotes > 50 THEN 'Highly Upvoted' WHEN PP.UpVotes < PP.DownVotes THEN 'More Downvotes Than Upvotes' ELSE 'Moderate Engagement' END AS EngagementStatus
// FROM PopularPosts PP WHERE PP.UpVotes > 10 OR (PP.TotalComments > 5 AND PP.UserPostRank < 4) ORDER BY PP.UpVotes DESC, DaysSincePosted ASC LIMIT 100;
fn q24149(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + t.is_some() as i64]);
    let v = drain((&ps).and(owner_user));
    let v = ranked(v, |&(_, (a, u))| (u, Reverse(a[3])), false);
    let v = per_group(v, |&(_, (_, u))| u);
    let v = drain(rel(v).filt(|((_, (a, _)), r)| a[1] > 10 || (a[0] > 5 && r < 4)));
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let days = |p: Id<Post>| (t0 - creation_date.get(p).unwrap()) / DAY_US;
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&((p, (a, _)), _)| (Reverse(a[1]), days(p), p), 100);
    rows(v.into_iter().map(|((p, (a, u)), r)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[0])]);
        f.push(V::S(if r == 1 { "Top Contributor" } else if (2..=5).contains(&r) { "Notable Contributor" } else { "Regular Contributor" }));
        f.push(V::I(days(p)));
        f.push(V::S(if a[1] - a[2] > 50 { "Highly Upvoted" } else if a[1] < a[2] { "More Downvotes Than Upvotes" } else { "Moderate Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.OwnerUserId,
//        RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank FROM Posts P),
// UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(P.Score) AS TotalScore, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName),
// TopUserStats AS (SELECT US.UserId, US.DisplayName, US.TotalPosts, US.TotalScore, US.TotalBounty, ROW_NUMBER() OVER (ORDER BY US.TotalScore DESC) AS ScoreRanking FROM UserStatistics US WHERE US.TotalPosts > 10),
// RecentPostInfo AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerUserId, US.TotalPosts FROM RankedPosts RP JOIN TopUserStats US ON RP.OwnerUserId = US.UserId WHERE RP.PostRank = 1)
// SELECT RPI.Title, RPI.CreationDate, RPI.Score, RPI.ViewCount, US.DisplayName, US.TotalPosts, COALESCE(PHT.Name, 'No Change') AS PostHistoryType, COUNT(CM.Id) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes
// FROM RecentPostInfo RPI LEFT JOIN PostHistory PH ON RPI.PostId = PH.PostId LEFT JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id LEFT JOIN Comments CM ON RPI.PostId = CM.PostId
// LEFT JOIN Votes V ON RPI.PostId = V.PostId JOIN UserStatistics US ON RPI.OwnerUserId = US.UserId
// GROUP BY RPI.Title, RPI.CreationDate, RPI.Score, RPI.ViewCount, US.DisplayName, US.TotalPosts, PHT.Name ORDER BY RPI.CreationDate DESC;
//
// The GROUP BY names post and history-type columns but not the post id, so the (post, history) rows are materialised and grouped by those columns.
fn q32661(db: &'static So) -> String {
    let Post { owner_user, creation_date, title, score, view_count, .. } = &db.post;
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let first = top_per(drain(db.post.select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = (&first).with(owner_user.select((&tp).filt(|n| n > 10))).select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let h_of = (&j).flat_map(|(_, h)| h);
    let key = (&post_of)
        .select(title.opt())
        .and((&post_of).select(creation_date))
        .and((&post_of).select(score))
        .and((&post_of).select(view_count.opt()))
        .and((&post_of).select(owner_user).select(&db.user.display_name))
        .and((&post_of).select(owner_user).select(&tp))
        .and((&h_of).select(htype_name(db)).opt());
    let g = (&j)
        .group_by(key)
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(2)) as i64]);
    rows(drain(&g).into_iter().map(|(((((((t, d), s), w), n), k), h), a)| {
        row(vec![ostr(t), V::T(d), V::I(s), oint(w), V::S(n), V::I(k), V::S(h.unwrap_or("No Change")), V::I(a[0]), V::I(a[1]), V::I(a[2])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, u.DisplayName AS OwnerDisplayName FROM RankedPosts rp INNER JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.PostRank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, AVG(c.Score) AS AverageCommentScore FROM Comments c GROUP BY c.PostId),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END) AS NetVoteScore FROM Votes v GROUP BY v.PostId),
// FinalResults AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score + COALESCE(pc.CommentCount, 0) AS TotalEngagementScore, tp.ViewCount, tp.OwnerDisplayName,
//        COALESCE(pv.NetVoteScore, 0) AS NetVoteScore,
//        CASE WHEN tp.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 'New'
//             WHEN tp.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND tp.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '60 days' THEN 'Moderate'
//             ELSE 'Old' END AS PostAgeCategory
//     FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId LEFT JOIN PostVotes pv ON tp.PostId = pv.PostId)
// SELECT fr.*, CASE WHEN fr.NetVoteScore > 0 THEN 'Positive' WHEN fr.NetVoteScore < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM FinalResults fr ORDER BY fr.TotalEngagementScore DESC, fr.NetVoteScore DESC;
//
// PostRank is taken over every post, owned or not, before the join to Users drops the ownerless ones.
fn q33808(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| {
        (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).with(owner_user).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let nv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + match t {
        Some(2) => 1,
        Some(3) => -1,
        _ => 0,
    });
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    rows(drain((&cc).and(&nv)).into_iter().map(|(p, (c, n))| {
        let d = creation_date.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::I(score.get(p).unwrap() + c));
        f.extend(post_fields(db, p, &["views", "owner"]));
        f.push(V::I(n));
        f.push(V::S(if d >= add_days(t0, -30) { "New" } else if d >= add_days(t0, -60) { "Moderate" } else { "Old" }));
        f.push(V::S(if n > 0 { "Positive" } else if n < 0 { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank, COUNT(b.Id) AS BadgeCount,
//        MAX(u.CreationDate) AS LastAccountCreate FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostViewCounts AS (SELECT p.OwnerUserId, SUM(p.ViewCount) AS TotalViews FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// PostStatistics AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COALESCE(v.VoteCount, 0) AS UpVoteCount, COALESCE(v.DownVoteCount, 0) AS DownVoteCount,
//        MAX(p.CreationDate) AS LatestPostDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS VoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.CreationDate BETWEEN cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND cast('2024-10-01 12:34:56' as timestamp) GROUP BY p.Id, p.OwnerUserId, v.VoteCount, v.DownVoteCount)
// SELECT ru.DisplayName, ru.Reputation, ru.UserRank, ps.PostId, ps.CommentCount, pv.TotalViews,
//        CASE WHEN ps.UpVoteCount > ps.DownVoteCount THEN 'More Upvotes' WHEN ps.UpVoteCount < ps.DownVoteCount THEN 'More Downvotes' ELSE 'Equal Votes' END AS VoteStatus,
//        CASE WHEN ru.Reputation >= 1000 THEN 'High Reputation' WHEN ru.Reputation BETWEEN 500 AND 999 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationStatus
// FROM RankedUsers ru LEFT JOIN PostStatistics ps ON ru.UserId = ps.OwnerUserId LEFT JOIN PostViewCounts pv ON ru.UserId = pv.OwnerUserId
// WHERE ru.UserRank <= 50 AND ps.CommentCount > 0 ORDER BY ru.Reputation DESC, ps.CommentCount DESC;
//
// A tie on Reputation inside UserRank goes to the smaller user id.
fn q24581(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ur = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 50);
    let ur = rel(ur.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let pv = db.post.with(creation_date.gt(add_years(t0, -1))).group_by(owner_user).select(view_count.opt()).fold((0i64, 0i64), |(n, s), w| (n + w.is_some() as i64, s + w.unwrap_or(0)));
    let in_year = Ident::<Post>::new().with(creation_date.between(add_years(t0, -1), t0));
    let ps = db.post.with(creation_date.between(add_years(t0, -1), t0)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type R = (Id<User>, i64);
    let v = drain((&ur).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(posts_of(db).select(in_year).select(Ident::<Post>::new().and((&ps).filt(|c| c > 0)).and((&vc).opt()))).and(Same::<R>::new().map(|(u, _): R| u).select((&pv).opt())))));
    rows(v.into_iter().map(|(_, ((u, r), (((p, c), a), w)))| {
        let rep = db.user.reputation.get(u).unwrap();
        let a = a.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(r), post_fields(db, p, &["id"]).remove(0), V::I(c), w.map_or(V::Null, |(n, s)| nullable(s, n))]);
        f.push(V::S(if a[0] > a[1] { "More Upvotes" } else if a[0] < a[1] { "More Downvotes" } else { "Equal Votes" }));
        f.push(V::S(if rep >= 1000 { "High Reputation" } else if (500..=999).contains(&rep) { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 1000),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
//        AVG(p.Score) AS AverageScore FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserBadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// PostHistoryData AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS HistoryCount FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId HAVING COUNT(*) > 1),
// UserPostInfo AS (SELECT p.OwnerUserId, MAX(p.CreationDate) AS LastPostDate, MIN(p.CreationDate) AS FirstPostDate, COUNT(p.Id) AS TotalPosts, SUM(COALESCE(ph.HistoryCount, 0)) AS TotalHistoryChanges
//     FROM Posts p LEFT JOIN PostHistoryData ph ON p.Id = ph.PostId GROUP BY p.OwnerUserId)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.Views, ubc.BadgeCount, p.TotalPosts, p.PositivePosts, p.NegativePosts, p.AverageScore, pii.LastPostDate, pii.TotalHistoryChanges,
//        CASE WHEN pii.TotalPosts = 0 THEN 'No posts' ELSE CAST((SELECT COUNT(*) FROM Comments c WHERE c.PostId IN (SELECT Id FROM Posts WHERE OwnerUserId = u.UserId)) AS TEXT) END AS TotalComments
// FROM RankedUsers u LEFT JOIN UserBadgeCounts ubc ON u.UserId = ubc.UserId LEFT JOIN PostStats p ON u.UserId = p.OwnerUserId LEFT JOIN UserPostInfo pii ON u.UserId = pii.OwnerUserId
// WHERE u.ReputationRank <= 50 ORDER BY u.Reputation DESC NULLS LAST;
fn q20438(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let rr = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(rr.into_iter().take_while(|x| x.1 <= 50).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ps = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score).fold([0i64; 4], |a, s| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + s]);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let phd = db.post_history.group_by(post.and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pv = rel(drain((&phd).filt(|n| n > 1)));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&pv).map(|((p, _), _)| p).inv().select(&pv).collect();
    let pii = db.post.group_by(owner_user).select(creation_date.and((&by_post).map(|(_, n)| n).opt())).fold((i64::MIN, 0i64, 0i64), |(m, n, s), (d, h)| (m.max(d), n + 1, s + h.unwrap_or(0)));
    let tc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tu).select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()).and((&pii).opt()).and(&tc)));
    rows(v.into_iter().map(|(_, ((((u, b), p), i), c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.push(oint(b));
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match i {
            Some((m, _, s)) => [V::T(m), V::I(s)],
            None => [V::Null, V::Null],
        });
        f.push(if i.map(|x| x.1) == Some(0) { V::S("No posts") } else { V::Owned(c.to_string()) });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY u.Id ORDER BY COUNT(DISTINCT p.Id) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN Comments c ON p.Id = c.PostId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, UpVotes, DownVotes, CommentCount FROM UserActivity WHERE Rank <= 10),
// PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title),
// HighScoringPosts AS (SELECT ps.PostId, ps.Title, ps.TotalComments, ps.TotalUpVotes, ps.TotalDownVotes, RANK() OVER (ORDER BY ps.TotalUpVotes - ps.TotalDownVotes DESC) AS PostRank
//     FROM PostStats ps WHERE ps.TotalUpVotes > 0)
// SELECT t.DisplayName, t.PostCount, t.UpVotes, t.DownVotes, t.CommentCount, h.PostId, h.Title, h.TotalComments, h.TotalUpVotes, h.TotalDownVotes
// FROM TopUsers t LEFT JOIN HighScoringPosts h ON t.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = h.PostId) WHERE h.PostRank <= 5 ORDER BY t.UpVotes DESC, h.TotalUpVotes - h.TotalDownVotes DESC;
//
// Partitioned by its own id, every user has Rank 1, so TopUsers is every user over 100 reputation. The WHERE on h.PostRank makes the LEFT JOIN inner, so the five top posts
// are picked first and UserActivity is folded for their owners alone.
fn q764(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cv = || comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt());
    let ps = db.post.select(recent()).group_by(Ident::<Post>::new()).select(cv()).fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let hs = ranked(drain((&ps).filt(|a| a[1] > 0)), |&(_, a)| Reverse(a[1] - a[2]), false);
    let hs: MatSet<Id<Post>> = rel(hs.into_iter().take_while(|x| x.1 <= 5).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&hs).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100)))).collect();
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(recent()).select(cv()).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((c, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64],
            None => a,
        });
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(recent()).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&hs).select(Ident::<Post>::new().and(&ps).and(owner_user.select(Ident::<User>::new().and(&ua).and(&pc)))));
    rows(v.into_iter().map(|(_, ((p, h), ((u, a), n)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend(h.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Author, p.Score, p.ViewCount, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Author, rp.Score, rp.ViewCount, rp.AnswerCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON c.PostId = rp.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) WHERE rp.rn = 1),
// FilteredPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Author, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.BadgeCount, DENSE_RANK() OVER (ORDER BY pd.Score DESC) AS ScoreRank
//     FROM PostDetails pd WHERE pd.Score > 5 AND (pd.CommentCount = 0 OR pd.BadgeCount > 1))
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Author, fp.Score, fp.ViewCount, fp.AnswerCount, fp.CommentCount, fp.BadgeCount, CASE WHEN fp.ScoreRank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM FilteredPosts fp WHERE EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = fp.PostId AND v.VoteTypeId IN (2, 3) AND v.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days') ORDER BY fp.ScoreRank;
//
// A tie on CreationDate inside rn goes to the smaller post id; the answer is empty on this data, so that is untested.
fn q31219(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let since = add_days(ts(2024, 10, 1, 0, 0, 0), -30);
    let top = top_per(drain(db.post.with(creation_date.ge(since)).with(owner_user).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let voted: MatSet<Id<Post>> = db.vote.with((&db.vote.vote_type_id).is_in([2, 3]).and((&db.vote.creation_date).ge(since))).select(&db.vote.post).collect();
    let fp = drain((&tp).with(score.gt(5)).select(Ident::<Post>::new().and(&cc).and(owner_user.select(&bc))).filt(|((_, c), b): ((Id<Post>, i64), i64)| c == 0 || b > 1));
    let fp = ranked(fp, |&(_, ((p, _), _))| Reverse(score.get(p).unwrap()), true);
    type X = ((Id<Post>, ((Id<Post>, i64), i64)), i64);
    let v = drain(rel(fp).select(Same::<X>::new().with(Same::<X>::new().map(|((_, ((p, _), _)), _): X| p).select(Ident::<Post>::new().with(&voted)))));
    rows(v.into_iter().map(|(_, ((_, ((p, c), b)), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views", "answers"]);
        f.extend([V::I(c), V::I(b), V::S(if r <= 5 { "Top Post" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStatistics AS (SELECT p.Id AS PostId, p.OwnerUserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(c.Id) AS CommentCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount, MAX(COALESCE(p.LastActivityDate, p.CreationDate)) AS MostRecentActivity
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId GROUP BY p.Id, p.OwnerUserId),
// AcceptedAnswers AS (SELECT p.Id AS QuestionId, a.Id AS AcceptedAnswerId, a.OwnerUserId AS AnswerOwnerId FROM Posts p JOIN Posts a ON p.AcceptedAnswerId = a.Id WHERE p.PostTypeId = 1),
// UserPostDetails AS (SELECT ur.UserId, ur.DisplayName, ps.PostId, ps.Upvotes, ps.Downvotes, COALESCE(aa.AcceptedAnswerId, 0) AS AcceptedAnswerId, ps.CommentCount, ps.RelatedPostCount,
//        CASE WHEN ps.Upvotes > ps.Downvotes THEN 'Positive' WHEN ps.Upvotes < ps.Downvotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
//     FROM PostStatistics ps JOIN UserReputation ur ON ps.OwnerUserId = ur.UserId LEFT JOIN AcceptedAnswers aa ON ps.PostId = aa.QuestionId)
// SELECT ud.DisplayName, ud.PostId, ud.Upvotes, ud.Downvotes, ud.AcceptedAnswerId, ud.CommentCount, ud.RelatedPostCount, ud.VoteSentiment, COALESCE(u.ReputationRank, 0) AS UserReputationRank
// FROM UserPostDetails ud LEFT JOIN UserReputation u ON ud.UserId = u.UserId WHERE ud.VoteSentiment = 'Positive' AND (ud.CommentCount > 5 OR ud.RelatedPostCount > 2) ORDER BY u.ReputationRank, ud.Upvotes DESC;
fn q1840(db: &'static So) -> String {
    let Post { owner_user, post_type_id, accepted_answer, .. } = &db.post;
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let ps = db
        .post
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(links_of(db).opt()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let rc = db.post.with(owner_user).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let aa = Ident::<Post>::new().with(post_type_id.eq(1)).select(accepted_answer).select(&db.post.origid);
    let v = drain((&ps).and((&rc).opt()).filt(|(a, r): ([i64; 3], Option<i64>)| a[0] > a[1] && (a[2] > 5 || r.unwrap_or(0) > 2)).and(aa.opt()).and(owner_user.select(&rank)));
    rows(v.into_iter().map(|(p, (((a, r), x), (u, k)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(x.unwrap_or(0)), V::I(a[2]), V::I(r.unwrap_or(0)), V::S("Positive"), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// PostHistoryAggregates AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (11, 53) THEN 1 END) AS ReopenCount, MAX(CASE WHEN ph.PostHistoryTypeId = 6 THEN ph.CreationDate END) AS LastEditDate FROM PostHistory ph GROUP BY ph.PostId),
// UserReputation AS (SELECT u.Id, SUM(CASE WHEN b.Class = 1 THEN 3 WHEN b.Class = 2 THEN 2 WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBadgePoints FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, u.Reputation + COALESCE(ur.TotalBadgePoints, 0) AS TotalReputation, ROW_NUMBER() OVER (ORDER BY u.LastAccessDate DESC) AS rn
//     FROM Users u LEFT JOIN UserReputation ur ON u.Id = ur.Id WHERE u.LastAccessDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'))
// SELECT rp.Title, rp.OwnerDisplayName, rp.CreationDate, COALESCE(ph.CloseCount, 0) AS CloseCount, COALESCE(ph.DeleteCount, 0) AS DeleteCount, COALESCE(ph.ReopenCount, 0) AS ReopenCount,
//        ph.LastEditDate, au.DisplayName AS ActiveUser, au.TotalReputation
// FROM RankedPosts rp LEFT JOIN PostHistoryAggregates ph ON rp.Id = ph.PostId LEFT JOIN ActiveUsers au ON au.TotalReputation >= 100 WHERE rp.rn = 1
// ORDER BY rp.CreationDate DESC, au.TotalReputation DESC LIMIT 50;
//
// Partitioned by its own id, every question has rn 1. The ON names only au, so questions and active users are crossed; the order leads with the question's
// CreationDate, so only the 50 newest questions can reach the LIMIT and they alone are crossed.
fn q20040(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let qs = top_n(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(p, d)| (Reverse(d), p), 50);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([0, 0, 0, i64::MIN], |a, (t, d)| {
        [a[0] + (t == 10) as i64, a[1] + (t == 12) as i64, a[2] + matches!(t, 11 | 53) as i64, if t == 6 { a[3].max(d) } else { a[3] }]
    });
    let rp = rel(drain(rel(qs).select(Same::<(Id<Post>, i64)>::new().map(|(p, _)| p).select(Ident::<Post>::new().and((&pha).opt())))).into_iter().map(|x| x.1).collect());
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(None::<i64>, |s, c| match c {
        Some(c) => Some(s.unwrap_or(0) + match c {
            1 => 3,
            2 => 2,
            3 => 1,
            _ => 0,
        }),
        None => s,
    });
    let au = drain(db.user.with((&db.user.last_access_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select((&db.user.reputation).and(&ur)).map(|(r, b)| r + b.unwrap_or(0)).filt(|t| t >= 100));
    let au = left_all(au);
    let mut v = Vec::new();
    (&rp).cross(&au).drive(|_, ((p, h), a)| v.push((p, h, a)));
    let v = top_n(v, |&(p, _, a)| (Reverse(creation_date.get(p).unwrap()), a.is_none(), Reverse(a.map(|a| a.1)), p, a.map(|a| a.0)), 50);
    rows(v.into_iter().map(|(p, h, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        let h = h.unwrap_or([0, 0, 0, i64::MIN]);
        f.extend([V::I(h[0]), V::I(h[1]), V::I(h[2]), tmax(h[3])]);
        f.extend(match a {
            Some((u, t)) => [user_col(db, u, "name"), V::I(t)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedQuestions AS (SELECT Q.Id AS QuestionId, Q.Title, Q.CreationDate, Q.ViewCount, Q.OwnerUserId, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY EXTRACT(YEAR FROM Q.CreationDate) ORDER BY Q.ViewCount DESC) AS Rank FROM Posts AS Q JOIN Users AS U ON Q.OwnerUserId = U.Id
//     WHERE Q.PostTypeId = 1 AND Q.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// CloseVotes AS (SELECT PH.PostId, PH.Comment, PH.CreationDate AS CloseDate, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS VoteCount FROM PostHistory AS PH WHERE PH.PostHistoryTypeId = 10),
// TopBadgedUsers AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount FROM Users AS U LEFT JOIN Badges AS B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName HAVING COUNT(B.Id) > 5),
// RecentActivePosts AS (SELECT P.Id AS PostId, P.Title, P.LastActivityDate, P.AnswerCount, P.CommentCount, P.ViewCount, R.OwnerDisplayName, COALESCE(CV.CloseCount, 0) AS CloseCount, P.OwnerUserId
//     FROM Posts AS P LEFT JOIN RankedQuestions AS R ON P.Id = R.QuestionId LEFT JOIN (SELECT PostId, COUNT(*) AS CloseCount FROM CloseVotes GROUP BY PostId) AS CV ON P.Id = CV.PostId
//     WHERE P.LastActivityDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND P.PostTypeId IN (1, 2))
// SELECT RA.Title, RA.ViewCount, RA.OwnerDisplayName, COALESCE(BU.BadgeCount, 0) AS BadgeCount, RA.CloseCount, CASE WHEN RA.CloseCount > 0 THEN 'Closed' ELSE 'Active' END AS PostStatus
// FROM RecentActivePosts AS RA LEFT JOIN TopBadgedUsers AS BU ON RA.OwnerUserId = BU.UserId ORDER BY RA.ViewCount DESC, RA.LastActivityDate DESC LIMIT 100;
fn q30169(db: &'static So) -> String {
    let Post { post_type_id, creation_date, last_activity_date, owner_user, view_count, .. } = &db.post;
    let rq = Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user);
    let cv = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let bu = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain(db.post.with(last_activity_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.is_in([1, 2]))).select(rq.opt().and((&cv).opt()).and(owner_user.select((&bu).filt(|n| n > 5)).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(last_activity_date.get(p).unwrap()), p)
    }, 100);
    rows(v.into_iter().map(|(p, ((r, c), b))| {
        let c = c.unwrap_or(0);
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([r.map_or(V::Null, |u| user_col(db, u, "name")), V::I(b.unwrap_or(0)), V::I(c), V::S(if c > 0 { "Closed" } else { "Active" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PopularTags AS (SELECT t.Id AS TagId, t.TagName, COUNT(p.Id) AS PostsCount, SUM(p.Score) AS TotalScore FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.Id, t.TagName HAVING COUNT(p.Id) > 0),
// RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, RANK() OVER (ORDER BY p.Score DESC) AS RankScore, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// ActiveUserPosts AS (SELECT ua.UserId, ua.DisplayName, COUNT(rp.PostId) AS ActivePostsCount, SUM(rp.ViewCount) AS TotalViews, SUM(rp.RankScore) AS TotalRank
//     FROM UserActivity ua JOIN RankedPosts rp ON ua.UserId = rp.OwnerUserId GROUP BY ua.UserId, ua.DisplayName HAVING COUNT(rp.PostId) > 0)
// SELECT ua.DisplayName, ua.TotalPosts, ua.QuestionsCount, ua.AnswersCount, ua.UpVotes, ua.DownVotes, ap.ActivePostsCount, ap.TotalViews, ap.TotalRank, pt.TagName, pt.PostsCount, pt.TotalScore
// FROM UserActivity ua JOIN ActiveUserPosts ap ON ua.UserId = ap.UserId LEFT JOIN PopularTags pt ON pt.PostsCount = (SELECT MAX(PostsCount) FROM PopularTags)
// WHERE ua.TotalPosts >= 10 ORDER BY ua.UpVotes DESC, ua.TotalPosts DESC;
//
// The ON of PopularTags names only pt, so the users are crossed with the tags that have the largest post count.
fn q33813(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, view_count, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 4], |a, x| match x {
        Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
        None => a,
    });
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let rs = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(_, s)| Reverse(s), false);
    let rs = rel(rs.into_iter().map(|((p, _), r)| (p, r)).collect());
    type R = (Id<Post>, i64);
    let ap = (&rs).group_by(Same::<R>::new().map(|(p, _): R| p).select(owner_user)).select(Same::<R>::new().map(|(p, r): R| (view_count.get(p), r))).fold([0i64; 4], |a, (w, r)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + r]
    });
    let ts_ = tag_stats(db);
    let mx = (&ts_).filt(|a| a[0] > 0).select(Same::<[i64; 6]>::new()).fold_flat(0i64, |m, a| m.max(a[0]));
    let pt = left_all(drain((&ts_).filt(move |a| a[0] > 0 && a[0] == mx)));
    let us = rel(drain(db.user.select((&tp).filt(|n| n >= 10).and(&ua).and(&ap))));
    let mut v = Vec::new();
    (&us).cross(&pt).drive(|_, ((u, ((n, a), b)), t)| v.push((u, n, a, b, t)));
    rows(v.into_iter().map(|(u, n, a, b, t)| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.extend([V::I(b[0]), nullable(b[2], b[1]), V::I(b[3])]);
        f.extend(match t {
            Some((t, c)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(c[0]), V::I(c[3])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// ClosedPosts AS (SELECT ph.UserId, ph.PostId, ph.CreationDate, MIN(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS CloseDate FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.UserId, ph.PostId, ph.CreationDate),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.QuestionCount, 0) AS QuestionCount,
//        COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(ps.TotalViews, 0) AS TotalViews, COUNT(cp.PostId) AS ClosedPostCount, MAX(cp.CloseDate) AS LastClosedDate
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN ClosedPosts cp ON u.Id = cp.UserId
//     GROUP BY u.Id, u.DisplayName, ub.BadgeCount, ps.PostCount, ps.QuestionCount, ps.TotalScore, ps.TotalViews)
// SELECT ua.UserId, ua.DisplayName, ua.BadgeCount, ua.PostCount, ua.QuestionCount, ua.TotalScore, ua.TotalViews, ua.ClosedPostCount, ua.LastClosedDate,
//        CASE WHEN ua.BadgeCount >= 10 THEN 'Gold' WHEN ua.BadgeCount >= 5 THEN 'Silver' ELSE 'Bronze' END AS BadgeLevel,
//        CASE WHEN ua.ClosedPostCount > 0 THEN 'Has Closed Posts' ELSE 'No Closed Posts' END AS PostClosureStatus
// FROM UserActivity ua WHERE ua.TotalViews > 1000 OR ua.QuestionCount > 5 ORDER BY ua.TotalScore DESC, ua.TotalViews DESC LIMIT 100;
//
// BadgeNames is never read.
fn q22580(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ps = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 4], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + s, a[3] + w.unwrap_or(0)]
    });
    let PostHistory { user, post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cpf = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(user.and(post).and(hd)).select(post_history_type_id.and(hd)).fold(i64::MAX, |m, (t, d)| if t == 10 { m.min(d) } else { m });
    let cpv = rel(drain(&cpf));
    type C = (((Id<User>, Id<Post>), i64), i64);
    let cu = (&cpv).group_by(Same::<C>::new().map(|(((u, _), _), _): C| u)).select(Same::<C>::new().map(|(_, m): C| m)).fold((0i64, i64::MIN), |(n, x), m| (n + 1, if m == i64::MAX { x } else { x.max(m) }));
    let v = drain(db.user.select((&ub).opt().and((&ps).opt()).and((&cu).opt())).filt(|((_, p), _): ((Option<i64>, Option<[i64; 4]>), Option<(i64, i64)>)| p.map_or(false, |p| p[3] > 1000 || p[1] > 5)));
    let v = top_n(v, |&(u, ((_, p), _))| {
        let p = p.unwrap();
        (Reverse(p[2]), Reverse(p[3]), u)
    }, 100);
    rows(v.into_iter().map(|(u, ((b, p), c))| {
        let b = b.unwrap_or(0);
        let p = p.unwrap();
        let (n, m) = c.unwrap_or((0, i64::MIN));
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), V::I(n), tmax(m)]);
        f.push(V::S(if b >= 10 { "Gold" } else if b >= 5 { "Silver" } else { "Bronze" }));
        f.push(V::S(if n > 0 { "Has Closed Posts" } else { "No Closed Posts" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, TotalBounty, RANK() OVER (ORDER BY PostCount DESC, Reputation DESC) AS Rank FROM UserStats),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, U.DisplayName AS OwnerDisplayName, COUNT(C.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY P.Id, P.Title, P.Score, P.CreationDate, U.DisplayName),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, OwnerDisplayName, CommentCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Score DESC, UpVotes DESC) AS Rank FROM PostDetails)
// SELECT TU.Rank AS UserRank, TU.DisplayName AS UserName, TU.Reputation AS UserReputation, TP.Rank AS PostRank, TP.Title AS PostTitle, TP.Score AS PostScore, TP.CommentCount AS PostCommentCount,
//        TP.UpVotes AS PostUpVotes, TP.DownVotes AS PostDownVotes
// FROM TopUsers TU JOIN TopPosts TP ON TU.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = TP.PostId) WHERE TU.Rank <= 10 AND TP.Rank <= 10 ORDER BY TU.Rank, TP.Rank;
fn q6403(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt()).fold([0i64; 2], |a, x| match x {
        Some((t, _)) => [a[0] + 1, a[1] + (t == 2) as i64],
        None => a,
    });
    let tu = ranked(drain(&us), |&(u, a)| (Reverse(a[0]), Reverse(db.user.reputation.get(u).unwrap())), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let urank: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let pd = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tp = ranked(drain(&pd), |&(p, a)| (Reverse(score.get(p).unwrap()), Reverse(a[1])), false);
    let tp = rel(tp.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), r)| (p, a, r)).collect());
    type R = (Id<Post>, [i64; 3], i64);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _): R| p).select(owner_user).select(&urank))));
    rows(v.into_iter().map(|(_, ((p, a, r), (u, k)))| {
        let mut f = vec![V::I(k), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(r)];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostActivity AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY p.OwnerUserId),
// UserEngagement AS (SELECT u.Id AS UserId, COALESCE(b.GoldBadges, 0) AS GoldBadges, COALESCE(b.SilverBadges, 0) AS SilverBadges, COALESCE(b.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(p.PostCount, 0) AS PostCount, COALESCE(p.TotalBounty, 0) AS TotalBounty, COALESCE(p.Questions, 0) AS Questions, COALESCE(p.Answers, 0) AS Answers
//     FROM Users u LEFT JOIN UserBadgeStats b ON u.Id = b.UserId LEFT JOIN PostActivity p ON u.Id = p.OwnerUserId),
// UserRankedEngagement AS (SELECT ue.*, RANK() OVER (ORDER BY (GoldBadges * 3 + SilverBadges * 2 + BronzeBadges) + PostCount * 0.5 DESC) AS EngagementRank FROM UserEngagement ue)
// SELECT UserId, GoldBadges, SilverBadges, BronzeBadges, PostCount, TotalBounty, Questions, Answers, EngagementRank,
//        CASE WHEN EngagementRank IS NULL THEN 'Unranked' WHEN EngagementRank <= 5 THEN 'Top Engaged' WHEN EngagementRank <= 10 THEN 'Moderately Engaged' ELSE 'Less Engaged' END AS EngagementCategory,
//        CASE WHEN TotalBounty > 500 THEN 'High Bounty Contributor' WHEN TotalBounty BETWEEN 100 AND 500 THEN 'Moderate Bounty Contributor' ELSE 'Low Bounty Contributor' END AS BountyContributorCategory
// FROM UserRankedEngagement WHERE EngagementRank IS NOT NULL ORDER BY EngagementRank;
//
// The rank key `badges + PostCount * 0.5` is ordered as twice itself, an integer.
fn q24183(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let Post { owner_user, post_type_id, .. } = &db.post;
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let pa = db.post.group_by(owner_user).select(post_type_id.and(bv.opt())).fold([0i64; 4], |a, (t, b)| [a[0] + 1, a[1] + b.flatten().unwrap_or(0), a[2] + (t == 1) as i64, a[3] + (t == 2) as i64]);
    let v = drain((&ub).and((&pa).opt()));
    let v = ranked(v, |&(_, (b, p))| Reverse(2 * (b[0] * 3 + b[1] * 2 + b[2]) + p.map_or(0, |p| p[0])), false);
    rows(v.into_iter().map(|((u, (b, p)), r)| {
        let p = p.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(b.map(V::I));
        f.extend(p.map(V::I));
        f.push(V::I(r));
        f.push(V::S(if r <= 5 { "Top Engaged" } else if r <= 10 { "Moderately Engaged" } else { "Less Engaged" }));
        f.push(V::S(if p[1] > 500 { "High Bounty Contributor" } else if (100..=500).contains(&p[1]) { "Moderate Bounty Contributor" } else { "Low Bounty Contributor" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, u.UpVotes, u.DownVotes, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.Score) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views, u.UpVotes, u.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, UpVotes, DownVotes, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS RankByScore FROM UserStats),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.Tags, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COALESCE(ah.AnswerCount, 0) AS AnswerCount,
//        COALESCE(c.CommentCount, 0) AS CommentCount FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) ah ON p.Id = ah.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId)
// SELECT tu.DisplayName AS TopUser, tu.Reputation, pd.Title AS PostTitle, pd.Score, pd.ViewCount, pd.Tags, COUNT(DISTINCT v.Id) AS TotalVotes
// FROM TopUsers tu JOIN PostDetails pd ON tu.UserId = pd.OwnerUserId LEFT JOIN Votes v ON pd.PostId = v.PostId
// GROUP BY tu.DisplayName, tu.Reputation, pd.Title, pd.Score, pd.ViewCount, pd.Tags HAVING COUNT(DISTINCT v.Id) > 5 ORDER BY tu.Reputation DESC, pd.Score DESC LIMIT 10;
//
// RankByScore is never filtered, so TopUsers is every user. The GROUP BY names no id, so posts group by (owner name, reputation, title, score, views, tags);
// each vote belongs to one post, so a group's distinct votes are the votes of its posts.
fn q9615(db: &'static So) -> String {
    let Post { owner_user, title, score, view_count, tags_str, .. } = &db.post;
    let g = db
        .post
        .with(owner_user)
        .group_by(owner_user.select(&db.user.display_name).and(owner_user.select(&db.user.reputation)).and(title.opt()).and(score).and(view_count.opt()).and(tags_str.opt()))
        .select(votes_of(db))
        .fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&g).filt(|n| n > 5)), |&(k, _)| (Reverse((k.0).0 .0 .0 .1), Reverse((k.0).0 .1), k), 10);
    rows(v.into_iter().map(|((((((name, rep), t), s), w), tg), n)| row(vec![V::S(name), V::I(rep), ostr(t), V::I(s), oint(w), ostr(tg), V::I(n)])))
}

// WITH UserBadgeSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(*) AS PostCount, SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
//        SUM(CASE WHEN P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 1 ELSE 0 END) AS RecentPosts FROM Posts P GROUP BY P.OwnerUserId),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UBS.GoldBadges, 0) AS GoldBadges, COALESCE(UBS.SilverBadges, 0) AS SilverBadges, COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.NegativePosts, 0) AS NegativePosts, COALESCE(PS.RecentPosts, 0) AS RecentPosts, U.Reputation, U.CreationDate
//     FROM Users U LEFT JOIN UserBadgeSummary UBS ON U.Id = UBS.UserId LEFT JOIN PostSummary PS ON U.Id = PS.OwnerUserId),
// ActiveUsers AS (SELECT UA.UserId, UA.DisplayName, UA.GoldBadges, UA.SilverBadges, UA.BronzeBadges, UA.PostCount, UA.Reputation,
//        DENSE_RANK() OVER (ORDER BY UA.Reputation DESC, UA.PostCount DESC) AS Rank FROM UserActivity UA WHERE UA.Reputation > 1000 AND UA.RecentPosts > 0)
// SELECT A.UserId, A.DisplayName, A.GoldBadges, A.SilverBadges, A.BronzeBadges, A.PostCount, A.Reputation, CASE WHEN A.Rank <= 10 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributorStatus,
//        CASE WHEN A.BronzeBadges > 0 THEN 'Promising Star' ELSE 'Newbie' END AS NewbieStatus FROM ActiveUsers A ORDER BY A.Rank OFFSET 0 ROWS FETCH NEXT 50 ROWS ONLY;
fn q23631(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let Post { owner_user, creation_date, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let ps = db.post.group_by(owner_user).select(creation_date).fold([0i64; 2], move |a, d| [a[0] + 1, a[1] + (d >= since) as i64]);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ub).and((&ps).filt(|a| a[1] > 0))));
    let v = ranked(v, |&(u, (_, p))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(p[0])), true);
    let v = top_n(v, |&((u, _), r)| (r, u), 50);
    rows(v.into_iter().map(|((u, (b, p)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([V::I(p[0]), user_col(db, u, "rep"), V::S(if r <= 10 { "Top Contributor" } else { "Contributor" }), V::S(if b[2] > 0 { "Promising Star" } else { "Newbie" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
//        COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount, AVG(CASE WHEN P.PostTypeId = 1 THEN P.Score END) AS AvgQuestionScore, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
//        COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, TotalViews, QuestionCount, AnswerCount, AvgQuestionScore, GoldBadges, SilverBadges, BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalViews DESC) AS Rank FROM UserActivity),
// RecentPosts AS (SELECT P.Id, P.OwnerUserId, P.Title, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RN FROM Posts P
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// PostLinkStats AS (SELECT PL.PostId, COUNT(PL.RelatedPostId) AS RelatedCount FROM PostLinks PL WHERE PL.CreationDate <= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY PL.PostId)
// SELECT AU.DisplayName, AU.Reputation, AU.TotalViews, AU.QuestionCount, AU.AnswerCount, AU.AvgQuestionScore, AU.GoldBadges, AU.SilverBadges, AU.BronzeBadges, R.Title AS LastPostTitle,
//        R.CreationDate AS LastPostDate, COALESCE(PLS.RelatedCount, 0) AS TotalRelatedPosts
// FROM ActiveUsers AU LEFT JOIN RecentPosts R ON AU.UserId = R.OwnerUserId AND R.RN = 1 LEFT JOIN PostLinkStats PLS ON R.Id = PLS.PostId
// WHERE (AU.Reputation > 100 AND AU.QuestionCount > 0) OR (AU.Reputation < 50 AND AU.AnswerCount > 10) ORDER BY AU.Rank LIMIT 50;
//
// Ties inside the two ROW_NUMBERs go to the smaller user id and the larger post id.
fn q22745(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, owner_user, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 8], |a, (p, b)| {
            let (t, s, w) = p.map_or((0, 0, None), |((t, s), w)| (t, s, w));
            [a[0] + w.unwrap_or(0), a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }, a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64, 0]
        });
    let v = drain(&ua);
    let v = ranked(v, |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), u), false);
    let v = drain(rel(v).filt(|((u, a), _)| {
        let r = db.user.reputation.get(u).unwrap();
        (r > 100 && a[1] > 0) || (r < 50 && a[2] > 10)
    }));
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&(_, r)| r, 50);
    let rp = top_per(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let last: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let pls = db.post_link.with((&db.post_link.creation_date).le(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    type R = ((Id<User>, [i64; 8]), i64);
    let v = drain(rel(v).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| u).select((&last).map(|(_, p)| p).select(Ident::<Post>::new().and((&pls).opt())).opt()))));
    rows(v.into_iter().map(|(_, (((u, a), _), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[1]), V::I(a[4]), V::I(a[5]), V::I(a[6])]);
        match p {
            Some((p, n)) => {
                f.extend(post_fields(db, p, &["title", "created"]));
                f.push(V::I(n.unwrap_or(0)));
            }
            None => f.extend([V::Null, V::Null, V::I(0)]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v
//     WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY v.PostId),
// AggregatedPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes,
//        CASE WHEN rp.Score - COALESCE(rv.DownVotes, 0) < 0 THEN 'Negative Engagement' WHEN rp.Score + COALESCE(rv.UpVotes, 0) <= 10 THEN 'Low Engagement' ELSE 'High Engagement' END AS EngagementLevel
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 AND ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY ph.PostId),
// FinalResults AS (SELECT ap.PostId, ap.Title, ap.Score, ap.ViewCount, ap.UpVotes, ap.DownVotes, ap.EngagementLevel, COALESCE(cp.CloseCount, 0) AS CloseCount FROM AggregatedPosts ap LEFT JOIN ClosedPosts cp ON ap.PostId = cp.PostId)
// SELECT *, CASE WHEN EngagementLevel = 'High Engagement' AND CloseCount = 0 THEN 'Promote' WHEN EngagementLevel = 'Negative Engagement' AND CloseCount > 0 THEN 'Review' ELSE 'Neutral' END AS NextSteps
// FROM FinalResults WHERE UpVotes > DownVotes ORDER BY EngagementLevel DESC, Score DESC;
//
// Rank is never read.
fn q23731(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rv = db.vote.with((&db.vote.creation_date).ge(add_months(t0, -1))).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10).and(hd.ge(add_months(t0, -6)))).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).select((&rv).filt(|a| a[0] > a[1]).and((&cp).opt())));
    rows(v.into_iter().map(|(p, (a, c))| {
        let s = score.get(p).unwrap();
        let c = c.unwrap_or(0);
        let e = if s - a[1] < 0 { "Negative Engagement" } else if s + a[0] <= 10 { "Low Engagement" } else { "High Engagement" };
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(e), V::I(c)]);
        f.push(V::S(if e == "High Engagement" && c == 0 { "Promote" } else if e == "Negative Engagement" && c > 0 { "Review" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT u.Id AS UserId, SUM(v.BountyAmount) AS TotalBounties, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        MAX(p.CreationDate) AS MostRecentPost FROM Posts p GROUP BY p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(up.TotalBounties, 0) AS TotalBounties, (COALESCE(up.Upvotes, 0) - COALESCE(up.Downvotes, 0)) AS ReputationScore, ps.TotalPosts,
//        ps.TotalQuestions, ps.TotalAnswers, ps.MostRecentPost FROM Users u LEFT JOIN UserReputation up ON u.Id = up.UserId LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId),
// ClosedPostHistory AS (SELECT ph.UserId, COUNT(DISTINCT ph.PostId) AS ClosedPostCount, MIN(ph.CreationDate) AS FirstClosedPostDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.UserId),
// UserActivity AS (SELECT us.UserId, us.DisplayName, us.ReputationScore, us.TotalPosts, us.TotalQuestions, us.TotalAnswers, us.MostRecentPost, COALESCE(cph.ClosedPostCount, 0) AS ClosedPosts
//     FROM UserStats us LEFT JOIN ClosedPostHistory cph ON us.UserId = cph.UserId)
// SELECT ua.DisplayName, ua.ReputationScore, ua.TotalPosts, ua.TotalQuestions, ua.TotalAnswers, ua.ClosedPosts,
//        CASE WHEN ua.ClosedPosts > 5 THEN 'Active Contributor' WHEN ua.ReputationScore >= 100 THEN 'Reputable User' ELSE 'New User' END AS UserCategory,
//        RANK() OVER (ORDER BY ua.ReputationScore DESC) AS UserRank FROM UserActivity ua WHERE ua.TotalPosts > 10 ORDER BY ua.ReputationScore DESC, ua.TotalPosts DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q32745(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let ur = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let ps = db.post.group_by(owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let PostHistory { user, post, post_history_type_id, .. } = &db.post_history;
    let cph = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(user).select(post).count_distinct();
    let v = drain((&ur).and((&ps).filt(|a| a[0] > 10)).and((&cph).opt()));
    let v = ranked(v, |&(_, ((s, _), _))| Reverse(s), false);
    rows(v.into_iter().map(|((u, ((s, a), c)), r)| {
        let c = c.unwrap_or(0);
        let mut f = vec![user_col(db, u, "name"), V::I(s), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)];
        f.push(V::S(if c > 5 { "Active Contributor" } else if s >= 100 { "Reputable User" } else { "New User" }));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges, COALESCE(SUM(V.BountyAmount), 0) AS TotalBountyAmount
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON U.Id = V.UserId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName),
// RecentQuestions AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COUNT(C.Id) AS CommentCount, RANK() OVER (ORDER BY P.CreationDate DESC) AS RecentRank FROM Posts P
//     LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// ActiveUsers AS (SELECT U.Id AS UserId, U.DisplayName, ROW_NUMBER() OVER (ORDER BY U.LastAccessDate DESC) AS RecentUserRank, U.LastAccessDate, U.Reputation FROM Users U WHERE U.Reputation > 1000),
// PostClosureDetails AS (SELECT PH.PostId, MAX(CASE WHEN PH.PostHistoryTypeId = 10 THEN PH.CreationDate END) AS ClosedDate,
//        COALESCE(NULLIF(MAX(CASE WHEN PH.PostHistoryTypeId = 10 THEN PH.Comment END), ''), 'N/A') AS CloseReason FROM PostHistory PH GROUP BY PH.PostId)
// SELECT UB.UserId, UB.DisplayName, Q.Title AS RecentQuestionTitle, Q.CreationDate AS QuestionCreationDate, Q.Score AS QuestionScore, Q.CommentCount AS QuestionCommentCount,
//        A.UserId AS ActiveUserId, A.DisplayName AS ActiveUserName, PCD.ClosedDate, PCD.CloseReason, (UB.GoldBadges + UB.SilverBadges + UB.BronzeBadges) AS TotalBadges,
//        CASE WHEN A.RecentUserRank IS NOT NULL THEN 'Active User' ELSE 'Inactive User' END AS UserStatus
// FROM UserBadgeStats UB JOIN RecentQuestions Q ON Q.RecentRank <= 10 FULL OUTER JOIN ActiveUsers A ON UB.UserId = A.UserId LEFT JOIN PostClosureDetails PCD ON Q.PostId = PCD.PostId
// WHERE Q.CommentCount > 5 ORDER BY TotalBadges DESC, Q.Score DESC LIMIT 50 OFFSET 0;
//
// The first ON names only Q, so the users are crossed with the ten newest questions. Every ActiveUsers row is a user, and the WHERE on Q drops any row
// the FULL OUTER JOIN adds from its right side, so it is a LEFT JOIN.
fn q24304(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let bounty = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(bounty.opt())).fold([0i64; 4], |a, (c, b)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + b.flatten().unwrap_or(0)]
    });
    let rq = ranked(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(_, d)| Reverse(d), false);
    let rq: MatSet<Id<Post>> = rel(rq.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&rq).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let pcd = db.post_history.group_by(post).select(post_history_type_id.and(hd).and(comment.opt())).fold((i64::MIN, None::<Str>), |(m, r), ((t, d), c)| {
        if t == 10 { (m.max(d), match (r, c) { (Some(a), Some(b)) => Some(if b > a { b } else { a }), (a, b) => a.or(b) }) } else { (m, r) }
    });
    let q = rel(drain((&cc).filt(|c| c > 5).and((&pcd).opt())));
    let active = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let us = rel(drain(db.user.select((&ub).and(active.opt()))));
    let mut v = Vec::new();
    (&us).cross(&q).drive(|_, ((u, (b, a)), (p, (c, d)))| v.push((u, b, a, p, c, d)));
    let v = top_n(v, |&(u, b, _, p, _, _)| (Reverse(b[0] + b[1] + b[2]), Reverse(score.get(p).unwrap()), u, p), 50);
    rows(v.into_iter().map(|(u, b, a, p, c, d)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.push(V::I(c));
        f.extend(match a {
            Some(a) => ucols(db, a, &["uid", "name"]),
            None => vec![V::Null, V::Null],
        });
        let (m, r) = d.unwrap_or((i64::MIN, None));
        f.extend([tmax(m), V::S(r.filter(|r| !r.is_empty()).unwrap_or("N/A")), V::I(b[0] + b[1] + b[2]), V::S(if a.is_some() { "Active User" } else { "Inactive User" })]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS Downvotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts, SUM(P.Score) AS TotalScore FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, Upvotes, Downvotes, TotalPosts, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserVoteStats WHERE TotalPosts > 0),
// PostScoreHistory AS (SELECT P.Id AS PostId, P.Title, P.LastActivityDate, P.Score, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpvoteCount,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownvoteCount FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostRecentActivity AS (SELECT PH.PostId, PH.UserId, PH.CreationDate, PH.Comment, P.Title FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id
//     WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' AND PH.PostHistoryTypeId IN (10, 11, 12))
// SELECT TU.DisplayName, TU.Upvotes, TU.Downvotes, TU.TotalPosts, TU.TotalScore, PS.PostId, PS.Title, PS.LastActivityDate, PS.Score, PS.UpvoteCount, PS.DownvoteCount, COUNT(PRA.UserId) AS RecentActivityCount
// FROM TopUsers TU JOIN PostScoreHistory PS ON TU.UserId = PS.PostId LEFT JOIN PostRecentActivity PRA ON PS.PostId = PRA.PostId
// GROUP BY TU.DisplayName, TU.Upvotes, TU.Downvotes, TU.TotalPosts, TU.TotalScore, PS.PostId, PS.Title, PS.LastActivityDate, PS.Score, PS.UpvoteCount, PS.DownvoteCount
// ORDER BY TU.TotalScore DESC, PS.Score DESC LIMIT 100;
//
// `TU.UserId = PS.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q9667(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let Vote { vote_type_id, post, .. } = &db.vote;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id.and(post.select(score).opt())).opt()).fold([0i64; 4], |a, x| match x {
        Some((t, s)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + s.is_some() as i64, a[3] + s.unwrap_or(0)],
        None => a,
    });
    let tpc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(post)).count_distinct();
    let pidx: HashIdx<i64, Id<Post>> = db.post.with(creation_date.ge(add_years(t0, -1))).select(&db.post.origid).inv().collect();
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let pra = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_months(t0, -1)).and(post_history_type_id.is_in([10, 11, 12]))));
    let ra = db.post.group_by(Ident::<Post>::new()).select(pra.select(user.opt()).opt()).fold(0i64, |n, u| n + u.flatten().is_some() as i64);
    let v = drain((&uvs).and((&tpc).filt(|n| n > 0)).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pv).and(&ra))));
    let v = top_n(v, |&(u, ((a, _), ((p, _), _)))| (a[2] == 0, Reverse(a[3]), Reverse(score.get(p).unwrap()), u, p), 100);
    rows(v.into_iter().map(|(u, ((a, n), ((p, b), c)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n), nullable(a[3], a[2])];
        f.extend(post_fields(db, p, &["id", "title", "activity", "score"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.Id) AS UpvoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.Id) AS DownvoteCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostDetails AS (SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, RP.Score, RP.CreationDate, RP.PostRank, RP.UpvoteCount, RP.DownvoteCount, (RP.UpvoteCount - RP.DownvoteCount) AS NetScore,
//        CASE WHEN RP.Score > 10 THEN 'High Engagement' WHEN RP.Score BETWEEN 1 AND 10 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel FROM RankedPosts RP WHERE RP.PostRank <= 5),
// ClosedPosts AS (SELECT PH.PostId, COUNT(*) AS CloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId),
// FinalPosts AS (SELECT PD.PostId, PD.Title, PD.OwnerDisplayName, PD.Score, PD.CreationDate, PD.PostRank, PD.NetScore, PD.EngagementLevel, COALESCE(CP.CloseCount, 0) AS CloseCount
//     FROM PostDetails PD LEFT JOIN ClosedPosts CP ON PD.PostId = CP.PostId)
// SELECT F.*, CASE WHEN F.CloseCount > 5 THEN 'Frequent Closure' WHEN F.CloseCount BETWEEN 1 AND 5 THEN 'Occasional Closure' ELSE 'No Closure' END AS ClosureBehavior,
//        (SELECT COUNT(*) FROM Badges B WHERE B.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = F.PostId)) AS BadgeCount
// FROM FinalPosts F WHERE F.NetScore > 0 ORDER BY F.Score DESC, F.CreationDate ASC LIMIT 10;
//
// RankedPosts has no GROUP BY, so PostRank ranks the joined (post, vote) rows; they are materialised and ranked.
fn q21884(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = recent().select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let ud = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let rk = ranked(drain(&j).into_iter().map(|x| x.0).collect(), |&(p, _)| (post_type_id.get(p).unwrap(), Reverse(score.get(p).unwrap())), false);
    let rk = per_group(rk, |&(p, _)| post_type_id.get(p).unwrap());
    let rk = rel(rk.into_iter().filter(|x| x.1 <= 5).collect());
    let cp = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type R = ((Id<Post>, Option<Id<Vote>>), i64);
    let v = drain((&rk).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select((&ud).filt(|a| a[0] - a[1] > 0).and((&cp).opt()).and(owner_user.select(&bc).opt())))));
    let v = top_n(v, |&(_, (((p, x), _), _))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p, x), 10);
    rows(v.into_iter().map(|(_, (((p, _), r), ((a, c), b)))| {
        let s = score.get(p).unwrap();
        let c = c.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "created"]);
        f.extend([V::I(r), V::I(a[0] - a[1])]);
        f.push(V::S(if s > 10 { "High Engagement" } else if (1..=10).contains(&s) { "Moderate Engagement" } else { "Low Engagement" }));
        f.push(V::I(c));
        f.push(V::S(if c > 5 { "Frequent Closure" } else if (1..=5).contains(&c) { "Occasional Closure" } else { "No Closure" }));
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, RANK() OVER (ORDER BY U.Reputation DESC) as ReputationRank FROM Users U WHERE U.Reputation > 0),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId, COALESCE(COUNT(C.Id), 0) AS CommentCount, COALESCE(SUM(VB.BountyAmount), 0) AS TotalBounty
//     FROM Posts P LEFT JOIN Comments C ON C.PostId = P.Id LEFT JOIN Votes VB ON P.Id = VB.PostId AND VB.VoteTypeId IN (8, 9) GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId),
// PostHistoryAggregate AS (SELECT PH.PostId, PH.PostHistoryTypeId, COUNT(*) AS ChangeCount, MAX(PH.CreationDate) AS LastChangeDate FROM PostHistory PH
//     WHERE PH.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR' GROUP BY PH.PostId, PH.PostHistoryTypeId),
// ClosedPosts AS (SELECT P.Id AS ClosedPostId, PH.Comment AS CloseReason FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId WHERE PH.PostHistoryTypeId IN (10, 11))
// SELECT UR.DisplayName, DENSE_RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS ScoreRank, P.Title, P.CreationDate, P.Score, P.ViewCount, PH.LastChangeDate,
//        COALESCE(CP.CloseReason, 'Not Closed') AS CloseReason,
//        CASE WHEN UR.Reputation < 100 THEN 'Low Reputation' WHEN UR.Reputation BETWEEN 100 AND 1000 THEN 'Medium Reputation' ELSE 'High Reputation' END AS ReputationTier
// FROM PostDetails P JOIN UserReputation UR ON P.OwnerUserId = UR.UserId LEFT JOIN PostHistoryAggregate PH ON P.PostId = PH.PostId LEFT JOIN ClosedPosts CP ON P.PostId = CP.ClosedPostId
// WHERE P.Score > 0 AND P.ViewCount > 100 AND UR.ReputationRank <= 100 ORDER BY UR.Reputation DESC, P.Score DESC;
//
// CommentCount and TotalBounty are never read. DENSE_RANK over the joined rows is the dense rank of each post's score among the owner's posts that pass the WHERE.
fn q2333(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let rr = ranked(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(rr.into_iter().take_while(|x| x.1 <= 100).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ps = drain((&tu).select(posts_of(db).select(Ident::<Post>::new().with(score.gt(0)).with(view_count.gt(100)))));
    let ps = ranked(ps, |&(u, p)| (u, Reverse(score.get(p).unwrap())), true);
    let ps = rel(per_group(ps, |&(u, _)| u).into_iter().map(|((_, p), r)| (p, r)).collect());
    let PostHistory { post, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let pha = db.post_history.with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post.and(post_history_type_id)).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let pv = rel(drain(&pha));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&pv).map(|((p, _), _)| p).inv().select(&pv).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))).select(comment.opt());
    type R = (Id<Post>, i64);
    let v = drain((&ps).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&by_post).opt().and(closes.opt())))));
    rows(v.into_iter().map(|(_, ((p, r), (h, c)))| {
        let u = owner_user.get(p).unwrap();
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(r)];
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.push(h.map_or(V::Null, |(_, d)| V::T(d)));
        f.push(V::S(c.flatten().unwrap_or("Not Closed")));
        f.push(V::S(if rep < 100 { "Low Reputation" } else if (100..=1000).contains(&rep) { "Medium Reputation" } else { "High Reputation" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankByViews,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted Answer' ELSE 'Not Accepted Answer' END AS AcceptanceStatus
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.Score, p.AcceptedAnswerId, p.PostTypeId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostLinkStatistics AS (SELECT pl.PostId, COUNT(pl.Id) AS RelatedPostCount, MAX(CASE WHEN lt.Name = 'Duplicate' THEN 1 ELSE 0 END) AS ContainsDuplicateLinks FROM PostLinks pl
//     JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id GROUP BY pl.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, pls.RelatedPostCount,
//        CASE WHEN pls.ContainsDuplicateLinks = 1 THEN 'Yes' ELSE 'No' END AS HasDuplicates, rp.AcceptanceStatus
// FROM RankedPosts rp JOIN UserBadges ub ON rp.PostId = ub.UserId LEFT JOIN PostLinkStatistics pls ON rp.PostId = pls.PostId
// WHERE (rp.ViewCount > 100 OR rp.CommentCount > 5) AND (rp.AcceptanceStatus = 'Accepted Answer' OR rp.Score < 0) ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 100;
//
// RankByViews is never read. `rp.PostId = ub.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q23530(db: &'static So) -> String {
    let Post { creation_date, accepted_answer_id, score, view_count, .. } = &db.post;
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with((&db.post.origid).select(&uidx))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let pls = db.post_link.group_by(&db.post_link.post).select((&db.post_link.link_type).select(&db.link_type.name)).fold((0i64, 0i64), |(n, d), t| (n + 1, d.max((t == "Duplicate") as i64)));
    let v = drain(db.post.select(
        Ident::<Post>::new()
            .and(&rp)
            .filt(|(p, a): (Id<Post>, [i64; 3])| (view_count.get(p).map_or(false, |w| w > 100) || a[0] > 5) && (accepted_answer_id.get(p).is_some() || score.get(p).unwrap() < 0))
            .and((&db.post.origid).select(&uidx).select(&ub))
            .and((&pls).opt()),
    ));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 100);
    rows(v.into_iter().map(|(p, (((_, a), b), l))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.push(l.map_or(V::Null, |l| V::I(l.0)));
        f.push(V::S(if l.map_or(false, |l| l.1 == 1) { "Yes" } else { "No" }));
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Accepted Answer" } else { "Not Accepted Answer" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostID, P.Title, P.CreationDate, P.OwnerUserId, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.Id) AS UpvoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.Id) AS DownvoteCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostStatistics AS (SELECT RP.PostID, RP.Title, RP.CreationDate, COALESCE(U.DisplayName, 'Unknown') AS OwnerDisplayName, RP.Score, RP.UpvoteCount, RP.DownvoteCount,
//        CASE WHEN RP.Score < 0 THEN 'Negative' WHEN RP.Score > 0 THEN 'Positive' ELSE 'Neutral' END AS ScoreEvaluation,
//        CASE WHEN RP.RecentPostRank = 1 THEN 'Most Recent' ELSE 'Older Post' END AS PostAgeCategory FROM RankedPosts RP LEFT JOIN Users U ON RP.OwnerUserId = U.Id),
// FinalResults AS (SELECT PS.OwnerDisplayName, PS.Title, PS.CreationDate, PS.ScoreEvaluation, PS.UpvoteCount, PS.DownvoteCount, PS.PostAgeCategory,
//        CASE WHEN PS.PostAgeCategory = 'Most Recent' AND PS.ScoreEvaluation = 'Positive' THEN 'Promote' WHEN PS.PostAgeCategory = 'Most Recent' AND PS.ScoreEvaluation = 'Neutral' THEN 'Monitor'
//             WHEN PS.PostAgeCategory = 'Most Recent' AND PS.ScoreEvaluation = 'Negative' THEN 'Review' ELSE 'Archive' END AS ActionRecommendation FROM PostStatistics PS)
// SELECT OwnerDisplayName, Title, CreationDate, UpvoteCount, DownvoteCount, ScoreEvaluation, PostAgeCategory, ActionRecommendation FROM FinalResults
// WHERE (UpvoteCount > DownvoteCount) OR (ScoreEvaluation = 'Negative' AND PostAgeCategory = 'Most Recent') ORDER BY CreationDate DESC LIMIT 100;
//
// RankedPosts has no GROUP BY, so RecentPostRank numbers the joined (post, vote) rows; they are materialised and numbered, the ownerless posts as one partition.
// Which of a post's rows gets number 1 is open; the port gives it to the smallest vote id.
fn q21784(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = recent().select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let ud = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let rk = ranked(drain(&j).into_iter().map(|x| x.0).collect(), |&(p, x)| (owner_user.get(p), Reverse(creation_date.get(p).unwrap()), p, x), false);
    let rk = rel(per_group(rk, |&(p, _)| owner_user.get(p)));
    type R = ((Id<Post>, Option<Id<Vote>>), i64);
    let v = drain((&rk).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select(&ud))).filt(|(((p, _), r), a): (R, [i64; 2])| a[0] > a[1] || (score.get(p).unwrap() < 0 && r == 1)));
    let v = top_n(v, |&(_, (((p, x), _), _))| (Reverse(creation_date.get(p).unwrap()), p, x), 100);
    rows(v.into_iter().map(|(_, (((p, _), r), a))| {
        let s = score.get(p).unwrap();
        let e = if s < 0 { "Negative" } else if s > 0 { "Positive" } else { "Neutral" };
        let mut f = vec![V::S(owner_user.get(p).map_or("Unknown", |u| db.user.display_name.get(u).unwrap()))];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::S(e), V::S(if r == 1 { "Most Recent" } else { "Older Post" })]);
        f.push(V::S(match (r == 1, e) {
            (true, "Positive") => "Promote",
            (true, "Neutral") => "Monitor",
            (true, _) => "Review",
            _ => "Archive",
        }));
        row(f)
    }))
}

// WITH RECURSIVE UserVoteCounts AS (SELECT UserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes GROUP BY UserId),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, uc.UpVotes, uc.DownVotes, uc.TotalVotes FROM Users u LEFT JOIN UserVoteCounts uc ON u.Id = uc.UserId
//     WHERE u.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostScoreStatistics AS (SELECT p.Id AS PostId, p.Score, p.OwnerUserId, COUNT(c.Id) AS CommentCount, AVG(v.BountyAmount) AS AvgBounty FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Score, p.OwnerUserId),
// UserPostEngagement AS (SELECT au.Id AS UserId, au.DisplayName, COUNT(DISTINCT p.Id) AS PostsEngaged, SUM(ps.Score) AS TotalScore, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM ActiveUsers au LEFT JOIN Posts p ON p.OwnerUserId = au.Id LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN PostScoreStatistics ps ON ps.PostId = p.Id GROUP BY au.Id, au.DisplayName),
// TopEngagedUsers AS (SELECT UserId, DisplayName, PostsEngaged, TotalScore, TotalComments, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostEngagement)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, te.PostsEngaged, te.TotalScore, te.TotalComments, CASE WHEN te.ScoreRank <= 10 THEN 'Top Engaged' ELSE 'Regular Engaged' END AS EngagementLevel
// FROM ActiveUsers u JOIN TopEngagedUsers te ON u.Id = te.UserId ORDER BY te.TotalScore DESC, te.PostsEngaged DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. Of PostScoreStatistics only its Score is read, and only its posts (the last year's) have one.
fn q34091(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let au = || db.user.with((&db.user.last_access_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let ps = Ident::<Post>::new().with(creation_date.ge(since)).select(score);
    let upe = au().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).opt().and(ps.opt())).opt()).fold((0i64, 0i64), |(n, s), x| match x.and_then(|(_, s)| s) {
        Some(v) => (n + 1, s + v),
        None => (n, s),
    });
    let pe = au().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tc = au().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&upe).and(&pe).and(&tc));
    let v = ranked(v, |&(_, (((n, s), _), _))| (n == 0, Reverse(s)), false);
    rows(v.into_iter().map(|((u, (((n, s), p), c)), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(p), nullable(s, n), V::I(c), V::S(if r <= 10 { "Top Engaged" } else { "Regular Engaged" })]);
        row(f)
    }))
}

// WITH UserBadgeCount AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostDetail AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.ViewCount > 0 AND P.Score IS NOT NULL),
// RecentPostDetails AS (SELECT PD.PostId, PD.Title, PD.CreationDate, PD.Score, PD.ViewCount, U.DisplayName, U.Reputation, UBC.BadgeCount, UBC.GoldBadges, UBC.SilverBadges, UBC.BronzeBadges
//     FROM PostDetail PD JOIN Users U ON PD.OwnerUserId = U.Id JOIN UserBadgeCount UBC ON U.Id = UBC.UserId WHERE PD.PostRank = 1),
// ClosedPostHistory AS (SELECT PH.PostId, COUNT(*) AS CloseEventCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId),
// RankedPosts AS (SELECT RPD.*, COALESCE(CPH.CloseEventCount, 0) AS CloseCount,
//        CASE WHEN RPD.Score IS NULL THEN 'No Score' WHEN RPD.Score = 0 THEN 'Neutral' WHEN RPD.Score > 0 THEN 'Positive' ELSE 'Negative' END AS ScoreStatus
//     FROM RecentPostDetails RPD LEFT JOIN ClosedPostHistory CPH ON RPD.PostId = CPH.PostId)
// SELECT RP.PostId, RP.Title, RP.ViewCount, RP.CreationDate, RP.DisplayName AS Owner, RP.Reputation, RP.BadgeCount, RP.GoldBadges, RP.SilverBadges, RP.BronzeBadges, RP.CloseCount, RP.ScoreStatus
// FROM RankedPosts RP WHERE RP.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY RP.CreationDate DESC LIMIT 10 OFFSET 5;
//
// A tie on CreationDate inside PostRank goes to the larger post id.
fn q23545(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let Post { view_count, owner_user, creation_date, score, .. } = &db.post;
    let (sum, n) = db.user.select(reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let first = top_per(drain(db.post.with(view_count.gt(0)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cph = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |k, _| k + 1);
    let above = Ident::<User>::new().with(reputation.filt(move |r| r * n > sum));
    let v = drain((&first).select(owner_user.select(above.and(&ubc)).and((&cph).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 15);
    rows(v.into_iter().skip(5).map(|(p, ((u, b), c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "views", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(b.map(V::I));
        f.extend([V::I(c.unwrap_or(0)), V::S(if s == 0 { "Neutral" } else if s > 0 { "Positive" } else { "Negative" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.ViewCount),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COUNT(DISTINCT p.Id) AS QuestionsAsked, SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// PostDetail AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.UpVotes, rp.DownVotes, rp.CommentCount, us.DisplayName AS AuthorName, us.Reputation AS AuthorReputation, us.QuestionsAsked,
//        us.TotalViews, us.GoldBadges, us.SilverBadges, us.BronzeBadges FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.UpVotes, pd.DownVotes, pd.CommentCount, pd.AuthorName, pd.AuthorReputation, pd.QuestionsAsked, pd.TotalViews, pd.GoldBadges,
//        pd.SilverBadges, pd.BronzeBadges FROM PostDetail pd WHERE pd.CommentCount >= 5 ORDER BY pd.UpVotes DESC, pd.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
//
// PostRank is never read.
fn q27851(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt()).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 5], |a, (w, b)| {
        let w = w.flatten();
        [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + (b == Some(1)) as i64, a[3] + (b == Some(2)) as i64, a[4] + (b == Some(3)) as i64]
    });
    let qa = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&rp).filt(|a| a[2] >= 5).and(owner_user.select(Ident::<User>::new().and(&us).and(&qa))));
    let v = top_n(v, |&(p, (a, _))| {
        let w = view_count.get(p);
        (Reverse(a[0]), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (a, ((u, s), q)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(q), nullable(s[1], s[0]), V::I(s[2]), V::I(s[3]), V::I(s[4])]);
        row(f)
    }))
}

// WITH RECURSIVE UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COALESCE(SUM(P.ViewCount), 0) AS TotalViewCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts, RANK() OVER (ORDER BY COALESCE(SUM(P.ViewCount), 0) DESC) AS EngagementRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// RecentActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT PH.Id) AS PostHistoryCount, MAX(PH.CreationDate) AS LastActivityDate
//     FROM Users U LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN PostHistory PH ON U.Id = PH.UserId GROUP BY U.Id, U.DisplayName),
// TopContributors AS (SELECT UE.UserId, UE.DisplayName, UE.Reputation, UE.TotalViewCount, UE.TotalUpvotes, UE.TotalDownvotes, UA.CommentCount, UA.LastActivityDate
//     FROM UserEngagement UE JOIN RecentActivity UA ON UE.UserId = UA.UserId WHERE UE.TotalPosts > 0),
// EngagementSummary AS (SELECT UserId, DisplayName, Reputation, TotalViewCount, TotalUpvotes, TotalDownvotes, CommentCount, LastActivityDate,
//        CASE WHEN Reputation > 1000 THEN 'High' WHEN Reputation BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'Low' END AS ReputationLevel FROM TopContributors)
// SELECT UserId, DisplayName, Reputation, TotalViewCount, TotalUpvotes, TotalDownvotes, CommentCount, LastActivityDate, ReputationLevel,
//        LEAD(LastActivityDate) OVER (ORDER BY LastActivityDate DESC) AS NextActivityDate FROM EngagementSummary ORDER BY TotalViewCount DESC, Reputation DESC FETCH FIRST 10 ROWS ONLY;
//
// WITH RECURSIVE, but no CTE refers to itself. RecentActivity reads only COUNT(DISTINCT ..) and MAX of its comments x history product, which one fold per child gives.
// LEAD runs over every contributor in LastActivityDate order (NULLs last), ties broken by user id.
fn q32257(db: &'static So) -> String {
    let Post { view_count, .. } = &db.post;
    let ue = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((w, t)) => [a[0] + w.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
        None => a,
    });
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let cc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hist_by: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let la = db.user.group_by(Ident::<User>::new()).select((&hist_by).select(&db.post_history.creation_date)).fold(i64::MIN, |m, d| m.max(d));
    let v = drain((&ue).and(&tp).and(&cc).and((&la).opt()));
    let v = top_n(v, |&(u, (_, d))| (d.is_none(), Reverse(d), u), 0);
    let nx: Vec<Option<i64>> = (0..v.len()).map(|i| v.get(i + 1).and_then(|x| (x.1).1)).collect();
    let v: Vec<_> = v.into_iter().zip(nx).collect();
    let v = top_n(v, |&((u, (((a, _), _), _)), _)| (Reverse(a[0]), Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|((u, (((a, _), c), d)), nx)| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c), ots(d)]);
        f.push(V::S(if rep > 1000 { "High" } else if (500..=1000).contains(&rep) { "Medium" } else { "Low" }));
        f.push(ots(nx));
        row(f)
    }))
}

// WITH RecursivePostData AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.AcceptedAnswerId, COALESCE(a.Score, 0) AS AcceptedAnswerScore,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// VoteStats AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT pp.PostId, pp.Title, pp.CreationDate, u.DisplayName AS Owner, u.Reputation, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ps.Upvotes, ps.Downvotes, ps.TotalVotes,
//        COALESCE(pp.AcceptedAnswerScore, 0) AS AcceptedAnswerScore, pp.CommentCount,
//        CASE WHEN pp.UserPostRank = 1 AND ub.GoldBadges > 0 THEN 'Top Contributor with Gold Badge' WHEN pp.UserPostRank <= 5 THEN 'Active Contributor' ELSE 'Regular User' END AS UserStatus,
//        CASE WHEN pp.CommentCount > 50 THEN 'High Engagement' WHEN pp.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 'Old Post' ELSE 'Recent Activity' END AS PostEngagement
// FROM RecursivePostData pp JOIN Users u ON pp.OwnerUserId = u.Id JOIN UserBadges ub ON u.Id = ub.UserId JOIN VoteStats ps ON pp.PostId = ps.PostId
// WHERE pp.Title ILIKE '%SQL%' ORDER BY pp.CreationDate DESC LIMIT 100;
//
// A tie on CreationDate inside UserPostRank goes to the larger post id.
fn q23642(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, title, accepted_answer, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), Reverse(p)), false);
    let rk = rel(per_group(v, |&(_, u)| u).into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let sql = db.post.with(post_type_id.eq(1)).with(title.filt(|t: Str| t.to_lowercase().contains("sql")));
    let vs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]);
    let cc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain(sql.select(owner_user.select(Ident::<User>::new().and(&ub)).and(&vs).and(&cc).and((&rank).map(|(_, r)| r)).and(accepted_answer.select(score).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (((((u, b), s), c), r), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(b.map(V::I));
        f.extend(s.map(V::I));
        f.extend([V::I(a.unwrap_or(0)), V::I(c)]);
        f.push(V::S(if r == 1 && b[0] > 0 { "Top Contributor with Gold Badge" } else if r <= 5 { "Active Contributor" } else { "Regular User" }));
        f.push(V::S(if c > 50 { "High Engagement" } else if creation_date.get(p).unwrap() < add_years(ts(2024, 10, 1, 12, 34, 56), -1) { "Old Post" } else { "Recent Activity" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// HighScorePosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE RankByScore <= 5),
// PostCommentStats AS (SELECT PostId, COUNT(*) AS TotalComments FROM Comments GROUP BY PostId),
// PostVoteStats AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// JoinedPostStats AS (SELECT hsp.PostId, hsp.Title, hsp.CreationDate, hsp.Score, hsp.ViewCount, hsp.OwnerDisplayName, COALESCE(pcs.TotalComments, 0) AS TotalComments,
//        COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes FROM HighScorePosts hsp LEFT JOIN PostCommentStats pcs ON hsp.PostId = pcs.PostId LEFT JOIN PostVoteStats pvs ON hsp.PostId = pvs.PostId)
// SELECT jps.PostId, jps.Title, jps.CreationDate, jps.Score, jps.ViewCount, jps.OwnerDisplayName, jps.TotalComments, jps.UpVotes, jps.DownVotes,
//        CASE WHEN jps.UpVotes + jps.DownVotes = 0 THEN NULL ELSE ROUND((jps.UpVotes::DECIMAL / (jps.UpVotes + jps.DownVotes)) * 100, 2) END AS VotePercentage,
//        CASE WHEN jps.Score >= 100 THEN 'Hot' WHEN jps.Score BETWEEN 50 AND 99 THEN 'Trending' ELSE 'Needs Attention' END AS PostHeatLevel
// FROM JoinedPostStats jps WHERE jps.TotalComments > 0 ORDER BY jps.Score DESC, jps.ViewCount DESC;
//
// The ownerless posts rank as one partition of their own.
fn q22456(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&pv)).into_iter().map(|(p, (c, a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::S(owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.push(if a[0] + a[1] == 0 { V::Null } else { V::F((a[0] as f64 / (a[0] + a[1]) as f64 * 100.0 * 100.0).round() / 100.0) });
        f.push(V::S(if s >= 100 { "Hot" } else if (50..=99).contains(&s) { "Trending" } else { "Needs Attention" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        u.DisplayName AS OwnerDisplayName FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, c.Name AS CloseReason FROM PostHistory ph JOIN CloseReasonTypes c ON ph.Comment = CAST(c.Id AS VARCHAR) WHERE ph.PostHistoryTypeId = 10),
// PostVotes AS (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// PostComments AS (SELECT c.PostId, COUNT(*) AS CommentCount FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, COALESCE(pp.UpVotes, 0) AS UpVotes, COALESCE(pp.DownVotes, 0) AS DownVotes, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        cp.CloseDate, cp.CloseReason, rp.OwnerDisplayName, CASE WHEN rp.ViewCount > 100 THEN 'Popular' WHEN rp.Rank = 1 THEN 'Most Recent' ELSE 'Regular' END AS PostCategory
//     FROM RankedPosts rp LEFT JOIN PostVotes pp ON rp.PostId = pp.PostId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT PostId, Title, CreationDate, Score, UpVotes, DownVotes, CommentCount, CloseDate, CloseReason, OwnerDisplayName, PostCategory, CASE WHEN CloseDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus
// FROM FinalResults WHERE (UpVotes - DownVotes) > 5 ORDER BY Score DESC, CreationDate DESC;
//
// A tie on CreationDate inside Rank goes to the larger post id.
fn q23180(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let reason: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i: i64| -> Str { Box::leak(i.to_string().into_boxed_str()) }).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(Ident::<PostHistory>::new().and(comment.select(&reason)));
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select((&pv).filt(|a| a[0] - a[1] > 5).and((&pc).opt()).and(closes.opt()).and(Ident::<Post>::new().with(&first).opt())));
    rows(v.into_iter().map(|(p, (((a, c), h), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0))]);
        f.extend(match h {
            Some((h, n)) => [V::T(hd.get(h).unwrap()), V::S(n)],
            None => [V::Null, V::Null],
        });
        f.extend(post_fields(db, p, &["owner"]));
        f.push(V::S(if view_count.get(p).map_or(false, |w| w > 100) { "Popular" } else if r.is_some() { "Most Recent" } else { "Regular" }));
        f.push(V::S(if h.is_some() { "Closed" } else { "Active" }));
        row(f)
    }))
}

// WITH UserVoteDetails AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN vt.Name = 'BountyStart' THEN v.BountyAmount ELSE 0 END) AS TotalBounty,
//        DENSE_RANK() OVER (ORDER BY COUNT(v.Id) DESC) AS VoteRank FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalVotes, UpVotes, DownVotes, TotalBounty FROM UserVoteDetails WHERE VoteRank <= 10),
// PostWithComments AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, SUM(COALESCE(c.Score, 0)) AS TotalCommentScore FROM Posts p
//     LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate),
// PostHistoryDetails AS (SELECT ph.PostId, MIN(ph.CreationDate) AS FirstChangeDate, COUNT(CASE WHEN pht.Name IN ('Initial Title', 'Edit Title') THEN 1 END) AS TitleEdits,
//        COUNT(CASE WHEN pht.Name IN ('Initial Body', 'Edit Body') THEN 1 END) AS BodyEdits, COUNT(CASE WHEN pht.Name IN ('Initial Tags', 'Edit Tags') THEN 1 END) AS TagEdits
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT u.DisplayName AS UserDisplayName, u.TotalVotes, u.UpVotes, u.DownVotes, u.TotalBounty, pwd.PostId, pwd.Title AS PostTitle, pwd.CreationDate AS PostCreationDate, pwd.CommentCount,
//        pwd.TotalCommentScore, phd.FirstChangeDate, phd.TitleEdits, phd.BodyEdits, phd.TagEdits
// FROM TopUsers u JOIN PostWithComments pwd ON u.UserId = pwd.PostId LEFT JOIN PostHistoryDetails phd ON pwd.PostId = phd.PostId
// WHERE (pwd.CommentCount > 0 OR phd.TitleEdits > 0) ORDER BY u.TotalVotes DESC, pwd.TotalCommentScore DESC LIMIT 50;
//
// `u.UserId = pwd.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q23471(db: &'static So) -> String {
    let uvd = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vtype_name(db).and((&db.vote.bounty_amount).opt())).opt()).fold([0i64; 5], |a, x| match x {
        Some((n, b)) => {
            let t = if n == "BountyStart" { b } else { Some(0) };
            [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64, a[3] + t.unwrap_or(0), a[4] + t.is_some() as i64]
        }
        None => [a[0], a[1], a[2], a[3], a[4] + 1],
    });
    let vr = ranked(drain(&uvd), |&(_, a)| Reverse(a[0]), true);
    let tu = rel(vr.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let pwc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0)]);
    let phd = db.post_history.group_by(&db.post_history.post).select(htype_name(db).and(&db.post_history.creation_date)).fold([i64::MAX, 0, 0, 0], |a, (n, d)| {
        [a[0].min(d), a[1] + matches!(n, "Initial Title" | "Edit Title") as i64, a[2] + matches!(n, "Initial Body" | "Edit Body") as i64, a[3] + matches!(n, "Initial Tags" | "Edit Tags") as i64]
    });
    type R = (Id<User>, [i64; 5]);
    let j = Same::<R>::new().map(|(u, _): R| u).select(&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pwc).and((&phd).opt()));
    let v = drain((&tu).select(Same::<R>::new().and(j)).filt(|(_, ((_, c), h)): (R, ((Id<Post>, [i64; 2]), Option<[i64; 4]>))| c[0] > 0 || h.map_or(false, |h| h[1] > 0)));
    let v = top_n(v, |&(_, ((u, a), ((p, c), _)))| (Reverse(a[0]), Reverse(c[1]), u, p), 50);
    rows(v.into_iter().map(|(_, ((u, a), ((p, c), h)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[4])];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(c[0]), V::I(c[1])]);
        f.extend(match h {
            Some(h) => [V::T(h[0]), V::I(h[1]), V::I(h[2]), V::I(h[3])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY Reputation DESC, BadgeCount DESC) AS Rank FROM UserBadgeStats),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount FROM Posts p GROUP BY p.OwnerUserId),
// FinalReport AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, UPPER(u.Location) AS Location, COALESCE(ps.PostCount, 0) AS TotalPosts, COALESCE(ps.QuestionCount, 0) AS TotalQuestions,
//        COALESCE(ps.AnswerCount, 0) AS TotalAnswers, COALESCE(ps.PositiveScoreCount, 0) AS TotalPositiveScores, tb.BadgeCount, tb.GoldBadges, tb.SilverBadges, tb.BronzeBadges, tb.Rank
//     FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN TopUsers tb ON u.Id = tb.UserId)
// SELECT fr.UserId, fr.DisplayName, fr.Reputation, fr.Location, fr.TotalPosts, fr.TotalQuestions, fr.TotalAnswers, fr.TotalPositiveScores, fr.BadgeCount, fr.GoldBadges, fr.SilverBadges, fr.BronzeBadges, fr.Rank
// FROM FinalReport fr WHERE (fr.Reputation IS NOT NULL OR fr.BadgeCount > 0) AND (fr.TotalPosts > 0 OR fr.TotalQuestions > 0 OR fr.TotalAnswers > 0)
// ORDER BY fr.Rank ASC, fr.Reputation DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
fn q24240(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64]);
    let v = ranked(drain(&ub), |&(u, b)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b[0])), false);
    let v = rel(v.into_iter().map(|((u, b), r)| (u, b, r)).collect());
    type R = (Id<User>, [i64; 4], i64);
    let v = drain((&v).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _, _): R| u).select(&ps))));
    let v = top_n(v, |&(_, ((u, _, r), _))| (r, Reverse(db.user.reputation.get(u).unwrap()), u), 20);
    rows(v.into_iter().skip(10).map(|(_, ((u, b, r), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(db.user.location.get(u).map_or(V::Null, |l| V::Owned(l.to_uppercase())));
        f.extend(p.map(V::I));
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.Score > 0 THEN p.Id END) AS PositivePosts, COUNT(DISTINCT CASE WHEN p.Score <= 0 THEN p.Id END) AS NegativePosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// PostClosureCount AS (SELECT ph.UserId, COUNT(*) AS TotalClosures FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.UserId),
// PostMetrics AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.Score, p.CreationDate),
// RankedPosts AS (SELECT pm.Id, pm.Title, pm.Score, pm.CommentCount, pm.UpVoteCount, pm.DownVoteCount, RANK() OVER (ORDER BY pm.Score DESC, pm.CommentCount DESC) AS PostRank FROM PostMetrics pm)
// SELECT u.DisplayName, u.Reputation, COALESCE(uv.UpVotes, 0) AS TotalUpVotes, COALESCE(uv.DownVotes, 0) AS TotalDownVotes, COALESCE(pc.TotalClosures, 0) AS TotalClosures, rp.Title, rp.CommentCount,
//        rp.UpVoteCount, rp.DownVoteCount, rp.PostRank
// FROM Users u LEFT JOIN UserVoteStats uv ON u.Id = uv.UserId LEFT JOIN PostClosureCount pc ON u.Id = pc.UserId JOIN RankedPosts rp ON u.Id = rp.Id
// WHERE (COALESCE(uv.UpVotes, 0) + COALESCE(uv.DownVotes, 0)) > (SELECT AVG(COALESCE(uv2.UpVotes, 0) + COALESCE(uv2.DownVotes, 0)) FROM UserVoteStats uv2) AND rp.PostRank <= 10 ORDER BY rp.PostRank;
//
// `u.Id = rp.Id` joins a user id to a post id, so it goes through the raw ids. `x > AVG(x)` is compared exactly, as x * n > sum.
fn q24313(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let (sum, n) = (&uv).select(Same::<[i64; 2]>::new()).fold_flat((0i64, 0i64), |(s, n), a| (s + a[0] + a[1], n + 1));
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let pc = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |k, _| k + 1);
    let Post { score, .. } = &db.post;
    let pm = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let rp = ranked(drain(&pm), |&(p, a)| (Reverse(score.get(p).unwrap()), Reverse(a[0])), false);
    let rp = rel(rp.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), r)| (p, a, r)).collect());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type R = (Id<Post>, [i64; 3], i64);
    let us = Ident::<User>::new().and((&uv).filt(move |a| (a[0] + a[1]) * n > sum)).and((&pc).opt());
    let v = drain((&rp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _): R| p).select(&db.post.origid).select(&uidx).select(us))));
    rows(v.into_iter().map(|(_, ((p, a, r), ((u, b), c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(c.unwrap_or(0))]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(v.BountyAmount) OVER (PARTITION BY p.Id) AS TotalBounty,
//        CASE WHEN p.PostTypeId = 1 THEN 'Question' WHEN p.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostCategory
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TagsWithPosts AS (SELECT t.TagName, COUNT(p.Id) AS RelatedPostCount FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(p.Id) > 0),
// BountyPosts AS (SELECT p.Id, p.Title, SUM(v.BountyAmount) AS TotalBounties FROM Posts p JOIN Votes v ON p.Id = v.PostId WHERE v.VoteTypeId IN (8, 9) GROUP BY p.Id, p.Title HAVING SUM(v.BountyAmount) > 0),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CommentCount, rp.TotalBounty, rp.PostCategory, CASE WHEN rp.RankScore <= 10 THEN 'Top Posts' ELSE 'Others' END AS RankingCategory,
//        COALESCE(bp.TotalBounties, 0) AS TotalBounties FROM RankedPosts rp LEFT JOIN BountyPosts bp ON rp.PostId = bp.Id)
// SELECT fp.PostId, fp.Title, fp.Score, fp.CommentCount, fp.TotalBounty, fp.PostCategory, fp.RankingCategory,
//        CASE WHEN fp.CommentCount > 10 THEN 'Highly Discussed' WHEN fp.TotalBounties > 0 THEN 'Bounty Offered' ELSE 'Regular' END AS PostStatus
// FROM FilteredPosts fp ORDER BY CASE WHEN fp.RankingCategory = 'Top Posts' THEN 1 ELSE 2 END, fp.Score DESC, fp.TotalBounties DESC;
//
// TagsWithPosts is never referenced. RankedPosts has no GROUP BY, so RankScore numbers the joined (post, comment, vote) rows; they are materialised and numbered,
// ties on Score broken by post id, then comment and vote id.
fn q21709(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1)));
    let j: MatSet<((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>)> = recent().select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt())).collect();
    let pw = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 3], |a, (c, b)| {
        let b = b.flatten();
        [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
    });
    let bp = db.vote.with((&db.vote.vote_type_id).is_in([8, 9])).group_by(&db.vote.post).select((&db.vote.bounty_amount).opt()).fold((0i64, 0i64), |(n, s), b| (n + b.is_some() as i64, s + b.unwrap_or(0)));
    let rk = ranked(drain(&j).into_iter().map(|x| x.0).collect(), |&((p, c), v)| (post_type_id.get(p).unwrap(), Reverse(score.get(p).unwrap()), p, c, v), false);
    let rk = rel(per_group(rk, |&((p, _), _)| post_type_id.get(p).unwrap()));
    type R = (((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>), i64);
    let v = drain((&rk).select(Same::<R>::new().and(Same::<R>::new().map(|(((p, _), _), _): R| p).select((&pw).and((&bp).filt(|(n, s)| n > 0 && s > 0).opt())))));
    rows(v.into_iter().map(|(_, ((((p, _), _), r), (a, b)))| {
        let t = post_type_id.get(p).unwrap();
        let tb = b.map_or(0, |b| b.1);
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1])]);
        f.push(V::S(if t == 1 { "Question" } else if t == 2 { "Answer" } else { "Other" }));
        f.push(V::S(if r <= 10 { "Top Posts" } else { "Others" }));
        f.push(V::S(if a[0] > 10 { "Highly Discussed" } else if tb > 0 { "Bounty Offered" } else { "Regular" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation < 100 THEN 'Novice' WHEN u.Reputation BETWEEN 100 AND 1000 THEN 'Intermediate' WHEN u.Reputation > 1000 THEN 'Expert'
//        ELSE 'Undefined' END AS ReputationLevel FROM Users u),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseOpenCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (50, 52, 53) THEN 1 END) AS BumpCount FROM PostHistory ph GROUP BY ph.PostId),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId)
// SELECT up.DisplayName, rp.PostId, rp.Title, rp.Score, rp.ViewCount, ur.ReputationLevel, COALESCE(pus.CloseOpenCount, 0) AS CloseOpenCount, COALESCE(pus.DeleteCount, 0) AS DeleteCount,
//        COALESCE(pus.BumpCount, 0) AS BumpCount, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserReputation ur ON ur.UserId = up.Id LEFT JOIN PostHistorySummary pus ON pus.PostId = rp.PostId LEFT JOIN UserBadges ub ON ub.UserId = up.Id
// WHERE (rp.UserPostRank = 1 AND ur.Reputation >= 500) OR (rp.UserPostRank > 1 AND ur.Reputation < 500) ORDER BY rp.CreationDate DESC, ur.Reputation DESC LIMIT 100;
fn q23707(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).select(owner_user)), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap())), false);
    let rk = rel(per_group(v, |&(_, u)| u));
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id).fold([0i64; 3], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + (t == 12) as i64, a[2] + matches!(t, 50 | 52 | 53) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    type R = ((Id<Post>, Id<User>), i64);
    let v = drain((&rk).filt(|((_, u), r): R| {
        let rep = db.user.reputation.get(u).unwrap();
        (r == 1 && rep >= 500) || (r > 1 && rep < 500)
    }).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select((&phs).opt())).and(Same::<R>::new().map(|((_, u), _): R| u).select((&ub).opt()))));
    let v = top_n(v, |&(_, ((((p, u), _), _), _))| (Reverse(creation_date.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p), 100);
    rows(v.into_iter().map(|(_, ((((p, u), _), h), b))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.push(V::S(if rep < 100 { "Novice" } else if (100..=1000).contains(&rep) { "Intermediate" } else { "Expert" }));
        f.extend(h.unwrap_or([0; 3]).map(V::I));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        COALESCE(AVG(CAST(P.Score AS FLOAT)), 0) AS AverageScore, COALESCE(SUM(P.ViewCount), 0) AS TotalViews, DENSE_RANK() OVER (ORDER BY SUM(P.ViewCount) DESC) AS ViewRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UA.UserId, UA.DisplayName, UA.QuestionCount, UA.AnswerCount, UA.AverageScore, UA.TotalViews FROM UserActivity UA WHERE UA.ViewRank <= 10),
// RecentVotes AS (SELECT V.PostId, V.UserId, V.CreationDate, VT.Name AS VoteType FROM Votes V JOIN VoteTypes VT ON V.VoteTypeId = VT.Id WHERE V.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostsDetail AS (SELECT P.Id AS PostId, P.Title, P.Body, U.DisplayName AS Author, COALESCE(RV.TotalVotes, 0) AS RecentVotesCount, COALESCE(CH.ClosedCount, 0) AS ClosedPostCount
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN (SELECT PostId, COUNT(*) AS ClosedCount FROM PostHistory WHERE PostHistoryTypeId = 10 GROUP BY PostId) CH ON P.Id = CH.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS TotalVotes FROM RecentVotes GROUP BY PostId) RV ON P.Id = RV.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT TU.DisplayName AS TopUser, TU.QuestionCount, TU.AnswerCount, TU.AverageScore, TU.TotalViews, PD.Title AS PostTitle, PD.Author, PD.RecentVotesCount, PD.ClosedPostCount,
//        (CASE WHEN PD.ClosedPostCount > 0 THEN 'Closed' ELSE 'Open' END) AS PostStatus FROM TopUsers TU JOIN PostsDetail PD ON PD.Author = TU.DisplayName ORDER BY TU.TotalViews DESC, PD.RecentVotesCount DESC;
fn q3682(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, owner_user, .. } = &db.post;
    let ua = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 6], |a, x| match x {
        Some(((t, s), w)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + 1, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
        None => a,
    });
    let vr = ranked(drain(&ua), |&(_, a)| (a[4] == 0, Reverse(a[5])), true);
    let tu = rel(vr.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ch = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rv = db.vote.with((&db.vote.creation_date).ge(add_days(t0, -30))).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let by_author: HashIdx<Str, Id<Post>> = db.post.with(creation_date.ge(add_years(t0, -1))).select(owner_user.select(&db.user.display_name)).inv().collect();
    type R = (Id<User>, [i64; 6]);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&db.user.display_name).select(&by_author).select(Ident::<Post>::new().and((&rv).opt()).and((&ch).opt())))));
    rows(v.into_iter().map(|(_, ((u, a), ((p, r), c)))| {
        let c = c.unwrap_or(0);
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::F(if a[2] == 0 { 0.0 } else { a[3] as f64 / a[2] as f64 }), V::I(a[5])];
        f.extend(post_fields(db, p, &["title", "owner"]));
        f.extend([V::I(r.unwrap_or(0)), V::I(c), V::S(if c > 0 { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT b.Id) AS TotalBadges, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5 AND SUM(p.Score) IS NOT NULL),
// NextPostHistory AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, ph.Comment, ph.Text, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS RevisionRank FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (10, 11)),
// ClosedPosts AS (SELECT p.Id AS PostId, COUNT(nph.PostId) AS RevCount, MAX(nph.CreationDate) AS LastRevision, COUNT(DISTINCT ph.UserId) AS UserCount,
//        (SELECT COUNT(1) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS TotalDownVotes FROM Posts p JOIN NextPostHistory nph ON p.Id = nph.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.ClosedDate IS NOT NULL GROUP BY p.Id),
// FinalReport AS (SELECT tu.UserId, tu.DisplayName, tu.TotalScore, tu.TotalPosts, cp.PostId, cp.RevCount, cp.UserCount, cp.LastRevision, cp.TotalDownVotes FROM TopUsers tu JOIN ClosedPosts cp ON tu.UserId = cp.PostId)
// SELECT fr.UserId, fr.DisplayName, fr.TotalScore, fr.TotalPosts, fr.PostId, fr.RevCount, fr.UserCount, fr.LastRevision, fr.TotalDownVotes FROM FinalReport fr ORDER BY fr.TotalScore DESC, fr.UserCount DESC;
//
// RankedPosts is never referenced. `tu.UserId = cp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q31598(db: &'static So) -> String {
    let Post { score, closed_date, .. } = &db.post;
    let ts_ = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt().and(badges_of(db).opt())).fold((0i64, 0i64), |(n, s), (p, _)| match p {
        Some(x) => (n + 1, s + x),
        None => (n, s),
    });
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let nph = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))).select(hd);
    let closed = || db.post.with(closed_date);
    let cp = closed().group_by(Ident::<Post>::new()).select(nph.and(history_of(db).opt())).fold((0i64, i64::MIN), |(n, m), (d, _)| (n + 1, m.max(d)));
    let uc = closed().group_by(Ident::<Post>::new()).select(history_of(db).select(user)).count_distinct();
    let dv = closed().group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(3))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let pidx: HashIdx<i64, Id<Post>> = closed().select(&db.post.origid).inv().collect();
    let v = drain((&ts_).filt(|(n, _)| n > 0).and((&tp).filt(|n| n > 5)).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&cp).and((&uc).opt()).and(&dv))));
    let v = top_n(v, |&(u, (((_, s), _), (((_, _), c), _)))| (Reverse(s), Reverse(c.unwrap_or(0)), u), 0);
    rows(v.into_iter().map(|(u, (((_, s), n), (((p, (k, m)), c), d)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(s), V::I(n)]);
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(k), V::I(c.unwrap_or(0)), V::T(m), V::I(d)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, MAX(u.CreationDate) AS AccountCreationDate, MAX(u.LastAccessDate) AS LastAccessDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostRanking AS (SELECT p.Id AS PostId, p.Title, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        SUM(CASE WHEN ph.UserId IS NOT NULL THEN 1 ELSE 0 END) AS HistoryCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title),
// TopPosts AS (SELECT PostId, Title, VoteCount, Upvotes, Downvotes, HistoryCount, RANK() OVER (ORDER BY VoteCount DESC) AS Rank FROM PostRanking)
// SELECT us.UserId, us.DisplayName, us.TotalPosts, us.Questions, us.Answers, us.TotalComments, us.GoldBadges, us.SilverBadges, us.BronzeBadges, us.AccountCreationDate, us.LastAccessDate,
//        tp.PostId, tp.Title AS TopPostTitle, tp.VoteCount AS TopPostVoteCount, tp.Upvotes AS TopPostUpvotes, tp.Downvotes AS TopPostDownvotes, tp.HistoryCount AS TopPostEditHistory, tp.Rank AS PostRank
// FROM UserStats us LEFT JOIN TopPosts tp ON us.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId LIMIT 1) WHERE us.TotalPosts > 0 ORDER BY us.TotalPosts DESC, tp.VoteCount DESC FETCH FIRST 10 ROWS ONLY;
//
// The order leads with TotalPosts, a plain count, so users are taken in that order until their posts fill the ten rows, and UserStats' post x comment x badge
// product is folded for those users alone.
fn q5328(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let pr = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(history_of(db).select((&db.post_history.user).opt()).opt())).fold([0i64; 4], |a, (t, h)| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + h.flatten().is_some() as i64]
    });
    let rk = ranked(drain(&pr), |&(_, a)| Reverse(a[0]), false);
    let rk = rel(rk.into_iter().map(|((p, a), r)| (p, (a, r))).collect());
    let rank: HashIdx<Id<Post>, (Id<Post>, ([i64; 4], i64))> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let order = top_n(drain(&tp), |&(u, n)| (Reverse(n), u), 0);
    let mut k = 0;
    let mut take = 0;
    while take < order.len() && (k < 10 || order[take].1 == order[take - 1].1) {
        k += order[take].1 as usize;
        take += 1;
    }
    let cand: MatSet<Id<User>> = rel(order[..take].iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let t = p.map(|x| x.0);
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (b == Some(1)) as i64, a[3] + (b == Some(2)) as i64, a[4] + (b == Some(3)) as i64]
        });
    let tc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&us).and(&tc).and(&tp).and(posts_of(db).select(&rank)));
    let v = top_n(v, |&(u, (((_, _), n), (p, (a, _))))| (Reverse(n), Reverse(a[0]), u, p), 10);
    let _ = owner_user;
    rows(v.into_iter().map(|(u, (((a, c), n), (p, (x, r))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(c), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.extend(ucols(db, u, &["ucreated", "last_access"]));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(x[0]), V::I(x[1]), V::I(x[2]), V::I(x[3]), V::I(r)]);
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.Id, ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserId, ph.UserDisplayName, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) as VersionRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 13)),
// UserReputationTotales AS (SELECT u.Id AS UserId, SUM(u.Reputation) AS TotalReputation FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.CreationDate >= '2021-01-01' GROUP BY u.Id),
// PostVoteCounts AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(v.Id) AS TotalVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// QuestionsWithAcceptedAnswers AS (SELECT p.Id AS QuestionId, p.AcceptedAnswerId, p.Title, p.CreationDate, p.Score, COALESCE(ans.OwnerUserId, -1) AS AcceptedAnswerUserId
//     FROM Posts p LEFT JOIN Posts ans ON p.AcceptedAnswerId = ans.Id WHERE p.PostTypeId = 1)
// SELECT q.Title, q.CreationDate, q.Score, COALESCE(ph.VersionRank, 0) AS CloseOpenDeleteCount, u.TotalReputation, pov.UpVotes, pov.DownVotes, pov.TotalVotes,
//        CASE WHEN q.AcceptedAnswerUserId <> -1 THEN 'Has Accepted Answer' ELSE 'No Accepted Answer' END AS AnswerStatus,
//        CASE WHEN ph.VersionRank IS NULL THEN 'Not Closed/Open/Deleted' ELSE 'Closed/Open/Deleted' END AS PostHistoryStatus
// FROM QuestionsWithAcceptedAnswers q LEFT JOIN RecursivePostHistory ph ON q.QuestionId = ph.PostId LEFT JOIN UserReputationTotales u ON q.AcceptedAnswerUserId = u.UserId
// LEFT JOIN PostVoteCounts pov ON q.QuestionId = pov.PostId WHERE (ph.VersionRank IS NOT NULL OR q.AcceptedAnswerUserId IS NOT NULL) ORDER BY q.Score DESC, u.TotalReputation DESC LIMIT 100;
//
// AcceptedAnswerUserId is a COALESCE, never NULL, so the WHERE keeps every row; its -1 meets the Community user (Id -1), so the join goes through the raw ids.
// A question's history rows are numbered 1..n whatever the order of ties, and nothing else projected tells them apart.
fn q31862(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, owner_user, owner_user_id, creation_date, score, .. } = &db.post;
    let urt = db.post.with(creation_date.ge(ts(2021, 1, 1, 0, 0, 0))).group_by(owner_user).select(owner_user.select(&db.user.reputation)).fold(0i64, |s, r| s + r);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pov = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let rph = ranked(drain(db.post_history.with(post_history_type_id.is_in([10, 11, 12, 13])).select(post)), |&(h, p)| (p, Reverse(hd.get(h).unwrap()), h), false);
    let rph = rel(per_group(rph, |&(_, p)| p).into_iter().map(|((_, p), r)| (p, r)).collect());
    let vr: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rph).map(|(p, _)| p).inv().select(&rph).collect();
    let aid = accepted_answer.select(owner_user_id.opt()).opt().map(|x: Option<Option<i64>>| x.flatten().unwrap_or(-1));
    let v = drain(db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and((&vr).map(|(_, r)| r).opt()).and(aid).and(&pov)));
    let v = rel(v.into_iter().map(|x| x.1).collect());
    type R = (((Id<Post>, Option<i64>), i64), [i64; 3]);
    let v = drain((&v).select(Same::<R>::new().and(Same::<R>::new().map(|(((_, _), a), _): R| a).select(&uidx).select(&urt).opt())));
    let v = top_n(v, |&(_, ((((p, r), _), _), t))| (Reverse(score.get(p).unwrap()), t.is_none(), Reverse(t), p, r), 100);
    rows(v.into_iter().map(|(_, ((((p, r), a), c), t))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(r.unwrap_or(0)), oint(t)]);
        f.extend(c.map(V::I));
        f.push(V::S(if a != -1 { "Has Accepted Answer" } else { "No Accepted Answer" }));
        f.push(V::S(if r.is_none() { "Not Closed/Open/Deleted" } else { "Closed/Open/Deleted" }));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges, COUNT(DISTINCT B.Id) AS TotalBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers, SUM(P.Score) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// ClosedPosts AS (SELECT PH.UserId, COUNT(PH.PostId) AS ClosedPostCount, MIN(PH.CreationDate) AS FirstCloseDate, MAX(PH.CreationDate) AS LastCloseDate FROM PostHistory PH
//     WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.UserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, COALESCE(UBS.TotalBadges, 0) AS TotalBadges, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(CP.ClosedPostCount, 0) AS ClosedPosts,
//        CASE WHEN COALESCE(PS.TotalPosts, 0) = 0 THEN 0 ELSE COALESCE(PS.TotalScore, 0) / NULLIF(COALESCE(PS.TotalPosts, 1), 0) END AS ScorePerPost
//     FROM Users U LEFT JOIN UserBadgeStats UBS ON U.Id = UBS.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId LEFT JOIN ClosedPosts CP ON U.Id = CP.UserId
//     WHERE U.Reputation > 50 ORDER BY ScorePerPost DESC, TotalBadges DESC),
// UserRanks AS (SELECT *, ROW_NUMBER() OVER (ORDER BY ScorePerPost DESC, TotalBadges DESC) AS Rank FROM TopUsers)
// SELECT UR.Rank, UR.DisplayName, UR.TotalBadges, UR.TotalPosts, UR.ClosedPosts, UR.ScorePerPost FROM UserRanks UR WHERE UR.Rank <= 10 AND (UR.TotalPosts > 5 OR UR.TotalBadges > 3) ORDER BY UR.Rank;
//
// A tie inside the ROW_NUMBER goes to the smaller user id.
fn q21726(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ps = db.post.group_by(owner_user).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let spp = |p: Option<(i64, i64)>| match p {
        Some((n, s)) if n > 0 => s as f64 / n as f64,
        _ => 0.0,
    };
    let v = drain(db.user.with((&db.user.reputation).gt(50)).select((&bc).opt().and((&ps).opt()).and((&cp).opt())));
    let v = top_n(v, |&(u, ((b, p), _))| (Reverse(fkey(spp(p))), Reverse(b.unwrap_or(0)), u), 10);
    let v: Vec<_> = v.into_iter().enumerate().map(|(i, x)| (i as i64 + 1, x)).collect();
    let v = drain(rel(v).filt(|(_, (_, ((b, p), _)))| p.map_or(0, |p| p.0) > 5 || b.unwrap_or(0) > 3));
    rows(v.into_iter().map(|(_, (r, (u, ((b, p), c))))| row(vec![V::I(r), user_col(db, u, "name"), V::I(b.unwrap_or(0)), V::I(p.map_or(0, |p| p.0)), V::I(c.unwrap_or(0)), V::F(spp(p))])))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, MAX(U.Reputation) AS MaxReputation FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
//        COUNT(CASE WHEN P.PostTypeId = 3 THEN 1 END) AS Wikis, SUM(COALESCE(P.Score, 0)) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, COALESCE(UB.BadgeCount, 0) AS BadgeCount, PS.TotalPosts, PS.Questions, PS.Answers, PS.Wikis, PS.TotalScore,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank, ROW_NUMBER() OVER (ORDER BY COALESCE(UB.BadgeCount, 0) DESC) AS BadgeRank
//     FROM Users U LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId WHERE U.Reputation IS NOT NULL)
// SELECT TU.DisplayName, TU.Reputation, TU.BadgeCount, TU.TotalPosts, TU.Questions, TU.Answers, TU.Wikis, TU.TotalScore,
//        CASE WHEN TU.ReputationRank <= 10 THEN 'Top Reputation' WHEN TU.BadgeCount > 5 THEN 'Very Badged User' ELSE 'Regular User' END AS UserClassification,
//        CASE WHEN EXISTS (SELECT 1 FROM Posts P WHERE P.OwnerUserId = TU.Id AND P.AcceptedAnswerId IS NOT NULL) THEN 'Has Accepted Answers' ELSE 'No Accepted Answers' END AS AnswerStatus,
//        CASE WHEN TU.BadgeRank <= 10 THEN 'Top Badged User' ELSE 'Regular Badged User' END AS BadgeStatus
// FROM TopUsers TU WHERE TU.TotalPosts > 0 ORDER BY TU.Reputation DESC, TU.BadgeCount DESC;
//
// Ties inside the two ROW_NUMBERs go to the smaller user id.
fn q21986(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, accepted_answer, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 5], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + s]);
    let acc: MatSet<Id<User>> = db.post.with(accepted_answer).select(owner_user).collect();
    let v = drain((&bc).and((&ps).opt()));
    let v = ranked(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), false);
    let v = ranked(v, |&((u, (b, _)), _)| (Reverse(b), u), false);
    let v = drain(rel(v).filt(|(((_, (_, p)), _), _)| p.map_or(false, |p| p[0] > 0)).select(
        Same::<(((Id<User>, (i64, Option<[i64; 5]>)), i64), i64)>::new().and(Same::<(((Id<User>, (i64, Option<[i64; 5]>)), i64), i64)>::new().map(|(((u, _), _), _)| u).select(Ident::<User>::new().with(&acc)).opt()),
    ));
    rows(v.into_iter().map(|(_, ((((u, (b, p)), rr), br), a))| {
        let p = p.unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(p.map(V::I));
        f.push(V::S(if rr <= 10 { "Top Reputation" } else if b > 5 { "Very Badged User" } else { "Regular User" }));
        f.push(V::S(if a.is_some() { "Has Accepted Answers" } else { "No Accepted Answers" }));
        f.push(V::S(if br <= 10 { "Top Badged User" } else { "Regular Badged User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, P.Score, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS PostRank,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.OwnerUserId) AS UpVoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.OwnerUserId) AS DownVoteCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldCount, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverCount, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeCount
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// CloseReasonCounts AS (SELECT PH.UserId, PH.PostHistoryTypeId, COUNT(PH.Id) AS CloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId, PH.PostHistoryTypeId),
// MergedStats AS (SELECT R.OwnerUserId, COUNT(R.PostId) AS TotalPosts, SUM(CASE WHEN R.PostRank = 1 THEN 1 ELSE 0 END) AS TopPosts, U.GoldCount, U.SilverCount, U.BronzeCount,
//        COALESCE(CC.CloseCount, 0) AS TotalCloseVotes FROM RankedPosts R JOIN UserBadges U ON R.OwnerUserId = U.UserId LEFT JOIN CloseReasonCounts CC ON R.OwnerUserId = CC.UserId
//     GROUP BY R.OwnerUserId, U.GoldCount, U.SilverCount, U.BronzeCount, CC.CloseCount)
// SELECT M.OwnerUserId, M.TotalPosts, M.TopPosts, M.GoldCount, M.SilverCount, M.BronzeCount, M.TotalCloseVotes,
//        CASE WHEN M.TotalPosts > 100 THEN 'Veteran' WHEN M.TotalPosts > 50 THEN 'Experienced' ELSE 'Newcomer' END AS UserCategory
// FROM MergedStats M WHERE M.TotalPosts > (SELECT AVG(TotalPosts) FROM MergedStats) ORDER BY M.TotalPosts DESC OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY;
//
// RankedPosts has no GROUP BY, so TotalPosts counts the owner's joined (post, vote) rows, and exactly one of them has PostRank 1.
fn q24761(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cc = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let ms = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).group_by(owner_user).select(votes_of(db).opt()).fold(0i64, |n, _| n + 1);
    let m = (&ms).and(&ub).and((&cc).opt());
    let (sum, n) = (&m).select(Same::<((i64, [i64; 3]), Option<i64>)>::new()).fold_flat((0i64, 0i64), |(s, n), ((t, _), _)| (s + t, n + 1));
    let v = drain((&m).filt(move |((t, _), _)| t * n > sum));
    let v = top_n(v, |&(u, ((t, _), _))| (Reverse(t), u), 15);
    rows(v.into_iter().skip(5).map(|(u, ((t, b), c))| {
        let mut f = vec![user_col(db, u, "uid"), V::I(t), V::I(1)];
        f.extend(b.map(V::I));
        f.push(V::I(c.unwrap_or(0)));
        f.push(V::S(if t > 100 { "Veteran" } else if t > 50 { "Experienced" } else { "Newcomer" }));
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY U.Reputation DESC) AS Rank FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Comments C ON U.Id = C.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.OwnerUserId, COUNT(C.Id) AS TotalComments, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, SUM(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseVotes,
//        SUM(CASE WHEN PH.PostHistoryTypeId = 24 THEN 1 ELSE 0 END) AS SuggestedEdits FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.CreationDate >= '2023-01-01' GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.OwnerUserId),
// TopPosts AS (SELECT PA.PostId, PA.Title, PA.Score, PA.ViewCount, RANK() OVER (ORDER BY PA.Score DESC) AS PostRank FROM PostActivity PA)
// SELECT UE.UserId, UE.DisplayName, UE.Reputation, UE.Upvotes AS UserUpvotes, UE.Downvotes AS UserDownvotes, PA.Title AS PostTitle, PA.Score AS PostScore, PA.ViewCount AS PostViewCount, T.PostRank,
//        CASE WHEN PA.Upvotes - PA.Downvotes < 0 THEN 'Negative Feedback' WHEN PA.CloseVotes > 0 THEN 'Under Review' ELSE 'Active' END AS PostStatus
// FROM UserEngagement UE JOIN PostActivity PA ON UE.UserId = PA.OwnerUserId JOIN TopPosts T ON PA.PostId = T.PostId ORDER BY UE.Reputation DESC, PA.Score DESC;
//
// UserEngagement's vote x comment product is folded only for the owners of PostActivity's posts, the only users the final join keeps.
fn q21066(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let pa_posts = || db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)));
    let pa = pa_posts()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, ((_, t), h)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(h, Some(10 | 11)) as i64]);
    let tp = ranked(drain(&pa), |&(p, _)| Reverse(score.get(p).unwrap()), false);
    let tp = rel(tp.into_iter().map(|((p, a), r)| (p, a, r)).collect());
    let owners: MatSet<Id<User>> = pa_posts().select(owner_user).collect();
    let ue = (&owners).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(comments_by(db).opt())).fold([0i64; 2], |a, (t, _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    type R = (Id<Post>, [i64; 3], i64);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _): R| p).select(owner_user).select(Ident::<User>::new().and(&ue)))));
    rows(v.into_iter().map(|(_, ((p, a, r), (u, e)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(e[0]), V::I(e[1])]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(V::I(r));
        f.push(V::S(if a[0] - a[1] < 0 { "Negative Feedback" } else if a[2] > 0 { "Under Review" } else { "Active" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswersCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionsCount, COALESCE(SUM(CASE WHEN P.PostTypeId IN (10, 11) THEN 1 ELSE 0 END), 0) AS ClosedPostCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// BadgeSummary AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId),
// RankStats AS (SELECT UserId, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank, DENSE_RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank FROM UserStats),
// CombinedStats AS (SELECT US.UserId, US.DisplayName, US.Reputation, US.PostCount, US.AnswersCount, US.QuestionsCount, US.ClosedPostCount, COALESCE(BS.GoldBadges, 0) AS GoldBadges,
//        COALESCE(BS.SilverBadges, 0) AS SilverBadges, COALESCE(BS.BronzeBadges, 0) AS BronzeBadges, RS.ReputationRank, RS.PostCountRank
//     FROM UserStats US LEFT JOIN BadgeSummary BS ON US.UserId = BS.UserId LEFT JOIN RankStats RS ON US.UserId = RS.UserId)
// SELECT CB.DisplayName, CB.Reputation, CB.PostCount, CB.AnswersCount, CB.QuestionsCount, CB.ClosedPostCount, CB.GoldBadges, CB.SilverBadges, CB.BronzeBadges, CB.ReputationRank, CB.PostCountRank,
//        CASE WHEN CB.ClosedPostCount > 0 THEN 'Has Closed Posts' ELSE 'No Closed Posts' END AS ClosedPostStatus,
//        CASE WHEN CB.Reputation > 1000 THEN 'High Reputation' WHEN CB.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationTier
// FROM CombinedStats CB WHERE CB.PostCount > 5 ORDER BY CB.Reputation DESC, CB.PostCount DESC LIMIT 10 OFFSET 5;
fn q2832(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 4], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + matches!(t, 10 | 11) as i64],
        None => a,
    });
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = ranked(drain(&us), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[0]), true);
    type R = (((Id<User>, [i64; 4]), i64), i64);
    let v = drain(rel(v).filt(|(((_, a), _), _): R| a[0] > 5).select(Same::<R>::new().and(Same::<R>::new().map(|(((u, _), _), _): R| u).select((&bs).opt()))));
    let v = top_n(v, |&(_, ((((u, a), _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), u), 15);
    rows(v.into_iter().skip(5).map(|(_, ((((u, a), rr), pr), b))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend([V::I(rr), V::I(pr), V::S(if a[3] > 0 { "Has Closed Posts" } else { "No Closed Posts" })]);
        f.push(V::S(if rep > 1000 { "High Reputation" } else if (500..=1000).contains(&rep) { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT ub.UserId, ub.DisplayName, COALESCE(ps.Questions, 0) AS TotalQuestions, COALESCE(ps.Answers, 0) AS TotalAnswers, COALESCE(ps.TotalScore, 0) AS TotalScore,
//        COALESCE(ps.TotalViews, 0) AS TotalViews, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges FROM UserBadges ub LEFT JOIN PostStats ps ON ub.UserId = ps.OwnerUserId),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.TotalQuestions, ua.TotalAnswers, ua.TotalScore, ua.TotalViews, RANK() OVER (ORDER BY ua.TotalScore DESC, ua.TotalViews DESC) AS ScoreRank
//     FROM UserActivity ua WHERE ua.BadgeCount > 0)
// SELECT tu.DisplayName, tu.TotalQuestions, tu.TotalAnswers, tu.TotalScore, tu.TotalViews, tu.ScoreRank,
//        (CASE WHEN tu.ScoreRank <= 10 THEN 'Top User' WHEN tu.ScoreRank > 10 AND tu.ScoreRank <= 20 THEN 'Promising User' ELSE 'Newbie' END) AS UserCategory, COALESCE(ub.BadgeCount, 0) AS TotalBadges
// FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId
// WHERE (SELECT COUNT(*) FROM Votes v WHERE v.UserId = tu.UserId AND v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR') > 5 ORDER BY tu.ScoreRank LIMIT 50;
fn q21432(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 4], |a, ((t, s), w)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.unwrap_or(0)]);
    let rv = db.vote.with((&db.vote.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.select((&bc).and((&ps).opt())));
    let v = ranked(v, |&(_, (_, p))| {
        let p = p.unwrap_or([0; 4]);
        (Reverse(p[2]), Reverse(p[3]))
    }, false);
    type R = ((Id<User>, (i64, Option<[i64; 4]>)), i64);
    let v = drain(rel(v).with(Same::<R>::new().map(|((u, _), _): R| u).select((&rv).filt(|n| n > 5))));
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&((u, _), r)| (r, u), 50);
    rows(v.into_iter().map(|((u, (b, p)), r)| {
        let p = p.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(p.map(V::I));
        f.push(V::I(r));
        f.push(V::S(if r <= 10 { "Top User" } else if r <= 20 { "Promising User" } else { "Newbie" }));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT p.Id, p.Title, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// PostWithBadges AS (SELECT p.Id AS PostId, p.Title, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Posts p LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE b.Class IN (1, 2, 3) GROUP BY p.Id, p.Title),
// ClosedPostDetails AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseVotes, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// FinalOutput AS (SELECT p.Id, p.Title, u.DisplayName, COALESCE(uv.TotalVotes, 0) AS UserVotes, COALESCE(pb.BadgeCount, 0) AS BadgeCount, COALESCE(pb.HighestBadgeClass, 0) AS HighestBadgeClass,
//        COALESCE(cp.CloseVotes, 0) AS CloseVotes, COALESCE(cp.LastClosedDate, '1900-01-01') AS LastClosedDate
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserVoteCounts uv ON u.Id = uv.UserId LEFT JOIN PostWithBadges pb ON p.Id = pb.PostId LEFT JOIN ClosedPostDetails cp ON p.Id = cp.PostId
//     WHERE (uv.TotalVotes IS NULL OR uv.TotalVotes > 10) AND u.Reputation >= 100)
// SELECT *, CASE WHEN CloseVotes > 0 THEN 'Closed' ELSE 'Open' END AS PostStatus FROM FinalOutput WHERE BadgeCount > 0 OR CloseVotes > 0 ORDER BY HighestBadgeClass DESC, UserVotes DESC, Title;
//
// TopPosts is never referenced. Every user has a UserVoteCounts row and COUNT is never NULL, so the first WHERE is TotalVotes > 10.
fn q3442(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let pb = db.post.group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).select(Ident::<Badge>::new().with((&db.badge.class).is_in([1, 2, 3]))).select(&db.badge.class)).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let users = Ident::<User>::new().with((&db.user.reputation).ge(100)).and((&uv).filt(|n| n > 10));
    let v = drain(db.post.select(owner_user.select(users).and((&pb).opt()).and((&cp).opt())).filt(|((_, b), c): (((Id<User>, i64), Option<(i64, i64)>), Option<(i64, i64)>)| b.is_some() || c.is_some()));
    rows(v.into_iter().map(|(p, (((u, n), b), c))| {
        let b = b.unwrap_or((0, 0));
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(n), V::I(b.0), V::I(b.1)]);
        f.extend(match c {
            Some((k, d)) => [V::I(k), V::T(d), V::S("Closed")],
            None => [V::I(0), V::T(ts(1900, 1, 1, 0, 0, 0)), V::S("Open")],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//        SUM(CASE WHEN p.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopReputedUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, PositiveScorePosts, TotalComments, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation WHERE Reputation > 0),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT ph.Id) AS EditCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId GROUP BY p.Id, p.Title, p.CreationDate),
// ProminentPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.EditCount, ps.RelatedPostsCount,
//        ROW_NUMBER() OVER (ORDER BY ps.UpVotes - ps.DownVotes DESC) AS PopularityRank FROM PostStats ps WHERE ps.CommentCount > 5)
// SELECT u.DisplayName AS UserName, u.Reputation AS UserReputation, pp.Title AS PopularPostTitle, pp.UpVotes, pp.DownVotes, pp.CommentCount AS Comments, pp.EditCount AS Edits, pp.RelatedPostsCount AS RelatedLinks
// FROM TopReputedUsers u JOIN ProminentPosts pp ON u.TotalPosts > 10 WHERE pp.PopularityRank <= 10 ORDER BY u.Reputation DESC, pp.UpVotes DESC;
//
// The ON names only u, so the users with more than ten posts are crossed with the ten most popular posts. A tie inside PopularityRank goes to the smaller post id.
fn q28557(db: &'static So) -> String {
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).opt()).and(links_of(db).opt()))
        .fold([0i64; 3], |a, (((c, t), _), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ec = db.post.group_by(Ident::<Post>::new()).select(history_of(db)).fold(0i64, |n, _| n + 1);
    let rc = db.post.group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let pp = top_n(drain((&ps).filt(|a| a[0] > 5)), |&(p, a)| (Reverse(a[1] - a[2]), p), 10);
    let pp = rel(drain(rel(pp).select(Same::<(Id<Post>, [i64; 3])>::new().and(Same::<(Id<Post>, [i64; 3])>::new().map(|(p, _)| p).select((&ec).opt().and((&rc).opt()))))).into_iter().map(|x| x.1).collect());
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let us = rel(drain(db.user.with((&db.user.reputation).gt(0)).select((&tp).filt(|n| n > 10))));
    let mut v = Vec::new();
    (&us).cross(&pp).drive(|_, ((u, _), ((p, a), (e, r)))| v.push((u, p, a, e, r)));
    rows(v.into_iter().map(|(u, p, a, e, r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[0]), V::I(e.unwrap_or(0)), V::I(r.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore FROM Posts p WHERE p.Score IS NOT NULL),
// PostWithBadges AS (SELECT rp.PostId, rp.OwnerUserId, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM RankedPosts rp LEFT JOIN Badges b ON rp.OwnerUserId = b.UserId GROUP BY rp.PostId, rp.OwnerUserId),
// AggregateData AS (SELECT bw.PostId, bw.BadgeCount, bw.GoldBadges, bw.SilverBadges, bw.BronzeBadges, p.Tags, COUNT(c.Id) FILTER (WHERE c.Score > 0) AS PositiveComments,
//        COUNT(c.Id) FILTER (WHERE c.Score < 0) AS NegativeComments, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPosts FROM PostWithBadges bw JOIN Posts p ON bw.PostId = p.Id
//     LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId GROUP BY bw.PostId, bw.BadgeCount, bw.GoldBadges, bw.SilverBadges, bw.BronzeBadges, p.Tags)
// SELECT ad.PostId, ad.BadgeCount, ad.GoldBadges, ad.SilverBadges, ad.BronzeBadges, ad.Tags, ad.PositiveComments, ad.NegativeComments, ad.RelatedPosts,
//        CASE WHEN ad.BadgeCount > 0 THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus, CASE WHEN ad.RelatedPosts = 0 THEN 'No Related Posts' ELSE 'Has Related Posts' END AS RelatedStatus
// FROM AggregateData ad WHERE EXISTS (SELECT 1 FROM Posts p WHERE ad.PostId = p.Id AND p.PostTypeId = 1 AND p.CreationDate BETWEEN '2023-01-01' AND '2023-12-31')
//     AND (ad.PositiveComments > 5 OR ad.NegativeComments <= 3) ORDER BY ad.BadgeCount DESC, ad.PositiveComments DESC, ad.NegativeComments ASC OFFSET 10 ROWS FETCH NEXT 20 ROWS ONLY;
//
// RankScore is never read. The EXISTS is a filter on the post itself, so only the 2023 questions are aggregated.
fn q24973(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.between(ts(2023, 1, 1, 0, 0, 0), ts(2023, 12, 31, 0, 0, 0))));
    let pwb = qs().group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db).select(&db.badge.class)).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let cm = qs().group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt().and(links_of(db).opt())).fold([0i64; 2], |a, (s, _)| [a[0] + (s.map_or(false, |s| s > 0)) as i64, a[1] + (s.map_or(false, |s| s < 0)) as i64]);
    let rp = qs().group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let v = drain((&pwb).and((&cm).filt(|a| a[0] > 5 || a[1] <= 3)).and((&rp).opt()));
    let v = top_n(v, |&(p, ((b, c), _))| (Reverse(b[0]), Reverse(c[0]), c[1], p), 30);
    rows(v.into_iter().skip(10).map(|(p, ((b, c), r))| {
        let r = r.unwrap_or(0);
        let mut f = post_fields(db, p, &["id"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["tags"]));
        f.extend([V::I(c[0]), V::I(c[1]), V::I(r), V::S(if b[0] > 0 { "Has Badges" } else { "No Badges" }), V::S(if r == 0 { "No Related Posts" } else { "Has Related Posts" })]);
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankInType,
//        MAX(CASE WHEN b.Class = 1 THEN b.Date END) AS LastGoldBadgeDate, MAX(CASE WHEN b.Class = 2 THEN b.Date END) AS LastSilverBadgeDate, MAX(CASE WHEN b.Class = 3 THEN b.Date END) AS LastBronzeBadgeDate
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.PostTypeId),
// FilteredPosts AS (SELECT PostId, Title, ViewCount, Score, Upvotes, Downvotes, CommentCount, RankInType, LastGoldBadgeDate, LastSilverBadgeDate, LastBronzeBadgeDate FROM PostMetrics
//     WHERE ViewCount > (SELECT AVG(ViewCount) FROM Posts) AND (LastGoldBadgeDate IS NOT NULL OR LastSilverBadgeDate IS NOT NULL OR LastBronzeBadgeDate IS NOT NULL)),
// FinalSelection AS (SELECT *, CASE WHEN ViewCount > 1000 THEN 'High Traffic' WHEN ViewCount BETWEEN 500 AND 1000 THEN 'Medium Traffic' ELSE 'Low Traffic' END AS TrafficCategory FROM FilteredPosts)
// SELECT fs.PostId, fs.Title, fs.ViewCount, fs.Score, fs.Upvotes, fs.Downvotes, fs.CommentCount, fs.RankInType, fs.LastGoldBadgeDate, fs.LastSilverBadgeDate, fs.LastBronzeBadgeDate, fs.TrafficCategory,
//        (SELECT AVG(ViewCount) FROM FilteredPosts WHERE RankInType = fs.RankInType) AS AvgViewCountByType, CASE WHEN fs.Score IS NULL THEN 'Unscored Post' ELSE 'Scored Post' END AS PostScoreStatus
// FROM FinalSelection fs WHERE fs.RankInType < 5 ORDER BY fs.TrafficCategory DESC, fs.Score DESC;
//
// RankInType reads only Score, so the posts ranked below 5 in their type are picked first and the vote x comment x badge product is folded for them alone;
// AvgViewCountByType averages over posts of the same rank, which are among them. A tie on Score inside RankInType goes to the smaller post id.
fn q24237(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let (sum, n) = db.post.select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let rk = ranked(drain(db.post.select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false);
    let rk = rel(per_group(rk, |&(_, t)| t).into_iter().filter(|x| x.1 < 5).map(|((p, _), r)| (p, r)).collect());
    let top: MatSet<Id<Post>> = (&rk).map(|(p, _)| p).collect();
    let Badge { class, date, .. } = &db.badge;
    let pm = (&top)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user.select(badges_of(db).select(class.and(date))).opt()))
        .fold([0, 0, 0, i64::MIN, i64::MIN, i64::MIN], |a, ((t, c), b)| {
            let (k, d) = b.map_or((0, i64::MIN), |x| x);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64, if k == 1 { a[3].max(d) } else { a[3] }, if k == 2 { a[4].max(d) } else { a[4] }, if k == 3 { a[5].max(d) } else { a[5] }]
        });
    type R = (Id<Post>, i64);
    let fp = drain((&rk).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&pm).filt(|a| a[3] > i64::MIN || a[4] > i64::MIN || a[5] > i64::MIN)))).filt(move |((p, _), _): (R, [i64; 6])| {
        view_count.get(p).map_or(false, |w| w * n > sum)
    }));
    let fv = rel(fp.iter().map(|x| x.1).collect());
    let by_rank = (&fv).group_by(Same::<(R, [i64; 6])>::new().map(|((_, r), _)| r)).select(Same::<(R, [i64; 6])>::new().map(|((p, _), _)| view_count.get(p).unwrap())).fold((0i64, 0i64), |(s, k), w| (s + w, k + 1));
    let v = drain((&fv).select(Same::<(R, [i64; 6])>::new().and(Same::<(R, [i64; 6])>::new().map(|((_, r), _)| r).select(&by_rank))));
    rows(v.into_iter().map(|(_, (((p, r), a), (s, k)))| {
        let w = view_count.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r), tmax(a[3]), tmax(a[4]), tmax(a[5])]);
        f.push(V::S(if w > 1000 { "High Traffic" } else if (500..=1000).contains(&w) { "Medium Traffic" } else { "Low Traffic" }));
        f.extend([avg(s, k), V::S("Scored Post")]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, CASE WHEN Reputation >= 1000 THEN 'High Reputation' WHEN Reputation BETWEEN 500 AND 999 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationLevel FROM Users),
// NestedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Body, p.Score, COALESCE(c.CreationDate, p.CreationDate) AS FirstActivityDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate) AS PostOrder, COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPosts
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostVoteCounts AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostDetails AS (SELECT np.PostId, np.Title, np.CreationDate, np.Body, np.Score, np.FirstActivityDate, ur.DisplayName AS Author, ur.ReputationLevel, pvc.UpVotes, pvc.DownVotes,
//        (np.Score + COALESCE(pvc.UpVotes, 0) - COALESCE(pvc.DownVotes, 0)) AS NetScore FROM NestedPosts np JOIN UserReputation ur ON np.OwnerUserId = ur.Id LEFT JOIN PostVoteCounts pvc ON np.PostId = pvc.PostId)
// SELECT pd.Title, pd.Author, pd.ReputationLevel, pd.NetScore, pd.CreationDate, pd.Body, pd.FirstActivityDate,
//        CASE WHEN pd.NetScore > 10 THEN 'Highly Engaging' WHEN pd.NetScore BETWEEN -10 AND 10 THEN 'Moderately Engaging' ELSE 'Low Engagement' END AS EngagementLevel,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pd.PostId) AS CommentCount
// FROM PostDetails pd WHERE pd.ReputationLevel = 'High Reputation' AND pd.CreationDate BETWEEN cast('2024-10-01' as date) - INTERVAL '6 MONTH' AND cast('2024-10-01' as date)
//     AND pd.PostId NOT IN (SELECT RelatedPostId FROM PostLinks) ORDER BY pd.NetScore DESC, pd.CreationDate ASC LIMIT 100;
//
// PostOrder and TotalPosts are never read. Each post is one row per comment (FirstActivityDate differs); ties are broken on post and comment id.
fn q24030(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 0, 0, 0);
    let related: MatSet<Id<Post>> = db.post_link.select(&db.post_link.related_post).collect();
    let high = Ident::<User>::new().with((&db.user.reputation).ge(1000));
    let pvc = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let base = db.post.with(creation_date.ge(add_years(t0, -1)).and(creation_date.between(add_months(t0, -6), t0))).with(owner_user.select(high)).minus(&related);
    let v = drain(base.select(Ident::<Post>::new().and(comments_of(db).opt()).and((&pvc).opt()).and((&cc).opt())));
    let net = |p: Id<Post>, a: Option<[i64; 2]>| score.get(p).unwrap() + a.map_or(0, |a| a[0] - a[1]);
    let v = top_k(v, |&(_, (((p, _), a), _))| (Reverse(net(p, a)), creation_date.get(p).unwrap()), |&(_, (((p, c), _), _))| (p, c), 100);
    rows(v.into_iter().map(|(_, (((p, c), a), k))| {
        let n = net(p, a);
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([V::S("High Reputation"), V::I(n)]);
        f.extend(post_fields(db, p, &["created", "body"]));
        f.push(V::T(c.map_or(creation_date.get(p).unwrap(), |c| db.comment.creation_date.get(c).unwrap())));
        f.push(V::S(if n > 10 { "Highly Engaging" } else if (-10..=10).contains(&n) { "Moderately Engaging" } else { "Low Engagement" }));
        f.push(V::I(k.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, p.PostTypeId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        COALESCE(v.UpVoteCount, 0) AS UpVoteCount, COALESCE(v.DownVoteCount, 0) AS DownVoteCount FROM Posts p
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId),
// RecentActivity AS (SELECT p.Id AS PostId, p.Title, p.LastActivityDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.LastActivityDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.LastActivityDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(v.UpVotes, 0)) AS TotalUpVotes, SUM(COALESCE(v.DownVotes, 0)) AS TotalDownVotes FROM Users u JOIN RankedPosts rp ON u.Id = rp.OwnerUserId
//     LEFT JOIN (SELECT UserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY UserId) v ON u.Id = v.UserId
//     GROUP BY u.Id, u.DisplayName HAVING SUM(COALESCE(v.UpVotes, 0)) > 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVoteCount, rp.DownVoteCount, ra.CommentCount, tu.DisplayName AS OwnerDisplayName, tu.TotalUpVotes, tu.TotalDownVotes
// FROM RankedPosts rp JOIN RecentActivity ra ON rp.PostId = ra.PostId JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId WHERE rp.RankScore <= 5 OR (rp.PostTypeId = 1 AND rp.Score > 100)
// ORDER BY rp.Score DESC, ra.CommentCount DESC;
//
// TopUsers sums the user's vote summary once per joined post row, so the fold runs over the user's posts x the one summary row.
fn q30228(db: &'static So) -> String {
    let Post { post_type_id, score, last_activity_date, owner_user, .. } = &db.post;
    let rk = ranked(drain(db.post.select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap())), false);
    let rk = rel(per_group(rk, |&(_, t)| t).into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).and((&uv).opt())).fold([0i64; 2], |a, (_, v)| {
        let v = v.unwrap_or([0, 0]);
        [a[0] + v[0], a[1] + v[1]]
    });
    let ra = db.post.with(last_activity_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type X = ((((Id<Post>, i64), i64), Option<[i64; 2]>), (Id<User>, [i64; 2]));
    let v = drain(db.post.select(Ident::<Post>::new().and(&ra).and((&rank).map(|(_, r)| r)).and((&pv).opt()).and(owner_user.select(Ident::<User>::new().and((&tu).filt(|a| a[0] > 10)))).filt(
        |((((p, _), r), _), _): X| r <= 5 || (post_type_id.get(p).unwrap() == 1 && score.get(p).unwrap() > 100),
    )));
    rows(v.into_iter().map(|(_, ((((p, c), _), a), (u, t)))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), user_col(db, u, "name"), V::I(t[0]), V::I(t[1])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName, ph.Comment AS CloseReason, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS CloseRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10),
// VotesSummary AS (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// CombiningData AS (SELECT p.PostId, p.Title, p.ViewCount, p.CreationDate, up.UpVotes, up.DownVotes, cr.UserDisplayName AS ClosedBy, cr.CloseReason, ur.Reputation AS UserReputation
//     FROM RecentPosts p LEFT JOIN VotesSummary up ON p.PostId = up.PostId LEFT JOIN ClosedPosts cr ON p.PostId = cr.PostId AND cr.CloseRank = 1 JOIN UserReputation ur ON p.OwnerDisplayName = ur.DisplayName)
// SELECT cd.PostId, cd.Title, cd.ViewCount, cd.CreationDate, COALESCE(cd.UpVotes, 0) AS UpVotes, COALESCE(cd.DownVotes, 0) AS DownVotes, cd.ClosedBy, cd.CloseReason, cd.UserReputation,
//        CASE WHEN cd.UserReputation > 1000 THEN 'Highly Reputed' WHEN cd.UserReputation BETWEEN 500 AND 1000 THEN 'Moderately Reputed' ELSE 'Needs Attention' END AS ReputationStatus
// FROM CombiningData cd WHERE cd.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days') ORDER BY cd.UserReputation DESC, cd.ViewCount DESC LIMIT 100;
//
// The join to UserReputation is on DisplayName, so a post meets every user of its owner's name. A tie on CreationDate inside CloseRank goes to the larger history id.
fn q3938(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let vs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cl = top_per(drain(db.post_history.with(post_history_type_id.eq(10)).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let cl = rel(cl.into_iter().map(|(h, p)| (p, h)).collect());
    let last: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&cl).map(|(p, _)| p).inv().select(&cl).collect();
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain(db.post.with(creation_date.ge(add_days(t0, -30)).and(creation_date.ge(add_days(t0, -7)))).with(owner_user).select(
        (&vs).opt().and((&last).map(|(_, h)| h).opt()).and(owner_user.select(&db.user.display_name).select(&by_name)),
    ));
    let v = top_k(v, |&(p, (_, u))| {
        let w = view_count.get(p);
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w))
    }, |&(p, (_, u))| (p, u), 100);
    rows(v.into_iter().map(|(p, ((a, h), u))| {
        let a = a.unwrap_or([0, 0]);
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "views", "created"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [ostr(db.post_history.user_display_name.get(h)), ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null],
        });
        f.push(V::I(rep));
        f.push(V::S(if rep > 1000 { "Highly Reputed" } else if (500..=1000).contains(&rep) { "Moderately Reputed" } else { "Needs Attention" }));
        row(f)
    }))
}

// WITH RecursivePosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.AcceptedAnswerId, p.CreationDate, p.ViewCount, COALESCE(NULLIF(p.Body, ''), 'No content') AS Body, p.OwnerUserId, p.Score,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank FROM Posts p),
// UserRankings AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(p.Score, 0)) AS TotalScore, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.DisplayName),
// AboveAverageUsers AS (SELECT ur.UserId, ur.DisplayName FROM UserRankings ur WHERE ur.TotalScore > (SELECT AVG(TotalScore) FROM UserRankings)),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.Body, rp.ViewCount, au.DisplayName AS Author, COALESCE(pc.CommentCount, 0) AS Comments, ur.TotalScore, ur.QuestionCount, ur.PostCount
//     FROM RecursivePosts rp JOIN AboveAverageUsers au ON rp.OwnerUserId = au.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId JOIN UserRankings ur ON ur.UserId = rp.OwnerUserId)
// SELECT fr.PostId, fr.Title, fr.Body, fr.ViewCount, fr.Author, fr.Comments, fr.TotalScore, fr.QuestionCount, fr.PostCount,
//        CASE WHEN fr.Comments > 10 THEN 'Highly Discussed' WHEN fr.Comments BETWEEN 5 AND 10 THEN 'Moderately Discussed' ELSE 'Less Discussed' END AS DiscussionLevel,
//        CASE WHEN fr.TotalScore >= 100 THEN 'High Reputation' WHEN fr.TotalScore >= 50 THEN 'Moderate Reputation' ELSE 'Low Reputation' END AS ReputationLevel
// FROM FinalResults fr WHERE fr.ViewCount > 50 ORDER BY fr.ViewCount DESC, fr.TotalScore DESC LIMIT 100 OFFSET 0;
//
// OwnerPostRank is never read. `TotalScore > AVG(TotalScore)` is compared exactly, as s * n > sum.
fn q20076(db: &'static So) -> String {
    let Post { owner_user, score, post_type_id, view_count, body, .. } = &db.post;
    let ur = db.post.group_by(owner_user).select(score.and(post_type_id)).fold([0i64; 3], |a, (s, t)| [a[0] + s, a[1] + (t == 1) as i64, a[2] + 1]);
    let (sum, n) = (&ur).select(Same::<[i64; 3]>::new()).fold_flat((0i64, 0i64), |(s, n), a| (s + a[0], n + 1));
    let pc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |k, _| k + 1);
    let v = drain(db.post.with(view_count.gt(50)).select(owner_user.select((&ur).filt(move |a| a[0] * n > sum)).and((&pc).opt())));
    let v = top_k(v, |&(p, (a, _))| (Reverse(view_count.get(p)), Reverse(a[0])), |&(p, _)| p, 100);
    rows(v.into_iter().map(|(p, (a, c))| {
        let c = c.unwrap_or(0);
        let b = body.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(if b.is_empty() { "No content" } else { b }));
        f.extend(post_fields(db, p, &["views", "owner"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if c > 10 { "Highly Discussed" } else if (5..=10).contains(&c) { "Moderately Discussed" } else { "Less Discussed" }));
        f.push(V::S(if a[0] >= 100 { "High Reputation" } else if a[0] >= 50 { "Moderate Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, SUM(u.Reputation) AS TotalReputation FROM Users u GROUP BY u.Id),
// RecentBadges AS (SELECT b.UserId, b.Name AS BadgeName, b.Date, ROW_NUMBER() OVER (PARTITION BY b.UserId ORDER BY b.Date DESC) AS BadgeRank FROM Badges b WHERE b.Class = 1),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 10 AND ph.CreationDate > '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days' THEN 1 END) AS RecentCloseCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 12 AND ph.CreationDate > '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days' THEN 1 END) AS RecentDeleteCount FROM PostHistory ph GROUP BY ph.PostId),
// FinalSummary AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerName, rp.Score, ur.TotalReputation, COALESCE(rb.BadgeName, 'No Gold Badge') AS LastGoldBadge, phs.CloseCount, phs.RecentCloseCount,
//        phs.DeleteCount, phs.RecentDeleteCount FROM RankedPosts rp LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN RecentBadges rb ON rp.OwnerUserId = rb.UserId AND rb.BadgeRank = 1
//     LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId)
// SELECT *, CASE WHEN Score > 10 THEN 'High Score' WHEN Score BETWEEN 1 AND 10 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory FROM FinalSummary WHERE TotalReputation > 1000 ORDER BY CreationDate DESC, Score DESC;
//
// UserPostRank is never read. A tie on Date inside BadgeRank goes to the larger badge id.
fn q31994(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let Badge { user, class, date, .. } = &db.badge;
    let rb = top_per(drain(db.badge.with(class.eq(1)).select(user)), |&(_, u)| u, |&(b, _)| (Reverse(date.get(b).unwrap()), Reverse(b)), 1, false);
    let rb = rel(rb.into_iter().map(|(b, u)| (u, b)).collect());
    let last: HashIdx<Id<User>, (Id<User>, Id<Badge>)> = (&rb).map(|(u, _)| u).inv().select(&rb).collect();
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([0i64; 4], move |a, (t, d)| {
        [a[0] + (t == 10) as i64, a[1] + (t == 10 && d > since) as i64, a[2] + (t == 12) as i64, a[3] + (t == 12 && d > since) as i64]
    });
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and((&last).map(|(_, b)| b).opt())).and((&phs).opt())));
    rows(v.into_iter().map(|(p, ((u, b), h))| {
        let s = db.post.score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score"]);
        f.push(user_col(db, u, "rep"));
        f.push(V::S(b.map_or("No Gold Badge", |b| db.badge.name.get(b).unwrap())));
        f.extend(match h {
            Some(h) => h.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::S(if s > 10 { "High Score" } else if (1..=10).contains(&s) { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS TotalGoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS TotalSilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopActiveUsers AS (SELECT UserId, TotalPosts, TotalQuestions, TotalAnswers, TotalComments, TotalUpVotes, TotalDownVotes, TotalGoldBadges, TotalSilverBadges, TotalBronzeBadges,
//        RANK() OVER (ORDER BY TotalPosts DESC) AS UserRank FROM UserActivity),
// UserResults AS (SELECT u.DisplayName, u.Reputation, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalComments, tu.TotalUpVotes, tu.TotalDownVotes, tu.TotalGoldBadges, tu.TotalSilverBadges,
//        tu.TotalBronzeBadges FROM TopActiveUsers tu JOIN Users u ON tu.UserId = u.Id WHERE tu.UserRank <= 10)
// SELECT DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalComments, TotalUpVotes, TotalDownVotes, TotalGoldBadges, TotalSilverBadges, TotalBronzeBadges,
//        CONCAT('Total Activity Score: ', (TotalPosts + TotalAnswers * 2 + TotalUpVotes * 3 - TotalDownVotes * 1 + TotalGoldBadges * 10 + TotalSilverBadges * 5 + TotalBronzeBadges * 2)) AS ActivityScore
// FROM UserResults ORDER BY ActivityScore DESC;
//
// UserRank reads only COUNT(DISTINCT p.Id), a plain count, so the ten top posters are picked first and the posts x comments x own-votes x badges product is folded for them alone.
fn q29403(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rk = ranked(drain(&tp), |&(_, n)| Reverse(n), false);
    let top: MatSet<Id<User>> = rel(rk.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ov = own_votes(db);
    let ua = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(comments_of(db).opt()).and((&ov).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 8], |a, (p, b)| {
            let (t, c, v) = p.map_or((0, false, None), |((t, c), v)| (t, c.is_some(), v));
            [
                a[0] + (t == 1) as i64,
                a[1] + (t == 2) as i64,
                a[2] + c as i64,
                a[3] + (v == Some(2)) as i64,
                a[4] + (v == Some(3)) as i64,
                a[5] + (b == Some(1)) as i64,
                a[6] + (b == Some(2)) as i64,
                a[7] + (b == Some(3)) as i64,
            ]
        });
    rows(drain((&ua).and(&tp)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        let s = n + a[1] * 2 + a[3] * 3 - a[4] + a[5] * 10 + a[6] * 5 + a[7] * 2;
        f.push(V::Owned(format!("Total Activity Score: {s}")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.PostTypeId, p.CreationDate, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.OwnerUserId, p.PostTypeId, p.CreationDate),
// UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostActivity AS (SELECT rp.PostId, rp.Title, rp.OwnerUserId AS CreatorUserId, us.Reputation, rp.CommentCount, rp.UpVotes - rp.DownVotes AS NetScore FROM RankedPosts rp JOIN UserStatistics us ON rp.OwnerUserId = us.UserId
//     WHERE rp.PostRank = 1)
// SELECT pa.PostId, pa.Title, pa.Reputation, pa.CommentCount, pa.NetScore, CASE WHEN pa.NetScore > 0 THEN 'Popular' WHEN pa.NetScore = 0 THEN 'Neutral' ELSE 'Unpopular' END AS Popularity,
//        CASE WHEN (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pa.PostId AND v.VoteTypeId = 6) > 0 THEN 'Closed' ELSE 'Open' END AS Status
// FROM PostActivity pa LEFT JOIN (SELECT PostId, COUNT(*) AS LinkCount FROM PostLinks GROUP BY PostId) pl ON pa.PostId = pl.PostId
// ORDER BY pa.Reputation DESC, pa.NetScore DESC, COALESCE(pl.LinkCount, 0) DESC LIMIT 100;
fn q23063(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let first = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&first).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let closed = (&first).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(6)))).fold(0i64, |n, _| n + 1);
    let pl = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&rp).and(owner_user).and((&closed).opt()).and((&pl).opt()));
    let v = top_k(v, |&(_, (((a, u), _), l))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1] - a[2]), Reverse(l.unwrap_or(0))), |&(p, _)| p, 100);
    rows(v.into_iter().map(|(p, (((a, u), c), _))| {
        let n = a[1] - a[2];
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "rep"), V::I(a[0]), V::I(n)]);
        f.push(V::S(if n > 0 { "Popular" } else if n == 0 { "Neutral" } else { "Unpopular" }));
        f.push(V::S(if c.map_or(false, |c| c > 0) { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount, P.OwnerUserId, ROW_NUMBER() OVER(PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn
//     FROM Posts P WHERE P.PostTypeId = 1 AND P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// PostLinksCount AS (SELECT PL.PostId, COUNT(PL.RelatedPostId) AS LinksCount FROM PostLinks PL GROUP BY PL.PostId),
// ClosedPosts AS (SELECT PH.PostId, COUNT(*) AS CloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId),
// UserPostStats AS (SELECT U.DisplayName, U.Reputation, COALESCE(RP.PostId, -1) AS RecentPostId, P.Score, P.ViewCount, COALESCE(PLC.LinksCount, 0) AS LinksCount, COALESCE(CP.CloseCount, 0) AS CloseCount
//     FROM Users U LEFT JOIN RecentPosts RP ON U.Id = RP.OwnerUserId AND RP.rn = 1 LEFT JOIN Posts P ON P.Id = RP.PostId LEFT JOIN PostLinksCount PLC ON P.Id = PLC.PostId LEFT JOIN ClosedPosts CP ON P.Id = CP.PostId)
// SELECT U.DisplayName, U.Reputation, R.RecentPostId AS PostId, P.Score, P.ViewCount, COALESCE(CP.CloseCount, 0) AS CloseCount, COALESCE(PLC.LinksCount, 0) AS LinksCount,
//        CASE WHEN U.Reputation > 1000 THEN 'High Reputation' WHEN U.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory,
//        DENSE_RANK() OVER(ORDER BY U.Reputation DESC) AS ReputationRank
// FROM UserPostStats R JOIN Users U ON R.RecentPostId = U.Id LEFT JOIN Posts P ON P.Id = R.RecentPostId LEFT JOIN PostLinksCount PLC ON P.Id = PLC.PostId LEFT JOIN ClosedPosts CP ON P.Id = CP.PostId
// WHERE U.Reputation > 0 ORDER BY U.Reputation DESC;
//
// `R.RecentPostId = U.Id` joins a post id to a user id, so it goes through the raw ids; the -1 of a user with no recent question meets the Community user (Id -1).
// A tie on CreationDate inside rn goes to the larger post id.
fn q32890(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let rp = top_per(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let last: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let plc = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rid = (&last).map(|(_, p)| p).select(&db.post.origid).opt().map(|x: Option<i64>| x.unwrap_or(-1));
    let recent = (&last).map(|(_, p)| p).select(Ident::<Post>::new().and((&cp).opt()).and((&plc).opt())).opt();
    let v = drain(db.user.select(rid.select(&uidx).select(Ident::<User>::new().with((&db.user.reputation).gt(0))).and(recent)));
    let v = ranked(v, |&(_, (u, _))| Reverse(db.user.reputation.get(u).unwrap()), true);
    rows(v.into_iter().map(|((_, (u, p)), r)| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        match p {
            Some(((p, c), l)) => {
                f.extend(post_fields(db, p, &["id", "score", "views"]));
                f.extend([V::I(c.unwrap_or(0)), V::I(l.unwrap_or(0))]);
            }
            None => f.extend([V::I(-1), V::Null, V::Null, V::I(0), V::I(0)]),
        }
        f.push(V::S(if rep > 1000 { "High Reputation" } else if (500..=1000).contains(&rep) { "Medium Reputation" } else { "Low Reputation" }));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, u.LastAccessDate, u.Views, CASE WHEN u.Reputation IS NULL THEN 'Unknown' WHEN u.Reputation < 100 THEN 'Novice'
//        WHEN u.Reputation BETWEEN 100 AND 1000 THEN 'Intermediate' ELSE 'Expert' END AS ReputationClass FROM Users u WHERE u.Reputation IS NOT NULL),
// PostHistoryAggregate AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 13 THEN 1 END) AS UndeleteCount FROM PostHistory ph GROUP BY ph.PostId),
// UsersWithTopPosts AS (SELECT ur.UserId, ur.DisplayName, rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, RANK() OVER (PARTITION BY ur.UserId ORDER BY rp.Score DESC) AS TitleRank
//     FROM UserReputation ur JOIN RankedPosts rp ON ur.UserId = rp.OwnerUserId WHERE rp.PostRank <= 5)
// SELECT ur.DisplayName, ur.ReputationClass, utp.PostId, utp.Title, utp.CreationDate, COALESCE(pha.CloseCount, 0) AS TotalClosed, COALESCE(pha.ReopenCount, 0) AS TotalReopened,
//        CASE WHEN utp.CommentCount > 1 THEN 'Multiple Comments' ELSE 'Single Comment or No Comments' END AS CommentStatus
// FROM UserReputation ur LEFT JOIN UsersWithTopPosts utp ON ur.UserId = utp.UserId LEFT JOIN PostHistoryAggregate pha ON utp.PostId = pha.PostId WHERE utp.TitleRank = 1 ORDER BY ur.Reputation DESC, utp.CreationDate DESC;
//
// A tie on CreationDate inside PostRank goes to the larger post id.
fn q21353(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 5, false);
    let best = top_per(top, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let best: MatSet<Id<Post>> = rel(best.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&best).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pha = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    rows(drain((&cc).and(owner_user).and((&pha).opt())).into_iter().map(|(p, ((c, u), h))| {
        let rep = db.user.reputation.get(u).unwrap();
        let h = h.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name"), V::S(if rep < 100 { "Novice" } else if rep <= 1000 { "Intermediate" } else { "Expert" })];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(h[0]), V::I(h[1]), V::S(if c > 1 { "Multiple Comments" } else { "Single Comment or No Comments" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) FILTER (WHERE vt.Name = 'UpMod') AS UpVotes,
//        COUNT(DISTINCT v.Id) FILTER (WHERE vt.Name = 'DownMod') AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(c.Id) DESC) AS Rank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, (SELECT COUNT(b.Id) FROM Badges b WHERE b.UserId = u.Id AND b.Class = 1) AS GoldBadges,
//        (SELECT COUNT(b.Id) FROM Badges b WHERE b.UserId = u.Id AND b.Class = 2) AS SilverBadges, (SELECT COUNT(b.Id) FROM Badges b WHERE b.UserId = u.Id AND b.Class = 3) AS BronzeBadges FROM Users u),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, (rp.UpVotes - rp.DownVotes) AS NetVotes, ur.Reputation, COALESCE(ur.GoldBadges, 0) AS GoldBadges,
//        COALESCE(ur.SilverBadges, 0) AS SilverBadges, COALESCE(ur.BronzeBadges, 0) AS BronzeBadges,
//        CASE WHEN ur.Reputation > 1000 THEN 'Veteran User' WHEN ur.Reputation BETWEEN 500 AND 1000 THEN 'Experienced User' ELSE 'New User' END AS UserCategory
//     FROM RankedPosts rp LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId),
// FinalResults AS (SELECT pd.*, RANK() OVER (ORDER BY pd.NetVotes DESC, pd.CreationDate DESC) AS VoteRank FROM PostDetails pd WHERE pd.CommentCount > 0)
// SELECT PostId, Title, CreationDate, CommentCount, UpVotes, DownVotes, NetVotes, Reputation, GoldBadges, SilverBadges, BronzeBadges, UserCategory, VoteRank FROM FinalResults WHERE VoteRank <= 10
// ORDER BY UserCategory, NetVotes DESC, CreationDate ASC;
//
// Rank is never read.
fn q24329(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).and(votes_of(db).opt())).fold(0i64, |n, _| n + 1);
    let ud = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&cc).and((&ud).opt()));
    let v = ranked(v, |&(p, (_, a))| {
        let a = a.unwrap_or([0, 0]);
        (Reverse(a[0] - a[1]), Reverse(creation_date.get(p).unwrap()))
    }, false);
    let v = drain(rel(v.into_iter().take_while(|x| x.1 <= 10).collect()).select(Same::<((Id<Post>, (i64, Option<[i64; 2]>)), i64)>::new().and(Same::<((Id<Post>, (i64, Option<[i64; 2]>)), i64)>::new().map(|((p, _), _)| p).select(owner_user.select(Ident::<User>::new().and((&ub).opt())).opt()))));
    rows(v.into_iter().map(|(_, (((p, (c, a)), r), u))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        match u {
            Some((u, b)) => {
                let rep = db.user.reputation.get(u).unwrap();
                f.push(V::I(rep));
                f.extend(b.unwrap_or([0; 3]).map(V::I));
                f.push(V::S(if rep > 1000 { "Veteran User" } else if (500..=1000).contains(&rep) { "Experienced User" } else { "New User" }));
            }
            None => f.extend([V::Null, V::I(0), V::I(0), V::I(0), V::S("New User")]),
        }
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.rn = 1),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.GoldBadges, us.SilverBadges, us.BronzeBadges, COALESCE(trp.Title, 'No Posts') AS LatestPostTitle,
//        COALESCE(trp.CreationDate, DATE '1900-01-01') AS LatestPostDate, COALESCE(trp.ViewCount, 0) AS LatestPostViewCount, COALESCE(trp.CommentCount, 0) AS LatestPostCommentCount,
//        COALESCE(trp.UpVotes, 0) AS LatestPostUpVotes, COALESCE(trp.DownVotes, 0) AS LatestPostDownVotes,
//        CASE WHEN us.Reputation > 1000 THEN 'High' WHEN us.Reputation BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'Low' END AS ReputationCategory
// FROM UserStatistics us LEFT JOIN TopRankedPosts trp ON us.UserId = trp.PostId ORDER BY us.Reputation DESC, LatestPostDate DESC;
//
// `us.UserId = trp.PostId` joins a user id to a post id, so it goes through the raw ids. The ownerless questions rank as one partition; a tie on CreationDate
// inside rn goes to the larger post id.
fn q1392(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let trp = (&first).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let pidx: HashIdx<i64, Id<Post>> = (&first).select(&db.post.origid).inv().collect();
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, (_, b)| {
        [a[0] + (b == Some(1)) as i64, a[1] + (b == Some(2)) as i64, a[2] + (b == Some(3)) as i64]
    });
    let tp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&us).and(&tp).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&trp)).opt()));
    rows(v.into_iter().map(|(u, ((b, n), t))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(b.map(V::I));
        match t {
            Some((p, a)) => {
                f.push(V::S(db.post.title.get(p).unwrap_or("No Posts")));
                f.push(V::T(creation_date.get(p).unwrap()));
                f.push(V::I(db.post.view_count.get(p).unwrap_or(0)));
                f.extend(a.map(V::I));
            }
            None => f.extend([V::S("No Posts"), V::T(ts(1900, 1, 1, 0, 0, 0)), V::I(0), V::I(0), V::I(0), V::I(0)]),
        }
        f.push(V::S(if rep > 1000 { "High" } else if (500..=1000).contains(&rep) { "Medium" } else { "Low" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByUser,
//        LEAD(p.Score) OVER (ORDER BY p.CreationDate) AS NextPostScore, SUM(p.Score) OVER () AS TotalScore FROM Posts p WHERE p.CreationDate >= (DATE '2024-10-01' - INTERVAL '1 year')),
// UserReputationHistory AS (SELECT u.Id AS UserId, u.Reputation, u.CreationDate, (CASE WHEN u.Reputation IS NULL THEN 'No Reputation' WHEN u.Reputation < 100 THEN 'Low Reputation' ELSE 'High Reputation' END) AS ReputationCategory
//     FROM Users u WHERE u.CreationDate < DATE '2024-10-01'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.RankByUser, uh.Reputation, uh.ReputationCategory FROM RankedPosts rp
//     LEFT JOIN UserReputationHistory uh ON rp.PostId = uh.UserId WHERE rp.RankByUser = 1 AND rp.ViewCount > 50),
// PostStats AS (SELECT COUNT(*) AS TotalPosts, AVG(ViewCount) AS AverageViews, SUM(Score) AS TotalScore, MAX(Score) AS MaxScore FROM FilteredPosts)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.ViewCount, fp.Score, p.TotalPosts, p.AverageViews, p.TotalScore, p.MaxScore,
//        (CASE WHEN fp.Score >= p.MaxScore * 0.8 THEN 'Top Performer' ELSE 'Needs Improvement' END) AS PerformanceStatus, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = fp.PostId) AS CommentCount
// FROM FilteredPosts fp, PostStats p WHERE fp.ReputationCategory = 'High Reputation' OR (fp.ReputationCategory = 'Low Reputation' AND fp.Score >= (SELECT AVG(Score) FROM FilteredPosts WHERE ReputationCategory = 'Low Reputation'))
// GROUP BY fp.PostId, fp.Title, fp.CreationDate, fp.ViewCount, fp.Score, p.TotalPosts, p.AverageViews, p.TotalScore, p.MaxScore, fp.ReputationCategory ORDER BY fp.ViewCount DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// NextPostScore and TotalScore are never read. `rp.PostId = uh.UserId` joins a post id to a user id, so it goes through the raw ids. The ownerless posts rank as one
// partition; a tie on CreationDate inside RankByUser goes to the larger post id. Each FilteredPosts row is one post, so the GROUP BY keeps them as they are.
// `Score >= MaxScore * 0.8` and `Score >= AVG(Score)` are compared exactly in integers.
fn q20274(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 0, 0, 0);
    let first = top_per(drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = db.user.with((&db.user.creation_date).lt(t0)).select(&db.user.origid).inv().collect();
    let fp = rel(drain((&first).with(view_count.gt(50)).select(Ident::<Post>::new().and((&db.post.origid).select(&uidx).select(&db.user.reputation).opt()))).into_iter().map(|x| x.1).collect());
    type F = (Id<Post>, Option<i64>);
    let st = (&fp).select(Same::<F>::new()).fold_flat([0i64, 0, 0, i64::MIN], |a, (p, _)| [a[0] + 1, a[1] + view_count.get(p).unwrap(), a[2] + score.get(p).unwrap(), a[3].max(score.get(p).unwrap())]);
    let low = (&fp).filt(|(_, r): F| r.map_or(false, |r| r < 100)).select(Same::<F>::new()).fold_flat((0i64, 0i64), |(s, n), (p, _)| (s + score.get(p).unwrap(), n + 1));
    let cc = (&fp).group_by(Same::<F>::new().map(|(p, _): F| p)).select(Same::<F>::new().map(|(p, _): F| p).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&fp).filt(move |(p, r): F| match r {
        Some(r) if r >= 100 => true,
        Some(_) => score.get(p).unwrap() * low.1 >= low.0,
        None => false,
    }).select(Same::<F>::new().and(Same::<F>::new().map(|(p, _): F| p).select(&cc))));
    let v = top_k(v, |&(_, ((p, _), _))| Reverse(view_count.get(p)), |&(_, ((p, _), _))| p, 10);
    rows(v.into_iter().map(|(_, ((p, _), c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(st[0]), avg(st[1], st[0]), V::I(st[2]), V::I(st[3])]);
        f.push(V::S(if s * 10 >= st[3] * 8 { "Top Performer" } else { "Needs Improvement" }));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PopularPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, COALESCE(badge_count.BadgeCount, 0) AS BadgeCount FROM RankedPosts rp
//     LEFT JOIN (SELECT u.Id, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id) badge_count
//     ON badge_count.Id = rp.OwnerUserId WHERE rp.Score > 10 AND rp.CommentCount > 5),
// HighlyLinkedPosts AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS LinkCount FROM PostLinks pl GROUP BY pl.PostId),
// FinalSelection AS (SELECT pp.PostId, pp.Title, pp.Score, pp.ViewCount, hl.LinkCount, pp.BadgeCount, CASE WHEN hl.LinkCount > 10 THEN 'Highly Linked' WHEN pp.BadgeCount > 5 THEN 'Badge Holder' ELSE 'Regular' END AS PostCategory
//     FROM PopularPosts pp LEFT JOIN HighlyLinkedPosts hl ON pp.PostId = hl.PostId)
// SELECT fs.PostId, fs.Title, fs.Score, fs.ViewCount, fs.LinkCount, fs.BadgeCount, fs.PostCategory, ph.CreationDate AS LastEditDate, ph.UserDisplayName AS LastEditor
// FROM FinalSelection fs LEFT JOIN PostHistory ph ON fs.PostId = ph.PostId AND ph.CreationDate = (SELECT MAX(sub_ph.CreationDate) FROM PostHistory sub_ph WHERE sub_ph.PostId = fs.PostId)
// WHERE fs.PostCategory <> 'Regular' ORDER BY fs.Score DESC, fs.ViewCount DESC LIMIT 50;
//
// PostRank is never read. RankedPosts has no GROUP BY, so every post is one row per comment, and each row meets every history row at the latest date.
fn q20734(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let bc = db.badge.with((&db.badge.date).ge(add_years(t0, -1))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let hl = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    let pp = db.post.with(creation_date.ge(add_years(t0, -1)).and(score.gt(10))).with((&cc).filt(|n| n > 5));
    let fs = pp.select(Ident::<Post>::new().and((&hl).opt()).and(owner_user.select(&bc).opt())).filt(|((_, l), b): ((Id<Post>, Option<i64>), Option<i64>)| l.map_or(false, |l| l > 10) || b.unwrap_or(0) > 5);
    let v = drain(fs.select(Same::<((Id<Post>, Option<i64>), Option<i64>)>::new().and(Same::<((Id<Post>, Option<i64>), Option<i64>)>::new().map(|((p, _), _)| p).select(comments_of(db).and(Ident::<Post>::new().and(&md).select(&at).opt())))));
    let v = top_k(v, |&(_, (((p, _), _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, |&(_, (((p, _), _), (c, h)))| (p, c, h), 50);
    rows(v.into_iter().map(|(_, (((p, l), b), (_, h)))| {
        let b = b.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([oint(l), V::I(b), V::S(if l.map_or(false, |l| l > 10) { "Highly Linked" } else { "Badge Holder" })]);
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), ostr(db.post_history.user_display_name.get(h))],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT Users.Id AS UserId, Users.DisplayName, COUNT(DISTINCT Posts.Id) AS PostCount, SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(Comments.Score), 0) AS TotalCommentScore, COALESCE(SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId LEFT JOIN Comments ON Posts.Id = Comments.PostId
//     LEFT JOIN Votes ON Posts.Id = Votes.PostId GROUP BY Users.Id, Users.DisplayName),
// PostStatistics AS (SELECT Posts.Id AS PostId, Posts.Title, Posts.CreationDate, Posts.Score, Posts.ViewCount, COALESCE(Users.DisplayName, 'Community') AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Comments WHERE Comments.PostId = Posts.Id) AS CommentCount, (SELECT COUNT(*) FROM Votes WHERE Votes.PostId = Posts.Id AND Votes.VoteTypeId = 2) AS UpVoteCount,
//        (SELECT COUNT(*) FROM Votes WHERE Votes.PostId = Posts.Id AND Votes.VoteTypeId = 3) AS DownVoteCount FROM Posts LEFT JOIN Users ON Posts.OwnerUserId = Users.Id),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalCommentScore, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStatistics)
// SELECT TopUsers.Rank, TopUsers.DisplayName, TopUsers.PostCount, TopUsers.QuestionCount, TopUsers.AnswerCount, TopUsers.TotalCommentScore, TopUsers.TotalUpVotes, TopUsers.TotalDownVotes,
//        PostStatistics.Title, PostStatistics.Score, PostStatistics.ViewCount, PostStatistics.CommentCount, PostStatistics.UpVoteCount, PostStatistics.DownVoteCount
// FROM TopUsers LEFT JOIN PostStatistics ON TopUsers.DisplayName = PostStatistics.OwnerDisplayName WHERE TopUsers.Rank <= 10 ORDER BY TopUsers.Rank;
//
// Rank reads only COUNT(DISTINCT Posts.Id), a plain count, so the top ten posters are picked first and the posts x comments x votes product is folded for them alone.
// PostStatistics names an ownerless post 'Community', so the join is on that name.
fn q13487(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rk = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let rk = rel(rk.into_iter().take_while(|x| x.1 <= 10).map(|((u, n), r)| (u, n, r)).collect());
    let top: MatSet<Id<User>> = (&rk).map(|(u, _, _)| u).collect();
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(comments_of(db).select(&db.comment.score).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some(((t, c), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.unwrap_or(0), a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, 0],
            None => a,
        });
    let owner_name = owner_user.select(&db.user.display_name).opt().map(|n: Option<Str>| n.unwrap_or("Community"));
    let by_name: HashIdx<Str, Id<Post>> = db.post.select(owner_name).inv().collect();
    let ps = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type R = (Id<User>, i64, i64);
    let v = drain((&rk).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _, _): R| u).select(&us)).and(Same::<R>::new().map(|(u, _, _): R| u).select(&db.user.display_name).select((&by_name).select(Ident::<Post>::new().and(&ps).and(&pv))).opt())));
    rows(v.into_iter().map(|(_, (((u, n, r), a), p))| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])];
        match p {
            Some(((p, c), b)) => {
                f.extend(post_fields(db, p, &["title", "score", "views"]));
                f.extend([V::I(c), V::I(b[0]), V::I(b[1])]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS Upvotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS Downvotes,
//        (COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) - COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3)) AS NetScore, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosedDate, COUNT(*) AS CloseCount, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE NULL END) AS ClosedByUser FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT us.DisplayName, us.Reputation, us.BadgeCount, pd.Title, pd.CommentCount, pd.Upvotes, pd.Downvotes, pd.NetScore, cp.LastClosedDate, COALESCE(cp.CloseCount, 0) AS CloseCount,
//        (CASE WHEN cp.ClosedByUser IS NOT NULL THEN 'Closed' ELSE 'Active' END) AS PostStatus, CASE WHEN pd.NetScore > 0 THEN 'Positive' WHEN pd.NetScore < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory
// FROM UserStats us JOIN Posts p ON us.UserId = p.OwnerUserId LEFT JOIN PostDetails pd ON p.Id = pd.PostId LEFT JOIN ClosedPosts cp ON p.Id = cp.PostId
// WHERE us.Reputation > 1000 AND pd.CommentCount > 0 AND pd.PostRank = 1 ORDER BY us.Reputation DESC, pd.NetScore DESC LIMIT 100;
//
// PostRank reads only CreationDate, so each owner's newest post is picked first and the comments x votes product is folded for those alone. A tie on CreationDate
// inside PostRank goes to the larger post id.
fn q21085(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let first = top_per(drain(db.post.select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pd = (&first).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(post_history_type_id.and(hd)).fold((i64::MIN, 0i64, false), |(m, n, c), (t, d)| (m.max(d), n + 1, c || t == 10));
    let v = drain((&pd).filt(|a| a[0] > 0).and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and(&bc))).and((&cp).opt()));
    let v = top_k(v, |&(_, ((a, (u, _)), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1] - a[2])), |&(p, _)| p, 100);
    rows(v.into_iter().map(|(p, ((a, (u, b)), c))| {
        let n = a[1] - a[2];
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n)]);
        f.extend(match c {
            Some((m, k, x)) => [V::T(m), V::I(k), V::S(if x { "Closed" } else { "Active" })],
            None => [V::Null, V::I(0), V::S("Active")],
        });
        f.push(V::S(if n > 0 { "Positive" } else if n < 0 { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankByScore
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId),
// ClosedPosts AS (SELECT p.Id AS PostId, ph.Comment AS CloseReason, ph.CreationDate AS ClosedDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, SUM(v.BountyAmount) AS TotalBounty FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName)
// SELECT p.PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, rb.BadgeCount, rb.GoldBadges, rb.SilverBadges, rb.BronzeBadges, ph.EditCount, ph.LastEditDate, c.CloseReason, c.ClosedDate,
//        au.DisplayName AS ActiveUser, au.PostsCount, au.TotalBounty
// FROM RankedPosts p LEFT JOIN UserBadges rb ON p.OwnerUserId = rb.UserId LEFT JOIN PostHistoryCounts ph ON p.PostId = ph.PostId LEFT JOIN ClosedPosts c ON p.PostId = c.PostId
// LEFT JOIN MostActiveUsers au ON p.OwnerUserId = au.UserId WHERE p.RankByScore <= 5 ORDER BY p.Score DESC, p.CreationDate DESC;
fn q30444(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let phc = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)));
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let au = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(bv.opt())).fold((0i64, 0i64), |(n, s), b| match b.flatten() {
        Some(b) => (n + 1, s + b),
        None => (n, s),
    });
    let apc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&rb).opt()).and((&phc).opt()).and(closes.opt()).and(owner_user.select(Ident::<User>::new().and(&apc).and(&au)).opt())));
    rows(v.into_iter().map(|(_, ((((p, b), e), c), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match e {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        f.extend(match c {
            Some(h) => [ostr(comment.get(h)), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        f.extend(match a {
            Some(((u, n), (k, s))) => [user_col(db, u, "name"), V::I(n), nullable(s, k)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RECURSIVE TopUsers AS (SELECT Id, DisplayName, Reputation, CreationDate, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Ranking FROM Users),
// UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, MAX(B.Class) AS HighestBadgeClass, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.Score, COALESCE(COUNT(C.Id), 0) AS CommentCount, ROW_NUMBER() OVER (ORDER BY P.Score DESC) AS PopularityRank FROM Posts P
//     LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 MONTH' GROUP BY P.Id, P.Title, P.OwnerUserId, P.Score),
// UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostsCreated, SUM(COALESCE(P.Score, 0)) AS TotalScore, AVG(P.Score) AS AveragePostScore, SUM(COALESCE(UPV.VoteCount, 0)) AS TotalUpVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) UPV ON P.Id = UPV.PostId GROUP BY U.Id, U.DisplayName)
// SELECT TU.DisplayName, TU.Reputation, UB.TotalBadges, UB.HighestBadgeClass, COALESCE(UP.PostsCreated, 0) AS PostsCount, COALESCE(UP.TotalScore, 0) AS TotalPostScore, COALESCE(UP.AveragePostScore, 0) AS AvgPostScore,
//        COALESCE(UP.TotalUpVotes, 0) AS TotalUpVotes, PP.Title AS PopularPostTitle, PP.Score as PopularPostScore
// FROM TopUsers TU LEFT JOIN UserBadges UB ON TU.Id = UB.UserId LEFT JOIN UserPostStats UP ON TU.Id = UP.UserId LEFT JOIN PopularPosts PP ON TU.Id = PP.OwnerUserId AND PP.PopularityRank <= 5
// WHERE TU.Ranking <= 10 ORDER BY TU.Reputation DESC, UB.TotalBadges DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. Ties inside the two ROW_NUMBERs go to the smaller id.
fn q34313(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, i64::MIN), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let upv = db.vote.with((&db.vote.vote_type_id).eq(2)).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let up = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(score.and((&upv).opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((s, v)) => [a[0] + 1, a[1] + s, a[2] + v.unwrap_or(0)],
        None => a,
    });
    let pp = top_n(drain(db.post.with(creation_date.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(p, s)| (Reverse(s), p), 5);
    let pp: MatSet<Id<Post>> = rel(pp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_owner: HashIdx<Id<User>, Id<Post>> = (&pp).select(&db.post.owner_user).inv().collect();
    let v = drain((&tu).select(Ident::<User>::new().and(&ub).and(&up).and((&by_owner).opt())));
    rows(v.into_iter().map(|(_, (((u, (n, m)), a), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), omax(m, n), V::I(a[0]), V::I(a[1]), if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }, V::I(a[2])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputationCTE AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (PARTITION BY CASE WHEN u.Reputation > 1000 THEN 'High' WHEN u.Reputation > 500 THEN 'Medium' ELSE 'Low' END
//        ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation IS NOT NULL),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(NULLIF(p.Score, 0), NULL) AS PostScore, u.DisplayName AS OwnerDisplayName, COALESCE(ah.AcceptedAnswerId, 0) AS AcceptedAnswerId,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Posts ah ON p.AcceptedAnswerId = ah.Id
//     GROUP BY p.Id, p.Title, p.ViewCount, p.Score, u.DisplayName, ah.AcceptedAnswerId),
// ReputationStatus AS (SELECT ur.UserId, ur.DisplayName, CASE WHEN ur.Reputation > (SELECT AVG(Reputation) FROM Users) THEN 'Above Average' ELSE 'Below Average' END AS ReputationGroup FROM UserReputationCTE ur)
// SELECT pd.PostId, pd.Title, pd.ViewCount, pd.PostScore, pd.OwnerDisplayName, rs.ReputationGroup, pd.CommentCount, pd.UpVotes, pd.DownVotes, COUNT(pl.RelatedPostId) AS RelatedPostsCount
// FROM PostDetails pd LEFT JOIN PostLinks pl ON pd.PostId = pl.PostId JOIN ReputationStatus rs ON pd.OwnerDisplayName = rs.DisplayName
// WHERE pd.ViewCount > 50 AND (pd.PostScore < 5 OR pd.PostScore IS NULL)
// GROUP BY pd.PostId, pd.Title, pd.ViewCount, pd.PostScore, pd.OwnerDisplayName, rs.ReputationGroup, pd.CommentCount, pd.UpVotes, pd.DownVotes HAVING COUNT(pl.RelatedPostId) > 0
// ORDER BY pd.ViewCount DESC, pd.PostScore DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
//
// ReputationRank is never read. The join to ReputationStatus is on DisplayName, so the (post, link, same-name user) rows are materialised and grouped by
// (post, reputation group); the comments x votes product is folded for the posts that reach the final rows.
fn q24705(db: &'static So) -> String {
    let User { reputation, display_name, .. } = &db.user;
    let Post { owner_user, view_count, score, .. } = &db.post;
    let (sum, n) = db.user.select(reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let by_name: HashIdx<Str, Id<User>> = (display_name).inv().collect();
    let cand = || db.post.with(view_count.gt(50)).with(score.lt(5)).with(owner_user);
    let j: MatSet<((Id<Post>, Id<PostLink>), Id<User>)> = cand().select(Ident::<Post>::new().and(links_of(db)).and(owner_user.select(display_name).select(&by_name))).collect();
    type J = ((Id<Post>, Id<PostLink>), Id<User>);
    let g = (&j)
        .group_by(Same::<J>::new().map(move |((p, _), u): J| (p, reputation.get(u).unwrap() * n > sum)))
        .select(Same::<J>::new())
        .fold(0i64, |k, _| k + 1);
    let pd = cand().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain(rel(drain(&g)).select(Same::<((Id<Post>, bool), i64)>::new().and(Same::<((Id<Post>, bool), i64)>::new().map(|((p, _), _)| p).select(&pd))));
    let ps = |p: Id<Post>| Some(score.get(p).unwrap()).filter(|&s| s != 0);
    let v = top_k(v, |&(_, (((p, _), _), _))| (Reverse(view_count.get(p)), ps(p).is_none(), Reverse(ps(p))), |&(_, (((p, a), _), _))| (p, a), 20);
    rows(v.into_iter().skip(10).map(|(_, (((p, a), k), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.push(oint(ps(p)));
        f.extend(post_fields(db, p, &["owner"]));
        f.push(V::S(if a { "Above Average" } else { "Below Average" }));
        f.extend(c.map(V::I));
        f.push(V::I(k));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= '2023-01-01'),
// LatestAnswers AS (SELECT a.Id AS AnswerId, a.Score AS AnswerScore, a.ParentId AS QuestionId, p.Title AS QuestionTitle FROM Posts a JOIN Posts p ON a.ParentId = p.Id
//     WHERE a.PostTypeId = 2 AND a.CreationDate = (SELECT MAX(CreationDate) FROM Posts WHERE ParentId = a.ParentId)),
// CloseReasons AS (SELECT ph.PostId, ph.Comment, ph.CreationDate, crt.Name AS CloseReason FROM PostHistory ph JOIN CloseReasonTypes crt ON CAST(ph.Comment AS INT) = crt.Id WHERE ph.PostHistoryTypeId = 10),
// UserBadges AS (SELECT u.Id AS UserId, b.Name AS BadgeName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, b.Name),
// PostScoreAnalysis AS (SELECT p.Id AS PostId, (COALESCE(p.Score, 0) + COALESCE(a.AnswerCount, 0)) AS TotalScore, (p.ViewCount * COALESCE(b.BadgeCount, 1)) AS AdjustedViews FROM Posts p
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId LEFT JOIN UserBadges b ON p.OwnerUserId = b.UserId WHERE p.PostTypeId = 1)
// SELECT rp.Title AS LatestQuestionTitle, rp.CreationDate AS LatestQuestionDate, la.AnswerId, la.QuestionTitle, la.AnswerScore, CASE WHEN cr.PostId IS NOT NULL THEN cr.CloseReason ELSE 'Open' END AS QuestionStatus,
//        psa.TotalScore AS QuestionScore, psa.AdjustedViews
// FROM RankedPosts rp LEFT JOIN LatestAnswers la ON rp.PostId = la.QuestionId LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId LEFT JOIN PostScoreAnalysis psa ON rp.PostId = psa.PostId
// WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC, la.AnswerScore DESC;
//
// UserBadges has one row per (user, badge name), and one all-NULL-name row with count 0 for a user without badges, so each question meets every group of its
// owner's badges. The ownerless questions rank as one partition; a tie on CreationDate inside rn goes to the larger post id.
fn q22887(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)))).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let maxc = db.post.group_by(Ident::<Post>::new()).select(children_of(db).select(creation_date)).fold(i64::MIN, |m, d| m.max(d));
    let ans_at: HashIdx<(Id<Post>, i64), Id<Post>> = db.post.with(post_type_id.eq(2)).select((&db.post.parent).and(creation_date)).inv().collect();
    let la = Ident::<Post>::new().and(&maxc).select(&ans_at);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cr = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason));
    let ac = db.post.with(post_type_id.eq(2)).group_by(&db.post.parent).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let ubg = db.badge.group_by((&db.badge.user).and(&db.badge.name)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ubv = rel(drain(&ubg));
    let ub_by_user: HashIdx<Id<User>, ((Id<User>, Str), i64)> = (&ubv).map(|((u, _), _)| u).inv().select(&ubv).collect();
    let psa = owner_user.select((&ub_by_user).map(|(_, n)| n).opt()).opt();
    let v = drain((&first).select(Ident::<Post>::new().and(la.opt()).and(cr.opt()).and((&ac).opt()).and(psa)));
    rows(v.into_iter().map(|(_, ((((p, a), c), n), b))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        match a {
            Some(a) => {
                f.extend(post_fields(db, a, &["id"]));
                f.extend(post_fields(db, p, &["title"]));
                f.push(V::I(score.get(a).unwrap()));
            }
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.push(V::S(c.unwrap_or("Open")));
        f.push(V::I(score.get(p).unwrap() + n.unwrap_or(0)));
        let k = match b {
            None => 1,
            Some(None) => 0,
            Some(Some(k)) => k,
        };
        f.push(oint(view_count.get(p).map(|w| w * k)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, u.DisplayName AS OwnerName, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore FROM Posts p
//     JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= '2020-01-01' AND p.Score IS NOT NULL),
// EnhancedPosts AS (SELECT PostId, Title, ViewCount, Score, OwnerName, RankScore, CASE WHEN RankScore <= 10 THEN 'Top' WHEN RankScore BETWEEN 11 AND 100 THEN 'Intermediate' ELSE 'Low' END AS PerformanceCategory
//     FROM RankedPosts),
// PostVoteCounts AS (SELECT PostId, COUNT(*) FILTER (WHERE VoteTypeId = 2) AS Upvotes, COUNT(*) FILTER (WHERE VoteTypeId = 3) AS Downvotes FROM Votes GROUP BY PostId),
// PostHistoryData AS (SELECT ph.PostId, MIN(CASE WHEN ph.PostHistoryTypeId = 1 THEN ph.CreationDate END) AS InitialTitleDate, MIN(CASE WHEN ph.PostHistoryTypeId = 2 THEN ph.CreationDate END) AS InitialBodyDate,
//        MIN(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT ep.PostId, ep.Title, ep.ViewCount, ep.Score, ep.OwnerName, ep.PerformanceCategory, COALESCE(pvc.Upvotes, 0) AS TotalUpvotes, COALESCE(pvc.Downvotes, 0) AS TotalDownvotes, ph.InitialTitleDate,
//        ph.InitialBodyDate, ph.ClosedDate, CASE WHEN ph.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus,
//        CASE WHEN ep.PerformanceCategory = 'Top' AND ph.ClosedDate IS NOT NULL THEN 'Highly active post that got closed. Review required.'
//             WHEN ep.PerformanceCategory = 'Low' AND ph.ClosedDate IS NULL THEN 'Needs attention for improvement.' ELSE 'Regular post.' END AS StatusMessage
// FROM EnhancedPosts ep LEFT JOIN PostVoteCounts pvc ON ep.PostId = pvc.PostId LEFT JOIN PostHistoryData ph ON ep.PostId = ph.PostId WHERE ep.RankScore <= 100 ORDER BY ep.Score DESC NULLS LAST, ep.ViewCount ASC;
fn q23723(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let rk = ranked(drain(db.post.with(creation_date.ge(ts(2020, 1, 1, 0, 0, 0))).with(owner_user).select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap())), false);
    let rk = rel(per_group(rk, |&(_, t)| t).into_iter().filter(|x| x.1 <= 100).map(|((p, _), r)| (p, r)).collect());
    let pvc = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([i64::MAX; 3], |a, (t, d)| {
        [if t == 1 { a[0].min(d) } else { a[0] }, if t == 2 { a[1].min(d) } else { a[1] }, if t == 10 { a[2].min(d) } else { a[2] }]
    });
    type R = (Id<Post>, i64);
    let v = drain((&rk).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&pvc).opt().and((&phd).opt())))));
    rows(v.into_iter().map(|(_, ((p, r), (a, h)))| {
        let a = a.unwrap_or([0, 0]);
        let h = h.unwrap_or([i64::MAX; 3]);
        let cat = if r <= 10 { "Top" } else if r <= 100 { "Intermediate" } else { "Low" };
        let closed = h[2] != i64::MAX;
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        f.extend([V::S(cat), V::I(a[0]), V::I(a[1]), tmin(h[0]), tmin(h[1]), tmin(h[2]), V::S(if closed { "Closed" } else { "Open" })]);
        f.push(V::S(if cat == "Top" && closed { "Highly active post that got closed. Review required." } else if cat == "Low" && !closed { "Needs attention for improvement." } else { "Regular post." }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(COALESCE(vs.VoteScore, 0)) AS TotalVoteScore, SUM(COALESCE(b.Id, 0)) AS TotalBadges FROM Users u
//     LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS VoteScore FROM Votes GROUP BY PostId) vs ON p.Id = vs.PostId
//     LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostHistoryAnalysis AS (SELECT ph.UserId, ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount, COUNT(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteUndeleteCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (24) THEN 1 END) AS SuggestedEditsApplied FROM PostHistory ph GROUP BY ph.UserId, ph.PostId),
// AggregateData AS (SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalVoteScore, ups.TotalBadges, COALESCE(SUM(pha.CloseReopenCount), 0) AS TotalCloseReopen,
//        COALESCE(SUM(pha.DeleteUndeleteCount), 0) AS TotalDeleteUndelete, COALESCE(SUM(pha.SuggestedEditsApplied), 0) AS TotalSuggestedEdit, ROW_NUMBER() OVER (ORDER BY ups.TotalVoteScore DESC) AS Rank
//     FROM UserPostStats ups LEFT JOIN PostHistoryAnalysis pha ON ups.UserId = pha.UserId GROUP BY ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalVoteScore, ups.TotalBadges)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalVoteScore, TotalBadges, TotalCloseReopen, TotalDeleteUndelete, TotalSuggestedEdit, Rank FROM AggregateData
// WHERE TotalVoteScore > 0 ORDER BY TotalVoteScore DESC, TotalPosts DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
//
// A tie inside the ROW_NUMBER goes to the smaller user id.
fn q23152(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |s, t| s + match t {
        2 => 1,
        3 => -1,
        _ => 0,
    });
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&vs).opt()).opt().and(badges_of(db).select(&db.badge.origid).opt())).fold([0i64; 2], |a, (s, b)| {
        [a[0] + s.flatten().unwrap_or(0), a[1] + b.unwrap_or(0)]
    });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let PostHistory { user, post, post_history_type_id, .. } = &db.post_history;
    let pha = db.post_history.group_by(user.and(post)).select(post_history_type_id).fold([0i64; 3], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + matches!(t, 12 | 13) as i64, a[2] + (t == 24) as i64]);
    let phv = rel(drain(&pha));
    type H = ((Id<User>, Id<Post>), [i64; 3]);
    let ph_user = (&phv).group_by(Same::<H>::new().map(|((u, _), _): H| u)).select(Same::<H>::new().map(|(_, a): H| a)).fold([0i64; 3], |s, a| [s[0] + a[0], s[1] + a[1], s[2] + a[2]]);
    let v = drain((&ups).and(&pc).and((&ph_user).opt()));
    let v = ranked(v, |&(u, ((a, _), _))| (Reverse(a[0]), u), false);
    let v = drain(rel(v).filt(|((_, ((a, _), _)), _)| a[0] > 0));
    let v = top_k(v, |&(_, ((_, ((a, p), _)), _))| (Reverse(a[0]), Reverse(p[0])), |&(_, ((u, _), _))| u, 20);
    rows(v.into_iter().skip(10).map(|(_, ((u, ((a, p), h)), r))| {
        let h = h.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(a[0]), V::I(a[1]), V::I(h[0]), V::I(h[1]), V::I(h[2]), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Date) AS LastBadgeDate FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, BadgeCount, LastBadgeDate, RANK() OVER (ORDER BY BadgeCount DESC) AS UserRank FROM UserBadges WHERE BadgeCount > 5),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, AVG(p.ViewCount) AS AverageViews, MAX(p.LastActivityDate) AS LastActivity
//     FROM Posts p GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, p.TotalPosts, p.QuestionCount, p.AverageViews, COALESCE(p.LastActivity, u.CreationDate) AS LastActivityDate, b.BadgeCount AS BadgeCount
//     FROM Users u LEFT JOIN PostStats p ON u.Id = p.OwnerUserId LEFT JOIN UserBadges b ON u.Id = b.UserId WHERE COALESCE(p.TotalPosts, 0) > 0 OR b.BadgeCount > 0),
// HighlyActiveUsers AS (SELECT ua.UserId, ua.DisplayName, ua.BadgeCount, ua.LastActivityDate, DENSE_RANK() OVER (ORDER BY ua.LastActivityDate DESC) AS ActivityRank FROM UserActivity ua WHERE ua.AverageViews > 15),
// ClosedPosts AS (SELECT p.OwnerUserId, COUNT(h.Id) AS ClosedCount FROM Posts p INNER JOIN PostHistory h ON p.Id = h.PostId AND h.PostHistoryTypeId = 10 GROUP BY p.OwnerUserId)
// SELECT ah.UserId, ah.DisplayName, ah.BadgeCount, ah.LastActivityDate, COALESCE(cp.ClosedCount, 0) AS ClosedPostsCount, CASE WHEN ah.ActivityRank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM HighlyActiveUsers ah LEFT JOIN ClosedPosts cp ON ah.UserId = cp.OwnerUserId WHERE ah.LastActivityDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY ah.BadgeCount DESC, ah.LastActivityDate DESC LIMIT 20;
//
// TopUsers is never referenced. `AverageViews > 15` is compared exactly, as sum > 15 * n.
fn q20137(db: &'static So) -> String {
    let Post { owner_user, view_count, last_activity_date, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(owner_user).select(view_count.opt().and(last_activity_date)).fold((0i64, 0i64, i64::MIN), |(n, s, m), (w, d)| (n + w.is_some() as i64, s + w.unwrap_or(0), m.max(d)));
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post.select(owner_user)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&ps).filt(|(n, s, _)| n > 0 && s > 15 * n).and(&ub));
    let v = ranked(v, |&(_, ((_, _, m), _))| Reverse(m), true);
    type R = ((Id<User>, ((i64, i64, i64), i64)), i64);
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let v = drain(rel(v).filt(move |((_, ((_, _, m), _)), _): R| m >= since).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| u).select((&cp).opt()))));
    let v = top_k(v, |&(_, (((_, ((_, _, m), b)), _), _))| (Reverse(b), Reverse(m)), |&(_, (((u, _), _), _))| u, 20);
    rows(v.into_iter().map(|(_, (((u, ((_, _, m), b)), r), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::T(m), V::I(c.unwrap_or(0)), V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" })]);
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(b.Id) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostSummary AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount,
//        SUM(COALESCE(p.Score, 0)) AS TotalScore, MAX(p.CreationDate) AS LastPostDate FROM Posts p GROUP BY p.OwnerUserId),
// VoteCounts AS (SELECT v.UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT v.PostId) AS VotedPosts
//     FROM Votes v GROUP BY v.UserId)
// SELECT u.Id, u.DisplayName, COALESCE(ubs.GoldBadges, 0) AS GoldBadges, COALESCE(ubs.SilverBadges, 0) AS SilverBadges, COALESCE(ubs.BronzeBadges, 0) AS BronzeBadges, COALESCE(ps.TotalPosts, 0) AS TotalPosts,
//        COALESCE(ps.QuestionCount, 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount, COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(vcs.UpVotes, 0) AS UpVotes, COALESCE(vcs.DownVotes, 0) AS DownVotes,
//        COALESCE(vcs.VotedPosts, 0) AS VotedPosts,
//        CASE WHEN COALESCE(ubs.TotalBadges, 0) > 0 THEN 'Active User' WHEN COALESCE(ps.TotalPosts, 0) = 0 AND COALESCE(vcs.VotedPosts, 0) = 0 THEN 'Inactive User' ELSE 'Moderate User' END AS UserStatus,
//        LEAD(u.CreationDate) OVER (ORDER BY u.CreationDate) AS NextUserCreationDate, ROW_NUMBER() OVER (ORDER BY COALESCE(ps.TotalScore, 0) DESC, u.Reputation DESC) AS Rank
// FROM Users u LEFT JOIN UserBadgeStats ubs ON u.Id = ubs.UserId LEFT JOIN PostSummary ps ON u.Id = ps.OwnerUserId LEFT JOIN VoteCounts vcs ON u.Id = vcs.UserId
// WHERE u.Reputation > 1000 AND (COALESCE(ubs.TotalBadges, 0) > 2 OR COALESCE(ps.TotalPosts, 0) > 5) ORDER BY Rank, u.DisplayName OFFSET 10 ROWS FETCH NEXT 5 ROWS ONLY;
//
// Both windows run over the users the WHERE keeps. Ties in CreationDate (LEAD) and in the ROW_NUMBER go to the smaller user id.
fn q23496(db: &'static So) -> String {
    let ubs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64, a[3] + 1],
        None => a,
    });
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let Vote { user, vote_type_id, post_id, .. } = &db.vote;
    let vc = db.vote.group_by(user).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let vp = db.vote.group_by(user).select(post_id).count_distinct();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ubs).and((&ps).opt()).and((&vc).opt()).and((&vp).opt())).filt(
        |(((b, p), _), _): ((([i64; 4], Option<[i64; 4]>), Option<[i64; 2]>), Option<i64>)| b[3] > 2 || p.map_or(0, |p| p[0]) > 5,
    ));
    let uc = |u: Id<User>| db.user.creation_date.get(u).unwrap();
    let v = top_n(v, |&(u, _)| (uc(u), u), 0);
    let nx: Vec<Option<i64>> = (0..v.len()).map(|i| v.get(i + 1).map(|x| uc(x.0))).collect();
    let v: Vec<_> = v.into_iter().zip(nx).collect();
    let v = ranked(v, |&((u, (((_, p), _), _)), _)| (Reverse(p.map_or(0, |p| p[3])), Reverse(db.user.reputation.get(u).unwrap()), u), false);
    let v = top_k(v, |&((_, _), r)| r, |&(((u, _), _), _)| db.user.display_name.get(u).unwrap(), 15);
    rows(v.into_iter().skip(10).map(|(((u, (((b, p), c), k)), nx), r)| {
        let p = p.unwrap_or([0; 4]);
        let c = c.unwrap_or([0; 2]);
        let k = k.unwrap_or(0);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), V::I(c[0]), V::I(c[1]), V::I(k)]);
        f.push(V::S(if b[3] > 0 { "Active User" } else if p[0] == 0 && k == 0 { "Inactive User" } else { "Moderate User" }));
        f.extend([ots(nx), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionsCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty FROM Users u
//     LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, ur.QuestionsCount, ur.TotalBounty, DENSE_RANK() OVER (ORDER BY ur.Reputation DESC) AS ReputationRank FROM UserReputation ur WHERE ur.QuestionsCount > 5),
// OpenQuestions AS (SELECT p.Id, p.Title, p.CreationDate, c.UserDisplayName AS LastCommenter, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY c.CreationDate DESC) AS LatestCommentRank FROM Posts p
//     LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 AND p.ClosedDate IS NULL),
// FinalResults AS (SELECT tp.UserId, tp.DisplayName, tp.Reputation, tp.QuestionsCount, tp.TotalBounty, rq.PostId, rq.Title, rq.CreationDate, rq.Score, rq.ViewCount, oq.LastCommenter
//     FROM TopUsers tp JOIN RankedPosts rq ON tp.UserId = rq.OwnerUserId LEFT JOIN OpenQuestions oq ON rq.PostId = oq.Id WHERE tp.ReputationRank <= 10)
// SELECT fr.UserId, fr.DisplayName, fr.Reputation, fr.QuestionsCount, fr.TotalBounty, fr.PostId, fr.Title AS PostTitle, fr.CreationDate, fr.Score, fr.ViewCount, COALESCE(fr.LastCommenter, 'No comments yet') AS LastCommenter
// FROM FinalResults fr ORDER BY fr.Reputation DESC, fr.CreationDate DESC;
//
// PostRank and LatestCommentRank are never filtered, so every question of a top user meets every comment of it.
fn q33929(db: &'static So) -> String {
    let Post { post_type_id, closed_date, .. } = &db.post;
    let asked = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ur = db.user.with((&db.user.reputation).gt(0)).group_by(Ident::<User>::new()).select(asked().select(bv.opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let qc = db.user.group_by(Ident::<User>::new()).select(asked()).fold(0i64, |n, _| n + 1);
    let tu = ranked(drain((&ur).and((&qc).filt(|n| n > 5))), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), true);
    let tu: MatSet<(Id<User>, (i64, i64))> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect()).map(|x| x).collect();
    let open = Ident::<Post>::new().minus(closed_date);
    let oq = open.select(comments_of(db).select((&db.comment.user_display_name).opt()).opt());
    type T = (Id<User>, (i64, i64));
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(asked().select(Ident::<Post>::new().and(oq.opt()))))));
    rows(v.into_iter().map(|(_, ((u, (b, n)), (p, c)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(b)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.push(V::S(c.flatten().flatten().unwrap_or("No comments yet")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteAnalytics AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment AS CloseReason, ph.UserId AS CloserId FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// FinalPostReport AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.OwnerUserId AS PostOwner, us.GoldBadges, us.SilverBadges, us.BronzeBadges, pva.UpVotes, pva.DownVotes, cp.CloseReason
//     FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId LEFT JOIN PostVoteAnalytics pva ON rp.PostId = pva.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.UserPostRank <= 5)
// SELECT PostId, Title, Score, PostOwner, GoldBadges, SilverBadges, BronzeBadges, UpVotes, DownVotes, CASE WHEN CloseReason IS NOT NULL THEN 'Closed: ' || CloseReason ELSE 'Active' END AS Status
// FROM FinalPostReport ORDER BY Score DESC, CreationDate DESC LIMIT 100;
//
// A tie on CreationDate inside UserPostRank goes to the larger post id.
fn q24947(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let pva = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))).select(comment.opt());
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&us)).and((&pva).opt()).and(cp.opt())));
    let v = top_k(v, |&(_, (((p, _), _), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), |&(_, (((p, _), _), c))| (p, c), 100);
    rows(v.into_iter().map(|(_, (((p, b), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner_id"]);
        f.extend(b.map(V::I));
        match a {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1])]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(match c.flatten() {
            Some(r) => V::Owned(format!("Closed: {r}")),
            None => V::S("Active"),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id),
// FilteredPosts AS (SELECT rp.*, ph.PostHistoryTypeId, ph.CreationDate AS HistoryCreationDate, ROW_NUMBER() OVER (PARTITION BY rp.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank FROM RankedPosts rp
//     LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, ViewCount, Score FROM FilteredPosts WHERE ScoreRank <= 10 AND UserPostRank = 1),
// PostStats AS (SELECT tp.PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId),
// FinalOutput AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.ViewCount, tp.Score, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount,
//        CASE WHEN ps.UpvoteCount IS NULL THEN 'No Upvotes' WHEN ps.UpvoteCount = 0 THEN 'Zero Upvotes' ELSE 'Has Upvotes' END AS UpvoteStatus,
//        CASE WHEN ps.DownvoteCount IS NULL THEN 'No Downvotes' WHEN ps.DownvoteCount = 0 THEN 'Zero Downvotes' ELSE 'Has Downvotes' END AS DownvoteStatus FROM TopPosts tp LEFT JOIN PostStats ps ON tp.PostId = ps.PostId)
// SELECT PostId, Title, OwnerDisplayName, ViewCount, Score, CommentCount, UpvoteCount, DownvoteCount, UpvoteStatus, DownvoteStatus FROM FinalOutput WHERE Score > 0 ORDER BY Score DESC, CommentCount DESC LIMIT 50;
//
// TopPosts keeps one row per close/reopen/delete history row of each post, so PostStats folds (history rows x comments x votes) and FinalOutput repeats each post
// once per history row. The ownerless posts number as one partition; a tie on CreationDate inside UserPostRank goes to the larger post id.
fn q23549(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let rk = ranked(drain(db.post.select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap())), false);
    let top10: MatSet<Id<Post>> = rel(per_group(rk, |&(_, t)| t).into_iter().filter(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let first = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let hist = || history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11, 12])));
    let tp = || (&top10).with(&first).with(score.gt(0));
    let ps = tp().group_by(Ident::<Post>::new()).select(hist().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, ((_, c), t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain(tp().select(hist().and(&ps)));
    let v = top_k(v, |&(p, (_, a))| (Reverse(score.get(p).unwrap()), Reverse(a[0])), |&(p, (h, _))| (p, h), 50);
    rows(v.into_iter().map(|(p, (_, a))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend(post_fields(db, p, &["views", "score"]));
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] == 0 { "Zero Upvotes" } else { "Has Upvotes" }));
        f.push(V::S(if a[2] == 0 { "Zero Downvotes" } else { "Has Downvotes" }));
        row(f)
    }))
}

// WITH RecursivePostStats AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Score, P.CreationDate, P.Title, P.ViewCount, COUNT(A.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS UserPostRank FROM Posts P LEFT JOIN Posts A ON P.Id = A.ParentId WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.OwnerUserId, P.Score, P.CreationDate, P.Title, P.ViewCount),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostClosureStats AS (SELECT PostId, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount FROM PostHistory PH GROUP BY PostId),
// HighPerformanceAnalytics AS (SELECT UPS.PostId, U.DisplayName AS OwnerName, UPS.Score AS PostScore, UPS.ViewCount, CASE WHEN U.Location IS NOT NULL THEN U.Location ELSE 'Location not specified' END AS UserLocation,
//        COALESCE(UB.TotalBadges, 0) AS UserBadges, COALESCE(PCS.CloseCount, 0) AS CloseCount, COALESCE(PCS.ReopenCount, 0) AS ReopenCount, UPS.Title, UPS.CreationDate
//     FROM RecursivePostStats UPS JOIN Users U ON UPS.OwnerUserId = U.Id LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostClosureStats PCS ON UPS.PostId = PCS.PostId WHERE UPS.UserPostRank <= 5)
// SELECT HPA.OwnerName, HPA.Title, HPA.PostScore, HPA.ViewCount, HPA.UserLocation, HPA.UserBadges, HPA.CloseCount, HPA.ReopenCount FROM HighPerformanceAnalytics HPA ORDER BY HPA.PostScore DESC, HPA.ViewCount DESC LIMIT 10;
//
// AnswerCount is never read. A tie on CreationDate inside UserPostRank goes to the larger post id.
fn q33680(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(p)), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pcs = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ub))).and((&pcs).opt())));
    let v = top_k(v, |&(_, ((p, _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, |&(_, ((p, _), _))| p, 10);
    rows(v.into_iter().map(|(_, ((p, (u, b)), c))| {
        let c = c.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(V::S(db.user.location.get(u).unwrap_or("Location not specified")));
        f.extend([V::I(b), V::I(c[0]), V::I(c[1])]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("23058", q23058),
    ("20378", q20378),
    ("22775", q22775),
    ("31431", q31431),
    ("4163", q4163),
    ("27027", q27027),
    ("22740", q22740),
    ("22214", q22214),
    ("33962", q33962),
    ("30067", q30067),
    ("21241", q21241),
    ("34916", q34916),
    ("4615", q4615),
    ("25827", q25827),
    ("21604", q21604),
    ("21530", q21530),
    ("20323", q20323),
    ("21818", q21818),
    ("21773", q21773),
    ("34292", q34292),
    ("20180", q20180),
    ("24461", q24461),
    ("22416", q22416),
    ("24149", q24149),
    ("32661", q32661),
    ("33808", q33808),
    ("24581", q24581),
    ("20438", q20438),
    ("764", q764),
    ("31219", q31219),
    ("1840", q1840),
    ("20040", q20040),
    ("30169", q30169),
    ("33813", q33813),
    ("22580", q22580),
    ("6403", q6403),
    ("24183", q24183),
    ("9615", q9615),
    ("23631", q23631),
    ("22745", q22745),
    ("23731", q23731),
    ("32745", q32745),
    ("24304", q24304),
    ("9667", q9667),
    ("21884", q21884),
    ("2333", q2333),
    ("23530", q23530),
    ("21784", q21784),
    ("34091", q34091),
    ("23545", q23545),
    ("27851", q27851),
    ("32257", q32257),
    ("23642", q23642),
    ("22456", q22456),
    ("23180", q23180),
    ("23471", q23471),
    ("24240", q24240),
    ("24313", q24313),
    ("21709", q21709),
    ("23707", q23707),
    ("3682", q3682),
    ("31598", q31598),
    ("5328", q5328),
    ("31862", q31862),
    ("21726", q21726),
    ("21986", q21986),
    ("24761", q24761),
    ("21066", q21066),
    ("2832", q2832),
    ("21432", q21432),
    ("3442", q3442),
    ("28557", q28557),
    ("24973", q24973),
    ("24237", q24237),
    ("24030", q24030),
    ("30228", q30228),
    ("3938", q3938),
    ("20076", q20076),
    ("31994", q31994),
    ("29403", q29403),
    ("23063", q23063),
    ("32890", q32890),
    ("21353", q21353),
    ("24329", q24329),
    ("1392", q1392),
    ("20274", q20274),
    ("20734", q20734),
    ("13487", q13487),
    ("21085", q21085),
    ("30444", q30444),
    ("34313", q34313),
    ("24705", q24705),
    ("22887", q22887),
    ("23723", q23723),
    ("23152", q23152),
    ("20137", q20137),
    ("23496", q23496),
    ("33929", q33929),
    ("24947", q24947),
    ("23549", q23549),
    ("33680", q33680),
];
