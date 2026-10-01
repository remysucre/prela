use harness::prelude::*;
use std::cmp::Reverse;

fn ids_set<X: Copy>(v: &[X], f: impl Fn(&X) -> Id<User>) -> MatSet<Id<User>> {
    rel(v.iter().map(|x| f(x)).collect()).map(|u| u).collect()
}

fn post_set<X: Copy>(v: &[X], f: impl Fn(&X) -> Id<Post>) -> MatSet<Id<Post>> {
    rel(v.iter().map(|x| f(x)).collect()).map(|p| p).collect()
}

// WITH RecursivePostHistory AS (SELECT ph.Id, ph.PostId, ph.CreationDate, ph.UserId, ph.UserDisplayName, ph.PostHistoryTypeId,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank FROM PostHistory ph WHERE ph.UserId IS NOT NULL),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, COUNT(c.Id) AS TotalComments, COUNT(DISTINCT v.UserId) AS TotalVotes,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostWithLatestHistory AS (SELECT rp.*, COALESCE(rh.UserDisplayName, 'Unknown') AS LastEditor,
//        CASE WHEN rh.PostHistoryTypeId IN (10, 11) THEN 'Closed' WHEN rh.PostHistoryTypeId IS NULL THEN 'Active' ELSE 'Edited' END AS PostStatus
//     FROM RecentPosts rp LEFT JOIN RecursivePostHistory rh ON rp.PostId = rh.PostId AND rh.HistoryRank = 1)
// SELECT pwlh.PostId, pwlh.Title, pwlh.OwnerUserId, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, pwlh.TotalComments, pwlh.TotalVotes, pwlh.UpVotes, pwlh.DownVotes, pwlh.LastEditor, pwlh.PostStatus,
//        CASE WHEN pwlh.TotalVotes > 0 THEN ROUND((1.0 * pwlh.UpVotes / pwlh.TotalVotes) * 100, 2)::TEXT || '%' ELSE 'N/A' END AS VoteRatio
// FROM PostWithLatestHistory pwlh LEFT JOIN UserBadges ub ON pwlh.OwnerUserId = ub.UserId ORDER BY pwlh.TotalVotes DESC, pwlh.Title;
//
// The latest history row is only needed for the recent posts, so the ranking is done for those alone.
fn q23397(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let Vote { vote_type_id, user_id, .. } = &db.vote;
    let PostHistory { user, creation_date: hd, user_display_name, post_history_type_id, .. } = &db.post_history;
    let recent = || db.post.with(creation_date.ge(add_days(current_date(), -30)));
    let rp = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id.and(user_id.opt())).opt()))
        .buf_fold(|v| {
            let c = v.iter().filter(|x| x.0.is_some()).count() as i64;
            let d = distinct_some(v.iter().map(|x| x.1.and_then(|y| y.1)));
            let up = v.iter().filter(|x| x.1.map(|y| y.0) == Some(2)).count() as i64;
            let dn = v.iter().filter(|x| x.1.map(|y| y.0) == Some(3)).count() as i64;
            [c, d, up, dn]
        });
    let hs = drain(recent().select(history_of(db).select(Ident::<PostHistory>::new().with(user))));
    let hs = top_per(hs, |&(p, _)| p, |&(_, h)| Reverse(hd.get(h).unwrap()), 1, false);
    let hr = rel(hs);
    let hi: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&hr).map(|(p, _)| p).inv().select(&hr).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    rows(drain((&rp).and((&hi).opt()).and(owner_user.select(&ub).opt())).into_iter().map(|(p, ((a, h), b))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "owner_id"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        let h = h.map(|x| x.1);
        f.push(V::S(h.and_then(|h| user_display_name.get(h)).unwrap_or("Unknown")));
        let t = h.map(|h| post_history_type_id.get(h).unwrap());
        f.push(V::S(match t {
            Some(10) | Some(11) => "Closed",
            None => "Active",
            _ => "Edited",
        }));
        f.push(if a[1] > 0 { V::Owned(format!("{:?}%", (a[2] as f64 / a[1] as f64 * 100.0 * 100.0).round() / 100.0)) } else { V::S("N/A") });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty,
//        ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation IS NOT NULL GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBounty, UserRank FROM UserStatistics WHERE UserRank <= 10),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, MAX(CASE WHEN vs.UserId IS NOT NULL THEN 1 ELSE 0 END) AS HasAcceptedAnswer
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Votes vs ON p.AcceptedAnswerId = vs.PostId AND vs.VoteTypeId = 1
//     WHERE p.CreationDate >= (CURRENT_DATE - INTERVAL '1 year') GROUP BY p.Id, p.Title),
// PostSummary AS (SELECT pa.PostId, pa.Title, pa.CommentCount, pa.UpVoteCount, pa.DownVoteCount, pa.HasAcceptedAnswer,
//        ROW_NUMBER() OVER (ORDER BY pa.UpVoteCount - pa.DownVoteCount DESC) AS PostRank FROM PostActivity pa)
// SELECT tu.DisplayName, tu.Reputation, ps.Title AS PostTitle, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, ps.HasAcceptedAnswer,
//        (CASE WHEN ps.HasAcceptedAnswer = 1 THEN 'Yes' ELSE 'No' END) AS AcceptedAnswer,
//        (CASE WHEN ps.CommentCount > 0 THEN 'Active Discussion' ELSE 'No Comments' END) AS DiscussionStatus
// FROM TopUsers tu JOIN PostSummary ps ON tu.UserId = (SELECT OwnerUserId FROM Posts WHERE Title = ps.Title LIMIT 1)
// WHERE ps.PostRank <= 5 ORDER BY tu.Reputation DESC, ps.UpVoteCount DESC;
//
// Only the post count of UserStatistics is read. The unordered `LIMIT 1` takes the post with the smallest id among those with the title, and the
// ROW_NUMBER ties go to the smaller id; the answer is empty on this data, so neither choice is tested.
fn q20335(db: &'static So) -> String {
    let Post { creation_date, title, owner_user, accepted_answer_id, .. } = &db.post;
    let Vote { vote_type_id, user_id, post_id, .. } = &db.vote;
    let tu = top_n(drain(&user_distinct_posts(db)), |&(u, n)| (Reverse(n), u), 10);
    let tus = ids_set(&tu, |x| x.0);
    let vby: HashIdx<i64, Id<Vote>> = db.vote.with(vote_type_id.eq(1)).select(post_id).inv().collect();
    let pa = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt()).and(accepted_answer_id.select(&vby).select(user_id.opt()).opt()))
        .fold([0i64; 4], |a, ((c, t), s)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3].max(s.flatten().is_some() as i64)]);
    let ps = top_n(drain(&pa), |&(p, a)| (Reverse(a[1] - a[2]), p), 5);
    let psr = rel(ps);
    let by_title: HashIdx<Str, Id<Post>> = title.inv().collect();
    let first = (&psr).map(|(p, _)| p).select(title).group_by(Same::<Str>::new()).select(&by_title).fold(None, |m: Option<Id<Post>>, q| Some(m.map_or(q, |m| m.min(q))));
    let v = drain((&psr).select(Same::<(Id<Post>, [i64; 4])>::new().and(Same::<(Id<Post>, [i64; 4])>::new().map(|x: (Id<Post>, [i64; 4])| x.0).select(title).select(&first).flat_map(|m: Option<Id<Post>>| m).select(owner_user).with(&tus))));
    rows(v.into_iter().map(|(_, ((p, a), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::S(if a[3] == 1 { "Yes" } else { "No" }), V::S(if a[0] > 0 { "Active Discussion" } else { "No Comments" })]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.OwnerUserId, p.AcceptedAnswerId, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotesCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotesCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT um.UserId, um.DisplayName, um.Reputation, um.PostCount, um.GoldBadges, um.SilverBadges, um.BronzeBadges, um.UpVotesCount, um.DownVotesCount,
//        RANK() OVER (ORDER BY um.Reputation DESC) AS ReputationRank FROM UserMetrics um)
// SELECT ru.PostId, ru.Title AS PostTitle, ru.PostTypeId, ru.CreationDate, ru.OwnerUserId, u.DisplayName AS OwnerDisplayName, COALESCE(u.Reputation, 0) AS OwnerReputation,
//        COALESCE(ru.Score, 0) AS PostScore, CASE WHEN ru.Rank = 1 THEN 'Latest Post' ELSE 'Older Post' END AS PostCategory,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = ru.PostId) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = ru.PostId AND v.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = ru.PostId AND v.VoteTypeId = 3) AS DownVotes, t.ReputationRank
// FROM RecentPosts ru JOIN Users u ON ru.OwnerUserId = u.Id JOIN TopUsers t ON u.Id = t.UserId WHERE ru.AcceptedAnswerId IS NULL
// ORDER BY ru.CreationDate DESC LIMIT 100 OFFSET 0;
//
// TopUsers is every user and only its rank is read. Rank = 1 is the owner's latest recent post, compared against the latest date per owner.
fn q22588(db: &'static So) -> String {
    let Post { creation_date, owner_user, accepted_answer_id, .. } = &db.post;
    let vote_type_id = &db.vote.vote_type_id;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let latest = recent().group_by(owner_user).select(creation_date).fold(i64::MIN, |m, d| m.max(d));
    let rk = ranked(drain(db.user.select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let rk = rel(rk.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rki: HashIdx<Id<User>, (Id<User>, i64)> = (&rk).map(|(u, _)| u).inv().select(&rk).collect();
    let v = top_n(drain(recent().minus(accepted_answer_id).select(owner_user)), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let tp = post_set(&v, |x| x.0);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc).and(owner_user.select(Ident::<User>::new().and(&latest).and(&rki)))).into_iter().map(|(p, ((c, a), ((u, l), (_, r))))| {
        let mut f = post_fields(db, p, &["id", "title", "type_id", "created", "owner_id"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["score"]));
        f.push(V::S(if creation_date.get(p).unwrap() == l { "Latest Post" } else { "Older Post" }));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS RankReputation, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId,
//        CASE WHEN p.AcceptedAnswerId IS NULL THEN 'No Accepted Answer' ELSE 'Has Accepted Answer' END AS AnswerStatus, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.PostTypeId = 1),
// UserPostAnalytics AS (SELECT ur.UserId, ur.DisplayName, COUNT(DISTINCT pp.PostId) AS QuestionsAsked, SUM(CASE WHEN pp.AcceptedAnswerId <> -1 THEN 1 ELSE 0 END) AS QuestionsAnswered,
//        AVG(pp.Score) AS AvgQuestionScore FROM UserReputation ur LEFT JOIN TopPosts pp ON ur.UserId = pp.AcceptedAnswerId GROUP BY ur.UserId, ur.DisplayName),
// RankedAnalytics AS (SELECT upa.UserId, upa.DisplayName, upa.QuestionsAsked, upa.QuestionsAnswered, upa.AvgQuestionScore,
//        RANK() OVER (ORDER BY upa.AvgQuestionScore DESC, upa.QuestionsAsked DESC) AS RankByScore FROM UserPostAnalytics upa),
// FinalReport AS (SELECT ra.UserId, ra.DisplayName, ra.QuestionsAsked, ra.QuestionsAnswered, ra.AvgQuestionScore,
//        CASE WHEN ra.QuestionsAnswered > 0 THEN 'Active Responder' ELSE 'Inactive Responder' END AS ResponseCategory,
//        CASE WHEN rb.RankReputation <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserCategory
//     FROM RankedAnalytics ra LEFT JOIN UserReputation rb ON ra.UserId = rb.UserId)
// SELECT fr.UserId, fr.DisplayName, fr.QuestionsAsked, fr.QuestionsAnswered, fr.AvgQuestionScore, fr.ResponseCategory, fr.UserCategory
// FROM FinalReport fr WHERE fr.QuestionsAsked > 0 AND fr.AvgQuestionScore IS NOT NULL ORDER BY fr.AvgQuestionScore DESC, fr.QuestionsAsked DESC LIMIT 100;
//
// `ur.UserId = pp.AcceptedAnswerId` joins a user id to a post id (or -1), so it goes through the raw ids.
fn q23755(db: &'static So) -> String {
    let Post { creation_date, post_type_id, accepted_answer_id, score, .. } = &db.post;
    let tp = Ident::<Post>::new()
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(post_type_id.eq(1))
        .select(accepted_answer_id.opt())
        .map(|a: Option<i64>| a.unwrap_or(-1));
    let byk: HashIdx<i64, Id<Post>> = db.post.select(tp).inv().collect();
    let top = top_n(drain(db.user.select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let top = ids_set(&top, |x| x.0);
    let upa = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.origid).select(&byk).select(score.and(accepted_answer_id.opt())))
        .fold([0i64; 3], |a, (s, acc)| [a[0] + 1, a[1] + acc.is_some() as i64, a[2] + s]);
    let v = top_n(drain((&upa).and((&top).opt())), |&(u, (a, _))| (Reverse(fkey(a[2] as f64 / a[0] as f64)), Reverse(a[0]), u), 100);
    rows(v.into_iter().map(|(u, (a, t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[0])]);
        f.push(V::S(if a[1] > 0 { "Active Responder" } else { "Inactive Responder" }));
        f.push(V::S(if t.is_some() { "Top User" } else { "Regular User" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClassSum FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate BETWEEN DATE('2024-10-01') - INTERVAL '1 year' AND DATE('2024-10-01') GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.AcceptedAnswerId),
// EnhancedRankedPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.Upvotes, pd.Downvotes,
//        ROW_NUMBER() OVER (ORDER BY pd.ViewCount DESC) AS ViewRank, ROW_NUMBER() OVER (ORDER BY pd.AnswerCount DESC) AS AnswerRank FROM PostDetails pd)
// SELECT ub.UserId, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, COALESCE(er.PostId, -1) AS TopPostId, er.Title AS TopPostTitle, er.ViewCount AS TopPostViewCount,
//        er.AnswerCount AS TopPostAnswerCount, er.CommentCount AS TopPostCommentCount, (ub.TotalBadgeClassSum + COALESCE(er.Upvotes, 0) - COALESCE(er.Downvotes, 0)) AS UserEngagementScore,
//        CASE WHEN ub.TotalBadgeClassSum IS NULL THEN 'No Badges' ELSE 'Has Badges' END AS BadgeStatus,
//        CASE WHEN ub.TotalBadgeClassSum IS NULL AND er.PostId IS NULL THEN 'No Engagement' WHEN ub.TotalBadgeClassSum IS NOT NULL AND er.PostId IS NULL THEN 'Engaged, No Posts'
//        ELSE 'Engaged with Posts' END AS EngagementStatus
// FROM UserBadges ub LEFT JOIN EnhancedRankedPosts er ON ub.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = er.PostId LIMIT 1)
// ORDER BY UserEngagementScore DESC, BadgeStatus, EngagementStatus;
//
// The subquery reads er's own post, so the join is on er's owner. TotalBadgeClassSum is a SUM of COALESCE over at least one row, never NULL.
fn q23203(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let vote_type_id = &db.vote.vote_type_id;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + c.unwrap_or(0)]
    });
    let pd = db
        .post
        .with(creation_date.between(date(2023, 10, 1), date(2024, 10, 1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt()))
        .buf_fold(|v| {
            let c = distinct_some(v.iter().map(|x| x.0));
            let up = v.iter().filter(|x| x.1 == Some(2)).count() as i64;
            let dn = v.iter().filter(|x| x.1 == Some(3)).count() as i64;
            [c, up, dn]
        });
    rows(drain((&ub).and(posts_of(db).select(Ident::<Post>::new().and(&pd)).opt())).into_iter().map(|(u, (b, e))| {
        let mut f = ucols(db, u, &["uid"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        match e {
            Some((p, a)) => {
                f.extend(post_fields(db, p, &["id", "title", "views", "answers"]));
                f.extend([V::I(a[0]), V::I(b[3] + a[1] - a[2])]);
            }
            None => f.extend([V::I(-1), V::Null, V::Null, V::Null, V::Null, V::I(b[3])]),
        }
        f.push(V::S("Has Badges"));
        f.push(V::S(if e.is_none() { "Engaged, No Posts" } else { "Engaged with Posts" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostAggregates AS (SELECT P.Id AS PostId, P.OwnerUserId, COUNT(DISTINCT C.Id) AS CommentCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
//     WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY P.Id, P.OwnerUserId),
// RankedPosts AS (SELECT PA.PostId, PA.OwnerUserId, PA.CommentCount, PA.TotalBounty, RANK() OVER (PARTITION BY PA.OwnerUserId ORDER BY PA.CommentCount DESC) AS CommentRank,
//        DENSE_RANK() OVER (ORDER BY PA.TotalBounty DESC) AS BountyRank FROM PostAggregates PA),
// UserDetails AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UBC.GoldBadges, 0) AS GoldBadges, COALESCE(UBC.SilverBadges, 0) AS SilverBadges,
//        COALESCE(UBC.BronzeBadges, 0) AS BronzeBadges, RP.PostId, RP.CommentCount, RP.TotalBounty, RP.CommentRank, RP.BountyRank
//     FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN RankedPosts RP ON U.Id = RP.OwnerUserId WHERE U.Reputation > 0)
// SELECT UD.DisplayName, UD.Reputation, UD.GoldBadges, UD.SilverBadges, UD.BronzeBadges, UD.CommentCount,
//        CASE WHEN UD.TotalBounty > 0 THEN 'Has Bounties' ELSE 'No Bounties' END AS BountyStatus,
//        CASE WHEN UD.CommentRank IS NULL THEN 'N/A' ELSE CAST(UD.CommentRank AS varchar) END AS CommentRank,
//        CASE WHEN UD.BountyRank IS NULL THEN 'N/A' ELSE CAST(UD.BountyRank AS varchar) END AS BountyRank
// FROM UserDetails UD WHERE (UD.CommentCount > 0 OR UD.TotalBounty > 0) AND (UD.GoldBadges + UD.SilverBadges + UD.BronzeBadges) > 0
// ORDER BY UD.Reputation DESC, UD.CommentCount DESC;
//
// The WHERE needs a RankedPosts row, so the LEFT JOIN to it behaves as an inner join.
fn q20691(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let pa = db
        .post
        .with(creation_date.ge(date(2023, 10, 1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt()).opt()))
        .buf_fold(|v| (distinct_some(v.iter().map(|x| x.0)), v.iter().map(|x| x.1.flatten().unwrap_or(0)).sum::<i64>()));
    let v = drain((&pa).and(owner_user.opt()));
    let v = ranked(v, |&(_, (a, _))| Reverse(a.1), true);
    let v = ranked(v, |&((_, (a, o)), _)| (o, Reverse(a.0)), false);
    let v = per_group(v, |&((_, (_, o)), _)| o);
    type R = (((Id<Post>, ((i64, i64), Option<Id<User>>)), i64), i64);
    let r = rel(v);
    let byu: HashIdx<Id<User>, R> = (&r).filt(|x: R| x.0 .0 .1 .0 .0 > 0 || x.0 .0 .1 .0 .1 > 0).flat_map(|x: R| x.0 .0 .1 .1).inv().select(&r).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(0)).select(Ident::<User>::new().and((&ubc).filt(|a| a[0] + a[1] + a[2] > 0)).and(&byu)));
    rows(v.into_iter().map(|(_, ((u, b), (((_, ((c, t), _)), br), cr)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(c)]);
        f.push(V::S(if t > 0 { "Has Bounties" } else { "No Bounties" }));
        f.push(V::Owned(cr.to_string()));
        f.push(V::Owned(br.to_string()));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, AcceptedAnswers, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS Rank
//     FROM UserStats WHERE Reputation > 1000),
// RecentPostHistory AS (SELECT p.Id AS PostId, p.Title, ph.UserDisplayName, ph.CreationDate, ph.Comment, ph.Text FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') AND ph.PostHistoryTypeId IN (10, 11, 12)),
// UserPostDetails AS (SELECT u.UserId, u.DisplayName, COALESCE(rp.RecentEditCount, 0) AS RecentEditCount, COALESCE(th.TopPostedCount, 0) AS TopPostedCount
//     FROM TopUsers u LEFT JOIN (SELECT p.OwnerUserId, COUNT(DISTINCT rp.PostId) AS RecentEditCount FROM Posts p JOIN RecentPostHistory rp ON p.Id = rp.PostId GROUP BY p.OwnerUserId) rp
//     ON u.UserId = rp.OwnerUserId
//     LEFT JOIN (SELECT p.OwnerUserId, COUNT(*) AS TopPostedCount FROM Posts p GROUP BY p.OwnerUserId HAVING COUNT(*) > 10) th ON u.UserId = th.OwnerUserId)
// SELECT ud.DisplayName, ud.RecentEditCount, ut.Reputation, ut.PostCount, ut.AnswerCount, ut.AcceptedAnswers, ud.TopPostedCount
// FROM UserPostDetails ud JOIN TopUsers ut ON ud.UserId = ut.UserId ORDER BY ut.Rank LIMIT 10;
//
// Rank reads only Reputation, so the ten users are picked first and the post x vote product is driven for them alone.
fn q2509(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let PostHistory { creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let rep = &db.user.reputation;
    let tu = top_n(drain(db.user.with(rep.gt(1000)).select(rep)), |&(u, r)| (Reverse(r), u), 10);
    let tus = ids_set(&tu, |x| x.0);
    let us = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(post_type_id).and(accepted_answer_id.opt()).and(votes_of(db).opt())).opt())
        .buf_fold(|v| {
            let n = distinct_some(v.iter().map(|x| x.map(|y| y.0 .0 .0)));
            let a = v.iter().filter(|x| x.map(|y| y.0 .0 .1) == Some(2)).count() as i64;
            let q = v.iter().filter(|x| x.map_or(false, |y| y.0 .0 .1 == 1 && y.0 .1.is_some())).count() as i64;
            [n, a, q]
        });
    let rph = history_of(db).select(Ident::<PostHistory>::new().with(hd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(post_history_type_id.is_in([10, 11, 12])));
    let re = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).with(rph).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let th = (&tus).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&us).and(&re).and((&th).filt(|n| n > 10).opt()));
    let v = top_n(v, |&(u, _)| (Reverse(rep.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, ((a, r), t))| {
        let mut f = ucols(db, u, &["name"]);
        f.push(V::I(r));
        f.extend(ucols(db, u, &["rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(t.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostInfo AS (SELECT P.Id AS PostId, P.OwnerUserId, P.PostTypeId, P.Title, P.CreationDate, P.AcceptedAnswerId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN V.Id END) AS UpvoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN V.Id END) AS DownvoteCount,
//        COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN PH.Id END) AS CloseVotes, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     GROUP BY P.Id, P.OwnerUserId, P.PostTypeId, P.Title, P.CreationDate, P.AcceptedAnswerId),
// ActiveUsers AS (SELECT U.Id, U.Reputation, U.DisplayName, U.CreationDate, (SELECT COUNT(*) FROM Posts WHERE OwnerUserId = U.Id) AS TotalPosts,
//        (SELECT COUNT(*) FROM Comments WHERE UserId = U.Id) AS TotalComments FROM Users U WHERE U.LastAccessDate >= DATE '2024-10-01' - INTERVAL '30 days')
// SELECT U.DisplayName AS ActiveUser, COALESCE(UB.BadgeCount, 0) AS Badges, COALESCE(UB.GoldBadges, 0) AS Gold, COALESCE(UB.SilverBadges, 0) AS Silver,
//        COALESCE(UB.BronzeBadges, 0) AS Bronze, COUNT(DISTINCT PI.PostId) AS TotalPosts, SUM(PI.CommentCount) AS TotalComments, SUM(PI.UpvoteCount) AS TotalUpvotes,
//        SUM(PI.DownvoteCount) AS TotalDownvotes, SUM(PI.CloseVotes) AS TotalCloseVotes
// FROM ActiveUsers U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostInfo PI ON U.Id = PI.OwnerUserId WHERE U.Reputation > 1000
// GROUP BY U.DisplayName, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, U.Reputation HAVING COUNT(DISTINCT PI.PostId) > 5
// ORDER BY U.Reputation DESC, TotalPosts DESC LIMIT 10;
//
// The groups (name, badge counts, reputation) and their post counts are found first; the comment x vote x history product is driven only for the
// users of the ten groups kept.
fn q23808(db: &'static So) -> String {
    let User { reputation, last_access_date, display_name, .. } = &db.user;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let au = || db.user.with(last_access_date.ge(date(2024, 9, 1))).with(reputation.gt(1000));
    let key = display_name.and(reputation).and(&ub);
    let gp = au().group_by(&key).select(posts_of(db).opt()).buf_fold(distinct_some);
    let top = top_n(drain((&gp).filt(|n| n > 5)), |&(((_, r), _), n)| (Reverse(r), Reverse(n)), 10);
    let tk = rel(top.clone());
    let keep: MatSet<((Str, i64), [i64; 4])> = (&tk).map(|(k, _)| k).collect();
    let users: MatSet<Id<User>> = au().with((&key).with(&keep)).collect();
    let pi = (&users)
        .group_by(&key)
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt())))
        .fold([0i64; 4], |a, ((c, v), h)| [a[0] + c.is_some() as i64, a[1] + (v == Some(2)) as i64, a[2] + (v == Some(3)) as i64, a[3] + (h == Some(10)) as i64]);
    rows(drain((&pi).and(&gp)).into_iter().map(|(((nm, _), b), (a, n))| row(vec![V::S(nm), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])])))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, NTILE(4) OVER (ORDER BY u.Reputation DESC) AS ReputationQuartile
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// PostHistoryDetails AS (SELECT p.Id AS PostId, MAX(ph.CreationDate) AS LastEditDate, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Comment END) AS CloseReason,
//        COUNT(DISTINCT p2.Id) AS RelatedPostLinks, SUM(CASE WHEN ph.PostHistoryTypeId IN (14, 15) THEN 1 ELSE 0 END) AS LockUnlockCount
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId LEFT JOIN Posts p2 ON pl.RelatedPostId = p2.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id),
// UserFinalStats AS (SELECT us.UserId, us.DisplayName, us.TotalPosts, us.QuestionCount, us.AnswerCount, us.Reputation, us.ReputationQuartile,
//        COALESCE(p.LastEditDate, '1970-01-01') AS LastPostEdit, COALESCE(p.CloseReason, 'No Close Reason') AS LastCloseReason, COALESCE(p.RelatedPostLinks, 0) AS RelatedPostLinks,
//        COALESCE(p.LockUnlockCount, 0) AS LockUnlockCount FROM UserStats us LEFT JOIN PostHistoryDetails p ON us.UserId = p.PostId)
// SELECT ufs.DisplayName, ufs.TotalPosts, ufs.QuestionCount, ufs.AnswerCount, ufs.Reputation, ufs.LastPostEdit, ufs.LastCloseReason, ufs.RelatedPostLinks, ufs.LockUnlockCount,
//        CASE WHEN ufs.ReputationQuartile IS NULL THEN 'No Data' ELSE CONCAT('Q', ufs.ReputationQuartile) END AS ReputationBand,
//        CASE WHEN ufs.AnswerCount / NULLIF(ufs.QuestionCount, 0) > 2 THEN 'High Answer Rate' WHEN ufs.AnswerCount IS NULL THEN 'No Answers' ELSE 'Normal Answer Rate' END AS AnswerRateDescription
// FROM UserFinalStats ufs WHERE ufs.Reputation > 1000 ORDER BY ufs.Reputation DESC, ufs.TotalPosts DESC LIMIT 100;
//
// `us.UserId = p.PostId` joins a user id to a post id, so it goes through the raw ids. NTILE numbers the users in reputation order; a tie at a
// bucket boundary would make it ambiguous, which the top hundred are nowhere near.
fn q20667(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let PostHistory { creation_date: hd, post_history_type_id, comment, .. } = &db.post_history;
    let rep = &db.user.reputation;
    let ups = user_posts(db);
    let all = ranked(drain(db.user.select(rep)), |&(u, r)| (Reverse(r), u), false);
    let n = all.len() as i64;
    let (q, rm) = (n / 4, n % 4);
    let nt = rel(all.into_iter().map(|((u, _), i)| {
        let i = i - 1;
        let b = if i < rm * (q + 1) { i / (q + 1) } else { rm + (i - rm * (q + 1)) / q };
        (u, b + 1)
    }).collect());
    let nti: HashIdx<Id<User>, i64> = (&nt).map(|(u, _)| u).inv().select((&nt).map(|(_, b)| b)).collect();
    let dp = user_distinct_posts(db);
    let v = top_n(drain(db.user.with(rep.gt(1000)).select(&dp)), |&(u, n)| (Reverse(rep.get(u).unwrap()), Reverse(n), u), 100);
    let tu = ids_set(&v, |x| x.0);
    let phd = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(hd.and(post_history_type_id).and(comment.opt())).opt().and(links_of(db).select((&db.post_link.related_post).opt()).opt()))
        .buf_fold(|v| {
            let d = v.iter().filter_map(|x| x.0.map(|y| y.0 .0)).max();
            let c = v.iter().filter_map(|x| x.0.and_then(|y| if y.0 .1 == 10 { y.1 } else { None })).max();
            let l = distinct_some(v.iter().map(|x| x.1.flatten()));
            let k = v.iter().filter(|x| x.0.map_or(false, |y| y.0 .1 == 14 || y.0 .1 == 15)).count() as i64;
            (d, c, l, k)
        });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let out = drain((&tu).select(Ident::<User>::new().and(&ups).and(&dp).and(&nti).and((&db.user.origid).select(&pidx).select(&phd).opt())));
    rows(out.into_iter().map(|(_, ((((u, a), n), b), p))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(n), V::I(a[2]), V::I(a[3])]);
        f.extend(ucols(db, u, &["rep"]));
        let (d, c, l, k) = p.unwrap_or((None, None, 0, 0));
        f.extend([V::T(d.unwrap_or(0)), V::S(c.unwrap_or("No Close Reason")), V::I(l), V::I(k), V::Owned(format!("Q{b}"))]);
        f.push(V::S(if a[2] != 0 && a[3] as f64 / a[2] as f64 > 2.0 { "High Answer Rate" } else { "Normal Answer Rate" }));
        row(f)
    }))
}

// WITH PostAnalytics AS (SELECT P.Id AS PostId, P.Title, P.PostTypeId, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT PH.UserId) AS UniqueEditors, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        SUM(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseReopenCount, P.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, U.DisplayName, P.Title, P.CreationDate, P.PostTypeId),
// PostRecommendations AS (SELECT P.Id as PostId, P.Title, COALESCE(T.TagName, 'Unlabeled') AS TagName, RANK() OVER (PARTITION BY P.Title ORDER BY P.ViewCount DESC) AS TagRank
//     FROM Posts P LEFT JOIN Tags T ON POSITION(',' || T.TagName || ',' IN ',' || P.Tags || ',') > 0 WHERE P.ViewCount IS NOT NULL),
// UniqueEditorStats AS (SELECT PostId, UniqueEditors, CASE WHEN UniqueEditors > 10 THEN 'Highly Collaborative' WHEN UniqueEditors BETWEEN 5 AND 10 THEN 'Moderately Collaborative'
//        ELSE 'Low Collaboration' END AS CollaborationLevel FROM PostAnalytics)
// SELECT PA.PostId, PA.Title, PA.OwnerDisplayName, PA.CommentCount, PA.UpVoteCount, PA.DownVoteCount, PA.CloseReopenCount, PA.CreationDate, PA.PostRank,
//        COALESCE(PR.TagName, 'No Tags') AS RecommendedTag, UES.UniqueEditors, UES.CollaborationLevel
// FROM PostAnalytics PA LEFT JOIN PostRecommendations PR ON PA.PostId = PR.PostId JOIN UniqueEditorStats UES ON PA.PostId = UES.PostId
// WHERE PA.CommentCount > 5 AND PA.UpVoteCount > PA.DownVoteCount AND (PA.PostRank <= 5 OR UES.CollaborationLevel = 'Highly Collaborative')
// ORDER BY PA.CreationDate DESC, PA.UpVoteCount DESC LIMIT 100;
//
// PostRecommendations is only read for the posts that pass the WHERE, so its tag scan is run for those alone. TagRank is never read.
fn q21109(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, tags_str, .. } = &db.post;
    let PostHistory { user_id, post_history_type_id, .. } = &db.post_history;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rk = per_group(ranked(drain(recent().select(post_type_id)), |&(p, t)| (t, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, t)| t);
    let rk = rel(rk.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rki: HashIdx<Id<Post>, i64> = (&rk).map(|(p, _)| p).inv().select((&rk).map(|(_, r)| r)).collect();
    let pa = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(post_history_type_id.and(user_id.opt())).opt()))
        .buf_fold(|v| {
            let c = v.iter().filter(|x| x.0 .0.is_some()).count() as i64;
            let up = v.iter().filter(|x| x.0 .1 == Some(2)).count() as i64;
            let dn = v.iter().filter(|x| x.0 .1 == Some(3)).count() as i64;
            let ue = distinct_some(v.iter().map(|x| x.1.and_then(|h| h.1)));
            let cr = v.iter().filter(|x| x.1.map_or(false, |h| h.0 == 10 || h.0 == 11)).count() as i64;
            [c, up, dn, ue, cr]
        });
    let surv: MatSet<Id<Post>> = db.post.with((&pa).and(&rki).filt(|(a, r): ([i64; 5], i64)| a[0] > 5 && a[1] > a[2] && (r <= 5 || a[3] > 10))).collect();
    let pr = Ident::<Post>::new().with(view_count).and(tags_str.select_where((&db.tag.tag_name).inv(), |s: Str, n: Str| format!(",{s},").contains(&format!(",{n},"))).opt());
    let v = drain((&surv).select(Ident::<Post>::new().and(&pa).and(&rki).and(pr.opt())));
    let v = top_n(v, |&(_, (((p, a), _), _))| (Reverse(creation_date.get(p).unwrap()), Reverse(a[1])), 100);
    rows(v.into_iter().map(|(_, (((p, a), r), t))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[4])]);
        f.extend(post_fields(db, p, &["created"]));
        f.push(V::I(r));
        f.push(V::S(match t {
            None => "No Tags",
            Some((_, None)) => "Unlabeled",
            Some((_, Some(t))) => db.tag.tag_name.get(t).unwrap(),
        }));
        f.push(V::I(a[3]));
        f.push(V::S(if a[3] > 10 { "Highly Collaborative" } else if a[3] >= 5 { "Moderately Collaborative" } else { "Low Collaboration" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalVotes, UpVotes, DownVotes, QuestionCount, AnswerCount, BadgeCount, RANK() OVER (ORDER BY TotalVotes DESC, UpVotes DESC) AS VoteRank
//     FROM UserVoteStats),
// RecentPostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        CASE WHEN p.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus FROM Posts p WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days'),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS EngagedPosts, SUM(COALESCE(v.VoteTypeId, 0)) AS VotingActivity, AVG(p.Score) AS AvgPostScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// FinalResult AS (SELECT ru.DisplayName, ru.TotalVotes, ru.UpVotes, ru.DownVotes, ru.QuestionCount, ru.AnswerCount, ru.BadgeCount, rps.Title AS RecentPostTitle, rps.PostStatus,
//        ue.EngagedPosts, ue.VotingActivity, ue.AvgPostScore
//     FROM RankedUsers ru LEFT JOIN RecentPostStats rps ON ru.UserId = rps.PostId LEFT JOIN UserEngagement ue ON ru.UserId = ue.UserId)
// SELECT *, CASE WHEN BadgeCount > 10 AND UpVotes > DownVotes THEN 'Prominent User' ELSE 'Regular User' END AS UserCategory
// FROM FinalResult WHERE EngagedPosts > 5 ORDER BY TotalVotes DESC, UpVotes DESC;
//
// Only users with more than five posts reach the output, so both products are driven for those alone. `ru.UserId = rps.PostId` joins a user id to a
// post id, so it goes through the raw ids.
fn q24164(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, closed_date, .. } = &db.post;
    let Vote { vote_type_id, post, .. } = &db.vote;
    let dp = user_distinct_posts(db);
    let eu: MatSet<Id<User>> = db.user.with((&dp).filt(|n| n > 5)).collect();
    let uvs = (&eu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id.and(post.select(Ident::<Post>::new().and(post_type_id)).opt())).opt().and(badges_of(db).opt()))
        .buf_fold(|v| {
            let n = v.iter().filter(|x| x.0.is_some()).count() as i64;
            let up = v.iter().filter(|x| x.0.map(|y| y.0) == Some(2)).count() as i64;
            let dn = v.iter().filter(|x| x.0.map(|y| y.0) == Some(3)).count() as i64;
            let q = distinct_some(v.iter().map(|x| x.0.and_then(|y| y.1).and_then(|(p, t)| if t == 1 { Some(p) } else { None })));
            let a = distinct_some(v.iter().map(|x| x.0.and_then(|y| y.1).and_then(|(p, t)| if t == 2 { Some(p) } else { None })));
            let b = distinct_some(v.iter().map(|x| x.1));
            [n, up, dn, q, a, b]
        });
    let ue = (&eu).group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(votes_of(db).select(vote_type_id).opt()))).fold([0i64; 3], |a, (s, t)| [a[0] + t.unwrap_or(0), a[1] + s, a[2] + 1]);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rps = (&db.user.origid).select(&pidx).select(Ident::<Post>::new().with(creation_date.gt(add_days(date(2024, 10, 1), -30))));
    rows(drain((&uvs).and(&dp).and(&ue).and(rps.opt())).into_iter().map(|(u, (((a, n), e), p))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend(a.iter().map(|&x| V::I(x)));
        match p {
            Some(p) => f.extend([title(db, p), V::S(if closed_date.get(p).is_some() { "Closed" } else { "Open" })]),
            None => f.extend([V::Null, V::Null]),
        }
        f.extend([V::I(n), V::I(e[0]), avg(e[1], e[2])]);
        f.push(V::S(if a[5] > 10 && a[1] > a[2] { "Prominent User" } else { "Regular User" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(v.VoteScore) AS AvgVoteScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS VoteScore FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistoryDetails AS (SELECT ph.PostId, ph.UserId AS EditorId, ph.UserDisplayName AS EditorName, ph.CreationDate, ph.Comment,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS EditRank FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6, 12)),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(p.ClosedDate, '1970-01-01') AS ClosedDateFallback, p.AcceptedAnswerId, phs.EditorName, phs.EditRank
//     FROM Posts p LEFT JOIN PostHistoryDetails phs ON p.Id = phs.PostId AND phs.EditRank = 1 WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostLinkCounts AS (SELECT pl.RelatedPostId, COUNT(*) AS LinkCount FROM PostLinks pl GROUP BY pl.RelatedPostId)
// SELECT us.UserId, us.DisplayName, us.Reputation, COALESCE(rp.PostId, -1) AS PostId, COALESCE(rp.Title, 'No Recent Posts') AS Title, rp.CreationDate, rp.Score,
//        rp.ClosedDateFallback, plc.LinkCount,
//        CASE WHEN rp.Title IS NOT NULL THEN 'Active' WHEN rp.ClosedDateFallback > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 'Recently Closed' ELSE 'Inactive' END AS PostStatus
// FROM UserStats us LEFT JOIN RecentPosts rp ON us.UserId = rp.AcceptedAnswerId LEFT JOIN PostLinkCounts plc ON rp.PostId = plc.RelatedPostId
// WHERE us.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY us.Reputation DESC, rp.CreationDate DESC NULLS LAST LIMIT 100;
//
// None of UserStats' aggregates is projected, and the edit history row joined into RecentPosts is at most one per post and never read.
// `us.UserId = rp.AcceptedAnswerId` joins a user id to a post id, so it goes through the raw ids.
fn q23586(db: &'static So) -> String {
    let Post { creation_date, accepted_answer_id, closed_date, title: pt, .. } = &db.post;
    let rep = &db.user.reputation;
    let (s, n) = db.user.select(rep).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let a = s as f64 / n as f64;
    let lo = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let byacc: HashIdx<i64, Id<Post>> = db.post.with(creation_date.gt(lo)).select(accepted_answer_id).inv().collect();
    let plc = db.post_link.group_by(&db.post_link.related_post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with(rep.filt(|r| r as f64 > a)).select((&db.user.origid).select(&byacc).select(Ident::<Post>::new().and((&plc).opt())).opt()));
    let v = top_n(v, |&(u, p)| (Reverse(rep.get(u).unwrap()), p.is_none(), Reverse(p.map(|p| creation_date.get(p.0).unwrap())), u), 100);
    rows(v.into_iter().map(|(u, p)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        match p {
            Some((p, l)) => {
                let t = pt.get(p);
                let c = closed_date.get(p).unwrap_or(0);
                f.extend(post_fields(db, p, &["id"]));
                f.push(V::S(t.unwrap_or("No Recent Posts")));
                f.extend(post_fields(db, p, &["created", "score"]));
                f.extend([V::T(c), oint(l)]);
                f.push(V::S(if t.is_some() { "Active" } else if c > lo { "Recently Closed" } else { "Inactive" }));
            }
            None => f.extend([V::I(-1), V::S("No Recent Posts"), V::Null, V::Null, V::Null, V::Null, V::S("Inactive")]),
        }
        row(f)
    }))
}

// rewrites/30662.sql (the original ORDER BY with the group key appended):
// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 DAYS'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT p.Id) AS TotalPosts,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.TotalBounty, ua.TotalPosts, ua.UpVotes, ua.DownVotes, ROW_NUMBER() OVER (ORDER BY ua.Reputation DESC) AS UserRank
//     FROM UserActivity ua WHERE ua.TotalPosts > 5),
// PostHistoryDetails AS (SELECT ph.PostId, ph.UserId, p.Title, p.Body, ph.CreationDate AS EditDate, p.OwnerDisplayName, ph.Comment,
//        CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 'Closed/Reopened' ELSE 'Edited' END AS EditType
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '60 DAYS')
// SELECT rp.PostId, rp.Title, u.DisplayName AS Author, u.Reputation, u.TotalBounty, u.TotalPosts, rp.ViewCount, rp.CreationDate, rp.Score, phd.EditDate, phd.EditType, phd.Comment,
//        COALESCE(SUM(CASE WHEN phd.EditDate IS NOT NULL THEN 1 ELSE 0 END), 0) AS EditCount
// FROM RecentPosts rp JOIN TopUsers u ON rp.OwnerUserId = u.UserId LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId
// GROUP BY rp.PostId, rp.Title, u.UserId, u.DisplayName, u.Reputation, u.TotalBounty, u.TotalPosts, rp.ViewCount, rp.CreationDate, rp.Score, phd.EditDate, phd.EditType, phd.Comment
// ORDER BY rp.ViewCount DESC, rp.CreationDate DESC, rp.PostId, phd.EditDate, phd.EditType, phd.Comment LIMIT 100;
//
// UserActivity is only needed for the owners of recent posts. A recent post with no history in the window keeps one row with EditCount 0.
fn q30662(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, .. } = &db.post;
    let Vote { bounty_amount, .. } = &db.vote;
    let PostHistory { creation_date: hd, post_history_type_id, comment, post, .. } = &db.post_history;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let ua = (&owners)
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(votes_of(db).select(bounty_amount.opt()).opt())).opt())
        .buf_fold(|v| (v.iter().map(|x| x.and_then(|y| y.1.flatten()).unwrap_or(0)).sum::<i64>(), distinct_some(v.iter().map(|x| x.map(|y| y.0)))));
    let cand: MatSet<Id<Post>> = recent().with(owner_user.select((&ua).filt(|a| a.1 > 5))).collect();
    let win = || db.post_history.with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -60)));
    let key = post.and(hd).and(post_history_type_id.map(|t| if t == 10 || t == 11 { "Closed/Reopened" } else { "Edited" })).and(comment.opt());
    let g = win().with(post.with(&cand)).group_by(key).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let mut v: Vec<(Id<Post>, Option<(i64, Str, Option<Str>)>, i64)> = drain(&g).into_iter().map(|((((p, d), t), c), n)| (p, Some((d, t, c)), n)).collect();
    v.extend(drain((&cand).minus(history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -60)))))).into_iter().map(|(p, _)| (p, None, 0)));
    let v = top_n(
        v,
        |&(p, e, _)| {
            let vc = view_count.get(p);
            (vc.is_none(), Reverse(vc), Reverse(creation_date.get(p).unwrap()), db.post.origid.get(p).unwrap(), e.is_none(), e.map(|x| (x.0, x.1, x.2.is_none(), x.2)))
        },
        100,
    );
    type R = (Id<Post>, Option<(i64, Str, Option<Str>)>, i64);
    let r = rel(v);
    let v = drain((&r).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(owner_user.select(Ident::<User>::new().and(&ua))))));
    rows(v.into_iter().map(|(_, ((p, e, n), (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a.0), V::I(a.1)]);
        f.extend(post_fields(db, p, &["views", "created"]));
        f.push(V::I(score.get(p).unwrap()));
        match e {
            Some((d, t, c)) => f.extend([V::T(d), V::S(t), ostr(c)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostID, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS RankScore,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpVoteCount,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// UserStatistics AS (SELECT U.Id AS UserID, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        AVG(COALESCE(P.ViewCount, 0)) AS AverageViewCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// FilteredPosts AS (SELECT RP.*, US.PostCount AS UserPostCount, US.BadgeCount,
//        CASE WHEN US.Reputation > 1000 THEN 'High Reputation User' WHEN US.Reputation BETWEEN 100 AND 1000 THEN 'Medium Reputation User' ELSE 'Low Reputation User' END AS UserReputationCategory
//     FROM RankedPosts RP LEFT JOIN UserStatistics US ON RP.OwnerUserId = US.UserID WHERE RP.RankScore <= 5),
// InactivityWarning AS (SELECT OwnerUserId, MAX(LastActivityDate) AS LastActive, COUNT(*) AS InactivePosts FROM Posts
//     WHERE LastActivityDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '90 days' GROUP BY OwnerUserId)
// SELECT FP.Title, FP.CreationDate, FP.Score, FP.UpVoteCount, FP.DownVoteCount, FP.CommentCount, FP.UserReputationCategory, US.DisplayName,
//        COALESCE(IW.InactivePosts, 0) AS InactivePostCount, CASE WHEN IW.LastActive IS NOT NULL THEN 'Inactive User' ELSE 'Active User' END AS UserActivityStatus
// FROM FilteredPosts FP LEFT JOIN InactivityWarning IW ON FP.OwnerUserId = IW.OwnerUserId JOIN UserStatistics US ON FP.OwnerUserId = US.UserID
// WHERE FP.ViewCount > 50 ORDER BY FP.Score DESC, FP.CreationDate DESC;
//
// Only the name and reputation of UserStatistics are read, and the inner join to it needs an owner.
fn q21299(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, view_count, last_activity_date, .. } = &db.post;
    let vote_type_id = &db.vote.vote_type_id;
    let top = top_per(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 5, true);
    let tp = post_set(&top, |x| x.0);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let iw = db.post.with(last_activity_date.lt(add_days(ts(2024, 10, 1, 12, 34, 56), -90))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).with(view_count.gt(50)).select(Ident::<Post>::new().and(&cc).and(&vc).and(owner_user.select(Ident::<User>::new().and((&iw).opt())))));
    rows(v.into_iter().map(|(_, (((p, c), a), (u, w)))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.push(V::S(if r > 1000 { "High Reputation User" } else if r >= 100 { "Medium Reputation User" } else { "Low Reputation User" }));
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(w.unwrap_or(0)), V::S(if w.is_some() { "Inactive User" } else { "Active User" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, COALESCE(p.Body, 'No content') AS PostBody, p.CreationDate, p.OwnerUserId, p.AcceptedAnswerId, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.AcceptedAnswerId),
// FilteredPosts AS (SELECT rp.*, CASE WHEN rp.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Pending' END AS AnswerStatus, (UpVoteCount - DownVoteCount) AS NetVote,
//        CASE WHEN rp.UserPostRank <= 5 THEN 'Top Posts' ELSE 'Other Posts' END AS PostCategory
//     FROM RankedPosts rp WHERE rp.PostBody IS NOT NULL AND rp.PostBody <> 'No content'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(DISTINCT p.Id) > 0),
// FinalReport AS (SELECT f.PostId, f.Title, f.PostBody, f.CreationDate, ua.UserId, ua.DisplayName, ua.Reputation, ua.TotalPosts, f.AnswerStatus, f.NetVote,
//        DENSE_RANK() OVER (ORDER BY ua.Reputation DESC) AS UserRank, f.PostCategory
//     FROM FilteredPosts f JOIN UserActivity ua ON f.OwnerUserId = ua.UserId WHERE f.NetVote > 0 AND f.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month')
// SELECT PostId, Title, PostBody, CreationDate, DisplayName, Reputation, TotalPosts, AnswerStatus, NetVote, UserRank, PostCategory FROM FinalReport ORDER BY UserRank, CreationDate DESC;
//
// Only posts from the last month reach the output, so the comment x vote product is driven for those. UserPostRank numbers every post of the owner.
fn q20027(db: &'static So) -> String {
    let Post { creation_date, owner_user, body, accepted_answer_id, .. } = &db.post;
    let vote_type_id = &db.vote.vote_type_id;
    let pr = per_group(ranked(drain(db.post.select(owner_user)), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let pr = rel(pr.into_iter().map(|((p, _), r)| (p, r)).collect());
    let pri: HashIdx<Id<Post>, i64> = (&pr).map(|(p, _)| p).inv().select((&pr).map(|(_, r)| r)).collect();
    let dp = user_distinct_posts(db);
    let recent = db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).with(body.ne("No content")).with(owner_user.select((&dp).filt(|n| n > 0)));
    let net = recent.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt())).fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let v = drain((&net).filt(|n| n > 0).and(owner_user.select(Ident::<User>::new().and(&dp))).and(&pri));
    let v = ranked(v, |&(_, ((_, (u, _)), _))| Reverse(db.user.reputation.get(u).unwrap()), true);
    rows(v.into_iter().map(|((p, ((n, (u, t)), r)), k)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(t));
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Accepted" } else { "Pending" }));
        f.extend([V::I(n), V::I(k), V::S(if r <= 5 { "Top Posts" } else { "Other Posts" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     AND p.Score > (SELECT AVG(Score) FROM Posts WHERE CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// RecentVotes AS (SELECT v.PostId, SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE v.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// FinalResults AS (SELECT p.PostId, p.Title, p.CreationDate, COALESCE(rv.UpVotesCount, 0) AS UpVotesCount, COALESCE(rv.DownVotesCount, 0) AS DownVotesCount,
//        COALESCE(b.BadgeCount, 0) AS BadgeCount, COALESCE(b.HighestBadgeClass, 0) AS HighestBadgeClass, cp.LastClosedDate, RANK() OVER (ORDER BY p.Score DESC) AS PostScoreRank
//     FROM RankedPosts p LEFT JOIN RecentVotes rv ON p.PostId = rv.PostId LEFT JOIN UserBadges b ON p.OwnerUserId = b.UserId LEFT JOIN ClosedPosts cp ON p.PostId = cp.PostId
//     WHERE (cp.LastClosedDate IS NULL OR cp.LastClosedDate < p.CreationDate) AND p.PostRank <= 5)
// SELECT fr.*, CASE WHEN fr.UpVotesCount > fr.DownVotesCount THEN 'Positive Engagement' WHEN fr.UpVotesCount < fr.DownVotesCount THEN 'Negative Engagement' ELSE 'Neutral Engagement' END AS EngagementLevel,
//        (SELECT COUNT(DISTINCT pl.RelatedPostId) FROM PostLinks pl WHERE pl.PostId = fr.PostId) AS RelatedPostsCount
// FROM FinalResults fr WHERE fr.BadgeCount > 0 ORDER BY fr.PostScoreRank, fr.CreationDate DESC;
//
// PostRank partitions by the raw OwnerUserId, ownerless posts together. A tie on CreationDate inside it goes to the smaller id.
fn q22517(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, owner_user_id, .. } = &db.post;
    let Vote { vote_type, creation_date: vd, post: vp, .. } = &db.vote;
    let PostHistory { creation_date: hd, post_history_type_id, post: hp, .. } = &db.post_history;
    let lo = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let (s, n) = db.post.with(creation_date.gt(lo)).select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let a = s as f64 / n as f64;
    let rp = drain(db.post.with(creation_date.gt(lo)).with(score.filt(|x| x as f64 > a)).select(owner_user_id.opt()));
    let rp = top_per(rp, |&(_, o)| o, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp = post_set(&rp, |x| x.0);
    let rv = db.vote.with(vd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(vp).select(vote_type.select(&db.vote_type.origid)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| (n + c.is_some() as i64, m.max(c.unwrap_or(0))));
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(hp).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let rc = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).buf_fold(|v| distinct_some(v.iter().map(|&x| Some(x))));
    let fr = drain((&tp).select(Ident::<Post>::new().and((&cp).opt()).filt(|(p, c): (Id<Post>, Option<i64>)| c.map_or(true, |c| c < creation_date.get(p).unwrap())).and((&rv).opt()).and(owner_user.select(&ub).opt()).and((&rc).opt())));
    let fr = ranked(fr, |&(_, ((((p, _), _), _), _))| Reverse(score.get(p).unwrap()), false);
    type R = ((Id<Post>, ((((Id<Post>, Option<i64>), Option<[i64; 2]>), Option<(i64, i64)>), Option<i64>)), i64);
    let v = drain(rel(fr).filt(|x: R| x.0 .1 .0 .1.map_or(0, |b| b.0) > 0));
    rows(v.into_iter().map(|(_, ((_, ((((p, c), a), b), l)), k))| {
        let a = a.unwrap_or([0; 2]);
        let b = b.unwrap_or((0, 0));
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b.0), V::I(b.1), ots(c), V::I(k)]);
        f.push(V::S(if a[0] > a[1] { "Positive Engagement" } else if a[0] < a[1] { "Negative Engagement" } else { "Neutral Engagement" }));
        f.push(V::I(l.unwrap_or(0)));
        row(f)
    }))
}

// rewrites/22640.sql (the original ORDER BY with pp.PostId, pld.LinkType appended):
// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadgeCount,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadgeCount, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadgeCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '30 days') GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// PopularPosts AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, ps.OwnerUserId, ps.CreationDate, ps.UserPostRank FROM PostStats ps
//     WHERE ps.CommentCount > 5 OR ps.UpVoteCount > 10 OR (ps.DownVoteCount = 0 AND ps.CommentCount >= 3)),
// LatestPost AS (SELECT p.Id, p.Title, p.CreationDate FROM Posts p WHERE p.CreationDate = (SELECT MAX(CreationDate) FROM Posts)),
// PostLinksDetails AS (SELECT pl.PostId, pl.RelatedPostId, lt.Name AS LinkType FROM PostLinks pl JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id)
// SELECT pp.PostId, pp.Title, pp.CommentCount, pp.UpVoteCount, pp.DownVoteCount, u.DisplayName AS UserDisplayName,
//        CASE WHEN u.Reputation IS NULL THEN 'No Reputation Data' ELSE CAST(u.Reputation AS VARCHAR) || ' Reputation Points' END AS UserReputation,
//        COALESCE(lp.Title, 'No Recent Post') AS LatestPostTitle, COALESCE(pld.LinkType, 'No Links') AS LinkType,
//        EXTRACT(EPOCH FROM (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - pp.CreationDate)) / 60 AS AgeInMinutes
// FROM PopularPosts pp LEFT JOIN Users u ON pp.OwnerUserId = u.Id LEFT JOIN LatestPost lp ON pp.PostId = lp.Id LEFT JOIN PostLinksDetails pld ON pp.PostId = pld.PostId
// ORDER BY pp.UpVoteCount DESC, pp.CommentCount DESC, pp.PostId, pld.LinkType OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// Only the three distinct counts of PostStats are read, and the badge join cannot change them, so the comment and vote counts are separate folds.
fn q22640(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let mx = db.post.select(creation_date).fold_flat(i64::MIN, |m, d| m.max(d));
    let latest = Ident::<Post>::new().with(creation_date.eq(mx));
    let pld = links_of(db).select((&db.post_link.link_type).select(&db.link_type.name));
    let pp = (&cc).and(&vc).filt(|(c, a): (i64, [i64; 2])| c > 5 || a[0] > 10 || (a[1] == 0 && c >= 3));
    let v = drain(pp.and(owner_user.opt()).and(latest.opt()).and(pld.opt()));
    let v = top_n(v, |&(p, ((((c, a), _), _), l))| (Reverse(a[0]), Reverse(c), db.post.origid.get(p).unwrap(), l.is_none(), l), 10);
    rows(v.into_iter().map(|(p, ((((c, a), u), lp), l))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        match u {
            Some(u) => f.extend([user_col(db, u, "name"), V::Owned(format!("{} Reputation Points", db.user.reputation.get(u).unwrap()))]),
            None => f.extend([V::Null, V::S("No Reputation Data")]),
        }
        f.push(V::S(lp.and_then(|q| db.post.title.get(q)).unwrap_or("No Recent Post")));
        f.push(V::S(l.unwrap_or("No Links")));
        f.push(V::F(secs(ts(2024, 10, 1, 12, 34, 56) - creation_date.get(p).unwrap()) / 60.0));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.PostTypeId,
//        COALESCE(SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes vt ON p.Id = vt.PostId AND vt.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND (p.Body IS NOT NULL AND p.Body != '')
//     GROUP BY p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName, p.PostTypeId),
// RankedPosts AS (SELECT PostId, Title, Score, CreationDate, OwnerDisplayName, UpVotes, DownVotes, CommentCount, RANK() OVER (PARTITION BY PostTypeId ORDER BY Score DESC, CreationDate DESC) AS ScoreRank
//     FROM RecentPosts),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, OwnerDisplayName, UpVotes, DownVotes, CommentCount FROM RankedPosts WHERE ScoreRank <= 5),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, AVG(p.Score) AS AveragePostScore,
//        COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 1) AS QuestionCount, COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 2) AS AnswerCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.OwnerDisplayName AS TopPostOwner, tp.Title AS TopPostTitle, tp.Score AS TopPostScore, tp.CommentCount AS TopPostComments, us.DisplayName AS UserDisplayName,
//        us.GoldBadges, us.AveragePostScore, us.QuestionCount, us.AnswerCount
// FROM TopPosts tp JOIN RecentPosts rp ON tp.PostId = rp.PostId JOIN UserStats us ON rp.OwnerDisplayName = us.DisplayName
// WHERE EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = tp.PostId AND v.VoteTypeId = 2 GROUP BY v.PostId HAVING COUNT(v.Id) >= 10)
// ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// ScoreRank reads only base columns, so the top posts are picked first. UserStats is only needed for the users whose name matches a top post's owner.
fn q21062(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, body, .. } = &db.post;
    let Vote { vote_type_id, creation_date: vd, .. } = &db.vote;
    let top = top_per(
        drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(body.ne("")).select(post_type_id)),
        |&(_, t)| t,
        |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())),
        5,
        true,
    );
    let tp = post_set(&top, |x| x.0);
    let up10 = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2)));
    let n10 = (&tp).group_by(Ident::<Post>::new()).select(up10).fold(0i64, |n, _| n + 1);
    let tp10: MatSet<Id<Post>> = (&tp).with((&n10).filt(|n| n >= 10)).collect();
    let cc = (&tp10).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(vd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).opt().and(comments_of(db).opt())).fold(0i64, |n, (_, c)| n + c.is_some() as i64);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let names = owner_user.select(&db.user.display_name);
    let us_users: MatSet<Id<User>> = (&tp10).select(&names).select(&by_name).collect();
    let us = (&us_users)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(Ident::<Post>::new().and(score).and(post_type_id)).opt()))
        .buf_fold(|v| {
            let g = v.iter().filter(|x| x.0 == Some(1)).count() as i64;
            let s: i64 = v.iter().filter_map(|x| x.1.map(|y| y.0 .1)).sum();
            let n = v.iter().filter(|x| x.1.is_some()).count() as i64;
            let q = distinct_some(v.iter().map(|x| x.1.and_then(|y| if y.1 == 1 { Some(y.0 .0) } else { None })));
            let a = distinct_some(v.iter().map(|x| x.1.and_then(|y| if y.1 == 2 { Some(y.0 .0) } else { None })));
            [g, s, n, q, a]
        });
    rows(drain((&cc).and(names.select(&by_name).select(Ident::<User>::new().and(&us)))).into_iter().map(|(p, (c, (u, a)))| {
        let mut f = post_fields(db, p, &["owner", "title", "score"]);
        f.push(V::I(c));
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(a[0]), avg(a[1], a[2]), V::I(a[3]), V::I(a[4])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, CASE WHEN U.Reputation IS NULL THEN 'No reputation' WHEN U.Reputation < 1000 THEN 'Novice'
//        WHEN U.Reputation < 5000 THEN 'Experienced' ELSE 'Expert' END AS ReputationLevel FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostMetaData AS (SELECT RP.PostId, UR.ReputationLevel, COALESCE(PH2.Comment, 'No Close Reason') AS CloseReason, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = RP.PostId) AS CommentCount,
//        MAX(V.CreationDate) AS LastVoteDate, SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount, COUNT(DISTINCT PL.RelatedPostId) AS RelatedLinksCount
//     FROM RecentPosts RP LEFT JOIN UserReputation UR ON RP.OwnerUserId = UR.UserId LEFT JOIN PostHistory PH2 ON RP.PostId = PH2.PostId AND PH2.PostHistoryTypeId IN (10, 11)
//     LEFT JOIN Votes V ON RP.PostId = V.PostId LEFT JOIN PostLinks PL ON RP.PostId = PL.PostId WHERE RP.PostRank = 1 GROUP BY RP.PostId, UR.ReputationLevel, PH2.Comment),
// PostAnalytics AS (SELECT PMD.PostId, PMD.ReputationLevel, PMD.CloseReason, PMD.CommentCount, PMD.LastVoteDate, PMD.VoteCount, PMD.RelatedLinksCount,
//        CASE WHEN PMD.CloseReason IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus,
//        CASE WHEN PMD.CommentCount = 0 THEN 'No comments' WHEN PMD.CommentCount BETWEEN 1 AND 5 THEN 'Few comments' ELSE 'Many comments' END AS CommentStatus,
//        CASE WHEN PMD.VoteCount < 0 THEN 'Nega-voted' WHEN PMD.VoteCount = 0 THEN 'Neutral' WHEN PMD.VoteCount > 0 THEN 'Posi-voted' END AS VoteStatus FROM PostMetaData PMD)
// SELECT PA.PostId, PA.ReputationLevel, PA.CloseReason, PA.CommentCount, PA.LastVoteDate, PA.VoteCount, PA.RelatedLinksCount, PA.PostStatus, PA.CommentStatus, PA.VoteStatus
// FROM PostAnalytics PA WHERE PA.ReputationLevel <> 'No reputation' AND PA.PostStatus = 'Active' AND PA.CommentCount > 2 ORDER BY PA.VoteCount DESC, PA.CommentCount DESC;
//
// PostRank partitions by the raw OwnerUserId, ownerless posts together; a tie on CreationDate goes to the smaller id. The groups are one per
// (post, close comment): the close rows with a comment value, and one row for the posts with no close row. CloseReason is a COALESCE, so it is never
// NULL and PostStatus is never 'Active'; the filter below says so as the SQL does.
fn q23059(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, .. } = &db.post;
    let Vote { creation_date: vd, vote_type_id, .. } = &db.vote;
    let PostHistory { post_history_type_id, post, comment, .. } = &db.post_history;
    let r1 = top_per(drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user_id.opt())), |&(_, o)| o, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let r1 = post_set(&r1, |x| x.0);
    let tail = || votes_of(db).select(vd.and(vote_type_id)).opt().and(links_of(db).select(&db.post_link.related_post_id).opt());
    let agg = |v: SVecAlias| {
        let d = v.iter().filter_map(|x| x.0.map(|y| y.0)).max();
        let n = v.iter().filter(|x| x.0.map_or(false, |y| y.1 == 2 || y.1 == 3)).count() as i64;
        let l = distinct_some(v.iter().map(|x| x.1));
        (d, n, l)
    };
    let close = db.post_history.with(post_history_type_id.is_in([10, 11])).with(post.with(&r1));
    let g1 = close.group_by(post.and(comment.opt())).select(post.select(tail())).buf_fold(|v| agg(v.into_iter().collect()));
    let bare = (&r1).minus(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))));
    let g0 = bare.group_by(Ident::<Post>::new().map(|p: Id<Post>| (p, None::<Str>))).select(tail()).buf_fold(|v| agg(v.into_iter().collect()));
    let mut all = drain(&g1);
    all.extend(drain(&g0));
    type G = ((Id<Post>, Option<Str>), (Option<i64>, i64, i64));
    let cc = (&r1).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let lvl = owner_user.select(&db.user.reputation).map(|r: i64| if r < 1000 { "Novice" } else if r < 5000 { "Experienced" } else { "Expert" });
    let r = rel(all);
    let key = Same::<G>::new().map(|x: G| x.0 .0);
    let v = drain((&r).select(Same::<G>::new().and(key.select(&cc)).and(Same::<G>::new().map(|x: G| x.0 .0).select(&lvl))).filt(|((g, c), l): ((G, i64), &'static str)| {
        let status = if Some(g.0 .1.unwrap_or("No Close Reason")).is_some() { "Closed" } else { "Active" };
        l != "No reputation" && status == "Active" && c > 2
    }));
    rows(v.into_iter().map(|(_, (((k, (d, n, l)), c), lv))| {
        row(vec![
            V::I(db.post.origid.get(k.0).unwrap()),
            V::S(lv),
            V::S(k.1.unwrap_or("No Close Reason")),
            V::I(c),
            ots(d),
            V::I(n),
            V::I(l),
            V::S("Active"),
            V::S(if c == 0 { "No comments" } else if c <= 5 { "Few comments" } else { "Many comments" }),
            V::S(if n < 0 { "Nega-voted" } else if n == 0 { "Neutral" } else { "Posi-voted" }),
        ])
    }))
}

type SVecAlias = Vec<(Option<(i64, i64)>, Option<i64>)>;

// WITH RECURSIVE UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(CASE WHEN p.Score IS NULL THEN 1 ELSE 0 END) AS NullScorePosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, PositivePosts, NegativePosts, NullScorePosts, RANK() OVER (ORDER BY TotalPosts DESC) AS Ranking FROM UserPostStats WHERE TotalPosts > 0),
// RecentPostVotes AS (SELECT p.Id AS PostId, p.OwnerUserId, v.VoteTypeId, COUNT(v.Id) AS VoteCount FROM Posts p JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 month' GROUP BY p.Id, p.OwnerUserId, v.VoteTypeId),
// PostEngagements AS (SELECT p.Id AS PostId, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount, SUM(COALESCE(rv.VoteCount, 0)) AS RecentVoteCount,
//        p.OwnerUserId, p.Title, p.CreationDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId LEFT JOIN RecentPostVotes rv ON p.Id = rv.PostId
//     GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate),
// FinalReport AS (SELECT u.DisplayName AS UserName, COUNT(DISTINCT pe.PostId) AS TotalEngagedPosts, SUM(pe.CommentCount) AS TotalComments, SUM(pe.RelatedPostsCount) AS TotalRelatedPosts,
//        SUM(pe.RecentVoteCount) AS TotalRecentVotes, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY SUM(pe.RecentVoteCount) DESC) AS VoteRanking
//     FROM Users u JOIN PostEngagements pe ON u.Id = pe.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT fu.UserName, fu.TotalEngagedPosts, fu.TotalComments, fu.TotalRelatedPosts, fu.TotalRecentVotes,
//        CASE WHEN fu.TotalRecentVotes > 10 THEN 'Highly Engaged' WHEN fu.TotalRecentVotes BETWEEN 5 AND 10 THEN 'Moderately Engaged' ELSE 'Less Engaged' END AS EngagementLevel
// FROM FinalReport fu WHERE fu.TotalEngagedPosts > 5 AND fu.TotalComments IS NOT NULL ORDER BY fu.TotalRecentVotes DESC, fu.TotalEngagedPosts DESC;
//
// RECURSIVE, but no CTE refers to itself. The TIMESTAMP column is compared with CURRENT_TIMESTAMP in the session zone.
fn q33465(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let Vote { post, vote_type_id, .. } = &db.vote;
    let lo = add_months(utc_to_ny(now_utc()), -1);
    let rv = db.vote.with(post.select(creation_date.ge(lo))).group_by(post.and(vote_type_id)).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let rr = rel(drain(&rv));
    let rvi: HashIdx<Id<Post>, i64> = (&rr).map(|((p, _), _)| p).inv().select((&rr).map(|(_, n)| n)).collect();
    let pe = db
        .post
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(links_of(db).select(&db.post_link.related_post_id).opt()).and((&rvi).opt()))
        .buf_fold(|v| [distinct_some(v.iter().map(|x| x.0 .0)), distinct_some(v.iter().map(|x| x.0 .1)), v.iter().map(|x| x.1.unwrap_or(0)).sum::<i64>()]);
    let fr = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&pe)).fold([0i64; 4], |a, e| [a[0] + 1, a[1] + e[0], a[2] + e[1], a[3] + e[2]]);
    rows(drain((&fr).filt(|a| a[0] > 5)).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.push(V::S(if a[3] > 10 { "Highly Engaged" } else if a[3] >= 5 { "Moderately Engaged" } else { "Less Engaged" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, CASE WHEN Reputation IS NULL THEN 'No Reputation' WHEN Reputation < 100 THEN 'Novice'
//        WHEN Reputation BETWEEN 100 AND 500 THEN 'Intermediate' ELSE 'Expert' END AS ReputationLevel FROM Users),
// PostStatistics AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, p.CreationDate, EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - p.CreationDate)) / 3600 AS AgeInHours
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes vote ON p.Id = vote.PostId GROUP BY p.Id, p.CreationDate),
// PostLinksStatistics AS (SELECT pl.PostId AS PostId, COUNT(pl.RelatedPostId) AS TotalLinks, COUNT(DISTINCT pl.LinkTypeId) AS UniqueLinkTypes FROM PostLinks pl GROUP BY pl.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseReasonCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// TopPostStats AS (SELECT ps.PostId, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, ps.AgeInHours, COALESCE(pls.TotalLinks, 0) AS TotalLinks, COALESCE(cps.CloseReasonCount, 0) AS CloseReasonCount,
//        CASE WHEN ps.DownVoteCount > ps.UpVoteCount THEN 'Negatively Rated' WHEN ps.UpVoteCount > ps.DownVoteCount THEN 'Positively Rated' ELSE 'Neutral' END AS PostRating
//     FROM PostStatistics ps LEFT JOIN PostLinksStatistics pls ON ps.PostId = pls.PostId LEFT JOIN ClosedPosts cps ON ps.PostId = cps.PostId WHERE ps.AgeInHours < 720),
// RankedPosts AS (SELECT tps.PostId, tps.CommentCount, tps.TotalLinks, tps.PostRating, ROW_NUMBER() OVER (ORDER BY tps.UpVoteCount DESC, tps.CommentCount DESC) AS Rank FROM TopPostStats tps)
// SELECT up.UserId, up.ReputationLevel, rp.PostId, rp.CommentCount, rp.TotalLinks, rp.PostRating
// FROM UserReputation up JOIN RankedPosts rp ON rp.PostId IN (SELECT DISTINCT p.Id FROM Posts p WHERE p.OwnerUserId = up.UserId)
// WHERE up.ReputationLevel <> 'No Reputation' AND EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = up.UserId AND p.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')
// ORDER BY up.Reputation DESC, rp.PostRating DESC;
//
// The IN joins each ranked post to its owner. Rank is never read.
fn q20135(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let tps = db
        .post
        .with(creation_date.filt(move |c| secs(t0 - c) / 3600.0 < 720.0))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let pls = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let old = posts_of(db).select(Ident::<Post>::new().with(creation_date.lt(add_years(t0, -1))));
    let v = drain((&tps).and((&pls).opt()).and(owner_user.select(Ident::<User>::new().with(old))));
    rows(v.into_iter().map(|(p, ((a, l), u))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid"]);
        f.push(V::S(if r < 100 { "Novice" } else if r <= 500 { "Intermediate" } else { "Expert" }));
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(a[0]), V::I(l.unwrap_or(0))]);
        f.push(V::S(if a[2] > a[1] { "Negatively Rated" } else if a[1] > a[2] { "Positively Rated" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.UserId, ph.Comment, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 13)),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
//        MAX(p.CreationDate) AS LastPostDate FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// TopTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(p.Id) > 10),
// PostStats AS (SELECT p.Id AS PostId, p.Title, COALESCE(UPVOTES.UpCount, 0) AS UpVoteCount, COALESCE(DOWNVOTES.DownCount, 0) AS DownVoteCount, COALESCE(CLOSEHIST.CloseCount, 0) AS CloseCount,
//        COALESCE(REOPENHIST.ReopenCount, 0) AS ReopenCount, p.CreationDate, DENSE_RANK() OVER (ORDER BY COALESCE(UPVOTES.UpCount, 0) - COALESCE(DOWNVOTES.DownCount, 0) DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS UpCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) AS UPVOTES ON p.Id = UPVOTES.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS DownCount FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) AS DOWNVOTES ON p.Id = DOWNVOTES.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CloseCount FROM RecursivePostHistory WHERE PostHistoryTypeId = 10 GROUP BY PostId) AS CLOSEHIST ON p.Id = CLOSEHIST.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS ReopenCount FROM RecursivePostHistory WHERE PostHistoryTypeId = 11 GROUP BY PostId) AS REOPENHIST ON p.Id = REOPENHIST.PostId
//     WHERE p.CreationDate > (cast('2024-10-01' as date) - INTERVAL '1 year'))
// SELECT ps.PostId, ps.Title, ps.UpVoteCount, ps.DownVoteCount, ps.CloseCount, ps.ReopenCount, pht.Name AS PostHistoryType, u.DisplayName AS LastEditor, ur.Reputation AS EditorReputation,
//        tt.TagName AS PopularTag
// FROM PostStats ps LEFT JOIN RecursivePostHistory rph ON ps.PostId = rph.PostId AND rph.rn = 1 LEFT JOIN PostHistoryTypes pht ON rph.PostHistoryTypeId = pht.Id
// LEFT JOIN Users u ON rph.UserId = u.Id LEFT JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN TopTags tt ON tt.PostCount > 0
// WHERE (ps.UpVoteCount - ps.DownVoteCount) > 5 ORDER BY ps.Rank;
//
// `ON tt.PostCount > 0` names only TopTags, so it is a cross join with the popular tags. The latest history row is only ranked for the posts that pass
// the WHERE; a tie on its CreationDate goes to the smaller id.
fn q32149(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let vc = db
        .post
        .with(creation_date.gt(add_years(date(2024, 10, 1), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let surv: MatSet<Id<Post>> = db.post.with((&vc).filt(|a| a[0] - a[1] > 5)).collect();
    let rph = || history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11, 12, 13])));
    let ch = (&surv).group_by(Ident::<Post>::new()).select(rph().select(post_history_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(11)) as i64]);
    let h1 = top_per(drain((&surv).select(rph())), |&(p, _)| p, |&(_, h)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let hr = rel(h1);
    let hi: HashIdx<Id<Post>, Id<PostHistory>> = (&hr).map(|(p, _)| p).inv().select((&hr).map(|(_, h)| h)).collect();
    let ts_ = tag_stats(db);
    let tt: MatSet<Id<Tag>> = db.tag.with((&ts_).filt(|a| a[0] > 10)).collect();
    let left = (&surv).select(Ident::<Post>::new().and(&vc).and(&ch).and((&hi).select(htype_name(db).and(user.opt())).opt()));
    rows(drain(left.cross(&tt)).into_iter().map(|(_, ((((p, a), c), h), t))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c[0]), V::I(c[1])]);
        match h {
            Some((n, u)) => {
                f.push(V::S(n));
                match u {
                    Some(u) => f.extend(ucols(db, u, &["name", "rep"])),
                    None => f.extend([V::Null, V::Null]),
                }
            }
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.push(V::S(db.tag.tag_name.get(t).unwrap()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.Reputation),
// UserTopPosts AS (SELECT r.PostId, r.Title, r.Score, r.OwnerUserId, ur.TotalPosts, ur.Reputation, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges,
//        RANK() OVER (PARTITION BY ur.UserId ORDER BY r.Score DESC) AS PostRank FROM RankedPosts r JOIN UserReputation ur ON r.OwnerUserId = ur.UserId WHERE r.rn = 1),
// ClosedPosts AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS ClosedCount, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT utp.PostId, utp.Title, utp.Score, ur.TotalPosts, ur.Reputation, COALESCE(upv.UpVoteCount, 0) AS UpVoteCount, COALESCE(dnv.DownVoteCount, 0) AS DownVoteCount, cp.ClosedCount, cp.LastClosedDate
// FROM UserTopPosts utp JOIN UserReputation ur ON utp.OwnerUserId = ur.UserId
// LEFT JOIN (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id) upv ON utp.PostId = upv.PostId
// LEFT JOIN (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id) dnv ON utp.PostId = dnv.PostId
// LEFT JOIN ClosedPosts cp ON utp.PostId = cp.PostId
// WHERE ur.Reputation IS NOT NULL AND (cp.ClosedCount > 0 OR cp.LastClosedDate IS NULL) ORDER BY utp.Score DESC, ur.TotalPosts DESC;
//
// rn = 1 keeps one joined row of each owner's latest recent post, so UserTopPosts is one post per owner; the window counts are never read.
fn q21894(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date: hd, post, .. } = &db.post_history;
    let r1 = top_per(drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let r1 = post_set(&r1, |x| x.0);
    let dp = user_distinct_posts(db);
    let cp = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold((0i64, i64::MIN), |(n, m), (t, d)| (n + (t == 10) as i64, m.max(d)));
    let vc = (&r1).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain(
        (&r1)
            .select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and(&dp))).and(&vc).and((&cp).opt()))
            .filt(|(_, c): (((Id<Post>, (Id<User>, i64)), [i64; 2]), Option<(i64, i64)>)| c.map_or(true, |c| c.0 > 0)),
    );
    rows(v.into_iter().map(|(_, (((p, (u, n)), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.push(V::I(n));
        f.extend(ucols(db, u, &["rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), oint(c.map(|c| c.0)), ots(c.map(|c| c.1))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesCount, COUNT(DISTINCT p.Id) AS PostsCount, COUNT(DISTINCT b.Id) AS BadgesCount,
//        ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, COALESCE(MAX(ph.CreationDate) FILTER (WHERE ph.PostHistoryTypeId = 10), p.CreationDate) AS CloseDate,
//        COALESCE(MAX(ph.CreationDate) FILTER (WHERE ph.PostHistoryTypeId = 11), NULL) AS ReopenDate
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE (p.ViewCount > 100 OR p.Score >= 5) AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.Score, p.CreationDate),
// TagExcerpts AS (SELECT t.TagName, p.Id AS PostId, p.Title FROM Tags t INNER JOIN Posts p ON t.ExcerptPostId = p.Id WHERE t.IsModeratorOnly IS NULL),
// UserPostRank AS (SELECT ps.OwnerUserId AS UserId, COUNT(ps.Id) AS TotalPosts, RANK() OVER (PARTITION BY ps.OwnerUserId ORDER BY COUNT(ps.Id) DESC) AS PostRank FROM Posts ps GROUP BY ps.OwnerUserId),
// FinalStats AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.UpVotesCount, us.DownVotesCount, pa.PostId, pa.Title, pa.Score, pa.CloseDate, pa.ReopenDate, te.TagName, up.TotalPosts, up.PostRank
//     FROM UserStats us LEFT JOIN PostActivity pa ON us.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pa.PostId) LEFT JOIN TagExcerpts te ON pa.PostId = te.PostId
//     LEFT JOIN UserPostRank up ON us.UserId = up.UserId WHERE us.Rank <= 10 AND (pa.CloseDate IS NULL OR pa.ReopenDate IS NOT NULL))
// SELECT DisplayName AS "User", Reputation, UpVotesCount AS "Total Upvotes", DownVotesCount AS "Total Downvotes", Title AS "Post Title", Score AS "Post Score", TagName AS "Associated Tag",
//        TotalPosts AS "Posts Total", PostRank AS "Post Rank"
// FROM FinalStats ORDER BY Reputation DESC, Score DESC;
//
// Rank reads only Reputation, so the ten users are picked first. The subquery reads pa's own post, so the join is on its owner. CloseDate is a COALESCE
// with the creation date, never NULL once a PostActivity row joined, so a joined row passes only with a ReopenDate. The vote x post x badge product is
// driven only for the users that reach the output. PostRank partitions by the group key, so it is 1.
fn q23897(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let rep = &db.user.reputation;
    let tu = top_n(drain(db.user.select(rep)), |&(u, r)| (Reverse(r), u), 10);
    let tus = ids_set(&tu, |x| x.0);
    let pa_posts = db.post.with(view_count.gt(100).or(score.ge(5))).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let reopen = pa_posts.group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(11))).select(hd)).fold(i64::MIN, |m, d| m.max(d));
    let pa: MatSet<Id<Post>> = db.post.with(view_count.gt(100).or(score.ge(5))).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).collect();
    let te: HashIdx<Id<Post>, Id<Tag>> = db.tag.minus(&db.tag.is_moderator_only).select(&db.tag.excerpt_post).inv().collect();
    let up = db.post.group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let joined = drain((&tus).select(Ident::<User>::new().and(posts_of(db).select(Ident::<Post>::new().and(&reopen).and((&te).opt())))));
    let bare = drain((&tus).minus(posts_of(db).with(&pa)));
    let mut rs: Vec<(Id<User>, Option<(Id<Post>, Option<Id<Tag>>)>)> = joined.into_iter().map(|(_, (u, ((p, _), t)))| (u, Some((p, t)))).collect();
    rs.extend(bare.into_iter().map(|(u, _)| (u, None)));
    let out: MatSet<Id<User>> = rel(rs.clone()).map(|(u, _)| u).collect();
    let us = (&out)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt()).and(badges_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type R = (Id<User>, Option<(Id<Post>, Option<Id<Tag>>)>);
    let r = rel(rs);
    let v = drain((&r).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&us).and((&up).opt())))));
    rows(v.into_iter().map(|(_, ((u, e), (a, n)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        match e {
            Some((p, t)) => {
                f.extend(post_fields(db, p, &["title", "score"]));
                f.push(ostr(t.map(|t| db.tag.tag_name.get(t).unwrap())));
            }
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.extend([oint(n), oint(n.map(|_| 1))]);
        row(f)
    }))
}

// WITH TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U WHERE U.Reputation IS NOT NULL),
// PostStatistics AS (SELECT P.Id AS PostId, P.OwnerUserId, P.PostTypeId, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(DISTINCT L.RelatedPostId) AS LinkCount, MAX(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS IsClosed
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN PostLinks L ON P.Id = L.PostId
//     WHERE P.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') GROUP BY P.Id, P.OwnerUserId, P.PostTypeId),
// UserPerformance AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN P.OwnerUserId = U.Id THEN 1 ELSE 0 END) AS PostsCreated, SUM(COALESCE(P.CommentCount, 0)) AS TotalComments,
//        SUM(COALESCE(P.Upvotes, 0)) - SUM(COALESCE(P.Downvotes, 0)) AS NetVotes, AVG(COALESCE(P.LinkCount, 0)) AS AvgLinksPerPost, MAX(COALESCE(P.IsClosed, 0)) AS HasClosedPosts
//     FROM Users U LEFT JOIN PostStatistics P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// BadgeStats AS (SELECT B.UserId, COUNT(*) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE B.Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE B.Class = 3) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId),
// FinalStats AS (SELECT UP.UserId, UP.DisplayName, UP.PostsCreated, COALESCE(UP.TotalComments, 0) AS TotalComments, COALESCE(UP.NetVotes, 0) AS NetVotes, COALESCE(UP.AvgLinksPerPost, 0) AS AvgLinksPerPost,
//        COALESCE(BS.GoldBadges, 0) AS GoldBadges, COALESCE(BS.SilverBadges, 0) AS SilverBadges, COALESCE(BS.BronzeBadges, 0) AS BronzeBadges,
//        RANK() OVER (ORDER BY UP.TotalComments DESC, UP.NetVotes DESC) AS UserRank
//     FROM UserPerformance UP LEFT JOIN BadgeStats BS ON UP.UserId = BS.UserId WHERE UP.NetVotes IS NOT NULL OR UP.TotalComments IS NOT NULL)
// SELECT FS.UserId, FS.DisplayName, FS.PostsCreated, FS.TotalComments, FS.NetVotes, FS.AvgLinksPerPost, FS.GoldBadges, FS.SilverBadges, FS.BronzeBadges, FS.UserRank,
//        CASE WHEN FS.TotalComments = 0 THEN 'No Comments' WHEN FS.UserRank <= 10 THEN 'Top Commenter' WHEN FS.PostsCreated > 100 THEN 'Veteran Contributor' ELSE 'Regular User' END AS UserCategory
// FROM FinalStats FS WHERE FS.UserRank <= 20 OR FS.GoldBadges > 0 ORDER BY FS.UserRank;
//
// The SUMs of COALESCE run over at least one row per user, so the WHERE in FinalStats keeps every user.
fn q23169(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(links_of(db).select(&db.post_link.related_post_id).opt()))
        .buf_fold(|v| {
            let c = v.iter().filter(|x| x.0 .0.is_some()).count() as i64;
            let up = v.iter().filter(|x| x.0 .1 == Some(2)).count() as i64;
            let dn = v.iter().filter(|x| x.0 .1 == Some(3)).count() as i64;
            [c, up - dn, distinct_some(v.iter().map(|x| x.1))]
        });
    let upf = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&ps).opt()).fold([0i64; 5], |a, p| match p {
        Some(p) => [a[0] + 1, a[1] + p[0], a[2] + p[1], a[3] + p[2], a[4] + 1],
        None => [a[0], a[1], a[2], a[3], a[4] + 1],
    });
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = ranked(drain((&upf).and((&bs).opt())), |&(_, (a, _))| (Reverse(a[1]), Reverse(a[2])), false);
    type R = ((Id<User>, ([i64; 5], Option<[i64; 3]>)), i64);
    let v = drain(rel(v).filt(|x: R| x.1 <= 20 || x.0 .1 .1.map_or(0, |b| b[0]) > 0));
    rows(v.into_iter().map(|(_, ((u, (a, b)), k))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(k)]);
        f.push(V::S(if a[1] == 0 { "No Comments" } else if k <= 10 { "Top Commenter" } else if a[0] > 100 { "Veteran Contributor" } else { "Regular User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year')
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COUNT(DISTINCT rp.PostId) AS TotalPosts, AVG(rp.Score) AS AverageScore,
//        CASE WHEN AVG(rp.ViewCount) IS NOT NULL THEN AVG(rp.ViewCount) ELSE 0 END AS AverageViews, ARRAY_AGG(DISTINCT t.TagName) AS TagsUsed,
//        MAX(b.Date) FILTER (WHERE b.Class = 1) AS GoldBadgeDate
// FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId AND rp.PostRank <= 5 LEFT JOIN Votes v ON u.Id = v.UserId AND v.VoteTypeId IN (1, 2)
// LEFT JOIN PostLinks pl ON rp.PostId = pl.PostId LEFT JOIN Tags t ON t.Id = pl.RelatedPostId LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName, u.Reputation ORDER BY TotalPosts DESC, Reputation DESC LIMIT 10 OFFSET 5;
//
// TotalPosts is the number of ranked posts, so the fifteen users are picked on it first and the product is driven for the ten kept. A tie on
// CreationDate inside PostRank goes to the smaller id. `t.Id = pl.RelatedPostId` joins a tag id to a post id, so it goes through the raw ids.
fn q24713(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let rp = top_per(drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rps = post_set(&rp, |x| x.0);
    let nrp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).with(&rps).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let top = top_n(drain(&nrp), |&(u, n)| (Reverse(n), Reverse(db.user.reputation.get(u).unwrap())), 15);
    let keep = ids_set(&top[5.min(top.len())..], |x| x.0);
    let tidx: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let prow = posts_of(db).with(&rps).select(Ident::<Post>::new().and(score).and(view_count.opt()).and(links_of(db).select((&db.post_link.related_post_id).select(&tidx).opt()).opt()));
    let vrow = votes_by(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([1, 2]))).select(bounty_amount.opt());
    let brow = badges_of(db).select((&db.badge.class).and(&db.badge.date));
    let agg = (&keep).group_by(Ident::<User>::new()).select(prow.opt().and(vrow.opt()).and(brow.opt())).buf_fold(|v| {
        let b: i64 = v.iter().map(|x| x.0 .1.flatten().unwrap_or(0)).sum();
        let n = distinct_some(v.iter().map(|x| x.0 .0.map(|p| p.0 .0 .0)));
        let (mut ss, mut sn, mut vs, mut vn) = (0i64, 0i64, 0i64, 0i64);
        for x in v.iter() {
            if let Some((((_, s), w), _)) = x.0 .0 {
                ss += s;
                sn += 1;
                if let Some(w) = w {
                    vs += w;
                    vn += 1;
                }
            }
        }
        let mut t: Vec<Option<Str>> = v.iter().map(|x| x.0 .0.and_then(|p| p.1.flatten()).map(|t| db.tag.tag_name.get(t).unwrap())).collect();
        t.sort_unstable();
        t.dedup();
        let g = v.iter().filter_map(|x| x.1.and_then(|(c, d)| if c == 1 { Some(d) } else { None })).max();
        (b, n, [ss, sn, vs, vn], &*Box::leak(t.into_boxed_slice()), g)
    });
    let v = top_n(drain(&agg), |&(u, a)| (Reverse(a.1), Reverse(db.user.reputation.get(u).unwrap())), 0);
    rows(v.into_iter().map(|(u, (b, n, s, t, g))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(n), avg(s[0], s[1]), if s[3] == 0 { V::F(0.0) } else { avg(s[2], s[3]) }]);
        f.push(V::L(t.iter().map(|&x| ostr(x)).collect()));
        f.push(ots(g));
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, UNNEST(STRING_TO_ARRAY(SUBSTRING(p.Tags, 2, LENGTH(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COUNT(v.Id) AS VoteCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT t.Tag) AS TagCount,
//        ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC, COUNT(v.Id) DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostTags t ON p.Id = t.PostId
//     GROUP BY p.Id, p.Title, p.ViewCount),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.VoteCount, rp.CommentCount, rp.TagCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT t.Tag, COUNT(*) AS NumberOfPosts, SUM(tr.ViewCount) AS TotalViewCount, SUM(tr.VoteCount) AS TotalVotes, SUM(tr.CommentCount) AS TotalComments
// FROM TopRankedPosts tr JOIN PostTags t ON tr.PostId = t.PostId GROUP BY t.Tag ORDER BY NumberOfPosts DESC, TotalViewCount DESC;
//
// Rank leads with ViewCount, so the product is driven only for the posts whose ViewCount reaches the tenth largest.
fn q25882(db: &'static So) -> String {
    let Post { view_count, post_type_id, tags_str, .. } = &db.post;
    let pt = || Ident::<Post>::new().with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list));
    let vs = top_n(drain(db.post.select(view_count)), |&(_, v)| Reverse(v), 10);
    let cut = vs.last().map_or(i64::MIN, |x| x.1);
    let cand = db.post.with(view_count.ge(cut));
    let agg = cand
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).opt().and(comments_of(db).opt()).and(pt().opt()))
        .fold([0i64; 2], |a, ((v, c), _)| [a[0] + v.is_some() as i64, a[1] + c.is_some() as i64]);
    let top = top_n(drain(&agg), |&(p, a)| (Reverse(view_count.get(p)), Reverse(a[0]), p), 10);
    let tr = rel(top);
    type R = (Id<Post>, [i64; 2]);
    let rows_ = drain((&tr).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(pt()))));
    type T = (R, Str);
    let g = rel(rows_.into_iter().map(|(_, x)| x).collect())
        .group_by(Same::<T>::new().map(|x: T| x.1))
        .select(Same::<T>::new().map(|x: T| x.0).and(Same::<T>::new().map(|x: T| x.0 .0).select(view_count.opt())))
        .fold([0i64; 5], |a, ((_, b), v)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + b[0], a[4] + b[1]]);
    rows(drain(&g).into_iter().map(|(t, a)| row(vec![V::S(t), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4])])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000 AND p.PostTypeId = 1),
// PostComments AS (SELECT c.PostId, COUNT(*) AS CommentCount, MAX(c.CreationDate) AS LatestCommentDate FROM Comments c GROUP BY c.PostId),
// HighScoringPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, COALESCE(pc.CommentCount, 0) AS CommentCount, pc.LatestCommentDate
//     FROM RankedPosts rp LEFT JOIN PostComments pc ON rp.Id = pc.PostId WHERE rp.UserPostRank <= 3)
// SELECT hsp.Id, hsp.Title, hsp.CreationDate, hsp.Score, hsp.CommentCount, CASE WHEN hsp.LatestCommentDate IS NULL THEN 'No comments' ELSE 'Has comments' END AS CommentStatus
// FROM HighScoringPosts hsp WHERE hsp.Score > 10
// UNION ALL SELECT NULL, 'Total High Scoring Posts', NULL, COUNT(*), NULL, NULL FROM HighScoringPosts;
//
// A tie on CreationDate inside UserPostRank goes to the smaller id.
fn q4346(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let rp = top_per(
        drain(db.post.with(post_type_id.eq(1)).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))),
        |&(_, u)| u,
        |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p),
        3,
        false,
    );
    let hs = post_set(&rp, |x| x.0);
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let n = count(&hs);
    let mut out: Vec<String> = drain((&hs).with(score.gt(10)).select(Ident::<Post>::new().and((&pc).opt()))).into_iter().map(|(_, (p, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c.unwrap_or(0)), V::S(if c.is_some() { "Has comments" } else { "No comments" })]);
        row(f)
    }).collect();
    out.push(row(vec![V::Null, V::S("Total High Scoring Posts"), V::Null, V::I(n), V::Null, V::Null]));
    rows(out)
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.AnswerCount, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, AnswerCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// PostAnnotations AS (SELECT H.PostId, STRING_AGG(CASE WHEN H.PostHistoryTypeId = 10 THEN C.Name END, ', ') AS CloseReasons, MAX(H.CreationDate) AS LastEdited, COUNT(C.Id) AS CommentCount
//     FROM PostHistory H LEFT JOIN CloseReasonTypes C ON CAST(H.Comment AS TEXT) = CAST(C.Id AS TEXT) GROUP BY H.PostId)
// SELECT T.Title, T.Score, T.ViewCount, T.AnswerCount, T.OwnerDisplayName, A.CloseReasons, A.LastEdited, A.CommentCount
// FROM TopPosts T LEFT JOIN PostAnnotations A ON T.PostId = A.PostId ORDER BY T.Score DESC, T.ViewCount DESC;
//
// PostAnnotations is only read for the ten posts. The close reason ids are joined as text, as the SQL casts them.
fn q5962(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let top = top_n(
        drain(db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user)),
        |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())),
        10,
    );
    let tp = post_set(&top, |x| x.0);
    let crt: HashIdx<Str, Id<CloseReasonType>> = (&db.close_reason_type.origid).map(|i: i64| &*Box::leak(i.to_string().into_boxed_str())).inv().collect();
    let pa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().and(post_history_type_id).and(hd).and(comment.select(&crt).opt())))
        .buf_fold(|v| {
            let mut v: Vec<_> = v.into_iter().collect();
            v.sort_by_key(|x| x.0 .0 .0);
            let names: Vec<Str> = v.iter().filter(|x| x.0 .0 .1 == 10).filter_map(|x| x.1.map(|c| db.close_reason_type.name.get(c).unwrap())).collect();
            let s: Option<Str> = if names.is_empty() { None } else { Some(&*Box::leak(names.join(", ").into_boxed_str())) };
            (s, v.iter().map(|x| x.0 .1).max().unwrap(), v.iter().filter(|x| x.1.is_some()).count() as i64)
        });
    rows(drain((&tp).select(Ident::<Post>::new().and(owner_user).and((&pa).opt()))).into_iter().map(|(_, ((p, u), a))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "answers"]);
        f.extend(ucols(db, u, &["name"]));
        match a {
            Some((s, d, n)) => f.extend([ostr(s), V::T(d), V::I(n)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, p.AnswerCount, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2)),
// FilteredPosts AS (SELECT PostId, Title, Body, CreationDate, Score, AnswerCount, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.Tags IS NOT NULL),
// PostCommentStats AS (SELECT PostId, COUNT(*) AS CommentCount, AVG(Score) AS AvgCommentScore FROM Comments GROUP BY PostId)
// SELECT fp.PostId, fp.Title, fp.Body, fp.CreationDate, fp.Score, fp.AnswerCount, fp.ViewCount, fp.OwnerDisplayName, pt.Tag, pcs.CommentCount, pcs.AvgCommentScore
// FROM FilteredPosts fp LEFT JOIN PostTags pt ON fp.PostId = pt.PostId LEFT JOIN PostCommentStats pcs ON fp.PostId = pcs.PostId ORDER BY fp.PostId, pt.Tag;
//
// A tie on Score inside Rank goes to the smaller id.
fn q29104(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, tags_str, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.is_in([1, 2])).with(owner_user).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp = post_set(&top, |x| x.0);
    let pcs = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    rows(drain((&tp).select(Ident::<Post>::new().and(owner_user).and(tags_str.flat_map(tag_list).opt()).and((&pcs).opt()))).into_iter().map(|(_, (((p, u), t), c))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "score", "answers", "views"]);
        f.extend(ucols(db, u, &["name"]));
        f.push(ostr(t));
        match c {
            Some((n, s)) => f.extend([V::I(n), avg(s, n)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankByDate
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, OwnerDisplayName, CommentCount, VoteCount, RankByScore, RankByDate FROM RankedPosts WHERE RankByScore <= 10 OR RankByDate <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount, (SELECT STRING_AGG(t.TagName, ', ') FROM Tags t WHERE t.ExcerptPostId = tp.PostId) AS Tags
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// Both ranks read only base columns, so the posts are picked first and the product is driven for them.
fn q5051(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let base = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let a = top_per(base.clone(), |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let b = top_per(base, |&(_, t)| t, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 10, true);
    let ra = post_set(&a, |x| x.0);
    let rb = post_set(&b, |x| x.0);
    let tp: MatSet<Id<Post>> = db.post.with((&ra).or(&rb)).collect();
    let cv = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).buf_fold(|v| (v.iter().filter(|x| x.0.is_some()).count() as i64, distinct_some(v.iter().map(|x| x.1))));
    let te = (&db.tag.excerpt_post).inv().collect::<HashIdx<Id<Post>, Id<Tag>>>();
    let tg = (&tp).group_by(Ident::<Post>::new()).select((&te).select(&db.tag.tag_name)).buf_fold(|v| &*Box::leak(v.join(", ").into_boxed_str()));
    rows(drain((&cv).and((&tg).opt())).into_iter().map(|(p, ((c, n), t))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner"]);
        f.extend([V::I(c), V::I(n), ostr(t)]);
        row(f)
    }))
}

// rewrites/26499.sql (the STRING_AGG given ORDER BY t.TagName):
// WITH TagStatistics AS (SELECT t.TagName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        AVG(u.Reputation) AS AverageReputation FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN Users u ON p.OwnerUserId = u.Id GROUP BY t.TagName),
// TopTags AS (SELECT TagName, TotalPosts, QuestionsCount, AnswersCount, AverageReputation, RANK() OVER (ORDER BY TotalPosts DESC) AS TagRank FROM TagStatistics),
// RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags FROM Posts p JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%'
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY p.Id, p.Title, p.CreationDate)
// SELECT tt.TagName, tt.TotalPosts, tt.QuestionsCount, tt.AnswersCount, tt.AverageReputation, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate, rp.Tags AS RecentPostTags
// FROM TopTags tt LEFT JOIN RecentPosts rp ON rp.Tags LIKE '%' || tt.TagName || '%' WHERE tt.TagRank <= 10 ORDER BY tt.TagRank, rp.CreationDate DESC;
//
// Tag names are unique, so grouping by TagName is grouping by tag. The second LIKE is a substring join, scanned over the recent posts' strings.
fn q26499(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let st = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select(post_type_id.and(owner_user.select(&db.user.reputation))))
        .fold([0i64; 4], |a, (t, r)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + r]);
    let tt = ranked(drain(&st), |&(_, a)| Reverse(a[0]), false);
    type R = ((Id<Tag>, [i64; 4]), i64);
    let tt = rel(tt);
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&lt).map(|(p, _)| p).inv().collect();
    let rp = db
        .post
        .with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select((&by_post).map(|(_, t)| t).select(&db.tag.tag_name))
        .buf_fold(|v| {
            let mut v: Vec<Str> = v.into_iter().collect();
            v.sort_unstable();
            &*Box::leak(v.join(", ").into_boxed_str())
        });
    let rpi: HashIdx<Str, Id<Post>> = (&rp).inv().collect();
    let hit = (&db.tag.tag_name).select_where(&rpi, |n: Str, s: Str| s.contains(n)).select(Ident::<Post>::new().and(&rp));
    let v = drain((&tt).filt(|x: R| x.1 <= 10).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(hit.opt()))));
    rows(v.into_iter().map(|(_, (((t, a), _), p))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])];
        match p {
            Some((p, s)) => {
                f.extend(post_fields(db, p, &["title", "created"]));
                f.push(V::S(s));
            }
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount, COALESCE((SELECT COUNT(*) FROM Badges b WHERE b.UserId = p.OwnerUserId), 0) AS BadgeCount,
//        ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rnk
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rnk <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.UpvoteCount, tp.CommentCount, tp.BadgeCount, STRING_AGG(t.TagName, ', ') AS Tags
// FROM TopPosts tp LEFT JOIN Posts p ON tp.PostId = p.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.Id
// GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.UpvoteCount, tp.CommentCount, tp.BadgeCount ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// Rnk reads only base columns, so the ten questions are picked first.
fn q8715(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap())), 10);
    let tp = post_set(&top, |x| x.0);
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let te = (&db.tag.excerpt_post).inv().collect::<HashIdx<Id<Post>, Id<Tag>>>();
    let tg = (&tp).group_by(Ident::<Post>::new()).select((&te).select(&db.tag.tag_name)).buf_fold(|v| &*Box::leak(v.join(", ").into_boxed_str()));
    rows(drain((&uv).and(&cc).and(owner_user.select(&bc).opt()).and((&tg).opt())).into_iter().map(|(p, (((u, c), b), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(u), V::I(c), V::I(b.unwrap_or(0)), ostr(t)]);
        row(f)
    }))
}

