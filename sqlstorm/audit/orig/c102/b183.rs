use harness::prelude::*;
use std::cmp::Reverse;

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


// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionsAsked,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswersGiven, MIN(u.CreationDate) AS AccountSince, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// UserStats AS (SELECT UserId, DisplayName, Upvotes - Downvotes AS NetVotes, QuestionsAsked + AnswersGiven AS TotalEngagement, AccountSince,
//        ROW_NUMBER() OVER (ORDER BY Upvotes - Downvotes DESC) AS RankNetVotes, ROW_NUMBER() OVER (ORDER BY QuestionsAsked + AnswersGiven DESC) AS RankEngagement FROM UserActivity),
// ClosedPosts AS (SELECT p.OwnerUserId, COUNT(*) AS ClosedCount, MAX(p.ClosedDate) AS LastClosedDate FROM Posts p WHERE p.ClosedDate IS NOT NULL GROUP BY p.OwnerUserId),
// UserRankings AS (SELECT u.UserId, u.DisplayName, u.NetVotes, u.TotalEngagement, c.ClosedCount, c.LastClosedDate,
//        CASE WHEN c.ClosedCount IS NULL THEN 'No Closed Posts' WHEN c.ClosedCount > 5 THEN 'Frequent Closer' WHEN c.ClosedCount BETWEEN 1 AND 5 THEN 'Occasional Closer' ELSE 'Unknown' END AS ClosureBehavior
//     FROM UserStats u LEFT JOIN ClosedPosts c ON u.UserId = c.OwnerUserId)
// SELECT UR.DisplayName, UR.NetVotes, UR.TotalEngagement, UR.ClosedCount, COALESCE(CAST(UR.LastClosedDate AS DATE), DATE '1970-01-01') AS LastClosedDate, UR.ClosureBehavior
// FROM UserRankings UR WHERE (UR.NetVotes != 0 OR UR.TotalEngagement > 0) AND (UR.TotalEngagement BETWEEN 10 AND 100 OR UR.ClosedCount IS NOT NULL)
// ORDER BY UR.NetVotes DESC, UR.TotalEngagement DESC LIMIT 50;
fn q23122(db: &'static So) -> String {
    let Post { post_type_id, closed_date, owner_user, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, _)| match p {
            Some((t, v)) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64],
            None => a,
        });
    let cp = db.post.with(closed_date).group_by(owner_user).select(closed_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&ua).and((&cp).opt()).filt(|(a, c): ([i64; 4], Option<(i64, i64)>)| {
        let (nv, te) = (a[0] - a[1], a[2] + a[3]);
        (nv != 0 || te > 0) && ((10..=100).contains(&te) || c.is_some())
    }));
    let v = top_n(v, |&(u, (a, _))| (Reverse(a[0] - a[1]), Reverse(a[2] + a[3]), u), 50);
    rows(v.into_iter().map(|(u, (a, c))| {
        let beh = match c {
            None => "No Closed Posts",
            Some((n, _)) if n > 5 => "Frequent Closer",
            Some((n, _)) if (1..=5).contains(&n) => "Occasional Closer",
            _ => "Unknown",
        };
        row(vec![user_col(db, u, "name"), V::I(a[0] - a[1]), V::I(a[2] + a[3]), oint(c.map(|c| c.0)), V::D(c.map_or(0, |c| trunc_day(c.1))), V::S(beh)])
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS TotalPosts, COALESCE(SUM(p.Score), 0) AS TotalScore, SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount FROM Posts p GROUP BY p.OwnerUserId),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ps.TotalPosts, 0) AS PostsMade, COALESCE(ps.TotalScore, 0) AS Score, COALESCE(ps.TotalAnswers, 0) AS AnswersGiven,
//        CASE WHEN COALESCE(ps.QuestionCount, 0) > 0 THEN ROUND(COALESCE(ps.TotalScore, 0) / COALESCE(ps.QuestionCount, 1), 2) ELSE 0 END AS AvgScorePerQuestion,
//        ub.BadgeCount AS TotalBadges, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
//     FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE u.Reputation > 100),
// TopUsers AS (SELECT ue.*, RANK() OVER (ORDER BY ue.Score DESC, ue.PostsMade DESC) AS ScoreRank FROM UserEngagement ue)
// SELECT tu.DisplayName, tu.PostsMade, tu.Score, tu.AvgScorePerQuestion, tu.TotalBadges, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges,
//        CASE WHEN tu.ScoreRank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorCategory,
//        EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = tu.UserId AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR'
//                AND p.Title NOT LIKE '%deleted%') AS ActiveInPastYear
// FROM TopUsers tu WHERE tu.ScoreRank <= 50 ORDER BY tu.Score DESC, tu.PostsMade DESC;
fn q23834(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, score, answer_count, post_type_id, creation_date, title, .. } = &db.post;
    let ps = db
        .post
        .group_by(owner_user)
        .select(score.and(answer_count.opt()).and(post_type_id))
        .fold([0i64; 4], |a, ((s, n), t)| [a[0] + 1, a[1] + s, a[2] + n.unwrap_or(0), a[3] + (t == 1) as i64]);
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let active: MatSet<Id<User>> = db.post.with(creation_date.ge(since)).with(title.filt(|t: Str| !t.contains("deleted"))).select(owner_user).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select((&ps).opt().and(&ub).and(Ident::<User>::new().with(&active).opt())));
    let v = ranked(v, |&(_, ((p, _), _))| {
        let a = p.unwrap_or([0; 4]);
        (Reverse(a[1]), Reverse(a[0]))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 50).map(|((u, ((p, b), act)), r)| {
        let a = p.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::F(if a[3] > 0 { (a[1] as f64 / a[3] as f64 * 100.0).round() / 100.0 } else { 0.0 })];
        f.extend(b.map(V::I));
        f.push(V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" }));
        f.push(V::B(act.is_some()));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, COUNT(DISTINCT P.Id) AS PostCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, UpVotes, DownVotes, PostCount, QuestionCount, AnswerCount FROM UserStats WHERE UserRank <= 50),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.PostTypeId,
//        COALESCE((SELECT AVG(V.BountyAmount) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 8), 0) AS AvgBounty,
//        COALESCE((SELECT COUNT(C.Id) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount,
//        COALESCE((SELECT COUNT(V.Id) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2), 0) AS UpVoteCount,
//        COALESCE((SELECT COUNT(V.Id) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3), 0) AS DownVoteCount
//     FROM Posts P WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// PostRanking AS (SELECT D.PostId, D.Title, D.CreationDate, D.PostTypeId, D.AvgBounty, D.CommentCount, D.UpVoteCount, D.DownVoteCount,
//        RANK() OVER (ORDER BY D.UpVoteCount DESC) AS RankByUpVotes FROM PostDetails D)
// SELECT U.DisplayName, U.Reputation, P.Title, P.CreationDate, P.AvgBounty, P.CommentCount, P.UpVoteCount, P.DownVoteCount,
//        CASE WHEN P.UpVoteCount > P.DownVoteCount THEN 'Positive' WHEN P.UpVoteCount < P.DownVoteCount THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM TopUsers U JOIN PostRanking P ON U.UserId = P.PostId WHERE P.RankByUpVotes <= 10 ORDER BY U.Reputation DESC, P.UpVoteCount DESC;
//
// `U.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids. A Reputation tie at UserRank 50 goes to the smaller user id.
fn q4634(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 50);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let recent = || db.post.with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let vs = recent()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt())
        .fold([0i64; 4], |a, v| match v {
            Some((t, b)) => {
                let b = if t == 8 { b } else { None };
                [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (t == 2) as i64, a[3] + (t == 3) as i64]
            }
            None => a,
        });
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pr = ranked(drain((&vs).and(&cc)), |&(_, (a, _))| Reverse(a[2]), false);
    let pr = rel(pr.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let by_post: HashIdx<Id<Post>, (Id<Post>, ([i64; 4], i64))> = (&pr).map(|(p, _)| p).inv().select(&pr).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let mut v = drain((&tu).select((&db.user.origid).select(&pidx).select(&by_post)));
    v.sort_by_key(|&(u, (_, (a, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[2])));
    rows(v.into_iter().map(|(u, (p, (a, c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([
            V::F(if a[0] == 0 { 0.0 } else { a[1] as f64 / a[0] as f64 }),
            V::I(c),
            V::I(a[2]),
            V::I(a[3]),
            V::S(if a[2] > a[3] { "Positive" } else if a[2] < a[3] { "Negative" } else { "Neutral" }),
        ]);
        row(f)
    }))
}

// WITH RecursiveUserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COUNT(P.Id) AS PostsCount,
//        SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesGiven,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesGiven, SUM(COALESCE(B.Class, 0)) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     LEFT JOIN (SELECT UserId, SUM(Class) AS Class FROM Badges GROUP BY UserId) B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// UserPostStats AS (SELECT UserId, DisplayName, Reputation, CreationDate, LastAccessDate, PostsCount, TotalScore, UpVotesGiven, DownVotesGiven, TotalBadges,
//        ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM RecursiveUserActivity),
// FilteredUsers AS (SELECT UserId, DisplayName, Reputation, CreationDate, LastAccessDate, PostsCount, TotalScore, UpVotesGiven, DownVotesGiven, TotalBadges
//     FROM UserPostStats WHERE Reputation > 100 AND PostsCount > 5)
// SELECT FU.DisplayName, FU.Reputation, FU.PostsCount, FU.TotalScore, FU.UpVotesGiven, FU.DownVotesGiven, FU.TotalBadges, PT.Name AS PostType,
//        COUNT(Ph.PostId) AS HistoryCount, COUNT(DISTINCT C.Id) AS CommentCount, AVG(COALESCE(PV.VotesPerPost, 0)) AS AverageVotesPerPost
// FROM FilteredUsers FU LEFT JOIN Posts P ON FU.UserId = P.OwnerUserId LEFT JOIN PostHistory Ph ON P.Id = Ph.PostId LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN (SELECT PostId, COUNT(*) AS VotesPerPost FROM Votes GROUP BY PostId) PV ON P.Id = PV.PostId LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id
// GROUP BY FU.UserId, FU.DisplayName, FU.Reputation, FU.PostsCount, FU.TotalScore, FU.UpVotesGiven, FU.DownVotesGiven, FU.TotalBadges, PT.Name
// HAVING COUNT(P.Id) > 3 AND SUM(PV.VotesPerPost) > 0 ORDER BY FU.TotalScore DESC, FU.Reputation DESC;
//
// Not recursive despite the name. Every group kept by `COUNT(P.Id) > 3` has posts, so it is grouped over the filtered users' posts by (owner, type name).
fn q33172(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let bsum = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold(0i64, |s, c| s + c);
    let rua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and((&bsum).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let b = b.unwrap_or(0);
            match p {
                Some((s, v)) => [a[0] + 1, a[1] + s, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b],
                None => [a[0], a[1], a[2], a[3], a[4] + b],
            }
        });
    let fu: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(100)).with((&rua).filt(|a| a[0] > 5)).collect();
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let base = || db.post.with(owner_user.select(Ident::<User>::new().with(&fu)));
    let g = base()
        .group_by(owner_user.and(ptype_name(db)))
        .select(history_of(db).opt().and(comments_of(db).opt()).and((&pv).opt()))
        .fold([0i64; 4], |a, ((h, _), w)| [a[0] + 1, a[1] + h.is_some() as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let cd = base().group_by(owner_user.and(ptype_name(db))).select(comments_of(db)).count_distinct();
    type K = (Id<User>, Str);
    let v = drain((&g).filt(|a| a[0] > 3 && a[2] > 0 && a[3] > 0).and((&cd).opt()).and(Same::<K>::new().map(|(u, _): K| u).select(&rua)));
    rows(v.into_iter().map(|((u, t), ((a, c), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(r.map(V::I));
        f.extend([V::S(t), V::I(a[1]), V::I(c.unwrap_or(0)), avg(a[3], a[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// RecentBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b WHERE b.Date > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY b.UserId),
// PostHistoryData AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEditDate, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Comment END) AS CloseReason,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount FROM PostHistory ph GROUP BY ph.PostId),
// UserVoteStats AS (SELECT v.UserId, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 1 THEN 1 ELSE 0 END) AS AcceptedCount FROM Votes v GROUP BY v.UserId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, COALESCE(rb.BadgeCount, 0) AS RecentBadgeCount, COALESCE(phd.CloseReason, 'No Closure') AS LastCloseReason,
//        phd.LastEditDate, usr.Reputation AS UserReputation, CASE WHEN uvs.UpvoteCount IS NULL THEN 0 ELSE uvs.UpvoteCount END AS TotalUpvotes,
//        CASE WHEN uvs.DownvoteCount IS NULL THEN 0 ELSE uvs.DownvoteCount END AS TotalDownvotes
//     FROM RankedPosts rp LEFT JOIN RecentBadges rb ON rp.OwnerUserId = rb.UserId LEFT JOIN PostHistoryData phd ON rp.PostId = phd.PostId
//     LEFT JOIN Users usr ON rp.OwnerUserId = usr.Id LEFT JOIN UserVoteStats uvs ON rp.OwnerUserId = uvs.UserId WHERE rp.rn = 1)
// SELECT FR.*, CASE WHEN FR.TotalUpvotes > FR.TotalDownvotes THEN 'Positive' WHEN FR.TotalDownvotes > FR.TotalUpvotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM FinalResults FR ORDER BY FR.RecentBadgeCount DESC, FR.LastEditDate DESC, FR.UserReputation DESC;
//
// The ownerless questions are one partition (NULL OwnerUserId), whose newest question is kept too. A CreationDate tie within an owner goes to the smaller post id.
fn q23107(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rb = db.badge.with((&db.badge.date).gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, creation_date: hd, post_history_type_id: ht, comment, .. } = &db.post_history;
    let phd = db.post_history.group_by(post).select(hd.and(ht).and(comment.opt())).fold((i64::MIN, None::<Str>), |(m, r), ((d, t), c)| {
        (m.max(d), if t == 10 { match (r, c) { (Some(a), Some(b)) => Some(a.max(b)), (a, b) => a.or(b) } } else { r })
    });
    let uvs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 2 | 3) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&tp).select(owner_user.select(&rb).opt().and((&phd).opt()).and(owner_user.select(&uvs).opt())));
    rows(v.into_iter().map(|(p, ((b, h), u))| {
        let u = u.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(b.unwrap_or(0)), V::S(h.and_then(|h| h.1).unwrap_or("No Closure")), ots(h.map(|h| h.0))]);
        f.extend(post_fields(db, p, &["rep"]));
        f.extend([V::I(u[0]), V::I(u[1]), V::S(if u[0] > u[1] { "Positive" } else if u[1] > u[0] { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// Rewritten (rewrites/33771.sql): the RankedPosts window is tie-broken on p.Id and TopTags' ORDER BY TagCount DESC LIMIT 5, where every tag counts 1, on t.TagName.
// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id) AS Rank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentPostHistory AS (SELECT ph.Id, ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.Comment, ph.UserDisplayName, p.Title AS PostTitle FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopTags AS (SELECT t.TagName, COUNT(*) AS TagCount FROM Tags t JOIN Posts p ON t.ExcerptPostId = p.Id GROUP BY t.TagName ORDER BY TagCount DESC, t.TagName LIMIT 5),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS PostsCreated FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY u.Id, u.DisplayName),
// FinalResults AS (SELECT r.Id AS PostId, r.Title, r.CreationDate, r.ViewCount, r.AnswerCount, ph.UserDisplayName AS LastEditor, T.TagName, ua.DisplayName AS UserName, ua.UpVotes,
//        ua.DownVotes, ua.PostsCreated
//     FROM RankedPosts r LEFT JOIN RecentPostHistory ph ON r.Id = ph.PostId LEFT JOIN TopTags T ON POSITION(T.TagName IN r.Title) > 0 LEFT JOIN UserActivity ua ON r.Id = ua.UserId
//     WHERE r.Rank <= 10)
// SELECT PostId, Title, CreationDate, ViewCount, AnswerCount, LastEditor, TagName, UserName, UpVotes, DownVotes, PostsCreated FROM FinalResults
// WHERE UserName IS NOT NULL OR TagName IS NOT NULL ORDER BY CreationDate DESC;
//
// `r.Id = ua.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q33771(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, title, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.gt(add_years(t0, -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), origid.get(p).unwrap())
    }, 10, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tc = db.tag.with(&db.tag.excerpt_post).group_by(&db.tag.tag_name).select(Ident::<Tag>::new()).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&tc), |&(n, c)| (Reverse(c), n), 5));
    let tt_idx: HashIdx<Str, (Str, i64)> = (&tt).map(|(n, _)| n).inv().select(&tt).collect();
    let titles: MatSet<Str> = (&rp).select(title).collect();
    let hit: HashIdx<Str, (Str, i64)> = (&titles).select_where(&tt_idx, |t: Str, n: Str| t.contains(n)).collect();
    let recent_h = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).gt(add_days(t0, -30))));
    let newu = || db.user.with((&db.user.creation_date).gt(add_months(t0, -1)));
    let ua = newu()
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ud = newu().group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let user = origid.select(&uidx).select(Ident::<User>::new().and(&ua).and((&ud).opt()));
    let v = drain((&rp).select(recent_h.opt().and(title.select(&hit).opt()).and(user.opt())).filt(|((_, t), u)| t.is_some() || u.is_some()));
    rows(v.into_iter().map(|(p, ((h, t), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers"]);
        f.push(h.map_or(V::Null, |h| ostr(db.post_history.user_display_name.get(h))));
        f.push(t.map_or(V::Null, |t| V::S(t.0)));
        f.extend(match u {
            Some(((u, a), n)) => [user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n.unwrap_or(0))],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.Reputation,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
//     AND p.ViewCount > (SELECT AVG(ViewCount) FROM Posts WHERE CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year')),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.Reputation,
//        CASE WHEN rp.Score IS NULL OR rp.Score = 0 THEN 'No Score' WHEN rp.Reputation IS NULL OR rp.Reputation = 0 THEN 'Anonymous' ELSE 'Active User' END AS UserStatus
//     FROM RankedPosts rp WHERE rp.PostRank <= 10),
// PostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COALESCE(NULLIF(SUM(c.Score), 0), -1) AS TotalCommentScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.Id IN (SELECT PostId FROM TopPosts) GROUP BY p.Id),
// FinalReport AS (SELECT tp.PostId, tp.Title, tp.Score, tp.Reputation, tp.UserStatus, pc.CommentCount, pc.TotalCommentScore,
//        CASE WHEN pc.TotalCommentScore < 0 THEN 'Insufficient Data' WHEN pc.TotalCommentScore > 10 THEN 'Highly Rated' ELSE 'Moderately Rated' END AS CommentScoreStatus
//     FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId)
// SELECT fr.PostId, fr.Title, fr.Score, fr.Reputation, fr.UserStatus, fr.CommentCount, fr.TotalCommentScore, fr.CommentScoreStatus,
//        CASE WHEN fr.CommentCount IS NULL THEN 'No Comments Yet' ELSE CONCAT(fr.CommentCount, ' comments') END AS CommentsSummary,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = fr.PostId AND v.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = fr.PostId AND v.VoteTypeId = 3) AS DownVotes
// FROM FinalReport fr ORDER BY fr.Score DESC, fr.CommentCount DESC;
//
// The uncorrelated AVG is computed once and compared against in the filter. A (Score, CreationDate) tie at PostRank 10 goes to the smaller post id.
fn q22168(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, post_type_id, score, .. } = &db.post;
    let t0 = add_years(ts(2024, 10, 1, 0, 0, 0), -1);
    let (s, n) = db.post.with(creation_date.ge(t0)).select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let avgv = s as f64 / n as f64;
    let v = drain(db.post.with(creation_date.ge(t0)).with(view_count.filt(move |w: i64| w as f64 > avgv)).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold((0i64, 0i64), |(n, s), c| match c {
        Some(c) => (n + 1, s + c),
        None => (n, s),
    });
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let mut v = drain((&pc).and(&vc));
    v.sort_by_key(|&(p, ((n, _), _))| (Reverse(score.get(p).unwrap()), Reverse(n)));
    rows(v.into_iter().map(|(p, ((n, s), u))| {
        let sc = score.get(p).unwrap();
        let rep = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        let tcs = if n == 0 || s == 0 { -1 } else { s };
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([
            V::I(rep),
            V::S(if sc == 0 { "No Score" } else if rep == 0 { "Anonymous" } else { "Active User" }),
            V::I(n),
            V::I(tcs),
            V::S(if tcs < 0 { "Insufficient Data" } else if tcs > 10 { "Highly Rated" } else { "Moderately Rated" }),
            V::Owned(format!("{n} comments")),
            V::I(u[0]),
            V::I(u[1]),
        ]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank,
//        COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, ph.UserId AS LastEditorId, ph.CreationDate AS LastEditDate,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS EditRank FROM Posts p INNER JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10),
// RecentClosedPosts AS (SELECT cp.*, us.DisplayName AS LastEditorName, us.Reputation AS LastEditorReputation FROM ClosedPosts cp JOIN Users us ON us.Id = cp.LastEditorId
//     WHERE cp.LastEditDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// TopUsers AS (SELECT UserId, SUM(CASE WHEN PostCount > 0 THEN 1 ELSE 0 END) AS ActivePostsCount, MAX(Reputation) AS MaxReputation FROM UserStats GROUP BY UserId HAVING COUNT(*) > 1),
// FinalStats AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.TotalBounties, rcp.Title AS RecentClosedPost, rcp.LastEditorName, rcp.LastEditorReputation
//     FROM UserStats us LEFT JOIN RecentClosedPosts rcp ON us.UserId = rcp.LastEditorId JOIN TopUsers t ON us.UserId = t.UserId)
// SELECT fs.UserId, fs.DisplayName, fs.Reputation, fs.PostCount, fs.TotalBounties, fs.RecentClosedPost, COALESCE(fs.LastEditorName, 'N/A') AS LastEditorName,
//        COALESCE(fs.LastEditorReputation, 0) AS LastEditorReputation
// FROM FinalStats fs ORDER BY fs.Reputation DESC, fs.PostCount DESC FETCH FIRST 10 ROWS ONLY;
//
// TopUsers groups UserStats, which has one row per user, by UserId, so it is computed from the users alone; the posts x votes product is driven only for the users it keeps.
fn q1080(db: &'static So) -> String {
    let pos = || db.user.with((&db.user.reputation).gt(0));
    let n = pos().group_by(Ident::<User>::new()).select(Ident::<User>::new()).fold(0i64, |n, _| n + 1);
    let tu: MatSet<Id<User>> = db.user.with((&n).filt(|n| n > 1)).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold(0i64, |s, (_, b)| s + b.flatten().unwrap_or(0));
    let PostHistory { post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let rcp: HashIdx<Id<User>, Id<PostHistory>> =
        db.post_history.with(post_history_type_id.eq(10)).with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(user).inv().collect();
    let v = drain((&us).and(user_distinct_posts(db)).and((&rcp).opt()));
    let v = top_n(v, |&(u, ((_, n), h))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u, h), 10);
    rows(v.into_iter().map(|(u, ((b, n), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(b)]);
        f.extend(match h {
            Some(h) => [post_fields(db, db.post_history.post.get(h).unwrap(), &["title"]).remove(0), user_col(db, u, "name"), user_col(db, u, "rep")],
            None => [V::Null, V::S("N/A"), V::I(0)],
        });
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(v.BountyAmount) AS TotalBounty, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName, u.Reputation),
// ActiveUsers AS (SELECT um.UserId, um.DisplayName, um.Reputation, ROW_NUMBER() OVER (ORDER BY um.Reputation DESC) AS ReputationRank,
//        DENSE_RANK() OVER (PARTITION BY CASE WHEN um.QuestionCount > 0 THEN 'Active Questions' WHEN um.AnswerCount > 0 THEN 'Active Answers' ELSE 'Inactive' END
//                           ORDER BY um.PostCount DESC) AS ActivityRank
//     FROM UserMetrics um WHERE um.LastPostDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'),
// BadgeData AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount FROM Badges b GROUP BY b.UserId),
// FinalMetrics AS (SELECT au.UserId, au.DisplayName, au.Reputation, au.ReputationRank, au.ActivityRank, COALESCE(bd.BadgeCount, 0) AS BadgeCount,
//        COALESCE(bd.GoldBadgeCount, 0) AS GoldBadgeCount, COALESCE(bd.SilverBadgeCount, 0) AS SilverBadgeCount, COALESCE(bd.BronzeBadgeCount, 0) AS BronzeBadgeCount
//     FROM ActiveUsers au LEFT JOIN BadgeData bd ON au.UserId = bd.UserId)
// SELECT f.DisplayName, f.Reputation, f.ReputationRank, f.ActivityRank, f.BadgeCount, f.GoldBadgeCount, f.SilverBadgeCount, f.BronzeBadgeCount
// FROM FinalMetrics f WHERE f.ReputationRank <= 10 ORDER BY f.Reputation DESC, f.ActivityRank ASC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// A Reputation tie at ReputationRank 10 goes to the smaller user id.
fn q22465(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let um = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(creation_date)).opt().and(votes_by(db).opt()))
        .fold([0i64, 0, i64::MIN], |a, (p, _)| match p {
            Some((t, d)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2].max(d)],
            None => a,
        });
    let t = add_months(ts(2024, 10, 1, 12, 34, 56), -1);
    let cat = |a: [i64; 3]| if a[0] > 0 { 0 } else if a[1] > 0 { 1 } else { 2 };
    let au = drain((&um).filt(move |a| a[2] >= t).and(user_distinct_posts(db)));
    let r = per_group(ranked(au, |&(_, (a, n))| (cat(a), Reverse(n)), true), |&(_, (a, _))| cat(a));
    let r = top_n(r, |&((u, _), _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    let bd = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    type R = (Id<User>, i64, i64);
    let rv = rel(r.into_iter().enumerate().map(|(i, ((u, _), ar))| (u, i as i64 + 1, ar)).collect::<Vec<R>>());
    let v = drain((&rv).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(&bd).opt())));
    rows(v.into_iter().map(|(_, ((u, rr, ar), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(rr), V::I(ar)]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, u.Reputation,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// TopRankedPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate, Reputation FROM RankedPosts WHERE rn = 1),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVotesCount
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// PostStatistics AS (SELECT t.PostId, t.Title, t.Score, t.ViewCount, COALESCE(rv.UpVotesCount, 0) AS UpVotesCount, COALESCE(rv.DownVotesCount, 0) AS DownVotesCount, t.Reputation,
//        CASE WHEN t.Score >= 10 THEN 'Popular' WHEN t.Reputation >= 1000 THEN 'Influencer' ELSE 'Regular' END AS Classification
//     FROM TopRankedPosts t LEFT JOIN RecentVotes rv ON t.PostId = rv.PostId),
// ClosedPosts AS (SELECT PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseCount FROM PostHistory ph GROUP BY PostId)
// SELECT ps.Title, ps.Score, ps.UpVotesCount, ps.DownVotesCount, ps.Classification, COALESCE(cp.CloseCount, 0) AS CloseCount, ps.ViewCount,
//        CASE WHEN ps.UpVotesCount > ps.DownVotesCount THEN 'Positive' WHEN ps.UpVotesCount < ps.DownVotesCount THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM PostStatistics ps LEFT JOIN ClosedPosts cp ON ps.PostId = cp.PostId WHERE (ps.UpVotesCount + ps.DownVotesCount) > 10
// ORDER BY ps.Score DESC, ps.Reputation DESC, ps.ViewCount DESC OFFSET 0 ROWS FETCH NEXT 100 ROWS ONLY;
//
// The ownerless questions are one partition (NULL OwnerUserId). A (Score, CreationDate) tie within an owner goes to the smaller post id.
fn q20861(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(recent.select(vtype_name(db)).opt())
        .fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    let cp = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold(0i64, |n, t| n + matches!(t, 10 | 11) as i64);
    let v = drain((&ps).filt(|a| a[0] + a[1] > 10).and((&cp).opt()));
    let v = top_n(v, |&(p, _)| {
        let r = owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap());
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), r.is_none(), Reverse(r), w.is_none(), Reverse(w), p)
    }, 100);
    rows(v.into_iter().map(|(p, (a, c))| {
        let s = score.get(p).unwrap();
        let r = owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap());
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if s >= 10 { "Popular" } else if r.map_or(false, |r| r >= 1000) { "Influencer" } else { "Regular" }), V::I(c.unwrap_or(0))]);
        f.extend(post_fields(db, p, &["views"]));
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, p.Score, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, p.Score),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
//        COALESCE(SUM(p.Score), 0) AS TotalScore
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostHistoryDetails AS (SELECT ph.PostId, ph.UserId AS HistoryUserId, ph.CreationDate AS HistoryDate, ph.Comment AS ChangeComment, ph.PostHistoryTypeId,
//        CASE WHEN ph.PostHistoryTypeId = 10 THEN 'Closed' WHEN ph.PostHistoryTypeId = 11 THEN 'Reopened' ELSE 'Other Change' END AS ChangeType
//     FROM PostHistory ph WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 months')
// SELECT rp.PostId, rp.Title, u.DisplayName AS OwnerDisplayName, us.GoldBadges, us.SilverBadges, us.BronzeBadges, us.TotalBounty, us.TotalViews, us.TotalScore,
//        phd.ChangeComment, phd.HistoryDate, phd.ChangeType
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserStatistics us ON u.Id = us.UserId LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId
// WHERE (phd.ChangeType = 'Closed' OR phd.ChangeType = 'Reopened' OR phd.ChangeComment IS NULL) ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// CommentCount and ScoreRank are never read. UserStatistics is joined to the post owners only, so its badges x votes x posts product is driven for those users alone.
fn q24816(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let owners: MatSet<Id<User>> = rp().select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).and(posts_of(db).select(score.and(view_count.opt())).opt()))
        .fold([0i64; 6], |a, ((c, b), p)| {
            let b = b.flatten();
            let (s, w) = p.map_or((0, None), |(s, w)| (s, w));
            [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + b.unwrap_or(0), a[4] + w.unwrap_or(0), a[5] + s]
        });
    let PostHistory { creation_date: hd, post_history_type_id: ht, comment, .. } = &db.post_history;
    let phd = history_of(db)
        .select(Ident::<PostHistory>::new().with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -2))))
        .select(Ident::<PostHistory>::new().and(ht).and(comment.opt()))
        .opt()
        .filt(|x: Option<((Id<PostHistory>, i64), Option<Str>)>| match x {
            None => true,
            Some(((_, t), c)) => t == 10 || t == 11 || c.is_none(),
        });
    let v = drain(rp().select(owner_user.select(&us).and(phd)));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(((h, t), c)) => [ostr(c), V::T(hd.get(h).unwrap()), V::S(if t == 10 { "Closed" } else if t == 11 { "Reopened" } else { "Other Change" })],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS Rank FROM Posts p WHERE p.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '1 year')),
// TopPosts AS (SELECT PostId, Title, ViewCount, CommentCount FROM RankedPosts WHERE Rank <= 10),
// PostVoteCounts AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostHistoryDetails AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN ph.CreationDate END) AS LastEditDate
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId),
// ComplexMetrics AS (SELECT tp.PostId, tp.Title, tp.ViewCount, tp.CommentCount, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes,
//        COALESCE(phd.EditCount, 0) AS EditCount, COALESCE(phd.LastEditDate, '1900-01-01') AS LastEditDate,
//        CASE WHEN pvc.UpVotes > pvc.DownVotes THEN 'More Upvotes' WHEN pvc.DownVotes > pvc.UpVotes THEN 'More Downvotes' ELSE 'Neutral' END AS VoteSentiment
//     FROM TopPosts tp LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId LEFT JOIN PostHistoryDetails phd ON tp.PostId = phd.PostId)
// SELECT cm.*, CASE WHEN EditCount > 5 THEN 'Highly Edited' WHEN EditCount BETWEEN 2 AND 5 THEN 'Moderately Edited' ELSE 'Rarely Edited' END AS EditFrequency,
//        CASE WHEN VoteSentiment = 'More Upvotes' AND ViewCount > 100 THEN 'Trending' ELSE 'Stable' END AS TrendStatus
// FROM ComplexMetrics cm WHERE ViewCount IS NOT NULL AND (LastEditDate IS NULL OR LastEditDate < cast('2024-10-01' as date) - INTERVAL '30 days')
// ORDER BY ViewCount DESC, EditCount DESC;
//
// A ViewCount tie at Rank 10 goes to the smaller post id.
fn q21859(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).with(view_count).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pvc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.with(ht.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let floor = ts(1900, 1, 1, 0, 0, 0);
    let cut = add_days(ts(2024, 10, 1, 0, 0, 0), -30);
    let v = drain((&cc).and((&pvc).opt()).and((&phd).opt()).filt(move |((_, _), h): ((i64, Option<[i64; 2]>), Option<(i64, i64)>)| h.map_or(floor, |h| h.1) < cut));
    rows(v.into_iter().map(|(p, ((c, u), h))| {
        let w = view_count.get(p).unwrap();
        let (up, dn) = u.map_or((0, 0), |u| (u[0], u[1]));
        let e = h.map_or(0, |h| h.0);
        let sent = match u {
            Some(u) if u[0] > u[1] => "More Upvotes",
            Some(u) if u[1] > u[0] => "More Downvotes",
            _ => "Neutral",
        };
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c), V::I(up), V::I(dn), V::I(e), V::T(h.map_or(floor, |h| h.1)), V::S(sent)]);
        f.push(V::S(if e > 5 { "Highly Edited" } else if (2..=5).contains(&e) { "Moderately Edited" } else { "Rarely Edited" }));
        f.push(V::S(if sent == "More Upvotes" && w > 100 { "Trending" } else { "Stable" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= timestamp '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(p.Score, 0)) AS TotalScore, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE v.CreationDate >= timestamp '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY v.PostId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT p.PostId, p.Title, u.DisplayName, us.TotalPosts, us.TotalScore, COALESCE(rc.CommentCount, 0) AS TotalComments, COALESCE(rv.VoteCount, 0) AS TotalVotes,
//        COALESCE(rv.UpVotes, 0) AS UpVotesCount, COALESCE(rv.DownVotes, 0) AS DownVotesCount, CASE WHEN p.PostRank = 1 THEN 'Most Recent Post' ELSE 'Previous Posts' END AS PostStatus
//     FROM RankedPosts p JOIN Users u ON p.OwnerUserId = u.Id JOIN UserStatistics us ON u.Id = us.UserId LEFT JOIN RecentVotes rv ON p.PostId = rv.PostId
//     LEFT JOIN PostComments rc ON p.PostId = rc.PostId)
// SELECT fr.PostId, fr.Title, fr.DisplayName, fr.TotalPosts, fr.TotalScore, fr.TotalComments, fr.TotalVotes, fr.UpVotesCount, fr.DownVotesCount, fr.PostStatus
// FROM FinalResults fr WHERE fr.TotalComments > 5 ORDER BY fr.TotalScore DESC, fr.TotalVotes DESC;
//
// UserStatistics is joined to the post owners only, so its posts x badges product is driven for those users alone. A CreationDate tie within an owner goes to the smaller post id.
fn q24372(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = || db.post.with(creation_date.ge(add_years(t0, -1)));
    let first = top_per(drain(rp().with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = rp().select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (s, b)| [a[0] + s.unwrap_or(0), a[1] + b.is_some() as i64]);
    let rv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(t0, -6)))).select(vtype_name(db));
    let vc = rp().group_by(Ident::<Post>::new()).select(rv.opt()).fold([0i64; 3], |a, n| match n {
        Some(n) => [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64],
        None => a,
    });
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&cc).filt(|n| n > 5).and(&vc).and(owner_user.select(Ident::<User>::new().and(&us).and(user_distinct_posts(db)))).and(Ident::<Post>::new().with(&first).opt()));
    rows(v.into_iter().map(|(p, (((c, a), ((u, s), n)), f1))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(n), V::I(s[0]), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if f1.is_some() { "Most Recent Post" } else { "Previous Posts" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostRankings AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Score, p.CreationDate, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) WHERE p.ViewCount > 0
//     GROUP BY p.Id, p.OwnerUserId, p.Score, p.CreationDate HAVING COUNT(DISTINCT c.Id) > 2 OR COUNT(DISTINCT v.Id) > 5),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.LastAccessDate, COALESCE(SUM(ps.Score), 0) AS TotalScore FROM Users u LEFT JOIN Posts ps ON u.Id = ps.OwnerUserId
//     WHERE u.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY u.Id, u.DisplayName, u.LastAccessDate)
// SELECT us.UserId, us.DisplayName AS UserName, us.Reputation, us.BadgeCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges, COALESCE(pr.Score, 0) AS TopPostScore, pr.ScoreRank,
//        au.TotalScore AS UserTotalScore, COUNT(DISTINCT ph.Id) AS PostHistoryCount
// FROM UserStats us LEFT JOIN PostRankings pr ON us.UserId = pr.OwnerUserId AND pr.ScoreRank = 1 LEFT JOIN PostHistory ph ON ph.UserId = us.UserId
// LEFT JOIN ActiveUsers au ON us.UserId = au.UserId WHERE us.Reputation > 100 OR us.BadgeCount > 5
// GROUP BY us.UserId, us.DisplayName, us.Reputation, us.BadgeCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges, pr.Score, pr.ScoreRank, au.TotalScore
// ORDER BY UserTotalScore DESC, us.Reputation DESC LIMIT 25 OFFSET 0;
//
// The rank-1 posts of an owner share one Score, so the GROUP BY on (pr.Score, pr.ScoreRank) makes one group per user; PostHistoryCount is the user's history rows.
fn q20220(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { view_count, owner_user, score, .. } = &db.post;
    let viewed = || db.post.with(view_count.gt(0));
    let nc = viewed().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ud = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let nv = viewed().group_by(Ident::<Post>::new()).select(ud.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let q = drain((&nc).and(&nv).filt(|(c, v)| c > 2 || v > 5).and(owner_user));
    let top = top_per(q, |&(_, (_, u))| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let pr: MatSet<(Id<User>, i64)> = rel(top.into_iter().map(|(p, (_, u))| (u, score.get(p).unwrap())).collect()).map(|x| x).collect();
    let pr_by: HashIdx<Id<User>, (Id<User>, i64)> = (&pr).map(|(u, _)| u).inv().collect();
    let au = db
        .user
        .with((&db.user.last_access_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt())
        .fold(0i64, |s, x| s + x.unwrap_or(0));
    let hc = db.post_history.group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let us = (&ub).and(&db.user.reputation).filt(|(b, r): ([i64; 4], i64)| r > 100 || b[0] > 5).map(|(b, _)| b);
    let v = drain(us.and((&pr_by).map(|(_, s)| s).opt()).and((&au).opt()).and((&hc).opt()));
    let v = top_n(v, |&(u, ((_, a), _))| (a.is_none(), Reverse(a), Reverse(db.user.reputation.get(u).unwrap()), u), 25);
    rows(v.into_iter().map(|(u, (((b, s), a), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(s.unwrap_or(0)), if s.is_some() { V::I(1) } else { V::Null }, oint(a), V::I(h.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, COALESCE(SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Votes vt ON p.Id = vt.PostId WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePostCount,
//        AVG(p.Score) AS AverageScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, PositivePostCount, AverageScore, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStatistics WHERE Reputation > 1000),
// CommentsSum AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT tp.UserId, tp.DisplayName, tp.Reputation, tp.PostCount, tp.PositivePostCount, tp.AverageScore, COALESCE(ps.UpVotes, 0) AS TotalUpVotes, COALESCE(ps.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(cs.CommentCount, 0) AS TotalComments, COUNT(DISTINCT l.RelatedPostId) AS RelatedPostsCount
// FROM TopUsers tp LEFT JOIN RankedPosts ps ON tp.UserId = ps.PostId LEFT JOIN CommentsSum cs ON ps.PostId = cs.PostId LEFT JOIN PostLinks l ON ps.PostId = l.PostId
// WHERE (tp.PostCount > 5 OR tp.PositivePostCount > 2) AND (tp.AverageScore IS NOT NULL OR tp.Reputation > 1500)
//   AND EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = tp.UserId AND b.Class = 1)
// GROUP BY tp.UserId, tp.DisplayName, tp.Reputation, tp.PostCount, tp.PositivePostCount, tp.AverageScore, ps.UpVotes, ps.DownVotes, cs.CommentCount, tp.Rank
// ORDER BY tp.Rank;
//
// `vt` aliases Votes, so `vt.Id = 2` tests the vote's own id, not its type.
// `tp.UserId = ps.PostId` joins a user id to a post id, so it goes through the raw ids. RankedPosts has one row per vote of a post, all with the same windowed
// counts, so the GROUP BY collapses them to the post.
fn q20227(db: &'static So) -> String {
    let Post { score, post_type_id, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt())
        .fold([0i64; 3], |a, s| match s {
            Some(s) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + s],
            None => a,
        });
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let tu = (&us).filt(|a| a[0] > 5 || a[1] > 2).and(&db.user.reputation).filt(|(a, r): ([i64; 3], i64)| a[0] > 0 || r > 1500).map(|(a, _)| a);
    let rq = || db.post.with(post_type_id.eq(1).and(score.gt(0)));
    let pv = rq().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.origid).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cs = rq().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rl = rq().group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let ps = (&db.user.origid).select(&pidx).select((&pv).and(&cs).and((&rl).opt()));
    let v = drain((&gold).select(tu.and(ps.opt())));
    let v = ranked(v, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    rows(v.into_iter().map(|((u, (a, p)), _)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[0])]);
        f.extend(match p {
            Some(((v, c), l)) => [V::I(v[0]), V::I(v[1]), V::I(c), V::I(l.unwrap_or(0))],
            None => [V::I(0), V::I(0), V::I(0), V::I(0)],
        });
        row(f)
    }))
}

// WITH RecursivePostAnalysis AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, COALESCE(votes.UpVotes, 0) AS UpVotes, COALESCE(votes.DownVotes, 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate) AS EntryRank, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentTotal, COALESCE(ph.TotalEdits, 0) AS EditHistory
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//                             FROM Votes GROUP BY PostId) votes ON p.Id = votes.PostId
//     LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS TotalEdits FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId) ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// MaxScoreCTE AS (SELECT PostId, MAX(Score) AS MaxScore FROM RecursivePostAnalysis GROUP BY PostId),
// TopPosts AS (SELECT r.Title, r.Score, r.ViewCount, r.UpVotes, r.DownVotes, r.CommentTotal, (r.UpVotes - r.DownVotes) AS NetVotes, AVG(r.EditHistory) OVER () AS AvgEdits,
//        CASE WHEN r.Score = (SELECT MAX(Score) FROM RecursivePostAnalysis) THEN 'Top Performer' ELSE 'Regular Post' END AS PostType
//     FROM RecursivePostAnalysis r JOIN MaxScoreCTE m ON r.PostId = m.PostId WHERE r.EntryRank = 1 ORDER BY r.Score DESC)
// SELECT *, CASE WHEN NetVotes IS NULL THEN 'No Votes' WHEN NetVotes > 0 THEN 'Positive Feedback' WHEN NetVotes < 0 THEN 'Negative Feedback' ELSE 'Neutral' END AS VoteFeedback,
//        CASE WHEN CommentTotal < AVG(CommentTotal) OVER () THEN 'Needs Attention' WHEN CommentTotal > AVG(CommentTotal) OVER () THEN 'Engaged Discussion' ELSE 'Moderate Engagement' END AS EngagementStatus
// FROM TopPosts WHERE PostType = 'Top Performer' OR (CommentTotal >= 10 AND NetVotes IS NOT NULL) ORDER BY ViewCount DESC;
//
// Not recursive despite the name. EntryRank = 1 keeps one of each post's comment rows, which agree in every column read, so TopPosts has one row per post.
// Both AVG() OVER () are means of integers, computed as one exact sum over their rows.
fn q24373(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let vs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id: ht, .. } = &db.post_history;
    let ed = db.post_history.with(ht.is_in([4, 5, 6])).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tp = (&cc).and((&vs).opt()).and((&ed).opt());
    let (es, en) = (&tp).fold_flat((0i64, 0i64), |(s, n), (_, e)| (s + e.unwrap_or(0), n + 1));
    let top = recent().select(score).fold_flat(i64::MIN, |m, s| m.max(s));
    let fin = (&tp).filt(|((c, _), _)| c >= 10).union(recent().with(score.eq(top)).select(&tp).filt(|((c, _), _)| c < 10));
    let (cs, cn) = (&fin).fold_flat((0i64, 0i64), |(s, n), ((c, _), _)| (s + c, n + 1));
    let v = drain(&fin);
    rows(v.into_iter().map(|(p, ((c, u), _))| {
        let u = u.unwrap_or([0; 2]);
        let nv = u[0] - u[1];
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(u[0]), V::I(u[1]), V::I(c), V::I(nv), V::F(es as f64 / en as f64)]);
        f.push(V::S(if score.get(p).unwrap() == top { "Top Performer" } else { "Regular Post" }));
        f.push(V::S(if nv > 0 { "Positive Feedback" } else if nv < 0 { "Negative Feedback" } else { "Neutral" }));
        f.push(V::S(if c * cn < cs { "Needs Attention" } else if c * cn > cs { "Engaged Discussion" } else { "Moderate Engagement" }));
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadge, MAX(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadge
//     FROM Badges b GROUP BY b.UserId),
// RecentPostHistory AS (SELECT ph.UserId, ph.PostId, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (10, 12) AND ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UsersWithCounts AS (SELECT upc.UserId, upc.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ub.GoldBadge, 0) AS GoldBadge, COALESCE(ub.SilverBadge, 0) AS SilverBadge,
//        upc.PostCount, upc.QuestionCount, upc.AnswerCount, MAX(rph.CreationDate) AS LastPostAction
//     FROM UserPostCounts upc LEFT JOIN UserBadges ub ON upc.UserId = ub.UserId LEFT JOIN RecentPostHistory rph ON upc.UserId = rph.UserId
//     GROUP BY upc.UserId, upc.DisplayName, ub.BadgeCount, ub.GoldBadge, ub.SilverBadge, upc.PostCount, upc.QuestionCount, upc.AnswerCount)
// SELECT uc.UserId, uc.DisplayName, uc.PostCount, uc.QuestionCount, uc.AnswerCount, uc.BadgeCount, CASE WHEN uc.GoldBadge = 1 THEN 'Yes' ELSE 'No' END AS HasGoldBadge,
//        CASE WHEN uc.SilverBadge = 1 THEN 'Yes' ELSE 'No' END AS HasSilverBadge, uc.LastPostAction, COUNT(DISTINCT c.Id) AS CommentCount
// FROM UsersWithCounts uc LEFT JOIN Comments c ON uc.UserId = c.UserId
// GROUP BY uc.UserId, uc.DisplayName, uc.PostCount, uc.QuestionCount, uc.AnswerCount, uc.BadgeCount, uc.GoldBadge, uc.SilverBadge, uc.LastPostAction
// ORDER BY uc.PostCount DESC, uc.QuestionCount DESC, uc.AnswerCount DESC;
fn q600(db: &'static So) -> String {
    let pt = &db.post.post_type_id;
    let upc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(pt).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + 1, a[1].max((c == 1) as i64), a[2].max((c == 2) as i64)]);
    let PostHistory { post_history_type_id: ht, creation_date: hd, user, .. } = &db.post_history;
    let rph = db.post_history.with(ht.is_in([10, 12]).and(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).group_by(user).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let cc = db.comment.group_by(&db.comment.user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&upc).and((&ub).opt()).and((&rph).opt()).and((&cc).opt()));
    rows(v.into_iter().map(|(u, (((a, b), l), c))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b[0]), V::S(if b[1] == 1 { "Yes" } else { "No" }), V::S(if b[2] == 1 { "Yes" } else { "No" }), ots(l), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.Score > 0 AND p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// UserRanks AS (SELECT u.Id AS UserId, COALESCE(SUM(b.Class), 0) AS TotalBadges, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// ActiveUsers AS (SELECT ur.UserId, u.DisplayName, ur.TotalBadges, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges FROM UserRanks ur INNER JOIN Users u ON ur.UserId = u.Id WHERE ur.TotalBadges > 0),
// PostHistoryEntries AS (SELECT ph.PostId, ph.UserId AS EditorId, ph.CreationDate AS EditDate, pt.Name AS PostHistoryType, ph.Comment
//     FROM PostHistory ph INNER JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id WHERE pt.Name IN ('Post Closed', 'Post Reopened')),
// FinalReport AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, au.DisplayName AS UserDisplayName, au.TotalBadges, ph.EditorId, ph.EditDate, ph.PostHistoryType
//     FROM RankedPosts rp JOIN ActiveUsers au ON rp.OwnerUserId = au.UserId LEFT JOIN PostHistoryEntries ph ON rp.PostId = ph.PostId WHERE rp.rn <= 3)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.ViewCount, fr.Score, fr.UserDisplayName, fr.TotalBadges, fr.EditorId, fr.EditDate, fr.PostHistoryType
// FROM FinalReport fr ORDER BY fr.Score DESC, fr.ViewCount DESC;
//
// CommentCount is never read. A CreationDate tie within an owner goes to the smaller post id.
fn q34625(db: &'static So) -> String {
    let Post { score, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(score.gt(0).and(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    let phe = history_of(db).select(Ident::<PostHistory>::new().with(htype_name(db).filt(|n: Str| n == "Post Closed" || n == "Post Reopened")));
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and((&tb).filt(|s| s > 0))).and(phe.opt())));
    rows(v.into_iter().map(|(p, ((u, b), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([user_col(db, u, "name"), V::I(b)]);
        f.extend(match h {
            Some(h) => [oint(db.post_history.user_id.get(h)), V::T(db.post_history.creation_date.get(h).unwrap()), V::S(htype_name(db).get(h).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, U.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY CASE WHEN U.Reputation >= 1000 THEN 'High' WHEN U.Reputation >= 100 THEN 'Medium' ELSE 'Low' END ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U),
// PostStats AS (SELECT P.Id AS PostId, P.Title, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(COALESCE(VB.BountyAmount, 0)) AS TotalBounty,
//        COUNT(DISTINCT PL.RelatedPostId) AS RelatedPostCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes VB ON P.Id = VB.PostId AND VB.VoteTypeId IN (8, 9) LEFT JOIN PostLinks PL ON P.Id = PL.PostId
//     WHERE P.CreationDate >= '2020-01-01' GROUP BY P.Id, P.Title),
// HighActivityUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Views DESC) AS ActivityRank FROM Users U WHERE U.Views > 1000),
// UserPosts AS (SELECT U.Id AS UserId, COUNT(P.Id) AS PostCount, SUM(COALESCE(C.CommentCount, 0)) AS TotalComments FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId GROUP BY U.Id),
// UserPostAnalytics AS (SELECT U.DisplayName, UR.Reputation, UR.ReputationRank, UP.PostCount, UP.TotalComments, PS.CommentCount AS PostCommentCount, PS.RelatedPostCount, PS.TotalBounty
//     FROM HighActivityUsers U JOIN UserReputation UR ON UR.UserId = U.Id JOIN UserPosts UP ON UP.UserId = U.Id
//     JOIN PostStats PS ON PS.PostId = (SELECT P.Id FROM Posts P WHERE P.OwnerUserId = U.Id ORDER BY P.CreationDate DESC LIMIT 1))
// SELECT UPA.DisplayName, UPA.Reputation, UPA.ReputationRank, UPA.PostCount, UPA.TotalComments, COALESCE(UPA.PostCommentCount, 0) AS AveragePostComments,
//        COALESCE(UPA.RelatedPostCount, 0) AS RelatedPostLinks, COALESCE(UPA.TotalBounty, 0) AS TotalBounty
// FROM UserPostAnalytics UPA WHERE UPA.ReputationRank = 1 ORDER BY UPA.TotalComments DESC, UPA.PostCount DESC;
//
// ReputationRank reads only Reputation, so the top user of each tier is picked first (a tie goes to the smaller user id). The correlated
// `ORDER BY P.CreationDate DESC LIMIT 1` is each user's newest post; a CreationDate tie goes to the smaller post id.
fn q24734(db: &'static So) -> String {
    let User { reputation, views, .. } = &db.user;
    let tier = |r: i64| if r >= 1000 { 0 } else if r >= 100 { 1 } else { 2 };
    let r1 = top_per(drain(reputation), |&(_, r)| tier(r), |&(u, r)| (Reverse(r), u), 1, false);
    let r1: MatSet<Id<User>> = rel(r1.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let hau: MatSet<Id<User>> = (&r1).with(views.gt(1000)).collect();
    let Post { creation_date, .. } = &db.post;
    let cpp = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let up = (&hau).group_by(Ident::<User>::new()).select(posts_of(db).select((&cpp).opt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(c) => [a[0] + 1, a[1] + c.unwrap_or(0)],
        None => a,
    });
    let lp = top_per(drain((&hau).select(posts_of(db))), |&(u, _)| u, |&(_, p)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let lp = rel(lp);
    let lp_idx: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&lp).map(|(u, _)| u).inv().select(&lp).collect();
    let lps: MatSet<Id<Post>> = (&lp).map(|(_, p)| p).collect();
    let since = (&lps).with(creation_date.ge(ts(2020, 1, 1, 0, 0, 0)));
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ps = since.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bv.opt()).and(links_of(db).opt())).fold([0i64; 2], |a, ((c, b), _)| {
        [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]
    });
    let rl = (&lps).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let v = drain((&hau).select(Ident::<User>::new().and(&up).and((&lp_idx).map(|(_, p)| p).select((&ps).and((&rl).opt())))));
    let mut v = v;
    v.sort_by_key(|&(_, ((_, a), _))| (Reverse(a[1]), Reverse(a[0])));
    rows(v.into_iter().map(|(_, ((u, a), (s, l)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(1), V::I(a[0]), V::I(a[1]), V::I(s[0]), V::I(l.unwrap_or(0)), V::I(s[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.ViewCount, p.PostTypeId, p.Score),
// PostHistoryDetails AS (SELECT ph.PostId, MIN(ph.CreationDate) AS FirstChangeDate, MAX(ph.CreationDate) AS LastChangeDate,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount FROM PostHistory ph GROUP BY ph.PostId),
// BadgeSummary AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges FROM Badges GROUP BY UserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(b.GoldBadges, 0) AS GoldBadgeCount, COALESCE(b.SilverBadges, 0) AS SilverBadgeCount,
//        COALESCE(b.BronzeBadges, 0) AS BronzeBadgeCount, pu.PostCount
//     FROM Users u LEFT JOIN (SELECT OwnerUserId AS UserId, COUNT(*) AS PostCount FROM Posts GROUP BY OwnerUserId) pu ON u.Id = pu.UserId LEFT JOIN BadgeSummary b ON u.Id = b.UserId)
// SELECT r.PostId, r.Title, r.ViewCount, r.RankScore, r.CommentCount, r.NetVotes, p.FirstChangeDate, p.LastChangeDate, p.CloseReopenCount, u.DisplayName, u.GoldBadgeCount,
//        u.SilverBadgeCount, u.BronzeBadgeCount
// FROM RankedPosts r JOIN PostHistoryDetails p ON r.PostId = p.PostId LEFT JOIN UserActivity u ON u.UserId IN (SELECT AcceptedAnswerId FROM Posts WHERE AcceptedAnswerId IS NOT NULL)
// WHERE r.RankScore = 1 AND r.NetVotes >= 10 AND (p.CloseReopenCount <= 2 OR p.CloseReopenCount IS NULL) ORDER BY r.ViewCount DESC, u.GoldBadgeCount DESC FETCH FIRST 10 ROWS ONLY;
//
// RankScore reads only Score, so each type's top post is picked first (a Score tie goes to the smaller post id) and the comment x vote product is driven for those.
// The ON clause names only u, so the posts are crossed with the users whose raw id is an AcceptedAnswerId; a GoldBadgeCount tie at the cut goes to the smaller user id.
fn q23523(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, accepted_answer_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]);
    let PostHistory { post, creation_date: hd, post_history_type_id: ht, .. } = &db.post_history;
    let phd = db.post_history.group_by(post).select(hd.and(ht)).fold((i64::MAX, i64::MIN, 0i64), |(lo, hi, n), (d, t)| (lo.min(d), hi.max(d), n + matches!(t, 10 | 11) as i64));
    let posts = drain((&s).filt(|a| a[1] >= 10).and((&phd).filt(|h| h.2 <= 2)));
    let acc: MatSet<i64> = db.post.select(accepted_answer_id).collect();
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ua = drain(db.user.with((&db.user.origid).select(&acc)).select((&bs).opt()));
    let v = cross_top(posts, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, ua, |&(u, b)| (Reverse(b.map_or(0, |b| b[0])), u), 10);
    rows(v.into_iter().map(|((p, (a, h)), (u, b))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(1), V::I(a[0]), V::I(a[1]), V::T(h.0), V::T(h.1), V::I(h.2), user_col(db, u, "name")]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS OwnerRank,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalQuestions, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostLinkCount AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS LinkCount FROM PostLinks pl GROUP BY pl.PostId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, COALESCE(pl.LinkCount, 0) AS LinkCount, COALESCE(us.TotalQuestions, 0) AS UserQuestions,
//        COALESCE(us.GoldBadges, 0) AS GoldBadges, COALESCE(us.SilverBadges, 0) AS SilverBadges, COALESCE(us.BronzeBadges, 0) AS BronzeBadges
//     FROM RankedPosts rp LEFT JOIN UserStatistics us ON rp.OwnerUserId = us.UserId LEFT JOIN PostLinkCount pl ON rp.PostId = pl.PostId WHERE rp.OwnerRank <= 3)
// SELECT fp.PostId, fp.Title, fp.CreationDate, COALESCE(fp.Score, 0) AS TotalScore, COALESCE(fp.LinkCount, 0) AS TotalLinks, fp.ViewCount, fp.AnswerCount,
//        CASE WHEN fp.GoldBadges > 0 THEN 'Gold Member' WHEN fp.SilverBadges > 0 THEN 'Silver Member' WHEN fp.BronzeBadges > 0 THEN 'Bronze Member' ELSE 'New Member' END AS MembershipStatus,
//        CONCAT('User has earned ', COALESCE(fp.GoldBadges, 0), ' Gold, ', COALESCE(fp.SilverBadges, 0), ' Silver, and ', COALESCE(fp.BronzeBadges, 0), ' Bronze badges.') AS BadgeDetails
// FROM FilteredPosts fp ORDER BY fp.Score DESC, fp.ViewCount DESC LIMIT 50;
//
// UserStatistics is joined to the question owners only, so its posts x badges product is driven for those users alone. The ownerless questions are one
// partition (NULL OwnerUserId). A Score tie within an owner goes to the smaller post id.
fn q2123(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let top = top_per(drain(qs().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = qs().select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, (_, c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let lc = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(owner_user.select(&us).opt().and((&lc).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, (b, l))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(l.unwrap_or(0)));
        f.extend(post_fields(db, p, &["views", "answers"]));
        f.push(V::S(if b[0] > 0 { "Gold Member" } else if b[1] > 0 { "Silver Member" } else if b[2] > 0 { "Bronze Member" } else { "New Member" }));
        f.push(V::Owned(format!("User has earned {} Gold, {} Silver, and {} Bronze badges.", b[0], b[1], b[2])));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvotesReceived,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvotesReceived, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostDetail AS (SELECT p.Id AS PostId, p.Title, pt.Name AS PostType, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 1 THEN ph.CreationDate END) AS InitialTitleDate, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS ClosedPost, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     GROUP BY p.Id, p.Title, pt.Name, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// FinalStats AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.UpvotesReceived, us.DownvotesReceived, ps.PostId, ps.Title, ps.PostType, ps.CreationDate, ps.ViewCount, ps.Score,
//        ps.CommentCount, ps.InitialTitleDate, ps.ClosedPost, ROW_NUMBER() OVER (PARTITION BY us.UserId ORDER BY ps.CreationDate DESC) AS UserPostRank
//     FROM UserStats us JOIN PostDetail ps ON us.UserId = ps.OwnerUserId)
// SELECT f.UserId, f.DisplayName, f.Reputation, f.UpvotesReceived, f.DownvotesReceived, f.PostId, f.Title, f.PostType, f.CreationDate, f.ViewCount, f.Score, f.CommentCount,
//        f.InitialTitleDate, f.ClosedPost, CASE WHEN f.UserPostRank = 1 THEN 'Most Recent Post' ELSE NULL END AS RankDescription
// FROM FinalStats f WHERE f.Reputation > 500 AND (f.ClosedPost IS NULL OR f.Score > 0) ORDER BY f.Reputation DESC, f.UserId, f.CreationDate DESC LIMIT 100 OFFSET 0;
//
// The ORDER BY leads with the user's own columns, so the users are taken in that order until their kept posts fill the LIMIT, and the posts x votes x badges
// product is driven for those users alone. UserPostRank is over all of a user's posts, before the WHERE; a CreationDate tie goes to the smaller post id.
fn q21566(db: &'static So) -> String {
    let Post { owner_user, score, creation_date, .. } = &db.post;
    let PostHistory { post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let pd = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(ht.and(hd)).opt()))
        .fold((0i64, i64::MIN, false), |(n, m, cl), (c, h)| (n + c.is_some() as i64, match h { Some((1, d)) => m.max(d), _ => m }, cl || matches!(h, Some((10, _)))));
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(500));
    let qp: MatSet<Id<Post>> = db.post.with(owner_user.select(rich)).with((&pd).and(score).filt(|((_, _, cl), s): ((i64, i64, bool), i64)| !cl || s > 0)).collect();
    let per = (&qp).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let uord = top_n(drain(&per), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), db.user.origid.get(u).unwrap()), 0);
    let (mut need, mut k) = (100i64, 0);
    while k < uord.len() && need > 0 {
        need -= uord[k].1;
        k += 1;
    }
    let picked: MatSet<Id<User>> = rel(uord[..k].iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&picked)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| [a[0] + (p.flatten() == Some(2)) as i64, a[1] + (p.flatten() == Some(3)) as i64]);
    let newest = top_per(drain(db.post.with(owner_user.select(Ident::<User>::new().with(&picked))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let newest: MatSet<Id<Post>> = rel(newest.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&qp).select(owner_user.select(Ident::<User>::new().and(&us)).and(&pd).and(Ident::<Post>::new().with(&newest).opt())));
    let v = top_n(v, |&(p, (((u, _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), db.user.origid.get(u).unwrap(), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (((u, a), (n, it, cl)), nw))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["id", "title", "type", "created", "views", "score"]));
        f.extend([V::I(n), tmax(it), if cl { V::I(1) } else { V::Null }, if nw.is_some() { V::S("Most Recent Post") } else { V::Null }]);
        row(f)
    }))
}

// Rewritten (rewrites/26401.sql): the RowNum window is tie-broken on upa.PostId (carried through UserPostActivity), and `SELECT *` is spelled out without it.
// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(vb.VoteCount, 0)) AS UpVotes,
//        SUM(COALESCE(vd.VoteCount, 0)) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) vb ON p.Id = vb.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) vd ON p.Id = vd.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, EXTRACT(MONTH FROM p.CreationDate) AS MonthCreated, EXTRACT(YEAR FROM p.CreationDate) AS YearCreated,
//        COUNT(c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS EditCount, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Id END) AS CloseCount,
//        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.Id END) AS ReopenCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate),
// UserPostActivity AS (SELECT ur.DisplayName, ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.EditCount, ps.CloseCount, ps.ReopenCount, ps.TotalUpVotes, ps.TotalDownVotes,
//        ur.Reputation FROM PostStatistics ps JOIN Users u ON ps.PostId = u.Id JOIN UserReputation ur ON u.Id = ur.UserId),
// FinalStatistics AS (SELECT upa.DisplayName, upa.PostId, upa.Title, upa.CreationDate, upa.CommentCount, upa.EditCount, upa.CloseCount, upa.ReopenCount, upa.TotalUpVotes,
//        upa.TotalDownVotes, upa.Reputation, ROW_NUMBER() OVER (PARTITION BY upa.DisplayName ORDER BY upa.CreationDate DESC, upa.PostId) AS RowNum FROM UserPostActivity upa)
// SELECT DisplayName, Title, CreationDate, CommentCount, EditCount, CloseCount, ReopenCount, TotalUpVotes, TotalDownVotes, Reputation, RowNum
// FROM FinalStatistics WHERE RowNum = 1 ORDER BY Reputation DESC, TotalUpVotes DESC;
//
// `ps.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. UserReputation has one row per user and only its DisplayName and Reputation
// are read, so its aggregates are not computed. RowNum reads only the user's name and the post's date and id, so the winners are picked first and the
// comments x history x votes product is driven for those alone.
fn q26401(db: &'static So) -> String {
    let Post { origid, creation_date, .. } = &db.post;
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pm = drain(db.post.select(origid.select(&uidx).select(&db.user.display_name)));
    let top = top_per(pm, |&(_, n)| n, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), origid.get(p).unwrap()), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, ((c, _), t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let hs = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 10) as i64, a[2] + (t == 11) as i64],
        None => a,
    });
    let v = drain((&ps).and(&hs).and(origid.select(&uidx)));
    rows(v.into_iter().map(|(p, ((a, h), u))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(h[0]), V::I(h[1]), V::I(h[2]), V::I(a[1]), V::I(a[2]), user_col(db, u, "rep"), V::I(1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, p.PostTypeId,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT c.PostId) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostHistoryDetails AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate,
//        MIN(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenedDate FROM PostHistory ph GROUP BY ph.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, ra.DisplayName AS OwnerDisplayName, ua.UserId, ua.CommentCount, ua.UpVotes, ua.DownVotes, pHd.ClosedDate, pHd.ReopenedDate,
//        CASE WHEN pHd.ClosedDate IS NOT NULL THEN CASE WHEN pHd.ReopenedDate IS NOT NULL THEN 'Reopened' ELSE 'Closed' END ELSE 'Active' END AS PostStatus
//     FROM RankedPosts rp JOIN Users ra ON rp.OwnerUserId = ra.Id JOIN UserActivity ua ON ua.UserId = ra.Id LEFT JOIN PostHistoryDetails pHd ON rp.PostId = pHd.PostId
//     WHERE rp.ScoreRank <= 5)
// SELECT PostId, Title, OwnerDisplayName, CommentCount, UpVotes, DownVotes, PostStatus,
//        CASE WHEN PostStatus = 'Closed' THEN 'This post is closed.' WHEN PostStatus = 'Reopened' THEN 'This post is reopened.' ELSE 'This post is active.' END AS PostMessage
// FROM FinalResults ORDER BY UpVotes DESC, CommentCount DESC;
//
// UserActivity is joined to the ranked posts' owners only, so its comments x votes x badges product is driven for those users alone.
fn q20686(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()).and(badges_of(db).opt()))
        .fold([0i64; 2], |a, ((_, t), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cd = (&owners).group_by(Ident::<User>::new()).select(comments_by(db).select(&db.comment.post_id)).count_distinct();
    let PostHistory { post, post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.group_by(post).select(ht.and(hd)).fold((i64::MIN, i64::MAX), |(c, r), (t, d)| (if t == 10 { c.max(d) } else { c }, if t == 11 { r.min(d) } else { r }));
    let mut v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ua).and((&cd).opt())).and((&phd).opt())));
    v.sort_by_key(|&(_, (((_, a), c), _))| (Reverse(a[0]), Reverse(c.unwrap_or(0))));
    rows(v.into_iter().map(|(p, (((u, a), c), h))| {
        let st = match h {
            Some((c, r)) if c != i64::MIN => if r != i64::MAX { "Reopened" } else { "Closed" },
            _ => "Active",
        };
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::S(st)]);
        f.push(V::S(if st == "Closed" { "This post is closed." } else if st == "Reopened" { "This post is reopened." } else { "This post is active." }));
        row(f)
    }))
}

// WITH UserVoteCount AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(v.Id) AS TotalVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseOpenCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankPerUser
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// TopPosts AS (SELECT pm.PostId, pm.Title, pm.Score, pm.ViewCount, pm.CommentCount, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName,
//        CASE WHEN pm.RankPerUser <= 3 THEN 'Top' ELSE 'Normal' END AS PostCategory
//     FROM PostMetrics pm LEFT JOIN Users u ON u.Id = (SELECT AcceptedAnswerId FROM Posts WHERE Id = pm.PostId) WHERE pm.Score > 10),
// RecentActivity AS (SELECT p.Id, MAX(ph.CreationDate) AS LastActivityDate FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, tp.OwnerDisplayName, ra.LastActivityDate,
//        CASE WHEN uc.UpVotes IS NULL AND uc.DownVotes IS NULL THEN 0 ELSE COALESCE(uc.UpVotes, 0) - COALESCE(uc.DownVotes, 0) END AS NetVotes,
//        CASE WHEN tp.PostCategory = 'Top' THEN 'Highly Active' ELSE 'Regular Activity' END AS ActivityLevel
// FROM TopPosts tp LEFT JOIN UserVoteCount uc ON uc.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) LEFT JOIN RecentActivity ra ON ra.Id = tp.PostId
// WHERE EXISTS (SELECT 1 FROM Comments c WHERE c.PostId = tp.PostId AND c.CreationDate >= (CURRENT_TIMESTAMP - INTERVAL '30 days'))
// ORDER BY tp.Score DESC, ra.LastActivityDate DESC LIMIT 100;
//
// `u.Id = AcceptedAnswerId` joins a user id to a post id, so it goes through the raw ids. RankPerUser reads only Score, so it is taken over all posts
// first (a Score tie goes to the smaller post id); the comment x history product is driven only for the posts the EXISTS keeps.
fn q22868(db: &'static So) -> String {
    let Post { score, owner_user, accepted_answer_id, .. } = &db.post;
    let cut = add_days(utc_to_ny(now_utc()), -30);
    let recent: MatSet<Id<Post>> = db.comment.with((&db.comment.creation_date).ge(cut)).select(&db.comment.post).collect();
    let tp = || (&recent).with(score.gt(10));
    let top3 = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let top3: MatSet<Id<Post>> = rel(top3.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = tp().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold(0i64, |n, t| n + (t == 2) as i64 - (t == 3) as i64);
    let ra = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let v = drain((&cc).and(accepted_answer_id.select(&uidx).opt()).and(owner_user.select((&uv).opt()).opt()).and((&ra).opt()).and(Ident::<Post>::new().with(&top3).opt()));
    let v = top_n(v, |&(p, ((_, r), _))| (Reverse(score.get(p).unwrap()), r.is_none(), Reverse(r), p), 100);
    rows(v.into_iter().map(|(p, ((((c, a), u), r), t))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), a.map_or(V::S("Anonymous"), |a| user_col(db, a, "name")), ots(r), V::I(u.flatten().unwrap_or(0))]);
        f.push(V::S(if t.is_some() { "Highly Active" } else { "Regular Activity" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId IN (2, 3)), 0) AS TotalVotes,
//        ARRAY_LENGTH(string_to_array(COALESCE(p.Tags, ''), '><'), 1) AS TagCount
//     FROM Posts p WHERE p.ViewCount IS NOT NULL AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostStats AS (SELECT r.OwnerUserId, AVG(r.Score) AS AvgScore, SUM(r.ViewCount) AS TotalViews, COUNT(*) AS PostsCount, COUNT(CASE WHEN r.Rank = 1 THEN 1 END) AS AcceptedAnswers
//     FROM RankedPosts r WHERE r.Rank <= 10 GROUP BY r.OwnerUserId),
// UserWithBadges AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(b.Name, 'No Badge') AS BadgeName, b.Class AS BadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users)),
// RecentCloseReasons AS (SELECT ph.PostId, ph.Comment, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10
//     AND ph.CreationDate >= DATE_TRUNC('month', cast('2024-10-01 12:34:56' as timestamp)) GROUP BY ph.PostId, ph.Comment),
// FinalResults AS (SELECT ub.UserId, ub.DisplayName, ps.AvgScore, ps.TotalViews, ps.PostsCount, ps.AcceptedAnswers,
//        CASE WHEN rc.CloseCount IS NULL THEN 'Not Closed' ELSE 'Closed' END AS RecentCloseStatus, rc.Comment AS RecentCloseComment
//     FROM UserWithBadges ub JOIN PostStats ps ON ub.UserId = ps.OwnerUserId LEFT JOIN RecentCloseReasons rc ON rc.PostId = (SELECT MAX(Id) FROM Posts WHERE OwnerUserId = ub.UserId))
// SELECT UserId, DisplayName, AvgScore, TotalViews, PostsCount, AcceptedAnswers, RecentCloseStatus, RecentCloseComment FROM FinalResults WHERE AvgScore > 0
// ORDER BY AvgScore DESC, TotalViews DESC LIMIT 100;
//
// TotalVotes and TagCount are never read. The uncorrelated AVG(Reputation) is computed once. Every user row repeats once per badge (UserWithBadges).
// `SELECT MAX(Id) FROM Posts WHERE OwnerUserId = ub.UserId` is a per-owner fold of the raw post id, joined back through the raw ids.
fn q21519(db: &'static So) -> String {
    let Post { view_count, creation_date, post_type_id, score, owner_user, origid, .. } = &db.post;
    let v = drain(db.post.with(view_count).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let key = |p: Id<Post>| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p);
    let top = top_per(v.clone(), |&(_, t)| t, |&(p, _)| key(p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let r1 = top_per(v, |&(_, t)| t, |&(p, _)| key(p), 1, false);
    let r1: MatSet<Id<Post>> = rel(r1.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .group_by(owner_user)
        .select(score.and(view_count).and(Ident::<Post>::new().with(&r1).opt()))
        .fold([0i64; 4], |a, ((s, w), f)| [a[0] + s, a[1] + w, a[2] + 1, a[3] + f.is_some() as i64]);
    let (rs, rn) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let avgr = rs as f64 / rn as f64;
    let PostHistory { post, post_history_type_id: ht, creation_date: hd, comment, .. } = &db.post_history;
    let rc = db.post_history.with(ht.eq(10).and(hd.ge(trunc_month(ts(2024, 10, 1, 12, 34, 56))))).group_by(post.and(comment.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rcv = rel(drain(&rc));
    let rc_by: HashIdx<Id<Post>, ((Id<Post>, Option<Str>), i64)> = (&rcv).map(|((p, _), _)| p).inv().select(&rcv).collect();
    let maxid = db.post.group_by(owner_user).select(origid).fold(i64::MIN, |m, i| m.max(i));
    let pidx: HashIdx<i64, Id<Post>> = origid.inv().collect();
    let users = db.user.with((&db.user.reputation).filt(move |r: i64| r as f64 > avgr));
    let v = drain(users.select(badges_of(db).opt().and((&ps).filt(|a| a[0] > 0)).and((&maxid).select(&pidx).select(&rc_by).opt())));
    let v = top_n(v, |&(u, ((_, a), _))| (Reverse(fkey(a[0] as f64 / a[2] as f64)), Reverse(a[1]), u), 100);
    rows(v.into_iter().map(|(u, ((_, a), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([avg(a[0], a[2]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(match c {
            Some(((_, cm), _)) => [V::S("Closed"), ostr(cm)],
            None => [V::S("Not Closed"), V::Null],
        });
        row(f)
    }))
}

// WITH RecursiveTopPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalQuestions, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, (SELECT COUNT(*) FROM Comments c WHERE c.UserId = u.Id) AS TotalComments,
//        (SELECT SUM(pw.Score) FROM Posts pw WHERE pw.OwnerUserId = u.Id) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ue.UserId, ue.DisplayName, ue.TotalQuestions, ue.TotalUpvotes, ue.TotalDownvotes, ue.TotalComments, ue.TotalScore,
//        ROW_NUMBER() OVER (ORDER BY ue.TotalScore DESC) AS UserRank FROM UserEngagement ue WHERE ue.TotalScore IS NOT NULL),
// FilteredPosts AS (SELECT p.Id, p.Title, p.Tags, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseVotes,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Tags)
// SELECT r.Id AS PostId, r.Title AS TopPostTitle, r.Score AS TopPostScore, r.ViewCount AS TopPostViews, r.AnswerCount AS TopPostAnswers, tu.DisplayName AS TopUserDisplayName,
//        tu.TotalQuestions AS TopUserQuestions, tu.TotalUpvotes AS TopUserUpvotes, tu.TotalDownvotes AS TopUserDownvotes, f.CommentCount AS RelatedPostComments,
//        f.CloseVotes AS TotalCloseVotes, f.ReopenVotes AS TotalReopenVotes
// FROM RecursiveTopPosts r JOIN TopUsers tu ON r.Rank <= 10 LEFT JOIN FilteredPosts f ON r.Id = f.Id ORDER BY r.Score DESC, tu.TotalScore DESC;
//
// Not recursive despite the name. The ON clause names only r, so the ten top questions (a Score tie at Rank 10 goes to the smaller post id) are crossed with
// every user who owns a post (TotalScore IS NOT NULL). TotalComments and UserRank are never read.
fn q34845(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let top = rel(top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 10));
    let tps: MatSet<Id<Post>> = (&top).map(|(p, _)| p).collect();
    let PostHistory { post_history_type_id: ht, .. } = &db.post_history;
    let fp = (&tps).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).select(ht).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(11)) as i64]);
    let fc = (&tps).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ue = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let tsc = db.post.group_by(&db.post.owner_user).select(score).fold(0i64, |s, x| s + x);
    let tu = rel(drain((&ue).and(&tsc).and(user_distinct_posts(db))));
    let tr = rel(drain((&top).map(|(p, _)| p).select(Ident::<Post>::new().and((&fp).and(&fc).opt()))));
    let mut v = Vec::new();
    (&tr).cross(&tu).drive(|_, ((_, (p, f)), (u, ((a, s), n)))| v.push((p, f, u, a, s, n)));
    rows(v.into_iter().map(|(p, f, u, a, _, n)| {
        let mut r = post_fields(db, p, &["id", "title", "score", "views", "answers"]);
        r.extend([user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1])]);
        r.extend(match f {
            Some((h, c)) => [V::I(c), V::I(h[0]), V::I(h[1])],
            None => [V::Null, V::Null, V::Null],
        });
        row(r)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankByScoreViews
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// UserEngagement AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// MaxEngagement AS (SELECT UserId, PostCount, UpvoteCount, DownvoteCount, RANK() OVER (ORDER BY PostCount DESC, UpvoteCount DESC) AS EngagementRank FROM UserEngagement),
// BadgedUsers AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId),
// PostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// FinalReport AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, pe.PostCount, pe.UpvoteCount, pe.DownvoteCount, bu.BadgeCount, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        CASE WHEN COALESCE(pc.CommentCount, 0) > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus
//     FROM RankedPosts rp LEFT JOIN MaxEngagement pe ON pe.UserId = rp.PostId LEFT JOIN BadgedUsers bu ON bu.UserId = rp.PostId LEFT JOIN PostComments pc ON pc.PostId = rp.PostId
//     WHERE rp.RankByScoreViews <= 5)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.Score, fr.PostCount, fr.UpvoteCount, fr.DownvoteCount, fr.BadgeCount, fr.CommentCount, fr.CommentStatus
// FROM FinalReport fr ORDER BY fr.Score DESC, fr.CreationDate DESC;
//
// `pe.UserId = rp.PostId` and `bu.UserId = rp.PostId` join a user id to a post id, so they go through the raw ids, and UserEngagement's posts x votes
// product is driven for those users alone. EngagementRank is never read. A (Score, ViewCount) tie at rank 5 goes to the smaller post id.
fn q22149(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let us: MatSet<Id<User>> = (&tp).select(origid.select(&uidx)).collect();
    let ue = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let bu = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pe = Ident::<User>::new().and(&ue).and(user_distinct_posts(db)).and((&bu).opt());
    let v = drain((&pc).and(origid.select(&uidx).select(pe).opt()));
    rows(v.into_iter().map(|(p, (c, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(match u {
            Some((((_, a), n), b)) => [V::I(n), V::I(a[0]), V::I(a[1]), oint(b)],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([V::I(c), V::S(if c > 0 { "Has Comments" } else { "No Comments" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(DISTINCT V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVoteCount, COALESCE(MAX(PH.CreationDate), P.CreationDate) AS LastEdited, U.Reputation AS OwnerReputation
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId IN (4, 5, 6)
//     WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, U.Reputation),
// ClosedPostStats AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.ViewCount, PS.CommentCount, PS.UpVoteCount, PS.DownVoteCount, PS.LastEdited, PS.OwnerReputation,
//        CASE WHEN PH.PostHistoryTypeId = 10 THEN 'Closed' WHEN PH.PostHistoryTypeId = 11 THEN 'Reopened' ELSE 'Open' END AS PostStatus
//     FROM PostStatistics PS LEFT JOIN PostHistory PH ON PS.PostId = PH.PostId AND PH.PostHistoryTypeId IN (10, 11) WHERE PS.CommentCount > 0),
// SelectedUser AS (SELECT UR.UserId, UR.Reputation, CASE WHEN EXISTS (SELECT 1 FROM ClosedPostStats CPS WHERE CPS.OwnerReputation > UR.Reputation) THEN 'Inferior' ELSE 'Superior' END AS ReputationRelation
//     FROM UserReputation UR WHERE UR.Reputation > 1000),
// FinalResults AS (SELECT CPS.*, SU.ReputationRelation FROM ClosedPostStats CPS JOIN SelectedUser SU ON CPS.OwnerReputation = SU.Reputation)
// SELECT F.*, (CASE WHEN F.PostStatus = 'Closed' THEN 'This post has been closed due to reasons outlined above.' ELSE 'This post is currently active and has received activity.' END) AS StatusComment
// FROM FinalResults F ORDER BY F.LastEdited DESC, F.ViewCount DESC LIMIT 100 OFFSET 0;
//
// Every aggregate of PostStatistics is a COUNT(DISTINCT) or a MAX, which the other joins cannot change, so each is its own fold. The uncorrelated-after-all
// EXISTS (some ClosedPostStats row has a larger OwnerReputation) is `Reputation < MAX(OwnerReputation)`. `CPS.OwnerReputation = SU.Reputation` joins on the value,
// so a post repeats once per user of that reputation.
fn q24000(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let le = recent().group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(ht.is_in([4, 5, 6]))).select(hd).opt()).fold(i64::MIN, |m, d| m.max(d.unwrap_or(i64::MIN)));
    let cr = history_of(db).select(Ident::<PostHistory>::new().with(ht.is_in([10, 11]))).select(ht);
    let cps = (&cc).and(&vc).and(&le).and(owner_user.select(&db.user.reputation).opt()).and(cr.opt());
    let maxrep = (&cps).fold_flat(i64::MIN, |m, (((_, _), r), _)| m.max(r.unwrap_or(i64::MIN)));
    let su: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation).inv().collect();
    let v = drain((&cps).and(owner_user.select(&db.user.reputation).select(&su)));
    let v = top_n(v, |&(p, (((((_, _), l), _), _), _))| {
        let w = view_count.get(p);
        let l = if l == i64::MIN { creation_date.get(p).unwrap() } else { l };
        (Reverse(l), w.is_none(), Reverse(w), p)
    }, 100);
    rows(v.into_iter().map(|(p, (((((c, u), l), r), t), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        let l = if l == i64::MIN { creation_date.get(p).unwrap() } else { l };
        let st = match t {
            Some(10) => "Closed",
            Some(11) => "Reopened",
            _ => "Open",
        };
        f.extend([V::I(c), V::I(u[0]), V::I(u[1]), V::T(l), oint(r), V::S(st), V::S(if r.unwrap() < maxrep { "Inferior" } else { "Superior" })]);
        f.push(V::S(if st == "Closed" { "This post has been closed due to reasons outlined above." } else { "This post is currently active and has received activity." }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, u.DisplayName AS OwnerDisplayName,
//        CASE WHEN rp.ViewCount = 0 THEN 'No views' WHEN rp.ViewCount > 100 THEN 'High views' ELSE 'Moderate views' END AS ViewCategory
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.PostRank <= 5),
// PostStats AS (SELECT fp.PostId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount, fp.ViewCategory, MIN(ph.CreationDate) AS FirstHistoryDate, MAX(ph.CreationDate) AS LastHistoryDate
//     FROM FilteredPosts fp LEFT JOIN Comments c ON c.PostId = fp.PostId LEFT JOIN Votes v ON v.PostId = fp.PostId LEFT JOIN PostHistory ph ON ph.PostId = fp.PostId
//     GROUP BY fp.PostId, fp.ViewCategory),
// FinalResults AS (SELECT s.PostId, fp.Title, fp.CreationDate, fp.ViewCount, s.CommentCount, s.UpvoteCount, s.DownvoteCount, s.ViewCategory,
//        CASE WHEN s.CommentCount > 0 THEN 'Commented' ELSE 'Not commented' END AS CommentStatus, CASE WHEN s.FirstHistoryDate IS NULL THEN 'No history' ELSE 'Has history' END AS HistoryStatus,
//        EXTRACT(YEAR FROM AGE(s.FirstHistoryDate)) AS YearsSinceFirstHistory FROM PostStats s JOIN FilteredPosts fp ON s.PostId = fp.PostId)
// SELECT PostId, Title, CreationDate, ViewCount, CommentCount, UpvoteCount, DownvoteCount, ViewCategory, CommentStatus, HistoryStatus, YearsSinceFirstHistory
// FROM FinalResults WHERE YearsSinceFirstHistory > 2 ORDER BY UpvoteCount DESC, ViewCount DESC LIMIT 50;
//
// AGE(ts) is measured from CURRENT_DATE (local midnight), so YearsSinceFirstHistory depends on the day it runs. A (Score, CreationDate) tie within an owner
// goes to the smaller post id.
fn q23124(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| {
        (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let today = current_date();
    let tod = |t: i64| t - trunc_day(t);
    let age = move |t: i64| year(today) - year(t) - ((month(today), day(today), 0) < (month(t), day(t), tod(t))) as i64;
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold([0, 0, 0, i64::MAX], |a, ((c, t), h)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3].min(h.unwrap_or(i64::MAX))]);
    let v = drain((&ps).filt(move |a| a[3] != i64::MAX && age(a[3]) > 2));
    let v = top_n(v, |&(p, a)| {
        let w = view_count.get(p);
        (Reverse(a[1]), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, a)| {
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(match w {
            Some(0) => "No views",
            Some(w) if w > 100 => "High views",
            _ => "Moderate views",
        }));
        f.extend([V::S(if a[0] > 0 { "Commented" } else { "Not commented" }), V::S("Has history"), V::I(age(a[3]))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostInteractions AS (SELECT p.Id AS PostId, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(v.UpVoteCount, 0) AS UpVoteCount, COALESCE(v.DownVoteCount, 0) AS DownVoteCount,
//        COALESCE(ph.ClosedCount, 0) AS ClosedCount
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Votes GROUP BY PostId) v
//       ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS ClosedCount FROM PostHistory WHERE PostHistoryTypeId = 10 GROUP BY PostId) ph ON p.Id = ph.PostId)
// SELECT up.DisplayName, up.Reputation, up.TotalBounty, up.BadgeCount, pp.Title, pp.CreationDate AS PostDate, pp.Score, pp.ViewCount, pp.AnswerCount, pp.CommentCount,
//        pi.CommentCount AS InteractionsCommentCount, pi.UpVoteCount, pi.DownVoteCount, pi.ClosedCount,
//        CASE WHEN pi.ClosedCount > 0 THEN 'Closed' WHEN pp.AnswerCount > 0 AND pp.Score IS NOT NULL THEN 'Answered' ELSE 'Unanswered' END AS PostStatus
// FROM UserStats up INNER JOIN RankedPosts pp ON up.UserId = pp.Id LEFT JOIN PostInteractions pi ON pp.Id = pi.PostId WHERE pp.PostRank <= 5
// ORDER BY up.Reputation DESC, pp.CreationDate DESC;
//
// `up.UserId = pp.Id` joins a user id to a post id, so it goes through the raw ids, and UserStats' votes x badges product is driven for those users alone.
// The ownerless questions are one partition (NULL OwnerUserId). A CreationDate tie within an owner goes to the smaller post id.
fn q148(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, origid, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let us: MatSet<Id<User>> = (&tp).select(origid.select(&uidx)).collect();
    let ust = (&us).group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt())).fold(0i64, |s, (b, _)| s + b.flatten().unwrap_or(0));
    let bc = (&us).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id: ht, .. } = &db.post_history;
    let cl = db.post_history.with(ht.eq(10)).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(origid.select(&uidx).select(Ident::<User>::new().and(&ust).and(&bc)).and((&cc).opt()).and((&vc).opt()).and((&cl).opt())));
    rows(v.into_iter().map(|(p, (((((u, b), n), c), vv), k))| {
        let (c, vv, k) = (c.unwrap_or(0), vv.unwrap_or([0; 2]), k.unwrap_or(0));
        let ac = db.post.answer_count.get(p);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(n)]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views", "answers", "comments"]));
        f.extend([V::I(c), V::I(vv[0]), V::I(vv[1]), V::I(k)]);
        f.push(V::S(if k > 0 { "Closed" } else if ac.map_or(false, |a| a > 0) { "Answered" } else { "Unanswered" }));
        row(f)
    }))
}

// WITH RECURSIVE UserVoteCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, DisplayName, TotalVotes, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalVotes DESC) AS VoteRank FROM UserVoteCounts),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(voteCounts.UpVotes, 0) AS UpVotes, COALESCE(voteCounts.DownVotes, 0) AS DownVotes,
//        COUNT(c.Id) AS CommentCount, COUNT(DISTINCT b.UserId) AS BadgeCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) AS voteCounts
//       ON p.Id = voteCounts.PostId
//     WHERE p.OwnerUserId IS NOT NULL GROUP BY p.Id, p.Title, p.CreationDate, p.Score, voteCounts.UpVotes, voteCounts.DownVotes),
// RankedPosts AS (SELECT PostId, Title, CreationDate, Score, UpVotes, DownVotes, CommentCount, BadgeCount, RANK() OVER (ORDER BY Score DESC, CreationDate DESC) AS PostRank FROM PostStatistics),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.UpVotes, rp.DownVotes, rp.CommentCount, rp.BadgeCount FROM RankedPosts rp WHERE rp.PostRank <= 50)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.UpVotes, fp.DownVotes, fp.CommentCount, u.DisplayName AS PostOwner,
//        CASE WHEN fp.BadgeCount > 0 THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus, CASE WHEN fp.Score > 10 THEN 'High Score' ELSE 'Low Score' END AS ScoreCategory
// FROM FilteredPosts fp LEFT JOIN Users u ON fp.PostId = u.Id ORDER BY fp.Score DESC, fp.CreationDate DESC;
//
// WITH RECURSIVE, but no CTE refers to itself; TopUsers is never read. PostRank reads only Score and CreationDate, so the top 50 posts are picked first and the
// votes x comments x badges product is driven for those alone. `fp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q31926(db: &'static So) -> String {
    let Post { owner_user, score, creation_date, origid, .. } = &db.post;
    let r = ranked(drain(db.post.with(owner_user).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap())), false);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 50).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt().and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt())).fold(0i64, |n, ((_, c), _)| n + c.is_some() as i64);
    let bd = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).select(&db.badge.user)).count_distinct();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&vc).and(&cc).and((&bd).opt()).and(origid.select(&uidx).opt()));
    rows(v.into_iter().map(|(p, (((a, c), b), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), u.map_or(V::Null, |u| user_col(db, u, "name"))]);
        f.push(V::S(if b.unwrap_or(0) > 0 { "Has Badges" } else { "No Badges" }));
        f.push(V::S(if score.get(p).unwrap() > 10 { "High Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RecursiveBadgeCounts AS (SELECT UserId, COUNT(Id) AS BadgeTotal, MAX(Date) AS LastBadgeDate FROM Badges GROUP BY UserId),
// RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days'),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(bc.BadgeTotal, 0) AS TotalBadges, COALESCE(np.RecentPostCount, 0) AS RecentPostCount,
//        COALESCE(AP.RecentAcceptedCount, 0) AS AcceptedAnswers
//     FROM Users u LEFT JOIN RecursiveBadgeCounts bc ON u.Id = bc.UserId
//     LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS RecentPostCount FROM RecentPosts GROUP BY OwnerUserId) np ON u.Id = np.OwnerUserId
//     LEFT JOIN (SELECT p.OwnerUserId, COUNT(*) AS RecentAcceptedCount FROM Posts p WHERE AcceptedAnswerId IS NOT NULL AND CreationDate > cast('2024-10-01' as date) - INTERVAL '90 days'
//                GROUP BY p.OwnerUserId) AP ON u.Id = AP.OwnerUserId),
// PostVoteStatistics AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT us.UserId, us.DisplayName, us.TotalBadges, us.RecentPostCount, us.AcceptedAnswers, pp.Title, pp.Score, ps.VoteCount, ps.Upvotes, ps.Downvotes,
//        CASE WHEN ps.VoteCount = 0 THEN 'No Votes' WHEN ps.Upvotes > ps.Downvotes THEN 'Positive Engagement' ELSE 'Negative Engagement' END AS EngagementLevel
// FROM UserStatistics us LEFT JOIN RecentPosts pp ON us.UserId = pp.OwnerUserId AND pp.RecentPostRank = 1 LEFT JOIN PostVoteStatistics ps ON pp.Id = ps.PostId
// WHERE us.TotalBadges > 0 OR us.RecentPostCount > 0 ORDER BY us.TotalBadges DESC, us.RecentPostCount DESC;
//
// Not recursive despite the name. A CreationDate tie within an owner goes to the smaller post id.
fn q33077(db: &'static So) -> String {
    let Post { creation_date, owner_user, accepted_answer, score, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let rp = || db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 0, 0, 0), -30)));
    let np = rp().group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let ap = db.post.with(accepted_answer).with(creation_date.gt(add_days(ts(2024, 10, 1, 0, 0, 0), -90))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let first = top_per(drain(rp().with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = rel(first.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&first).map(|(u, _)| u).inv().select(&first).collect();
    let pvs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let us = db
        .user
        .select((&bc).opt().and((&np).opt()).and((&ap).opt()))
        .filt(|((b, n), _): ((Option<i64>, Option<i64>), Option<i64>)| b.unwrap_or(0) > 0 || n.unwrap_or(0) > 0);
    let v = drain(us.and((&by_user).map(|(_, p)| p).select(Ident::<Post>::new().and(&pvs)).opt()));
    rows(v.into_iter().map(|(u, (((b, n), a), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(n.unwrap_or(0)), V::I(a.unwrap_or(0))]);
        f.extend(match p {
            Some((p, s)) => [ostr(db.post.title.get(p)), V::I(score.get(p).unwrap()), V::I(s[0]), V::I(s[1]), V::I(s[2]),
                V::S(if s[0] == 0 { "No Votes" } else if s[1] > s[2] { "Positive Engagement" } else { "Negative Engagement" })],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null, V::S("Negative Engagement")],
        });
        row(f)
    }))
}

// Rewritten (rewrites/22783.sql): the final ORDER BY is tie-broken on PostId.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
//     AND u.Reputation > (SELECT AVG(Reputation) FROM Users) AND COALESCE(p.ClosedDate, '9999-12-31') = '9999-12-31'),
// PostHistoryAggregates AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 END) AS EditCount FROM PostHistory ph GROUP BY ph.PostId),
// PostVotes AS (SELECT PostId, VoteTypeId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId IN (2, 3) GROUP BY PostId, VoteTypeId),
// CombinedPostData AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Reputation, COALESCE(pv2.VoteCount, 0) AS UpVotes, COALESCE(pv3.VoteCount, 0) AS DownVotes,
//        COALESCE(ph.CloseCount, 0) AS CloseCount, COALESCE(ph.DeleteCount, 0) AS DeleteCount, COALESCE(ph.EditCount, 0) AS EditCount
//     FROM RankedPosts rp LEFT JOIN PostVotes pv2 ON rp.PostId = pv2.PostId AND pv2.VoteTypeId = 2 LEFT JOIN PostVotes pv3 ON rp.PostId = pv3.PostId AND pv3.VoteTypeId = 3
//     LEFT JOIN PostHistoryAggregates ph ON rp.PostId = ph.PostId)
// SELECT Title, CreationDate, Score, ViewCount, Reputation, UpVotes, DownVotes, CloseCount, DeleteCount, EditCount,
//        CASE WHEN EditCount > 0 AND CloseCount = 0 THEN 'Active and Edited' WHEN CloseCount > 0 THEN 'Closed' WHEN DeleteCount > 0 THEN 'Deleted' ELSE 'Active' END AS PostStatus,
//        CASE WHEN Reputation IS NULL THEN 'No Reputation' ELSE CONCAT('User Reputation: ', Reputation) END AS UserStatus
// FROM CombinedPostData WHERE (UpVotes - DownVotes) > 0 ORDER BY Score DESC, ViewCount DESC, PostId LIMIT 100 OFFSET 0;
//
// ScoreRank is never read. The uncorrelated AVG(Reputation) is computed once.
fn q22783(db: &'static So) -> String {
    let Post { creation_date, owner_user, closed_date, score, view_count, origid, .. } = &db.post;
    let (rs, rn) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let avgr = rs as f64 / rn as f64;
    let far = ts(9999, 12, 31, 0, 0, 0);
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).filt(move |r: i64| r as f64 > avgr))))
        .minus(closed_date.filt(move |d: i64| d != far));
    let pv = rp.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pha = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| {
        [a[0] + (t == 10) as i64, a[1] + (t == 12) as i64, a[2] + matches!(t, 4 | 5 | 6) as i64]
    });
    let v = drain((&pv).filt(|a| a[0] - a[1] > 0).and((&pha).opt()));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), origid.get(p).unwrap())
    }, 100);
    rows(v.into_iter().map(|(p, (a, h))| {
        let h = h.unwrap_or([0; 3]);
        let rep = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(rep), V::I(a[0]), V::I(a[1]), V::I(h[0]), V::I(h[1]), V::I(h[2])]);
        f.push(V::S(if h[2] > 0 && h[0] == 0 { "Active and Edited" } else if h[0] > 0 { "Closed" } else if h[1] > 0 { "Deleted" } else { "Active" }));
        f.push(V::Owned(format!("User Reputation: {rep}")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount IS NOT NULL),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM RankedPosts rp
//     LEFT JOIN (SELECT p.Id AS PostId, SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END) AS DownVotes
//                FROM Votes v JOIN Posts p ON v.PostId = p.Id JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//                WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY p.Id) v ON rp.PostId = v.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY UserId) b
//       ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId)
//     WHERE rp.PostRank <= 5),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS HistoryCount FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
//     WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years' AND pht.Id IN (10, 11, 12) GROUP BY ph.PostId),
// FinalMetrics AS (SELECT pm.*, COALESCE(phc.HistoryCount, 0) AS HistoryCount FROM PostMetrics pm LEFT JOIN PostHistoryCounts phc ON pm.PostId = phc.PostId)
// SELECT f.PostId, f.Title, f.ViewCount, f.Score, f.UpVotes, f.DownVotes, f.BadgeCount, f.HistoryCount, CASE WHEN f.HistoryCount > 0 THEN 'Active' ELSE 'Inactive' END AS PostStatus
// FROM FinalMetrics f WHERE f.UpVotes > f.DownVotes ORDER BY f.Score DESC, f.ViewCount DESC;
//
// A (Score, ViewCount) tie at PostRank 5 goes to the smaller post id.
fn q24104(db: &'static So) -> String {
    let Post { creation_date, view_count, post_type_id, score, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(view_count).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(t0, -6)))).select(&db.vote.vote_type_id);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(rv.opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = db.badge.with((&db.badge.date).ge(add_years(t0, -1))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let hc = db.post_history.with(hd.ge(add_years(t0, -2)).and(ht.is_in([10, 11, 12]))).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&vc).filt(|a| a[0] > a[1]).and(owner_user.select(&bc).opt()).and((&hc).opt()));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((a, b), h))| {
        let h = h.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b.unwrap_or(0)), V::I(h), V::S(if h > 0 { "Active" } else { "Inactive" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        AVG(COALESCE(p.Score, 0)) AS AvgScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// ActiveUsers AS (SELECT ua.UserId, ua.DisplayName, ua.UpVotes, ua.DownVotes, ua.PostCount, ua.TotalViews, ua.AvgScore,
//        RANK() OVER (PARTITION BY ua.UpVotes - ua.DownVotes ORDER BY ua.TotalViews DESC) AS ActivityRank FROM UserActivity ua WHERE ua.PostCount > 0),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, TotalViews, AvgScore, ActivityRank FROM ActiveUsers WHERE ActivityRank <= 10),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RecentPostRank FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, tu.DisplayName AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 2) AS UpVoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 3) AS DownVoteCount
//     FROM RecentPosts rp LEFT JOIN Posts p ON rp.PostId = p.Id JOIN TopUsers tu ON p.OwnerUserId = tu.UserId WHERE rp.RecentPostRank <= 5)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.Score, pd.CommentCount, pd.OwnerDisplayName, pd.UpVoteCount, pd.DownVoteCount
// FROM PostDetails pd WHERE pd.ViewCount > 100 ORDER BY pd.ViewCount DESC, pd.Score DESC;
//
// A CreationDate tie at RecentPostRank 5 goes to the smaller post id.
fn q23835(db: &'static So) -> String {
    let Post { view_count, creation_date, owner_user, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (w, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + w.unwrap_or(0)]);
    let ar = per_group(ranked(drain(&ua), |&(_, a)| (a[0] - a[1], Reverse(a[2])), false), |&(_, a)| a[0] - a[1]);
    let tu: MatSet<Id<User>> = rel(ar.into_iter().filter(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let rp = top_n(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(creation_date)), |&(p, d)| (Reverse(d), p), 5);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pd = (&rp).with(owner_user.select(Ident::<User>::new().with(&tu))).with(view_count.gt(100));
    let cc = pd.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&cc).and((&vc).opt()));
    rows(v.into_iter().map(|(p, (c, u))| {
        let u = u.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c)]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(u[0]), V::I(u[1])]);
        row(f)
    }))
}

// WITH RecentUserVotes AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE v.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY v.UserId),
// PostDetails AS (SELECT p.Id AS PostId, p.PostTypeId, p.Title, p.CreationDate, p.ViewCount, COALESCE((SELECT COUNT(c.Id) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        COALESCE((SELECT MAX(ph.CreationDate) FROM PostHistory ph WHERE ph.PostId = p.Id AND ph.PostHistoryTypeId IN (10, 11)), NULL) AS LastClosedDate, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// AggregatedPosts AS (SELECT pd.PostId, pd.ViewCount, pd.CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY pd.PostId ORDER BY pd.ViewCount DESC) AS RankInViews, ROW_NUMBER() OVER (PARTITION BY pd.PostId ORDER BY pd.CommentCount DESC) AS RankInComments
//     FROM PostDetails pd LEFT JOIN Votes v ON pd.PostId = v.PostId GROUP BY pd.PostId, pd.ViewCount, pd.CommentCount),
// FinalReport AS (SELECT ap.PostId, pd.Title, pd.ViewCount, pd.CommentCount, ap.UpVotes, ap.DownVotes, CASE WHEN pd.LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus,
//        COALESCE(ru.VoteCount, 0) AS RecentVotes, ap.RankInViews, ap.RankInComments
//     FROM AggregatedPosts ap JOIN PostDetails pd ON ap.PostId = pd.PostId LEFT JOIN RecentUserVotes ru ON ru.UserId = pd.OwnerUserId)
// SELECT PostId, Title, ViewCount, CommentCount, UpVotes, DownVotes, PostStatus, RecentVotes,
//        CASE WHEN RankInViews <= 10 THEN 'Top View' WHEN RankInComments <= 10 THEN 'Top Commented' ELSE 'Other' END AS Classification
// FROM FinalReport WHERE RecentVotes > 5 ORDER BY ViewCount DESC, CommentCount DESC LIMIT 50;
//
// Both windows partition by PostId, one row each, so every rank is 1 and every row is 'Top View'.
fn q20603(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ru = db.vote.with((&db.vote.creation_date).ge(add_days(t0, -30))).group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let pd = db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user.select((&ru).filt(|n| n > 5)));
    let ap = pd.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id: ht, .. } = &db.post_history;
    let cl: MatSet<Id<Post>> = db.post_history.with(ht.is_in([10, 11])).select(post).collect();
    let v = drain((&ap).and((&cc).opt()).and(owner_user.select(&ru)).and(Ident::<Post>::new().with(&cl).opt()));
    let v = top_n(v, |&(p, (((_, c), _), _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(c.unwrap_or(0)), p)
    }, 50);
    rows(v.into_iter().map(|(p, (((a, c), r), k))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::S(if k.is_some() { "Closed" } else { "Active" }), V::I(r), V::S("Top View")]);
        row(f)
    }))
}

// WITH RECURSIVE UserVotes AS (SELECT v.UserId, p.OwnerUserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v JOIN Posts p ON v.PostId = p.Id GROUP BY v.UserId, p.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// PostStats AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS HasAcceptedAnswer,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS UserRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.OwnerUserId, p.PostTypeId),
// UserPostMetrics AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(v.VoteCount, 0) AS TotalVotes, COALESCE(b.GoldBadges, 0) AS GoldBadges, COALESCE(b.SilverBadges, 0) AS SilverBadges,
//        COALESCE(b.BronzeBadges, 0) AS BronzeBadges, p.PostId, p.CommentCount, p.HasAcceptedAnswer, p.TotalUpVotes, p.UserRank
//     FROM Users u LEFT JOIN UserVotes v ON u.Id = v.UserId LEFT JOIN UserBadges b ON u.Id = b.UserId LEFT JOIN PostStats p ON u.Id = p.OwnerUserId)
// SELECT um.UserId, um.DisplayName, COALESCE(SUM(um.TotalVotes), 0) AS TotalVoteCount, COALESCE(SUM(um.GoldBadges), 0) AS TotalGoldBadges, COALESCE(SUM(um.SilverBadges), 0) AS TotalSilverBadges,
//        COALESCE(SUM(um.BronzeBadges), 0) AS TotalBronzeBadges, COALESCE(SUM(um.CommentCount), 0) AS TotalComments,
//        COUNT(DISTINCT CASE WHEN um.HasAcceptedAnswer = 1 THEN um.PostId END) AS AcceptedAnswers, AVG(um.TotalUpVotes) AS AvgUpVotes
// FROM UserPostMetrics um GROUP BY um.UserId, um.DisplayName ORDER BY TotalVoteCount DESC, TotalGoldBadges DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. UserVotes has one row per (voter, post owner), and each user's rows are those crossed with the user's
// PostStats rows (one per post), so every SUM runs over that product. UserRank is never read.
fn q33358(db: &'static So) -> String {
    let Post { owner_user, accepted_answer, .. } = &db.post;
    let Vote { user, post, vote_type_id, .. } = &db.vote;
    let uv = db.vote.with(post).group_by(user.and(post.select(owner_user).opt())).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let uvv = rel(drain(&uv));
    let uv_by: HashIdx<Id<User>, ((Id<User>, Option<Id<User>>), i64)> = (&uvv).map(|((u, _), _)| u).inv().select(&uvv).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(accepted_answer.opt().and(comments_of(db).opt()).and(votes_of(db).select(vote_type_id).opt()))
        .fold([0i64; 3], |a, ((x, c), t)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + (t == Some(2)) as i64]);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select((&uv_by).map(|(_, n)| n).opt().and((&ub).opt()).and(posts_of(db).select(&ps).opt()))
        .fold([0i64; 7], |a, ((n, b), p)| {
            let b = b.unwrap_or([0; 3]);
            let (c, up, has) = p.map_or((0, 0, false), |p| (p[0], p[2], true));
            [a[0] + n.unwrap_or(0), a[1] + b[0], a[2] + b[1], a[3] + b[2], a[4] + c, a[5] + up, a[6] + has as i64]
        });
    let acc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&ps)).fold(0i64, |n, p| n + (p[1] == 1) as i64);
    let v = drain((&s).and((&acc).opt()));
    rows(v.into_iter().map(|(u, (a, c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a[..5].iter().map(|&x| V::I(x)));
        f.extend([V::I(c.unwrap_or(0)), avg(a[5], a[6])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS ScoreRank,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY p.Id) AS UpVoteCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY p.Id) AS DownVoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND p.Score IS NOT NULL),
// HighlyVotedPosts AS (SELECT PostId, Title, CreationDate, OwnerUserId, Score, ViewCount, ScoreRank, UpVoteCount, DownVoteCount FROM RankedPosts WHERE ScoreRank <= 10),
// UserBadges AS (SELECT u.Id AS UserId, b.Name AS BadgeName, COUNT(b.Id) AS BadgeCount FROM Users u JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, b.Name),
// PostHistoryReflections AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosedDate,
//        MAX(CASE WHEN ph.PostHistoryTypeId IN (11, 13) THEN ph.CreationDate END) AS LastActionDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT hp.PostId, hp.Title, hp.CreationDate, u.DisplayName AS Owner, (UPV.UpVoteCount - COALESCE(DWV.DownVoteCount, 0)) AS NetVotes, COALESCE(b.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN ph.LastClosedDate IS NOT NULL THEN 'Closed' WHEN ph.LastActionDate IS NOT NULL THEN 'Active' ELSE 'Inactive' END AS PostStatus
// FROM HighlyVotedPosts hp LEFT JOIN Users u ON hp.OwnerUserId = u.Id LEFT JOIN UserBadges b ON u.Id = b.UserId LEFT JOIN PostHistoryReflections ph ON hp.PostId = ph.PostId
// LEFT JOIN (SELECT PostId, COUNT(*) AS UpVoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) UPV ON hp.PostId = UPV.PostId
// LEFT JOIN (SELECT PostId, COUNT(*) AS DownVoteCount FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) DWV ON hp.PostId = DWV.PostId
// WHERE b.BadgeCount > 1 OR b.BadgeCount IS NULL ORDER BY hp.Score DESC;
//
// ScoreRank numbers the post x vote rows, so the joined rows are ranked; a tie within a post goes to the smaller vote id.
fn q20768(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = db.post.with(creation_date.ge(add_years(current_date(), -1))).select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let jv = drain((&j).map(|(p, _)| p).select(post_type_id));
    let top = top_per(jv, |&(_, t)| t, |&((p, v), _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, v)
    }, 10, false);
    let hp = rel(top.into_iter().map(|x| x.0 .0).collect());
    let ubg = db.badge.group_by((&db.badge.user).and(&db.badge.name)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ubv = rel(drain(&ubg));
    let ub_by: HashIdx<Id<User>, ((Id<User>, Str), i64)> = (&ubv).map(|((u, _), _)| u).inv().select(&ubv).collect();
    let PostHistory { post, post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let phr = db.post_history.group_by(post).select(ht.and(hd)).fold((i64::MIN, i64::MIN), |(c, a), (t, d)| (if t == 10 { c.max(d) } else { c }, if matches!(t, 11 | 13) { a.max(d) } else { a }));
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let b = owner_user.select((&ub_by).map(|(_, n)| n)).opt().filt(|b: Option<i64>| b.map_or(true, |n| n > 1));
    let mut v = drain((&hp).select(Ident::<Post>::new().and(b).and((&phr).opt()).and((&vc).opt())));
    v.sort_by_key(|&(_, (((p, _), _), _))| Reverse(score.get(p).unwrap()));
    rows(v.into_iter().map(|(_, (((p, b), h), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(match c {
            Some(c) if c[0] > 0 => V::I(c[0] - c[1]),
            _ => V::Null,
        });
        f.push(V::I(b.unwrap_or(0)));
        f.push(V::S(match h {
            Some((c, _)) if c != i64::MIN => "Closed",
            Some((_, a)) if a != i64::MIN => "Active",
            _ => "Inactive",
        }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, CreationDate, COALESCE(LastAccessDate, CreationDate) AS LastActiveDate, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM Users WHERE Reputation > 1000),
// PopularPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.PostTypeId = 1 GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.Score
//     HAVING COUNT(v.Id) > 5),
// RecentPostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserDisplayName, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS RecentHistory
//     FROM PostHistory ph WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '60 days'),
// TopTags AS (SELECT t.TagName, t.Count, DENSE_RANK() OVER (ORDER BY t.Count DESC) AS TagRank FROM Tags t WHERE t.Count >= 100)
// SELECT ur.UserId, ur.Reputation, pp.Title, pp.CreationDate AS PostDate, pp.CommentCount, pp.UpVotes, pp.DownVotes, COALESCE(rph.UserDisplayName, 'No recent history') AS LastEditor,
//        rph.CreationDate AS LastEditedDate, tt.TagName, pp.Score,
//        CASE WHEN pp.Score > 10 THEN 'High Score' WHEN pp.Score BETWEEN 1 AND 10 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM UserReputation ur JOIN PopularPosts pp ON ur.UserId = pp.OwnerUserId LEFT JOIN RecentPostHistory rph ON pp.PostId = rph.PostId AND rph.RecentHistory = 1
// LEFT JOIN PostLinks pl ON pp.PostId = pl.PostId LEFT JOIN Tags tt ON pl.RelatedPostId = tt.WikiPostId
// WHERE (rph.CreationDate IS NULL OR rph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days')
// ORDER BY ur.Reputation DESC, pp.CommentCount DESC, pp.Score DESC LIMIT 100;
//
// TopTags is never read. A CreationDate tie for a post's latest history row goes to the larger history id.
fn q22865(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let base = db.post.with(creation_date.ge(add_days(t0, -30)).and(post_type_id.eq(1))).with(owner_user.select(rich));
    let pp = base
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let rh = drain(db.post_history.with(hd.ge(add_days(t0, -60))).select(post));
    let rh = top_per(rh, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let rh = rel(rh.into_iter().map(|(h, p)| (p, h)).collect());
    let rh_by: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&rh).map(|(p, _)| p).inv().select(&rh).collect();
    let rph = (&rh_by).map(|(_, h)| h).opt().filt(move |h: Option<Id<PostHistory>>| h.map_or(true, |h| hd.get(h).unwrap() > add_days(t0, -7)));
    let by_wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let pl = links_of(db).select(Ident::<PostLink>::new().and((&db.post_link.related_post).select(&by_wiki).opt())).opt();
    let v = drain((&pp).filt(|a| a[1] > 5).and(rph).and(pl));
    let v = top_n(v, |&(p, ((a, h), l))| (Reverse(db.user.reputation.get(owner_user.get(p).unwrap()).unwrap()), Reverse(a[0]), Reverse(score.get(p).unwrap()), p, h, l), 100);
    rows(v.into_iter().map(|(p, ((a, h), l))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["uid", "rep", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[3])]);
        f.extend(match h {
            Some(h) => [V::S(db.post_history.user_display_name.get(h).unwrap_or("No recent history")), V::T(hd.get(h).unwrap())],
            None => [V::S("No recent history"), V::Null],
        });
        f.push(match l {
            Some((_, Some(t))) => V::S(db.tag.tag_name.get(t).unwrap()),
            _ => V::Null,
        });
        f.push(V::I(s));
        f.push(V::S(if s > 10 { "High Score" } else if (1..=10).contains(&s) { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank,
//        COUNT(c.Id) AS CommentTotal FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.PostTypeId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount,
//        AVG(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - u.LastAccessDate)) / 3600) AS AvgHoursSinceLastAccess
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation >= 1000 GROUP BY u.Id, u.DisplayName),
// UserPostSummary AS (SELECT ua.UserId, ua.DisplayName, COUNT(rp.Id) AS PostCount, MAX(rp.RecentPostRank) AS HighestRecentPostRank
//     FROM UserActivity ua LEFT JOIN RankedPosts rp ON ua.UserId = rp.OwnerUserId GROUP BY ua.UserId, ua.DisplayName),
// HighEngagementPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.PostTypeId, ut.DisplayName AS OwnerName, COUNT(co.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoteCount,
//        CASE WHEN COUNT(DISTINCT v.UserId) > 50 AND COUNT(co.Id) > 25 THEN 'High Engagement' ELSE 'Standard Engagement' END AS EngagementType
//     FROM Posts p JOIN Users ut ON p.OwnerUserId = ut.Id LEFT JOIN Comments co ON p.Id = co.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.OwnerUserId, p.PostTypeId, ut.DisplayName)
// SELECT ups.DisplayName AS UserDisplayName, ups.PostCount, ups.HighestRecentPostRank, he.PostTypeId, he.Title, he.OwnerName, he.CommentCount, he.UniqueVoteCount, he.EngagementType
// FROM UserPostSummary ups JOIN HighEngagementPosts he ON ups.UserId = he.OwnerUserId WHERE ups.PostCount > 10 AND ups.HighestRecentPostRank = 1
// ORDER BY ups.PostCount DESC, he.UniqueVoteCount DESC LIMIT 50;
//
// UserActivity has one row per user and only its DisplayName is read, so its vote aggregates are not computed. HighEngagementPosts is driven for the
// kept users' posts alone. A CreationDate tie within an owner goes to the smaller post id.
fn q22769(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let rk = per_group(ranked(rp, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    type R = ((Id<Post>, Id<User>), i64);
    let rk = rel(rk);
    let ups = (&rk).group_by(Same::<R>::new().map(|((_, u), _): R| u)).select(Same::<R>::new().map(|(_, r): R| r)).fold((0i64, 0i64), |(n, m), r| (n + 1, m.max(r)));
    let keep: MatSet<Id<User>> = db.user.with((&db.user.reputation).ge(1000)).with((&ups).filt(|(n, m)| n > 10 && m == 1)).collect();
    let owned = || db.post.with(owner_user.select(Ident::<User>::new().with(&keep)));
    let he = owned().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let uv = owned().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.user)).count_distinct();
    let v = drain((&he).and((&uv).opt()).and(owner_user.select(&ups)));
    let v = top_n(v, |&(p, ((_, u), (n, _)))| (Reverse(n), Reverse(u.unwrap_or(0)), p), 50);
    rows(v.into_iter().map(|(p, ((c, u), (n, m)))| {
        let u = u.unwrap_or(0);
        let mut f = post_fields(db, p, &["owner"]);
        f.extend([V::I(n), V::I(m)]);
        f.extend(post_fields(db, p, &["type_id", "title", "owner"]));
        f.extend([V::I(c), V::I(u), V::S(if u > 50 && c > 25 { "High Engagement" } else { "Standard Engagement" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// ClosedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(ch.Id) AS CommentCount, MAX(ph.CreationDate) AS LastClosedDate, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 6) AS CloseVoteCount
//     FROM Posts p LEFT JOIN Comments ch ON p.Id = ch.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (10, 11) LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.Score < 0 GROUP BY p.Id, p.OwnerUserId),
// UserPostStats AS (SELECT ps.OwnerUserId AS UserId, COUNT(DISTINCT ps.Id) AS PostsCreated, COALESCE(SUM(ps.ViewCount), 0) AS TotalViews,
//        COALESCE(AVG(CASE WHEN ps.Score IS NOT NULL THEN ps.Score ELSE 0 END), 0) AS AvgScore FROM Posts ps GROUP BY ps.OwnerUserId),
// PostMetrics AS (SELECT up.UserId, MAX(up.PostsCreated) AS MaxPostsCreated, MAX(up.TotalViews) AS MaxTotalViews, MAX(up.AvgScore) AS MaxAvgScore, us.Reputation, us.ReputationRank,
//        us.GoldBadges, us.SilverBadges, us.BronzeBadges
//     FROM UserPostStats up JOIN UserStats us ON up.UserId = us.UserId GROUP BY up.UserId, us.Reputation, us.ReputationRank, us.GoldBadges, us.SilverBadges, us.BronzeBadges)
// SELECT p.UserId, COALESCE(c.CommentCount, 0) AS ClosedCommentCount, COALESCE(p.MaxPostsCreated, 0) AS MaxPostsCreated, COALESCE(p.MaxTotalViews, 0) AS MaxTotalViews,
//        COALESCE(p.MaxAvgScore, 0) AS MaxAvgScore, us.Reputation, us.GoldBadges, us.SilverBadges, us.BronzeBadges
// FROM PostMetrics p LEFT JOIN ClosedPosts c ON p.UserId = c.OwnerUserId JOIN UserStats us ON p.UserId = us.UserId WHERE p.MaxPostsCreated > 0 OR us.Reputation > 1000
// ORDER BY us.Reputation DESC, ClosedCommentCount DESC;
//
// PostMetrics groups UserPostStats, one row per owner, by that owner, so each MAX is the value itself. Every owner has a post, so MaxPostsCreated > 0 holds;
// a user repeats once per negative-score post (ClosedPosts).
fn q24920(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let Post { score, owner_user, view_count, .. } = &db.post;
    let ups = db.post.group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let PostHistory { post_history_type_id: ht, .. } = &db.post_history;
    let cp = db
        .post
        .with(score.lt(0))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(Ident::<PostHistory>::new().with(ht.is_in([10, 11]))).opt()).and(votes_of(db).opt()))
        .fold(0i64, |n, ((c, _), _)| n + c.is_some() as i64);
    let cp_by: HashIdx<Id<User>, Id<Post>> = db.post.with(&cp).select(owner_user).inv().collect();
    let v = drain((&ups).filt(|a| a[0] > 0).and(&ub).and((&cp_by).select(&cp).opt()));
    rows(v.into_iter().map(|(u, ((a, b), c))| {
        let mut f = vec![user_col(db, u, "uid"), V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), user_col(db, u, "rep")];
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(a.AnswerCount, 0) AS AnswerCount, COALESCE(c.CommentCount, 0) AS CommentCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserVoteStats AS (SELECT v.UserId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS TotalDownVotes,
//        SUM(CASE WHEN vt.Name = 'AcceptedByOriginator' THEN 1 ELSE 0 END) AS AcceptedAnswers FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// PostHistoryInfo AS (SELECT ph.PostId, MAX(CASE WHEN pht.Name LIKE 'Edit%' THEN ph.CreationDate END) AS LastEdited, MAX(CASE WHEN pht.Name = 'Post Closed' THEN ph.CreationDate END) AS ClosedDate
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.CommentCount, COALESCE(uv.TotalUpVotes, 0) AS UserTotalUpVotes, COALESCE(uv.TotalDownVotes, 0) AS UserTotalDownVotes,
//        COALESCE(uv.AcceptedAnswers, 0) AS UserAcceptedAnswers, ph.LastEdited, ph.ClosedDate,
//        CASE WHEN ph.ClosedDate IS NOT NULL THEN 'Closed' WHEN rp.AnswerCount > 0 THEN 'Active' ELSE 'Inactive' END AS PostStatus,
//        CASE WHEN rp.ViewCount > 1000 THEN 'High Visibility' ELSE 'Normal Visibility' END AS Visibility
// FROM RecentPosts rp LEFT JOIN UserVoteStats uv ON rp.OwnerUserId = uv.UserId LEFT JOIN PostHistoryInfo ph ON rp.PostId = ph.PostId WHERE rp.rn = 1
// ORDER BY rp.ViewCount DESC, rp.CreationDate DESC LIMIT 50;
//
// The ownerless posts are one partition (NULL OwnerUserId). A CreationDate tie within an owner goes to the smaller post id.
fn q22321(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ac = db.post.with((&db.post.post_type_id).eq(2)).group_by(&db.post.parent).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let uv = db.vote.group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 3], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64, a[2] + (n == "AcceptedByOriginator") as i64]);
    let phi = db.post_history.group_by(&db.post_history.post).select(htype_name(db).and(&db.post_history.creation_date)).fold((i64::MIN, i64::MIN), |(e, c), (n, d)| {
        (if n.starts_with("Edit") { e.max(d) } else { e }, if n == "Post Closed" { c.max(d) } else { c })
    });
    let v = drain((&tp).select((&ac).opt().and((&cc).opt()).and(owner_user.select(&uv).opt()).and((&phi).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()), p)
    }, 50);
    rows(v.into_iter().map(|(p, (((a, c), u), h))| {
        let (a, c, u) = (a.unwrap_or(0), c.unwrap_or(0), u.unwrap_or([0; 3]));
        let (e, cl) = h.unwrap_or((i64::MIN, i64::MIN));
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(a), V::I(c), V::I(u[0]), V::I(u[1]), V::I(u[2]), tmax(e), tmax(cl)]);
        f.push(V::S(if cl != i64::MIN { "Closed" } else if a > 0 { "Active" } else { "Inactive" }));
        f.push(V::S(if view_count.get(p).map_or(false, |w| w > 1000) { "High Visibility" } else { "Normal Visibility" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Owner, p.CreationDate, p.LastActivityDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 month'),
// PostHistoryCTE AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate AS HistoryDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// ClosedPosts AS (SELECT rp.PostId, rp.Title, rp.Owner, rp.CreationDate, hp.HistoryDate AS LastClosedDate, hp.PostHistoryTypeId AS LastAction
//     FROM RankedPosts rp LEFT JOIN PostHistoryCTE hp ON rp.PostId = hp.PostId AND hp.HistoryRank = 1 WHERE rp.Rank <= 5),
// PostScoreReview AS (SELECT cp.PostId, cp.Title, cp.Owner, cp.CreationDate, cp.LastClosedDate, cp.LastAction, (cp.LastClosedDate IS NOT NULL) AS IsClosed,
//        CASE WHEN cp.LastAction = 10 THEN 'Closed' WHEN cp.LastAction = 11 THEN 'Reopened' ELSE 'N/A' END AS CurrentState FROM ClosedPosts cp)
// SELECT psr.PostId, psr.Title, psr.Owner, psr.CreationDate, psr.LastClosedDate, psr.CurrentState, COALESCE(rp.UpVotes, 0) AS PostUpVotes, COALESCE(rp.DownVotes, 0) AS PostDownVotes,
//        CASE WHEN psr.IsClosed THEN 'This post is currently closed.' ELSE 'This post is open for interaction.' END AS InteractionStatus,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = psr.PostId) AS CommentCount
// FROM PostScoreReview psr LEFT JOIN RankedPosts rp ON psr.PostId = rp.PostId
// WHERE psr.LastClosedDate IS NULL OR psr.LastClosedDate >= cast('2024-10-01' as date) - INTERVAL '1 week' ORDER BY psr.CreationDate DESC NULLS LAST;
//
// A Score tie at Rank 5 goes to the smaller post id, and a CreationDate tie for a post's latest close/reopen row to the larger history id.
fn q20402(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 0, 0, 0);
    let v = drain(db.post.with(creation_date.ge(add_months(t0, -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let lh = top_per(drain(db.post_history.with(ht.is_in([10, 11])).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let lh = rel(lh.into_iter().map(|(h, p)| (p, h)).collect());
    let lh_by: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lh).map(|(p, _)| p).inv().select(&lh).collect();
    let cut = add_days(t0, -7);
    let last = (&lh_by).map(|(_, h)| h).opt().filt(move |h: Option<Id<PostHistory>>| h.map_or(true, |h| hd.get(h).unwrap() >= cut));
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain((&vc).and(&cc).and(last));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, ((u, c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.push(h.map_or(V::Null, |h| V::T(hd.get(h).unwrap())));
        f.push(V::S(match h.map(|h| ht.get(h).unwrap()) {
            Some(10) => "Closed",
            Some(11) => "Reopened",
            _ => "N/A",
        }));
        f.extend([V::I(u[0]), V::I(u[1]), V::S(if h.is_some() { "This post is currently closed." } else { "This post is open for interaction." }), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COALESCE(p.AnswerCount, 0) AS AnswerCount FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.PostTypeId = 1),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.Body, rp.ViewCount, rp.AnswerCount, COALESCE(ph.PostHistoryTypeId, 0) AS LastHistoryType,
//        COALESCE(ph.CreationDate, CAST('1970-01-01' AS TIMESTAMP)) AS LastHistoryDate FROM RankedPosts rp LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId WHERE rp.Rank = 1),
// TagStats AS (SELECT t.Id AS TagId, t.TagName, COUNT(p.Id) AS PostCount FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.Id, t.TagName),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT pd.PostId, pd.Title, pd.Body, pd.ViewCount, pd.AnswerCount, ts.TagName, COALESCE(ub.GoldBadges, 0) AS UserGoldBadges, COALESCE(ub.SilverBadges, 0) AS UserSilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS UserBronzeBadges,
//        CASE WHEN pd.LastHistoryType IS NULL THEN 'No History' ELSE CASE WHEN pd.LastHistoryType = 10 THEN 'Closed' WHEN pd.LastHistoryType = 11 THEN 'Reopened' ELSE 'Other Action' END END AS LastAction,
//        CASE WHEN pd.LastHistoryDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' THEN 'Recently Updated' ELSE 'Stale' END AS UpdateStatus
// FROM PostDetails pd LEFT JOIN TagStats ts ON ts.PostCount > 0 LEFT JOIN UserBadges ub ON ub.UserId = pd.PostId
// WHERE pd.ViewCount > (SELECT AVG(ViewCount) FROM Posts) AND (pd.AnswerCount > 0 OR pd.LastHistoryType IN (10, 11)) ORDER BY pd.ViewCount DESC LIMIT 100;
//
// The ON clause names only ts, so the kept post x history rows are crossed with the tags that some post mentions; a ViewCount tie at the cut goes to the
// smaller post, history row and tag id. `ub.UserId = pd.PostId` joins a user id to a post id, so it goes through the raw ids. The ownerless questions are one
// partition (NULL OwnerUserId); a CreationDate tie within an owner goes to the smaller post id.
fn q23052(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, view_count, answer_count, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.eq(1))).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| {
        (Reverse(creation_date.get(p).unwrap()), p)
    }, 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let (vs, vn) = db.post.select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let avgv = vs as f64 / vn as f64;
    let PostHistory { post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let ph = history_of(db).select(Ident::<PostHistory>::new().and(ht)).opt();
    let pd = (&tp)
        .with(view_count.filt(move |w: i64| w as f64 > avgv))
        .select(Ident::<Post>::new().and(ph).and(answer_count.opt()))
        .filt(|((_, h), a): ((Id<Post>, Option<(Id<PostHistory>, i64)>), Option<i64>)| a.unwrap_or(0) > 0 || matches!(h, Some((_, 10 | 11))));
    let pd = drain(pd);
    let ts_ = tag_stats(db);
    let tags: Vec<Id<Tag>> = drain(db.tag.with((&ts_).filt(|a| a[0] > 0)).select(Ident::<Tag>::new())).into_iter().map(|x| x.1).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = cross_top(pd, |&(_, ((p, h), _))| (Reverse(view_count.get(p)), p, h.map(|h| h.0)), tags, |&t| t, 100);
    let rv = rel(v);
    type R = ((Id<Post>, ((Id<Post>, Option<(Id<PostHistory>, i64)>), Option<i64>)), Id<Tag>);
    let out = drain((&rv).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select(origid).select(&uidx).select(&ub).opt())));
    rows(out.into_iter().map(|(_, (((p, ((_, h), a)), t), b))| {
        let b = b.unwrap_or([0; 3]);
        let lt = h.map_or(0, |h| h.1);
        let ld = h.map_or(0, |h| hd.get(h.0).unwrap());
        let mut f = post_fields(db, p, &["id", "title", "body", "views"]);
        f.push(V::I(a.unwrap_or(0)));
        f.push(V::S(db.tag.tag_name.get(t).unwrap()));
        f.extend(b.map(V::I));
        f.push(V::S(if lt == 10 { "Closed" } else if lt == 11 { "Reopened" } else { "Other Action" }));
        f.push(V::S(if ld > add_days(t0, -30) { "Recently Updated" } else { "Stale" }));
        row(f)
    }))
}

// WITH RecursivePostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, (SELECT COALESCE(MAX(CreationDate), '1970-01-01 00:00:00') FROM Comments c WHERE c.PostId = p.Id) AS LastCommentDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Posts a ON a.ParentId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// RecentPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.AnswerCount, ps.UpVotes, ps.DownVotes, ps.LastCommentDate, ps.UserPostRank, DENSE_RANK() OVER (ORDER BY ps.CreationDate DESC) AS RecentRank
//     FROM RecursivePostStats ps WHERE ps.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT rp.Title, rp.AnswerCount, rp.UpVotes, rp.DownVotes, rp.LastCommentDate, rp.UserPostRank, a.UserId, a.DisplayName, a.TotalUpVotes, a.TotalDownVotes, a.BadgeCount, pc.CommentCount,
//        CASE WHEN rp.RecentRank <= 5 THEN 'High Activity' WHEN rp.RecentRank BETWEEN 6 AND 15 THEN 'Moderate Activity' ELSE 'Low Activity' END AS ActivityLevel
// FROM RecentPosts rp JOIN ActiveUsers a ON rp.UserPostRank = a.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE a.TotalUpVotes > a.TotalDownVotes
// ORDER BY rp.CreationDate DESC LIMIT 50;
//
// Not recursive despite the name. `rp.UserPostRank = a.UserId` joins a row number to a user id, through the raw ids. The TIMESTAMP is compared with
// CURRENT_TIMESTAMP as an instant in the session zone. UserPostRank is over every question of the owner (NULL owners are one partition; a CreationDate
// tie goes to the smaller post id), so it is taken first and the recent questions are filtered from it in prela.
fn q30079(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let rk = per_group(ranked(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let cut = add_days(now_utc(), -30);
    let rk = drain(rel(rk).filt(move |((p, _), _)| ny_to_utc(creation_date.get(p).unwrap()) >= cut)).into_iter().map(|x| x.1).collect::<Vec<_>>();
    let rr = ranked(rk, |&((p, _), _)| Reverse(creation_date.get(p).unwrap()), true);
    type R = (((Id<Post>, Option<Id<User>>), i64), i64);
    let rp = rel(rr);
    let st = (&rp)
        .group_by(Same::<R>::new())
        .select(Same::<R>::new().map(|(((p, _), _), _): R| p).select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ranks: MatSet<i64> = (&rp).map(|((_, r), _): R| r).collect();
    let au: MatSet<Id<User>> = db.user.with((&db.user.origid).select(&ranks)).collect();
    let ua = (&au)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let lc = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pc = Same::<R>::new().map(|(((p, _), _), _): R| p).select((&lc).opt());
    let user = Same::<R>::new().map(|((_, r), _): R| r).select(&uidx).select(Ident::<User>::new().and((&ua).filt(|a| a[0] > a[1])));
    let v = drain((&st).and(user).and(pc));
    let v = top_n(v, |&((((p, _), _), _), _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|((((p, _), r), dr), ((a, (u, b)), c))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(c.map_or(ts(1970, 1, 1, 0, 0, 0), |c| c.1)), V::I(r)]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(c.map_or(0, |c| c.0))]);
        f.push(V::S(if dr <= 5 { "High Activity" } else if (6..=15).contains(&dr) { "Moderate Activity" } else { "Low Activity" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven, COUNT(DISTINCT b.Id) AS BadgesCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, ROW_NUMBER() OVER (ORDER BY us.Reputation DESC) AS UserRank FROM UserStatistics us),
// PostHistoryDetails AS (SELECT ph.PostId, ph.UserId, ph.Comment, ph.CreationDate, p.Title AS PostTitle, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS CommentRank FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// RecentActivePosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, COALESCE(MAX(ph.CreationDate), p.CreationDate) AS LastActivity
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 11 GROUP BY p.Id, p.Title, p.CreationDate HAVING COUNT(c.Id) > 0)
// SELECT rp.PostId, rp.Title AS PostTitle, rup.DisplayName AS TopUser, rup.Reputation AS UserReputation, p.CommentCount AS CommentsCount, ph.Comment AS UserComment, ph.CreationDate AS CommentDate,
//        ROW_NUMBER() OVER (PARTITION BY rp.PostId ORDER BY ph.CreationDate DESC) AS RecentCommentRank
// FROM RankedPosts rp LEFT JOIN TopUsers rup ON rp.OwnerUserId = rup.UserId LEFT JOIN RecentActivePosts p ON rp.PostId = p.PostId
// LEFT JOIN PostHistoryDetails ph ON rp.PostId = ph.PostId AND ph.CommentRank = 1 WHERE rp.PostRank <= 5 ORDER BY rp.PostTypeId, rp.Score DESC;
//
// TopUsers has one row per user and only DisplayName and Reputation are read, so UserStatistics' aggregates are not computed. CommentCount is COUNT(c.Id)
// over the post x reopen-history rows. A Score tie at PostRank 5 goes to the smaller post id, a CreationDate tie for the latest history row to the larger id.
// Each post has at most one CommentRank = 1 row, so RecentCommentRank is 1.
fn q32412(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id: ht, creation_date: hd, comment, .. } = &db.post_history;
    let reopen = history_of(db).select(Ident::<PostHistory>::new().with(ht.eq(11)));
    let rap = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(reopen.opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let lh = top_per(drain((&tp).select(history_of(db).select(Ident::<PostHistory>::new().with(ht.is_in([10, 11, 12]))))), |&(p, _)| p, |&(_, h)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let lh = rel(lh);
    let lh_by: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lh).map(|(p, _)| p).inv().select(&lh).collect();
    let _ = post;
    let mut v = drain((&tp).select(owner_user.opt().and((&rap).filt(|n| n > 0).opt()).and((&lh_by).map(|(_, h)| h).opt())));
    v.sort_by_key(|&(p, _)| (post_type_id.get(p).unwrap(), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((u, c), h))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(match u {
            Some(u) => ucols(db, u, &["name", "rep"]),
            None => vec![V::Null, V::Null],
        });
        f.push(oint(c));
        f.extend(match h {
            Some(h) => [ostr(comment.get(h)), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        f.push(V::I(1));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, CreationDate, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// TopUsers AS (SELECT UserId, Reputation, CreationDate, ROW_NUMBER() OVER (PARTITION BY ReputationRank / 10 ORDER BY Reputation DESC) AS RankGroup FROM UserReputation WHERE Reputation > 1000),
// PostDetails AS (SELECT p.Id AS PostId, p.PostTypeId, p.OwnerUserId, COALESCE(UPD.VoteCount, 0) AS VoteCount, COALESCE(ANS.AnswerCount, 0) AS AnswerCount, COALESCE(EDT.EditCount, 0) AS EditCount,
//        EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) AS PostAgeInSeconds, CASE WHEN p.Score IS NULL THEN NULL ELSE p.Score + (10 * COALESCE(UPD.VoteCount, 0)) END AS AdjustedScore
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) AS UPD ON p.Id = UPD.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) AS ANS ON p.Id = ANS.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId) AS EDT ON p.Id = EDT.PostId WHERE p.OwnerUserId IS NOT NULL),
// TopPosts AS (SELECT pd.PostId, pd.PostTypeId, pd.OwnerUserId, pd.VoteCount, pd.AnswerCount, pd.EditCount, pd.PostAgeInSeconds, pd.AdjustedScore,
//        ROW_NUMBER() OVER (ORDER BY pd.AdjustedScore DESC) AS PostRank FROM PostDetails pd WHERE pd.AdjustedScore IS NOT NULL)
// SELECT tu.UserId, tu.Reputation AS UserReputation, pp.PostId, pp.VoteCount, pp.AnswerCount, pp.EditCount, pp.PostAgeInSeconds, pp.AdjustedScore, pp.PostRank
// FROM TopUsers tu JOIN TopPosts pp ON pp.OwnerUserId = tu.UserId WHERE pp.PostRank <= 10
// GROUP BY tu.UserId, tu.Reputation, pp.PostId, pp.VoteCount, pp.AnswerCount, pp.EditCount, pp.PostAgeInSeconds, pp.AdjustedScore, pp.PostRank
// HAVING COUNT(*) > 2 AND (MAX(pp.AdjustedScore) - MIN(pp.AdjustedScore) <= 50) LIMIT 1000;
//
// The GROUP BY includes pp.PostId and a user id, so each group is one (user, post) row and COUNT(*) > 2 keeps none; the groups are still built
// and filtered in prela. An AdjustedScore tie at PostRank 10 goes to the smaller post id.
fn q21526(db: &'static So) -> String {
    let Post { owner_user, score, last_activity_date, creation_date, .. } = &db.post;
    let up = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)))).fold(0i64, |n, _| n + 1);
    let adj = drain(db.post.with(owner_user).select(score.and((&up).opt()).map(|(s, u): (i64, Option<i64>)| (u.unwrap_or(0), s + 10 * u.unwrap_or(0)))));
    let tp = top_n(adj, |&(p, (_, a))| (Reverse(a), p), 10);
    type R = (usize, (Id<Post>, (i64, i64)));
    let tpr = rel(tp.into_iter().enumerate().collect::<Vec<_>>());
    let ac = db.post.with((&db.post.post_type_id).eq(2)).group_by(&db.post.parent).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id: ht, .. } = &db.post_history;
    let ec = db.post_history.with(ht.is_in([4, 5, 6])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let g = (&tpr)
        .group_by(Same::<R>::new().map(|(_, (p, _)): R| p).select(owner_user.select(rich)).and(Same::<R>::new()))
        .select(Same::<R>::new().map(|(_, (_, (_, a))): R| a))
        .fold((0i64, i64::MIN, i64::MAX), |(n, hi, lo), a| (n + 1, hi.max(a), lo.min(a)));
    type K = (Id<User>, R);
    let pk = || Same::<K>::new().map(|(_, (_, (p, _))): K| p);
    let v = drain((&g).filt(|(n, hi, lo)| n > 2 && hi - lo <= 50).and(pk().select((&ac).opt()).and(pk().select((&ec).opt()))));
    rows(v.into_iter().take(1000).map(|((u, (i, (p, (n, a)))), (_, (c, e)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(n), V::I(c.unwrap_or(0)), V::I(e.unwrap_or(0)), V::F(secs(last_activity_date.get(p).unwrap() - creation_date.get(p).unwrap())), V::I(a), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.CreationDate >= '2022-01-01' AND (p.Title LIKE '%SQL%' OR p.Body LIKE '%JOIN%') GROUP BY p.Id, p.Title, p.CreationDate, p.PostTypeId),
// PostLinksWithTypes AS (SELECT pl.PostId, pl.RelatedPostId, lt.Name AS LinkTypeName, pl.CreationDate AS LinkCreationDate,
//        ROW_NUMBER() OVER (PARTITION BY pl.PostId ORDER BY pl.CreationDate DESC) AS LinkRN FROM PostLinks pl JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id),
// ClosedPosts AS (SELECT p.Id AS PostId, MAX(ph.CreationDate) AS LastClosedDate FROM Posts p JOIN PostHistory ph ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY p.Id),
// CombinedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, COALESCE(lp.LinkTypeName, 'No Links') AS LinkType, cp.LastClosedDate,
//        CASE WHEN cp.LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus, (rp.UpVotes - rp.DownVotes) AS ScoreDifference
//     FROM RankedPosts rp LEFT JOIN PostLinksWithTypes lp ON rp.PostId = lp.PostId AND lp.LinkRN = 1 LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT Title, CreationDate, CommentCount, LinkType, PostStatus, ScoreDifference,
//        CASE WHEN ScoreDifference > 0 THEN 'Positive' WHEN ScoreDifference < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreNature,
//        CONCAT('This post is ', CASE WHEN PostStatus = 'Closed' THEN 'not useful' ELSE 'useful' END, ' for further discussions.') AS PostUtility
// FROM CombinedPosts WHERE (ScoreDifference > 0 OR CommentCount > 5) AND PostStatus = 'Open' ORDER BY CommentCount DESC, CreationDate DESC LIMIT 50;
//
// RN is never read. A CreationDate tie for a post's latest link goes to the smaller link id, and a tie at the LIMIT to the smaller post id.
fn q21899(db: &'static So) -> String {
    let Post { creation_date, title, body, .. } = &db.post;
    let like = || title.opt().and(body).filt(|(t, b): (Option<Str>, Str)| t.map_or(false, |t| t.contains("SQL")) || b.contains("JOIN"));
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    let rp = || db.post.with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0))).with(like()).minus(&closed);
    let sd = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostLink { post, creation_date: ld, .. } = &db.post_link;
    let ll = top_per(drain(db.post_link.select(post)), |&(_, p)| p, |&(l, _)| (Reverse(ld.get(l).unwrap()), l), 1, false);
    let ll = rel(ll.into_iter().map(|(l, p)| (p, l)).collect());
    let ll_by: HashIdx<Id<Post>, (Id<Post>, Id<PostLink>)> = (&ll).map(|(p, _)| p).inv().select(&ll).collect();
    let v = drain((&sd).and(&cc).filt(|(s, c)| s > 0 || c > 5).and((&ll_by).map(|(_, l)| l).select((&db.post_link.link_type).select(&db.link_type.name)).opt()));
    let v = top_n(v, |&(p, ((_, c), _))| (Reverse(c), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((s, c), l))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(c), V::S(l.unwrap_or("No Links")), V::S("Open"), V::I(s)]);
        f.push(V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }));
        f.push(V::S("This post is useful for further discussions."));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(COALESCE(V.VoteCount, 0)) AS TotalVotes, COALESCE(SUM(B.Class), 0) AS TotalBadges,
//        CASE WHEN U.Reputation BETWEEN 0 AND 1000 THEN 'Novice' WHEN U.Reputation BETWEEN 1001 AND 5000 THEN 'Intermediate' ELSE 'Expert' END AS ReputationLevel
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) V ON V.PostId = P.Id
//     LEFT JOIN Badges B ON B.UserId = U.Id GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT PL.RelatedPostId) AS RelatedPostCount,
//        SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount, SUM(CASE WHEN PH.PostHistoryTypeId = 52 THEN 1 ELSE 0 END) AS HotCount,
//        DENSE_RANK() OVER (ORDER BY SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS VoteRank
//     FROM Posts P LEFT JOIN Comments C ON C.PostId = P.Id LEFT JOIN PostLinks PL ON PL.PostId = P.Id LEFT JOIN PostHistory PH ON PH.PostId = P.Id LEFT JOIN Votes V ON V.PostId = P.Id
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// UserRanking AS (SELECT UA.UserId, UA.DisplayName, UA.TotalPosts, UA.TotalQuestions, UA.TotalAnswers, UA.TotalVotes, UA.TotalBadges, UA.ReputationLevel,
//        CASE WHEN UA.TotalVotes > 100 THEN 'Highly Active' WHEN UA.TotalPosts >= 10 THEN 'Moderately Active' ELSE 'Less Active' END AS ActivityLevel
//     FROM UserActivity UA WHERE UA.TotalPosts > 0)
// SELECT UR.DisplayName AS User, UR.ReputationLevel, UR.ActivityLevel, PS.Title AS PostTitle, PS.Score AS PostScore, PS.CommentCount, PS.RelatedPostCount, PS.CloseCount, PS.HotCount, PS.VoteRank
// FROM UserRanking UR JOIN PostStatistics PS ON PS.VoteRank <= 10 ORDER BY UR.ReputationLevel, UR.ActivityLevel, PS.Score DESC;
//
// The ON clause names only PS, so the users with posts are crossed with the posts of VoteRank <= 10.
fn q24456(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and((&vc).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 2], |a, (p, _b)| [a[0] + p.and_then(|p| p.1).unwrap_or(0), a[1] + p.is_some() as i64]);
    let tp = user_distinct_posts(db);
    let ur = rel(drain((&ua).and((&tp).filt(|n| n > 0))));
    let PostHistory { post_history_type_id: ht, .. } = &db.post_history;
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(links_of(db).opt()).and(history_of(db).select(ht).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (((c, _), h), t)| [a[0] + c.is_some() as i64, a[1] + (h == Some(10)) as i64, a[2] + (h == Some(52)) as i64, a[3] + (t == Some(2)) as i64]);
    let rk = ranked(drain(&ps), |&(_, a)| Reverse(a[3]), true);
    let top = rel(rk.into_iter().take_while(|x| x.1 <= 10).collect::<Vec<_>>());
    let rl = db.post.group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    type R = ((Id<Post>, [i64; 4]), i64);
    let top = rel(drain((&top).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select((&rl).opt())))).into_iter().map(|x| x.1).collect::<Vec<_>>());
    let mut v = Vec::new();
    (&ur).cross(&top).drive(|_, ((u, (a, n)), (((p, s), r), l))| v.push((u, a, n, p, s, r, l)));
    rows(v.into_iter().map(|(u, a, n, p, s, r, l)| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name")];
        f.push(V::S(if (0..=1000).contains(&rep) { "Novice" } else if (1001..=5000).contains(&rep) { "Intermediate" } else { "Expert" }));
        f.push(V::S(if a[0] > 100 { "Highly Active" } else if n >= 10 { "Moderately Active" } else { "Less Active" }));
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(s[0]), V::I(l.unwrap_or(0)), V::I(s[1]), V::I(s[2]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, COUNT(*) AS CloseReasonsCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate),
// PostStatistics AS (SELECT p.Id AS PostId, COALESCE(cp.CloseReasonsCount, 0) AS CloseCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, p.ViewCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN ClosedPosts cp ON p.Id = cp.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, cp.CloseReasonsCount, p.ViewCount),
// FinalOutput AS (SELECT rp.PostId, rp.Title, rp.CreationDate, us.UsersWithBadges, ps.CloseCount, ps.UpVotes, ps.DownVotes, ps.ViewCount,
//        CASE WHEN ps.UpVotes - ps.DownVotes > 0 THEN 'Positive' WHEN ps.UpVotes - ps.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment
//     FROM RankedPosts rp LEFT JOIN (SELECT u.Id, COUNT(DISTINCT b.Id) AS UsersWithBadges FROM Users u INNER JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id) us ON rp.PostId = us.Id
//     LEFT JOIN PostStatistics ps ON rp.PostId = ps.PostId WHERE rp.Rank = 1)
// SELECT fo.PostId, fo.Title, COALESCE(fo.CreationDate, CAST('2024-10-01 12:34:56' AS TIMESTAMP)) AS PostCreationDate, fo.CloseCount, fo.UpVotes, fo.DownVotes, fo.ViewCount, fo.Sentiment
// FROM FinalOutput fo WHERE fo.CloseCount IS NULL OR fo.CloseCount < 5 ORDER BY fo.ViewCount DESC OFFSET 0 ROWS FETCH NEXT 100 ROWS ONLY;
//
// The `us` subquery has one row per user and UsersWithBadges is never projected, so that LEFT JOIN is left out. ClosedPosts has a row per (post, close
// date), so PostStatistics is grouped over the post x votes x close rows by (post, CloseReasonsCount). The ownerless questions are one partition; a
// CreationDate tie within an owner goes to the smaller post id.
fn q22193(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let cpg = db.post_history.with(ht.eq(10)).group_by(post.and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cpv = rel(drain(&cpg));
    let cp_by: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&cpv).map(|((p, _), _)| p).inv().select(&cpv).collect();
    let j: MatSet<(Id<Post>, Option<Id<Vote>>, Option<((Id<Post>, i64), i64)>)> = (&tp)
        .select(Ident::<Post>::new().and(votes_of(db).opt()).and((&cp_by).opt()))
        .map(|((p, v), c)| (p, v, c))
        .collect();
    type J = (Id<Post>, Option<Id<Vote>>, Option<((Id<Post>, i64), i64)>);
    let ps = (&j)
        .group_by(Same::<J>::new().map(|(p, _, c): J| (p, c.map_or(0, |c| c.1))))
        .select(Same::<J>::new().map(|(_, v, _): J| v).flat_map(|v: Option<Id<Vote>>| v).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain(rel(drain(&ps)).filt(|((_, c), _)| c < 5)).into_iter().map(|x| x.1).collect::<Vec<_>>();
    let v = top_n(v, |&((p, c), _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p, c)
    }, 100);
    rows(v.into_iter().map(|((p, c), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["views"]));
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.Tags, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 days' AND p.ViewCount > 100 AND p.Score IS NOT NULL),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopCommentedPosts AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COALESCE(MIN(c.CreationDate), '9999-12-31') AS FirstCommentDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id),
// LastActivity AS (SELECT p.Id AS PostId, MAX(p.LastActivityDate) AS LatestActivity FROM Posts p GROUP BY p.Id),
// EnhancedPostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Tags, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, tcp.CommentCount, tcp.FirstCommentDate,
//        la.LatestActivity, ROW_NUMBER() OVER (ORDER BY rp.Score DESC) AS Rank
//     FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN TopCommentedPosts tcp ON rp.PostId = tcp.PostId LEFT JOIN LastActivity la ON rp.PostId = la.PostId)
// SELECT ep.PostId, ep.Title, ep.CreationDate, ep.Score, ep.ViewCount, ep.Tags, ep.GoldBadges, ep.SilverBadges, ep.BronzeBadges, ep.CommentCount, ep.FirstCommentDate, ep.LatestActivity,
//        CASE WHEN ep.Score > 100 THEN 'High Score' WHEN ep.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory,
//        CASE WHEN ep.FirstCommentDate < CURRENT_DATE - INTERVAL '7 days' THEN 'Stale' ELSE 'Recent' END AS CommentRecency
// FROM EnhancedPostDetails ep WHERE ep.Rank <= 50 AND ep.GoldBadges > 0 ORDER BY ep.Score DESC, ep.CommentCount DESC;
//
// rn is never read. Rank reads only Score, so the top 50 posts are picked first (a tie goes to the smaller post id).
fn q24215(db: &'static So) -> String {
    let Post { creation_date, view_count, score, owner_user, post_type_id, .. } = &db.post;
    let today = current_date();
    let v = drain(db.post.with(creation_date.ge(add_days(today, -30))).with(view_count.gt(100)).select(score));
    let tp = top_n(v, |&(p, s)| (Reverse(s), p), 50);
    let tp: MatSet<Id<Post>> = rel(tp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let tc = (&tp).with(post_type_id.is_in([1, 2])).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MAX), |(n, m), d| match d {
        Some(d) => (n + 1, m.min(d)),
        None => (n, m),
    });
    let mut v = drain((&tp).select(owner_user.select((&ub).filt(|b| b[0] > 0)).and((&tc).opt())));
    v.sort_by_key(|&(p, (_, c))| (Reverse(score.get(p).unwrap()), c.map(|c| Reverse(c.0))));
    let far = ts(9999, 12, 31, 0, 0, 0);
    rows(v.into_iter().map(|(p, (b, c))| {
        let s = score.get(p).unwrap();
        let fc = c.map(|c| if c.1 == i64::MAX { far } else { c.1 });
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "tags"]);
        f.extend(b.map(V::I));
        f.extend([oint(c.map(|c| c.0)), ots(fc), V::T(db.post.last_activity_date.get(p).unwrap())]);
        f.push(V::S(if s > 100 { "High Score" } else if (50..=100).contains(&s) { "Medium Score" } else { "Low Score" }));
        f.push(V::S(if fc.map_or(false, |d| d < add_days(today, -7)) { "Stale" } else { "Recent" }));
        row(f)
    }))
}

// WITH RecentUserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyReceived,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY u.LastAccessDate DESC) AS ActivityRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.UserId = u.Id WHERE u.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY u.Id, u.DisplayName, u.LastAccessDate),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(*) AS PostsCreated, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount, SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId),
// ClosedPosts AS (SELECT ph.UserId, COUNT(ph.Id) AS ClosedPostCount, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalClosed,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS TotalReopened FROM PostHistory ph WHERE ph.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY ph.UserId)
// SELECT ua.UserId, ua.DisplayName, ua.PostCount, ua.QuestionsAsked, ua.AnswersGiven, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, COALESCE(ps.PostsCreated, 0) AS PostsCreated, COALESCE(ps.AvgViewCount, 0) AS AvgViewCount, COALESCE(ps.TotalScore, 0) AS TotalScore,
//        COALESCE(cp.ClosedPostCount, 0) AS ClosedPosts, COALESCE(cp.TotalClosed, 0) AS TotalClosed, COALESCE(cp.TotalReopened, 0) AS TotalReopened
// FROM RecentUserActivity ua LEFT JOIN UserBadges ub ON ua.UserId = ub.UserId LEFT JOIN PostStats ps ON ua.UserId = ps.OwnerUserId LEFT JOIN ClosedPosts cp ON ua.UserId = cp.UserId
// WHERE ua.ActivityRank <= 10 ORDER BY ua.PostCount DESC, ua.DisplayName;
//
// ActivityRank partitions by the user's own id, so it is 1 for every user.
fn q22199(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, owner_user, view_count, score, .. } = &db.post;
    let nu = || db.user.with((&db.user.creation_date).gt(add_years(t0, -1)));
    let ua = nu()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.flatten().unwrap_or(0)]);
    let pc = nu().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ps = db.post.group_by(owner_user).select(view_count.opt().and(score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let PostHistory { post_history_type_id: ht, creation_date: hd, user, .. } = &db.post_history;
    let cp = db.post_history.with(hd.gt(add_months(t0, -6))).group_by(user).select(ht).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 10) as i64, a[2] + (t == 11) as i64]);
    let v = drain((&ua).and(&pc).and((&ub).opt()).and((&ps).opt()).and((&cp).opt()));
    rows(v.into_iter().map(|(u, ((((a, n), b), p), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(match p {
            Some(p) => [V::I(p[0]), avg(p[1], p[0]), V::I(p[2])],
            None => [V::I(0), V::F(0.0), V::I(0)],
        });
        f.extend(c.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserScore AS (SELECT u.Id, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Score,
//        COUNT(DISTINCT ph.PostId) AS PostHistoryCount, COUNT(DISTINCT b.Id) AS BadgeCount,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS RowNum
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.CreationDate <= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, AVG(LENGTH(p.Body)) AS AvgBodyLength
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY p.Id, p.Title),
// CloseReasonCounts AS (SELECT ph.PostId, ph.Comment AS CloseReason, COUNT(*) AS ReasonCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.Comment),
// AggregatedData AS (SELECT us.Id AS UserId, us.DisplayName, us.Score, ps.PostId, ps.Title, ps.CommentCount, ps.VoteCount, ps.AvgBodyLength,
//        COALESCE(cr.CloseReason, 'No close reasons') AS CloseReason, COALESCE(cr.ReasonCount, 0) AS ReasonCount,
//        CASE WHEN us.Score > 100 THEN 'High Scorer' WHEN us.Score BETWEEN 50 AND 100 THEN 'Medium Scorer' ELSE 'Low Scorer' END AS ScoreCategory
//     FROM UserScore us LEFT JOIN PostStats ps ON us.Id = ps.PostId LEFT JOIN CloseReasonCounts cr ON ps.PostId = cr.PostId WHERE us.RowNum <= 10)
// SELECT UserId, DisplayName, Score, PostId, Title, CommentCount, VoteCount, AvgBodyLength, CloseReason, ReasonCount, ScoreCategory
// FROM AggregatedData ORDER BY Score DESC, ReasonCount DESC, AvgBodyLength ASC LIMIT 50;
//
// PostHistoryCount and BadgeCount are never read. `us.Id = ps.PostId` joins a user id to a post id, so it goes through the raw ids. A Score tie at RowNum 10
// goes to the smaller user id.
fn q22747(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { origid, creation_date, body, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.creation_date).le(add_years(t0, -1)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(history_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold(0i64, |n, (p, _)| match p {
            Some((t, _)) => n + (t == Some(2)) as i64 - (t == Some(3)) as i64,
            None => n,
        });
    let tu = rel(top_n(drain(&us), |&(u, s)| (Reverse(s), u), 10));
    let recent = || db.post.with(creation_date.ge(add_months(t0, -1)));
    let pc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let pv = recent().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let PostHistory { post, post_history_type_id: ht, comment, .. } = &db.post_history;
    let crc = db.post_history.with(ht.eq(10)).group_by(post.and(comment.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let crv = rel(drain(&crc));
    let cr_by: HashIdx<Id<Post>, ((Id<Post>, Option<Str>), i64)> = (&crv).map(|((p, _), _)| p).inv().select(&crv).collect();
    let pidx: HashIdx<i64, Id<Post>> = origid.inv().collect();
    type R = (Id<User>, i64);
    let ps = Same::<R>::new().map(|(u, _): R| u).select(&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&pc).and(&pv).and((&cr_by).opt()));
    let v = drain((&tu).select(Same::<R>::new().and(ps.opt())));
    let v = top_n(v, |&(_, ((u, s), p))| {
        let rc = p.and_then(|(_, c)| c).map_or(0, |c| c.1);
        let bl = p.map(|(((q, _), _), _)| body.get(q).unwrap().chars().count() as i64);
        (Reverse(s), Reverse(rc), bl.is_none(), bl, u)
    }, 50);
    rows(v.into_iter().map(|(_, ((u, s), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(s));
        f.extend(match p {
            Some((((q, c), n), cr)) => {
                let mut g = post_fields(db, q, &["id", "title"]);
                g.extend([V::I(c), V::I(n), V::F(body.get(q).unwrap().chars().count() as f64)]);
                g.extend(match cr {
                    Some(((_, r), k)) => [ostr(r), V::I(k)],
                    None => [V::S("No close reasons"), V::I(0)],
                });
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null, V::S("No close reasons"), V::I(0)],
        });
        f.push(V::S(if s > 100 { "High Scorer" } else if (50..=100).contains(&s) { "Medium Scorer" } else { "Low Scorer" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS ViewRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, pt.Name AS HistoryType, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph INNER JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id WHERE pt.Name IN ('Post Closed', 'Post Reopened')),
// AggregatedPostStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount,
//        COALESCE(cp.HistoryType, 'No History') AS ClosureStatus, DENSE_RANK() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS ScoreViewRank
//     FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId AND cp.HistoryRank = 1)
// SELECT aps.PostId, aps.Title, aps.CreationDate, aps.Score, aps.ViewCount, aps.CommentCount, aps.UpVoteCount, aps.DownVoteCount, aps.ClosureStatus,
//        CASE WHEN aps.ClosureStatus = 'Post Closed' THEN 'Closed' WHEN aps.ClosureStatus = 'Post Reopened' THEN 'Reopened' ELSE 'Active' END AS Status,
//        (aps.UpVoteCount - aps.DownVoteCount) AS NetVotes, NULLIF(aps.CommentCount, 0) AS NonZeroComments, CASE WHEN aps.Score IS NULL THEN 'No Score' ELSE 'Scored' END AS ScorePresence,
//        CONCAT('Title: ', aps.Title, ' | Status: ', CASE WHEN aps.ClosureStatus = 'No History' THEN 'Active' ELSE aps.ClosureStatus END) AS DisplayInfo
// FROM AggregatedPostStats aps WHERE aps.ScoreViewRank <= 10 OR aps.ClosureStatus != 'No History' ORDER BY aps.Score DESC, aps.ViewCount DESC;
//
// RankedPosts has no GROUP BY, so it has one row per post x comment x vote; each is emitted. ScoreViewRank is DENSE_RANK over values the rows of one post
// share, so it is taken over the posts. ViewRank is never read. A CreationDate tie for a post's latest close/reopen row goes to the larger history id.
fn q23666(db: &'static So) -> String {
    let Post { creation_date, score, view_count, title, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let dr = ranked(drain(recent().select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, true);
    let dr = rel(dr.into_iter().map(|((p, _), r)| (p, r)).collect());
    let dr_by: HashIdx<Id<Post>, (Id<Post>, i64)> = (&dr).map(|(p, _)| p).inv().select(&dr).collect();
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let cl = drain(db.post_history.with(htype_name(db).filt(|n: Str| n == "Post Closed" || n == "Post Reopened")).select(post));
    let cl = top_per(cl, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let cl = rel(cl.into_iter().map(|(h, p)| (p, h)).collect());
    let cl_by: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&cl).map(|(p, _)| p).inv().select(&cl).collect();
    let keep: MatSet<Id<Post>> = recent()
        .select((&dr_by).map(|(_, r)| r).and((&cl_by).opt()))
        .filt(|(r, c): (i64, Option<(Id<Post>, Id<PostHistory>)>)| r <= 10 || c.is_some())
        .map(|_| ())
        .inv()
        .map(|p| p)
        .collect();
    let agg = (&keep).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&keep).select(comments_of(db).opt().and(votes_of(db).opt())).and(&agg).and((&cl_by).map(|(_, h)| h).select(htype_name(db)).opt()));
    rows(v.into_iter().map(|(p, (((_, _), a), h))| {
        let st = h.unwrap_or("No History");
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(st)]);
        f.push(V::S(if st == "Post Closed" { "Closed" } else if st == "Post Reopened" { "Reopened" } else { "Active" }));
        f.extend([V::I(a[1] - a[2]), if a[0] == 0 { V::Null } else { V::I(a[0]) }, V::S("Scored")]);
        f.push(V::Owned(format!("Title: {} | Status: {}", title.get(p).unwrap_or(""), if st == "No History" { "Active" } else { st })));
        row(f)
    }))
}

// Rewritten (rewrites/22945.sql): the final ORDER BY is tie-broken on RP.PostId, PT.TagName.
// WITH RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, U.DisplayName, P.PostTypeId, COALESCE(AR.AcceptedAnswerId, -1) AS AcceptedAnswerId,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes, P.OwnerUserId, P.Tags
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Posts AR ON P.Id = AR.AcceptedAnswerId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate > DATE('2024-10-01') - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.ViewCount, P.CreationDate, U.DisplayName, P.PostTypeId, AR.AcceptedAnswerId, P.OwnerUserId, P.Tags),
// ActiveUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U WHERE U.LastAccessDate > DATE('2024-10-01') - INTERVAL '60 days'),
// PopularTags AS (SELECT T.TagName, COUNT(P.Id) AS PostCount FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%') GROUP BY T.TagName HAVING COUNT(P.Id) > 10),
// ExceptionalCase AS (SELECT PH.PostId, PH.PostHistoryTypeId, COUNT(PH.Id) AS HistoryCount FROM PostHistory PH GROUP BY PH.PostId, PH.PostHistoryTypeId
//     HAVING COUNT(PH.Id) > 5 AND PH.PostHistoryTypeId IN (10, 12))
// SELECT RP.PostId, RP.Title, RP.ViewCount, RP.CreationDate, RP.DisplayName, RP.PostTypeId, RP.AcceptedAnswerId, RP.UpVotes, RP.DownVotes, AU.UserRank, PT.TagName,
//        CASE WHEN EC.PostId IS NOT NULL THEN 'Caution: Post Closed or Deleted' ELSE 'Active Post' END AS PostStatus,
//        CASE WHEN RP.ViewCount IS NULL THEN 'No Views Yet' WHEN RP.ViewCount < 100 THEN 'Low Visibility' WHEN RP.ViewCount BETWEEN 100 AND 500 THEN 'Moderate Visibility' ELSE 'High Visibility' END AS VisibilityStatus
// FROM RecentPosts RP LEFT JOIN ActiveUsers AU ON RP.OwnerUserId = AU.UserId LEFT JOIN PopularTags PT ON RP.Tags LIKE CONCAT('%', PT.TagName, '%')
// LEFT JOIN ExceptionalCase EC ON RP.PostId = EC.PostId WHERE (RP.UpVotes - RP.DownVotes) >= 0
// ORDER BY RP.ViewCount DESC, AU.UserRank ASC NULLS LAST, RP.PostId, PT.TagName LIMIT 100;
//
// The LIKE against a tag name is `tag_mentions` (a tag name never contains < or >). The AR join groups by the accepting question's AcceptedAnswerId,
// which is this post's id whatever question it is, so the vote counts run over the post x accepting-questions x votes rows.
fn q22945(db: &'static So) -> String {
    let Post { creation_date, accepted_answer, view_count, owner_user, origid, .. } = &db.post;
    let rp = || db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 0, 0, 0), -30)));
    let acc_by: HashIdx<Id<Post>, Id<Post>> = db.post.select(accepted_answer).inv().collect();
    let g = rp().group_by(Ident::<Post>::new().and((&acc_by).opt().map(|a: Option<Id<Post>>| a.is_some())))
        .select((&acc_by).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let au = ranked(drain(db.user.with((&db.user.last_access_date).gt(add_days(ts(2024, 10, 1, 0, 0, 0), -60))).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let au = rel(au.into_iter().map(|((u, _), r)| (u, r)).collect());
    let au_by: HashIdx<Id<User>, (Id<User>, i64)> = (&au).map(|(u, _)| u).inv().select(&au).collect();
    let pop: MatSet<Id<Tag>> = db.tag.with((&tag_stats(db)).filt(|a| a[0] > 10)).collect();
    let tm = tag_mentions(db);
    type PT = (Id<Post>, Id<Tag>);
    let pairs: MatSet<PT> = (&tm).with(Same::<PT>::new().map(|(_, t): PT| t).select(Ident::<Tag>::new().with(&pop))).collect();
    let pt_by: HashIdx<Id<Post>, PT> = (&pairs).map(|(p, _): PT| p).inv().collect();
    let PostHistory { post, post_history_type_id: ht, .. } = &db.post_history;
    let ec = db.post_history.with(ht.is_in([10, 12])).group_by(post.and(ht)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let ecv = rel(drain((&ec).filt(|n| n > 5)));
    let ec_by: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&ecv).map(|((p, _), _)| p).inv().select(&ecv).collect();
    type K = (Id<Post>, bool);
    let pk = || Same::<K>::new().map(|(p, _): K| p);
    let v = drain((&g).filt(|a| a[0] - a[1] >= 0).and(pk().select(owner_user.select(&au_by).opt()).opt()).and(pk().select((&pt_by).opt())).and(pk().select((&ec_by).opt())));
    let v = top_n(v, |&((p, _), (((_, r), t), _))| {
        let w = view_count.get(p);
        let r = r.flatten().map(|x| x.1);
        let tn = t.map(|t| db.tag.tag_name.get(t.1).unwrap());
        (w.is_none(), Reverse(w), r.is_none(), r, origid.get(p).unwrap(), tn.is_none(), tn)
    }, 100);
    rows(v.into_iter().map(|((p, acc), (((a, r), t), e))| {
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "owner", "type_id"]);
        f.push(V::I(if acc { origid.get(p).unwrap() } else { -1 }));
        f.extend([V::I(a[0]), V::I(a[1]), oint(r.flatten().map(|x| x.1)), t.map_or(V::Null, |t| V::S(db.tag.tag_name.get(t.1).unwrap()))]);
        f.push(V::S(if e.is_some() { "Caution: Post Closed or Deleted" } else { "Active Post" }));
        f.push(V::S(match w {
            None => "No Views Yet",
            Some(w) if w < 100 => "Low Visibility",
            Some(w) if w <= 500 => "Moderate Visibility",
            _ => "High Visibility",
        }));
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT u.Id AS UserId, u.Reputation, u.CreationDate, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount,
//        SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation, u.CreationDate, u.DisplayName),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// AcceptedAnswers AS (SELECT p.OwnerUserId, COUNT(a.Id) AS AcceptedAnswerCount FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId),
// PostHistoryCount AS (SELECT ph.UserId, COUNT(*) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.UserId),
// UserSummary AS (SELECT um.UserId, um.DisplayName, um.Reputation, COALESCE(rp.PostRank, 0) AS RecentPostRank, COALESCE(a.AcceptedAnswerCount, 0) AS AcceptedAnswerCount,
//        COALESCE(ph.EditCount, 0) AS TotalEdits
//     FROM UserMetrics um LEFT JOIN RecentPosts rp ON um.UserId = rp.OwnerUserId AND rp.PostRank = 1 LEFT JOIN AcceptedAnswers a ON um.UserId = a.OwnerUserId
//     LEFT JOIN PostHistoryCount ph ON um.UserId = ph.UserId)
// SELECT us.DisplayName, us.Reputation, CASE WHEN us.Reputation >= 10000 THEN 'High Reputation' WHEN us.Reputation BETWEEN 5000 AND 9999 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationTier,
//        us.RecentPostRank AS MostRecentPostRank, us.AcceptedAnswerCount, us.TotalEdits, CASE WHEN us.Reputation IS NULL THEN 'No Reputation Data' ELSE NULL END AS ReputationNullCheck
// FROM UserSummary us WHERE us.Reputation >= 10000 ORDER BY us.Reputation DESC LIMIT 10;
//
// UserMetrics has one row per user and only DisplayName and Reputation are read, so its aggregates are not computed. RecentPosts' rank-1 row exists exactly
// when the user has a post in the window, so MostRecentPostRank is 1 or 0.
fn q23733(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, accepted_answer, .. } = &db.post;
    let recent: MatSet<Id<User>> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user).collect();
    let aa = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(accepted_answer.opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let PostHistory { post_history_type_id: ht, user, .. } = &db.post_history;
    let ec = db.post_history.with(ht.is_in([4, 5, 6])).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with((&db.user.reputation).ge(10000)).select(Ident::<User>::new().with(&recent).opt().and((&aa).opt()).and((&ec).opt())));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, ((r, a), e))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::S("High Reputation"), V::I(r.is_some() as i64), V::I(a.unwrap_or(0)), V::I(e.unwrap_or(0)), V::Null]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Score, P.Title, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS OwnerRank
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND P.PostTypeId = 1),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate,
//        MAX(PH.CreationDate) OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) AS MostRecentCloseDate
//     FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11)),
// PostDetails AS (SELECT RP.PostId, RP.Title, UR.DisplayName, UR.Reputation, RP.Score, C.MostRecentCloseDate, CASE WHEN C.MostRecentCloseDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus
//     FROM RankedPosts RP JOIN UserReputation UR ON RP.OwnerUserId = UR.UserId LEFT JOIN ClosedPosts C ON RP.PostId = C.PostId),
// FinalResults AS (SELECT PD.PostId, PD.Title, PD.DisplayName, PD.Reputation, PD.Score, PD.MostRecentCloseDate, PD.PostStatus, COUNT(CMT.Id) AS CommentCount, AVG(V.BountyAmount) AS AverageBounty
//     FROM PostDetails PD LEFT JOIN Comments CMT ON PD.PostId = CMT.PostId LEFT JOIN Votes V ON PD.PostId = V.PostId AND V.VoteTypeId = 8
//     GROUP BY PD.PostId, PD.Title, PD.DisplayName, PD.Reputation, PD.Score, PD.MostRecentCloseDate, PD.PostStatus)
// SELECT PostId, Title, DisplayName, Reputation, Score, PostStatus, CommentCount, COALESCE(AverageBounty, 0) AS AverageBounty,
//        CASE WHEN Reputation >= 1000 THEN 'High Reputation' WHEN Reputation BETWEEN 500 AND 999 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationTier
// FROM FinalResults WHERE CommentCount > 5 ORDER BY Score DESC, Reputation DESC;
//
// OwnerRank and the badge sums are never read. ClosedPosts has a row per close/reopen history row, all with the post's MostRecentCloseDate, so the
// final group runs over the post x close rows x comments x bounty votes.
fn q24187(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let rp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).with(owner_user);
    let PostHistory { post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let cl = history_of(db).select(Ident::<PostHistory>::new().with(ht.is_in([10, 11]))).select(hd);
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let fr = rp
        .group_by(Ident::<Post>::new())
        .select(cl.opt().and(comments_of(db).opt()).and(bv.opt()))
        .fold([0i64, 0, 0, i64::MIN], |a, ((d, c), b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0), a[3].max(d.unwrap_or(i64::MIN))]
        });
    let mut v = drain((&fr).filt(|a| a[0] > 5));
    v.sort_by_key(|&(p, _)| (Reverse(db.post.score.get(p).unwrap()), Reverse(db.user.reputation.get(owner_user.get(p).unwrap()).unwrap())));
    rows(v.into_iter().map(|(p, a)| {
        let rep = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "rep", "score"]);
        f.extend([V::S(if a[3] != i64::MIN { "Closed" } else { "Active" }), V::I(a[0]), if a[1] == 0 { V::F(0.0) } else { avg(a[2], a[1]) }]);
        f.push(V::S(if rep >= 1000 { "High Reputation" } else if (500..=999).contains(&rep) { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostWithBadges AS (SELECT rp.PostId, rp.Title, rp.Score, rp.OwnerDisplayName, b.Class, COUNT(b.Id) AS BadgeCount
//     FROM RankedPosts rp LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) GROUP BY rp.PostId, rp.Title, rp.Score, rp.OwnerDisplayName, b.Class),
// TopPosts AS (SELECT PostId, Title, Score, OwnerDisplayName, BadgeCount, RANK() OVER (ORDER BY Score DESC) AS OverallRank FROM PostWithBadges WHERE BadgeCount > 0),
// RecentEdits AS (SELECT p.Id AS PostId, ph.UserDisplayName AS Editor, ph.CreationDate AS EditDate, ph.Comment FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) AND ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(DISTINCT ph.PostId) AS PostsEdited
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts p ON U.Id = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY U.Id, U.DisplayName)
// SELECT tp.Title, tp.Score, tp.OwnerDisplayName, tp.BadgeCount, ua.Upvotes, ua.Downvotes, ua.PostsEdited, COALESCE(recent.Editor, 'No Edits') AS LastEditor,
//        COALESCE(CAST(recent.EditDate AS VARCHAR), 'N/A') AS LastEditDate, COALESCE(recent.Comment, 'No comments') AS EditComment
// FROM TopPosts tp LEFT JOIN RecentEdits recent ON tp.PostId = recent.PostId LEFT JOIN UserActivity ua ON tp.OwnerDisplayName = ua.DisplayName
// WHERE tp.OverallRank <= 10 ORDER BY tp.Score DESC, tp.BadgeCount DESC;
//
// PostRank is never read. PostWithBadges has a row per (post, badge class of its owner), built from the per-(user, class) badge counts, and OverallRank ranks
// those rows. UserActivity is joined on the display name, so it is computed only for users sharing a top post owner's name.
fn q32792(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let ubc = db.badge.group_by((&db.badge.user).and(&db.badge.class)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ubv = rel(drain(&ubc));
    let ub_by: HashIdx<Id<User>, ((Id<User>, i64), i64)> = (&ubv).map(|((u, _), _)| u).inv().select(&ubv).collect();
    let pwb = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.select(&ub_by)));
    let rk = ranked(pwb, |&(p, _)| Reverse(score.get(p).unwrap()), false);
    let tp = rel(rk.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect::<Vec<_>>());
    let PostHistory { post_history_type_id: ht, creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    let re = history_of(db).select(Ident::<PostHistory>::new().with(ht.is_in([4, 5, 6]).and(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))));
    let names: MatSet<Str> = (&tp).map(|(p, _)| p).select(owner_user).select(&db.user.display_name).collect();
    let named: MatSet<Id<User>> = db.user.with((&db.user.display_name).select(&names)).collect();
    let ua = (&named)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(history_of(db).opt()).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ue = (&named).group_by(Ident::<User>::new()).select(posts_of(db).select(history_of(db)).select(&db.post_history.post)).count_distinct();
    let by_name: HashIdx<Str, Id<User>> = (&named).select(&db.user.display_name).inv().collect();
    type R = (Id<Post>, ((Id<User>, i64), i64));
    let pk = || Same::<R>::new().map(|(p, _): R| p);
    let v = drain((&tp).select(Same::<R>::new().and(pk().select(re.opt())).and(pk().select(owner_user).select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&ua).and((&ue).opt())).opt())));
    let mut v = v;
    v.sort_by_key(|&(_, (((p, (_, n)), _), _))| (Reverse(score.get(p).unwrap()), Reverse(n)));
    rows(v.into_iter().map(|(_, (((p, (_, n)), h), u))| {
        let mut f = post_fields(db, p, &["title", "score", "owner"]);
        f.push(V::I(n));
        f.extend(match u {
            Some(((_, a), e)) => [V::I(a[0]), V::I(a[1]), V::I(e.unwrap_or(0))],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match h {
            Some(h) => [V::S(user_display_name.get(h).unwrap_or("No Edits")), V::Owned(ts_text(hd.get(h).unwrap())), V::S(comment.get(h).unwrap_or("No comments"))],
            None => [V::S("No Edits"), V::S("N/A"), V::S("No comments")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(b.Class, 0)) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// PostHistoryMetrics AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '6 months' AND ph.PostHistoryTypeId IN (10, 11, 12)),
// CombinedMetrics AS (SELECT ra.PostId, ra.Title, ra.CreationDate, ua.UserId, ua.DisplayName, ra.PostRank, ra.CommentCount, COALESCE(pm.PostHistoryTypeId, 0) AS RecentActivity,
//        ua.TotalBounties, ua.TotalPosts, ua.TotalBadges
//     FROM RankedPosts ra LEFT JOIN UserActivity ua ON ra.OwnerUserId = ua.UserId LEFT JOIN PostHistoryMetrics pm ON ra.PostId = pm.PostId)
// SELECT cm.PostId, cm.Title, cm.CreationDate, cm.DisplayName AS Owner, cm.CommentCount, CASE WHEN cm.PostRank = 1 THEN 'Most Recent Post' ELSE 'Older Post' END AS PostClassification,
//        COUNT(DISTINCT ul.UserId) AS UniqueLikers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalLikes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDislikes
// FROM CombinedMetrics cm LEFT JOIN Votes v ON v.PostId = cm.PostId LEFT JOIN (SELECT DISTINCT UserId FROM Votes WHERE VoteTypeId = 2) ul ON ul.UserId = v.UserId
// WHERE cm.TotalBadges >= 3 GROUP BY cm.PostId, cm.Title, cm.CreationDate, cm.DisplayName, cm.PostRank, cm.CommentCount
// ORDER BY cm.CreationDate DESC, cm.PostRank LIMIT 100 OFFSET 0;
//
// RankedPosts has one row per post x comment, numbered within the owner; the rows of one post tie on CreationDate and the tie goes to the smaller comment id.
// Each (post, PostRank) group is one of those rows, so its aggregates run over the post's recent history x votes x likers. TotalBadges is SUM(Class) over
// the owner's posts x votes cast x badges.
fn q22725(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(t0, -1)));
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let tb = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(votes_by(db).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold(0i64, |s, (_, c)| s + c.unwrap_or(0));
    let keep = || recent().with(owner_user.select((&tb).filt(|s| s >= 3)));
    let j: MatSet<(Id<Post>, Option<Id<Comment>>)> = keep().select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    type J = (Id<Post>, Option<Id<Comment>>);
    let jv = drain((&j).map(|(p, _): J| p).select(owner_user));
    let rk = per_group(ranked(jv, |&((p, c), u)| (u, Reverse(creation_date.get(p).unwrap()), p, c), false), |&(_, u)| u);
    let cc = keep().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let ph = || history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_months(t0, -6)).and(ht.is_in([10, 11, 12]))));
    let likers: MatSet<Id<User>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.user).collect();
    let vt = votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.user).select(Ident::<User>::new().with(&likers)).opt()));
    let agg = keep().group_by(Ident::<Post>::new()).select(ph().opt().and(vt.opt())).fold([0i64; 2], |a, (_, v)| {
        let t = v.map(|v| v.0);
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let ul = keep().group_by(Ident::<Post>::new()).select(ph().opt().and(votes_of(db).select((&db.vote.user).select(Ident::<User>::new().with(&likers))))).select(Same::<(Option<Id<PostHistory>>, Id<User>)>::new().map(|(_, u)| u)).count_distinct();
    type R = ((J, Id<User>), i64);
    let rr = rel(rk);
    let pk = || Same::<R>::new().map(|(((p, _), _), _): R| p);
    let v = drain((&rr).select(Same::<R>::new().and(pk().select(&cc)).and(pk().select(&agg)).and(pk().select((&ul).opt()))));
    let v = top_n(v, |&(_, ((((((p, c), _), r), _), _), _))| (Reverse(creation_date.get(p).unwrap()), r, p, c), 100);
    rows(v.into_iter().map(|(_, ((((((p, _), u), r), c), a), l))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(c), V::S(if r == 1 { "Most Recent Post" } else { "Older Post" }), V::I(l.unwrap_or(0)), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation IS NOT NULL),
// FilteredPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.AcceptedAnswerId,
//        CASE WHEN P.ViewCount > 1000 THEN 'High' WHEN P.ViewCount BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'Low' END AS ViewCategory
//     FROM Posts P WHERE P.CreationDate >= '2023-10-01 12:34:56'::timestamp - INTERVAL '1 YEAR' AND P.ViewCount IS NOT NULL),
// PostStatistics AS (SELECT FP.PostId, FP.Title, FP.ViewCategory, PS.Score, COALESCE(NULLIF(COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END), 0), 0) AS UpVotes,
//        COALESCE(NULLIF(COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END), 0), 0) AS DownVotes, COALESCE(NULLIF(FP.AcceptedAnswerId, -1), 0) AS HasAcceptedAnswer
//     FROM FilteredPosts FP LEFT JOIN Posts PS ON FP.PostId = PS.Id LEFT JOIN Votes V ON PS.Id = V.PostId GROUP BY FP.PostId, FP.Title, FP.ViewCategory, PS.Score, FP.AcceptedAnswerId),
// PostHistoryAnalyzed AS (SELECT PH.PostId, PH.UserId, PH.PostHistoryTypeId, COUNT(*) AS HistoryCount FROM PostHistory PH
//     WHERE PH.CreationDate >= '2024-04-01 12:34:56'::timestamp - INTERVAL '6 MONTH' AND PH.PostHistoryTypeId IN (10, 11, 12) GROUP BY PH.PostId, PH.UserId, PH.PostHistoryTypeId),
// AggregatedData AS (SELECT PS.PostId, PS.Title, PS.ViewCategory, PS.UpVotes, PS.DownVotes, PH.UserId AS HistoryUserId, SUM(PH.HistoryCount) AS PostHistoryChangeCount
//     FROM PostStatistics PS LEFT JOIN PostHistoryAnalyzed PH ON PS.PostId = PH.PostId GROUP BY PS.PostId, PS.Title, PS.ViewCategory, PS.UpVotes, PS.DownVotes, PH.UserId)
// SELECT RU.DisplayName, AD.Title, AD.ViewCategory, AD.UpVotes, AD.DownVotes, AD.PostHistoryChangeCount,
//        CASE WHEN AD.UpVotes > AD.DownVotes THEN 'Positive' WHEN AD.UpVotes < AD.DownVotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM RankedUsers RU JOIN AggregatedData AD ON RU.UserId = AD.HistoryUserId WHERE RU.ReputationRank <= 100 ORDER BY AD.UpVotes DESC, AD.DownVotes ASC, RU.DisplayName ASC;
//
// PostHistoryChangeCount sums the per-type counts of one (post, user), which is that user's history rows on the post. A Reputation tie at rank 100 goes to the
// smaller user id.
fn q21525(db: &'static So) -> String {
    let ru = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 100);
    let ru: MatSet<Id<User>> = rel(ru.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { creation_date, view_count, .. } = &db.post;
    let fp: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2023, 10, 1, 12, 34, 56), -1))).with(view_count).collect();
    let PostHistory { post, user, post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let ad = db
        .post_history
        .with(hd.ge(add_months(ts(2024, 4, 1, 12, 34, 56), -6)).and(ht.is_in([10, 11, 12])))
        .with(post.select(Ident::<Post>::new().with(&fp)))
        .with(user.select(Ident::<User>::new().with(&ru)))
        .group_by(post.and(user))
        .select(Ident::<PostHistory>::new())
        .fold(0i64, |n, _| n + 1);
    let ps = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type K = (Id<Post>, Id<User>);
    let v = drain((&ad).and(Same::<K>::new().map(|(p, _): K| p).select(&ps)));
    rows(v.into_iter().map(|((p, u), (n, a))| {
        let w = view_count.get(p).unwrap();
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::S(if w > 1000 { "High" } else if w >= 500 { "Medium" } else { "Low" }), V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges, SUM(CASE WHEN b.TagBased THEN 1 ELSE 0 END) AS TagBasedBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostActivity AS (SELECT p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        COALESCE(SUM(p.FavoriteCount), 0) AS FavoriteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId),
// PostHistoryDetails AS (SELECT ph.UserId, ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, r.Name AS PostHistoryTypeName FROM PostHistory ph JOIN PostHistoryTypes r ON ph.PostHistoryTypeId = r.Id
//     WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RecentPostRank FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '365 days')
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ub.TagBasedBadges, pa.CommentCount, pa.UpvoteCount, pa.DownvoteCount, pa.FavoriteCount,
//        COUNT(DISTINCT ph.PostId) AS PostHistoryCount, COUNT(DISTINCT rp.Id) AS RecentPostsCount
// FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN PostActivity pa ON u.Id = pa.OwnerUserId LEFT JOIN PostHistoryDetails ph ON u.Id = ph.UserId
// LEFT JOIN RankedPosts rp ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Posts.Id = rp.Id LIMIT 1)
// GROUP BY u.Id, u.DisplayName, u.Reputation, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ub.TagBasedBadges, pa.CommentCount, pa.UpvoteCount, pa.DownvoteCount, pa.FavoriteCount
// HAVING COUNT(DISTINCT rp.Id) > 5 AND SUM(COALESCE(pa.UpvoteCount, 0) - COALESCE(pa.DownvoteCount, 0)) > 10 ORDER BY u.Reputation DESC, PostHistoryCount DESC;
//
// The correlated LIMIT 1 looks a post up by its id, so it is the post's owner. RecentPostRank is never read. The HAVING SUM runs over the user's recent
// history x recent posts rows, each carrying the user's one PostActivity value.
fn q21162(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { owner_user, favorite_count, creation_date, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select((&db.badge.class).and(&db.badge.tag_based)).opt()).fold([0i64; 4], |a, b| match b {
        Some((c, t)) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64, a[3] + (t != 0) as i64],
        None => a,
    });
    let pa = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(favorite_count.opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 5], |a, ((f, c), t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + f.is_some() as i64, a[4] + f.unwrap_or(0)]);
    let PostHistory { creation_date: hd, user, post, .. } = &db.post_history;
    let ph_by: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(hd.ge(add_days(t0, -30))).select(user).inv().collect();
    let rp_by: HashIdx<Id<User>, Id<Post>> = db.post.with(creation_date.ge(add_days(t0, -365))).select(owner_user).inv().collect();
    let prod = db
        .user
        .group_by(Ident::<User>::new())
        .select((&pa).opt().and((&ph_by).opt()).and((&rp_by).opt()))
        .fold(0i64, |s, ((a, _), _)| s + a.map_or(0, |a| a[1] - a[2]));
    let phc = db.user.group_by(Ident::<User>::new()).select((&ph_by).select(post)).count_distinct();
    let rpc = db.user.group_by(Ident::<User>::new()).select(&rp_by).count_distinct();
    let mut v = drain((&rpc).filt(|n| n > 5).and((&prod).filt(|s| s > 10)).and(&ub).and((&pa).opt()).and((&phc).opt()));
    v.sort_by_key(|&(u, (_, h))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(h.unwrap_or(0))));
    rows(v.into_iter().map(|(u, ((((n, _), b), a), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(if a[3] == 0 { 0 } else { a[4] })],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([V::I(h.unwrap_or(0)), V::I(n)]);
        row(f)
    }))
}

// WITH RecursivePosts AS (SELECT p.Id, p.PostTypeId, p.Title, p.AcceptedAnswerId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.AcceptedAnswerId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserVoteSummary AS (SELECT v.UserId, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes, COUNT(DISTINCT v.PostId) AS UniquePostsVoted FROM Votes v GROUP BY v.UserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COALESCE(bs.TotalBadges, 0) AS TotalBadges,
//        CASE WHEN u.Reputation >= 1000 THEN 'Expert' WHEN u.Reputation >= 100 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel
//     FROM Users u LEFT JOIN (SELECT UserId, COUNT(*) AS TotalBadges FROM Badges GROUP BY UserId) bs ON u.Id = bs.UserId
//     WHERE u.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostHistoryDetails AS (SELECT ph.PostId, ph.UserId AS EditorUserId, ph.CreationDate, p.Title, ph.Comment, ph.PostHistoryTypeId,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS EditRank FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (4, 5, 6)),
// TopPostEditors AS (SELECT ph.EditorUserId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistoryDetails ph GROUP BY ph.EditorUserId)
// SELECT pu.UserId, pu.DisplayName, pu.Reputation, pu.UserLevel, pu.TotalBadges, us.TotalVotes, us.UniquePostsVoted, COALESCE(te.EditCount, 0) AS TotalEdits, te.LastEditDate,
//        COUNT(DISTINCT rp.Id) AS QuestionsAnswered, COUNT(DISTINCT ph.PostId) AS EditsCount
// FROM TopUsers pu LEFT JOIN UserVoteSummary us ON pu.UserId = us.UserId LEFT JOIN TopPostEditors te ON te.EditorUserId = pu.UserId
// LEFT JOIN RecursivePosts rp ON pu.UserId = rp.AcceptedAnswerId LEFT JOIN PostHistoryDetails ph ON pu.UserId = ph.EditorUserId
// WHERE pu.Reputation > 0 GROUP BY pu.UserId, pu.DisplayName, pu.Reputation, pu.UserLevel, pu.TotalBadges, us.TotalVotes, us.UniquePostsVoted, te.EditCount, te.LastEditDate
// ORDER BY pu.Reputation DESC, TotalVotes DESC LIMIT 50;
//
// Not recursive despite the name; rn and EditRank are never read. `pu.UserId = rp.AcceptedAnswerId` joins a user id to a post id, so it goes through the raw ids.
// The two COUNT(DISTINCT)s are each their own fold. The ORDER BY is not total; a tie at the cut goes to the smaller user id.
fn q33170(db: &'static So) -> String {
    let User { creation_date, reputation, origid, .. } = &db.user;
    let pu = || db.user.with(creation_date.lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(reputation.gt(0));
    let bs = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let Vote { user, post_id, vote_type_id, .. } = &db.vote;
    let us = db.vote.group_by(user).select(vote_type_id).fold(0i64, |n, t| n + matches!(t, 2 | 3) as i64);
    let upv = db.vote.group_by(user).select(post_id).count_distinct();
    let PostHistory { post_history_type_id: ht, creation_date: hd, user: hu, post, .. } = &db.post_history;
    let ed = || db.post_history.with(ht.is_in([4, 5, 6]));
    let te = ed().group_by(hu).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let ep = ed().group_by(hu).select(post).count_distinct();
    let acc: HashIdx<i64, Id<Post>> = db.post.with((&db.post.post_type_id).eq(1)).select(&db.post.accepted_answer_id).inv().collect();
    let qa = pu().group_by(Ident::<User>::new()).select(origid.select(&acc)).count_distinct();
    let v = drain(pu().select((&bs).opt().and((&us).opt()).and((&upv).opt()).and((&te).opt()).and((&qa).opt()).and((&ep).opt())));
    let v = top_n(v, |&(u, (((((_, t), _), _), _), _))| (Reverse(reputation.get(u).unwrap()), t.is_none(), Reverse(t), u), 50);
    rows(v.into_iter().map(|(u, (((((b, t), n), e), q), p))| {
        let r = reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::S(if r >= 1000 { "Expert" } else if r >= 100 { "Intermediate" } else { "Novice" }), V::I(b.unwrap_or(0)), oint(t), oint(n)]);
        f.extend([V::I(e.map_or(0, |e| e.0)), ots(e.map(|e| e.1)), V::I(q.unwrap_or(0)), V::I(p.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, U.Reputation, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostScoreSummary AS (SELECT P.Id AS PostId, COALESCE(V.TotalVotes, 0) AS VoteCount, COALESCE(C.CommentCount, 0) AS CommentCount, COALESCE(A.AnswerCount, 0) AS AnswerCount
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId) V ON P.Id = V.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
//     LEFT JOIN (SELECT ParentId AS PostId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) A ON P.Id = A.PostId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, U.DisplayName, U.Reputation, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, PS.VoteCount, PS.CommentCount, PS.AnswerCount,
//        CASE WHEN PS.AnswerCount = 0 THEN 'No Answers' WHEN PS.AnswerCount > 0 AND PS.VoteCount = 0 THEN 'Needs Attention' ELSE 'Active' END AS PostStatus,
//        COUNT(DISTINCT C.Id) AS TotalPostHistoryChanges
// FROM RankedPosts RP JOIN Users U ON RP.OwnerUserId = U.Id JOIN UserBadges UB ON U.Id = UB.UserId JOIN PostScoreSummary PS ON RP.PostId = PS.PostId
// LEFT JOIN PostHistory PH ON RP.PostId = PH.PostId LEFT JOIN Comments C ON RP.PostId = C.PostId
// WHERE (RP.PostRank = 1 AND PS.VoteCount > 0) OR (RP.PostRank = 1 AND PS.CommentCount > 5)
// GROUP BY RP.PostId, RP.Title, RP.CreationDate, U.DisplayName, U.Reputation, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, PS.VoteCount, PS.CommentCount, PS.AnswerCount
// ORDER BY PS.VoteCount DESC, RP.CreationDate DESC LIMIT 100;
//
// The PostHistory join cannot change COUNT(DISTINCT C.Id), which is the post's comments, so it is left out. A tie at the cut goes to the smaller post id.
fn q23988(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let v = drain((&vc).and(&cc).filt(|(v, c)| v > 0 || c > 5).and(&ac).and(owner_user.select(&ub)).and((&dc).opt()));
    let v = top_n(v, |&(p, ((((vc, _), _), _), _))| (Reverse(vc), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((((vc, cc), ac), b), d))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(vc), V::I(cc), V::I(ac), V::S(if ac == 0 { "No Answers" } else if vc == 0 { "Needs Attention" } else { "Active" }), V::I(d.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS OwnerPostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostScore AS (SELECT PostId, Score, ViewCount, CASE WHEN Score > 0 THEN 'Positive' WHEN Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory FROM RankedPosts),
// PostInteractions AS (SELECT RP.PostId, COALESCE(COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END), 0) AS UpVotes, COALESCE(COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END), 0) AS DownVotes,
//        COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount FROM RankedPosts RP LEFT JOIN Votes V ON RP.PostId = V.PostId LEFT JOIN Comments C ON RP.PostId = C.PostId
//     GROUP BY RP.PostId),
// FinalSummary AS (SELECT PS.PostId, PS.Score, PS.ViewCount, PS.ScoreCategory, PI.UpVotes, PI.DownVotes, PI.CommentCount, RP.OwnerDisplayName,
//        CASE WHEN PI.UpVotes IS NULL AND PI.DownVotes IS NULL THEN 'No Interactions' WHEN PI.UpVotes > PI.DownVotes THEN 'Positive Engagement'
//             WHEN PI.UpVotes < PI.DownVotes THEN 'Negative Engagement' ELSE 'Balanced Engagement' END AS InteractionSummary
//     FROM PostScore PS JOIN PostInteractions PI ON PS.PostId = PI.PostId JOIN RankedPosts RP ON PS.PostId = RP.PostId)
// SELECT FS.OwnerDisplayName, COUNT(*) AS PostCount, SUM(FS.UpVotes) AS TotalUpVotes, SUM(FS.DownVotes) AS TotalDownVotes, AVG(FS.Score) AS AverageScore,
//        COUNT(CASE WHEN FS.InteractionSummary = 'Positive Engagement' THEN 1 END) AS PositiveEngagementCount,
//        COUNT(CASE WHEN FS.InteractionSummary = 'Negative Engagement' THEN 1 END) AS NegativeEngagementCount,
//        COUNT(CASE WHEN FS.InteractionSummary = 'Balanced Engagement' THEN 1 END) AS BalancedEngagementCount,
//        COUNT(CASE WHEN FS.InteractionSummary = 'No Interactions' THEN 1 END) AS NoInteractionsCount
// FROM FinalSummary FS GROUP BY FS.OwnerDisplayName HAVING SUM(FS.UpVotes) > SUM(FS.DownVotes) + 10 OR AVG(FS.Score) < -5 ORDER BY PostCount DESC, TotalUpVotes DESC NULLS LAST;
//
// UpVotes and DownVotes are counts, never NULL, so no row is 'No Interactions'.
fn q24002(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let pi = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (t, _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let g = rp()
        .group_by(owner_user.select(&db.user.display_name))
        .select((&pi).and(score))
        .fold([0i64; 7], |a, (v, s)| [a[0] + 1, a[1] + v[0], a[2] + v[1], a[3] + s, a[4] + (v[0] > v[1]) as i64, a[5] + (v[0] < v[1]) as i64, a[6] + (v[0] == v[1]) as i64]);
    let mut v = drain((&g).filt(|a| a[1] > a[2] + 10 || (a[3] as f64 / a[0] as f64) < -5.0));
    v.sort_by_key(|&(_, a)| (Reverse(a[0]), Reverse(a[1])));
    rows(v.into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(a[4]), V::I(a[5]), V::I(a[6]), V::I(0)])))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, COUNT(DISTINCT p.Tags) AS UniqueTags, AVG(p.Score) AS AvgScore FROM Posts p GROUP BY p.OwnerUserId),
// ActiveUserStats AS (SELECT u.Id AS UserId, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(bs.BadgeCount, 0) AS BadgeCount, COALESCE(bs.GoldBadges, 0) AS GoldBadges,
//        COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges, COALESCE(ps.UniqueTags, 0) AS UniqueTags, COALESCE(ps.AvgScore, 0) AS AvgScore,
//        CASE WHEN COALESCE(bs.BadgeCount, 0) = 0 THEN 'No Badges' WHEN COALESCE(bs.GoldBadges, 0) > 0 THEN 'Gold Badge Holder' ELSE 'Regular User' END AS UserCategory,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(ps.AvgScore, 0) DESC, u.Reputation DESC) AS Rank
//     FROM Users u LEFT JOIN UserBadgeStats bs ON u.Id = bs.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
//     WHERE u.LastAccessDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR'),
// FilteredUsers AS (SELECT u.UserId, u.PostCount, u.BadgeCount, u.GoldBadges, u.SilverBadges, u.BronzeBadges, u.UniqueTags, u.AvgScore, u.UserCategory FROM ActiveUserStats u WHERE u.Rank <= 10)
// SELECT f.UserId, f.PostCount, f.BadgeCount, f.GoldBadges, f.SilverBadges, f.BronzeBadges, f.UniqueTags, f.AvgScore, f.UserCategory,
//        COALESCE(NULLIF(most_recent.CloseDate, '1970-01-01 00:00:00'), most_recent.ReopenDate) AS LastCloseOrReopenDate
// FROM FilteredUsers f LEFT JOIN (SELECT p.OwnerUserId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS CloseDate,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.OwnerUserId) most_recent
//   ON f.UserId = most_recent.OwnerUserId
// ORDER BY f.AvgScore DESC, f.PostCount DESC;
//
// A tie at Rank 10 goes to the smaller user id.
fn q24168(db: &'static So) -> String {
    let User { reputation, last_access_date, .. } = &db.user;
    let bs = db.user.with(reputation.gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, score, tags_str, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let ut = db.post.group_by(owner_user).select(tags_str).count_distinct();
    let au = drain(db.user.with(last_access_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select((&ps).opt().and((&bs).opt()).and((&ut).opt())));
    let key = |p: Option<(i64, i64)>| p.map_or(0.0, |(n, s)| s as f64 / n as f64);
    let v = top_n(au, |&(u, ((p, _), _))| (Reverse(fkey(key(p))), Reverse(reputation.get(u).unwrap()), u), 10);
    let PostHistory { post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let mr = db.post.group_by(owner_user).select(history_of(db).select(ht.and(hd))).fold((i64::MIN, i64::MIN), |(c, r), (t, d)| (if t == 10 { c.max(d) } else { c }, if t == 11 { r.max(d) } else { r }));
    let tv = rel(v);
    type R = (Id<User>, ((Option<(i64, i64)>, Option<[i64; 4]>), Option<i64>));
    let mut v = drain((&tv).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&mr).opt()))));
    v.sort_by_key(|&(_, ((_, ((p, _), _)), _))| (Reverse(fkey(key(p))), Reverse(p.map_or(0, |p| p.0))));
    rows(v.into_iter().map(|(_, ((u, ((p, b), t)), m))| {
        let b = b.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "uid"), V::I(p.map_or(0, |p| p.0))];
        f.extend(b.map(V::I));
        f.extend([V::I(t.unwrap_or(0)), V::F(key(p))]);
        f.push(V::S(if b[0] == 0 { "No Badges" } else if b[1] > 0 { "Gold Badge Holder" } else { "Regular User" }));
        f.push(match m {
            Some((c, _)) if c != i64::MIN && c != 0 => V::T(c),
            Some((_, r)) if r != i64::MIN => V::T(r),
            _ => V::Null,
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, p.Score, p.ViewCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank, COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS UserPostCount FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(b.Count, 0) AS BadgeCount FROM Users u LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM Badges GROUP BY UserId) b ON u.Id = b.UserId),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, ur.Reputation, ur.BadgeCount, CASE WHEN rp.UserPostRank = 1 THEN 'Top Post' ELSE 'Other Post' END AS PostCategory
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.UserPostCount > 5),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// FinalMetrics AS (SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.Reputation, ps.BadgeCount, rc.CommentCount, rc.LastCommentDate,
//        CASE WHEN rc.CommentCount IS NULL THEN 'No Comments' ELSE CASE WHEN rc.CommentCount > 5 THEN 'Highly Discussed' ELSE 'Moderately Discussed' END END AS DiscussionLevel
//     FROM PostStatistics ps LEFT JOIN RecentComments rc ON ps.PostId = rc.PostId)
// SELECT fm.PostId, fm.Title, fm.Score, fm.ViewCount, fm.Reputation, fm.BadgeCount, fm.CommentCount, fm.LastCommentDate, fm.DiscussionLevel,
//        CASE WHEN fm.Reputation > 1000 THEN 'High Reputation' WHEN fm.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory,
//        CASE WHEN fm.LastCommentDate IS NULL THEN 'Comments Not Available' ELSE CASE WHEN fm.LastCommentDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 'Recent Activity'
//             ELSE 'Older Activity' END END AS ActivityRecency
// FROM FinalMetrics fm ORDER BY fm.Score DESC, fm.ViewCount DESC LIMIT 100;
//
// PostCategory (from UserPostRank) is never read. A tie at the cut goes to the smaller post id.
fn q22870(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let upc = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let rc = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user.select((&upc).filt(|n| n > 5))).select(owner_user.select((&bc).opt()).and((&rc).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 100);
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    rows(v.into_iter().map(|(p, (b, c))| {
        let rep = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "rep"]);
        f.extend([V::I(b.unwrap_or(0)), oint(c.map(|c| c.0)), ots(c.map(|c| c.1))]);
        f.push(V::S(match c {
            None => "No Comments",
            Some((n, _)) if n > 5 => "Highly Discussed",
            _ => "Moderately Discussed",
        }));
        f.push(V::S(if rep > 1000 { "High Reputation" } else if (500..=1000).contains(&rep) { "Medium Reputation" } else { "Low Reputation" }));
        f.push(V::S(match c {
            None => "Comments Not Available",
            Some((_, d)) if d >= cut => "Recent Activity",
            _ => "Older Activity",
        }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, CASE WHEN Reputation IS NULL OR Reputation < 100 THEN 'Newbie' WHEN Reputation BETWEEN 100 AND 500 THEN 'Intermediate'
//        WHEN Reputation BETWEEN 501 AND 1000 THEN 'Experienced' ELSE 'Veteran' END AS ReputationTier FROM Users),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')),
// EnhancedPostMetrics AS (SELECT p.PostId, p.Title, p.CreationDate, COALESCE(c.Score, 0) AS CommentScore, COALESCE(v.UpVotes, 0) - COALESCE(v.DownVotes, 0) AS VoteNet, up.ReputationTier
//     FROM RecentPosts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v
//       ON p.PostId = v.PostId
//     LEFT JOIN (SELECT PostId, SUM(Score) AS Score FROM Comments GROUP BY PostId) c ON p.PostId = c.PostId LEFT JOIN UserReputation up ON p.OwnerUserId = up.Id),
// ClosedPosts AS (SELECT p.Id, MIN(h.CreationDate) AS FirstCloseDate FROM Posts p JOIN PostHistory h ON p.Id = h.PostId WHERE h.PostHistoryTypeId = 10 GROUP BY p.Id),
// PostInteractionMetrics AS (SELECT e.PostId, e.Title, e.CreationDate, e.CommentScore, e.VoteNet, cp.FirstCloseDate, CASE WHEN cp.FirstCloseDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus,
//        e.ReputationTier FROM EnhancedPostMetrics e LEFT JOIN ClosedPosts cp ON e.PostId = cp.Id)
// SELECT pim.PostId, pim.Title, pim.CreationDate, pim.CommentScore, pim.VoteNet, pim.PostStatus, COALESCE(CASE WHEN pim.VoteNet < 0 THEN 'Needs Attention' ELSE 'Good' END, 'Unknown') AS PostQuality,
//        pim.ReputationTier, CASE WHEN pim.CommentScore IS NOT NULL THEN 'Active Discussion' ELSE 'No Comments' END AS DiscussionState,
//        CONCAT('Post ID: ', pim.PostId, ' | Status: ', pim.PostStatus) AS PostSummary
// FROM PostInteractionMetrics pim WHERE pim.PostStatus = 'Active' AND (pim.VoteNet > 5 OR pim.CommentScore > 5) ORDER BY pim.CreationDate DESC LIMIT 50;
//
// RecentPostRank is never read; CommentScore is a COALESCE, so DiscussionState is always 'Active Discussion'.
fn q23976(db: &'static So) -> String {
    let Post { creation_date, owner_user, origid, .. } = &db.post;
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    let rp = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).minus(&closed);
    let vn = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let cs = rp().group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold(0i64, |n, s| n + s.unwrap_or(0));
    let v = drain((&vn).and(&cs).filt(|(v, c)| v > 5 || c > 5).and(owner_user.select(&db.user.reputation).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((n, c), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(n), V::S("Active"), V::S(if n < 0 { "Needs Attention" } else { "Good" })]);
        f.push(match r {
            Some(r) => V::S(if r < 100 { "Newbie" } else if r <= 500 { "Intermediate" } else if r <= 1000 { "Experienced" } else { "Veteran" }),
            None => V::Null,
        });
        f.extend([V::S("Active Discussion"), V::Owned(format!("Post ID: {} | Status: Active", origid.get(p).unwrap()))]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT p.Id) AS PostsCount,
//        AVG(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - u.CreationDate)) / 3600) AS AvgHoursOnline
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// QuestionStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS QuestionsAsked, COALESCE(SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END), 0) AS PositiveScoredQuestions,
//        COALESCE(MAX(p.CreationDate), DATE '1970-01-01') AS LastQuestionDate FROM Posts p WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId),
// UserEngagement AS (SELECT us.DisplayName, us.Reputation, us.UpVotes, us.DownVotes, qs.QuestionsAsked, qs.PositiveScoredQuestions, qs.LastQuestionDate,
//        RANK() OVER (ORDER BY us.Reputation DESC) AS ReputationRank, us.UserId FROM UserStatistics us LEFT JOIN QuestionStatistics qs ON us.UserId = qs.OwnerUserId)
// SELECT ue.DisplayName, ue.Reputation, ue.ReputationRank, nd.CreationDate AS NewestVoteDate, nd.NewestVoteType, COALESCE(ue.QuestionsAsked, 0) AS QuestionsAsked,
//        COALESCE(ue.PositiveScoredQuestions, 0) AS PositiveScoredQuestions,
//        CASE WHEN ue.QuestionsAsked > 0 AND ue.PositiveScoredQuestions = 0 THEN 'Needs Improvement' WHEN ue.PositiveScoredQuestions > 0 THEN 'Contributing' ELSE 'Passive User' END AS UserCategory,
//        pn.CommentsCount AS PostNoticeCommentsCount
// FROM UserEngagement ue
// LEFT JOIN (SELECT u.Id AS UserId, MAX(v.CreationDate) AS CreationDate,
//            CASE WHEN SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) > SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) THEN 'Positive' ELSE 'Negative' END AS NewestVoteType
//            FROM Users u JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id) nd ON ue.UserId = nd.UserId
// LEFT JOIN (SELECT post.OwnerUserId, COUNT(DISTINCT c.Id) AS CommentsCount FROM Posts post LEFT JOIN Comments c ON post.Id = c.PostId GROUP BY post.OwnerUserId) pn ON ue.UserId = pn.OwnerUserId
// WHERE ue.ReputationRank <= 10 ORDER BY ue.Reputation DESC, ue.ReputationRank;
//
// UserStatistics has one row per user and only DisplayName and Reputation are read, so its aggregates are not computed.
fn q23436(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let rk = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(rk.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let qs = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + (s > 0) as i64]);
    let nd = db.vote.group_by(&db.vote.user).select((&db.vote.vote_type_id).and(&db.vote.creation_date)).fold([0, 0, i64::MIN], |a, (t, d)| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2].max(d)]);
    let pn = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).count_distinct();
    let pnz = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    type R = (Id<User>, i64);
    let uk = || Same::<R>::new().map(|(u, _): R| u);
    let mut v = drain((&tu).select(Same::<R>::new().and(uk().select((&qs).opt())).and(uk().select((&nd).opt())).and(uk().select((&pnz).opt())).and(uk().select((&pn).opt()))));
    v.sort_by_key(|&(_, (((((u, r), _), _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), r));
    rows(v.into_iter().map(|(_, (((((u, r), q), n), z), c))| {
        let q = q.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(r));
        f.extend(match n {
            Some(n) => [V::T(n[2]), V::S(if n[0] > n[1] { "Positive" } else { "Negative" })],
            None => [V::Null, V::Null],
        });
        f.extend([V::I(q[0]), V::I(q[1]), V::S(if q[0] > 0 && q[1] == 0 { "Needs Improvement" } else if q[1] > 0 { "Contributing" } else { "Passive User" })]);
        f.push(if z.is_some() { V::I(c.unwrap_or(0)) } else { V::Null });
        row(f)
    }))
}

// WITH UserVotes AS (SELECT U.Id AS UserId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id),
// PostAnalytics AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswerId, P.CreationDate, EXTRACT(YEAR FROM P.CreationDate) AS CreationYear,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(COALESCE(GREATEST(V.BountyAmount, 0), 0)) AS TotalBountyAmount,
//        DENSE_RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS UserPostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.OwnerUserId, P.AcceptedAnswerId, P.CreationDate),
// PostHistoryDetails AS (SELECT PH.PostId, PH.PostHistoryTypeId, PH.CreationDate, PH.UserId,
//        CASE WHEN PH.PostHistoryTypeId = 10 THEN 'Closed' WHEN PH.PostHistoryTypeId = 11 THEN 'Reopened' ELSE 'Other Changes' END AS ChangeType
//     FROM PostHistory PH WHERE PH.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '1 year')),
// RelevantPosts AS (SELECT PA.PostId, PA.Title, PA.CreationDate, U.DisplayName AS OwnerDisplayName, COALESCE(UP.TotalVotes, 0) AS UserTotalVotes, COALESCE(UP.UpVotes, 0) AS UserUpVotes,
//        COALESCE(UP.DownVotes, 0) AS UserDownVotes, PHD.ChangeType, ROW_NUMBER() OVER (PARTITION BY PA.PostId ORDER BY PHD.CreationDate DESC) AS ChangeRank
//     FROM PostAnalytics PA JOIN Users U ON PA.OwnerUserId = U.Id LEFT JOIN UserVotes UP ON U.Id = UP.UserId LEFT JOIN PostHistoryDetails PHD ON PA.PostId = PHD.PostId
//     WHERE PA.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '2 years'))
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.OwnerDisplayName, RP.UserTotalVotes, RP.UserUpVotes, RP.UserDownVotes, RP.ChangeType,
//        CASE WHEN RP.ChangeType = 'Closed' THEN 'Post is closed.' WHEN RP.ChangeType = 'Reopened' THEN 'Post has been reopened.' ELSE 'No closure activity.' END AS ClosureStatus,
//        RANK() OVER (ORDER BY RP.UserTotalVotes DESC) AS PopularityRank
// FROM RelevantPosts RP WHERE RP.ChangeRank = 1 AND RP.UserTotalVotes IS NOT NULL ORDER BY PopularityRank, RP.CreationDate DESC;
//
// PostAnalytics is one row per post and only its id, title, owner and date are read, so its aggregates are not computed. A CreationDate tie for a post's
// latest history row goes to the larger history id.
fn q24657(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 0, 0, 0);
    let Post { creation_date, owner_user, .. } = &db.post;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let PostHistory { post, creation_date: hd, post_history_type_id: ht, .. } = &db.post_history;
    let lh = top_per(drain(db.post_history.with(hd.ge(add_years(t0, -1))).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let lh = rel(lh.into_iter().map(|(h, p)| (p, h)).collect());
    let lh_by: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lh).map(|(p, _)| p).inv().select(&lh).collect();
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -2))).select(owner_user.select(&uv).and((&lh_by).map(|(_, h)| h).select(ht).opt())));
    let v = ranked(v, |&(_, (a, _))| Reverse(a[0]), false);
    rows(v.into_iter().map(|((p, (a, t)), r)| {
        let ct = t.map(|t| if t == 10 { "Closed" } else if t == 11 { "Reopened" } else { "Other Changes" });
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), ct.map_or(V::Null, V::S)]);
        f.push(V::S(match ct {
            Some("Closed") => "Post is closed.",
            Some("Reopened") => "Post has been reopened.",
            _ => "No closure activity.",
        }));
        f.push(V::I(r));
        row(f)
    }))
}

// Rewritten (rewrites/24918.sql): the RankedPosts window is tie-broken on P.Id.
// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.Id) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' AND P.Score IS NOT NULL),
// TagStatistics AS (SELECT T.Id AS TagId, T.TagName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%'
//     GROUP BY T.Id, T.TagName),
// PostScores AS (SELECT PH.PostId, MAX(PH.CreationDate) AS LastHistoryChange, SUM(CASE WHEN PH.PostHistoryTypeId IN (10, 12) THEN 1 ELSE 0 END) AS CloseDeleteCount,
//        SUM(CASE WHEN PH.PostHistoryTypeId = 19 THEN 1 ELSE 0 END) AS ProtectedCount FROM PostHistory PH GROUP BY PH.PostId),
// UserSummaries AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(B.Class) AS TotalBadges, COALESCE(SUM(P.Score), 0) AS UserScore
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// CombinedStatistics AS (SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, RP.CreationDate, RP.Score, COALESCE(PS.LastHistoryChange, NULL) AS LastHistoryChange,
//        COALESCE(PS.CloseDeleteCount, 0) AS CloseDeleteCount, COALESCE(PS.ProtectedCount, 0) AS ProtectedCount, TS.TagId, TS.TagName, TS.PostCount, TS.TotalViews, US.UserId,
//        US.DisplayName AS UserDisplayName, US.BadgeCount, US.TotalBadges, US.UserScore
//     FROM RankedPosts RP LEFT JOIN PostScores PS ON RP.PostId = PS.PostId LEFT JOIN TagStatistics TS ON RP.PostId = TS.TagId LEFT JOIN UserSummaries US ON RP.OwnerDisplayName = US.DisplayName
//     WHERE RP.Rank <= 5)
// SELECT CBS.PostId, CBS.Title, CBS.OwnerDisplayName, CBS.CreationDate, CBS.Score, CBS.LastHistoryChange, CBS.CloseDeleteCount, CBS.ProtectedCount, CBS.TagName, CBS.PostCount, CBS.TotalViews,
//        CBS.UserDisplayName, CBS.BadgeCount, CBS.TotalBadges, CBS.UserScore
// FROM CombinedStatistics CBS WHERE CBS.Score > 10 AND CBS.CloseDeleteCount = 0 ORDER BY CBS.Score DESC, CBS.CreationDate ASC LIMIT 100;
//
// `RP.PostId = TS.TagId` joins a post id to a tag id, so it goes through the raw ids. UserSummaries is joined on the display name, so it is computed only for
// users sharing a kept post owner's name, over their badges x posts. A tie at the cut goes to the smaller post and user id.
fn q24918(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post_history_type_id: ht, creation_date: hd, .. } = &db.post_history;
    let ps = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(ht.and(hd))).fold([0, 0, i64::MIN], |a, (t, d)| [a[0] + matches!(t, 10 | 12) as i64, a[1] + (t == 19) as i64, a[2].max(d)]);
    let keep = (&tp).with(score.gt(10)).with((&ps).opt().filt(|p: Option<[i64; 3]>| p.map_or(true, |p| p[0] == 0)));
    let kept: MatSet<Id<Post>> = keep.collect();
    let tstats = tag_stats(db);
    let tidx: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let names: MatSet<Str> = (&kept).select(owner_user).select(&db.user.display_name).collect();
    let named: MatSet<Id<User>> = db.user.with((&db.user.display_name).select(&names)).collect();
    let us = (&named).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(score).opt())).fold([0i64; 3], |a, (c, s)| {
        [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0), a[2] + s.unwrap_or(0)]
    });
    let by_name: HashIdx<Str, Id<User>> = (&named).select(&db.user.display_name).inv().collect();
    let v = drain((&kept).select((&ps).opt().and(origid.select(&tidx).select(Ident::<Tag>::new().and(&tstats)).opt()).and(owner_user.select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&us)))));
    let v = top_n(v, |&(p, (_, (u, _)))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p, u), 100);
    rows(v.into_iter().map(|(p, ((h, t), (u, a)))| {
        let h = h.unwrap_or([0, 0, i64::MIN]);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([tmax(h[2]), V::I(h[0]), V::I(h[1])]);
        f.extend(match t {
            Some((t, s)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(s[0]), V::I(s[2])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([user_col(db, u, "name"), V::I(a[0]), nullable(a[1], a[0]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserScoreStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COUNT(DISTINCT P.Id) AS PostCount, AVG(COALESCE(P.Score, 0)) AS AverageScore,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS Ranking
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVoteCount, DownVoteCount, PostCount, AverageScore, Ranking FROM UserScoreStats WHERE Ranking <= 10),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName FROM Posts P JOIN Users U ON U.Id = P.OwnerUserId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate AS CloseDate, PH.Comment AS CloseReason, P.Title, U.DisplayName AS ClosedBy
//     FROM PostHistory PH JOIN Posts P ON P.Id = PH.PostId AND PH.PostHistoryTypeId = 10 LEFT JOIN Users U ON U.Id = PH.UserId),
// UserPostInteraction AS (SELECT U.DisplayName AS UserName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT COALESCE(V.Id, -1)) AS TotalVotes,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON V.PostId = P.Id WHERE U.Reputation > 100 GROUP BY U.DisplayName)
// SELECT TU.DisplayName AS TopUser, TU.UpVoteCount, TU.DownVoteCount, TU.PostCount, R.PostId AS RecentPostId, R.Title AS RecentPostTitle, R.CreationDate AS RecentPostDate,
//        R.OwnerDisplayName AS RecentPostOwner, CP.CloseDate, CP.CloseReason, CP.ClosedBy, UP.TotalPosts, UP.TotalVotes, UP.UpVotes, UP.DownVotes
// FROM TopUsers TU LEFT JOIN RecentPosts R ON R.OwnerDisplayName = TU.DisplayName LEFT JOIN ClosedPosts CP ON CP.Title = R.Title
// LEFT JOIN UserPostInteraction UP ON UP.UserName = TU.DisplayName ORDER BY TU.Ranking, R.CreationDate DESC NULLS LAST, CP.CloseDate DESC NULLS LAST;
//
// A net-vote tie at Ranking 10 goes to the smaller user id. UserPostInteraction is grouped by name, so it is computed only for the names of the top users.
fn q22872(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let tu = rel(top_n(drain(&us), |&(u, a)| (Reverse(a[0] - a[1]), u), 10));
    let names: MatSet<Str> = (&tu).map(|(u, _)| u).select(&db.user.display_name).collect();
    let rp: HashIdx<Str, Id<Post>> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.select(&db.user.display_name)).inv().collect();
    let PostHistory { post, post_history_type_id: ht, .. } = &db.post_history;
    let cp: HashIdx<Str, Id<PostHistory>> = db.post_history.with(ht.eq(10)).select(post.select(title)).inv().collect();
    let named = || db.user.with((&db.user.reputation).gt(100)).with((&db.user.display_name).select(&names));
    let upi = named().group_by(&db.user.display_name).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let upp = named().group_by(&db.user.display_name).select(posts_of(db)).count_distinct();
    let upv = named().group_by(&db.user.display_name).select(posts_of(db).select(votes_of(db).select(&db.vote.origid).opt()).opt().map(|x: Option<Option<i64>>| x.flatten().unwrap_or(-1))).count_distinct();
    let pd = user_distinct_posts(db);
    type T = (Id<User>, [i64; 2]);
    let j1: MatSet<(T, Option<Id<Post>>)> = (&tu).map(|t: T| t).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(&db.user.display_name).select(&rp).opt())).map(|x| x).collect();
    type J = (T, Option<Id<Post>>);
    let user = || Same::<J>::new().map(|((u, _), _): J| u);
    let closes = Same::<J>::new().map(|(_, p): J| p).flat_map(|p: Option<Id<Post>>| p).select(title).select(&cp).opt();
    let up = user().select(&db.user.display_name).select((&upi).and((&upp).opt()).and(&upv)).opt();
    let v = drain((&j1).select(Same::<J>::new().and(user().select(&pd)).and(closes).and(up)));
    rows(v.into_iter().map(|(_, (((((u, a), r), n), h), x))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n)];
        f.extend(match r {
            Some(p) => post_fields(db, p, &["id", "title", "created", "owner"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match h {
            Some(h) => [V::T(db.post_history.creation_date.get(h).unwrap()), ostr(db.post_history.comment.get(h)), db.post_history.user.get(h).map_or(V::Null, |u| user_col(db, u, "name"))],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match x {
            Some(((a, p), n)) => [V::I(p.unwrap_or(0)), V::I(n), V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation >= 1000 THEN 'High' WHEN u.Reputation BETWEEN 500 AND 999 THEN 'Medium' ELSE 'Low' END AS ReputationLevel FROM Users u),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, r.ReputationLevel, r.Reputation FROM Users u JOIN UserReputation r ON u.Id = r.UserId
//     WHERE u.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 week'),
// PostHistoryAggregates AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteCount FROM PostHistory ph GROUP BY ph.PostId),
// TopUsers AS (SELECT a.UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(ph.CloseCount, 0)) AS TotalClosedPosts, SUM(COALESCE(ph.ReopenCount, 0)) AS TotalReopenedPosts,
//        SUM(COALESCE(ph.DeleteCount, 0)) AS TotalDeletedPosts, SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes
//     FROM ActiveUsers a LEFT JOIN Posts p ON a.UserId = p.OwnerUserId LEFT JOIN PostHistoryAggregates ph ON p.Id = ph.PostId
//     LEFT JOIN (SELECT v.PostId, COUNT(v.Id) AS VoteCount FROM Votes v GROUP BY v.PostId) v ON p.Id = v.PostId GROUP BY a.UserId)
// SELECT u.UserId, u.DisplayName, u.ReputationLevel, u.Reputation, COALESCE(t.PostCount, 0) AS TotalPosts, COALESCE(t.TotalClosedPosts, 0) AS TotalClosed,
//        COALESCE(t.TotalReopenedPosts, 0) AS TotalReopened, COALESCE(t.TotalDeletedPosts, 0) AS TotalDeleted,
//        (COALESCE(t.TotalClosedPosts, 0) * 1.0 / NULLIF(COALESCE(t.PostCount, 0), 0)) * 100 AS CloseRate, (COALESCE(t.TotalReopenedPosts, 0) * 1.0 / NULLIF(COALESCE(t.PostCount, 0), 0)) * 100 AS ReopenRate,
//        (COALESCE(t.TotalDeletedPosts, 0) * 1.0 / NULLIF(COALESCE(t.PostCount, 0), 0)) * 100 AS DeleteRate
// FROM ActiveUsers u LEFT JOIN TopUsers t ON u.UserId = t.UserId WHERE u.Reputation >= 500 ORDER BY CloseRate DESC, ReopenRate DESC, DeleteRate DESC;
//
// RecentPosts is never read. PostHistoryAggregates has one row per post, so TopUsers sums over the user's posts.
fn q21328(db: &'static So) -> String {
    let au = || db.user.with((&db.user.last_access_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -7))).with((&db.user.reputation).ge(500));
    let pha = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| {
        [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + matches!(t, 12 | 13) as i64]
    });
    let t = au().group_by(Ident::<User>::new()).select(posts_of(db).select((&pha).opt()).opt()).fold([0i64; 4], |a, p| match p {
        Some(h) => {
            let h = h.unwrap_or([0; 3]);
            [a[0] + 1, a[1] + h[0], a[2] + h[1], a[3] + h[2]]
        }
        None => a,
    });
    let v = drain(au().select(&t));
    rows(v.into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let rate = |x: i64| if a[0] == 0 { V::Null } else { V::F(x as f64 * 1.0 / a[0] as f64 * 100.0) };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::S(if r >= 1000 { "High" } else if r >= 500 { "Medium" } else { "Low" }), V::I(r), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), rate(a[1]), rate(a[2]), rate(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '1 year') AND p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT p.Id) AS PostsCount, COUNT(DISTINCT c.Id) AS CommentsCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.Reputation, u.DisplayName),
// RecentVotes AS (SELECT v.PostId, COUNT(v.VoteTypeId) FILTER (WHERE v.VoteTypeId IN (2, 3)) AS VoteCount, MAX(v.CreationDate) AS LastVoteDate FROM Votes v
//     WHERE v.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '1 month') GROUP BY v.PostId),
// CombinedStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, us.DisplayName AS OwnerDisplayName, us.Reputation AS OwnerReputation, us.GoldBadges,
//        us.SilverBadges, us.BronzeBadges, COALESCE(rv.VoteCount, 0) AS RecentVoteCount, rv.LastVoteDate
//     FROM RankedPosts rp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) JOIN UserStatistics us ON u.Id = us.UserId LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId)
// SELECT cs.PostId, cs.Title, cs.CreationDate, cs.Score, cs.ViewCount, cs.CommentCount, cs.OwnerDisplayName, cs.OwnerReputation, cs.GoldBadges, cs.SilverBadges, cs.BronzeBadges,
//        cs.RecentVoteCount, cs.LastVoteDate, CASE WHEN cs.OwnerReputation = (SELECT MAX(Reputation) FROM Users) THEN 'Top Reputation' ELSE 'Normal User' END AS UserCategory,
//        CASE WHEN cs.RecentVoteCount > 10 THEN 'Highly Active' ELSE 'Less Active' END AS ActivityLevel
// FROM CombinedStats cs WHERE cs.ViewCount > 100 AND cs.CommentCount > 5 ORDER BY cs.Score DESC, cs.ViewCount DESC FETCH FIRST 50 ROWS ONLY;
//
// UserRank is never read; the correlated subquery looks a post up by id, so it is the owner. The badge sums of UserStatistics run over the owner's
// badges x posts x comments, driven only for the owners of the kept posts. A tie at the cut goes to the smaller post id.
fn q24848(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 0, 0, 0);
    let Post { creation_date, post_type_id, owner_user, view_count, score, .. } = &db.post;
    let rp = db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.is_in([1, 2]))).with(view_count.gt(100)).with(owner_user);
    let cc = rp.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let kept: MatSet<Id<Post>> = db.post.with((&cc).filt(|n| n > 5)).collect();
    let owners: MatSet<Id<User>> = (&kept).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(comments_of(db).opt()).opt()))
        .fold([0i64; 3], |a, (c, _)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let rv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(t0, -1)))).select((&db.vote.vote_type_id).and(&db.vote.creation_date));
    let rvs = (&kept).group_by(Ident::<Post>::new()).select(rv).fold((0i64, i64::MIN), |(n, m), (t, d)| (n + matches!(t, 2 | 3) as i64, m.max(d)));
    let maxrep = db.user.select(&db.user.reputation).fold_flat(i64::MIN, |m, r| m.max(r));
    let v = drain((&kept).select((&cc).and(owner_user.select(&us)).and((&rvs).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((c, b), r))| {
        let rep = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        let n = r.map_or(0, |r| r.0);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend(b.map(V::I));
        f.extend([V::I(n), r.map_or(V::Null, |r| V::T(r.1)), V::S(if rep == maxrep { "Top Reputation" } else { "Normal User" }), V::S(if n > 10 { "Highly Active" } else { "Less Active" })]);
        row(f)
    }))
}

// WITH UserDetails AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPosts FROM Posts p GROUP BY p.OwnerUserId),
// ActiveUserStats AS (SELECT ud.UserId, ud.DisplayName, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.QuestionCount, 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount,
//        COALESCE(ps.ClosedPosts, 0) AS ClosedPosts, ROW_NUMBER() OVER (ORDER BY ud.Reputation DESC) AS UserRank
//     FROM UserDetails ud LEFT JOIN PostStats ps ON ud.UserId = ps.OwnerUserId WHERE ud.LastAccessDate > (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.ViewCount, p.OwnerUserId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Tags, p.CreationDate, p.ViewCount, p.OwnerUserId),
// FinalStats AS (SELECT au.UserId, au.DisplayName, au.TotalPosts, au.QuestionCount, au.AnswerCount, au.ClosedPosts, pd.PostId, pd.Title, pd.Tags, pd.CreationDate, pd.ViewCount, pd.CommentCount,
//        MAX(pd.ViewCount) OVER (PARTITION BY au.UserId) AS MaxViewCount FROM ActiveUserStats au JOIN PostDetails pd ON au.UserId = pd.OwnerUserId)
// SELECT fs.UserId, fs.DisplayName, fs.TotalPosts, fs.QuestionCount, fs.AnswerCount, fs.ClosedPosts, fs.PostId, fs.Title, fs.Tags, fs.CreationDate, fs.ViewCount, fs.CommentCount, fs.MaxViewCount,
//        CASE WHEN fs.ViewCount > 100 THEN 'High Engagement' WHEN fs.ViewCount BETWEEN 50 AND 100 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM FinalStats fs WHERE fs.MaxViewCount IS NOT NULL AND fs.QuestionCount > 0 ORDER BY fs.QuestionCount DESC, fs.TotalPosts DESC;
//
// UserDetails' badge aggregates and UserRank are never read.
fn q23167(db: &'static So) -> String {
    let Post { owner_user, post_type_id, closed_date, creation_date, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(closed_date.opt())).fold([0i64; 4], |a, (t, c)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64]);
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let mv = recent().group_by(owner_user).select(view_count).fold(i64::MIN, |m, w| m.max(w));
    let au = Ident::<User>::new().with((&db.user.last_access_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).and((&ps).filt(|a| a[1] > 0)).and(&mv);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain(recent().select(owner_user.select(au).and(&cc)));
    rows(v.into_iter().map(|(p, (((u, a), m), c))| {
        let w = view_count.get(p);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "tags", "created", "views"]));
        f.extend([V::I(c), V::I(m)]);
        f.push(V::S(match w {
            Some(w) if w > 100 => "High Engagement",
            Some(w) if w >= 50 => "Moderate Engagement",
            _ => "Low Engagement",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, DENSE_RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS ScoreRank,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS RecentRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostMetrics AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, Score, ViewCount, ScoreRank, RecentRank, COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = R.PostId), 0) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = R.PostId AND V.VoteTypeId = 2), 0) AS UpVotes FROM RankedPosts R),
// HighScorePosts AS (SELECT PostId, Title, OwnerDisplayName, Score, ViewCount, CommentCount, UpVotes,
//        CASE WHEN ScoreRank = 1 THEN 'Top Scoring' WHEN ScoreRank <= 5 THEN 'High Scoring' ELSE 'Moderate Scoring' END AS ScoreCategory FROM PostMetrics WHERE Score > 10),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, PH.UserDisplayName, PH.Comment, PH.Text AS ClosureReason, P.Title AS ClosedPostTitle FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id
//     WHERE PH.PostHistoryTypeId = 10),
// ScoreAnalysis AS (SELECT H.PostId, H.Title, H.OwnerDisplayName, H.Score, H.CommentCount, H.UpVotes, C.CreationDate AS ClosedDate, C.UserDisplayName AS Closer, C.ClosureReason
//     FROM HighScorePosts H LEFT JOIN ClosedPosts C ON H.PostId = C.PostId)
// SELECT PostId, Title, OwnerDisplayName, Score, CommentCount, UpVotes, CASE WHEN ClosedDate IS NOT NULL THEN 'Closed Post' ELSE 'Open Post' END AS PostStatus,
//        COALESCE(CAST(ClosedDate AS DATE), CAST('2024-10-01 12:34:56' AS DATE)) AS ClosureDateOrCurrentDate, COALESCE(Closer, 'N/A') AS CloserName, COALESCE(ClosureReason, 'Not Applicable') AS ClosureReason
// FROM ScoreAnalysis WHERE (Score > 20 OR CommentCount > 10)
//   AND (CASE WHEN ClosedDate IS NOT NULL THEN 'Closed Post' ELSE 'Open Post' END = 'Open Post' OR (CASE WHEN ClosedDate IS NOT NULL THEN 'Closed Post' ELSE 'Open Post' END = 'Closed Post' AND UpVotes > 5))
// ORDER BY Score DESC, CommentCount DESC LIMIT 50;
//
// ScoreCategory and RecentRank are never read. A tie at the cut goes to the smaller post and history id.
fn q20656(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let hs = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).with(score.gt(10));
    let cc = hs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uv = hs().group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let PostHistory { post_history_type_id: ht, creation_date: hd, user_display_name, text, .. } = &db.post_history;
    let cl = history_of(db).select(Ident::<PostHistory>::new().with(ht.eq(10))).opt();
    let v = drain((&cc).and(&uv).and(cl).and(score).filt(|(((c, u), h), s): (((i64, i64), Option<Id<PostHistory>>), i64)| (s > 20 || c > 10) && (h.is_none() || u > 5)).map(|(x, _)| x));
    let v = top_n(v, |&(p, ((c, _), h))| (Reverse(score.get(p).unwrap()), Reverse(c), p, h), 50);
    rows(v.into_iter().map(|(p, ((c, u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score"]);
        f.extend([V::I(c), V::I(u)]);
        f.extend(match h {
            Some(h) => [V::S("Closed Post"), V::D(trunc_day(hd.get(h).unwrap())), V::S(user_display_name.get(h).unwrap_or("N/A")), V::S(text.get(h).unwrap_or("Not Applicable"))],
            None => [V::S("Open Post"), V::D(ts(2024, 10, 1, 0, 0, 0)), V::S("N/A"), V::S("Not Applicable")],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, p.OwnerUserId, MAX(c.CreationDate) AS LastCommentDate, COUNT(c.Id) AS TotalComments,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.OwnerUserId),
// AggregatedPostStats AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.LastActivityDate, pd.OwnerUserId, pd.TotalComments, pd.UpVotes, pd.DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY pd.OwnerUserId ORDER BY pd.LastActivityDate DESC) AS rn,
//        CASE WHEN pd.UpVotes > pd.DownVotes THEN 'Positive' WHEN pd.UpVotes < pd.DownVotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment FROM PostDetails pd),
// RankedUserStats AS (SELECT ub.UserId, ub.DisplayName, ub.TotalBadges, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, COUNT(DISTINCT ap.PostId) AS PostsContributed,
//        SUM(CASE WHEN ap.Sentiment = 'Positive' THEN 1 ELSE 0 END) AS PositiveSentimentPosts, SUM(CASE WHEN ap.Sentiment = 'Negative' THEN 1 ELSE 0 END) AS NegativeSentimentPosts
//     FROM UserBadges ub LEFT JOIN AggregatedPostStats ap ON ub.UserId = ap.OwnerUserId GROUP BY ub.UserId, ub.DisplayName, ub.TotalBadges, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges)
// SELECT u.UserId, u.DisplayName, u.TotalBadges, u.GoldBadges, u.SilverBadges, u.BronzeBadges, COALESCE(ps.PostsContributed, 0) AS PostsContributed,
//        COALESCE(ps.PositiveSentimentPosts, 0) AS PositiveSentiment, COALESCE(ps.NegativeSentimentPosts, 0) AS NegativeSentiment,
//        CASE WHEN COALESCE(ps.PostsContributed, 0) = 0 THEN 'No Contributions' WHEN u.TotalBadges > 3 THEN 'Highly Active' ELSE 'Moderately Active' END AS ActivityLevel
// FROM RankedUserStats u LEFT JOIN RankedUserStats ps ON u.UserId = ps.UserId WHERE u.TotalBadges > 0 ORDER BY u.TotalBadges DESC, u.DisplayName ASC;
//
// The self-join is on the user id of a one-row-per-user CTE, so ps is u. rn is never read.
fn q22411(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let recent = || db.post.with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pd = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let rs = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&pd)).fold([0i64; 3], |a, s| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64]);
    let v = drain((&ub).and((&rs).opt()));
    rows(v.into_iter().map(|(u, (b, r))| {
        let r = r.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(r.map(V::I));
        f.push(V::S(if r[0] == 0 { "No Contributions" } else if b[0] > 3 { "Highly Active" } else { "Moderately Active" }));
        row(f)
    }))
}

// Rewritten (rewrites/24609.sql): VoteId and the ids of psa and ps are carried through, and both the LEAD window and the final ORDER BY are tie-broken on
// p.PostId, p.VoteId, psa.UserId, ps.Id.
// WITH UserScoreStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(v.Id) AS TotalVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Author, p.CreationDate, v.Id AS VoteId, COALESCE(c.CommentCount, 0) AS Comments, COALESCE(a.AnswerCount, 0) AS Answers,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS TotalDownvotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.ViewCount IS NOT NULL),
// FinalMetrics AS (SELECT psa.UserId, psa.DisplayName, p.PostId, p.VoteId, ps.Id AS PsId, p.Title, p.Author, p.CreationDate, p.Comments, p.Answers, p.TotalUpvotes, p.TotalDownvotes,
//        COALESCE(ps.Reputation, 0) AS ReputationScore, CASE WHEN ps.Reputation > 1000 THEN 'Gold' WHEN ps.Reputation BETWEEN 500 AND 1000 THEN 'Silver' ELSE 'Bronze' END AS BadgeType,
//        LEAD(ps.Reputation) OVER (ORDER BY p.CreationDate DESC, p.PostId, p.VoteId, psa.UserId, ps.Id) AS NextUserReputation
//     FROM PostActivity p LEFT JOIN UserScoreStats psa ON psa.DisplayName = p.Author LEFT JOIN Users ps ON p.Author = ps.DisplayName
//     WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 YEAR')
// SELECT fm.UserId, fm.DisplayName, fm.Title, fm.Author, fm.CreationDate, fm.Comments, fm.Answers, fm.TotalUpvotes, fm.TotalDownvotes, fm.ReputationScore, fm.BadgeType,
//        CASE WHEN fm.ReputationScore IS NULL THEN 'No reputation' WHEN fm.ReputationScore < 100 THEN 'Newbie' ELSE NULL END AS ReputationMessage,
//        CASE WHEN fm.NextUserReputation IS NOT NULL THEN CASE WHEN fm.NextUserReputation > fm.ReputationScore THEN 'Next user has higher reputation'
//             ELSE 'Next user has lower or equal reputation' END ELSE 'No subsequent user record' END AS UserComparison
// FROM FinalMetrics fm WHERE fm.TotalUpvotes > fm.TotalDownvotes OR (fm.TotalUpvotes IS NULL AND fm.TotalDownvotes IS NULL)
// ORDER BY fm.ReputationScore DESC, fm.CreationDate DESC, fm.PostId, fm.VoteId, fm.UserId, fm.PsId FETCH FIRST 10 ROWS ONLY;
//
// UserScoreStats is joined on the display name and only its UserId and DisplayName are read, so it is the users of that name. The LEAD is a prela window
// over the one partition of all FinalMetrics rows.
fn q24609(db: &'static So) -> String {
    let Post { view_count, creation_date, owner_user, origid, .. } = &db.post;
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let named = || owner_user.select(&db.user.display_name).select(&by_name).opt();
    let fm = || db.post.with(view_count).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let jr: MatSet<(Id<Post>, Option<Id<Vote>>, Option<Id<User>>, Option<Id<User>>)> =
        fm().select(Ident::<Post>::new().and(votes_of(db).opt()).and(named()).and(named())).map(|(((p, v), a), b)| (p, v, a, b)).collect();
    type Rw = (Id<Post>, Option<Id<Vote>>, Option<Id<User>>, Option<Id<User>>);
    let rep = |u: Option<Id<User>>| u.map(|u| db.user.reputation.get(u).unwrap());
    let oid = |u: Option<Id<User>>| u.map(|u| db.user.origid.get(u).unwrap());
    let key = move |(p, v, a, b): Rw| {
        let vo = v.map(|v| db.vote.origid.get(v).unwrap());
        (Reverse(creation_date.get(p).unwrap()), origid.get(p).unwrap(), (vo.is_none(), vo), (a.is_none(), oid(a)), (b.is_none(), oid(b)))
    };
    let lead = (&jr)
        .group_by(Same::<Rw>::new().map(|_: Rw| ()))
        .select(Same::<Rw>::new())
        .window(move |g: &[(_, Rw)], out: &mut Vec<Option<i64>>| {
            for i in 0..g.len() {
                out.push(g.get(i + 1).and_then(|x| rep(x.1 .3)));
            }
        }, key, |a, b| a.cmp(b));
    let pv = fm().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ac = db.post.with((&db.post.post_type_id).eq(2)).group_by(&db.post.parent).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    type L = (Rw, Option<i64>);
    let lv = rel(drain(&lead).into_iter().map(|x| x.1).collect::<Vec<L>>());
    let pk = || Same::<L>::new().map(|((p, _, _, _), _): L| p);
    let v = drain((&lv).select(Same::<L>::new().and(pk().select((&pv).filt(|a| a[0] > a[1]))).and(pk().select((&cc).opt())).and(pk().select((&ac).opt()))));
    let v = top_n(v, |&(_, ((((r, _), _), _), _))| (Reverse(rep(r.3).unwrap_or(0)), key(r)), 10);
    rows(v.into_iter().map(|(_, ((((r, l), u), c), a))| {
        let (p, _, psa, ps) = r;
        let sc = rep(ps).unwrap_or(0);
        let mut f = match psa {
            Some(x) => ucols(db, x, &["uid", "name"]),
            None => vec![V::Null, V::Null],
        };
        f.extend(post_fields(db, p, &["title", "owner", "created"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0)), V::I(u[0]), V::I(u[1]), V::I(sc)]);
        f.push(V::S(match rep(ps) {
            Some(r) if r > 1000 => "Gold",
            Some(r) if r >= 500 => "Silver",
            _ => "Bronze",
        }));
        f.push(if sc < 100 { V::S("Newbie") } else { V::Null });
        f.push(V::S(match l {
            Some(n) if n > sc => "Next user has higher reputation",
            Some(_) => "Next user has lower or equal reputation",
            None => "No subsequent user record",
        }));
        row(f)
    }))
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN LATERAL (SELECT TRIM(BOTH ' ' FROM UNNEST(string_to_array(p.Tags, ','))) AS tag) AS tag ON tag IS NOT NULL LEFT JOIN Tags t ON t.TagName = tag
// WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY p.CreationDate DESC LIMIT 100;
//
// The LATERAL splits Tags on ',' (a flat_map over the string) and each piece is joined to Tags by name; the counts run over comments x votes x pieces.
// STRING_AGG(DISTINCT) is sorted. A CreationDate tie at the cut goes to the smaller post id.
fn q12954(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, .. } = &db.post;
    let rp = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(creation_date));
    let rp = top_n(rp, |&(p, d)| (Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_name: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pieces = || tags_str.flat_map(|t: Str| t.split(',').map(|x| x.trim_matches(' '))).select((&by_name).opt()).opt();
    let cv = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt()).and(pieces())).fold([0i64; 2], |a, ((c, v), _)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let tg = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt()).and(pieces()).map(|(_, t)| t.flatten())).buf_fold(|v| {
        let mut n: Vec<Str> = v.into_iter().flatten().map(|t| db.tag.tag_name.get(t).unwrap()).collect();
        n.sort_unstable();
        n.dedup();
        if n.is_empty() { None } else { Some(&*Box::leak(n.join(", ").into_boxed_str())) }
    });
    let v = drain((&cv).and(&tg));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), ostr(t)]);
        row(f)
    }))
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ARRAY_AGG(DISTINCT t.TagName) AS Tags, COALESCE(b.BadgeCount, 0) AS UserBadges, u.Reputation, u.DisplayName
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId
// LEFT JOIN (SELECT unnest(string_to_array(p.Tags, ',')) AS TagName, p.Id FROM Posts p) t ON p.Id = t.Id
// WHERE p.CreationDate >= '2022-01-01' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.Id, u.Reputation, u.DisplayName, b.BadgeCount ORDER BY p.CreationDate DESC LIMIT 100;
//
// The unnest splits Tags on ',' (a flat_map over the string). ARRAY_AGG(DISTINCT) is sorted. A CreationDate tie at the cut goes to the smaller post id.
fn q5054(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, post_type_id, .. } = &db.post;
    let rp = drain(db.post.with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0)).and(post_type_id.eq(1))).with(owner_user).select(creation_date));
    let rp = top_n(rp, |&(p, d)| (Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pieces = || tags_str.flat_map(|t: Str| t.split(',')).opt();
    let uv = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(pieces())).fold([0i64; 2], |a, ((_, t), _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let tg = (&tp).group_by(Ident::<Post>::new()).select(pieces()).buf_fold(|v| {
        let mut n: Vec<Option<Str>> = v.into_iter().collect();
        n.sort_unstable();
        n.dedup();
        &*Box::leak(n.into_boxed_slice())
    });
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&uv).and((&dc).opt()).and(&tg).and(owner_user.select((&bc).opt())));
    rows(v.into_iter().map(|(p, (((a, c), t), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::L(t.iter().map(|&x| ostr(x)).collect()), V::I(b.unwrap_or(0))]);
        f.extend(post_fields(db, p, &["rep", "owner"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 YEAR'
//     GROUP BY p.Title, p.CreationDate, p.Score, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT Title, CreationDate, OwnerDisplayName, Score FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.Score, STRING_AGG(t.TagName, ', ') AS Tags
// FROM TopPosts tp LEFT JOIN Posts p ON tp.Title = p.Title LEFT JOIN Tags t ON p.Id = t.ExcerptPostId GROUP BY tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.Score ORDER BY tp.Score DESC;
//
// RankedPosts groups by the column tuple, not the post, so the tuples are grouped first and ranked. CommentCount is never read. The STRING_AGG has no
// ORDER BY; the names are joined in tag id order.
fn q8483(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, score, post_type_id, .. } = &db.post;
    type K = (Option<Str>, i64, i64, Str, i64);
    let rp = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .group_by(title.opt().and(creation_date).and(score).and(owner_user.select(&db.user.display_name)).and(post_type_id).map(|((((t, d), s), n), k)| (t, d, s, n, k)))
        .select(Ident::<Post>::new())
        .fold((), |_, _| ());
    let rk = per_group(ranked(drain(&rp), |&((_, d, s, _, k), _): &(K, ())| (k, Reverse(s), Reverse(d)), false), |&((_, _, _, _, k), _)| k);
    let tp: MatSet<(Option<Str>, i64, Str, i64)> = rel(rk)
        .filt(|(_, r)| r <= 10)
        .map(|(((t, d, s, n, _), _), _)| (t, d, n, s))
        .collect();
    let by_title: HashIdx<Str, Id<Post>> = title.inv().collect();
    let by_excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    type T = (Option<Str>, i64, Str, i64);
    let tags = Same::<T>::new().flat_map(|(t, _, _, _): T| t).select(&by_title).select((&by_excerpt).opt()).opt();
    let g = (&tp).group_by(Same::<T>::new()).select(tags).buf_fold(|v| {
        let mut ids: Vec<Id<Tag>> = v.into_iter().flatten().flatten().collect();
        ids.sort_unstable();
        if ids.is_empty() { None } else { Some(&*Box::leak(ids.iter().map(|&t| db.tag.tag_name.get(t).unwrap()).collect::<Vec<_>>().join(", ").into_boxed_str())) }
    });
    rows(drain(&g).into_iter().map(|((t, d, n, s), a)| row(vec![ostr(t), V::T(d), V::S(n), V::I(s), ostr(a)])))
}

// WITH PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagStats AS (SELECT pt.Tag, COUNT(DISTINCT pt.PostId) AS QuestionCount, COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers, SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, AVG(p.Score) AS AverageScore FROM PostTags pt JOIN Posts p ON pt.PostId = p.Id GROUP BY pt.Tag),
// RankedTags AS (SELECT ts.Tag, ts.QuestionCount, ts.UniqueUsers, ts.TotalAnswers, ts.TotalViews, ts.AverageScore, ROW_NUMBER() OVER (ORDER BY ts.QuestionCount DESC, ts.TotalViews DESC) AS Rank
//     FROM TagStats ts)
// SELECT rt.Tag, rt.QuestionCount, rt.UniqueUsers, rt.TotalAnswers, rt.TotalViews, rt.AverageScore FROM RankedTags rt WHERE rt.Rank <= 10;
//
// A (QuestionCount, TotalViews) tie at Rank 10 goes to the smaller tag string.
fn q28100(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, answer_count, view_count, score, .. } = &db.post;
    let pt = rel(drain(db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list)))).into_iter().map(|x| x.1).collect::<Vec<_>>());
    type P = (Id<Post>, Str);
    let post = || Same::<P>::new().map(|(p, _): P| p);
    let tag = || Same::<P>::new().map(|(_, t): P| t);
    let ts = (&pt).group_by(tag()).select(post().select(answer_count.opt().and(view_count.opt()).and(score))).fold([0i64; 4], |a, ((n, w), s)| [a[0] + n.unwrap_or(0), a[1] + w.unwrap_or(0), a[2] + s, a[3] + 1]);
    let qc = (&pt).group_by(tag()).select(post()).count_distinct();
    let uu = (&pt).group_by(tag()).select(post().select(owner_user)).count_distinct();
    let v = top_n(drain((&ts).and(&qc).and((&uu).opt())), |&(t, ((a, q), _))| (Reverse(q), Reverse(a[1]), t), 10);
    rows(v.into_iter().map(|(t, ((a, q), u))| row(vec![V::S(t), V::I(q), V::I(u.unwrap_or(0)), V::I(a[0]), V::I(a[1]), avg(a[2], a[3])])))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, U.DisplayName AS Author, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.Tags,
//        ROW_NUMBER() OVER (PARTITION BY P.Tags ORDER BY P.CreationDate DESC) AS RankByTag
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.CreationDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND P.ViewCount > 100),
// TagDetails AS (SELECT T.TagName, COUNT(DISTINCT RP.PostId) AS QuestionCount, SUM(RP.AnswerCount) AS TotalAnswers, AVG(RP.Score) AS AverageScore, SUM(RP.ViewCount) AS TotalViews
//     FROM RankedPosts RP JOIN UNNEST(string_to_array(RP.Tags, '><')) AS T(TagName) ON true GROUP BY T.TagName)
// SELECT TD.TagName, TD.QuestionCount, TD.TotalAnswers, TD.AverageScore, TD.TotalViews FROM TagDetails TD WHERE TD.QuestionCount >= 5 ORDER BY TD.TotalAnswers DESC, TD.AverageScore DESC;
//
// RankByTag is never read. The Tags string is split on '><' as it stands, so the first and last pieces keep their '<' and '>' (a flat_map over the string).
fn q25467(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, owner_user, tags_str, answer_count, score, .. } = &db.post;
    let rp = db.post.with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(view_count.gt(100)).with(owner_user);
    let pt = rel(drain(rp.select(Ident::<Post>::new().and(tags_str.flat_map(|t: Str| t.split("><"))))).into_iter().map(|x| x.1).collect::<Vec<_>>());
    type P = (Id<Post>, Str);
    let post = || Same::<P>::new().map(|(p, _): P| p);
    let tag = || Same::<P>::new().map(|(_, t): P| t);
    let td = (&pt).group_by(tag()).select(post().select(answer_count.opt().and(score).and(view_count))).fold([0i64; 5], |a, ((n, s), w)| {
        [a[0] + n.is_some() as i64, a[1] + n.unwrap_or(0), a[2] + s, a[3] + 1, a[4] + w]
    });
    let qc = (&pt).group_by(tag()).select(post()).count_distinct();
    let v = drain((&td).and((&qc).filt(|q| q >= 5)));
    rows(v.into_iter().map(|(t, (a, q))| row(vec![V::S(t), V::I(q), nullable(a[1], a[0]), avg(a[2], a[3]), V::I(a[4])])))
}

// WITH FilteredPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// RankedPosts AS (SELECT fp.PostId, fp.Title, fp.Body, fp.Tag, fp.CommentCount, ROW_NUMBER() OVER (PARTITION BY fp.Tag ORDER BY fp.CommentCount DESC) AS TagRank FROM FilteredPosts fp),
// PopularTags AS (SELECT Tag, COUNT(*) AS TotalPosts FROM RankedPosts GROUP BY Tag HAVING COUNT(*) > 5)
// SELECT pt.Tag, COUNT(rp.PostId) AS NumberOfPosts, MAX(rp.CommentCount) AS MostComments, MIN(rp.CommentCount) AS LeastComments, AVG(rp.CommentCount) AS AvgComments
// FROM RankedPosts rp JOIN PopularTags pt ON rp.Tag = pt.Tag GROUP BY pt.Tag ORDER BY NumberOfPosts DESC;
//
// TagRank is never read; an answer's NULL Tags unnests to no rows.
fn q28667(db: &'static So) -> String {
    let Post { creation_date, post_type_id, tags_str, .. } = &db.post;
    let fp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])));
    let cc = fp.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rows_ = rel(drain(db.post.with(&cc).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list)))).into_iter().map(|x| x.1).collect::<Vec<_>>());
    type P = (Id<Post>, Str);
    let g = (&rows_)
        .group_by(Same::<P>::new().map(|(_, t): P| t))
        .select(Same::<P>::new().map(|(p, _): P| p).select(&cc))
        .fold([0, i64::MIN, i64::MAX, 0], |a, c| [a[0] + 1, a[1].max(c), a[2].min(c), a[3] + c]);
    let v = drain((&g).filt(|a| a[0] > 5));
    rows(v.into_iter().map(|(t, a)| row(vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])])))
}

// WITH TagSummary AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore, ARRAY_AGG(DISTINCT U.DisplayName) AS Contributors,
//        AVG(P.CreationDate::date - U.CreationDate::date) AS AvgPostAge
//     FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' JOIN Users U ON P.OwnerUserId = U.Id GROUP BY T.TagName),
// TopPosts AS (SELECT P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate, P.Tags, ROW_NUMBER() OVER (PARTITION BY T.TagName ORDER BY P.ViewCount DESC) AS RN
//     FROM Posts P JOIN Tags T ON P.Tags LIKE '%' || T.TagName || '%' WHERE P.PostTypeId = 1)
// SELECT TS.TagName, TS.PostCount, TS.TotalViews, TS.TotalScore, TS.Contributors, TS.AvgPostAge, TP.Title AS TopPostTitle, TP.Score AS TopPostScore, TP.ViewCount AS TopPostViews,
//        TP.CreationDate AS TopPostDate
// FROM TagSummary TS LEFT JOIN TopPosts TP ON TS.TagName = SPLIT_PART(TP.Tags, ',', 1) WHERE TP.RN = 1 ORDER BY TS.TotalScore DESC, TS.PostCount DESC;
//
// The LIKE against a tag name is `tag_mentions`. `WHERE TP.RN = 1` makes the LEFT JOIN inner, so the rank-1 posts are keyed by SPLIT_PART(Tags, ',', 1)
// and TagSummary is computed only for the tags that name matches. A ViewCount tie for a tag's top question goes to the smaller post id.
fn q26960(db: &'static So) -> String {
    let Post { post_type_id, view_count, tags_str, owner_user, score, creation_date, .. } = &db.post;
    let tm = tag_mentions(db);
    type PT = (Id<Post>, Id<Tag>);
    let q = drain((&tm).with(Same::<PT>::new().map(|(p, _): PT| p).select(Ident::<Post>::new().with(post_type_id.eq(1)))));
    let rn1 = top_per(q, |&(_, (_, t))| t, |&(_, (p, _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 1, false);
    let rn1: MatSet<Id<Post>> = rel(rn1.into_iter().map(|x| x.1 .0).collect()).map(|p| p).collect();
    let by_split: HashIdx<Str, Id<Post>> = (&rn1).select(tags_str.map(|t: Str| t.split(',').next().unwrap())).inv().collect();
    let tags = || db.tag.with((&db.tag.tag_name).select(&by_split));
    let by_tag_: HashIdx<Id<Tag>, PT> = (&tm).map(|(_, t): PT| t).inv().collect();
    let by_tag = (&by_tag_).map(|(p, _): PT| p);
    let owned = (&by_tag).select(Ident::<Post>::new().with(owner_user));
    let ts = tags().group_by(Ident::<Tag>::new()).select(owned.select(view_count.opt().and(score).and(creation_date).and(owner_user.select(&db.user.creation_date))))
        .fold([0i64; 5], |a, (((w, s), d), ud)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + s, a[3] + (trunc_day(d) - trunc_day(ud)) / DAY_US, a[4] + 1]);
    let pc = tags().group_by(Ident::<Tag>::new()).select((&by_tag).select(Ident::<Post>::new().with(owner_user))).count_distinct();
    let names = tags().group_by(Ident::<Tag>::new()).select((&by_tag).select(owner_user).select(&db.user.display_name)).buf_fold(|mut v| {
        v.sort_unstable();
        v.dedup();
        &*Box::leak(v.into_boxed_slice())
    });
    let v = drain((&ts).and(&pc).and(&names).and((&db.tag.tag_name).select(&by_split)));
    rows(v.into_iter().map(|(t, (((a, n), c), p))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(n), nullable(a[1], a[0]), V::I(a[2]), V::L(c.iter().map(|&x| V::S(x)).collect()), avg(a[3], a[4])];
        f.extend(post_fields(db, p, &["title", "score", "views", "created"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(v.BountyAmount) OVER (PARTITION BY p.Id) AS TotalBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 1000)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.RankByScore, COALESCE(ut.UserCount, 0) AS UniqueUserVotes,
//        CASE WHEN rp.TotalBounty IS NULL THEN 'No Bounty' ELSE CONCAT('Total Bounty: $', rp.TotalBounty) END AS BountyInfo
// FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(DISTINCT UserId) AS UserCount FROM Votes WHERE VoteTypeId IN (2, 3) GROUP BY PostId) ut ON rp.PostId = ut.PostId
// WHERE rp.RankByScore <= 10 ORDER BY rp.Score DESC, rp.ViewCount ASC LIMIT 50;
//
// RankedPosts has no GROUP BY, so RankByScore numbers the post x comment x vote rows; rows of one post tie on Score and agree in every projected column but
// the number, and the tie goes to the smaller comment and vote id.
fn q4270(db: &'static So) -> String {
    let Post { creation_date, view_count, score, post_type_id, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(view_count.gt(1000));
    let j = rel(drain(rp().select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt()))).into_iter().map(|x| x.1).collect::<Vec<_>>());
    type J = ((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>);
    let jv = drain((&j).select(Same::<J>::new().and(Same::<J>::new().map(|((p, _), _): J| p).select(post_type_id))));
    let rk = per_group(ranked(jv.into_iter().map(|x| x.1).collect(), |&(((p, c), v), t)| (t, Reverse(score.get(p).unwrap()), p, c, v), false), |&(_, t)| t);
    let tb = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).fold((0i64, 0i64), |(n, s), (_, b)| {
        let b = b.flatten();
        (n + b.is_some() as i64, s + b.unwrap_or(0))
    });
    let uu = db.vote.with((&db.vote.vote_type_id).is_in([2, 3])).group_by(&db.vote.post).select(&db.vote.user).count_distinct();
    type R = ((J, i64), i64);
    let top = rel(rk);
    let v = drain((&top).filt(|(_, r)| r <= 10).select(Same::<R>::new().and(Same::<R>::new().map(|((((p, _), _), _), _): R| p).select((&tb).and((&uu).opt())))));
    let v = top_n(v, |&(_, (((((p, c), vt), _), r), _))| (Reverse(score.get(p).unwrap()), view_count.get(p), p, r, c, vt), 50);
    rows(v.into_iter().map(|(_, (((((p, _), _), _), r), ((n, s), u)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(r), V::I(u.unwrap_or(0)), if n == 0 { V::S("No Bounty") } else { V::Owned(format!("Total Bounty: ${s}")) }]);
        row(f)
    }))
}

// WITH Tag_List AS (SELECT split_part(tag, '>', 1) AS TagName, COUNT(*) AS TagCount
//     FROM (SELECT unnest(string_to_array(substring(Tags, 2, length(Tags)-2), '><')) AS tag, Id FROM Posts WHERE PostTypeId = 1) AS Tags_Sub GROUP BY TagName),
// Active_Users AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// Top_Tags AS (SELECT TagName, TagCount, ROW_NUMBER() OVER (ORDER BY TagCount DESC) AS TagRank FROM Tag_List WHERE TagCount > 5)
// SELECT au.DisplayName AS ActiveUser, au.PostCount AS NumberOfPosts, au.UpVotes AS TotalUpVotes, au.DownVotes AS TotalDownVotes, tt.TagName AS MostUsedTag, tt.TagCount AS UsageCount
// FROM Active_Users au JOIN Top_Tags tt ON TRUE WHERE au.UpVotes > 10 ORDER BY au.PostCount DESC, tt.TagCount DESC LIMIT 10;
//
// `JOIN .. ON TRUE` is a cross join (cross_top drives it); TagRank is never read. split_part(tag, '>', 1) is applied to each tag, which has no '>' left.
fn q29793(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, owner_user, score, .. } = &db.post;
    let tl = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list).map(|t: Str| t.split('>').next().unwrap())).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tt = drain((&tl).filt(|n| n > 5));
    let au = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score).fold([0i64; 3], |a, s| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64]);
    let au = drain((&au).filt(|a| a[1] > 10));
    let v = cross_top(au, |&(u, a)| (Reverse(a[0]), u), tt, |&(t, n)| (Reverse(n), t), 10);
    rows(v.into_iter().map(|((u, a), (t, n))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t), V::I(n)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// CommentsSummary AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, ' | ') AS SampleComments FROM Comments c GROUP BY c.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.AnswerCount, tp.OwnerDisplayName, cs.CommentCount, cs.SampleComments
// FROM TopPosts tp LEFT JOIN CommentsSummary cs ON tp.PostId = cs.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// The STRING_AGG has no ORDER BY; the comments are joined in comment id order. A (Score, ViewCount) tie at Rank 10 goes to the smaller post id.
fn q7561(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30)).and(post_type_id.eq(1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).buf_fold(|mut v| {
        v.sort_unstable();
        (v.len() as i64, &*Box::leak(v.iter().map(|&c| db.comment.text.get(c).unwrap()).collect::<Vec<_>>().join(" | ").into_boxed_str()))
    });
    let v = drain((&tp).select((&cs).opt()));
    rows(v.into_iter().map(|(p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "owner"]);
        f.extend(match c {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, u.DisplayName AS Owner, COUNT(c.Id) AS CommentCount,
//        (SELECT COUNT(v.Id) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVotes, (SELECT COUNT(v.Id) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVotes
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '6 months'
//     GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, u.DisplayName),
// PostTags AS (SELECT p.Id AS PostId, STRING_AGG(t.TagName, ', ') AS Tags FROM Posts p JOIN (SELECT unnest(string_to_array(p.Tags, '>')) AS tag FROM Posts p) AS tag ON TRUE
//     JOIN Tags t ON t.TagName = tag GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.Owner, rp.CommentCount, rp.UpVotes, rp.DownVotes, pt.Tags FROM RecentPosts rp JOIN PostTags pt ON rp.PostId = pt.PostId
// ORDER BY rp.ViewCount DESC, rp.CreationDate DESC LIMIT 10;
//
// The subquery's `p` is its own Posts, so every post is crossed with every '>'-piece of every post's Tags that names a tag; the pieces are joined to Tags
// first, and the cross is driven only for the recent posts the final join keeps. STRING_AGG is joined in tag id order.
fn q8019(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, view_count, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_months(current_date(), -6))).with(owner_user);
    let by_name: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pieces = rel(drain(tags_str.flat_map(|t: Str| t.split('>')).select(&by_name)).into_iter().map(|x| x.1).collect::<Vec<_>>());
    let recent: MatSet<Id<Post>> = rp().collect();
    let mut pairs = Vec::new();
    (&recent).cross(&pieces).drive(|(p, _), (_, t)| pairs.push((p, t)));
    let pr = rel(pairs);
    type PT = (Id<Post>, Id<Tag>);
    let pt = (&pr).group_by(Same::<PT>::new().map(|(p, _): PT| p)).select(Same::<PT>::new().map(|(_, t): PT| t)).buf_fold(|mut v| {
        v.sort_unstable();
        &*Box::leak(v.iter().map(|&t| db.tag.tag_name.get(t).unwrap()).collect::<Vec<_>>().join(", ").into_boxed_str())
    });
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(&vc).and(&pt));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()), p)
    }, 10);
    rows(v.into_iter().map(|(p, ((c, u), t))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "owner"]);
        f.extend([V::I(c), V::I(u[0]), V::I(u[1]), V::S(t)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN VB.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN VB.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS PostCount, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Votes VB ON U.Id = VB.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostScore AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Score, P.CreationDate, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')
// SELECT U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, U.PostCount, PS.PostId, PS.Score, PS.CreationDate, PS.ScoreRank
// FROM UserStats U JOIN PostScore PS ON U.UserId = PS.OwnerUserId WHERE (U.UpVotes - U.DownVotes) >= 10 AND PS.ScoreRank = 1 AND PS.CreationDate IS NOT NULL
// ORDER BY U.Reputation DESC, PS.Score DESC LIMIT 50;
//
// UserRank is never read; UserStats' votes-cast x posts product is driven for the owners of the recent posts alone. A tie at the cut goes to the smaller post id.
fn q3159(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let ps = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let top = top_per(ps, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt())).fold(0i64, |n, (t, _)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let pc = user_distinct_posts(db);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and((&us).filt(|n| n >= 10)).and(pc))));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 50);
    let vt = (&owners).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt())).fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let rv = rel(v);
    type R = (Id<Post>, ((Id<User>, i64), i64));
    let v = drain((&rv).select(Same::<R>::new().and(Same::<R>::new().map(|(_, ((u, _), _)): R| u).select(&vt))));
    rows(v.into_iter().map(|(_, ((p, ((u, _), n)), a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.extend(post_fields(db, p, &["id", "score", "created"]));
        f.push(V::I(1));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS Author,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.Author FROM RankedPosts rp WHERE rp.Rank <= 5),
// CommentsAggregate AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, ARRAY_AGG(c.Text ORDER BY c.CreationDate DESC) AS LatestComments
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.Author, ca.CommentCount, ca.LatestComments
// FROM TopPosts tp JOIN CommentsAggregate ca ON tp.PostId = ca.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// A post with no comments aggregates its one NULL row into [NULL]. A (Score, ViewCount) tie at Rank 5 goes to the smaller post id, and a CreationDate tie
// inside the ARRAY_AGG to the smaller comment id.
fn q7788(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, post_type_id, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ca = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(|mut v| {
        v.sort_by_key(|c| c.map(|c| (Reverse(db.comment.creation_date.get(c).unwrap()), c)));
        (v.iter().filter(|c| c.is_some()).count() as i64, &*Box::leak(v.into_boxed_slice()))
    });
    let v = drain(&ca);
    rows(v.into_iter().map(|(p, (n, cs))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        f.extend([V::I(n), V::L(cs.iter().map(|c| c.map_or(V::Null, |c| V::S(db.comment.text.get(c).unwrap()))).collect())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, STRING_AGG(t.TagName, ', ') AS Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Tags t ON t.WikiPostId = p.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '1 year' AND p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS QuestionsAsked, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id AND p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(p.Id) > 10)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.QuestionsAsked, u.TotalViews, u.TotalScore, rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.Tags
// FROM TopUsers u JOIN RankedPosts rp ON u.UserId = rp.UserPostRank WHERE rp.UserPostRank <= 5 ORDER BY u.Reputation DESC, u.TotalViews DESC;
//
// `u.UserId = rp.UserPostRank` joins a user id to a row number, through the raw ids. The ownerless questions are one partition; a ViewCount tie within an
// owner goes to the smaller post id. STRING_AGG is joined in tag id order.
fn q29831(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, view_count, score, .. } = &db.post;
    let rq = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)));
    let rk = per_group(ranked(drain(rq().select(owner_user.opt())), |&(p, u)| {
        let w = view_count.get(p);
        (u, w.is_none(), Reverse(w), p)
    }, false), |&(_, u)| u);
    let rp = rel(rk.into_iter().filter(|x| x.1 <= 5).map(|((p, _), r)| (p, r)).collect());
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(view_count.opt().and(score)))
        .fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let by_wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let tg = rq().group_by(Ident::<Post>::new()).select((&by_wiki).opt()).buf_fold(|mut v| {
        v.sort_unstable();
        let n: Vec<Str> = v.into_iter().flatten().map(|t| db.tag.tag_name.get(t).unwrap()).collect();
        if n.is_empty() { None } else { Some(&*Box::leak(n.join(", ").into_boxed_str())) }
    });
    type R = (Id<Post>, i64);
    let v = drain((&rp).select(Same::<R>::new().and(Same::<R>::new().map(|(_, r): R| r).select(&uidx).select(Ident::<User>::new().and((&tu).filt(|a| a[0] > 10)))).and(Same::<R>::new().map(|(p, _): R| p).select(&tg))));
    rows(v.into_iter().map(|(_, (((p, _), (u, a)), t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
        f.push(ostr(t));
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, ub.BadgeCount, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u JOIN UserBadgeCounts ub ON u.Id = ub.UserId
//     WHERE u.Reputation > 1000),
// PopularTags AS (SELECT Tag, COUNT(*) AS TagCount FROM PostTags GROUP BY Tag ORDER BY TagCount DESC LIMIT 10)
// SELECT tu.DisplayName AS TopUserName, tu.Reputation AS TopUserReputation, pt.Tag AS PopularTag, pt.TagCount AS PopularTagCount
// FROM TopUsers tu JOIN PopularTags pt ON pt.Tag IN (SELECT UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) FROM Posts p WHERE p.OwnerUserId = tu.Id)
// ORDER BY tu.Reputation DESC, pt.TagCount DESC;
//
// UserBadgeCounts has a row for every user and BadgeCount is never read. The IN is a semi-join on the set of (user, tag) pairs of the user's posts.
fn q26544(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, .. } = &db.post;
    let tc = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let pt = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10));
    let pt_by: HashIdx<Str, (Str, i64)> = (&pt).map(|(t, _)| t).inv().select(&pt).collect();
    let ut: MatSet<(Id<User>, Str)> = db.post.select(owner_user.and(tags_str.flat_map(tag_list))).map(|x| x).collect();
    type UT = (Id<User>, Str);
    let v = drain((&ut).with(Same::<UT>::new().map(|(u, _): UT| u).select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).select(Same::<UT>::new().map(|(_, t): UT| t).select(&pt_by)));
    rows(v.into_iter().map(|((u, _), (t, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("23122", q23122),
    ("23834", q23834),
    ("4634", q4634),
    ("33172", q33172),
    ("23107", q23107),
    ("33771", q33771),
    ("22168", q22168),
    ("1080", q1080),
    ("22465", q22465),
    ("20861", q20861),
    ("24816", q24816),
    ("21859", q21859),
    ("24372", q24372),
    ("20220", q20220),
    ("20227", q20227),
    ("24373", q24373),
    ("600", q600),
    ("34625", q34625),
    ("24734", q24734),
    ("23523", q23523),
    ("2123", q2123),
    ("21566", q21566),
    ("26401", q26401),
    ("20686", q20686),
    ("22868", q22868),
    ("21519", q21519),
    ("34845", q34845),
    ("22149", q22149),
    ("24000", q24000),
    ("23124", q23124),
    ("148", q148),
    ("31926", q31926),
    ("33077", q33077),
    ("22783", q22783),
    ("24104", q24104),
    ("23835", q23835),
    ("20603", q20603),
    ("33358", q33358),
    ("20768", q20768),
    ("22865", q22865),
    ("22769", q22769),
    ("24920", q24920),
    ("22321", q22321),
    ("20402", q20402),
    ("23052", q23052),
    ("30079", q30079),
    ("32412", q32412),
    ("21526", q21526),
    ("21899", q21899),
    ("24456", q24456),
    ("22193", q22193),
    ("24215", q24215),
    ("22199", q22199),
    ("22747", q22747),
    ("23666", q23666),
    ("22945", q22945),
    ("23733", q23733),
    ("24187", q24187),
    ("32792", q32792),
    ("22725", q22725),
    ("21525", q21525),
    ("21162", q21162),
    ("33170", q33170),
    ("23988", q23988),
    ("24002", q24002),
    ("24168", q24168),
    ("22870", q22870),
    ("23976", q23976),
    ("23436", q23436),
    ("24657", q24657),
    ("24918", q24918),
    ("22872", q22872),
    ("21328", q21328),
    ("24848", q24848),
    ("23167", q23167),
    ("20656", q20656),
    ("22411", q22411),
    ("24609", q24609),
    ("12954", q12954),
    ("5054", q5054),
    ("8483", q8483),
    ("28100", q28100),
    ("25467", q25467),
    ("28667", q28667),
    ("26960", q26960),
    ("4270", q4270),
    ("29793", q29793),
    ("7561", q7561),
    ("8019", q8019),
    ("3159", q3159),
    ("7788", q7788),
    ("29831", q29831),
    ("26544", q26544),
];
