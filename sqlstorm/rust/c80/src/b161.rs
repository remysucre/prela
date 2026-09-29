use harness::prelude::*;
use std::cmp::Reverse;

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS Upvotes, COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS Downvotes,
//        COUNT(V.Id) AS TotalVotes, AVG(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AvgUpvote, AVG(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AvgDownvote
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// ClosedPostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS ClosedPosts, SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS ClosedByUser,
//        SUM(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenedByUser FROM Posts P JOIN PostHistory PH ON PH.PostId = P.Id
//     WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY P.OwnerUserId),
// UserBadgeStats AS (SELECT U.Id AS UserId, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges, COUNT(B.Id) AS TotalBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// FinalStats AS (SELECT UV.UserId, UV.DisplayName, COALESCE(CPS.ClosedPosts, 0) AS ClosedPosts, COALESCE(CPS.ClosedByUser, 0) AS ClosedByUser,
//        COALESCE(CPS.ReopenedByUser, 0) AS ReopenedByUser, COALESCE(UBS.GoldBadges, 0) AS GoldBadges, COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
//        COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges, UV.Upvotes, UV.Downvotes, UV.TotalVotes, UV.AvgUpvote, UV.AvgDownvote
//     FROM UserVoteStats UV LEFT JOIN ClosedPostStats CPS ON UV.UserId = CPS.OwnerUserId LEFT JOIN UserBadgeStats UBS ON UV.UserId = UBS.UserId)
// SELECT *, CASE WHEN ClosedPosts > 0 THEN 'Active Participant' ELSE 'Newcomer' END AS UserCategory,
//        CASE WHEN GoldBadges >= 3 THEN 'Super User' WHEN SilverBadges >= 5 THEN 'Contributing Member' ELSE 'Member' END AS BadgeCategory,
//        (CASE WHEN TotalVotes > 0 THEN CAST(Upvotes AS FLOAT) / TotalVotes ELSE NULL END) AS UpvoteRatio,
//        (SELECT STRING_AGG(DISTINCT T.TagName, ', ') FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' WHERE P.OwnerUserId = FinalStats.UserId) AS AssociatedTags
// FROM FinalStats WHERE Upvotes > Downvotes ORDER BY UpvoteRatio DESC NULLS LAST, DisplayName ASC;
//
// The STRING_AGG has no ORDER BY; the port joins the names sorted.
fn q24494(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 4], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64, a[3] + 1]
    });
    let ph = &db.post_history;
    let cps = db
        .post_history
        .with((&ph.post_history_type_id).is_in([10, 11]))
        .group_by((&ph.post).select(&db.post.owner_user))
        .select(&ph.post_history_type_id)
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 10) as i64, a[2] + (t == 11) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    type R = (Id<Post>, Id<Tag>);
    let lt = tag_mentions(db);
    let tags = (&lt)
        .group_by(Same::<R>::new().map(|(p, _): R| p).select(&db.post.owner_user))
        .select(Same::<R>::new().map(|(_, t): R| t).select(&db.tag.tag_name))
        .buf_fold(|it| {
            let mut n: Vec<Str> = it.into_iter().collect();
            n.sort_unstable();
            n.dedup();
            &*Box::leak(n.join(", ").into_boxed_str())
        });
    let v = drain((&uv).filt(|a| a[0] > a[1]).and((&cps).opt()).and(&ub).and((&tags).opt()));
    rows(v.into_iter().map(|(u, (((a, c), b), t))| {
        let c = c.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(c.map(V::I));
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(a[0] as f64 / a[3] as f64), V::F(a[1] as f64 / a[3] as f64)]);
        f.push(V::S(if c[0] > 0 { "Active Participant" } else { "Newcomer" }));
        f.push(V::S(if b[0] >= 3 { "Super User" } else if b[1] >= 5 { "Contributing Member" } else { "Member" }));
        f.push(if a[2] > 0 { V::F(a[0] as f32 as f64 / a[2] as f64) } else { V::Null });
        f.push(ostr(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND p.Score > 0),
// TopPosts AS (SELECT r.PostId, r.Title, r.Score, r.ViewCount, pt.Name AS PostTypeName FROM RankedPosts r JOIN PostTypes pt ON r.Rank <= 5 AND r.PostTypeId = pt.Id)
// SELECT p.OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, AVG(u.Reputation) AS AverageReputation
// FROM TopPosts tp LEFT JOIN Posts p ON tp.PostId = p.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
// LEFT JOIN Users u ON p.OwnerUserId = u.Id GROUP BY p.OwnerDisplayName ORDER BY TotalBounties DESC, CommentCount DESC LIMIT 10;
fn q9336(db: &'static So) -> String {
    let Post { post_type, creation_date, score, owner_display_name, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(score.gt(0))).select(post_type));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let g = (&tp)
        .group_by(owner_display_name.opt())
        .select(comments_of(db).opt().and(bounty.opt()).and(owner_user.select(&db.user.reputation).opt()))
        .fold([0i64; 3], |a, ((_, b), r)| [a[0] + b.flatten().unwrap_or(0), a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0)]);
    let dc = (&tp).group_by(owner_display_name.opt()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&g).and(&dc)), |&(n, (a, c))| (Reverse(a[0]), Reverse(c), n), 10);
    rows(v.into_iter().map(|(n, (a, c))| row(vec![ostr(n), V::I(c), V::I(a[0]), avg(a[2], a[1])])))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, COUNT(v.Id) AS VoteCount, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(DISTINCT bh.Id) AS EditHistoryCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory bh ON p.Id = bh.PostId GROUP BY p.Id, p.Title),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.VoteCount, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.EditHistoryCount,