// rewrites/1043.sql (the original ORDER BY with rp.Id appended):
// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.AnswerCount, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn, p.OwnerUserId, p.Tags
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// PopularTags AS (SELECT TRIM(unnest(string_to_array(Tags, '><'))) AS Tag FROM Posts WHERE PostTypeId = 1),
// TagStatistics AS (SELECT t.Tag AS TagName, COUNT(*) AS TagCount FROM PopularTags t GROUP BY t.Tag ORDER BY TagCount DESC LIMIT 10)
// SELECT up.UserId, up.Reputation, up.TotalBounty, rp.Title, rp.Score, rp.AnswerCount, rp.ViewCount, tt.TagName
// FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN TagStatistics tt ON tt.TagName = rp.Tags
// WHERE up.Reputation > 1000 AND (rp.AnswerCount > 5 OR rp.ViewCount > 100) ORDER BY up.Reputation DESC, rp.Score DESC, rp.Id LIMIT 50;
//
// The tags are split on '><' without stripping the outer brackets, so only a one-tag string like '<mysql>' can equal an element whole. rn is never read.
fn q1043(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, answer_count, view_count, score, .. } = &db.post;
    let rep = &db.user.reputation;
    let el = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(|t: Str| t.split("><").map(|s| s.trim()))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = top_n(drain(&el), |&(t, n)| (Reverse(n), t), 10);
    let tt: MatSet<Str> = rel(top).map(|(t, _)| t).collect();
    let tb = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let rp = db.post.with(post_type_id.eq(1)).with(answer_count.gt(5).or(view_count.gt(100)));
    let v = drain(rp.select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().with(rep.gt(1000)).and(&tb))).and(tags_str.with(&tt).opt())));
    let v = top_n(v, |&(p, ((_, (u, _)), _))| (Reverse(rep.get(u).unwrap()), Reverse(score.get(p).unwrap()), db.post.origid.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(_, ((p, (u, b)), t))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "score", "answers", "views"]));
        f.push(ostr(t));
        row(f)
    }))
}

