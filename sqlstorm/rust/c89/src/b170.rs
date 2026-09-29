use harness::prelude::*;
use std::cmp::Reverse;

// WITH RecursivePostHistory AS (SELECT ph.Id, ph.PostId, ph.UserId, ph.CreationDate, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) as rn FROM PostHistory ph),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN ph.PostId END) AS TotalClosedReopenedPosts,
//        AVG(COALESCE(p.Score, 0)) AS AvgPostScore, SUM(u.UpVotes) - SUM(u.DownVotes) AS ReputationDifference
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT u.UserId, u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalClosedReopenedPosts, u.AvgPostScore,
//        RANK() OVER (ORDER BY u.TotalPosts DESC) AS TotalPostsRank FROM UserPostStats u WHERE u.TotalPosts > 0)
// SELECT u.UserId, u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalClosedReopenedPosts, u.AvgPostScore,
//        CASE WHEN u.TotalQuestions > 0 THEN CAST(u.TotalAnswers AS DECIMAL) / u.TotalQuestions ELSE NULL END AS AnswerToQuestionRatio, u.TotalPostsRank
// FROM TopUsers u WHERE u.TotalPostsRank <= 10 ORDER BY u.TotalPostsRank;
//
// RecursivePostHistory is never referenced.
fn q31347(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(history_of(db).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, s), _)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + 1],
            None => [a[0], a[1], a[2], a[3], a[4] + 1],
        });
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).select(&db.post_history.post).collect();
    let cd = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(&closed)).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&s).filt(|a| a[0] > 0).and(&cd)), |&(_, (a, _))| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, c)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c), avg(a[3], a[4])]);
        f.push(if a[1] > 0 { V::F(a[2] as f64 / a[1] as f64) } else { V::Null });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RECURSIVE UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS PostRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// RecentVotes AS (SELECT V.PostId, COUNT(V.Id) AS VoteCount FROM Votes V WHERE V.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY V.PostId),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.AnswerCount, COALESCE(RV.VoteCount, 0) AS RecentVoteCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (ORDER BY P.Score DESC, COALESCE(RV.VoteCount, 0) DESC) AS PostRank