//        ROW_NUMBER() OVER (ORDER BY ps.VoteCount DESC, ps.CommentCount DESC) AS RowNum FROM PostStatistics ps)
// SELECT tp.* FROM TopPosts tp WHERE tp.RowNum <= 10;
fn q12995(db: &'static So) -> String {
    let s = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 4], |a, ((t, c), _)| [a[0] + t.is_some() as i64, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let hc = db.post.group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = top_n(drain((&s).and(&hc)), |&(p, (a, _))| (Reverse(a[0]), Reverse(a[1]), p), 10);
    rows(v.into_iter().enumerate().map(|(i, (p, (a, h)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(a.map(V::I));
        f.extend([V::I(h), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, Questions, Answers, TotalViews, TotalScore, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, Questions, Answers, TotalViews, TotalScore, Rank FROM TopUsers WHERE Rank <= 10;
fn q12354(db: &'static So) -> String {
    let v = top_n(drain(&user_posts(db)), |&(u, a)| (Reverse(a[4]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalPostScore,
//        SUM(COALESCE(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END, 0)) AS QuestionCount, SUM(COALESCE(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AnswerCount,
//        SUM(COALESCE(CASE WHEN P.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END, 0)) AS WikiCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, TotalPostScore, QuestionCount, AnswerCount, WikiCount, ROW_NUMBER() OVER (ORDER BY TotalPostScore DESC) AS Rank FROM UserStats)
// SELECT UserId, Reputation, PostCount, TotalPostScore, QuestionCount, AnswerCount, WikiCount, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY TotalPostScore DESC;
fn q10426(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + matches!(t, 3 | 4 | 5) as i64],
        None => a,
    });
    let v = top_n(drain(&s), |&(u, a)| (Reverse(a[1]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, a))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH TagSplit AS (SELECT Id AS PostId, UNNEST(string_to_array(substring(Tags, 2, length(Tags)-2), '><')) AS Tag FROM Posts WHERE PostTypeId = 1),
// TagSummary AS (SELECT Tag, COUNT(*) AS PostCount, SUM(ViewCount) AS TotalViews, AVG(Score) AS AverageScore FROM TagSplit JOIN Posts ON TagSplit.PostId = Posts.Id GROUP BY Tag),
// TopTags AS (SELECT Tag, PostCount, TotalViews, AverageScore, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagSummary)
// SELECT T.Tag, T.PostCount, T.TotalViews, T.AverageScore, (SELECT COUNT(*) FROM Posts P WHERE P.Tags LIKE '%' || T.Tag || '%') AS TotalPosts,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId IN (SELECT PostId FROM TagSplit WHERE Tag = T.Tag)) AS CommentCount
// FROM TopTags T WHERE T.TagRank <= 10 ORDER BY T.TagRank;
fn q29609(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, view_count, .. } = &db.post;
    type R = (Id<Post>, Str);
    let pairs: MatSet<R> = db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list))).collect();
    let tag = || Same::<R>::new().map(|(_, t): R| t);
    let post = || Same::<R>::new().map(|(p, _): R| p);
    let sm = (&pairs).group_by(tag()).select(post().select(score.and(view_count.opt()))).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let top: Vec<_> = ranked(drain(&sm), |&(_, a)| Reverse(a[0]), false).into_iter().take_while(|x| x.1 <= 10).collect();
    let tv = rel(top.into_iter().map(|((t, a), r)| (t, a, r)).collect());
    let tidx: HashIdx<Str, (Str, [i64; 4], i64)> = (&tv).map(|(t, _, _)| t).inv().select(&tv).collect();
    let elems: MatSet<Str> = tags_str.flat_map(tag_list).collect();
    let contains: HashIdx<Str, Str> = (&elems).select_where((&tidx).map(|(t, _, _)| t), |e: Str, t: Str| e.contains(t)).collect();
    let hits: MatSet<R> = db.post.select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).select(&contains))).collect();
    let tp = (&hits).group_by(tag()).select(post()).fold(0i64, |n, _| n + 1);
    let cc = (&pairs).group_by(tag()).select(post().select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let mut v = drain((&tidx).and((&tp).opt()).and((&cc).opt()));
    v.sort_by_key(|&(t, (((_, _, r), _), _))| (r, t));
    rows(v.into_iter().map(|(t, (((_, a, _), n), c))| row(vec![V::S(t), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), V::I(n.unwrap_or(0)), V::I(c.unwrap_or(0))])))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPosts, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 9 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpvotedPosts, DownvotedPosts, TotalBounty, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank
//     FROM UserReputation WHERE PostCount > 0)
// SELECT TU.Rank, TU.DisplayName, TU.Reputation, TU.PostCount, TU.UpvotedPosts, TU.DownvotedPosts, TU.TotalBounty FROM TopUsers TU WHERE TU.Rank <= 10 ORDER BY TU.Rank;
fn q5477(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(bounty.opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((s, b)) => [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let pc = user_distinct_posts(db);
    let v = top_n(drain((&s).and((&pc).filt(|n| n > 0))), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (a, n)))| {
        let mut f = vec![V::I(i as i64 + 1)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
//     FROM UserPostStats WHERE TotalPosts > 0)
// SELECT T.DisplayName, T.TotalPosts, T.TotalQuestions, T.TotalAnswers, T.TotalScore, T.TotalViews, RANK() OVER (ORDER BY T.TotalViews DESC) AS ViewRank
// FROM TopUsers T WHERE T.ScoreRank <= 10 ORDER BY T.ScoreRank;
fn q7079(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain((&ups).filt(|a| a[1] > 0)), |&(_, a)| Reverse(a[4]), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).collect();
    let v = ranked(v, |&((_, a), _)| Reverse(a[6]), false);
    rows(v.into_iter().map(|(((u, a), _), w)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6]), V::I(w)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalViews, QuestionCount, AnswerCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.PostCount, u.TotalViews, u.QuestionCount, u.AnswerCount FROM TopUsers u WHERE u.ReputationRank <= 10 ORDER BY u.Reputation DESC;
fn q7878(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain(&ups), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[6]), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, QuestionCount, AnswerCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPostStats)
// SELECT u.DisplayName, u.Reputation, u.CreationDate, u.Location, u.Views, t.QuestionCount, t.AnswerCount, t.TotalScore, COALESCE(b.Name, 'No Badge') AS BadgeName,
//        COALESCE(b.Class, 0) AS BadgeClass
// FROM TopUsers t JOIN Users u ON u.Id = t.UserId LEFT JOIN Badges b ON b.UserId = u.Id AND b.Class = 1 WHERE t.Rank <= 10 ORDER BY t.TotalScore DESC;
fn q4907(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain(&ups), |&(_, a)| Reverse(a[4]), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let v = drain((&tu).select((&ups).and(gold.opt())));
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep", "ucreated"]);
        f.extend([ostr(db.user.location.get(u)), user_col(db, u, "uviews"), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.extend(match b {
            Some(b) => [V::S(db.badge.name.get(b).unwrap()), V::I(db.badge.class.get(b).unwrap())],
            None => [V::S("No Badge"), V::I(0)],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties, COUNT(DISTINCT V.PostId) AS BountyCount,
//        COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        DENSE_RANK() OVER (ORDER BY COALESCE(SUM(V.BountyAmount), 0) DESC) AS BountyRank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId AND V.VoteTypeId = 8 LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// RankedUsers AS (SELECT *, ROW_NUMBER() OVER (ORDER BY TotalBounties DESC, PostCount DESC) AS RowNum FROM UserStats)
// SELECT R.UserId, R.DisplayName, R.TotalBounties, R.BountyCount, R.QuestionCount, R.AnswerCount, R.BountyRank FROM RankedUsers R WHERE R.RowNum <= 10 ORDER BY R.TotalBounties DESC;
fn q4750(db: &'static So) -> String {
    let Vote { vote_type_id, post_id, bounty_amount, .. } = &db.vote;
    let bv = || votes_by(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8)));
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(bv().select(bounty_amount.opt()).opt().and(posts_of(db).select(&db.post.post_type_id).opt()))
        .fold([0i64; 3], |a, (b, t)| [a[0] + b.flatten().unwrap_or(0), a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]);
    let bc = db.user.group_by(Ident::<User>::new()).select(bv().select(post_id).opt()).buf_fold(distinct_some);
    let pc = user_distinct_posts(db);
    let v = ranked(drain((&s).and(&bc).and(&pc)), |&(_, ((a, _), _))| Reverse(a[0]), true);
    let v = top_n(v, |&((u, ((a, _), n)), _)| (Reverse(a[0]), Reverse(n), u), 10);
    rows(v.into_iter().map(|((u, ((a, b), _)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(b), V::I(a[1]), V::I(a[2]), V::I(r)]);
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount, COUNT(DISTINCT c.Id) AS CommentCount, AVG(u.Reputation) AS AverageUserReputation
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE t.Count > 100 GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, TotalViews, QuestionCount, AnswerCount, CommentCount, AverageUserReputation, ROW_NUMBER() OVER (ORDER BY TotalViews DESC) AS Rank FROM TagStats)
// SELECT TagName, PostCount, TotalViews, QuestionCount, AnswerCount, CommentCount, AverageUserReputation FROM TopTags WHERE Rank <= 10;
fn q27384(db: &'static So) -> String {
    let Post { view_count, owner_user, post_type_id, .. } = &db.post;
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let posts = || (&by_tag).map(|(p, _)| p);
    let tags = || db.tag.with((&db.tag.count).gt(100)).group_by(&db.tag.tag_name);
    let s = tags()
        .select(posts().select(view_count.opt().and(comments_of(db).opt()).and(owner_user.select(&db.user.reputation).opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some(((w, _), r)) => [a[0] + w.unwrap_or(0), a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0)],
            None => a,
        });
    let pc = tags().select(posts().select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let cc = tags().select(posts().select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(&pc).and(&cc)), |&(t, ((a, _), _))| (Reverse(a[0]), t), 10);
    rows(v.into_iter().map(|(t, ((a, p), c))| row(vec![V::S(t), V::I(p[0]), V::I(a[0]), V::I(p[1]), V::I(p[2]), V::I(c), avg(a[2], a[1])])))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN PT.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN PT.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation)
// SELECT U.DisplayName, U.Reputation, U.PostCount, U.AnswerCount, U.QuestionCount, (SELECT COUNT(*) FROM TopUsers) AS TotalUsers,
//        ROUND((CAST(U.Reputation AS DECIMAL) / NULLIF((SELECT MAX(Reputation) FROM TopUsers), 0)) * 100, 2) AS ReputationPercentage
// FROM TopUsers U WHERE U.Rank <= 10 ORDER BY U.Reputation DESC;
fn q6812(db: &'static So) -> String {
    let s = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(ptype_name(db)).opt()).fold([0i64; 3], |a, n| match n {
        Some(n) => [a[0] + 1, a[1] + (n == "Answer") as i64, a[2] + (n == "Question") as i64],
        None => a,
    });
    let total = count(&db.user.reputation);
    let max = (&db.user.reputation).fold_flat(i64::MIN, |m, r| m.max(r));
    let v = top_n(drain(&s), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::I(total));
        f.push(if max == 0 { V::Null } else { V::F((r as f64 / max as f64 * 100.0 * 100.0).round() / 100.0) });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount, COALESCE(b.Name, 'No Badge') AS BadgeName
// FROM TopPosts tp LEFT JOIN Badges b ON tp.PostId = b.UserId ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the top posts are picked first. `tp.PostId = b.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q7852(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let v = drain((&s).and(&vc).and(origid.select(&bidx).opt()));
    rows(v.into_iter().map(|(p, ((c, n), b))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), V::I(n), V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap()))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(COUNT(DISTINCT P.Id), 0) AS PostCount, COALESCE(COUNT(DISTINCT C.Id), 0) AS CommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
//     WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// MostActiveUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, PostCount, CommentCount, RANK() OVER (ORDER BY PostCount DESC, UpVotes DESC) AS Rank FROM UserStats)
// SELECT U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, U.PostCount, U.CommentCount FROM MostActiveUsers U WHERE U.Rank <= 10 ORDER BY U.Rank;
//
// `V.UserId = U.Id` with `P.OwnerUserId = U.Id` keeps the votes cast by the post's owner (`own_votes`).
fn q8147(db: &'static So) -> String {
    let own = own_votes(db);
    let users = || db.user.with((&db.user.reputation).gt(0)).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let cc = users().select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&pc).and(&cc)), |&(_, ((a, p), _))| (Reverse(p), Reverse(a[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, p), c)), _)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(p), V::I(c)]);
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.VoteCount, ps.UpVotes, ps.DownVotes, ROW_NUMBER() OVER (ORDER BY ps.UpVotes DESC) AS Rank FROM PostStatistics ps)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.CommentCount, tp.VoteCount, tp.UpVotes, tp.DownVotes FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Rank;
fn q11924(db: &'static So) -> String {
    let recent = || db.post.with((&db.post.creation_date).ge(add_years(current_date(), -1))).group_by(Ident::<Post>::new());
    let s = recent()
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let vc = recent().select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = top_n(drain((&s).and(&vc)), |&(p, (a, _))| (Reverse(a[1]), p), 10);
    rows(v.into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, CommentCount, VoteCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY VoteCount DESC, UpVotes DESC) AS Rank FROM PostStats)
// SELECT * FROM TopPosts WHERE Rank <= 10;
fn q14475(db: &'static So) -> String {
    let recent = || db.post.with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(Ident::<Post>::new());
    let s = recent()
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let vc = recent().select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = top_n(drain((&s).and(&vc)), |&(p, (a, n))| (Reverse(n), Reverse(a[1]), p), 10);
    rows(v.into_iter().enumerate().map(|(i, (p, (a, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2]), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, CommentCount, VoteCount, TotalScore, BadgeCount, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, CommentCount, VoteCount, TotalScore, BadgeCount FROM TopUsers WHERE Rank <= 10;
fn q14695(db: &'static So) -> String {
    let users = || db.user.group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).opt()).and(votes_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, b)| [a[0] + p.map_or(0, |((s, _), _)| s), a[1] + b.is_some() as i64]);
    let pc = user_distinct_posts(db);
    let cc = users().select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let vc = users().select(posts_of(db).select(votes_of(db)).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(&pc).and(&cc).and(&vc)), |&(u, (((a, _), _), _))| (Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (((a, p), c), n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p), V::I(c), V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBountyAmount, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, TotalBountyAmount, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, TotalBountyAmount, BadgeCount, ReputationRank FROM TopUsers WHERE ReputationRank <= 10;
//
// ReputationRank reads only Reputation, so the top users are picked first and the product is driven for them alone.
fn q12422(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tr: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let users = || (&tr).map(|(u, _)| u).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, _)| match p {
            Some((t, b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let bc = users().select(badges_of(db).opt()).buf_fold(distinct_some);
    let v = drain((&tr).and(&s).and(&pc).and(&bc));
    rows(v.into_iter().map(|(u, ((((_, r), a), p), b))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.QuestionCount, tu.TotalBounties, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score
// FROM TopUsers tu LEFT JOIN RankedPosts rp ON tu.UserId = rp.PostId WHERE rp.Rank = 1 AND rp.ViewCount IS NOT NULL ORDER BY tu.Reputation DESC, rp.Score DESC;
//
// `tu.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q31429(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, view_count, origid, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| {
        (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p)
    }, 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&first).with(view_count).select(origid).inv().collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let users = || db.user.with((&db.user.reputation).gt(1000)).with((&db.user.origid).select(&pidx));
    let s = users().group_by(Ident::<User>::new()).select(qs().select(bounty.opt()).opt()).fold(0i64, |n, b| n + b.flatten().flatten().unwrap_or(0));
    let qc = users().group_by(Ident::<User>::new()).select(qs().opt()).buf_fold(distinct_some);
    let v = drain((&s).and(&qc).and((&db.user.origid).select(&pidx)));
    rows(v.into_iter().map(|(u, ((b, q), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(q), V::I(b)]);
        f.extend(post_fields(db, p, &["title", "created", "views", "score"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(V.BountyAmount) AS TotalBounty FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, PostCount, AnswerCount, TotalBounty, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT TU.DisplayName, TU.Reputation, TU.Views, TU.PostCount, TU.AnswerCount, COALESCE(B.Name, 'No Badge') AS Badge,
//        CASE WHEN TU.TotalBounty > 100 THEN 'High Bounty Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM TopUsers TU LEFT JOIN Badges B ON TU.UserId = B.UserId AND B.Class = 1 WHERE TU.Rank <= 10 ORDER BY TU.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the product is driven for them alone.
fn q447(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let users = || (&tu).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + (t == 2) as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
            }
            None => a,
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let v = drain((&s).and(&pc).and(gold.opt()));
    rows(v.into_iter().map(|(u, ((a, p), b))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.extend([V::I(p), V::I(a[0]), V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap()))]);
        f.push(V::S(if a[1] > 0 && a[2] > 100 { "High Bounty Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews,
//        SUM(CASE WHEN P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' THEN 1 ELSE 0 END) AS RecentPosts
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalScore, TotalViews, RecentPosts, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStats)
// SELECT U.DisplayName, U.TotalPosts, U.Questions, U.Answers, U.TotalScore, U.TotalViews, U.RecentPosts FROM TopUsers U WHERE U.ScoreRank <= 10 ORDER BY U.TotalScore DESC;
fn q11236(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let since = add_months(ts(2024, 10, 1, 12, 34, 56), -1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).opt())
        .fold([0i64; 7], |a, p| match p {
            Some((((t, s), w), d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + (d >= since) as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (a[0] == 0, Reverse(a[3])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), nullable(a[1], a[0]), nullable(a[2], a[0]), nullable(a[3], a[0]), nullable(a[5], a[4]), nullable(a[6], a[0])])
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(P.ViewCount, 0)) AS TotalViewCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionsCount, AnswersCount, TotalScore, TotalViewCount, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionsCount, AnswersCount, TotalScore, TotalViewCount, UserRank FROM TopUsers WHERE UserRank <= 10 ORDER BY UserRank;
fn q11294(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(0)).select(&ups)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), nullable(a[2], a[0]), nullable(a[3], a[0]), V::I(a[4]), V::I(a[6]), V::I(r)]);
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionsAsked,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswersGiven, COALESCE(SUM(P.Score), 0) AS TotalScore,
//        COALESCE(SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgesCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionsAsked, AnswersGiven, TotalScore, BadgesCount, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserMetrics)
// SELECT TU.DisplayName, TU.Reputation, TU.QuestionsAsked, TU.AnswersGiven, TU.TotalScore, TU.BadgesCount FROM TopUsers TU WHERE TU.ScoreRank <= 10 ORDER BY TU.TotalScore DESC;
fn q8126(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score)).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, s) = p.map_or((0, 0), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + b.is_some() as i64]
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[2]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, UpvoteCount, DownvoteCount, BadgeCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT * FROM TopUsers WHERE Rank <= 10 ORDER BY Reputation DESC;
//
// Rank reads only Reputation, so the top users are picked first and the product is driven for them alone.
fn q13140(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(v.into_iter().map(|x| x.0).collect());
    let tu: MatSet<Id<User>> = (&tu).map(|u| u).collect();
    let users = || (&tu).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(&pc)), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (a, p)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(p));
        f.extend(a.map(V::I));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS Owner, COUNT(DISTINCT c.Id) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId)
// SELECT PostId, Title, CreationDate, Score, ViewCount, Owner, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10 ORDER BY ViewCount DESC, Score DESC;
fn q8010(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(&vc));
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPosts AS (SELECT p.OwnerUserId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days')
// SELECT us.DisplayName, us.Reputation, us.BadgeCount, us.UpVotes, us.DownVotes, rp.Title AS RecentPostTitle, rp.Score AS RecentPostScore
// FROM UserStats us LEFT JOIN RecentPosts rp ON us.UserId = rp.OwnerUserId AND rp.rn = 1 WHERE us.Reputation > 1000 AND us.BadgeCount > 2
// ORDER BY us.Reputation DESC, us.DisplayName LIMIT 10 OFFSET 0;
fn q3590(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new());
    let s = users()
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = users().select(badges_of(db).opt()).buf_fold(distinct_some);
    let since = now_utc() - 30 * DAY_US;
    let recent = drain(db.post.with(creation_date.map(ny_to_utc).ge(since)).select(owner_user));
    let rp = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let last: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let v = drain((&s).and((&bc).filt(|n| n > 2)).and((&last).map(|(_, p)| p).opt()));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), db.user.display_name.get(u).unwrap(), u), 10);
    rows(v.into_iter().map(|(u, ((a, b), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.PostTypeId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS RevisionCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.PostTypeId),
// TopPosts AS (SELECT ps.PostId, ps.PostTypeId, ps.UpVotes, ps.DownVotes, ps.CommentCount, ps.RevisionCount, ROW_NUMBER() OVER (ORDER BY ps.UpVotes DESC, ps.CommentCount DESC) AS Rank
//     FROM PostStats ps WHERE ps.PostTypeId IN (1, 2))
// SELECT tp.PostId, tp.PostTypeId, tp.UpVotes, tp.DownVotes, tp.CommentCount, tp.RevisionCount FROM TopPosts tp WHERE tp.Rank <= 10;
fn q10629(db: &'static So) -> String {
    let posts = || db.post.with((&db.post.post_type_id).is_in([1, 2])).group_by(Ident::<Post>::new());
    let s = posts()
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let hc = posts().select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = top_n(drain((&s).and(&hc)), |&(p, (a, _))| (Reverse(a[0]), Reverse(a[2]), p), 10);
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(h)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, COUNT(DISTINCT P.Id) AS TotalPosts,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, TotalViews, TotalPosts, TotalQuestions, TotalAnswers,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, BadgeCount DESC, TotalViews DESC) AS Rank FROM UserStats)
// SELECT T.DisplayName, T.Reputation, T.BadgeCount, T.TotalViews, T.TotalPosts, T.TotalQuestions, T.TotalAnswers FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Rank;
fn q6807(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(view_count.opt())).opt()))
        .fold([0i64; 4], |a, (b, p)| {
            let (t, w) = p.map_or((0, None), |x| x);
            [a[0] + b.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + (t == 1) as i64, a[3] + (t == 2) as i64]
        });
    let pc = user_distinct_posts(db);
    let v = top_n(drain((&s).and(&pc)), |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(p), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        COALESCE(MAX(ph.CreationDate), '1900-01-01') AS LastEditDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// TopPosts AS (SELECT *, ROW_NUMBER() OVER (ORDER BY Score DESC, CommentCount DESC) AS Rank FROM RankedPosts)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.AnswerCount, tp.LastEditDate, COALESCE(u.DisplayName, 'Deleted User') AS OwnerName,
//        COALESCE(u.Reputation, 0) AS OwnerReputation
// FROM TopPosts tp LEFT JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) WHERE tp.Rank <= 10 ORDER BY tp.Score DESC;
fn q9182(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let posts = || db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).group_by(Ident::<Post>::new());
    let cc = posts().select(comments_of(db).opt()).buf_fold(distinct_some);
    let ac = posts().select(answers_of(db).opt()).buf_fold(distinct_some);
    let md = posts().select(history_of(db).select(&db.post_history.creation_date).opt()).fold(None, |m: Option<i64>, d| match d {
        Some(d) => Some(m.map_or(d, |m| m.max(d))),
        None => m,
    });
    let v = top_n(drain((&cc).and(&ac).and(&md)), |&(p, ((c, _), _))| (Reverse(score.get(p).unwrap()), Reverse(c), p), 10);
    rows(v.into_iter().map(|(p, ((c, a), d))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(a), V::T(d.unwrap_or(date(1900, 1, 1)))]);
        f.extend(match owner_user.get(p) {
            Some(u) => ucols(db, u, &["name", "rep"]),
            None => vec![V::S("Deleted User"), V::I(0)],
        });
        row(f)
    }))
}

// WITH UserVotes AS (SELECT v.UserId, COUNT(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 END) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 END) AS NetVotes
//     FROM Votes v GROUP BY v.UserId),
// PopularPosts AS (SELECT p.Id, p.Title, p.Score, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1),
// PostBadges AS (SELECT b.UserId, COUNT(DISTINCT b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// UserReputation AS (SELECT u.Id, u.Reputation, COALESCE(uv.VoteCount, 0) AS TotalVotes, COALESCE(pb.BadgeCount, 0) AS TotalBadges
//     FROM Users u LEFT JOIN UserVotes uv ON u.Id = uv.UserId LEFT JOIN PostBadges pb ON u.Id = pb.UserId)
// SELECT ur.Id, ur.Reputation, ur.TotalVotes, ur.TotalBadges, pp.Title, pp.Score FROM UserReputation ur JOIN PopularPosts pp ON ur.Reputation > 5000
// WHERE ur.TotalVotes > 50 ORDER BY ur.Reputation DESC, pp.Score DESC LIMIT 10;
//
// The ON clause names only ur, so the users and the questions are crossed.
fn q7459(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold(0i64, |n, t| n + matches!(t, 2 | 3) as i64);
    let pb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ur = db.user.with((&db.user.reputation).gt(5000)).select((&uv).filt(|n| n > 50).and(&pb));
    let ur: HashIdx<Id<User>, (i64, i64)> = ur.collect();
    let qs: MatSet<Id<Post>> = db.post.with((&db.post.post_type_id).eq(1)).collect();
    let mut v = Vec::new();
    (&ur).cross(&qs).drive(|(u, p), (a, _)| v.push((u, p, a)));
    let v = top_n(v, |&(u, p, _)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(db.post.score.get(p).unwrap()), u, p), 10);
    rows(v.into_iter().map(|(u, p, (n, b))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(b)]);
        f.extend(post_fields(db, p, &["title", "score"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, TotalVotes, TotalBadges, RANK() OVER (ORDER BY TotalVotes DESC) AS VoteRank,
//        RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, TotalViews, TotalVotes, TotalBadges, VoteRank, ViewRank FROM TopUsers WHERE VoteRank <= 10 OR ViewRank <= 10 ORDER BY VoteRank, ViewRank;
fn q11377(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let (w, v) = p.map_or((None, None), |x| x);
            [a[0] + w.unwrap_or(0), a[1] + v.is_some() as i64, a[2] + b.is_some() as i64]
        });
    let pc = user_distinct_posts(db);
    let v = ranked(drain((&s).and(&pc)), |&(_, (a, _))| Reverse(a[1]), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[0]), false);
    rows(v.into_iter().filter(|&(((_, _), r), w)| r <= 10 || w <= 10).map(|(((u, (a, p)), r), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r), V::I(w)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.AccountId, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, AVG(P.Score) AS AveragePostScore, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.AccountId),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, AveragePostScore, BadgeCount,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserStats)
// SELECT T.DisplayName, T.Reputation, T.PostCount, T.AnswerCount, T.QuestionCount, T.AveragePostScore, T.BadgeCount FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Reputation DESC, T.PostCount DESC;
fn q6421(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score)).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| match p {
            Some((t, s)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + 1, a[3] + s, a[4] + b.is_some() as i64],
            None => [a[0], a[1], a[2], a[3], a[4] + b.is_some() as i64],
        });
    let pc = user_distinct_posts(db);
    let v = top_n(drain((&s).and(&pc)), |&(u, (_, p))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(p), u), 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(a[4])]);
        row(f)
    }))
}

// WITH UserVotes AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS PostCount, AVG(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount END) AS AvgViews, AVG(CASE WHEN p.Score IS NOT NULL THEN p.Score END) AS AvgScore
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, PostCount, AvgViews, AvgScore, RANK() OVER (ORDER BY UpVotes DESC, PostCount DESC) AS Rank FROM UserVotes)
// SELECT t.DisplayName, t.UpVotes, t.DownVotes, t.PostCount, t.AvgViews, t.AvgScore, CASE WHEN t.Rank <= 10 THEN 'Top 10' WHEN t.Rank <= 50 THEN 'Top 50' ELSE 'Others' END AS UserCategory
// FROM TopUsers t WHERE t.Rank <= 100 ORDER BY t.Rank;
fn q5343(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new());
    let s = users()
        .select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).select(view_count.opt().and(score)).opt())).opt())
        .fold([0i64; 6], |a, v| match v {
            Some((t, p)) => {
                let (w, s) = p.map_or((None, None), |(w, s)| (w, Some(s)));
                [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s.is_some() as i64, a[5] + s.unwrap_or(0)]
            }
            None => a,
        });
    let pc = users().select(votes_by(db).select(&db.vote.post).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&pc)), |&(_, (a, p))| (Reverse(a[0]), Reverse(p)), false);
    rows(v.into_iter().take_while(|x| x.1 <= 100).map(|((u, (a, p)), r)| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(p), avg(a[3], a[2]), avg(a[5], a[4]), V::S(if r <= 10 { "Top 10" } else if r <= 50 { "Top 50" } else { "Others" })])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserAggregates AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id)