fn cross_top<A: Copy, B: Copy, KA: Ord, KB: Ord>(a: Vec<A>, ka: impl Fn(&A) -> KA, b: Vec<B>, kb: impl Fn(&B) -> KB, n: usize) -> Vec<(A, B)> {
    let n = if n == usize::MAX { 0 } else { n };
    let a = top_n(a, &ka, 0);
    let ra = rel(a);
    let rb = rel(b);
    let v = if n > 0 && !rb.v.is_empty() && (n - 1) / rb.v.len() + 1 < ra.v.len() {
        let cut = ka(&ra.v[(n - 1) / rb.v.len()]);
        drain((&ra).filt(|x| ka(&x) <= cut).cross(&rb))
    } else {
        drain((&ra).cross(&rb))
    };
    top_n(v, |(_, (x, y))| (ka(x), kb(y)), n).into_iter().map(|x| x.1).collect()
}

fn id_text(db: &'static So) -> HashIdx<Str, Id<CloseReasonType>> {
    (&db.close_reason_type.origid).map(|i: i64| &*Box::leak(i.to_string().into_boxed_str())).inv().collect()
}

// WITH FilteredPosts AS (SELECT p.Id AS PostID, p.Title, p.ViewCount, p.Tags, STRING_AGG(t.TagName, ', ') AS TagList, U.DisplayName AS OwnerDisplayName
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Tags t ON t.WikiPostId = p.Id OR t.ExcerptPostId = p.Id
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 MONTH' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.ViewCount, p.Tags, U.DisplayName),
// PostScore AS (SELECT PostID, Title, ViewCount, TagList, OwnerDisplayName, RANK() OVER (ORDER BY ViewCount DESC) AS ViewRank FROM FilteredPosts),
// ClosedPostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName AS ClosedBy, c.Name AS CloseReason FROM PostHistory ph JOIN CloseReasonTypes c ON ph.Comment::int = c.Id
//     WHERE ph.PostHistoryTypeId = 10)
// SELECT ps.PostID, ps.Title, ps.ViewCount, ps.TagList, ps.OwnerDisplayName, ps.ViewRank, COALESCE(cph.ClosedBy, 'Not Closed') AS ClosedBy, COALESCE(cph.CloseReason, 'N/A') AS CloseReason
// FROM PostScore ps LEFT JOIN ClosedPostHistory cph ON ps.PostID = cph.PostId ORDER BY ps.ViewRank;
//
// The OR join is the union of the wiki and excerpt matches, each tag once.
fn q29664(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, view_count, .. } = &db.post;
    let PostHistory { post_history_type_id, comment, user_display_name, .. } = &db.post_history;
    let wt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).union(&db.tag.excerpt_post).inv().collect();
    let fp = || db.post.with(creation_date.ge(add_months(date(2024, 10, 1), -1))).with(post_type_id.is_in([1, 2])).with(owner_user);
    let tl = fp().group_by(Ident::<Post>::new()).select(&wt).buf_fold(|v| {
        let mut v: Vec<Id<Tag>> = v.into_iter().collect();
        v.sort_unstable();
        v.dedup();
        let names: Vec<Str> = v.iter().map(|&t| db.tag.tag_name.get(t).unwrap()).collect();
        &*Box::leak(names.join(", ").into_boxed_str())
    });
    let crt: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    let cph = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt)));
    let v = ranked(drain(fp().select(Ident::<Post>::new().and(owner_user).and((&tl).opt()))), |&(_, ((p, _), _))| (view_count.get(p).is_none(), Reverse(view_count.get(p))), false);
    type R = ((Id<Post>, ((Id<Post>, Id<User>), Option<Str>)), i64);
    let v = drain(rel(v).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(&cph).opt())));
    rows(v.into_iter().map(|(_, (((_, ((p, u), t)), k), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.push(ostr(t));
        f.extend(ucols(db, u, &["name"]));
        f.push(V::I(k));
        match c {
            Some((h, r)) => f.extend([V::S(user_display_name.get(h).unwrap_or("Not Closed")), V::S(db.close_reason_type.name.get(r).unwrap())]),
            None => f.extend([V::S("Not Closed"), V::S("N/A")]),
        }
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// PopularTags AS (SELECT Tag, COUNT(*) AS TagCount FROM PostTags GROUP BY Tag HAVING COUNT(*) > 10),
// RecentUserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount,
//        SUM(CASE WHEN p.LastActivityDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' THEN 1 ELSE 0 END) AS RecentActivityCount,
//        AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgPostAge
//     FROM Users u JOIN Posts p ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionCount, RecentActivityCount, AvgPostAge, RANK() OVER (ORDER BY QuestionCount DESC) AS UserRank FROM RecentUserActivity WHERE RecentActivityCount > 0)
// SELECT t.Tag, t.TagCount, tu.DisplayName, tu.QuestionCount, tu.RecentActivityCount, tu.AvgPostAge
// FROM PopularTags t JOIN TopUsers tu ON tu.QuestionCount > 5 ORDER BY t.TagCount DESC, tu.QuestionCount DESC LIMIT 10;
//
// `ON tu.QuestionCount > 5` names only TopUsers, so it is a cross join. UserRank is never read.
fn q26831(db: &'static So) -> String {
    let Post { post_type_id, tags_str, last_activity_date, creation_date, .. } = &db.post;
    let tc = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let pt = drain((&tc).filt(|n| n > 10));
    let lo = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(last_activity_date.and(creation_date)))
        .fold((0i64, 0i64, 0.0f64), |(n, r, s), (l, c)| (n + 1, r + (l >= lo) as i64, s + secs(l - c)));
    let tu = drain((&ua).filt(|a| a.1 > 0 && a.0 > 5));
    let v = cross_top(pt, |&(_, n)| Reverse(n), tu, |&(_, a)| Reverse(a.0), 10);
    rows(v.into_iter().map(|((t, n), (u, a))| {
        let mut f = vec![V::S(t), V::I(n)];
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(a.0), V::I(a.1), V::F(a.2 / a.0 as f64)]);
        row(f)
    }))
}

// WITH UserVotes AS (SELECT UserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes GROUP BY UserId),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(v.Upvotes, 0) AS TotalUpvotes, COALESCE(v.Downvotes, 0) AS TotalDownvotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank FROM Posts p LEFT JOIN UserVotes v ON p.OwnerUserId = v.UserId),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, STRING_AGG(CASE WHEN ph.PostHistoryTypeId = 10 THEN cr.Name END, ', ') AS CloseReasons, COUNT(*) AS CloseCount
//     FROM PostHistory ph LEFT JOIN CloseReasonTypes cr ON ph.Comment = CAST(cr.Id AS VARCHAR) WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.TotalUpvotes, ps.TotalDownvotes, cl.CloseReasons, cl.CloseCount
// FROM PostStats ps LEFT JOIN ClosedPosts cl ON ps.PostId = cl.PostId WHERE ps.RecentPostRank <= 5 ORDER BY ps.Score DESC, ps.CreationDate DESC FETCH FIRST 100 ROWS ONLY;
//
// RecentPostRank partitions by the raw OwnerUserId, ownerless posts together; a tie on CreationDate goes to the smaller id. UserVotes joins on the raw
// ids. Each post keeps at least one row after the LEFT JOIN, so the hundred best posts are picked before it.
fn q3202(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, .. } = &db.post;
    let PostHistory { post_history_type_id, comment, creation_date: hd, post, .. } = &db.post_history;
    let rp = top_per(drain(db.post.select(owner_user_id.opt())), |&(_, o)| o, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let key = |p: Id<Post>| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()));
    let tp = top_n(rp, |&(p, _)| key(p), 100);
    let tp = post_set(&tp, |x| x.0);
    let uv = db.vote.group_by(&db.vote.user_id).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let crt = id_text(db);
    let cl = db
        .post_history
        .with(post_history_type_id.eq(10))
        .with(post.with(&tp))
        .group_by(post.and(hd))
        .select(Ident::<PostHistory>::new().and(comment.select(&crt).opt()))
        .buf_fold(|v| {
            let mut v: Vec<_> = v.into_iter().collect();
            v.sort_by_key(|x| x.0);
            let names: Vec<Str> = v.iter().filter_map(|x| x.1.map(|c| db.close_reason_type.name.get(c).unwrap())).collect();
            let s: Option<Str> = if names.is_empty() { None } else { Some(&*Box::leak(names.join(", ").into_boxed_str())) };
            (s, v.len() as i64)
        });
    let cr = rel(drain(&cl));
    let cli: HashIdx<Id<Post>, ((Id<Post>, i64), (Option<Str>, i64))> = (&cr).map(|((p, _), _)| p).inv().select(&cr).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user_id.select(&uv).opt()).and((&cli).opt())));
    let v = top_n(v, |&(_, ((p, _), _))| key(p), 100);
    rows(v.into_iter().map(|(_, ((p, a), c))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        match c {
            Some((_, (s, n))) => f.extend([ostr(s), V::I(n)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// UserPostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId IN (0, 4, 5) THEN 1 ELSE 0 END) AS WikiCount FROM Posts P GROUP BY P.OwnerUserId),
// RankedUsers AS (SELECT U.Id, COALESCE(BadgeCount, 0) AS BadgeCount, COALESCE(PostCount, 0) AS PostCount, USERPOSTSTATS.QuestionCount, USERPOSTSTATS.AnswerCount, USERPOSTSTATS.WikiCount,
//        RANK() OVER (ORDER BY COALESCE(BadgeCount, 0) DESC, COALESCE(PostCount, 0) DESC) AS UserRank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN UserPostStats USERPOSTSTATS ON U.Id = USERPOSTSTATS.OwnerUserId)
// SELECT U.DisplayName, R.UserRank, R.BadgeCount, R.PostCount, R.QuestionCount, R.AnswerCount, R.WikiCount
// FROM RankedUsers R JOIN Users U ON R.Id = U.Id WHERE R.UserRank <= 10 AND R.BadgeCount > 0 ORDER BY R.UserRank;
//
// BadgeNames is never read.
fn q891(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = db.post.group_by(owner_user).select(post_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + [0, 4, 5].contains(&t) as i64]);
    let v = ranked(drain((&ub).and((&ups).opt())), |&(_, (b, p))| (Reverse(b), Reverse(p.map_or(0, |p| p[0]))), false);
    type R = ((Id<User>, (i64, Option<[i64; 4]>)), i64);
    let v = drain(rel(v).filt(|x: R| x.1 <= 10 && x.0 .1 .0 > 0));
    rows(v.into_iter().map(|(_, ((u, (b, p)), k))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(k), V::I(b), V::I(p.map_or(0, |p| p[0])), oint(p.map(|p| p[1])), oint(p.map(|p| p[2])), oint(p.map(|p| p[3]))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount, t.TagName
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2
//     LEFT JOIN LATERAL (SELECT unnest(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) AS TagName) AS t ON true
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, t.TagName),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, CommentCount, AnswerCount, ROW_NUMBER() OVER (ORDER BY ViewCount DESC, AnswerCount DESC, CreationDate ASC) AS Rank FROM RankedPosts)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.CommentCount, tp.AnswerCount, t.TagName, u.DisplayName AS OwnerName, u.Reputation AS OwnerReputation
// FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id
// JOIN LATERAL (SELECT unnest(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) AS TagName) AS t ON true
// WHERE tp.Rank <= 10 ORDER BY tp.Rank;
//
// RankedPosts has one row per question and tag, and the rows of one question agree on every column TopPosts keeps, so which of them a tie at the
// cut keeps does not change the answer. The order leads with ViewCount, so AnswerCount is only counted for the questions reaching the tenth view count.
fn q26209(db: &'static So) -> String {
    let Post { view_count, post_type_id, tags_str, creation_date, owner_user, .. } = &db.post;
    let vs = top_n(drain(db.post.with(post_type_id.eq(1)).select(view_count)), |&(_, v)| Reverse(v), 10);
    let cut = vs.last().map_or(i64::MIN, |x| x.1);
    let cand = || db.post.with(post_type_id.eq(1)).with(view_count.ge(cut));
    let ac = cand().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let cc = cand().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rp = drain(cand().select(Ident::<Post>::new().and(&ac).and(&cc).and(tags_str.flat_map(tag_list).opt())));
    let tp = top_n(rp, |&(_, (((p, a), _), _))| (Reverse(view_count.get(p)), Reverse(a), creation_date.get(p).unwrap()), 10);
    type R = (Id<Post>, (((Id<Post>, i64), i64), Option<Str>));
    let v = drain(rel(tp).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(owner_user.and(tags_str.flat_map(tag_list))))));
    rows(v.into_iter().map(|(_, ((_, (((p, a), c), _)), (u, t)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(c), V::I(a), V::S(t)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.RankScore, rp.CommentCount, COALESCE(b.Name, 'No Badge') AS UserBadge
//     FROM RankedPosts rp LEFT JOIN Badges b ON rp.PostId = b.UserId WHERE rp.RankScore <= 5),
// ClosedPosts AS (SELECT ph.PostId AS ClosedPostId, ph.CreationDate, ph.UserDisplayName, STRING_AGG(pt.Name, ', ') AS CloseReasons
//     FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id WHERE pt.Name IN ('Post Closed', 'Post Reopened') GROUP BY ph.PostId, ph.CreationDate, ph.UserDisplayName)
// SELECT tp.PostId, tp.Title, tp.Score, tp.CommentCount, tp.UserBadge, cp.CloseReasons FROM TopPosts tp LEFT JOIN ClosedPosts cp ON tp.PostId = cp.ClosedPostId
// WHERE tp.CommentCount > 0 ORDER BY tp.Score DESC;
//
// RankScore numbers the post x comment rows. `rp.PostId = b.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q4776(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let PostHistory { post, creation_date: hd, user_display_name, .. } = &db.post_history;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rp = drain(recent().select(post_type_id.and(comments_of(db).opt())));
    let tp = top_per(rp, |&(_, (t, _))| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, false);
    let tps = post_set(&tp, |x| x.0);
    let cc = (&tps).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let names = htype_name(db);
    let cp = db
        .post_history
        .with(post.with(&tps))
        .with((&names).filt(|n: Str| n == "Post Closed" || n == "Post Reopened"))
        .group_by(post.and(hd).and(user_display_name.opt()))
        .select(&names)
        .buf_fold(|v| &*Box::leak(v.join(", ").into_boxed_str()));
    let cr = rel(drain(&cp));
    let cpi: HashIdx<Id<Post>, Str> = (&cr).map(|(((p, _), _), _)| p).inv().select((&cr).map(|(_, s)| s)).collect();
    type R = (Id<Post>, (i64, Option<Id<Comment>>));
    let v = drain(rel(tp).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&cc).filt(|n| n > 0).and((&db.post.origid).select(&uidx).select(badges_of(db)).select(&db.badge.name).opt()).and((&cpi).opt())))));
    rows(v.into_iter().map(|(_, ((p, _), ((c, b), s)))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(c), V::S(b.unwrap_or("No Badge")), ostr(s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'),
// HighScoreComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, ' | ') AS CommentTexts FROM Comments c JOIN RankedPosts rp ON c.PostId = rp.PostId
//     WHERE rp.Rank <= 5 GROUP BY c.PostId),
// FinalResult AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplayName, hcc.CommentCount, hcc.CommentTexts
//     FROM RankedPosts rp LEFT JOIN HighScoreComments hcc ON rp.PostId = hcc.PostId)
// SELECT fr.PostId, fr.Title, fr.Body, fr.CreationDate, fr.ViewCount, fr.Score, fr.OwnerDisplayName, COALESCE(fr.CommentCount, 0) AS CommentCount,
//        COALESCE(fr.CommentTexts, 'No comments available') AS CommentTexts
// FROM FinalResult fr ORDER BY fr.Score DESC, fr.ViewCount DESC LIMIT 10;
//
// The STRING_AGG is built in comment id order. A tie on Score inside Rank goes to the smaller id.
fn q28069(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, view_count, .. } = &db.post;
    let rp = drain(db.post.with(post_type_id.eq(1)).with(creation_date.gt(add_years(date(2024, 10, 1), -1))).select(owner_user));
    let r5 = post_set(&top_per(rp.clone(), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false), |x| x.0);
    let top = top_n(rp, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p))), 10);
    let hc = (&r5).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().and(&db.comment.text))).buf_fold(|v| {
        let mut v: Vec<_> = v.into_iter().collect();
        v.sort_by_key(|x| x.0);
        let t: Vec<Str> = v.iter().map(|x| x.1).collect();
        (v.len() as i64, &*Box::leak(t.join(" | ").into_boxed_str()))
    });
    type R = (Id<Post>, Id<User>);
    let v = drain(rel(top).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&hc).opt()))));
    rows(v.into_iter().map(|(_, ((p, u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "views", "score"]);
        f.extend(ucols(db, u, &["name"]));
        match h {
            Some((n, s)) => f.extend([V::I(n), V::S(s)]),
            None => f.extend([V::I(0), V::S("No comments available")]),
        }
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, TotalScore, AvgViewCount, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats),
// PopularTags AS (SELECT TagName, COUNT(*) AS TagCount FROM (SELECT TRIM(unnest(string_to_array(Tags, '<>'))) AS TagName FROM Posts) AS TagsList GROUP BY TagName),
// TopTags AS (SELECT TagName, TagCount, RANK() OVER (ORDER BY TagCount DESC) AS TagRank FROM PopularTags)
// SELECT tu.DisplayName, tu.TotalPosts, tu.QuestionCount, tu.AnswerCount, tu.TotalScore, tu.AvgViewCount, tt.TagName, tt.TagCount
// FROM TopUsers tu JOIN TopTags tt ON tu.QuestionCount > 10 AND tu.AnswerCount > 30 WHERE tu.ScoreRank <= 10 AND tt.TagRank <= 5 ORDER BY tu.TotalScore DESC, tt.TagCount DESC;
//
// '<>' never occurs in a tag list, so each list is one element, the whole string. The ON names only TopUsers, so it is a cross join.
fn q6348(db: &'static So) -> String {
    let ups = user_posts(db);
    let tu = ranked(drain(&ups), |&(_, a)| (a[1] == 0, Reverse(a[4])), false);
    type U = ((Id<User>, [i64; 10]), i64);
    let tu = drain(rel(tu).filt(|x: U| x.1 <= 10 && x.0 .1[2] > 10 && x.0 .1[3] > 30));
    let pt = db.post.group_by((&db.post.tags_str).flat_map(|t: Str| t.split("<>").map(|s| s.trim()))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tt = ranked(drain(&pt), |&(_, n)| Reverse(n), false);
    type T = ((Str, i64), i64);
    let tt = drain(rel(tt).filt(|x: T| x.1 <= 5));
    let a = rel(tu.into_iter().map(|x| x.1).collect());
    let b = rel(tt.into_iter().map(|x| x.1).collect());
    rows(drain((&a).cross(&b)).into_iter().map(|(_, (((u, a), _), ((t, n), _)))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), avg(a[6], a[5]), V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount FROM RankedPosts rp WHERE rp.UserPostRank <= 3),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT p.PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.CommentCount, u.DisplayName AS PostOwner, COALESCE(ub.BadgeCount, 0) AS UserBadgeCount,
//        COALESCE(ub.BadgeNames, 'No Badges') AS UserBadges
// FROM TopPosts p INNER JOIN Users u ON p.PostId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE p.CommentCount > 5
// ORDER BY p.Score DESC, p.ViewCount ASC FETCH FIRST 10 ROWS ONLY;
//
// UserPostRank partitions by the raw OwnerUserId, ownerless posts together; a tie on Score goes to the smaller id. `p.PostId = u.Id` joins a post id
// to a user id, so it goes through the raw ids. The badge names are joined in badge id order; the answer is empty on this data, so that is untested.
fn q3868(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, view_count, .. } = &db.post;
    let rp = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user_id.opt())), |&(_, o)| o, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp = post_set(&rp, |x| x.0);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(|v| {
        let mut b: Vec<Id<Badge>> = v.into_iter().flatten().collect();
        b.sort_unstable();
        let names: Vec<Str> = b.iter().map(|&x| db.badge.name.get(x).unwrap()).collect();
        (b.len() as i64, if b.is_empty() { None } else { Some(&*Box::leak(names.join(", ").into_boxed_str())) })
    });
    let v = drain((&cc).filt(|n| n > 5).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ub))));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), view_count.get(p).is_none(), view_count.get(p)), 10);
    rows(v.into_iter().map(|(p, (c, (u, (n, s))))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(V::I(c));
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(n), V::S(s.unwrap_or("No Badges"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS AuthorName, P.CreationDate, P.Score, P.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName AS UserName, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        AVG(U.Reputation) AS AvgReputation FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId WHERE P.PostTypeId IN (1, 2) GROUP BY U.Id, U.DisplayName HAVING COUNT(P.Id) > 10),
// UserBadges AS (SELECT B.UserId, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Badges B GROUP BY B.UserId)
// SELECT T.UserId, T.UserName, T.AvgReputation, T.QuestionCount, T.AnswerCount, COALESCE(UB.BadgeNames, 'No Badges') AS Badges, RP.PostId, RP.Title, RP.CreationDate, RP.Score
// FROM TopUsers T LEFT JOIN UserBadges UB ON T.UserId = UB.UserId LEFT JOIN RankedPosts RP ON T.UserId = RP.PostRank WHERE RP.PostRank = 1
// ORDER BY T.AvgReputation DESC, T.QuestionCount DESC;
//
// `T.UserId = RP.PostRank` joins a user id to a rank number, and the WHERE keeps rank 1, so only a top user with id 1 can match, against every
// owner's latest question. The badge names are joined in badge id order; the answer is empty on this data, so that is untested.
fn q9490(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let r1 = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let r = rel(r1.into_iter().map(|(p, _)| (1i64, p)).collect());
    let by_rank: HashIdx<i64, Id<Post>> = (&r).map(|(k, _)| k).inv().select((&r).map(|(_, p)| p)).collect();
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.is_in([1, 2]))).select(post_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).buf_fold(|v| {
        let mut b: Vec<Id<Badge>> = v.into_iter().collect();
        b.sort_unstable();
        let n: Vec<Str> = b.iter().map(|&x| db.badge.name.get(x).unwrap()).collect();
        &*Box::leak(n.join(", ").into_boxed_str())
    });
    let v = drain((&tu).filt(|a| a[0] > 10).and((&ub).opt()).and((&db.user.origid).select(&by_rank)));
    rows(v.into_iter().map(|(u, ((a, b), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f.extend([V::I(a[1]), V::I(a[2]), V::S(b.unwrap_or("No Badges"))]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Tags, p.CreationDate, p.Score, COALESCE(a.AnswerCount, 0) AS AnswerCount, COALESCE(c.CommentCount, 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId WHERE p.PostTypeId = 1),
// ExplodedTags AS (SELECT rp.Id AS PostId, unnest(string_to_array(rp.Tags, ',')) AS Tag FROM RankedPosts rp WHERE rp.rn = 1),
// TagPopularity AS (SELECT Tag, COUNT(PostId) AS TagCount FROM ExplodedTags GROUP BY Tag ORDER BY TagCount DESC),
// TopTags AS (SELECT Tag FROM TagPopularity LIMIT 10)
// SELECT rp.Title, rp.CreationDate, rp.Score, tp.Tag FROM RankedPosts rp JOIN ExplodedTags et ON rp.Id = et.PostId JOIN TopTags tp ON et.Tag = tp.Tag
// ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// rn partitions by the post, so it is always 1. A tag list has no ',', so each list is one element, the whole string. The answer and comment counts
// are never read.
fn q25460(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let et = || db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(|t: Str| t.split(',')));
    let tp = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(|t: Str| t.split(','))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = top_n(drain(&tp), |&(_, n)| Reverse(n), 10);
    let tt: MatSet<Str> = rel(top).map(|(t, _)| t).collect();
    rows(drain(et().with(&tt)).into_iter().map(|(p, t)| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.push(V::S(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) DESC) AS VoteRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// TopPosts AS (SELECT PostId, Title, ViewCount, UpVotes, DownVotes, VoteRank FROM RankedPosts WHERE VoteRank <= 10),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, '; ') AS Comments FROM Comments c JOIN TopPosts tp ON c.PostId = tp.PostId GROUP BY c.PostId)
// SELECT tp.PostId, tp.Title, tp.ViewCount, tp.UpVotes, tp.DownVotes, pc.CommentCount, COALESCE(pc.Comments, 'No comments') AS Comments,
//        CASE WHEN tp.UpVotes - tp.DownVotes > 0 THEN 'Positive' WHEN tp.UpVotes - tp.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId ORDER BY tp.ViewCount DESC;
//
// The STRING_AGG is built in comment id order.
fn q2882(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let rp = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let top = top_n(drain(&rp), |&(_, a)| Reverse(a[0]), 10);
    let tp = post_set(&top, |x| x.0);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().and(&db.comment.text))).buf_fold(|v| {
        let mut v: Vec<_> = v.into_iter().collect();
        v.sort_by_key(|x| x.0);
        let t: Vec<Str> = v.iter().map(|x| x.1).collect();
        (v.len() as i64, &*Box::leak(t.join("; ").into_boxed_str()))
    });
    rows(drain((&tp).select(Ident::<Post>::new().and(&rp).and((&pc).opt()))).into_iter().map(|(_, ((p, a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), oint(c.map(|c| c.0)), V::S(c.map_or("No comments", |c| c.1))]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RECURSIVE UserVoteCounts AS (SELECT u.Id AS UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN v.VoteTypeId IN (1, 8) THEN 1 ELSE 0 END) AS AcceptedVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// PostLinksAggregated AS (SELECT p.Id AS PostId, COUNT(pl.RelatedPostId) AS LinkCount, STRING_AGG(DISTINCT lt.Name, ', ') AS LinkTypes
//     FROM Posts p LEFT JOIN PostLinks pl ON p.Id = pl.PostId LEFT JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id GROUP BY p.Id)
// SELECT u.DisplayName, u.Reputation, ul.TotalVotes, ul.UpVotes, ul.DownVotes, pp.PostId, pp.Title, pp.ViewCount, pp.CreationDate, pla.LinkCount, pla.LinkTypes
// FROM Users u JOIN UserVoteCounts ul ON u.Id = ul.UserId JOIN TopPosts pp ON pp.Rank <= 10 LEFT JOIN PostLinksAggregated pla ON pp.PostId = pla.PostId
// WHERE u.Reputation > 100 AND (SELECT COUNT(*) FROM Badges b WHERE b.UserId = u.Id AND b.Class = 1) > 0 ORDER BY ul.TotalVotes DESC, pp.ViewCount DESC LIMIT 50;
//
// RECURSIVE, but no CTE refers to itself. `ON pp.Rank <= 10` names only TopPosts, so it is a cross join. The distinct link type names are joined in
// name order.
fn q34356(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, view_count, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).with(score.gt(0)).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap())), 10);
    let tp = post_set(&top, |x| x.0);
    let pla = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).select((&db.post_link.link_type).select(&db.link_type.name)).opt()).buf_fold(|v| {
        let n = v.iter().filter(|x| x.is_some()).count() as i64;
        let mut t: Vec<Str> = v.into_iter().flatten().collect();
        t.sort_unstable();
        t.dedup();
        (n, if t.is_empty() { None } else { Some(&*Box::leak(t.join(", ").into_boxed_str())) })
    });
    let ul = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let us = drain(db.user.with((&db.user.reputation).gt(100)).with(gold).select(&ul));
    let ps = drain((&tp).select(Ident::<Post>::new().and(&pla)));
    let v = cross_top(us, |&(_, a)| Reverse(a[0]), ps, |&(_, (p, _))| Reverse(view_count.get(p)), 50);
    rows(v.into_iter().map(|((u, a), (_, (p, (n, t))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["id", "title", "views", "created"]));
        f.extend([V::I(n), ostr(t)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostInfo AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COALESCE(A.AcceptedAnswerId, 0) AS AcceptedAnswerId, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount
//     FROM Posts P LEFT JOIN Posts A ON P.Id = A.AcceptedAnswerId WHERE P.PostTypeId = 1),
// PopularUsers AS (SELECT U.Id AS UserId, U.DisplayName, RANK() OVER (ORDER BY SUM(V.BountyAmount) DESC) AS RankByBounties, SUM(V.BountyAmount) AS TotalBounties
//     FROM Users U JOIN Votes V ON U.Id = V.UserId WHERE V.VoteTypeId IN (8, 9) GROUP BY U.Id, U.DisplayName)
// SELECT U.DisplayName AS UserName, U.BadgeCount, PI.Title, PI.Score, PI.ViewCount, PI.CommentCount, PU.RankByBounties, PU.TotalBounties,
//        CASE WHEN PI.AcceptedAnswerId != 0 THEN 'Has Accepted Answer' ELSE 'No Accepted Answer' END AS AnswerStatus
// FROM UserBadges U JOIN PostInfo PI ON U.UserId = PI.PostId LEFT JOIN PopularUsers PU ON U.UserId = PU.UserId WHERE U.BadgeCount > 0
// ORDER BY PU.RankByBounties, U.BadgeCount DESC, PI.Score DESC;
//
// `U.UserId = PI.PostId` joins a user id to a post id, so it goes through the raw ids. BadgeNames is never read.
fn q1933(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let acc_by: HashIdx<Id<Post>, Id<Post>> = accepted_answer.inv().collect();
    let cc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pu = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt()))
        .fold(None::<i64>, |s, b| match (s, b) {
            (s, None) => s,
            (None, Some(b)) => Some(b),
            (Some(s), Some(b)) => Some(s + b),
        });
    let pr = ranked(drain(&pu), |&(_, s)| (s.is_none(), Reverse(s)), false);
    let pr = rel(pr.into_iter().map(|((u, s), k)| (u, (s, k))).collect());
    let pri: HashIdx<Id<User>, (Option<i64>, i64)> = (&pr).map(|(u, _)| u).inv().select((&pr).map(|(_, x)| x)).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain((&ub).filt(|n| n > 0).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&cc).and((&acc_by).opt()))).and((&pri).opt()));
    rows(v.into_iter().map(|(u, ((b, ((p, c), a)), r))| {
        let mut f = ucols(db, u, &["name"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(V::I(c));
        match r {
            Some((s, k)) => f.extend([V::I(k), oint(s)]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(V::S(if a.is_some() && db.post.origid.get(p).unwrap() != 0 { "Has Accepted Answer" } else { "No Accepted Answer" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS OwnerDisplayName, P.CreationDate, P.Score, P.ViewCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount, RANK() OVER (ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, U.DisplayName, P.CreationDate, P.Score, P.ViewCount),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, RP.CreationDate, RP.Score, RP.ViewCount, RP.CommentCount, RP.AnswerCount FROM RankedPosts RP WHERE RP.PostRank <= 10)
// SELECT T.Title, T.OwnerDisplayName, T.CreationDate, T.Score, T.ViewCount, T.CommentCount, T.AnswerCount,
//        (SELECT STRING_AGG(CONCAT(U.DisplayName, ': ', V.CreationDate), ', ') FROM Votes V JOIN Users U ON V.UserId = U.Id WHERE V.PostId = T.PostId AND V.VoteTypeId IN (2, 3)) AS VoterInfo
// FROM TopPosts T ORDER BY T.Score DESC, T.ViewCount DESC;
//
// PostRank reads only CreationDate, so the questions are picked first. The voter list is built in vote id order.
fn q9908(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let Vote { vote_type_id, user, creation_date: vd, .. } = &db.vote;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(post_type_id)), |_| 0, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 10, true);
    let tp = post_set(&top, |x| x.0);
    let ca = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(answers_of(db).opt())).fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let vi = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([2, 3])).and(vd).and(user.select(&db.user.display_name)))).buf_fold(|v| {
        let mut v: Vec<_> = v.into_iter().collect();
        v.sort_by_key(|x| x.0 .0);
        let s: Vec<String> = v.iter().map(|x| format!("{}: {}", x.1, ts_text(x.0 .1))).collect();
        &*Box::leak(s.join(", ").into_boxed_str())
    });
    rows(drain((&ca).and((&vi).opt())).into_iter().map(|(p, (a, s))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), ostr(s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.AnswerCount, u.DisplayName AS AuthorName,
//        lag(p.Score) OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate) AS PrevScore, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, '; ') AS Comments FROM Comments c GROUP BY c.PostId),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6, 24) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.AuthorName, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        COALESCE(pc.Comments, 'No comments') AS Comments, COALESCE(phs.EditCount, 0) AS EditCount, rp.Rank
// FROM RankedPosts rp LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId WHERE rp.Rank <= 10
// ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// PrevScore is never read. The comment texts are joined in comment id order.
fn q8204(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let top = top_n(
        drain(db.post.with(post_type_id.eq(1)).with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user)),
        |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())),
        10,
    );
    let tr = rel(top.into_iter().enumerate().map(|(i, (p, u))| (p, (u, i as i64 + 1))).collect());
    let tp: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().and(&db.comment.text))).buf_fold(|v| {
        let mut v: Vec<_> = v.into_iter().collect();
        v.sort_by_key(|x| x.0);
        let t: Vec<Str> = v.iter().map(|x| x.1).collect();
        (v.len() as i64, &*Box::leak(t.join("; ").into_boxed_str()))
    });
    let phs = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6, 24])))).fold(0i64, |n, _| n + 1);
    type R = (Id<Post>, (Id<User>, i64));
    let v = drain((&tr).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&pc).opt().and((&phs).opt())))));
    rows(v.into_iter().map(|(_, ((p, (u, k)), (c, h)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "views", "answers"]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(c.map_or(0, |c| c.0)), V::S(c.map_or("No comments", |c| c.1)), V::I(h.unwrap_or(0)), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS Author, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount, P.CommentCount,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.ViewCount DESC, P.Score DESC) AS RankByPopularity FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= '2023-01-01'),
// TopPosts AS (SELECT PostId, Title, Author, CreationDate, ViewCount, Score, AnswerCount, CommentCount FROM RankedPosts WHERE RankByPopularity <= 10),
// PostDetails AS (SELECT TP.*, ARRAY_AGG(DISTINCT T.TagName) AS Tags, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties
//     FROM TopPosts TP LEFT JOIN Posts P ON TP.PostId = P.Id LEFT JOIN PostLinks PL ON P.Id = PL.PostId LEFT JOIN Tags T ON PL.RelatedPostId = T.Id
//     LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
//     GROUP BY TP.PostId, TP.Title, TP.Author, TP.CreationDate, TP.ViewCount, TP.Score, TP.AnswerCount, TP.CommentCount)
// SELECT PD.PostId, PD.Title, PD.Author, PD.CreationDate, PD.ViewCount, PD.Score, PD.AnswerCount, PD.CommentCount, PD.Tags, PD.TotalBounties
// FROM PostDetails PD ORDER BY PD.ViewCount DESC, PD.Score DESC;
//
// `PL.RelatedPostId = T.Id` joins a post id to a tag id, so it goes through the raw ids. The distinct tag names are listed in name order, NULL last.
fn q5691(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, view_count, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(date(2023, 1, 1))).with(owner_user).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(view_count.get(p)), Reverse(score.get(p).unwrap())), 10, true);
    let tp = post_set(&top, |x| x.0);
    let tidx: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let pd = (&tp)
        .group_by(Ident::<Post>::new())
        .select(links_of(db).select((&db.post_link.related_post_id).select(&tidx).select(&db.tag.tag_name).opt()).opt().and(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt()).opt()))
        .buf_fold(|v| {
            let mut t: Vec<(bool, Option<Str>)> = v.iter().map(|x| x.0.flatten()).map(|t| (t.is_none(), t)).collect();
            t.sort_unstable();
            t.dedup();
            let t: Vec<Option<Str>> = t.into_iter().map(|x| x.1).collect();
            (&*Box::leak(t.into_boxed_slice()), v.iter().map(|x| x.1.flatten().unwrap_or(0)).sum::<i64>())
        });
    rows(drain((&pd).and(owner_user)).into_iter().map(|(p, ((t, b), u))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name"]));
        f.extend(post_fields(db, p, &["created", "views", "score", "answers", "comments"]));
        f.extend([V::L(t.iter().map(|&x| ostr(x)).collect()), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Owner, u.Reputation AS OwnerReputation, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PopularTags AS (SELECT unnest(string_to_array(Tags, ',')) AS TagName FROM Posts WHERE PostTypeId = 1),
// TopTags AS (SELECT TagName, COUNT(*) AS TagCount FROM PopularTags GROUP BY TagName ORDER BY TagCount DESC LIMIT 10),
// PostWithTopTags AS (SELECT rp.PostId, rp.Title, rp.Owner, rp.OwnerReputation, rp.CreationDate, rt.TagName FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id
//     JOIN TopTags rt ON rt.TagName = ANY (string_to_array(p.Tags, ',')))
// SELECT pwap.*, COUNT(c.Id) AS CommentCount, AVG(v.BountyAmount) AS AvgBounty FROM PostWithTopTags pwap LEFT JOIN Comments c ON pwap.PostId = c.PostId
// LEFT JOIN Votes v ON pwap.PostId = v.PostId AND v.VoteTypeId = 8 GROUP BY pwap.PostId, pwap.Title, pwap.Owner, pwap.OwnerReputation, pwap.CreationDate, pwap.TagName
// ORDER BY pwap.CreationDate DESC;
//
// A tag list has no ',', so each list is one element, the whole string. Rank is never read.
fn q6451(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, tags_str, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let split = || tags_str.flat_map(|t: Str| t.split(','));
    let tc = db.post.with(post_type_id.eq(1)).group_by(split()).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = top_n(drain(&tc), |&(_, n)| Reverse(n), 10);
    let tt: MatSet<Str> = rel(top).map(|(t, _)| t).collect();
    let rp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let agg = rp.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt()).opt())).fold([0i64; 3], |a, (c, b)| {
        let b = b.flatten();
        [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
    });
    rows(drain((&agg).and(owner_user).and(split().with(&tt))).into_iter().map(|(p, ((a, u), t))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["created"]));
        f.extend([V::S(t), V::I(a[0]), avg(a[2], a[1])]);
        row(f)
    }))
}

