use harness::prelude::*;
use std::cmp::Reverse;

fn like_counts(db: &'static So) -> Fold<Id<Tag>, i64> {
    let tm = tag_mentions(db);
    (&tm).group_by((&tm).map(|(_, t)| t)).fold(0i64, |n, _| n + 1)
}

fn cross_top<A: Copy, B: Copy, KA: Ord + Copy, KB: Ord>(a: Vec<A>, ka: impl Fn(&A) -> KA, b: Vec<B>, kb: impl Fn(&B) -> KB, n: usize) -> Vec<(A, B)> {
    let ra = rel(a);
    let rb = rel(b);
    let ranked = (&ra).map(|_| ()).inv().select(&ra).window(rank, |x: A| ka(&x), asc);
    let nb = count(&rb).max(1);
    let picked = (&ranked).filt(move |(_, r)| (r - 1) * nb < n as i64).map(|(x, _)| x);
    top_n(drain(picked.cross(&rb)), |(_, (x, y))| (ka(x), kb(y)), n).into_iter().map(|x| x.1).collect()
}

fn tname_of(db: &'static So, t: Id<Tag>) -> V {
    V::S(db.tag.tag_name.get(t).unwrap())
}

// SELECT pt.Name AS PostType, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViewCount, COUNT(DISTINCT p.OwnerUserId) AS TotalUsers,
//        COUNT(DISTINCT t.Id) AS TotalTags
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Tags t ON p.Tags LIKE CONCAT('%', t.TagName, '%')
// GROUP BY pt.Name ORDER BY AverageScore DESC;
fn q10681(db: &'static So) -> String {
    let Post { score, view_count, owner_user_id, .. } = &db.post;
    let tm = tag_mentions(db);
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&tm).map(|(p, _)| p).inv().collect();
    let g = db
        .post
        .group_by(ptype_name(db))
        .select(score.and(view_count.opt()).and(owner_user_id.opt()).and((&by_post).map(|(_, t)| t).opt()))
        .buf_fold(|v| {
            let s: i64 = v.iter().map(|x| x.0 .0 .0).sum();
            let vs: i64 = v.iter().map(|x| x.0 .0 .1.unwrap_or(0)).sum();
            let vn = v.iter().filter(|x| x.0 .0 .1.is_some()).count() as i64;
            (v.len() as i64, s, vs, vn, distinct_some(v.iter().map(|x| x.0 .1)), distinct_some(v.iter().map(|x| x.1)))
        });
    rows(drain(&g).into_iter().map(|(t, (n, s, vs, vn, u, k))| row(vec![V::S(t), avg(s, n), avg(vs, vn), V::I(u), V::I(k)])))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) GROUP BY U.Id, U.DisplayName),