// SELECT u.DisplayName, u.Reputation, ua.QuestionCount, ua.TotalBounties, rp.Title AS MostRecentPostTitle, rp.Score
// FROM Users u LEFT JOIN UserAggregates ua ON u.Id = ua.UserId LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId AND rp.PostRank = 1
// WHERE u.Reputation > 1000 ORDER BY ua.QuestionCount DESC, ua.TotalBounties DESC LIMIT 10;
fn q1745(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let users = || db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new());
    let s = users().select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |n, b| n + b.flatten().flatten().unwrap_or(0));
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let recent = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let rp = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let last: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let v = top_n(drain((&s).and(&pc).and((&last).map(|(_, p)| p).opt())), |&(u, ((b, q), _))| (Reverse(q), Reverse(b), u), 10);
    rows(v.into_iter().map(|(u, ((b, q), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(q), V::I(b)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
//        COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, QuestionCount, AnswerCount, CommentCount, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, TotalScore, QuestionCount, AnswerCount, CommentCount, ScoreRank, PostCountRank FROM TopUsers
// WHERE ScoreRank <= 10 OR PostCountRank <= 10 ORDER BY ScoreRank, PostCountRank;
fn q13111(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(comments_of(db).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, s), c)) => [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + c.is_some() as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[1]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[0]), false);
    rows(v.into_iter().filter(|&((_, r), w)| r <= 10 || w <= 10).map(|(((u, a), r), w)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(w)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, TotalPosts, Questions, Answers, TotalViews, TotalUpVotes, TotalDownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT UserId, Reputation, TotalPosts, Questions, Answers, TotalViews, TotalUpVotes, TotalDownVotes, Rank FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only Reputation, so the top users are picked first and the product is driven for them alone.
fn q11643(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let users = || (&tu).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, w), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0), a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(&pc)), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (a, p)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(p));
        f.extend(a.map(V::I));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalBounties, TotalComments, RANK() OVER (ORDER BY TotalPosts DESC, TotalBounties DESC) AS UserRank
//     FROM UserActivity)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalBounties, U.TotalComments FROM TopUsers U WHERE U.UserRank <= 10
// ORDER BY U.TotalPosts DESC, U.TotalBounties DESC;
fn q9043(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let users = || db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some(((t, b), _)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let cc = users().select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&pc).and(&cc)), |&(_, ((a, p), _))| (Reverse(p), Reverse(a[2])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, p), c)), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p), V::I(a[1]), V::I(a[0]), V::I(a[2]), V::I(c)]);
        row(f)
    }))
}