// WITH TagCounts AS (SELECT UNNEST(string_to_array(substring(Tags, 2, length(Tags)-2), '><')) AS Tag, Id AS PostId FROM Posts WHERE PostTypeId = 1),
// QuestionVotes AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// QuestionDetails AS (SELECT p.Id AS QuestionId, p.Title, p.CreationDate, p.Score, tc.Tag, qv.UpVotes, qv.DownVotes, COUNT(c.Id) AS CommentCount
//     FROM Posts p JOIN TagCounts tc ON p.Id = tc.PostId LEFT JOIN QuestionVotes qv ON p.Id = qv.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, tc.Tag, qv.UpVotes, qv.DownVotes),
// Ranking AS (SELECT QuestionId, Title, CreationDate, Score, Tag, UpVotes, DownVotes, CommentCount,
//        RANK() OVER (PARTITION BY Tag ORDER BY (UpVotes - DownVotes) DESC, Score DESC) AS RankWithinTag FROM QuestionDetails)
// SELECT r.QuestionId, r.Title, r.CreationDate, r.Score, r.Tag, r.UpVotes, r.DownVotes, r.CommentCount, r.RankWithinTag FROM Ranking r WHERE r.RankWithinTag <= 5
// ORDER BY r.Tag, r.RankWithinTag;
//
// A question with no votes has no QuestionVotes row, so its difference is NULL and sorts last.
fn q29705(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, .. } = &db.post;
    let qv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain(db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list).and((&qv).opt()).and(&cc)));
    let v = ranked(v, |&(p, ((t, q), _))| (t, q.is_none(), Reverse(q.map(|q| q[0] - q[1])), Reverse(score.get(p).unwrap())), false);
    let v = per_group(v, |&(_, ((t, _), _))| t);
    type R = ((Id<Post>, ((Str, Option<[i64; 2]>), i64)), i64);
    rows(drain(rel(v).filt(|x: R| x.1 <= 5)).into_iter().map(|(_, ((p, ((t, q), c)), k))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::S(t), oint(q.map(|q| q[0])), oint(q.map(|q| q[1])), V::I(c), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPostVotes AS (SELECT p.Id AS PostId, p.Title, COUNT(v.Id) AS VoteCount, ROW_NUMBER() OVER(PARTITION BY p.Id ORDER BY COUNT(v.Id) DESC) AS VoteRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(p.Id) > 10),
// CommentsWithKeywords AS (SELECT c.PostId, c.Text, c.CreationDate, SUM(CASE WHEN LOWER(c.Text) LIKE '%help%' THEN 1 ELSE 0 END) AS HelpKeywordCount,
//        SUM(CASE WHEN LOWER(c.Text) LIKE '%issue%' THEN 1 ELSE 0 END) AS IssueKeywordCount FROM Comments c GROUP BY c.PostId, c.Text, c.CreationDate)
// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, pv.VoteCount, tt.TagName, cwk.Text AS CommentText, cwk.HelpKeywordCount, cwk.IssueKeywordCount
// FROM Posts p JOIN RankedPostVotes pv ON p.Id = pv.PostId JOIN PostLinks pl ON p.Id = pl.PostId JOIN PopularTags tt ON tt.TagName IN (SELECT UNNEST(string_to_array(p.Tags, '><')))
// LEFT JOIN CommentsWithKeywords cwk ON cwk.PostId = p.Id WHERE pv.VoteRank = 1 ORDER BY pv.VoteCount DESC, p.CreationDate DESC;
//
// VoteRank partitions by the post, so it is always 1. The tags are split on '><' without stripping the outer brackets, so a tag name can only equal
// an element that is neither first nor last in the list (or the whole list).
fn q28394(db: &'static So) -> String {
    let Post { tags_str, .. } = &db.post;
    let Comment { post, text, creation_date: cd, .. } = &db.comment;
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let ts_ = tag_stats(db);
    let pop: HashIdx<Str, Id<Tag>> = db.tag.with((&ts_).filt(|a| a[0] > 10)).select(&db.tag.tag_name).inv().collect();
    let cw = db.comment.group_by(post.and(text).and(cd)).select(text).fold([0i64; 2], |a, t| {
        let l = t.to_lowercase();
        [a[0] + l.contains("help") as i64, a[1] + l.contains("issue") as i64]
    });
    let cr = rel(drain(&cw));
    let cwi: HashIdx<Id<Post>, (((Id<Post>, Str), i64), [i64; 2])> = (&cr).map(|(((p, _), _), _)| p).inv().select(&cr).collect();
    let v = drain(db.post.select(Ident::<Post>::new().and(&vc).and(links_of(db)).and(tags_str.flat_map(|t: Str| t.split("><")).select(&pop)).and((&cwi).opt())));
    rows(v.into_iter().map(|(_, ((((p, n), _), t), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(n), V::S(db.tag.tag_name.get(t).unwrap())]);
        match c {
            Some((((_, s), _), a)) => f.extend([V::S(s), V::I(a[0]), V::I(a[1])]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, p.OwnerUserId, p.Tags, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COUNT(DISTINCT p.Id) AS TotalQuestions, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.Views, us.TotalQuestions, us.GoldBadges, us.SilverBadges, us.BronzeBadges, rp.PostId, rp.Title AS RecentPostTitle,
//        rp.CreationDate AS RecentPostDate, rp.Tags, STRING_AGG(rp.Body, ' ') AS CombinedBody
// FROM UserStats us LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.PostRank = 1
// GROUP BY us.UserId, us.DisplayName, us.Reputation, us.Views, us.TotalQuestions, us.GoldBadges, us.SilverBadges, us.BronzeBadges, rp.PostId, rp.Title, rp.CreationDate, rp.Tags
// ORDER BY us.Reputation DESC, us.Views DESC;
//
// PostRank = 1 is every question at the owner's latest question date. Each group holds one post, so CombinedBody is that post's body.
fn q29302(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let upb = user_posts_badges(db);
    let dp = user_distinct_posts(db);
    let q = || db.post.with(post_type_id.eq(1));
    let latest = q().group_by(owner_user).select(creation_date).fold(i64::MIN, |m, d| m.max(d));
    let r1: MatSet<Id<Post>> = q().with(creation_date.and(owner_user.select(&latest)).filt(|(d, m): (i64, i64)| d == m)).collect();
    let v = drain((&dp).and(&upb).and(posts_of(db).with(&r1).opt()));
    rows(v.into_iter().map(|(u, ((n, a), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend([V::I(n), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "created", "tags", "body"])),
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankInType
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(v.BountyAmount) AS TotalBounties, COUNT(DISTINCT p.Id) AS TotalPosts, AVG(u.Reputation) AS AvgReputation
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount, STRING_AGG(ct.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes ct ON CAST(ph.Comment AS INTEGER) = ct.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, tu.TotalBounties, tu.TotalPosts, tu.AvgReputation, cp.CloseCount, cp.CloseReasons
// FROM RankedPosts rp LEFT JOIN TopUsers tu ON rp.PostId = tu.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.RankInType <= 10
// ORDER BY rp.Score DESC, tu.TotalBounties DESC NULLS LAST;
//
// RankInType reads only Score, so the posts are picked first. `rp.PostId = tu.UserId` joins a post id to a user id, so it goes through the raw ids,
// and TopUsers is only built for the users those ids name. The close reasons are joined in history id order.
fn q4306(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp = post_set(&top, |x| x.0);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let cand: MatSet<Id<User>> = (&tp).select(&db.post.origid).select(&uidx).collect();
    let tu = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .buf_fold(|v| {
            let b: Vec<i64> = v.iter().filter_map(|x| x.1.flatten()).collect();
            (if b.is_empty() { None } else { Some(b.iter().sum::<i64>()) }, distinct_some(v.iter().map(|x| Some(x.0))))
        });
    let crt: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    let cp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11])).and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt))))
        .buf_fold(|v| {
            let mut v: Vec<_> = v.into_iter().collect();
            v.sort_by_key(|x| x.0);
            let n: Vec<Str> = v.iter().map(|x| db.close_reason_type.name.get(x.1).unwrap()).collect();
            (v.len() as i64, &*Box::leak(n.join(", ").into_boxed_str()))
        });
    let v = drain((&tp).select(Ident::<Post>::new().and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and((&tu).filt(|a| a.1 > 5))).opt()).and((&cp).opt())));
    rows(v.into_iter().map(|(_, ((p, u), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers"]);
        match u {
            Some((u, (b, n))) => f.extend([oint(b), V::I(n), V::F(db.user.reputation.get(u).unwrap() as f64)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        match c {
            Some((n, s)) => f.extend([V::I(n), V::S(s)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagStatistics AS (SELECT Tag, COUNT(*) AS QuestionCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM PostTags pt JOIN Posts p ON pt.PostId = p.Id GROUP BY Tag),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAsked, SUM(p.ViewCount) AS TotalViews, SUM(p.AnswerCount) AS TotalAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName),
// TopTags AS (SELECT Tag, QuestionCount, TotalViews, TotalScore, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM TagStatistics),
// TopUsers AS (SELECT UserId, DisplayName, QuestionsAsked, TotalViews, TotalAnswers, ROW_NUMBER() OVER (ORDER BY TotalViews DESC) AS Rank FROM UserEngagement)
// SELECT tt.Tag, tt.QuestionCount, tt.TotalViews, tt.TotalScore, tu.DisplayName AS TopUser, tu.QuestionsAsked, tu.TotalViews AS UserTotalViews, tu.TotalAnswers
// FROM TopTags tt JOIN TopUsers tu ON tt.Rank = tu.Rank WHERE tt.Rank <= 5;
//
// The WHERE on p.PostTypeId turns the LEFT JOIN into an inner join. TotalViews DESC puts NULL sums last.
fn q26374(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, score, answer_count, .. } = &db.post;
    let q = || db.post.with(post_type_id.eq(1));
    let ts_ = q().group_by(tags_str.flat_map(tag_list)).select(view_count.opt().and(score)).fold([0i64; 4], |a, (v, s)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s]);
    let tt = top_n(drain(&ts_), |&(_, a)| Reverse(a[3]), 5);
    let ue = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(view_count.opt().and(answer_count.opt()))).fold([0i64; 5], |a, (v, n)| {
        [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + n.is_some() as i64, a[4] + n.unwrap_or(0)]
    });
    let tu = top_n(drain(&ue), |&(_, a)| (a[1] == 0, Reverse(a[2])), 5);
    let a = rel(tt.into_iter().enumerate().map(|(i, x)| (i, x)).collect());
    let b = rel(tu.into_iter().enumerate().map(|(i, x)| (i, x)).collect());
    let bi: HashIdx<usize, (usize, (Id<User>, [i64; 5]))> = (&b).map(|(i, _)| i).inv().select(&b).collect();
    rows(drain((&a).select(Same::<(usize, (Str, [i64; 4]))>::new().and(Same::<(usize, (Str, [i64; 4]))>::new().map(|x: (usize, (Str, [i64; 4]))| x.0).select(&bi)))).into_iter().map(|(_, ((_, (t, a)), (_, (u, b))))| {
        let mut f = vec![V::S(t), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3])];
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(b[0]), nullable(b[2], b[1]), nullable(b[4], b[3])]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ') AS Badges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// UserPostDetails AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.QuestionCount, 0) AS QuestionCount, COALESCE(PS.AnswerCount, 0) AS AnswerCount,
//        COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.AverageScore, 0) AS AverageScore FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId),
// RankedUsers AS (SELECT U.*, RANK() OVER (ORDER BY TotalViews DESC, AverageScore DESC) AS Rank FROM UserPostDetails U WHERE QuestionCount > 0)
// SELECT R.UserId, R.DisplayName, R.BadgeCount, R.QuestionCount, R.AnswerCount, R.TotalViews, R.AverageScore,
//        CASE WHEN R.BadgeCount > 5 THEN 'Expert' WHEN R.BadgeCount BETWEEN 1 AND 5 THEN 'Novice' ELSE 'No Badges' END AS UserLevel
// FROM RankedUsers R WHERE R.Rank <= 10 ORDER BY R.Rank;
//
// Badges is never read.
fn q617(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, v), s)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + v.unwrap_or(0), a[3] + s, a[4] + 1]);
    let v = drain((&ub).and((&ps).filt(|a| a[0] > 0)));
    let v = ranked(v, |&(_, (_, a))| (Reverse(a[2]), Reverse(fkey(a[3] as f64 / a[4] as f64))), false);
    type R = ((Id<User>, (i64, [i64; 5])), i64);
    rows(drain(rel(v).filt(|x: R| x.1 <= 10)).into_iter().map(|(_, ((u, (b, a)), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4])]);
        f.push(V::S(if b > 5 { "Expert" } else if b >= 1 { "Novice" } else { "No Badges" }));
        row(f)
    }))
}

// WITH TagCounts AS (SELECT UNNEST(STRING_TO_ARRAY(SUBSTRING(Tags FROM 2 FOR LENGTH(Tags) - 2), '><')) AS Tag, Id AS PostId FROM Posts WHERE PostTypeId = 1),
// TagStatistics AS (SELECT Tag, COUNT(DISTINCT PostId) AS PostCount, COUNT(*) AS TotalOccurrences FROM TagCounts GROUP BY Tag),
// UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(b.Class) AS TotalBadgeClass
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges b ON U.Id = b.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopTags AS (SELECT Tag, PostCount, TotalOccurrences, ROW_NUMBER() OVER (ORDER BY TotalOccurrences DESC) AS TagRank FROM TagStatistics WHERE TotalOccurrences > 1),
// PopularUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalBadgeClass, ROW_NUMBER() OVER (ORDER BY PostCount DESC, Reputation DESC) AS UserRank FROM UserReputation)
// SELECT T.Tag, T.PostCount, T.TotalOccurrences, U.DisplayName AS TopUser, U.Reputation AS UserReputation, U.TotalBadgeClass AS UserTotalBadgeClass
// FROM TopTags T JOIN PopularUsers U ON U.PostCount > 1 WHERE T.TagRank <= 10 ORDER BY T.TotalOccurrences DESC;
//
// `ON U.PostCount > 1` names only PopularUsers, so it is a cross join. UserRank is never read.
fn q29365(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let ts_ = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).buf_fold(|v| (distinct_some(v.iter().map(|&p| Some(p))), v.len() as i64));
    let tt = top_n(drain((&ts_).filt(|a| a.1 > 1)), |&(_, a)| Reverse(a.1), 10);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold((0i64, 0i64), |(n, s), (c, _)| match c {
        Some(c) => (n + 1, s + c),
        None => (n, s),
    });
    let dp = user_distinct_posts(db);
    let us = drain((&dp).filt(|n| n > 1).and(&bc));
    let a = rel(tt);
    let b = rel(us);
    rows(drain((&a).cross(&b)).into_iter().map(|(_, ((t, (n, o)), (u, (_, (k, s)))))| {
        let mut f = vec![V::S(t), V::I(n), V::I(o)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(nullable(s, k));
        row(f)
    }))
}

// WITH TagAggregates AS (SELECT Tags.TagName, COUNT(DISTINCT Posts.Id) AS PostCount, SUM(Posts.ViewCount) AS TotalViews, AVG(Posts.Score) AS AvgScore,
//        STRING_AGG(DISTINCT Users.DisplayName, ', ') AS Contributors
//     FROM Tags JOIN Posts ON Posts.Tags LIKE CONCAT('%<', Tags.TagName, '>%') JOIN Users ON Posts.OwnerUserId = Users.Id
//     WHERE Posts.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY Tags.TagName),
// TopTags AS (SELECT TagName, PostCount, TotalViews, AvgScore, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM TagAggregates WHERE PostCount > 0),
// TopContributors AS (SELECT Users.DisplayName, COUNT(DISTINCT Posts.Id) AS ContributedPosts, SUM(Posts.ViewCount) AS TotalViews FROM Users JOIN Posts ON Posts.OwnerUserId = Users.Id
//     WHERE Posts.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY Users.DisplayName ORDER BY ContributedPosts DESC LIMIT 10)
// SELECT t.TagName, t.PostCount, t.TotalViews, t.AvgScore, c.DisplayName AS TopContributor, c.ContributedPosts, c.TotalViews AS ContributorTotalViews
// FROM TopTags t LEFT JOIN TopContributors c ON c.ContributedPosts = (SELECT MAX(ContributedPosts) FROM TopContributors) WHERE t.Rank <= 10 ORDER BY t.PostCount DESC;
//
// A tag name has no '<' or '>', so `LIKE '%<name>%'` holds exactly when the name is one of the post's tags. Contributors is never read. The join
// condition names only TopContributors, so every top tag meets the contributors with the largest count.
fn q29909(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, view_count, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(owner_user);
    let by_name: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let ta = recent().group_by(tags_str.flat_map(tag_list).select(&by_name)).select(view_count.opt().and(score)).fold([0i64; 4], |a, (v, s)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s]);
    let tt = top_n(drain(&ta), |&(_, a)| Reverse(a[0]), 10);
    let tc = recent().group_by(owner_user.select(&db.user.display_name)).select(view_count.opt()).fold([0i64; 3], |a, v| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0)]);
    let tc = top_n(drain(&tc), |&(_, a)| Reverse(a[0]), 10);
    let mx = tc.iter().map(|x| x.1[0]).max().unwrap_or(0);
    let cr = rel(tc);
    type C = (Str, [i64; 3]);
    let best: Vec<Option<C>> = left_all(drain((&cr).filt(|x: C| x.1[0] == mx)).into_iter().map(|x| x.1).collect()).v;
    let a = rel(tt);
    let b = rel(best);
    rows(drain((&a).cross(&b)).into_iter().map(|(_, ((t, a), c))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0])];
        match c {
            Some((n, c)) => f.extend([V::S(n), V::I(c[0]), nullable(c[2], c[1])]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS (SELECT Id, Reputation, CreationDate, 1 AS Level FROM Users WHERE Reputation > 1000
//     UNION ALL SELECT u.Id, u.Reputation, u.CreationDate, ur.Level + 1 FROM Users u INNER JOIN UserReputationCTE ur ON u.Reputation > ur.Reputation WHERE ur.Level < 3),
// MostActiveUsers AS (SELECT u.Id, u.DisplayName, COUNT(c.Id) AS CommentCount, SUM(p.ViewCount) AS TotalViews FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId
//     LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 500 GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT p.Id, p.Title, p.Score, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT u.DisplayName AS UserName, u.Reputation AS UserReputation, COALESCE(ua.CommentCount, 0) AS TotalComments, COALESCE(ua.TotalViews, 0) AS TotalViews, tp.Title AS TopPostTitle,
//        tp.Score AS TopPostScore
// FROM Users u LEFT JOIN MostActiveUsers ua ON u.Id = ua.Id LEFT JOIN TopPosts tp ON u.Id = tp.Id
// WHERE u.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND (u.Location IS NOT NULL OR u.WebsiteUrl IS NOT NULL)
// ORDER BY UserReputation DESC, TotalViews DESC LIMIT 10;
//
// UserReputationCTE is never read. The order leads with Reputation, so the comment x post product is driven only for the users reaching the tenth
// reputation. `u.Id = tp.Id` joins a user id to a post id, so it goes through the raw ids.
fn q33944(db: &'static So) -> String {
    let User { creation_date, location, website_url, reputation, .. } = &db.user;
    let Post { creation_date: pd, view_count, .. } = &db.post;
    let base = || db.user.with(creation_date.lt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(location.or(website_url));
    let rs = top_n(drain(base().select(reputation)), |&(_, r)| Reverse(r), 10);
    let cut = rs.last().map_or(i64::MIN, |x| x.1);
    let cand: MatSet<Id<User>> = base().with(reputation.ge(cut)).collect();
    let ua = (&cand)
        .with(reputation.gt(500))
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(posts_of(db).select(view_count.opt()).opt()))
        .fold([0i64; 3], |a, (c, v)| {
            let v = v.flatten();
            [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0)]
        });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let tp = (&db.user.origid).select(&pidx).select(Ident::<Post>::new().with(pd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let v = drain((&cand).select(Ident::<User>::new().and((&ua).opt()).and(tp.opt())));
    let v = top_n(v, |&(_, ((u, a), _))| (Reverse(reputation.get(u).unwrap()), Reverse(a.map_or(0, |a| a[2]))), 10);
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[2])]);
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "score"])),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// PostDetails AS (SELECT R.PostId, R.Title, U.DisplayName AS OwnerName, R.CreationDate, UB.BadgeCount FROM RecentPosts R JOIN Users U ON R.OwnerUserId = U.Id
//     JOIN UserBadges UB ON U.Id = UB.UserId WHERE R.rn = 1),
// PostVoteDetails AS (SELECT PD.PostId, PD.Title, PD.OwnerName, PD.CreationDate, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM PostDetails PD LEFT JOIN Votes V ON PD.PostId = V.PostId GROUP BY PD.PostId, PD.Title, PD.OwnerName, PD.CreationDate)
// SELECT PDT.PostId, PDT.Title, PDT.OwnerName, PDT.CreationDate, PDT.UpVotes, PDT.DownVotes,
//        CASE WHEN PDT.UpVotes > PDT.DownVotes THEN 'Positive' WHEN PDT.UpVotes < PDT.DownVotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment,
//        (SELECT STRING_AGG(Name, ', ') FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id WHERE PH.PostId = PDT.PostId) AS HistoryTypes
// FROM PostVoteDetails PDT ORDER BY PDT.CreationDate DESC LIMIT 50;
//
// The history type names are joined in history id order. A tie on CreationDate inside rn goes to the smaller id.
fn q8655(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let r1 = top_per(drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let top = top_n(r1, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 50);
    let tp = post_set(&top, |x| x.0);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ht = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().and(htype_name(db)))).buf_fold(|v| {
        let mut v: Vec<_> = v.into_iter().collect();
        v.sort_by_key(|x| x.0);
        let n: Vec<Str> = v.iter().map(|x| x.1).collect();
        &*Box::leak(n.join(", ").into_boxed_str())
    });
    rows(drain((&vc).and(owner_user).and((&ht).opt())).into_iter().map(|(p, ((a, u), h))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name"]));
        f.extend(post_fields(db, p, &["created"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }), ostr(h)]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS (SELECT Id, Reputation, CreationDate, UpVotes, DownVotes, (UpVotes - DownVotes) AS NetVotes FROM Users WHERE Reputation > 0
//     UNION ALL SELECT u.Id, u.Reputation, u.CreationDate, u.UpVotes, u.DownVotes, (u.UpVotes - u.DownVotes) AS NetVotes FROM Users u INNER JOIN UserReputationCTE ur ON ur.Id = u.Id
//     WHERE u.Reputation > ur.Reputation),
// RecentPostCTE AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, DENSE_RANK() OVER (PARTITION BY p.ParentId ORDER BY p.CreationDate DESC) AS LatestPostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 weeks'),
// PostHistoryCount AS (SELECT PostId, COUNT(*) AS HistoryCount FROM PostHistory WHERE PostHistoryTypeId IN (10, 12, 6) GROUP BY PostId),
// UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId)
// SELECT u.DisplayName AS UserName, u.Reputation, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(rp.Score, 0) AS PostScore, COALESCE(ph.HistoryCount, 0) AS PostHistoryChanges,
//        COUNT(DISTINCT rp.Id) AS RecentPostsCount, SUM(rp.ViewCount) AS TotalViews, SUM(rp.Score) AS TotalPostScore
// FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN RecentPostCTE rp ON u.Id = rp.OwnerUserId LEFT JOIN PostHistoryCount ph ON rp.Id = ph.PostId
// WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users) GROUP BY u.Id, u.DisplayName, u.Reputation, ub.BadgeCount, rp.Score, ph.HistoryCount ORDER BY UserName LIMIT 100;
//
// UserReputationCTE is never read, and neither is LatestPostRank. The groups are (user, recent post score, history count), a user with no recent
// post keeping one group of NULLs.
fn q33474(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let rep = &db.user.reputation;
    let (s, n) = db.user.select(rep).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let a = s as f64 / n as f64;
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let phc = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 12, 6])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rp = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -14))).and(score).and((&phc).opt()));
    type R = (Id<User>, Option<((Id<Post>, i64), Option<i64>)>);
    let rs = rel(drain(db.user.with(rep.filt(|r| r as f64 > a)).select(Ident::<User>::new().and(rp.opt()))).into_iter().map(|x| x.1).collect());
    let g = (&rs)
        .group_by(Same::<R>::new().map(|x: R| (x.0, x.1.map(|y| y.0 .1), x.1.and_then(|y| y.1))))
        .select(Same::<R>::new().map(|x: R| x.1.map(|y| y.0)).and(Same::<R>::new().map(|x: R| x.1.map(|y| y.0 .0)).flat_map(|p: Option<Id<Post>>| p).select(view_count).opt()))
        .buf_fold(|v| {
            let n = distinct_some(v.iter().map(|x| x.0.map(|y| y.0)));
            let vs: Vec<i64> = v.iter().filter_map(|x| x.1).collect();
            let ss: Vec<i64> = v.iter().filter_map(|x| x.0.map(|y| y.1)).collect();
            (n, if vs.is_empty() { None } else { Some(vs.iter().sum::<i64>()) }, if ss.is_empty() { None } else { Some(ss.iter().sum::<i64>()) })
        });
    type K = (Id<User>, Option<i64>, Option<i64>);
    let v = drain((&g).and(Same::<K>::new().map(|k: K| k.0).select((&ub).opt())));
    let v = top_n(v, |&((u, _, _), _)| db.user.display_name.get(u).unwrap(), 100);
    rows(v.into_iter().map(|((u, s, h), ((n, vs, ss), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(s.unwrap_or(0)), V::I(h.unwrap_or(0)), V::I(n), oint(vs), oint(ss)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, CASE WHEN Reputation >= 1000 THEN 'High' WHEN Reputation >= 500 THEN 'Medium' ELSE 'Low' END AS ReputationCategory FROM Users),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ARRAY_AGG(pr.Name) AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes pr ON CAST(ph.Comment AS INTEGER) = pr.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.CreationDate)
// SELECT ps.PostId, ps.Title, ps.CreationDate, u.DisplayName AS OwnerDisplayName, ur.Reputation, ur.ReputationCategory, COALESCE(cp.CloseReasons, ARRAY[NULL]) AS CloseReasons,
//        ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount, ps.RecentPostRank
// FROM PostStatistics ps JOIN Users u ON ps.OwnerUserId = u.Id JOIN UserReputation ur ON u.Id = ur.Id LEFT JOIN ClosedPosts cp ON ps.PostId = cp.PostId
// WHERE ps.Score > 0 AND ps.RecentPostRank <= 5 ORDER BY ps.CreationDate DESC;
//
// RecentPostRank reads only CreationDate, so the posts are picked first and the comment x vote product is driven for them. The close reasons are
// listed in history id order.
fn q3208(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let PostHistory { post_history_type_id, comment, post, creation_date: hd, .. } = &db.post_history;
    let rk = per_group(ranked(drain(db.post.select(owner_user)), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap())), false), |&(_, u)| u);
    type K = ((Id<Post>, Id<User>), i64);
    let rk = rel(rk);
    let rki: HashIdx<Id<Post>, i64> = (&rk).filt(|x: K| x.1 <= 5).map(|x: K| x.0 .0).inv().select((&rk).map(|x: K| x.1)).collect();
    let tp: MatSet<Id<Post>> = db.post.with(score.gt(0)).with(&rki).collect();
    let ps = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let crt: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .with(post.with(&tp))
        .group_by(post.and(hd))
        .select(Ident::<PostHistory>::new().and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt)))
        .buf_fold(|v| {
            let mut v: Vec<_> = v.into_iter().collect();
            v.sort_by_key(|x| x.0);
            let n: Vec<Str> = v.iter().map(|x| db.close_reason_type.name.get(x.1).unwrap()).collect();
            &*Box::leak(n.into_boxed_slice())
        });
    let cr = rel(drain(&cp));
    let cpi: HashIdx<Id<Post>, &'static [Str]> = (&cr).map(|((p, _), _)| p).inv().select((&cr).map(|(_, s)| s)).collect();
    rows(drain((&ps).and(owner_user).and(&rki).and((&cpi).opt())).into_iter().map(|(p, (((a, u), k), c))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::S(if r >= 1000 { "High" } else if r >= 500 { "Medium" } else { "Low" }));
        f.push(match c {
            Some(c) => V::L(c.iter().map(|&x| V::S(x)).collect()),
            None => V::L(vec![V::Null]),
        });
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(k)]);
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Tags, u.DisplayName AS OwnerDisplayName, COALESCE(a.Id, -1) AS AcceptedAnswerId,
//        COALESCE(a.Title, 'No Accepted Answer') AS AcceptedAnswerTitle, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, STRING_AGG(DISTINCT pt.Name, ', ') AS PostTypeName,
//        STRING_AGG(DISTINCT t.TagName, ', ') AS TagsList
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN (SELECT Id, unnest(string_to_array(substring(Tags, 2, length(Tags)-2), '><')) AS TagName FROM Posts) t ON p.Id = t.Id
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Tags, u.DisplayName, a.Id, a.Title),
// AggregatePostStats AS (SELECT OwnerDisplayName, COUNT(PostId) AS TotalPosts, SUM(ViewCount) AS TotalViews, SUM(CommentCount) AS TotalComments, AVG(ViewCount) AS AvgViewCount
//     FROM PostDetails GROUP BY OwnerDisplayName ORDER BY TotalPosts DESC)
// SELECT aps.OwnerDisplayName, aps.TotalPosts, aps.TotalViews, aps.TotalComments, aps.AvgViewCount FROM AggregatePostStats aps WHERE aps.TotalPosts > 5 ORDER BY aps.TotalViews DESC;
//
// PostDetails has one row per post (a.Id is the post's own accepted answer). Only CommentCount of its aggregates is read, over the comment x vote x
// tag rows.
fn q25617(db: &'static So) -> String {
    let Post { owner_user, view_count, tags_str, .. } = &db.post;
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt()).and(tags_str.flat_map(tag_list).opt())).fold(0i64, |n, ((c, _), _)| n + c.is_some() as i64);
    let g = db.post.group_by(owner_user.select(&db.user.display_name).opt()).select(view_count.opt().and(&cc)).fold([0i64; 4], |a, (v, c)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + c]);
    rows(drain((&g).filt(|a| a[0] > 5)).into_iter().map(|(n, a)| row(vec![ostr(n), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), avg(a[2], a[1])])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COALESCE(SUM(b.Class), 0) AS TotalBadges, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostHistoryAnalysis AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS ChangeCount, MAX(ph.CreationDate) AS LastChangeDate, STRING_AGG(ph.Comment, '; ') AS ChangeComments
//     FROM PostHistory ph WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY ph.PostId, ph.PostHistoryTypeId)
// SELECT up.UserId, up.DisplayName, up.TotalPosts, up.TotalBadges, up.TotalBounties, rp.Title, rp.ViewCount, rp.Score, ph.ChangeCount, ph.LastChangeDate, ph.ChangeComments
// FROM UserActivity up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN PostHistoryAnalysis ph ON rp.Id = ph.PostId
// WHERE up.TotalPosts > 10 AND rp.ScoreRank <= 5 ORDER BY up.TotalPosts DESC, rp.Score DESC LIMIT 100;
//
// ScoreRank reads only Score, so the posts are picked first, and the post x badge x vote product is driven only for their owners. The comments are
// joined in history id order.
fn q30746(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let PostHistory { creation_date: hd, post_history_type_id, comment, post, .. } = &db.post_history;
    let lo = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let top = top_per(drain(db.post.with(creation_date.ge(lo)).select(post_type_id)), |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp = post_set(&top, |x| x.0);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, ((p, c), b)| [a[0] + p.is_some() as i64, a[1] + c.unwrap_or(0), a[2] + b.flatten().unwrap_or(0)]);
    let pha = db
        .post_history
        .with(hd.ge(lo))
        .with(post.with(&tp))
        .group_by(post.and(post_history_type_id))
        .select(Ident::<PostHistory>::new().and(hd).and(comment.opt()))
        .buf_fold(|v| {
            let mut v: Vec<_> = v.into_iter().collect();
            v.sort_by_key(|x| x.0 .0);
            let c: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            (v.len() as i64, v.iter().map(|x| x.0 .1).max().unwrap(), if c.is_empty() { None } else { Some(&*Box::leak(c.join("; ").into_boxed_str())) })
        });
    let pr = rel(drain(&pha));
    let phi: HashIdx<Id<Post>, (i64, i64, Option<Str>)> = (&pr).map(|((p, _), _)| p).inv().select((&pr).map(|(_, a)| a)).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and((&ua).filt(|a| a[0] > 10)))).and((&phi).opt())));
    let v = top_n(v, |&(_, ((p, (_, a)), _))| (Reverse(a[0]), Reverse(score.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(_, ((p, (u, a)), h))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        match h {
            Some((n, d, c)) => f.extend([V::I(n), V::T(d), ostr(c)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS (SELECT U.Id AS UserId, U.Reputation, U.CreationDate, U.LastAccessDate, 1 AS Level FROM Users U WHERE U.Reputation > 1000
//     UNION ALL SELECT U.Id, U.Reputation + 100, U.CreationDate, U.LastAccessDate, CTE.Level + 1 FROM Users U JOIN UserReputationCTE CTE ON CTE.UserId = U.Id WHERE CTE.Level < 10),
// PopularTags AS (SELECT T.TagName, COUNT(P.Id) AS PostCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName HAVING COUNT(P.Id) > 50)
// SELECT U.DisplayName, U.Reputation, COALESCE(B.BadgeCount, 0) AS BadgeCount, U.CreationDate, U.LastAccessDate,
//        (SELECT COUNT(*) FROM Posts WHERE OwnerUserId = U.Id AND PostTypeId = 1) AS QuestionCount, (SELECT COUNT(*) FROM Posts WHERE OwnerUserId = U.Id AND PostTypeId = 2) AS AnswerCount,
//        P.TagName, P.PostCount
// FROM Users U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON B.UserId = U.Id
// JOIN PopularTags P ON P.TagName IN (SELECT UNNEST(string_to_array(Tags, ',')) FROM Posts WHERE OwnerUserId = U.Id)
// WHERE U.Reputation > 1000 AND (U.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' OR U.Location IS NOT NULL)
// ORDER BY U.Reputation DESC, QuestionCount DESC LIMIT 100;
//
// RECURSIVE, but UserReputationCTE is never read. A tag list has no ',', so the IN compares the whole list with the tag name; each (user, tag) pair
// is kept once, as IN does.
fn q30531(db: &'static So) -> String {
    let User { reputation, last_access_date, location, .. } = &db.user;
    let Post { tags_str, post_type_id, .. } = &db.post;
    let ts_ = tag_stats(db);
    let pop: HashIdx<Str, Id<Tag>> = db.tag.with((&ts_).filt(|a| a[0] > 50)).select(&db.tag.tag_name).inv().collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let qa = db.post.group_by(&db.post.owner_user).select(post_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64]);
    let us = db.user.with(reputation.gt(1000)).with(last_access_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).or(location));
    let pairs = us.group_by(Ident::<User>::new().and(posts_of(db).select(tags_str.flat_map(|t: Str| t.split(','))).select(&pop))).select(Ident::<User>::new()).fold(0i64, |n, _| n + 1);
    type K = (Id<User>, Id<Tag>);
    let v = drain((&pairs).and(Same::<K>::new().map(|k: K| k.0).select((&bc).opt())).and(Same::<K>::new().map(|k: K| k.0).select((&qa).opt())).and(Same::<K>::new().map(|k: K| k.1).select(&ts_)));
    let v = top_n(v, |&((u, _), ((_, q), _))| (Reverse(reputation.get(u).unwrap()), Reverse(q.map_or(0, |q| q[0]))), 100);
    rows(v.into_iter().map(|((u, t), (((_, b), q), c))| {
        let q = q.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b.unwrap_or(0)));
        f.extend(ucols(db, u, &["ucreated", "last_access"]));
        f.extend([V::I(q[0]), V::I(q[1]), V::S(db.tag.tag_name.get(t).unwrap()), V::I(c[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS Owner, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName, p.CreationDate, p.Score),
// PopularTags AS (SELECT t.TagName, SUM(p.ViewCount) AS TotalViews, COUNT(DISTINCT p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
//     GROUP BY t.TagName ORDER BY TotalViews DESC LIMIT 10),
// PostHistoryInfo AS (SELECT ph.PostId, STRING_AGG(DISTINCT pht.Name, ', ') AS HistoryTypes, COUNT(ph.Id) AS EditCount FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
//     WHERE ph.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Owner, rp.CreationDate, rp.Score, rp.CommentCount, pht.HistoryTypes, pht.EditCount, pt.TagName AS PopularTag, pt.TotalViews, pt.PostCount
// FROM RankedPosts rp LEFT JOIN PostHistoryInfo pht ON rp.PostId = pht.PostId JOIN PopularTags pt ON pt.TagName = ANY(string_to_array(rp.Tags, ',')) WHERE rp.rn = 1
// ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// rn partitions by the post, so it is always 1. A tag list has no ',', so ANY compares the whole list with the tag name. The distinct history type
// names are joined in name order; the answer is empty on this data, so that is untested.
fn q26835(db: &'static So) -> String {
    let Post { tags_str, post_type_id, owner_user, .. } = &db.post;
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let ts_ = tag_stats(db);
    let top = top_n(drain(&ts_), |&(_, a)| (a[1] == 0, Reverse(a[2])), 10);
    let tr = rel(top);
    type T = (Id<Tag>, [i64; 6]);
    let pop: HashIdx<Str, T> = (&tr).map(|x: T| x.0).select(&db.tag.tag_name).inv().select(&tr).collect();
    let q = db.post.with(post_type_id.eq(1)).with(owner_user);
    let cand: MatSet<Id<Post>> = q.with(tags_str.flat_map(|t: Str| t.split(',')).select(&pop)).collect();
    let cc = (&cand).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let phi = (&cand).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(hd.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(htype_name(db))).buf_fold(|v| {
        let n = v.len() as i64;
        let mut t: Vec<Str> = v.into_iter().collect();
        t.sort_unstable();
        t.dedup();
        (&*Box::leak(t.join(", ").into_boxed_str()), n)
    });
    rows(drain((&cc).and((&phi).opt()).and(tags_str.flat_map(|t: Str| t.split(',')).select(&pop))).into_iter().map(|(p, ((c, h), (t, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.push(V::I(c));
        match h {
            Some((s, n)) => f.extend([V::S(s), V::I(n)]),
            None => f.extend([V::Null, V::Null]),
        }
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), nullable(a[2], a[1]), V::I(a[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// RecentBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b WHERE b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY b.UserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(rb.BadgeCount, 0) AS RecentBadgeCount, COALESCE(rb.BadgeNames, 'None') AS RecentBadges,
//        (SELECT COUNT(*) FROM Comments c WHERE c.UserId = u.Id) AS CommentTotal,
//        (SELECT COUNT(*) FROM Votes v WHERE v.UserId = u.Id AND v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') AS VoteTotal
//     FROM Users u LEFT JOIN RecentBadges rb ON u.Id = rb.UserId)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.RecentBadgeCount, u.RecentBadges, p.PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount
// FROM UserActivity u JOIN RankedPosts p ON u.UserId = p.PostId WHERE p.PostRank <= 5 ORDER BY u.Reputation DESC, p.CreationDate DESC LIMIT 100;
//
// `u.UserId = p.PostId` joins a user id to a post id, so it goes through the raw ids. CommentTotal and VoteTotal are never read. The badge names are
// joined in badge id order; the answer is empty on this data, so that is untested.
fn q33801(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(score.gt(0)).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp = post_set(&top, |x| x.0);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let rb = db.badge.with((&db.badge.date).ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).group_by(&db.badge.user).select(Ident::<Badge>::new()).buf_fold(|v| {
        let mut b: Vec<Id<Badge>> = v.into_iter().collect();
        b.sort_unstable();
        let n: Vec<Str> = b.iter().map(|&x| db.badge.name.get(x).unwrap()).collect();
        (b.len() as i64, &*Box::leak(n.join(", ").into_boxed_str()))
    });
    let v = drain((&tp).select(Ident::<Post>::new().and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and((&rb).opt())))));
    let v = top_n(v, |&(_, (p, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(_, (p, (u, b)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b.map_or(0, |b| b.0)), V::S(b.map_or("None", |b| b.1))]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank = 1 AND (rp.UpVotes - rp.DownVotes) > 0)
// SELECT fp.*, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN bh.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount, STRING_AGG(pt.Name, ', ') AS PostTypeNames
// FROM FilteredPosts fp LEFT JOIN Badges b ON fp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId) LEFT JOIN PostHistory bh ON fp.PostId = bh.PostId
// LEFT JOIN PostTypes pt ON (SELECT PostTypeId FROM Posts WHERE Id = fp.PostId) = pt.Id
// GROUP BY fp.PostId, fp.Title, fp.CreationDate, fp.OwnerDisplayName, fp.CommentCount, fp.UpVotes, fp.DownVotes ORDER BY fp.UpVotes DESC, fp.CreationDate DESC LIMIT 100;
//
// Rank partitions by the post, so it is always 1. The badge join is on the owner's display name, so it takes the badges of every user with that name.
// Every joined row carries the same post type name, so PostTypeNames is that name once per row. The hundred posts are picked before the badge x
// history product, which is driven for them alone.
fn q6529(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let fp = top_n(drain((&rp).filt(|a| a[1] - a[2] > 0)), |&(p, a)| (Reverse(a[1]), Reverse(creation_date.get(p).unwrap())), 100);
    let tp = post_set(&fp, |x| x.0);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let bh = (&tp)
        .group_by(Ident::<Post>::new())
        .select(owner_user.select(&db.user.display_name).select(&by_name).select(badges_of(db)).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, (b, h)| [a[0] + b.is_some() as i64, a[1] + (h == Some(10)) as i64, a[2] + 1]);
    rows(drain((&tp).select(Ident::<Post>::new().and(&rp).and(&bh).and(ptype_name(db)))).into_iter().map(|(_, (((p, a), b), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b[0]), V::I(b[1])]);
        f.push(V::Owned(vec![n; b[2] as usize].join(", ")));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("23397", q23397),
    ("20335", q20335),
    ("22588", q22588),
    ("23755", q23755),
    ("23203", q23203),
    ("20691", q20691),
    ("2509", q2509),
    ("23808", q23808),
    ("20667", q20667),
    ("21109", q21109),
    ("24164", q24164),
    ("23586", q23586),
    ("30662", q30662),
    ("21299", q21299),
    ("20027", q20027),
    ("22517", q22517),
    ("22640", q22640),
    ("21062", q21062),
    ("23059", q23059),
    ("33465", q33465),
    ("20135", q20135),
    ("32149", q32149),
    ("21894", q21894),
    ("23897", q23897),
    ("23169", q23169),
    ("24713", q24713),
    ("25882", q25882),
    ("4346", q4346),
    ("5962", q5962),
    ("29104", q29104),
    ("5051", q5051),
    ("26499", q26499),
    ("8715", q8715),
    ("1043", q1043),
    ("29664", q29664),
    ("26831", q26831),
    ("3202", q3202),
    ("891", q891),
    ("26209", q26209),
    ("4776", q4776),
    ("28069", q28069),
    ("6348", q6348),
    ("3868", q3868),
    ("9490", q9490),
    ("25460", q25460),
    ("2882", q2882),
    ("34356", q34356),
    ("1933", q1933),
    ("9908", q9908),
    ("8204", q8204),
    ("5691", q5691),
    ("6451", q6451),
    ("29705", q29705),
    ("28394", q28394),
    ("29302", q29302),
    ("4306", q4306),
    ("26374", q26374),
    ("617", q617),
    ("29365", q29365),
    ("29909", q29909),
    ("33944", q33944),
    ("8655", q8655),
    ("33474", q33474),
    ("3208", q3208),
    ("25617", q25617),
    ("30746", q30746),
    ("30531", q30531),
    ("26835", q26835),
    ("33801", q33801),
    ("6529", q6529),
];