// TagStats AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS TotalPostsWithTag FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%') GROUP BY T.TagName)
// SELECT US.UserId, US.DisplayName, US.TotalPosts, US.TotalQuestions, US.TotalAnswers, US.TotalBounty, (SELECT COUNT(*) FROM TagStats) AS UniqueTagsCount
// FROM UserStats US ORDER BY US.TotalPosts DESC LIMIT 10;
fn q12726(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bv = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.in_v(vec![8, 9])).select(bounty_amount.opt()));
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(&db.post.post_type_id).and(bv.opt())).opt())
        .buf_fold(|v| {
            let mut a = [0i64; 4];
            for x in v.iter().flatten() {
                a[0] += (x.0 .1 == 1) as i64;
                a[1] += (x.0 .1 == 2) as i64;
                if let Some(Some(b)) = x.1 {
                    a[2] += b;
                    a[3] += 1;
                }
            }
            (distinct_some(v.iter().map(|x| x.map(|x| x.0 .0))), a)
        });
    let lc = like_counts(db);
    let k = count(&lc);
    let v = top_n(drain(&us), |&(_, (n, _))| Reverse(n), 10);
    rows(v.into_iter().map(|(u, (n, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[2], a[3]), V::I(k)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PopularTags AS (SELECT T.TagName, COUNT(P.Tags) AS TagPostCount FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%<', T.TagName, '>%')
//     GROUP BY T.TagName ORDER BY TagPostCount DESC LIMIT 5),
// TopUsers AS (SELECT U.Id, U.DisplayName, US.Reputation, US.TotalPosts, US.TotalQuestions, US.TotalAnswers, US.TotalUpvotedPosts
//     FROM UserStatistics US JOIN Users U ON US.UserId = U.Id WHERE US.Reputation > 1000 ORDER BY US.Reputation DESC LIMIT 10)
// SELECT TU.DisplayName AS TopUser, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalUpvotedPosts, PT.TagName AS PopularTag, PT.TagPostCount
// FROM TopUsers TU CROSS JOIN PopularTags PT ORDER BY TU.Reputation DESC, PT.TagPostCount DESC;
fn q5626(db: &'static So) -> String {
    let reputation = &db.user.reputation;
    let up = user_posts(db);
    let tu = top_n(drain(db.user.with(reputation.gt(1000)).select(Ident::<User>::new().and(reputation).and(&up))), |&(_, ((_, r), _))| Reverse(r), 10);
    let pc = db.post.group_by(&db.post.tags).fold(0i64, |n, _| n + 1);
    let pt = top_n(drain(&pc), |&(_, n)| Reverse(n), 5);
    let mut out = Vec::new();
    rel(tu).cross(rel(pt)).drive(|_, ((_, ((u, r), a)), (t, n))| {
        out.push(row(vec![user_col(db, u, "name"), V::I(r), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), tname_of(db, t), V::I(n)]))
    });
    rows(out)
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(p.Score) AS AverageScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// MostActiveUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, AverageScore, RANK() OVER (ORDER BY PostCount DESC) AS UserRank FROM UserPostStats),
// TopTags AS (SELECT t.TagName, COUNT(p.Id) AS UsageCount FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%') GROUP BY t.TagName ORDER BY UsageCount DESC LIMIT 10)
// SELECT mu.DisplayName, mu.PostCount, mu.QuestionCount, mu.AnswerCount, mu.AverageScore, tt.TagName, tt.UsageCount
// FROM MostActiveUsers mu CROSS JOIN TopTags tt WHERE mu.UserRank <= 10 ORDER BY mu.PostCount DESC, tt.UsageCount DESC;
fn q5361(db: &'static So) -> String {
    let up = user_posts(db);
    let mu = ranked(drain(&up), |&(_, a)| Reverse(a[1]), false);
    let tt = top_n(drain(&like_counts(db)), |&(_, n)| Reverse(n), 10);
    let mut out = Vec::new();
    rel(mu).filt(|(_, r)| r <= 10).cross(rel(tt)).drive(|_, (((u, a), _), (t, n))| {
        out.push(row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1]), tname_of(db, t), V::I(n)]))
    });
    rows(out)
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalScore, AnswerCount, QuestionCount, RANK() OVER (ORDER BY TotalScore DESC, Reputation DESC) AS UserRank FROM UserStats),
// PopularTags AS (SELECT T.TagName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount
//     FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%') GROUP BY T.TagName HAVING COUNT(P.Id) > 50),
// TopTags AS (SELECT TagName, PostCount, QuestionCount, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM PopularTags)
// SELECT U.DisplayName AS TopUser, U.Reputation, U.PostCount AS TotalPosts, U.TotalScore AS Score, U.QuestionCount, U.AnswerCount, T.TagName AS MostPopularTag, T.PostCount AS TagPostCount
// FROM TopUsers U JOIN TopTags T ON U.QuestionCount > 0 WHERE U.UserRank <= 10 AND T.TagRank = 1 ORDER BY U.TotalScore DESC, U.Reputation DESC;
fn q6181(db: &'static So) -> String {
    let up = user_posts(db);
    let tu = ranked(drain(db.user.select(Ident::<User>::new().and(&db.user.reputation).and(&up))), |&(_, ((_, r), a))| (Reverse(a[4]), Reverse(r)), false);
    let tt = ranked(drain(like_counts(db).filt(|n| n > 50)), |&(_, n)| Reverse(n), false);
    let mut out = Vec::new();
    rel(tu).filt(|((_, (_, a)), r)| r <= 10 && a[2] > 0).cross(rel(tt).filt(|(_, r)| r == 1)).drive(|_, (((_, ((u, rep), a)), _), ((t, n), _))| {
        out.push(row(vec![user_col(db, u, "name"), V::I(rep), V::I(a[1]), V::I(a[4]), V::I(a[2]), V::I(a[3]), tname_of(db, t), V::I(n)]))
    });
    rows(out)
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS HighViews, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, PositivePosts, NegativePosts, HighViews, LastPostDate, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats),
// TopPostTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore, ROW_NUMBER() OVER (ORDER BY COUNT(p.Id) DESC) AS TagRank
//     FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%') GROUP BY t.TagName)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.PositivePosts, tu.NegativePosts, tu.HighViews, tu.LastPostDate, tpt.TagName, tpt.PostCount AS TagPostCount, tpt.TotalViews, tpt.TotalScore
// FROM TopUsers tu JOIN TopPostTags tpt ON tu.PostCount > 0 AND tpt.TagRank <= 5 WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC, tpt.PostCount DESC;
fn q9482(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(creation_date)).opt())
        .fold([0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some(((s, v), d)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + (v.unwrap_or(0) > 100) as i64, a[4].max(d)],
            None => a,
        });
    let tu = ranked(drain(db.user.select(Ident::<User>::new().and(&db.user.reputation).and(&us))), |&(_, ((_, r), _))| Reverse(r), false);
    let tm = tag_mentions(db);
    let ts = (&tm)
        .group_by((&tm).map(|(_, t)| t))
        .select((&tm).map(|(p, _)| p).select(view_count.opt().and(score)))
        .fold([0i64; 4], |a, (v, s)| [a[0] + 1, a[1] + v.unwrap_or(0), a[2] + v.is_some() as i64, a[3] + s]);
    let tt = top_n(drain(&ts), |&(_, a)| Reverse(a[0]), 5);
    let mut out = Vec::new();
    rel(tu).filt(|((_, (_, a)), r)| r <= 10 && a[0] > 0).cross(rel(tt)).drive(|_, (((_, ((u, rep), a)), _), (t, b))| {
        out.push(row(vec![user_col(db, u, "name"), V::I(rep), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), tmax(a[4]), tname_of(db, t), V::I(b[0]), nullable(b[1], b[2]), V::I(b[3])]))
    });
    rows(out)
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, AVG(P.Score) AS AverageScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopTags AS (SELECT T.TagName, COUNT(P.Id) AS PostCount FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%') GROUP BY T.TagName ORDER BY PostCount DESC LIMIT 10),
// RecentVotes AS (SELECT V.UserId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes V JOIN VoteTypes VT ON V.VoteTypeId = VT.Id WHERE V.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 DAYS' GROUP BY V.UserId)
// SELECT U.DisplayName AS UserDisplayName, U.Reputation AS UserReputation, COALESCE(US.TotalPosts, 0) AS UserTotalPosts, COALESCE(US.PositivePosts, 0) AS UserPositivePosts,
//        COALESCE(US.NegativePosts, 0) AS UserNegativePosts, COALESCE(US.AverageScore, 0) AS UserAverageScore, TT.TagName AS PopularTag, TT.PostCount AS TagPostCount,
//        RV.VoteCount AS RecentVoteCount, RV.UpVotes AS RecentUpVotes, RV.DownVotes AS RecentDownVotes
// FROM Users U LEFT JOIN UserStats US ON U.Id = US.UserId LEFT JOIN TopTags TT ON TRUE LEFT JOIN RecentVotes RV ON U.Id = RV.UserId
// WHERE U.Reputation > 1000 ORDER BY U.Reputation DESC, TagPostCount DESC LIMIT 50;
fn q333(db: &'static So) -> String {
    let Post { score, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt())
        .fold([0i64; 4], |a, s| match s {
            Some(s) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + s],
            None => a,
        });
    let Vote { creation_date, user, .. } = &db.vote;
    let rv = db
        .vote
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(user)
        .select(vtype_name(db))
        .fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let uv = drain((&us).and(&db.user.reputation).and((&rv).opt()));
    let tt = top_n(drain(&like_counts(db)), |&(_, n)| Reverse(n), 10);
    let v = cross_top(uv, |&(_, ((_, r), _))| Reverse(r), tt, |&(_, n)| Reverse(n), 50);
    rows(v.into_iter().map(|((u, ((a, r), rv)), (t, n))| {
        let mut f = vec![user_col(db, u, "name"), V::I(r), V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[0] == 0 { V::F(0.0) } else { avg(a[3], a[0]) }, tname_of(db, t), V::I(n)];
        match rv {
            Some(b) => f.extend(b.map(V::I)),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// ActiveUsers AS (SELECT UserId, Reputation, PostCount, AnswerCount, AcceptedAnswerCount FROM UserReputation WHERE Reputation > 1000 AND PostCount > 5),
// TopTags AS (SELECT t.TagName, COUNT(p.Id) AS TagPostCount FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%') GROUP BY t.TagName ORDER BY TagPostCount DESC LIMIT 10),
// UserTagEngagement AS (SELECT au.UserId, tt.TagName, COUNT(p.Id) AS EngagementCount FROM ActiveUsers au JOIN Posts p ON p.OwnerUserId = au.UserId
//     JOIN Tags t ON p.Tags LIKE CONCAT('%', t.TagName, '%') JOIN TopTags tt ON tt.TagName = t.TagName GROUP BY au.UserId, tt.TagName)
// SELECT au.UserId, au.Reputation, tt.TagName, COALESCE(uge.EngagementCount, 0) AS EngagementCount
// FROM ActiveUsers au CROSS JOIN TopTags tt LEFT JOIN UserTagEngagement uge ON au.UserId = uge.UserId AND tt.TagName = uge.TagName
// ORDER BY au.Reputation DESC, tt.TagPostCount DESC, EngagementCount DESC;
fn q8972(db: &'static So) -> String {
    let up = user_posts(db);
    let au = drain(db.user.with((&db.user.reputation).gt(1000)).with((&up).filt(|a| a[1] > 5)).select(Ident::<User>::new())).into_iter().map(|x| x.1).collect::<Vec<_>>();
    let tt = top_n(drain(&like_counts(db)), |&(_, n)| Reverse(n), 10);
    let tm = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    let uge = (&tm).group_by(Same::<M>::new().map(|x: M| x.0).select(&db.post.owner_user).and(Same::<M>::new().map(|x: M| x.1))).fold(0i64, |n, _| n + 1);
    type P = (Id<User>, (Id<Tag>, i64));
    let pairs = rel(au).cross(rel(tt));
    let mut out = Vec::new();
    (&pairs).select(Same::<P>::new().and(Same::<P>::new().map(|x: P| (x.0, x.1 .0)).select(&uge).opt())).drive(|_, ((u, (t, _)), e)| {
        out.push(row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), tname_of(db, t), V::I(e.unwrap_or(0))]))
    });
    rows(out)
}

// WITH TagStats AS (SELECT Tags.TagName, COUNT(DISTINCT Posts.Id) AS PostCount, SUM(Posts.ViewCount) AS TotalViews, AVG(Users.Reputation) AS AvgReputation, COUNT(DISTINCT Users.Id) AS UserCount
//     FROM Tags JOIN Posts ON Posts.Tags LIKE CONCAT('%<', Tags.TagName, '>%' ) JOIN Users ON Posts.OwnerUserId = Users.Id GROUP BY Tags.TagName),
// TopTags AS (SELECT TagName, PostCount, TotalViews, AvgReputation, UserCount, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM TagStats),
// PopularTags AS (SELECT TagName, PostCount, TotalViews, AvgReputation, UserCount FROM TopTags WHERE ViewRank <= 10),
// UserBadges AS (SELECT Users.Id AS UserId, Users.DisplayName, COUNT(Badges.Id) AS BadgeCount, SUM(CASE WHEN Badges.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
//        SUM(CASE WHEN Badges.Class = 2 THEN 1 ELSE 0 END) AS SilverCount, SUM(CASE WHEN Badges.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
//     FROM Users LEFT JOIN Badges ON Users.Id = Badges.UserId GROUP BY Users.Id, Users.DisplayName),
// TopUsers AS (SELECT DisplayName, BadgeCount, GoldCount, SilverCount, BronzeCount, RANK() OVER (ORDER BY BadgeCount DESC) AS BadgeRank FROM UserBadges WHERE BadgeCount > 0)
// SELECT tt.TagName, tt.PostCount, tt.TotalViews, tt.AvgReputation, uu.DisplayName AS TopUser, uu.BadgeCount, uu.GoldCount, uu.SilverCount, uu.BronzeCount
// FROM PopularTags tt JOIN TopUsers uu ON uu.BadgeRank = 1 ORDER BY tt.TotalViews DESC;
fn q25571(db: &'static So) -> String {
    let Post { owner_user, view_count, tags, .. } = &db.post;
    let ts = db
        .post
        .with(owner_user)
        .group_by(tags)
        .select(view_count.opt().and(owner_user.select(&db.user.reputation)))
        .fold([0i64; 4], |a, (v, r)| [a[0] + 1, a[1] + v.unwrap_or(0), a[2] + v.is_some() as i64, a[3] + r]);
    let tt = ranked(drain(&ts), |&(_, a)| (a[2] > 0, Reverse(a[1])), false);
    let ub = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class))
        .fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let uu = ranked(drain(&ub), |&(_, a)| Reverse(a[0]), false);
    let mut out = Vec::new();
    rel(tt).filt(|(_, r)| r <= 10).cross(rel(uu).filt(|(_, r)| r == 1)).drive(|_, (((t, a), _), ((u, b), _))| {
        let mut f = vec![tname_of(db, t), V::I(a[0]), nullable(a[1], a[2]), avg(a[3], a[0]), user_col(db, u, "name")];
        f.extend(b.map(V::I));
        out.push(row(f))
    });
    rows(out)
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// PopularTags AS (SELECT T.TagName, COUNT(P.Id) AS TagUsageCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName HAVING COUNT(P.Id) > 0),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, CONCAT('/posts/', P.Id) AS PostUrl FROM Posts P WHERE P.PostTypeId IN (1, 2) ORDER BY P.Score DESC, P.ViewCount DESC LIMIT 10)
// SELECT RU.UserId, RU.DisplayName, RU.Reputation, RU.TotalPosts, RU.QuestionCount, RU.AnswerCount, PT.TagName, PT.TagUsageCount, TP.PostId, TP.Title, TP.ViewCount, TP.Score, TP.PostUrl
// FROM RankedUsers RU CROSS JOIN PopularTags PT CROSS JOIN TopPosts TP WHERE RU.UserRank <= 100 ORDER BY RU.Reputation DESC, PT.TagUsageCount DESC, TP.Score DESC;
fn q26702(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let up = user_posts(db);
    let ru = top_n(drain(db.user.with(reputation.gt(0)).select(Ident::<User>::new().and(reputation).and(&up))), |&(_, ((_, r), _))| Reverse(r), 100);
    let pt = drain(like_counts(db).filt(|n| n > 0));
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let tp = top_n(drain(db.post.with(post_type_id.in_v(vec![1, 2])).select(Ident::<Post>::new().and(score).and(view_count.opt()))), |&(_, ((_, s), v))| (Reverse(s), v.is_some(), Reverse(v)), 10);
    let mut out = Vec::new();
    rel(ru).cross(rel(pt)).cross(rel(tp)).drive(|_, (((_, ((u, rep), a)), (t, n)), (_, ((p, s), v)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(rep), V::I(a[1]), V::I(a[2]), V::I(a[3]), tname_of(db, t), V::I(n)]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([oint(v), V::I(s), V::Owned(format!("/posts/{}", db.post.origid.get(p).unwrap()))]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(a.Id) AS AnswerCount, COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount,
//        SUM(v.BountyAmount) AS TotalBountyAmount
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.AnswerCount, ps.CommentCount, ps.BadgeCount AS PostOwnerBadgeCount, ps.TotalBountyAmount AS PostBountyAmount,
//        us.UserId, us.DisplayName, us.BadgeCount AS UserBadgeCount, us.TotalBountyAmount AS UserTotalBountyAmount
// FROM PostStatistics ps JOIN Users u ON ps.PostId = u.Id JOIN UserStatistics us ON u.Id = us.UserId ORDER BY ps.CreationDate DESC;
//
// Only the posts whose Id is also a user Id reach the output, so the product is driven for those.
fn q12105(db: &'static So) -> String {
    let Post { origid, post_type_id, owner_user, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pu: HashIdx<Id<Post>, Id<User>> = origid.select(&uid).collect();
    let bv = || Ident::<Vote>::new().with(vote_type_id.in_v(vec![8, 9])).select(bounty_amount.opt());
    let ps = db
        .post
        .with(&pu)
        .group_by(Ident::<Post>::new())
        .select(
            Ident::<Post>::new()
                .with(post_type_id.eq(1))
                .select(children_of(db))
                .opt()
                .and(comments_of(db).opt())
                .and(owner_user.select(badges_of(db)).opt())
                .and(votes_of(db).select(bv()).opt()),
        )
        .fold([0i64; 5], |a, (((x, c), b), v)| {
            let bo = v.flatten();
            [a[0] + x.is_some() as i64, a[1] + c.is_some() as i64, a[2] + b.is_some() as i64, a[3] + bo.unwrap_or(0), a[4] + bo.is_some() as i64]
        });
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(bv()).opt()))
        .fold([0i64; 3], |a, (b, v)| {
            let bo = v.flatten();
            [a[0] + b.is_some() as i64, a[1] + bo.unwrap_or(0), a[2] + bo.is_some() as i64]
        });
    let mut out = Vec::new();
    (&ps).and((&pu).select(Ident::<User>::new().and(&us))).drive(|p, (a, (u, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[4])]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b[0]), nullable(b[1], b[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (SELECT p.Id AS PostId, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, MAX(p.LastActivityDate) AS LastActivityDate, MAX(p.CreationDate) AS CreationDate, p.Title, p.Body
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.Body),
// UsersStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostsCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT ps.PostId, ps.Title, ps.CommentCount, ps.AnswerCount, ps.UpVotes, ps.DownVotes, ps.LastActivityDate, ps.CreationDate, us.UserId, us.PostsCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges
// FROM PostStats ps JOIN UsersStats us ON ps.PostId = us.UserId ORDER BY ps.LastActivityDate DESC LIMIT 100;
//
// The order reads only base columns, so the hundred posts (and their namesake users) are picked first.
fn q10567(db: &'static So) -> String {
    let Post { origid, post_type_id, last_activity_date, .. } = &db.post;
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pu: HashIdx<Id<Post>, Id<User>> = origid.select(&uid).collect();
    let top = top_n(drain(db.post.with(&pu).select(Ident::<Post>::new().and(last_activity_date))), |&(_, (_, d))| Reverse(d), 100);
    let tp: MatSet<Id<Post>> = rel(top).map(|(_, (p, _))| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(Ident::<Post>::new().with(post_type_id.eq(1)).select(children_of(db)).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .buf_fold(|v| {
            (
                distinct_some(v.iter().map(|x| x.0 .0)),
                distinct_some(v.iter().map(|x| x.0 .1)),
                v.iter().filter(|x| x.1 == Some(2)).count() as i64,
                v.iter().filter(|x| x.1 == Some(3)).count() as i64,
            )
        });
    let tu: MatSet<Id<User>> = (&tp).select(&pu).collect();
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).buf_fold(|v| {
        (
            distinct_some(v.iter().map(|x| x.0)),
            v.iter().filter(|x| x.1 == Some(1)).count() as i64,
            v.iter().filter(|x| x.1 == Some(2)).count() as i64,
            v.iter().filter(|x| x.1 == Some(3)).count() as i64,
        )
    });
    let mut out = Vec::new();
    (&ps).and((&pu).select(Ident::<User>::new().and(&us))).drive(|p, ((c, a, up, dn), (u, (n, g, s, b)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c), V::I(a), V::I(up), V::I(dn)]);
        f.extend(post_fields(db, p, &["activity", "created"]));
        f.extend([user_col(db, u, "uid"), V::I(n), V::I(g), V::I(s), V::I(b)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(p.Score) AS TotalScore,
//        SUM(ROUND(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) / 3600)) AS TotalActiveHours
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostTags AS (SELECT p.Id AS PostId, p.Title, ARRAY_LENGTH(STRING_TO_ARRAY(p.Tags, '<>'), 1) AS TagCount FROM Posts p)
// SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalScore, ups.TotalActiveHours, pt.TagCount
// FROM UserPostStats ups LEFT JOIN PostTags pt ON ups.TotalPosts > 0 AND pt.PostId = ups.TotalPosts ORDER BY ups.TotalScore DESC;
//
// EXTRACT is numeric in Postgres, so the hours are the exact quotient rounded half away from zero.
fn q11603(db: &'static So) -> String {
    let Post { post_type_id, score, last_activity_date, creation_date, tags_str, .. } = &db.post;
    const H: i64 = 3_600_000_000;
    let hours = last_activity_date.and(creation_date).map(|(l, c): (i64, i64)| {
        let d = l - c;
        d / H + if 2 * (d % H).abs() >= H { (d % H).signum() } else { 0 }
    });
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(hours)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, s), h)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + h],
            None => a,
        });
    let pid: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let tc = (&ups).filt(|a| a[0] > 0).map(|a| a[0]).select(&pid).select(tags_str.opt());
    let mut out = Vec::new();
    (&ups).and(tc.opt()).drive(|u, (a, t)| {
        let n = match t.flatten() {
            Some(t) if !t.is_empty() => V::I(t.split("<>").count() as i64),
            _ => V::Null,
        };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), if a[0] == 0 { V::Null } else { V::F(a[4] as f64) }, n]);
        out.push(row(f))
    });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id)
// SELECT up.UserId, up.BadgeCount, up.GoldBadges, up.SilverBadges, up.BronzeBadges, rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CreationDate, pc.CommentCount, pc.LastCommentDate
// FROM UserBadges up JOIN RankedPosts rp ON up.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) JOIN PostComments pc ON rp.PostId = pc.PostId
// WHERE up.BadgeCount > 0 ORDER BY up.BadgeCount DESC, rp.Score DESC, pc.CommentCount DESC;
//
// Rank is never read.
fn q5190(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let ub = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class))
        .fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let q = || db.post.with(post_type_id.eq(1));
    let pc = q()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(&db.comment.creation_date).opt())
        .fold((0i64, i64::MIN), |(n, m), d| match d {
            Some(d) => (n + 1, m.max(d)),
            None => (n, m),
        });
    let mut out = Vec::new();
    q().with(score.gt(0)).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ub))).and(&pc)).drive(|_, ((p, (u, a)), (n, m))| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "answers", "created"]));
        f.extend([V::I(n), tmax(m)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.rn = 1 AND rp.CommentCount > 5 AND rp.VoteCount >= 10),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, COUNT(DISTINCT b.Id) AS BadgesEarned
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 3)
// SELECT fp.Title AS PostTitle, fp.CreationDate AS PostCreationDate, fp.Score AS PostScore, tu.DisplayName AS UserName, tu.PostsCreated, tu.BadgesEarned,
//        LPAD(CAST(tu.BadgesEarned AS TEXT), 2, '0') AS FormattedBadgesEarned
// FROM FilteredPosts fp JOIN Users u ON fp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = u.Id) JOIN TopUsers tu ON tu.UserId = u.Id
// ORDER BY fp.Score DESC, tu.BadgesEarned DESC LIMIT 10;
//
// rn is 1 on every row (each partition is one post). LPAD to 2 also truncates a longer count to its first two digits.
fn q28481(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let vt = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![2, 3])));
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(vt.opt()))
        .buf_fold(|v| (v.iter().filter(|x| x.0.is_some()).count() as i64, distinct_some(v.iter().map(|x| x.1))));
    let tu = user_distinct_posts(db);
    let bc = badges_per_user(db);
    let fp = db.post.with((&rp).filt(|(c, n)| c > 5 && n >= 10));
    let cand = drain(fp.select(Ident::<Post>::new().and(score).and(owner_user.select(Ident::<User>::new().with((&tu).filt(|n| n > 3)).and(&tu).and(&bc)))));
    let v = top_n(cand, |&(_, ((_, s), (_, b)))| (Reverse(s), Reverse(b)), 10);
    rows(v.into_iter().map(|(_, ((p, _), ((u, n), b)))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        let t = b.to_string();
        f.extend([user_col(db, u, "name"), V::I(n), V::I(b), V::Owned(if t.len() >= 2 { t[..2].to_string() } else { format!("{t:0>2}") })]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostCounts AS (SELECT OwnerUserId, COUNT(Id) AS PostCount FROM Posts WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '1 year' GROUP BY OwnerUserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, COALESCE(UPC.PostCount, 0) AS TotalPosts FROM Users U LEFT JOIN UserPostCounts UPC ON U.Id = UPC.OwnerUserId
//     WHERE U.Reputation > (SELECT AVG(Reputation) FROM Users)),
// RecentPosts AS (SELECT P.Id, P.Title, P.CreationDate, P.ViewCount, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RowNum
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '3 days')
// SELECT T.DisplayName, T.Reputation, T.TotalPosts, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate, RP.ViewCount AS RecentPostViews,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId IN (SELECT Id FROM Posts WHERE OwnerUserId = T.Id) AND V.VoteTypeId = 2) AS TotalUpvotes,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId IN (SELECT Id FROM Posts WHERE OwnerUserId = T.Id)) AS TotalComments
// FROM TopUsers T LEFT JOIN RecentPosts RP ON T.Id = RP.OwnerUserId AND RP.RowNum = 1
// WHERE EXISTS (SELECT 1 FROM Badges B WHERE B.UserId = T.Id AND B.Class = 1) ORDER BY T.Reputation DESC, RecentPostViews DESC LIMIT 10;
//
// Not recursive: no CTE refers to itself. The order reads no subquery, so the ten users are picked first.
fn q30248(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let reputation = &db.user.reputation;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let upc = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user).fold(0i64, |n, _| n + 1);
    let (s, n) = db.user.select(reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let rp: HashIdx<Id<User>, Id<Post>> = db
        .post
        .with(creation_date.ge(add_days(t0, -3)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, k)| k == 1)
        .map(|((p, _), _)| p)
        .collect();
    let tu = db.user.with(reputation.filt(move |r| r * n > s)).with(badges_of(db).select(&db.badge.class).eq(1));
    let cand = drain(tu.select(Ident::<User>::new().and(reputation).and((&rp).select(view_count.opt()).opt())));
    let top = top_n(cand, |&(_, ((_, r), v))| (Reverse(r), v.flatten().is_some(), Reverse(v.flatten())), 10);
    let tu: MatSet<Id<User>> = rel(top).map(|(_, ((u, _), _))| u).collect();
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let mut out = Vec::new();
    let nv = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(up).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let nc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    (&nv)
        .and(&nc)
        .and((&upc).opt())
        .and((&rp).opt())
        .drive(|u, (((v, c), k), p)| {
            let mut f = ucols(db, u, &["name", "rep"]);
            f.push(V::I(k.unwrap_or(0)));
            match p {
                Some(p) => f.extend(post_fields(db, p, &["title", "created", "views"])),
                None => f.extend([V::Null, V::Null, V::Null]),
            }
            f.extend([V::I(v), V::I(c)]);
            out.push(row(f))
        });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Comments c ON u.Id = c.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, (SELECT u.DisplayName FROM Users u WHERE u.Id = p.OwnerUserId) AS OwnerDisplayName, p.Score, p.CreationDate, p.ViewCount,
//        COUNT(c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.OwnerUserId, p.Score, p.CreationDate, p.ViewCount
//     HAVING COUNT(c.Id) > 5 OR SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) > 10 ORDER BY p.Score DESC LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, ue.DisplayName AS EngagedUser, ue.VoteCount, ue.CommentCount, pp.TotalComments, pp.UpVotes, pp.DownVotes, pp.OwnerDisplayName
// FROM RankedPosts rp JOIN UserEngagement ue ON ue.UserId IN (SELECT UserId FROM Votes WHERE PostId = rp.PostId AND VoteTypeId IN (2, 3) GROUP BY UserId HAVING COUNT(*) > 5)
// JOIN PopularPosts pp ON pp.Id = rp.PostId ORDER BY rp.Rank, ue.VoteCount DESC;
//
// Rank is only sorted on. UserEngagement is folded only for the users the IN list can name.
fn q31905(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let Vote { vote_type_id, post, user, .. } = &db.vote;
    let pv = db.vote.with(vote_type_id.in_v(vec![2, 3])).group_by(post.and(user)).fold(0i64, |n, _| n + 1);
    let hv = rel(drain((&pv).filt(|n| n > 5)).into_iter().map(|(k, _)| k).collect());
    let hi: HashIdx<Id<Post>, (Id<Post>, Id<User>)> = (&hv).map(|(p, _)| p).inv().select(&hv).collect();
    let uu: MatSet<Id<User>> = (&hv).map(|(_, u)| u).collect();
    let ue = (&uu).group_by(Ident::<User>::new()).select(votes_by(db).opt().and(comments_by(db).opt())).fold((0i64, 0i64), |(a, b), (v, c)| (a + v.is_some() as i64, b + c.is_some() as i64));
    let pp = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ppv = rel(top_n(drain((&pp).filt(|a| a[0] > 5 || a[1] > 10).and(score)), |&(_, (_, s))| Reverse(s), 10));
    let ppi: HashIdx<Id<Post>, (Id<Post>, ([i64; 3], i64))> = (&ppv).map(|(p, _)| p).inv().select(&ppv).collect();
    let mut out = Vec::new();
    db.post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .select(Ident::<Post>::new().and((&hi).map(|(_, u)| u).select(Ident::<User>::new().and(&ue))).and(&ppi))
        .drive(|_, ((p, (u, (v, c))), (_, (a, _)))| {
            let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
            f.extend([user_col(db, u, "name"), V::I(v), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
            f.extend(post_fields(db, p, &["owner"]));
            out.push(row(f))
        });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName AS OwnerName, COUNT(DISTINCT a.Id) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        RANK() OVER (ORDER BY COUNT(DISTINCT a.Id) DESC) AS RankByAnswerCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName),
// RecentActivity AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, MAX(ph.CreationDate) AS LastEditDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id)
// SELECT r.PostId, r.Title, r.Body, r.Tags, r.CreationDate, r.OwnerName, r.AnswerCount, r.UpVotes, r.DownVotes, ra.CommentCount, ra.LastEditDate,
//        CASE WHEN ra.LastEditDate IS NOT NULL THEN 'Edited' ELSE 'Not Edited' END AS EditStatus, r.RankByAnswerCount
// FROM RankedPosts r JOIN RecentActivity ra ON r.PostId = ra.PostId WHERE r.RankByAnswerCount <= 10 ORDER BY r.RankByAnswerCount;
//
// The rank reads only the distinct answer count, so the ranked questions are picked first.
fn q26729(db: &'static So) -> String {
    let q = db.post.with((&db.post.post_type_id).eq(1));
    let ac = q.group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let rk = whole(&ac).select(Same::new().and(&ac)).window(rank, |(_, n)| n, desc);
    let tr = rel(drain((&rk).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((p, n), r))| (p, (n, r))).collect());
    let ti: HashIdx<Id<Post>, (Id<Post>, (i64, i64))> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let rs = (&ti)
        .map(|(p, _)| p)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(u, d), (_, t)| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let ra = (&ti)
        .map(|(p, _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, d.map_or(m, |d| m.max(d))));
    let mut out = Vec::new();
    (&rs).and(&ra).and(&ti).drive(|p, (((up, dn), (c, m)), (_, (n, r)))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "owner"]);
        f.extend([V::I(n), V::I(up), V::I(dn), V::I(c), tmax(m), V::S(if m == i64::MIN { "Not Edited" } else { "Edited" }), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, RANK() OVER (ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalUpVotes, TotalDownVotes, Rank FROM UserStatistics WHERE Rank <= 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(COUNT(DISTINCT c.Id), 0) AS CommentCount, COALESCE(AVG(v.BountyAmount), 0) AS AverageBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalComments, tu.TotalUpVotes, tu.TotalDownVotes, pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.CommentCount, pd.AverageBounty
// FROM TopUsers tu JOIN PostDetails pd ON tu.UserId = pd.PostId ORDER BY tu.Rank, pd.ViewCount DESC;
//
// The distinct counts come from one row per parent; PostDetails is folded only for the posts whose Id is a top user's.
fn q401(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(comments_by(db).opt()).and(votes_by(db).select(vote_type_id).opt()))
        .fold((0i64, 0i64), |(u, d), (_, t)| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let rk = whole(&ua).select(Same::new().and(&ua)).window(rank, |(_, (u, _))| u, desc);
    let tu = rel(drain((&rk).filt(|(_, r)| r <= 10)).into_iter().map(|(_, (x, _))| x).collect());
    let tui: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pu: HashIdx<Id<Post>, Id<User>> = (&db.post.origid).select(&uid).collect();
    let since = add_days(utc_to_ny(now_utc()), -30);
    let pd = db.post.with((&db.post.creation_date).ge(since)).with((&pu).select(&tui));
    let pds = pd
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8)).select(bounty_amount.opt())).opt()))
        .buf_fold(|v| {
            let b: Vec<i64> = v.iter().filter_map(|x| x.1.flatten()).collect();
            (distinct_some(v.iter().map(|x| x.0)), b.iter().sum::<i64>(), b.len() as i64)
        });
    let dp = user_distinct_posts(db);
    let mut out = Vec::new();
    (&pds).and((&pu).select((&tui).and(&dp).and(&comments_per_user(db)))).drive(|p, ((c, bs, bn), (((u, (up, dn)), n), k))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(k), V::I(up), V::I(dn)];
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend([V::I(c), if bn == 0 { V::F(0.0) } else { avg(bs, bn) }]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// ActiveUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalUpvotes, TotalDownvotes, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank,
//        RANK() OVER (ORDER BY TotalComments DESC) AS CommentRank FROM UserActivity),
// ActivePostStats AS (SELECT P.Id AS PostId, P.Title AS PostTitle, P.CreationDate, P.Score, COALESCE(PV.VoteCount, 0) AS VoteCount, COALESCE(PC.CommentCount, 0) AS CommentCount, P.Tags, P.OwnerUserId
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) PV ON P.Id = PV.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) PC ON P.Id = PC.PostId WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '30 days')
// SELECT AU.DisplayName, AU.TotalPosts, AU.TotalComments, AU.TotalUpvotes, AU.TotalDownvotes, PPS.PostTitle, PPS.CreationDate AS PostDate, PPS.Score, PPS.VoteCount AS PostVoteCount,
//        PPS.CommentCount AS PostCommentCount
// FROM ActiveUsers AU JOIN ActivePostStats PPS ON AU.UserId = PPS.OwnerUserId WHERE AU.TotalPosts > 0 OR AU.TotalComments > 0 ORDER BY AU.TotalPosts DESC, AU.TotalComments DESC LIMIT 100;
//
// The ranks are never read. UserActivity is folded only for the owners of the recent posts the join keeps.
fn q3250(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let ap = || db.post.with(creation_date.ge(add_days(current_date(), -30))).with(owner_user);
    let au: MatSet<Id<User>> = ap().select(owner_user).collect();
    let ua = (&au)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(comments_by(db).opt()))
        .buf_fold(|v| {
            (
                distinct_some(v.iter().map(|x| x.0.map(|p| p.0))),
                distinct_some(v.iter().map(|x| x.1)),
                v.iter().filter(|x| x.0.and_then(|p| p.1) == Some(2)).count() as i64,
                v.iter().filter(|x| x.0.and_then(|p| p.1) == Some(3)).count() as i64,
            )
        });
    let cand = drain(ap().select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and((&ua).filt(|(p, c, _, _)| p > 0 || c > 0)))).and(&votes_per_post(db)).and(&comments_per_post(db))));
    let v = top_n(cand, |&(_, (((_, (_, (p, c, _, _))), _), _))| (Reverse(p), Reverse(c)), 100);
    rows(v.into_iter().map(|(_, (((p, (u, (n, c, up, dn))), vc), cc))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(c), V::I(up), V::I(dn)];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(vc), V::I(cc)]);
        row(f)
    }))
}