// WITH UserBadgeCount AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(c.CommentCount, 0) AS TotalComments, COALESCE(uc.BadgeCount, 0) AS UserBadgeCount
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId LEFT JOIN UserBadgeCount uc ON p.OwnerUserId = uc.UserId)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.TotalComments, ps.UserBadgeCount,
//        CASE WHEN ps.UserBadgeCount > 5 THEN 'Highly Recognized' WHEN ps.UserBadgeCount BETWEEN 1 AND 5 THEN 'Moderately Recognized' ELSE 'New User' END AS UserRecognition,
//        DENSE_RANK() OVER (ORDER BY ps.Score DESC) AS ScoreRank
// FROM PostStatistics ps WHERE ps.CreationDate >= cast('2024-10-01' as date) - INTERVAL '6 months' AND ps.ViewCount >= 100 ORDER BY ps.Score DESC, ps.ViewCount DESC FETCH FIRST 50 ROWS ONLY;
fn q550(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, score, .. } = &db.post;
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let base = db.post.with(creation_date.ge(add_months(date(2024, 10, 1), -6)).and(view_count.ge(100)));
    let v = ranked(drain(base.select((&cc).opt().and(owner_user.select(&ub).opt()))), |&(p, _)| Reverse(score.get(p).unwrap()), true);
    let v = top_n(v, |&((p, _), _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p), 50);
    rows(v.into_iter().map(|((p, (c, b)), r)| {
        let b = b.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(b), V::S(if b > 5 { "Highly Recognized" } else if b >= 1 { "Moderately Recognized" } else { "New User" }), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(V.BountyAmount) AS TotalBounty, AVG(CASE WHEN C.Score IS NOT NULL THEN C.Score ELSE 0 END) AS AvgCommentScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, TotalBounty, AvgCommentScore, ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank
//     FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, TotalBounty, AvgCommentScore FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only Reputation and PostCount, so the top users are picked first and the vote x comment product is driven for them alone.
fn q8473(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let v = top_n(drain(&pc), |&(u, p)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(p), u), 10);
    let tu = rel(v);
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let s = (&tu)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).and(comments_of(db).select(&db.comment.score).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, b), c)) => {
                let b = b.flatten();
                [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0), a[4] + 1, a[5] + c.unwrap_or(0)]
            }
            None => [a[0], a[1], a[2], a[3], a[4] + 1, a[5]],
        });
    let v = top_n(drain((&tu).and(&s)), |&(u, ((_, p), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(p), u), 10);
    rows(v.into_iter().map(|(u, ((_, p), a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[5], a[4])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(c.Score, 0)) AS TotalCommentScore, SUM(v.BountyAmount) AS TotalBountyAmount, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id GROUP BY u.Id),
// TopUsers AS (SELECT UserId, PostCount, QuestionCount, AnswerCount, TotalCommentScore, TotalBountyAmount, AvgReputation, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStats)
// SELECT u.DisplayName, u.Reputation, t.PostCount, t.QuestionCount, t.AnswerCount, t.TotalCommentScore, t.TotalBountyAmount, t.AvgReputation, t.Rank
// FROM TopUsers t JOIN Users u ON t.UserId = u.Id WHERE t.Rank <= 10 ORDER BY t.Rank;
//
// Rank reads only PostCount, so the top users are picked first and the product is driven for them alone.
// `v.UserId = u.Id` with `p.OwnerUserId = u.Id` keeps the votes cast by the post's owner (`own_votes`).
fn q13392(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let v = top_n(drain(&pc), |&(u, p)| (Reverse(p), u), 10);
    let tu = rel(v);
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let own = own_votes(db);
    let s = (&tu)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).select(&db.comment.score).opt()).and((&own).select((&db.vote.bounty_amount).opt()).opt())).opt()))
        .fold([0i64; 7], |a, (r, p)| match p {
            Some(((t, c), b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.unwrap_or(0), a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0), a[5] + 1, a[6] + r]
            }
            None => [a[0], a[1], a[2], a[3], a[4], a[5] + 1, a[6] + r],
        });
    let v = top_n(drain((&tu).and(&s)), |&(u, ((_, p), _))| (Reverse(p), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, ((_, p), a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), avg(a[6], a[5]), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalUpVotes, TotalDownVotes, BadgeCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC, TotalUpVotes DESC) AS Rank
//     FROM UserMetrics)
// SELECT DisplayName, PostCount, QuestionCount, AnswerCount, TotalUpVotes, TotalDownVotes, BadgeCount, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q5993(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let pc = user_distinct_posts(db);
    let v = top_n(drain((&s).and(&pc)), |&(u, (a, p))| (Reverse(p), Reverse(a[2]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (a, p)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(p)];
        f.extend(a.map(V::I));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes vt ON p.Id = vt.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, UpvoteCount, DownvoteCount, BadgeCount, LastPostDate, ROW_NUMBER() OVER (ORDER BY PostCount DESC, UpvoteCount DESC) AS Rank FROM UserActivity)
// SELECT tu.UserId, tu.DisplayName, tu.PostCount, tu.UpvoteCount, tu.DownvoteCount, tu.BadgeCount, tu.LastPostDate FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.PostCount DESC, tu.UpvoteCount DESC;
fn q8047(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0, 0, 0, i64::MIN], |a, (p, b)| {
            let (d, v) = p.map_or((i64::MIN, None), |x| x);
            [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + b.is_some() as i64, a[3].max(d)]
        });
    let pc = user_distinct_posts(db);
    let v = top_n(drain((&s).and(&pc)), |&(u, (a, p))| (Reverse(p), Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), V::I(a[2]), tmax(a[3])]);
        row(f)
    }))
}

// WITH PostSummary AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.AnswerCount, U.DisplayName AS OwnerDisplayName, COUNT(DISTINCT C.Id) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.PostTypeId = 1 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.AnswerCount, U.DisplayName),
// TopPosts AS (SELECT *, RANK() OVER (ORDER BY UpVoteCount DESC, AnswerCount DESC, ViewCount DESC) AS PostRank FROM PostSummary)
// SELECT PostId, Title, CreationDate, ViewCount, AnswerCount, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount FROM TopPosts WHERE PostRank <= 10;
fn q5720(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, answer_count, view_count, .. } = &db.post;
    let posts = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).group_by(Ident::<Post>::new());
    let s = posts()
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = posts().select(comments_of(db).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&cc)), |&(p, (a, _))| {
        let (n, w) = (answer_count.get(p), view_count.get(p));
        (Reverse(a[0]), n.is_none(), Reverse(n), w.is_none(), Reverse(w))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (a, c)), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionsAsked,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswersGiven, COALESCE(SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END), 0) AS VoteCount,
//        COALESCE(SUM(CASE WHEN B.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgesEarned
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON V.UserId = U.Id AND V.PostId = P.Id LEFT JOIN Badges B ON B.UserId = U.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionsAsked, AnswersGiven, VoteCount, BadgesEarned, RANK() OVER (ORDER BY QuestionsAsked DESC, AnswersGiven DESC, VoteCount DESC) AS UserRank
//     FROM UserActivity)
// SELECT UserId, DisplayName, QuestionsAsked, AnswersGiven, VoteCount, BadgesEarned, UserRank FROM TopUsers WHERE UserRank <= 10 ORDER BY UserRank;
//
// `V.UserId = U.Id AND V.PostId = P.Id` keeps the votes cast by the post's owner (`own_votes`).
fn q27058(db: &'static So) -> String {
    let own = own_votes(db);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&own).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(v, Some(2 | 3)) as i64, a[3] + b.is_some() as i64]
        });
    let v = ranked(drain(&s), |&(_, a)| (Reverse(a[0]), Reverse(a[1]), Reverse(a[2])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, TotalPosts, QuestionCount, AnswerCount, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostCounts)
// SELECT u.DisplayName, u.Reputation, COALESCE(up.QuestionCount, 0) AS NumberOfQuestions, COALESCE(up.AnswerCount, 0) AS NumberOfAnswers, COALESCE(SUM(c.Score), 0) AS TotalCommentScore,
//        ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
// FROM Users u LEFT JOIN TopUsers up ON u.Id = up.UserId LEFT JOIN Comments c ON c.UserId = u.Id WHERE u.CreationDate < (DATE '2024-10-01' - INTERVAL '1 year')
// GROUP BY u.Id, u.DisplayName, u.Reputation, up.QuestionCount, up.AnswerCount HAVING COALESCE(up.QuestionCount, 0) + COALESCE(up.AnswerCount, 0) > 10
// ORDER BY ReputationRank, NumberOfQuestions DESC LIMIT 50;
fn q4207(db: &'static So) -> String {
    let ups = user_posts(db);
    let users = db.user.with((&db.user.creation_date).lt(add_years(date(2024, 10, 1), -1)));
    let cs = users.group_by(Ident::<User>::new()).select(comments_by(db).select(&db.comment.score).opt()).fold(0i64, |n, s| n + s.unwrap_or(0));
    let v = drain((&cs).and((&ups).filt(|a| a[2] + a[3] > 10)));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 50);
    rows(v.into_iter().enumerate().map(|(i, (u, (s, a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[2]), V::I(a[3]), V::I(s), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(MAX(b.Class), 0) AS HighestBadgeClass
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON b.UserId = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, HighestBadgeClass, ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM RankedPosts)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount,
//        CASE WHEN tp.HighestBadgeClass = 1 THEN 'Gold' WHEN tp.HighestBadgeClass = 2 THEN 'Silver' WHEN tp.HighestBadgeClass = 3 THEN 'Bronze' ELSE 'No Badge' END AS BadgeLevel
// FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Rank;
//
// Rank reads only base columns, so the top questions are picked first.
fn q5480(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let hb = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).select(&db.badge.class).opt()).fold(0i64, |m, c| m.max(c.unwrap_or(0)));
    let v = top_n(drain((&cc).and(&hb)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::S(match b { 1 => "Gold", 2 => "Silver", 3 => "Bronze", _ => "No Badge" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation)
// SELECT T.DisplayName, T.Reputation, T.TotalPosts, T.UpVotes, T.DownVotes, COALESCE(AVG(P.ViewCount), 0) AS AverageViewCount, COALESCE(AVG(P.Score), 0) AS AverageScore
// FROM TopUsers T LEFT JOIN Posts P ON T.UserId = P.OwnerUserId WHERE T.Rank <= 10 GROUP BY T.UserId, T.DisplayName, T.Reputation, T.TotalPosts, T.UpVotes, T.DownVotes, T.Rank ORDER BY T.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the product is driven for them alone.
fn q8984(db: &'static So) -> String {
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let users = || (&tu).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let pa = users().select(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((w, s)) => [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + 1, a[3] + s],
        None => a,
    });
    let v = top_n(drain((&s).and(&pc).and(&pa)), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, ((a, p), m))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1])]);
        f.push(if m[0] == 0 { V::F(0.0) } else { avg(m[1], m[0]) });
        f.push(if m[2] == 0 { V::F(0.0) } else { avg(m[3], m[2]) });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, CommentCount, TotalScore, QuestionCount, AnswerCount, WikiCount, AvgViewCount, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, CommentCount, TotalScore, QuestionCount, AnswerCount, WikiCount, AvgViewCount FROM TopUsers WHERE Rank <= 10;
fn q13337(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some((((t, s), w), _)) => [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + (t == 3) as i64, a[5] + 1, a[6] + w.unwrap_or(0)],
            None => [a[0], a[1], a[2], a[3], a[4], a[5] + 1, a[6]],
        });
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(&cc)), |&(u, (a, _))| (Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (a, c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(c), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[6], a[5])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
//        DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY p.Id, p.Title, u.DisplayName, p.PostTypeId, p.CreationDate),
// FilteredPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.PostRank <= 10)
// SELECT fp.PostId, fp.Title, fp.OwnerDisplayName, fp.CommentCount, fp.VoteCount, fp.UpVoteCount, fp.DownVoteCount FROM FilteredPosts fp ORDER BY fp.UpVoteCount DESC, fp.CommentCount DESC;
//
// PostRank reads only base columns, so the newest posts are picked first and the comment x vote product is driven for those alone.
fn q6141(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)), |&(p, t)| (t, Reverse(creation_date.get(p).unwrap())), true);
    let v = per_group(v, |&(_, t)| t);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().filter(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let v = drain(&s);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// TopUsers AS (SELECT ru.UserId, ru.DisplayName FROM RankedUsers ru WHERE ru.ReputationRank <= 10),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.OwnerUserId)
// SELECT tu.DisplayName, ps.PostCount, ps.TotalViews, ps.AcceptedAnswers, ps.AnswerCount,
//        CASE WHEN ps.PostCount = 0 THEN 0 ELSE (CAST(ps.AcceptedAnswers AS FLOAT) / ps.PostCount) * 100 END AS AcceptanceRate
// FROM TopUsers tu LEFT JOIN PostStats ps ON tu.UserId = ps.OwnerUserId ORDER BY tu.DisplayName;
fn q6202(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, accepted_answer_id, post_type_id, .. } = &db.post;
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(view_count.opt().and(accepted_answer_id.opt()).and(post_type_id))
        .fold([0i64; 5], |a, ((w, x), t)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + x.is_some() as i64, a[4] + (t == 2) as i64]);
    let v = drain((&tu).select((&ps).opt()));
    rows(v.into_iter().map(|(u, a)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(match a {
            Some(a) => [V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4]), V::F((a[3] as f32 / a[0] as f32 * 100.0f32) as f64)],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UserRank, COALESCE(AVG(b.Class), 0) AS AverageBadgeClass
//     FROM RankedPosts rp LEFT JOIN Badges b ON rp.PostId = b.UserId WHERE rp.UserRank <= 3 GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UserRank)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AverageBadgeClass FROM TopPosts tp WHERE tp.AverageBadgeClass IS NOT NULL ORDER BY tp.Score DESC LIMIT 10;
//
// `rp.PostId = b.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q3182(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let ab = (&tp).group_by(Ident::<Post>::new()).select(origid.select(&bidx).select(&db.badge.class).opt()).fold([0i64; 2], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + c],
        None => a,
    });
    let v = top_n(drain(&ab), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, AcceptedAnswerCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT u.DisplayName, u.Reputation, t.QuestionCount, t.AnswerCount, t.AcceptedAnswerCount, (SELECT COUNT(*) FROM Votes v WHERE v.UserId = u.Id AND v.VoteTypeId = 2) AS Upvotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.UserId = u.Id AND v.VoteTypeId = 3) AS Downvotes