//     FROM Posts P LEFT JOIN RecentVotes RV ON P.Id = RV.PostId LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '90 days' AND P.PostTypeId = 1)
// SELECT UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.PositivePosts, UPS.NegativePosts, TP.PostId, TP.Title, TP.Score, TP.AnswerCount, TP.RecentVoteCount
// FROM UserPostStats UPS INNER JOIN TopPosts TP ON UPS.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = TP.PostId)
// WHERE UPS.TotalPosts > 10 AND UPS.PostRank <= 100 ORDER BY UPS.PositivePosts DESC, TP.Score DESC LIMIT 50;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q32217(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt())
        .fold([0i64; 3], |a, s| match s {
            Some(s) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64],
            None => a,
        });
    let v = ranked(drain(&ups), |&(_, a)| Reverse(a[0]), false);
    let top = rel(drain(rel(v).filt(|((_, a), r)| a[0] > 10 && r <= 100)).into_iter().map(|x| x.1 .0).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 3])> = (&top).map(|(u, _)| u).inv().select(&top).collect();
    let rv = db.vote.with((&db.vote.creation_date).ge(add_days(t0, -30))).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.post.with(creation_date.ge(add_days(t0, -90)).and(post_type_id.eq(1))).select(owner_user.select(&by_user).and((&rv).opt())));
    let v = top_n(v, |&(p, ((_, a), _))| (Reverse(a[1]), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((u, a), n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["id", "title", "score", "answers"]));
        f.push(V::I(n.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, COALESCE(SUM(prv.RevisionCount), 0) AS TotalRevisions
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS RevisionCount FROM PostHistory GROUP BY PostId) prv ON prv.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, TotalRevisions,
//        RANK() OVER (ORDER BY TotalUpVotes DESC) AS UpVoteRank FROM UserPostStats)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalUpVotes, tu.TotalDownVotes, tu.TotalRevisions,
//        CASE WHEN tu.TotalPosts > 100 THEN 'Active Contributor' WHEN tu.TotalPosts BETWEEN 50 AND 100 THEN 'Moderately Active' ELSE 'New User' END AS UserActivityLevel,
//        COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount
// FROM TopUsers tu LEFT JOIN PostLinks pl ON tu.UserId = pl.PostId
// GROUP BY tu.UserId, tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalUpVotes, tu.TotalDownVotes, tu.TotalRevisions, tu.UpVoteRank
// HAVING COUNT(DISTINCT pl.RelatedPostId) > 0 ORDER BY tu.UpVoteRank LIMIT 10;
//
// `tu.UserId = pl.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q1097(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let rc = db.post.group_by(Ident::<Post>::new()).select(history_of(db)).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&rc).opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((t, v), r)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + r.unwrap_or(0), a[6] + r.is_some() as i64],
            None => a,
        });
    let links: HashIdx<i64, Id<PostLink>> = (&db.post_link.post_id).inv().collect();
    let lc = db.user.group_by(Ident::<User>::new()).select((&db.user.origid).select(&links).select(&db.post_link.related_post_id)).count_distinct();
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[3]), false);
    let rk = rel(v);
    let by_user: HashIdx<Id<User>, ((Id<User>, [i64; 7]), i64)> = (&rk).map(|((u, _), _)| u).inv().select(&rk).collect();
    let v = drain((&lc).filt(|n| n > 0).and(&by_user));
    let v = top_n(v, |&(u, (_, (_, r)))| (r, u), 10);
    rows(v.into_iter().map(|(u, (n, ((_, a), _)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a[..5].iter().map(|&x| V::I(x)));
        f.push(V::I(a[5]));
        f.push(V::S(if a[0] > 100 { "Active Contributor" } else if a[0] >= 50 { "Moderately Active" } else { "New User" }));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldCount, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverCount,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeCount FROM Badges b GROUP BY b.UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId GROUP BY p.OwnerUserId),
// RankedUsers AS (SELECT u.Id, u.DisplayName, COALESCE(ub.GoldCount, 0) AS GoldBadges, COALESCE(ub.SilverCount, 0) AS SilverBadges, COALESCE(ub.BronzeCount, 0) AS BronzeBadges,
//        ps.PostCount, ps.TotalScore, ps.TotalComments, RANK() OVER (ORDER BY COALESCE(ps.TotalScore, 0) DESC, ps.PostCount DESC) AS UserRank
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT r.DisplayName, r.GoldBadges, r.SilverBadges, r.BronzeBadges, r.PostCount, r.TotalScore, r.TotalComments,
//        CASE WHEN r.UserRank <= 10 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributorStatus,
//        CASE WHEN EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = r.Id AND p.ClosedDate IS NOT NULL) THEN 'Has Closed Posts' ELSE 'No Closed Posts' END AS PostClosureStatus
// FROM RankedUsers r WHERE r.PostCount > 0 ORDER BY r.TotalScore DESC, r.PostCount DESC LIMIT 20;
fn q971(db: &'static So) -> String {
    let Post { owner_user, score, closed_date, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let ps = db.post.group_by(owner_user).select(score.and((&cc).opt())).fold([0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c.unwrap_or(0)]);
    let closed: MatSet<Id<User>> = db.post.with(closed_date).select(owner_user).collect();
    let v = ranked(drain((&ub).and((&ps).opt()).and(Ident::<User>::new().with(&closed).opt())), |&(_, ((_, p), _))| (Reverse(p.map_or(0, |a| a[1])), p.is_none(), Reverse(p.map(|a| a[0]))), false);
    let v = drain(rel(v).filt(|((_, ((_, p), _)), _)| p.map_or(false, |a| a[0] > 0))).into_iter().map(|x| x.1).collect();
    let v = top_n(v, |&((u, ((_, p), _)), _)| (Reverse(p.unwrap()[1]), Reverse(p.unwrap()[0]), u), 20);
    rows(v.into_iter().map(|((u, ((b, p), c)), r)| {
        let p = p.unwrap();
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend(p.map(V::I));
        f.push(V::S(if r <= 10 { "Top Contributor" } else { "Contributor" }));
        f.push(V::S(if c.is_some() { "Has Closed Posts" } else { "No Closed Posts" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
//        COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore,
//        SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, TotalScore, BadgeCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT TU.DisplayName, TU.Reputation, TU.QuestionCount, TU.AnswerCount, TU.TotalScore, TU.BadgeCount, HF.UserDisplayName AS TopVoter,
//        COUNT(DISTINCT P.Id) AS VotedPosts, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived
// FROM TopUsers TU LEFT JOIN Votes V ON TU.UserId = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id
// LEFT JOIN (SELECT U.Id, U.DisplayName AS UserDisplayName, COUNT(V2.Id) AS VotesGiven FROM Votes V2 JOIN Users U ON V2.UserId = U.Id
//     GROUP BY U.Id, U.DisplayName ORDER BY VotesGiven DESC LIMIT 1) HF ON 1=1
// WHERE TU.Rank <= 10 GROUP BY TU.DisplayName, TU.Reputation, TU.QuestionCount, TU.AnswerCount, TU.TotalScore, TU.BadgeCount, HF.UserDisplayName
// ORDER BY TU.Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x badges product is driven for those alone. HF is one row, crossed in.
fn q9952(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, s) = p.map_or((0, 0), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + b.is_some() as i64]
        });
    let vs = (&tu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).opt())).opt())
        .buf_fold(|v| (distinct_some(v.iter().map(|x| x.and_then(|y| y.1))), v.iter().filter(|x| x.map(|y| y.0) == Some(2)).count() as i64));
    let vg = db.vote.with(&db.vote.user).group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let hf = rel(top_n(drain(&vg), |&(u, n)| (Reverse(n), u), 1).into_iter().map(|x| x.0).collect());
    let mut v = Vec::new();
    (&us).and(&vs).cross(&hf).drive(|(u, _), ((a, (n, up)), h)| v.push((u, a, n, up, h)));
    rows(v.into_iter().map(|(u, a, n, up, h)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([user_col(db, h, "name"), V::I(n), V::I(up)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COUNT(c.Id) AS CommentCount, p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId AND v.VoteTypeId IN (8, 9) LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Rank, us.Reputation FROM RankedPosts rp JOIN UserStatistics us ON rp.OwnerUserId = us.UserId WHERE rp.Rank = 1),
// PostHistoryData AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseVotes, COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteVotes
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT tp.PostId, tp.Title, tp.Reputation, COALESCE(ph.CloseVotes, 0) AS CloseVoteCount, COALESCE(ph.DeleteVotes, 0) AS DeleteVoteCount
// FROM TopPosts tp LEFT JOIN PostHistoryData ph ON tp.PostId = ph.PostId WHERE tp.Reputation > 1000 ORDER BY tp.Reputation DESC, tp.Title ASC;
//
// Only Reputation is read from UserStatistics, whose one row per user makes the join a plain lookup of the owner.
fn q33610(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ph = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 12) as i64]);
    let v = drain((&tp).select(owner_user.select((&db.user.reputation).gt(1000)).and((&ph).opt())));
    rows(v.into_iter().map(|(p, (r, h))| {
        let h = h.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(r), V::I(h[0]), V::I(h[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(u.Reputation, 0) AS OwnerReputation, rp.CommentCount,
//        CASE WHEN rp.Score >= 100 THEN 'High Score' WHEN rp.Score >= 50 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
//     FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.ScoreRank = 1),
// AggregateData AS (SELECT ScoreCategory, COUNT(*) AS PostCount, AVG(OwnerReputation) AS AvgReputation, MAX(ViewCount) AS MaxViewCount,
//        MIN(COALESCE(CommentCount, 0)) AS MinCommentCount FROM PostStats GROUP BY ScoreCategory)
// SELECT ad.ScoreCategory, ad.PostCount, ad.AvgReputation, ad.MaxViewCount, ad.MinCommentCount,
//        CASE WHEN ad.PostCount > 0 THEN ROUND(AVG(ad.AvgReputation) OVER (), 2) ELSE NULL END AS OverallAvgReputation
// FROM AggregateData ad ORDER BY CASE ad.ScoreCategory WHEN 'High Score' THEN 1 WHEN 'Medium Score' THEN 2 WHEN 'Low Score' THEN 3 END;
//
// ScoreRank numbers post x comment rows, but the rows of one post agree in every column PostStats reads, so the rank is taken over posts
// (partitioned by the raw OwnerUserId, NULL included); a score tie between two posts of one owner goes to the smaller post id.
fn q20553(db: &'static So) -> String {
    let Post { owner_user_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user_id.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let cat = |s: i64| if s >= 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" };
    let g = (&tp)
        .group_by(score.map(cat))
        .select(owner_user.select(&db.user.reputation).opt().and(view_count.opt()).and((&cc).opt()))
        .fold([0i64, 0, i64::MIN, i64::MAX], |a, ((r, w), c)| [a[0] + 1, a[1] + r.unwrap_or(0), w.map_or(a[2], |w| a[2].max(w)), a[3].min(c.unwrap_or(0))]);
    let v = drain(&g);
    let overall = v.iter().map(|(_, a)| a[1] as f64 / a[0] as f64).sum::<f64>() / v.len() as f64;
    rows(v.into_iter().map(|(c, a)| {
        row(vec![V::S(c), V::I(a[0]), avg(a[1], a[0]), if a[2] == i64::MIN { V::Null } else { V::I(a[2]) }, V::I(a[3]), V::F((overall * 100.0).round() / 100.0)])
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(p.Score) AS AvgPostScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AvgPostScore, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats),
// RecentVotes AS (SELECT v.UserId, v.PostId, v.CreationDate, vt.Name AS VoteType, ROW_NUMBER() OVER (PARTITION BY v.UserId ORDER BY v.CreationDate DESC) AS VoteRank
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE v.CreationDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'))
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.AvgPostScore, COUNT(rv.UserId) FILTER (WHERE rv.VoteType = 'UpMod') AS RecentUpVotes,
//        COUNT(rv.UserId) FILTER (WHERE rv.VoteType = 'DownMod') AS RecentDownVotes, COALESCE((SELECT SUM(Score) FROM Posts WHERE OwnerUserId = tu.UserId), 0) AS TotalPostScore
// FROM TopUsers tu LEFT JOIN RecentVotes rv ON tu.UserId = rv.UserId AND rv.VoteRank <= 5 WHERE tu.PostRank <= 10
// GROUP BY tu.UserId, tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.AvgPostScore ORDER BY tu.TotalPosts DESC, tu.AvgPostScore DESC;
//
// A tie on CreationDate at a user's fifth recent vote goes to the larger vote id.
fn q769(db: &'static So) -> String {
    let Vote { user, creation_date, .. } = &db.vote;
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[1]), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let rv = drain(db.vote.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(user).select(user));
    let rv = top_per(rv, |&(_, u)| u, |&(v, _)| (Reverse(creation_date.get(v).unwrap()), Reverse(v)), 5, false);
    let rv: MatSet<Id<Vote>> = rel(rv.into_iter().map(|x| x.0).collect()).map(|v| v).collect();
    let rc = (&rv).group_by(user).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 10])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let v = drain((&by_user).and((&rc).opt()));
    rows(v.into_iter().map(|(u, ((_, a), c))| {
        let c = c.unwrap_or([0, 0]);
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1]), V::I(c[0]), V::I(c[1]), V::I(a[4])])
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(bs.Class), 0) AS TotalBadges FROM Users u LEFT JOIN Badges bs ON u.Id = bs.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostDetails AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.UpVotes, ps.DownVotes, ur.DisplayName, ur.Reputation, ur.TotalBadges
//     FROM PostStatistics ps INNER JOIN UserReputation ur ON ps.OwnerUserId = ur.UserId WHERE ps.UserPostRank <= 5)
// SELECT pd.Title, pd.CreationDate, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.DisplayName, pd.Reputation, pd.TotalBadges, 'Score: ' || (pd.UpVotes - pd.DownVotes) AS PostScore
// FROM PostDetails pd ORDER BY pd.CommentCount DESC, (pd.UpVotes - pd.DownVotes) DESC LIMIT 10;
//
// UserPostRank reads only base columns, so each owner's five newest questions are picked first and the comment x vote product is driven for those alone.
fn q2713(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |n, c| n + c.unwrap_or(0));
    let v = drain((&s).and(owner_user.select(Ident::<User>::new().and(&tb))));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[0]), Reverse(a[1] - a[2]), p), 10);
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::Owned(format!("Score: {}", a[1] - a[2]))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, MAX(u.CreationDate) AS AccountCreated, MAX(u.LastAccessDate) AS LastAccessed
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostWithVoteCounts AS (SELECT p.Id, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotesCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotesCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ur.Reputation, ur.BadgeCount, COALESCE(pvc.UpVotesCount, 0) AS UpVotes, COALESCE(pvc.DownVotesCount, 0) AS DownVotes,
//        CASE WHEN ur.Reputation > 1000 THEN 'High Reputation' WHEN ur.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory,
//        CASE WHEN rp.PostRank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostRating
// FROM RankedPosts rp JOIN UserReputation ur ON ur.UserId = rp.PostId LEFT JOIN PostWithVoteCounts pvc ON pvc.Id = rp.PostId WHERE ur.BadgeCount > 0
// ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// `ur.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. A score tie at the fifth place goes to the smaller post id.
fn q4965(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let top = top_per(drain(recent().select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let top: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain(recent().select(origid.select(&by_raw).select(Ident::<User>::new().and((&bc).filt(|n| n > 0)))).and(&pv).and(Ident::<Post>::new().with(&top).opt()));
    rows(v.into_iter().map(|(p, (((u, b), a), t))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(r), V::I(b), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        f.push(V::S(if t.is_some() { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// Rewritten (rewrites/189.sql): the RecentPostRank order is tie-broken on P.Id.
// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC, P.Id) AS RecentPostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// AggregateVotes AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, COUNT(*) FILTER (WHERE V.VoteTypeId = 3) AS DownVotes FROM Votes V GROUP BY V.PostId)
// SELECT U.DisplayName, U.Reputation, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, RP.PostId, RP.Title, RP.CreationDate, COALESCE(AV.UpVotes, 0) AS UpVotes,
//        COALESCE(AV.DownVotes, 0) AS DownVotes, CASE WHEN UB.BadgeCount > 10 THEN 'Veteran' WHEN UB.BadgeCount > 5 THEN 'Experienced' ELSE 'Novice' END AS UserLevel
// FROM Users U JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN RecentPosts RP ON U.Id = RP.OwnerUserId AND RP.RecentPostRank = 1 LEFT JOIN AggregateVotes AV ON RP.PostId = AV.PostId
// WHERE U.Reputation > 50 ORDER BY U.Reputation DESC, RP.CreationDate DESC LIMIT 100;
fn q189(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let rp = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let rp = top_per(rp, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp);
    let newest: HashIdx<Id<User>, Id<Post>> = (&rp).map(|(p, u)| (u, p)).map(|(u, _)| u).inv().select((&rp).map(|(p, _)| p)).collect();
    let av = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain(db.user.with((&db.user.reputation).gt(50)).select((&ub).and((&newest).select(Ident::<Post>::new().and((&av).opt())).opt())));
    let v = top_n(v, |&(u, (_, p))| {
        let d = p.map(|(p, _)| creation_date.get(p).unwrap());
        (Reverse(db.user.reputation.get(u).unwrap()), d.is_none(), Reverse(d), u)
    }, 100);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        match p {
            Some((p, a)) => {
                f.extend(post_fields(db, p, &["id", "title", "created"]));
                let a = a.unwrap_or([0, 0]);
                f.extend([V::I(a[0]), V::I(a[1])]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::I(0), V::I(0)]),
        }
        f.push(V::S(if b[0] > 10 { "Veteran" } else if b[0] > 5 { "Experienced" } else { "Novice" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId IN (2, 8) THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COALESCE(AVG(P.Score), 0) AS AveragePostScore, RANK() OVER (ORDER BY COUNT(V.Id) DESC) AS VoteRank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalVotes, UpVotes, DownVotes, AveragePostScore, VoteRank FROM UserVoteStats WHERE TotalVotes > 0 ORDER BY TotalVotes DESC LIMIT 10),
// PostSummary AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2), 0) AS UpVotes,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3), 0) AS DownVotes, P.OwnerUserId FROM Posts P WHERE P.OwnerUserId IS NOT NULL)
// SELECT U.DisplayName AS TopUser, P.Title AS PostTitle, P.CreationDate AS PostDate, P.Score AS PostScore, P.CommentCount AS TotalComments, P.UpVotes AS TotalUpVotes, P.DownVotes AS TotalDownVotes
// FROM TopUsers U INNER JOIN PostSummary P ON U.UserId = P.OwnerUserId ORDER BY U.TotalVotes DESC, P.Score DESC LIMIT 5;
//
// Only TotalVotes is read from UserVoteStats; the Posts join matches at most one post per vote, so it is the vote count.
fn q3967(db: &'static So) -> String {
    let tv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.post).opt()).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let tu = top_n(drain((&tv).filt(|n| n > 0)), |&(u, n)| (Reverse(n), u), 10);
    let tu = rel(tu);
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain(db.post.select((&db.post.owner_user).select(&by_user).and((&cc).opt()).and((&vc).opt())));
    let v = top_n(v, |&(p, (((_, n), _), _))| (Reverse(n), Reverse(db.post.score.get(p).unwrap()), p), 5);
    rows(v.into_iter().map(|(p, (((u, _), c), a))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// MostActiveUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalQuestions, TotalUpVotes, TotalDownVotes, UserRank FROM UserStatistics WHERE TotalPosts > 5)
// SELECT A.UserId, A.DisplayName, A.Reputation, A.TotalPosts, A.TotalAnswers, A.TotalQuestions, A.TotalUpVotes, A.TotalDownVotes, A.UserRank,
//        COALESCE(B.BadgeCount, 0) AS BadgeCount, COALESCE(C.CommentCount, 0) AS CommentCount
// FROM MostActiveUsers A LEFT JOIN (SELECT UserId, COUNT(Id) AS BadgeCount FROM Badges GROUP BY UserId) B ON A.UserId = B.UserId
// LEFT JOIN (SELECT UserId, COUNT(Id) AS CommentCount FROM Comments GROUP BY UserId) C ON A.UserId = C.UserId
// WHERE A.Reputation BETWEEN 1000 AND 10000 ORDER BY A.Reputation DESC, A.UserRank ASC LIMIT 10;
//
// The WHERE, the ORDER BY and TotalPosts (a COUNT(DISTINCT), one row per post) read no joined votes, so the ten users are picked first and the
// posts x votes product is driven for those alone. UserRank numbers every user by Reputation; a tie goes to the smaller user id.
fn q4466(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let rk = rel(top_n(drain(reputation), |&(u, r)| (Reverse(r), u), 0).into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rk).map(|(u, _)| u).inv().select(&rk).collect();
    let pc = db.user.with(reputation.between(1000, 10000)).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tu = top_n(drain((&pc).filt(|n| n > 5).and(&rank)), |&(_, (_, (_, r)))| r, 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let cc = (&tu).group_by(Ident::<User>::new()).select(comments_by(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and(&pc).and(&rank).and((&bc).opt()).and((&cc).opt()));
    rows(v.into_iter().map(|(u, ((((a, n), (_, r)), b), c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(b.unwrap_or(0)), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, COUNT(DISTINCT c.Id) AS TotalComments,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        ROW_NUMBER() OVER (PARTITION BY rp.PostId ORDER BY rp.CreationDate DESC) AS Rank
//     FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId
//     GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName),
// FilteredPosts AS (SELECT *, (TotalUpVotes - TotalDownVotes) AS NetVotes FROM PostDetails WHERE Rank <= 5)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.AnswerCount, fp.CommentCount, fp.OwnerDisplayName, fp.TotalComments, fp.TotalUpVotes, fp.TotalDownVotes, fp.NetVotes
// FROM FilteredPosts fp ORDER BY fp.CreationDate DESC;
//
// PostDetails.Rank partitions by the post it groups on, so it is 1 for every row and the filter keeps them all.
fn q8994(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let tc = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(&tc));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Location, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND u.Reputation > 100 GROUP BY u.Id, u.DisplayName, u.Reputation, u.Location),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate AS ClosedDate, ph.UserDisplayName AS ClosedBy, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10),
// PostStatistics AS (SELECT rp.Title, rp.ViewCount, a.DisplayName AS OwnerDisplayName, a.BadgeCount, cp.ClosedDate, cp.ClosedBy, cp.CloseReason,
//        CASE WHEN cp.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus
//     FROM RankedPosts rp JOIN ActiveUsers a ON rp.OwnerUserId = a.UserId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId)
// SELECT ps.Title, ps.ViewCount, ps.OwnerDisplayName, ps.BadgeCount, ps.ClosedDate, ps.ClosedBy, ps.CloseReason, ps.PostStatus
// FROM PostStatistics ps WHERE ps.PostStatus = 'Active' ORDER BY ps.ViewCount DESC LIMIT 10;
//
// PostStatus is 'Active' exactly when the LEFT JOIN found no close record (ClosedDate is the history row's CreationDate, never NULL), so it is an anti-join.
fn q32707(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let User { last_access_date, reputation, .. } = &db.user;
    let active = Ident::<User>::new().with(last_access_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(reputation.gt(100)));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    let v = drain(db.post.with(post_type_id.eq(1)).minus(&closed).select(owner_user.select(active.and(&bc))));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (u, b))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([user_col(db, u, "name"), V::I(b), V::Null, V::Null, V::Null, V::S("Active")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.PostTypeId, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.PostTypeId, u.DisplayName),
// PostAnalytics AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Author, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.rn, (rp.UpVotes - rp.DownVotes) AS Score,
//        CASE WHEN rp.CommentCount > 10 THEN 'Highly Engaged' WHEN rp.CommentCount BETWEEN 5 AND 10 THEN 'Moderately Engaged' ELSE 'Minimally Engaged' END AS EngagementLevel FROM RankedPosts rp)
// SELECT pa.Id, pa.Title, pa.CreationDate, pa.Author, pa.CommentCount, pa.UpVotes, pa.DownVotes, pa.Score, pa.EngagementLevel,
//        CASE WHEN pa.Score > 5 THEN 'Popular' WHEN pa.Score BETWEEN 0 AND 5 THEN 'Average' ELSE 'Unpopular' END AS PopularityLevel
// FROM PostAnalytics pa WHERE pa.rn <= 10 ORDER BY pa.Score DESC, pa.CreationDate DESC LIMIT 50;
//
// rn reads only base columns, so the ten newest posts of each type are picked first and the comment x vote product is driven for those alone.
fn q68(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, a)| (Reverse(a[1] - a[2]), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, a)| {
        let sc = a[1] - a[2];
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(sc)]);
        f.push(V::S(if a[0] > 10 { "Highly Engaged" } else if a[0] >= 5 { "Moderately Engaged" } else { "Minimally Engaged" }));
        f.push(V::S(if sc > 5 { "Popular" } else if sc >= 0 { "Average" } else { "Unpopular" }));
        row(f)
    }))
}

// WITH UserRankings AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 0),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.LastActivityDate, COUNT(C.Id) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Score
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (2, 3)
//     WHERE P.PostTypeId = 1 AND P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.OwnerUserId, P.LastActivityDate),
// PostDetails AS (SELECT RP.PostId, RP.Title, U.DisplayName AS AuthorDisplayName, RP.Score, RP.CommentCount, UR.ReputationRank, COALESCE(PH.Comment, 'No History') AS LastHistoryAction
//     FROM RecentPosts RP JOIN Users U ON RP.OwnerUserId = U.Id JOIN UserRankings UR ON U.Id = UR.UserId
//     LEFT JOIN PostHistory PH ON RP.PostId = PH.PostId AND PH.CreationDate = (SELECT MAX(Ph.CreationDate) FROM PostHistory Ph WHERE Ph.PostId = RP.PostId))
// SELECT PD.PostId, PD.Title, PD.AuthorDisplayName, PD.Score, PD.CommentCount, PD.ReputationRank, PD.LastHistoryAction
// FROM PostDetails PD WHERE PD.Score > 0 ORDER BY PD.ReputationRank, PD.Score DESC LIMIT 10 OFFSET 0;
fn q3084(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let ur = rel(ranked(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let updown = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select(&db.vote.vote_type_id);
    let rp = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(updown.opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    let v = drain((&rp).filt(|a| a[1] - a[2] > 0).and(owner_user.select(&rank)).and(Ident::<Post>::new().and(&md).select(&at).opt()));
    let v = top_n(v, |&(p, ((a, (_, r)), h))| (r, Reverse(a[1] - a[2]), p, h), 10);
    rows(v.into_iter().map(|(p, ((a, (u, r)), h))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(a[1] - a[2]), V::I(a[0]), V::I(r)]);
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No History")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        SUM(COALESCE(b.Class, 0)) AS TotalBadges, COUNT(DISTINCT CASE WHEN b.Class = 1 THEN b.Id END) AS GoldBadges, COUNT(DISTINCT CASE WHEN b.Class = 2 THEN b.Id END) AS SilverBadges,
//        COUNT(DISTINCT CASE WHEN b.Class = 3 THEN b.Id END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopQuestions AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplayName, us.TotalScore, us.TotalViews, us.QuestionCount
//     FROM RankedPosts rp JOIN UserStats us ON rp.OwnerDisplayName = us.DisplayName WHERE rp.rn <= 3)
// SELECT tq.PostId, tq.Title, tq.CreationDate, tq.ViewCount, tq.Score, tq.OwnerDisplayName, tq.TotalScore, tq.TotalViews, tq.QuestionCount
// FROM TopQuestions tq ORDER BY tq.TotalScore DESC, tq.ViewCount DESC;
//
// The join is on DisplayName, so each top question meets every user of its owner's name. A tie on (Score, CreationDate) goes to the smaller post id.
fn q5636(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let q = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let us = db.user.group_by(Ident::<User>::new()).select(q().select(score.and(view_count.opt())).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (p, _)| match p {
        Some((s, w)) => [a[0] + s, a[1] + w.unwrap_or(0)],
        None => a,
    });
    let qc = db.user.group_by(Ident::<User>::new()).select(q().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain((&tp).select(owner_user.select(&db.user.display_name).select(&by_name).select((&us).and(&qc))));
    rows(v.into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// PostVoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT u.DisplayName, COUNT(DISTINCT rp.Id) AS QuestionCount, SUM(pvc.UpVotes) AS TotalUpVotes, SUM(pvc.DownVotes) AS TotalDownVotes,
//        CASE WHEN ub.GoldBadges IS NULL THEN 0 ELSE ub.GoldBadges END AS GoldBadges, CASE WHEN ub.SilverBadges IS NULL THEN 0 ELSE ub.SilverBadges END AS SilverBadges,
//        CASE WHEN ub.BronzeBadges IS NULL THEN 0 ELSE ub.BronzeBadges END AS BronzeBadges, MAX(rp.CreationDate) AS LatestPostDate
// FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN PostVoteCounts pvc ON rp.Id = pvc.PostId LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE u.Reputation > 1000 GROUP BY u.DisplayName, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges HAVING COUNT(DISTINCT rp.Id) > 5
// ORDER BY TotalUpVotes DESC, QuestionCount DESC LIMIT 10;
//
// Every joined row carries a distinct question (PostVoteCounts has one row per post, and two users own different posts), so COUNT(DISTINCT rp.Id) counts the rows with a question.
fn q761(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let pvc = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let rp = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1).and(score.gt(0))));
    let g = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by((&db.user.display_name).and((&ub).opt()))
        .select(rp.select(creation_date.and((&pvc).opt())).opt())
        .fold([0i64, 0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some((d, c)) => [a[0] + 1, a[1] + c.is_some() as i64, a[2] + c.map_or(0, |c| c[0]), a[3] + c.map_or(0, |c| c[1]), 0, a[5].max(d)],
            None => a,
        });
    let v = top_n(drain((&g).filt(|a| a[0] > 5)), |&((n, b), a)| (a[1] == 0, Reverse(a[2]), Reverse(a[0]), n, b), 10);
    rows(v.into_iter().map(|((n, b), a)| {
        let b = b.unwrap_or([0; 3]);
        row(vec![V::S(n), V::I(a[0]), nullable(a[2], a[1]), nullable(a[3], a[1]), V::I(b[0]), V::I(b[1]), V::I(b[2]), tmax(a[5])])
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
//        COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, LastPostDate, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats),
// ClosedPostStats AS (SELECT p.OwnerUserId, COUNT(ph.Id) AS TotalClosures, COUNT(DISTINCT p.Id) AS UniqueClosedPosts FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId IN (10, 11, 12) GROUP BY p.OwnerUserId)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalScore, tu.TotalViews, tu.LastPostDate, COALESCE(cps.TotalClosures, 0) AS TotalClosures,
//        COALESCE(cps.UniqueClosedPosts, 0) AS UniqueClosedPosts, CASE WHEN tu.TotalAnswers = 0 THEN 0 ELSE (tu.TotalScore * 1.0 / NULLIF(tu.TotalAnswers, 0)) END AS ScorePerAnswer
// FROM TopUsers tu LEFT JOIN ClosedPostStats cps ON tu.UserId = cps.OwnerUserId WHERE tu.ScoreRank <= 10 ORDER BY tu.TotalScore DESC, tu.TotalPosts DESC;
fn q872(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[4]), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 10])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let cl = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11, 12])));
    let tc = db.post.group_by(&db.post.owner_user).select(cl).fold(0i64, |n, _| n + 1);
    let uc = db.post.with(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11, 12])))).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&by_user).and((&tc).opt()).and((&uc).opt()));
    rows(v.into_iter().map(|(u, (((_, a), c), d))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6]), tmax(a[7]), V::I(c.unwrap_or(0)), V::I(d.unwrap_or(0))]);
        f.push(V::F(if a[3] == 0 { 0.0 } else { a[4] as f64 / a[3] as f64 }));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges, SUM(U.Reputation) AS TotalReputation FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT UBS.UserId, UBS.DisplayName, UBS.GoldBadges, UBS.SilverBadges, UBS.BronzeBadges, COALESCE(PS.TotalPosts, 0) AS TotalPosts,
//        COALESCE(PS.TotalViews, 0) AS TotalViews, UBS.TotalReputation,
//        CASE WHEN UBS.TotalReputation > 10000 THEN 'High' WHEN UBS.TotalReputation BETWEEN 5000 AND 10000 THEN 'Medium' ELSE 'Low' END AS ReputationLevel
//     FROM UserBadgeStats UBS LEFT JOIN PostStats PS ON UBS.UserId = PS.OwnerUserId)
// SELECT U.DisplayName, U.GoldBadges, U.SilverBadges, U.BronzeBadges, U.TotalPosts, U.TotalViews, U.TotalReputation, U.ReputationLevel,
//        RANK() OVER (ORDER BY U.TotalReputation DESC) AS ReputationRank
// FROM UserPerformance U WHERE U.TotalPosts > 5 ORDER BY U.TotalReputation DESC, U.GoldBadges DESC, U.SilverBadges DESC;
fn q4357(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, .. } = &db.post;
    let ubs = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (r, c)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + r]);
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(view_count.opt())
        .fold([0i64; 2], |a, w| [a[0] + 1, a[1] + w.unwrap_or(0)]);
    let v = ranked(drain((&ubs).and((&ps).filt(|a| a[0] > 5))), |&(_, (b, _))| Reverse(b[3]), false);
    rows(v.into_iter().map(|((u, (b, p)), r)| {
        let mut f = vec![user_col(db, u, "name"), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(p[0]), V::I(p[1]), V::I(b[3])];
        f.push(V::S(if b[3] > 10000 { "High" } else if b[3] >= 5000 { "Medium" } else { "Low" }));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankByScore, COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPostsByUser
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// RecentActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Comments c ON c.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// TopPostsWithComments AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, ra.CommentCount, rp.OwnerUserId FROM RankedPosts rp LEFT JOIN RecentActivity ra ON ra.UserId = rp.OwnerUserId
//     WHERE rp.RankByScore <= 5)
// SELECT tp.Title, tp.Score, tp.CreationDate, COALESCE(ra.DisplayName, 'Anonymous') AS OwnerDisplayName, tp.CommentCount,
//        CASE WHEN tp.Score > 10 THEN 'High Score' WHEN tp.Score BETWEEN 5 AND 10 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory
// FROM TopPostsWithComments tp LEFT OUTER JOIN RecentActivity ra ON tp.OwnerUserId = ra.UserId ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// RankByScore reads only base columns, so the top posts are picked first and the comments x votes product of RecentActivity is driven only for their owners.
fn q30026(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ra = (&owners).group_by(Ident::<User>::new()).select(comments_by(db).opt().and(votes_by(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ra)).opt()));
    rows(v.into_iter().map(|(p, u)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "score", "created"]);
        match u {
            Some((u, n)) => f.extend([user_col(db, u, "name"), V::I(n)]),
            None => f.extend([V::S("Anonymous"), V::Null]),
        }
        f.push(V::S(if s > 10 { "High Score" } else if s >= 5 { "Moderate Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, ph.Comment, ph.Text, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6)),
// RecentPosts AS (SELECT p.Id AS PostId, p.CreationDate, p.Title, p.Body, p.ViewCount, p.OwnerUserId, p.AcceptedAnswerId, p.AnswerCount, COALESCE(MAX(rph.CreationDate), p.CreationDate) AS LastEditDate
//     FROM Posts p LEFT JOIN RecursivePostHistory rph ON p.Id = rph.PostId WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'
//     GROUP BY p.Id, p.CreationDate, p.Title, p.Body, p.ViewCount, p.OwnerUserId, p.AcceptedAnswerId, p.AnswerCount),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation FROM Users u ORDER BY u.Reputation DESC LIMIT 10),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.LastEditDate, u.DisplayName AS OwnerName, u.Reputation, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 2), 0) AS UpVotes,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 3), 0) AS DownVotes
//     FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN TopUsers tu ON u.Id = tu.UserId WHERE tu.UserId IS NOT NULL)
// SELECT ps.PostId, ps.Title, ps.ViewCount, ps.LastEditDate, ps.OwnerName, ps.Reputation, ps.CommentCount, ps.UpVotes, ps.DownVotes
// FROM PostStatistics ps ORDER BY ps.ViewCount DESC, ps.LastEditDate DESC;
fn q30254(db: &'static So) -> String {
    let now = now_utc();
    let since = ny_to_utc(add_days(utc_to_ny(now), -30));
    let Post { creation_date, owner_user, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))).select(&db.post_history.creation_date);
    let rp = db
        .post
        .with(creation_date.filt(move |d| ny_to_utc(d) >= since))
        .with(owner_user.select(Ident::<User>::new().with(&tu)))
        .group_by(Ident::<Post>::new())
        .select(creation_date.and(edits.opt()))
        .fold(i64::MIN, |m, (c, e)| m.max(e.unwrap_or(c)));
    let cc = db.post.with(&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = db.post.with(&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&rp).and(&cc).and(&vc).and(owner_user));
    rows(v.into_iter().map(|(p, (((d, c), a), u))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::T(d), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(c.Id) DESC) AS RankByComments
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadges, COALESCE(SUM(rp.CommentCount), 0) AS TotalComments,
//        COALESCE(SUM(rp.UpVoteCount) - SUM(rp.DownVoteCount), 0) AS VoteScore
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalBadges, TotalComments, VoteScore, ROW_NUMBER() OVER (ORDER BY TotalComments DESC, VoteScore DESC) AS UserRank
//     FROM UserStats WHERE TotalComments > 10 AND Reputation > 100)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalBadges, tu.TotalComments, tu.VoteScore, tu.UserRank FROM TopUsers tu WHERE tu.UserRank <= 10 ORDER BY tu.UserRank;
fn q33034(db: &'static So) -> String {
    let rp = db
        .post
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let us = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(&rp).opt()))
        .fold([0i64; 5], |a, (c, r)| match r {
            Some(r) => [a[0] + c.unwrap_or(0), a[1] + r[0], a[2] + r[1], a[3] + r[2], a[4] + 1],
            None => [a[0] + c.unwrap_or(0), a[1], a[2], a[3], a[4]],
        });
    let v = top_n(drain((&us).filt(|a| a[1] > 10)), |&(u, a)| (Reverse(a[1]), Reverse(a[2] - a[3]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(if a[4] == 0 { 0 } else { a[2] - a[3] }), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(SUM(c.Score), 0) AS TotalCommentScore FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount)
// SELECT ua.DisplayName, ua.PostCount, rp.PostId, rp.Title, rp.Score, rp.ViewCount, pd.TotalCommentScore, CASE WHEN pd.TotalCommentScore > 0 THEN 'Active' ELSE 'Inactive' END AS PostActivityStatus,
//        CASE WHEN ua.TotalBountyAmount > 100 THEN 'High Bounty Buyer' WHEN ua.TotalBountyAmount BETWEEN 50 AND 100 THEN 'Moderate Bounty Buyer' ELSE 'Low Bounty Buyer' END AS BountyBuyingStatus
// FROM UserActivity ua INNER JOIN RankedPosts rp ON ua.UserId = rp.PostId INNER JOIN PostDetails pd ON rp.PostId = pd.PostId WHERE rp.rn = 1
// ORDER BY ua.PostCount DESC, pd.TotalCommentScore DESC;
//
// `ua.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids; UserActivity is driven only for the users that join.
// rn partitions by the raw OwnerUserId (NULL included); a tie on CreationDate goes to the smaller post id.
fn q1616(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user_id.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let users: MatSet<Id<User>> = (&tp).select(origid.select(&by_raw)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ua = (&users).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(b) => [a[0] + 1, a[1] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let pd = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold(0i64, |n, s| n + s.unwrap_or(0));
    let v = drain((&tp).select(origid.select(&by_raw).select(Ident::<User>::new().and(&ua))).and(&pd));
    rows(v.into_iter().map(|(p, ((u, a), s))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0])];
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(s), V::S(if s > 0 { "Active" } else { "Inactive" })]);
        f.push(V::S(if a[1] > 100 { "High Bounty Buyer" } else if a[1] >= 50 { "Moderate Bounty Buyer" } else { "Low Bounty Buyer" }));
        row(f)
    }))
}

// WITH UserPerformance AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers, SUM(V.BountyAmount) AS TotalBountyEarned, AVG(CASE WHEN V.VoteTypeId = 2 THEN V.BountyAmount END) AS AvgUpvotes,
//        AVG(CASE WHEN V.VoteTypeId = 3 THEN V.BountyAmount END) AS AvgDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ClosedPosts AS (SELECT PH.UserId, COUNT(PH.PostId) AS ClosedCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.UserId),
// TopUsers AS (SELECT UP.UserId, UP.DisplayName, UP.Reputation, UP.TotalPosts, UP.TotalQuestions, UP.TotalAnswers, COALESCE(CP.ClosedCount, 0) AS ClosedCount,
//        ROW_NUMBER() OVER (ORDER BY UP.Reputation DESC) AS Rank FROM UserPerformance UP LEFT JOIN ClosedPosts CP ON UP.UserId = CP.UserId)
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.ClosedCount, TU.Reputation,
//        CASE WHEN TU.Reputation >= 10000 THEN 'Gold' WHEN TU.Reputation >= 5000 THEN 'Silver' WHEN TU.Reputation >= 1000 THEN 'Bronze' ELSE 'New User' END AS BadgeRank
// FROM TopUsers TU WHERE TU.Rank <= 10 ORDER BY TU.Reputation DESC;
//
// Only the COUNT(DISTINCT)s of UserPerformance are read, one row per post each; Rank reads only Reputation, so the ten users are picked first.
fn q3236(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let cp = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&pc).and((&cp).opt()));
    rows(v.into_iter().map(|(u, (a, c))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c.unwrap_or(0)), V::I(r)];
        f.push(V::S(if r >= 10000 { "Gold" } else if r >= 5000 { "Silver" } else if r >= 1000 { "Bronze" } else { "New User" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 GROUP BY u.Id, u.DisplayName, u.Reputation),
// BadgeCounts AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges GROUP BY UserId),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.QuestionCount, us.AnswerCount, us.WikiCount, us.TotalBountyAmount, COALESCE(bc.GoldBadges, 0) AS GoldBadges,
//        COALESCE(bc.SilverBadges, 0) AS SilverBadges, COALESCE(bc.BronzeBadges, 0) AS BronzeBadges, ROW_NUMBER() OVER (ORDER BY us.Reputation DESC) AS Rank
//     FROM UserStats us LEFT JOIN BadgeCounts bc ON us.UserId = bc.UserId)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, WikiCount, TotalBountyAmount, GoldBadges, SilverBadges, BronzeBadges
// FROM TopUsers WHERE Rank <= 10 ORDER BY Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x bounty-vote product is driven for those alone.
fn q1480(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&us).and(&pc).and((&bc).opt()));
    rows(v.into_iter().map(|(u, ((a, n), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.PostRank <= 3),
// PostDetails AS (SELECT trp.PostId, trp.Title, trp.Score, trp.ViewCount, trp.AnswerCount, trp.CommentCount, trp.OwnerDisplayName,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(c.Id) AS TotalComments
//     FROM TopRankedPosts trp LEFT JOIN Votes v ON trp.PostId = v.PostId LEFT JOIN Comments c ON trp.PostId = c.PostId
//     GROUP BY trp.PostId, trp.Title, trp.Score, trp.ViewCount, trp.AnswerCount, trp.CommentCount, trp.OwnerDisplayName)
// SELECT pd.Title, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.OwnerDisplayName, pd.TotalUpVotes, pd.TotalDownVotes, pd.TotalComments
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC LIMIT 10;
//
// PostRank reads only Score, so each owner's top questions are picked first and the vote x comment product is driven for those alone.
fn q6834(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.and(score)));
    let v = per_group(ranked(v, |&(_, (u, s))| (u, Reverse(s)), true), |&(_, (u, _))| u);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().filter(|x| x.1 <= 3).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let v = top_n(drain(&s), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "score", "views", "answers", "comments", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotesCount,
//        COUNT(CASE WHEN V.VoteTypeId = 5 THEN 1 END) AS FavoriteVotesCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, COUNT(C) AS CommentCount, SUM(COALESCE(PH.PostHistoryTypeId, 0)) AS PostEditCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
//     WHERE P.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U WHERE U.Reputation > 1000)
// SELECT RP.PostId, RP.Title, RP.CreationDate, UVC.DisplayName AS PostOwner, UVC.UpVotesCount, UVC.DownVotesCount, UVC.FavoriteVotesCount, T.UserRank, RP.CommentCount, RP.PostEditCount
// FROM RecentPosts RP JOIN UserVoteCounts UVC ON RP.OwnerUserId = UVC.UserId JOIN TopUsers T ON RP.OwnerUserId = T.Id
// WHERE RP.CommentCount > 5 AND (UVC.UpVotesCount - UVC.DownVotesCount) > 10 ORDER BY RP.CreationDate DESC LIMIT 50;
//
// COUNT(C) counts the whole row of C, which an unmatched LEFT JOIN still has, so it is the joined-row count.
fn q374(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let recent_h = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).gt(add_days(t0, -30)))).select(&db.post_history.post_history_type_id);
    let rp = db
        .post
        .with(creation_date.gt(add_years(t0, -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(recent_h.opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + 1, a[1] + t.unwrap_or(0)]);
    let uvc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(5)) as i64]
    });
    let tr = rel(ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let v = drain((&rp).filt(|a| a[0] > 5).and(owner_user.select(Ident::<User>::new().and((&uvc).filt(|a| a[0] - a[1] > 10)).and(&rank))));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (a, ((u, c), (_, r))))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(user_col(db, u, "name"));
        f.extend(c.map(V::I));
        f.extend([V::I(r), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalScore, QuestionCount, AnswerCount, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM TopUsers),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount FROM Badges B JOIN Users U ON B.UserId = U.Id GROUP BY U.Id)
// SELECT AU.UserId, AU.DisplayName, AU.Reputation, AU.PostCount, AU.TotalScore, AU.QuestionCount, AU.AnswerCount, COALESCE(UB.BadgeCount, 0) AS BadgeCount,
//        COALESCE(UB.GoldBadgeCount, 0) AS GoldBadgeCount, COALESCE(UB.SilverBadgeCount, 0) AS SilverBadgeCount, COALESCE(UB.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM ActiveUsers AU LEFT JOIN UserBadges UB ON AU.UserId = UB.UserId WHERE AU.ScoreRank <= 10 OR AU.PostRank <= 10 ORDER BY AU.TotalScore DESC, AU.PostCount DESC;
fn q5401(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[4]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[1]), false);
    let tu = rel(drain(rel(v).filt(|((_, s), p)| s <= 10 || p <= 10)).into_iter().map(|x| x.1 .0 .0).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 10])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&by_user).and((&ub).opt()));
    rows(v.into_iter().map(|(u, ((_, a), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[4]), V::I(a[2]), V::I(a[3])]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS Wikis, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties,
//        COALESCE(AVG(V.BountyAmount), 0) AS AverageBounty, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, Questions, Answers, Wikis, TotalBounties, AverageBounty, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserStatistics),
// UserComments AS (SELECT C.UserId, COUNT(C.Id) AS CommentCount, COUNT(DISTINCT C.PostId) AS UniquePostComments FROM Comments C GROUP BY C.UserId)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.Questions, TU.Answers, TU.Wikis, TU.TotalBounties, TU.AverageBounty, TU.BadgeCount, COALESCE(UC.CommentCount, 0) AS CommentCount,
//        COALESCE(UC.UniquePostComments, 0) AS UniquePostComments
// FROM TopUsers TU LEFT JOIN UserComments UC ON TU.UserId = UC.UserId WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC;
//
// ReputationRank reads only Reputation, so the ten users are picked first and the posts x votes x badges product is driven for those alone.
fn q25176(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 3..=5) as i64, a[3] + b.unwrap_or(0), a[4] + b.is_some() as i64]
            }
            None => a,
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let uc = (&tu).group_by(Ident::<User>::new()).select(comments_by(db).select(&db.comment.post)).buf_fold(|v| (v.len() as i64, distinct_some(v.iter().map(|&p| Some(p)))));
    let v = drain((&s).and(&pc).and(&bc).and((&uc).opt()));
    rows(v.into_iter().map(|(u, (((a, n), b), c))| {
        let c = c.unwrap_or((0, 0));
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[4] == 0 { V::F(0.0) } else { avg(a[3], a[4]) }, V::I(b), V::I(c.0), V::I(c.1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY EXTRACT(YEAR FROM p.CreationDate) ORDER BY p.ViewCount DESC) AS YearlyRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.YearlyRank <= 5),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, ub.BadgeCount,
//        CASE WHEN tp.Score > 100 THEN 'High Score' WHEN tp.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM TopPosts tp JOIN Users u ON tp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = u.Id) LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// ORDER BY tp.ViewCount DESC, tp.CreationDate DESC;
//
// YearlyRank reads only base columns, so the top questions of each year are picked first and the comment x vote product is driven for those alone.
// The IN subquery is the post's owner. A ViewCount tie at the fifth place goes to the smaller post id.
fn q26933(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(creation_date.map(year)));
    let top = top_per(v, |&(_, y)| y, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.user).opt())).opt()))
        .buf_fold(|v| {
            let voters = |t: i64| distinct_some(v.iter().map(|&(_, x)| x.filter(|x| x.0 == t).and_then(|x| x.1)));
            [v.iter().filter(|x| x.0.is_some()).count() as i64, voters(2), voters(3)]
        });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&s).and(owner_user.select(&bc)));
    rows(v.into_iter().map(|(p, (a, b))| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend(a.map(V::I));
        f.push(V::I(b));
        f.push(V::S(if sc > 100 { "High Score" } else if sc >= 50 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id WHERE u.Reputation > 1000 GROUP BY u.Id, u.Reputation),
// TopContributors AS (SELECT ur.UserId, ur.Reputation, ur.PostCount, ur.TotalBounty, ROW_NUMBER() OVER (ORDER BY ur.Reputation DESC, ur.PostCount DESC) AS ContributorRank
//     FROM UserReputation ur WHERE ur.PostCount > 5)
// SELECT tp.UserId, tp.Reputation, tp.PostCount, tp.TotalBounty,
//        (SELECT p.Title FROM Posts p WHERE p.OwnerUserId = tp.UserId AND p.Score = (SELECT MAX(Score) FROM Posts WHERE OwnerUserId = tp.UserId)) AS TopPostTitle,
//        (SELECT p.Score FROM Posts p WHERE p.OwnerUserId = tp.UserId AND p.Score = (SELECT MAX(Score) FROM Posts WHERE OwnerUserId = tp.UserId)) AS TopPostScore,
//        (SELECT p.ViewCount FROM Posts p WHERE p.OwnerUserId = tp.UserId AND p.Score = (SELECT MAX(Score) FROM Posts WHERE OwnerUserId = tp.UserId)) AS TopPostViewCount
// FROM TopContributors tp WHERE tp.ContributorRank <= 10 ORDER BY tp.Reputation DESC, tp.PostCount DESC;
//
// The votes join matches a user's own votes on their own posts (own_votes). Each scalar subquery reads the user's top-scoring post; the ten users here each have one.
fn q1538(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let ur = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(own_votes(db).select((&db.vote.bounty_amount).opt()).opt()).opt())
        .fold(0i64, |n, b| n + b.flatten().flatten().unwrap_or(0));
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tc = top_n(drain((&pc).filt(|n| n > 5).and(&ur)), |&(u, (n, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u), 10);
    let tc = rel(tc);
    let by_user: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&tc).map(|(u, _)| u).inv().select(&tc).collect();
    let ms = db.post.group_by(owner_user).select(score).fold(i64::MIN, |m, s| m.max(s));
    let at: HashIdx<(Id<User>, i64), Id<Post>> = db.post.select(owner_user.and(score)).inv().collect();
    let tops = (&by_user).map(|(u, _)| u).select(Ident::<User>::new().and(&ms).select(&at).opt());
    let v = drain((&by_user).and(tops));
    rows(v.into_iter().map(|(u, ((_, (n, b)), p))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(n), V::I(b)];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score", "views"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, p.OwnerUserId,
//        u.DisplayName AS OwnerDisplayName FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerUserId, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.PostRank <= 5),
// CommentStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, AVG(c.Score) AS AvgCommentScore FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// FinalResults AS (SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, cs.CommentCount, cs.AvgCommentScore,
//        CASE WHEN cs.AvgCommentScore IS NULL THEN 'No Comments' WHEN cs.AvgCommentScore > 2 THEN 'Highly Engaged' ELSE 'Low Engagement' END AS EngagementLevel
//     FROM TopPosts tp LEFT JOIN CommentStats cs ON tp.Id = cs.PostId)
// SELECT fr.Title, fr.CreationDate, fr.Score, fr.ViewCount, fr.CommentCount, fr.AvgCommentScore, fr.EngagementLevel
// FROM FinalResults fr WHERE fr.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1) ORDER BY fr.Score DESC, fr.ViewCount DESC;
//
// A Score tie at an owner's fifth place goes to the smaller post id. `Score > AVG(Score)` is compared exactly, as `Score * n > sum`.
fn q4579(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let (sum, n) = db.post.with(post_type_id.eq(1)).select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let v = drain((&cs).and(score.filt(move |s| (s as i128) * (n as i128) > sum as i128)));
    rows(v.into_iter().map(|(p, (a, _))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(a[0]), avg(a[1], a[0])]);
        f.push(V::S(if a[0] == 0 { "No Comments" } else if a[1] as f64 / a[0] as f64 > 2.0 { "Highly Engaged" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptanceRate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.DisplayName, us.Reputation, us.TotalPosts, us.QuestionCount, us.AnswerCount, us.AcceptanceRate, rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes
// FROM UserStats us LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.rn = 1
// WHERE us.Reputation > 1000 AND (us.AcceptanceRate IS NULL OR us.AcceptanceRate > 0.5) ORDER BY us.Reputation DESC, rp.CreationDate DESC LIMIT 50;
//
// rn reads only base columns, so each owner's newest post is picked first and the comment x vote product is driven for those alone.
// A user with no posts still has one joined row, whose CASE is 0, so AcceptanceRate is never NULL.
fn q4820(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, accepted_answer_id, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + x.is_some() as i64, a[4] + 1],
            None => [a[0], a[1], a[2], a[3], a[4] + 1],
        });
    let us = (&us).filt(|a| (a[3] as f64 / a[4] as f64) > 0.5);
    let first = top_per(drain(db.post.select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = rel(first);
    let newest: HashIdx<Id<User>, Id<Post>> = (&first).map(|(_, u)| u).inv().select((&first).map(|(p, _)| p)).collect();
    let ph: MatSet<Id<Post>> = (&newest).map(|p| p).collect();
    let pc = (&ph)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&ph).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain(us.and((&newest).select(Ident::<Post>::new().and(&pc).and(&cc)).opt()));
    let v = top_n(v, |&(u, (_, p))| {
        let d = p.map(|((p, _), _)| creation_date.get(p).unwrap());
        (Reverse(db.user.reputation.get(u).unwrap()), d.is_none(), Reverse(d), u)
    }, 50);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4])]);
        match p {
            Some(((p, x), c)) => {
                f.extend(post_fields(db, p, &["id", "title", "created"]));
                f.extend([V::I(c), V::I(x[0]), V::I(x[1])]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserScore AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 1 THEN 1 ELSE 0 END), 0) AS AcceptedAnswers,
//        COUNT(DISTINCT p.Id) AS TotalPosts FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalUpvotes, TotalDownvotes, AcceptedAnswers, TotalPosts, RANK() OVER (ORDER BY TotalUpvotes - TotalDownvotes DESC) AS UserRank FROM UserScore),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        COALESCE(MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END), '1970-01-01') AS ClosedDate, (SELECT COUNT(*) FROM PostLinks pl WHERE pl.PostId = p.Id) AS RelatedLinks
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount)
// SELECT tu.UserId, tu.DisplayName, tu.TotalUpvotes, tu.TotalDownvotes, tu.AcceptedAnswers, ps.PostId, ps.Title, ps.ViewCount, ps.CommentCount, ps.ClosedDate, ps.RelatedLinks
// FROM TopUsers tu JOIN PostStatistics ps ON ps.ViewCount > 50 AND (tu.AcceptedAnswers > 0 OR ps.CommentCount > 10) WHERE tu.UserRank <= 10
// ORDER BY (tu.TotalUpvotes - tu.TotalDownvotes) DESC, ps.ViewCount DESC;
//
// The ON condition relates no key of the two sides, so it is a cross join filtered on both.
fn q1969(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 3], |a, v| {
            let t = v.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(1)) as i64]
        });
    let v = ranked(drain(&us), |&(_, a)| Reverse(a[0] - a[1]), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ps = db
        .post
        .with((&db.post.view_count).gt(50))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(post_history_type_id.and(hd)).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, h)| (n + c.is_some() as i64, match h {
            Some((10, d)) => m.max(d),
            _ => m,
        }));
    let lc = db.post.with((&db.post.view_count).gt(50)).group_by(Ident::<Post>::new()).select(links_of(db).opt()).fold(0i64, |n, l| n + l.is_some() as i64);
    let psl = rel(drain((&ps).and(&lc)));
    let mut v = Vec::new();
    (&tu).cross(&psl).filt(|((_, a), (_, ((n, _), _)))| a[2] > 0 || n > 10).drive(|_, ((u, a), (p, ((n, m), l)))| v.push((u, a, p, n, m, l)));
    rows(v.into_iter().map(|(u, a, p, n, m, l)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["id", "title", "views"]));
        f.extend([V::I(n), V::T(if m == i64::MIN { 0 } else { m }), V::I(l)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties, COUNT(DISTINCT B.Id) AS TotalBadges,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.AnswerCount, ROW_NUMBER() OVER (ORDER BY P.Score DESC, P.ViewCount DESC) AS PopularityRank
//     FROM Posts P WHERE P.CreationDate > (CURRENT_TIMESTAMP - INTERVAL '1 year')),
// PostWithComments AS (SELECT P.Id AS PostId, P.Title, COUNT(C.Id) AS CommentCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title),
// TopPosts AS (SELECT PP.PostId, PP.Title, PP.Score, PP.ViewCount, PP.AnswerCount, PC.CommentCount FROM PopularPosts PP JOIN PostWithComments PC ON PP.PostId = PC.PostId WHERE PP.PopularityRank <= 10)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.TotalBounties, U.TotalBadges, TP.Title AS TopPostTitle, TP.Score AS PostScore, TP.ViewCount AS PostViewCount,
//        TP.AnswerCount AS PostAnswerCount, TP.CommentCount AS PostCommentCount
// FROM UserStats U LEFT JOIN TopPosts TP ON U.UserId = TP.PostId WHERE U.Reputation > 1000 ORDER BY U.Reputation DESC, TP.Score DESC;
//
// `U.UserId = TP.PostId` joins a user id to a post id, so it goes through the raw ids. UserRank is never read; the WHERE only keeps whole users, so
// the votes x badges product is driven for the users it keeps.
fn q2089(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let now = now_utc();
    let since = ny_to_utc(add_years(utc_to_ny(now), -1));
    let v = drain(db.post.with(creation_date.filt(move |d| ny_to_utc(d) > since)).select(score));
    let tp = top_n(v, |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(tp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let by_raw: HashIdx<i64, Id<Post>> = (&tp).select(&db.post.origid).inv().collect();
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let us = users()
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()))
        .fold(0i64, |n, (b, _)| n + b.flatten().unwrap_or(0));
    let bc = users().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&us).and(&bc).and((&db.user.origid).select(&by_raw).select(Ident::<Post>::new().and(&pc)).opt()));
    rows(v.into_iter().map(|(u, ((b, n), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(n)]);
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["title", "score", "views", "answers"]));
                f.push(V::I(c));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
//        COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate),
// TopUsers AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, COALESCE(SUM(rp.CommentCount), 0) AS TotalComments, COALESCE(SUM(rp.UpVoteCount), 0) AS TotalUpVotes,
//        COALESCE(SUM(rp.DownVoteCount), 0) AS TotalDownVotes FROM UserReputation ur LEFT JOIN RecentPosts rp ON ur.UserId = rp.OwnerUserId WHERE ur.Rank <= 10
//     GROUP BY ur.UserId, ur.DisplayName, ur.Reputation)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalComments, tu.TotalUpVotes, tu.TotalDownVotes,
//        CASE WHEN tu.Reputation > 1000 THEN 'High Reputation' WHEN tu.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM TopUsers tu ORDER BY tu.Reputation DESC;
fn q2879(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let rp = db
        .post
        .with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let s = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&rp).opt()).fold([0i64; 3], |a, r| match r {
        Some(r) => [a[0] + r[0], a[1] + r[1], a[2] + r[2]],
        None => a,
    });
    rows(drain(&s).into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostsWithStats AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE(NULLIF(p.AcceptedAnswerId, -1), 0) AS AcceptedAnswerId,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS EditCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (4, 5, 6)
//     GROUP BY p.Id, p.Title, p.Score, p.AcceptedAnswerId),
// TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, ub.BadgeCount, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u JOIN UserBadgeCounts ub ON u.Id = ub.UserId)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, ph.PostId, ph.Title, ph.Score, ph.AcceptedAnswerId, ph.UpVotes, ph.DownVotes, ph.CommentCount, ph.EditCount,
//        CASE WHEN ph.Score > 0 THEN 'Positive' WHEN ph.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM PostsWithStats ph JOIN TopUsers tu ON ph.AcceptedAnswerId = tu.Id WHERE (ph.UpVotes - ph.DownVotes) > 10 ORDER BY tu.Rank ASC, ph.Score DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// WITH RECURSIVE, but no CTE refers to itself. `ph.AcceptedAnswerId = tu.Id` joins a post id to a user id, so it goes through the raw ids;
// the vote x comment x edit product is driven for the posts whose id matches a user. Rank numbers users by Reputation; a tie goes to the smaller id.
fn q31356(db: &'static So) -> String {
    let rk = rel(top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 0).into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let by_raw: HashIdx<i64, (Id<User>, i64)> = (&rk).map(|(u, _)| u).select(&db.user.origid).inv().select(&rk).collect();
    let acc = || (&db.post.accepted_answer_id).opt().map(|a: Option<i64>| match a {
        Some(-1) | None => 0,
        Some(x) => x,
    });
    let matched: MatSet<Id<Post>> = db.post.with(acc().select(&by_raw)).collect();
    let edits = || history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let s = (&matched)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(edits().opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&matched).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ec = (&matched).group_by(Ident::<Post>::new()).select(edits().opt()).fold(0i64, |n, e| n + e.is_some() as i64);
    let v = drain((&s).filt(|a| a[0] - a[1] > 10).and(&cc).and(&ec).and(acc().select(&by_raw)));
    let v = top_n(v, |&(p, (_, (_, r)))| (r, Reverse(db.post.score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (((a, c), e), (u, r)))| {
        let s = db.post.score.get(p).unwrap();
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["id", "title", "score"]));
        f.extend([V::I(db.user.origid.get(u).unwrap()), V::I(a[0]), V::I(a[1]), V::I(c), V::I(e)]);
        f.push(V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score ELSE 0 END) AS TotalScore,
//        DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalScore, UserRank FROM UserStats WHERE UserRank <= 50),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// PostHistorySummary AS (SELECT ph.UserId, COUNT(*) AS EditCount, COUNT(DISTINCT ph.PostId) AS EditedPostCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.UserId)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.PostCount, u.QuestionCount, u.AnswerCount, u.TotalScore, COALESCE(b.BadgeCount, 0) AS BadgeCount, COALESCE(ph.EditCount, 0) AS EditCount,
//        COALESCE(ph.EditedPostCount, 0) AS EditedPostCount, ph.LastEditDate
// FROM TopUsers u LEFT JOIN UserBadges b ON u.UserId = b.UserId LEFT JOIN PostHistorySummary ph ON u.UserId = ph.UserId ORDER BY u.UserRank;
fn q6758(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 50).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if matches!(t, 1 | 2) { s } else { 0 }],
        None => a,
    });
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { user, post, creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(user).select(post.and(hd)).buf_fold(|v| {
        (v.len() as i64, distinct_some(v.iter().map(|x| Some(x.0))), v.iter().map(|x| x.1).max().unwrap())
    });
    let v = drain((&us).and((&bc).opt()).and((&ph).opt()));
    rows(v.into_iter().map(|(u, ((a, b), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::I(b.unwrap_or(0)));
        match h {
            Some((n, d, m)) => f.extend([V::I(n), V::I(d), V::T(m)]),
            None => f.extend([V::I(0), V::I(0), V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Tags, u.DisplayName AS Author, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Tags, rp.Author, ph.Comment AS LastEditComment, ph.CreationDate AS LastEditDate
//     FROM RankedPosts rp LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId
//     WHERE ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = rp.PostId) AND ph.PostHistoryTypeId IN (4, 5, 6)),
// VoteSummary AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// FinalResults AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Tags, pd.Author, pd.LastEditComment, pd.LastEditDate, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes
//     FROM PostDetails pd LEFT JOIN VoteSummary vs ON pd.PostId = vs.PostId)
// SELECT PostId, Title, CreationDate, Tags, Author, LastEditComment, LastEditDate, UpVotes, DownVotes FROM FinalResults WHERE UpVotes > DownVotes ORDER BY CreationDate DESC;
//
// The WHERE on ph makes the LEFT JOIN an inner join onto the post's latest history rows that are edits. Rank is never read.
fn q28938(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let PostHistory { post, creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).select(post.and(hd)).inv().collect();
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain(
        db.post
            .with(post_type_id.eq(1).and(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))
            .with(owner_user)
            .select(Ident::<Post>::new().and(&md).select(&at).and((&vs).filt(|a| a[0] > a[1]))),
    );
    rows(v.into_iter().map(|(p, (h, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "tags", "owner"]);
        f.extend([ostr(db.post_history.comment.get(h)), V::T(hd.get(h).unwrap()), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RecursiveUserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate, COUNT(DISTINCT p.Id) AS PostCount,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate),
// FilteredUserActivity AS (SELECT UserId, DisplayName, Reputation, CreationDate, LastAccessDate, PostCount, TotalViews, TotalBounties, PostRank FROM RecursiveUserActivity WHERE PostCount > 5),
// TopActiveUsers AS (SELECT UserId, DisplayName, Reputation, TotalViews, TotalBounties, PostRank, ROW_NUMBER() OVER (PARTITION BY Reputation ORDER BY TotalViews DESC) AS ViewsRank
//     FROM FilteredUserActivity WHERE Reputation >= 1000)
// SELECT u.UserId, u.DisplayName, u.Reputation, fa.PostCount, fa.TotalViews, fa.TotalBounties, CASE WHEN u.ViewsRank <= 5 THEN 'Top User' ELSE 'Regular User' END AS UserType
// FROM TopActiveUsers u JOIN FilteredUserActivity fa ON u.UserId = fa.UserId WHERE EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = u.UserId AND b.Class = 1)
// ORDER BY u.Reputation DESC, u.TotalViews DESC;
//
// Only users with Reputation >= 1000 reach the output and PostRank is never read, so the posts x bounty-vote product is driven for those alone.
// A TotalViews tie within a Reputation partition goes to the smaller user id.
fn q32086(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).ge(1000));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(bounty.opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((w, b)) => [a[0] + w.unwrap_or(0), a[1] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let fa = drain((&s).and((&pc).filt(|n| n > 5)));
    let r = top_per(fa.clone(), |&(u, _)| db.user.reputation.get(u).unwrap(), |&(u, (a, _))| (Reverse(a[0]), u), 5, false);
    let top: MatSet<Id<User>> = rel(r.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let v = drain(rel(fa).map(|(u, _)| u).select(Ident::<User>::new().with(&gold).and((&s).and(&pc)).and(Ident::<User>::new().with(&top).opt())));
    rows(v.into_iter().map(|(_, ((u, (a, n)), t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::S(if t.is_some() { "Top User" } else { "Regular User" })]);
        row(f)
    }))
}

// WITH UserScores AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, (U.UpVotes - U.DownVotes) AS Score, COUNT(DISTINCT P.Id) AS PostCount,
//        COUNT(DISTINCT B.Id) AS BadgeCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score AS PostScore, P.ViewCount, COALESCE(CNT.CommentsCount, 0) AS CommentsCount, COALESCE(AN.Count, 0) AS AnswerCount
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS CommentsCount FROM Comments GROUP BY PostId) CNT ON P.Id = CNT.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS Count FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) AN ON P.Id = AN.ParentId
//     WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY Score DESC) AS Rank FROM UserScores),
// TopPosts AS (SELECT *, RANK() OVER (ORDER BY PostScore DESC) AS Rank FROM PostStatistics)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.Score AS UserScore, P.PostId, P.Title, P.PostScore, P.ViewCount, P.CommentsCount, P.AnswerCount, U.Rank AS UserRank, P.Rank AS PostRank
// FROM TopUsers U JOIN TopPosts P ON U.UserId = P.PostId WHERE U.Rank <= 10 AND P.Rank <= 10 ORDER BY U.Rank, P.Rank;
//
// `U.UserId = P.PostId` joins a user id to a post id, so it goes through the raw ids. The ranks read only base columns.
fn q7793(db: &'static So) -> String {
    let User { up_votes, down_votes, origid, .. } = &db.user;
    let ur = ranked(drain(up_votes.and(down_votes)), |&(_, (a, b))| Reverse(a - b), false);
    let tu = rel(ur.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let by_raw: HashIdx<i64, (Id<User>, i64)> = (&tu).map(|(u, _)| u).select(origid).inv().select(&tu).collect();
    let Post { creation_date, score, .. } = &db.post;
    let pr = ranked(drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(_, s)| Reverse(s), false);
    let tp = rel(pr.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let tpk: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tp).map(|(p, _)| p).inv().select(&tp).collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let ac = db.post.with((&db.post.post_type_id).eq(2)).group_by(&db.post.parent).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tpk).and((&db.post.origid).select(&by_raw)).and((&cc).opt()).and((&ac).opt()));
    let mut v = v;
    v.sort_by_key(|&(p, ((((_, pr), (_, ur)), _), _))| (ur, pr, p));
    rows(v.into_iter().map(|(p, ((((_, pr), (u, ur)), c), a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(up_votes.get(u).unwrap() - down_votes.get(u).unwrap()));
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0)), V::I(ur), V::I(pr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.Tags, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, u.DisplayName AS OwnerName, ua.TotalBounties, ua.CommentCount, ua.BadgeCount,
//        RANK() OVER (ORDER BY rp.Score DESC) AS PostRank FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserActivity ua ON u.Id = ua.UserId WHERE rp.UserRank = 1)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.OwnerName, tp.TotalBounties, tp.CommentCount, tp.BadgeCount,
//        CASE WHEN tp.Score >= 100 THEN 'Hot Post' WHEN tp.Score BETWEEN 50 AND 99 THEN 'Trending Post' ELSE 'New Post' END AS PostCategory
// FROM TopPosts tp WHERE tp.PostRank <= 10 ORDER BY tp.Score DESC;
//
// Both ranks read only Score, so the ten top posts are picked first and the votes x comments x badges product of UserActivity is driven only for their owners.
// A Score tie between two of one owner's questions goes to the smaller post id.
fn q30354(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let fp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let r = ranked(drain((&fp).select(score)), |&(_, s)| Reverse(s), false);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(comments_by(db).opt()).and(badges_of(db).opt()))
        .fold([0i64; 3], |a, ((v, c), b)| [a[0] + v.flatten().unwrap_or(0), a[1] + c.is_some() as i64, a[2] + b.is_some() as i64]);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ua))));
    rows(v.into_iter().map(|(p, (u, a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        f.push(V::S(if s >= 100 { "Hot Post" } else if s >= 50 { "Trending Post" } else { "New Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.ViewCount > 100),
// UserActivity AS (SELECT u.Id AS UserId, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN p.ViewCount > 500 THEN 1 ELSE 0 END) AS HighViewCountPosts, COUNT(DISTINCT v.Id) AS VoteCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// ActiveUsers AS (SELECT ua.UserId, ua.BadgeCount, ua.HighViewCountPosts, ua.VoteCount, RANK() OVER (ORDER BY ua.VoteCount DESC, ua.BadgeCount DESC) AS UserRank
//     FROM UserActivity ua WHERE ua.HighViewCountPosts > 0)
// SELECT rp.Title, rp.CreationDate AS PostCreatedDate, rp.Score, rp.ViewCount, rp.AnswerCount, u.DisplayName AS AuthorDisplayName, ua.BadgeCount AS AuthorBadges,
//        ua.HighViewCountPosts AS AuthorHighViewPosts, ua.VoteCount AS AuthorVoteCount, au.UserRank AS AuthorRank
// FROM RankedPosts rp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) JOIN ActiveUsers au ON au.UserId = u.Id JOIN UserActivity ua ON ua.UserId = u.Id
// WHERE rp.Rank <= 5 ORDER BY rp.ViewCount DESC, rp.CreationDate DESC;
//
// Every post of a user appears in at least one row of its badges x posts x votes product, so HighViewCountPosts > 0 exactly when the user owns a post
// with ViewCount > 500; the ranking is taken over those users with their COUNT(DISTINCT)s, and the product itself is driven only for the authors printed.
fn q5850(db: &'static So) -> String {
    let Post { creation_date, view_count, score, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(view_count.gt(100))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let high: MatSet<Id<User>> = db.post.with(view_count.gt(500)).select(owner_user).collect();
    let bc = (&high).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let vc = (&high).group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let ar = rel(ranked(drain((&vc).and(&bc)), |&(_, (v, b))| (Reverse(v), Reverse(b)), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&ar).map(|(u, _)| u).inv().select(&ar).collect();
    let authors: MatSet<Id<User>> = (&tp).select(owner_user.select(Ident::<User>::new().with(&rank))).collect();
    let hv = (&authors)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(view_count.opt()).opt()).and(votes_by(db).opt()))
        .fold(0i64, |n, ((_, w), _)| n + (w.flatten().map_or(false, |w| w > 500)) as i64);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&hv).and(&bc).and(&vc).and(&rank))));
    rows(v.into_iter().map(|(p, ((((u, h), b), n), (_, r)))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers"]);
        f.extend([user_col(db, u, "name"), V::I(b), V::I(h), V::I(n), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, u.DisplayName AS Author, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostScores AS (SELECT PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes v GROUP BY PostId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.Author, ps.Upvotes, ps.Downvotes, COALESCE(CAST(o.ReportedCount AS INT), 0) AS ReportedCount,
//        CASE WHEN rp.Score > 100 THEN 'High Score' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
//     FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS ReportedCount FROM Comments c WHERE c.Text ILIKE '%spam%' GROUP BY PostId) o ON rp.PostId = o.PostId
//     JOIN PostScores ps ON rp.PostId = ps.PostId WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.Author, tp.Score, tp.Upvotes, tp.Downvotes, tp.ReportedCount, tp.ScoreCategory FROM TopPosts tp ORDER BY tp.Score DESC, tp.Upvotes DESC;
fn q30325(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let spam = db.comment.with((&db.comment.text).filt(|t: Str| t.to_lowercase().contains("spam"))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(Ident::<Post>::new().and(&ps).and((&spam).opt())));
    rows(v.into_iter().map(|(_, ((p, a), n))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n.unwrap_or(0))]);
        f.push(V::S(if s > 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, SUM(CASE WHEN v.VoteTypeId = 5 THEN 1 ELSE 0 END) AS Favorites, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, Upvotes, Downvotes, Favorites, Questions, Answers, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank, RANK() OVER (ORDER BY Upvotes DESC) AS VoteRank FROM UserActivity)
// SELECT UserId, DisplayName, TotalPosts, TotalComments, Upvotes, Downvotes, Favorites, Questions, Answers, GoldBadges, SilverBadges, BronzeBadges, PostRank, VoteRank
// FROM TopUsers WHERE PostRank <= 10 OR VoteRank <= 10 ORDER BY PostRank, VoteRank;
fn q8472(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 8], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |((t, _), v)| (t, v));
            [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (v == Some(5)) as i64, a[3] + (t == 1) as i64, a[4] + (t == 2) as i64, a[5] + (b == Some(1)) as i64, a[6] + (b == Some(2)) as i64, a[7] + (b == Some(3)) as i64]
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&s).and(&pc).and(&cc)), |&(_, ((_, n), _))| Reverse(n), false);
    let v = ranked(v, |&((_, ((a, _), _)), _)| Reverse(a[0]), false);
    let mut v: Vec<_> = drain(rel(v).filt(|((_, p), w)| p <= 10 || w <= 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(((u, _), p), w)| (p, w, u));
    rows(v.into_iter().map(|(((u, ((a, n), c)), p), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c)]);
        f.extend(a.map(V::I));
        f.extend([V::I(p), V::I(w)]);
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE U.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY U.Id, U.Reputation),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, PT.Name AS PostType, T.TagName FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id
//     LEFT JOIN Tags T ON P.Tags ILIKE '%' || T.TagName || '%' WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PD.PostId, PD.Title, PD.CreationDate, PD.ViewCount, PD.Score, PD.PostType, ROW_NUMBER() OVER (PARTITION BY PD.PostType ORDER BY PD.Score DESC) AS Rank FROM PostDetails PD)
// SELECT UM.UserId, UM.Reputation, UM.TotalPosts, UM.TotalComments, UM.TotalUpvotes, UM.TotalDownvotes, TP.Title AS TopPostTitle, TP.ViewCount AS TopPostViews, TP.Score AS TopPostScore
// FROM UserMetrics UM LEFT JOIN TopPosts TP ON UM.TotalPosts > 0 AND TP.Rank = 1 ORDER BY UM.Reputation DESC, UM.TotalPosts DESC;
//
// The Tags join only repeats a post once per matching tag, and Rank = 1 keeps one row per post type, whose projected columns are the post's, so it is not computed.
// A Score tie for a type's top post goes to the smaller post id. The ON condition names no key of TP, so it is a cross join onto the users with posts.
fn q6179(db: &'static So) -> String {
    let Post { creation_date, score, post_type, .. } = &db.post;
    let t = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let users = || db.user.with((&db.user.creation_date).ge(t));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(comments_by(db).opt()))
        .fold([0i64; 2], |a, (v, _)| {
            let v = v.flatten();
            [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64]
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = users().group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tp = top_per(drain(db.post.with(creation_date.ge(t)).select(post_type)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp = left_all(tp.into_iter().map(|x| x.0).collect());
    let mut v = Vec::new();
    (&s).and((&pc).filt(|n| n > 0)).and(&cc).cross(&tp).drive(|(u, _), (((a, n), c), p)| v.push((u, a, n, c, p)));
    (&s).and((&pc).filt(|n| n == 0)).and(&cc).drive(|u, ((a, n), c)| v.push((u, a, n, c, None)));
    rows(v.into_iter().map(|(u, a, n, c, p)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "views", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId IN (1, 2) THEN P.Score ELSE 0 END) AS TotalScore, SUM(U.UpVotes) AS TotalUpVotes,
//        SUM(U.DownVotes) AS TotalDownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation, U.CreationDate),
// ActiveUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, TotalScore, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserStatistics WHERE PostCount > 0),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, TotalScore FROM ActiveUsers WHERE Rank <= 10)
// SELECT U.DisplayName, U.Reputation AS UserReputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.TotalScore, COALESCE(C.CommentCount, 0) AS CommentCount, COALESCE(B.BadgeCount, 0) AS BadgeCount
// FROM TopUsers TU JOIN Users U ON TU.UserId = U.Id LEFT JOIN (SELECT UserId, COUNT(*) AS CommentCount FROM Comments GROUP BY UserId) C ON U.Id = C.UserId
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON U.Id = B.UserId ORDER BY TU.TotalScore DESC;
fn q7017(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score))).fold([0i64; 4], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if matches!(t, 1 | 2) { s } else { 0 }]
    });
    let tu = top_n(drain(&us), |&(u, a)| (Reverse(a[3]), u), 10);
    let tu = rel(tu);
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 4])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let cc = db.comment.group_by(&db.comment.user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&by_user).and((&cc).opt()).and((&bc).opt()));
    rows(v.into_iter().map(|(u, (((_, a), c), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(c.unwrap_or(0)), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(c.Id) DESC) AS RankByComments
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT up.DisplayName, up.Reputation, up.GoldBadges, up.SilverBadges, up.BronzeBadges, rp.Title, rp.CommentCount, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.CommentCount > 10 THEN 'Highly Engaged' WHEN rp.CommentCount BETWEEN 5 AND 10 THEN 'Moderately Engaged' ELSE 'Low Engagement' END AS EngagementLevel
// FROM UserStats up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId WHERE rp.RankByComments = 1 ORDER BY up.Reputation DESC, rp.CommentCount DESC LIMIT 50;
fn q510(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let top = top_per(drain((&rp).and(owner_user)), |&(_, (_, u))| u, |&(_, (a, _))| Reverse(a[0]), 1, true);
    let top = rel(top);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let v = drain((&top).select(Same::<(Id<Post>, ([i64; 3], Id<User>))>::new().and(Same::<(Id<Post>, ([i64; 3], Id<User>))>::new().map(|(_, (_, u))| u).select(&ub))));
    let v = top_n(v, |&(_, ((p, (a, u)), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), p), 50);
    rows(v.into_iter().map(|(_, ((p, (a, u)), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["title"]));
        f.extend(a.map(V::I));
        f.push(V::S(if a[0] > 10 { "Highly Engaged" } else if a[0] >= 5 { "Moderately Engaged" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        COUNT(DISTINCT p.Id) AS PostsCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, COUNT(DISTINCT ph.Id) AS CloseHistoryCount FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10 GROUP BY p.Id, p.Title),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, COALESCE(cp.CloseHistoryCount, 0) AS CloseCount, COUNT(c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBounty
//     FROM Posts p LEFT JOIN ClosedPosts cp ON p.Id = cp.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
//     GROUP BY p.Id, p.Title, cp.CloseHistoryCount),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.CloseCount, ps.CommentCount, ps.TotalBounty, ROW_NUMBER() OVER (PARTITION BY ps.CloseCount ORDER BY ps.CommentCount DESC, ps.TotalBounty DESC) AS Rank
//     FROM PostStatistics ps)
// SELECT ups.UserId, ups.DisplayName, rp.Title, rp.CloseCount, rp.CommentCount, rp.TotalBounty, ups.UpVotesCount, ups.DownVotesCount
// FROM UserVoteStats ups JOIN RankedPosts rp ON ups.PostsCount > 5 AND ups.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) WHERE rp.Rank <= 10
// ORDER BY ups.UpVotesCount DESC, rp.CommentCount DESC;
//
// A tie on (CommentCount, TotalBounty) at a partition's tenth place goes to the smaller post id.
fn q1274(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cp = db.post.group_by(Ident::<Post>::new()).select(closes).fold(0i64, |n, _| n + 1);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ps = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 3], |a, (c, b)| {
        let b = b.flatten();
        [a[0] + c.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + b.is_some() as i64]
    });
    let v = drain((&ps).and((&cp).opt()));
    let top = top_per(v, |&(_, (_, c))| c.unwrap_or(0), |&(p, (a, _))| (Reverse(a[0]), a[2] == 0, Reverse(a[1]), p), 10, false);
    let top = rel(top);
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).opt())).opt()).buf_fold(|v| {
        [
            v.iter().filter(|x| x.map(|y| y.0) == Some(2)).count() as i64,
            v.iter().filter(|x| x.map(|y| y.0) == Some(3)).count() as i64,
            distinct_some(v.iter().map(|x| x.and_then(|y| y.1))),
        ]
    });
    type T = (Id<Post>, ([i64; 3], Option<i64>));
    let v = drain((&top).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select(owner_user.select(Ident::<User>::new().and((&uvs).filt(|a| a[2] > 5)))))));
    rows(v.into_iter().map(|(_, ((p, (a, c)), (u, s)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(a[0]), nullable(a[1], a[2]), V::I(s[0]), V::I(s[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.Body, p.Tags, p.CreationDate, p.LastActivityDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.LastActivityDate, p.OwnerUserId, u.DisplayName),
// FilteredPosts AS (SELECT rp.*, ARRAY_TO_STRING(string_to_array(rp.Tags, '<>'), ', ') AS FormattedTags FROM RankedPosts rp WHERE rp.PostRank <= 5),
// UserStats AS (SELECT u.Id AS UserID, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(p.ViewCount) AS TotalViews FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id
//     LEFT JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.Reputation)
// SELECT fp.OwnerDisplayName, fp.Title, fp.FormattedTags, fp.CreationDate, fp.LastActivityDate, us.Reputation, us.BadgeCount, us.TotalViews, fp.AnswerCount, fp.UpVotes, fp.DownVotes
// FROM FilteredPosts fp JOIN UserStats us ON fp.OwnerUserId = us.UserID ORDER BY fp.CreationDate DESC;
//
// PostRank reads only base columns, so each owner's five newest questions are picked first and the answer x vote product is driven for those alone;
// UserStats' badges x posts product is driven for their owners.
fn q28592(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt().and(posts_of(db).select(view_count.opt()).opt())).fold([0i64; 2], |a, (_, w)| {
        let w = w.flatten();
        [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0)]
    });
    let bc = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&s).and(owner_user.select(Ident::<User>::new().and(&us).and(&bc))));
    rows(v.into_iter().map(|(p, (a, ((u, t), b)))| {
        let mut f = post_fields(db, p, &["owner", "title"]);
        f.push(match tags_str.get(p) {
            Some(t) => V::Owned(t.split("<>").collect::<Vec<_>>().join(", ")),
            None => V::Null,
        });
        f.extend(post_fields(db, p, &["created", "activity"]));
        f.extend([user_col(db, u, "rep"), V::I(b), nullable(t[1], t[0])]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostEngagement AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(v.UpVotes, 0) AS UpVotes,
//        COALESCE(v.DownVotes, 0) AS DownVotes FROM TopPosts tp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON tp.PostId = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON tp.PostId = v.PostId)
// SELECT pe.Title, pe.OwnerDisplayName, pe.CreationDate, pe.Score, pe.ViewCount, pe.CommentCount, pe.UpVotes, pe.DownVotes FROM PostEngagement pe ORDER BY pe.Score DESC, pe.ViewCount DESC;
fn q6568(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(&vc));
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostsCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
//        SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS UpVotesCount, SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS DownVotesCount,
//        COUNT(CASE WHEN B.Id IS NOT NULL THEN 1 END) AS BadgesCount, AVG(U.Reputation) AS AvgReputation, MIN(P.CreationDate) AS FirstPostDate, MAX(P.LastActivityDate) AS LastActivityDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id LEFT JOIN Badges B ON U.Id = B.UserId
//     WHERE U.CreationDate > '2020-01-01' GROUP BY U.Id, U.DisplayName),
// AggregatedData AS (SELECT UserId, DisplayName, PostsCount, PositivePosts, NegativePosts, UpVotesCount, DownVotesCount, BadgesCount, AvgReputation,
//        DENSE_RANK() OVER (ORDER BY PostsCount DESC) AS PostRank, DENSE_RANK() OVER (ORDER BY UpVotesCount DESC) AS UpVotesRank FROM UserActivity),
// FinalResults AS (SELECT UserId, DisplayName, PostsCount, PositivePosts, NegativePosts, UpVotesCount, DownVotesCount, BadgesCount, AvgReputation, PostRank, UpVotesRank
//     FROM AggregatedData WHERE PostsCount > 10)
// SELECT * FROM FinalResults ORDER BY PostRank, UpVotesRank;
//
// The votes join matches a user's own votes on their own posts (own_votes).
fn q9173(db: &'static So) -> String {
    let s = db
        .user
        .with((&db.user.creation_date).gt(ts(2020, 1, 1, 0, 0, 0)))
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select((&db.post.score).and(own_votes(db).select(&db.vote.vote_type_id).opt())).opt()).and(badges_of(db).opt()))
        .fold([0i64; 8], |a, ((r, p), b)| {
            let (n, s, t) = p.map_or((0, 0, None), |(s, t)| (1, s, t));
            [a[0] + n, a[1] + (n == 1 && s > 0) as i64, a[2] + (n == 1 && s < 0) as i64, a[3] + (t == Some(2)) as i64, a[4] + (t == Some(3)) as i64, a[5] + b.is_some() as i64, a[6] + r, a[7] + 1]
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), true);
    let v = ranked(v, |&((_, a), _)| Reverse(a[3]), true);
    let v: Vec<_> = drain(rel(v).filt(|(((_, a), _), _)| a[0] > 10)).into_iter().map(|x| x.1).collect();
    rows(v.into_iter().map(|(((u, a), pr), ur)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a[..6].iter().map(|&x| V::I(x)));
        f.extend([avg(a[6], a[7]), V::I(pr), V::I(ur)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerName, rp.Score, rp.ViewCount, rp.CommentCount, rp.BadgeCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.OwnerName, tp.Score, tp.ViewCount, CASE WHEN tp.BadgeCount > 0 THEN 'Active User' ELSE 'Newcomer' END AS UserCategory,
//        CASE WHEN tp.CommentCount > 10 THEN 'Highly Discussed' ELSE 'Less Discussed' END AS DiscussionLevel
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = tp.PostId)
// WHERE ph.PostHistoryTypeId = 10 ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// The WHERE on ph makes the LEFT JOIN an inner join onto the post's latest history rows that are closes. A Score tie at the fifth place goes to the smaller post id.
fn q310(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(post.and(hd)).inv().collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&cc).and(Ident::<Post>::new().and(&md).select(&at)).and(owner_user.select(&bc).opt()));
    rows(v.into_iter().map(|(p, ((c, _), b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.push(V::S(if b.unwrap_or(0) > 0 { "Active User" } else { "Newcomer" }));
        f.push(V::S(if c > 10 { "Highly Discussed" } else { "Less Discussed" }));
        row(f)
    }))
}

// WITH PostScores AS (SELECT p.Id AS PostId, p.Title, p.Score, p.OwnerUserId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS HistoryCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, p.Score, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS MaxBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.Score + ps.UpVotes - ps.DownVotes AS NetScore, ROW_NUMBER() OVER (PARTITION BY ps.OwnerUserId ORDER BY ps.Score DESC) AS RowNum,
//        ub.BadgeCount, ub.MaxBadgeClass FROM PostScores ps JOIN UserBadges ub ON ps.OwnerUserId = ub.UserId WHERE ps.Score + ps.UpVotes - ps.DownVotes > 10)
// SELECT tp.PostId, tp.Title, tp.NetScore, tp.BadgeCount,
//        CASE WHEN tp.MaxBadgeClass = 1 THEN 'Gold' WHEN tp.MaxBadgeClass = 2 THEN 'Silver' WHEN tp.MaxBadgeClass = 3 THEN 'Bronze' ELSE 'No Badges' END AS HighestBadge,
//        CASE WHEN tp.RowNum <= 5 THEN 'Top' ELSE 'Others' END AS PostRank
// FROM TopPosts tp WHERE tp.BadgeCount > 0 ORDER BY tp.NetScore DESC, tp.Title ASC LIMIT 50;
//
// A Score tie at an owner's fifth place goes to the smaller post id.
fn q1585(db: &'static So) -> String {
    let Post { owner_user, score, title, .. } = &db.post;
    let ps = db
        .post
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let tp = drain((&ps).and(score).filt(|(a, s): ([i64; 2], i64)| s + a[0] - a[1] > 10).and(owner_user));
    let top = top_per(tp.clone(), |&(_, (_, u))| u, |&(p, ((_, s), _))| (Reverse(s), p), 5, false);
    let top: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let v = drain(rel(tp).map(|(p, _)| p).select(Ident::<Post>::new().and((&ps).and(score)).and(owner_user.select(&ub)).and(Ident::<Post>::new().with(&top).opt())));
    let v = top_n(v, |&(_, (((p, (a, s)), _), _))| (Reverse(s + a[0] - a[1]), title.get(p).is_none(), title.get(p), p), 50);
    rows(v.into_iter().map(|(_, (((p, (a, s)), (n, m)), t))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s + a[0] - a[1]), V::I(n)]);
        f.push(V::S(match m {
            1 => "Gold",
            2 => "Silver",
            3 => "Bronze",
            _ => "No Badges",
        }));
        f.push(V::S(if t.is_some() { "Top" } else { "Others" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.PostTypeId, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        COALESCE(v_up.VoteCount, 0) AS UpVotes, COALESCE(v_down.VoteCount, 0) AS DownVotes
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) v_up ON p.Id = v_up.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) v_down ON p.Id = v_down.PostId),
// RecentComments AS (SELECT c.PostId, COUNT(*) AS CommentCount FROM Comments c WHERE c.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY c.PostId),
// PostDetails AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.ViewCount, rp.UpVotes, rp.DownVotes, rc.CommentCount, (rp.UpVotes - rp.DownVotes) AS NetScore,
//        CASE WHEN rp.ViewCount > 1000 THEN 'Highly Viewed' WHEN rp.ViewCount BETWEEN 500 AND 1000 THEN 'Moderately Viewed' ELSE 'Low Engagement' END AS EngagementLevel
//     FROM RankedPosts rp LEFT JOIN RecentComments rc ON rp.Id = rc.PostId WHERE rp.rn <= 10)
// SELECT pd.Title, pd.CreationDate, pd.ViewCount, pd.UpVotes, pd.DownVotes, pd.CommentCount, pd.NetScore, pd.EngagementLevel FROM PostDetails pd WHERE pd.NetScore > 0
// ORDER BY pd.NetScore DESC, pd.CreationDate DESC LIMIT 20;
fn q4428(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let rc = db.comment.with((&db.comment.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&vc).filt(|a| a[0] - a[1] > 0).and((&rc).opt()));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[0] - a[1]), Reverse(creation_date.get(p).unwrap()), p), 20);
    rows(v.into_iter().map(|(p, (a, c))| {
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), oint(c), V::I(a[0] - a[1])]);
        f.push(V::S(match w {
            Some(w) if w > 1000 => "Highly Viewed",
            Some(w) if w >= 500 => "Moderately Viewed",
            _ => "Low Engagement",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId),
// ClosedPosts AS (SELECT ph.PostId, ph.UserId, COUNT(*) AS CloseReasonCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.UserId),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes, COALESCE(cp.CloseReasonCount, 0) AS CloseReasonCount,
//        CASE WHEN rp.Score IS NULL THEN 'Score Not Calculated' WHEN rp.Score >= 100 THEN 'High Score' WHEN rp.Score > 0 AND rp.Score < 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
//     FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT pm.PostId, pm.Title, pm.CreationDate, pm.Score, pm.CommentCount, pm.UpVotes, pm.DownVotes, pm.CloseReasonCount, pm.ScoreCategory
// FROM PostMetrics pm WHERE pm.CloseReasonCount > 0 OR pm.ScoreCategory = 'High Score' ORDER BY pm.Score DESC, pm.CommentCount DESC;
//
// A row survives when it joined a ClosedPosts group (every group counts at least one close) or its post scores 100 or more, so only those posts' joined
// rows are driven; the per-post windows are a fold over the same rows. ScoreRank is never read.
fn q22577(db: &'static So) -> String {
    let Post { score, .. } = &db.post;
    let PostHistory { post, user, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post.and(user.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cv = rel(drain(&cp));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, Option<Id<User>>), i64)> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let keep: MatSet<Id<Post>> = db.post.with(score.ge(100)).union(db.post.with(&by_post)).collect();
    let prod = || comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt());
    let w = (&keep).group_by(Ident::<Post>::new()).select(prod()).fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = Vec::new();
    (&keep).select(prod().and(&w).and((&by_post).map(|(_, n)| n).opt())).drive(|p, ((_, a), n)| v.push((p, a, n)));
    rows(v.into_iter().map(|(p, a, n)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(a.map(V::I));
        f.push(V::I(n.unwrap_or(0)));
        f.push(V::S(if s >= 100 { "High Score" } else if s > 0 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RECURSIVE UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, UserRank FROM UserPostStats WHERE PostCount > 10 ORDER BY UserRank LIMIT 10),
// PostVoteStats AS (SELECT P.OwnerUserId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY P.OwnerUserId),
// TopPostVoters AS (SELECT U.Id, U.DisplayName, COALESCE(V.VoteCount, 0) AS TotalVotes, COALESCE(V.UpVotes, 0) AS UpVoteCount, COALESCE(V.DownVotes, 0) AS DownVoteCount
//     FROM Users U LEFT JOIN PostVoteStats V ON U.Id = V.OwnerUserId WHERE COALESCE(V.VoteCount, 0) > 5)
// SELECT TU.DisplayName AS TopUser, TU.QuestionCount, TU.AnswerCount, TU.TotalViews, TPV.TotalVotes, TPV.UpVoteCount, TPV.DownVoteCount
// FROM TopUsers TU LEFT JOIN TopPostVoters TPV ON TU.UserId = TPV.Id ORDER BY TU.UserRank, TPV.TotalVotes DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q31093(db: &'static So) -> String {
    let tu = top_n(drain((&user_posts(db)).filt(|a| a[1] > 10)), |&(u, a)| (Reverse(a[1]), u), 10);
    let tu = rel(tu);
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 10])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let pv = db
        .post
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(&db.post.owner_user)
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&by_user).and((&pv).filt(|a| a[0] > 5).opt()));
    rows(v.into_iter().map(|(u, ((_, a), t))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[2]), V::I(a[3]), V::I(a[6])];
        f.extend(match t {
            Some(t) => t.iter().map(|&x| V::I(x)).collect(),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.PostTypeId = 10 THEN 1 ELSE 0 END) AS TotalClosedPosts,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS TotalAnswersToQuestions FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId),
// RankedUsers AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation, UR.TotalPosts, UR.TotalQuestions, UR.TotalAnswers, UR.TotalClosedPosts, UR.TotalAnswersToQuestions,
//        COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, ROW_NUMBER() OVER (ORDER BY UR.Reputation DESC) AS Rank
//     FROM UserReputation UR LEFT JOIN UserBadges UB ON UR.UserId = UB.UserId)
// SELECT Rank, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalClosedPosts, TotalAnswersToQuestions, GoldBadges, SilverBadges, BronzeBadges
// FROM RankedUsers WHERE Rank <= 10 ORDER BY Rank;
fn q7566(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(tu.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let s = (&by_user).map(|(u, _)| u).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and((&db.post.answer_count).opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some((t, n)) => {
            let x = if t == 1 { n } else { Some(0) };
            [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 10) as i64, a[4] + x.unwrap_or(0), a[5] + x.is_some() as i64]
        }
        None => [a[0], a[1], a[2], a[3], a[4], a[5] + 1],
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&by_user).and(&s).and((&ub).opt()));
    rows(v.into_iter().map(|(u, (((_, r), a), b))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[5])]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, CASE WHEN Reputation >= 1000 THEN 'High' WHEN Reputation >= 500 THEN 'Medium' ELSE 'Low' END AS ReputationTier FROM Users),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 10) AS CloseCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT p.Title, p.Score, p.CreationDate, ur.Reputation, ur.ReputationTier, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes, COALESCE(cp.CloseCount, 0) AS ClosedCount,
//        CASE WHEN cp.CloseCount > 0 THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM Posts p JOIN UserReputation ur ON p.OwnerUserId = ur.Id LEFT JOIN PostVoteCounts pvc ON p.Id = pvc.PostId LEFT JOIN ClosedPosts cp ON p.Id = cp.PostId
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND ur.ReputationTier IN ('High', 'Medium') AND p.ViewCount > 0
//   AND (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) > 3
// ORDER BY p.CreationDate DESC, p.Score DESC LIMIT 100;
//
// RecentPosts is never referenced.
fn q4796(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, score, .. } = &db.post;
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let pvc = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cp = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold(0i64, |n, t| n + (t == 10) as i64);
    let v = drain(
        db.post
            .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(view_count.gt(0)))
            .with((&cc).filt(|n| n > 3))
            .select(owner_user.select((&db.user.reputation).ge(500)).and((&pvc).opt()).and((&cp).opt())),
    );
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(score.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((r, a), c))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["title", "score", "created"]);
        f.extend([V::I(r), V::S(if r >= 1000 { "High" } else { "Medium" }), V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0))]);
        f.push(V::S(if c.map_or(false, |c| c > 0) { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS QuestionCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.Score, 0)) AS TotalScore, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY SUM(P.ViewCount) DESC) AS ActivityRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// RecentVotes AS (SELECT V.UserId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes V JOIN VoteTypes VT ON V.VoteTypeId = VT.Id WHERE V.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY V.UserId),
// UserRankings AS (SELECT UA.UserId, UA.DisplayName, UA.QuestionCount, UA.AnswerCount, UA.TotalViews, UA.TotalScore, RV.VoteCount, RV.UpVotes, RV.DownVotes,
//        RANK() OVER (ORDER BY UA.TotalScore DESC) AS OverallRank FROM UserActivity UA LEFT JOIN RecentVotes RV ON UA.UserId = RV.UserId)
// SELECT UR.DisplayName, UR.QuestionCount, UR.AnswerCount, UR.TotalViews, UR.TotalScore, COALESCE(UR.VoteCount, 0) AS VoteCount, COALESCE(UR.UpVotes, 0) AS UpVotes,
//        COALESCE(UR.DownVotes, 0) AS DownVotes, UR.OverallRank FROM UserRankings UR WHERE UR.OverallRank <= 10 ORDER BY UR.TotalScore DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q32779(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((t, w), s)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0), a[3] + s],
            None => a,
        });
    let rv = db.vote.with((&db.vote.creation_date).ge(add_days(date(2024, 10, 1), -30))).group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 3], |a, n| {
        [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]
    });
    let v = ranked(drain((&ua).and((&rv).opt())), |&(_, (a, _))| Reverse(a[3]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, r)), k)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(r.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(k));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, MAX(P.CreationDate) AS LatestActivity
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title),
// ClosedPosts AS (SELECT PH.PostId, COUNT(PH.Id) AS CloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId),
// AggregateData AS (SELECT PS.PostId, PS.Title, PS.UpVotes, PS.DownVotes, PS.CommentCount, PS.TotalViews, COALESCE(CP.CloseCount, 0) AS CloseCount, R.ReputationRank
//     FROM PostStatistics PS LEFT JOIN ClosedPosts CP ON PS.PostId = CP.PostId JOIN UserReputation R ON R.Id = PS.PostId)
// SELECT AD.Title, AD.UpVotes, AD.DownVotes, AD.CommentCount, AD.TotalViews, AD.CloseCount,
//        CASE WHEN AD.UpVotes > AD.DownVotes THEN 'Positive' WHEN AD.UpVotes < AD.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment,
//        CASE WHEN AD.TotalViews > 1000 THEN 'High' WHEN AD.TotalViews BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'Low' END AS ViewCategory
// FROM AggregateData AD WHERE AD.ReputationRank <= 100 ORDER BY AD.CloseCount DESC, AD.Title LIMIT 10;
//
// `R.Id = PS.PostId` joins a user id to a post id, so it goes through the raw ids, and the vote x comment product is driven only for the posts that join
// the hundred top users. A Reputation tie at the hundredth place goes to the smaller user id.
fn q3942(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 100);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let by_raw: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let ps: MatSet<Id<Post>> = (&tu).select((&db.user.origid).select(&by_raw)).collect();
    let s = (&ps)
        .group_by(Ident::<Post>::new())
        .select((&db.post.view_count).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()))
        .fold([0i64; 4], |a, ((w, t), c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and((&cp).opt()));
    let t = &db.post.title;
    let v = top_n(v, |&(p, (_, c))| (Reverse(c.unwrap_or(0)), t.get(p).is_none(), t.get(p), p), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(c.unwrap_or(0))]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        f.push(V::S(if a[3] > 1000 { "High" } else if a[3] >= 500 { "Medium" } else { "Low" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, Questions, Answers, Wikis, RANK() OVER (ORDER BY Reputation DESC) AS RankByReputation FROM UserReputation WHERE Reputation > 0),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// FinalResults AS (SELECT tu.UserId, tu.Reputation, tu.PostCount, tu.Questions, tu.Answers, tu.Wikis, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ub.GoldBadges, 0) AS GoldBadges,
//        COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, tu.RankByReputation FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId)
// SELECT fr.UserId, fr.Reputation, fr.PostCount, fr.Questions, fr.Answers, fr.Wikis, fr.BadgeCount, fr.GoldBadges, fr.SilverBadges, fr.BronzeBadges
// FROM FinalResults fr WHERE fr.RankByReputation <= 10 ORDER BY fr.Reputation DESC;
fn q6186(db: &'static So) -> String {
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 4], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&s).and((&ub).opt()));
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(CASE WHEN p.ViewCount IS NULL THEN 0 ELSE p.ViewCount END) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// TopUsers AS (SELECT u.Id, u.DisplayName, COALESCE(bs.GoldBadges, 0) AS GoldBadges, COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.Questions, 0) AS Questions, COALESCE(ps.Answers, 0) AS Answers, COALESCE(ps.TotalViews, 0) AS TotalViews,
//        RANK() OVER (ORDER BY COALESCE(ps.TotalPosts, 0) DESC, COALESCE(ps.TotalViews, 0) DESC) AS UserRank
//     FROM Users u LEFT JOIN UserBadgeStats bs ON u.Id = bs.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId WHERE u.Reputation > 100)
// SELECT u.DisplayName, u.UserRank, CONCAT(u.GoldBadges, ' Gold, ', u.SilverBadges, ' Silver, ', u.BronzeBadges, ' Bronze') AS BadgeCount, u.TotalPosts, u.Questions, u.Answers, u.TotalViews
// FROM TopUsers u WHERE u.UserRank <= 10 ORDER BY u.UserRank;
fn q2349(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 4], |a, (t, w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0)]);
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(100)).select((&ps).opt())), |&(_, p)| {
        let p = p.unwrap_or([0; 4]);
        (Reverse(p[0]), Reverse(p[3]))
    }, false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, p), r)| (u, (p, r))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, (Option<[i64; 4]>, i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&by_user).and((&bs).opt()));
    rows(v.into_iter().map(|(u, ((_, (p, r)), b))| {
        let b = b.unwrap_or([0; 3]);
        let p = p.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "name"), V::I(r), V::Owned(format!("{} Gold, {} Silver, {} Bronze", b[0], b[1], b[2]))];
        f.extend(p.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id LEFT JOIN Badges B ON U.Id = B.UserId
//     WHERE U.CreationDate >= '2023-01-01' GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, P.Tags, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT V.Id) AS VoteCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.Tags),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.ViewCount, PS.Score, PS.Tags, PS.CommentCount, PS.VoteCount, ROW_NUMBER() OVER (ORDER BY PS.Score DESC, PS.ViewCount DESC) AS Rank
//     FROM PostStatistics PS)
// SELECT UA.DisplayName, UA.UpVotes, UA.DownVotes, UA.PostCount, UA.CommentCount, UA.BadgeCount, TP.Title AS TopPostTitle, TP.ViewCount AS TopPostViewCount, TP.Score AS TopPostScore, TP.Rank
// FROM UserActivity UA LEFT JOIN TopPosts TP ON UA.UserId = TP.PostId WHERE TP.Rank <= 10 ORDER BY UA.BadgeCount DESC, UA.UpVotes DESC;
//
// `UA.UserId = TP.PostId` joins a user id to a post id, so it goes through the raw ids. Rank reads only base columns, so the ten posts are picked first and
// UserActivity is driven only for the users they join (own_votes: a user's votes on their own posts). A tie at the tenth place goes to the smaller post id.
fn q7073(db: &'static So) -> String {
    let Post { score, view_count, origid, .. } = &db.post;
    let tp = top_n(drain(score), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp = rel(tp.into_iter().enumerate().map(|(i, (p, _))| (p, i as i64 + 1)).collect());
    let by_raw: HashIdx<i64, (Id<Post>, i64)> = (&tp).map(|(p, _)| p).select(origid).inv().select(&tp).collect();
    let ua: MatSet<Id<User>> = db.user.with((&db.user.creation_date).ge(ts(2023, 1, 1, 0, 0, 0))).with((&db.user.origid).select(&by_raw)).collect();
    let s = (&ua)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(own_votes(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = (&ua).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = (&ua).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = (&ua).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&s).and(&pc).and(&cc).and(&bc).and((&db.user.origid).select(&by_raw)));
    let mut v = v;
    v.sort_by_key(|&(u, ((((a, _), _), b), _))| (Reverse(b), Reverse(a[0]), u));
    rows(v.into_iter().map(|(u, ((((a, n), c), b), (p, r)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n), V::I(c), V::I(b)];
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount, MAX(ph.CreationDate) AS LastEditDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopActiveUsers AS (SELECT ua.UserId, ua.DisplayName, ua.PostsCount, ua.Upvotes, ua.Downvotes, RANK() OVER (ORDER BY ua.PostsCount DESC) AS Rank FROM UserActivity ua
//     WHERE ua.PostsCount > 0 ORDER BY ua.PostsCount DESC)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.CommentCount, rp.AnswerCount, rp.LastEditDate, tau.UserId, tau.DisplayName AS TopPoster, tau.PostsCount, tau.Upvotes, tau.Downvotes
// FROM RecentPosts rp LEFT JOIN TopActiveUsers tau ON tau.Rank = 1 ORDER BY rp.ViewCount DESC LIMIT 10;
//
// Rank reads only the COUNT(DISTINCT), so the users at rank 1 are found first and the posts x votes product is driven for them alone.
// The ON condition names only tau, so the recent posts are crossed with the rank-1 users.
fn q7567(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let r = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let first: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 == 1).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ua = (&first).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let tau = left_all(drain((&ua).and(&pc)));
    let recent = || db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.eq(1)));
    let rc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = recent().group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let le = recent().group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.creation_date).opt()).fold(i64::MIN, |m, d| d.map_or(m, |d| m.max(d)));
    let rp = rel(drain((&rc).and(&ac).and(&le)));
    let mut v = Vec::new();
    (&rp).cross(&tau).drive(|_, ((p, ((c, a), e)), t)| v.push((p, c, a, e, t)));
    let v = top_n(v, |&(p, _, _, _, t)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p, t.map(|x| x.0))
    }, 10);
    rows(v.into_iter().map(|(p, c, a, e, t)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(c), V::I(a), tmax(e)]);
        match t {
            Some((u, (x, n))) => {
                f.extend(ucols(db, u, &["uid", "name"]));
                f.extend([V::I(n), V::I(x[0]), V::I(x[1])]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COUNT(P.Id) AS PostCount,
//        SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(COALESCE(CM.Score, 0)) AS CommentScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments CM ON P.Id = CM.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// ActiveUsers AS (SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.CreationDate, UA.LastAccessDate, UA.PostCount, UA.PositivePosts, UA.NegativePosts, UA.CommentScore,
//        RANK() OVER (ORDER BY UA.Reputation DESC) AS ReputationRank FROM UserActivity UA WHERE UA.LastAccessDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// SELECT A.UserId, A.DisplayName, A.Reputation, A.PostCount, A.PositivePosts, A.NegativePosts, A.CommentScore, COALESCE(SUM(B.Id), 0) AS BadgeCount,
//        CASE WHEN A.Reputation > 1000 THEN 'High Reputation' WHEN A.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM ActiveUsers A LEFT JOIN Badges B ON A.UserId = B.UserId WHERE A.PostCount > 5
// GROUP BY A.UserId, A.DisplayName, A.Reputation, A.PostCount, A.PositivePosts, A.NegativePosts, A.CommentScore HAVING AVG(A.CommentScore) > 0 ORDER BY A.Reputation DESC LIMIT 10 OFFSET 0;
//
// WITH RECURSIVE, but no CTE refers to itself. AVG(A.CommentScore) is over one group's copies of one value.
fn q34497(db: &'static So) -> String {
    let active = || db.user.with((&db.user.last_access_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ua = active()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).select(&db.comment.score).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, c)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + c.unwrap_or(0)],
            None => a,
        });
    let bs = active().group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.origid).opt()).fold(0i64, |n, b| n + b.unwrap_or(0));
    let v = drain((&ua).filt(|a| a[0] > 5 && a[3] > 0).and(&bs));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::I(b));
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, PP.DisplayName AS OwnerDisplayName, COALESCE(AVG(V.BountyAmount), 0) AS AverageBounty,
//        COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END) AS CommentCount, COUNT(CASE WHEN PV.PostId IS NOT NULL THEN 1 END) AS VoteCount
//     FROM Posts P LEFT JOIN Users PP ON P.OwnerUserId = PP.Id LEFT JOIN Comments C ON C.PostId = P.Id LEFT JOIN Votes V ON V.PostId = P.Id AND V.VoteTypeId = 9
//     LEFT JOIN Votes PV ON PV.PostId = P.Id GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, PP.DisplayName),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate AS ClosedDate, C.Name AS CloseReason FROM PostHistory PH INNER JOIN CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id
//     WHERE PH.PostHistoryTypeId = 10),
// TopPosts AS (SELECT PD.*, UR.ReputationRank FROM PostDetails PD JOIN UserReputation UR ON PD.OwnerDisplayName = UR.DisplayName WHERE PD.Score > 10 AND PD.ViewCount > 100)
// SELECT TP.Title, TP.CreationDate, TP.AverageBounty, TP.CommentCount, TP.VoteCount, CP.ClosedDate, CP.CloseReason, TP.ReputationRank
// FROM TopPosts TP LEFT JOIN ClosedPosts CP ON TP.PostId = CP.PostId ORDER BY TP.ReputationRank, TP.Score DESC;
//
// The WHERE reads only base columns of the post, so the comments x bounty-votes x votes product is driven only for the posts it keeps.
fn q3147(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let ur = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank_by_name: HashIdx<Str, (Id<User>, i64)> = (&ur).map(|(u, _)| u).select(&db.user.display_name).inv().select(&ur).collect();
    let keep = || db.post.with(score.gt(10).and(view_count.gt(100))).with(owner_user);
    let b9 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let pd = keep()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(b9.opt()).and(votes_of(db).opt()))
        .fold([0i64; 4], |a, ((c, b), v)| {
            let b = b.flatten();
            [a[0] + b.unwrap_or(0), a[1] + b.is_some() as i64, a[2] + c.is_some() as i64, a[3] + v.is_some() as i64]
        });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).select(post.and(Ident::<PostHistory>::new()).and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)));
    let cp = rel(drain(cp).into_iter().map(|(_, x)| x).collect());
    let closes: HashIdx<Id<Post>, ((Id<Post>, Id<PostHistory>), Str)> = (&cp).map(|((p, _), _)| p).inv().select(&cp).collect();
    let v = drain((&pd).and(owner_user.select(&db.user.display_name).select(&rank_by_name)).and((&closes).opt()));
    rows(v.into_iter().map(|(p, ((a, (_, r)), c))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([if a[1] == 0 { V::F(0.0) } else { avg(a[0], a[1]) }, V::I(a[2]), V::I(a[3])]);
        match c {
            Some(((_, h), n)) => f.extend([V::T(db.post_history.creation_date.get(h).unwrap()), V::S(n)]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS Upvotes,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS Downvotes FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostWithBadges AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.CommentCount, rp.Upvotes, rp.Downvotes, COUNT(b.Id) AS BadgeCount FROM RankedPosts rp
//     LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) GROUP BY rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.CommentCount, rp.Upvotes, rp.Downvotes),
// PostSummary AS (SELECT p.Title, p.Score, p.ViewCount, p.CommentCount, p.Upvotes, p.Downvotes, CASE WHEN p.BadgeCount > 0 THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus
//     FROM PostWithBadges p WHERE p.CommentCount > (SELECT AVG(CommentCount) FROM PostWithBadges))
// SELECT Title, Score, ViewCount, CommentCount, Upvotes, Downvotes, BadgeStatus FROM PostSummary WHERE Score > 10 ORDER BY Score DESC, ViewCount DESC LIMIT 50 OFFSET 0;
//
// `CommentCount > AVG(CommentCount)` is compared exactly, as `CommentCount * n > sum`.
fn q3799(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let (sum, n) = (&cc).fold_flat((0i64, 0i64), |(s, n), c| (s + c, n + 1));
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(recent().with(score.gt(10)).select((&cc).filt(move |c| (c as i128) * (n as i128) > sum as i128).and(&vc).and(owner_user.select(&bc).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, ((c, a), b))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(if b.unwrap_or(0) > 0 { "Has Badges" } else { "No Badges" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, SUM(CASE WHEN P.PostTypeId = 1 THEN P.Score ELSE 0 END) AS TotalQuestionScore,
//        COUNT(CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount,
//        COUNT(DISTINCT CASE WHEN V.VoteTypeId = 2 THEN V.Id END) AS UpvoteCount, COUNT(DISTINCT CASE WHEN V.VoteTypeId = 3 THEN V.Id END) AS DownvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY U.CreationDate DESC) AS LatestActivity
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// FilteredUsers AS (SELECT UserId, DisplayName, Reputation, TotalQuestionScore, QuestionCount, AnswerCount, UpvoteCount, DownvoteCount FROM UserActivity WHERE LatestActivity = 1 AND Reputation > 100),
// HighScoringUsers AS (SELECT UserId, DisplayName, Reputation, (TotalQuestionScore + UpvoteCount * 2 - DownvoteCount) AS FinalScore FROM FilteredUsers WHERE QuestionCount > 5)
// SELECT U.UserId, U.DisplayName, U.Reputation AS OriginalReputation, COALESCE(B.BadgeCount, 0) AS BadgeCount, U.FinalScore
// FROM HighScoringUsers U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON U.UserId = B.UserId
// WHERE U.FinalScore > 50 ORDER BY U.FinalScore DESC, U.DisplayName ASC;
//
// LatestActivity partitions by the user it groups on, so it is 1 for every row. Each vote joins one post, so COUNT(DISTINCT V.Id) of a type counts the joined rows of that type.
fn q23065(db: &'static So) -> String {
    let s = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((t, s), v)) => [a[0] + if t == 1 { s } else { 0 }, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&s).filt(|a| a[1] > 5 && a[0] + a[2] * 2 - a[3] > 50).and((&bc).opt()));
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(a[0] + a[2] * 2 - a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS OwnerPostRank, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY P.Id) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY P.Id), 0) AS TotalDownVotes
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// ClosedPostOptions AS (SELECT PH.PostId, COUNT(*) AS CloseHistoryCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId),
// PostStatistics AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, RP.TotalUpVotes, RP.TotalDownVotes, COALESCE(CPO.CloseHistoryCount, 0) AS NumberOfClosures
//     FROM RankedPosts RP LEFT JOIN ClosedPostOptions CPO ON RP.PostId = CPO.PostId)
// SELECT PS.*, CASE WHEN PS.NumberOfClosures > 0 THEN 'Closed' ELSE 'Active' END AS PostStatus,
//        (CASE WHEN PS.TotalUpVotes - PS.TotalDownVotes < 0 THEN 'Negative Impact' ELSE 'Positive Impact' END) AS UserImpact
// FROM PostStatistics PS WHERE PS.TotalUpVotes > PS.TotalDownVotes * 1.5 ORDER BY PS.Score DESC, PS.CreationDate ASC LIMIT 100;
//
// RankedPosts has no GROUP BY, so each post appears once per vote; the windows are a fold over the same rows. `up > down * 1.5` is compared as `2 up > 3 down`.
fn q1681(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let w = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let mut v = Vec::new();
    recent().select(votes_of(db).opt().and((&w).filt(|a| 2 * a[0] > 3 * a[1])).and((&cp).opt())).drive(|p, ((_, a), c)| v.push((p, a, c)));
    let v = top_n(v, |&(p, _, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 100);
    rows(v.into_iter().map(|(p, a, c)| {
        let c = c.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if c > 0 { "Closed" } else { "Active" }), V::S(if a[0] - a[1] < 0 { "Negative Impact" } else { "Positive Impact" })]);
        row(f)
    }))
}

// WITH UserActivities AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViewCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, DENSE_RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS ActivityRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// TopUserActivities AS (SELECT UserId, DisplayName, PostCount, TotalViewCount, UpVotes, DownVotes FROM UserActivities WHERE ActivityRank <= 10),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS Gold, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS Silver, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS Bronze
//     FROM Badges B GROUP BY B.UserId)
// SELECT TUA.DisplayName, TUA.PostCount, TUA.TotalViewCount, COALESCE(UB.Gold, 0) AS GoldBadges, COALESCE(UB.Silver, 0) AS SilverBadges, COALESCE(UB.Bronze, 0) AS BronzeBadges,
//        CASE WHEN (TUA.TotalViewCount > 1000 AND TUA.PostCount > 20) THEN 'High Contributor' WHEN (TUA.TotalViewCount <= 1000 AND TUA.PostCount <= 20) THEN 'Low Contributor' ELSE 'Moderate Contributor' END AS ContributionLevel,
//        CASE WHEN TUA.TotalViewCount IS NULL THEN 'Views Not Recorded' ELSE 'Views Recorded' END AS ViewStatus
// FROM TopUserActivities TUA LEFT JOIN UserBadges UB ON TUA.UserId = UB.UserId ORDER BY TUA.TotalViewCount DESC, TUA.PostCount DESC;
fn q20546(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((w, _)) => [a[0] + 1, a[1] + w.unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), true);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 2])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&by_user).and((&ub).opt()));
    rows(v.into_iter().map(|(u, ((_, a), b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])];
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(V::S(if a[1] > 1000 && a[0] > 20 { "High Contributor" } else if a[1] <= 1000 && a[0] <= 20 { "Low Contributor" } else { "Moderate Contributor" }));
        f.push(V::S("Views Recorded"));
        row(f)
    }))
}

