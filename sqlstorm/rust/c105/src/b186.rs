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
];