// FROM Users u JOIN TopUsers t ON u.Id = t.UserId WHERE t.Rank <= 10 ORDER BY t.Rank, t.Reputation DESC;
fn q7634(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, x)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 1 && x.is_some()) as i64],
        None => a,
    });
    let uv = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&s).and(&uv)), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH PostActivity AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.LastActivityDate, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT ph.Id) AS HistoryCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.LastActivityDate, p.ViewCount),
// PostRanked AS (SELECT pa.*, RANK() OVER (ORDER BY pa.ViewCount DESC, pa.UpVotes DESC, pa.LastActivityDate DESC) AS Rank FROM PostActivity pa)
// SELECT Rank, Title, OwnerDisplayName, CreationDate, LastActivityDate, ViewCount, CommentCount, UpVotes, DownVotes, HistoryCount FROM PostRanked WHERE Rank <= 10;
fn q6041(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, last_activity_date, .. } = &db.post;
    let posts = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -1)))).group_by(Ident::<Post>::new());
    let s = posts()
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let hc = posts().select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = ranked(drain((&s).and(&hc)), |&(p, (a, _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(a[1]), Reverse(last_activity_date.get(p).unwrap()))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (a, h)), r)| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["title", "owner", "created", "activity", "views"]));
        f.extend(a.map(V::I));
        f.push(V::I(h));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.Reputation, U.DisplayName),