// WITH TagStats AS (SELECT T.Id AS TagId, T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, SUM(COALESCE(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END, 0)) AS QuestionCount,
//        SUM(COALESCE(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AnswerCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.Id, T.TagName),
// BadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PopularTags AS (SELECT TagId, TagName, PostCount, TotalViews, QuestionCount, AnswerCount, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM TagStats),
// HighlyActiveUsers AS (SELECT U.Id, U.DisplayName, COALESCE(BC.BadgeCount, 0) AS BadgeCount, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsSubmitted,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersSubmitted FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN BadgeCounts BC ON U.Id = BC.UserId
//     GROUP BY U.Id, U.DisplayName, BC.BadgeCount)
// SELECT PT.TagName, PT.PostCount, PT.TotalViews, PT.QuestionCount, PT.AnswerCount, AU.DisplayName AS ActiveUser, AU.TotalPosts, AU.BadgeCount
// FROM PopularTags PT JOIN HighlyActiveUsers AU ON AU.QuestionsSubmitted > 0 WHERE PT.ViewRank <= 10 ORDER BY PT.TotalViews DESC, AU.BadgeCount DESC;
//
// The ON condition names only AU, so the top tags are crossed with the users who asked a question.
fn q29993(db: &'static So) -> String {
    let ts = tag_stats(db);
    let v = ranked(drain((&ts).filt(|a| a[0] > 0)), |&(_, a)| (a[1] == 0, Reverse(a[2])), false);
    let pt = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let au = rel(drain((&ups).filt(|a| a[2] > 0).and(&bc)));
    let mut v = Vec::new();
    (&pt).cross(&au).drive(|_, ((t, a), (u, (p, b)))| v.push((t, a, u, p, b)));
    rows(v.into_iter().map(|(t, a, u, p, b)| {
        row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1]), V::I(a[4]), V::I(a[5]), user_col(db, u, "name"), V::I(p[1]), V::I(b)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.Body, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.Body, p.Tags, p.OwnerUserId),
// TopCommentedPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, Body, Tags, CommentCount, RANK() OVER (ORDER BY CommentCount DESC, Score DESC) AS CommentRank FROM RankedPosts)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.Body, tp.Tags, tp.CommentCount, tp.CommentRank, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        b.Name AS BadgeName, pht.Name AS PostHistoryTypeName
// FROM TopCommentedPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) LEFT JOIN Badges b ON b.UserId = u.Id AND b.Class = 1
// LEFT JOIN PostHistory ph ON ph.PostId = tp.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE tp.CommentRank <= 10 ORDER BY tp.CommentRank, tp.Score DESC;
fn q27008(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let v = ranked(drain((&rp).and(score)), |&(_, (c, s))| (Reverse(c), Reverse(s)), false);
    let tp = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (c, _)), r)| (p, (c, r))).collect());
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    type T = (Id<Post>, (i64, i64));
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select(owner_user.select(Ident::<User>::new().and(gold.opt())).and(history_of(db).select(htype_name(db)).opt())))));
    rows(v.into_iter().map(|(_, ((p, (c, r)), ((u, b), h)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "body", "tags"]);
        f.extend([V::I(c), V::I(r)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(b.map_or(V::Null, |b| V::S(db.badge.name.get(b).unwrap())));
        f.push(ostr(h));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostHistoryStats AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosedDate, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS LastReopenedDate,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 12 THEN ph.CreationDate END) AS LastDeletedDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, COALESCE(pv.UpVotes, 0) AS UpVotes, COALESCE(pv.DownVotes, 0) AS DownVotes,
//        CASE WHEN phs.LastClosedDate IS NOT NULL AND (phs.LastReopenedDate IS NULL OR phs.LastClosedDate > phs.LastReopenedDate) THEN 'Closed' WHEN phs.LastDeletedDate IS NOT NULL THEN 'Deleted' ELSE 'Active' END AS PostStatus,
//        CASE WHEN rp.rn = 1 THEN 'Most Recent' ELSE 'Older Post' END AS PostRank
// FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId LEFT JOIN PostHistoryStats phs ON rp.PostId = phs.PostId WHERE rp.rn <= 5 ORDER BY rp.OwnerUserId, rp.CreationDate DESC;
//
// A tie on CreationDate at an owner's fifth place goes to the smaller post id.
fn q31938(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let first = top_per(top.clone(), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([i64::MIN; 3], |a, (t, d)| match t {
        10 => [a[0].max(d), a[1], a[2]],
        11 => [a[0], a[1].max(d), a[2]],
        12 => [a[0], a[1], a[2].max(d)],
        _ => a,
    });
    let v = drain((&tp).select((&pv).opt().and((&phs).opt()).and(Ident::<Post>::new().with(&first).opt())));
    rows(v.into_iter().map(|(p, ((a, h), r))| {
        let a = a.unwrap_or([0, 0]);
        let h = h.unwrap_or([i64::MIN; 3]);
        let st = if h[0] != i64::MIN && (h[1] == i64::MIN || h[0] > h[1]) { "Closed" } else if h[2] != i64::MIN { "Deleted" } else { "Active" };
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(st), V::S(if r.is_some() { "Most Recent" } else { "Older Post" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// PostVoteDetails AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(*) AS TotalVotes FROM Votes v GROUP BY v.PostId),
// TopPosts AS (SELECT rp.PostID, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rv.UpVotes, rv.DownVotes, rv.TotalVotes, RANK() OVER (ORDER BY rp.Score DESC, rv.UpVotes DESC) AS PostRank
//     FROM RankedPosts rp LEFT JOIN PostVoteDetails rv ON rp.PostID = rv.PostId)
// SELECT tp.PostID, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.UpVotes, tp.DownVotes, tp.TotalVotes, tp.PostRank, COALESCE(c.CommentCount, 0) AS TotalComments,
//        COALESCE(b.BadgeCount, 0) AS TotalBadges
// FROM TopPosts tp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON tp.PostID = c.PostId
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON tp.PostID = b.UserId WHERE tp.PostRank <= 10 ORDER BY tp.Score DESC, tp.UpVotes DESC;
//
// `tp.PostID = b.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q7752(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, origid, .. } = &db.post;
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(date(2024, 10, 1), -30)))).with(owner_user).select(score.and((&pv).opt())));
    let v = ranked(v, |&(_, (s, a))| (Reverse(s), a.is_none(), Reverse(a.map(|a| a[0]))), false);
    let tp = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (_, a)), r)| (p, (a, r))).collect());
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    type T = (Id<Post>, (Option<[i64; 3]>, i64));
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select((&cc).opt().and(origid.select(&bc).opt())))));
    rows(v.into_iter().map(|(_, ((p, (a, r)), (c, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend(match a {
            Some(a) => a.iter().map(|&x| V::I(x)).collect::<Vec<_>>(),
            None => vec![V::Null, V::Null, V::Null],
        });
        f.extend([V::I(r), V::I(c.unwrap_or(0)), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH TagCounts AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadgeScore, u.Reputation, COUNT(DISTINCT v.Id) AS VoteCount
//     FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, ur.TotalBadgeScore, ur.VoteCount, ROW_NUMBER() OVER (ORDER BY ur.Reputation DESC, ur.TotalBadgeScore DESC) AS UserRank
//     FROM UserReputation ur WHERE ur.Reputation > 1000),
// FilteredTags AS (SELECT tc.TagName, tc.PostCount, tc.TotalViews, tc.TotalScore, tc.QuestionCount, tc.AnswerCount, ROW_NUMBER() OVER (ORDER BY tc.TotalViews DESC) AS TagRank FROM TagCounts tc)
// SELECT ft.TagName, ft.PostCount, ft.TotalViews, ft.TotalScore, ft.QuestionCount, ft.AnswerCount, tu.DisplayName AS TopUser, tu.Reputation AS TopUserReputation,
//        tu.TotalBadgeScore AS TopUserBadgeScore, tu.VoteCount AS TopUserVoteCount
// FROM FilteredTags ft JOIN TopUsers tu ON ft.QuestionCount > 0 WHERE ft.TagRank <= 10 AND tu.UserRank <= 5 ORDER BY ft.TotalViews DESC, tu.Reputation DESC;
//
// UserRank leads with Reputation, so only users at or above the fifth-highest Reputation can rank in the top five; the badges x votes product is driven
// for those alone. The ON condition names only ft, so the top tags are crossed with the top users.
fn q26534(db: &'static So) -> String {
    let ts = tag_stats(db);
    let tt = top_n(drain((&ts).filt(|a| a[0] > 0)), |&(t, a)| (Reverse(a[2]), t), 10);
    let tt = rel(tt);
    let fifth = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 5).last().unwrap().1;
    let cand = || db.user.with((&db.user.reputation).gt(1000).and((&db.user.reputation).ge(fifth)));
    let ur = cand().group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).opt())).fold([0i64; 2], |a, (c, _)| [a[0] + c.unwrap_or(0), a[1] + c.is_some() as i64]);
    let vc = cand().group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let tu = top_n(drain((&ur).and(&vc)), |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), a[1] == 0, Reverse(a[0]), u), 5);
    let tu = rel(tu);
    let mut v = Vec::new();
    (&tt).filt(|(_, a): (Id<Tag>, [i64; 6])| a[4] > 0).cross(&tu).drive(|_, ((t, a), (u, (b, n)))| v.push((t, a, u, b, n)));
    rows(v.into_iter().map(|(t, a, u, b, n)| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[5])];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([nullable(b[0], b[1]), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        DENSE_RANK() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC, p.CreationDate DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentActivity AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS ChangeCount FROM PostHistory ph WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
//     AND ph.PostHistoryTypeId IN (10, 11, 12, 13) GROUP BY ph.PostId, ph.PostHistoryTypeId),
// AggregatedVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v
//     WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, COALESCE(ra.ChangeCount, 0) AS RecentChanges, COALESCE(av.UpVotes, 0) AS UpVotes,
//        COALESCE(av.DownVotes, 0) AS DownVotes, rp.TagRank
// FROM RankedPosts rp LEFT JOIN RecentActivity ra ON rp.PostId = ra.PostId LEFT JOIN AggregatedVotes av ON rp.PostId = av.PostId WHERE rp.TagRank = 1
// ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 10;
fn q8332(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, tags_str, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).with(owner_user).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ra = db.post_history.with(hd.ge(add_months(t0, -1)).and(post_history_type_id.is_in([10, 11, 12, 13]))).group_by(post.and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rv = rel(drain(&ra));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&rv).map(|((p, _), _)| p).inv().select(&rv).collect();
    let av = db.vote.with((&db.vote.creation_date).ge(add_months(t0, -1))).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&tp).select((&by_post).opt().and((&av).opt())));
    let v = top_n(v, |&(p, (r, _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, r.map(|x| x.0)), 10);
    rows(v.into_iter().map(|(p, (r, a))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "answers", "comments", "owner"]);
        f.extend([V::I(r.map_or(0, |x| x.1)), V::I(a[0]), V::I(a[1]), V::I(1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostCount,
//        SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.Score, us.DisplayName, us.UserId, rp.CreationDate, rp.CommentCount FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.PostRank = 1)
// SELECT tp.Title, tp.Score, tp.CommentCount, us.Reputation, us.BadgeCount, us.TotalUpVotes, us.TotalDownVotes,
//        CASE WHEN us.Reputation > 1000 THEN 'High Reputation' WHEN us.Reputation BETWEEN 501 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM TopPosts tp JOIN UserStats us ON tp.UserId = us.UserId ORDER BY tp.Score DESC, us.Reputation DESC LIMIT 10;
//
// The ORDER BY reads only the post's Score and the owner's Reputation, so the ten rows are picked first and the badges x posts product is driven for their owners.
// A Score tie between two of one owner's posts goes to the smaller post id.
fn q33652(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let first = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let top = top_n(first, |&(p, u)| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(badges_of(db).opt()).and(posts_of(db).opt()))
        .fold([0i64; 4], |a, (((up, dn), b), p)| [a[0] + b.is_some() as i64, a[1] + p.is_some() as i64, a[2] + up, a[3] + dn]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&us))));
    let v = top_n(v, |&(p, (_, (u, _)))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(c), V::I(r), V::I(a[0]), V::I(a[2]), V::I(a[3])]);
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 501 { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentComments AS (SELECT C.PostId, COUNT(*) AS CommentCount, MAX(C.CreationDate) AS LastCommentDate FROM Comments C
//     WHERE C.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY C.PostId),
// PostAggregation AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, COALESCE(RC.CommentCount, 0) AS CommentCount, RC.LastCommentDate
//     FROM RankedPosts RP LEFT JOIN RecentComments RC ON RP.PostId = RC.PostId WHERE RP.PostRank <= 5)
// SELECT PA.PostId, PA.Title, PA.OwnerDisplayName, PA.CreationDate, PA.Score, PA.ViewCount, PA.CommentCount, CASE WHEN PA.CommentCount > 0 THEN 'Active' ELSE 'Inactive' END AS PostActivity,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = PA.PostId AND V.VoteTypeId = 2) AS UpvoteCount, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = PA.PostId AND V.VoteTypeId = 3) AS DownvoteCount
// FROM PostAggregation PA ORDER BY PA.Score DESC, PA.CommentCount DESC;
fn q2490(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rc = db.comment.with((&db.comment.creation_date).ge(add_months(t0, -1))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&vc).and((&rc).opt()));
    rows(v.into_iter().map(|(p, (a, c))| {
        let c = c.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(c), V::S(if c > 0 { "Active" } else { "Inactive" }), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CreationDate, ARRAY_LENGTH(string_to_array(p.Tags, '><'), 1) AS TagCount,
//        RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CreationDate, p.Tags),
// UserPostInteraction AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(COALESCE(ph.RankScore, 0)) AS TotalPostRank, SUM(u.UpVotes + u.DownVotes) AS TotalVotes,
//        AVG(p.ViewCount) AS AvgViewCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN RankedPosts ph ON p.Id = ph.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// FinalReport AS (SELECT uli.UserId, uli.DisplayName, uli.PostsCreated, uli.TotalPostRank, uli.TotalVotes, uli.AvgViewCount, ROW_NUMBER() OVER (ORDER BY uli.TotalPostRank DESC) AS UserRank
//     FROM UserPostInteraction uli WHERE uli.PostsCreated > 10)
// SELECT fr.UserId, fr.DisplayName, fr.PostsCreated, fr.TotalPostRank, fr.TotalVotes, fr.AvgViewCount, fr.UserRank FROM FinalReport fr WHERE fr.UserRank <= 10 ORDER BY fr.UserRank;
//
// Only RankScore is read from RankedPosts, one row per question.
fn q27449(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let r = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    let rk = rel(r.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).select(view_count.opt().and((&rank).opt())).opt()))
        .fold([0i64; 5], |a, ((up, dn), p)| match p {
            Some((w, r)) => [a[0] + 1, a[1] + r.map_or(0, |x| x.1), a[2] + up + dn, a[3] + w.unwrap_or(0), a[4] + w.is_some() as i64],
            None => [a[0], a[1], a[2] + up + dn, a[3], a[4]],
        });
    let v = top_n(drain((&s).filt(|a| a[0] > 10)), |&(u, a)| (Reverse(a[1]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4]), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.Tags FROM RankedPosts rp WHERE rp.Rank <= 10),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// PostEngagement AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, ue.UserId, ue.DisplayName, ue.CommentCount, ue.UpVotes, ue.DownVotes FROM TopPosts tp
//     JOIN UserEngagement ue ON ue.UserId IN (SELECT DISTINCT OwnerUserId FROM Posts WHERE Id = tp.PostId))
// SELECT pe.PostId, pe.Title, pe.Score, pe.ViewCount, pe.UserId, pe.DisplayName, pe.CommentCount, pe.UpVotes, pe.DownVotes FROM PostEngagement pe ORDER BY pe.Score DESC, pe.ViewCount DESC;
//
// The IN subquery is the post's owner, so UserEngagement is driven only for the owners of the top posts.
fn q6034(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let since = add_years(current_date(), -1);
    let v = drain(db.post.with(creation_date.ge(since)).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user.select(Ident::<User>::new().with((&db.user.creation_date).ge(since)))).collect();
    let ue = (&owners).group_by(Ident::<User>::new()).select(comments_by(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&ue))));
    rows(v.into_iter().map(|(p, (u, a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostsWithBestAnswer AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(V.VoteCount, 0) AS VoteCount, P.AcceptedAnswerId,
//        CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN (SELECT COUNT(*) FROM Votes WHERE PostId = P.AcceptedAnswerId AND VoteTypeId = 2) ELSE 0 END AS AcceptedAnswerVoteCount
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) V ON P.Id = V.PostId WHERE P.PostTypeId = 1),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U)
// SELECT Ub.DisplayName AS UserDisplayName, P.Title AS PostTitle, P.CreationDate AS PostCreationDate, COALESCE(P.AcceptedAnswerVoteCount, 0) AS AcceptedAnswerVotes,
//        Ub.GoldBadges, Ub.SilverBadges, Ub.BronzeBadges, T.UserId AS TopUserId, T.DisplayName AS TopUserDisplayName, T.Rank AS UserRank
// FROM UserBadges Ub FULL OUTER JOIN PostsWithBestAnswer P ON Ub.UserId = P.AcceptedAnswerId JOIN TopUsers T ON Ub.UserId = T.UserId
// WHERE (Ub.BadgeCount > 0 OR P.AcceptedAnswerVoteCount > 0) AND T.Rank <= 10 ORDER BY T.Rank, P.CreationDate DESC;
//
// The inner join on Ub.UserId drops every row the FULL OUTER JOIN adds for an unmatched question, so it is Users LEFT JOIN PostsWithBestAnswer.
// `Ub.UserId = P.AcceptedAnswerId` joins a user id to a post id, so it goes through the raw ids. A Reputation tie at the tenth place goes to the smaller user id.
fn q1042(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(tu.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let up = db.vote.with((&db.vote.vote_type_id).eq(2)).group_by(&db.vote.post_id).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let qa: HashIdx<i64, Id<Post>> = db.post.with(post_type_id.eq(1)).select(accepted_answer_id).inv().collect();
    let pwba = (&qa).select(Ident::<Post>::new().and(accepted_answer_id.select(&up).opt()));
    let v = drain((&by_user).and(&ub).and((&db.user.origid).select(pwba).opt()));
    let v: Vec<_> = drain(rel(v).filt(|(_, (((_, _), b), p))| b[0] > 0 || p.map_or(false, |(_, n)| n.unwrap_or(0) > 0))).into_iter().map(|x| x.1).collect();
    let v = top_n(v, |&(u, (((_, r), _), p))| {
        let d = p.map(|(p, _)| db.post.creation_date.get(p).unwrap());
        (r, d.is_none(), Reverse(d), u)
    }, 0);
    rows(v.into_iter().map(|(u, (((_, r), b), p))| {
        let mut f = vec![user_col(db, u, "name")];
        match p {
            Some((p, n)) => {
                f.extend(post_fields(db, p, &["title", "created"]));
                f.push(V::I(n.unwrap_or(0)));
            }
            None => f.extend([V::Null, V::Null, V::I(0)]),
        }
        f.extend([V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT P.Id) AS TotalPosts, AVG(P.ViewCount) AS AverageViewCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalUpvotes, TotalDownvotes, TotalPosts, AverageViewCount, ROW_NUMBER() OVER (ORDER BY TotalUpvotes DESC) AS Rank FROM UserVoteStats WHERE TotalPosts > 0),
// ClosedPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, PH.CreationDate AS ClosedDate FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId WHERE PH.PostHistoryTypeId = 10),
// UserPostClosedStats AS (SELECT C.OwnerUserId, COUNT(DISTINCT C.PostId) AS TotalClosedPosts, MIN(C.ClosedDate) AS FirstClosedDate FROM ClosedPosts C GROUP BY C.OwnerUserId)
// SELECT T.UserId, T.DisplayName, T.TotalUpvotes, T.TotalDownvotes, T.TotalPosts, T.AverageViewCount, COALESCE(U.TotalClosedPosts, 0) AS TotalClosedPosts, U.FirstClosedDate,
//        CASE WHEN T.TotalUpvotes - T.TotalDownvotes > 10 THEN 'Active Contributor' WHEN T.TotalUpvotes - T.TotalDownvotes < 0 THEN 'Negative Feedback' ELSE 'Moderate Activity' END AS UserActivityStatus
// FROM TopUsers T LEFT JOIN UserPostClosedStats U ON T.UserId = U.OwnerUserId WHERE T.Rank <= 10 ORDER BY T.TotalUpvotes DESC, T.TotalDownvotes ASC;
fn q1381(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (w, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + w.unwrap_or(0), a[3] + w.is_some() as i64]);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tu = top_n(drain((&s).and(&pc)), |&(u, (a, _))| (Reverse(a[0]), u), 10);
    let tu = rel(tu);
    let by_user: HashIdx<Id<User>, (Id<User>, ([i64; 4], i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).select(&db.post_history.creation_date);
    let cs = db.post.group_by(&db.post.owner_user).select(Ident::<Post>::new().and(closes)).buf_fold(|v| (distinct_some(v.iter().map(|x| Some(x.0))), v.iter().map(|x| x.1).min().unwrap()));
    let v = drain((&by_user).and((&cs).opt()));
    rows(v.into_iter().map(|(u, ((_, (a, n)), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), avg(a[2], a[3])]);
        match c {
            Some((k, d)) => f.extend([V::I(k), V::T(d)]),
            None => f.extend([V::I(0), V::Null]),
        }
        let d = a[0] - a[1];
        f.push(V::S(if d > 10 { "Active Contributor" } else if d < 0 { "Negative Feedback" } else { "Moderate Activity" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, RANK() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS ScoreRank, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.Score, p.CreationDate, pt.Name),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ScoreRank, rp.CommentCount, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.ScoreRank = 1 THEN 'Top Post' WHEN rp.ScoreRank <= 5 THEN 'Popular' ELSE 'Regular' END AS PostCategory FROM RankedPosts rp WHERE rp.CommentCount > 0),
// LatestActivity AS (SELECT p.Id, MAX(p.LastActivityDate) AS LastActivity FROM Posts p WHERE p.LastActivityDate IS NOT NULL GROUP BY p.Id)
// SELECT fp.PostId, fp.Title, fp.Score, fp.CreationDate, fp.CommentCount, fp.UpVotes, fp.DownVotes, fp.PostCategory, la.LastActivity,
//        CASE WHEN la.LastActivity < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' THEN 'Inactive' ELSE 'Active' END AS ActivityStatus
// FROM FilteredPosts fp LEFT JOIN LatestActivity la ON fp.PostId = la.Id WHERE fp.Score IS NOT NULL ORDER BY fp.Score DESC, fp.CreationDate ASC;
fn q33027(db: &'static So) -> String {
    let Post { score, last_activity_date, .. } = &db.post;
    let rp = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = per_group(ranked(drain((&rp).and(ptype_name(db))), |&(p, (_, n))| (n, Reverse(score.get(p).unwrap())), false), |&(_, (_, n))| n);
    let t = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let v: Vec<_> = drain(rel(v).filt(|((_, (a, _)), _)| a[0] > 0)).into_iter().map(|x| x.1).collect();
    rows(v.into_iter().map(|((p, (a, _)), r)| {
        let la = last_activity_date.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend(a.map(V::I));
        f.extend([V::S(if r == 1 { "Top Post" } else if r <= 5 { "Popular" } else { "Regular" }), V::T(la), V::S(if la < t { "Inactive" } else { "Active" })]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 1000),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COALESCE(a.Body, 'No Answer') AS AcceptedAnswerBody, COUNT(c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBounty
//     FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, a.Body),
// UserPostCounts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount FROM Posts p GROUP BY p.OwnerUserId),
// FilteredPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.AcceptedAnswerBody, upc.PostCount, r.DisplayName, r.ReputationRank, pd.CommentCount, pd.TotalBounty
//     FROM PostDetails pd JOIN Users up ON pd.OwnerUserId = up.Id JOIN RankedUsers r ON up.Id = r.UserId LEFT JOIN UserPostCounts upc ON upc.OwnerUserId = pd.OwnerUserId
//     WHERE pd.CommentCount > 5 AND upc.PostCount IS NOT NULL)
// SELECT fp.*, CASE WHEN fp.TotalBounty IS NULL THEN 'No Bounty' ELSE CONCAT('Total Bounty: $', fp.TotalBounty) END AS BountyStatus FROM FilteredPosts fp
// ORDER BY fp.ReputationRank, fp.CommentCount DESC LIMIT 10;
//
// PostDetails groups by the post, so only the questions whose owner is in RankedUsers are aggregated.
fn q1291(db: &'static So) -> String {
    let Post { post_type_id, owner_user, accepted_answer, body, .. } = &db.post;
    let ru = rel(ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&ru).map(|(u, _)| u).inv().select(&ru).collect();
    let b9 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let pd = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user.select(&rank))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(b9.opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + b.is_some() as i64]
        });
    let upc = db.post.group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&pd).filt(|a| a[0] > 5).and(owner_user.select(Ident::<User>::new().and(&rank).and(&upc))).and(accepted_answer.select(body).opt()));
    let v = top_n(v, |&(p, ((a, ((_, (_, r)), _)), _))| (r, Reverse(a[0]), p), 10);
    rows(v.into_iter().map(|(p, ((a, ((u, (_, r)), n)), ab))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::S(ab.unwrap_or("No Answer")), V::I(n), user_col(db, u, "name"), V::I(r), V::I(a[0]), nullable(a[1], a[2])]);
        f.push(if a[2] == 0 { V::S("No Bounty") } else { V::Owned(format!("Total Bounty: ${}", a[1])) });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY SUM(rp.UpVoteCount) DESC) AS Rank FROM Users u JOIN RecentPosts rp ON u.Id = rp.OwnerUserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, min(cr.Name) AS CloseReason FROM PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment::int = cr.Id WHERE ph.PostHistoryTypeId IN (10, 11)
//     GROUP BY ph.PostId, ph.CreationDate)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, u.DisplayName AS OwnerDisplayName, tu.Reputation, rp.UpVoteCount, rp.DownVoteCount, cp.CloseReason
// FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN TopUsers tu ON u.Id = tu.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE tu.Rank <= 5 OR cp.CloseReason IS NOT NULL ORDER BY rp.CreationDate DESC, tu.Rank ASC;
fn q2219(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tu = db.post.with(&rp).group_by(owner_user).select(&rp).fold(0i64, |n, a| n + a[1]);
    let tr = rel(ranked(drain(&tu), |&(_, n)| Reverse(n), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(hd))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .fold(None::<Str>, |m, n| Some(m.map_or(n, |m| if n < m { n } else { m })));
    let cv = rel(drain(&cp));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), Option<Str>)> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let v = drain((&rp).and(owner_user.select(Ident::<User>::new().and((&rank).opt()))).and((&by_post).opt()));
    let v: Vec<_> = drain(rel(v).filt(|(_, ((_, (_, r)), c))| r.map_or(false, |(_, r)| r <= 5) || c.map_or(false, |(_, n)| n.is_some()))).into_iter().map(|x| x.1).collect();
    rows(v.into_iter().map(|(p, ((a, (u, r)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), user_col(db, u, "name"), if r.is_some() { user_col(db, u, "rep") } else { V::Null }, V::I(a[1]), V::I(a[2])]);
        f.push(ostr(c.and_then(|(_, n)| n)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE vt.Name = 'UpMod') AS UpVotes,
//        COUNT(v.Id) FILTER (WHERE vt.Name = 'DownMod') AS DownVotes,
//        RANK() OVER (PARTITION BY p.Tags ORDER BY (COUNT(v.Id) FILTER (WHERE vt.Name = 'UpMod') - COUNT(v.Id) FILTER (WHERE vt.Name = 'DownMod')) DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//     GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName),
// TagStatistics AS (SELECT t.TagName, COUNT(DISTINCT rp.PostId) AS PostCount, SUM(rp.CommentCount) AS TotalComments, AVG(rp.UpVotes) AS AverageUpVotes, AVG(rp.DownVotes) AS AverageDownVotes,
//        MAX(rp.Rank) AS MaxRank FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN RankedPosts rp ON p.Id = rp.PostId GROUP BY t.TagName),
// FinalStatistics AS (SELECT ts.TagName, ts.PostCount, ts.TotalComments, ts.AverageUpVotes, ts.AverageDownVotes, CASE WHEN ts.MaxRank IS NOT NULL THEN 'Active' ELSE 'Inactive' END AS TagStatus
//     FROM TagStatistics ts)
// SELECT fs.TagName, fs.PostCount, fs.TotalComments, fs.AverageUpVotes, fs.AverageDownVotes, fs.TagStatus FROM FinalStatistics fs ORDER BY fs.PostCount DESC, fs.TotalComments DESC;
//
// MaxRank is the MAX of a RANK over a non-empty group, never NULL, so every tag is 'Active' and the rank is not computed.
fn q28434(db: &'static So) -> String {
    let rp = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vtype_name(db)).opt()))
        .fold([0i64; 3], |a, (c, n)| [a[0] + c.is_some() as i64, a[1] + (n == Some("UpMod")) as i64, a[2] + (n == Some("DownMod")) as i64]);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select(&rp))
        .fold([0i64; 4], |a, r| [a[0] + 1, a[1] + r[0], a[2] + r[1], a[3] + r[2]]);
    rows(drain(&ts).into_iter().map(|(t, a)| row(vec![V::S(t), V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), avg(a[3], a[0]), V::S("Active")])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RowNum,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v WHERE v.CreationDate >= DATE('2024-10-01') - INTERVAL '30 days' GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, ur.Reputation, ur.BadgeCount, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges,
//        COALESCE(rv.VoteCount, 0) AS TotalVotes, COALESCE(rv.UpVotes, 0) AS TotalUpVotes, COALESCE(rv.DownVotes, 0) AS TotalDownVotes
// FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId
// WHERE ur.Reputation > 1000 AND rp.RowNum = 1 ORDER BY rp.CreationDate DESC;
//
// A tie on CreationDate for an owner's newest question goes to the smaller post id.
fn q32773(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let rv = db.vote.with((&db.vote.creation_date).ge(add_days(date(2024, 10, 1), -30))).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| {
        [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]
    });
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and(&ub)).and((&rv).opt())));
    rows(v.into_iter().map(|(p, ((u, b), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(user_col(db, u, "rep"));
        f.extend(b.map(V::I));
        f.extend(r.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 END), 0) AS Downvotes,
//        COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, Upvotes, Downvotes, TotalPosts, TotalComments, RANK() OVER (ORDER BY Upvotes DESC, Downvotes ASC) AS UserRank FROM UserActivity),
// UserBadges AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges GROUP BY UserId),
// FinalStats AS (SELECT ru.UserId, ru.DisplayName, ru.Upvotes, ru.Downvotes, ru.TotalPosts, ru.TotalComments, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ru.UserRank
//     FROM RankedUsers ru LEFT JOIN UserBadges ub ON ru.UserId = ub.UserId)
// SELECT fs.DisplayName, (fs.Upvotes - fs.Downvotes) AS NetUpvotes, fs.TotalPosts, fs.TotalComments, (fs.GoldBadges + fs.SilverBadges + fs.BronzeBadges) AS TotalBadges, fs.UserRank
// FROM FinalStats fs WHERE fs.TotalPosts > 10 ORDER BY NetUpvotes DESC, fs.TotalPosts DESC LIMIT 10;
fn q1833(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, p| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = ranked(drain((&s).and(&pc).and(&cc)), |&(_, ((a, _), _))| (Reverse(a[0]), a[1]), false);
    let v: Vec<_> = drain(rel(v).filt(|((_, ((_, n), _)), _)| n > 10)).into_iter().map(|x| x.1).collect();
    let v = top_n(v, |&((u, ((a, n), _)), _)| (Reverse(a[0] - a[1]), Reverse(n), u), 10);
    let top = rel(v);
    type T = ((Id<User>, (([i64; 2], i64), i64)), i64);
    let v = drain((&top).select(Same::<T>::new().and(Same::<T>::new().map(|((u, _), _): T| u).select((&ub).opt()))));
    rows(v.into_iter().map(|(_, (((u, ((a, n), c)), r), b))| {
        row(vec![user_col(db, u, "name"), V::I(a[0] - a[1]), V::I(n), V::I(c), b.map_or(V::Null, |b| V::I(b[0] + b[1] + b[2])), V::I(r)])
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS RN
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Posts A ON P.Id = A.ParentId JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName, P.PostTypeId),
// RecentPosts AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.OwnerDisplayName, RP.CommentCount, RP.AnswerCount FROM RankedPosts RP WHERE RP.RN <= 10),
// VoteSummary AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// FinalResult AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.OwnerDisplayName, RP.CommentCount, RP.AnswerCount, COALESCE(VS.UpVotes, 0) AS TotalUpVotes, COALESCE(VS.DownVotes, 0) AS TotalDownVotes
//     FROM RecentPosts RP LEFT JOIN VoteSummary VS ON RP.PostId = VS.PostId)
// SELECT FR.PostId, FR.Title, FR.CreationDate, FR.OwnerDisplayName, FR.CommentCount, FR.AnswerCount, FR.TotalUpVotes, FR.TotalDownVotes FROM FinalResult FR ORDER BY FR.CreationDate DESC;
//
// RN reads only base columns, so the ten newest questions are picked first and the comment x answer product is driven for those alone.
fn q8885(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(children_of(db).opt())).fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&s).and(&vs));
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.Score > 0),
// ActiveUsers AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE u.LastAccessDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '3 months' GROUP BY u.Id),
// TopBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b WHERE b.Date >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY b.UserId HAVING COUNT(*) >= 5),
// PostInteractions AS (SELECT rp.PostId, rp.Title, rp.ViewCount, ru.UserId, ru.PostCount, ru.UpVotes, ru.DownVotes, tb.BadgeCount FROM RankedPosts rp JOIN ActiveUsers ru ON ru.PostCount > 5
//     LEFT JOIN TopBadges tb ON tb.UserId = ru.UserId)
// SELECT pi.PostId, pi.Title, pi.ViewCount, pi.PostCount, pi.UpVotes, pi.DownVotes, COALESCE(pi.BadgeCount, 0) AS BadgeCount FROM PostInteractions pi WHERE COALESCE(pi.BadgeCount, 0) >= 1
// ORDER BY pi.ViewCount DESC, pi.PostCount DESC;
//
// The ON condition names only ru, so the posts are crossed with the active users; Rank is never read. A user passes the WHERE only with a TopBadges row.
fn q6690(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, score, .. } = &db.post;
    let rp: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(t0, -1)).and(score.gt(0))).collect();
    let au = db
        .user
        .with((&db.user.last_access_date).ge(add_months(t0, -3)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 3], |a, p| match p {
            Some(t) => [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let tb = db.badge.with((&db.badge.date).ge(add_years(t0, -1))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ru = rel(drain((&au).filt(|a| a[0] > 5).and((&tb).filt(|n| n >= 5))));
    let mut v = Vec::new();
    (&rp).cross(&ru).drive(|_, (p, (_, (a, b)))| v.push((p, a, b)));
    rows(v.into_iter().map(|(p, a, b)| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend(a.map(V::I));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadgeStats AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostEngagement AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, ubs.UserId, ubs.BadgeCount, ubs.GoldBadges, ubs.SilverBadges, ubs.BronzeBadges,
//        pe.Upvotes, pe.Downvotes, pe.CommentCount AS EngagementComments
// FROM RankedPosts rp JOIN UserBadgeStats ubs ON rp.PostId IN (SELECT AcceptedAnswerId FROM Posts WHERE AcceptedAnswerId IS NOT NULL) JOIN PostEngagement pe ON rp.PostId = pe.PostId
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.CommentCount DESC;
//
// The ON condition names only rp, so each top post that is some question's accepted answer is crossed with every user. The IN list holds raw ids.
// A Score tie at the fifth place goes to the smaller post id.
fn q6710(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, origid, accepted_answer_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let acc: MatSet<i64> = db.post.select(accepted_answer_id).map(|a| a).collect();
    let acc_idx: HashIdx<i64, i64> = (&acc).map(|a| a).inv().collect();
    let pe = (&tp)
        .with(origid.select(&acc_idx))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let ubs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let mut v = Vec::new();
    (&pe).cross(&ubs).drive(|(p, u), (e, b)| v.push((p, u, e, b)));
    rows(v.into_iter().map(|(p, u, e, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments"]);
        f.push(user_col(db, u, "uid"));
        f.extend(b.map(V::I));
        f.extend(e.map(V::I));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPosts FROM Posts p GROUP BY p.OwnerUserId),
// UserPostPerformance AS (SELECT ub.DisplayName, ub.BadgeCount, ps.PostCount, ps.AverageScore, ps.TotalViews, ps.ClosedPosts, RANK() OVER (ORDER BY ps.PostCount DESC) AS PostRank
//     FROM UserBadgeStats ub JOIN PostStatistics ps ON ub.UserId = ps.OwnerUserId)
// SELECT upp.DisplayName, upp.BadgeCount, upp.PostCount, upp.AverageScore, upp.TotalViews, upp.ClosedPosts, CASE WHEN upp.ClosedPosts > 0 THEN TRUE ELSE FALSE END AS HasClosedPosts,
//        CASE WHEN upp.PostCount > 0 THEN CAST(upp.ClosedPosts AS DECIMAL) / upp.PostCount ELSE NULL END AS ClosedPostPercentage
// FROM UserPostPerformance upp WHERE upp.BadgeCount > 1 AND upp.PostCount > 5 ORDER BY upp.PostRank, upp.DisplayName;
fn q4945(db: &'static So) -> String {
    let Post { owner_user, score, view_count, closed_date, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt()).and(closed_date.opt())).fold([0i64; 4], |a, ((s, w), c)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + c.is_some() as i64]);
    let v = drain((&bc).and(&ps));
    let v = ranked(v, |&(_, (_, a))| Reverse(a[0]), false);
    let v: Vec<_> = drain(rel(v).filt(|((_, (b, a)), _)| b > 1 && a[0] > 5)).into_iter().map(|x| x.1).collect();
    rows(v.into_iter().map(|((u, (b, a)), _)| {
        row(vec![user_col(db, u, "name"), V::I(b), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::B(a[3] > 0), V::F(a[3] as f64 / a[0] as f64)])
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(COALESCE(v.UpVotes, 0)) AS TotalUpVotes, SUM(COALESCE(v.DownVotes, 0)) AS TotalDownVotes,
//        ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes GROUP BY PostId) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes FROM UserPostStats WHERE UserRank <= 10),
// PostHistories AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 24 THEN ph.Id END) AS EditCount,
//        MAX(ph.CreationDate) AS LastChangeDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT u.DisplayName, u.TotalPosts, u.TotalQuestions, ph.CloseReopenCount, ph.EditCount, ph.LastChangeDate, CASE WHEN ph.CloseReopenCount > 0 THEN 'Closed/Reopened' ELSE 'Active' END AS Status
// FROM TopUsers u LEFT JOIN PostHistories ph ON u.UserId = ph.PostId ORDER BY u.TotalPosts DESC, u.TotalUpVotes DESC;
//
// `u.UserId = ph.PostId` joins a user id to a post id, so it goes through the raw ids. A TotalPosts tie at the tenth place goes to the smaller user id.
fn q523(db: &'static So) -> String {
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |n, t| n + (t == 2) as i64);
    let s = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and((&pv).opt()))).fold([0i64; 3], |a, (t, u)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + u.unwrap_or(0)]);
    let tu = top_n(drain(&s), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu = rel(tu);
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 3])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let PostHistory { post_id, post_history_type_id, creation_date, .. } = &db.post_history;
    let ph = db.post_history.group_by(post_id).select(post_history_type_id.and(creation_date)).fold([0i64, 0, i64::MIN], |a, (t, d)| [a[0] + matches!(t, 10 | 11) as i64, a[1] + (t == 24) as i64, a[2].max(d)]);
    let v = drain((&by_user).and((&db.user.origid).select(&ph).opt()));
    rows(v.into_iter().map(|(u, ((_, a), h))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])];
        match h {
            Some(h) => f.extend([V::I(h[0]), V::I(h[1]), V::T(h[2]), V::S(if h[0] > 0 { "Closed/Reopened" } else { "Active" })]),
            None => f.extend([V::Null, V::Null, V::Null, V::S("Active")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        p.OwnerUserId FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.GoldBadges, us.SilverBadges, us.BronzeBadges, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount
//     FROM UserStatistics us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId WHERE rp.Rank <= 5)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, tu.Title, tu.CreationDate, tu.Score, tu.ViewCount, tu.AnswerCount, tu.CommentCount
// FROM TopUsers tu ORDER BY tu.Reputation DESC, tu.Score DESC;
//
// UserStatistics is driven only for the owners of the ranked questions, the only users that join.
fn q9718(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, (_, c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&pc).and(&us))));
    rows(v.into_iter().map(|(p, ((u, n), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["title", "created", "score", "views", "answers", "comments"]));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("31347", q31347),
    ("32217", q32217),
    ("1097", q1097),
    ("971", q971),
    ("9952", q9952),
    ("33610", q33610),
    ("20553", q20553),
    ("769", q769),
    ("2713", q2713),
    ("4965", q4965),
    ("189", q189),
    ("3967", q3967),
    ("4466", q4466),
    ("8994", q8994),
    ("32707", q32707),
    ("68", q68),
    ("3084", q3084),
    ("5636", q5636),
    ("761", q761),
    ("872", q872),
    ("4357", q4357),
    ("30026", q30026),
    ("30254", q30254),
    ("33034", q33034),
    ("1616", q1616),
    ("3236", q3236),
    ("1480", q1480),
    ("6834", q6834),
    ("374", q374),
    ("5401", q5401),
    ("25176", q25176),
    ("26933", q26933),
    ("1538", q1538),
    ("4579", q4579),
    ("4820", q4820),
    ("1969", q1969),
    ("2089", q2089),
    ("2879", q2879),
    ("31356", q31356),
    ("6758", q6758),
    ("28938", q28938),
    ("32086", q32086),
    ("7793", q7793),
    ("30354", q30354),
    ("5850", q5850),
    ("30325", q30325),
    ("8472", q8472),
    ("6179", q6179),
    ("7017", q7017),
    ("510", q510),
    ("1274", q1274),
    ("28592", q28592),
    ("6568", q6568),
    ("9173", q9173),
    ("310", q310),
    ("1585", q1585),
    ("4428", q4428),
    ("22577", q22577),
    ("31093", q31093),
    ("7566", q7566),
    ("4796", q4796),
    ("32779", q32779),
    ("3942", q3942),
    ("6186", q6186),
    ("2349", q2349),
    ("7073", q7073),
    ("7567", q7567),
    ("34497", q34497),
    ("3147", q3147),
    ("3799", q3799),
    ("23065", q23065),
    ("1681", q1681),
    ("20546", q20546),
    ("29993", q29993),
    ("27008", q27008),
    ("31938", q31938),
    ("7752", q7752),
    ("26534", q26534),
    ("8332", q8332),
    ("33652", q33652),
    ("2490", q2490),
    ("27449", q27449),
    ("6034", q6034),
    ("1042", q1042),
    ("1381", q1381),
    ("33027", q33027),
    ("1291", q1291),
    ("2219", q2219),
    ("28434", q28434),
    ("32773", q32773),
    ("1833", q1833),
    ("8885", q8885),
    ("6690", q6690),
    ("6710", q6710),
    ("4945", q4945),
    ("523", q523),
    ("9718", q9718),
];
