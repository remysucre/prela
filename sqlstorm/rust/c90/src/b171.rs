use harness::prelude::*;
use std::cmp::Reverse;

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, u.DisplayName AS OwnerDisplayName, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.ViewCount > 0 GROUP BY p.Id, p.Title, p.PostTypeId, u.DisplayName, p.CreationDate),
// TagCounts AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON t.ExcerptPostId = p.Id GROUP BY t.TagName),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation IS NOT NULL)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.CommentCount, rp.VoteCount,
//        CASE WHEN rp.PostTypeId = 1 THEN 'Question' WHEN rp.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType, tc.PostCount AS TagPostCount, ur.Reputation, ur.ReputationRank
// FROM RankedPosts rp LEFT JOIN TagCounts tc ON rp.Title LIKE '%' || tc.TagName || '%' LEFT JOIN UserReputation ur ON rp.OwnerDisplayName = ur.DisplayName
// WHERE rp.rn <= 5 AND (ur.Reputation IS NULL OR ur.Reputation > 1000) ORDER BY rp.CreationDate DESC, rp.VoteCount DESC FETCH FIRST 50 ROWS ONLY;
//
// rn reads only base columns, so the five newest posts of each type are picked first and the comment x vote product is driven for those alone.
fn q22357(db: &'static So) -> String {
    let Post { view_count, post_type_id, creation_date, owner_user, title, .. } = &db.post;
    let v = drain(db.post.with(view_count.gt(0)).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(up().opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let tc = db.tag.with(&db.tag.excerpt_post).group_by(&db.tag.tag_name).select(Ident::<Tag>::new()).fold(0i64, |n, _| n + 1);
    let dr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let dr = rel(dr.into_iter().map(|((u, r), k)| (u, (r, k))).collect());
    let ur: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&dr).map(|(u, _)| u).inv().select(&dr).collect();
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let q = (&cc)
        .and(&vc)
        .and(title.select_where(&tc, |t: Str, n: Str| t.contains(n)).opt())
        .and(owner_user.select(&db.user.display_name).select(&by_name).select(&ur).opt())
        .filt(|(_, u): (_, Option<(Id<User>, (i64, i64))>)| u.map_or(true, |(_, (r, _))| r > 1000));
    let v = top_n(drain(q), |&(p, (((_, n), _), u))| (Reverse(creation_date.get(p).unwrap()), Reverse(n), p, u.map(|x| x.0)), 50);
    rows(v.into_iter().map(|(p, (((c, n), t), u))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::I(c), V::I(n)]);
        f.push(V::S(match post_type_id.get(p).unwrap() { 1 => "Question", 2 => "Answer", _ => "Other" }));
        f.push(oint(t));
        f.extend(match u {
            Some((_, (r, k))) => [V::I(r), V::I(k)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStats),
// PostHistoryAnalysis AS (SELECT P.Id AS PostId, P.Title, PH.CreationDate, PH.PostHistoryTypeId, PH.UserId, PH.UserDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY PH.CreationDate DESC) AS HistoryRank
//     FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId WHERE PH.PostHistoryTypeId IN (10, 11, 12, 13))
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalScore, PHA.PostId, PHA.Title, PHA.CreationDate, PHA.PostHistoryTypeId, PHA.UserDisplayName AS ActionBy,
//        CASE WHEN PHA.PostHistoryTypeId = 10 THEN 'Closed' WHEN PHA.PostHistoryTypeId = 11 THEN 'Reopened' WHEN PHA.PostHistoryTypeId = 12 THEN 'Deleted'
//             WHEN PHA.PostHistoryTypeId = 13 THEN 'Undeleted' ELSE 'Other' END AS ActionType
// FROM TopUsers TU LEFT JOIN PostHistoryAnalysis PHA ON TU.UserId = PHA.UserId WHERE TU.ScoreRank <= 10 ORDER BY TU.TotalScore DESC, PHA.CreationDate DESC;
fn q3478(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| Reverse(a[4]), false);
    let tu: MatSet<(Id<User>, [i64; 10])> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect()).map(|x| x).collect();
    let PostHistory { post, post_history_type_id, user, creation_date, user_display_name, .. } = &db.post_history;
    let by_user: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(post_history_type_id.is_in([10, 11, 12, 13])).select(user).inv().collect();
    let v = drain((&tu).select(Same::<(Id<User>, [i64; 10])>::new().map(|(u, _): (Id<User>, [i64; 10])| u).select(&by_user).opt()));
    rows(v.into_iter().map(|((u, a), h)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[4])]);
        f.extend(match h {
            Some(h) => {
                let p = post.get(h).unwrap();
                let t = post_history_type_id.get(h).unwrap();
                vec![
                    post_fields(db, p, &["id"]).remove(0),
                    ostr(db.post.title.get(p)),
                    V::T(creation_date.get(h).unwrap()),
                    V::I(t),
                    ostr(user_display_name.get(h)),
                    V::S(match t { 10 => "Closed", 11 => "Reopened", 12 => "Deleted", 13 => "Undeleted", _ => "Other" }),
                ]
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null, V::S("Other")],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, AVG(COALESCE(p.Score, 0)) AS AvgScore, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY COUNT(p.Id) DESC) AS ActivityRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(ph.HistoryCount, 0) AS HistoryCount
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS HistoryCount FROM PostHistory GROUP BY PostId) ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// TopViewedPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, HistoryCount, RANK() OVER (ORDER BY ViewCount DESC) AS ViewRank FROM PostDetails)
// SELECT ua.DisplayName, ua.PostCount, ua.Upvotes, ua.Downvotes, ua.AvgScore, tp.Title, tp.ViewCount, tp.CommentCount, tp.HistoryCount
// FROM UserActivity ua JOIN TopViewedPosts tp ON ua.UserId = tp.PostId WHERE ua.ActivityRank <= 10 AND tp.ViewRank <= 50 ORDER BY ua.Upvotes DESC, tp.ViewCount DESC;
//
// `ua.UserId = tp.PostId` joins a user id to a post id, so it goes through the raw ids. ActivityRank partitions by the user, so it is 1 for every row;
// the users are picked through the join first and the posts x votes product is driven for those alone.
fn q3540(db: &'static So) -> String {
    let Post { creation_date, view_count, score, origid, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 50).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pu: MatSet<(Id<Post>, Id<User>)> = (&tp).select(Ident::<Post>::new().and(origid.select(&uid))).map(|x| x).collect();
    let us: MatSet<Id<User>> = (&pu).map(|(_, u)| u).collect();
    let ua = (&us)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((s, t)) => [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + s, a[4] + 1],
            None => [a[0], a[1], a[2], a[3], a[4] + 1],
        });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&pu).select(Same::<(Id<Post>, Id<User>)>::new().and(Same::<(Id<Post>, Id<User>)>::new().map(|(_, u)| u).select(&ua))));
    let mut v: Vec<_> = v.into_iter().map(|(_, ((p, u), a))| (p, u, a, cc.get(p).unwrap(), hc.get(p).unwrap())).collect();
    v.sort_by_key(|&(p, _, a, _, _)| {
        let w = view_count.get(p);
        (Reverse(a[1]), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, u, a, c, h)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4])];
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(c), V::I(h)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, MAX(P.CreationDate) AS LastPostDate, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionsCount, AnswersCount, LastPostDate, ReputationRank FROM UserStatistics WHERE Reputation > 1000),
// RecentPostHistory AS (SELECT PH.UserId, PH.PostId, PH.CreationDate, P.Title, P.PostTypeId, ROW_NUMBER() OVER (PARTITION BY PH.UserId ORDER BY PH.CreationDate DESC) AS RecentAction
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionsCount, TU.AnswersCount, TU.LastPostDate, COALESCE(RPH.Title, 'No Recent Activity') AS RecentPostTitle,
//        RPH.CreationDate AS RecentActionDate, CASE WHEN TU.ReputationRank <= 10 THEN 'Top Contributor' WHEN TU.ReputationRank <= 50 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorLevel
// FROM TopUsers TU LEFT JOIN RecentPostHistory RPH ON TU.UserId = RPH.UserId AND RPH.RecentAction = 1 ORDER BY TU.Reputation DESC, TU.DisplayName ASC;
fn q877(db: &'static So) -> String {
    let ups = user_posts(db);
    let rr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let rr = rel(rr.into_iter().map(|((u, _), k)| (u, k)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let PostHistory { post, user, creation_date: hd, .. } = &db.post_history;
    let h = drain(db.post_history.with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(user));
    let h = top_per(h, |&(_, u)| u, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let h = rel(h.into_iter().map(|(h, u)| (u, h)).collect());
    let last: HashIdx<Id<User>, (Id<User>, Id<PostHistory>)> = (&h).map(|(u, _)| u).inv().select(&h).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ups).and(&rank).and((&last).map(|(_, h)| h).opt())));
    rows(v.into_iter().map(|(u, ((a, (_, k)), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), tmax(a[7])]);
        match h {
            Some(h) => {
                f.push(V::S(db.post.title.get(post.get(h).unwrap()).unwrap_or("No Recent Activity")));
                f.push(V::T(hd.get(h).unwrap()));
            }
            None => f.extend([V::S("No Recent Activity"), V::Null]),
        }
        f.push(V::S(if k <= 10 { "Top Contributor" } else if k <= 50 { "Active Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, pt.Name AS PostType, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS TotalComments,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, pt.Name, u.DisplayName, p.Title, p.Body, p.CreationDate),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.PostType, rp.OwnerDisplayName, rp.TotalComments, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.rn = 1),
// PostPopularity AS (SELECT fp.*, (fp.UpVotes - fp.DownVotes) AS PopularityScore FROM FilteredPosts fp)
// SELECT pp.PostId, pp.Title, pp.OwnerDisplayName, pp.CreationDate, pp.PostType, pp.TotalComments, pp.UpVotes, pp.DownVotes, pp.PopularityScore,
//        CASE WHEN pp.PopularityScore > 5 THEN 'High' WHEN pp.PopularityScore BETWEEN 1 AND 5 THEN 'Medium' ELSE 'Low' END AS PopularityCategory
// FROM PostPopularity pp ORDER BY pp.PopularityScore DESC, pp.CreationDate DESC LIMIT 10;
//
// rn partitions by the post, so it is 1 for every row.
fn q25404(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, a)| (Reverse(a[1] - a[2]), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let d = a[1] - a[2];
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "type"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(d), V::S(if d > 5 { "High" } else if (1..=5).contains(&d) { "Medium" } else { "Low" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserDetails AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes v GROUP BY v.PostId),
// PostInteractions AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ud.DisplayName, ud.Reputation, ud.BadgeCount, COALESCE(pvs.Upvotes, 0) AS Upvotes,
//        COALESCE(pvs.Downvotes, 0) AS Downvotes, rp.RankScore
//     FROM RankedPosts rp JOIN UserDetails ud ON rp.OwnerUserId = ud.UserId LEFT JOIN PostVoteSummary pvs ON rp.PostId = pvs.PostId)
// SELECT p.PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.DisplayName, p.Reputation, p.BadgeCount, p.Upvotes, p.Downvotes,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.PostId) AS CommentCount,
//        CASE WHEN p.RankScore = 1 THEN 'Top Post' WHEN p.RankScore < 4 THEN 'Popular Post' ELSE 'Regular Post' END AS PostCategory
// FROM PostInteractions p WHERE p.Reputation > 1000 ORDER BY p.Score DESC, p.ViewCount DESC LIMIT 100;
fn q1120(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).select(owner_user.and(score)));
    let v = per_group(ranked(v, |&(_, (u, s))| (u, Reverse(s)), false), |&(_, (u, _))| u);
    let v = top_n(v, |&((p, (_, s)), _)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 100);
    let tp = rel(v.into_iter().map(|((p, (u, _)), r)| (p, (u, r))).collect());
    let tp: HashIdx<Id<Post>, (Id<Post>, (Id<User>, i64))> = (&tp).map(|(p, _)| p).inv().select(&tp).collect();
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let vs = (&tps).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tps).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain((&vs).and(&cc).and((&tp).map(|(_, x)| x)).and(owner_user.select(&bc)));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    });
    rows(v.into_iter().map(|(p, (((a, c), (u, r)), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(c), V::S(if r == 1 { "Top Post" } else if r < 4 { "Popular Post" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH TagArray AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagStats AS (SELECT t.Tag, COUNT(DISTINCT t.PostId) AS QuestionCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount
//     FROM TagArray t JOIN Posts p ON t.PostId = p.Id GROUP BY t.Tag),
// RelevantUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// BenchmarkData AS (SELECT ts.Tag, ts.QuestionCount, ts.AcceptedAnswerCount, ru.UserId, ru.DisplayName, ru.QuestionCount AS UserQuestionCount, ru.Upvotes, ru.Downvotes
//     FROM TagStats ts JOIN RelevantUsers ru ON true)
// SELECT Tag, QuestionCount, AcceptedAnswerCount, COUNT(DISTINCT UserId) AS ActiveUserCount, AVG(UserQuestionCount) AS AvgUserQuestions, SUM(Upvotes) AS TotalUpvotes,
//        SUM(Downvotes) AS TotalDownvotes
// FROM BenchmarkData GROUP BY Tag, QuestionCount, AcceptedAnswerCount ORDER BY QuestionCount DESC, AcceptedAnswerCount DESC;
//
// `JOIN ... ON true` is a cross join of the tag stats with the users.
fn q25670(db: &'static So) -> String {
    let Post { post_type_id, tags_str, accepted_answer, .. } = &db.post;
    type TP = (Id<Post>, Str);
    let ta = rel(drain(db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list)))).into_iter().map(|x| x.1).collect());
    let ts = (&ta).group_by(Same::<TP>::new().map(|(_, t): TP| t)).select(Same::<TP>::new().map(|(p, _): TP| p).and(Same::<TP>::new().map(|(p, _): TP| p).select(accepted_answer.opt()))).buf_fold(|v| {
        (distinct_some(v.iter().map(|&(p, _)| Some(p))), v.iter().map(|x| x.1.is_some() as i64).sum::<i64>())
    });
    let ru = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .buf_fold(|v| (distinct_some(v.iter().map(|&(p, _)| Some(p))), v.iter().map(|x| (x.1 == Some(2)) as i64).sum::<i64>(), v.iter().map(|x| (x.1 == Some(3)) as i64).sum::<i64>()));
    type B = ((Str, Id<User>), ((i64, i64), (i64, i64, i64)));
    let bd = rel(drain((&ts).cross(&ru)));
    let g = (&bd)
        .group_by(Same::<B>::new().map(|((t, _), (a, _)): B| (t, a)))
        .select(Same::<B>::new().map(|((_, u), (_, r)): B| (u, r)))
        .buf_fold(|v| (distinct_some(v.iter().map(|&(u, _)| Some(u))), v.len() as i64, v.iter().map(|x| x.1 .0).sum::<i64>(), v.iter().map(|x| x.1 .1).sum::<i64>(), v.iter().map(|x| x.1 .2).sum::<i64>()));
    let mut v = drain(&g);
    v.sort_by_key(|&((_, (q, a)), _)| (Reverse(q), Reverse(a)));
    rows(v.into_iter().map(|((t, (q, a)), (n, k, s, u, d))| row(vec![V::S(t), V::I(q), V::I(a), V::I(n), avg(s, k), V::I(u), V::I(d)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ur.Reputation, ur.BadgeCount, pc.CommentCount,
//        CASE WHEN rp.Score > 100 THEN 'High Score' WHEN rp.Score IS NULL THEN 'No Score' ELSE 'Moderate Score' END AS ScoreCategory
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE rp.PostRank = 1)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.Score, fr.ViewCount, fr.Reputation, fr.BadgeCount, COALESCE(fr.CommentCount, 0) AS TotalComments, fr.ScoreCategory
// FROM FinalResults fr ORDER BY fr.Reputation DESC, fr.Score DESC LIMIT 10;
fn q438(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let top = top_n(top, |&(p, u)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&bc).and(&cc).and(owner_user));
    rows(v.into_iter().map(|(p, ((b, c), u))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([user_col(db, u, "rep"), V::I(b), V::I(c), V::S(if s > 100 { "High Score" } else { "Moderate Score" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, CASE WHEN P.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS Status,
//        PH.CreationDate AS LastEditDate, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY PH.CreationDate DESC) AS EditRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN PostHistory PH ON P.Id = PH.PostId),
// TopUsers AS (SELECT UR.DisplayName, UR.Reputation, RANK() OVER (ORDER BY UR.Reputation DESC) AS ReputationRank FROM UserReputation UR WHERE UR.PostCount > 5),
// RecentPosts AS (SELECT PD.PostId, PD.Title, PD.OwnerDisplayName, PD.Status, PD.LastEditDate, ROW_NUMBER() OVER (ORDER BY PD.LastEditDate DESC) AS RecentRank FROM PostDetails PD WHERE PD.EditRank = 1)
// SELECT TU.DisplayName AS TopUser, TU.Reputation, RP.Title AS RecentPostTitle, RP.Status, RP.LastEditDate
// FROM TopUsers TU JOIN RecentPosts RP ON RP.LastEditDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') ORDER BY TU.Reputation DESC, RP.LastEditDate DESC LIMIT 10;
//
// The ON clause names only RP, so the top users and the recently edited posts are crossed. EditRank = 1 is each post's latest history row.
fn q1573(db: &'static So) -> String {
    let ups = user_posts(db);
    let tu: MatSet<Id<User>> = db.user.with((&ups).filt(|a| a[1] > 5)).collect();
    let md = db.post.with(&db.post.owner_user).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.creation_date)).fold(i64::MIN, |m, d| m.max(d));
    let rp = rel(drain((&md).filt(|d| d >= add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let v = drain((&tu).cross(&rp));
    let v = top_n(v, |&((u, _), (_, (p, d)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(d), u, p), 10);
    rows(v.into_iter().map(|((u, _), (_, (p, d)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::S(if db.post.closed_date.get(p).is_some() { "Closed" } else { "Active" }), V::T(d)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId, p.ParentId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// UserBadges AS (SELECT b.UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// CommentsStats AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT rp.Title, rp.Score, rp.CreationDate, u.DisplayName AS Owner, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, COALESCE(pv.UpVotes, 0) AS TotalUpVotes, COALESCE(pv.DownVotes, 0) AS TotalDownVotes, cs.CommentCount, cs.LastCommentDate
// FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON ub.UserId = u.Id LEFT JOIN PostVotes pv ON pv.PostId = rp.Id
// LEFT JOIN CommentsStats cs ON cs.PostId = rp.Id WHERE rp.PostRank = 1 ORDER BY rp.Score DESC LIMIT 10;
fn q553(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let top = top_n(top, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let v = drain((&ub).and(&pv).and(&cs));
    rows(v.into_iter().map(|(p, ((b, u), (n, m)))| {
        let mut f = post_fields(db, p, &["title", "score", "created", "owner"]);
        f.extend(b.map(V::I));
        f.extend(u.map(V::I));
        f.extend(if n == 0 { [V::Null, V::Null] } else { [V::I(n), V::T(m)] });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, p.CreationDate, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.ViewCount, rp.Score, CASE WHEN rp.Rank <= 5 THEN 'Top 5' WHEN rp.Rank <= 10 THEN 'Top 10' ELSE 'Others' END AS RankBracket
//     FROM RankedPosts rp WHERE rp.Rank <= 10),
// VotesAggregated AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// FinalReport AS (SELECT ps.PostId, ps.Title, ps.OwnerDisplayName, ps.ViewCount, ps.Score, ps.RankBracket, va.VoteCount, va.UpVotes, va.DownVotes
//     FROM PostStatistics ps JOIN VotesAggregated va ON ps.PostId = va.PostId)
// SELECT fr.PostId, fr.Title, fr.OwnerDisplayName, fr.ViewCount, fr.Score, fr.RankBracket, fr.VoteCount, fr.UpVotes, fr.DownVotes FROM FinalReport fr ORDER BY fr.Score DESC, fr.ViewCount DESC LIMIT 50;
fn q5939(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let r = ranked(top_n(v, |&(p, u)| (u, Reverse(score.get(p).unwrap()), p), 0), |&(p, u)| (u, Reverse(score.get(p).unwrap()), p), false);
    let r = per_group(r, |&(_, u)| u);
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    };
    let top = top_n(r.into_iter().filter(|x| x.1 <= 10).collect(), |&((p, _), _)| key(p), 50);
    let tp = rel(top.into_iter().map(|((p, _), r)| (p, r)).collect());
    let tp: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tp).map(|(p, _)| p).inv().select(&tp).collect();
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let va = (&tps).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&va).and((&tp).map(|(_, r)| r)));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(owner_user.get(p).map_or(V::S("Community User"), |u| user_col(db, u, "name")));
        f.extend(post_fields(db, p, &["views", "score"]));
        f.push(V::S(if r <= 5 { "Top 5" } else { "Top 10" }));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(P.Score) AS TotalScore,
//        SUM(P.ViewCount) AS TotalViews, COUNT(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswerCount FROM Posts P GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT UB.UserId, UB.DisplayName, COALESCE(PS.QuestionCount, 0) AS QuestionCount, COALESCE(PS.AnswerCount, 0) AS AnswerCount, COALESCE(PS.TotalScore, 0) AS TotalScore,
//        COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.AcceptedAnswerCount, 0) AS AcceptedAnswerCount, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges
//     FROM UserBadges UB LEFT JOIN PostStats PS ON UB.UserId = PS.OwnerUserId),
// RankedStats AS (SELECT *, RANK() OVER (ORDER BY TotalScore DESC, QuestionCount DESC, AnswerCount DESC) AS OverallRank FROM CombinedStats)
// SELECT UserId, DisplayName, TotalScore, QuestionCount, AnswerCount, AcceptedAnswerCount, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, OverallRank
// FROM RankedStats WHERE OverallRank <= 100 ORDER BY OverallRank;
fn q6009(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, score, accepted_answer, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(accepted_answer.opt())).fold([0i64; 4], |a, ((t, s), x)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + x.is_some() as i64]);
    let v = ranked(drain((&ub).and((&ps).opt())), |&(_, (_, p))| {
        let a = p.unwrap_or([0; 4]);
        (Reverse(a[2]), Reverse(a[0]), Reverse(a[1]))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 100).map(|((u, (b, p)), r)| {
        let a = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[2]), V::I(a[0]), V::I(a[1]), V::I(a[3])]);
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RecentActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = u.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT r.PostId, r.Title, r.CreationDate, r.ViewCount, r.Score, r.OwnerDisplayName, r.CommentCount, au.DisplayName AS ActiveUserName, au.PostCount, (au.UpVotes - au.DownVotes) AS NetVotes,
//        CASE WHEN r.Score > 0 THEN 'Popular' WHEN r.Score < 0 THEN 'Controversial' ELSE 'Neutral' END AS PostType,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = r.PostId AND ph.PostHistoryTypeId IN (10, 11, 12) AND ph.CreationDate >= r.CreationDate) AS CloseReopenCount
// FROM RecentActivity r LEFT JOIN ActiveUsers au ON r.OwnerDisplayName = au.DisplayName WHERE r.rn = 1 ORDER BY r.CreationDate DESC, COALESCE(au.PostCount, 0) DESC, r.Score DESC FETCH FIRST 50 ROWS ONLY;
fn q24146(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post_history_type_id, creation_date: hd, post, .. } = &db.post_history;
    let cr = (&tp)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11, 12])).with(hd.and(post.select(creation_date)).filt(|(h, c)| h >= c))).opt())
        .fold(0i64, |n, h| n + h.is_some() as i64);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let au = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let name = owner_user.select(&db.user.display_name).opt().map(|n: Option<Str>| n.unwrap_or("Community User"));
    let v = drain((&cc).and(&cr).and(name.select((&by_name).select(Ident::<User>::new().and(&pc).and(&au)).opt())));
    let v = top_n(v, |&(p, (_, u))| (Reverse(creation_date.get(p).unwrap()), Reverse(u.map_or(0, |((_, n), _)| n)), Reverse(score.get(p).unwrap()), p, u.map(|x| x.0 .0)), 50);
    rows(v.into_iter().map(|(p, ((c, k), u))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(owner_user.get(p).map_or(V::S("Community User"), |u| user_col(db, u, "name")));
        f.push(V::I(c));
        f.extend(match u {
            Some(((u, n), d)) => [user_col(db, u, "name"), V::I(n), V::I(d)],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::S(if s > 0 { "Popular" } else if s < 0 { "Controversial" } else { "Neutral" }), V::I(k)]);
        row(f)
    }))
}

// WITH UserReputationStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS TotalPosts,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId IN (1, 2) THEN P.ViewCount ELSE 0 END) AS TotalViews, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// ActivePostHistory AS (SELECT PH.PostId, PH.PostHistoryTypeId, PH.CreationDate, PH.UserId, PH.UserDisplayName, PH.Comment,
//        ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS ActivityRank
//     FROM PostHistory PH WHERE PH.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// SELECT UPS.DisplayName, UPS.Reputation, UPS.TotalPosts, UPS.TotalQuestions, UPS.TotalAnswers, UPS.TotalViews, PH.UserDisplayName AS RecentModifier,
//        COUNT(DISTINCT PH.PostId) AS RecentPostModifications, MAX(PH.CreationDate) AS LastModificationDate,
//        CASE WHEN UPS.TotalPosts > 0 THEN (UPS.TotalViews / CAST(UPS.TotalPosts AS FLOAT)) ELSE 0 END AS AvgViewsPerPost
// FROM UserReputationStatistics UPS LEFT JOIN ActivePostHistory PH ON UPS.UserId = PH.UserId WHERE UPS.ReputationRank <= 100
// GROUP BY UPS.DisplayName, UPS.Reputation, UPS.TotalPosts, UPS.TotalQuestions, UPS.TotalAnswers, UPS.TotalViews, PH.UserDisplayName ORDER BY UPS.Reputation DESC, RecentPostModifications DESC;
fn q101(db: &'static So) -> String {
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 100).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, view_count, .. } = &db.post;
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, w)) => {
            let x = if t == 1 || t == 2 { w } else { Some(0) };
            [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + x.is_some() as i64, a[4] + x.unwrap_or(0)]
        }
        None => [a[0], a[1], a[2], a[3] + 1, a[4]],
    });
    let PostHistory { user, user_display_name, post, creation_date: hd, .. } = &db.post_history;
    let hist: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(user).inv().collect();
    type J = (Id<User>, Option<Id<PostHistory>>);
    let j: MatSet<J> = (&tu).select(Ident::<User>::new().and((&hist).opt())).collect();
    let h = || Same::<J>::new().map(|(_, h): J| h).flat_map(|h: Option<Id<PostHistory>>| h);
    let g = (&j)
        .group_by(Same::<J>::new().map(|(u, _): J| u).and(h().select(user_display_name).opt()))
        .select(h().select(post.and(hd)).opt())
        .buf_fold(|v| (distinct_some(v.iter().map(|x| x.map(|y| y.0))), v.iter().flat_map(|x| x.map(|y| y.1)).max()));
    type K = (Id<User>, Option<Str>);
    let mut v = drain((&g).and(Same::<K>::new().map(|(u, _): K| u).select(&us)));
    v.sort_by_key(|&((u, _), ((n, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)));
    rows(v.into_iter().map(|((u, m), ((n, d), a))| {
        let tv = if a[3] > 0 { Some(a[4]) } else { None };
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), oint(tv), ostr(m), V::I(n), ots(d)]);
        f.push(if a[0] > 0 { ofloat(tv.map(|t| (t as f32 / a[0] as f32) as f64)) } else { V::F(0.0) });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore, MAX(u.Reputation) AS Reputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// BadgeStats AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// CombinedStats AS (SELECT us.UserId, us.PostCount, us.QuestionCount, us.AnswerCount, us.TotalScore, us.Reputation, COALESCE(bs.BadgeCount, 0) AS BadgeCount,
//        COALESCE(bs.GoldBadges, 0) AS GoldBadges, COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
//     FROM UserStats us LEFT JOIN BadgeStats bs ON us.UserId = bs.UserId),
// FinalStats AS (SELECT UserId, PostCount, QuestionCount, AnswerCount, TotalScore, Reputation, BadgeCount, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM CombinedStats)
// SELECT UserId, PostCount, QuestionCount, AnswerCount, TotalScore, Reputation, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, ScoreRank
// FROM FinalStats WHERE ScoreRank <= 10 ORDER BY TotalScore DESC, Reputation DESC;
fn q7360(db: &'static So) -> String {
    let v = ranked(drain(&user_posts(db)), |&(_, a)| (a[1] == 0, Reverse(a[4])), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| (u, (a, r))).collect());
    let tu: HashIdx<Id<User>, (Id<User>, ([i64; 10], i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let bs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let mut v = drain((&tu).map(|(_, x)| x).and(&bs));
    v.sort_by_key(|&(u, ((a, _), _))| (a[1] == 0, Reverse(a[4]), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(u, ((a, r), b))| {
        let mut f = vec![user_col(db, u, "uid"), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), user_col(db, u, "rep")];
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// UserPosts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(P.Score) AS AvgScore FROM Posts P GROUP BY P.OwnerUserId),
// RankedUsers AS (SELECT U.Id, U.DisplayName, UB.TotalBadges, UP.TotalPosts, UP.TotalQuestions, UP.TotalAnswers, UP.AvgScore,
//        ROW_NUMBER() OVER (ORDER BY UB.TotalBadges DESC, UP.TotalPosts DESC) AS UserRank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN UserPosts UP ON U.Id = UP.OwnerUserId)
// SELECT R.UserRank, R.DisplayName, COALESCE(R.TotalBadges, 0) AS TotalBadges, COALESCE(R.TotalPosts, 0) AS TotalPosts, COALESCE(R.TotalQuestions, 0) AS TotalQuestions,
//        COALESCE(R.TotalAnswers, 0) AS TotalAnswers, COALESCE(R.AvgScore, 0) AS AvgScore,
//        CASE WHEN R.AvgScore IS NULL THEN 'No Posts' WHEN R.AvgScore > 10 THEN 'High Scorer' WHEN R.AvgScore BETWEEN 1 AND 10 THEN 'Average Scorer' ELSE 'Low Scorer' END AS ScoreCategory
// FROM RankedUsers R WHERE R.TotalPosts > 5 ORDER BY R.UserRank OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q2039(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let up = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let v = drain((&bc).and((&up).opt()));
    let v = top_n(v, |&(u, (b, p))| (Reverse(b), p.is_none(), Reverse(p.map(|a| a[0])), u), 0);
    let v = drain(rel(v.into_iter().enumerate().map(|(i, x)| (i as i64 + 1, x)).collect()).filt(|(_, (_, (_, p)))| p.map_or(false, |a| a[0] > 5)));
    rows(v.into_iter().take(10).map(|(_, (r, (u, (b, p))))| {
        let a = p.unwrap();
        let m = a[3] as f64 / a[0] as f64;
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(m)];
        f.push(V::S(if m > 10.0 { "High Scorer" } else if (1.0..=10.0).contains(&m) { "Average Scorer" } else { "Low Scorer" }));
        row(f)
    }))
}

// WITH UserVotes AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// PostMetrics AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Id END) AS CloseCount,
//        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.Id END) AS ReopenCount, AVG(COALESCE(vb.BountyAmount, 0)) AS AverageBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     LEFT JOIN (SELECT Id, UserId, SUM(BountyAmount) AS BountyAmount FROM Votes WHERE VoteTypeId = 9 GROUP BY Id, UserId) vb ON p.Id = vb.Id GROUP BY p.Id, p.OwnerUserId),
// RankedPosts AS (SELECT pm.PostId, pm.OwnerUserId, pm.CommentCount, pm.CloseCount, pm.ReopenCount, pm.AverageBounty,
//        RANK() OVER (PARTITION BY pm.OwnerUserId ORDER BY pm.CommentCount DESC, pm.CloseCount - pm.ReopenCount DESC) AS RankByMetrics FROM PostMetrics pm)
// SELECT u.DisplayName, u.Reputation, p.Title, rp.CommentCount, rp.CloseCount, rp.ReopenCount, rp.AverageBounty, COALESCE(uv.VoteCount, 0) AS TotalVotes,
//        COALESCE(uv.UpVotes, 0) AS UpVoteCount, COALESCE(uv.DownVotes, 0) AS DownVoteCount
// FROM Users u JOIN RankedPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN UserVotes uv ON u.Id = uv.UserId JOIN Posts p ON rp.PostId = p.Id
// WHERE rp.RankByMetrics <= 5 ORDER BY u.Reputation DESC, rp.CommentCount DESC;
//
// `p.Id = vb.Id` joins a post id to a vote id, so it goes through the raw ids.
fn q4658(db: &'static So) -> String {
    let Vote { vote_type_id, origid: vid, bounty_amount, .. } = &db.vote;
    let vb: HashIdx<i64, Id<Vote>> = db.vote.with(vote_type_id.eq(9)).select(vid).inv().collect();
    let Post { owner_user, origid, .. } = &db.post;
    let ht = &db.post_history.post_history_type_id;
    let pm = db
        .post
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).opt()).and(origid.select(&vb).select(bounty_amount.opt()).opt()))
        .fold([0i64; 3], |a, ((c, _), b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0), a[2] + 1]);
    let cr = db.post.with(owner_user).group_by(Ident::<Post>::new()).select(history_of(db).select(ht).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(11)) as i64]);
    let v = drain((&pm).and(&cr).and(owner_user));
    let v = per_group(ranked(v, |&(_, ((a, c), u))| (u, Reverse(a[0]), Reverse(c[0] - c[1])), false), |&(_, (_, u))| u);
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vtype_name(db)).opt()).fold([0i64; 3], |a, n| [a[0] + n.is_some() as i64, a[1] + (n == Some("UpMod")) as i64, a[2] + (n == Some("DownMod")) as i64]);
    type R = ((Id<Post>, (([i64; 3], [i64; 2]), Id<User>)), i64);
    let rp = rel(v);
    let v = drain((&rp).filt(|(_, r): R| r <= 5).select(Same::<R>::new().and(Same::<R>::new().map(|((_, (_, u)), _): R| u).select(&uv))));
    let mut v: Vec<_> = v.into_iter().map(|(_, (((p, ((a, c), u)), _), w))| (p, a, c, u, w)).collect();
    v.sort_by_key(|&(_, a, _, u, _)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])));
    rows(v.into_iter().map(|(p, a, c, u, w)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(c[0]), V::I(c[1]), avg(a[1], a[2]), V::I(w[0]), V::I(w[1]), V::I(w[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// RecentVotes AS (SELECT V.PostId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes V WHERE V.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY V.PostId),
// PostHistoryCounts AS (SELECT PH.PostId, COUNT(PH.Id) AS EditCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5, 6) GROUP BY PH.PostId),
// FinalResults AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, COALESCE(RV.VoteCount, 0) AS TotalVotes, COALESCE(RV.UpVotes, 0) AS UpVotes,
//        COALESCE(RV.DownVotes, 0) AS DownVotes, COALESCE(PHC.EditCount, 0) AS EditCount, RP.PostRank
//     FROM RankedPosts RP LEFT JOIN RecentVotes RV ON RP.PostId = RV.PostId LEFT JOIN PostHistoryCounts PHC ON RP.PostId = PHC.PostId)
// SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, TotalVotes, UpVotes, DownVotes, EditCount FROM FinalResults WHERE PostRank <= 5
// ORDER BY Score DESC, ViewCount DESC LIMIT 10 OFFSET 0;
fn q826(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    };
    let top = top_n(top, |&(p, _)| key(p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(current_date(), -6)))).select(&db.vote.vote_type_id);
    let rv = (&tp).group_by(Ident::<Post>::new()).select(recent.opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let ec = (&tp).group_by(Ident::<Post>::new()).select(edits.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let mut v = drain((&rv).and(&ec));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, (a, e))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(e));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, p.OwnerUserId FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// AggregatedPostData AS (SELECT p.OwnerUserId, COUNT(*) AS TotalQuestions, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, MAX(p.CreationDate) AS LastPostDate
//     FROM Posts p WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId),
// PostHistoryData AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosedDate,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS LastReopenedDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.OwnerDisplayName, r.AnswerCount, a.TotalQuestions, a.TotalScore, a.TotalViews, a.LastPostDate, ph.LastClosedDate, ph.LastReopenedDate,
//        CASE WHEN ph.LastClosedDate IS NOT NULL AND (ph.LastReopenedDate IS NULL OR ph.LastClosedDate > ph.LastReopenedDate) THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RankedPosts r JOIN AggregatedPostData a ON r.OwnerUserId = a.OwnerUserId LEFT JOIN PostHistoryData ph ON r.PostId = ph.PostId WHERE r.Rank = 1
// ORDER BY r.Score DESC, r.ViewCount DESC FETCH FIRST 50 ROWS ONLY;
fn q31438(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    };
    let first = top_n(first, |&(p, _)| key(p), 50);
    let tp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let apd = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(score.and(view_count.opt()).and(creation_date)).fold([0, 0, 0, i64::MIN], |a, ((s, w), d)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3].max(d)]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phd = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold([i64::MIN; 2], |a, (t, d)| [if t == 10 { a[0].max(d) } else { a[0] }, if t == 11 { a[1].max(d) } else { a[1] }]);
    let mut v = drain((&tp).select(owner_user.select(&apd).and((&phd).opt())));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, (a, h))| {
        let (c, r) = h.map_or((None, None), |h| ((h[0] != i64::MIN).then_some(h[0]), (h[1] != i64::MIN).then_some(h[1])));
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "answers"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(a[3]), ots(c), ots(r)]);
        f.push(V::S(if c.is_some() && (r.is_none() || c > r) { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, p.PostTypeId),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 END) AS TotalVotes FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes,
//        pvs.TotalVotes, rp.CommentCount, CASE WHEN rp.RankScore <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostRank
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostVoteSummary pvs ON rp.PostId = pvs.PostId
// WHERE rp.ViewCount > 100 AND rp.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 50;
fn q2872(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let top5: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let key = |p: Id<Post>| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p);
    let v = top_n(drain(db.post.with(view_count.gt(100).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))), |&(p, _)| key(p), 50);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 2 || t == 3) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let mut v = drain((&cc).and(owner_user.select(&ub).opt()).and((&pvs).opt()).and(Ident::<Post>::new().with(&top5).opt()));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, (((c, b), s), t))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        let s3 = s.unwrap_or([0; 3]);
        f.extend([V::I(s3[0]), V::I(s3[1]), s.map_or(V::Null, |a| V::I(a[2])), V::I(c), V::S(if t.is_some() { "Top Post" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(P.Score) AS AvgScore FROM Posts P WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT UBS.UserId, UBS.DisplayName, UBS.BadgeCount, UBS.GoldBadges, UBS.SilverBadges, UBS.BronzeBadges, COALESCE(PS.TotalPosts, 0) AS TotalPosts,
//        COALESCE(PS.Questions, 0) AS Questions, COALESCE(PS.Answers, 0) AS Answers, COALESCE(PS.AvgScore, 0) AS AvgScore FROM UserBadgeStats UBS LEFT JOIN PostStats PS ON UBS.UserId = PS.OwnerUserId),
// RankedStats AS (SELECT *, RANK() OVER (ORDER BY TotalPosts DESC, AvgScore DESC) AS PostRank FROM CombinedStats)
// SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, TotalPosts, Questions, Answers, AvgScore, PostRank
// FROM RankedStats WHERE BadgeCount > 0 AND (TotalPosts > 0 OR Questions > 0) ORDER BY PostRank, BadgeCount DESC;
fn q20895(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, score, creation_date, .. } = &db.post;
    let ps = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let m = |a: [i64; 4]| if a[0] == 0 { 0.0 } else { a[3] as f64 / a[0] as f64 };
    let v = ranked(drain((&ub).and((&ps).opt())), |&(_, (_, p))| {
        let a = p.unwrap_or([0; 4]);
        (Reverse(a[0]), Reverse(fkey(m(a))))
    }, false);
    type R = ((Id<User>, ([i64; 4], Option<[i64; 4]>)), i64);
    let mut v = drain(rel(v).filt(|((_, (b, p)), _): R| {
        let a = p.unwrap_or([0; 4]);
        b[0] > 0 && (a[0] > 0 || a[1] > 0)
    }));
    v.sort_by_key(|&(_, ((_, (b, _)), r))| (r, Reverse(b[0])));
    rows(v.into_iter().map(|(_, ((u, (b, p)), r))| {
        let a = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(m(a)), V::I(r)]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// PostStatistics AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, AVG(COALESCE(v.BountyAmount, 0)) AS AverageBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// TopBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u JOIN Badges b ON u.Id = b.UserId WHERE b.Class = 1 GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerName, ps.CommentCount, ps.VoteCount, ps.AverageBounty, COALESCE(tb.BadgeCount, 0) AS GoldBadgeCount,
//        CASE WHEN rp.Score >= 0 THEN 'Positive' WHEN rp.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory,
//        CASE WHEN ps.VoteCount > 10 THEN 'High Engagement' WHEN ps.VoteCount BETWEEN 5 AND 10 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM RecentPosts rp LEFT JOIN PostStatistics ps ON rp.PostId = ps.PostId LEFT JOIN TopBadges tb ON rp.OwnerName = (SELECT DisplayName FROM Users WHERE Id = tb.UserId)
// WHERE rp.rn <= 5 ORDER BY rp.CreationDate DESC;
//
// The TopBadges join compares the owner's name with each gold-badge holder's name, so it goes through a name index.
fn q31486(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 4], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + v.flatten().unwrap_or(0), a[3] + 1]);
    let tb = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = db.user.with(&tb).select(&db.user.display_name).inv().collect();
    let mut v = drain((&ps).and(owner_user.select(&db.user.display_name).select((&by_name).select(&tb)).opt()));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (a, g))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[3]), V::I(g.unwrap_or(0))]);
        f.push(V::S(if s >= 0 { "Positive" } else { "Negative" }));
        f.push(V::S(if a[1] > 10 { "High Engagement" } else if (5..=10).contains(&a[1]) { "Moderate Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(v.Id) AS TotalVotes, COUNT(DISTINCT p.Id) AS TotalPosts FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// PostHistoryWithReopened AS (SELECT ph.PostId, MIN(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate, MIN(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenedDate,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteUndeleteCount FROM PostHistory ph GROUP BY ph.PostId),
// PostWithTags AS (SELECT p.Id AS PostId, p.Title, ARRAY_AGG(DISTINCT t.TagName) AS TagsArray, p.CreationDate FROM Posts p LEFT JOIN Tags t ON t.ExcerptPostId = p.Id GROUP BY p.Id, p.Title, p.CreationDate)
// SELECT ups.DisplayName, u.Reputation, pt.TagsArray, ups.UpVotes - COALESCE(ups.DownVotes, 0) AS NetVotes, pwh.ClosedDate, pwh.ReopenedDate,
//        CASE WHEN pwh.ReopenedDate IS NOT NULL THEN 'Reopened' WHEN pwh.ClosedDate IS NOT NULL AND pwh.ReopenedDate IS NULL THEN 'Closed' ELSE 'Active' END AS PostStatus, pwh.DeleteUndeleteCount
// FROM UserVoteSummary ups JOIN PostHistoryWithReopened pwh ON pwh.PostId = ups.UserId JOIN PostWithTags pt ON pt.PostId = pwh.PostId JOIN Users u ON u.Id = ups.UserId
// WHERE ups.TotalVotes > 0 AND (pt.TagsArray @> ARRAY['sql'] OR pt.TagsArray @> ARRAY['database']) ORDER BY NetVotes DESC, u.Reputation DESC LIMIT 20;
//
// `pwh.PostId = ups.UserId` joins a post id to a user id, so it goes through the raw ids. TagsArray contains 'sql' exactly when the tag named 'sql' has this post
// as its excerpt, so the posts are those tags' excerpt posts.
fn q21260(db: &'static So) -> String {
    let Tag { tag_name, excerpt_post, .. } = &db.tag;
    let cand: MatSet<Id<Post>> = db.tag.with(tag_name.is_in(["sql", "database"])).select(excerpt_post).collect();
    let tags_of: HashIdx<Id<Post>, Id<Tag>> = excerpt_post.inv().collect();
    let arr = (&cand).group_by(Ident::<Post>::new()).select((&tags_of).select(tag_name)).buf_fold(|v| {
        let mut t: Vec<Str> = v.iter().copied().collect();
        t.sort_unstable();
        t.dedup();
        &*Box::leak(t.into_boxed_slice())
    });
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pwh = (&cand).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold([i64::MAX, i64::MAX, 0], |a, (t, d)| {
        [if t == 10 { a[0].min(d) } else { a[0] }, if t == 11 { a[1].min(d) } else { a[1] }, a[2] + matches!(t, 12 | 13) as i64]
    });
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pu: MatSet<(Id<Post>, Id<User>)> = (&cand).with(&pwh).select(Ident::<Post>::new().and((&db.post.origid).select(&uid))).map(|x| x).collect();
    let us: MatSet<Id<User>> = (&pu).map(|(_, u)| u).collect();
    let uvs = (&us).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]);
    type PU = (Id<Post>, Id<User>);
    let q = (&pu).select(
        Same::<PU>::new()
            .and(Same::<PU>::new().map(|(p, _): PU| p).select((&pwh).and(&arr)))
            .and(Same::<PU>::new().map(|(_, u): PU| u).select((&uvs).filt(|a: [i64; 3]| a[2] > 0))),
    );
    let v = top_n(drain(q), |&(_, (((p, u), _), a))| (Reverse(a[0] - a[1]), Reverse(db.user.reputation.get(u).unwrap()), u, p), 20);
    rows(v.into_iter().map(|(_, (((_, u), (h, t)), a))| {
        let (c, r) = ((h[0] != i64::MAX).then_some(h[0]), (h[1] != i64::MAX).then_some(h[1]));
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::L(t.iter().map(|&s| V::S(s)).collect()), V::I(a[0] - a[1]), ots(c), ots(r)]);
        f.extend([V::S(if r.is_some() { "Reopened" } else if c.is_some() { "Closed" } else { "Active" }), V::I(h[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AcceptedAnswerId, pd.RevisionGUID,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY pd.CreationDate DESC) AS RecentEditRank
//     FROM Posts p LEFT JOIN PostHistory pd ON p.Id = pd.PostId WHERE p.CreationDate >= '2023-01-01'),
// TopQuestions AS (SELECT rp.PostId, rp.Title, rp.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(MAX(v.CreationDate), '1970-01-01') AS LastVoteDate
//     FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId AND v.VoteTypeId = 2 WHERE rp.RecentEditRank = 1 GROUP BY rp.PostId, rp.Title, rp.ViewCount),
// TopBadgedUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT tq.Title AS QuestionTitle, tq.ViewCount, tu.DisplayName AS UserName, tu.BadgeCount AS TotalBadges, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, tq.LastVoteDate
// FROM TopQuestions tq JOIN Posts p ON tq.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id JOIN TopBadgedUsers tu ON u.Id = tu.UserId WHERE tq.ViewCount > 1000
// ORDER BY tq.ViewCount DESC, tq.LastVoteDate DESC LIMIT 10;
//
// RecentEditRank = 1 keeps one row per post whether or not it has history, so TopQuestions has one row per post.
fn q25168(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, .. } = &db.post;
    let tb = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).select(&db.vote.creation_date);
    let tq = db
        .post
        .with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)).and(view_count.gt(1000)))
        .group_by(Ident::<Post>::new())
        .select(up.opt())
        .fold(i64::MIN, |m, d| d.map_or(m, |d| m.max(d)));
    let v = drain((&tq).map(|m| if m == i64::MIN { 0 } else { m }).and(owner_user.select(Ident::<User>::new().and(&tb))));
    let v = top_n(v, |&(p, (d, (u, _)))| (Reverse(view_count.get(p)), Reverse(d), p, u), 10);
    rows(v.into_iter().map(|(p, (d, (u, b)))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.push(user_col(db, u, "name"));
        f.extend(b.map(V::I));
        f.push(V::T(d));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(v.BountyAmount) OVER (PARTITION BY p.Id) AS TotalBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerUserId, rp.CommentCount, rp.TotalBounty FROM RankedPosts rp WHERE rp.rn <= 3),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, u.DisplayName AS OwnerDisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        CASE WHEN tp.TotalBounty IS NULL THEN 'No Bounty' ELSE CONCAT('Total Bounty: $', tp.TotalBounty) END AS BountyInfo
// FROM TopPosts tp JOIN Users u ON tp.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// rn numbers the joined post x comment x vote rows, so a post appears once per joined row among the first three of its type; those rows agree in every projected column.
fn q4167(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let posts = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)));
    let bounty = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8)));
    type J = (Id<Post>, (Option<Id<Comment>>, Option<Id<Vote>>));
    let j = rel(drain(posts().select(comments_of(db).opt().and(bounty().opt()))));
    let v = drain((&j).select(Same::<J>::new().and(Same::<J>::new().map(|(p, _): J| p).select(post_type_id))));
    let top = top_per(v, |&(_, (_, t))| t, |&(i, ((p, _), _))| (Reverse(creation_date.get(p).unwrap()), p, i), 3, false);
    let tp = rel(top.into_iter().map(|(_, ((p, _), _))| p).collect());
    let agg = posts()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty().select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let mut v = drain((&tp).select(Ident::<Post>::new().and(&agg).and(owner_user.select(Ident::<User>::new().and(&ub)))));
    v.sort_by_key(|&(_, ((p, _), _))| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(_, ((p, a), (u, b)))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(a[0]), user_col(db, u, "name")]);
        f.extend(b.map(V::I));
        f.push(if a[1] == 0 { V::S("No Bounty") } else { V::Owned(format!("Total Bounty: ${}", a[2])) });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Tags, U.DisplayName AS Author, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS Rank
//     FROM Posts P LEFT JOIN Comments C ON C.PostId = P.Id LEFT JOIN Votes V ON V.PostId = P.Id AND V.VoteTypeId = 2 JOIN Users U ON U.Id = P.OwnerUserId
//     WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.Tags, U.DisplayName, P.CreationDate, P.PostTypeId),
// TopRankedPosts AS (SELECT PostId, Title, Tags, Author, CommentCount, VoteCount, CreationDate FROM RankedPosts WHERE Rank <= 10),
// DailyTrendingTags AS (SELECT T.TagName, COUNT(P.Id) AS PostCount, CAST(P.CreationDate AS DATE) AS PostDate FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%'
//     WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '7 days' GROUP BY T.TagName, CAST(P.CreationDate AS DATE)),
// TrendingTags AS (SELECT TagName, SUM(PostCount) AS TotalPosts FROM DailyTrendingTags GROUP BY TagName ORDER BY TotalPosts DESC LIMIT 5)
// SELECT TR.PostId, TR.Title, TR.Author, TR.CommentCount, TR.VoteCount, TR.CreationDate, TT.TagName
// FROM TopRankedPosts TR LEFT JOIN TrendingTags TT ON TR.Tags LIKE '%' || TT.TagName || '%' ORDER BY TR.CreationDate DESC;
fn q28356(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, tags_str, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up.opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let lt = tag_mentions(db);
    let week = add_days(current_date(), -7);
    let tt = (&lt).with(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _): (Id<Post>, Id<Tag>)| p).select(creation_date).filt(move |d| d >= week)).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t): (Id<Post>, Id<Tag>)| t).select(&db.tag.tag_name)).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&tt), |&(t, n)| (Reverse(n), t), 5));
    let tti: HashIdx<Str, (Str, i64)> = (&tt).map(|(t, _)| t).inv().select(&tt).collect();
    let mut v = drain((&s).and(tags_str.select_where(&tti, |s: Str, t: Str| s.contains(t)).opt()));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["created"]));
        f.push(t.map_or(V::Null, |(t, _)| V::S(t)));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) GROUP BY U.Id, U.DisplayName, U.Reputation),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation WHERE PostCount > 0),
// HighReputationUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, ReputationRank FROM ActiveUsers WHERE ReputationRank <= 10),
// TopTags AS (SELECT T.TagName, COUNT(P.Id) AS UsageCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName ORDER BY UsageCount DESC LIMIT 5),
// UserTagStats AS (SELECT U.DisplayName, T.TagName, COUNT(P.Id) AS TagUsage FROM HighReputationUsers U JOIN Posts P ON U.UserId = P.OwnerUserId
//     JOIN Tags T ON P.Tags LIKE '%' || T.TagName || '%' WHERE T.TagName IN (SELECT TagName FROM TopTags) GROUP BY U.DisplayName, T.TagName)
// SELECT U.DisplayName, U.Reputation, U.PostCount, T.TagName, COALESCE(SUM(UT.TagUsage), 0) AS TagsUsed
// FROM HighReputationUsers U JOIN TopTags T ON TRUE LEFT JOIN UserTagStats UT ON U.DisplayName = UT.DisplayName AND T.TagName = UT.TagName
// GROUP BY U.DisplayName, U.Reputation, U.PostCount, T.TagName ORDER BY U.Reputation DESC, T.TagName;
//
// `JOIN TopTags T ON TRUE` is a cross join. COUNT(DISTINCT P.Id) over the posts x bounty votes product is the post count.
fn q4934(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain(db.user.with((&pc).filt(|n| n > 0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let hru: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let lt = tag_mentions(db);
    type PT = (Id<Post>, Id<Tag>);
    let usage = (&lt).group_by(Same::<PT>::new().map(|(_, t): PT| t)).select(Same::<PT>::new()).fold(0i64, |n, _| n + 1);
    let top = top_n(drain(&usage), |&(t, n)| (Reverse(n), t), 5);
    let tt: MatSet<Id<Tag>> = rel(top.into_iter().map(|x| x.0).collect()).map(|t| t).collect();
    let tags_of: HashIdx<Id<Post>, Id<Tag>> = (&lt).map(|(p, _)| p).inv().select(Same::<PT>::new().map(|(_, t): PT| t)).collect();
    type UPT = (Id<User>, (Id<Post>, Id<Tag>));
    let upt: MatSet<UPT> = (&hru).select(Ident::<User>::new().and(posts_of(db).select(Ident::<Post>::new().and((&tags_of).select(Ident::<Tag>::new().with(&tt)))))).collect();
    let uts = (&upt)
        .group_by(Same::<UPT>::new().map(|(u, _): UPT| u).select(&db.user.display_name).and(Same::<UPT>::new().map(|(_, (_, t)): UPT| t).select(&db.tag.tag_name)))
        .select(Same::<UPT>::new())
        .fold(0i64, |n, _| n + 1);
    let v = drain((&hru).cross(&tt));
    let v = drain(rel(v).select(Same::<((Id<User>, Id<Tag>), (Id<User>, Id<Tag>))>::new().and(
        Same::<((Id<User>, Id<Tag>), (Id<User>, Id<Tag>))>::new().map(|((u, t), _)| (db.user.display_name.get(u).unwrap(), db.tag.tag_name.get(t).unwrap())).select(&uts).opt(),
    ).and(Same::<((Id<User>, Id<Tag>), (Id<User>, Id<Tag>))>::new().map(|((u, _), _)| u).select(&pc))));
    let mut v: Vec<_> = v.into_iter().map(|(_, ((((u, t), _), n), c))| (u, t, n, c)).collect();
    v.sort_by_key(|&(u, t, _, _)| (Reverse(db.user.reputation.get(u).unwrap()), db.tag.tag_name.get(t).unwrap()));
    rows(v.into_iter().map(|(u, t, n, c)| row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(c), V::S(db.tag.tag_name.get(t).unwrap()), V::I(n.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.Score > 0 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostWithVotes AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 1 THEN 1 ELSE 0 END), 0) AS AcceptedByOriginator FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, us.DisplayName AS UserName, us.BadgeCount, us.UpVoteCount, us.DownVoteCount, pw.TotalUpVotes, pw.TotalDownVotes, pw.AcceptedByOriginator
// FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id LEFT JOIN UserStats us ON p.OwnerUserId = us.UserId LEFT JOIN PostWithVotes pw ON p.Id = pw.PostId
// WHERE rp.rn = 1 AND (us.BadgeCount > 3 OR us.UpVoteCount > 10) ORDER BY rp.ViewCount DESC, rp.Score DESC;
//
// rn reads only base columns, so the newest post of each type is picked first and the badges x votes product is driven for its owner alone.
fn q37(db: &'static So) -> String {
    let Post { score, creation_date, post_type_id, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(score.gt(0).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (b, t)| [a[0] + b.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let pw = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(1)) as i64]);
    let mut v = drain((&pw).and(owner_user.select(Ident::<User>::new().and((&us).filt(|a| a[0] > 3 || a[1] > 10)))));
    v.sort_by_key(|&(p, _)| (Reverse(view_count.get(p)), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (w, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        f.extend(w.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, U.Views, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.Reputation, U.DisplayName, U.Views),
// AcceptedAnswers AS (SELECT P.OwnerUserId, COUNT(P.AcceptedAnswerId) AS AcceptedCount FROM Posts P WHERE P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL GROUP BY P.OwnerUserId),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS Rank FROM Posts P WHERE P.Score > 0),
// UserBadges AS (SELECT B.UserId, COUNT(*) AS BadgeCount FROM Badges B GROUP BY B.UserId)
// SELECT UR.UserId, UR.DisplayName, UR.Reputation, UR.Views, UR.Upvotes, UR.Downvotes, COALESCE(AA.AcceptedCount, 0) AS TotalAcceptedAnswers,
//        COALESCE(PP.Title, 'No Popular Posts') AS PopularPostTitle, COALESCE(UB.BadgeCount, 0) AS TotalBadges,
//        CASE WHEN UR.Reputation >= 1000 THEN 'Veteran' WHEN UR.Reputation >= 500 THEN 'Experienced' ELSE 'Newbie' END AS UserLevel
// FROM UserReputation UR LEFT JOIN AcceptedAnswers AA ON UR.UserId = AA.OwnerUserId LEFT JOIN PopularPosts PP ON UR.UserId = PP.OwnerUserId AND PP.Rank = 1
// LEFT JOIN UserBadges UB ON UR.UserId = UB.UserId ORDER BY UR.Reputation DESC, UR.Views DESC LIMIT 50;
//
// The order reads only Users columns, so the fifty users are picked first and their votes counted for them alone.
fn q3529(db: &'static So) -> String {
    let User { reputation, views, .. } = &db.user;
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), Reverse(views.get(u).unwrap()), u), 50);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ur = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let Post { post_type_id, accepted_answer, owner_user, score, .. } = &db.post;
    let aa = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)).with(accepted_answer)).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pp = top_per(drain(db.post.with(score.gt(0)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let pp = rel(pp.into_iter().map(|(p, u)| (u, p)).collect());
    let pp: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&pp).map(|(u, _)| u).inv().select(&pp).collect();
    let ub = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&ur).and(&aa).and((&pp).map(|(_, p)| p).opt()).and(&ub));
    v.sort_by_key(|&(u, _)| (Reverse(reputation.get(u).unwrap()), Reverse(views.get(u).unwrap()), u));
    rows(v.into_iter().map(|(u, (((a, n), p), b))| {
        let r = reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.push(V::S(p.and_then(|p| db.post.title.get(p)).unwrap_or("No Popular Posts")));
        f.extend([V::I(b), V::S(if r >= 1000 { "Veteran" } else if r >= 500 { "Experienced" } else { "Newbie" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1)),
// UserVoteCount AS (SELECT v.UserId, COUNT(v.Id) AS TotalVotes FROM Votes v GROUP BY v.UserId),
// TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, COALESCE(uv.TotalVotes, 0) AS TotalVotes, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC, COALESCE(uv.TotalVotes, 0) DESC) AS UserRank
//     FROM Users u LEFT JOIN UserVoteCount uv ON u.Id = uv.UserId WHERE u.Reputation > 100),
// LatestPostHistory AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, ph.Comment, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 19))
// SELECT tp.DisplayName, tp.Reputation, tp.TotalVotes, rp.PostId, rp.Title, rp.CreationDate, rp.Score, COALESCE(LPH.Comment, 'No recent history') AS RecentHistoryComment,
//        CASE WHEN LPH.PostHistoryTypeId IS NOT NULL THEN 'Closed/Reopened/Deleted' ELSE 'Active' END AS PostStatus
// FROM TopUsers tp LEFT JOIN RankedPosts rp ON tp.Id = rp.OwnerUserId AND rp.PostRank = 1 LEFT JOIN LatestPostHistory LPH ON rp.PostId = LPH.PostId AND LPH.HistoryRank = 1
// WHERE rp.PostId IS NOT NULL ORDER BY tp.UserRank LIMIT 50;
fn q24773(db: &'static So) -> String {
    let Post { score, post_type_id, owner_user, creation_date, .. } = &db.post;
    let (s, n) = db.post.with(post_type_id.eq(1)).select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let mean = s as f64 / n as f64;
    let rp = top_per(drain(db.post.with(score.filt(move |x| x as f64 > mean)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let rp: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let uv = db.vote.group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select((&uv).opt().and((&rp).map(|(_, p)| p))));
    let v = top_n(v, |&(u, (n, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n.unwrap_or(0)), u), 50);
    let tp = rel(v);
    let PostHistory { post, creation_date: hd, post_history_type_id, comment, .. } = &db.post_history;
    let lh = drain(db.post_history.with(post_history_type_id.is_in([10, 11, 12, 19])).select(post));
    let lh = top_per(lh, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let lh = rel(lh.into_iter().map(|(h, p)| (p, h)).collect());
    let lh: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lh).map(|(p, _)| p).inv().select(&lh).collect();
    type T = (Id<User>, (Option<i64>, Id<Post>));
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(_, (_, p)): T| p).select((&lh).map(|(_, h)| h)).opt())));
    rows(v.into_iter().map(|(_, ((u, (n, p)), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n.unwrap_or(0)));
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.push(V::S(h.and_then(|h| comment.get(h)).unwrap_or("No recent history")));
        f.push(V::S(if h.is_some() { "Closed/Reopened/Deleted" } else { "Active" }));
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(SUM(v.Id), 0) AS TotalVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(ph.RevisionCount, 0) AS RevisionCount,
//        ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS RevisionCount FROM PostHistory GROUP BY PostId) ph ON p.Id = ph.PostId WHERE p.CreationDate > CURRENT_DATE - INTERVAL '30 days'),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, RANK() OVER (ORDER BY ua.TotalVotes DESC) AS UserRank FROM UserActivity ua WHERE ua.TotalVotes > 0)
// SELECT pu.PostId, pu.Title, pu.Score, pu.ViewCount, pu.CommentCount, pu.RevisionCount, tu.DisplayName AS TopUser, tu.Reputation AS UserReputation, pu.PostRank,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pu.PostId AND v.VoteTypeId = 2) AS PostUpVotes, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pu.PostId AND v.VoteTypeId = 3) AS PostDownVotes
// FROM PostMetrics pu JOIN TopUsers tu ON pu.PostRank <= 10 ORDER BY pu.PostRank;
//
// WITH RECURSIVE, but no CTE refers to itself. `ON pu.PostRank <= 10` names only pu, so the ten top posts are crossed with every top user.
// TotalVotes is SUM(v.Id), the sum of the vote ids.
fn q30053(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let v = top_n(drain(db.post.with(creation_date.gt(add_days(current_date(), -30))).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let pm = rel(v.into_iter().enumerate().map(|(i, (p, _))| (p, i as i64 + 1)).collect());
    let tp: MatSet<Id<Post>> = (&pm).map(|(p, _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pm = rel(drain((&pm).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _): (Id<Post>, i64)| p).select((&cc).and(&hc).and(&vc))))));
    let ua = db.vote.group_by(&db.vote.user).select(&db.vote.origid).fold(0i64, |s, i| s + i);
    let tu = rel(drain((&ua).filt(|s| s > 0)));
    let mut v = drain((&pm).cross(&tu));
    v.sort_by_key(|&(_, ((_, ((_, r), _)), _))| r);
    rows(v.into_iter().map(|(_, ((_, ((p, r), ((c, h), w))), (u, _)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(h)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(r), V::I(w[0]), V::I(w[1])]);
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, U.Views, COUNT(DISTINCT P.Id) AS PostCount,
//        SUM(COALESCE(VB.BountyAmount, 0)) AS TotalBountyAwarded
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes VB ON P.Id = VB.PostId AND VB.VoteTypeId IN (8, 9) WHERE U.Reputation > 0
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, U.Views),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, CreationDate, LastAccessDate, Views, PostCount, TotalBountyAwarded, RANK() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserActivity),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(DISTINCT C.Id) AS CommentCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore, MAX(P.CreationDate) AS LastPostDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.CreationDate, TU.LastAccessDate, TU.Views, TU.PostCount, TU.TotalBountyAwarded, COALESCE(PS.CommentCount, 0) AS CommentCount,
//        COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.TotalScore, 0) AS TotalScore, PS.LastPostDate
// FROM TopUsers TU LEFT JOIN PostStatistics PS ON TU.UserId = PS.OwnerUserId WHERE TU.Rank <= 10 ORDER BY TU.Rank;
//
// WITH RECURSIVE, but no CTE refers to itself. Rank reads the reputation and the distinct post count, so the ten users are picked first and the bounty product is driven for them alone.
fn q31057(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(0));
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain(&pc), |&(u, n)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, n), r)| (u, (n, r))).collect());
    let tu: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt());
    let ba = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let Post { creation_date, view_count, score, .. } = &db.post;
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let ps = (&tus)
        .group_by(Ident::<User>::new())
        .select(recent.select(view_count.opt().and(score).and(creation_date).and(comments_of(db).opt())))
        .fold([0, 0, 0, 0, i64::MIN], |a, (((w, s), d), c)| [a[0] + c.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + s, a[3] + 1, a[4].max(d)]);
    let mut v = drain((&tu).map(|(_, x)| x).and(&ba).and((&ps).opt()));
    v.sort_by_key(|&(_, (((_, r), _), _))| r);
    rows(v.into_iter().map(|(u, (((n, _), b), s))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated", "last_access", "uviews"]);
        f.extend([V::I(n), V::I(b)]);
        f.extend(match s {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(a[4])],
            None => [V::I(0), V::I(0), V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty, AVG(COALESCE(P.Score, 0)) AS AverageScore, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalBounty, AverageScore, LastPostDate, RANK() OVER (ORDER BY TotalPosts DESC, TotalBounty DESC) AS UserRank FROM UserActivity),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS AuthorName, P.Score, COUNT(C.Id) AS CommentCount
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate > CURRENT_DATE - INTERVAL '30 days'
//     GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName, P.Score)
// SELECT T.DisplayName, T.TotalPosts, T.TotalQuestions, T.TotalAnswers, T.TotalBounty, T.AverageScore, T.LastPostDate, R.PostId, R.Title AS RecentPostTitle, R.CreationDate AS RecentPostDate,
//        R.Score AS RecentPostScore, R.CommentCount
// FROM TopUsers T LEFT JOIN RecentPosts R ON T.DisplayName = R.AuthorName WHERE T.UserRank <= 10 ORDER BY T.UserRank, R.CreationDate DESC;
fn q9801(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt());
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(creation_date).and(bounty.opt())).opt())
        .fold([0, 0, 0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some((((t, s), d), b)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.flatten().unwrap_or(0), a[4] + s, a[5] + 1, a[6].max(d)],
            None => [a[0], a[1], a[2], a[3], a[4], a[5] + 1, a[6]],
        });
    let v = ranked(drain(&ua), |&(_, a)| (Reverse(a[0]), Reverse(a[3])), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| (u, (a, r))).collect());
    let tu: HashIdx<Id<User>, (Id<User>, ([i64; 7], i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let recent: MatSet<Id<Post>> = db.post.with(creation_date.gt(add_days(current_date(), -30))).with(owner_user).collect();
    let rc = (&recent).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let by_name: HashIdx<Str, Id<Post>> = (&recent).select(owner_user.select(&db.user.display_name)).inv().collect();
    let mut v = drain((&tu).map(|(_, x)| x).and((&db.user.display_name).select((&by_name).select(Ident::<Post>::new().and(&rc))).opt()));
    v.sort_by_key(|&(_, ((_, r), p))| (r, Reverse(p.map(|(p, _)| creation_date.get(p).unwrap()))));
    rows(v.into_iter().map(|(u, ((a, _), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[5]), tmax(a[6])];
        f.extend(match p {
            Some((p, c)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "score"]);
                g.push(V::I(c));
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.UserId, ph.Comment, ph.Text FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)
//     AND ph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '60 days'),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT up.DisplayName, up.Reputation, rp.Title, rp.CreationDate, rp.CommentCount, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(cp.Comment, 'No comments') AS ClosureComment,
//        COALESCE(cp.Text, '') AS ClosureText, (rp.UpVotes - rp.DownVotes) AS NetVotes, CASE WHEN reputationRank <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserTier
// FROM UserReputation up JOIN RecentPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN UserBadges ub ON up.UserId = ub.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// ORDER BY up.Reputation DESC, rp.CreationDate DESC LIMIT 100;
fn q2856(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rr = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let top10: MatSet<Id<User>> = rel(rr.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.gt(add_days(t0, -30)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, comment, text, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]).and(hd.gt(add_days(t0, -60)))));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&rp).and(owner_user.select(Ident::<User>::new().and(&bc).and(Ident::<User>::new().with(&top10).opt()))).and(cp.opt()));
    let v = top_n(v, |&(p, ((_, ((u, _), _)), h))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p, h), 100);
    rows(v.into_iter().map(|(p, ((a, ((u, b), t)), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(b)]);
        f.push(V::S(h.and_then(|h| comment.get(h)).unwrap_or("No comments")));
        f.push(V::S(h.and_then(|h| text.get(h)).unwrap_or("")));
        f.extend([V::I(a[1]), V::S(if t.is_some() { "Top User" } else { "Regular User" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, UPPER(CONCAT(u.DisplayName, ' - ', CAST(u.Reputation AS VARCHAR))) AS UserInfo
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation IS NOT NULL GROUP BY u.Id, u.DisplayName, u.Reputation),
// ClosedPostHistory AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, ur.UserInfo, COALESCE(cph.CloseCount, 0) AS TotalClosed,
//        CASE WHEN rp.CommentCount > 5 THEN 'Highly Commented' WHEN rp.CommentCount BETWEEN 1 AND 5 THEN 'Moderately Commented' ELSE 'No Comments' END AS CommentStatus,
//        CASE WHEN ur.BadgeCount > 0 THEN 'Awarded Badges' ELSE 'No Badges' END AS BadgeStatus
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN ClosedPostHistory cph ON rp.PostId = cph.PostId
// WHERE ur.Reputation BETWEEN 100 AND 500 AND (rp.Score IS NULL OR rp.Score > 10) ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 100;
//
// PostRank is never read. DuckDB's UPPER maps 'ß' to 'ẞ'.
fn q23590(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(10))).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).between(100, 500)))));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cl = (&tp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&cc).and(&cl).and(owner_user.select(Ident::<User>::new().and(&bc))));
    let upper = |s: String| -> String { s.chars().flat_map(|c| if c == 'ß' { vec!['ẞ'] } else { c.to_uppercase().collect() }).collect() };
    rows(v.into_iter().map(|(p, ((c, k), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::Owned(upper(format!("{} - {}", db.user.display_name.get(u).unwrap(), db.user.reputation.get(u).unwrap()))));
        f.push(V::I(k));
        f.push(V::S(if c > 5 { "Highly Commented" } else if c >= 1 { "Moderately Commented" } else { "No Comments" }));
        f.push(V::S(if b > 0 { "Awarded Badges" } else { "No Badges" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.OwnerUserId, rp.PostRank, rp.CommentCount, rp.Upvotes, rp.Downvotes,
//        CASE WHEN rp.Score > 0 THEN 'Positive' WHEN rp.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName
//     FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.PostRank <= 5)
// SELECT fp.PostId, CONCAT('Post Title: ', fp.Title, ' | Score: ', fp.Score, ' | Created: ', fp.CreationDate, ' | Comments: ', fp.CommentCount, ' | Upvotes: ', fp.Upvotes,
//        ' | Downvotes: ', fp.Downvotes, ' | Owner: ', fp.OwnerDisplayName, ' | Score categorization: ', fp.ScoreCategory) AS PostSummary
// FROM FilteredPosts fp WHERE fp.CommentCount > 0 ORDER BY fp.Score DESC, fp.CreationDate DESC LIMIT 10 OFFSET (SELECT COUNT(*) FROM FilteredPosts) / 2;
fn q23784(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let fp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let half = count(&fp) / 2;
    let s = (&fp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain((&s).filt(|a| a[0] > 0)), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), half as usize + 10);
    rows(v.into_iter().skip(half as usize).map(|(p, a)| {
        let sc = score.get(p).unwrap();
        let owner = owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap());
        let cat = if sc > 0 { "Positive" } else if sc < 0 { "Negative" } else { "Neutral" };
        let s = format!(
            "Post Title: {} | Score: {} | Created: {} | Comments: {} | Upvotes: {} | Downvotes: {} | Owner: {} | Score categorization: {}",
            db.post.title.get(p).unwrap_or(""),
            sc,
            ts_text(creation_date.get(p).unwrap()),
            a[0],
            a[1],
            a[2],
            owner,
            cat
        );
        row(vec![post_fields(db, p, &["id"]).remove(0), V::Owned(s)])
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.UserId, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 13)),
// UserReputation AS (SELECT u.Id, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, DENSE_RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT ph.PostId, p.Title AS PostTitle, u.DisplayName AS UserDisplayName, u.Reputation AS UserReputation, COALESCE(b.BadgeCount, 0) AS UserBadgeCount, ph.CreationDate AS HistoryDate,
//        CASE WHEN ph.PostHistoryTypeId = 10 THEN 'Closed' WHEN ph.PostHistoryTypeId = 11 THEN 'Reopened' WHEN ph.PostHistoryTypeId = 12 THEN 'Deleted' ELSE 'Undeleted' END AS Action,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = ph.PostId) AS CommentCount,
//        (SELECT AVG(v.BountyAmount) FROM Votes v WHERE v.PostId = ph.PostId AND v.VoteTypeId IN (9, 10)) AS AverageBounty
// FROM RecursivePostHistory ph JOIN Posts p ON ph.PostId = p.Id JOIN Users u ON ph.UserId = u.Id LEFT JOIN UserReputation b ON u.Id = b.Id
// WHERE ph.rn = 1 AND (SELECT COUNT(*) FROM Comments c WHERE c.PostId = ph.PostId) > 0 AND u.Reputation > 100 ORDER BY ph.CreationDate DESC LIMIT 100;
//
// TopPosts is never referenced.
fn q23400(db: &'static So) -> String {
    let PostHistory { post, post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let v = drain(db.post_history.with(post_history_type_id.is_in([10, 11, 12, 13])).select(post));
    let last = top_per(v, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let last: MatSet<Id<PostHistory>> = rel(last.into_iter().map(|x| x.0).collect()).map(|h| h).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&last).with(user.select(Ident::<User>::new().with((&db.user.reputation).gt(100)))).with(post.select(&cc)));
    let v = top_n(v, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 100);
    let tl: MatSet<Id<PostHistory>> = rel(v.into_iter().map(|x| x.0).collect()).map(|h| h).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([9, 10]))).select(&db.vote.bounty_amount);
    let ab = (&tl).group_by(Ident::<PostHistory>::new()).select(post.select(bounty).opt()).fold([0i64; 2], |a, b| [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&ab).and(post.select(Ident::<Post>::new().and(&cc))).and(user.select(Ident::<User>::new().and(&bc))));
    v.sort_by_key(|&(h, _)| (Reverse(hd.get(h).unwrap()), h));
    rows(v.into_iter().map(|(h, ((a, (p, c)), (u, b)))| {
        let t = post_history_type_id.get(h).unwrap();
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::T(hd.get(h).unwrap())]);
        f.push(V::S(match t { 10 => "Closed", 11 => "Reopened", 12 => "Deleted", _ => "Undeleted" }));
        f.extend([V::I(c), avg(a[1], a[0])]);
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.CreationDate, U.Reputation, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        SUM(COALESCE(C.CommentCount, 0)) AS TotalComments FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON C.PostId = P.Id GROUP BY U.Id, U.DisplayName, U.CreationDate, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalScore, TotalComments, RANK() OVER (ORDER BY Reputation DESC) AS RankByReputation FROM UserActivity),
// RecentActivity AS (SELECT U.UserId, U.DisplayName, U.Reputation, U.PostCount, GREATEST(P.LastActivityDate, COALESCE(C.CreationDate, '1900-01-01')::timestamp) AS MostRecentActivityDate
//     FROM TopUsers U LEFT JOIN Posts P ON U.UserId = P.OwnerUserId LEFT JOIN Comments C ON U.UserId = C.UserId)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.PostCount, CASE WHEN U.PostCount > 0 THEN (SELECT COUNT(*) FROM Votes V WHERE V.UserId = U.UserId AND V.VoteTypeId = 2) ELSE 0 END AS UpVoteCount,
//        A.MostRecentActivityDate, CASE WHEN U.RankByReputation <= 10 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributionLevel
// FROM TopUsers U JOIN RecentActivity A ON U.UserId = A.UserId WHERE A.MostRecentActivityDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
// ORDER BY U.Reputation DESC, U.PostCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. GREATEST skips a NULL argument. RecentActivity is each user's posts x comments product, filtered row by row.
fn q34215(db: &'static So) -> String {
    let cutoff = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let rr = rel(rr.into_iter().map(|((u, _), k)| (u, k)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let up = db.vote.with((&db.vote.vote_type_id).eq(2)).group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let ra = posts_of(db).select(&db.post.last_activity_date).opt().and(comments_by(db).select(&db.comment.creation_date).opt()).map(|(a, c): (Option<i64>, Option<i64>)| {
        let c = c.unwrap_or(ts(1900, 1, 1, 0, 0, 0));
        a.map_or(c, |a| a.max(c))
    });
    let mut v = drain(db.user.select((&pc).and(&rank).and((&up).opt()).and(ra.filt(move |d| d > cutoff))));
    v.sort_by_key(|&(u, (((n, _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)));
    rows(v.into_iter().map(|(u, (((n, (_, k)), w), d))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(if n > 0 { w.unwrap_or(0) } else { 0 }), V::T(d), V::S(if k <= 10 { "Top Contributor" } else { "Contributor" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes, COALESCE(u.Reputation, 0) AS UserReputation
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT rp.PostID, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.PostRank, rp.UpVotes, rp.DownVotes, rp.UserReputation,
//        CASE WHEN rp.UserReputation IS NULL THEN 'Unknown Reputation' WHEN rp.UserReputation > 1000 THEN 'High Reputation' ELSE 'Moderate Reputation' END AS ReputationCategory
//     FROM RankedPosts rp WHERE rp.PostRank = 1 AND rp.ViewCount > 10)
// SELECT fp.PostID, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.UpVotes, fp.DownVotes, fp.ReputationCategory, COALESCE(c.CommentCount, 0) AS TotalComments
// FROM FilteredPosts fp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON fp.PostID = c.PostId WHERE fp.Score > 5 ORDER BY fp.ViewCount DESC, fp.Score DESC;
fn q24714(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let fp = (&tp).with(view_count.gt(10).and(score.gt(5)));
    let s = fp.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).with(view_count.gt(10).and(score.gt(5))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(&cc).and(owner_user.select(&db.user.reputation).opt()));
    rows(v.into_iter().map(|(p, ((a, c), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if r.unwrap_or(0) > 1000 { "High Reputation" } else { "Moderate Reputation" }), V::I(c)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, COALESCE(SUM(V.BountyAmount), 0) AS TotalBountyAmount, RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS PostRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName),
// ClosedPostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS ClosedPosts, COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.ClosedDate IS NOT NULL GROUP BY P.OwnerUserId),
// FinalStats AS (SELECT UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.TotalAnswers, UPS.TotalQuestions, UPS.TotalBountyAmount, COALESCE(CPS.ClosedPosts, 0) AS ClosedPosts,
//        COALESCE(CPS.CloseReopenCount, 0) AS CloseReopenCount FROM UserPostStats UPS LEFT JOIN ClosedPostStats CPS ON UPS.UserId = CPS.OwnerUserId)
// SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalBountyAmount, ClosedPosts, CloseReopenCount,
//        CASE WHEN TotalPosts > 100 THEN 'Veteran' WHEN TotalPosts > 50 THEN 'Experienced' WHEN TotalPosts > 10 THEN 'Novice' ELSE 'Newcomer' END AS UserStatus
// FROM FinalStats WHERE TotalPosts > 0 ORDER BY TotalBountyAmount DESC, TotalPosts DESC;
//
// PostRank is never read.
fn q2284(db: &'static So) -> String {
    let Post { post_type_id, closed_date, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt());
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(bounty.opt()))).fold([0i64; 4], |a, (t, b)| {
        [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + b.flatten().unwrap_or(0)]
    });
    let ht = &db.post_history.post_history_type_id;
    let cps = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(closed_date)).select(history_of(db).select(ht).opt())).fold([0i64; 2], |a, t| {
        [a[0] + 1, a[1] + matches!(t, Some(10 | 11)) as i64]
    });
    let mut v = drain((&ups).and((&cps).opt()));
    v.sort_by_key(|&(u, (a, _))| (Reverse(a[3]), Reverse(a[0]), u));
    rows(v.into_iter().map(|(u, (a, c))| {
        let c = c.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(c[0]), V::I(c[1])]);
        f.push(V::S(if a[0] > 100 { "Veteran" } else if a[0] > 50 { "Experienced" } else if a[0] > 10 { "Novice" } else { "Newcomer" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(CASE WHEN PH.Id IS NOT NULL THEN 1 END) AS EditCount,
//        COUNT(DISTINCT PL.RelatedPostId) AS RelatedLinksCount, P.OwnerUserId
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN PostLinks PL ON P.Id = PL.PostId
//     GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.OwnerUserId ORDER BY P.CreationDate DESC LIMIT 10)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.PostCount, U.BadgeCount, U.UpVotes, U.DownVotes, U.ReputationRank, P.PostId, P.Title, P.CreationDate, P.ViewCount, P.Score,
//        P.CommentCount, P.EditCount, P.RelatedLinksCount
// FROM UserStats U JOIN PostActivity P ON U.UserId = P.OwnerUserId ORDER BY U.Reputation DESC, P.CreationDate DESC;
//
// PostActivity's LIMIT reads only CreationDate, so the ten posts are picked first; UserStats is joined to their owners, so its product is driven for those alone.
// `V.PostId = P.Id AND V.UserId = U.Id` is a vote by a post's owner on that post (`own_votes`).
fn q6235(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = top_n(drain(&db.post.creation_date), |&(p, d)| (Reverse(d), p), 10);
    let pa: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let act = (&pa)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).opt()).and(links_of(db).select(&db.post_link.related_post).opt()))
        .buf_fold(|v| (v.iter().map(|x| x.0 .0.is_some() as i64).sum::<i64>(), v.iter().map(|x| x.0 .1.is_some() as i64).sum::<i64>(), distinct_some(v.iter().map(|x| x.1))));
    let owners: MatSet<Id<User>> = (&pa).select(owner_user).collect();
    let ov = own_votes(db);
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and((&ov).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .buf_fold(|v| {
            (
                distinct_some(v.iter().map(|x| x.0.map(|y| y.0))),
                v.iter().map(|x| x.1.is_some() as i64).sum::<i64>(),
                v.iter().map(|x| (x.0.and_then(|y| y.1) == Some(2)) as i64).sum::<i64>(),
                v.iter().map(|x| (x.0.and_then(|y| y.1) == Some(3)) as i64).sum::<i64>(),
            )
        });
    let rr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let rr = rel(rr.into_iter().map(|((u, _), k)| (u, k)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let mut v = drain((&act).and(owner_user.select(Ident::<User>::new().and(&us).and(&rank))));
    v.sort_by_key(|&(p, (_, ((u, _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((c, e, l), ((u, (n, b, up, dn)), (_, k))))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(b), V::I(up), V::I(dn), V::I(k)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(c), V::I(e), V::I(l)]);
        row(f)
    }))
}

// WITH RecentUserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments, COUNT(DISTINCT B.Id) AS TotalBadges,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON U.Id = V.UserId
//     WHERE U.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalComments, TotalBadges, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM RecentUserActivity WHERE TotalPosts > 10),
// ActiveTags AS (SELECT T.Id AS TagId, T.TagName, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%')
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY T.Id, T.TagName ORDER BY PostCount DESC LIMIT 5)
// SELECT U.DisplayName AS TopUserDisplayName, U.Reputation AS TopUserReputation, U.TotalPosts AS TopUserTotalPosts, U.TotalComments AS TopUserTotalComments, A.TagName AS ActiveTagName,
//        A.PostCount AS ActiveTagPostCount, A.TotalViews AS ActiveTagTotalViews
// FROM TopUsers U CROSS JOIN ActiveTags A WHERE U.ReputationRank <= 10 ORDER BY U.Reputation DESC, A.PostCount DESC;
//
// Only the distinct counts are projected, and each is its table's own count per user, so the vote sums over the product are not computed.
fn q7305(db: &'static So) -> String {
    let users = || db.user.with((&db.user.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = users().group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&pc).filt(|n| n > 10).and(&cc)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let lt = tag_mentions(db);
    type PT = (Id<Post>, Id<Tag>);
    let recent = Same::<PT>::new().map(|(p, _): PT| p).select(Ident::<Post>::new().with((&db.post.creation_date).ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))));
    let at = (&lt)
        .with(recent)
        .group_by(Same::<PT>::new().map(|(_, t): PT| t))
        .select(Same::<PT>::new().map(|(p, _): PT| p).select((&db.post.view_count).opt()))
        .fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let at = rel(top_n(drain(&at), |&(t, a)| (Reverse(a[0]), t), 5));
    let mut v = drain((&tu).cross(&at));
    v.sort_by_key(|&(_, ((u, _), (_, a)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])));
    rows(v.into_iter().map(|(_, ((u, (n, c)), (t, a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(c), V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, p.CreationDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// PostVotes AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS Score, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
//        COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, u.DisplayName, COALESCE(pb.Score, 0) AS PostScore, rp.CommentCount, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, CASE WHEN rp.Rank = 1 THEN 'Latest Post' ELSE 'Older Post' END AS PostStatus
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN PostVotes pb ON rp.PostId = pb.PostId LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE rp.CommentCount > 0 ORDER BY PostScore DESC, rp.CreationDate DESC OFFSET 20 ROWS FETCH NEXT 10 ROWS ONLY;
fn q2653(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let posts = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(owner_user);
    let first = top_per(drain(posts().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = posts().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let ps = posts().with(&cc).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |s, t| s + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&cc).and(&ps).and(owner_user.select(Ident::<User>::new().and(&ub))).and(Ident::<Post>::new().with(&first).opt()));
    let v = top_n(v, |&(p, (((_, s), _), _))| (Reverse(s), Reverse(creation_date.get(p).unwrap()), p), 30);
    rows(v.into_iter().skip(20).map(|(p, (((c, s), (u, b)), f))| {
        let mut r = post_fields(db, p, &["id", "title"]);
        r.extend([user_col(db, u, "name"), V::I(s), V::I(c)]);
        r.extend(b.map(V::I));
        r.push(V::S(if f.is_some() { "Latest Post" } else { "Older Post" }));
        row(r)
    }))
}

// WITH PostActivity AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, COALESCE(P.AnswerCount, 0) AS AnswerCount, COALESCE(P.CommentCount, 0) AS CommentCount,
//        COALESCE(P.FavoriteCount, 0) AS FavoriteCount, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
//        COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount, COUNT(CASE WHEN PH.PostHistoryTypeId IN (1, 2, 4, 5) THEN 1 END) AS EditCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.CreationDate >= '2020-01-01' AND P.Title IS NOT NULL
//     GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, AnswerCount, CommentCount, FavoriteCount, U.DisplayName),
// PostStatistics AS (SELECT PA.PostId, PA.Title, PA.CreationDate, PA.ViewCount, PA.AnswerCount, PA.CommentCount, PA.FavoriteCount, PA.OwnerDisplayName, PA.CloseCount, PA.ReopenCount, PA.EditCount,
//        (PA.ViewCount + PA.FavoriteCount + PA.AnswerCount + PA.CommentCount) AS EngagementScore,
//        ROW_NUMBER() OVER (ORDER BY (PA.ViewCount + PA.FavoriteCount + PA.AnswerCount + PA.CommentCount) DESC) AS Rank FROM PostActivity PA)
// SELECT PS.Rank, PS.PostId, PS.Title, PS.OwnerDisplayName, PS.CreationDate, PS.ViewCount, PS.AnswerCount, PS.CommentCount, PS.FavoriteCount, PS.CloseCount, PS.ReopenCount, PS.EditCount, PS.EngagementScore
// FROM PostStatistics PS WHERE PS.Rank <= 10 ORDER BY PS.EngagementScore DESC;
//
// Rank reads only base columns, so the ten posts are picked first and their history counted for them alone.
fn q29519(db: &'static So) -> String {
    let Post { creation_date, title, view_count, answer_count, comment_count, favorite_count, .. } = &db.post;
    let eng = |p: Id<Post>| view_count.get(p).map(|w| w + favorite_count.get(p).unwrap_or(0) + answer_count.get(p).unwrap_or(0) + comment_count.get(p).unwrap());
    let v = top_n(drain(db.post.with(creation_date.ge(ts(2020, 1, 1, 0, 0, 0))).with(title)), |&(p, _)| {
        let e = eng(p);
        (e.is_none(), Reverse(e), p)
    }, 10);
    let tp = rel(v.into_iter().enumerate().map(|(i, (p, _))| (p, i as i64 + 1)).collect());
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let hc = (&tps).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(11)) as i64, a[2] + matches!(t, Some(1 | 2 | 4 | 5)) as i64]
    });
    let v = drain((&tp).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _): (Id<Post>, i64)| p).select(&hc))));
    rows(v.into_iter().map(|(_, ((p, r), h))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["id", "title", "owner", "created", "views"]));
        f.extend([V::I(answer_count.get(p).unwrap_or(0)), V::I(comment_count.get(p).unwrap()), V::I(favorite_count.get(p).unwrap_or(0))]);
        f.extend(h.map(V::I));
        f.push(oint(eng(p)));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostAnalytics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, A.AnswerCount, C.CommentCount, PH.EditsCount
//     FROM Posts P LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) A ON P.Id = A.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS EditsCount FROM PostHistory GROUP BY PostId) PH ON P.Id = PH.PostId)
// SELECT UR.UserId, UR.DisplayName, UR.Reputation, UR.PostCount, PA.PostId, PA.Title, PA.CreationDate, PA.ViewCount, PA.AnswerCount, PA.CommentCount, PA.EditsCount,
//        RANK() OVER (ORDER BY UR.Reputation DESC) AS ReputationRank
// FROM UserReputation UR JOIN PostAnalytics PA ON UR.UserId = PA.PostId WHERE UR.Reputation > 1000 ORDER BY UR.Reputation DESC, PA.ViewCount DESC;
//
// `UR.UserId = PA.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q5991(db: &'static So) -> String {
    let pid: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let users = || db.user.with((&db.user.reputation).gt(1000)).with((&db.user.origid).select(&pid));
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pa = users().select((&db.user.origid).select(&pid));
    let pas: MatSet<Id<Post>> = pa.map(|p| p).collect();
    let ac = (&pas).group_by(Ident::<Post>::new()).select(answers_of(db)).fold(0i64, |n, _| n + 1);
    let cc = (&pas).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let hc = (&pas).group_by(Ident::<Post>::new()).select(history_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&pc).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and((&ac).opt()).and((&cc).opt()).and((&hc).opt()))));
    let v = ranked(v, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let mut v = v;
    v.sort_by_key(|&((u, (_, (((p, _), _), _))), _)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(db.post.view_count.get(p))));
    rows(v.into_iter().map(|((u, (n, (((p, a), c), h))), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend([oint(a), oint(c), oint(h), V::I(r)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TagsCount AS (SELECT tag.TagName, COUNT(*) AS TotalPosts, SUM(CASE WHEN P.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPostCount
//     FROM Tags tag LEFT JOIN Posts P ON P.Tags LIKE '%' || tag.TagName || '%' GROUP BY tag.TagName),
// TopUsers AS (SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.QuestionCount, UA.AnswerCount, UA.CommentCount, UA.UpvoteCount, UA.DownvoteCount,
//        ROW_NUMBER() OVER (ORDER BY UA.Reputation DESC) AS Rank FROM UserActivity UA WHERE UA.Reputation > 100)
// SELECT TU.DisplayName, TU.Reputation, TU.QuestionCount, TU.AnswerCount, TU.CommentCount, TU.UpvoteCount, TU.DownvoteCount, TC.TagName, TC.TotalPosts, TC.PopularPostCount
// FROM TopUsers TU LEFT JOIN TagsCount TC ON TU.QuestionCount > 0 WHERE TU.Rank <= 10 ORDER BY TU.Reputation DESC, TC.TotalPosts DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x comments x votes product is driven for them alone.
// `ON TU.QuestionCount > 0` names only TU: a user with questions is crossed with every tag, one without keeps a single all-NULL row.
fn q25691(db: &'static So) -> String {
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(100)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, c), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t)| t).inv().map(|(p, _)| p).collect();
    let tc = db.tag.group_by(&db.tag.tag_name).select((&by_tag).select((&db.post.view_count).opt()).opt()).fold([0i64; 2], |a, p| [a[0] + 1, a[1] + (p.flatten().map_or(false, |w| w > 100)) as i64]);
    let with_q = rel(drain((&ua).filt(|a| a[0] > 0)));
    let without_q = rel(drain((&ua).filt(|a| a[0] == 0)));
    let tags = rel(drain(&tc).into_iter().map(Some).collect());
    let none = rel(vec![None::<(Str, [i64; 2])>]);
    let mut v: Vec<(Id<User>, [i64; 5], Option<(Str, [i64; 2])>)> = drain((&with_q).cross(&tags).union((&without_q).cross(&none))).into_iter().map(|(_, ((u, a), t))| (u, a, t)).collect();
    v.sort_by_key(|&(u, _, t)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(t.map(|x| x.1[0]))));
    rows(v.into_iter().map(|(u, a, t)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match t {
            Some((n, c)) => [V::S(n), V::I(c[0]), V::I(c[1])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.Score, 0)) AS TotalScore
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, TotalViews, TotalScore, RANK() OVER (ORDER BY Reputation DESC, TotalScore DESC) AS UserRank FROM UserReputation),
// ActivePosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, COUNT(C.Id) AS CommentCount, MAX(V.CreationDate) AS LastVoteDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') AND P.ViewCount > 100 GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score)
// SELECT U.UserRank, U.DisplayName, U.Reputation, U.BadgeCount, A.PostId, A.Title AS PostTitle, A.CreationDate AS PostCreationDate, A.ViewCount AS PostViewCount, A.Score AS PostScore,
//        A.CommentCount AS PostCommentCount, A.LastVoteDate
// FROM TopUsers U JOIN ActivePosts A ON A.LastVoteDate IS NOT NULL WHERE U.UserId IN (SELECT OwnerUserId FROM Posts WHERE ParentId IS NULL AND PostTypeId = 1)
// ORDER BY U.UserRank, A.Score DESC LIMIT 50;
//
// The ON clause names only A, so the users and the voted-on active posts are crossed.
fn q6894(db: &'static So) -> String {
    let Post { score, creation_date, view_count, parent, post_type_id, owner_user, .. } = &db.post;
    let ur = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(score).opt()))
        .fold([0i64; 2], |a, (b, s)| [a[0] + b.is_some() as i64, a[1] + s.unwrap_or(0)]);
    let v = ranked(drain(&ur), |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1])), false);
    let tu = rel(v.into_iter().map(|((u, a), r)| (u, (a, r))).collect());
    let askers: MatSet<Id<User>> = db.post.with(post_type_id.eq(1)).minus(parent).select(owner_user).collect();
    let tu = rel(drain((&tu).with(Same::<(Id<User>, ([i64; 2], i64))>::new().map(|(u, _): (Id<User>, ([i64; 2], i64))| u).select(Ident::<User>::new().with(&askers)))));
    let ap = db
        .post
        .with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(view_count.gt(100)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, d.map_or(m, |d| m.max(d))));
    let ap = rel(drain((&ap).filt(|(_, m)| m != i64::MIN)));
    let v = drain((&tu).cross(&ap));
    let v = top_n(v, |&(_, ((_, (u, (_, r))), (p, _)))| (r, Reverse(score.get(p).unwrap()), u, p), 50);
    rows(v.into_iter().map(|(_, ((_, (u, (a, r))), (p, (n, m))))| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0])];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(n), V::T(m)]);
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// RecentPosts AS (SELECT P.OwnerUserId, P.Title, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank FROM Posts P
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
// SELECT U.DisplayName, COALESCE(UB.TotalBadges, 0) AS TotalBadges, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.Questions, 0) AS TotalQuestions, COALESCE(PS.Answers, 0) AS TotalAnswers,
//        COALESCE(PS.AverageScore, 0) AS AveragePostScore, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate
// FROM Users U LEFT JOIN UserBadgeStats UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId LEFT JOIN RecentPosts RP ON U.Id = RP.OwnerUserId AND RP.RecentPostRank = 1
// WHERE COALESCE(UB.TotalBadges, 0) > 0 OR COALESCE(PS.TotalPosts, 0) > 0 ORDER BY COALESCE(UB.TotalBadges, 0) DESC, COALESCE(PS.TotalPosts, 0) DESC;
fn q3461(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, post_type_id, score, creation_date, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let rp = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let rp = top_per(rp, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let rp: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let v = drain((&bc).and((&ps).opt()).and((&rp).map(|(_, p)| p).opt()).filt(|((b, p), _): ((i64, Option<[i64; 4]>), Option<Id<Post>>)| b > 0 || p.is_some()));
    rows(v.into_iter().map(|(u, ((b, p), r))| {
        let a = p.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[0] == 0 { V::F(0.0) } else { avg(a[3], a[0]) }];
        f.extend(match r {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.AcceptedAnswerId, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) as Rank,
//        COALESCE(b.Id, 0) AS BadgeId
//     FROM Posts p LEFT JOIN Badges b ON p.OwnerUserId = b.UserId AND b.Class = 1 AND b.Date > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' WHERE p.PostTypeId = 1),
// AnswersWithVotes AS (SELECT a.Id AS AnswerId, a.Score, a.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes FROM Posts a LEFT JOIN Votes v ON a.Id = v.PostId WHERE a.PostTypeId = 2 GROUP BY a.Id, a.Score, a.CreationDate),
// CommentStatistics AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments, MAX(c.CreationDate) AS LatestCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Rank, COALESCE(av.Upvotes, 0) AS TotalUpvotes, COALESCE(av.Downvotes, 0) AS TotalDownvotes, cs.TotalComments, cs.LatestCommentDate,
//        CASE WHEN rp.BadgeId > 0 THEN 'Gold Badge Holder' ELSE 'No Badge' END AS UserBadgeStatus
// FROM RankedPosts rp LEFT JOIN AnswersWithVotes av ON rp.PostId = av.AnswerId LEFT JOIN CommentStatistics cs ON rp.PostId = cs.PostId
// WHERE rp.Rank = 1 AND rp.ViewCount > (SELECT AVG(ViewCount) FROM Posts WHERE PostTypeId = 1) ORDER BY rp.ViewCount DESC LIMIT 10;
fn q21505(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let (s, n) = db.post.with(post_type_id.eq(1)).select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let mean = s as f64 / n as f64;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1).and((&db.badge.date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))));
    let rows_ = drain((&first).with(view_count.filt(move |w| w as f64 > mean)).select(owner_user.select(gold).opt()));
    let v = top_n(rows_, |&(p, b)| (Reverse(view_count.get(p)), p, b), 10);
    let tp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let av = (&tp).with(post_type_id.eq(2)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain(rel(v).select(Same::<(Id<Post>, Option<Id<Badge>>)>::new().and(Same::<(Id<Post>, Option<Id<Badge>>)>::new().map(|(p, _): (Id<Post>, Option<Id<Badge>>)| p).select((&av).opt().and((&cs).opt())))));
    rows(v.into_iter().map(|(_, ((p, b), (a, c)))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(1), V::I(a[0]), V::I(a[1])]);
        f.extend(match c {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if b.is_some() { "Gold Badge Holder" } else { "No Badge" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'),
// UserActivity AS (SELECT u.Id AS UserId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(COUNT(c.Id), 0) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id LEFT JOIN Comments c ON c.UserId = u.Id GROUP BY u.Id),
// ClosedPosts AS (SELECT p.Id, p.Title, ph.UserDisplayName, ph.CreationDate AS ClosedDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10
//     AND ph.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '12 months'),
// TopUsers AS (SELECT ua.UserId, RANK() OVER (ORDER BY ua.UpVotes - ua.DownVotes DESC) AS UserRank FROM UserActivity ua WHERE ua.UpVotes > 0)
// SELECT rp.Id AS RecentPostId, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostCreationDate, ua.UserId AS OwnerUserId, ua.UpVotes AS OwnerUpVotes, ua.DownVotes AS OwnerDownVotes,
//        ua.CommentCount AS OwnerCommentCount, cp.ClosedDate AS PostClosedDate, tu.UserRank
// FROM RecentPosts rp JOIN UserActivity ua ON rp.OwnerUserId = ua.UserId LEFT JOIN ClosedPosts cp ON rp.Id = cp.Id JOIN TopUsers tu ON ua.UserId = tu.UserId
// WHERE rp.rn = 1 AND (ua.UpVotes > 10 OR ua.CommentCount > 5) ORDER BY tu.UserRank, rp.CreationDate DESC LIMIT 50;
//
// `v.UserId = u.Id AND v.PostId = p.Id` is a vote by a post's owner on that post (`own_votes`).
fn q429(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ov = own_votes(db);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&ov).select(&db.vote.vote_type_id).opt()).opt().and(comments_by(db).opt()))
        .fold([0i64; 3], |a, (v, c)| {
            let t = v.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]
        });
    let tr = ranked(drain((&ua).filt(|a| a[0] > 0)), |&(_, a)| Reverse(a[0] - a[1]), false);
    let tr = rel(tr.into_iter().map(|((u, _), r)| (u, r)).collect());
    let tr: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = top_per(drain(db.post.with(creation_date.ge(add_days(t0, -30))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10).and((&db.post_history.creation_date).ge(add_months(t0, -12)))));
    let v = drain((&rp).select(owner_user.select(Ident::<User>::new().and((&ua).filt(|a| a[0] > 10 || a[2] > 5)).and((&tr).map(|(_, r)| r))).and(closes.opt())));
    let v = top_n(v, |&(p, (((_, _), r), h))| (r, Reverse(creation_date.get(p).unwrap()), p, h), 50);
    rows(v.into_iter().map(|(p, (((u, a), r), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(user_col(db, u, "uid"));
        f.extend(a.map(V::I));
        f.extend([h.map_or(V::Null, |h| V::T(db.post_history.creation_date.get(h).unwrap())), V::I(r)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.CommentCount, ps.AnswerCount, ps.Upvotes, ps.Downvotes, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        CASE WHEN ps.UserPostRank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostRanking, CASE WHEN ps.Score IS NULL OR ps.Score < 0 THEN 'Negative' ELSE 'Positive' END AS ScoreStatus
// FROM PostStats ps LEFT JOIN UserBadges ub ON ps.OwnerUserId = ub.UserId WHERE (ps.CommentCount > 0 OR ps.AnswerCount > 0) AND ps.ViewCount >= 100 ORDER BY ps.CreationDate DESC;
//
// UserPostRank ranks every question and the WHERE reads only the distinct counts and ViewCount, so both are taken first and the comments x answers x votes product is driven
// for the questions that pass.
fn q4172(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let top = top_per(drain(qs().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let top5: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = qs().with(view_count.ge(100)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = qs().with(view_count.ge(100)).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let keep: MatSet<Id<Post>> = qs().with((&cc).and(&ac).filt(|(c, a): (i64, i64)| c > 0 || a > 0)).collect();
    let pv = (&keep)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let mut v = drain((&pv).and(&cc).and(&ac).and(owner_user.select(&ub).opt()).and(Ident::<Post>::new().with(&top5).opt()));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, ((((a, c), n), b), t))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if t.is_some() { "Top Post" } else { "Regular Post" }));
        f.push(V::S(if score.get(p).unwrap() < 0 { "Negative" } else { "Positive" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostsCount, COUNT(DISTINCT c.Id) AS CommentsCount,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
//        SUM(p.ViewCount) AS TotalViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     LEFT JOIN Comments c ON u.Id = c.UserId AND c.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     LEFT JOIN Badges b ON u.Id = b.UserId AND b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostsCount, CommentsCount, GoldBadges, SilverBadges, BronzeBadges, TotalViewCount, RANK() OVER (ORDER BY TotalViewCount DESC) AS ViewRank,
//        RANK() OVER (ORDER BY PostsCount DESC) AS PostRank, RANK() OVER (ORDER BY CommentsCount DESC) AS CommentRank, RANK() OVER (ORDER BY (GoldBadges + SilverBadges + BronzeBadges) DESC) AS BadgeRank
//     FROM UserActivity),
// UserScores AS (SELECT UserId, DisplayName, Reputation, (ViewRank + PostRank + CommentRank + BadgeRank) AS TotalRankScore FROM TopUsers)
// SELECT UserId, DisplayName, Reputation, TotalRankScore FROM UserScores WHERE TotalRankScore <= 10 ORDER BY TotalRankScore ASC;
fn q25172(db: &'static So) -> String {
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let rposts = || posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(since)));
    let rcoms = || comments_by(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).ge(since)));
    let bads = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.date).ge(since))).select(&db.badge.class);
    let ua = db.user.group_by(Ident::<User>::new()).select(rposts().select((&db.post.view_count).opt()).opt().and(rcoms().opt()).and(bads.opt())).fold([0i64; 3], |a, ((w, _), b)| {
        let w = w.flatten();
        [a[0] + matches!(b, Some(1 | 2 | 3)) as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]
    });
    let pc = db.user.group_by(Ident::<User>::new()).select(rposts().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(rcoms().opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&ua).and(&pc).and(&cc));
    let v = ranked(v, |&(_, ((a, _), _))| (a[1] == 0, Reverse(a[2])), false);
    let v = ranked(v, |&((_, ((_, p), _)), _)| Reverse(p), false);
    let v = ranked(v, |&(((_, (_, c)), _), _)| Reverse(c), false);
    let v = ranked(v, |&((((_, ((a, _), _)), _), _), _)| Reverse(a[0]), false);
    type R = (((((Id<User>, (([i64; 3], i64), i64)), i64), i64), i64), i64);
    let mut v = drain(rel(v).filt(|((((_, a), b), c), d): R| a + b + c + d <= 10));
    v.sort_by_key(|&(_, ((((_, a), b), c), d))| a + b + c + d);
    rows(v.into_iter().map(|(_, (((((u, _), a), b), c), d))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(a + b + c + d));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostVoteSummary AS (SELECT P.OwnerUserId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// PostCounts AS (SELECT OwnerUserId, COUNT(*) AS PostCount FROM Posts WHERE PostTypeId IN (1, 2) GROUP BY OwnerUserId),
// UserStatistics AS (SELECT RU.UserId, RU.DisplayName, COALESCE(PCS.PostCount, 0) AS TotalPosts, COALESCE(VS.VoteCount, 0) AS TotalVotes, COALESCE(VS.UpVotes, 0) AS UpVotes,
//        COALESCE(VS.DownVotes, 0) AS DownVotes, RU.ReputationRank FROM RankedUsers RU LEFT JOIN PostCounts PCS ON RU.UserId = PCS.OwnerUserId LEFT JOIN PostVoteSummary VS ON RU.UserId = VS.OwnerUserId)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalVotes, U.UpVotes, U.DownVotes, CASE WHEN U.TotalVotes > 0 THEN ROUND((U.UpVotes::decimal / U.TotalVotes) * 100, 2) ELSE NULL END AS UpvotePercentage,
//        CASE WHEN U.ReputationRank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM UserStatistics U JOIN RankedUsers RU ON U.UserId = RU.UserId ORDER BY U.TotalPosts DESC, U.UpVotes DESC;
fn q321(db: &'static So) -> String {
    let rr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let rr = rel(rr.into_iter().map(|((u, _), k)| (u, k)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let Post { owner_user, creation_date, post_type_id, .. } = &db.post;
    let vs = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let pcs = db.post.with(post_type_id.is_in([1, 2])).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.select((&rank).map(|(_, k)| k).and((&pcs).opt()).and((&vs).opt())));
    rows(v.into_iter().map(|(u, ((k, n), s))| {
        let a = s.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(if a[0] > 0 { V::F((a[1] as f64 / a[0] as f64 * 100.0 * 100.0).round() / 100.0) } else { V::Null });
        f.push(V::S(if k <= 10 { "Top Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS QuestionsAsked, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes V ON p.Id = V.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// LatestEdits AS (SELECT ph.PostId, ph.UserId AS EditorId, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS edit_rn
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (24, 10))
// SELECT us.DisplayName, COALESCE(rp.Title, 'N/A') AS LatestQuestionTitle, COALESCE(rp.CreationDate, '1970-01-01') AS LastQuestionDate, us.QuestionsAsked, us.UpVotesReceived, us.DownVotesReceived,
//        MAX(le.CreationDate) AS LastEditDate
// FROM UserStats us LEFT JOIN RankedPosts rp ON us.UserId = rp.Id AND rp.rn = 1 LEFT JOIN LatestEdits le ON us.UserId = le.EditorId AND le.edit_rn = 1
// WHERE us.QuestionsAsked > 0 GROUP BY us.DisplayName, rp.Title, rp.CreationDate, us.QuestionsAsked, us.UpVotesReceived, us.DownVotesReceived
// ORDER BY us.UpVotesReceived DESC, us.QuestionsAsked DESC LIMIT 50;
//
// `us.UserId = rp.Id` joins a user id to a post id, so it goes through the raw ids.
fn q1992(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, origid, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let rp = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let rp = top_per(rp, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp_by_id: HashIdx<i64, Id<Post>> = (&rp).select(origid).inv().collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let le = drain(db.post_history.with(post_history_type_id.is_in([24, 10])).select(post));
    let le = top_per(le, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let le: MatSet<Id<PostHistory>> = rel(le.into_iter().map(|x| x.0).collect()).map(|h| h).collect();
    let by_editor: HashIdx<Id<User>, Id<PostHistory>> = (&le).select(user).inv().collect();
    type K = (Str, Option<Id<Post>>, [i64; 3]);
    let j = rel(drain(
        db.user
            .with(&us)
            .select((&db.user.display_name).and((&db.user.origid).select(&rp_by_id).opt()).and(&us).map(|((n, p), a): ((Str, Option<Id<Post>>), [i64; 3])| (n, p, a)).and((&by_editor).select(hd).opt())),
    ));
    let g = (&j)
        .group_by(Same::<(Id<User>, (K, Option<i64>))>::new().map(|(_, (k, _)): (Id<User>, (K, Option<i64>))| (k.0, k.1.and_then(|p| db.post.title.get(p)), k.1.map(|p| creation_date.get(p).unwrap()), k.2)))
        .select(Same::<(Id<User>, (K, Option<i64>))>::new().map(|(_, (_, d)): (Id<User>, (K, Option<i64>))| d))
        .fold(i64::MIN, |m, d| d.map_or(m, |d| m.max(d)));
    let v = top_n(drain(&g), |&((n, t, d, a), _)| (Reverse(a[1]), Reverse(a[0]), n, t, d), 50);
    rows(v.into_iter().map(|((n, t, d, a), m)| {
        row(vec![V::S(n), V::S(t.unwrap_or("N/A")), V::T(d.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[2]), tmax(m)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.Score, tp.CreationDate, tp.ViewCount, tp.OwnerDisplayName, COALESCE(C.Count, 0) AS CommentCount, COALESCE(V.UpVoteCount, 0) AS UpVoteCount,
//        COALESCE(V.DownVoteCount, 0) AS DownVoteCount FROM TopPosts tp LEFT JOIN (SELECT PostId, COUNT(*) AS Count FROM Comments GROUP BY PostId) C ON tp.PostId = C.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Votes GROUP BY PostId) V
//     ON tp.PostId = V.PostId)
// SELECT pd.Title, pd.OwnerDisplayName, pd.Score, pd.ViewCount, pd.CommentCount, pd.UpVoteCount, pd.DownVoteCount, EXTRACT(YEAR FROM pd.CreationDate) AS PostYear,
//        CASE WHEN pd.Score >= 100 THEN 'High' WHEN pd.Score BETWEEN 50 AND 99 THEN 'Medium' ELSE 'Low' END AS ScoreCategory
// FROM PostDetails pd WHERE pd.ViewCount > 1000 ORDER BY pd.Score DESC, pd.CreationDate DESC;
fn q7552(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pd = (&tp).with(view_count.gt(1000));
    let cv = pd.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).with(view_count.gt(1000)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain((&cv).and(&cc));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (a, c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(year(creation_date.get(p).unwrap()))]);
        f.push(V::S(if s >= 100 { "High" } else if (50..=99).contains(&s) { "Medium" } else { "Low" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC) AS ViewRank FROM Posts p WHERE p.Score > 0),
// RecentPostHistory AS (SELECT ph.PostId, ph.CreationDate, p.OwnerUserId, ph.Comment AS CloseReason, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS RecentEditRank
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (10, 11)),
// CombinedData AS (SELECT ub.UserId, ub.DisplayName, pp.Title AS PopularPostTitle, pp.ViewCount AS PopularPostViews, pp.CreationDate AS PopularPostDate, php.CloseReason, php.RecentEditRank
//     FROM UserBadges ub JOIN Posts pp ON ub.UserId = pp.OwnerUserId LEFT JOIN RecentPostHistory php ON pp.Id = php.PostId WHERE ub.BadgeCount > 0)
// SELECT cd.UserId, cd.DisplayName, cd.PopularPostTitle, cd.PopularPostViews, COALESCE(cd.CloseReason, 'N/A') AS CloseReason, cd.RecentEditRank
// FROM CombinedData cd WHERE cd.RecentEditRank IS NULL OR cd.RecentEditRank = 1 ORDER BY cd.PopularPostViews DESC, cd.DisplayName;
//
// PopularPosts is never referenced. A post with close/reopen history keeps only its latest such row, one without keeps its NULL row.
fn q33196(db: &'static So) -> String {
    let PostHistory { post, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let lh = drain(db.post_history.with(post_history_type_id.is_in([10, 11])).select(post));
    let lh = top_per(lh, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let lh = rel(lh.into_iter().map(|(h, p)| (p, h)).collect());
    let lh: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lh).map(|(p, _)| p).inv().select(&lh).collect();
    let v = drain(db.user.with(badges_of(db)).select(posts_of(db).select(Ident::<Post>::new().and((&lh).map(|(_, h)| h).opt()))));
    rows(v.into_iter().map(|(u, (p, h))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "views"]));
        f.push(V::S(h.and_then(|h| comment.get(h)).unwrap_or("N/A")));
        f.push(if h.is_some() { V::I(1) } else { V::Null });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserReputations AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount FROM PostHistory ph GROUP BY ph.PostId),
// CombinedData AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, ur.Reputation AS UserReputation, ur.TotalBadges,
//        COALESCE(cp.CloseCount, 0) AS CloseCount, COALESCE(cp.ReopenCount, 0) AS ReopenCount
//     FROM RankedPosts rp JOIN Users u ON rp.PostId = u.Id JOIN UserReputations ur ON u.Id = ur.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, UserReputation, TotalBadges, CloseCount, ReopenCount FROM CombinedData
// WHERE CloseCount > 1 AND TotalBadges >= 3 ORDER BY Score DESC, CreationDate DESC LIMIT 10 OFFSET 0;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. rn is never read.
fn q1402(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    let cp = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let v = drain(
        db.post
            .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)))
            .select(origid.select(&uid).select(Ident::<User>::new().and((&tb).filt(|s| s >= 3))).and((&cp).filt(|a| a[0] > 1))),
    );
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, ((u, t), c))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "comments"]);
        f.extend([user_col(db, u, "rep"), V::I(t), V::I(c[0]), V::I(c[1])]);
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT U.Id AS UserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id),
// RecentActivity AS (SELECT U.Id AS UserId, P.Title, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY P.CreationDate DESC) AS RecentPostRank FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId
//     WHERE P.LastActivityDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// BadgeSummary AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldCount, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount FROM Badges B GROUP BY B.UserId)
// SELECT U.DisplayName, COALESCE(UPC.PostCount, 0) AS TotalPosts, COALESCE(UPC.QuestionCount, 0) AS TotalQuestions, COALESCE(UPC.AnswerCount, 0) AS TotalAnswers, COALESCE(Badge.BadgeCount, 0) AS TotalBadges,
//        COALESCE(Badge.GoldCount, 0) AS TotalGoldBadges, COALESCE(Badge.SilverCount, 0) AS TotalSilverBadges, COALESCE(Badge.BronzeCount, 0) AS TotalBronzeBadges, R.Title AS RecentPostTitle,
//        R.CreationDate AS RecentPostDate
// FROM Users U LEFT JOIN UserPostCounts UPC ON U.Id = UPC.UserId LEFT JOIN BadgeSummary Badge ON U.Id = Badge.UserId LEFT JOIN RecentActivity R ON U.Id = R.UserId AND R.RecentPostRank = 1
// WHERE U.Reputation > 1000 ORDER BY U.Reputation DESC LIMIT 10;
fn q1648(db: &'static So) -> String {
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ups = user_posts(db);
    let bs = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { last_activity_date, creation_date, owner_user, .. } = &db.post;
    let ra = drain(db.post.with(last_activity_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let ra = top_per(ra, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let ra = rel(ra.into_iter().map(|(p, u)| (u, p)).collect());
    let ra: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&ra).map(|(u, _)| u).inv().select(&ra).collect();
    let mut v = drain((&bs).and(&ups).and((&ra).map(|(_, p)| p).opt()));
    v.sort_by_key(|&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u));
    rows(v.into_iter().map(|(u, ((b, a), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(b.map(V::I));
        f.extend(match r {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, SUM(b.Class) AS TotalBadgeValue, MAX(u.Reputation) AS MaxReputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// CommentStats AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// PostWithComments AS (SELECT rp.PostId, rp.Title, COALESCE(cs.TotalComments, 0) AS TotalComments, COALESCE(cs.LastCommentDate, '1970-01-01') AS LastCommentDate, rp.CreationDate, rp.Score
//     FROM RankedPosts rp LEFT JOIN CommentStats cs ON rp.PostId = cs.PostId WHERE rp.RecentPostRank = 1)
// SELECT pwc.PostId, pwc.Title, pwc.TotalComments, pwc.LastCommentDate, COALESCE(ur.MaxReputation, 0) AS UserReputation, pwc.Score,
//        CASE WHEN pwc.Score > 0 THEN 'Positive' WHEN pwc.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment, DENSE_RANK() OVER (ORDER BY pwc.Score DESC) AS ScoreRank
// FROM PostWithComments pwc LEFT JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = pwc.PostId) LEFT JOIN UserReputation ur ON u.Id = ur.UserId ORDER BY ScoreRank, TotalComments DESC;
//
// The correlated subquery reads the post's own OwnerUserId.
fn q23219(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let v = drain((&cs).and(owner_user.select(&db.user.reputation).opt()));
    let v = ranked(v, |&(p, _)| Reverse(score.get(p).unwrap()), true);
    let mut v = v;
    v.sort_by_key(|&((_, ((n, _), _)), r)| (r, Reverse(n)));
    rows(v.into_iter().map(|((p, ((n, m), r)), k)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(n), V::T(if n == 0 { 0 } else { m }), V::I(r.unwrap_or(0)), V::I(s)]);
        f.extend([V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }), V::I(k)]);
        row(f)
    }))
}

// WITH RecursiveTagCounts AS (SELECT p.Id AS PostId, COUNT(t.Id) AS TagCount FROM Posts p LEFT JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY p.Id),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation >= 1000 THEN 'High Reputation' WHEN u.Reputation >= 100 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationLevel
//     FROM Users u),
// ActivePosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) AS ActivityRank
//     FROM Posts p WHERE p.LastActivityDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// ClosedPostHistory AS (SELECT ph.PostId, ph.CreationDate AS ClosedDate, ph.Comment, u.DisplayName AS ClosedBy FROM PostHistory ph JOIN Users u ON ph.UserId = u.Id WHERE ph.PostHistoryTypeId = 10)
// SELECT p.Title AS PostTitle, p.ViewCount, t.TagCount, u.DisplayName AS OwnerDisplayName, u.Reputation, ur.ReputationLevel, pp.ClosedDate, pp.ClosedBy
// FROM Posts p JOIN RecursiveTagCounts t ON p.Id = t.PostId JOIN Users u ON p.OwnerUserId = u.Id JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN ClosedPostHistory pp ON p.Id = pp.PostId
// WHERE t.TagCount >= 3 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND (u.Reputation > 500 OR pp.ClosedDate IS NOT NULL)
// ORDER BY p.Score DESC, p.ViewCount DESC LIMIT 50;
//
// ActivePosts is never referenced.
fn q32498(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let lt = tag_mentions(db);
    let tc = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _): (Id<Post>, Id<Tag>)| p)).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, user, creation_date: hd, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).with(user));
    let q = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .select((&tc).filt(|n| n >= 3).and(owner_user.select(Ident::<User>::new().and(&db.user.reputation))).and(closes.opt()))
        .filt(|((_, (_, r)), h): ((i64, (Id<User>, i64)), Option<Id<PostHistory>>)| r > 500 || h.is_some());
    let v = top_n(drain(q), |&(p, (_, h))| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p, h), 50);
    rows(v.into_iter().map(|(p, ((n, (u, r)), h))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::I(n), user_col(db, u, "name"), V::I(r)]);
        f.push(V::S(if r >= 1000 { "High Reputation" } else if r >= 100 { "Medium Reputation" } else { "Low Reputation" }));
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), user_col(db, user.get(h).unwrap(), "name")],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(SUM(b.Class), 0) AS TotalBadges,
//        ROW_NUMBER() OVER (ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS PopularityRank FROM Posts p WHERE p.PostTypeId = 1),
// ClosedPostHistory AS (SELECT p.Id AS PostId, MAX(ph.CreationDate) AS LastClosedDate, COUNT(ph.Id) AS CloseCount FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10 GROUP BY p.Id)
// SELECT us.DisplayName, us.UpVotes, us.DownVotes, us.PostCount, us.CommentCount, us.TotalBadges, pp.PostId, pp.Title AS PopularPostTitle, pp.Score AS PopularPostScore, pp.ViewCount AS PopularPostViews,
//        cph.LastClosedDate, cph.CloseCount
// FROM UserStatistics us INNER JOIN PopularPosts pp ON us.UserId = pp.PostId LEFT JOIN ClosedPostHistory cph ON pp.PostId = cph.PostId WHERE us.Rank <= 10 ORDER BY us.UpVotes DESC, us.DownVotes ASC;
//
// `us.UserId = pp.PostId` joins a user id to a post id, so it goes through the raw ids. The full user x posts x votes x comments x badges product (3.1e8 rows) is driven for the rank.
fn q1153(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let t = p.and_then(|x| x.0);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.unwrap_or(0)]
        });
    let v = top_n(drain(&us), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu = rel(v);
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let pc = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pid: HashIdx<i64, Id<Post>> = db.post.with((&db.post.post_type_id).eq(1)).select(&db.post.origid).inv().collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).select(&db.post_history.creation_date);
    let cph = db.post.group_by(Ident::<Post>::new()).select(closes).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&tus).select((&us).and(&pc).and(&cc).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and((&cph).opt())))));
    v.sort_by_key(|&(_, (((a, _), _), _))| (Reverse(a[0]), a[1]));
    rows(v.into_iter().map(|(u, (((a, n), c), (p, h)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n), V::I(c), V::I(a[2])];
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend(match h {
            Some((n, m)) => [V::T(m), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS rn
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount FROM RankedPosts rp WHERE rp.rn <= 5),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, (SELECT COUNT(*) FROM Posts po WHERE po.OwnerUserId = u.Id) AS PostCount,
//        (SELECT COUNT(*) FROM Comments c WHERE c.UserId = u.Id) AS CommentCount FROM Users u WHERE u.Reputation > 1000),
// PostVoteSummary AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, us.DisplayName AS TopContributor, us.Reputation AS ContributorReputation, pvs.UpVotes, pvs.DownVotes, (pvs.UpVotes - pvs.DownVotes) AS NetVotes,
//        CASE WHEN pvs.TotalVotes IS NULL THEN 'No Votes' ELSE 'Votes Present' END AS VoteStatus
// FROM TopPosts tp JOIN UserStats us ON us.PostCount = (SELECT MAX(PostCount) FROM UserStats) LEFT JOIN PostVoteSummary pvs ON pvs.PostId = tp.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// The ON clause names only us, against an uncorrelated MAX, so the top posts are crossed with the users holding the most posts.
fn q1845(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    };
    let top = top_per(drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| key(p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let most = (&pc).fold_flat(0i64, |m, n| m.max(n));
    let us = rel(drain((&pc).filt(move |n| n == most)));
    let mut v = drain((&pvs).cross(&us));
    v.sort_by_key(|&((p, _), _)| key(p));
    rows(v.into_iter().map(|((p, _), (a, (u, _)))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "answers"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::S("Votes Present")]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStats AS (SELECT OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, AVG(ViewCount) AS AvgViews FROM Posts GROUP BY OwnerUserId),
// ClosedPostStats AS (SELECT ph.UserId, COUNT(*) AS ClosedPostsCount FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId),
// BadgesSummary AS (SELECT UserId, COUNT(*) AS TotalBadges, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId)
// SELECT u.DisplayName, u.Reputation, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.QuestionsCount, 0) AS QuestionsCount, COALESCE(ps.AvgViews, 0) AS AvgViews,
//        COALESCE(cps.ClosedPostsCount, 0) AS ClosedPostsCount, COALESCE(bs.TotalBadges, 0) AS TotalBadges, COALESCE(bs.GoldBadges, 0) AS GoldBadges, COALESCE(bs.SilverBadges, 0) AS SilverBadges,
//        COALESCE(bs.BronzeBadges, 0) AS BronzeBadges, CASE WHEN u.Location IS NULL THEN 'Location Not Provided' ELSE u.Location END AS UserLocation,
//        CASE WHEN u.Views > 1000 THEN 'High Viewer' ELSE 'Regular Viewer' END AS ViewerStatus
// FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN ClosedPostStats cps ON u.Id = cps.UserId LEFT JOIN BadgesSummary bs ON u.Id = bs.UserId
// WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY u.Reputation DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q4143(db: &'static So) -> String {
    let User { reputation, views, location, .. } = &db.user;
    let (s, n) = reputation.fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let mean = s as f64 / n as f64;
    let v = top_n(drain(db.user.with(reputation.filt(move |r| r as f64 > mean)).select(reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, view_count, .. } = &db.post;
    let ps = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt()))).fold([0i64; 4], |a, (t, w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let cps = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let bs = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let mut v = drain((&tu).select((&ps).opt().and((&cps).opt()).and((&bs).opt())));
    v.sort_by_key(|&(u, _)| (Reverse(reputation.get(u).unwrap()), u));
    rows(v.into_iter().map(|(u, ((p, c), b))| {
        let a = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), if a[2] == 0 { V::F(0.0) } else { avg(a[3], a[2]) }, V::I(c.unwrap_or(0))]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.push(V::S(location.get(u).unwrap_or("Location Not Provided")));
        f.push(V::S(if views.get(u).unwrap() > 1000 { "High Viewer" } else { "Regular Viewer" }));
        row(f)
    }))
}

// WITH RecursivePostCTE AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END), 0) AS VoteScore,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// PostHistoryDetails AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN ph.CreationDate END) AS LastModified, COUNT(ph.Id) AS EditCount,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Comment END) AS LastCloseReason FROM PostHistory ph GROUP BY ph.PostId),
// UserStatistics AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS QuestionsAsked, SUM(p.Score) AS TotalScore, SUM(COALESCE(b.Class, 0)) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT p.Id AS PostId, p.Title AS PostTitle, p.CreationDate, u.DisplayName AS OwnerDisplayName, ps.QuestionsAsked, ps.TotalScore AS UserTotalScore, phd.LastModified, phd.EditCount, phd.LastCloseReason,
//        r.VoteScore AS PostVoteScore
// FROM RecursivePostCTE r JOIN Posts p ON r.Id = p.Id JOIN Users u ON p.OwnerUserId = u.Id JOIN UserStatistics ps ON u.Id = ps.UserId LEFT JOIN PostHistoryDetails phd ON p.Id = phd.PostId
// WHERE r.VoteScore > 0 AND ps.QuestionsAsked > 5 ORDER BY p.CreationDate DESC LIMIT 100;
//
// WITH RECURSIVE is not written, and no CTE refers to itself; rn is never read.
fn q30384(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let qa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)))).fold(0i64, |n, _| n + 1);
    let vs = db.post.with(post_type_id.eq(1)).with(owner_user.select((&qa).filt(|n| n > 5))).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |s, t| {
        s + (t == Some(2)) as i64 - (t == Some(3)) as i64
    });
    let v = top_n(drain((&vs).filt(|s| s > 0)), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let tp = rel(v);
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let owners: MatSet<Id<User>> = (&tps).select(owner_user).collect();
    let ts = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(score).and(badges_of(db).opt())).fold(0i64, |s, (x, _)| s + x);
    let PostHistory { post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let phd = (&tps).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd).and(comment.opt()))).fold((i64::MIN, 0i64, None::<Str>), |(m, n, c), ((t, d), x)| {
        (if t == 10 || t == 11 { m.max(d) } else { m }, n + 1, if t == 10 { match (c, x) { (Some(a), Some(b)) => Some(a.max(b)), (a, b) => a.or(b) } } else { c })
    });
    let mut v = drain((&tps).select(Ident::<Post>::new().and(&vs).and(owner_user.select(Ident::<User>::new().and(&qa).and(&ts))).and((&phd).opt())));
    v.sort_by_key(|&(p, _)| (Reverse(creation_date.get(p).unwrap()), p));
    rows(v.into_iter().map(|(_, (((p, s), ((u, q), t)), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(q), V::I(t)]);
        f.extend(match h {
            Some((m, n, c)) => [tmax(m), V::I(n), ostr(c)],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::I(s));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostSummary AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, COALESCE(cp.CloseCount, 0) AS CloseCount, COALESCE(cp.LastClosedDate, '1970-01-01') AS LastClosedDate,
//        (rp.UpvoteCount - rp.DownvoteCount) AS NetVote, CASE WHEN COALESCE(cp.LastClosedDate, '1970-01-01') = '1970-01-01' THEN 'Open' ELSE 'Closed' END AS PostStatus
//     FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT ps.Title, ps.OwnerDisplayName, ps.CreationDate, ps.CommentCount, ps.CloseCount, ps.NetVote, ps.PostStatus FROM PostSummary ps WHERE ps.NetVote > 0 ORDER BY ps.NetVote DESC, ps.CreationDate DESC LIMIT 10;
fn q31673(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = top_n(drain((&rp).filt(|a| a[1] > 0).and((&cp).opt())), |&(p, (a, _))| (Reverse(a[1]), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(c.map_or(0, |c| c.0)), V::I(a[1]), V::S(if c.map_or(true, |c| c.1 == 0) { "Open" } else { "Closed" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.Score, p.CreationDate, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount, COALESCE(SUM(p.ViewCount), 0) AS TotalViews
//     FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.Tags, rp.Score, rp.CreationDate, us.Reputation, us.BadgeCount, us.TotalBountyAmount, us.TotalViews, rp.RankByScore, rp.CommentCount
//     FROM RankedPosts rp JOIN UserStats us ON us.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId))
// SELECT ps.PostId, ps.Title, ps.Tags, ps.Score, ps.CreationDate, ps.Reputation, ps.BadgeCount, ps.TotalBountyAmount, ps.TotalViews, ps.RankByScore,
//        CASE WHEN ps.RankByScore <= 3 THEN 'Top Performer' ELSE 'Regular Contributor' END AS ContributorType
// FROM PostStatistics ps WHERE ps.CommentCount > 5 ORDER BY ps.RankByScore, ps.Score DESC;
//
// The correlated subquery reads the post's own OwnerUserId. UserStats is joined to the owners of the qualifying posts, so its badges x posts x votes product is driven for those alone.
fn q26526(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let rp = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let rp = per_group(ranked(rp, |&(p, u)| (u, Reverse(score.get(p).unwrap())), false), |&(_, u)| u);
    let rp = rel(rp.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rp: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let tp: MatSet<Id<Post>> = (&rp).map(|(p, _)| p).with((&cc).filt(|n| n > 5)).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100)))).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(view_count.opt()).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, ((b, w), v)| [a[0] + b.is_some() as i64, a[1] + v.flatten().unwrap_or(0), a[2] + w.flatten().unwrap_or(0)]);
    let mut v = drain((&tp).select((&rp).map(|(_, r)| r).and(owner_user.select(Ident::<User>::new().and(&us)))));
    v.sort_by_key(|&(p, (r, _))| (r, Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (r, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "score", "created"]);
        f.extend([user_col(db, u, "rep"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r), V::S(if r <= 3 { "Top Performer" } else { "Regular Contributor" })]);
        row(f)
    }))
}

// Rewritten (rewrites/33887.sql): the PostsByUserRank window is refined with `, p.Id, ah.AcceptedAnswerId` and the final order with `, ps.PostId, AnswerStatus`.
// WITH RECURSIVE UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// PostSummary AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(ah.AcceptedAnswerId, -1) AS AcceptedAnswerId, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id, ah.AcceptedAnswerId) AS PostsByUserRank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT OwnerUserId, AcceptedAnswerId FROM Posts WHERE PostTypeId = 1) ah ON p.OwnerUserId = ah.OwnerUserId WHERE p.ViewCount > 100)
// SELECT ua.DisplayName, ua.Reputation, ua.PostsCreated, ua.UpVotesReceived, ua.DownVotesReceived, ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.CommentCount,
//        CASE WHEN ps.AcceptedAnswerId != -1 THEN 'Accepted' ELSE 'Not Accepted' END AS AnswerStatus
// FROM UserActivity ua JOIN PostSummary ps ON ua.UserId = ps.OwnerUserId WHERE ua.Reputation > 1000 AND (ua.PostRank <= 10 OR ps.PostsByUserRank <= 5)
// ORDER BY ua.Reputation DESC, ps.CreationDate DESC, ps.PostId, AnswerStatus LIMIT 50 OFFSET 0;
//
// WITH RECURSIVE, but no CTE refers to itself. PostsByUserRank numbers the post x owner's-questions rows, which are materialised to be ranked.
fn q33887(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, accepted_answer_id, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pr = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let pr = rel(pr.into_iter().map(|((u, _), r)| (u, r)).collect());
    let pr: HashIdx<Id<User>, (Id<User>, i64)> = (&pr).map(|(u, _)| u).inv().select(&pr).collect();
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let asked = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    type J = (Id<Post>, (Id<User>, Option<Id<Post>>));
    let j = rel(drain(db.post.with(view_count.gt(100)).select(owner_user.select(rich.and(asked.opt())))));
    let acc = |q: Option<Id<Post>>| q.and_then(|q| accepted_answer_id.get(q));
    let v = per_group(ranked(drain(&j), |&(_, (p, (u, q)))| (u, Reverse(creation_date.get(p).unwrap()), p, acc(q).is_none(), acc(q)), false), |&(_, (_, (u, _)))| u);
    type R = ((usize, J), i64);
    let rows_ = rel(v);
    let q = (&rows_)
        .select(Same::<R>::new().and(Same::<R>::new().map(|((_, (_, (u, _))), _): R| u).select((&pr).map(|(_, r)| r))))
        .filt(|((_, n), r): (R, i64)| r <= 10 || n <= 5);
    let v = top_n(drain(q), |&(_, (((_, (p, (u, q))), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p, acc(q).is_none()), 50);
    let us: MatSet<Id<User>> = rel(v.iter().map(|x| ((x.1 .0 .0).1).1 .0).collect()).map(|u| u).collect();
    let ua = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(rel(v.into_iter().map(|x| x.1 .0 .0 .1).collect()).select(Same::<J>::new().and(Same::<J>::new().map(|(_, (u, _)): J| u).select((&ua).and(&pc))).and(Same::<J>::new().map(|(p, _): J| p).select((&cc).opt()))));
    rows(v.into_iter().map(|(_, (((p, (u, q)), (a, n)), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend([V::I(c.unwrap_or(0)), V::S(if acc(q).is_some() { "Accepted" } else { "Not Accepted" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.AnswerCount, p.Score, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(p.Id) AS PostCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.Reputation > 1000 AND p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName HAVING COUNT(p.Id) > 5),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate FROM PostHistory ph GROUP BY ph.PostId),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.OwnerUserId, (EXTRACT(EPOCH FROM cast('2024-10-01 12:34:56' as timestamp)) - EXTRACT(EPOCH FROM rp.CreationDate)) / 86400 AS AgeInDays,
//        CASE WHEN phs.EditCount IS NULL THEN 0 ELSE phs.EditCount END AS EditCount, phs.ClosedDate FROM RankedPosts rp LEFT JOIN PostHistorySummary phs ON rp.Id = phs.PostId WHERE rp.UserPostRank = 1)
// SELECT tu.DisplayName, tp.Title, tp.Score, tp.AgeInDays, tp.EditCount, CASE WHEN tp.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM TopPosts tp JOIN TopUsers tu ON tp.OwnerUserId = tu.UserId ORDER BY tp.Score DESC, AgeInDays ASC;
fn q542(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)))).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = (&first).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold((0i64, i64::MIN), |(n, m), (t, d)| (n + 1, if t == 10 { m.max(d) } else { m }));
    let t0 = secs(ts(2024, 10, 1, 12, 34, 56));
    let age = |p: Id<Post>| (t0 - secs(creation_date.get(p).unwrap())) / 86400.0;
    let mut v = drain((&first).select(owner_user.select(Ident::<User>::new().with((&tu).filt(|n| n > 5))).and((&phs).opt())));
    v.sort_by(|&(p, _), &(q, _)| score.get(q).unwrap().cmp(&score.get(p).unwrap()).then(age(p).partial_cmp(&age(q)).unwrap()));
    rows(v.into_iter().map(|(p, (u, h))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::F(age(p)), V::I(h.map_or(0, |h| h.0)), V::S(if h.map_or(false, |h| h.1 != i64::MIN) { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(COALESCE(b.Class, 0)) AS TotalBadges, MAX(p.ViewCount) AS MaxPostViewCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.DisplayName, u.Reputation),
// ClosedPostStats AS (SELECT p.Id AS PostId, COUNT(ph.Id) AS CloseCount, MAX(ph.CreationDate) AS LastClosed FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY p.Id)
// SELECT rp.Title, rp.Score, us.DisplayName, us.Reputation, us.TotalBadges, us.MaxPostViewCount, cps.CloseCount, cps.LastClosed, CASE WHEN rp.ScoreRank = 1 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId LEFT JOIN ClosedPostStats cps ON rp.Id = cps.PostId WHERE rp.CommentCount > 0 ORDER BY us.Reputation DESC, rp.Score DESC LIMIT 100;
//
// CURRENT_TIMESTAMP is TIMESTAMPTZ, so CreationDate is read as New York local time.
fn q3805(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let since = ny_to_utc(add_years(utc_to_ny(now_utc()), -1));
    let rp = || db.post.with(creation_date.filt(move |d| ny_to_utc(d) >= since));
    let v = drain(rp().select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = rp().with(owner_user).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = top_n(drain(rp().with(&cc).select(owner_user)), |&(p, u): &(Id<Post>, Id<User>)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(view_count.opt()).opt())).fold((0i64, None::<i64>), |(s, m), (b, w)| {
        (s + b.unwrap_or(0), match (m, w.flatten()) { (Some(a), Some(b)) => Some(a.max(b)), (a, b) => a.or(b) })
    });
    let cps = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11]))).select(&db.post_history.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&us)).and((&cps).opt()).and(Ident::<Post>::new().with(&first).opt())));
    v.sort_by_key(|&(p, (((u, _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, (((u, (b, m)), c), t))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), oint(m)]);
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if t.is_some() { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// UserVotes AS (SELECT v.PostId, v.UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId, v.UserId),
// HighestVoted AS (SELECT PostId, SUM(UpVotes) - SUM(DownVotes) AS VoteDifferential FROM UserVotes GROUP BY PostId),
// PostHistoryUpdates AS (SELECT ph.PostId, COUNT(*) AS HistoryCount, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastCloseDate,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS LastReopenDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, COALESCE(hv.VoteDifferential, 0) AS TotalVotes, COALESCE(ph.LastCloseDate, ph.LastReopenDate, NULL) AS LastUpdate,
//        CASE WHEN rp.RankScore = 1 THEN 'Top Post' ELSE 'Regular Post' END AS PostRank
// FROM RankedPosts rp LEFT JOIN HighestVoted hv ON rp.PostId = hv.PostId LEFT JOIN PostHistoryUpdates ph ON rp.PostId = ph.PostId WHERE rp.CommentCount > 5 ORDER BY rp.Score DESC, rp.CreationDate ASC LIMIT 50;
//
// HighestVoted sums the per-(post, voter) sums, which is the post's own up minus down count.
fn q20430(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let posts = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let v = drain(posts().select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let top: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = posts().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&cc).filt(|n| n > 5)), |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 50);
    let tp = rel(v);
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let uv = db.vote.with(&db.vote.post).group_by((&db.vote.post).and((&db.vote.user).opt())).select(&db.vote.vote_type_id).fold(0i64, |s, t| s + (t == 2) as i64 - (t == 3) as i64);
    let uvr = rel(drain(&uv));
    let hv = (&uvr).with(Same::<((Id<Post>, Option<Id<User>>), i64)>::new().map(|((p, _), _): ((Id<Post>, Option<Id<User>>), i64)| p).select(Ident::<Post>::new().with(&tps)))
        .group_by(Same::<((Id<Post>, Option<Id<User>>), i64)>::new().map(|((p, _), _): ((Id<Post>, Option<Id<User>>), i64)| p))
        .select(Same::<((Id<Post>, Option<Id<User>>), i64)>::new().map(|(_, s): ((Id<Post>, Option<Id<User>>), i64)| s))
        .fold(0i64, |a, s| a + s);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = (&tps).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold([i64::MIN; 2], |a, (t, d)| {
        [if t == 10 { a[0].max(d) } else { a[0] }, if t == 11 { a[1].max(d) } else { a[1] }]
    });
    let v = drain((&tp).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _): (Id<Post>, i64)| p).select((&hv).opt().and((&ph).opt()).and(Ident::<Post>::new().with(&top).opt())))));
    rows(v.into_iter().map(|(_, ((p, c), ((h, u), t)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(h.unwrap_or(0))]);
        let lu = u.and_then(|a| if a[0] != i64::MIN { Some(a[0]) } else if a[1] != i64::MIN { Some(a[1]) } else { None });
        f.extend([ots(lu), V::S(if t.is_some() { "Top Post" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpvoteCount, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, (SELECT COUNT(*) FROM Badges B WHERE B.UserId = U.Id) AS BadgeCount FROM Users U WHERE U.Reputation > 1000),
// PostHistories AS (SELECT PH.PostId, PH.PostHistoryTypeId, COUNT(PH.Id) AS ChangeCount, MAX(PH.CreationDate) AS LastChangeDate FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11, 12)
//     GROUP BY PH.PostId, PH.PostHistoryTypeId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, U.DisplayName AS Owner, U.Reputation, U.BadgeCount, RP.CommentCount, RP.UpvoteCount, RP.DownvoteCount, PH.ChangeCount, PH.LastChangeDate,
//        COALESCE(NULLIF(PH.ChangeCount, 0), 1) AS NonZeroChangeCount, CASE WHEN RP.Score > 100 THEN 'Hot Post' ELSE 'Regular Post' END AS PostStatus
// FROM RecentPosts RP LEFT JOIN UserReputation U ON RP.OwnerUserId = U.UserId LEFT JOIN PostHistories PH ON RP.PostId = PH.PostId WHERE RP.PostRank <= 5 ORDER BY RP.CreationDate DESC LIMIT 100;
fn q21403(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let top = top_n(top, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phg = db.post_history.with(post_history_type_id.is_in([10, 11, 12])).group_by(post.and(post_history_type_id)).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let phr = rel(drain(&phg));
    let ph_by: HashIdx<Id<Post>, ((Id<Post>, i64), (i64, i64))> = (&phr).map(|((p, _), _)| p).inv().select(&phr).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&pv).and(&cc).and(owner_user.select(Ident::<User>::new().and(&bc)).opt()).and((&ph_by).opt()));
    let v = top_n(v, |&(p, (_, h))| (Reverse(creation_date.get(p).unwrap()), p, h.map(|x| x.0 .1)), 100);
    rows(v.into_iter().map(|(p, (((a, c), u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some((_, (n, d))) => [V::I(n), V::T(d), V::I(if n == 0 { 1 } else { n })],
            None => [V::Null, V::Null, V::I(1)],
        });
        f.push(V::S(if score.get(p).unwrap() > 100 { "Hot Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT PH.PostId, PH.PostHistoryTypeId, PH.CreationDate, CAST(NULL AS VARCHAR(400)) AS PreviousAction, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS RowNum
//     FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11, 12)),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostsCreated, SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore, AVG(COALESCE(P.ViewCount, 0)) AS AvgViewCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TagStats AS (SELECT T.TagName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName)
// SELECT U.DisplayName AS UserName, U.PostsCreated, U.TotalScore, U.AvgViewCount, P.Title AS PostTitle, PH.PostHistoryTypeId, PH.CreationDate AS ActionDate,
//        CASE WHEN PH.PostHistoryTypeId = 10 THEN 'Closed' WHEN PH.PostHistoryTypeId = 11 THEN 'Reopened' WHEN PH.PostHistoryTypeId = 12 THEN 'Deleted' ELSE 'Other Action' END AS ActionType,
//        T.TagName AS PostTag, TS.PostCount AS TagPostCount, TS.TotalViews AS TagTotalViews
// FROM UserActivity U INNER JOIN RecursivePostHistory PH ON U.UserId = PH.PostId INNER JOIN Posts P ON P.Id = PH.PostId LEFT JOIN Tags T ON P.Tags LIKE '%' || T.TagName || '%'
// LEFT JOIN TagStats TS ON T.TagName = TS.TagName WHERE U.TotalScore > 0 ORDER BY U.TotalScore DESC, PH.CreationDate DESC;
//
// `U.UserId = PH.PostId` joins a user id to a post id, so it goes through the raw ids. RowNum is never read.
fn q32768(db: &'static So) -> String {
    let Post { score, view_count, origid, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + 1],
        None => [a[0], a[1], a[2], a[3] + 1],
    });
    let lt = tag_mentions(db);
    type PT = (Id<Post>, Id<Tag>);
    let tags_of: HashIdx<Id<Post>, Id<Tag>> = (&lt).map(|(p, _)| p).inv().map(|(_, t): PT| t).collect();
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t)| t).inv().map(|(p, _): PT| p).collect();
    let ts = db.tag.group_by(Ident::<Tag>::new()).select((&by_tag).select(view_count.opt()).opt()).fold([0i64; 2], |a, w| match w {
        Some(w) => [a[0] + 1, a[1] + w.unwrap_or(0)],
        None => a,
    });
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let v = drain(
        db.post_history
            .with(post_history_type_id.is_in([10, 11, 12]))
            .select(post.select(origid.select(&uid).select(Ident::<User>::new().and((&ua).filt(|a| a[1] > 0)))).and(post.select((&tags_of).select(Ident::<Tag>::new().and(&ts)).opt()))),
    );
    let mut v = v;
    v.sort_by_key(|&(h, ((_, a), _))| (Reverse(a[1]), Reverse(hd.get(h).unwrap())));
    rows(v.into_iter().map(|(h, ((u, a), t))| {
        let ty = post_history_type_id.get(h).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), avg(a[2], a[3])];
        f.extend(post_fields(db, post.get(h).unwrap(), &["title"]));
        f.extend([V::I(ty), V::T(hd.get(h).unwrap()), V::S(match ty { 10 => "Closed", 11 => "Reopened", _ => "Deleted" })]);
        f.extend(match t {
            Some((t, s)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(s[0]), V::I(s[1])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankScore,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS RankViews FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostLinksInfo AS (SELECT pl.PostId, pl.RelatedPostId, lt.Name AS LinkType FROM PostLinks pl JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COALESCE(AVG(p.Score), 0) AS AvgPostScore
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT u.DisplayName, rp.Title, rp.Score, rp.ViewCount, pb.BadgeCount, pb.GoldBadges, pli.LinkType, ua.TotalBounties, ua.AvgPostScore
// FROM RankedPosts rp JOIN Users u ON rp.Id = u.Id LEFT JOIN PostBadges pb ON u.Id = pb.UserId LEFT JOIN PostLinksInfo pli ON rp.Id = pli.PostId JOIN UserActivity ua ON u.Id = ua.UserId
// WHERE (pb.BadgeCount > 0 OR u.Reputation > 1000) AND (rp.RankScore <= 5 OR rp.RankViews <= 5) ORDER BY rp.Score DESC, ua.TotalBounties DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
//
// `rp.Id = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q1292(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let mut keep = top_per(v.clone(), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    keep.extend(top_per(v, |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 5, false));
    let rp: MatSet<Id<Post>> = rel(keep.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 2], |a, c| [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64]);
    let us: MatSet<Id<User>> = (&rp).select(origid.select(&uid)).collect();
    let ua = (&us).group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(posts_of(db).select(score).opt())).fold([0i64; 3], |a, (b, s)| {
        [a[0] + b.flatten().unwrap_or(0), a[1] + s.is_some() as i64, a[2] + s.unwrap_or(0)]
    });
    let ok = Ident::<User>::new().and(&pb).and(&ua).filt(|((u, b), _): ((Id<User>, [i64; 2]), [i64; 3])| b[0] > 0 || db.user.reputation.get(u).unwrap() > 1000);
    let lt = links_of(db).select((&db.post_link.link_type).select(&db.link_type.name));
    let v = drain((&rp).select(origid.select(&uid).select(ok).and(lt.opt())));
    let v = top_n(v, |&(p, (((_, _), a), l))| (Reverse(score.get(p).unwrap()), Reverse(a[0]), p, l), 20);
    rows(v.into_iter().skip(10).map(|(p, (((u, b), a), l))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(b[0]), V::I(b[1]), ostr(l), V::I(a[0]), if a[1] == 0 { V::F(0.0) } else { avg(a[2], a[1]) }]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS ActivePosts FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// FinalResult AS (SELECT uvs.DisplayName, ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.TotalBounty, CASE WHEN ps.CommentCount = 0 THEN 'No Comments' ELSE 'Comments Available' END AS CommentStatus,
//        uvs.UpVotes, uvs.DownVotes, uvs.ActivePosts FROM UserVoteStats uvs JOIN PostStats ps ON uvs.UserId = ps.OwnerUserId WHERE uvs.ActivePosts > 5)
// SELECT FR.DisplayName, FR.Title, FR.CreationDate, FR.CommentCount, FR.TotalBounty, FR.CommentStatus, (FR.UpVotes - FR.DownVotes) AS NetVotes,
//        CASE WHEN FR.TotalBounty > 0 THEN 'Bounty Offered' WHEN FR.CommentCount > 10 THEN 'Popular Discussion' ELSE 'Standard Post' END AS PostCategory
// FROM FinalResult FR ORDER BY FR.CreationDate DESC LIMIT 100 OFFSET 0;
//
// The order reads only CreationDate, so the hundred newest posts of the qualifying users are picked first and their comment x bounty product is driven for them alone.
fn q3777(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let uvs = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).opt())).opt())
        .buf_fold(|v| (v.iter().map(|x| (x.map(|y| y.0) == Some(2)) as i64 - (x.map(|y| y.0) == Some(3)) as i64).sum::<i64>(), distinct_some(v.iter().map(|x| x.and_then(|y| y.1)))));
    let v = top_n(drain(db.post.select(owner_user.select(Ident::<User>::new().and((&uvs).filt(|(_, n)| n > 5))))), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let tp = rel(v);
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let ps = (&tps).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let mut v = drain((&tp).select(Same::<(Id<Post>, (Id<User>, (i64, i64)))>::new().and(Same::<(Id<Post>, (Id<User>, (i64, i64)))>::new().map(|(p, _): (Id<Post>, (Id<User>, (i64, i64)))| p).select(&ps))));
    v.sort_by_key(|&(i, _)| i);
    rows(v.into_iter().map(|(_, ((p, (u, (net, _))), a))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if a[0] == 0 { "No Comments" } else { "Comments Available" }), V::I(net)]);
        f.push(V::S(if a[1] > 0 { "Bounty Offered" } else if a[0] > 10 { "Popular Discussion" } else { "Standard Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(*) OVER (PARTITION BY p.PostTypeId) AS TotalCount FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// PostVotes AS (SELECT v.PostId, vt.Name AS VoteType, COUNT(v.Id) AS VoteCount FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId, vt.Name),
// FilteredPostVotes AS (SELECT pv.PostId, SUM(CASE WHEN pv.VoteType = 'UpMod' THEN pv.VoteCount ELSE 0 END) AS UpVotes, SUM(CASE WHEN pv.VoteType = 'DownMod' THEN pv.VoteCount ELSE 0 END) AS DownVotes
//     FROM PostVotes pv GROUP BY pv.PostId),
// TopRatedPosts AS (SELECT rp.PostId, rp.Title, rp.TotalCount, COALESCE(fp.UpVotes, 0) AS UpVotes, COALESCE(fp.DownVotes, 0) AS DownVotes FROM RankedPosts rp LEFT JOIN FilteredPostVotes fp ON rp.PostId = fp.PostId
//     WHERE rp.rn <= 5)
// SELECT trp.PostId, trp.Title, trp.TotalCount, trp.UpVotes, trp.DownVotes, CASE WHEN trp.UpVotes + trp.DownVotes > 0 THEN ROUND((trp.UpVotes * 1.0 / (trp.UpVotes + trp.DownVotes)) * 100, 2) ELSE 0 END AS UpVotePercentage,
//        CASE WHEN EXISTS (SELECT 1 FROM PostHistory ph WHERE ph.PostId = trp.PostId AND ph.PostHistoryTypeId = 10) THEN 'Closed' ELSE 'Active' END AS PostStatus
// FROM TopRatedPosts trp ORDER BY trp.UpVotes DESC NULLS LAST, trp.DownVotes ASC NULLS FIRST;
//
// FilteredPostVotes sums the per-(post, type) counts, which is the post's own UpMod and DownMod count.
fn q24823(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let posts = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)));
    let tc = posts().group_by(post_type_id).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = top_per(drain(posts().select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let fv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    let v = drain((&fv).and(post_type_id.select(&tc)).and(Ident::<Post>::new().with(&closed).opt()));
    rows(v.into_iter().map(|(p, ((a, n), c))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.push(V::F(if a[0] + a[1] > 0 { (a[0] as f64 / (a[0] + a[1]) as f64 * 100.0 * 100.0).round() / 100.0 } else { 0.0 }));
        f.push(V::S(if c.is_some() { "Closed" } else { "Active" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COALESCE(badge_count.Count, 0) AS BadgeCount, u.Views,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY u.CreationDate DESC) AS UserRank
//     FROM Users u LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM Badges WHERE Class = 1 GROUP BY UserId) badge_count ON u.Id = badge_count.UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, ur.BadgeCount, ps.TotalPosts, ps.TotalViews, ps.TotalScore, COALESCE(ur.Views, 0) + COALESCE(ps.TotalViews, 0) AS CombinedViews
//     FROM UserReputation ur LEFT JOIN PostStats ps ON ur.UserId = ps.OwnerUserId),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY CombinedViews DESC) AS ViewRank FROM UserPerformance)
// SELECT tuv.UserId, tuv.DisplayName, tuv.Reputation, tuv.BadgeCount, tuv.TotalPosts, tuv.TotalViews, tuv.TotalScore, tuv.CombinedViews,
//        CASE WHEN tuv.BadgeCount > 5 THEN 'Enthusiast' WHEN tuv.BadgeCount BETWEEN 3 AND 5 THEN 'Intermediate' ELSE 'Novice' END AS UserTier, COALESCE(u2.Location, 'N/A') AS Location
// FROM TopUsers tuv LEFT JOIN Users u2 ON tuv.UserId = u2.Id WHERE tuv.ViewRank <= 10 ORDER BY tuv.CombinedViews DESC OFFSET (SELECT COUNT(*) FROM TopUsers) * 0.2 LIMIT 5;
fn q24758(db: &'static So) -> String {
    let gold = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let Post { owner_user, creation_date, view_count, score, .. } = &db.post;
    let ps = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(view_count.opt().and(score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let v = drain(db.user.select((&db.user.views).and((&gold).opt()).and((&ps).opt())));
    let n = v.len() as f64;
    let v = ranked(v, |&(_, ((w, _), p))| Reverse(w + p.map_or(0, |a| a[1])), false);
    let offset = (n * 0.2).round() as usize;
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).collect();
    rows(v.into_iter().skip(offset).take(5).map(|((u, ((w, g), p)), _)| {
        let b = g.unwrap_or(0);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(b));
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2])],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::I(w + p.map_or(0, |a| a[1])));
        f.push(V::S(if b > 5 { "Enthusiast" } else if (3..=5).contains(&b) { "Intermediate" } else { "Novice" }));
        f.push(V::S(db.user.location.get(u).unwrap_or("N/A")));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM Users),
// PostMetrics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId),
// TopUsers AS (SELECT ur.Id AS UserId, ur.Reputation, pm.TotalPosts, pm.Questions, pm.Answers, pm.Upvotes, pm.Downvotes FROM UserReputation ur JOIN PostMetrics pm ON ur.Id = pm.OwnerUserId WHERE ur.Reputation IS NOT NULL),
// PostHistoryCounts AS (SELECT ph.UserId, COUNT(ph.Id) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.UserId),
// FinalMetrics AS (SELECT tu.UserId, tu.Reputation, COALESCE(phc.EditCount, 0) AS EditCount, tu.TotalPosts, tu.Questions, tu.Answers, tu.Upvotes, tu.Downvotes,
//        CASE WHEN tu.Reputation > 10000 THEN 'High Rank' ELSE 'Low Rank' END AS ReputationCategory FROM TopUsers tu LEFT JOIN PostHistoryCounts phc ON tu.UserId = phc.UserId)
// SELECT f.UserId, f.Reputation, f.EditCount, f.TotalPosts, f.Questions, f.Answers, f.Upvotes, f.Downvotes, f.ReputationCategory FROM FinalMetrics f WHERE f.EditCount > 10
// ORDER BY f.Reputation DESC, f.TotalPosts DESC FETCH FIRST 10 ROWS ONLY;
fn q2090(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let pm = db.post.group_by(owner_user).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 5], |a, (t, v)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]
    });
    let ec = db.post_history.with((&db.post_history.post_history_type_id).is_in([4, 5])).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&pm).and((&ec).filt(|n| n > 10))), |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (a, e))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "uid"), V::I(r), V::I(e)];
        f.extend(a.map(V::I));
        f.push(V::S(if r > 10000 { "High Rank" } else { "Low Rank" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, COUNT(DISTINCT p.Id) AS PostCount,
//        COUNT(DISTINCT CASE WHEN p.Score > 0 THEN p.Id END) AS PositivePosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionsCount,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswersCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// Ranking AS (SELECT UserId, DisplayName, Reputation, GoldBadges, SilverBadges, BronzeBadges, PostCount, PositivePosts, QuestionsCount, AnswersCount, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserStats),
// TopUsers AS (SELECT * FROM Ranking WHERE UserRank <= 10)
// SELECT t.DisplayName, t.Reputation, t.GoldBadges, t.SilverBadges, t.BronzeBadges, t.PostCount, t.PositivePosts, (t.QuestionsCount * 1.0 / NULLIF(t.PostCount, 0)) AS QuestionRatio,
//        (t.AnswersCount * 1.0 / NULLIF(t.PostCount, 0)) AS AnswerRatio, ps.TotalVotes
// FROM TopUsers t LEFT JOIN (SELECT v.UserId, COUNT(v.Id) AS TotalVotes FROM Votes v GROUP BY v.UserId) ps ON t.UserId = ps.UserId WHERE (t.GoldBadges + t.SilverBadges + t.BronzeBadges) > 0 ORDER BY t.Reputation DESC;
//
// UserRank reads only Reputation, so the ten users are picked first and the badges x posts product is driven for them alone.
fn q3969(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { score, post_type_id, .. } = &db.post;
    let bp = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold([0i64; 3], |a, (c, _)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let pd = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(post_type_id)).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, t)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64],
        None => a,
    });
    let tv = db.vote.group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&bp).filt(|a| a[0] + a[1] + a[2] > 0).and(&pd).and((&tv).opt()));
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, ((b, p), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(p[0]), V::I(p[1])]);
        f.extend(if p[0] == 0 { [V::Null, V::Null] } else { [V::F(p[2] as f64 / p[0] as f64), V::F(p[3] as f64 / p[0] as f64)] });
        f.push(oint(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.PostTypeId, p.Score),
// TopQuestions AS (SELECT PostId, Title, CommentCount FROM RankedPosts WHERE Rank <= 5 AND CommentCount > 0),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, COALESCE(u.DisplayName, 'System') AS UserDisplayName, ph.Comment, ph.Text,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank FROM PostHistory ph LEFT JOIN Users u ON ph.UserId = u.Id WHERE ph.PostHistoryTypeId IN (10, 11, 52, 66)),
// CommentsSummary AS (SELECT p.Id AS PostId, SUM(COALESCE(c.Score, 0)) AS TotalCommentScore, COUNT(c.Id) AS TotalComments FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id)
// SELECT tq.PostId, tq.Title, tq.CommentCount, ph.UserDisplayName, ph.Comment, ph.Text, ph.CreationDate AS HistoryDate, cs.TotalCommentScore, cs.TotalComments
// FROM TopQuestions tq JOIN PostHistoryDetails ph ON tq.PostId = ph.PostId JOIN CommentsSummary cs ON tq.PostId = cs.PostId WHERE ph.HistoryRank = 1 AND (cs.TotalCommentScore IS NULL OR cs.TotalCommentScore >= 10)
// ORDER BY tq.CommentCount DESC, ph.CreationDate DESC;
fn q21314(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tq = (&tp).with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let PostHistory { post, post_history_type_id, creation_date: hd, user, comment, text, .. } = &db.post_history;
    let lh = drain(db.post_history.with(post_history_type_id.is_in([10, 11, 52, 66])).select(post));
    let lh = top_per(lh, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let lh = rel(lh.into_iter().map(|(h, p)| (p, h)).collect());
    let lh: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&lh).map(|(p, _)| p).inv().select(&lh).collect();
    let mut v = drain((&tq).filt(|a| a[1] >= 10).and((&lh).map(|(_, h)| h)));
    v.sort_by_key(|&(p, (a, h))| (Reverse(a[0]), Reverse(hd.get(h).unwrap()), p));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::I(a[0]));
        f.push(user.get(h).map_or(V::S("System"), |u| user_col(db, u, "name")));
        f.extend([ostr(comment.get(h)), ostr(text.get(h)), V::T(hd.get(h).unwrap()), V::I(a[1]), V::I(a[0])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// BadgeStats AS (SELECT b.UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// CombinedStats AS (SELECT us.UserId, us.PostCount, us.QuestionCount, us.AnswerCount, us.WikiCount, us.Upvotes, us.Downvotes, COALESCE(bs.BadgeCount, 0) AS BadgeCount, COALESCE(bs.GoldBadges, 0) AS GoldBadges,
//        COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges FROM UserStats us LEFT JOIN BadgeStats bs ON us.UserId = bs.UserId)
// SELECT c.UserId, c.PostCount, c.QuestionCount, c.AnswerCount, c.WikiCount, c.Upvotes, c.Downvotes, c.BadgeCount, c.GoldBadges, c.SilverBadges, c.BronzeBadges,
//        RANK() OVER (ORDER BY (c.Upvotes - c.Downvotes) DESC) AS UserRank FROM CombinedStats c WHERE c.PostCount > 0 ORDER BY UserRank FETCH FIRST 10 ROWS ONLY;
fn q9991(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 5], |a, (t, v)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&us).and(&pc)), |&(_, (a, _))| Reverse(a[3] - a[4]), false);
    let v = top_n(v, |&((u, (a, _)), r)| (r, Reverse(a[3] - a[4]), u), 10);
    let tu = rel(v);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    type T = ((Id<User>, ([i64; 5], i64)), i64);
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|((u, _), _): T| u).select((&bs).opt()))));
    v.sort_by_key(|&(i, _)| i);
    rows(v.into_iter().map(|(_, (((u, (a, n)), r), b))| {
        let mut f = vec![user_col(db, u, "uid"), V::I(n)];
        f.extend(a.map(V::I));
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(v.BountyAmount) AS TotalBounties, COUNT(DISTINCT p.Id) AS TotalPosts FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Votes v ON v.UserId = u.Id WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5),
// PostHistoryDetails AS (SELECT ph.PostId, ph.UserId, ph.PostHistoryTypeId, ph.CreationDate AS HistoryDate, ph.Comment, ph.Text, p.Title FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.CreationDate >= cast('2024-10-01' as date) - INTERVAL '6 months')
// SELECT ru.UserId, ru.DisplayName, COUNT(DISTINCT r.PostId) AS UserTopPosts, COALESCE(SUM(phd.TotalChanges), 0) AS RecentPostEdits, COALESCE(AVG(r.Score), 0) AS AvgPostScore,
//        COUNT(DISTINCT ph.PostId) AS RecentPostHistoryCount
// FROM TopUsers ru LEFT JOIN RankedPosts r ON ru.UserId = r.PostId AND r.ScoreRank <= 10
// LEFT JOIN (SELECT PostId, COUNT(*) AS TotalChanges FROM PostHistory WHERE CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY PostId) phd ON r.PostId = phd.PostId
// LEFT JOIN PostHistoryDetails ph ON ru.UserId = ph.UserId GROUP BY ru.UserId, ru.DisplayName ORDER BY UserTopPosts DESC, AvgPostScore DESC;
//
// `ru.UserId = r.PostId` joins a user id to a post id, so it goes through the raw ids. TotalBounties is never read, and COUNT(DISTINCT p.Id) is the post count.
fn q1990(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, origid, .. } = &db.post;
    let d0 = date(2024, 10, 1);
    let v = drain(db.post.with(creation_date.ge(add_years(d0, -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp_by: HashIdx<i64, Id<Post>> = (&rp).select(origid).inv().collect();
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let phd = db.post_history.with((&db.post_history.creation_date).ge(add_days(d0, -30))).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { user, post, creation_date: hd, .. } = &db.post_history;
    let ph: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(hd.ge(add_months(d0, -6))).select(user).inv().collect();
    let g = db
        .user
        .with((&pc).filt(|n| n > 5))
        .group_by(Ident::<User>::new())
        .select((&db.user.origid).select(&rp_by).select(Ident::<Post>::new().and(score).and((&phd).opt())).opt().and((&ph).select(post).opt()))
        .buf_fold(|v| {
            let n = distinct_some(v.iter().map(|x| x.0.map(|y| y.0 .0)));
            let e: i64 = v.iter().map(|x| x.0.and_then(|y| y.1).unwrap_or(0)).sum();
            let k: i64 = v.iter().map(|x| x.0.is_some() as i64).sum();
            let s: i64 = v.iter().map(|x| x.0.map_or(0, |y| y.0 .1)).sum();
            (n, e, k, s, distinct_some(v.iter().map(|x| x.1)))
        });
    let mut v = drain(&g);
    let m = |k: i64, s: i64| if k == 0 { 0.0 } else { s as f64 / k as f64 };
    v.sort_by(|&(_, (n, _, k, s, _)), &(_, (n2, _, k2, s2, _))| n2.cmp(&n).then(m(k2, s2).partial_cmp(&m(k, s)).unwrap()));
    rows(v.into_iter().map(|(u, (n, e, k, s, c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(e), V::F(m(k, s)), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// PostVoteCounts AS (SELECT PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes FROM Votes v
//     JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId),
// RecentPostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, pht.Name AS HistoryType FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
//     WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes,
//        (COALESCE(pvc.UpVotes, 0) - COALESCE(pvc.DownVotes, 0)) AS NetVotes, COUNT(rph.PostId) AS RecentHistoryCount
//     FROM RankedPosts rp LEFT JOIN PostVoteCounts pvc ON rp.PostId = pvc.PostId LEFT JOIN RecentPostHistory rph ON rp.PostId = rph.PostId WHERE rp.Rank <= 3
//     GROUP BY rp.PostId, rp.Title, rp.Score, rp.ViewCount, pvc.UpVotes, pvc.DownVotes)
// SELECT *, CASE WHEN RecentHistoryCount > 0 THEN 'Has Recent Changes' ELSE 'No Recent Changes' END AS ChangeStatus FROM FinalResults ORDER BY NetVotes DESC, Score DESC, ViewCount DESC;
fn q518(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    let recent = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let rh = (&tp).group_by(Ident::<Post>::new()).select(recent.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&pv).and(&rh));
    rows(v.into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::I(n), V::S(if n > 0 { "Has Recent Changes" } else { "No Recent Changes" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserActivity AS (SELECT u.Id AS UserId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        COUNT(DISTINCT ph.Id) AS EditCount, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id),
// TopUsers AS (SELECT u.Id, u.DisplayName, ua.Upvotes, ua.Downvotes, ua.EditCount, ua.CommentCount, RANK() OVER (ORDER BY ua.Upvotes - ua.Downvotes DESC, ua.EditCount DESC) AS ActivityRank
//     FROM Users u JOIN UserActivity ua ON u.Id = ua.UserId WHERE u.Reputation > 1000)
// SELECT r.PostId, r.Title, u.DisplayName, r.CreationDate, r.Score, r.ViewCount, COALESCE(ua.Upvotes, 0) AS UserUpvotes, COALESCE(ua.Downvotes, 0) AS UserDownvotes, COALESCE(ua.EditCount, 0) AS UserEditCount,
//        COALESCE(ua.CommentCount, 0) AS UserCommentCount, CASE WHEN r.Rank = 1 THEN 'Most Recent' ELSE 'Other' END AS PostStatus
// FROM RankedPosts r JOIN Users u ON r.OwnerUserId = u.Id LEFT JOIN UserActivity ua ON u.Id = ua.UserId WHERE r.Rank <= 5 ORDER BY r.CreationDate DESC;
//
// TopUsers is never referenced. The distinct counts are each table's own count over the user's posts, so they come from one fold per child.
fn q3560(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let top = per_group(ranked(top, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let tp = rel(top.into_iter().map(|((p, u), r)| (p, u, r)).collect());
    let owners: MatSet<Id<User>> = (&tp).map(|(_, u, _)| u).collect();
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(history_of(db).opt().and(comments_of(db).opt())).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ec = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(history_of(db)).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let cc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type T = (Id<Post>, Id<User>, i64);
    let mut v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(_, u, _): T| u).select((&ua).and(&ec).and(&cc)))));
    v.sort_by_key(|&(_, ((p, _, _), _))| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(_, ((p, u, r), ((a, e), c)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["created", "score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(e), V::I(c), V::S(if r == 1 { "Most Recent" } else { "Other" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS Downvotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2023-01-01'),
// TopPosts AS (SELECT PostId, Title, DisplayName, Score, CreationDate, Upvotes, Downvotes, CommentCount FROM RankedPosts WHERE rn = 1),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, cr.Name AS CloseReason FROM PostHistory ph INNER JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId = 10),
// FinalSelection AS (SELECT tp.*, cp.CloseDate, cp.CloseReason FROM TopPosts tp LEFT JOIN ClosedPosts cp ON tp.PostId = cp.PostId)
// SELECT F.Title, F.DisplayName, F.Score, F.CreationDate, COALESCE(F.CommentCount, 0) AS TotalComments, COALESCE(F.Upvotes, 0) AS TotalUpvotes, COALESCE(F.Downvotes, 0) AS TotalDownvotes,
//        CASE WHEN F.CloseDate IS NOT NULL THEN TRUE ELSE FALSE END AS IsClosed, F.CloseReason
// FROM FinalSelection F WHERE F.Score > (SELECT AVG(Score) FROM Posts) ORDER BY F.Score DESC LIMIT 10;
//
// rn numbers each owner's joined post x comment x vote rows by score, so it picks one row of the owner's top post; the window aggregates are that post's.
fn q4326(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let (s, n) = score.fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let mean = s as f64 / n as f64;
    let v = drain(db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let agg = (&tp)
        .with(score.filt(move |x| x as f64 > mean))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)));
    let v = top_n(drain((&agg).and(closes.opt())), |&(p, (_, c))| (Reverse(score.get(p).unwrap()), p, c.map(|x| x.0)), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "created"]);
        f.extend(a.map(V::I));
        f.push(V::B(c.is_some()));
        f.push(c.map_or(V::Null, |(_, r)| V::S(r)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, AnswerCount, CreationDate, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.OwnerDisplayName, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 END), 0) AS AverageUpVotes,
//        COALESCE(AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 END), 0) AS AverageDownVotes FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.OwnerDisplayName)
// SELECT pd.Title, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.OwnerDisplayName, pt.Name AS PostTypeName, MAX(b.Date) AS LastBadgeDate
// FROM PostDetails pd JOIN PostTypes pt ON pd.PostId = pt.Id LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pd.PostId)
// GROUP BY pd.Title, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.OwnerDisplayName, pt.Name ORDER BY pd.Score DESC, pd.ViewCount DESC LIMIT 10;
//
// `pd.PostId = pt.Id` joins a post id to a post-type id, so it goes through the raw ids; the correlated subquery reads the post's own owner.
fn q8692(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ptid: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let pd = (&tp).with(origid.select(&ptid)).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let lb = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).select(&db.badge.date)).fold(i64::MIN, |m, d| m.max(d));
    type K = (Option<Str>, i64, Option<i64>, Option<i64>, i64, Str, Str);
    let rows_ = rel(drain((&pd).and(origid.select(&ptid).select(&db.post_type.name)).and((&lb).opt())).into_iter().map(|(p, ((c, t), b))| {
        let k: K = (db.post.title.get(p), score.get(p).unwrap(), view_count.get(p), db.post.answer_count.get(p), c, db.user.display_name.get(owner_user.get(p).unwrap()).unwrap(), t);
        (k, b)
    }).collect());
    let g = (&rows_).group_by(Same::<(K, Option<i64>)>::new().map(|(k, _): (K, Option<i64>)| k)).select(Same::<(K, Option<i64>)>::new().map(|(_, b): (K, Option<i64>)| b)).fold(i64::MIN, |m, b| b.map_or(m, |b| m.max(b)));
    let v = top_n(drain(&g), |&(k, _)| (Reverse(k.1), k.2.is_none(), Reverse(k.2), k), 10);
    rows(v.into_iter().map(|((t, s, w, a, c, o, n), b)| row(vec![ostr(t), V::I(s), oint(w), oint(a), V::I(c), V::S(o), V::S(n), tmax(b)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, SUM(c.Score) AS TotalCommentScore FROM Comments c GROUP BY c.PostId),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostHistories AS (SELECT ph.PostId, COUNT(ph.Id) AS HistoryCount FROM PostHistory ph WHERE ph.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months') GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, COALESCE(pc.CommentCount, 0) AS Comments, COALESCE(pc.TotalCommentScore, 0) AS TotalCommentScore, COALESCE(pv.UpVotes, 0) AS UpVotes,
//        COALESCE(pv.DownVotes, 0) AS DownVotes, COALESCE(ph.HistoryCount, 0) AS HistoryCount, rp.Score + COALESCE(pc.TotalCommentScore, 0) AS AdjustedScore
// FROM RankedPosts rp LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId LEFT JOIN PostHistories ph ON rp.PostId = ph.PostId
// WHERE rp.rn = 1 AND (rp.Score > 0 OR COALESCE(pc.CommentCount, 0) > 0) ORDER BY AdjustedScore DESC, rp.CreationDate DESC LIMIT 100;
fn q2258(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0)]);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let recent = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).ge(add_months(t0, -6))));
    let ph = (&tp).group_by(Ident::<Post>::new()).select(recent.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let q = (&pc).and(score).filt(|(c, s): ([i64; 2], i64)| s > 0 || c[0] > 0).and(&pv).and(&ph);
    let v = top_n(drain(q), |&(p, (((c, s), _), _))| (Reverse(s + c[1]), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (((c, s), a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c[0]), V::I(c[1]), V::I(a[0]), V::I(a[1]), V::I(h), V::I(s + c[1])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, LastAccessDate, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users WHERE Reputation IS NOT NULL),
// PostStatistics AS (SELECT P.Id AS PostId, P.OwnerUserId, P.PostTypeId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(LENGTH(P.Body)) AS AvgBodyLength
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.OwnerUserId, P.PostTypeId),
// TopPosts AS (SELECT PS.PostId, PS.OwnerUserId, PS.CommentCount, PS.UpVotes, PS.DownVotes, PS.AvgBodyLength, COALESCE(UR.Reputation, 0) AS OwnerReputation,
//        CASE WHEN PS.CommentCount > 10 THEN 'Highly Discussed' WHEN PS.CommentCount BETWEEN 5 AND 10 THEN 'Moderately Discussed' ELSE 'Less Discussed' END AS DiscussionCategory
//     FROM PostStatistics PS LEFT JOIN UserReputation UR ON PS.OwnerUserId = UR.UserId),
// AggregateResults AS (SELECT DiscussionCategory, COUNT(*) AS TotalPosts, AVG(OwnerReputation) AS AvgOwnerReputation, SUM(UpVotes) AS TotalUpVotes, SUM(DownVotes) AS TotalDownVotes FROM TopPosts GROUP BY DiscussionCategory)
// SELECT AR.DiscussionCategory, AR.TotalPosts, AR.AvgOwnerReputation, AR.TotalUpVotes, AR.TotalDownVotes,
//        CASE WHEN AR.TotalPosts > 50 THEN 'Very Active' WHEN AR.TotalPosts BETWEEN 20 AND 50 THEN 'Active' ELSE 'Less Active' END AS ActivityLevel
// FROM AggregateResults AR ORDER BY AR.TotalPosts DESC;
fn q23542(db: &'static So) -> String {
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let cat = |n: i64| if n > 10 { "Highly Discussed" } else if (5..=10).contains(&n) { "Moderately Discussed" } else { "Less Discussed" };
    let tp = rel(drain((&ps).and((&db.post.owner_user).select(&db.user.reputation).opt())).into_iter().map(|(_, (a, r))| (cat(a[0]), a, r.unwrap_or(0))).collect());
    type T = (&'static str, [i64; 3], i64);
    let g = (&tp).group_by(Same::<T>::new().map(|(c, _, _): T| c)).select(Same::<T>::new()).fold([0i64; 4], |s, (_, a, r)| [s[0] + 1, s[1] + r, s[2] + a[1], s[3] + a[2]]);
    let mut v = drain(&g);
    v.sort_by_key(|&(_, s)| Reverse(s[0]));
    rows(v.into_iter().map(|(c, s)| {
        row(vec![V::S(c), V::I(s[0]), avg(s[1], s[0]), V::I(s[2]), V::I(s[3]), V::S(if s[0] > 50 { "Very Active" } else if s[0] >= 20 { "Active" } else { "Less Active" })])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 10),
// TopPostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, mu.UserId, mu.PostCount, mu.TotalViews, mu.TotalComments
//     FROM RankedPosts rp JOIN MostActiveUsers mu ON rp.OwnerDisplayName = mu.DisplayName WHERE rp.Rank <= 5)
// SELECT tpd.PostId, tpd.Title, tpd.CreationDate, tpd.Score, tpd.ViewCount, tpd.OwnerDisplayName, tpd.PostCount AS ActiveUserPostCount, tpd.TotalViews AS ActiveUserTotalViews,
//        tpd.TotalComments AS ActiveUserTotalComments FROM TopPostDetails tpd ORDER BY tpd.Score DESC, tpd.CreationDate DESC;
//
// MostActiveUsers is joined by display name, so a post matches every active user of its owner's name.
fn q6226(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let mu = db.user.with((&pc).filt(|n| n > 10)).group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt().and((&cc).opt()))).fold([0i64; 3], |a, (w, c)| {
        [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + c.unwrap_or(0)]
    });
    let by_name: HashIdx<Str, Id<User>> = db.user.with(&mu).select(&db.user.display_name).inv().collect();
    let mut v = drain((&tp).select(owner_user.select(&db.user.display_name).select((&by_name).select((&pc).and(&mu)))));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (n, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(n), nullable(a[1], a[0]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// RecentActivity AS (SELECT p.Id AS PostId, MAX(ph.CreationDate) AS LastEditedDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY p.Id),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes, ra.LastEditedDate, COALESCE(rp.UserPostRank, 0) AS UserPostRank,
//        (CASE WHEN rp.UpVotes > 0 THEN 'Upvoted' WHEN rp.CommentCount > 5 THEN 'Popular' ELSE 'New' END) AS PostStatus FROM RankedPosts rp LEFT JOIN RecentActivity ra ON rp.PostId = ra.PostId)
// SELECT pm.PostId, pm.Title, pm.CreationDate, pm.Score, pm.ViewCount, pm.CommentCount, pm.UpVotes, pm.LastEditedDate, pm.UserPostRank, pm.PostStatus
// FROM PostMetrics pm WHERE pm.Score > 0 OR pm.CommentCount > 0 ORDER BY pm.Score DESC, pm.ViewCount DESC LIMIT 100;
fn q882(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let qs = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)));
    let v = per_group(ranked(drain(qs().select(owner_user.opt())), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rk = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rk: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let uv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select(&db.vote.vote_type_id);
    let s = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(uv.opt())).fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))).select(&db.post_history.creation_date);
    let ra = db.post.group_by(Ident::<Post>::new()).select(edits).fold(i64::MIN, |m, d| m.max(d));
    let q = (&s).and(score).filt(|(a, sc): ([i64; 2], i64)| sc > 0 || a[0] > 0).and((&ra).opt()).and((&rk).map(|(_, r)| r));
    let v = top_n(drain(q), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p), 100);
    rows(v.into_iter().map(|(p, (((a, _), e), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), ots(e), V::I(r), V::S(if a[1] > 0 { "Upvoted" } else if a[0] > 5 { "Popular" } else { "New" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY YEAR(p.CreationDate) ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserVoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostHistoryDetails AS (SELECT ph.PostId, ph.UserId, ph.CreationDate AS HistoryDate, ph.Comment, p.Title FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (10, 11)),
// PostStats AS (SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.Score, COALESCE(uv.UpVotes, 0) AS UpVotes, COALESCE(uv.DownVotes, 0) AS DownVotes,
//        COALESCE(SUM(CASE WHEN ph.HistoryDate IS NOT NULL THEN 1 END), 0) AS HistoryCount
//     FROM RankedPosts rp LEFT JOIN UserVoteCounts uv ON rp.Id = uv.PostId LEFT JOIN PostHistoryDetails ph ON rp.Id = ph.PostId GROUP BY rp.Id, rp.Title, rp.CreationDate, rp.Score, uv.UpVotes, uv.DownVotes)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.UpVotes, ps.DownVotes, ps.HistoryCount,
//        CASE WHEN ps.Score > 50 THEN 'High Score' WHEN ps.Score BETWEEN 20 AND 50 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostStats ps WHERE ps.HistoryCount > 0 ORDER BY ps.Score DESC, ps.CreationDate ASC OFFSET 10 ROWS FETCH NEXT 5 ROWS ONLY;
//
// PostRank is never read.
fn q1033(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let hc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(closes).fold(0i64, |n, _| n + 1);
    let v = top_n(drain(&hc), |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 15);
    let tp = rel(v);
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let uv = (&tps).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let mut v = drain((&tp).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _): (Id<Post>, i64)| p).select(&uv))));
    v.sort_by_key(|&(i, _)| i);
    rows(v.into_iter().skip(10).map(|(_, ((p, h), a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(h), V::S(if s > 50 { "High Score" } else if (20..=50).contains(&s) { "Medium Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount FROM RankedPosts rp WHERE rp.rn <= 5),
// VoteSummary AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// PostsWithVotes AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes FROM TopPosts tp LEFT JOIN VoteSummary vs ON tp.PostId = vs.PostId),
// FinalResults AS (SELECT pwv.PostId, pwv.Title, pwv.Score, pwv.ViewCount, pwv.AnswerCount, pwv.UpVotes, pwv.DownVotes,
//        CASE WHEN pwv.UpVotes > pwv.DownVotes THEN 'Positive' WHEN pwv.UpVotes < pwv.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteStatus FROM PostsWithVotes pwv)
// SELECT fr.PostId, fr.Title, fr.Score, fr.ViewCount, fr.AnswerCount, fr.UpVotes, fr.DownVotes, fr.VoteStatus FROM FinalResults fr ORDER BY fr.Score DESC NULLS LAST, fr.ViewCount DESC NULLS LAST LIMIT 20;
fn q2276(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    let v = top_n(drain(&vs), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 20);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH RECURSIVE RecentPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// RankedUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyEarned, DENSE_RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(DISTINCT p.Id) > 0),
// LatestPostHistory AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, MAX(ph.CreationDate) OVER (PARTITION BY ph.PostId) AS LastEditedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6, 10)),
// AggregatedData AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COALESCE(rp.rn, 0) AS RecentPostRank, lph.UserId AS LastEditorId, lph.LastEditedDate, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN RecentPosts rp ON p.Id = rp.Id LEFT JOIN LatestPostHistory lph ON p.Id = lph.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, rp.rn, lph.UserId, lph.LastEditedDate)
// SELECT ru.DisplayName, ru.Reputation, ru.PostCount, ru.TotalBountyEarned, ad.PostId, ad.Title, ad.CreationDate, ad.RecentPostRank, ad.LastEditorId, ad.LastEditedDate, ad.CommentCount
// FROM RankedUsers ru JOIN AggregatedData ad ON ru.Id = ad.OwnerUserId WHERE ru.UserRank <= 10 ORDER BY ru.Reputation DESC, ad.CreationDate DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. UserRank reads only the post count, so the users are picked first and the bounty product and the post groups are built for them alone.
// An AggregatedData group is a post and one of the distinct editor ids of its history rows (NULL for none).
fn q30197(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let r = ranked(drain(&pc), |&(_, n)| Reverse(n), true);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let tb = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt())).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let rp = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.opt()));
    let rp = per_group(ranked(rp, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rp = rel(rp.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rn: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let PostHistory { post_history_type_id, user, creation_date: hd, .. } = &db.post_history;
    let lph = || history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6, 10])));
    let posts: MatSet<Id<Post>> = (&tu).select(posts_of(db)).collect();
    let last = (&posts).group_by(Ident::<Post>::new()).select(lph().select(hd)).fold(i64::MIN, |m, d| m.max(d));
    type J = (Id<Post>, Option<Id<PostHistory>>);
    let j: MatSet<J> = (&posts).select(Ident::<Post>::new().and(lph().opt())).collect();
    let hist = || Same::<J>::new().map(|(_, h): J| h).flat_map(|h: Option<Id<PostHistory>>| h);
    let g = (&j).group_by(Same::<J>::new().map(|(p, _): J| p).and(hist().select(user).opt())).select(Same::<J>::new()).fold(0i64, |n, _| n + 1);
    let cc = (&posts).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type K = (Id<Post>, Option<Id<User>>);
    let mut v = drain((&g).and(Same::<K>::new().map(|(p, _): K| p).select((&cc).and((&rn).map(|(_, r)| r).opt()).and((&last).opt()).and(owner_user.select(Ident::<User>::new().and(&pc).and(&tb))))));
    v.sort_by_key(|&((p, _), (_, (_, ((u, _), _))))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|((p, e), (_, (((c, r), l), ((u, n), b))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(b)]);
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(r.unwrap_or(0)), e.map_or(V::Null, |e| user_col(db, e, "uid")), l.map_or(V::Null, |l| tmax(l)), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(p.AnswerCount, 0) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank,
//        u.Reputation, p.OwnerUserId FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// RecentVotes AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id)
// SELECT rb.PostId, rb.Title, rb.CreationDate, rb.ViewCount, rb.AnswerCount, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, rv.VoteCount, rv.UpVotes, rv.DownVotes,
//        CASE WHEN rb.Rank = 1 THEN 'Most Viewed' WHEN rv.UpVotes > rv.DownVotes THEN 'Popular' ELSE 'Needs More Attention' END AS PostStatus
// FROM RankedPosts rb LEFT JOIN UserBadges ub ON rb.OwnerUserId = ub.UserId LEFT JOIN RecentVotes rv ON rb.PostId = rv.PostId WHERE rb.Reputation > 1000 ORDER BY rb.ViewCount DESC, ub.BadgeCount DESC LIMIT 50;
fn q32198(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, creation_date, owner_user, view_count, answer_count, .. } = &db.post;
    let vk = |p: Id<Post>| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    };
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).select(owner_user));
    let first = top_per(v.clone(), |&(_, u)| u, |&(p, _)| (vk(p), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let rich = drain(rel(v).select(Same::<(Id<Post>, Id<User>)>::new().and(Same::<(Id<Post>, Id<User>)>::new().map(|(_, u): (Id<Post>, Id<User>)| u).select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and(&ub)))));
    let v = top_n(rich, |&(_, ((p, _), (_, b)))| (vk(p), Reverse(b[0]), p), 50);
    let tp = rel(v.into_iter().map(|(_, ((p, _), (_, b)))| (p, b)).collect());
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let rv = (&tps).with(creation_date.ge(add_days(t0, -30))).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    type T = (Id<Post>, [i64; 4]);
    let mut v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select((&rv).opt().and(Ident::<Post>::new().with(&first).opt())))));
    v.sort_by_key(|&(i, _)| i);
    rows(v.into_iter().map(|(_, ((p, b), (r, t)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.push(V::I(answer_count.get(p).unwrap_or(0)));
        f.extend(b.map(V::I));
        f.extend(match r {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if t.is_some() { "Most Viewed" } else if r.map_or(false, |a| a[1] > a[2]) { "Popular" } else { "Needs More Attention" }));
        row(f)
    }))
}

// Rewritten (rewrites/2377.sql): the StatusRank window is refined with `, UserId`.
// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, COALESCE(SUM(p.Score), 0) AS TotalScore, COALESCE(SUM(p.ViewCount), 0) AS TotalViews, MAX(p.CreationDate) AS MostRecentPostDate
//     FROM Posts p GROUP BY p.OwnerUserId),
// CombinedStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ubc.BadgeCount, 0) AS BadgeCount, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.TotalScore, 0) AS TotalScore,
//        COALESCE(ps.TotalViews, 0) AS TotalViews, ps.MostRecentPostDate FROM Users u LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId)
// SELECT *, CASE WHEN BadgeCount > 0 AND PostCount > 0 THEN 'Active' WHEN BadgeCount > 0 AND PostCount = 0 THEN 'Badge Holder, No Posts' WHEN BadgeCount = 0 AND PostCount > 0 THEN 'Inactive Badge Seeker'
//        ELSE 'New User' END AS UserStatus,
//        ROW_NUMBER() OVER (PARTITION BY CASE WHEN BadgeCount > 0 AND PostCount > 0 THEN 'Active' WHEN BadgeCount > 0 AND PostCount = 0 THEN 'Badge Holder, No Posts'
//        WHEN BadgeCount = 0 AND PostCount > 0 THEN 'Inactive Badge Seeker' ELSE 'New User' END ORDER BY TotalScore DESC, UserId) AS StatusRank
// FROM CombinedStats WHERE UserId IS NOT NULL ORDER BY UserStatus, StatusRank LIMIT 100;
fn q2377(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, score, view_count, creation_date, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt()).and(creation_date)).fold([0, 0, 0, i64::MIN], |a, ((s, w), d)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3].max(d)]);
    let status = |b: i64, p: i64| if b > 0 && p > 0 { "Active" } else if b > 0 { "Badge Holder, No Posts" } else if p > 0 { "Inactive Badge Seeker" } else { "New User" };
    let v = drain((&bc).and((&ps).opt()));
    let v = per_group(ranked(v, |&(u, (b, p))| (status(b, p.map_or(0, |a| a[0])), Reverse(p.map_or(0, |a| a[1])), u), false), |&(_, (b, p))| status(b, p.map_or(0, |a| a[0])));
    let v = top_n(v, |&((_, (b, p)), r)| (status(b, p.map_or(0, |a| a[0])), r), 100);
    rows(v.into_iter().map(|((u, (b, p)), r)| {
        let a = p.unwrap_or([0, 0, 0, i64::MIN]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), tmax(a[3]), V::S(status(b, a[0])), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentlyCreatedPostRank FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesCount,
//        COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS ClosedCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT up.DisplayName, rp.Title, rp.ViewCount, rp.Score, COALESCE(cp.ClosedCount, 0) AS ClosedCount, up.UpVotesCount, up.DownVotesCount, up.GoldBadges, up.SilverBadges, up.BronzeBadges
// FROM RankedPosts rp JOIN UserActivity up ON rp.Id IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = up.UserId) LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
// WHERE rp.RankScore <= 10 AND (up.UpVotesCount - up.DownVotesCount) > 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// The IN subquery pairs each post with its owner. RankScore reads only base columns, so the top posts are picked first and the votes x badges product is driven for their owners alone.
fn q2543(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ua = (&owners).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 5], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
    });
    let cp = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(cp.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let mut v = drain((&cc).and(owner_user.select(Ident::<User>::new().and((&ua).filt(|a| a[0] - a[1] > 5)))));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p));
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.push(V::I(c));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RecursivePosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(p.Id) AS TotalQuestions, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 GROUP BY u.Id, u.Reputation),
// VoteStatistics AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes FROM Votes GROUP BY PostId),
// PostHistoryStats AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEditDate, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseOpenCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT u.DisplayName, u.Reputation, ur.TotalQuestions, ur.AcceptedAnswers, COALESCE(vs.TotalUpvotes, 0) AS TotalUpvotes, COALESCE(vs.TotalDownvotes, 0) AS TotalDownvotes, pp.PostId, pp.Title,
//        pp.CreationDate, pp.Score, ph.LastEditDate, ph.CloseOpenCount
// FROM Users u JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN RecursivePosts pp ON u.Id = pp.OwnerUserId AND pp.PostRank = 1 LEFT JOIN VoteStatistics vs ON pp.PostId = vs.PostId
// LEFT JOIN PostHistoryStats ph ON pp.PostId = ph.PostId WHERE u.Reputation > 1000 AND pp.Score > 0 ORDER BY u.Reputation DESC, pp.CreationDate DESC LIMIT 50;
fn q34276(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, accepted_answer, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = rel(first);
    let rich = drain((&first).with(Same::<(Id<Post>, Id<User>)>::new().map(|(p, _): (Id<Post>, Id<User>)| p).select(score.gt(0))).with(Same::<(Id<Post>, Id<User>)>::new().map(|(_, u): (Id<Post>, Id<User>)| u).select((&db.user.reputation).gt(1000))));
    let v = top_n(rich, |&(_, (p, u))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 50);
    let tp = rel(v.into_iter().map(|x| x.1).collect());
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let us: MatSet<Id<User>> = (&tp).map(|(_, u)| u).collect();
    let ur = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(accepted_answer.opt())).fold([0i64; 2], |a, x| [a[0] + 1, a[1] + x.is_some() as i64]);
    let vs = (&tps).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = (&tps).group_by(Ident::<Post>::new()).select(history_of(db).select(hd.and(post_history_type_id))).fold((i64::MIN, 0i64), |(m, n), (d, t)| (m.max(d), n + matches!(t, 10 | 11) as i64));
    type T = (Id<Post>, Id<User>);
    let mut v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(_, u): T| u).select(&ur)).and(Same::<T>::new().map(|(p, _): T| p).select((&vs).and((&ph).opt())))));
    v.sort_by_key(|&(i, _)| i);
    rows(v.into_iter().map(|(_, (((p, u), a), (w, h)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(w[0]), V::I(w[1])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend(match h {
            Some((m, n)) => [V::T(m), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.ViewCount DESC) AS Rank, COUNT(c.Id) AS CommentCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName, pt.Name),
// TopPosts AS (SELECT r.Id, r.Title, r.CreationDate, r.ViewCount, r.OwnerDisplayName, r.Rank, r.CommentCount FROM RankedPosts r WHERE r.Rank <= 5),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 years' GROUP BY u.Id, u.DisplayName)
// SELECT tp.Title, tp.OwnerDisplayName, tp.ViewCount, tp.CommentCount, ue.DisplayName AS EngagedUser, ue.TotalVotes, ue.GoldBadges, ue.SilverBadges, ue.BronzeBadges
// FROM TopPosts tp JOIN UserEngagement ue ON tp.ViewCount > 5000 AND ue.TotalVotes > 10 ORDER BY tp.ViewCount DESC, ue.TotalVotes DESC;
//
// The ON clause names each side separately, so the busy top posts are crossed with the engaged users.
fn q7339(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tc = (&tp).with(view_count.gt(5000)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ue = db
        .user
        .with((&db.user.creation_date).lt(add_years(t0, -2)))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (v, c)| [a[0] + v.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]);
    let ue = rel(drain((&ue).filt(|a| a[0] > 10)));
    let mut v = drain((&tc).cross(&ue));
    v.sort_by_key(|&((p, _), (_, (_, a)))| (Reverse(view_count.get(p)), Reverse(a[0])));
    rows(v.into_iter().map(|((p, _), (c, (u, a)))| {
        let mut f = post_fields(db, p, &["title", "owner", "views"]);
        f.extend([V::I(c), user_col(db, u, "name")]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY p.ViewCount DESC) AS ViewCountRank FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS EditCount, SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TitleEditCount, SUM(CASE WHEN ph.PostHistoryTypeId IN (6) THEN 1 ELSE 0 END) AS TagEditCount
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, us.UserId, us.DisplayName, us.QuestionCount, us.UpVotes, us.DownVotes, phc.EditCount, phc.TitleEditCount, phc.TagEditCount
// FROM RankedPosts rp JOIN Users u ON EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = u.Id AND p.Id = rp.PostId) JOIN UserStats us ON us.UserId = u.Id LEFT JOIN PostHistoryCounts phc ON phc.PostId = rp.PostId
// WHERE rp.ScoreRank <= 10 AND rp.ViewCountRank <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// The EXISTS pairs each post with its owner.
fn q9180(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))));
    let v = ranked(v, |&(p, _)| Reverse(score.get(p).unwrap()), false);
    let v = ranked(v, |&((p, _), _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().filter(|&((_, s), w)| s <= 10 && w <= 10).map(|x| x.0 .0 .0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let qc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let phc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5) as i64, a[2] + (t == 6) as i64]);
    let mut v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&qc).and(&us)).and((&phc).opt())));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(p, (((u, n), a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => h.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY SUM(COALESCE(v.BountyAmount, 0)) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(MAX(ph.CreationDate), '1970-01-01') AS LastHistoryChange
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopUsers AS (SELECT ua.*, PERCENT_RANK() OVER (ORDER BY TotalBounty DESC) AS BountyRank FROM UserActivity ua WHERE ua.Rank = 1)
// SELECT u.UserId, u.DisplayName, u.QuestionCount, u.AnswerCount, u.TotalBounty, p.PostId, p.Title, p.CreationDate AS PostCreationDate, p.Score, p.ViewCount, p.CommentCount,
//        CASE WHEN p.LastHistoryChange = '1970-01-01' THEN 'No Changes' ELSE 'Recently Edited' END AS PostStatus, COALESCE(t.BountyRank, 0) AS UserBountyRank
// FROM TopUsers u JOIN PostStats p ON u.UserId = p.PostId LEFT JOIN TopUsers t ON u.UserId = t.UserId ORDER BY u.TotalBounty DESC, p.Score DESC;
//
// `u.UserId = p.PostId` joins a user id to a post id, so it goes through the raw ids. Rank partitions by the user, so it is 1 for every row.
fn q4843(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(bounty.opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let n = count(&ua);
    let pr = ranked(drain(&ua), |&(_, a)| Reverse(a[2]), false);
    let pr = rel(pr.into_iter().map(|((u, a), r)| (u, (a, if n > 1 { (r - 1) as f64 / (n - 1) as f64 } else { 0.0 }))).collect());
    let pr: HashIdx<Id<User>, (Id<User>, ([i64; 3], f64))> = (&pr).map(|(u, _)| u).inv().select(&pr).collect();
    let pid: HashIdx<i64, Id<Post>> = db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1))).select(&db.post.origid).inv().collect();
    let mp: MatSet<Id<Post>> = (&pr).map(|(u, _)| u).select((&db.user.origid).select(&pid)).collect();
    let ps = (&mp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt())).fold((0i64, i64::MIN), |(n, m), (c, d)| {
        (n + c.is_some() as i64, d.map_or(m, |d| m.max(d)))
    });
    let mut v = drain((&pr).map(|(_, x)| x).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&ps))));
    v.sort_by_key(|&(_, ((a, _), (p, _)))| (Reverse(a[2]), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(u, ((a, r), (p, (c, m))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[0]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::S(if m == i64::MIN { "No Changes" } else { "Recently Edited" }), V::F(r)]);
        row(f)
    }))
}

// WITH PostVoteCounts AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId),
// PostMetadata AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(ph.Comment, 'No comments') AS LastEditComment, ROW_NUMBER() OVER(PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RN,
//        ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC ) AS UserPostRN
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (4, 5)
//     WHERE p.CreationDate >= '2023-01-01' AND EXISTS (SELECT 1 FROM PostLinks pl WHERE pl.PostId = p.Id AND pl.LinkTypeId = 1))
// SELECT pm.PostId, pm.Title, pm.Score, pm.ViewCount, pvc.UpVoteCount, pvc.DownVoteCount, CASE WHEN pm.RN = 1 THEN 'Latest Post in Type' ELSE 'Older Post in Type' END AS PostStatus,
//        CASE WHEN pm.UserPostRN = 1 THEN 'Most Active User Post' ELSE 'Other User Post' END AS UserPostStatus,
//        CASE WHEN pvc.TotalVotes IS NULL THEN 'No Votes Recorded' ELSE CASE WHEN pvc.UpVoteCount > pvc.DownVoteCount THEN 'Positive Feedback' ELSE 'Mixed or Negative Feedback' END END AS Feedback
// FROM PostMetadata pm LEFT JOIN PostVoteCounts pvc ON pm.PostId = pvc.PostId WHERE pm.RN <= 5 ORDER BY pm.ViewCount DESC, pm.Score DESC;
//
// Both windows number the post x edit-history rows, which are materialised to be ranked; copies of one post tie and are ordered by the history id.
fn q21042(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, last_activity_date, score, view_count, .. } = &db.post;
    let linked: MatSet<Id<Post>> = db.post_link.with((&db.post_link.link_type_id).eq(1)).select(&db.post_link.post).collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5])));
    type J = (Id<Post>, Option<Id<PostHistory>>);
    let j = rel(drain(db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).with(&linked).select(Ident::<Post>::new().and(edits.opt()))).into_iter().map(|x| x.1).collect::<Vec<J>>());
    let v = drain((&j).select(Same::<J>::new().and(Same::<J>::new().map(|(p, _): J| p).select(post_type_id.and(owner_user.opt())))));
    let v = per_group(ranked(v, |&(_, ((p, h), (t, _)))| (t, Reverse(creation_date.get(p).unwrap()), p, h), false), |&(_, (_, (t, _)))| t);
    let v = per_group(ranked(v, |&((_, ((p, h), (_, u))), _)| (u, Reverse(last_activity_date.get(p).unwrap()), p, h), false), |&((_, (_, (_, u))), _)| u);
    let v = rel(v.into_iter().map(|(((_, ((p, _), _)), rn), un)| (p, (rn, un))).collect());
    type R = (Id<Post>, (i64, i64));
    let pvc = db.vote.with(&db.vote.post).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let mut v = drain((&v).filt(|(_, (rn, _)): R| rn <= 5).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&pvc).opt()))));
    v.sort_by_key(|&(_, ((p, _), _))| (Reverse(view_count.get(p)), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((p, (rn, un)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(match c {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if rn == 1 { "Latest Post in Type" } else { "Older Post in Type" }));
        f.push(V::S(if un == 1 { "Most Active User Post" } else { "Other User Post" }));
        f.push(V::S(match c {
            None => "No Votes Recorded",
            Some(a) if a[0] > a[1] => "Positive Feedback",
            _ => "Mixed or Negative Feedback",
        }));
        row(f)
    }))
}

// WITH UserParticipation AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostsCreated, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopContributors AS (SELECT UserId, DisplayName, PostsCreated, UpVotesReceived, DownVotesReceived, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY PostsCreated DESC) AS PostRank,
//        RANK() OVER (ORDER BY UpVotesReceived DESC) AS UpVoteRank, RANK() OVER (ORDER BY DownVotesReceived DESC) AS DownVoteRank FROM UserParticipation),
// FinalRanking AS (SELECT UserId, DisplayName, PostsCreated, UpVotesReceived, DownVotesReceived, GoldBadges, SilverBadges, BronzeBadges, PostRank, UpVoteRank, DownVoteRank,
//        ROW_NUMBER() OVER (ORDER BY PostsCreated DESC, UpVotesReceived DESC) AS FinalRank FROM TopContributors)
// SELECT UserId, DisplayName, PostsCreated, UpVotesReceived, DownVotesReceived, GoldBadges, SilverBadges, BronzeBadges, FinalRank FROM FinalRanking WHERE FinalRank <= 20 ORDER BY FinalRank;
//
// FinalRank leads with the distinct post count, so only users with at least the twentieth-highest count can reach rank 20; the posts x votes x badges product is driven for
// those alone, and every other user ranks below all of them.
fn q28843(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let twentieth = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 20).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&pc).filt(move |n| n >= twentieth)).collect();
    let up = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (v, c)| {
            let t = v.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
        });
    let v = top_n(drain((&up).and(&pc)), |&(u, (a, n))| (Reverse(n), Reverse(a[0]), u), 20);
    rows(v.into_iter().enumerate().map(|(i, (u, (a, n)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("22357", q22357),
    ("3478", q3478),
    ("3540", q3540),
    ("877", q877),
    ("25404", q25404),
    ("1120", q1120),
    ("25670", q25670),
    ("438", q438),
    ("1573", q1573),
    ("553", q553),
    ("5939", q5939),
    ("6009", q6009),
    ("24146", q24146),
    ("101", q101),
    ("7360", q7360),
    ("2039", q2039),
    ("4658", q4658),
    ("826", q826),
    ("31438", q31438),
    ("2872", q2872),
    ("20895", q20895),
    ("31486", q31486),
    ("21260", q21260),
    ("25168", q25168),
    ("4167", q4167),
    ("28356", q28356),
    ("4934", q4934),
    ("37", q37),
    ("3529", q3529),
    ("24773", q24773),
    ("30053", q30053),
    ("31057", q31057),
    ("9801", q9801),
    ("2856", q2856),
    ("23590", q23590),
    ("23784", q23784),
    ("23400", q23400),
    ("34215", q34215),
    ("24714", q24714),
    ("2284", q2284),
    ("6235", q6235),
    ("7305", q7305),
    ("2653", q2653),
    ("29519", q29519),
    ("5991", q5991),
    ("25691", q25691),
    ("6894", q6894),
    ("3461", q3461),
    ("21505", q21505),
    ("429", q429),
    ("4172", q4172),
    ("25172", q25172),
    ("321", q321),
    ("1992", q1992),
    ("7552", q7552),
    ("33196", q33196),
    ("1402", q1402),
    ("1648", q1648),
    ("23219", q23219),
    ("32498", q32498),
    ("1153", q1153),
    ("1845", q1845),
    ("4143", q4143),
    ("30384", q30384),
    ("31673", q31673),
    ("26526", q26526),
    ("33887", q33887),
    ("542", q542),
    ("3805", q3805),
    ("20430", q20430),
    ("21403", q21403),
    ("32768", q32768),
    ("1292", q1292),
    ("3777", q3777),
    ("24823", q24823),
    ("24758", q24758),
    ("2090", q2090),
    ("3969", q3969),
    ("21314", q21314),
    ("9991", q9991),
    ("1990", q1990),
    ("518", q518),
    ("3560", q3560),
    ("4326", q4326),
    ("8692", q8692),
    ("2258", q2258),
    ("23542", q23542),
    ("6226", q6226),
    ("882", q882),
    ("1033", q1033),
    ("2276", q2276),
    ("30197", q30197),
    ("32198", q32198),
    ("2377", q2377),
    ("2543", q2543),
    ("34276", q34276),
    ("7339", q7339),
    ("9180", q9180),
    ("4843", q4843),
    ("21042", q21042),
    ("28843", q28843),
];