// TopQuestions AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, ROW_NUMBER() OVER (ORDER BY P.Score DESC) AS ScoreRank FROM Posts P
//     WHERE P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL)
// SELECT UR.DisplayName, UR.Reputation, UR.QuestionCount, UR.Upvotes, UR.Downvotes, TQ.Title, TQ.Score, TQ.CreationDate
// FROM UserReputation UR LEFT JOIN TopQuestions TQ ON UR.QuestionCount > 10 AND UR.Upvotes > UR.Downvotes WHERE TQ.ScoreRank <= 10 ORDER BY UR.Reputation DESC, TQ.Score DESC LIMIT 20;
//
// The ON clause names only UR, and the WHERE drops the unmatched rows, so the qualifying users are crossed with the ten top questions.
fn q1918(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, score, .. } = &db.post;
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let users = || db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new());
    let s = users().select(qs().select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let qc = users().select(qs().opt()).buf_fold(distinct_some);
    let ur: HashIdx<Id<User>, ([i64; 2], i64)> = (&s).and((&qc).filt(|n| n > 10)).filt(|(a, _)| a[0] > a[1]).collect();
    let tq = top_n(drain(db.post.with(post_type_id.eq(1)).with(accepted_answer_id).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tq: MatSet<Id<Post>> = rel(tq.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let mut v = Vec::new();
    (&ur).cross(&tq).drive(|(u, p), (a, _)| v.push((u, p, a)));
    let v = top_n(v, |&(u, p, _)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), u, p), 20);
    rows(v.into_iter().map(|(u, p, (a, q))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(q), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["title", "score", "created"]));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalScore, TotalViews, TotalBadges, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Ranking FROM UserPostStats)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalAnswers, tu.TotalQuestions, tu.TotalScore, tu.TotalViews, tu.TotalBadges, RANK() OVER (ORDER BY tu.TotalViews DESC) AS ViewsRanking
// FROM TopUsers tu WHERE tu.Ranking <= 10 ORDER BY tu.Ranking, tu.TotalScore DESC;
fn q6858(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + s, a[4] + w.unwrap_or(0)],
            None => a,
        });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(&bc)), |&(u, (a, _))| (Reverse(a[3]), u), 10);
    let v = ranked(v, |&(_, (a, _))| Reverse(a[4]), false);
    rows(v.into_iter().map(|((u, (a, b)), r)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, P.ViewCount, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT A.Id) AS AnswerCount, ROW_NUMBER() OVER (ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Posts A ON P.Id = A.ParentId WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName, P.ViewCount, P.Score),
// FilteredRankedPosts AS (SELECT RP.*, CASE WHEN RP.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus FROM RankedPosts RP WHERE RP.Rank <= 10)
// SELECT FRP.PostId, FRP.Title, FRP.CreationDate, FRP.OwnerDisplayName, FRP.ViewCount, FRP.Score, FRP.CommentCount, FRP.AnswerCount, FRP.CommentStatus
// FROM FilteredRankedPosts FRP ORDER BY FRP.Score DESC, FRP.CreationDate DESC;
//
// Rank reads only base columns, so the top questions are picked first and the comment x answer product is driven for those alone.
fn q6976(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(children_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).buf_fold(distinct_some);
    let v = drain((&s).and(&ac));
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "views", "score"]);
        f.extend([V::I(c), V::I(a), V::S(if c > 0 { "Has Comments" } else { "No Comments" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesCount, COUNT(DISTINCT p.Id) AS PostsCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank FROM Posts p)
// SELECT us.DisplayName, us.Reputation, us.UpVotesCount, us.DownVotesCount, us.PostsCount, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate
// FROM UserStatistics us LEFT JOIN RecentPosts rp ON us.UserId = rp.OwnerUserId AND rp.RecentPostRank = 1
// WHERE us.Reputation > 1000 AND (us.UpVotesCount - us.DownVotesCount) > 0 ORDER BY us.Reputation DESC LIMIT 10;
fn q4981(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new());
    let s = users()
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let rp = top_per(drain(db.post.select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let last: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let v = drain((&s).filt(|a| a[0] - a[1] > 0).and(&pc).and((&last).map(|(_, p)| p).opt()));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId IN (1, 2) AND p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, us.UserId, us.DisplayName, us.PostsCreated, us.UpVotesReceived, us.DownVotesReceived
// FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, us.UpVotesReceived DESC;
fn q5878(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, tags_str, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).buf_fold(|it| it.into_iter().count() as i64);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&pc).and(&us))));
    rows(v.into_iter().map(|(p, ((u, n), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore, COUNT(DISTINCT p.Id) AS TotalPosts,
//        COUNT(DISTINCT c.Id) AS TotalComments FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalViews, TotalScore, TotalPosts, TotalComments, ROW_NUMBER() OVER (ORDER BY TotalViews DESC) AS ViewRank,
//        ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserActivity)
// SELECT t.DisplayName, t.TotalViews, t.TotalScore, t.TotalPosts, t.TotalComments,
//        CASE WHEN t.ViewRank <= 10 THEN 'Top Viewers' WHEN t.ScoreRank <= 10 THEN 'Top Scorers' ELSE 'Regular User' END AS UserType
// FROM TopUsers t WHERE t.TotalPosts > 5 ORDER BY t.TotalViews DESC, t.TotalScore DESC;
fn q5413(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score).and(comments_of(db).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some(((w, s), _)) => [a[0] + w.unwrap_or(0), a[1] + s],
            None => a,
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let cc = users().select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let mut v: Vec<_> = drain((&s).and(&pc).and(&cc)).into_iter().map(|x| (x, 0usize, 0usize)).collect();
    v.sort_by_key(|&((u, ((a, _), _)), _, _)| (Reverse(a[0]), u));
    for (i, x) in v.iter_mut().enumerate() {
        x.1 = i + 1;
    }
    v.sort_by_key(|&((u, ((a, _), _)), _, _)| (Reverse(a[1]), u));
    for (i, x) in v.iter_mut().enumerate() {
        x.2 = i + 1;
    }
    let v = rel(v);
    let v = drain((&v).filt(|((_, ((_, p), _)), _, _)| p > 5));
    rows(v.into_iter().map(|(_, ((u, ((a, p), c)), w, r))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(p), V::I(c), V::S(if w <= 10 { "Top Viewers" } else if r <= 10 { "Top Scorers" } else { "Regular User" })])
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(COALESCE(C.VoteCount, 0)) AS TotalVotes, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) C ON P.Id = C.PostId
//     LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U)
// SELECT E.UserId, E.DisplayName, E.QuestionCount, E.AnswerCount, E.TotalVotes, E.BadgeCount, R.Reputation, R.ReputationRank
// FROM UserEngagement E JOIN UserReputation R ON E.UserId = R.UserId WHERE E.QuestionCount > 0 ORDER BY E.TotalVotes DESC, R.ReputationRank ASC FETCH FIRST 100 ROWS ONLY;
fn q7710(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&vc).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| match p {
            Some((t, c)) => [a[0] + (t == 2) as i64, a[1] + c.unwrap_or(0)],
            None => a,
        });
    let pc = user_distinct_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rk: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let v = drain((&s).and((&pc).filt(|n| n > 0)).and(&bc).and((&rk).map(|(_, r)| r)));
    let v = top_n(v, |&(u, (((a, _), _), r))| (Reverse(a[1]), r, u), 100);
    rows(v.into_iter().map(|(u, (((a, q), b), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(q), V::I(a[0]), V::I(a[1]), V::I(b), user_col(db, u, "rep"), V::I(r)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(CASE WHEN c.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount,
//        AVG(u.Reputation) AS AvgUserReputation FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON v.UserId = u.Id
//     WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// RankedPosts AS (SELECT ps.*, RANK() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC, ps.CommentCount DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY ps.VoteCount DESC, ps.AvgUserReputation DESC) AS VoteRank FROM PostStats ps)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount, rp.AvgUserReputation, rp.ScoreRank, rp.VoteRank
// FROM RankedPosts rp WHERE rp.ScoreRank <= 10 AND rp.VoteRank <= 10 ORDER BY rp.Score DESC, rp.VoteCount DESC;
fn q6641(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let Vote { user, user_id, .. } = &db.vote;
    let posts = || db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).group_by(Ident::<Post>::new());
    let s = posts()
        .select(comments_of(db).opt().and(votes_of(db).select(user.select(&db.user.reputation).opt()).opt()))
        .fold([0i64; 3], |a, (c, r)| {
            let r = r.flatten();
            [a[0] + c.is_some() as i64, a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0)]
        });
    let vc = posts().select(votes_of(db).select(user_id).opt()).buf_fold(distinct_some);
    let v = drain((&s).and(&vc));
    let ar = |a: [i64; 3]| if a[1] == 0 { None } else { Some(a[2] as f64 / a[1] as f64) };
    let v = ranked(v, |&(p, (a, _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), Reverse(a[0]))
    }, false);
    let v = ranked(v, |&((_, (a, n)), _)| (Reverse(n), ar(a).is_none(), Reverse(ar(a).map(fkey))), false);
    rows(v.into_iter().filter(|&((_, r), w)| r <= 10 && w <= 10).map(|(((p, (a, n)), r), w)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(n), avg(a[2], a[1]), V::I(r), V::I(w)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldCount, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeCount FROM Badges GROUP BY UserId),
// PopularPosts AS (SELECT OwnerUserId, COUNT(*) AS PostsCreated, SUM(ViewCount) AS TotalViews FROM Posts WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY OwnerUserId),
// UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PP.PostsCreated, 0) AS PostsCreated, COALESCE(PP.TotalViews, 0) AS TotalViews
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PopularPosts PP ON U.Id = PP.OwnerUserId),
// RankedUsers AS (SELECT UserId, DisplayName, BadgeCount, PostsCreated, TotalViews, ROW_NUMBER() OVER (ORDER BY TotalViews DESC, PostsCreated DESC, BadgeCount DESC) AS Rank FROM UserEngagement)
// SELECT Rank, UserId, DisplayName, BadgeCount, PostsCreated, TotalViews FROM RankedUsers WHERE Rank <= 10 ORDER BY Rank;
fn q5317(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let Post { owner_user, creation_date, view_count, .. } = &db.post;
    let pp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(view_count.opt()).fold([0i64; 2], |a, w| [a[0] + 1, a[1] + w.unwrap_or(0)]);
    let v = drain(db.user.select((&ub).opt().and((&pp).opt())));
    let key = |b: Option<i64>, p: Option<[i64; 2]>| (b.unwrap_or(0), p.map_or(0, |a| a[0]), p.map_or(0, |a| a[1]));
    let v = top_n(v, |&(u, (b, p))| {
        let (b, n, w) = key(b, p);
        (Reverse(w), Reverse(n), Reverse(b), u)
    }, 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (b, p)))| {
        let (b, n, w) = key(b, p);
        let mut f = vec![V::I(i as i64 + 1)];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b), V::I(n), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.LastActivityDate, p.ViewCount, p.Score, p.Tags, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.ViewCount DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= '2023-01-01' AND p.Score > 0),
// PopularTags AS (SELECT unnest(string_to_array(Tags, ',')) AS Tag FROM RankedPosts),
// TagStats AS (SELECT Tag, COUNT(*) AS QuestionCount, SUM(Score) AS TotalScore FROM PopularTags pt JOIN RankedPosts rp ON rp.Tags LIKE '%' || pt.Tag || '%' GROUP BY Tag)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.ViewCount, rp.Score, ts.Tag, ts.QuestionCount, ts.TotalScore
// FROM RankedPosts rp JOIN TagStats ts ON rp.Tags LIKE '%' || ts.Tag || '%' WHERE rp.TagRank <= 3 ORDER BY ts.TotalScore DESC, rp.ViewCount DESC;
//
// Tags never contains ',', so each PopularTags row is one RankedPosts row's whole Tags string, and the LIKE is a substring test between whole Tags strings.
fn q27279(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, tags_str, view_count, .. } = &db.post;
    let rp: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1).and(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).and(score.gt(0))).with(owner_user).collect();
    let full: MatSet<Str> = (&rp).select(tags_str).collect();
    let cont: HashIdx<Str, Str> = (&full).select_where(&full, |c: Str, t: Str| c.contains(t)).collect();
    let pat: HashIdx<Str, Str> = (&cont).inv().collect();
    let by_tags: HashIdx<Str, Id<Post>> = (&rp).select(tags_str).inv().collect();
    let tstats = (&rp).group_by(tags_str).select(tags_str.select(&pat).select(&by_tags).select(score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let top = top_per(drain((&rp).select(tags_str)), |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&tp).select(tags_str.select(&cont).select(Same::<Str>::new().and(&tstats))));
    rows(v.into_iter().map(|(p, (t, a))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views", "score"]);
        f.extend([V::S(t), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// TopUsers AS (SELECT ua.UserId, ua.PostCount, ua.TotalViews, (ua.UpVotes - ua.DownVotes) AS NetVotes, RANK() OVER (ORDER BY ua.PostCount DESC, ua.TotalViews DESC) AS Rank FROM UserActivity ua)
// SELECT tu.UserId, u.DisplayName, tu.PostCount, tu.TotalViews, tu.NetVotes,
//        CASE WHEN tu.Rank <= 10 THEN 'Top Contributor' WHEN tu.Rank <= 50 THEN 'Average Contributor' ELSE 'Needs Improvement' END AS ContributorLevel
// FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id
// WHERE tu.PostCount > 0 AND EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = u.Id AND p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')) ORDER BY tu.Rank;
fn q707(db: &'static So) -> String {
    let Post { view_count, creation_date, owner_user, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((w, t)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (Reverse(a[0]), Reverse(a[1])), false);
    let recent: MatSet<Id<User>> = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user).collect();
    let v = rel(v.into_iter().map(|((u, a), r)| (u, a, r)).collect());
    let v = drain((&v).filt(|(_, a, _)| a[0] > 0).with(Same::<(Id<User>, [i64; 4], i64)>::new().map(|(u, _, _)| u).select(&recent)));
    rows(v.into_iter().map(|(_, (u, a, r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2] - a[3]), V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Average Contributor" } else { "Needs Improvement" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, Upvotes, Downvotes, ROW_NUMBER() OVER (ORDER BY (PostCount + Upvotes) DESC) AS Rank FROM UserStats)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.Upvotes, tu.Downvotes, (SELECT COUNT(*) FROM Users) AS TotalUsers,
//        (SELECT COUNT(*) FROM Posts WHERE OwnerUserId IS NOT NULL) AS TotalPosts
// FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// `v.UserId = u.Id AND v.PostId = p.Id` keeps the votes cast by the post's owner (`own_votes`).
fn q8718(db: &'static So) -> String {
    let own = own_votes(db);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&own).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = user_distinct_posts(db);
    let users = count(&db.user.reputation);
    let posts = count(db.post.with(&db.post.owner_user_id));
    let v = top_n(drain((&s).and(&pc)), |&(u, (a, p))| (Reverse(p + a[2]), u), 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(p)];
        f.extend(a.map(V::I));
        f.extend([V::I(users), V::I(posts)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, TotalBounties, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserStatistics)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.PostCount, T.AnswerCount, T.QuestionCount, T.TotalBounties, T.UpVotes, T.DownVotes FROM TopUsers T WHERE T.ReputationRank <= 10
// ORDER BY T.Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the product is driven for them alone.
fn q8902(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let users = || (&tu).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => {
                let (vt, b) = v.map_or((0, None), |x| x);
                [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.unwrap_or(0), a[3] + (vt == 2) as i64, a[4] + (vt == 3) as i64]
            }
            None => a,
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let v = drain((&s).and(&pc));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(p));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END) AS PositiveScore,
//        SUM(CASE WHEN p.Score < 0 THEN p.Score ELSE 0 END) AS NegativeScore, AVG(COALESCE(CAST(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) AS FLOAT), 0)) AS AvgTimeToActivity
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, PositiveScore, NegativeScore, AvgTimeToActivity, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserPostStats)
// SELECT tu.Rank, tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.PositiveScore, tu.NegativeScore, tu.AvgTimeToActivity FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
fn q9416(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, last_activity_date, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(last_activity_date).and(creation_date)).opt())
        .fold(([0i64; 6], 0.0f64), |(a, t), p| match p {
            Some((((ty, s), la), cd)) => (
                [a[0] + 1, a[1] + (ty == 2) as i64, a[2] + (ty == 1) as i64, a[3] + s.max(0), a[4] + s.min(0), a[5] + 1],
                t + secs(la - cd) as f32 as f64,
            ),
            None => ([a[0], a[1], a[2], a[3], a[4], a[5] + 1], t),
        });
    let v = ranked(drain(&s), |&(_, (a, _))| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, t)), r)| {
        row(vec![V::I(r), user_col(db, u, "name"), V::I(a[0]), V::I(a[2]), V::I(a[1]), V::I(a[3]), V::I(a[4]), V::F(t / a[5] as f64)])
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        AVG(p.Score) AS AverageScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, AVG(p.AnswerCount) AS AverageAnswersPerQuestion
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, AcceptedAnswers, AverageScore, TotalViews, AverageAnswersPerQuestion, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank
//     FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, AcceptedAnswers, AverageScore, TotalViews, AverageAnswersPerQuestion FROM TopUsers WHERE PostRank <= 10 ORDER BY PostRank;
fn q12786(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, accepted_answer_id, answer_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(accepted_answer_id.opt()).and(answer_count.opt())).opt())
        .fold([0i64; 8], |a, p| match p {
            Some(((((t, s), w), x), n)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && x.is_some()) as i64, a[4] + s, a[5] + w.unwrap_or(0), a[6] + n.is_some() as i64, a[7] + n.unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), V::I(a[5]), avg(a[7], a[6])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AcceptedAnswers, TotalViews, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT tu.DisplayName, tu.Reputation, tu.QuestionCount, tu.AcceptedAnswers, tu.TotalViews, tu.UpVotes, tu.DownVotes, (tu.UpVotes - tu.DownVotes) AS NetVotes
// FROM TopUsers tu WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the product is driven for them alone.
fn q7591(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, view_count, .. } = &db.post;
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let users = || (&tu).group_by(Ident::<User>::new());
    let s = users()
        .select(qs().select(accepted_answer_id.opt().and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((x, w), t)) => [a[0] + x.is_some() as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + (t == Some(2)) as i64, a[4] + (t == Some(3)) as i64],
            None => a,
        });
    let qc = users().select(qs().opt()).buf_fold(distinct_some);
    let v = drain((&s).and(&qc));
    rows(v.into_iter().map(|(u, (a, q))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(q), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4]), V::I(a[3] - a[4])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank, p.PostTypeId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.PostTypeId),
// MaxScores AS (SELECT PostTypeId, MAX(Score) AS MaxScore FROM Posts GROUP BY PostTypeId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.AnswerCount, ms.MaxScore,
//        CASE WHEN rp.Score = ms.MaxScore THEN 'Top Performance' ELSE 'Average Performance' END AS PerformanceStatus
// FROM RankedPosts rp JOIN MaxScores ms ON rp.PostTypeId = ms.PostTypeId WHERE rp.Rank <= 10 ORDER BY rp.PostTypeId, rp.Score DESC;
//
// Rank reads only base columns, so the top posts are picked first.
fn q9515(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).buf_fold(distinct_some);
    let ms = db.post.group_by(post_type_id).select(score).fold(i64::MIN, |m, s| m.max(s));
    let v = drain((&cc).and(&ac).and(post_type_id.select(&ms)));
    rows(v.into_iter().map(|(p, ((c, a), m))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(a), V::I(m), V::S(if score.get(p).unwrap() == m { "Top Performance" } else { "Average Performance" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews, SUM(COALESCE(c.Score, 0)) AS TotalCommentsScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, TotalCommentsScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
//     FROM UserStats)
// SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, TotalCommentsScore FROM TopUsers WHERE ScoreRank <= 10 ORDER BY TotalScore DESC;
fn q10233(db: &'static So) -> String {
    let Post { score, view_count, post_type_id, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(comments_of(db).select(&db.comment.score).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((s, w), c)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c.unwrap_or(0)],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let v = ranked(drain((&s).and(&pc)), |&(_, (a, _))| (a[0] == 0, Reverse(a[1])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, p)), _)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(p.map(V::I));
        f.extend([nullable(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4])]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, COALESCE(a.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.AcceptedAnswerId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, a.AcceptedAnswerId),
// TopPostStats AS (SELECT *, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostStats)
// SELECT t.Title, t.Score, t.ViewCount, t.AcceptedAnswerId, t.CommentCount, t.UpVotes, t.DownVotes, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, t.Rank
// FROM TopPostStats t JOIN Users u ON t.PostId = u.Id WHERE t.Rank <= 10 ORDER BY t.Rank;
//
// Rank reads only base columns, so the top posts are picked first. The GROUP BY key a.AcceptedAnswerId is p.Id or NULL, one group per post.
// `t.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q6402(db: &'static So) -> String {
    let Post { creation_date, score, view_count, origid, accepted_answer, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = ranked(drain(db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1))).select(score)), |&(p, _)| key(p), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let tr: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let by: HashIdx<Id<Post>, Id<Post>> = accepted_answer.inv().collect();
    let g = (&tr)
        .map(|(p, _)| p)
        .group_by(Ident::<Post>::new())
        .select((&by).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, ((x, c), t)| [a[0] | x.is_some() as i64, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&g).and(origid.select(&uidx)).and(&tr));
    rows(v.into_iter().map(|(p, ((a, u), (_, r)))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.push(V::I(if a[0] == 1 { origid.get(p).unwrap() } else { 0 }));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, u.DisplayName AS OwnerName, u.Reputation AS OwnerReputation
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id ORDER BY tp.Score DESC;
//
// Rank reads only base columns, so the newest posts are picked first. `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q5049(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, .. } = &db.post;
    let since = add_years(utc_to_ny(now_utc()), -1);
    let v = drain(db.post.with(creation_date.ge(since)).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&s).and(origid.select(&uidx)));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        SUM(CASE WHEN b.Class IS NOT NULL THEN b.Class ELSE 0 END) AS BadgeScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, Upvotes, Downvotes, BadgeScore, RANK() OVER (ORDER BY PostCount DESC, Upvotes DESC) AS UserRank
//     FROM UserStats WHERE PostCount > 0)
// SELECT ru.UserRank, ru.DisplayName, ru.PostCount, ru.AnswerCount, ru.QuestionCount, ru.Upvotes, ru.Downvotes, ru.BadgeScore FROM RankedUsers ru WHERE ru.UserRank <= 10 ORDER BY ru.UserRank;
fn q7373(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.unwrap_or(0)]
        });
    let pc = user_distinct_posts(db);
    let v = ranked(drain((&s).and((&pc).filt(|n| n > 0))), |&(_, (a, p))| (Reverse(p), Reverse(a[2])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, p)), r)| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(p)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.ViewCount) AS TotalViews, SUM(COALESCE(vt.VoteCount, 0)) AS TotalVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) vt ON p.Id = vt.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalVotes, RANK() OVER (ORDER BY TotalVotes DESC, TotalViews DESC) AS Rank FROM UserStats)
// SELECT t.UserId, t.DisplayName, t.TotalPosts, t.TotalQuestions, t.TotalAnswers, t.TotalViews, t.TotalVotes, CASE WHEN t.Rank <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserCategory
// FROM TopUsers t WHERE t.TotalPosts > 0 ORDER BY t.Rank, t.TotalVotes DESC;
fn q5272(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.view_count).opt()).and((&vc).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, w), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + c.unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (Reverse(a[5]), a[3] == 0, Reverse(a[4])), false);
    let v = rel(v.into_iter().map(|((u, a), r)| (u, a, r)).collect());
    let v = drain((&v).filt(|(_, a, _)| a[0] > 0));
    rows(v.into_iter().map(|(_, (u, a, r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5]), V::S(if r <= 10 { "Top User" } else { "Regular User" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(v.Cnt, 0)) AS VoteCount, SUM(COALESCE(b.BadgeCount, 0)) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS Cnt FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, VoteCount, BadgeCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC, VoteCount DESC) AS Rank FROM UserStats)
// SELECT t.DisplayName AS TopUser, t.PostCount, t.QuestionCount, t.AnswerCount, t.VoteCount, t.BadgeCount FROM TopUsers t WHERE t.Rank <= 10 ORDER BY t.Rank;
fn q5489(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let users = || db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.post_type_id).and((&vc).opt())).opt().and((&bc).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, c) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.unwrap_or(0), a[3] + b.unwrap_or(0)]
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&s).and(&pc)), |&(u, (a, p))| (Reverse(p), Reverse(a[2]), u), 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(p)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(DISTINCT p.Id) AS PostCount, RANK() OVER (ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// RecentPostActivity AS (SELECT p.OwnerUserId, COUNT(c.Id) AS CommentCount, AVG(p.Score) AS AverageScore, COUNT(ph.Id) AS HistoryCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.OwnerUserId)
// SELECT ru.DisplayName, ru.Upvotes, ru.Downvotes, ru.PostCount, rpa.CommentCount, rpa.AverageScore, rpa.HistoryCount
// FROM RankedUsers ru JOIN RecentPostActivity rpa ON ru.UserId = rpa.OwnerUserId WHERE ru.UserRank <= 10 ORDER BY ru.Upvotes DESC, rpa.AverageScore DESC;
fn q5875(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = user_distinct_posts(db);
    let rpa = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user)
        .select(score.and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 4], |a, ((s, c), h)| [a[0] + c.is_some() as i64, a[1] + 1, a[2] + s, a[3] + h.is_some() as i64]);
    let v = ranked(drain((&s).and(&pc)), |&(_, (a, _))| Reverse(a[0]), false);
    let v = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, p)), _)| (u, (a, p))).collect());
    let v = drain((&v).select(Same::<(Id<User>, ([i64; 2], i64))>::new().and(Same::<(Id<User>, ([i64; 2], i64))>::new().map(|(u, _)| u).select(&rpa))));
    rows(v.into_iter().map(|(_, ((u, (a, p)), r))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(p), V::I(r[0]), avg(r[2], r[1]), V::I(r[3])])))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, P.Score, U.DisplayName AS Author,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.ViewCount, RP.CreationDate, RP.Score, RP.Author FROM RankedPosts RP WHERE RP.Rank <= 5),