// WITH RECURSIVE UserVoteCounts AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// TagUsage AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%') GROUP BY T.TagName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// RankedPosts AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.CommentCount, PS.UpvoteCount, PS.DownvoteCount,
//        ROW_NUMBER() OVER (ORDER BY PS.Score DESC, PS.CreationDate DESC) AS PostRank FROM PostStatistics PS)
// SELECT U.DisplayName AS UserName, UVC.TotalUpvotes, UVC.TotalDownvotes, TP.TagName, TP.PostCount, TP.TotalViews, RP.Title, RP.CreationDate, RP.Score, RP.CommentCount, RP.UpvoteCount, RP.DownvoteCount
// FROM UserVoteCounts UVC JOIN Users U ON U.Id = UVC.UserId LEFT JOIN TagUsage TP ON TP.PostCount > 10 LEFT JOIN RankedPosts RP ON RP.PostRank <= 5
// WHERE (UVC.TotalUpvotes - UVC.TotalDownvotes) > 0 ORDER BY UVC.TotalUpvotes DESC, UVC.TotalDownvotes ASC;
//
// Not recursive: no CTE refers to itself. PostRank reads only base columns, so the five posts are picked first.
fn q32987(db: &'static So) -> String {
    let Post { score, creation_date, .. } = &db.post;
    let vote_type_id = &db.vote.vote_type_id;
    let uvc = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id).opt())
        .fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let uv = rel(drain((&uvc).filt(|(u, d)| u - d > 0)));
    let tu = left_all(drain(tag_stats(db).filt(|a| a[0] > 10)));
    let top = top_n(drain(db.post.select(Ident::<Post>::new().and(score).and(creation_date))), |&(_, ((_, s), d))| (Reverse(s), Reverse(d)), 5);
    let tp: MatSet<Id<Post>> = rel(top).map(|(_, ((p, _), _))| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let rp = left_all(drain(&ps));
    let mut out = Vec::new();
    (&uv).cross(&tu).cross(&rp).drive(|_, (((u, (up, dn)), t), p)| {
        let mut f = vec![user_col(db, u, "name"), V::I(up), V::I(dn)];
        match t {
            Some((t, a)) => f.extend([tname_of(db, t), V::I(a[0]), nullable(a[2], a[1])]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        match p {
            Some((p, a)) => {
                f.extend(post_fields(db, p, &["title", "created", "score"]));
                f.extend(a.map(V::I));
            }
            None => f.extend((0..6).map(|_| V::Null)),
        }
        out.push(row(f))
    });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserActivities AS (SELECT u.Id AS UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// TagStats AS (SELECT t.Id AS TagId, t.TagName, COUNT(p.Id) AS PostCount FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%') GROUP BY t.Id, t.TagName),
// ClosedPostDetails AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseReasonCount, STRING_AGG(cr.Name, ', ') AS CloseReasons
//     FROM PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment = CAST(cr.Id AS VARCHAR) WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT u.DisplayName AS UserName, u.Reputation, rp.Title AS LatestPostTitle, rp.CreationDate AS LatestPostDate, rp.Score AS LatestPostScore, rp.ViewCount AS LatestPostViews,
//        ua.VoteCount AS UserVoteCount, ua.UpVoteCount, ua.DownVoteCount, ts.TagName, ts.PostCount AS AssociatedPostCount, cp.CloseReasonCount, cp.CloseReasons
// FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.PostId LEFT JOIN UserActivities ua ON u.Id = ua.UserId LEFT JOIN TagStats ts ON ts.PostCount >= 10
// LEFT JOIN ClosedPostDetails cp ON cp.PostId = rp.PostId WHERE rp.PostRank = 1 ORDER BY u.Reputation DESC, rp.CreationDate DESC LIMIT 100;
//
// The join is on a post Id equal to a user Id, and every post of the last year has a larger Id than any user, so the answer is empty
// here; the unordered STRING_AGG and the tag rows tied at the LIMIT would need a rewrite if it were not.
fn q369(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, origid, .. } = &db.post;
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let latest: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, k)| k == 1)
        .map(|((p, _), _)| p)
        .collect();
    let Vote { vote_type_id, .. } = &db.vote;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ts_ = left_all(drain(tag_stats(db).filt(|a| a[0] >= 10)));
    let PostHistory { post_history_type_id, comment, post, .. } = &db.post_history;
    let crn: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i: i64| -> Str { Box::leak(i.to_string().into_boxed_str()) }).inv().select(&db.close_reason_type.name).collect();
    let cp = db.post_history.with(post_history_type_id.in_v(vec![10, 11])).group_by(post).select(comment.select(&crn)).buf_fold(|v| {
        let mut n: Vec<Str> = v.to_vec();
        n.sort();
        (n.len() as i64, Box::leak(n.join(", ").into_boxed_str()) as Str)
    });
    let rows_ = drain((&latest).select(Ident::<Post>::new().and(creation_date).and(origid.select(&uid).select(Ident::<User>::new().and(&db.user.reputation).and(&ua))).and((&cp).opt())));
    let v = cross_top(rows_, |&(_, (((_, d), ((_, r), _)), _))| (Reverse(r), Reverse(d)), drain(&ts_), |_| (), 100);
    rows(v.into_iter().map(|((_, (((p, _), ((u, _), a)), c)), (_, t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend(a.map(V::I));
        match t {
            Some((t, b)) => f.extend([tname_of(db, t), V::I(b[0])]),
            None => f.extend([V::Null, V::Null]),
        }
        match c {
            Some((n, s)) => f.extend([V::I(n), V::S(s)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 0),
// PostAnalysis AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(P.AcceptedAnswerId, -1) AS AcceptedAnswerId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT PL.RelatedPostId) AS RelatedPostsCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN PostLinks PL ON P.Id = PL.PostId
//     WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.AcceptedAnswerId),
// TopPosts AS (SELECT PA.*, R.ReputationRank FROM PostAnalysis PA JOIN UserReputation R ON PA.AcceptedAnswerId IN (SELECT Id FROM Posts WHERE OwnerUserId = R.UserId)
//     ORDER BY PA.Score DESC, R.ReputationRank LIMIT 10)
// SELECT TP.Title, TP.CreationDate, TP.Score, TP.CommentCount, TP.UpVotes, TP.DownVotes, TP.RelatedPostsCount, COALESCE(U.DisplayName, 'Unknown') AS AcceptedAnswerer
// FROM TopPosts TP LEFT JOIN Users U ON TP.AcceptedAnswerId = U.Id
// WHERE TP.CommentCount > 5 AND TP.UpVotes - TP.DownVotes > 0 AND TP.Score >= (SELECT AVG(Score) FROM PostAnalysis)
// UNION ALL
// SELECT 'Average Score' AS Title, NULL AS CreationDate, AVG(Score) AS Score, NULL AS CommentCount, NULL AS UpVotes, NULL AS DownVotes, NULL AS RelatedPostsCount, NULL AS AcceptedAnswerer
// FROM PostAnalysis WHERE Score IS NOT NULL;
//
// The TopPosts order reads only base columns and the rank, so the ten posts are picked first.
fn q3758(db: &'static So) -> String {
    let Post { creation_date, score, accepted_answer, accepted_answer_id, owner_user, .. } = &db.post;
    let reputation = &db.user.reputation;
    let pa = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let (ss, sn) = pa().select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let ur = rel(ranked(drain(db.user.with(reputation.gt(0)).select(Ident::<User>::new().and(reputation))), |&(_, (_, r))| Reverse(r), true));
    let rk: HashIdx<Id<User>, ((Id<User>, (Id<User>, i64)), i64)> = (&ur).map(|((_, (u, _)), _)| u).inv().select(&ur).collect();
    let cand = drain(pa().select(Ident::<Post>::new().and(score).and(accepted_answer.select(owner_user).select((&rk).map(|(_, k)| k)))));
    let top = top_n(cand, |&(_, ((_, s), k))| (Reverse(s), k), 10);
    let tp: MatSet<Id<Post>> = rel(top).map(|(_, ((p, _), _))| p).collect();
    let st = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(links_of(db).select(&db.post_link.related_post_id).opt()))
        .buf_fold(|v| {
            (
                v.iter().filter(|x| x.0 .0.is_some()).count() as i64,
                v.iter().filter(|x| x.0 .1 == Some(2)).count() as i64,
                v.iter().filter(|x| x.0 .1 == Some(3)).count() as i64,
                distinct_some(v.iter().map(|x| x.1)),
            )
        });
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mut out = Vec::new();
    (&st)
        .filt(|(c, u, d, _)| c > 5 && u - d > 0)
        .and(score.filt(move |s| s * sn >= ss))
        .and(accepted_answer_id.select(&uid).select(&db.user.display_name).opt())
        .drive(|p, (((c, u, d, l), s), n)| {
            let mut f = post_fields(db, p, &["title", "created"]);
            f.extend([V::F(s as f64), V::I(c), V::I(u), V::I(d), V::I(l), V::S(n.unwrap_or("Unknown"))]);
            out.push(row(f))
        });
    out.push(row(vec![V::S("Average Score"), V::Null, avg(ss, sn), V::Null, V::Null, V::Null, V::Null, V::Null]));
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS Owner, p.CreationDate, COUNT(a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, RANK() OVER (ORDER BY COUNT(a.Id) DESC) AS RankByAnswers
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, u.DisplayName, p.Title, p.Body, p.Tags, p.CreationDate),
// FilteredPosts AS (SELECT * FROM RankedPosts WHERE RankByAnswers <= 50),
// TopTags AS (SELECT unnest(string_to_array(Tags, '><')) AS Tag FROM FilteredPosts),
// TagRanking AS (SELECT Tag, COUNT(*) AS TagCount FROM TopTags GROUP BY Tag ORDER BY TagCount DESC)
// SELECT fp.PostId, fp.Title, fp.Body, fp.Owner, fp.CreationDate, fp.AnswerCount, fp.UpVoteCount, fp.DownVoteCount, tr.Tag, tr.TagCount
// FROM FilteredPosts fp JOIN TagRanking tr ON tr.Tag = ANY(string_to_array(fp.Tags, '><')) ORDER BY fp.AnswerCount DESC, tr.TagCount DESC;
//
// `= ANY` is a semi-join, so each post meets each distinct element of its own tag array once.
fn q28681(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let rk = whole(&rp).select(Same::new().and(&rp)).window(rank, |(_, a)| a[0], desc);
    let fp = rel(drain((&rk).filt(|(_, r)| r <= 50)).into_iter().map(|(_, (x, _))| x).collect());
    let fpi: HashIdx<Id<Post>, (Id<Post>, [i64; 3])> = (&fp).map(|(p, _)| p).inv().select(&fp).collect();
    let fs: MatSet<Id<Post>> = (&fp).map(|(p, _)| p).collect();
    let tr = (&fs).group_by(tags_str.flat_map(|t: Str| t.split("><"))).fold(0i64, |n, _| n + 1);
    let els = tags_str.flat_map(|t: Str| {
        let mut v: Vec<Str> = t.split("><").collect();
        v.sort();
        v.dedup();
        v
    });
    let mut out = Vec::new();
    (&fpi).and(els.select(Same::<Str>::new().and(&tr))).drive(|_, ((p, a), (t, n))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t), V::I(n)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH RECURSIVE UserReputation AS (SELECT Id AS UserId, Reputation, CreationDate, WebsiteUrl, Location, AboutMe, 1 AS Level FROM Users WHERE Reputation > 1000
//     UNION ALL
//     SELECT u.Id, u.Reputation, u.CreationDate, u.WebsiteUrl, u.Location, u.AboutMe, ur.Level + 1 FROM Users u JOIN UserReputation ur ON u.Reputation < ur.Reputation WHERE ur.Level < 5)
// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags,
//        CASE WHEN EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 6) THEN 'Closed' ELSE 'Open' END AS PostStatus,
//        (SELECT AVG(Reputation) FROM Users WHERE CreationDate < p.CreationDate) AS AvgReputationBeforePostCreation
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN LATERAL (SELECT UNNEST(STRING_TO_ARRAY(p.Tags, ',')) AS TagName) t ON TRUE
// WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'
// GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation ORDER BY p.Score DESC, p.Title ASC LIMIT 50;
//
// The recursive CTE is never read. The order reads only base columns, so the fifty posts are picked first.
fn q31384(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, title, tags_str, .. } = &db.post;
    let vote_type_id = &db.vote.vote_type_id;
    let base = db.post.with(creation_date.ge(date(2023, 10, 1))).with(owner_user);
    let top = top_n(drain(base.select(Ident::<Post>::new().and(score).and(title.opt()))), |&(_, ((_, s), t))| (Reverse(s), t.is_none(), t), 50);
    let tp: MatSet<Id<Post>> = rel(top).map(|(_, ((p, _), _))| p).collect();
    let st = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt()).and(tags_str.flat_map(|t: Str| t.split(',')).opt()))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            t.sort();
            t.dedup();
            (
                v.iter().filter(|x| x.0 .0.is_some()).count() as i64,
                v.iter().filter(|x| x.0 .1 == Some(2)).count() as i64,
                v.iter().filter(|x| x.0 .1 == Some(3)).count() as i64,
                if t.is_empty() { None } else { Some(Box::leak(t.join(", ").into_boxed_str()) as Str) },
            )
        });
    let closed: MatSet<Id<Post>> = (&tp).with(votes_of(db).select(vote_type_id).eq(6)).collect();
    let before = (&tp)
        .group_by(Ident::<Post>::new())
        .select(creation_date.select_where((&db.user.creation_date).inv(), |pc: i64, ud: i64| ud < pc).select(&db.user.reputation))
        .fold((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let mut out = Vec::new();
    (&st).and((&closed).opt()).and((&before).opt()).drive(|p, (((c, u, d, t), k), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep"]);
        f.extend([V::I(c), V::I(u), V::I(d), ostr(t), V::S(if k.is_some() { "Closed" } else { "Open" }), b.map_or(V::Null, |(s, n)| avg(s, n))]);
        out.push(row(f))
    });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        COUNT(v.Id) OVER (PARTITION BY p.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.ViewCount IS NOT NULL),
// RecentPostHistory AS (SELECT ph.PostId, pht.Name AS HistoryType, ph.CreationDate, ph.UserDisplayName, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// FilteredTopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.VoteCount, COALESCE(rp.RankScore, 0) AS RankScore,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount,
//        (SELECT STRING_AGG(tag.TagName, ', ' ORDER BY tag.TagName) FROM Tags tag INNER JOIN Posts p ON p.Tags LIKE CONCAT('%', tag.TagName, '%') WHERE p.Id = rp.PostId) AS TagsUsed
//     FROM RankedPosts rp WHERE rp.RankScore <= 5)
// SELECT ftp.PostId, ftp.Title, ftp.ViewCount, ftp.Score, ftp.VoteCount, ftp.CommentCount, ftp.TagsUsed, ph.HistoryType, ph.UserDisplayName, ph.CreationDate AS HistoryCreationDate
// FROM FilteredTopPosts ftp LEFT JOIN RecentPostHistory ph ON ftp.PostId = ph.PostId WHERE (ph.HistoryRank IS NULL OR ph.HistoryRank = 1)
// ORDER BY ftp.Score DESC, ph.CreationDate DESC LIMIT 10;
//
// Ported from rewrites/21995.sql (the STRING_AGG ordered by name). RankScore numbers post x vote rows, so a post can fill several of the five places.
fn q21995(db: &'static So) -> String {
    let Post { creation_date, view_count, score, post_type_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .with(view_count)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(votes_of(db).opt()))
        .window(row_number, |((_, s), _)| s, desc);
    let picked = rel(drain((&w).filt(|(_, k)| k <= 5)).into_iter().map(|(_, (((p, _), _), _))| p).collect());
    let tm = tag_mentions(db);
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&tm).map(|(p, _)| p).inv().collect();
    let tu = db.post.group_by(Ident::<Post>::new()).select((&by_post).map(|(_, t)| t).select(&db.tag.tag_name)).buf_fold(|v| {
        let mut t: Vec<Str> = v.to_vec();
        t.sort();
        Box::leak(t.join(", ").into_boxed_str()) as Str
    });
    let PostHistory { creation_date: hd, post, .. } = &db.post_history;
    let rh: HashIdx<Id<Post>, Id<PostHistory>> = db
        .post_history
        .with(hd.ge(add_days(t0, -30)))
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(hd))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, k)| k == 1)
        .map(|((h, _), _)| h)
        .collect();
    let hr = (&rh).select(htype_name(db).and((&db.post_history.user_display_name).opt()).and(hd));
    let v = drain((&picked).select(Ident::<Post>::new().and(score).and(&votes_per_post(db)).and(&comments_per_post(db)).and((&tu).opt()).and(hr.opt())));
    let v = top_n(v, |&(_, (((((_, s), _), _), _), h))| (Reverse(s), h.is_some(), Reverse(h.map(|x| x.1))), 10);
    rows(v.into_iter().map(|(_, (((((p, _), nv), nc), t), h))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(nv), V::I(nc), ostr(t)]);
        match h {
            Some(((n, u), d)) => f.extend([V::S(n), ostr(u), V::T(d)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.LastActivityDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        SUM(CASE WHEN v.VoteTypeId IN (6, 10) THEN 1 ELSE 0 END) AS CloseVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.LastActivityDate, u.DisplayName),
// TagStatistics AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(ps.UpVoteCount) AS TotalUpVotes, SUM(ps.DownVoteCount) AS TotalDownVotes, SUM(ps.CloseVotes) AS TotalCloseVotes
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN PostStatistics ps ON p.Id = ps.PostId GROUP BY t.TagName),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(ps.UpVoteCount, 0)) AS TotalUpVotes, SUM(COALESCE(ps.DownVoteCount, 0)) AS TotalDownVotes, COUNT(DISTINCT p.Id) AS TotalPosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostStatistics ps ON p.Id = ps.PostId GROUP BY u.Id, u.DisplayName ORDER BY TotalUpVotes DESC LIMIT 10)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.OwnerDisplayName, ts.TagName, ts.PostCount AS RelatedPostsCount, tu.DisplayName AS TopUser, tu.TotalUpVotes, tu.TotalDownVotes
// FROM PostStatistics ps JOIN TagStatistics ts ON ps.PostId IN (SELECT p.Id FROM Posts p WHERE p.Tags LIKE '%' || ts.TagName || '%')
// JOIN TopUsers tu ON ps.OwnerDisplayName = tu.DisplayName ORDER BY ps.LastActivityDate DESC;
fn q26403(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(date(2023, 1, 1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(6) | Some(10)) as i64]);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&ps).opt()).opt())
        .fold((0i64, 0i64), |(u, d), p| match p.flatten() {
            Some(a) => (u + a[0], d + a[1]),
            None => (u, d),
        });
    let tu = rel(top_n(drain(&us), |&(_, (u, _))| Reverse(u), 10));
    let tun: HashIdx<Str, (Id<User>, (i64, i64))> = (&tu).map(|(u, _)| u).select(&db.user.display_name).inv().select(&tu).collect();
    let tm = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    let mp = || Same::<M>::new().map(|x: M| x.0);
    let ts_ = (&tm).with(mp().select(&ps)).group_by(Same::<M>::new().map(|x: M| x.1)).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&tm)
        .with(mp().select(&ps))
        .select(Same::<M>::new().and(mp().select(owner_user).select(&db.user.display_name).select(&tun)).and(Same::<M>::new().map(|x: M| x.1).select(&ts_)))
        .drive(|_, (((p, t), (u, (up, dn))), n)| {
            let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
            f.extend([tname_of(db, t), V::I(n), user_col(db, u, "name"), V::F(up as f64), V::F(dn as f64)]);
            out.push(row(f))
        });
    rows(out)
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10681", q10681),
    ("12726", q12726),
    ("5626", q5626),
    ("5361", q5361),
    ("6181", q6181),
    ("9482", q9482),
    ("333", q333),
    ("8972", q8972),
    ("25571", q25571),
    ("26702", q26702),
    ("12105", q12105),
    ("10567", q10567),
    ("11603", q11603),
    ("5190", q5190),
    ("28481", q28481),
    ("30248", q30248),
    ("31905", q31905),
    ("26729", q26729),
    ("401", q401),
    ("3250", q3250),
    ("32987", q32987),
    ("369", q369),
    ("3758", q3758),
    ("28681", q28681),
    ("31384", q31384),
    ("21995", q21995),
    ("26403", q26403),
];
