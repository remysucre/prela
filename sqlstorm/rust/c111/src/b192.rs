use harness::prelude::*;
use std::cmp::Reverse;

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn agg_distinct(mut v: Vec<&'static str>, sep: &str) -> Option<Str> {
    v.sort();
    v.dedup();
    if v.is_empty() { None } else { Some(leak(v.join(sep))) }
}


fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|x: (A, B)| x.0).inv().select(m.map(|x: (A, B)| x.1)).collect()
}

fn epoch(us: i64) -> f64 {
    (us / DAY_US) as f64 * 86400.0 + (us % DAY_US) as f64 / 1e6
}

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

// WITH RECURSIVE UserReputation AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 1000 WHEN b.Class = 2 THEN 500 WHEN b.Class = 3 THEN 100 END) AS CumulativeReputation
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// RecentPostHistory AS (SELECT p.Id AS PostId, p.Title, p.LastEditDate, p.LastActivityDate, ph.CreationDate AS HistoryDate, ph.PostHistoryTypeId, ph.Comment,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// AggregatedVotes AS (SELECT postId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN VoteTypeId = 1 THEN 1 END) AS AcceptedVotes FROM Votes GROUP BY postId),
// UserPostStats AS (SELECT u.Id AS UserId, COUNT(p.Id) AS TotalPosts, SUM(p.ViewCount) AS TotalViews, COALESCE(SUM(vs.UpVotes), 0) AS TotalUpVotes,
//        COALESCE(SUM(vs.DownVotes), 0) AS TotalDownVotes, COALESCE(SUM(CASE WHEN vs.AcceptedVotes > 0 THEN 1 ELSE 0 END), 0) AS AcceptedPosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN AggregatedVotes vs ON p.Id = vs.postId GROUP BY u.Id)
// SELECT u.DisplayName, u.Reputation AS OriginalReputation, ur.CumulativeReputation, ups.TotalPosts, ups.TotalViews, ups.TotalUpVotes, ups.TotalDownVotes, ups.AcceptedPosts,
//        COUNT(rph.PostId) AS RecentEdits, STRING_AGG(rph.Title, ', ') AS RecentEditedTitles
// FROM Users u LEFT JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN UserPostStats ups ON u.Id = ups.UserId LEFT JOIN RecentPostHistory rph ON u.Id = rph.PostId
// WHERE u.Reputation > 1000 AND ur.CumulativeReputation > 0
// GROUP BY u.DisplayName, u.Reputation, ur.CumulativeReputation, ups.TotalPosts, ups.TotalViews, ups.TotalUpVotes, ups.TotalDownVotes, ups.AcceptedPosts
// ORDER BY ur.CumulativeReputation DESC, ups.TotalPosts DESC LIMIT 10;
//
// No CTE refers to itself, so RECURSIVE changes nothing. `u.Id = rph.PostId` compares a user id with a post id. The STRING_AGG order is left open; the port joins in history id order.
fn q31090(db: &'static So) -> String {
    let User { reputation, display_name, origid: uo, .. } = &db.user;
    let Post { view_count, title, .. } = &db.post;
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold((0i64, 0i64), |(s, n), c| match c {
        1 => (s + 1000, n + 1),
        2 => (s + 500, n + 1),
        3 => (s + 100, n + 1),
        _ => (s, n),
    });
    let av = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 1) as i64]);
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt().and((&av).opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some((v, s)) => {
            let s = s.unwrap_or([0; 3]);
            [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s[0], a[4] + s[1], a[5] + (s[2] > 0) as i64]
        }
        None => a,
    });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let recent = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let us = (&ups).map(|a: [i64; 6]| (a[0], if a[1] == 0 { None } else { Some(a[2]) }, a[3], a[4], a[5]));
    let g = db
        .user
        .with(reputation.gt(1000))
        .with((&ur).filt(|(s, _): (i64, i64)| s > 0))
        .group_by(display_name.and(reputation).and((&ur).map(|(s, _): (i64, i64)| s)).and(us))
        .select(uo.select(&pidx).select(Ident::<Post>::new().and(recent).map(|(p, _)| p).select(title.opt())).opt())
        .buf_fold(|v| {
            let t: Vec<&str> = v.iter().filter_map(|x| x.flatten()).collect();
            (v.iter().filter(|x| x.is_some()).count() as i64, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let v = top_n(drain(&g), |&(((_, s), a), _)| (Reverse(s), Reverse(a.0)), 10);
    rows(v.into_iter().map(|((((d, r), s), (tp, tv, up, dn, acc)), (n, t))| row(vec![V::S(d), V::I(r), V::I(s), V::I(tp), oint(tv), V::I(up), V::I(dn), V::I(acc), V::I(n), ostr(t)])))
}

// WITH RECURSIVE PostCTE AS (SELECT p.Id, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.Score, p.ParentId, 0 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT a.Id, a.Title, a.Body, a.CreationDate, a.OwnerUserId, a.Score, a.ParentId, Level + 1 FROM Posts a INNER JOIN PostCTE q ON a.ParentId = q.Id WHERE a.PostTypeId = 2),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS RN FROM Users u WHERE u.Reputation > 0),
// PostVoteStats AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// ClosePosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, ph.UserId, ph.UserDisplayName, pt.Name AS PostHistoryType
//     FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id WHERE pt.Name IN ('Post Closed', 'Post Reopened'))
// SELECT p.Id AS PostId, p.Title, p.CreationDate AS QuestionDate, u.DisplayName AS OwnerName, COALESCE(r.Reputation, 0) AS OwnerReputation, COALESCE(vs.VoteCount, 0) AS TotalVotes,
//        COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes, CASE WHEN cp.PostId IS NOT NULL THEN 'Yes' ELSE 'No' END AS IsClosed, COUNT(DISTINCT c.Id) AS CommentCount
// FROM PostCTE p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserReputation r ON u.Id = r.UserId LEFT JOIN PostVoteStats vs ON p.Id = vs.PostId
// LEFT JOIN ClosePosts cp ON p.Id = cp.PostId LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.Level = 0 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, r.Reputation, vs.VoteCount, vs.UpVotes, vs.DownVotes, cp.PostId ORDER BY p.CreationDate DESC;
//
// Only the base case (Level = 0, the questions) is read, so the recursion is never needed.
fn q33131(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let q: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1)).collect();
    let vs = (&q).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(htype_name(db).filt(|n: Str| n == "Post Closed" || n == "Post Reopened")));
    let rep = owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(0))).select(&db.user.reputation);
    let cc = comments_per_post(db);
    let v = drain((&q).select(Ident::<Post>::new().and(&vs).and(rep.opt()).and(Ident::<Post>::new().with(closed).opt()).and(&cc)));
    rows(v.into_iter().map(|(_, ((((p, a), r), c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(r.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(if c.is_some() { "Yes" } else { "No" }), V::I(n)]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, 0 AS Level FROM Users u WHERE u.Reputation IS NOT NULL
//     UNION ALL SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, ur.Level + 1 FROM Users u JOIN UserReputation ur ON u.Id = ur.Id WHERE ur.Level < 2),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
//        AVG(v.BountyAmount) AS AverageBounty FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY p.OwnerUserId),
// UserBadgeSummary AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// FilteredPosts AS (SELECT p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.ViewCount, COALESCE(ct.Name, 'Other') AS CloseReason,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (10, 11) LEFT JOIN CloseReasonTypes ct ON ph.Comment::integer = ct.Id
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year')
// SELECT u.DisplayName, u.Reputation, u.CreationDate, us.PostCount, us.PositivePosts, us.NegativePosts, us.AverageBounty, ubs.GoldBadges, ubs.SilverBadges, ubs.BronzeBadges,
//        COUNT(f.Id) AS RecentPostsWithCloseReasons
// FROM Users u JOIN PostStats us ON u.Id = us.OwnerUserId JOIN UserBadgeSummary ubs ON u.Id = ubs.UserId LEFT JOIN FilteredPosts f ON u.Id = f.OwnerUserId
// WHERE u.Reputation > 1000
// GROUP BY u.DisplayName, u.Reputation, u.CreationDate, us.PostCount, us.PositivePosts, us.NegativePosts, us.AverageBounty, ubs.GoldBadges, ubs.SilverBadges, ubs.BronzeBadges
// HAVING COUNT(f.Id) > 5 ORDER BY u.Reputation DESC;
//
// The recursive UserReputation is never read. CloseReasonTypes joins at most one row per history row, so FilteredPosts is posts x their close/reopen history.
fn q31154(db: &'static So) -> String {
    let Post { owner_user, score, creation_date, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let us = db.post.group_by(owner_user).select(score.and(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt()).opt())).fold([0i64; 5], |a, (s, b)| {
        let b = b.flatten();
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + b.unwrap_or(0), a[4] + b.is_some() as i64]
    });
    let ubs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let hist = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let User { reputation, display_name, creation_date: ucd, .. } = &db.user;
    let g = db
        .user
        .with(reputation.gt(1000))
        .group_by(display_name.and(reputation).and(ucd).and(&us).and(&ubs))
        .select(posts_of(db).select(recent).select(hist.opt()).opt())
        .fold(0i64, |n, f| n + f.is_some() as i64);
    rows(drain((&g).filt(|n| n > 5)).into_iter().map(|(((((d, r), c), a), b), n)| row(vec![V::S(d), V::I(r), V::T(c), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(n)])))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.PostCount, ua.QuestionCount, ua.AnswerCount, ua.CommentCount, ua.BadgeCount,
//        ROW_NUMBER() OVER (ORDER BY ua.Reputation DESC) AS Rank FROM UserActivity ua WHERE ua.Reputation > 100),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.Tags, ARRAY_AGG(DISTINCT pi.LinkTypeId) AS RelatedLinkTypes, COUNT(c.Id) AS TotalComments,
//        MAX(p.LastActivityDate) AS LastActivity FROM Posts p LEFT JOIN PostLinks pi ON p.Id = pi.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.Tags),
// ActivitySummary AS (SELECT p.Title, COUNT(DISTINCT v.UserId) AS UniqueViewers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, MAX(p.LastActivityDate) AS LastActivityDate FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Title)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, tu.QuestionCount, tu.AnswerCount, ps.PostId, ps.Title AS PostTitle, ps.ViewCount, ps.Score, ps.Tags,
//        asu.UniqueViewers, asu.UpVotes, asu.DownVotes, asu.LastActivityDate
// FROM TopUsers tu JOIN PostStatistics ps ON ps.ViewCount > 100 JOIN ActivitySummary asu ON ps.Title = asu.Title WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the users x posts x comments x badges product is driven for them alone.
fn q27883(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let Post { post_type_id, creation_date, view_count, title, last_activity_date, .. } = &db.post;
    let uw = whole(db.user.with(reputation.gt(100))).select(Ident::<User>::new().and(reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    type U = (Id<User>, i64);
    let tu: MatSet<U> = (&uw).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let tus: MatSet<Id<User>> = (&tu).map(|x: U| x.0).collect();
    let ua = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| [a[0] + (p.map(|p| p.0) == Some(1)) as i64, a[1] + (p.map(|p| p.0) == Some(2)) as i64]);
    let Vote { user_id, vote_type_id, .. } = &db.vote;
    let by_title = || db.post.group_by(title);
    let asf = by_title()
        .select(last_activity_date.and(votes_of(db).select(vote_type_id).opt()))
        .fold((i64::MIN, 0i64, 0i64), |(m, up, dn), (l, t)| (m.max(l), up + (t == Some(2)) as i64, dn + (t == Some(3)) as i64));
    let asd = by_title().select(votes_of(db).select(user_id)).count_distinct();
    let v = drain(
        (&tu).select(Same::<U>::new().and(Same::<U>::new().map(|x: U| x.0).select(&ua))).cross(
            db.post
                .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
                .with(view_count.gt(100))
                .select(Ident::<Post>::new().and(title.select((&asf).and((&asd).opt())))),
        ),
    );
    rows(v.into_iter().map(|(_, (((u, k), a), (p, ((m, up, dn), n))))| {
        let mut f = vec![V::I(k)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["id", "title", "views", "score", "tags"]));
        f.extend([V::I(n.unwrap_or(0)), V::I(up), V::I(dn), V::T(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostActivities AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS ActivityCount FROM PostHistory ph
//     WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' AND ph.PostHistoryTypeId IN (10, 11, 12, 13) GROUP BY ph.PostId, ph.PostHistoryTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, ub.BadgeCount, ub.BadgeNames,
//        SUM(CASE WHEN pa.PostHistoryTypeId = 10 THEN pa.ActivityCount ELSE 0 END) AS CloseCount, SUM(CASE WHEN pa.PostHistoryTypeId = 11 THEN pa.ActivityCount ELSE 0 END) AS ReopenCount,
//        SUM(CASE WHEN pa.PostHistoryTypeId = 12 THEN pa.ActivityCount ELSE 0 END) AS DeleteCount, SUM(CASE WHEN pa.PostHistoryTypeId = 13 THEN pa.ActivityCount ELSE 0 END) AS UndeleteCount
//     FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.PostId = ub.UserId LEFT JOIN PostActivities pa ON rp.PostId = pa.PostId WHERE rp.Rank <= 5
//     GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, ub.BadgeCount, ub.BadgeNames)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.CommentCount, fp.BadgeCount, fp.BadgeNames, fp.CloseCount, fp.ReopenCount, fp.DeleteCount, fp.UndeleteCount,
//        CASE WHEN fp.CommentCount > 0 THEN 'Active' ELSE 'Inactive' END AS PostStatus, CASE WHEN fp.CloseCount > 0 THEN 'Closed' ELSE NULL END AS ClosureStatus
// FROM FilteredPosts fp WHERE fp.BadgeCount > 2 ORDER BY fp.Score DESC, fp.CreationDate ASC;
//
// `rp.PostId = ub.UserId` compares a post id with a user id. A Score tie inside the rank goes to the smaller post id, and the badge names are joined in badge id order (the SQL leaves both open).
fn q23866(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name).opt()).buf_fold(|v| {
        let n: Vec<&str> = v.iter().filter_map(|x| *x).collect();
        (n.len() as i64, if n.is_empty() { None } else { Some(leak(n.join(", "))) })
    });
    let PostHistory { creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let pa = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).with(post_history_type_id.is_in([10, 11, 12, 13]))).select(post_history_type_id);
    let fp = (&tp).group_by(Ident::<Post>::new()).select(pa.opt()).fold([0i64; 4], |a, t| match t {
        Some(t) => {
            let mut a = a;
            a[(t - 10) as usize] += 1;
            a
        }
        None => a,
    });
    let cc = comments_per_post(db);
    let v = drain((&tp).select(Ident::<Post>::new().and(&cc).and(origid.select(&uidx).select(&ub).filt(|(n, _): (i64, Option<Str>)| n > 2)).and(&fp)));
    rows(v.into_iter().map(|(_, (((p, c), (n, b)), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(n), ostr(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.push(V::S(if c > 0 { "Active" } else { "Inactive" }));
        f.push(if a[0] > 0 { V::S("Closed") } else { V::Null });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPosts
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostActivity AS (SELECT ph.PostId, COUNT(DISTINCT ph.UserId) AS EditorCount, MAX(ph.CreationDate) AS LastEditDate, MAX(ph.PostHistoryTypeId) AS LastActionType FROM PostHistory ph GROUP BY ph.PostId),
// CloseReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasonNames FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, pa.EditorCount, pa.LastEditDate, pa.LastActionType, cr.CloseReasonNames,
//        us.DisplayName AS UserName, us.TotalUpvotes, us.TotalDownvotes, us.GoldBadges, us.SilverBadges, us.BronzeBadges,
//        CASE WHEN rp.TotalPosts > 5 THEN 'Frequent Contributor' ELSE 'New Contributor' END AS ContributorStatus
// FROM RankedPosts rp LEFT JOIN PostActivity pa ON rp.PostId = pa.PostId LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId LEFT JOIN Users u ON rp.OwnerUserId = u.Id
// LEFT JOIN UserStats us ON u.Id = us.UserId WHERE (us.TotalUpvotes - us.TotalDownvotes) > 10 ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 100;
//
// The WHERE on UserStats drops ownerless posts, so the owners of the recent posts are the only users grouped. The STRING_AGG order is left open; the port joins in history id order.
fn q32383(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let tot = recent().group_by(owner_user).fold(0i64, |n, _| n + 1);
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]);
    let PostHistory { user_id, creation_date: hd, post_history_type_id, comment, .. } = &db.post_history;
    let pg = || recent().group_by(Ident::<Post>::new());
    let pam = pg().select(history_of(db).select(hd.and(post_history_type_id))).fold((i64::MIN, i64::MIN), |(d, t), (x, y)| (d.max(x), t.max(y)));
    let pac = pg().select(history_of(db).select(user_id)).count_distinct();
    let crt: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(&db.post_history.post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt))
        .buf_fold(|v| leak(v.join(", ")));
    let v = drain(
        recent().select(
            Ident::<Post>::new()
                .and(owner_user.select(Ident::<User>::new().and(&us).and(&tot)).filt(|((_, a), _): ((Id<User>, [i64; 5]), i64)| a[0] - a[1] > 10))
                .and((&pam).opt().and((&pac).opt()))
                .and((&cr).opt()),
        ),
    );
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(_, (((p, ((u, a), n)), (e, k)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend(match e {
            Some((d, t)) => [V::I(k.unwrap_or(0)), V::T(d), V::I(t)],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(ostr(c));
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        f.push(V::S(if n > 5 { "Frequent Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.ViewCount, p.Score, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TagDetails AS (SELECT TagName, COUNT(*) AS TagCount, MIN(PostId) AS SampleTagId
//     FROM (SELECT unnest(string_to_array(substring(Tags, 2, length(Tags)-2), '><')) AS TagName, p.Id AS PostId FROM Posts p WHERE p.PostTypeId = 1) sub GROUP BY TagName),
// TopTags AS (SELECT TagName, TagCount, (SELECT SUM(TagCount) FROM TagDetails) AS TotalTags, ROUND((TagCount * 1.0 / (SELECT SUM(TagCount) FROM TagDetails)) * 100, 2) AS Percentage
//     FROM TagDetails ORDER BY TagCount DESC LIMIT 10),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id),
// FinalOutput AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.OwnerUserId, rp.OwnerDisplayName, rp.ViewCount, rp.Score, tt.TagName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
//     FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN TopTags tt ON rp.Tags LIKE '%' || tt.TagName || '%' WHERE rp.PostRank = 1)
// SELECT PostId, Title, Body, CreationDate, OwnerUserId, OwnerDisplayName, ViewCount, Score, TagName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges
// FROM FinalOutput ORDER BY Score DESC, ViewCount DESC;
//
// A CreationDate tie inside PostRank goes to the smaller post id (the SQL leaves it open).
fn q28792(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, .. } = &db.post;
    let q = || db.post.with(post_type_id.eq(1));
    let td = q().select(tags_str.flat_map(tag_list)).inv().fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&td), |&(_, n)| Reverse(n), 10);
    let tn: MatSet<Str> = rel(tt.into_iter().map(|x| x.0).collect::<Vec<Str>>()).map(|n| n).collect();
    let w = q().with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let ub = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let hit = tags_str.select_where(&tn, |s: Str, n: Str| like(s, &format!("%{n}%")));
    let v = drain((&rp).select(Ident::<Post>::new().and(owner_user.select(&ub).opt()).and(hit.opt())));
    rows(v.into_iter().map(|(_, ((p, b), t))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner_id", "owner", "views", "score"]);
        f.push(ostr(t));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalUserPosts, STRING_AGG(t.TagName, ', ') AS AssociatedTags
//     FROM Posts p LEFT JOIN Tags t ON t.WikiPostId = p.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.PostTypeId, p.Score, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE NULL END), 0) AS TotalVotes, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON v.UserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostInteraction AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, us.DisplayName, us.Reputation AS UserReputation, rp.AssociatedTags,
//        CASE WHEN rp.Score >= 10 THEN 'Hot' WHEN rp.Score BETWEEN 5 AND 9 THEN 'Trending' ELSE 'Normal' END AS PopularityLevel
//     FROM RankedPosts rp JOIN UserStats us ON us.UserId = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = rp.PostId) WHERE rp.UserPostRank <= 3)
// SELECT pi.PostId, pi.Title, pi.CreationDate, pi.Score, pi.UserReputation, pi.PopularityLevel, pi.AssociatedTags, pht.Name AS PostHistoryType, ph.CreationDate AS HistoryCreationDate,
//        ph.UserDisplayName AS EditorDisplayName
// FROM PostInteraction pi LEFT JOIN PostHistory ph ON ph.PostId = pi.PostId LEFT JOIN PostHistoryTypes pht ON pht.Id = ph.PostHistoryTypeId
// WHERE ph.CreationDate >= pi.CreationDate AND pi.PopularityLevel = 'Hot' AND (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pi.PostId) > 5
// ORDER BY pi.Score DESC, UserReputation DESC LIMIT 100;
//
// The scalar subquery reads the post's own owner. A CreationDate tie inside UserPostRank goes to the smaller post id, and the tag names are joined in tag id order (the SQL leaves both open).
fn q23614(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 3).map(|((p, _), _)| p).collect();
    let wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let tags = (&rp).group_by(Ident::<Post>::new()).select((&wiki).select(&db.tag.tag_name)).buf_fold(|v| leak(v.join(", ")));
    let hot = (&rp)
        .with(score.ge(10))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .with(comments_per_post(db).filt(|n| n > 5));
    let PostHistory { creation_date: hd, user_display_name, .. } = &db.post_history;
    let v = drain(
        hot.select(Ident::<Post>::new().and(creation_date).and(history_of(db).select(Ident::<PostHistory>::new().and(hd))))
            .filt(|((_, c), (_, d)): ((Id<Post>, i64), (Id<PostHistory>, i64))| d >= c)
            .map(|((p, _), (h, _))| (p, h))
            .select(Same::<(Id<Post>, Id<PostHistory>)>::new().and(Same::<(Id<Post>, Id<PostHistory>)>::new().map(|x: (Id<Post>, Id<PostHistory>)| x.0).select(&tags).opt())),
    );
    let rep = |p: Id<Post>| db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
    let v = top_n(v, |&(_, ((p, _), _))| (Reverse(score.get(p).unwrap()), Reverse(rep(p))), 100);
    rows(v.into_iter().map(|(_, ((p, h), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "rep"]);
        f.extend([V::S("Hot"), ostr(t), V::S(db.post_history_type.name.get(db.post_history.post_history_type.get(h).unwrap()).unwrap()), V::T(hd.get(h).unwrap()), ostr(user_display_name.get(h))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostSummaries AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, COALESCE(CAST(NULLIF(rp.ViewCount, 0) AS FLOAT) / NULLIF(SUM(rp.ViewCount) OVER (), 0), 0) AS PercentageOfTotalViews,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpvoteCount, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS TotalUpvotes, AVG(COALESCE(c.Score, 0)) AS AvgCommentScore,
//        CASE WHEN COUNT(c.Id) > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus
//     FROM RankedPosts rp LEFT JOIN Votes v ON v.PostId = rp.PostId LEFT JOIN Comments c ON c.PostId = rp.PostId WHERE rp.rn <= 10
//     GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount),
// UserBadges AS (SELECT b.UserId, STRING_AGG(b.Name, ', ') AS BadgeNames, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// FinalAnalytics AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.PercentageOfTotalViews, ps.UpvoteCount, ps.AvgCommentScore, ps.CommentStatus,
//        COALESCE(ub.BadgeNames, 'No Badges') AS UserBadges, COALESCE(ub.GoldBadges, 0) AS GoldBadgeCount, COALESCE(ub.SilverBadges, 0) AS SilverBadgeCount, COALESCE(ub.BronzeBadges, 0) AS BronzeBadgeCount
//     FROM PostSummaries ps LEFT JOIN Users u ON u.Id = ps.PostId LEFT JOIN UserBadges ub ON ub.UserId = u.Id)
// SELECT PostId, Title, CreationDate, Score, PercentageOfTotalViews, UpvoteCount, AvgCommentScore, CommentStatus, UserBadges, GoldBadgeCount, SilverBadgeCount, BronzeBadgeCount
// FROM FinalAnalytics WHERE Score > 10 OR AvgCommentScore > 5 ORDER BY CreationDate DESC, Score DESC LIMIT 50;
//
// `u.Id = ps.PostId` compares a user id with a post id. A CreationDate tie inside rn goes to the smaller post id, and the badge names are joined in badge id order (the SQL leaves both open).
fn q22682(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let tv = (&rp).select(view_count).fold_flat(0i64, |s, w| s + w);
    let ps = (&rp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).select(&db.comment.score).opt()))
        .fold([0i64; 4], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + c.unwrap_or(0), a[2] + 1, a[3] + c.is_some() as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.badge.group_by(&db.badge.user).select((&db.badge.name).and(&db.badge.class)).buf_fold(|v| {
        let n: Vec<&str> = v.iter().map(|x| x.0).collect();
        (leak(n.join(", ")), [1, 2, 3].map(|k| v.iter().filter(|x| x.1 == k).count() as i64))
    });
    type J = (((Id<Post>, i64), [i64; 4]), Option<(Str, [i64; 3])>);
    let v = drain((&rp).select(Ident::<Post>::new().and(score).and(&ps).and(origid.select(&uidx).select(&ub).opt())).filt(|(((_, s), a), _): J| s > 10 || a[1] as f64 / a[2] as f64 > 5.0));
    let v = top_n(v, |&(_, (((p, s), _), _))| (Reverse(creation_date.get(p).unwrap()), Reverse(s)), 50);
    rows(v.into_iter().map(|(_, (((p, _), a), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        let w = view_count.get(p).filter(|&w| w != 0);
        f.push(V::F(match w {
            Some(w) if tv != 0 => w as f64 / tv as f64,
            _ => 0.0,
        }));
        f.extend([V::I(a[0]), avg(a[1], a[2]), V::S(if a[3] > 0 { "Has Comments" } else { "No Comments" })]);
        match b {
            Some((n, c)) => {
                f.push(V::S(n));
                f.extend(c.map(V::I));
            }
            None => f.extend([V::S("No Badges"), V::I(0), V::I(0), V::I(0)]),
        }
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount,
//        COUNT(DISTINCT B.Id) AS BadgeCount, COALESCE(SUM(P.ViewCount), 0) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, PostCount, CommentCount, BadgeCount, TotalViews, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStatistics),
// MostActiveUsers AS (SELECT UserId, COUNT(*) AS ActivityCount FROM (SELECT U.Id AS UserId, P.CreationDate FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId
//     UNION ALL SELECT U.Id AS UserId, C.CreationDate FROM Users U JOIN Comments C ON U.Id = C.UserId) AS UserActivity GROUP BY UserId),
// FinalReport AS (SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.UpVotes, TU.DownVotes, TU.PostCount, TU.CommentCount, TU.BadgeCount, TU.TotalViews,
//        COALESCE(MAU.ActivityCount, 0) AS ActivityCount FROM TopUsers TU LEFT JOIN MostActiveUsers MAU ON TU.UserId = MAU.UserId)
// SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, PostCount, CommentCount, BadgeCount, TotalViews, ActivityCount,
//        CASE WHEN Reputation > 1000 THEN 'Expert' WHEN Reputation BETWEEN 500 AND 1000 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel,
//        CASE WHEN TotalViews IS NULL OR TotalViews = 0 THEN 'Inactive' WHEN TotalViews > 10000 THEN 'Highly Active' ELSE 'Moderately Active' END AS ViewActivity,
//        CONCAT('User: ', DisplayName, ', Reputation Level: ', CASE WHEN Reputation < 500 THEN 'Low' WHEN Reputation BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'High' END) AS Remark
// FROM FinalReport WHERE ActivityCount > 0 ORDER BY Reputation DESC, ActivityCount DESC LIMIT 50;
//
// The WHERE and the ORDER BY read only Reputation and ActivityCount, so the 50 users are picked first and the product is driven for them alone.
fn q21514(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let act = (&db.post.owner_user).inv().map(|_| ()).union((&db.comment.user).inv().map(|_| ())).fold(0i64, |n, _| n + 1);
    let top = top_n(drain(db.user.select(reputation.and(&act)).filt(|(_, n): (i64, i64)| n > 0)), |&(u, (r, n))| (Reverse(r), Reverse(n), u), 50);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { view_count, .. } = &db.post;
    let prod = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, _)| match p {
            Some(((w, _), t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + w.unwrap_or(0)],
            None => a,
        });
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let nc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let nb = (&tu).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).select(Ident::<User>::new().and(&prod).and((&np).opt()).and((&nc).opt()).and((&nb).opt()).and(&act)));
    rows(v.into_iter().map(|(_, (((((u, a), p), c), b), n))| {
        let r = reputation.get(u).unwrap();
        let name = db.user.display_name.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(p.unwrap_or(0)), V::I(c.unwrap_or(0)), V::I(b.unwrap_or(0)), V::I(a[2]), V::I(n)]);
        f.push(V::S(if r > 1000 { "Expert" } else if r >= 500 { "Intermediate" } else { "Novice" }));
        f.push(V::S(if a[2] == 0 { "Inactive" } else if a[2] > 10000 { "Highly Active" } else { "Moderately Active" }));
        f.push(V::Owned(format!("User: {name}, Reputation Level: {}", if r < 500 { "Low" } else if r <= 1000 { "Medium" } else { "High" })));
        row(f)
    }))
}

// WITH UserTags AS (SELECT u.Id AS UserId, u.DisplayName, t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS TagName) t ON TRUE
//     GROUP BY u.Id, u.DisplayName, t.TagName),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostsActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, h.UserDisplayName, h.Comment, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY h.CreationDate DESC) AS ActivityRank
//     FROM Posts p LEFT JOIN PostHistory h ON p.Id = h.PostId WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '30 days'),
// TopUsers AS (SELECT ut.UserId, ut.DisplayName, SUM(ut.PostCount) AS TotalPosts, SUM(ub.GoldBadges) AS TotalGoldBadges, SUM(ub.SilverBadges) AS TotalSilverBadges,
//        SUM(ub.BronzeBadges) AS TotalBronzeBadges FROM UserTags ut LEFT JOIN UserBadges ub ON ut.UserId = ub.UserId GROUP BY ut.UserId, ut.DisplayName ORDER BY TotalPosts DESC LIMIT 10),
// PostMetrics AS (SELECT p.Id, p.Title, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '6 months' GROUP BY p.Id, p.Title, p.ViewCount, p.Score)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalGoldBadges, tu.TotalSilverBadges, tu.TotalBronzeBadges, pm.Title, pm.ViewCount, pm.Score, pm.CommentCount, pm.UpVotes, pm.DownVotes
// FROM TopUsers tu JOIN PostMetrics pm ON pm.Id IN (SELECT p.Id FROM Posts p ORDER BY p.ViewCount DESC LIMIT 5) ORDER BY tu.TotalPosts DESC;
//
// PostsActivity is never read. The IN subquery is the five most viewed posts, picked first; PostMetrics is driven for them alone.
fn q29235(db: &'static So) -> String {
    let Post { owner_user, tags_str, view_count, creation_date, .. } = &db.post;
    type T = (Id<User>, Str);
    let ut = db.post.select(owner_user.and(tags_str.flat_map(tag_list))).map(|(u, t)| (u, t)).inv().fold(0i64, |n, _| n + 1);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let utv = rel(drain(&ut));
    let tu = (&utv)
        .group_by(Same::<(T, i64)>::new().map(|x: (T, i64)| x.0 .0))
        .select(Same::<(T, i64)>::new().map(|x: (T, i64)| x.1).and(Same::<(T, i64)>::new().map(|x: (T, i64)| x.0 .0).select((&ub).opt())))
        .fold([0i64; 4], |a, (n, b)| {
            let b = b.unwrap_or([0; 3]);
            [a[0] + n, a[1] + b[0], a[2] + b[1], a[3] + b[2]]
        });
    let tu = top_n(drain(&tu), |&(_, a)| Reverse(a[0]), 10);
    let top5 = top_n(drain(&db.post.origid), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 5);
    let t5: MatSet<Id<Post>> = rel(top5.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pm = (&t5)
        .with(creation_date.ge(add_months(date(2024, 10, 1), -6)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain(rel(tu).cross(&pm));
    rows(v.into_iter().map(|((_, p), ((u, a), m))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend(m.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, u.UpVotes, u.DownVotes, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views, u.UpVotes, u.DownVotes),
// CommentStatistics AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostViewCounts AS (SELECT p.Id AS PostId, p.ViewCount, COALESCE(cs.CommentCount, 0) AS TotalComments, p.Score AS PostScore,
//        CASE WHEN p.Score >= 100 THEN 'High' WHEN p.Score BETWEEN 50 AND 99 THEN 'Medium' ELSE 'Low' END AS ScoreCategory
//     FROM Posts p LEFT JOIN CommentStatistics cs ON p.Id = cs.PostId WHERE p.PostTypeId = 1),
// FinalResults AS (SELECT ups.UserId, ups.DisplayName, ups.Reputation, ups.Views, ups.GoldBadges, ups.SilverBadges, ups.BronzeBadges, pvc.PostId, pvc.ViewCount, pvc.TotalComments,
//        pvc.PostScore, pvc.ScoreCategory, rp.Title AS TopScoringPostTitle
//     FROM UserStatistics ups LEFT JOIN PostViewCounts pvc ON ups.UserId = pvc.PostId LEFT JOIN RankedPosts rp ON ups.UserId = rp.OwnerUserId AND rp.ScoreRank = 1)
// SELECT UserId, DisplayName, Reputation, Views, GoldBadges, SilverBadges, BronzeBadges, COUNT(PostId) AS TotalPosts, SUM(ViewCount) AS TotalViews, SUM(TotalComments) AS TotalComments,
//        AVG(PostScore) AS AveragePostScore, STRING_AGG(DISTINCT ScoreCategory, ', ') AS ScoreCategories
// FROM FinalResults GROUP BY UserId, DisplayName, Reputation, Views, GoldBadges, SilverBadges, BronzeBadges ORDER BY Reputation DESC, TotalViews DESC;
//
// `ups.UserId = pvc.PostId` compares a user id with a post id, so each user meets at most one question. The rp join keeps one row (ScoreRank = 1) and nothing from it is aggregated.
fn q32055(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ubs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let cc = comments_per_post(db);
    let pvc = Ident::<Post>::new().with(post_type_id.eq(1)).select(view_count.opt().and(&cc).and(score));
    let v = drain(db.user.select(Ident::<User>::new().and(&ubs).and((&db.user.origid).select(&pidx).select(pvc).opt())));
    rows(v.into_iter().map(|(_, ((u, b), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(((w, c), s)) => [V::I(1), oint(w), V::I(c), V::F(s as f64), V::S(if s >= 100 { "High" } else if s >= 50 { "Medium" } else { "Low" })],
            None => [V::I(0), V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// ActivityRank AS (SELECT UserId, DisplayName, Reputation, PostCount, UpvoteCount, DownvoteCount, BadgeCount, RANK() OVER (ORDER BY PostCount DESC, UpvoteCount DESC, BadgeCount DESC) AS Rank FROM UserActivity),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpvoteCount, DownvoteCount, BadgeCount, Rank FROM ActivityRank WHERE Rank <= 10),
// PostHistories AS (SELECT ph.PostId, ph.PostHistoryTypeId, h.Name AS HistoryTypeName, ph.CreationDate, ph.UserDisplayName, ph.Comment, ph.Text
//     FROM PostHistory ph JOIN PostHistoryTypes h ON ph.PostHistoryTypeId = h.Id WHERE ph.CreationDate > CAST('2022-01-01' AS timestamp)),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, COALESCE(PH.RecentHistoryCount, 0) AS RecentHistoryCount, (SELECT COUNT(DISTINCT c.Id) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
//        (SELECT SUM(V.BountyAmount) FROM Votes V WHERE V.PostId = p.Id AND V.VoteTypeId = 8) AS TotalBounty, (SELECT ARRAY_AGG(DISTINCT t.TagName) FROM Tags t WHERE t.ExcerptPostId = p.Id) AS Tags
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS RecentHistoryCount FROM PostHistories GROUP BY PostId) PH ON p.Id = PH.PostId WHERE p.ViewCount > (SELECT AVG(ViewCount) FROM Posts))
// SELECT tu.DisplayName, tu.Reputation, ps.Title, ps.RecentHistoryCount, ps.CommentCount, ps.TotalBounty, ps.Tags
// FROM TopUsers tu JOIN PostStatistics ps ON tu.UserId = ps.PostId WHERE ps.RecentHistoryCount > 0 ORDER BY tu.Reputation DESC, ps.CommentCount DESC;
//
// The rank leads with PostCount, so only users ranked in the top 10 by PostCount alone can rank in the top 10; the product is driven for them alone.
// `tu.UserId = ps.PostId` compares a user id with a post id. The distinct tag names are listed in name order (the SQL leaves it open).
fn q21957(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let pc = db.user.with(reputation.gt(0)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pw = whole(db.user.with(reputation.gt(0))).select(Ident::<User>::new().and(&pc)).window(rank, |(_, n)| Reverse(n), asc);
    let cand: MatSet<Id<User>> = (&pw).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let nb = (&cand).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let rw = whole(&cand).select(Ident::<User>::new().and((&pc).and(&ua).and((&nb).opt()))).window(rank, |(_, ((p, a), b))| (Reverse(p), Reverse(a[0]), Reverse(b.unwrap_or(0))), asc);
    let tu: MatSet<Id<User>> = (&rw).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let Post { view_count, origid, .. } = &db.post;
    let (vs, vn) = db.post.select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let va = vs as f64 / vn as f64;
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.with(hd.gt(ts(2022, 1, 1, 0, 0, 0))).group_by(&db.post_history.post).fold(0i64, |n, _| n + 1);
    let cc = comments_per_post(db);
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let tb = db.vote.with(vote_type_id.eq(8)).group_by(&db.vote.post).select(bounty_amount.opt()).fold((0i64, 0i64), |(s, n), b| (s + b.unwrap_or(0), n + b.is_some() as i64));
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&ex).select(&db.tag.tag_name).gather();
    let pidx: HashIdx<i64, Id<Post>> = origid.inv().collect();
    let ps = Ident::<Post>::new().with(view_count.filt(move |w| w as f64 > va)).select(Ident::<Post>::new().and(&ph).and(&cc).and((&tb).opt()).and((&tg).opt()));
    let v = drain((&tu).select(Ident::<User>::new().and((&db.user.origid).select(&pidx).select(ps))));
    let v = top_n(v, |&(u, (_, ((((_, _), c), _), _)))| (Reverse(reputation.get(u).unwrap()), Reverse(c)), 0);
    rows(v.into_iter().map(|(_, (u, ((((p, h), c), b), t)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(h), V::I(c), b.map_or(V::Null, |(s, n)| nullable(s, n))]);
        f.push(match t {
            Some(t) => {
                let mut t: Vec<Str> = t.to_vec();
                t.sort();
                t.dedup();
                V::L(t.into_iter().map(V::S).collect())
            }
            None => V::Null,
        });
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, t.TagName
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN LATERAL unnest(string_to_array(p.Tags, '<>')) AS t(TagName) ON true
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, t.TagName),
// RankedPosts AS (SELECT *, RANK() OVER (PARTITION BY TagName ORDER BY VoteCount DESC, UpVotes DESC) AS Rank FROM PostStatistics)
// SELECT PostId, Title, CreationDate, CommentCount, VoteCount, UpVotes, DownVotes, TagName FROM RankedPosts WHERE Rank <= 10 ORDER BY TagName, Rank;
//
// The separator is '<>', which the Tags strings never contain, so each tagged post has one TagName (its whole Tags string) and an untagged one has NULL.
fn q6602(db: &'static So) -> String {
    let Post { creation_date, tags_str, .. } = &db.post;
    type K = (Id<Post>, Option<Str>);
    let pt = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(Ident::<Post>::new().and(tags_str.opt().flat_map(|t: Option<Str>| match t {
        Some(s) => s.split("<>").map(Some).collect::<Vec<_>>(),
        None => vec![None],
    })));
    let ks: MatSet<K> = pt.collect();
    let post = || Same::<K>::new().map(|x: K| x.0);
    let Vote { user_id, vote_type_id, .. } = &db.vote;
    let ps = (&ks)
        .group_by(Same::<K>::new())
        .select(post().select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let vc = (&ks).group_by(Same::<K>::new()).select(post().select(votes_of(db).select(user_id))).count_distinct();
    let w = (&ks)
        .group_by(Same::<K>::new().map(|x: K| x.1))
        .select(Same::<K>::new().and((&ps).and((&vc).opt())))
        .window(rank, |(_, (a, d))| (Reverse(d.unwrap_or(0)), Reverse(a[1])), asc);
    let v = drain((&w).filt(|(_, k)| k <= 10).map(|(x, _)| x));
    rows(v.into_iter().map(|(_, ((p, t), (a, d)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(d.unwrap_or(0)), V::I(a[1]), V::I(a[2])]);
        f.push(ostr(t));
        row(f)
    }))
}

// WITH ProcessedTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagUsage AS (SELECT Tag, COUNT(*) AS UsageCount FROM ProcessedTags GROUP BY Tag),
// TopTags AS (SELECT Tag, UsageCount, ROW_NUMBER() OVER (ORDER BY UsageCount DESC) AS Rank FROM TagUsage WHERE UsageCount > 5),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ARRAY_AGG(DISTINCT t.Tag) AS Tags
//     FROM Posts p JOIN ProcessedTags t ON p.Id = t.PostId WHERE p.PostTypeId = 1 AND p.Score > 10 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, t.Tag AS MostUsedTag
// FROM TopPosts tp JOIN TopTags t ON t.Tag = ANY(tp.Tags) WHERE t.Rank <= 5 ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q25841(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, .. } = &db.post;
    let q = || db.post.with(post_type_id.eq(1));
    let tu = q().select(tags_str.flat_map(tag_list)).inv().fold(0i64, |n, _| n + 1);
    let top = top_n(drain((&tu).filt(|n| n > 5)), |&(_, n)| Reverse(n), 5);
    let tt: MatSet<Str> = rel(top.into_iter().map(|x| x.0).collect::<Vec<Str>>()).map(|t| t).collect();
    let pt: MatSet<(Id<Post>, Str)> = q().with(score.gt(10)).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list))).collect();
    let v = drain((&pt).with(Same::<(Id<Post>, Str)>::new().map(|x: (Id<Post>, Str)| x.1).select(&tt)));
    rows(v.into_iter().map(|(_, (p, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::S(t));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStats AS (SELECT OwnerUserId, COUNT(*) AS PostCount, SUM(ViewCount) AS TotalViews, AVG(Score) AS AverageScore FROM Posts
//     WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY OwnerUserId),
// TopUsers AS (SELECT ur.DisplayName, ur.Reputation, ps.PostCount, ps.TotalViews, ps.AverageScore FROM UserReputation ur JOIN PostStats ps ON ur.Id = ps.OwnerUserId WHERE ur.Reputation > 1000),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, ph.UserDisplayName, ph.CreationDate AS CloseDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10)
// SELECT tu.DisplayName, tu.Reputation, COALESCE(COUNT(cp.PostId), 0) AS ClosedPostCount,
//        SUM(CASE WHEN cp.CloseDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' THEN 1 ELSE 0 END) AS RecentClosedPosts, STRING_AGG(DISTINCT cp.Title, ', ') AS ClosedPostTitles
// FROM TopUsers tu LEFT JOIN ClosedPosts cp ON tu.DisplayName = cp.UserDisplayName GROUP BY tu.DisplayName, tu.Reputation ORDER BY tu.Reputation DESC LIMIT 10;
//
// The distinct titles are joined in sorted order (the SQL leaves it open).
fn q3914(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user, title, .. } = &db.post;
    let ps: MatSet<Id<User>> = db.post.with(creation_date.ge(add_years(t0, -1))).select(owner_user).collect();
    let PostHistory { post_history_type_id, user_display_name, creation_date: hd, post, .. } = &db.post_history;
    let cp: HashIdx<Str, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(user_display_name).inv().collect();
    let User { display_name, reputation, .. } = &db.user;
    let g = db
        .user
        .with(reputation.gt(1000))
        .with(&ps)
        .group_by(display_name.and(reputation))
        .select(display_name.select(&cp).select(hd.and(post.select(title.opt()))).opt())
        .buf_fold(|v| {
            let n = v.iter().filter(|x| x.is_some()).count() as i64;
            let r = v.iter().filter(|x| x.map_or(false, |x| x.0 >= add_months(t0, -1))).count() as i64;
            (n, r, agg_distinct(v.iter().filter_map(|x| x.and_then(|x| x.1)).collect(), ", "))
        });
    let v = top_n(drain(&g), |&((_, r), _)| Reverse(r), 10);
    rows(v.into_iter().map(|((d, r), (n, k, t))| row(vec![V::S(d), V::I(r), V::I(n), V::I(k), ostr(t)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerName, p.CreationDate, p.AnswerCount, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 5),
// RecentCloseReasons AS (SELECT ph.PostId, ARRAY_AGG(DISTINCT ctr.Name) AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes ctr ON CAST(ph.Comment AS INTEGER) = ctr.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) AND ph.CreationDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month' GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.OwnerName, rp.CreationDate, rp.Score, COALESCE(rcr.CloseReasons, ARRAY['No recent close reasons']) AS CloseReasons,
//        CASE WHEN rp.AnswerCount = 0 THEN 'No Answers Yet' WHEN rp.AnswerCount >= 5 THEN 'Popular Question' ELSE 'Moderate Engagement' END AS EngagementLevel
// FROM RankedPosts rp LEFT JOIN RecentCloseReasons rcr ON rp.PostId = rcr.PostId WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC LIMIT 50;
//
// A CreationDate tie inside rn goes to the smaller post id, and the distinct reasons are listed in name order (the SQL leaves both open).
fn q4452(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, answer_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).with(score.gt(5)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first = top_n(drain((&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p)), |&(_, p)| Reverse(creation_date.get(p).unwrap()), 50);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.1).collect()).map(|p| p).collect();
    let crt: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, creation_date: hd, comment, post, .. } = &db.post_history;
    let rcr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .with(hd.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt))
        .buf_fold(|v| {
            let mut n: Vec<Str> = v.to_vec();
            n.sort();
            n.dedup();
            &*Box::leak(n.into_boxed_slice())
        });
    let v = drain((&rp).select(Ident::<Post>::new().and((&rcr).opt())));
    rows(v.into_iter().map(|(_, (p, c))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.push(V::L(match c {
            Some(c) => c.iter().map(|&s| V::S(s)).collect(),
            None => vec![V::S("No recent close reasons")],
        }));
        let a = answer_count.get(p);
        f.push(V::S(if a == Some(0) { "No Answers Yet" } else if a.map_or(false, |a| a >= 5) { "Popular Question" } else { "Moderate Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.PostTypeId),
// ClosedPostHistory AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM PostHistory ph
//     JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.UpVotes, rp.DownVotes, ch.CloseDate, ch.CloseReasons
// FROM RankedPosts rp LEFT JOIN ClosedPostHistory ch ON rp.PostId = ch.PostId WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 10;
//
// Rank reads only base columns, so the posts are ranked first. A Score tie inside the rank goes to the smaller post id, and the distinct reasons are joined in name order (the SQL leaves both open).
fn q412(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let ud = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let crt: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, creation_date: hd, comment, post, .. } = &db.post_history;
    let ch = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post.and(hd))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt))
        .buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    let chv = rel(drain(&ch));
    type C = ((Id<Post>, i64), Str);
    let chi: HashIdx<Id<Post>, C> = (&chv).map(|x: C| x.0 .0).inv().select(&chv).collect();
    let v = drain((&rp).select(Ident::<Post>::new().and(&ud).and((&chi).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(_, ((p, a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match c {
            Some(((_, d), n)) => [V::T(d), V::S(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH KeywordCounts AS (SELECT P.Id AS PostId, COUNT(*) AS KeywordCount, STRING_AGG(DISTINCT T.TagName, ', ') AS Tags
//     FROM Posts P JOIN LATERAL unnest(string_to_array(substring(P.Tags, 2, length(P.Tags)-2), '>')) AS T(TagName) ON TRUE WHERE P.PostTypeId = 1 GROUP BY P.Id),
// RecentVotes AS (SELECT V.PostId, COUNT(V.Id) AS VoteCount FROM Votes V WHERE V.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY V.PostId),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(KC.KeywordCount, 0) AS KeywordCount, COALESCE(RV.VoteCount, 0) AS RecentVoteCount
//     FROM Posts P LEFT JOIN KeywordCounts KC ON P.Id = KC.PostId LEFT JOIN RecentVotes RV ON P.Id = RV.PostId)
// SELECT PS.PostId, PS.Title, PS.CreationDate, PS.KeywordCount, PS.RecentVoteCount,
//        CASE WHEN PS.RecentVoteCount > 10 THEN 'Hot' WHEN PS.KeywordCount > 5 THEN 'Trending' ELSE 'Normal' END AS PostStatus
// FROM PostStatistics PS WHERE PS.KeywordCount > 0 ORDER BY PS.RecentVoteCount DESC, PS.KeywordCount DESC;
fn q29037(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let kc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(tags_str.flat_map(|s: Str| s[1..s.len() - 1].split('>'))).fold(0i64, |n, _| n + 1);
    let rv = db.vote.with((&db.vote.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(&db.vote.post).fold(0i64, |n, _| n + 1);
    let v = drain((&kc).and((&rv).opt()));
    rows(v.into_iter().map(|(p, (k, r))| {
        let r = r.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(k), V::I(r), V::S(if r > 10 { "Hot" } else if k > 5 { "Trending" } else { "Normal" })]);
        row(f)
    }))
}

// rewrites/4891.sql (window and STRING_AGG made total):
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT up.DisplayName, COUNT(DISTINCT rp.PostId) AS PostsRanked, SUM(NULLIF(rp.Score, 0)) AS TotalScore, AVG(COALESCE(ua.UpVotes, 0) - COALESCE(ua.DownVotes, 0)) AS AverageVoteBalance,
//        STRING_AGG(DISTINCT CASE WHEN rp.Rank <= 3 THEN rp.Title END, ', ' ORDER BY CASE WHEN rp.Rank <= 3 THEN rp.Title END) AS TopPosts
// FROM RankedPosts rp LEFT JOIN UserActivity ua ON rp.AcceptedAnswerId = ua.UserId LEFT JOIN Users up ON up.Id = rp.AcceptedAnswerId
// WHERE rp.Rank <= 5 GROUP BY up.Id, up.DisplayName HAVING COUNT(DISTINCT rp.PostId) > 0 ORDER BY TotalScore DESC, AverageVoteBalance DESC;
//
// `rp.AcceptedAnswerId = ua.UserId` compares a post id with a user id.
fn q4891(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, title, accepted_answer_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    type R = (Id<Post>, i64);
    let rp: MatSet<R> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), k)| (p, k)).collect();
    let ua = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let k = || Same::<R>::new().map(|x: R| x.0);
    let g = (&rp)
        .group_by(k().select(accepted_answer_id.select(&uidx).opt()))
        .select(Same::<R>::new().and(k().select(score)).and(k().select(title.opt())).and(k().select(accepted_answer_id.select(&uidx).select(&ua)).opt()))
        .buf_fold(|v| {
            let nz: Vec<i64> = v.iter().map(|x| x.0 .0 .1).filter(|&s| s != 0).collect();
            let bal: i64 = v.iter().map(|x| x.1.map_or(0, |a| a[0] - a[1])).sum();
            let t: Vec<Str> = v.iter().filter(|x| x.0 .0 .0 .1 <= 3).filter_map(|x| x.0 .1).collect();
            (v.len() as i64, if nz.is_empty() { None } else { Some(nz.iter().sum::<i64>()) }, bal, v.len() as i64, agg_distinct(t, ", "))
        });
    rows(drain(&g).into_iter().map(|(u, (n, s, b, m, t))| row(vec![u.map_or(V::Null, |u| user_col(db, u, "name")), V::I(n), oint(s), avg(b, m), ostr(t)])))
}

// WITH PostTagCounts AS (SELECT p.Id AS PostId, COUNT(t.TagName) AS TagCount, STRING_AGG(t.TagName, ', ') AS Tags
//     FROM Posts p JOIN unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS t(TagName) ON TRUE WHERE p.PostTypeId = 1 GROUP BY p.Id),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS QuestionCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// RankedPosts AS (SELECT p.Id, p.Title, pt.TagCount, ups.TotalScore, ups.TotalViews, ups.TotalAnswers, RANK() OVER (ORDER BY ups.TotalScore DESC, pt.TagCount DESC) AS Rank
//     FROM Posts p JOIN PostTagCounts pt ON p.Id = pt.PostId JOIN UserPostStats ups ON p.OwnerUserId = ups.UserId WHERE p.PostTypeId = 1 AND pt.TagCount >= 2)
// SELECT rp.Rank, rp.Title, rp.TagCount, rp.TotalScore, rp.TotalViews, rp.TotalAnswers FROM RankedPosts rp WHERE rp.Rank <= 10 ORDER BY rp.Rank;
fn q25576(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, score, view_count, answer_count, .. } = &db.post;
    let pt = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(tags_str.flat_map(tag_list)).fold(0i64, |n, _| n + 1);
    let ups = db.post.group_by(owner_user).select(score.and(view_count.opt()).and(answer_count.opt())).fold([0i64; 3], |a, ((s, w), n)| [a[0] + s, a[1] + w.unwrap_or(0), a[2] + n.unwrap_or(0)]);
    let w = whole(db.post.with(post_type_id.eq(1)))
        .select(Ident::<Post>::new().and((&pt).filt(|n| n >= 2).and(owner_user.select(&ups))))
        .window(rank, |(_, (n, a))| (Reverse(a[0]), Reverse(n)), asc);
    let v = drain((&w).filt(|(_, k)| k <= 10));
    rows(v.into_iter().map(|(_, ((p, (n, a)), k))| {
        let mut f = vec![V::I(k)];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// FilteredPosts AS (SELECT r.PostId, r.Title, r.Score, r.Rank, COALESCE(r.CommentCount, 0) AS CommentCount, r.UpVotes, r.DownVotes,
//        CASE WHEN r.UpVotes > r.DownVotes THEN 'Positive' WHEN r.DownVotes > r.UpVotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment FROM RankedPosts r WHERE r.Rank <= 5),
// AggPostData AS (SELECT f.Title, SUM(f.Score) AS TotalScore, AVG(f.UpVotes - f.DownVotes) AS AvgVoteDifference, STRING_AGG(f.Sentiment, ', ') AS SentimentList FROM FilteredPosts f GROUP BY f.Title)
// SELECT a.Title, a.TotalScore, a.AvgVoteDifference, CASE WHEN a.AvgVoteDifference > 0 THEN 'Generally Positive' WHEN a.AvgVoteDifference < 0 THEN 'Generally Negative' ELSE 'Balanced' END AS OverallSentiment,
//        CASE WHEN a.TotalScore IS NULL THEN 'No Activity' ELSE 'Active' END AS PostActivity
// FROM AggPostData a WHERE EXISTS (SELECT 1 FROM Posts p WHERE p.Title LIKE '%' || a.Title || '%'
//     AND p.CreationDate BETWEEN cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND cast('2024-10-01 12:34:56' as timestamp))
// ORDER BY a.TotalScore DESC NULLS LAST;
//
// Rank reads only base columns, so the posts are ranked first (a tie goes to the smaller post id; the SQL leaves it open).
fn q21230(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, post_type_id, score, title, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let ud = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let agg = (&fp).group_by(title.opt()).select(score.and(&ud)).fold([0i64; 3], |a, (s, u)| [a[0] + s, a[1] + u[0] - u[1], a[2] + 1]);
    let recent: MatSet<Str> = db.post.with(creation_date.between(add_days(t0, -30), t0)).select(title).collect();
    type A = (Option<Str>, [i64; 3]);
    let v = drain(rel(drain(&agg)).with(Same::<A>::new().flat_map(|x: A| x.0).select_where(&recent, |a: Str, t: Str| like(t, &format!("%{a}%")))));
    rows(v.into_iter().map(|(_, (t, a))| {
        let d = a[1] as f64 / a[2] as f64;
        row(vec![ostr(t), V::I(a[0]), V::F(d), V::S(if d > 0.0 { "Generally Positive" } else if d < 0.0 { "Generally Negative" } else { "Balanced" }), V::S("Active")])
    }))
}

// WITH RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, U.DisplayName AS OwnerDisplayName, COALESCE(V.UpVotes, 0) AS UpVotes,
//        COALESCE(V.DownVotes, 0) AS DownVotes, DENSE_RANK() OVER (PARTITION BY EXTRACT(YEAR FROM P.CreationDate) ORDER BY P.CreationDate DESC) AS YearRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) V ON P.Id = V.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, OwnerDisplayName, UpVotes, DownVotes FROM RecentPosts WHERE YearRank <= 10),
// PostDetails AS (SELECT T.*, PH.PostHistoryTypeId, PH.CreationDate AS HistoryDate, PH.Comment AS CloseReason FROM TopPosts T
//     LEFT JOIN PostHistory PH ON T.PostId = PH.PostId AND PH.PostHistoryTypeId IN (10, 11))
// SELECT PD.Title, PD.OwnerDisplayName, PD.CreationDate, PD.Score, PD.ViewCount, PD.AnswerCount, CASE WHEN PD.CloseReason IS NOT NULL THEN 'Closed: ' || PD.CloseReason ELSE 'Active' END AS PostStatus,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = PD.PostId) AS CommentCount, STRING_AGG(T.TagName, ', ') AS Tags
// FROM PostDetails PD LEFT JOIN PostLinks PL ON PD.PostId = PL.PostId LEFT JOIN Tags T ON PL.RelatedPostId = T.Id
// GROUP BY PD.PostId, PD.OwnerDisplayName, PD.Title, PD.CreationDate, PD.Score, PD.ViewCount, PD.AnswerCount, PD.CloseReason ORDER BY PD.Score DESC, PD.ViewCount DESC;
//
// YearRank reads only CreationDate, so the posts are ranked first. `PL.RelatedPostId = T.Id` compares a post id with a tag id. The STRING_AGG order is left open; the port joins in link id order.
fn q255(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(creation_date.map(year))
        .select(Ident::<Post>::new().and(creation_date))
        .window(dense_rank, |(_, d)| Reverse(d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    type K = (Id<Post>, Option<Str>);
    let pd = (&tp).select(
        Ident::<Post>::new().and(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))).select(comment.opt()).opt().map(|c: Option<Option<Str>>| c.flatten())),
    );
    let tidx: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let g = pd
        .group_by(Same::<K>::new())
        .select(Same::<K>::new().map(|x: K| x.0).select(links_of(db).select((&db.post_link.related_post_id).select(&tidx).select(&db.tag.tag_name).opt()).opt()))
        .buf_fold(|v| {
            let t: Vec<&str> = v.iter().filter_map(|x| x.flatten()).collect();
            if t.is_empty() { None } else { Some(leak(t.join(", "))) }
        });
    let cc = comments_per_post(db);
    let v = drain((&g).and(Same::<K>::new().map(|k: K| k.0).select(&cc)));
    rows(v.into_iter().map(|((p, c), (t, n))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views", "answers"]);
        f.push(match c {
            Some(c) => V::Owned(format!("Closed: {c}")),
            None => V::S("Active"),
        });
        f.extend([V::I(n), ostr(t)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, COUNT(DISTINCT B.Id) AS BadgeCount, COUNT(DISTINCT P.Id) AS PostCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes),
// PostTags AS (SELECT P.Id AS PostId, unnest(string_to_array(substring(P.Tags, 2, length(P.Tags) - 2), '><')) AS Tag FROM Posts P WHERE P.Tags IS NOT NULL),
// TagStats AS (SELECT PT.Tag, COUNT(DISTINCT PT.PostId) AS PostCount, COUNT(DISTINCT U.Id) AS UserCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM PostTags PT JOIN Posts P ON PT.PostId = P.Id LEFT JOIN Users U ON P.OwnerUserId = U.Id GROUP BY PT.Tag),
// TopTags AS (SELECT Tag, PostCount, UserCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagStats)
// SELECT US.UserId, US.DisplayName, US.Reputation, US.Views, US.BadgeCount, TT.Tag, TT.PostCount AS TagPostCount, TT.UserCount AS TagUserCount, TT.QuestionCount AS TagQuestionCount,
//        TT.AnswerCount AS TagAnswerCount
// FROM UserStats US JOIN TopTags TT ON TT.UserCount > 0 WHERE US.Reputation > 1000 ORDER BY TT.TagRank, US.Reputation DESC, US.Views DESC LIMIT 10;
//
// Only BadgeCount is read from UserStats, and COUNT(DISTINCT B.Id) is the user's badge count, so the posts x votes product is not driven. The ORDER BY
// leads with TagRank and then the user's (Reputation, Views), so only the ten best-ranked tags with users and the ten best-ranked users can reach the LIMIT.
fn q27355(db: &'static So) -> String {
    let Post { tags_str, post_type_id, owner_user, .. } = &db.post;
    let pt = || db.post.select(tags_str.flat_map(tag_list)).inv();
    let tn = pt().count_distinct();
    let tu = pt().select(owner_user).count_distinct();
    let tq = pt().select(post_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64]);
    let tags: MatSet<Str> = db.post.select(tags_str.flat_map(tag_list)).collect();
    let tr = whole(&tags).select(Same::<Str>::new().and(&tn)).window(rank, |(_, n)| Reverse(n), asc);
    let trk = by_first(&(&tr).map(|((t, _), r)| (t, r)).collect());
    let tw = whole((&tags).with((&tu).filt(|u| u > 0))).select(Same::<Str>::new().and(&tn)).window(rank, |(_, n)| Reverse(n), asc);
    let tt: MatSet<Str> = (&tw).filt(|(_, k)| k <= 10).map(|((t, _), _)| t).collect();
    let User { reputation, views, .. } = &db.user;
    let uw = whole(db.user.with(reputation.gt(1000))).select(Ident::<User>::new().and(reputation).and(views)).window(rank, |((_, r), w)| (Reverse(r), Reverse(w)), asc);
    let tus: MatSet<Id<User>> = (&uw).filt(|(_, k)| k <= 10).map(|(((u, _), _), _)| u).collect();
    let bc = (&tus).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&tt).select(Same::<Str>::new().and(&trk).and((&tn).and(&tu).and(&tq))).cross(&bc));
    let v = top_n(v, |&((_, u), (((_, r), _), _))| (r, Reverse(reputation.get(u).unwrap()), Reverse(views.get(u).unwrap())), 10);
    rows(v.into_iter().map(|((_, u), (((t, _), ((n, c), a)), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend([V::I(b), V::S(t), V::I(n), V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn, DENSE_RANK() OVER (ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 AND u.Reputation > 100
//     GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.Score),
// FilteredPosts AS (SELECT rp.Id, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.CommentCount, COALESCE(LENGTH(rp.Title), 0) AS TitleLength,
//        CASE WHEN rp.CommentCount > 5 THEN 'Highly Discussed' WHEN rp.CommentCount BETWEEN 1 AND 5 THEN 'Moderately Discussed' ELSE 'Not Discussed' END AS DiscussionLevel
//     FROM RankedPosts rp WHERE rp.rn = 1 AND rp.PostRank <= 10),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS HistoryCount, STRING_AGG(DISTINCT pht.Name, ', ') AS HistoryTypes FROM PostHistory ph
//     JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT fp.Title, fp.OwnerDisplayName, fp.CreationDate, fp.CommentCount, fp.TitleLength, fp.DiscussionLevel, COALESCE(phc.HistoryCount, 0) AS PostHistoryCount,
//        COALESCE(phc.HistoryTypes, 'None') AS PostHistoryTypes, (SELECT COUNT(DISTINCT v.Id) FROM Votes v WHERE v.PostId = fp.Id AND v.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(DISTINCT v.Id) FROM Votes v WHERE v.PostId = fp.Id AND v.VoteTypeId = 3) AS DownVotes
// FROM FilteredPosts fp LEFT JOIN PostHistoryCounts phc ON fp.Id = phc.PostId
// WHERE fp.CommentCount IS NOT NULL AND (fp.TitleLength > 50 OR fp.DiscussionLevel = 'Highly Discussed') ORDER BY fp.CreationDate DESC LIMIT 50 OFFSET 10;
//
// PostRank reads only Score, so the posts are ranked first; rn is 1 for every post. The distinct type names are joined in name order (the SQL leaves it open).
fn q21250(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, title, creation_date, .. } = &db.post;
    let w = whole(db.post.with(post_type_id.eq(1)).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100)))))
        .select(Ident::<Post>::new().and(score))
        .window(dense_rank, |(_, s)| Reverse(s), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let cc = comments_per_post(db);
    let tl = |t: Option<Str>| t.map_or(0, |t| t.chars().count() as i64);
    let phc = db.post_history.group_by(&db.post_history.post).select(htype_name(db)).buf_fold(|v| (v.len() as i64, agg_distinct(v.to_vec(), ", ").unwrap()));
    let up = votes_of_type(db, 2);
    let dn = votes_of_type(db, 3);
    let v = drain((&fp).select(Ident::<Post>::new().and(title.opt()).and(&cc).filt(move |((_, t), c): ((Id<Post>, Option<Str>), i64)| tl(t) > 50 || c > 5).and((&phc).opt()).and(&up).and(&dn)));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 60);
    rows(v.into_iter().skip(10).map(|(_, (((((p, t), c), h), u), d))| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend([V::I(c), V::I(tl(t)), V::S(if c > 5 { "Highly Discussed" } else if c >= 1 { "Moderately Discussed" } else { "Not Discussed" })]);
        f.extend([V::I(h.map_or(0, |h| h.0)), V::S(h.map_or("None", |h| h.1)), V::I(u), V::I(d)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.OwnerUserId, p.PostTypeId,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankViews
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostAnalysis AS (SELECT r.PostId, r.Title, r.CreationDate, r.ViewCount, r.Score, r.AnswerCount, r.RankScore, r.RankViews, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName,
//        COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM RankedPosts r LEFT JOIN Users u ON r.OwnerUserId = u.Id LEFT JOIN Comments c ON r.PostId = c.PostId LEFT JOIN Votes v ON r.PostId = v.PostId
//     GROUP BY r.PostId, r.Title, r.CreationDate, r.ViewCount, r.Score, r.AnswerCount, r.RankScore, r.RankViews, u.DisplayName),
// ClosedPostHistory AS (SELECT ph.PostId, ph.CreationDate, STRING_AGG(CONCAT(ph.Comment, ' on ', ph.CreationDate), '; ') AS ClosureDetails FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (10, 11, 12) GROUP BY ph.PostId, ph.CreationDate)
// SELECT pa.PostId, pa.Title, pa.CreationDate, pa.ViewCount, pa.Score, pa.AnswerCount, pa.RankScore, pa.RankViews, pa.OwnerDisplayName, pa.CommentCount, pa.UpVotes, pa.DownVotes,
//        COALESCE(cph.ClosureDetails, 'No closure history') AS ClosureHistory
// FROM PostAnalysis pa LEFT JOIN ClosedPostHistory cph ON pa.PostId = cph.PostId WHERE pa.RankScore <= 10 OR pa.RankViews <= 10 ORDER BY pa.RankScore, pa.RankViews DESC;
//
// Both ranks read only base columns, so the posts are ranked first and the comments x votes product is driven for them alone. The STRING_AGG order within one instant is left open; the port joins in history id order.
fn q33964(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(post_type_id.is_in([1, 2])).group_by(post_type_id);
    let sw = base().select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let vw = base().select(Ident::<Post>::new().and(view_count.opt())).window(rank, |(_, w)| (w.is_none(), Reverse(w)), asc);
    let rs = by_first(&(&sw).map(|((p, _), k)| (p, k)).collect());
    let rw = by_first(&(&vw).map(|((p, _), k)| (p, k)).collect());
    let pa: MatSet<Id<Post>> = (&sw).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).union((&vw).filt(|(_, k)| k <= 10).map(|((p, _), _)| p)).collect();
    let nc = comments_per_post(db);
    let pv = (&pa).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, comment, post, .. } = &db.post_history;
    let cph = db
        .post_history
        .with(post_history_type_id.is_in([10, 11, 12]))
        .group_by(post.and(hd))
        .select(comment.opt().and(hd))
        .buf_fold(|v| leak(v.iter().map(|(c, d)| format!("{} on {}", c.unwrap_or(""), ts_text(*d))).collect::<Vec<_>>().join("; ")));
    let cv = rel(drain(&cph));
    type C = ((Id<Post>, i64), Str);
    let ci: HashIdx<Id<Post>, Str> = (&cv).map(|x: C| x.0 .0).inv().select((&cv).map(|x: C| x.1)).collect();
    let v = drain((&pa).select(Ident::<Post>::new().and(&rs).and(&rw).and(&pv).and(&nc).and(ci.opt())));
    rows(v.into_iter().map(|(_, (((((p, rs), rw), a), n), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.extend([V::I(rs), V::I(rw)]);
        f.push(match db.post.owner_user.get(p) {
            Some(u) => user_col(db, u, "name"),
            None => V::S("Anonymous"),
        });
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("No closure history"))]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostAnalytics AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionsCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswersCount,
//        SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(P.ViewCount) AS TotalViews, P.CreationDate FROM Posts P
//     WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY P.OwnerUserId, P.CreationDate),
// ClosedPostReasons AS (SELECT PH.UserId, COUNT(*) AS CloseCount, STRING_AGG(DISTINCT CRT.Name, ', ') AS CloseReasons FROM PostHistory PH
//     JOIN CloseReasonTypes CRT ON CAST(PH.Comment AS INT) = CRT.Id WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.UserId),
// UserPerformance AS (SELECT UB.UserId, UB.DisplayName, COALESCE(PA.QuestionsCount, 0) AS Questions, COALESCE(PA.AnswersCount, 0) AS Answers, COALESCE(PA.TotalScore, 0) AS TotalScore,
//        COALESCE(PA.TotalViews, 0) AS TotalViews, COALESCE(CPR.CloseCount, 0) AS ClosedPosts, COALESCE(CPR.CloseReasons, 'None') AS CloseReasons, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges
//     FROM UserBadges UB LEFT JOIN PostAnalytics PA ON UB.UserId = PA.OwnerUserId LEFT JOIN ClosedPostReasons CPR ON UB.UserId = CPR.UserId)
// SELECT UPerformance.*, RANK() OVER (ORDER BY UPerformance.TotalScore DESC) AS ScoreRank FROM UserPerformance UPerformance WHERE UPerformance.Questions > 5
// ORDER BY UPerformance.TotalViews DESC, UPerformance.TotalScore DESC;
//
// Questions > 5 needs a PostAnalytics row, so the users are reached from those rows. The distinct reasons are joined in name order (the SQL leaves it open).
fn q325(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let pa = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user.and(creation_date))
        .select(post_type_id.and(score).and(view_count.opt()))
        .fold([0i64; 5], |a, ((t, s), w)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let crt: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, user, .. } = &db.post_history;
    let cpr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(user)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt))
        .buf_fold(|v| (v.len() as i64, agg_distinct(v.to_vec(), ", ").unwrap()));
    type K = (Id<User>, i64);
    type R = (K, [i64; 5]);
    let rv = rel(drain((&pa).filt(|a| a[0] > 5)));
    let up = Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(&ub)).and(Same::<R>::new().map(|x: R| x.0 .0).select(&cpr).opt());
    let w = whole(&rv).select((&rv).select(up)).window(rank, |(((_, a), _), _)| Reverse(a[2]), asc);
    rows(drain(&w).into_iter().map(|(_, (((((u, _), a), b), c), k))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[4])]);
        f.extend(match c {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::I(0), V::S("None")],
        });
        f.extend(b.map(V::I));
        f.push(V::I(k));
        row(f)
    }))
}

// WITH RECURSIVE UserPostCounts AS (SELECT U.Id AS UserId, COUNT(P.Id) AS PostCount, RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS UserRank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, U.DisplayName AS AuthorName, COALESCE(PH.UserDisplayName, 'N/A') AS LastEditedBy, PH.CreationDate AS LastEditDate, P.ViewCount,
//        (SELECT COUNT(C.Id) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount, (SELECT STRING_AGG(T.TagName, ', ') FROM Tags T WHERE T.WikiPostId = P.Id) AS TagNames
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = P.Id)
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPostAuthors AS (SELECT U.Id AS UserId, U.DisplayName, SUM(PD.ViewCount) AS TotalViews, SUM(PD.Score) AS TotalScore, (SELECT COUNT(*) FROM Posts P WHERE P.OwnerUserId = U.Id) AS PostCount
//     FROM Users U INNER JOIN PostDetails PD ON U.DisplayName = PD.AuthorName GROUP BY U.Id, U.DisplayName)
// SELECT UP.UserId, UP.UserRank, TAP.DisplayName, TAP.TotalViews, TAP.TotalScore, TAP.PostCount, PD.PostId, PD.Title, PD.CreationDate, PD.CommentCount, PD.TagNames,
//        CASE WHEN PD.Score > 10 THEN 'High Score' WHEN PD.Score BETWEEN 1 AND 10 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory
// FROM UserPostCounts UP JOIN TopPostAuthors TAP ON UP.UserId = TAP.UserId JOIN PostDetails PD ON TAP.DisplayName = PD.AuthorName WHERE TAP.TotalViews IS NOT NULL ORDER BY UP.UserRank, TAP.TotalScore DESC;
//
// No CTE refers to itself, so RECURSIVE changes nothing. PostDetails keeps one row per history row at the post's latest instant. The tag names are joined in tag id order (the SQL leaves it open).
fn q33629(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let upc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let uw = whole(&db.user.id).select(Ident::<User>::new().and(&upc)).window(rank, |(_, n)| Reverse(n), asc);
    let rki = by_first(&(&uw).map(|((u, _), k)| (u, k)).collect());
    let hm = history_max_date(db);
    let hd = &db.post_history.creation_date;
    let atmax = Ident::<Post>::new().and(&hm).and(history_of(db).select(Ident::<PostHistory>::new().and(hd))).filt(|((_, m), (_, d)): ((Id<Post>, i64), (Id<PostHistory>, i64))| m == d).map(|(_, (h, _))| h);
    type R = (Id<Post>, Option<Id<PostHistory>>);
    let pdv: MatSet<R> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(Ident::<Post>::new().and(atmax.opt())).collect();
    let dn = &db.user.display_name;
    let by_name: HashIdx<Str, R> = (&pdv).map(|x: R| x.0).select(owner_user).select(dn).inv().select(&pdv).collect();
    let tap = db.user.group_by(Ident::<User>::new()).select(dn.select(&by_name).map(|x: R| x.0).select(score.and(view_count.opt()))).fold([0i64; 3], |a, (s, w)| [a[0] + s, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let tags = db.post.group_by(Ident::<Post>::new()).select((&wiki).select(&db.tag.tag_name)).buf_fold(|v| leak(v.join(", ")));
    let cc = comments_per_post(db);
    let v = drain((&tap).filt(|a| a[1] > 0).and(&rki).and((&np).opt()).and(dn.select(&by_name).map(|x: R| x.0).select(Ident::<Post>::new().and(&cc).and((&tags).opt()))));
    rows(v.into_iter().map(|(u, (((a, k), n), ((p, c), t)))| {
        let s = score.get(p).unwrap();
        let mut f = vec![user_col(db, u, "uid"), V::I(k), user_col(db, u, "name"), V::I(a[2]), V::I(a[0]), V::I(n.unwrap_or(0))];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(c), ostr(t), V::S(if s > 10 { "High Score" } else if s >= 1 { "Moderate Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(DISTINCT p.Id) AS PostsCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId
//     LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// ClosedPostReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.Score, ua.Upvotes, ua.Downvotes, COALESCE(cpr.CloseReasons, 'No Close Reasons') AS CloseReasons, rp.CreationDate
//     FROM RankedPosts rp JOIN UserActivity ua ON rp.PostId = ua.UserId LEFT JOIN ClosedPostReasons cpr ON rp.PostId = cpr.PostId WHERE rp.PostRank <= 3)
// SELECT fr.PostId, fr.Title, fr.Score, fr.Upvotes, fr.Downvotes, fr.CloseReasons, CASE WHEN fr.CloseReasons = 'No Close Reasons' THEN 'Active' ELSE 'Closed' END AS PostStatus,
//        EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - fr.CreationDate)) / 3600 AS AgeInHours
// FROM FinalResults fr ORDER BY fr.Score DESC, fr.Title ASC;
//
// PostRank reads only Score, so the posts are ranked first. `rp.PostId = ua.UserId` compares a post id with a user id, and the votes x posts product is driven for those users alone.
// The STRING_AGG order is left open; the port joins in history id order.
fn q22506(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 3).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let us: MatSet<Id<User>> = (&rp).select(origid).select(&uidx).collect();
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(t0, -1)));
    let ua = (&us).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(recent).opt())).fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let crt: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, post, .. } = &db.post_history;
    let cpr = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt)).buf_fold(|v| leak(v.join(", ")));
    let v = drain((&rp).select(Ident::<Post>::new().and(origid.select(&uidx).select(&ua)).and((&cpr).opt())));
    rows(v.into_iter().map(|(_, ((p, a), c))| {
        let c = c.unwrap_or("No Close Reasons");
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(c), V::S(if c == "No Close Reasons" { "Active" } else { "Closed" })]);
        f.push(V::F(epoch(t0 - creation_date.get(p).unwrap()) / 3600.0));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS TotalUpVotes, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 years' GROUP BY u.Id, u.DisplayName),
// PostHistoryDetails AS (SELECT ph.PostId, STRING_AGG(DISTINCT pht.Name || ': ' || ph.Comment, '; ') AS HistoryComments, COUNT(*) AS HistoryCount FROM PostHistory ph
//     JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, u.DisplayName AS OwnerName, ua.TotalPosts, ua.TotalScore, COALESCE(phd.HistoryComments, 'No edits or comments') AS EditHistory,
//        phd.HistoryCount AS TotalHistory, CASE WHEN ua.TotalScore > 100 THEN 'Expert Contributor' WHEN ua.TotalPosts > 50 THEN 'Regular Contributor' ELSE 'New Contributor' END AS ContributorType
// FROM RankedPosts rp JOIN Users u ON rp.AcceptedAnswerId = u.Id LEFT JOIN UserActivity ua ON u.Id = ua.UserId LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId
// WHERE rp.RankByScore <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// RankByScore reads only base columns, so the posts are ranked first (a Score tie goes to the smaller post id; the SQL leaves it open). `rp.AcceptedAnswerId = u.Id` compares a post id
// with a user id, and UserActivity is driven for those users alone. The distinct history strings are joined in sorted order.
fn q22249(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, post_type_id, score, accepted_answer_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .with(score.gt(0))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let us: MatSet<Id<User>> = (&rp).select(accepted_answer_id).select(&uidx).collect();
    let uu = || (&us).with((&db.user.creation_date).ge(add_years(t0, -2))).group_by(Ident::<User>::new());
    let np = uu().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tsc = uu().select(posts_of(db).select(score.and(votes_of(db).opt())).opt()).fold(0i64, |n, x| n + x.map_or(0, |x| x.0));
    let ua = (&np).and(&tsc);
    let PostHistory { creation_date: hd, comment, post, .. } = &db.post_history;
    let phd = db
        .post_history
        .with(hd.ge(add_months(t0, -6)))
        .group_by(post)
        .select(htype_name(db).and(comment.opt()))
        .buf_fold(|v| (v.len() as i64, agg_distinct(v.iter().filter_map(|&(n, c)| c.map(|c| leak(format!("{n}: {c}")))).collect(), "; ")));
    let v = drain((&rp).select(Ident::<Post>::new().and(accepted_answer_id.select(&uidx).select(Ident::<User>::new().and((&ua).opt()))).and((&phd).opt())));
    rows(v.into_iter().map(|(_, ((p, (u, a)), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(user_col(db, u, "name"));
        f.extend(match a {
            Some((n, s)) => [V::I(n), V::I(s)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(h.and_then(|h| h.1).unwrap_or("No edits or comments")));
        f.push(h.map_or(V::Null, |h| V::I(h.0)));
        f.push(V::S(match a {
            Some((_, s)) if s > 100 => "Expert Contributor",
            Some((n, _)) if n > 50 => "Regular Contributor",
            _ => "New Contributor",
        }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, MAX(p.CreationDate) AS MostRecentActivity
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.OwnerUserId),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(DISTINCT crt.Name, ', ') AS CloseReasons, MIN(ph.CreationDate) AS FirstClosedDate FROM PostHistory ph
//     JOIN CloseReasonTypes crt ON ph.Comment = crt.Id::text WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// PostAnalytics AS (SELECT ps.PostId, ps.Title, ur.DisplayName AS OwnerName, ur.Reputation AS OwnerReputation, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount,
//        COALESCE(clp.CloseReasons, 'No Reasons') AS CloseReasons, COALESCE(clp.FirstClosedDate::text, 'Open') AS PostStatus,
//        RANK() OVER (PARTITION BY CASE WHEN clp.CloseReasons IS NOT NULL THEN 1 ELSE 0 END ORDER BY ps.UpVoteCount DESC) AS VoteRank
//     FROM PostStats ps JOIN UserReputation ur ON ps.OwnerUserId = ur.UserId LEFT JOIN ClosedPosts clp ON ps.PostId = clp.PostId)
// SELECT pa.Title, pa.OwnerName, pa.OwnerReputation, pa.UpVoteCount, pa.DownVoteCount, pa.CommentCount, pa.CloseReasons, pa.PostStatus,
//        CASE WHEN pa.CloseReasons != 'No Reasons' THEN 'Closed' ELSE 'Active' END AS PostLifecycle, CASE WHEN pa.OwnerReputation = 0 THEN 'Newbie' ELSE 'Established' END AS UserType
// FROM PostAnalytics pa WHERE (pa.UpVoteCount + pa.CommentCount) > 10 ORDER BY pa.VoteRank, pa.OwnerReputation DESC, pa.UpVoteCount DESC LIMIT 20;
//
// The distinct reasons are joined in name order (the SQL leaves it open).
fn q21397(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let crt: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i: i64| -> Str { leak(i.to_string()) }).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, post, .. } = &db.post_history;
    let clp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.select(&crt).and(hd))
        .buf_fold(|v| (agg_distinct(v.iter().map(|x| x.0).collect(), ", ").unwrap(), v.iter().map(|x| x.1).min().unwrap()));
    let w = db
        .post
        .with(owner_user)
        .group_by((&clp).opt().map(|c: Option<(Str, i64)>| c.is_some()))
        .select(Ident::<Post>::new().and((&ps).and((&clp).opt())))
        .window(rank, |(_, (a, _))| Reverse(a[1]), asc);
    let rep = |p: Id<Post>| db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
    let v = drain((&w).filt(|((_, (a, _)), _)| a[1] + a[0] > 10));
    let v = top_n(v, |&(_, ((p, (a, _)), k))| (k, Reverse(rep(p)), Reverse(a[1])), 20);
    rows(v.into_iter().map(|(_, ((p, (a, c)), _))| {
        let r = rep(p);
        let mut f = post_fields(db, p, &["title", "owner", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[0])]);
        match c {
            Some((n, d)) => f.extend([V::S(n), V::Owned(ts_text(d)), V::S("Closed")]),
            None => f.extend([V::S("No Reasons"), V::S("Open"), V::S("Active")]),
        }
        f.push(V::S(if r == 0 { "Newbie" } else { "Established" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS Rank, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.Id) AS UpVoteCount,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.Id) AS DownVoteCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND (P.ViewCount > 100 OR P.Score > 10)),
// ClosedPostHistory AS (SELECT PH.PostId, PH.Comment, MAX(PH.CreationDate) AS LatestCloseDate, STRING_AGG(CASE WHEN PHT.Name IS NOT NULL THEN PHT.Name ELSE 'Unknown' END, ', ') AS CloseReasons
//     FROM PostHistory PH LEFT JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId, PH.Comment),
// FinalRankings AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, RP.Rank, RP.UpVoteCount, RP.DownVoteCount,
//        COALESCE(CPH.LatestCloseDate, NULL) AS LatestCloseDate, COALESCE(CPH.CloseReasons, 'Not Closed') AS CloseReasons FROM RankedPosts RP LEFT JOIN ClosedPostHistory CPH ON RP.PostId = CPH.PostId)
// SELECT FR.PostId, FR.Title, FR.Score, FR.ViewCount, FR.OwnerDisplayName, CASE WHEN FR.LatestCloseDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus, FR.CloseReasons,
//        FR.UpVoteCount - FR.DownVoteCount AS NetVotes, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = FR.PostId) AS CommentCount
// FROM FinalRankings FR WHERE FR.Rank <= 5 AND (FR.UpVoteCount - FR.DownVoteCount) > 0 ORDER BY FR.Score DESC, FR.ViewCount DESC;
//
// Rank numbers the posts x votes rows, which differ in nothing projected within one post, so the rows are ranked first. A CreationDate tie between posts goes to the smaller post id,
// and the STRING_AGG joins in history id order (the SQL leaves both open).
fn q20607(db: &'static So) -> String {
    let Post { creation_date, view_count, score, post_type_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(view_count.gt(100).or(score.gt(10)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date).and(votes_of(db).opt()))
        .window(row_number, |((p, d), v)| (Reverse(d), p, v), asc);
    let rp = || (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p);
    let ud = votes_of(db).select(&db.vote.vote_type_id);
    let tp: MatSet<Id<Post>> = rp().collect();
    let uds = (&tp).group_by(Ident::<Post>::new()).select(ud.opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, comment, creation_date: hd, post, .. } = &db.post_history;
    let cph = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(comment.opt()))
        .select(hd.and(htype_name(db)))
        .buf_fold(|v| (v.iter().map(|x| x.0).max().unwrap(), leak(v.iter().map(|x| x.1).collect::<Vec<_>>().join(", "))));
    let cv = rel(drain(&cph));
    type C = ((Id<Post>, Option<Str>), (i64, Str));
    let ci: HashIdx<Id<Post>, (i64, Str)> = (&cv).map(|x: C| x.0 .0).inv().select((&cv).map(|x: C| x.1)).collect();
    let cc = comments_per_post(db);
    let v = drain(rp().select(Ident::<Post>::new().and(&uds).filt(|(_, a): (Id<Post>, [i64; 2])| a[0] - a[1] > 0).and((&ci).opt()).and(&cc)));
    rows(v.into_iter().map(|(_, (((p, a), c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::S(if c.is_some() { "Closed" } else { "Open" }), V::S(c.map_or("Not Closed", |c| c.1)), V::I(a[0] - a[1]), V::I(n)]);
        row(f)
    }))
}

// rewrites/20856.sql (window, STRING_AGG and ORDER BY made total):
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id) AS RankByScore,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPosts FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN v.VoteTypeId = 4 THEN 1 ELSE 0 END) AS OffensiveVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// OutstandingBadges AS (SELECT b.UserId, b.Name, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Class IN (1, 2) GROUP BY b.UserId, b.Name),
// PostHistoryDetails AS (SELECT ph.PostId, STRING_AGG(CONCAT(ph.UserDisplayName, ' (', ph.CreationDate, ') - ', ph.Comment), ' | ' ORDER BY ph.Id) AS EditHistory FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.PostId)
// SELECT u.DisplayName, COUNT(DISTINCT rp.PostId) AS NumberOfPosts, COALESCE(SUM(CASE WHEN uvs.UpVotes > uvs.DownVotes THEN uvs.UpVotes ELSE 0 END), 0) AS NetUpvoteScore,
//        COALESCE(SUM(CASE WHEN uvs.UpVotes < uvs.DownVotes THEN uvs.DownVotes ELSE 0 END), 0) AS NetDownvoteScore, ob.BadgeCount, pvd.EditHistory
// FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN UserVoteStats uvs ON u.Id = uvs.UserId LEFT JOIN OutstandingBadges ob ON u.Id = ob.UserId
// LEFT JOIN PostHistoryDetails pvd ON rp.PostId = pvd.PostId WHERE rp.RankByScore = 1 OR rp.TotalPosts > 3 GROUP BY u.Id, u.DisplayName, ob.BadgeCount, pvd.EditHistory
// HAVING COUNT(DISTINCT rp.PostId) > 0 ORDER BY NumberOfPosts DESC, NetUpvoteScore DESC, u.Id, ob.BadgeCount, pvd.EditHistory LIMIT 50;
//
// RankByScore and TotalPosts read only base columns, so the posts are ranked first.
fn q20856(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let tot = recent().group_by(owner_user_id.opt()).fold(0i64, |n, _| n + 1);
    let w = recent().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let kp: MatSet<Id<Post>> = (&w).and(&tot).filt(|((_, k), n)| k == 1 || n > 3).map(|(((p, _), _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&kp).select(owner_user).collect();
    let uvs = (&owners).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let Badge { class, user, name, .. } = &db.badge;
    let ob = db.badge.with(class.is_in([1, 2])).group_by(user.and(name)).fold(0i64, |n, _| n + 1);
    let obv = rel(drain(&ob));
    type B = ((Id<User>, Str), i64);
    let obi: HashIdx<Id<User>, i64> = (&obv).map(|x: B| x.0 .0).inv().select((&obv).map(|x: B| x.1)).collect();
    let PostHistory { post_history_type_id, user_display_name, creation_date: hd, comment, post, .. } = &db.post_history;
    let pvd = db
        .post_history
        .with(post_history_type_id.is_in([4, 5]))
        .group_by(post)
        .select(user_display_name.opt().and(hd).and(comment.opt()))
        .buf_fold(|v| leak(v.iter().map(|((n, d), c)| format!("{} ({}) - {}", n.unwrap_or(""), ts_text(*d), c.unwrap_or(""))).collect::<Vec<_>>().join(" | ")));
    type X = (Id<Post>, ((Id<User>, Option<i64>), Option<Str>));
    let jr = || (&kp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and((&obi).opt())).and((&pvd).opt())));
    let key = || Same::<X>::new().map(|(_, ((u, b), e)): X| (u, b, e));
    let nv = jr()
        .group_by(key())
        .select(Same::<X>::new().map(|(_, ((u, _), _)): X| u).select(&uvs))
        .fold((0i64, 0i64), |(up, dn), a| (up + if a[0] > a[1] { a[0] } else { 0 }, dn + if a[0] < a[1] { a[1] } else { 0 }));
    let np = jr().group_by(key()).select(Same::<X>::new().map(|(p, _): X| p)).count_distinct();
    let g = (&np).and(&nv).map(|(n, (up, dn))| (n, up, dn));
    let v = top_n(drain(&g), |&((u, b, e), (n, up, _))| (Reverse(n), Reverse(up), db.user.origid.get(u).unwrap(), b.is_none(), b, e.is_none(), e), 50);
    rows(v.into_iter().map(|((u, b, e), (n, up, dn))| row(vec![user_col(db, u, "name"), V::I(n), V::I(up), V::I(dn), oint(b), ostr(e)])))
}

// WITH ActiveUsers AS (SELECT Id, Reputation, DisplayName, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank,
//        COUNT(DISTINCT CASE WHEN CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN Id END) AS RecentActivity
//     FROM Users GROUP BY Id, Reputation, DisplayName HAVING SUM(UpVotes) > 100 OR SUM(DownVotes) < 20),
// PostDetails AS (SELECT P.Id AS PostId, P.PostTypeId, P.AcceptedAnswerId, P.OwnerUserId, COALESCE(P.Score, 0) AS PostScore, COALESCE(P.ViewCount, 0) AS PostViews, COUNT(C.Id) AS CommentCount,
//        COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpvoteCount, P.CreationDate, P.Title
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
//     GROUP BY P.Id, P.PostTypeId, P.AcceptedAnswerId, P.OwnerUserId, P.Score, P.ViewCount, P.Title, P.CreationDate),
// PostHistoryDetails AS (SELECT PH.PostId, MAX(PH.CreationDate) AS LastHistoryDate, STRING_AGG(PHT.Name, ', ') AS HistoryTypes FROM PostHistory PH
//     JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id GROUP BY PH.PostId),
// FilteredPosts AS (SELECT PD.*, COALESCE(PHD.HistoryTypes, 'No history') AS PostHistory, RANK() OVER (ORDER BY PD.PostScore DESC, PD.PostViews DESC) AS PostRank
//     FROM PostDetails PD LEFT JOIN PostHistoryDetails PHD ON PD.PostId = PHD.PostId)
// SELECT A.Id AS UserId, A.DisplayName, A.Reputation, COUNT(DISTINCT FP.PostId) AS ActivePostCount, AVG(FP.PostScore) AS AveragePostScore, MIN(FP.CreationDate) AS FirstActivePost,
//        MAX(FP.CreationDate) AS LastActivePost, STRING_AGG(FP.Title, '; ') AS AllActivePostTitles
// FROM ActiveUsers A LEFT JOIN FilteredPosts FP ON FP.OwnerUserId = A.Id WHERE A.RecentActivity > 0 GROUP BY A.Id, A.DisplayName, A.Reputation
// HAVING AVG(FP.PostScore) > 10 ORDER BY A.Reputation DESC, ActivePostCount DESC FETCH FIRST 10 ROWS ONLY;
//
// FilteredPosts is one row per post (PostHistoryDetails has one row per post), and none of its aggregates is read. The STRING_AGG order is left open; the port joins in post id order.
fn q24512(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let User { up_votes, down_votes, creation_date: ucd, reputation, .. } = &db.user;
    let Post { creation_date, score, title, .. } = &db.post;
    let recent = Ident::<Post>::new().with(creation_date.ge(add_months(t0, -1)));
    let g = db
        .user
        .with(up_votes.gt(100).or(down_votes.lt(20)))
        .with(ucd.gt(add_years(t0, -1)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(recent).select(score.and(creation_date).and(title.opt())))
        .buf_fold(|v| {
            let t: Vec<&str> = v.iter().filter_map(|x| x.1).collect();
            (v.len() as i64, v.iter().map(|x| x.0 .0).sum::<i64>(), v.iter().map(|x| x.0 .1).min().unwrap(), v.iter().map(|x| x.0 .1).max().unwrap(), if t.is_empty() { None } else { Some(leak(t.join("; "))) })
        });
    let v = drain((&g).filt(|(n, s, _, _, _): (i64, i64, i64, i64, Option<Str>)| s as f64 / n as f64 > 10.0));
    let v = top_n(v, |&(u, (n, ..))| (Reverse(reputation.get(u).unwrap()), Reverse(n)), 10);
    rows(v.into_iter().map(|(u, (n, s, a, b, t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), avg(s, n), V::T(a), V::T(b), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn_latest,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotesCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotesCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// PostHistoryDetails AS (SELECT ph.PostId, EXTRACT(HOUR FROM ph.CreationDate) AS EditHour, COUNT(ph.Id) AS EditCount, STRING_AGG(ph.Comment, '; ') AS EditComments
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId, EXTRACT(HOUR FROM ph.CreationDate)),
// ClosedPosts AS (SELECT p.Id AS PostId, ph.CreationDate AS ClosedDate, cr.Name AS CloseReasonName FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId
//     JOIN CloseReasonTypes cr ON cr.Id = CAST(ph.Comment AS INTEGER) WHERE ph.PostHistoryTypeId = 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, COALESCE(dp.ClosedDate, NULL) AS ClosedDate, COALESCE(dp.CloseReasonName, 'Not Closed') AS CloseReason,
//        rp.UpVotesCount, rp.DownVotesCount, CASE WHEN rp.UpVotesCount > rp.DownVotesCount THEN 'Positively Rated' WHEN rp.DownVotesCount > rp.UpVotesCount THEN 'Negatively Rated' ELSE 'Neutral' END AS RatingType,
//        phd.EditHour, phd.EditCount, phd.EditComments
// FROM RankedPosts rp LEFT JOIN ClosedPosts dp ON rp.PostId = dp.PostId LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId
// WHERE rp.rn_latest = 1 AND (rp.ViewCount > 100 OR (rp.Score > 0 AND dp.PostId IS NULL)) ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 50;
//
// rn_latest reads only CreationDate, so the posts are ranked first (a tie goes to the smaller post id). The STRING_AGG order is left open; the port joins in history id order.
fn q24731(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let ud = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let crt: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let dp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt)));
    let phd = db
        .post_history
        .with(post_history_type_id.is_in([4, 5, 6]))
        .group_by((&db.post_history.post).and(hd.map(hour)))
        .select(comment.opt())
        .buf_fold(|v| {
            let c: Vec<&str> = v.iter().filter_map(|x| *x).collect();
            (v.len() as i64, if c.is_empty() { None } else { Some(leak(c.join("; "))) })
        });
    let phv = rel(drain(&phd));
    type H = ((Id<Post>, i64), (i64, Option<Str>));
    let phi: HashIdx<Id<Post>, H> = (&phv).map(|x: H| x.0 .0).inv().select(&phv).collect();
    type J = ((((Id<Post>, Option<i64>), i64), [i64; 2]), Option<(i64, Str)>);
    let v = drain(
        (&rp)
            .select(Ident::<Post>::new().and(view_count.opt()).and(score).and(&ud).and(dp.opt()))
            .filt(|((((_, w), s), _), d): J| w.map_or(false, |w| w > 100) || (s > 0 && d.is_none()))
            .select(Same::<J>::new().and(Same::<J>::new().map(|x: J| x.0 .0 .0 .0).select((&phi).opt()))),
    );
    let v = top_n(v, |&(_, (((((_, w), s), _), _), _))| (Reverse(s), w.is_none(), Reverse(w)), 50);
    rows(v.into_iter().map(|(_, (((((p, _), _), a), d), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(match d {
            Some((t, n)) => [V::T(t), V::S(n)],
            None => [V::Null, V::S("Not Closed")],
        });
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "Positively Rated" } else if a[1] > a[0] { "Negatively Rated" } else { "Neutral" })]);
        f.extend(match h {
            Some(((_, hr), (n, c))) => [V::I(hr), V::I(n), ostr(c)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, SUM(v.BountyAmount) AS TotalBounties, COUNT(DISTINCT b.Id) AS BadgeCount, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// PostHistoryAggregates AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEditDate, COUNT(DISTINCT ph.UserId) AS EditorCount, ARRAY_AGG(DISTINCT ph.Comment) AS Comments,
//        SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS ClosureEvents FROM PostHistory ph GROUP BY ph.PostId),
// TopUsers AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, ur.TotalBounties, ur.BadgeCount, DENSE_RANK() OVER (ORDER BY ur.Reputation DESC) AS Rank FROM UserReputation ur
//     WHERE ur.Reputation IS NOT NULL AND ur.BadgeCount > 0),
// FilteredPosts AS (SELECT p.Id, p.Title, p.AcceptedAnswerId, p.CreationDate, COALESCE(phc.ClosureEvents, 0) AS ClosureCount, p.Tags,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount
//     FROM Posts p LEFT JOIN PostHistoryAggregates phc ON p.Id = phc.PostId WHERE p.ViewCount > 1000 AND (p.Score > 0 OR p.AcceptedAnswerId IS NOT NULL))
// SELECT f.Title, f.CommentCount, f.ClosureCount, u.DisplayName AS TopUser, u.Reputation AS TopUserReputation
// FROM FilteredPosts f LEFT JOIN (SELECT p.Id, u.DisplayName, u.Reputation FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id JOIN TopUsers tu ON u.Id = tu.UserId ORDER BY p.ViewCount DESC) u ON f.Id = u.Id
// WHERE f.ClosureCount = 0 ORDER BY f.CommentCount DESC, f.ClosureCount ASC LIMIT 10;
//
// Only BadgeCount > 0 is read from UserReputation, which is having a badge, so the votes x badges x posts product is not driven.
fn q22228(db: &'static So) -> String {
    let Post { view_count, score, accepted_answer_id, owner_user, .. } = &db.post;
    let closures = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let cc = comments_per_post(db);
    let top = Ident::<User>::new().with(badges_of(db));
    let v = drain(
        db.post
            .with(view_count.gt(1000))
            .with(score.gt(0).or(accepted_answer_id))
            .minus(closures)
            .select(Ident::<Post>::new().and(&cc).and(owner_user.select(top).opt())),
    );
    let v = top_n(v, |&(_, ((_, c), _))| Reverse(c), 10);
    rows(v.into_iter().map(|(_, ((p, c), u))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(c), V::I(0)]);
        f.extend(match u {
            Some(u) => ucols(db, u, &["name", "rep"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE U.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName, U.Reputation),
// VoteDetails AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS TotalUpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS TotalDownVotes
//     FROM Users U JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostSummary AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, COALESCE(H.Summary, 'No History') AS EditHistory, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentEdit
//     FROM Posts P LEFT JOIN (SELECT PostId, STRING_AGG(Comment, '; ') AS Summary FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId) H ON P.Id = H.PostId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month')
// SELECT US.UserId, US.DisplayName, US.Reputation, US.BadgeCount, US.UpVotes, US.DownVotes, US.QuestionCount, US.AnswerCount, US.TotalViews, PS.PostId, PS.Title, PS.Score, PS.ViewCount, PS.EditHistory
// FROM UserStats US JOIN PostSummary PS ON US.UserId = PS.PostId WHERE US.Reputation > 1000 ORDER BY US.Reputation DESC, PS.ViewCount DESC LIMIT 50;
//
// `US.UserId = PS.PostId` compares a user id with a post id; the badges x posts x votes product is driven for the matched users alone. The STRING_AGG order is left open; the port joins in history id order.
fn q7177(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, origid, post_type_id, view_count, .. } = &db.post;
    let User { creation_date: ucd, reputation, .. } = &db.user;
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let recent = || db.post.with(creation_date.ge(add_months(t0, -1)));
    let old = || Ident::<User>::new().with(ucd.lt(add_years(t0, -1))).with(reputation.gt(1000));
    let us: MatSet<Id<User>> = recent().select(origid.select(&uidx).select(old())).collect();
    let prod = (&us)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 6], |a, (_, p)| match p {
            Some(((t, w), v)) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
            None => a,
        });
    let nb = (&us).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, comment, post, .. } = &db.post_history;
    let h = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(comment.opt()).buf_fold(|v| {
        let c: Vec<&str> = v.iter().filter_map(|x| *x).collect();
        if c.is_empty() { None } else { Some(leak(c.join("; "))) }
    });
    let v = drain(recent().select(Ident::<Post>::new().and(origid.select(&uidx).select(Ident::<User>::new().and(&prod).and((&nb).opt()))).and((&h).opt())));
    let v = top_n(v, |&(p, ((_, ((u, _), _)), _))| {
        let w = view_count.get(p);
        (Reverse(reputation.get(u).unwrap()), w.is_none(), Reverse(w))
    }, 50);
    rows(v.into_iter().map(|(_, ((p, ((u, a), b)), e))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4])]);
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.push(V::S(e.flatten().unwrap_or("No History")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankByViews,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) OVER (PARTITION BY p.Id) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) OVER (PARTITION BY p.Id) AS DownVotes,
//        EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - p.CreationDate)) / 3600 AS AgeInHours, COALESCE((SELECT COUNT(c.Id) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '30 days'),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount, STRING_AGG(DISTINCT ctr.Name, ', ') AS CloseReasons FROM PostHistory ph
//     JOIN CloseReasonTypes ctr ON CAST(ph.Comment AS INTEGER) = ctr.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AgeInHours, rp.CommentCount, COALESCE(cp.CloseCount, 0) AS CloseCount,
//        COALESCE(cp.CloseReasons, 'None') AS CloseReasons, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.AgeInHours < 24 AND rp.UpVotes = 0 THEN 'New Post with No Upvotes' WHEN rp.Score < 0 THEN 'Negative Score' ELSE 'Regular Post' END AS PostCategory
//     FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.AgeInHours, ps.CommentCount, ps.CloseCount, ps.CloseReasons, ps.UpVotes, ps.DownVotes, ps.PostCategory
// FROM PostStatistics ps WHERE ps.CloseCount < 1 AND ps.AgeInHours < 72 ORDER BY ps.ViewCount DESC NULLS LAST, ps.Score DESC NULLS LAST;
//
// RankedPosts keeps one row per vote (no GROUP BY), and the window counts are the post's totals. CloseCount < 1 is having no close/reopen row that joins a reason.
fn q23002(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, score, .. } = &db.post;
    let crt: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt));
    let rp: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).with(creation_date.filt(move |d| epoch(t0 - d) / 3600.0 < 72.0)).minus(closed).collect();
    let ud = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = comments_per_post(db);
    let v = drain((&rp).select(votes_of(db).opt().and(&ud).and(&cc)));
    rows(v.into_iter().map(|(p, ((_, a), n))| {
        let h = epoch(t0 - creation_date.get(p).unwrap()) / 3600.0;
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::F(h), V::I(n), V::I(0), V::S("None"), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if h < 24.0 && a[0] == 0 { "New Post with No Upvotes" } else if score.get(p).unwrap() < 0 { "Negative Score" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS UserDisplayName, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' AND p.ViewCount IS NOT NULL),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UserDisplayName FROM RankedPosts rp WHERE rp.ScoreRank <= 10),
// VoteAggregation AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostHistoryDetails AS (SELECT ph.PostId, STRING_AGG(pt.Name, ', ') AS HistoryTypes, MAX(ph.CreationDate) AS LastUpdate FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id
//     WHERE ph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY ph.PostId)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.UserDisplayName, COALESCE(va.UpVotes, 0) AS UpVotes, COALESCE(va.DownVotes, 0) AS DownVotes,
//        COALESCE(phd.HistoryTypes, 'No history') AS HistoryTypes, phd.LastUpdate,
//        CASE WHEN fp.Score > 100 THEN 'High Performer' WHEN fp.Score BETWEEN 50 AND 100 THEN 'Moderate Performer' WHEN fp.Score < 50 THEN 'Low Performer' ELSE 'Unknown' END AS PerformanceCategory
// FROM FilteredPosts fp LEFT JOIN VoteAggregation va ON fp.PostId = va.PostId LEFT JOIN PostHistoryDetails phd ON fp.PostId = phd.PostId
// WHERE EXISTS (SELECT 1 FROM Comments c WHERE c.PostId = fp.PostId AND c.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// ORDER BY fp.Score DESC, fp.ViewCount DESC OFFSET 0 ROWS FETCH NEXT 100 ROWS ONLY;
//
// ScoreRank reads only base columns, so the posts are ranked first. The STRING_AGG order is left open; the port joins in history id order.
fn q20389(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_months(t0, -6)))
        .with(view_count)
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| Reverse(s), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let va = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { creation_date: hd, post, .. } = &db.post_history;
    let phd = db.post_history.with(hd.gt(add_years(t0, -1))).group_by(post).select(htype_name(db).and(hd)).buf_fold(|v| (leak(v.iter().map(|x| x.0).collect::<Vec<_>>().join(", ")), v.iter().map(|x| x.1).max().unwrap()));
    let old = comments_of(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).lt(add_days(t0, -30))));
    let v = drain((&fp).with(old).select(Ident::<Post>::new().and(&va).and((&phd).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p))), 100);
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(h.map_or("No history", |h| h.0)), h.map_or(V::Null, |h| V::T(h.1))]);
        f.push(V::S(if s > 100 { "High Performer" } else if s >= 50 { "Moderate Performer" } else { "Low Performer" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN V.VoteTypeId IN (1, 2, 4) THEN 1 ELSE 0 END) AS PositiveVotes, SUM(CASE WHEN V.VoteTypeId IN (3, 10) THEN 1 ELSE 0 END) AS NegativeVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, COALESCE(SUM(CASE WHEN C.Score IS NOT NULL THEN C.Score ELSE 0 END), 0) AS TotalCommentScore, COUNT(C.Id) AS TotalComments,
//        COUNT(DISTINCT P2.Id) AS RelatedPosts FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostLinks PL ON P.Id = PL.PostId LEFT JOIN Posts P2 ON PL.RelatedPostId = P2.Id
//     GROUP BY P.Id, P.Title, P.OwnerUserId),
// ClosedPosts AS (SELECT PH.PostId, PH.UserId, PH.CreationDate, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS rn FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11)),
// UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, STRING_AGG(DISTINCT B.Name, ', ') AS BadgeNames FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT U.DisplayName AS User, U.Reputation, UPS.VoteCount, UPS.UpVotes, UPS.DownVotes, UPS.PositiveVotes, UPS.NegativeVotes, PS.Title AS Post_Title, PS.TotalCommentScore, PS.TotalComments,
//        COALESCE(ToC.Score, 0) AS Closed_Post_Count, UBC.BadgeCount, UBC.BadgeNames, PS.RelatedPosts
// FROM Users U JOIN UserVoteStats UPS ON U.Id = UPS.UserId JOIN PostStats PS ON U.Id = PS.OwnerUserId
// LEFT JOIN (SELECT UserId, COUNT(PostId) AS Score FROM ClosedPosts WHERE rn = 1 GROUP BY UserId) ToC ON U.Id = ToC.UserId LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId
// WHERE U.Reputation > 100 AND UPS.PositiveVotes > UPS.NegativeVotes ORDER BY U.Reputation DESC, PS.TotalCommentScore DESC;
//
// The WHERE reads only UserVoteStats, so the users are picked first and PostStats is driven for their posts alone. A CreationDate tie inside rn goes to the smaller history id,
// and the distinct badge names are joined in name order (the SQL leaves both open).
fn q20678(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let ups = db.user.with(reputation.gt(100)).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 5], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + [1, 2, 4].contains(&t) as i64, a[4] + [3, 10].contains(&t) as i64],
        None => a,
    });
    let us: MatSet<Id<User>> = db.user.with((&ups).filt(|a| a[3] > a[4])).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let up: MatSet<Id<Post>> = (&us).select(posts_of(db)).collect();
    let psf = (&up)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(&db.comment.score).opt().and(links_of(db).opt()))
        .fold((0i64, 0i64), |(s, n), (c, _)| (s + c.unwrap_or(0), n + c.is_some() as i64));
    let psr = (&up).group_by(Ident::<Post>::new()).select(links_of(db).select((&db.post_link.related_post_id).select(&pidx))).count_distinct();
    let ps = (&psf).and((&psr).opt()).map(|((s, n), r): ((i64, i64), Option<i64>)| (s, n, r.unwrap_or(0)));
    let PostHistory { post_history_type_id, creation_date: hd, user, post, .. } = &db.post_history;
    let cw = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (Reverse(d), h), asc);
    let toc = (&cw).filt(|(_, k)| k == 1).map(|((h, _), _)| h).select(user).inv().fold(0i64, |n, _| n + 1);
    let ubc = (&us).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name).opt()).buf_fold(|v| (v.iter().filter(|x| x.is_some()).count() as i64, agg_distinct(v.iter().filter_map(|x| *x).collect(), ", ")));
    let v = drain((&us).select(Ident::<User>::new().and(&ups).and((&toc).opt()).and(&ubc).and(posts_of(db).select(Ident::<Post>::new().and(&ps)))));
    rows(v.into_iter().map(|(_, ((((u, a), c), (bn, bs)), (p, (s, n, r))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(s), V::I(n), V::I(c.unwrap_or(0)), V::I(bn), ostr(bs), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS ViewRank,
//        RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank, COALESCE(cmt.CommentCount, 0) AS CommentCount, COALESCE(b.UserId, -1) AS BadgeUserId,
//        COALESCE(CASE WHEN p.PostTypeId = 1 THEN b.Name END, 'No Badge') AS UserBadge
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) cmt ON p.Id = cmt.PostId LEFT JOIN Badges b ON b.UserId = p.OwnerUserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AggregatedData AS (SELECT r.PostId, r.Title, r.CreationDate, r.ViewCount, r.Score, r.AnswerCount, r.CommentCount, r.ViewRank, r.ScoreRank,
//        (CASE WHEN r.ViewRank <= 5 THEN 'Top Views' WHEN r.ScoreRank <= 5 THEN 'Top Scores' ELSE 'Others' END) AS PostCategory FROM RankedPosts r),
// CommentActivity AS (SELECT p.Id AS PostId, COUNT(c.Id) AS TotalComments, STRING_AGG(c.Text, '; ') AS SampleComments FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT a.PostId, a.Title, a.CreationDate, a.ViewCount, a.Score, a.AnswerCount, a.CommentCount, a.PostCategory, COALESCE(ca.TotalComments, 0) AS CountOfComments,
//        COALESCE(ca.SampleComments, 'No comments yet') AS ExampleComments,
//        CASE WHEN a.PostCategory = 'Top Views' AND a.CommentCount > 0 THEN 'Popular & Engaged' WHEN a.PostCategory = 'Top Scores' AND a.CommentCount = 0 THEN 'Highly Rated but No Engagement'
//        ELSE 'Standard Engagement' END AS EngagementStatus
// FROM AggregatedData a LEFT JOIN CommentActivity ca ON a.PostId = ca.PostId WHERE a.ViewCount > 10 AND a.CommentCount IS NOT NULL ORDER BY a.CommentCount DESC, a.ViewCount DESC LIMIT 50;
//
// RankedPosts is one row per badge of the owner, and both windows number those rows; a ViewCount tie between rows goes to the smaller post id, then badge id.
// The STRING_AGG order is left open; the port joins in comment id order.
fn q23890(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, score, owner_user, .. } = &db.post;
    type R = ((Id<Post>, Option<i64>), Option<Id<Badge>>);
    let vw = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(view_count.opt()).and(owner_user.select(badges_of(db)).opt()))
        .window(row_number, |((p, w), b)| (w.is_none(), Reverse(w), p, b), asc);
    let vr: MatSet<(R, i64)> = (&vw).map(|x| x).collect();
    type X = ((R, i64), i64);
    let k = || Same::<X>::new().map(|x: X| x.0 .0 .0 .0);
    let sw = whole(&vr).select(Same::<(R, i64)>::new().and(Same::<(R, i64)>::new().map(|x: (R, i64)| x.0 .0 .0).select(score))).window(rank, |(_, s)| Reverse(s), asc);
    let cc = comments_per_post(db);
    let big = || Ident::<Post>::new().with(view_count.gt(10)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ca = db.post.select(big()).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.text)).buf_fold(|v| (v.len() as i64, leak(v.join("; "))));
    let v = drain((&sw).map(|((x, _), r): (((R, i64), i64), i64)| (x, r)).with(k().select(big())).select(Same::<X>::new().and(k().select(&cc)).and(k().select((&ca).opt()))));
    let v = top_n(v, |&(_, ((((((p, _), _), _), _), c), _))| (Reverse(c), Reverse(view_count.get(p))), 50);
    rows(v.into_iter().map(|(_, ((((((p, _), _), vr), sr), c), a))| {
        let cat = if vr <= 5 { "Top Views" } else if sr <= 5 { "Top Scores" } else { "Others" };
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.extend([V::I(c), V::S(cat), V::I(a.map_or(0, |a| a.0)), V::S(a.map_or("No comments yet", |a| a.1))]);
        f.push(V::S(if cat == "Top Views" && c > 0 { "Popular & Engaged" } else if cat == "Top Scores" && c == 0 { "Highly Rated but No Engagement" } else { "Standard Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS RankByType,
//        COALESCE(p.AcceptedAnswerId, 0) AS HasAcceptedAnswer, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, pt.Name, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AcceptedAnswerId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, HasAcceptedAnswer, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE RankByType <= 5),
// UserBadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// EnhancedPostInfo AS (SELECT tp.*, u.DisplayName AS PostOwner, ub.BadgeCount, ub.BadgeNames FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId)
// SELECT epi.PostId, epi.Title, epi.CreationDate, epi.Score, epi.ViewCount, CASE WHEN epi.HasAcceptedAnswer = 0 THEN 'No Accepted Answer' ELSE 'Has Accepted Answer' END AS AnswerStatus,
//        epi.CommentCount, epi.UpVotes, epi.DownVotes, epi.PostOwner, COALESCE(epi.BadgeCount, 0) AS BadgeCount, COALESCE(epi.BadgeNames, 'None') AS BadgeNames,
//        CONCAT('Post Score: ', epi.Score, ' with ', epi.ViewCount, ' views.') AS PostScoreDetail
// FROM EnhancedPostInfo epi WHERE epi.Score > (SELECT AVG(Score) FROM Posts) ORDER BY epi.Score DESC, epi.ViewCount DESC LIMIT 50;
//
// RankByType reads only base columns, so the posts are ranked first (a tie goes to the smaller post id). `tp.PostId = u.Id` compares a post id with a user id.
// The badge names are joined in badge id order (the SQL leaves it open).
fn q22126(db: &'static So) -> String {
    let Post { creation_date, score, view_count, accepted_answer_id, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let (ss, sn) = db.post.select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let sa = ss as f64 / sn as f64;
    let ag = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = comments_per_post(db);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&tp).with(score.filt(move |s| s as f64 > sa)).select(Ident::<Post>::new().and(&cc).and(&ag).and(origid.select(&uidx).select(Ident::<User>::new().and((&ub).opt())))));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p))), 50);
    rows(v.into_iter().map(|(_, (((p, c), a), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::S(if accepted_answer_id.get(p).unwrap_or(0) == 0 { "No Accepted Answer" } else { "Has Accepted Answer" }));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), user_col(db, u, "name"), V::I(b.map_or(0, |b| b.0)), V::S(b.map_or("None", |b| b.1))]);
        f.push(V::Owned(format!("Post Score: {} with {} views.", score.get(p).unwrap(), view_count.get(p).map_or(String::new(), |w| w.to_string()))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn,
//        COUNT(*) OVER (PARTITION BY p.PostTypeId) AS TotalPosts FROM Posts p WHERE p.Score IS NOT NULL AND p.ViewCount IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, AVG(u.Reputation) AS AvgReputation, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// ClosedPostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, ph.UserDisplayName, DENSE_RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS CloseRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// PostWithLatestComment AS (SELECT p.Id AS PostId, p.Title, p.LastActivityDate, (SELECT COALESCE(MAX(c.CreationDate), '1970-01-01') FROM Comments c WHERE c.PostId = p.Id) AS LatestCommentDate
//     FROM Posts p WHERE p.ViewCount > 1000),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.TotalPosts, COALESCE(cl.CloseRank, 0) AS IsClosed, ul.AvgReputation, ul.BadgeCount
//     FROM RankedPosts rp LEFT JOIN ClosedPostHistory cl ON rp.PostId = cl.PostId JOIN UserReputation ul ON rp.PostId IN (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) WHERE rp.rn <= 10)
// SELECT t.PostId, t.Title, t.Score, t.ViewCount, t.CreationDate, t.IsClosed, ROUND(t.AvgReputation, 2) AS AvgUserReputation, t.BadgeCount,
//        'Post Type: ' || (SELECT pt.Name FROM PostTypes pt WHERE pt.Id = (SELECT PostTypeId FROM Posts WHERE Id = t.PostId)) AS PostType,
//        STRING_AGG(DISTINCT CASE WHEN t.IsClosed > 0 THEN 'Closed' ELSE 'Open' END, ', ') AS Status
// FROM TopPosts t GROUP BY t.PostId, t.Title, t.Score, t.ViewCount, t.CreationDate, t.IsClosed, t.AvgReputation, t.BadgeCount ORDER BY t.Score DESC, t.ViewCount DESC;
//
// rn reads only base columns, so the posts are ranked first (a tie goes to the smaller post id). The ON of the UserReputation join names only rp: it keeps the posts whose Id is their own
// OwnerUserId and pairs each with every user, a cross join.
fn q20755(db: &'static So) -> String {
    let Post { view_count, post_type_id, score, creation_date, origid, owner_user_id, .. } = &db.post;
    let w = db
        .post
        .with(view_count)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let own = origid.and(owner_user_id).filt(|(a, b): (i64, i64)| a == b);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).with(own).collect();
    let PostHistory { post_history_type_id, creation_date: hd, post, .. } = &db.post_history;
    let cw = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(dense_rank, |(_, d)| Reverse(d), asc);
    let ul = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ur: HashIdx<Id<User>, (i64, i64)> = db.user.select((&db.user.reputation).and(&ul)).collect();
    let tr = (&tp).select(Ident::<Post>::new().and((&cw).map(|(_, k)| k).opt().map(|k: Option<i64>| k.unwrap_or(0))));
    let g: MatSet<((Id<Post>, i64), (i64, i64))> = tr.cross(&ur).collect();
    rows(drain(&g).into_iter().map(|(_, ((p, c), (r, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend([V::I(c), V::F(r as f64), V::I(n)]);
        f.push(V::Owned(format!("Post Type: {}", db.post_type.name.get(db.post.post_type.get(p).unwrap()).unwrap())));
        f.push(V::S(if c > 0 { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPostsByUser, LEAD(p.ViewCount) OVER (ORDER BY p.CreationDate) AS NextPostViewCount
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, COALESCE(rp.NextPostViewCount - rp.ViewCount, 0) AS ViewCountDifference, rp.Score, rp.UserPostRank, rp.TotalPostsByUser
//     FROM RankedPosts rp WHERE rp.UserPostRank = 1 AND rp.TotalPostsByUser > 5),
// PostsWithComments AS (SELECT fp.PostId, fp.Title, fp.CreationDate, fp.ViewCountDifference, fp.Score, COUNT(c.Id) AS CommentCount,
//        CASE WHEN COUNT(c.Id) = 0 THEN 'No Comments' ELSE STRING_AGG(c.Text, '; ') END AS CommentText
//     FROM FilteredPosts fp LEFT JOIN Comments c ON fp.PostId = c.PostId GROUP BY fp.PostId, fp.Title, fp.CreationDate, fp.ViewCountDifference, fp.Score)
// SELECT p.Title, p.CreationDate, p.ViewCountDifference, p.Score, p.CommentCount, CASE WHEN p.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus,
//        CASE WHEN p.ViewCountDifference < 0 THEN 'Fewer Views than Next Post' WHEN p.ViewCountDifference = 0 THEN 'Equal Views to Next Post' ELSE 'More Views than Next Post' END AS ViewComparison,
//        CASE WHEN p.Score IS NULL THEN 'Unscored' WHEN p.Score > 0 THEN 'Positive Score' WHEN p.Score < 0 THEN 'Negative Score' ELSE 'Neutral Score' END AS ScoreDescription
// FROM PostsWithComments p WHERE p.Score IS NOT NULL OR EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = p.PostId AND v.VoteTypeId IN (2, 3)) ORDER BY p.CreationDate DESC LIMIT 10;
//
// Score is never NULL, so the WHERE keeps every row. A CreationDate tie inside UserPostRank or LEAD goes to the smaller post id (the SQL leaves it open).
fn q23114(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, view_count, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)));
    let tot = recent().group_by(owner_user_id.opt()).fold(0i64, |n, _| n + 1);
    let rw = recent().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&rw).and(&tot).filt(|((_, k), n)| k == 1 && n > 5).map(|(((p, _), _), _)| p).collect();
    type L = (i64, Id<Post>, Option<i64>);
    let lw = whole(recent()).select(Ident::<Post>::new().and(creation_date).and(view_count.opt())).window(lead, |((p, d), w)| -> L { (d, p, w) }, asc);
    let li = by_first(&(&lw).map(|(((p, _), _), l)| (p, l)).collect());
    let cc = comments_per_post(db);
    let v = drain((&fp).select(Ident::<Post>::new().and(&li).and(&cc)));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(_, ((p, l), c))| {
        let d = match (l.and_then(|l| l.2), view_count.get(p)) {
            (Some(a), Some(b)) => a - b,
            _ => 0,
        };
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(d), V::I(s), V::I(c), V::S(if c > 0 { "Has Comments" } else { "No Comments" })]);
        f.push(V::S(if d < 0 { "Fewer Views than Next Post" } else if d == 0 { "Equal Views to Next Post" } else { "More Views than Next Post" }));
        f.push(V::S(if s > 0 { "Positive Score" } else if s < 0 { "Negative Score" } else { "Neutral Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RecentRank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountySpent, COUNT(DISTINCT CASE WHEN b.Class = 1 THEN b.Id END) AS GoldBadges,
//        COUNT(DISTINCT CASE WHEN b.Class = 2 THEN b.Id END) AS SilverBadges, COUNT(DISTINCT CASE WHEN b.Class = 3 THEN b.Id END) AS BronzeBadges
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, '; ' ORDER BY c.Id) AS Comments FROM Comments c GROUP BY c.PostId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, u.DisplayName AS OwnerDisplayName, ps.CommentCount, ps.Comments, rp.Score, us.TotalBountySpent, us.GoldBadges, us.SilverBadges, us.BronzeBadges
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN PostComments ps ON rp.PostId = ps.PostId LEFT JOIN UserStatistics us ON u.Id = us.UserId
//     WHERE rp.RecentRank = 1 AND (rp.Score > 5 OR us.Reputation > 100))
// SELECT fp.Title, fp.OwnerDisplayName, fp.CommentCount, COALESCE(fp.Comments, 'No comments') AS CommentSummary, fp.Score,
//        CONCAT(fp.GoldBadges, ' Gold, ', fp.SilverBadges, ' Silver, ', fp.BronzeBadges, ' Bronze') AS BadgeSummary,
//        CASE WHEN fp.TotalBountySpent > 100 THEN 'High Spender' WHEN fp.TotalBountySpent BETWEEN 50 AND 100 THEN 'Moderate Spender' ELSE 'Low Spender' END AS SpendingCategory
// FROM FilteredPosts fp ORDER BY fp.Score DESC, fp.CommentCount DESC LIMIT 50 OFFSET 0;
//
// RecentRank reads only base columns, so the posts are ranked first (a tie goes to the smaller post id), and UserStatistics is driven for their owners alone.
fn q23297(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).with(owner_user).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let ou = || (&owners).group_by(Ident::<User>::new());
    let bs = ou().select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt())).fold(0i64, |s, (b, _)| s + b.flatten().unwrap_or(0));
    let bc = ou().select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let us = (&bs).and((&bc).opt()).map(|(b, c): (i64, Option<[i64; 3]>)| (b, c.unwrap_or([0; 3])));
    let pc = db.comment.group_by(&db.comment.post).select(&db.comment.text).buf_fold(|v| (v.len() as i64, leak(v.join("; "))));
    type J = ((Id<Post>, i64), ((Id<User>, i64), (i64, [i64; 3])));
    let v = drain(
        (&rp)
            .select(Ident::<Post>::new().and(score).and(owner_user.select(Ident::<User>::new().and(&db.user.reputation).and(&us))))
            .filt(|((_, s), ((_, r), _)): J| s > 5 || r > 100)
            .select(Same::<J>::new().and(Same::<J>::new().map(|x: J| x.0 .0).select((&pc).opt()))),
    );
    let v = top_n(v, |&(_, (((p, _), _), c))| (Reverse(score.get(p).unwrap()), c.is_none(), Reverse(c.map(|c| c.0))), 50);
    rows(v.into_iter().map(|(_, (((p, _), ((u, _), (b, g))), c))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([user_col(db, u, "name"), c.map_or(V::Null, |c| V::I(c.0)), V::S(c.map_or("No comments", |c| c.1))]);
        f.extend(post_fields(db, p, &["score"]));
        f.push(V::Owned(format!("{} Gold, {} Silver, {} Bronze", g[0], g[1], g[2])));
        f.push(V::S(if b > 100 { "High Spender" } else if b >= 50 { "Moderate Spender" } else { "Low Spender" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPosts FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 years'),
// UserReputation AS (SELECT u.Id, u.Reputation, u.DisplayName, u.Location, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.DisplayName, u.Location),
// PopularPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, ur.DisplayName, ur.Reputation, ur.Location, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.Id WHERE rp.Rank = 1 ORDER BY rp.Score DESC, ur.Reputation DESC),
// PostHistoryDetails AS (SELECT ph.PostId, STRING_AGG(DISTINCT CONCAT(pt.Name, ': ', ph.Comment), '; ') AS HistoryComments FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id
//     WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT pp.PostId, pp.Title, pp.CreationDate, pp.Score, pp.DisplayName, pp.Reputation, pp.Location, pp.GoldBadges, pp.SilverBadges, pp.BronzeBadges,
//        COALESCE(pHD.HistoryComments, 'No recent history') AS RecentHistory
// FROM PopularPosts pp LEFT JOIN PostHistoryDetails pHD ON pp.PostId = pHD.PostId WHERE pp.Score > 5 AND pp.Reputation >= 100 ORDER BY pp.Score DESC, pp.Reputation DESC LIMIT 10;
//
// A CreationDate tie inside Rank goes to the smaller post id, and the distinct history strings are joined in sorted order (the SQL leaves both open).
fn q22880(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(creation_date.ge(add_years(t0, -2)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let pp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).select(Ident::<Post>::new().with(score.gt(5)).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).ge(100))))).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let PostHistory { creation_date: hd, post_history_type_id, comment, post, .. } = &db.post_history;
    let phd = db
        .post_history
        .with(hd.ge(add_years(t0, -1)))
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(htype_name(db).and(comment.opt()))
        .buf_fold(|v| agg_distinct(v.iter().map(|&(n, c)| leak(format!("{n}: {}", c.unwrap_or("")))).collect(), "; ").unwrap());
    let v = drain((&pp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ur))).and((&phd).opt())));
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let v = top_n(v, |&(p, ((_, (u, _)), _))| (Reverse(score.get(p).unwrap()), Reverse(rep(u))), 10);
    rows(v.into_iter().map(|(_, ((p, (u, b)), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(ostr(db.user.location.get(u)));
        f.extend(b.map(V::I));
        f.push(V::S(h.unwrap_or("No recent history")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, COALESCE(u.DisplayName, 'Community User') AS OwnerName,
//        STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '>.<')) AS TagName) AS t ON TRUE
//     WHERE p.PostTypeId = 1 AND p.Score > 0 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, u.DisplayName),
// PostEngagement AS (SELECT rp.PostId, rp.OwnerName, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.Score, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        MAX(b.Class) AS HighestBadgeClass
//     FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId AND v.VoteTypeId IN (2, 3)
//     LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) GROUP BY rp.PostId, rp.OwnerName, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.Score),
// FinalResults AS (SELECT pe.*, CASE WHEN HighestBadgeClass = 1 THEN 'Gold' WHEN HighestBadgeClass = 2 THEN 'Silver' WHEN HighestBadgeClass = 3 THEN 'Bronze' ELSE 'None' END AS BadgeStatus,
//        CASE WHEN AnswerCount > 5 THEN 'Highly Engaging' WHEN ViewCount > 100 THEN 'Popular' ELSE 'Average Engagement' END AS EngagementLevel FROM PostEngagement pe)
// SELECT PostId, OwnerName, Title, CreationDate, ViewCount, AnswerCount, Score, CommentCount, VoteCount, BadgeStatus, EngagementLevel
// FROM FinalResults ORDER BY Score DESC, ViewCount DESC, CreationDate DESC FETCH FIRST 50 ROWS ONLY;
//
// RankedPosts is one row per question (the tag list is not read), and the ORDER BY reads only base columns, so the 50 questions are picked first.
fn q28796(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, view_count, owner_user, answer_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(score.gt(0)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score));
    let v = top_n(v, |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()))
    }, 50);
    let rp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = comments_per_post(db);
    let vc = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let hb = (&rp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).select(&db.badge.class)).fold(i64::MIN, |m, c| m.max(c));
    let v = drain((&rp).select(Ident::<Post>::new().and(&cc).and(&vc).and((&hb).opt())));
    rows(v.into_iter().map(|(_, (((p, c), n), b))| {
        let mut f = post_fields(db, p, &["id"]);
        f.push(V::S(owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend(post_fields(db, p, &["title", "created", "views", "answers", "score"]));
        f.extend([V::I(c), V::I(n)]);
        f.push(V::S(match b {
            Some(1) => "Gold",
            Some(2) => "Silver",
            Some(3) => "Bronze",
            _ => "None",
        }));
        f.push(V::S(if answer_count.get(p).map_or(false, |a| a > 5) { "Highly Engaging" } else if view_count.get(p).map_or(false, |w| w > 100) { "Popular" } else { "Average Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank,
//        COALESCE(v.VoteSum, 0) AS TotalVotes
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS VoteSum FROM Votes GROUP BY PostId) v ON p.Id = v.PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.Rank, rp.TotalVotes, PH.UserDisplayName, PH.CreationDate AS HistoryDate, PH.Comment AS HistoryComment, PH.PostHistoryTypeId,
//        DENSE_RANK() OVER (PARTITION BY rp.PostId ORDER BY PH.CreationDate DESC) AS HistoryRank
//     FROM RankedPosts rp LEFT JOIN PostHistory PH ON rp.PostId = PH.PostId WHERE PH.PostHistoryTypeId IN (10, 11, 12)),
// RecentActivity AS (SELECT PostId, STRING_AGG(DISTINCT UserDisplayName, ', ') AS UsersResponsible, COUNT(*) AS ChangeCount FROM PostDetails WHERE HistoryRank = 1 GROUP BY PostId),
// UserReputation AS (SELECT u.Id AS UserId, SUM(b.Class) AS TotalBadgePoints FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT pd.PostId, pd.Title, pd.Score, pd.ViewCount, pd.Rank, pd.TotalVotes, ra.UsersResponsible, ra.ChangeCount, COALESCE(ur.TotalBadgePoints, 0) AS BadgePoints,
//        CASE WHEN pd.TotalVotes IS NULL THEN 'No Votes' ELSE CONCAT(pd.TotalVotes, ' Total Votes') END AS VoteMessage,
//        CASE WHEN pd.Score > 0 THEN 'Prominent Post' WHEN pd.Score < 0 THEN 'Controversial Post' ELSE 'Neutral Post' END AS Sentiment
// FROM PostDetails pd LEFT JOIN RecentActivity ra ON pd.PostId = ra.PostId LEFT JOIN Users u ON pd.PostId = u.Id LEFT JOIN UserReputation ur ON pd.PostId = ur.UserId
// WHERE pd.Rank <= 5 AND pd.HistoryRank = 1 ORDER BY pd.Score DESC, pd.ViewCount DESC;
//
// Rank reads only base columns, so the posts are ranked first (a tie goes to the smaller post id). `pd.PostId = ur.UserId` compares a post id with a user id.
// The distinct names are joined in sorted order.
fn q23583(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, origid, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(score).and(view_count.opt())).window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let top: MatSet<(Id<Post>, i64)> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), k)| (p, k)).collect();
    let rk = by_first(&top);
    let rp: MatSet<Id<Post>> = (&top).map(|x: (Id<Post>, i64)| x.0).collect();
    let PostHistory { post_history_type_id, creation_date: hd, user_display_name, .. } = &db.post_history;
    let hw = (&rp)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11, 12])).and(hd)))
        .window(dense_rank, |(_, d)| Reverse(d), asc);
    let pd = || (&hw).filt(|(_, k)| k == 1).map(|((h, _), _)| h);
    let ra = pd().select(user_display_name.opt()).buf_fold(|v| (agg_distinct(v.iter().filter_map(|x| *x).collect(), ", "), v.len() as i64));
    let vs = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold(0i64, |s, t| s + if t == 2 { 1 } else if t == 3 { -1 } else { 0 });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold(0i64, |s, c| s + c);
    let v = drain(pd().and(&rk).and((&vs).opt()).and(&ra).and(origid.select(&uidx).select(&ur).opt()));
    rows(v.into_iter().map(|(p, ((((_, r), s), (n, c)), b))| {
        let s = s.unwrap_or(0);
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(r), V::I(s), ostr(n), V::I(c), V::I(b.unwrap_or(0)), V::Owned(format!("{s} Total Votes"))]);
        f.push(V::S(if sc > 0 { "Prominent Post" } else if sc < 0 { "Controversial Post" } else { "Neutral Post" }));
        row(f)
    }))
}

// rewrites/20507.sql (UserPostRank made total with `, p.Id`):
// WITH UserBadges AS (SELECT b.UserId, COUNT(*) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// FilteredPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS UserPostRank
//     FROM Posts p WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') AND p.Score > 0),
// PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.UserId) AS TotalVotes
//     FROM Votes v GROUP BY v.PostId),
// ClosePostReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT u.DisplayName, u.Reputation, ub.TotalBadges, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, fp.PostId, fp.CreationDate AS PostCreationDate, fp.Score AS PostScore, pvs.UpVotes, pvs.DownVotes,
//        pvs.TotalVotes, COALESCE(cpr.CloseReasons, 'No close reasons') AS CloseReasonDescriptions,
//        CASE WHEN ub.TotalBadges IS NULL THEN 'No badges yet!' WHEN ub.TotalBadges > 10 THEN 'Super user' ELSE 'Regular user' END AS UserType,
//        LEAD(fp.Score) OVER (PARTITION BY fp.OwnerUserId ORDER BY fp.CreationDate) AS NextPostScore
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN FilteredPosts fp ON u.Id = fp.OwnerUserId LEFT JOIN PostVoteSummary pvs ON fp.PostId = pvs.PostId
// LEFT JOIN ClosePostReasons cpr ON fp.PostId = cpr.PostId WHERE u.Reputation >= 1000 AND (fp.UserPostRank = 1 OR fp.UserPostRank IS NULL)
// ORDER BY ub.TotalBadges DESC, fp.CreationDate DESC LIMIT 100 OFFSET 0;
//
// The WHERE leaves one row per owner (UserPostRank = 1), and the users without a post share the NULL partition with a NULL Score, so NextPostScore is NULL on every row.
// The STRING_AGG joins in history id order (the SQL leaves it open).
fn q20507(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(score.gt(0))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fps: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let first: HashIdx<i64, Id<Post>> = (&fps).select(owner_user_id).inv().collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let pvs = db.vote.group_by(&db.vote.post).select((&db.vote.vote_type_id).and((&db.vote.user_id).opt())).fold([0i64; 3], |a, (t, u)| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + u.is_some() as i64]);
    let crt: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, post, .. } = &db.post_history;
    let cpr = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt)).buf_fold(|v| leak(v.join(", ")));
    let v = drain(db.user.with((&db.user.reputation).ge(1000)).select(Ident::<User>::new().and((&ub).opt()).and((&db.user.origid).select(&first).select(Ident::<Post>::new().and((&pvs).opt()).and((&cpr).opt())).opt())));
    let v = top_n(v, |&(_, ((_, b), p))| (b.is_none(), Reverse(b.map(|b| b[0])), p.is_none(), Reverse(p.map(|((p, _), _)| creation_date.get(p).unwrap()))), 100);
    rows(v.into_iter().map(|(_, ((u, b), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        match p {
            Some(((p, a), c)) => {
                f.extend(post_fields(db, p, &["id", "created", "score"]));
                f.extend(match a {
                    Some(a) => a.map(V::I),
                    None => [V::Null, V::Null, V::Null],
                });
                f.push(V::S(c.unwrap_or("No close reasons")));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::S("No close reasons")]),
        }
        f.push(V::S(match b {
            None => "No badges yet!",
            Some(b) if b[0] > 10 => "Super user",
            _ => "Regular user",
        }));
        f.push(V::Null);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, P.Score, U.Reputation AS OwnerReputation,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS ScoreRank FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId IN (1, 2)),
// ClosedPosts AS (SELECT PH.PostId, COUNT(PH.Id) AS CloseCount, STRING_AGG(CASE WHEN C.Name IS NOT NULL THEN C.Name ELSE 'Undefined' END, ', ') AS CloseReasons
//     FROM PostHistory PH LEFT JOIN CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId),
// UserWithBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostEngagements AS (SELECT P.Id AS PostId, COALESCE(V.UpVotes, 0) AS UpVotes, COALESCE(V.DownVotes, 0) AS DownVotes, COALESCE(C.CommentCount, 0) AS CommentCount FROM Posts P
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) V ON P.Id = V.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId)
// SELECT RP.PostId, RP.Title, RP.ViewCount, RP.CreationDate, RP.Score, RP.OwnerReputation, CP.CloseCount, CP.CloseReasons, U.BadgeCount, U.BadgeNames, PE.UpVotes, PE.DownVotes, PE.CommentCount
// FROM RankedPosts RP LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId LEFT JOIN UserWithBadges U ON RP.OwnerReputation = U.UserId LEFT JOIN PostEngagements PE ON RP.PostId = PE.PostId
// WHERE RP.ScoreRank = 1 AND RP.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' ORDER BY RP.CreationDate DESC;
//
// ScoreRank reads only base columns, so the posts are ranked first. `RP.OwnerReputation = U.UserId` compares a reputation with a user id. The STRING_AGGs join in id order (the SQL leaves it open).
fn q22266(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|(((p, _), _), _)| p).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).collect();
    let crt: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, post, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt).opt())
        .buf_fold(|v| {
            let n: Vec<Str> = v.iter().map(|c| c.unwrap_or("Undefined")).collect();
            (v.len() as i64, leak(n.join(", ")))
        });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name).opt()).buf_fold(|v| {
        let n: Vec<&str> = v.iter().filter_map(|x| *x).collect();
        (n.len() as i64, if n.is_empty() { None } else { Some(leak(n.join(", "))) })
    });
    let ud = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = comments_per_post(db);
    let v = drain((&rp).select(Ident::<Post>::new().and((&cp).opt()).and(owner_user.select(&db.user.reputation).select(&uidx).select(&ub).opt()).and(&ud).and(&cc)));
    rows(v.into_iter().map(|(_, ((((p, c), u), a), n))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score", "rep"]);
        f.extend(match c {
            Some((k, s)) => [V::I(k), V::S(s)],
            None => [V::Null, V::Null],
        });
        f.extend(match u {
            Some((k, s)) => [V::I(k), ostr(s)],
            None => [V::Null, V::Null],
        });
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        row(f)
    }))
}

// WITH UserVotes AS (SELECT U.Id AS UserId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        COUNT(DISTINCT V.PostId) AS TotalVotedPosts FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, PT.Name AS PostType, COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Badges B WHERE B.UserId = P.OwnerUserId AND B.Class = 1), 0) AS GoldBadgeCount FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id),
// EnhancedPosts AS (SELECT PS.PostId, PS.Title, PS.ViewCount, PS.CommentCount, PS.Score, PS.PostType,
//        CASE WHEN PS.Score < 0 THEN 'Poor' WHEN PS.Score BETWEEN 0 AND 10 THEN 'Average' WHEN PS.Score > 10 THEN 'Good' ELSE NULL END AS ScoreCategory,
//        ROW_NUMBER() OVER (PARTITION BY PS.PostType ORDER BY PS.Score DESC) AS Rank, FIRST_VALUE(PS.Title) OVER (PARTITION BY PS.PostType ORDER BY PS.Score DESC) AS TopPostTitle FROM PostStats PS),
// UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, UV.UpvoteCount, UV.DownvoteCount, PS.PostId, PS.Title AS PostTitle, PS.ScoreCategory, PS.Rank
//     FROM Users U LEFT JOIN UserVotes UV ON U.Id = UV.UserId LEFT JOIN EnhancedPosts PS ON U.Id = PS.PostId)
// SELECT UE.UserId, UE.DisplayName, UE.Reputation, UE.UpvoteCount, UE.DownvoteCount, (SELECT COUNT(*) FROM EnhancedPosts EP WHERE EP.Rank = 1 AND EP.PostId IS NOT NULL) AS TopPostCount,
//        STRING_AGG(DISTINCT CASE WHEN UE.ScoreCategory = 'Good' THEN UE.PostTitle END) AS GoodPosts, COUNT(DISTINCT UE.PostTitle) AS TotalEngagedPosts,
//        COALESCE(NULLIF(NULLIF(MAX(UE.DownvoteCount), 0), NULL), 0) AS NegativeEngagementLevel
// FROM UserEngagement UE GROUP BY UE.UserId, UE.DisplayName, UE.Reputation, UE.UpvoteCount, UE.DownvoteCount HAVING COUNT(UE.PostTitle) > 1
// ORDER BY UE.Reputation DESC, UE.UpvoteCount DESC LIMIT 50;
//
// `U.Id = PS.PostId` compares a user id with a post id, so each user group holds one row (COUNT(DISTINCT) is the row count). TopPostCount is the number of post type names with a post. The distinct titles are joined in sorted order.
fn q22201(db: &'static So) -> String {
    let Post { title, score, .. } = &db.post;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let tpc = db.post.group_by(ptype_name(db)).fold(0i64, |n, _| n + 1).fold_flat(0i64, |n, _| n + 1);
    let ue = db.user.group_by(Ident::<User>::new()).select((&db.user.origid).select(&pidx).select(title.opt().and(score)).opt()).buf_fold(|v| {
        let t: Vec<Str> = v.iter().filter_map(|x| x.and_then(|x| x.0)).collect();
        let g: Vec<Str> = v.iter().filter_map(|x| x.and_then(|x| if x.1 > 10 { x.0 } else { None })).collect();
        (t.len() as i64, t.len() as i64, agg_distinct(g, ","))
    });
    let v = drain(db.user.select(Ident::<User>::new().and(&uv).and((&ue).filt(|x: (i64, i64, Option<Str>)| x.0 > 1))));
    let v = top_n(v, |&(u, ((_, a), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])), 50);
    rows(v.into_iter().map(|(_, ((u, a), (_, d, g)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(tpc), ostr(g), V::I(d), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore,
//        ARRAY_AGG(t.TagName) AS Tags FROM Posts p LEFT JOIN Tags t ON t.WikiPostId = p.Id OR t.ExcerptPostId = p.Id WHERE p.CreationDate >= DATE('2024-10-01') - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.CreationDate < DATE('2024-10-01') - INTERVAL '6 months' GROUP BY u.Id, u.DisplayName),
// PostHistorySummary AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS HistoryCount, MIN(ph.CreationDate) AS FirstChange, MAX(ph.CreationDate) AS LastChange,
//        COUNT(CASE WHEN ph.UserId IS NULL THEN 1 END) AS AnonymousEdits FROM PostHistory ph WHERE ph.CreationDate >= DATE('2024-10-01') - INTERVAL '3 months' GROUP BY ph.PostId, ph.PostHistoryTypeId),
// UserReputation AS (SELECT u.Id AS UserId, SUM(u.Reputation) AS TotalReputation, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation IS NOT NULL GROUP BY u.Id)
// SELECT up.DisplayName, up.TotalPosts, up.UpVotes, up.DownVotes, phs.HistoryCount, phs.FirstChange, phs.LastChange, urep.TotalReputation, urep.BadgeCount, rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.Tags
// FROM UserActivity up JOIN PostHistorySummary phs ON phs.PostId = up.UserId JOIN UserReputation urep ON up.UserId = urep.UserId LEFT JOIN RankedPosts rp ON rp.PostId = phs.PostId
// WHERE urep.TotalReputation > 1000 AND phs.HistoryCount >= 3 ORDER BY rp.RankScore, up.TotalPosts DESC LIMIT 100;
//
// `phs.PostId = up.UserId` compares a post id with a user id; the posts x votes product is driven for the matched users alone. TotalReputation sums
// Reputation over the user x badges rows. A Score/ViewCount tie inside RankScore goes to the
// smaller post id, and the tag names are listed in tag id order (the SQL leaves both open).
fn q22829(db: &'static So) -> String {
    let d0 = date(2024, 10, 1);
    let Post { creation_date, post_type_id, score, view_count, origid, .. } = &db.post;
    let User { creation_date: ucd, reputation, .. } = &db.user;
    let PostHistory { creation_date: hd, post_history_type_id, post, .. } = &db.post_history;
    let phs = db.post_history.with(hd.ge(add_months(d0, -3))).group_by(post.and(post_history_type_id)).select(hd).fold((0i64, i64::MAX, i64::MIN), |(n, a, b), d| (n + 1, a.min(d), b.max(d)));
    type S = ((Id<Post>, i64), (i64, i64, i64));
    let pv = rel(drain((&phs).filt(|x: (i64, i64, i64)| x.0 >= 3)));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let okuser = || Ident::<User>::new().with(ucd.lt(add_months(d0, -6)));
    let k = || Same::<S>::new().map(|x: S| x.0 .0);
    let us: MatSet<Id<User>> = (&pv).map(|x: S| x.0 .0).select(origid).select(&uidx).select(okuser()).collect();
    let up = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let np = (&us).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tr = (&us).group_by(Ident::<User>::new()).select(reputation.and(badges_of(db).opt())).fold(0i64, |s, (r, _)| s + r);
    let nb = (&us).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let rw = db
        .post
        .with(creation_date.ge(add_years(d0, -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let rk = by_first(&(&rw).map(|(((p, _), _), k)| (p, k)).collect());
    let Tag { wiki_post, excerpt_post, tag_name, .. } = &db.tag;
    let pt: MatSet<(Id<Post>, Id<Tag>)> = db.tag.select(wiki_post.and(Ident::<Tag>::new())).union(db.tag.select(excerpt_post.and(Ident::<Tag>::new()))).collect();
    type T = (Id<Post>, Id<Tag>);
    let ti: HashIdx<Id<Post>, Id<Tag>> = (&pt).map(|x: T| x.0).inv().select((&pt).map(|x: T| x.1)).collect();
    let tags = db.post.with(&rk).group_by(Ident::<Post>::new()).select((&ti).select(tag_name).opt()).buf_fold(|v| &*Box::leak(v.to_vec().into_boxed_slice()));
    type W = (S, ((((Id<User>, [i64; 2]), i64), Option<i64>), i64));
    let kk = || Same::<W>::new().map(|x: W| x.0 .0 .0);
    let v = drain(
        (&pv)
            .select(Same::<S>::new().and(k().select(origid).select(&uidx).select(okuser()).select(Ident::<User>::new().and(&up).and(&np).and((&nb).opt()).and((&tr).filt(|t| t > 1000)))))
            .select(Same::<W>::new().and(kk().select(Ident::<Post>::new().and(&rk).and(&tags)).opt())),
    );
    let v = top_n(v, |&(_, ((_, ((((_, _), n), _), _)), r))| (r.is_none(), r.map(|r| r.0 .1), Reverse(n)), 100);
    rows(v.into_iter().map(|(_, (((_, (c, a, b)), ((((u, x), n), bc), t)), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(x[0]), V::I(x[1]), V::I(c), V::T(a), V::T(b), V::I(t), V::I(bc.unwrap_or(0))];
        match r {
            Some(((p, _), t)) => {
                f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
                f.push(V::L(t.iter().map(|&x| ostr(x)).collect()));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostCloseReasons AS (SELECT ph.PostId, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// TopPosts AS (SELECT rp.*, pcl.CloseReasons FROM RankedPosts rp LEFT JOIN PostCloseReasons pcl ON rp.PostId = pcl.PostId WHERE rp.Rank <= 5),
// PostSummary AS (SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, COALESCE(tp.CloseReasons, 'No close reasons') AS CloseReasons,
//        CASE WHEN tp.CommentCount > 10 THEN 'High Engagement' WHEN tp.CommentCount BETWEEN 5 AND 10 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel,
//        COALESCE(tp.UpVoteCount, 0) AS EffectiveUpVoteCount,
//        CONCAT('Post: ', tp.Title, ' - Engagement Level: ', CASE WHEN tp.CommentCount > 10 THEN 'High Engage' WHEN tp.CommentCount BETWEEN 5 AND 10 THEN 'Moderate Engage' ELSE 'Low Engage' END) AS EngagementDescription
//     FROM TopPosts tp),
// FinalOutput AS (SELECT *, CASE WHEN LENGTH(CloseReasons) > 0 THEN 'Has Close Reasons' ELSE 'Is Not Closed' END AS PostStatus, ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS FinalRank
//     FROM PostSummary)
// SELECT Title, CreationDate, Score, ViewCount, CloseReasons, EngagementLevel, EffectiveUpVoteCount, EngagementDescription, PostStatus, FinalRank
// FROM FinalOutput WHERE PostStatus = 'Is Not Closed' ORDER BY FinalRank, Score DESC;
//
// Rank reads only base columns, so the posts are ranked first; ties in both windows go to the smaller post id (the SQL leaves them open). The distinct reasons are joined in name order.
fn q23969(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let row_ = || Ident::<Post>::new().and(score).and(view_count.opt());
    type K = ((Id<Post>, i64), Option<i64>);
    let key = |((p, s), w): K| (Reverse(s), w.is_none(), Reverse(w), p);
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(row_()).window(row_number, key, asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let fw = whole(&tp).select(row_()).window(row_number, key, asc);
    let fr: MatSet<(Id<Post>, i64)> = (&fw).map(|(((p, _), _), r)| (p, r)).collect();
    let crt: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, post, .. } = &db.post_history;
    let pcl = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt)).buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    type F = (Id<Post>, i64);
    let k = || Same::<F>::new().map(|x: F| x.0);
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let v = drain((&fr).select(Same::<F>::new().and(k().select((&pcl).opt()).map(|c: Option<Str>| c.unwrap_or("No close reasons")).filt(|c: Str| c.chars().count() == 0)).and(k().select(&cc)).and(k().select(&up))));
    rows(v.into_iter().map(|(_, ((((p, r), c), n), u))| {
        let e = if n > 10 { "High" } else if n >= 5 { "Moderate" } else { "Low" };
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::S(c), V::Owned(format!("{e} Engagement")), V::I(u), V::Owned(format!("Post: {} - Engagement Level: {e} Engage", db.post.title.get(p).unwrap_or(""))), V::S("Is Not Closed"), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpvoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC NULLS LAST) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score >= 0),
// HighScoringPosts AS (SELECT rp.*, CASE WHEN rp.Score > 100 THEN 'High Score' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE NULL END AS ScoreCategory FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostHistoryDetails AS (SELECT ph.PostId, ph.Comment, ph.CreationDate AS HistCreationDate, p.Title AS PostTitle, ph.UserDisplayName,
//        CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 'Closed/Reopened' ELSE 'Other' END AS ActionType
//     FROM PostHistory ph JOIN Posts p ON p.Id = ph.PostId WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND ph.PostHistoryTypeId IN (10, 11, 12, 13)),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id)
// SELECT hsp.PostId, hsp.Title, hsp.Score, hsp.CommentCount, hsp.UpvoteCount, hsp.ScoreCategory, COALESCE(phd.ActionType, 'No Actions') AS MostRecentAction, COALESCE(ub.BadgeCount, 0) AS UserBadgeCount,
//        COALESCE(ub.BadgeNames, 'No Badges') AS UserBadges, CASE WHEN hsp.CommentCount = 0 THEN 'No Comments' WHEN hsp.UpvoteCount < 5 THEN 'Low Engagement' ELSE 'Engaged Post' END AS EngagementLevel
// FROM HighScoringPosts hsp LEFT JOIN PostHistoryDetails phd ON hsp.PostId = phd.PostId LEFT JOIN Users u ON hsp.PostId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE hsp.ScoreCategory IS NOT NULL ORDER BY hsp.CreationDate DESC NULLS LAST LIMIT 50;
//
// Rank reads only Score, so the posts are ranked first (a tie goes to the smaller post id; the SQL leaves it open). `hsp.PostId = u.Id` compares a post id with a user id.
// The badge names are joined in badge id order.
fn q21508(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).with(score.ge(0)).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let hsp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).select(Ident::<Post>::new().with(score.ge(50))).collect();
    let PostHistory { creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let phd = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_years(t0, -1))).with(post_history_type_id.is_in([10, 11, 12, 13]))).select(post_history_type_id);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name).opt()).buf_fold(|v| {
        let n: Vec<&str> = v.iter().filter_map(|x| *x).collect();
        (n.len() as i64, if n.is_empty() { None } else { Some(leak(n.join(", "))) })
    });
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let v = drain((&hsp).select(Ident::<Post>::new().and(&cc).and(&up).and(phd.opt()).and(origid.select(&uidx).select(&ub).opt())));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(_, ((((p, c), u), h), b))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(c), V::I(u), V::S(if s > 100 { "High Score" } else { "Medium Score" })]);
        f.push(V::S(match h {
            Some(10) | Some(11) => "Closed/Reopened",
            Some(_) => "Other",
            None => "No Actions",
        }));
        f.extend([V::I(b.map_or(0, |b| b.0)), V::S(b.and_then(|b| b.1).unwrap_or("No Badges"))]);
        f.push(V::S(if c == 0 { "No Comments" } else if u < 5 { "Low Engagement" } else { "Engaged Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.AcceptedAnswerId, p.OwnerUserId),
// RecentAcceptedAnswers AS (SELECT AcceptedAnswerId AS PostId, COUNT(*) AS AcceptedAnswerCount FROM Posts WHERE PostTypeId = 2
//     AND EXISTS (SELECT 1 FROM Posts p WHERE p.Id = Posts.AcceptedAnswerId AND p.Score > 0) GROUP BY AcceptedAnswerId),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT r.OwnerUserId, COALESCE(ue.DisplayName, 'Unknown User') AS UserName, SUM(r.Score) AS TotalScore, COUNT(r.PostId) AS TotalPosts, SUM(COALESCE(raa.AcceptedAnswerCount, 0)) AS TotalAcceptedAnswers,
//        MAX(r.RecentPostRank) AS MaxRecentPostRank, STRING_AGG(DISTINCT pht.Name, ', ') AS PostHistoryTypesChanged
// FROM RankedPosts r LEFT JOIN RecentAcceptedAnswers raa ON r.PostId = raa.PostId LEFT JOIN Users u ON r.OwnerUserId = u.Id LEFT JOIN PostHistory ph ON r.PostId = ph.PostId
// LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id LEFT JOIN UserEngagement ue ON u.Id = ue.UserId
// WHERE (r.RecentPostRank <= 3 AND r.CommentCount > 5) OR (r.CommentCount > 10 AND r.Score > 0) GROUP BY r.OwnerUserId, ue.DisplayName HAVING SUM(r.Score) > 50
// ORDER BY TotalScore DESC, UserName LIMIT 10;
//
// Only DisplayName is read from UserEngagement, so its product is not driven. A CreationDate tie inside RecentPostRank goes to the smaller post id; the distinct names are joined in name order.
fn q22750(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, score, post_type_id, accepted_answer, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    type R = (Id<Post>, i64);
    let rk: MatSet<R> = (&w).map(|((p, _), k)| (p, k)).collect();
    let cc = comments_per_post(db);
    let k = || Same::<R>::new().map(|x: R| x.0);
    type J = ((R, i64), i64);
    let rp = (&rk).select(Same::<R>::new().and(k().select(&cc)).and(k().select(score))).filt(|(((_, k), c), s): J| (k <= 3 && c > 5) || (c > 10 && s > 0));
    let raa = db.post.with(post_type_id.eq(2)).select(accepted_answer.select(Ident::<Post>::new().with(score.gt(0)))).inv().fold(0i64, |n, _| n + 1);
    let kj = || Same::<J>::new().map(|x: J| x.0 .0 .0);
    let g = rp
        .group_by(kj().select(owner_user_id.opt()).and(kj().select(owner_user.select(&db.user.display_name)).opt()))
        .select(Same::<J>::new().and(kj().select((&raa).opt())).and(kj().select(history_of(db).select(htype_name(db)).opt())))
        .buf_fold(|v| {
            let s: i64 = v.iter().map(|x| x.0 .0 .1).sum();
            let a: i64 = v.iter().map(|x| x.0 .1.unwrap_or(0)).sum();
            let m = v.iter().map(|x| x.0 .0 .0 .0 .1).max().unwrap();
            (s, v.len() as i64, a, m, agg_distinct(v.iter().filter_map(|x| x.1).collect(), ", "))
        });
    let v = top_n(drain((&g).filt(|x: (i64, i64, i64, i64, Option<Str>)| x.0 > 50)), |&((_, n), (s, ..))| (Reverse(s), n.unwrap_or("Unknown User")), 10);
    rows(v.into_iter().map(|((u, n), (s, c, a, m, t))| row(vec![oint(u), V::S(n.unwrap_or("Unknown User")), V::I(s), V::I(c), V::I(a), V::I(m), ostr(t)])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.Score, CASE WHEN p.Score > 0 THEN 'Positive' WHEN p.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreType,
//        ROW_NUMBER() OVER (PARTITION BY CASE WHEN p.Score > 0 THEN 'Positive' WHEN p.Score < 0 THEN 'Negative' ELSE 'Neutral' END ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// VoteStats AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVoteCount FROM Votes v
//     JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// PostHistoryDetails AS (SELECT ph.PostId, STRING_AGG(ph.Comment, ', ') AS Comments, MAX(CASE WHEN pht.Name = 'Post Closed' THEN ph.CreationDate END) AS LastClosedDate FROM PostHistory ph
//     JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId),
// FinalResults AS (SELECT rp.Id, rp.Title, rp.ViewCount, rp.Score, rp.ScoreType, COALESCE(vs.UpVoteCount, 0) AS UpVoteCount, COALESCE(vs.DownVoteCount, 0) AS DownVoteCount,
//        COALESCE(pjd.Comments, 'No Comments') AS Comments, CASE WHEN pjd.LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
//     FROM RankedPosts rp LEFT JOIN VoteStats vs ON rp.Id = vs.PostId LEFT JOIN PostHistoryDetails pjd ON rp.Id = pjd.PostId WHERE rp.rn <= 10)
// SELECT f.Id, f.Title, f.ViewCount, f.Score, f.ScoreType, f.UpVoteCount, f.DownVoteCount, f.Comments, f.PostStatus, CASE WHEN f.Score IS NULL THEN 'No Score' ELSE NULL END AS ScoreStatus,
//        SUM(COALESCE(CASE WHEN f.ScoreType = 'Positive' THEN 1 ELSE 0 END, 0)) OVER () AS TotalPositivePosts, SUM(COALESCE(CASE WHEN f.ScoreType = 'Negative' THEN 1 ELSE 0 END, 0)) OVER () AS TotalNegativePosts
// FROM FinalResults f WHERE f.ViewCount > 100 ORDER BY f.ViewCount DESC LIMIT 20;
//
// rn reads only base columns, so the posts are ranked first (a tie goes to the smaller post id). The windows OVER () count the rows the WHERE keeps. The STRING_AGG joins in history id order.
fn q20827(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let st = |s: i64| if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" };
    let w = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(score.map(st))
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fr: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).select(Ident::<Post>::new().with(view_count.gt(100))).collect();
    let (np, nn) = (&fr).select(score).fold_flat((0i64, 0i64), |(a, b), s| (a + (s > 0) as i64, b + (s < 0) as i64));
    let vs = (&fr).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some("UpMod")) as i64, a[1] + (t == Some("DownMod")) as i64]);
    let PostHistory { comment, creation_date: hd, post, .. } = &db.post_history;
    let pjd = db.post_history.group_by(post).select(comment.opt().and(htype_name(db)).and(hd)).buf_fold(|v| {
        let c: Vec<&str> = v.iter().filter_map(|x| x.0 .0).collect();
        (if c.is_empty() { None } else { Some(leak(c.join(", "))) }, v.iter().any(|x| x.0 .1 == "Post Closed"))
    });
    let v = drain((&fr).select(Ident::<Post>::new().and(&vs).and((&pjd).opt())));
    let v = top_n(v, |&(p, _)| Reverse(view_count.get(p)), 20);
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::S(st(score.get(p).unwrap())), V::I(a[0]), V::I(a[1]), V::S(h.and_then(|h| h.0).unwrap_or("No Comments"))]);
        f.extend([V::S(if h.map_or(false, |h| h.1) { "Closed" } else { "Open" }), V::Null, V::I(np), V::I(nn)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount, COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// HotQuestions AS (SELECT P.Id, P.Title, P.Score, COALESCE(P.ViewCount, 0) AS ViewCount, ROW_NUMBER() OVER (ORDER BY P.Score DESC, P.CreationDate DESC) AS HotRank FROM Posts P
//     WHERE P.PostTypeId = 1 AND P.Score > 10 AND (P.ViewCount IS NOT NULL OR P.ViewCount > 100)),
// ConnectionInfo AS (SELECT PL.PostId, COUNT(PL.RelatedPostId) AS RelatedPostCount, STRING_AGG(DISTINCT T.TagName, ', ') AS Tags FROM PostLinks PL JOIN Posts P ON PL.PostId = P.Id
//     LEFT JOIN Tags T ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY PL.PostId),
// PostHistorySummary AS (SELECT PH.PostId, COUNT(*) AS HistoryCount, MAX(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS Closed, MAX(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS Reopened,
//        MAX(CASE WHEN PH.PostHistoryTypeId = 12 THEN 1 ELSE 0 END) AS Deleted FROM PostHistory PH GROUP BY PH.PostId)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.ReputationRank, U.TotalBounty, HOT.Id AS HotPostId, HOT.Title AS HotPostTitle, HOT.Score AS HotPostScore, HOT.ViewCount AS HotPostViews,
//        COALESCE(CI.RelatedPostCount, 0) AS RelatedPostCount, COALESCE(CI.Tags, 'No Tags') AS TagsAssigned, PHS.HistoryCount, PHS.Closed, PHS.Reopened, PHS.Deleted
// FROM UserStats U LEFT JOIN HotQuestions HOT ON U.QuestionCount > 0 AND U.UserId = HOT.Id LEFT JOIN ConnectionInfo CI ON HOT.Id = CI.PostId LEFT JOIN PostHistorySummary PHS ON HOT.Id = PHS.PostId
// WHERE U.ReputationRank <= 50 AND (U.TotalBounty > 100 OR U.QuestionCount > 5) ORDER BY U.Reputation DESC, HOT.Score DESC NULLS LAST;
//
// ReputationRank reads only Reputation, so the users are ranked first and the votes x posts product is driven for them alone. `U.UserId = HOT.Id` compares a user id with a post id.
// The distinct tag names are joined in name order.
fn q20621(db: &'static So) -> String {
    let User { reputation, origid, .. } = &db.user;
    let uw = whole(&db.user.id).select(Ident::<User>::new().and(reputation)).window(dense_rank, |(_, r)| Reverse(r), asc);
    let rk = by_first(&(&uw).filt(|(_, k)| k <= 50).map(|((u, _), k)| (u, k)).collect());
    let top: MatSet<Id<User>> = (&uw).filt(|(_, k)| k <= 50).map(|((u, _), _)| u).collect();
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let tb = (&top)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(posts_of(db).opt()))
        .fold(0i64, |s, (b, _)| s + b.flatten().unwrap_or(0));
    let qc = (&top).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let us = (&tb).and(&qc);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let hot = Ident::<Post>::new().with(post_type_id.eq(1)).with(score.gt(10)).with(view_count);
    let tm = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    let mi: HashIdx<Id<Post>, Id<Tag>> = (&tm).map(|x: M| x.0).inv().select((&tm).map(|x: M| x.1)).collect();
    let ci = db.post.group_by(Ident::<Post>::new()).select(links_of(db).and((&mi).select(&db.tag.tag_name).opt())).buf_fold(|v| (v.len() as i64, agg_distinct(v.iter().filter_map(|x| x.1).collect(), ", ")));
    let phs = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).buf_fold(|v| (v.len() as i64, [10, 11, 12].map(|t| v.contains(&t) as i64)));
    type J = (Id<User>, (i64, i64));
    let v = drain(
        (&top)
            .select(Ident::<User>::new().and(&us))
            .filt(|(_, (b, q)): J| b > 100 || q > 5)
            .select(Same::<J>::new().and(Same::<J>::new().map(|x: J| x.0).select(&rk)).and(Same::<J>::new().filt(|(_, (_, q)): J| q > 0).map(|x: J| x.0).select(origid).select(&pidx).select(hot).select(Ident::<Post>::new().and((&ci).opt()).and((&phs).opt())).opt())),
    );
    rows(v.into_iter().map(|(_, (((u, (b, _)), k), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(k), V::I(b)]);
        match h {
            Some(((p, c), s)) => {
                f.extend(post_fields(db, p, &["id", "title", "score"]));
                f.push(V::I(view_count.get(p).unwrap_or(0)));
                f.extend([V::I(c.map_or(0, |c| c.0)), V::S(c.and_then(|c| c.1).unwrap_or("No Tags"))]);
                f.extend(match s {
                    Some((n, a)) => [V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])],
                    None => [V::Null, V::Null, V::Null, V::Null],
                });
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::I(0), V::S("No Tags"), V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerUserId, rp.CreationDate, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.rn = 1),
// TagsWithPostCounts AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, RANK() OVER (ORDER BY PostCount DESC) as Rank FROM TagsWithPostCounts WHERE PostCount >= 3),
// RecentClosures AS (SELECT p.Id AS PostId, ph.CreationDate AS ClosedDate, u.DisplayName AS ClosedBy FROM PostHistory ph JOIN Posts p ON p.Id = ph.PostId JOIN Users u ON ph.UserId = u.Id
//     WHERE ph.PostHistoryTypeId = 10 AND ph.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days'),
// ClosedPostCounts AS (SELECT rp.OwnerUserId, COUNT(rp.PostId) AS ClosedPostsCount FROM RecentClosures rc JOIN RankedPosts rp ON rc.PostId = rp.PostId GROUP BY rp.OwnerUserId)
// SELECT u.Id AS UserId, u.DisplayName, COUNT(tp.PostId) AS TotalPosts, SUM(COALESCE(cpc.ClosedPostsCount, 0)) AS TotalClosedPosts, SUM(tp.CommentCount) AS TotalComments,
//        SUM(tp.UpVoteCount) AS TotalUpVotes, SUM(tp.DownVoteCount) AS TotalDownVotes, STRING_AGG(tt.TagName, ', ') AS TopTags
// FROM Users u LEFT JOIN TopPosts tp ON u.Id = tp.OwnerUserId LEFT JOIN ClosedPostCounts cpc ON u.Id = cpc.OwnerUserId LEFT JOIN TopTags tt ON tt.Rank <= 3
// GROUP BY u.Id, u.DisplayName ORDER BY TotalPosts DESC, TotalUpVotes DESC LIMIT 100;
//
// rn reads only CreationDate, so the latest post of each owner is picked first (a tie goes to the smaller post id) and the comments x votes product is driven for those alone.
// `LEFT JOIN TopTags tt ON tt.Rank <= 3` names only tt, so every user row meets every top tag. The STRING_AGG joins the tags in rank order (the SQL leaves it open).
fn q24124(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = db.post.group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let tps: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let pc = (&tps).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tm = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    let tpc = (&tm).group_by(Same::<M>::new().map(|x: M| x.1).select(&db.tag.tag_name)).select(Same::<M>::new().map(|x: M| x.0)).count_distinct();
    let names: MatSet<Str> = db.tag.select(&db.tag.tag_name).collect();
    let tw = whole((&names).with((&tpc).filt(|n| n >= 3))).select(Same::<Str>::new().and(&tpc)).window(rank, |(_, n)| Reverse(n), asc);
    let tti: HashIdx<(), (i64, Str)> = (&tw).filt(|(_, k)| k <= 3).map(|((t, _), k)| (k, t)).collect();
    let PostHistory { post_history_type_id, creation_date: hd, user, post, .. } = &db.post_history;
    let cpc = db.post_history.with(post_history_type_id.eq(10)).with(hd.gt(add_days(date(2024, 10, 1), -30))).with(user).select(post).select(owner_user).inv().fold(0i64, |n, _| n + 1);
    let g = db
        .user
        .group_by(Ident::<User>::new())
        .select((&tp).select(&pc).opt().and((&cpc).opt()).and(Ident::<User>::new().map(|_| ()).select((&tti).opt())))
        .buf_fold(|v| {
            let mut t: Vec<(i64, Str)> = v.iter().filter_map(|x| x.1).collect();
            t.sort();
            let s = |i: usize| v.iter().map(|x| x.0 .0.map_or(0, |a| a[i])).sum::<i64>();
            let any = v.iter().any(|x| x.0 .0.is_some());
            let o = |x: i64| if any { Some(x) } else { None };
            (v.iter().filter(|x| x.0 .0.is_some()).count() as i64, v.iter().map(|x| x.0 .1.unwrap_or(0)).sum::<i64>(), o(s(0)), o(s(1)), o(s(2)), if t.is_empty() { None } else { Some(leak(t.iter().map(|x| x.1).collect::<Vec<_>>().join(", "))) })
        });
    let v = top_n(drain(&g), |&(_, (n, _, _, u, _, _))| (Reverse(n), u.is_none(), Reverse(u)), 100);
    rows(v.into_iter().map(|(u, (n, c, a, b, d, t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c), oint(a), oint(b), oint(d), ostr(t)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, RANK() OVER (ORDER BY COUNT(B.Id) DESC) AS BadgeRank FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, BadgeCount, COALESCE(GoldBadges, 0) AS GoldBadges, COALESCE(SilverBadges, 0) AS SilverBadges, COALESCE(BronzeBadges, 0) AS BronzeBadges FROM UserBadges WHERE BadgeCount > 0),
// PostActivity AS (SELECT P.OwnerUserId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, COUNT(P.Id) AS TotalPosts
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.OwnerUserId),
// FinalReport AS (SELECT U.Id AS UserId, U.DisplayName AS UserName, COALESCE(PA.CommentCount, 0) AS CommentsMade, COALESCE(PA.UpVoteCount, 0) AS UpVotesGiven, COALESCE(PA.DownVoteCount, 0) AS DownVotesGiven,
//        COALESCE(PA.TotalScore, 0) AS TotalScore, COALESCE(PA.TotalPosts, 0) AS PostsCreated, COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(UB.GoldBadges, 0) AS GoldBadges,
//        COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges FROM Users U LEFT JOIN PostActivity PA ON U.Id = PA.OwnerUserId LEFT JOIN TopUsers UB ON U.Id = UB.UserId)
// SELECT UserName, COUNT(UserId) AS UserCount, AVG(TotalScore) AS AverageScore, SUM(CommentsMade) AS TotalComments, SUM(UpVotesGiven) AS TotalUpVotes, SUM(DownVotesGiven) AS TotalDownVotes,
//        SUM(TotalBadges) AS OverallBadges,
//        STRING_AGG(DISTINCT CONCAT('Gold: ', CAST(GoldBadges AS TEXT), ', Silver: ', CAST(SilverBadges AS TEXT), ', Bronze: ', CAST(BronzeBadges AS TEXT)), '; ') AS BadgeSummary
// FROM FinalReport GROUP BY UserName HAVING AVG(TotalScore) > 0 AND SUM(CommentsMade) > 1 ORDER BY AverageScore DESC, UserCount DESC LIMIT 10;
//
// The distinct badge summaries are joined in sorted order (the SQL leaves it open).
fn q24058(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let pa = db
        .post
        .group_by(owner_user)
        .select(score.and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, ((s, c), t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + s]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let g = db
        .user
        .group_by(&db.user.display_name)
        .select((&pa).opt().and((&ub).opt()))
        .buf_fold(|v| {
            let p = |i: usize| v.iter().map(|x| x.0.map_or(0, |a| a[i])).sum::<i64>();
            let b: Vec<Str> = v.iter().map(|x| {
                let b = x.1.unwrap_or([0; 4]);
                leak(format!("Gold: {}, Silver: {}, Bronze: {}", b[1], b[2], b[3]))
            }).collect();
            (v.len() as i64, [p(3), p(0), p(1), p(2)], v.iter().map(|x| x.1.map_or(0, |b| b[0])).sum::<i64>(), agg_distinct(b, "; ").unwrap())
        });
    type G = (i64, [i64; 4], i64, Str);
    let v = top_n(drain((&g).filt(|(n, a, _, _): G| a[0] > 0 && a[1] > 1 && n > 0)), |&(_, (n, a, _, _))| (Reverse(fkey(a[0] as f64 / n as f64)), Reverse(n)), 10);
    rows(v.into_iter().map(|(u, (n, a, b, s))| row(vec![V::S(u), V::I(n), avg(a[0], n), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(b), V::S(s)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COALESCE(NULLIF(u.Reputation, 0), 1) AS EffectiveReputation FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Rank, COALESCE(ph.Comment, 'No history comment') AS LastEditComment, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId AND v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId AND ph.PostHistoryTypeId = 24 WHERE rp.Rank <= 5 GROUP BY rp.PostId, rp.Title, rp.ViewCount, rp.Rank, ph.Comment),
// FilteredPostDetails AS (SELECT *, CASE WHEN UpVotes > DownVotes THEN 'Popular' WHEN UpVotes < DownVotes THEN 'Controversial' ELSE 'Neutral' END AS Sentiment,
//        CASE WHEN ViewCount > 100 THEN 'Trending' ELSE 'Normal' END AS TrendingStatus FROM PostDetails WHERE LastEditComment LIKE '%helpful%' OR LastEditComment IS NULL),
// FinalOutput AS (SELECT f.PostId, f.Title, f.ViewCount, f.CommentCount, f.Sentiment, f.TrendingStatus, pht.Name AS LastEditType FROM FilteredPostDetails f
//     LEFT JOIN PostHistory ph ON f.PostId = ph.PostId AND ph.PostHistoryTypeId IN (5, 6, 10) LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id ORDER BY f.ViewCount DESC, f.CommentCount DESC)
// SELECT * FROM FinalOutput WHERE ViewCount IS NOT NULL OR CommentCount IS NOT NULL
// UNION SELECT NULL AS PostId, 'Aggregate Count' AS Title, COUNT(*) AS ViewCount, SUM(CommentCount) AS CommentCount, NULL AS Sentiment, NULL AS TrendingStatus, NULL AS LastEditType FROM FinalOutput;
//
// Rank reads only CreationDate, so the posts are ranked first (a tie goes to the smaller post id). LastEditComment is never NULL, so FilteredPostDetails keeps the groups whose comment
// contains 'helpful'. UNION removes duplicate rows, which the port does by collecting them into a set.
fn q21865(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user_id, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    type R = (Id<Post>, Option<Id<PostHistory>>);
    let h24 = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(24)));
    let rr = (&rp).select(Ident::<Post>::new().and(h24.opt()));
    let key = Same::<R>::new().map(|x: R| x.0).and(Same::<R>::new().flat_map(|x: R| x.1).select(comment).opt());
    let recent_v = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_years(t0, -1)))).select(&db.vote.vote_type_id);
    let pd = rr
        .group_by(key)
        .select(Same::<R>::new().map(|x: R| x.0).select(comments_of(db).opt().and(recent_v.opt())))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type K = (Id<Post>, Option<Str>);
    type P = (K, [i64; 2]);
    let cc = comments_per_post(db);
    let h5 = || history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([5, 6, 10]))).select(htype_name(db));
    let fp = rel(drain(&pd));
    type Q = (P, (i64, Option<Str>));
    let fov = || (&fp).filt(|((_, c), _): P| like(c.unwrap_or("No history comment"), "%helpful%")).select(Same::<P>::new().and(Same::<P>::new().map(|x: P| x.0 .0).select((&cc).and(h5().opt()))));
    let (n, s) = fov().fold_flat((0i64, 0i64), |(n, s), x: Q| (n + 1, s + x.1 .0));
    type O = (Id<Post>, i64, Str, Str, Option<Str>);
    let out: MatSet<O> = fov()
        .map(|(((p, _), a), (c, t)): Q| -> O {
            (p, c, if a[0] > a[1] { "Popular" } else if a[0] < a[1] { "Controversial" } else { "Neutral" }, if view_count.get(p).map_or(false, |w| w > 100) { "Trending" } else { "Normal" }, t)
        })
        .collect();
    let mut r: Vec<String> = drain(&out).into_iter().map(|(_, (p, c, se, tr, t))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c), V::S(se), V::S(tr), ostr(t)]);
        row(f)
    }).collect();
    r.push(row(vec![V::Null, V::S("Aggregate Count"), V::I(n), if n == 0 { V::Null } else { V::I(s) }, V::Null, V::Null, V::Null]));
    rows(r)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName FROM Users u WHERE u.Reputation < (SELECT AVG(Reputation) FROM Users) AND u.Location IS NOT NULL),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// UserActivity AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE v.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year' GROUP BY v.UserId)
// SELECT up.DisplayName, up.Reputation, rb.PostId, rb.Title AS LatestPostTitle, rb.CreationDate AS LatestPostDate, rb.Score AS LatestPostScore, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        ua.VoteCount AS RecentVoteCount, ua.UpVotes, ua.DownVotes,
//        CASE WHEN rb.Score < 0 THEN 'Needs Improvement' WHEN rb.Score BETWEEN 1 AND 10 THEN 'Moderate Engagement' ELSE 'Highly Engaged' END AS EngagementLevel,
//        STRING_AGG(DISTINCT TRIM(REGEXP_REPLACE(rb.Tags, '[<>]', '')), ', ') AS ConcatenatedTags
// FROM UserReputation up JOIN RankedPosts rb ON up.UserId = rb.OwnerUserId LEFT JOIN UserBadges ub ON up.UserId = ub.UserId LEFT JOIN UserActivity ua ON up.UserId = ua.UserId
// WHERE rb.Rank = 1 AND ub.BadgeCount IS NOT NULL AND ua.VoteCount IS NOT NULL
// GROUP BY up.DisplayName, up.Reputation, rb.PostId, rb.Title, rb.CreationDate, rb.Score, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ua.VoteCount, ua.UpVotes, ua.DownVotes
// HAVING COUNT(DISTINCT rb.PostId) > 0 ORDER BY up.Reputation DESC, rb.CreationDate DESC;
//
// Each group is one user's latest question. REGEXP_REPLACE without 'g' removes only the first '<' or '>'. A CreationDate tie inside Rank goes to the smaller post id.
fn q20086(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, .. } = &db.post;
    let User { reputation, location, .. } = &db.user;
    let (rs, rn) = db.user.select(reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let ra = rs as f64 / rn as f64;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rb: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let cut = ny_to_utc(add_years(utc_to_ny(now_utc()), -1));
    let ua = db.vote.with((&db.vote.creation_date).filt(move |d| ny_to_utc(d) >= cut)).group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == "UpMod") as i64, a[2] + (t == "DownMod") as i64]);
    let v = drain(db.user.with(reputation.filt(move |r| (r as f64) < ra)).with(location).select(Ident::<User>::new().and(&rb).and(&ub).and(&ua)));
    let v = top_n(v, |&(u, (((_, p), _), _))| (Reverse(reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())), 0);
    rows(v.into_iter().map(|(_, (((u, p), b), a))| {
        let s = db.post.score.get(p).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend(b.map(V::I));
        f.extend(a.map(V::I));
        f.push(V::S(if s < 0 { "Needs Improvement" } else if (1..=10).contains(&s) { "Moderate Engagement" } else { "Highly Engaged" }));
        f.push(ostr(tags_str.get(p).map(|t| leak(t.replacen(['<', '>'], "", 1).trim_matches(' ').to_string()))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.Score, p.ViewCount, p.CreationDate, p.AcceptedAnswerId,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankByScore FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// PostDetails AS (SELECT r.PostId, r.Title, r.Score, r.ViewCount, CASE WHEN r.AcceptedAnswerId IS NULL THEN 'No Accepted Answer' ELSE 'Accepted Answer Exists' END AS AcceptedAnswerStatus,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = r.PostId) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = r.PostId AND v.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = r.PostId AND v.VoteTypeId = 3) AS DownVotes FROM RankedPosts r WHERE r.RankByScore <= 5),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsMade, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 0),
// ClosedPosts AS (SELECT ph.PostId, ph.UserId, pt.Name AS CloseReason FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id WHERE ph.PostHistoryTypeId = 10),
// UnionResults AS (SELECT pd.PostId, pd.Title, pd.Score, pd.ViewCount, pd.AcceptedAnswerStatus, pd.CommentCount, 'Top Posts' AS Source FROM PostDetails pd
//     UNION ALL SELECT NULL AS PostId, NULL AS Title, NULL AS Score, NULL AS ViewCount, NULL AS AcceptedAnswerStatus, NULL AS CommentCount, 'Closed Posts' AS Source FROM ClosedPosts cp)
// SELECT ur.*, COALESCE(u.DisplayName, 'Anonymous') AS UserDisplayName, u.PostsMade, u.TotalUpVotes, u.TotalDownVotes
// FROM UnionResults ur LEFT JOIN UserActivity u ON ur.PostId = u.UserId ORDER BY ur.Source, ur.Score DESC;
//
// RankByScore reads only base columns, so the posts are ranked first. `ur.PostId = u.UserId` compares a post id with a user id, so UserActivity is driven for the matched users alone;
// the Closed Posts rows have a NULL PostId and match nothing.
fn q24212(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let pd: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let us: MatSet<Id<User>> = (&pd).select(origid).select(&uidx).select(Ident::<User>::new().with(posts_of(db))).collect();
    let ua = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let np = (&us).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let v = drain((&pd).select(Ident::<Post>::new().and(&cc).and(&up).and(origid.select(&uidx).select(Ident::<User>::new().and(&np).and(&ua)).opt())));
    let mut out: Vec<String> = v.into_iter().map(|(_, (((p, c), _), u))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.push(V::S(if db.post.accepted_answer_id.get(p).is_none() { "No Accepted Answer" } else { "Accepted Answer Exists" }));
        f.extend([V::I(c), V::S("Top Posts")]);
        f.extend(match u {
            Some(((u, n), a)) => [user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1])],
            None => [V::S("Anonymous"), V::Null, V::Null, V::Null],
        });
        row(f)
    }).collect();
    let closed = drain(db.post_history.with((&db.post_history.post_history_type_id).eq(10)));
    out.extend(closed.into_iter().map(|_| row(vec![V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::S("Closed Posts"), V::S("Anonymous"), V::Null, V::Null, V::Null])));
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Ranking
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Body, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Ranking <= 5),
// PostVoteSummary AS (SELECT PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY PostId),
// PostCommentSummary AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate, STRING_AGG(DISTINCT pht.Name, ', ') AS EditTypes FROM PostHistory ph
//     JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY ph.PostId),
// FinalPostReport AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Body, tp.ViewCount, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes,
//        COALESCE(cs.CommentCount, 0) AS CommentCount, COALESCE(phs.EditCount, 0) AS EditCount, phs.LastEditDate, phs.EditTypes
//     FROM TopPosts tp LEFT JOIN PostVoteSummary pvs ON tp.PostId = pvs.PostId LEFT JOIN PostCommentSummary cs ON tp.PostId = cs.PostId LEFT JOIN PostHistorySummary phs ON tp.PostId = phs.PostId)
// SELECT fpr.PostId, fpr.Title, fpr.CreationDate, fpr.Body, fpr.ViewCount, fpr.UpVotes, fpr.DownVotes, fpr.CommentCount, fpr.EditCount, fpr.LastEditDate, fpr.EditTypes,
//        CASE WHEN fpr.UpVotes > fpr.DownVotes THEN 'Positive' WHEN fpr.UpVotes < fpr.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM FinalPostReport fpr ORDER BY fpr.ViewCount DESC, fpr.LastEditDate DESC;
//
// Ranking reads only base columns, so the posts are ranked first. The distinct type names are joined in name order (the SQL leaves it open).
fn q22819(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = comments_per_post(db);
    let PostHistory { creation_date: hd, post, .. } = &db.post_history;
    let phs = db.post_history.with(hd.ge(add_months(t0, -1))).group_by(post).select(hd.and(htype_name(db))).buf_fold(|v| (v.len() as i64, v.iter().map(|x| x.0).max().unwrap(), agg_distinct(v.iter().map(|x| x.1).collect(), ", ").unwrap()));
    let v = drain((&tp).select(Ident::<Post>::new().and(&pvs).and(&cc).and((&phs).opt())));
    rows(v.into_iter().map(|(_, (((p, a), c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "body", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.extend(match h {
            Some((n, d, t)) => [V::I(n), V::T(d), V::S(t)],
            None => [V::I(0), V::Null, V::Null],
        });
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RecentActivePosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.Score, p.ViewCount, CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted Answer Exists' ELSE 'No Accepted Answer' END AS AcceptedAnswerStatus,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountiesWon FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN Votes v ON u.Id = v.UserId AND v.VoteTypeId IN (9, 10) GROUP BY u.Id, u.Reputation),
// PostAnalysis AS (SELECT p.Id, p.Title, u.DisplayName, u.Reputation, up.BadgeCount, p.Score, p.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(cl.Reason, 'N/A') AS CloseReason,
//        NTILE(4) OVER (ORDER BY p.Score DESC) AS ScoreQuartile
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserReputation up ON u.Id = up.UserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) AS c ON p.Id = c.PostId
//     LEFT JOIN (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS Reason FROM PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment = CAST(cr.Id AS VARCHAR) WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId) AS cl
//     ON p.Id = cl.PostId)
// SELECT pa.Title, pa.DisplayName, pa.Reputation, pa.BadgeCount, pa.Score, pa.ViewCount, pa.CommentCount, pa.CloseReason, ra.AcceptedAnswerStatus,
//        CASE WHEN pa.Score > 0 THEN 'Positive' WHEN pa.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreSentiment,
//        CASE WHEN pa.Score > 50 THEN 'High Engagement' WHEN pa.Score BETWEEN 20 AND 50 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM PostAnalysis pa LEFT JOIN RecentActivePosts ra ON pa.Id = ra.PostId WHERE pa.Score IS NOT NULL AND pa.ViewCount > 0 ORDER BY pa.Reputation DESC, pa.Score DESC OFFSET 10 ROWS FETCH NEXT 20 ROWS ONLY;
//
// The WHERE and ORDER BY read only base columns, so the 30 posts are picked first and the badges x votes product is driven for their owners alone. The STRING_AGG joins in history id order.
fn q23345(db: &'static So) -> String {
    let Post { owner_user, view_count, score, creation_date, accepted_answer_id, .. } = &db.post;
    let v = drain(db.post.with(view_count.gt(0)).select(owner_user.select(&db.user.reputation).and(score)));
    let v = top_n(v, |&(_, (r, s))| (Reverse(r), Reverse(s)), 30);
    let pa: MatSet<Id<Post>> = rel(v.into_iter().skip(10).map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&pa).select(owner_user).collect();
    let bc = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt().and(votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([9, 10]))).opt())).fold(0i64, |n, (b, _)| n + b.is_some() as i64);
    let crt: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i: i64| -> Str { leak(i.to_string()) }).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, post, .. } = &db.post_history;
    let cl = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(comment.select(&crt)).buf_fold(|v| leak(v.join(", ")));
    let cc = comments_per_post(db);
    let recent = Ident::<Post>::new().with(creation_date.ge(add_days(date(2024, 10, 1), -30)));
    let v = drain((&pa).select(Ident::<Post>::new().and(owner_user.select(&bc)).and(&cc).and((&cl).opt()).and(recent.opt())));
    rows(v.into_iter().map(|(_, ((((p, b), c), r), ra))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.extend([V::I(c), V::S(r.unwrap_or("N/A"))]);
        f.push(match ra {
            Some(_) => V::S(if accepted_answer_id.get(p).is_some() { "Accepted Answer Exists" } else { "No Accepted Answer" }),
            None => V::Null,
        });
        f.push(V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }));
        f.push(V::S(if s > 50 { "High Engagement" } else if s >= 20 { "Moderate Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U WHERE U.Reputation IS NOT NULL),
// UserBadges AS (SELECT B.UserId, COUNT(*) AS BadgeCount, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Badges B GROUP BY B.UserId),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.AnswerCount, P.ViewCount, COALESCE(P.AcceptedAnswerId > 0, FALSE) AS HasAcceptedAnswer FROM Posts P
//     WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '30 days'),
// PostHistoryAnalysis AS (SELECT PH.PostId, MAX(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN PH.CreationDate END) AS LastClosedDate, MAX(CASE WHEN PH.PostHistoryTypeId = 12 THEN PH.CreationDate END) AS LastDeletedDate,
//        COUNT(*) FILTER (WHERE PH.UserId IS NOT NULL) * 1.0 / NULLIF(COUNT(*), 0) AS VoteRatio FROM PostHistory PH GROUP BY PH.PostId),
// UserPostDetails AS (SELECT U.Id AS UserId, U.DisplayName, P.Title, P.CreationDate, COALESCE(PH.LastClosedDate, DATE '2099-12-31') AS LastClosedDate, COALESCE(PH.LastDeletedDate, DATE '2099-12-31') AS LastDeletedDate,
//        PH.VoteRatio, COUNT(COALESCE(CM.Id, NULL)) AS CommentCount, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(UB.BadgeNames, 'No Badges') AS BadgeNames
//     FROM Users U JOIN RecentPosts P ON U.Id = P.OwnerUserId LEFT JOIN PostHistoryAnalysis PH ON P.PostId = PH.PostId LEFT JOIN Comments CM ON P.PostId = CM.PostId LEFT JOIN UserBadges UB ON U.Id = UB.UserId
//     GROUP BY U.Id, U.DisplayName, P.Title, P.CreationDate, PH.LastClosedDate, PH.LastDeletedDate, PH.VoteRatio, UB.BadgeCount, UB.BadgeNames),
// FinalReport AS (SELECT U.UserId, U.DisplayName, U.Title, U.CreationDate, U.LastClosedDate, U.LastDeletedDate, U.VoteRatio, U.CommentCount, U.BadgeCount, U.BadgeNames,
//        CASE WHEN U.LastDeletedDate > U.LastClosedDate THEN 'Deleted Recently' WHEN U.LastClosedDate IS NOT NULL THEN 'Closed Recently' ELSE 'Active' END AS PostStatus,
//        CASE WHEN U.VoteRatio IS NULL OR U.VoteRatio < 0.5 THEN 'Needs Attention' ELSE 'Well Voted' END AS VoteStatus FROM UserPostDetails U WHERE U.CommentCount > 0)
// SELECT * FROM FinalReport ORDER BY UserId, CreationDate DESC;
//
// Every recent post has its own PostHistoryAnalysis row or none, so the groups are the user's posts keyed by title and date; the badge names are joined in badge id order.
fn q21211(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date: hd, user, post, .. } = &db.post_history;
    let pha = db.post_history.group_by(post).select(post_history_type_id.and(hd).and(user.opt())).buf_fold(|v| {
        let lc = v.iter().filter(|x| [10, 11].contains(&x.0 .0)).map(|x| x.0 .1).max();
        let ld = v.iter().filter(|x| x.0 .0 == 12).map(|x| x.0 .1).max();
        (lc, ld, [v.iter().filter(|x| x.1.is_some()).count() as i64, v.len() as i64])
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let g = db
        .post
        .with(creation_date.ge(add_days(current_date(), -30)))
        .group_by(owner_user.and(title.opt()).and(creation_date).and((&pha).opt()).and(owner_user.select(&ub).opt()))
        .select(comments_of(db).opt())
        .fold(0i64, |n, c| n + c.is_some() as i64);
    let far = date(2099, 12, 31);
    let v = drain((&g).filt(|n| n > 0));
    rows(v.into_iter().map(|(((((u, t), d), h), b), n)| {
        let (lc, ld, r) = match h {
            Some((a, b, r)) => (a.unwrap_or(far), b.unwrap_or(far), Some(r[0] as f64 / r[1] as f64)),
            None => (far, far, None),
        };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([ostr(t), V::T(d), V::T(lc), V::T(ld), ofloat(r), V::I(n), V::I(b.map_or(0, |b| b.0)), V::S(b.map_or("No Badges", |b| b.1))]);
        f.push(V::S(if ld > lc { "Deleted Recently" } else { "Closed Recently" }));
        f.push(V::S(if r.map_or(true, |r| r < 0.5) { "Needs Attention" } else { "Well Voted" }));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COALESCE(BadgeCount, 0) AS BadgeCount, COALESCE(PostCount, 0) AS PostCount,
//        COALESCE(CommentCount, 0) AS CommentCount, COALESCE(VoteCount, 0) AS VoteCount
//     FROM Users U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON U.Id = B.UserId LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS PostCount FROM Posts GROUP BY OwnerUserId) P ON U.Id = P.OwnerUserId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS CommentCount FROM Comments GROUP BY UserId) C ON U.Id = C.UserId LEFT JOIN (SELECT UserId, COUNT(*) AS VoteCount FROM Votes GROUP BY UserId) V ON U.Id = V.UserId),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.AcceptedAnswerId, COUNT(DISTINCT C.Id) AS TotalComments, AVG(P.Score) AS AvgPostScore FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.AcceptedAnswerId),
// UserPostHistory AS (SELECT U.UserId, PH.PostId, PH.CreationDate, P.Title, PH.UserDisplayName, PH.PostHistoryTypeId,
//        CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 'Close/Reopen' WHEN PH.PostHistoryTypeId IN (12, 13) THEN 'Delete/Undelete' ELSE 'Other' END AS HistoryType,
//        ROW_NUMBER() OVER (PARTITION BY U.UserId ORDER BY PH.CreationDate DESC) AS RowNum
//     FROM UserStatistics U JOIN PostHistory PH ON U.UserId = PH.UserId JOIN Posts P ON PH.PostId = P.Id
//     WHERE PH.CreationDate BETWEEN TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 years' AND TIMESTAMP '2024-10-01 12:34:56')
// SELECT U.DisplayName, U.Reputation, U.BadgeCount, P.Title AS PostTitle, DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank,
//        COUNT(DISTINCT PH.PostId) FILTER (WHERE PH.HistoryType = 'Close/Reopen') AS CloseReopenCount, COUNT(DISTINCT PH.PostId) FILTER (WHERE PH.HistoryType = 'Delete/Undelete') AS DeleteUndeleteCount,
//        STRING_AGG(DISTINCT PH.UserDisplayName, ', ') AS RelatedUserNames
// FROM UserStatistics U JOIN UserPostHistory PH ON U.UserId = PH.UserId JOIN PostDetails P ON PH.PostId = P.PostId WHERE U.Reputation > 1000 AND P.AvgPostScore >= 1
// GROUP BY U.DisplayName, U.Reputation, U.BadgeCount, P.Title HAVING COUNT(P.PostId) > 3 ORDER BY U.Reputation DESC, P.Title;
//
// PostDetails is one row per post, so AvgPostScore is the post's Score. The distinct names are joined in sorted order (the SQL leaves it open).
fn q24199(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, score, title, .. } = &db.post;
    let User { reputation, display_name, .. } = &db.user;
    let PostHistory { creation_date: hd, user, post, post_history_type_id, user_display_name, .. } = &db.post_history;
    let pd = Ident::<Post>::new().with(creation_date.ge(add_years(t0, -1))).with(score.ge(1));
    let by_user: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(hd.between(add_years(t0, -2), t0)).with(post.select(pd)).select(user).inv().collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type X = (Id<User>, Id<PostHistory>);
    let xs = || db.user.with(reputation.gt(1000)).select(Ident::<User>::new().and(&by_user));
    let key = || Same::<X>::new().map(|x: X| x.0).select(display_name.and(reputation).and(&bc)).and(Same::<X>::new().map(|x: X| x.1).select(post).select(title.opt()));
    let h = || Same::<X>::new().map(|x: X| x.1);
    let n = xs().group_by(key()).fold(0i64, |n, _| n + 1);
    let cd = |ts: &'static [i64]| xs().group_by(key()).select(h().select(Ident::<PostHistory>::new().with(post_history_type_id.filt(move |t| ts.contains(&t)))).select(post)).count_distinct();
    let (cr, du) = (cd(&[10, 11]), cd(&[12, 13]));
    let un = xs().group_by(key()).select(h().select(user_display_name.opt())).buf_fold(|v| agg_distinct(v.iter().filter_map(|x| *x).collect(), ", "));
    type K = (((Str, i64), i64), Option<Str>);
    type G = (K, (((i64, Option<i64>), Option<i64>), Option<Str>));
    let gv = rel(drain((&n).filt(|n| n > 3).and((&cr).opt()).and((&du).opt()).and(&un)));
    let w = whole(&gv).select(&gv).window(dense_rank, |(k, _): G| Reverse(k.0 .0 .1), asc);
    rows(drain(&w).into_iter().map(|(_, (((((nm, rp), b), t), (((_, c), d), u)), k))| row(vec![V::S(nm), V::I(rp), V::I(b), ostr(t), V::I(k), V::I(c.unwrap_or(0)), V::I(d.unwrap_or(0)), ostr(u)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank, STRING_AGG(tags.TagName, ', ') AS Tags
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN LATERAL (SELECT unnest(string_to_array(p.Tags, ', ')) AS tag) AS tag ON true
//     JOIN Tags tags ON tags.TagName = TRIM(BOTH '"' FROM tag.tag) WHERE p.PostTypeId = 1 AND p.Score > 0 GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score, p.OwnerUserId),
// TopPosts AS (SELECT rp.*, RANK() OVER (ORDER BY rp.Score DESC) AS OverallRank FROM RankedPosts rp WHERE rp.Rank <= 3)
// SELECT tp.OwnerDisplayName, tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.Tags, tp.OverallRank FROM TopPosts tp WHERE tp.OverallRank <= 10 ORDER BY tp.OverallRank, tp.Score DESC;
//
// A Score tie inside Rank goes to the smaller post id, and the tag names are joined in join order (the SQL leaves both open).
fn q7349(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, tags_str, .. } = &db.post;
    let tidx: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let hit = tags_str.flat_map(|s: Str| s.split(", ").map(|t| t.trim_matches('"')).collect::<Vec<_>>()).select(&tidx);
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .with(score.gt(0))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(hit.select(&db.tag.tag_name)))
        .buf_fold(|v| (v.iter().filter(|x| x.0.is_some()).count() as i64, leak(v.iter().map(|x| x.1).collect::<Vec<_>>().join(", "))));
    let w = db.post.with(&rp).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 3).map(|((p, _), _)| p).collect();
    let ow = whole(&top).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    type T = (Id<Post>, i64);
    let ok: MatSet<T> = (&ow).filt(|(_, k)| k <= 10).map(|((p, _), k)| (p, k)).collect();
    let v = drain((&ok).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select(&rp))));
    rows(v.into_iter().map(|(_, ((p, k), (c, t)))| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "score"]);
        f.extend([V::I(c), V::S(t), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Tags, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT rp.Id, rp.Title, rp.Tags, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.rn <= 5),
// PostScoreSummary AS (SELECT STRING_AGG(DISTINCT t.TagName, ', ') AS Tags, COUNT(fp.Id) AS QuestionCount, SUM(fp.Score) AS TotalScore, AVG(fp.ViewCount) AS AverageViewCount
//     FROM FilteredPosts fp JOIN Posts p ON fp.Id = p.Id JOIN LATERAL (SELECT unnest(string_to_array(fp.Tags, ',')) AS TagName) AS t ON TRUE GROUP BY t.TagName)
// SELECT ps.Tags, ps.QuestionCount, ps.TotalScore, ps.AverageViewCount,
//        CASE WHEN ps.QuestionCount > 10 THEN 'High Engagement' WHEN ps.QuestionCount > 5 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM PostScoreSummary ps ORDER BY ps.TotalScore DESC;
//
// rn reads only base columns, so the posts are ranked first (a CreationDate tie goes to the smaller post id). Each group has one TagName, so the distinct list is that name.
fn q25774(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(tags_str.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let g = (&fp)
        .select(tags_str.flat_map(|s: Str| s.split(',').collect::<Vec<_>>()))
        .inv()
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    rows(drain(&g).into_iter().map(|(t, a)| row(vec![V::S(t), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::S(if a[0] > 10 { "High Engagement" } else if a[0] > 5 { "Moderate Engagement" } else { "Low Engagement" })])))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS TotalBadges, STRING_AGG(Name, ', ') AS BadgeNames FROM Badges WHERE Class = 1 GROUP BY UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers,
//        COALESCE(SUM(p.ViewCount), 0) AS TotalViews, MAX(p.CreationDate) AS LastPostDate FROM Posts p GROUP BY p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(ub.TotalBadges, 0) AS GoldBadgeCount, COALESCE(ps.TotalPosts, 0) AS TotalPostsCount, COALESCE(ps.TotalQuestions, 0) AS TotalQuestionsCount,
//        COALESCE(ps.TotalAcceptedAnswers, 0) AS TotalAcceptedAnswersCount, COALESCE(ps.TotalViews, 0) AS TotalPostViews
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId),
// RankedUsers AS (SELECT UserId, Reputation, GoldBadgeCount, TotalPostsCount, TotalQuestionsCount, TotalAcceptedAnswersCount, TotalPostViews,
//        RANK() OVER (ORDER BY Reputation DESC, TotalPostsCount DESC) AS UserRank FROM UserReputation WHERE Reputation IS NOT NULL)
// SELECT ru.UserId, ru.Reputation, ru.GoldBadgeCount, ru.TotalPostsCount, ru.TotalQuestionsCount, ru.TotalAcceptedAnswersCount, ru.TotalPostViews, ru.UserRank,
//        STRING_AGG(DISTINCT ph.Comment, '; ') FILTER (WHERE ph.Comment IS NOT NULL) AS HistoryComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        (SELECT COUNT(*) FROM Votes v2 WHERE v2.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = ru.UserId) AND v2.VoteTypeId = 3) AS TotalDownVotes
// FROM RankedUsers ru LEFT JOIN PostHistory ph ON ru.UserId = ph.UserId LEFT JOIN Votes v ON v.UserId = ru.UserId
// GROUP BY ru.UserId, ru.Reputation, ru.GoldBadgeCount, ru.TotalPostsCount, ru.TotalQuestionsCount, ru.TotalAcceptedAnswersCount, ru.TotalPostViews, ru.UserRank
// HAVING COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) > COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) ORDER BY ru.UserRank;
//
// A user who cast no vote has both sums 0 and fails the HAVING, so the history x votes product is driven for the voters alone. The distinct comments are joined in sorted order.
fn q24582(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, view_count, .. } = &db.post;
    let gold = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let ps = db.post.group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt()).and(view_count.opt())).fold([0i64; 4], |a, ((t, c), w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + c.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let uw = whole(&db.user.id).select(Ident::<User>::new().and((&db.user.reputation).and((&ps).opt()))).window(rank, |(_, (r, p))| (Reverse(r), Reverse(p.map_or(0, |p| p[0]))), asc);
    let ki = by_first(&(&uw).map(|((u, x), k)| (u, (x, k))).collect());
    let by_user: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let voters: MatSet<Id<User>> = db.user.with(votes_by(db)).collect();
    let g = (&voters)
        .group_by(Ident::<User>::new())
        .select((&by_user).select((&db.post_history.comment).opt()).opt().and(votes_by(db).select(&db.vote.vote_type_id)))
        .buf_fold(|v| {
            let up = v.iter().filter(|x| x.1 == 2).count() as i64;
            let dn = v.iter().filter(|x| x.1 == 3).count() as i64;
            (up, dn, agg_distinct(v.iter().filter_map(|x| x.0.flatten()).collect(), "; "))
        });
    let down = db.post.group_by(&db.post.owner_user).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(3)))).fold(0i64, |n, _| n + 1);
    let v = drain((&g).filt(|(u, d, _): (i64, i64, Option<Str>)| u > d).and(&ki).and((&gold).opt()).and((&down).opt()));
    rows(v.into_iter().map(|(u, ((((up, _, c), ((r, p), k)), gb), dn))| {
        let p = p.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "uid"), V::I(r), V::I(gb.unwrap_or(0)), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), V::I(k)];
        f.extend([ostr(c), V::I(up), V::I(dn.unwrap_or(0))]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("31090", q31090),
    ("33131", q33131),
    ("31154", q31154),
    ("27883", q27883),
    ("23866", q23866),
    ("32383", q32383),
    ("28792", q28792),
    ("23614", q23614),
    ("22682", q22682),
    ("21514", q21514),
    ("29235", q29235),
    ("32055", q32055),
    ("21957", q21957),
    ("6602", q6602),
    ("25841", q25841),
    ("3914", q3914),
    ("4452", q4452),
    ("412", q412),
    ("29037", q29037),
    ("4891", q4891),
    ("25576", q25576),
    ("21230", q21230),
    ("255", q255),
    ("27355", q27355),
    ("21250", q21250),
    ("33964", q33964),
    ("325", q325),
    ("33629", q33629),
    ("22506", q22506),
    ("22249", q22249),
    ("21397", q21397),
    ("20607", q20607),
    ("20856", q20856),
    ("24512", q24512),
    ("24731", q24731),
    ("22228", q22228),
    ("7177", q7177),
    ("23002", q23002),
    ("20389", q20389),
    ("20678", q20678),
    ("23890", q23890),
    ("22126", q22126),
    ("20755", q20755),
    ("23114", q23114),
    ("23297", q23297),
    ("22880", q22880),
    ("28796", q28796),
    ("23583", q23583),
    ("20507", q20507),
    ("22266", q22266),
    ("22201", q22201),
    ("22829", q22829),
    ("23969", q23969),
    ("21508", q21508),
    ("22750", q22750),
    ("20827", q20827),
    ("20621", q20621),
    ("24124", q24124),
    ("24058", q24058),
    ("21865", q21865),
    ("20086", q20086),
    ("24212", q24212),
    ("22819", q22819),
    ("23345", q23345),
    ("21211", q21211),
    ("24199", q24199),
    ("7349", q7349),
    ("25774", q25774),
    ("24582", q24582),
];