// PostVoteDetails AS (SELECT V.PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes V GROUP BY V.PostId)
// SELECT TP.PostId, TP.Title, TP.ViewCount, TP.Score, TP.Author, COALESCE(PVD.UpVotes, 0) AS UpVotes, COALESCE(PVD.DownVotes, 0) AS DownVotes, TP.CreationDate
// FROM TopPosts TP LEFT JOIN PostVoteDetails PVD ON TP.PostId = PVD.PostId ORDER BY TP.ViewCount DESC;
fn q8516(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvd = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&tp).select((&pvd).opt()));
    rows(v.into_iter().map(|(p, a)| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT Rank, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only Reputation, so the top users are picked first and the product is driven for them alone.
fn q9535(db: &'static So) -> String {
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tr: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let users = || (&tr).map(|(u, _)| u).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let v = drain((&tr).and(&s).and(&pc));
    rows(v.into_iter().map(|(u, (((_, r), a), p))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(p));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// RankedUsers AS (SELECT UA.*, DENSE_RANK() OVER (ORDER BY UA.Reputation DESC) AS ReputationRank FROM UserActivity UA)
// SELECT RU.UserId, RU.DisplayName, RU.Reputation, RU.TotalPosts, RU.QuestionCount, RU.AnswerCount, COALESCE(RU.TotalBounty, 0) AS TotalBounty,
//        CASE WHEN RU.ReputationRank <= 10 THEN 'Top Contributor' WHEN RU.ReputationRank <= 50 THEN 'Active Contributor' ELSE 'Novice Contributor' END AS ContributionLevel
// FROM RankedUsers RU WHERE RU.QuestionCount > 5 AND RU.AnswerCount > 10 ORDER BY RU.Reputation DESC LIMIT 20;
fn q3500(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(0)).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&pc)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), true);
    let v = rel(v.into_iter().map(|((u, (a, p)), r)| (u, a, p, r)).collect());
    let v = drain((&v).filt(|(_, a, _, _)| a[0] > 5 && a[1] > 10));
    let v = top_n(v, |&(_, (u, _, _, _))| (Reverse(db.user.reputation.get(u).unwrap()), u), 20);
    rows(v.into_iter().map(|(_, (u, a, p, r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Active Contributor" } else { "Novice Contributor" }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT p.PostTypeId) AS UniquePostTypes, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews,
//        AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgPostDurationInSeconds FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, UniquePostTypes, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AvgPostDurationInSeconds, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
//     FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, UniquePostTypes, TotalQuestions, TotalAnswers, TotalScore, TotalViews, AvgPostDurationInSeconds, ScoreRank FROM TopUsers WHERE ScoreRank <= 10 ORDER BY TotalScore DESC;
//
// The float AVG is summed with Kahan compensation; a plain fold is one digit off (notes/translation-failures.md 10).
fn q10277(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, last_activity_date, creation_date, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(last_activity_date).and(creation_date)).opt())
        .fold(([0i64; 6], (0.0f64, 0.0f64)), |(a, d), p| match p {
            Some(((((t, s), w), la), cd)) => ([a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)], kahan(d, secs(la - cd))),
            None => (a, d),
        });
    let ut = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&ut)), |&(_, ((a, _), _))| (a[0] == 0, Reverse(a[3])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, d), t)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(t), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), nullable(a[5], a[4])]);
        f.push(if a[0] == 0 { V::Null } else { V::F(d.0 / a[0] as f64) });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserScores AS (SELECT u.Id AS UserId, u.Reputation, COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId
//     LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, COUNT(DISTINCT ph.UserId) AS CloseVoteCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, us.UserId, us.Reputation, us.CommentCount, us.BadgeCount, COALESCE(cp.CloseVoteCount, 0) AS CloseVoteCount
// FROM RankedPosts rp JOIN UserScores us ON us.UserId = rp.PostId LEFT JOIN ClosedPosts cp ON cp.PostId = rp.PostId WHERE rp.RankByScore <= 10
// ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 100;
//
// `us.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q3530(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tu: MatSet<Id<User>> = (&tp).select(origid.select(&uidx)).collect();
    let us = (&tu).group_by(Ident::<User>::new()).select(comments_by(db).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64]);
    let ph = &db.post_history;
    let cp = db.post_history.with((&ph.post_history_type_id).eq(10)).group_by(&ph.post).select((&ph.user_id).opt()).buf_fold(distinct_some);
    let v = drain((&tp).select(origid.select(&uidx).select(Ident::<User>::new().and(&us)).and((&cp).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 100);
    rows(v.into_iter().map(|(p, ((u, a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBounty, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        AVG(v.BountyAmount) AS AvgBounty FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.Title, rp.ViewCount, us.DisplayName, us.BadgeCount, us.TotalBounty, us.Upvotes, us.AvgBounty
// FROM RankedPosts rp JOIN Posts p ON rp.Id = p.Id LEFT JOIN UserStats us ON p.OwnerUserId = us.UserId
// WHERE (rp.Rank <= 5 OR us.BadgeCount >= 3) AND (p.AcceptedAnswerId IS NOT NULL OR p.AnswerCount > 0) ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q1591(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, accepted_answer_id, answer_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let v = ranked(v, |&(p, t)| {
        let w = view_count.get(p);
        (t, Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, false);
    let v = per_group(v, |&(_, t)| t);
    let rp = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()))
        .fold([0i64; 4], |a, (b, v)| {
            let (t, x) = v.map_or((0, None), |x| x);
            [a[0] + b.is_some() as i64, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0), a[3] + (t == 2) as i64]
        });
    type R = (Id<Post>, i64);
    let post = || Same::<R>::new().map(|(p, _): R| p);
    let ok = post().with(accepted_answer_id.or(answer_count.gt(0)));
    let v = drain((&rp).with(ok).select(Same::<R>::new().and(post().select(owner_user.select(Ident::<User>::new().and(&us))).opt())));
    let v = rel(v.into_iter().map(|(_, x)| x).collect());
    let v = drain((&v).filt(|((_, r), u)| r <= 5 || u.map_or(false, |(_, a)| a[0] >= 3)));
    rows(v.into_iter().map(|(_, ((p, _), u))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend(match u {
            Some((u, a)) => vec![user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), avg(a[2], a[1])],
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount, SUM(COALESCE(C.Score, 0)) AS TotalCommentScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT DisplayName, TotalViews, QuestionCount, AnswerCount, TotalCommentScore, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank,
//        RANK() OVER (ORDER BY QuestionCount DESC) AS QuestionRank, RANK() OVER (ORDER BY AnswerCount DESC) AS AnswerRank FROM UserEngagement)
// SELECT DisplayName, TotalViews, QuestionCount, AnswerCount, TotalCommentScore, ViewRank, QuestionRank, AnswerRank FROM TopUsers
// WHERE ViewRank <= 10 OR QuestionRank <= 10 OR AnswerRank <= 10 ORDER BY ViewRank, QuestionRank, AnswerRank;
fn q5508(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.view_count).opt().and(comments_of(db).select(&db.comment.score).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((w, c)) => [a[0] + w.unwrap_or(0), a[1] + c.unwrap_or(0)],
            None => a,
        });
    let qa = users().select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64]);
    let v = ranked(drain((&s).and(&qa)), |&(_, (a, _))| Reverse(a[0]), false);
    let v = ranked(v, |&((_, (_, q)), _)| Reverse(q[0]), false);
    let v = ranked(v, |&(((_, (_, q)), _), _)| Reverse(q[1]), false);
    rows(v.into_iter().filter(|&((((_, _), a), b), c)| a <= 10 || b <= 10 || c <= 10).map(|((((u, (s, q)), a), b), c)| {
        row(vec![user_col(db, u, "name"), V::I(s[0]), V::I(q[0]), V::I(q[1]), V::I(s[1]), V::I(a), V::I(b), V::I(c)])
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopActiveUsers AS (SELECT UserId, DisplayName, PostCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserActivity)
// SELECT TU.DisplayName, TU.PostCount, TU.UpVotes, TU.DownVotes, TU.GoldBadges, TU.SilverBadges, TU.BronzeBadges FROM TopActiveUsers TU WHERE TU.Rank <= 10 ORDER BY TU.PostCount DESC, TU.UpVotes DESC;
//
// Rank reads only PostCount, so the top users are picked first and the product is driven for them alone.
fn q8535(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let v = top_n(drain(&pc), |&(u, p)| (Reverse(p), u), 10);
    let tu = rel(v);
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let s = (&tu)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (v, b)| {
            let v = v.flatten();
            [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (b == Some(1)) as i64, a[3] + (b == Some(2)) as i64, a[4] + (b == Some(3)) as i64]
        });
    let v = drain((&tu).and(&s));
    rows(v.into_iter().map(|(u, ((_, p), a))| {
        let mut f = vec![user_col(db, u, "name"), V::I(p)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Reputation, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        (SELECT COUNT(DISTINCT bl.RelatedPostId) FROM PostLinks bl WHERE bl.PostId = rp.PostId AND bl.LinkTypeId = 3) AS DuplicateCount
// FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId WHERE rp.Rank <= 5
// GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Reputation ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q9978(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let dups = links_of(db).select(Ident::<PostLink>::new().with((&db.post_link.link_type_id).eq(3))).select(&db.post_link.related_post_id);
    let dc = (&tp).group_by(Ident::<Post>::new()).select(dups.opt()).buf_fold(distinct_some);
    let v = drain((&s).and(&dc));
    rows(v.into_iter().map(|(p, (a, d))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::I(d));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
//     GROUP BY p.Id, p.Title, p.Score, p.PostTypeId, p.CreationDate),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.Title, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, u.DisplayName AS OwnerDisplayName, u.Reputation
// FROM TopPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts p WHERE p.Id = tp.PostId) ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// Rank reads only base columns, so the newest posts are picked first and the comment x vote product is driven for those alone.
fn q8776(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&s).and(owner_user));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN p.PostTypeId IN (3, 4, 5, 7) THEN 1 ELSE 0 END), 0) AS WikiCount,
//        COUNT(DISTINCT c.Id) AS CommentCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, CommentCount, ROW_NUMBER() OVER (ORDER BY QuestionCount DESC, AnswerCount DESC) AS Rank FROM UserPostStats),
// TopComments AS (SELECT UserId, COUNT(*) AS CommentTotal FROM Comments GROUP BY UserId)
// SELECT tu.DisplayName, tu.QuestionCount, tu.AnswerCount, tu.CommentCount, COALESCE(tc.CommentTotal, 0) AS TotalComments
// FROM TopUsers tu LEFT JOIN TopComments tc ON tu.UserId = tc.UserId WHERE tu.Rank <= 10 ORDER BY tu.Rank;
fn q3670(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((t, _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64],
            None => a,
        });
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).buf_fold(distinct_some);
    let tc = db.comment.group_by(&db.comment.user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&s).and(&cc).and((&tc).opt())), |&(u, ((a, _), _))| (Reverse(a[0]), Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, ((a, c), t))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(c), V::I(t.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpvoteCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownvoteCount,
//        RANK() OVER (ORDER BY COUNT(DISTINCT c.Id) DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.PostRank <= 10)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.CommentCount, tp.UpvoteCount, tp.DownvoteCount,
//        CASE WHEN tp.UpvoteCount > tp.DownvoteCount THEN 'Popular' ELSE 'Less Popular' END AS Popularity
// FROM TopPosts tp ORDER BY tp.CommentCount DESC, tp.UpvoteCount DESC;
fn q5478(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let posts = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).group_by(Ident::<Post>::new());
    let cc = posts().select(comments_of(db).opt()).buf_fold(distinct_some);
    let vt = |t: i64| votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(t)));
    let up = posts().select(vt(2).opt()).buf_fold(distinct_some);
    let down = posts().select(vt(3).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&cc).and(&up).and(&down)), |&(p, ((c, _), _))| (Reverse(c), Reverse(creation_date.get(p).unwrap())), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, ((c, u), d)), _)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::S(if u > d { "Popular" } else { "Less Popular" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(B.Class) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RankedUserStats AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, TotalScore, TotalBadges, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        DENSE_RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, DENSE_RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserStats)
// SELECT U.DisplayName, U.Reputation, R.ReputationRank, R.ScoreRank, R.PostRank FROM RankedUserStats R JOIN Users U ON R.UserId = U.Id
// WHERE R.ReputationRank <= 10 OR R.ScoreRank <= 10 OR R.PostRank <= 10 ORDER BY R.ReputationRank, R.ScoreRank, R.PostRank;
fn q8359(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt().and(badges_of(db).opt()))
        .fold(0i64, |n, (s, _)| n + s.unwrap_or(0));
    let pc = user_distinct_posts(db);
    let v = ranked(drain((&s).and(&pc)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), true);
    let v = ranked(v, |&((_, (s, _)), _)| Reverse(s), true);
    let v = ranked(v, |&(((_, (_, p)), _), _)| Reverse(p), true);
    rows(v.into_iter().filter(|&(((_, a), b), c)| a <= 10 || b <= 10 || c <= 10).map(|((((u, _), a), b), c)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a), V::I(b), V::I(c)]);
        row(f)
    }))
}

