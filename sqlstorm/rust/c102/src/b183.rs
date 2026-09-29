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
];