// Rewritten (rewrites/8350.sql): the final ORDER BY is tie-broken on pp.PostId, v.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score),
// PopularPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, Score, CommentCount, VoteCount FROM RankedPosts WHERE PostRank = 1)
// SELECT pp.*, COALESCE(rvt.Name, 'No votes') AS MostRecentVoteType FROM PopularPosts pp LEFT JOIN Votes v ON pp.PostId = v.PostId LEFT JOIN VoteTypes rvt ON v.VoteTypeId = rvt.Id
// WHERE pp.VoteCount > 5 ORDER BY pp.Score DESC, pp.CommentCount DESC, pp.PostId, v.Id LIMIT 10;
//
// PostRank partitions by p.Id, so it is 1 for every post.
fn q8350(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let posts = || db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).group_by(Ident::<Post>::new());
    let s = posts().select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = posts().select(votes_of(db).opt()).buf_fold(distinct_some);
    let v = drain((&s).and((&vc).filt(|n| n > 5)).and(votes_of(db).opt()));
    let v = top_n(v, |&(p, ((c, _), x))| (Reverse(score.get(p).unwrap()), Reverse(c), p, x), 10);
    rows(v.into_iter().map(|(p, ((c, n), x))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([V::I(c), V::I(n), V::S(x.map_or("No votes", |x| vtype_name(db).get(x).unwrap()))]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.CreationDate) / 1e6 AS AvgActivityDuration
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, AvgActivityDuration, RANK() OVER (ORDER BY PostCount DESC) AS UserRank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, AvgActivityDuration FROM TopUsers WHERE UserRank <= 10 ORDER BY UserRank;
//
// Rewritten (rewrites/8495.sql): the float AVG of epoch seconds is an exact-integer mean.
fn q8495(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select(post_type_id.and(last_activity_date).and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold(([0i64; 5], 0i128), |(a, d), p| match p {
            Some((((t, la), cd), v)) => ([a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + 1], d + (la - cd) as i128),
            None => (a, d),
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&s).and(&pc)), |&(_, (_, p))| Reverse(p), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, d), p)), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.push(if a[4] == 0 { V::Null } else { V::F(d as f64 / a[4] as f64 / 1e6) });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, CommentCount, UpvoteCount, DownvoteCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, CommentCount, UpvoteCount, DownvoteCount, ReputationRank FROM TopUsers WHERE ReputationRank <= 10;
//
// ReputationRank reads only Reputation, so the top users are picked first and the product is driven for them alone.
fn q14823(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tr: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let users = || (&tr).map(|(u, _)| u).group_by(Ident::<User>::new());
    let s = users()
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, c), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = users().select(posts_of(db).opt()).buf_fold(distinct_some);
    let v = drain((&tr).and(&s).and(&pc));
    rows(v.into_iter().map(|(u, (((_, r), a), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(p));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("24494", q24494),
    ("9336", q9336),
    ("12995", q12995),
    ("12354", q12354),
    ("10426", q10426),
    ("29609", q29609),
    ("5477", q5477),
    ("7079", q7079),
    ("7878", q7878),
    ("4907", q4907),
    ("4750", q4750),
    ("27384", q27384),
    ("6812", q6812),
    ("7852", q7852),
    ("8147", q8147),
    ("11924", q11924),
    ("14475", q14475),
    ("14695", q14695),
    ("12422", q12422),
    ("31429", q31429),
    ("447", q447),
    ("11236", q11236),
    ("11294", q11294),
    ("8126", q8126),
    ("13140", q13140),
    ("8010", q8010),
    ("3590", q3590),
    ("10629", q10629),
    ("6807", q6807),
    ("9182", q9182),
    ("7459", q7459),
    ("11377", q11377),
    ("6421", q6421),
    ("5343", q5343),
    ("1745", q1745),
    ("13111", q13111),
    ("11643", q11643),
    ("9043", q9043),
    ("550", q550),
    ("8473", q8473),
    ("13392", q13392),
    ("5993", q5993),
    ("8047", q8047),
    ("5720", q5720),
    ("27058", q27058),
    ("4207", q4207),
    ("5480", q5480),
    ("8984", q8984),
    ("13337", q13337),
    ("6141", q6141),
    ("6202", q6202),
    ("3182", q3182),
    ("7634", q7634),
    ("6041", q6041),
    ("1918", q1918),
    ("6858", q6858),
    ("6976", q6976),
    ("4981", q4981),
    ("5878", q5878),
    ("5413", q5413),
    ("7710", q7710),
    ("6641", q6641),
    ("5317", q5317),
    ("27279", q27279),
    ("707", q707),
    ("8718", q8718),
    ("8902", q8902),
    ("9416", q9416),
    ("12786", q12786),
    ("7591", q7591),
    ("9515", q9515),
    ("10233", q10233),
    ("6402", q6402),
    ("5049", q5049),
    ("7373", q7373),
    ("5272", q5272),
    ("5489", q5489),
    ("5875", q5875),
    ("8516", q8516),
    ("9535", q9535),
    ("3500", q3500),
    ("10277", q10277),
    ("3530", q3530),
    ("1591", q1591),
    ("5508", q5508),
    ("8535", q8535),
    ("9978", q9978),
    ("8776", q8776),
    ("3670", q3670),
    ("5478", q5478),
    ("8359", q8359),
    ("8350", q8350),
    ("8495", q8495),
    ("14823", q14823),
];
