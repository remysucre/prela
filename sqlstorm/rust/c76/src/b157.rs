use harness::prelude::*;
use std::cmp::Reverse;

// WITH RecursivePostContributions AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate >= '2022-01-01' AND p.Score > 0 GROUP BY p.Id, p.OwnerUserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(r.CommentCount), 0) AS TotalComments, COALESCE(SUM(r.UpVotes), 0) AS TotalUpVotes,
//        COALESCE(SUM(r.DownVotes), 0) AS TotalDownVotes, ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(r.UpVotes), 0) DESC) AS RankByUpVotes
//     FROM Users u LEFT JOIN RecursivePostContributions r ON u.Id = r.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT up.UserId, up.DisplayName, up.TotalComments, up.TotalUpVotes, up.TotalDownVotes, (up.TotalUpVotes - up.TotalDownVotes) AS NetVotes,
//        CASE WHEN up.RankByUpVotes <= 10 THEN 'Top Contributor' WHEN up.TotalUpVotes > 100 THEN 'Active User' ELSE 'Regular User' END AS UserTier
// FROM UserPerformance up WHERE up.TotalUpVotes > 0 ORDER BY up.TotalUpVotes DESC;
//
// Summing each post's counts over the user's posts is the same as folding the user's posts x comments x votes rows.
fn q32936(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let rpc = Ident::<Post>::new().with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0)).and(score.gt(0)));
    let up = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(rpc).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((c, t)) => [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let v = top_n(drain(&up), |&(_, a)| Reverse(a[1]), 0);
    let v = drain(rel(v.into_iter().enumerate().map(|(i, (u, a))| (u, a, i as i64 + 1)).collect()).filt(|(_, a, _)| a[1] > 0));
    rows(v.into_iter().map(|(_, (u, a, rn))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])]);
        f.push(V::S(if rn <= 10 { "Top Contributor" } else if a[1] > 100 { "Active User" } else { "Regular User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(a.Id) AS AnswerCount, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 10),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Location, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Location)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.AnswerCount, tp.CommentCount, ur.DisplayName AS OwnerName, ur.Reputation AS OwnerReputation, ur.Location AS OwnerLocation
// FROM TopPosts tp JOIN UserReputation ur ON tp.PostId = ur.UserId ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// The join compares a post id with a user id, so it goes through origid. UserReputation has one row per user, so its aggregates are not read.
// Rank reads only base columns, so each owner's top ten questions are picked first and the answer x comment product is driven for those alone.
fn q6049(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (x, c)| [a[0] + x.is_some() as i64, a[1] + c.is_some() as i64]);
    let v = drain((&s).and(origid.select(&uidx)));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(ostr(db.user.location.get(u)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.OwnerDisplayName, pd.CommentCount, pd.UpVotes, pd.DownVotes, (pd.UpVotes - pd.DownVotes) AS NetScore
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
fn q6925(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.gt(date(2023, 10, 1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostVoteSummary AS (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(v.Id) AS TotalVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// FinalReport AS (SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, pvs.UpVotes, pvs.DownVotes, pvs.TotalVotes
//     FROM TopPosts tp JOIN PostVoteSummary pvs ON tp.Id = pvs.PostId)
// SELECT fr.Title, fr.OwnerDisplayName, fr.CreationDate, fr.Score, fr.ViewCount, fr.AnswerCount, fr.CommentCount, fr.UpVotes, fr.DownVotes, fr.TotalVotes
// FROM FinalReport fr ORDER BY fr.Score DESC, fr.ViewCount DESC;
fn q7785(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views", "answers", "comments"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT ph.PostId) AS TotalEdits,
//        SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS TitleBodyTagEdits FROM Users u JOIN PostHistory ph ON u.Id = ph.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TagStatistics AS (SELECT pt.Tag, COUNT(pt.PostId) AS PostCount, COUNT(DISTINCT pt.PostId) FILTER (WHERE ph.PostHistoryTypeId = 10) AS ClosedPostCount, AVG(u.Reputation) AS AvgUserReputation
//     FROM PostTags pt LEFT JOIN Posts p ON pt.PostId = p.Id LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Users u ON ph.UserId = u.Id GROUP BY pt.Tag),
// FinalBenchmark AS (SELECT ts.Tag, ts.PostCount, ts.ClosedPostCount, ts.AvgUserReputation,
//        CASE WHEN ts.PostCount > 0 THEN (CAST(ts.ClosedPostCount AS FLOAT) / ts.PostCount) * 100 ELSE 0 END AS ClosedPostPercentage FROM TagStatistics ts)
// SELECT fb.Tag, fb.PostCount, fb.ClosedPostCount, fb.AvgUserReputation, fb.ClosedPostPercentage FROM FinalBenchmark fb ORDER BY fb.ClosedPostPercentage DESC LIMIT 10;
//
// UserReputation is never read. FLOAT is f32, so the percentage is computed in f32.
fn q27897(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let PostHistory { post_history_type_id, user, .. } = &db.post_history;
    type R = (Id<Post>, Str);
    let pt = db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list)));
    let hist = history_of(db).select(post_history_type_id.and(user.select(&db.user.reputation).opt()));
    let g = pt
        .group_by(Same::<R>::new().map(|(_, t): R| t))
        .select(Same::<R>::new().map(|(p, _): R| p).and(Same::<R>::new().map(|(p, _): R| p).select(hist.opt())))
        .buf_fold(|v| {
            let (mut n, mut rn, mut rs) = (0i64, 0i64, 0i64);
            let mut closed: Vec<Id<Post>> = Vec::new();
            for &(p, h) in v.iter() {
                n += 1;
                if let Some((t, r)) = h {
                    if t == 10 {
                        closed.push(p);
                    }
                    if let Some(r) = r {
                        rn += 1;
                        rs += r;
                    }
                }
            }
            closed.sort_unstable();
            closed.dedup();
            [n, closed.len() as i64, rn, rs]
        });
    let pct = |a: [i64; 4]| (a[1] as f32 / a[0] as f32) * 100.0;
    let v = top_n(drain(&g), |&(_, a)| Reverse(fkey(pct(a) as f64)), 10);
    rows(v.into_iter().map(|(t, a)| row(vec![V::S(t), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::F(pct(a) as f64)])))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(P.ViewCount) AS TotalViews, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalViews, TotalBounty, RANK() OVER (ORDER BY TotalBounty DESC) AS BountyRank,
//        DENSE_RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats),
// ClosedPostReasons AS (SELECT PH.UserId, COUNT(*) AS TotalClosedPosts FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId)
// SELECT TU.DisplayName, TU.TotalPosts, TU.Questions, TU.Answers, TU.TotalViews, TU.TotalBounty, COALESCE(CPR.TotalClosedPosts, 0) AS TotalClosedPosts,
//        CASE WHEN TU.BountyRank = 1 THEN 'Gold' WHEN TU.PostRank = 1 THEN 'Diamond' ELSE 'Regular' END AS UserTier
// FROM TopUsers TU LEFT JOIN ClosedPostReasons CPR ON TU.UserId = CPR.UserId WHERE TU.TotalPosts > 0 ORDER BY TU.TotalBounty DESC, TU.TotalViews DESC;
fn q2582(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(bounty.opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some(((t, w), b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + b.unwrap_or(0)]
            }
            None => a,
        });
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cpr = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&ups).and((&cpr).opt())), |&(_, (a, _))| Reverse(a[5]), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[0]), true);
    let v = drain(rel(v).filt(|(((_, (a, _)), _), _)| a[0] > 0));
    rows(v.into_iter().map(|(_, x)| x).map(|(((u, (a, c)), br), pr)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5]), V::I(c.unwrap_or(0))];
        f.push(V::S(if br == 1 { "Gold" } else if pr == 1 { "Diamond" } else { "Regular" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, b.Name AS BadgeName, b.Class AS BadgeClass, b.Date AS BadgeDate,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY b.Class, b.Date DESC) AS BadgeRank FROM Users u JOIN Badges b ON u.Id = b.UserId),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, MAX(p.CreationDate) AS LastActivity, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.OwnerUserId),
// BadgedUsers AS (SELECT ub.UserId, ub.DisplayName, COUNT(DISTINCT ub.BadgeName) AS TotalBadges, MAX(ub.BadgeClass) AS HighestBadgeClass FROM UserBadges ub
//     WHERE ub.BadgeRank = 1 GROUP BY ub.UserId, ub.DisplayName)
// SELECT u.DisplayName, bu.TotalBadges, bu.HighestBadgeClass, ps.PostId, ps.Title, ps.CommentCount, ps.VoteCount, ps.UpVotes, ps.DownVotes, ps.LastActivity
// FROM BadgedUsers bu JOIN Users u ON bu.UserId = u.Id JOIN PostStatistics ps ON u.Id = ps.OwnerUserId WHERE bu.TotalBadges > 0
// ORDER BY bu.TotalBadges DESC, ps.LastActivity DESC LIMIT 10;
//
// BadgeRank ties can only pick between badges of one class, and the name count of one badge is 1 either way.
// LastActivity is the post's own CreationDate, so the ten posts are picked first and the comment x vote product is driven for those alone.
fn q27409(db: &'static So) -> String {
    let Badge { user, class, date: bd, name, .. } = &db.badge;
    let first = top_per(drain(db.badge.select(user)), |&(_, u)| u, |&(b, _)| (class.get(b).unwrap(), Reverse(bd.get(b).unwrap()), b), 1, false);
    let fb: MatSet<Id<Badge>> = rel(first.into_iter().map(|x| x.0).collect()).map(|b| b).collect();
    let names = (&fb).group_by(user).select(name).count_distinct();
    let hi = (&fb).group_by(user).select(class).fold(i64::MIN, |m, c| m.max(c));
    let bu = (&names).filt(|n| n > 0).and(&hi);
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.select(owner_user.select(Ident::<User>::new().and(&bu))));
    let v = top_n(v, |&(p, (_, (n, _)))| (Reverse(n), Reverse(creation_date.get(p).unwrap())), 10);
    let tp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().and(&db.vote.vote_type_id)).opt()))
        .buf_fold(|v| {
            let mut ids: Vec<Id<Vote>> = v.iter().filter_map(|&(_, x)| x.map(|x| x.0)).collect();
            ids.sort_unstable();
            ids.dedup();
            let c = v.iter().filter(|x| x.0.is_some()).count() as i64;
            let up = v.iter().filter(|x| x.1.map(|x| x.1) == Some(2)).count() as i64;
            let dn = v.iter().filter(|x| x.1.map(|x| x.1) == Some(3)).count() as i64;
            [c, ids.len() as i64, up, dn]
        });
    let mut out = drain((&ps).and(owner_user.select(Ident::<User>::new().and(&bu))));
    out.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(out.into_iter().map(|(p, (a, (u, (n, h))))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(h)];
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend(a.map(V::I));
        f.push(V::T(creation_date.get(p).unwrap()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AcceptedAnswerId, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AcceptedAnswerId, p.OwnerUserId),
// TopComments AS (SELECT c.PostId, c.UserId, c.Text, c.CreationDate, ROW_NUMBER() OVER (PARTITION BY c.PostId ORDER BY c.Score DESC) AS CommentRank FROM Comments c),
// PostVoteStats AS (SELECT postId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY postId),
// PopularPosts AS (SELECT rp.PostId, rp.Title, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes, rp.CommentCount
//     FROM RankedPosts rp LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.postId WHERE rp.rn <= 5)
// SELECT pp.PostId, pp.Title, pp.UpVotes, pp.DownVotes, COALESCE(tc.Text, 'No comments') AS TopComment, pp.CommentCount
// FROM PopularPosts pp LEFT JOIN TopComments tc ON pp.PostId = tc.PostId AND tc.CommentRank = 1 ORDER BY pp.UpVotes DESC, pp.CommentCount DESC LIMIT 10;
fn q663(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&cc).and((&pvs).opt()));
    let v = top_n(v, |&(_, (c, u))| (Reverse(u.map_or(0, |a| a[0])), Reverse(c)), 10);
    let t10: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let score = &db.comment.score;
    let tc = top_per(drain((&t10).select(comments_of(db))), |&(p, _)| p, |&(p, c)| (Reverse(score.get(c).unwrap()), c, p), 1, false);
    let tc = rel(tc);
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Comment>)> = (&tc).map(|(p, _)| p).inv().select(&tc).collect();
    let mut out = drain((&t10).select(Ident::<Post>::new().and(&cc).and((&pvs).opt()).and((&by_post).map(|(_, c)| c).opt())));
    out.sort_by_key(|&(_, (((_, c), u), _))| (Reverse(u.map_or(0, |a| a[0])), Reverse(c)));
    rows(out.into_iter().map(|(p, (((_, c), u), t))| {
        let u = u.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(u[0]), V::I(u[1]), V::S(t.map_or("No comments", |c| db.comment.text.get(c).unwrap())), V::I(c)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, MAX(CASE WHEN Class = 1 THEN Name END) AS GoldBadge, MAX(CASE WHEN Class = 2 THEN Name END) AS SilverBadge,
//        MAX(CASE WHEN Class = 3 THEN Name END) AS BronzeBadge FROM Badges GROUP BY UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
//        SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) GROUP BY p.OwnerUserId),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, COALESCE(b.BadgeCount, 0) AS BadgeCount, ps.TotalPosts, ps.Questions, ps.Answers, ps.TotalScore
//     FROM Users u LEFT JOIN UserBadges b ON u.Id = b.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId WHERE u.Reputation > 1000),
// RankedUsers AS (SELECT Id, DisplayName, BadgeCount, TotalPosts, Questions, Answers, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank,
//        DENSE_RANK() OVER (ORDER BY BadgeCount DESC) AS BadgeRank FROM ActiveUsers)
// SELECT *, CASE WHEN BadgeCount > 0 THEN 'Active Contributor' ELSE 'Non-contributor' END AS ContributorStatus FROM RankedUsers WHERE ScoreRank <= 10 ORDER BY ScoreRank, BadgeRank;
fn q833(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let active = || db.user.with((&db.user.reputation).gt(1000));
    let bc = active().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let vt = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let ps = active()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(vt.opt())))
        .fold([0i64; 4], |a, ((t, s), _)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let v = ranked(drain((&bc).and((&ps).opt())), |&(_, (_, p))| p.map_or((true, Reverse(0)), |a| (false, Reverse(a[3]))), false);
    let v = ranked(v, |&((_, (b, _)), _)| Reverse(b), true);
    rows(v.into_iter().filter(|x| x.0 .1 <= 10).map(|(((u, (b, p)), sr), br)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(b));
        f.extend(match p {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([V::I(sr), V::I(br), V::S(if b > 0 { "Active Contributor" } else { "Non-contributor" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(V.UpVotes, 0) AS UpVotes, COALESCE(V.DownVotes, 0) AS DownVotes, COALESCE(C.Count, 0) AS CommentCount,
//        COALESCE(B.BadgeCount, 0) AS BadgeCount, P.OwnerUserId FROM Posts P
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) V ON P.Id = V.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS Count FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON P.OwnerUserId = B.UserId),
// TopPosts AS (SELECT PS.*, R.ReputationRank FROM PostStats PS JOIN UserReputation R ON PS.OwnerUserId = R.UserId WHERE PS.Score > 0)
// SELECT TP.Title, TP.CreationDate, TP.Score, TP.UpVotes, TP.DownVotes, TP.CommentCount, TP.ReputationRank, U.DisplayName
// FROM TopPosts TP JOIN Users U ON TP.OwnerUserId = U.Id WHERE TP.ReputationRank <= 10 ORDER BY TP.Score DESC, TP.CommentCount DESC LIMIT 50;
//
// BadgeCount is never read.
fn q936(db: &'static So) -> String {
    let rr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let rr = rel(rr.into_iter().filter(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let Post { owner_user, score, .. } = &db.post;
    let tp = || db.post.with(score.gt(0)).with(owner_user.select(&by_user));
    let vs = tp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = tp().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain(tp().select(Ident::<Post>::new().and((&vs).opt()).and((&cc).opt()).and(owner_user.select(&by_user))));
    let v = top_n(v, |&(p, (((_, _), c), _))| (Reverse(score.get(p).unwrap()), Reverse(c.unwrap_or(0))), 50);
    rows(v.into_iter().map(|(p, (((_, u), c), (w, r)))| {
        let u = u.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(u[0]), V::I(u[1]), V::I(c.unwrap_or(0)), V::I(r), user_col(db, w, "name")]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AverageScore, TotalViews, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank,
//        RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserPostStats),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT tu.UserId, tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.AverageScore, tu.TotalViews, COALESCE(ub.TotalBadges, 0) AS TotalBadges,
//        COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId WHERE tu.PostRank <= 10 OR tu.ViewRank <= 10 ORDER BY tu.PostRank, tu.ViewRank;
fn q6269(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ups = user_posts(db);
    let v = ranked(drain((&ups).and(&ub)), |&(_, (a, _))| Reverse(a[1]), false);
    let v = ranked(v, |&((_, (a, _)), _)| (a[5] == 0, Reverse(a[6])), false);
    rows(v.into_iter().filter(|&((_, p), w)| p <= 10 || w <= 10).map(|(((u, (a, b)), _), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1]), nullable(a[6], a[5])]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, SUM(CASE WHEN v.VoteTypeId = 10 THEN 1 ELSE 0 END) AS Deletions,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// PopularityMetrics AS (SELECT PostId, Title, Author, CreationDate, CommentCount, Upvotes, Downvotes, Deletions, (Upvotes - Downvotes) AS VoteNet,
//        CASE WHEN (Upvotes - Downvotes) > 0 THEN 'Positive' WHEN (Upvotes - Downvotes) < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment, PostRank FROM RankedPosts)
// SELECT pm.PostId, pm.Title, pm.Author, pm.CreationDate, pm.CommentCount, pm.Upvotes, pm.Downvotes, pm.Deletions, pm.VoteNet, pm.Sentiment
// FROM PopularityMetrics pm WHERE pm.PostRank <= 5 ORDER BY pm.CreationDate DESC;
//
// PostRank reads only base columns, so the five newest posts of each type are picked first and the comment x vote product is driven for those alone.
fn q28435(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (t == Some(10)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let n = a[1] - a[2];
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend(a.map(V::I));
        f.extend([V::I(n), V::S(if n > 0 { "Positive" } else if n < 0 { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, p.ViewCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS RankByScore, RANK() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS RankByDate
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, AnswerCount, ViewCount, OwnerUserId, OwnerDisplayName, RankByScore, RankByDate FROM RankedPosts WHERE RankByScore <= 5 OR RankByDate <= 5),
// PostComments AS (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId),
// FinalResult AS (SELECT tp.*, COALESCE(pc.CommentCount, 0) AS TotalComments FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId)
// SELECT fr.Title, fr.CreationDate, fr.Score, fr.AnswerCount, fr.ViewCount, fr.OwnerDisplayName, fr.TotalComments, pt.Name AS PostType
// FROM FinalResult fr JOIN PostTypes pt ON fr.OwnerUserId = pt.Id ORDER BY fr.Score DESC, fr.CreationDate DESC LIMIT 50;
//
// The final join compares a user id with a post type id, so it goes through origid.
fn q7538(db: &'static So) -> String {
    let Post { post_type, owner_user, owner_user_id, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(post_type));
    let mut top = top_per(v.clone(), |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    top.extend(top_per(v, |&(_, t)| t, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 5, true));
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tidx: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user_id.select(&tidx)));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(p, (c, t))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "answers", "views", "owner"]);
        f.extend([V::I(c), tname(db, t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName AS Author, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, u.DisplayName, p.OwnerUserId, p.Title, p.Body, p.Tags, p.CreationDate),
// BadgedUsers AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges FROM Badges GROUP BY UserId)
// SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.CreationDate, rp.Author, rp.UpVotes, rp.DownVotes, rp.CommentCount, bu.GoldBadges, bu.SilverBadges, bu.BronzeBadges,
//        CASE WHEN rp.PostRank = 1 THEN 'Latest Question' ELSE 'Older Question' END AS RankingStatus
// FROM RankedPosts rp LEFT JOIN BadgedUsers bu ON rp.OwnerUserId = bu.UserId WHERE rp.UpVotes > 5 ORDER BY rp.CreationDate DESC LIMIT 10;
fn q28874(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let latest = top_per(drain(qs().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let latest: MatSet<Id<Post>> = rel(latest.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = qs()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let bu = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&s).filt(|a| a[0] > 5).and(owner_user.select((&bu).opt())).and(Ident::<Post>::new().with(&latest).opt()));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, ((a, b), l))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "owner"]);
        f.extend(a.map(V::I));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if l.is_some() { "Latest Question" } else { "Older Question" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes,
//        COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END), 0) AS CloseVotes, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END), 0) AS DeleteVotes
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId
// GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the top questions are picked first and the comment x vote product is driven for those alone.
fn q5027(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap())), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let h = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(12)) as i64]);
    rows(drain((&s).and(&h)).into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        f.extend(h.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteStats AS (SELECT p.Id AS PostId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, pvs.TotalVotes, pvs.UpVotes, pvs.DownVotes
//     FROM RankedPosts rp JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId WHERE rp.Rank <= 10)
// SELECT t.TagName, COUNT(tp.PostId) AS PostCount, AVG(tp.Score) AS AverageScore, AVG(tp.ViewCount) AS AverageViews, SUM(tp.TotalVotes) AS TotalVotes,
//        SUM(tp.UpVotes) AS TotalUpVotes, SUM(tp.DownVotes) AS TotalDownVotes
// FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN TopPosts tp ON p.Id = tp.PostId GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 5;
fn q5976(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let lt = tag_mentions(db);
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&lt).map(|(p, _)| p).inv().collect();
    type R = (Id<Post>, Id<Tag>);
    let g = (&tp)
        .select(&by_post)
        .group_by(Same::<R>::new().map(|(_, t): R| t).select(&db.tag.tag_name))
        .select(Same::<R>::new().map(|(p, _): R| p).select(score.and(view_count.opt()).and(&pvs)))
        .fold([0i64; 7], |a, ((s, w), c)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c[0], a[5] + c[1], a[6] + c[2]]);
    let v = top_n(drain(&g), |&(_, a)| Reverse(a[0]), 5);
    rows(v.into_iter().map(|(t, a)| row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::I(a[6])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankPerUser
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, u.DisplayName AS OwnerDisplayName FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.RankPerUser <= 5),
// PostStatistics AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.OwnerDisplayName)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.OwnerDisplayName, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount,
//        CASE WHEN ps.Score > 100 THEN 'High' WHEN ps.Score BETWEEN 50 AND 100 THEN 'Medium' ELSE 'Low' END AS ScoreCategory
// FROM PostStatistics ps ORDER BY ps.Score DESC LIMIT 10;
fn q9603(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, _)| Reverse(score.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(if s > 100 { "High" } else if s >= 50 { "Medium" } else { "Low" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswer, u.Reputation, p.OwnerUserId FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND u.Reputation > 100),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostsWithComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ub.TotalBadges, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, pc.CommentCount,
//        CASE WHEN rp.AcceptedAnswer <> 0 THEN 'Has Accepted Answer' ELSE 'No Accepted Answer' END AS AnswerStatus
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostsWithComments pc ON rp.PostId = pc.PostId
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 10 OFFSET 0;
fn q865(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, accepted_answer_id, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100)))));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let v = top_n(top, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cc).and(owner_user.select((&ub).opt()))).into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(c));
        f.push(V::S(if accepted_answer_id.get(p).unwrap_or(0) != 0 { "Has Accepted Answer" } else { "No Accepted Answer" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        DENSE_RANK() OVER (ORDER BY p.CreationDate DESC) AS RecentRank FROM Posts p WHERE p.Score IS NOT NULL AND p.ViewCount > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = u.Id) AS PostCount,
//        (SELECT COUNT(*) FROM Comments c WHERE c.UserId = u.Id) AS CommentCount FROM Users u WHERE u.Reputation > 1000),
// ClosedPosts AS (SELECT h.PostId, h.CreationDate, h.Comment AS CloseReason FROM PostHistory h WHERE h.PostHistoryTypeId = 10),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, u.DisplayName AS Owner, rp.Score, COALESCE(c.CloseReason, 'Not Closed') AS CloseStatus
//     FROM RankedPosts rp LEFT JOIN Users u ON rp.PostId = u.Id LEFT JOIN ClosedPosts c ON rp.PostId = c.PostId WHERE rp.RankScore <= 5)
// SELECT trp.Title, trp.CreationDate, trp.Owner, trp.Score, trp.CloseStatus, ur.Reputation, ur.PostCount, ur.CommentCount
// FROM TopRankedPosts trp JOIN UserReputation ur ON trp.Owner = ur.DisplayName ORDER BY trp.Score DESC, trp.CreationDate DESC LIMIT 10;
//
// rp.PostId = u.Id compares a post id with a user id, so it goes through origid.
fn q3928(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, creation_date, origid, .. } = &db.post;
    let v = drain(db.post.with(view_count.gt(0)).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let by_name: HashIdx<Str, Id<User>> = rich().select(&db.user.display_name).inv().collect();
    let pc = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = rich().group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).select((&db.post_history.comment).opt());
    let v = drain((&tp).select(origid.select(&uidx).select(&db.user.display_name).and(closes.opt()).and(origid.select(&uidx).select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&pc).and(&cc)))));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(p, ((n, c), ((u, pc), cc)))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::S(n), V::I(score.get(p).unwrap()), V::S(c.flatten().unwrap_or("Not Closed")), user_col(db, u, "rep"), V::I(pc), V::I(cc)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, SUM(b.Class) AS TotalBadgeClass, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation >= 1000 GROUP BY u.Id),
// PostHistoryDetails AS (SELECT ph.PostId, ph.CreationDate AS HistoryDate, ph.UserDisplayName, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS RowNum
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11))
// SELECT up.PostId, up.Title, up.ViewCount, up.CommentCount, ur.TotalBadgeClass, ur.BadgeCount, p_hd.HistoryDate, p_hd.UserDisplayName AS HistoryActor, p_hd.Comment AS HistoryComment
// FROM RankedPosts up JOIN UserReputation ur ON up.PostId = ur.UserId LEFT JOIN PostHistoryDetails p_hd ON up.PostId = p_hd.PostId AND p_hd.RowNum = 1
// WHERE up.PostRank <= 10 ORDER BY up.Score DESC, up.ViewCount DESC;
//
// up.PostId = ur.UserId compares a post id with a user id, so it goes through origid.
fn q2141(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ur = db.user.with((&db.user.reputation).ge(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 2], |a, c| [a[0] + c.unwrap_or(0), a[1] + c.is_some() as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = drain(db.post_history.with(post_history_type_id.in_v(vec![10, 11])).select(post));
    let ph = top_per(ph, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let ph = rel(ph.into_iter().map(|(h, p)| (p, h)).collect());
    let last: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&ph).map(|(p, _)| p).inv().select(&ph).collect();
    let mut v = drain((&cc).and(origid.select(&uidx).select(&ur)).and((&last).map(|(_, h)| h).opt()));
    v.sort_by_key(|&(p, _)| {
        let w = db.post.view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, ((c, u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c), nullable(u[0], u[1]), V::I(u[1])]);
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), ostr(db.post_history.user_display_name.get(h)), ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(rp.CommentCount) AS TotalComments, SUM(rp.UpVoteCount) AS TotalUpVotes, SUM(rp.DownVoteCount) AS TotalDownVotes
//     FROM Users u JOIN RankedPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName HAVING SUM(rp.UpVoteCount) > 5 OR SUM(rp.CommentCount) > 10)
// SELECT tu.UserId, tu.DisplayName, tu.TotalComments, tu.TotalUpVotes, tu.TotalDownVotes, (tu.TotalUpVotes - tu.TotalDownVotes) AS NetVotes,
//        CASE WHEN tu.TotalComments > 20 THEN 'Active Contributor' WHEN tu.TotalUpVotes > 10 THEN 'Popular User' ELSE 'New User' END AS UserStatus
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId WHERE b.Class = 1 OR b.Class = 2 ORDER BY NetVotes DESC FETCH FIRST 10 ROWS ONLY;
//
// Summing each post's counts over the user's posts is the same as folding the user's posts x comments x votes rows.
fn q4672(db: &'static So) -> String {
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(recent).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let gs = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).is_in([1, 2])));
    let v = drain((&tu).filt(|a| a[1] > 5 || a[0] > 10).and(gs));
    let v = top_n(v, |&(_, (a, _))| Reverse(a[1] - a[2]), 10);
    rows(v.into_iter().map(|(u, (a, _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])]);
        f.push(V::S(if a[0] > 20 { "Active Contributor" } else if a[1] > 10 { "Popular User" } else { "New User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Body, u.DisplayName),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Body, rp.OwnerDisplayName, rp.AnswerCount, rp.UpVotesCount, rp.DownVotesCount,
//        CAST(rp.UpVotesCount AS FLOAT) / NULLIF(rp.UpVotesCount + rp.DownVotesCount, 0) AS UpVoteRatio FROM RankedPosts rp WHERE rp.Rank = 1)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.OwnerDisplayName, fp.AnswerCount, fp.UpVotesCount, fp.DownVotesCount, fp.UpVoteRatio,
//        CASE WHEN fp.UpVoteRatio > 0.7 THEN 'Highly Upvoted' WHEN fp.UpVoteRatio BETWEEN 0.5 AND 0.7 THEN 'Moderately Upvoted' ELSE 'Low Upvotes' END AS VoteCategory
// FROM FilteredPosts fp ORDER BY fp.UpVoteRatio DESC, fp.CreationDate DESC LIMIT 20;
//
// Rank partitions by p.Id, one row per group, so it is always 1. FLOAT / BIGINT is FLOAT, so the ratio is f32.
fn q25628(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (x, t)| [a[0] + x.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ratio = |a: [i64; 3]| if a[1] + a[2] == 0 { None } else { Some(a[1] as f32 / (a[1] + a[2]) as f32) };
    let v = top_n(drain(&s), |&(p, a)| (ratio(a).is_none(), Reverse(ratio(a).map(|r| fkey(r as f64))), Reverse(creation_date.get(p).unwrap())), 20);
    rows(v.into_iter().map(|(p, a)| {
        let r = ratio(a);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(r.map_or(V::Null, |r| V::F(r as f64)));
        f.push(V::S(match r {
            Some(r) if r as f64 > 0.7 => "Highly Upvoted",
            Some(r) if r as f64 >= 0.5 => "Moderately Upvoted",
            _ => "Low Upvotes",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.CreationDate, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(a.AnswerCount, 0) AS AnswerCount,
//        CASE WHEN rp.Score >= 50 THEN 'High Scoring' WHEN rp.Score BETWEEN 20 AND 49 THEN 'Medium Scoring' ELSE 'Low Scoring' END AS ScoreCategory, rp.UserRank
//     FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON rp.PostId = c.PostId
//     LEFT JOIN (SELECT ParentId AS PostId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON rp.PostId = a.PostId)
// SELECT pd.Title, pd.ViewCount, pd.Score, pd.CommentCount, pd.AnswerCount, pd.ScoreCategory, u.DisplayName AS OwnerDisplayName, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM PostDetails pd JOIN Users u ON pd.PostId = u.Id LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Class = 1 GROUP BY UserId) b ON u.Id = b.UserId
// WHERE pd.UserRank <= 3 ORDER BY pd.ViewCount DESC, pd.Score DESC LIMIT 100;
//
// pd.PostId = u.Id compares a post id with a user id, so it goes through origid.
fn q4921(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&tp).select(origid.select(&uidx)));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()))
    }, 100);
    let t100: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&t100).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&t100).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let bc = db.user.group_by(Ident::<User>::new()).select(gold).fold(0i64, |n, _| n + 1);
    rows(drain((&cc).and(&ac).and(origid.select(&uidx).select(Ident::<User>::new().and((&bc).opt())))).into_iter().map(|(p, ((c, a), (u, b)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.extend([V::I(c), V::I(a), V::S(if s >= 50 { "High Scoring" } else if s >= 20 { "Medium Scoring" } else { "Low Scoring" }), user_col(db, u, "name"), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.Tags,
//        RANK() OVER (PARTITION BY p.Tags ORDER BY p.ViewCount DESC) AS TagRank FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '1 year')),
// UserInteractions AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Comments c LEFT JOIN Votes v ON c.PostId = v.PostId GROUP BY c.PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, ui.CommentCount AS InteractionCommentCount, ui.UpVoteCount,
//        ui.DownVoteCount, ARRAY_LENGTH(STRING_TO_ARRAY(rp.Tags, ','), 1) AS TagCount FROM RankedPosts rp JOIN UserInteractions ui ON rp.PostId = ui.PostId WHERE rp.TagRank <= 5)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.Score, pd.AnswerCount, pd.CommentCount, pd.InteractionCommentCount, pd.UpVoteCount, pd.DownVoteCount, pd.TagCount
// FROM PostDetails pd ORDER BY pd.ViewCount DESC, pd.Score DESC LIMIT 100;
fn q26771(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, view_count, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(date(2023, 10, 1)))).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ui = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (_, t)| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&ui), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()))
    }, 100);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments"]);
        f.extend(a.map(V::I));
        f.push(oint(tags_str.get(p).map(|t| t.split(',').count() as i64)));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverCount, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, BadgeCount, GoldCount, SilverCount, BronzeCount, RANK() OVER (ORDER BY BadgeCount DESC) AS Rank FROM UserBadges WHERE BadgeCount > 0),
// PostAnalytics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts, SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPosts
//     FROM Posts P GROUP BY P.OwnerUserId)
// SELECT U.Id AS UserId, U.DisplayName, COALESCE(TU.BadgeCount, 0) AS BadgeCount, COALESCE(TU.GoldCount, 0) AS GoldCount, COALESCE(TU.SilverCount, 0) AS SilverCount,
//        COALESCE(TU.BronzeCount, 0) AS BronzeCount, COALESCE(PA.PostCount, 0) AS PostCount, COALESCE(PA.UpvotedPosts, 0) AS UpvotedPosts, COALESCE(PA.DownvotedPosts, 0) AS DownvotedPosts
// FROM Users U LEFT JOIN TopUsers TU ON U.Id = TU.UserId LEFT JOIN PostAnalytics PA ON U.Id = PA.OwnerUserId WHERE U.Reputation > 1000 ORDER BY TU.Rank NULLS LAST, U.DisplayName;
fn q2178(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let score = &db.post.score;
    let pa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 3], |a, s| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64]);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ub).opt().and((&pa).opt())));
    rows(v.into_iter().map(|(u, (b, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.extend(a.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id as UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(PS.PostCount, 0) AS TotalPosts, COALESCE(PS.QuestionCount, 0) AS TotalQuestions,
//        COALESCE(PS.AnswerCount, 0) AS TotalAnswers, COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.AverageScore, 0) AS AvgScore
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId),
// RankedUsers AS (SELECT *, ROW_NUMBER() OVER (ORDER BY TotalViews DESC, TotalBadges DESC) AS Rank FROM CombinedStats)
// SELECT UserId, DisplayName, TotalBadges, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, AvgScore, Rank FROM RankedUsers WHERE Rank <= 10 ORDER BY Rank;
fn q4866(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let v = top_n(drain((&bc).and(&ups)), |&(_, (b, a))| (Reverse(a[6]), Reverse(b)), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (b, a)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), if a[1] == 0 { V::F(0.0) } else { avg(a[4], a[1]) }, V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// TopQuestions AS (SELECT PostId, Title, Body, CreationDate, ViewCount, Score, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// CommentsData AS (SELECT c.PostId, COUNT(*) AS TotalComments FROM Comments c GROUP BY c.PostId),
// BadgesData AS (SELECT b.UserId, COUNT(*) AS TotalBadges FROM Badges b GROUP BY b.UserId),
// PostStats AS (SELECT tq.PostId, tq.Title, tq.CreationDate, tq.ViewCount, tq.Score, tq.OwnerDisplayName, COALESCE(cd.TotalComments, 0) AS TotalComments, bd.TotalBadges
//     FROM TopQuestions tq LEFT JOIN CommentsData cd ON tq.PostId = cd.PostId LEFT JOIN BadgesData bd ON tq.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = bd.UserId))
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.TotalComments, ps.TotalBadges FROM PostStats ps ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q27771(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(current_date(), -1)))).with(owner_user).select(score));
    let v = top_n(v, |&(p, s)| (Reverse(s), p), 10);
    let tq: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tq).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bd = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let bdv = rel(drain(&bd));
    let by_name: HashIdx<Str, (i64, i64)> = (&bdv).map(|(u, _)| u).select(&uidx).select(&db.user.display_name).inv().select(&bdv).collect();
    let v = drain((&cc).and(owner_user.select(&db.user.display_name).select(&by_name).opt()));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), b.map_or(V::Null, |b| V::I(b.1))]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalViews, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics),
// PopularTags AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 10)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.TotalViews, tu.UpVotes, tu.DownVotes, pt.TagName
// FROM TopUsers tu CROSS JOIN PopularTags pt WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC, pt.PostCount DESC LIMIT 50;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q7142(db: &'static So) -> String {
    let rr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(rr.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, view_count, .. } = &db.post;
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(post_type_id).and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .buf_fold(|v| {
            let mut a = [0i64; 7];
            let mut ids: Vec<Id<Post>> = Vec::new();
            for x in v.iter() {
                if let Some((((p, t), w), vt)) = *x {
                    ids.push(p);
                    a[1] += (t == 1) as i64;
                    a[2] += (t == 2) as i64;
                    a[3] += w.is_some() as i64;
                    a[4] += w.unwrap_or(0);
                    a[5] += (vt == Some(2)) as i64;
                    a[6] += (vt == Some(3)) as i64;
                }
            }
            ids.sort_unstable();
            ids.dedup();
            a[0] = ids.len() as i64;
            a
        });
    let lt = tag_mentions(db);
    let pc = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t).select(&db.tag.tag_name)).select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p)).count_distinct();
    let pt = rel(top_n(drain(&pc), |&(_, n)| Reverse(n), 10));
    let mut v = Vec::new();
    (&us).cross(&pt).drive(|(u, _), (a, (t, n))| v.push((u, a, t, n)));
    let v = top_n(v, |&(u, _, _, n)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)), 50);
    rows(v.into_iter().map(|(u, a, t, _)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5]), V::I(a[6]), V::S(t)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId),
// BadgePostStats AS (SELECT UB.UserId, UB.DisplayName, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, PS.PostCount, PS.QuestionCount, PS.AnswerCount, PS.TotalViews
//     FROM UserBadgeCounts UB LEFT JOIN PostStatistics PS ON UB.UserId = PS.OwnerUserId)
// SELECT B.UserId, B.DisplayName, B.BadgeCount, B.GoldBadges, B.SilverBadges, B.BronzeBadges, COALESCE(B.PostCount, 0) AS PostCount, COALESCE(B.QuestionCount, 0) AS QuestionCount,
//        COALESCE(B.AnswerCount, 0) AS AnswerCount, COALESCE(B.TotalViews, 0) AS TotalViews, RANK() OVER (ORDER BY B.BadgeCount DESC) AS BadgeRank
// FROM BadgePostStats B ORDER BY BadgeCount DESC, DisplayName;
fn q28544(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ups = user_posts(db);
    let v = ranked(drain((&ub).and(&ups)), |&(_, (b, _))| Reverse(b[0]), false);
    rows(v.into_iter().map(|((u, (b, a)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(r)]);
        row(f)
    }))
}

// WITH TagMetrics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN PostHistory ph ON ph.PostId = p.Id WHERE p.PostTypeId = 1 GROUP BY t.TagName),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId JOIN Votes v ON v.PostId = p.Id AND v.UserId = u.Id WHERE p.PostTypeId = 2 GROUP BY u.Id, u.DisplayName),
// CombinedMetrics AS (SELECT tm.TagName, tm.PostCount, tm.CloseCount, tm.ReopenCount, ue.UserId, ue.DisplayName, ue.AnswerCount, ue.UpVotes, ue.DownVotes FROM TagMetrics tm CROSS JOIN UserEngagement ue)
// SELECT c.TagName, c.PostCount, c.CloseCount, c.ReopenCount, c.UserId, c.DisplayName, c.AnswerCount, c.UpVotes, c.DownVotes, RANK() OVER (ORDER BY c.PostCount DESC) AS TagRank
// FROM CombinedMetrics c ORDER BY c.PostCount DESC, c.CloseCount DESC, c.ReopenCount DESC, c.UserId;
fn q29944(db: &'static So) -> String {
    let asked = Ident::<Post>::new().with((&db.post.post_type_id).eq(1));
    let lt = tag_mentions(db);
    type R = (Id<Post>, Id<Tag>);
    let tm = (&lt)
        .group_by(Same::<R>::new().map(|(_, t): R| t).select(&db.tag.tag_name))
        .select(Same::<R>::new().map(|(p, _): R| p).select(asked).select(Ident::<Post>::new().and(history_of(db).select(&db.post_history.post_history_type_id).opt())))
        .buf_fold(|v| {
            let mut ids: Vec<Id<Post>> = v.iter().map(|x| x.0).collect();
            ids.sort_unstable();
            ids.dedup();
            [ids.len() as i64, v.iter().filter(|x| x.1 == Some(10)).count() as i64, v.iter().filter(|x| x.1 == Some(11)).count() as i64]
        });
    let answer = Ident::<Post>::new().with((&db.post.post_type_id).eq(2));
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(answer).select(Ident::<Post>::new().and(own_votes(db).select(&db.vote.vote_type_id))))
        .buf_fold(|v| {
            let mut ids: Vec<Id<Post>> = v.iter().map(|x| x.0).collect();
            ids.sort_unstable();
            ids.dedup();
            [ids.len() as i64, v.iter().filter(|x| x.1 == 2).count() as i64, v.iter().filter(|x| x.1 == 3).count() as i64]
        });
    let mut v = Vec::new();
    (&tm).cross(&ue).drive(|(t, u), (a, b)| v.push((t, u, a, b)));
    let v = ranked(v, |&(_, _, a, _)| Reverse(a[0]), false);
    rows(v.into_iter().map(|((t, u, a, b), r)| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.OwnerDisplayName, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.OwnerDisplayName, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount,
//        CASE WHEN tp.CommentCount > 5 THEN 'High Engagement' WHEN tp.CommentCount >= 3 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM TopPosts tp JOIN PostHistory ph ON tp.PostId = ph.PostId WHERE ph.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' ORDER BY tp.Score DESC;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q5015(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let recent = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).ge(add_days(t0, -30))));
    rows(drain((&s).and(recent)).into_iter().map(|(p, (a, _))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[0] > 5 { "High Engagement" } else if a[0] >= 3 { "Moderate Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments, SUM(V.BountyAmount) AS TotalBounty,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN V.VoteTypeId = 1 THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// PopularTags AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS TagUsageCount FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%,', T.TagName, ',%') GROUP BY T.TagName HAVING COUNT(DISTINCT P.Id) > 5),
// PostsRanked AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, ROW_NUMBER() OVER (ORDER BY P.Score DESC) AS Rank FROM Posts P WHERE P.PostTypeId = 1)
// SELECT U.DisplayName, UE.TotalPosts, UE.TotalComments, UE.TotalBounty, UE.UpVotes, UE.DownVotes, PT.TagName, PR.PostId, PR.Title, PR.Score, PR.ViewCount
// FROM UserEngagement UE JOIN PopularTags PT ON UE.TotalPosts > 10 JOIN PostsRanked PR ON PR.Rank <= 10 LEFT JOIN Users U ON U.Id = UE.UserId
// WHERE UE.UpVotes > 20 ORDER BY UE.UpVotes DESC, PR.Score DESC;
//
// Both ON clauses name one side only, so the three are crossed. The LIKE is a substring join, run over the distinct Tags strings.
fn q29414(db: &'static So) -> String {
    let Post { post_type_id, score, tags_str, .. } = &db.post;
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt())).opt())
        .buf_fold(|v| {
            let mut ps: Vec<Id<Post>> = Vec::new();
            let mut cs: Vec<Id<Comment>> = Vec::new();
            let mut a = [0i64; 6];
            for x in v.iter() {
                if let Some(((p, c), vt)) = *x {
                    ps.push(p);
                    cs.extend(c);
                    if let Some((t, b)) = vt {
                        a[2] += b.is_some() as i64;
                        a[3] += b.unwrap_or(0);
                        a[4] += (t == 2) as i64;
                        a[5] += (t == 3) as i64;
                    }
                }
            }
            ps.sort_unstable();
            ps.dedup();
            cs.sort_unstable();
            cs.dedup();
            a[0] = ps.len() as i64;
            a[1] = cs.len() as i64;
            a
        });
    let strs: MatSet<Str> = tags_str.collect();
    let hit: HashIdx<Str, Id<Tag>> = (&strs).select_where((&db.tag.tag_name).inv(), |s: Str, n: Str| s.contains(&format!(",{n},"))).collect();
    type R = (Id<Post>, Id<Tag>);
    let pt = db.post.select(Ident::<Post>::new().and(tags_str.select(&hit))).group_by(Same::<R>::new().map(|(_, t): R| t).select(&db.tag.tag_name)).select(Same::<R>::new().map(|(p, _): R| p)).count_distinct();
    let pt: Vec<_> = drain((&pt).filt(|n| n > 5));
    let pr = rel(top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 10));
    let ptr = rel(pt);
    let mut v = Vec::new();
    (&ue).filt(|a| a[0] > 10 && a[4] > 20).cross(&ptr).cross(&pr).drive(|((u, _), _), ((a, (t, _)), (p, _))| v.push((u, a, t, p)));
    v.sort_by_key(|&(_, a, _, p)| (Reverse(a[4]), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(u, a, t, p)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::S(t)];
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY U.CreationDate DESC) AS LatestActivity FROM Users U WHERE U.Reputation > 1000),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Posts P GROUP BY P.OwnerUserId),
// PostHistoryStats AS (SELECT PH.UserId, COUNT(PH.Id) AS EditCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5, 6) GROUP BY PH.UserId),
// CombinedStats AS (SELECT U.DisplayName, U.Reputation, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.QuestionCount, 0) AS QuestionCount, COALESCE(PS.AnswerCount, 0) AS AnswerCount,
//        COALESCE(PHS.EditCount, 0) AS EditCount FROM UserReputation U LEFT JOIN PostStats PS ON U.UserId = PS.OwnerUserId LEFT JOIN PostHistoryStats PHS ON U.UserId = PHS.UserId)
// SELECT C.DisplayName, C.Reputation, C.TotalPosts, C.QuestionCount, C.AnswerCount, C.EditCount,
//        CASE WHEN C.Reputation > 5000 THEN 'Expert' WHEN C.Reputation BETWEEN 1000 AND 5000 THEN 'Experienced' ELSE 'Novice' END AS UserLevel
// FROM CombinedStats C ORDER BY C.Reputation DESC, C.TotalPosts DESC LIMIT 10;
fn q4330(db: &'static So) -> String {
    let ups = user_posts(db);
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let ec = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ups).and((&ec).opt())));
    let v = top_n(v, |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1])), 10);
    rows(v.into_iter().map(|(u, (a, e))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(e.unwrap_or(0)), V::S(if r > 5000 { "Expert" } else if r >= 1000 { "Experienced" } else { "Novice" })]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(P.Score) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT U.Id, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.Questions, 0) AS Questions,
//        COALESCE(PS.Answers, 0) AS Answers, COALESCE(PS.TotalScore, 0) AS TotalScore, RANK() OVER (ORDER BY COALESCE(PS.TotalScore, 0) DESC) AS ScoreRank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT U.DisplayName, U.BadgeCount, U.PostCount, U.Questions, U.Answers, U.TotalScore,
//        CASE WHEN U.BadgeCount >= 10 THEN 'Star User' WHEN U.TotalScore > 1000 THEN 'Top Contributor' ELSE 'New User' END AS UserCategory
// FROM UserPerformance U WHERE U.ScoreRank <= 10 ORDER BY U.TotalScore DESC, U.BadgeCount DESC;
fn q4135(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let v = ranked(drain((&bc).and(&ups)), |&(_, (_, a))| Reverse(a[4]), false);
    rows(v.into_iter().filter(|x| x.1 <= 10).map(|((u, (b, a)), _)| {
        let mut f = vec![user_col(db, u, "name"), V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])];
        f.push(V::S(if b >= 10 { "Star User" } else if a[4] > 1000 { "Top Contributor" } else { "New User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(NULLIF(u.DisplayName, ''), 'Anonymous') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// PostStats AS (SELECT rp.OwnerDisplayName, COUNT(rp.Id) AS TotalPosts, SUM(rp.Score) AS TotalScore, AVG(rp.ViewCount) AS AverageViews, MAX(rp.CreationDate) AS MostRecentPostDate
//     FROM RankedPosts rp GROUP BY rp.OwnerDisplayName),
// CommentCount AS (SELECT PostId, COUNT(*) AS CommentTotal FROM Comments GROUP BY PostId),
// FinalResults AS (SELECT ps.OwnerDisplayName, ps.TotalPosts, ps.TotalScore, ps.AverageViews, ps.MostRecentPostDate, COALESCE(cc.CommentTotal, 0) AS TotalComments
//     FROM PostStats ps LEFT JOIN CommentCount cc ON cc.PostId = (SELECT Id FROM Posts WHERE OwnerDisplayName = ps.OwnerDisplayName ORDER BY CreationDate DESC LIMIT 1))
// SELECT OwnerDisplayName, TotalPosts, TotalScore, AverageViews, MostRecentPostDate, TotalComments FROM FinalResults WHERE TotalPosts > 5 AND TotalScore > 50
// ORDER BY TotalScore DESC, TotalPosts ASC;
//
// The correlated LIMIT 1 reads Posts.OwnerDisplayName (the post's own column), so it is an arg-max fold per that name; a tie on
// CreationDate keeps the larger id.
fn q2835(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, owner_display_name, .. } = &db.post;
    let name = owner_user.select(&db.user.display_name).opt().map(|n: Option<Str>| match n {
        Some(s) if !s.is_empty() => s,
        _ => "Anonymous",
    });
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .group_by(name)
        .select(score.and(view_count.opt()).and(creation_date))
        .fold([0, 0, 0, 0, i64::MIN], |a, ((s, w), d)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4].max(d)]);
    let newest = db.post.group_by(owner_display_name).select(creation_date.and(Ident::<Post>::new())).fold(None, |m: Option<(i64, Id<Post>)>, x| Some(m.map_or(x, |m| m.max(x))));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&ps).filt(|a| a[0] > 5 && a[1] > 50).and((&newest).flat_map(|m| m.map(|x| x.1)).select(&cc).opt()));
    rows(v.into_iter().map(|(n, (a, c))| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::T(a[4]), V::I(c.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(b.Class) AS BadgeCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, u.DisplayName, p.Score),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, CommentCount, UpVotes, DownVotes, BadgeCount, rn FROM RankedPosts WHERE rn = 1 ORDER BY UpVotes DESC, CommentCount DESC LIMIT 10)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.CommentCount, tp.UpVotes, tp.DownVotes, tp.BadgeCount,
//        CASE WHEN tp.BadgeCount >= 10 THEN 'Top Contributor' WHEN tp.BadgeCount >= 5 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorLevel
// FROM TopPosts tp;
//
// rn reads only base columns, so each owner's top question is picked first and the comment x vote x badge product is driven for those alone.
fn q5158(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let classes = owner_user.select(badges_of(db).select(&db.badge.class));
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(classes.opt()))
        .fold([0i64; 5], |a, ((c, t), b)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]);
    let v = top_n(drain(&s), |&(_, a)| (Reverse(a[1]), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3])]);
        f.push(V::S(if a[3] > 0 && a[4] >= 10 { "Top Contributor" } else if a[3] > 0 && a[4] >= 5 { "Active Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, AcceptedAnswers, AvgReputation, RANK() OVER (ORDER BY AvgReputation DESC) AS ReputationRank FROM UserPostStats),
// ClosedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY p.Id, p.OwnerUserId, p.Title)
// SELECT ru.UserId, ru.DisplayName, ru.PostCount, ru.AnswerCount, ru.AcceptedAnswers, ru.AvgReputation, cp.CloseCount,
//        CASE WHEN cp.CloseCount IS NULL THEN 'No closures' ELSE CONCAT('Closed ', cp.CloseCount, ' times') END AS ClosureInfo
// FROM RankedUsers ru LEFT JOIN ClosedPosts cp ON ru.UserId = cp.OwnerUserId WHERE ru.ReputationRank <= 10 AND (ru.PostCount > 5 OR ru.AvgReputation > 1000)
// ORDER BY ru.ReputationRank, ru.DisplayName;
fn q51(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(post_type_id.and(accepted_answer.opt())).opt()))
        .fold([0i64; 4], |a, (r, p)| match p {
            Some((t, x)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1 && x.is_some()) as i64, a[3] + r],
            None => [a[0], a[1], a[2], a[3] + r],
        });
    let rows_of = |a: [i64; 4]| a[0].max(1);
    let v = ranked(drain(&ups), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ph10 = (&db.post_history.post_history_type_id).is_in([10, 11]);
    let cp = db
        .post
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with(ph10)).select(&db.post_history.post_history_type_id))
        .fold(0i64, |n, t| n + (t == 10) as i64);
    let by_owner: HashIdx<Id<User>, Id<Post>> = db.post.with(&cp).select(&db.post.owner_user).inv().collect();
    let v = drain((&tu).select(Ident::<User>::new().and((&ups).filt(|a| a[0] > 5 || a[3] as f64 / rows_of(a) as f64 > 1000.0)).and((&by_owner).select(&cp).opt())));
    rows(v.into_iter().map(|(u, ((_, a), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(a[3] as f64 / rows_of(a) as f64), oint(c)]);
        f.push(match c {
            Some(c) => V::Owned(format!("Closed {c} times")),
            None => V::S("No closures"),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopAnsweredPosts AS (SELECT p.Id, p.Title, p.AnswerCount, p.Score, (SELECT COUNT(c.Id) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.AnswerCount > 0 ORDER BY p.AnswerCount DESC LIMIT 10)
// SELECT r.Title AS QuestionTitle, u.DisplayName AS OwnerName, u.Reputation AS OwnerReputation, r.Score AS QuestionScore, COALESCE(tap.CommentCount, 0) AS TotalComments,
//        tap.AnswerCount AS TotalAnswers, us.QuestionCount AS UserQuestionCount, us.TotalScore AS UserTotalScore
// FROM RankedPosts r JOIN Users u ON r.OwnerUserId = u.Id LEFT JOIN TopAnsweredPosts tap ON tap.Id = r.Id JOIN UserStats us ON us.UserId = r.OwnerUserId
// WHERE r.Rank = 1 ORDER BY r.Score DESC LIMIT 20;
fn q3764(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, answer_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let top = top_n(top, |&(p, _)| Reverse(score.get(p).unwrap()), 20);
    let r: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tap = top_n(drain(db.post.with(post_type_id.eq(1).and(answer_count.gt(0))).select(answer_count)), |&(p, n)| (Reverse(n), p), 10);
    let tap: MatSet<Id<Post>> = rel(tap.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tap).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt()).fold([0i64; 2], |a, s| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0)]);
    let mut v = drain((&r).select(Ident::<Post>::new().and((&cc).opt()).and(owner_user.select(Ident::<User>::new().and(&us)))));
    v.sort_by_key(|&(p, _)| Reverse(score.get(p).unwrap()));
    rows(v.into_iter().map(|(p, ((_, c), (u, a)))| {
        let mut f = vec![post_fields(db, p, &["title"]).remove(0)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(score.get(p).unwrap()), V::I(c.unwrap_or(0)), if c.is_some() { oint(answer_count.get(p)) } else { V::Null }, V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.ViewCount DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteDetails AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// PostHistorySummary AS (SELECT PH.PostId, COUNT(PH.Id) AS EditCount, MAX(PH.CreationDate) AS LastEditDate FROM PostHistory PH GROUP BY PH.PostId),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, RP.CreationDate, RP.Score, RP.ViewCount, PVD.UpVotes, PVD.DownVotes, PHS.EditCount, PHS.LastEditDate
//     FROM RankedPosts RP LEFT JOIN PostVoteDetails PVD ON RP.PostId = PVD.PostId LEFT JOIN PostHistorySummary PHS ON RP.PostId = PHS.PostId WHERE RP.PostRank <= 10)
// SELECT T.Id AS TagId, T.TagName, TP.* FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' JOIN TopPosts TP ON P.Id = TP.PostId ORDER BY T.TagName, TP.Score DESC;
fn q8672(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvd = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let phs = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let lt = tag_mentions(db);
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&lt).map(|(p, _)| p).inv().collect();
    let v = drain((&tp).select(Ident::<Post>::new().and((&by_post).map(|(_, t)| t)).and((&pvd).opt()).and((&phs).opt())));
    rows(v.into_iter().map(|(p, (((_, t), u), h))| {
        let mut f = vec![V::I(db.tag.origid.get(t).unwrap()), V::S(db.tag.tag_name.get(t).unwrap())];
        f.extend(post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]));
        f.extend(match u {
            Some(u) => u.map(V::I),
            None => [V::Null, V::Null],
        });
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentActivity AS (SELECT U.Id AS UserId, RANK() OVER (PARTITION BY U.Id ORDER BY C.CreationDate DESC) AS RecentCommentRank, C.Text AS LastCommentText
//     FROM Users U LEFT JOIN Comments C ON U.Id = C.UserId),
// AggregatedData AS (SELECT U.UserId, U.DisplayName, U.Reputation, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalBounty, R.LastCommentText
//     FROM UserStatistics U LEFT JOIN RecentActivity R ON U.UserId = R.UserId AND R.RecentCommentRank = 1)
// SELECT A.DisplayName, A.TotalPosts, A.TotalQuestions, A.TotalAnswers, A.TotalBounty, COALESCE(A.LastCommentText, 'No comments made') AS LastCommentText,
//        CASE WHEN A.Reputation > 1000 THEN 'High Reputation' WHEN A.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM AggregatedData A WHERE A.TotalPosts > 10 ORDER BY A.Reputation DESC LIMIT 50;
fn q3532(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 5], |a, (t, b)| {
            let b = b.flatten();
            [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
        });
    let Comment { user, creation_date: cd, .. } = &db.comment;
    let last = db.comment.group_by(user).select(cd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<User>, i64), Id<Comment>> = db.comment.select(user.and(cd)).inv().collect();
    let v = drain((&us).filt(|a| a[0] > 10).and(Ident::<User>::new().and(&last).select(&at).opt()));
    let v = top_n(v, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), 50);
    rows(v.into_iter().map(|(u, (a, c))| {
        let r = db.user.reputation.get(u).unwrap();
        row(vec![
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            nullable(a[4], a[3]),
            V::S(c.map_or("No comments made", |c| db.comment.text.get(c).unwrap())),
            V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes, COUNT(DISTINCT p.Id) AS TotalQuestions,
//        ROW_NUMBER() OVER (ORDER BY SUM(u.UpVotes) - SUM(u.DownVotes) DESC) AS UserRank
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 10),
// PostHistoryDetails AS (SELECT ph.PostId, p.Title, ph.CreationDate, ph.UserDisplayName, ph.Comment, ph.Text FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (10, 12))
// SELECT ru.UserId, ru.DisplayName, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ru.TotalUpVotes, ru.TotalDownVotes, ph.Comment, ph.Text
// FROM TopUsers ru JOIN RankedPosts rp ON ru.UserId = rp.PostId LEFT JOIN PostHistoryDetails ph ON rp.PostId = ph.PostId
// WHERE rp.RankByScore <= 5 ORDER BY ru.UserRank, rp.Score DESC;
//
// ru.UserId = rp.PostId compares a user id with a post id, so it goes through origid.
fn q5437(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let User { up_votes, down_votes, .. } = &db.user;
    let asked = Ident::<Post>::new().with(post_type_id.eq(1));
    let tu = db.user.group_by(Ident::<User>::new()).select(up_votes.and(down_votes).and(posts_of(db).select(asked))).fold([0i64; 3], |a, ((u, d), _)| [a[0] + u, a[1] + d, a[2] + 1]);
    let tr = top_n(drain((&tu).filt(|a| a[2] > 10)), |&(u, a)| (Reverse(a[0] - a[1]), u), 0);
    let tr = rel(tr.into_iter().enumerate().map(|(i, (u, a))| (db.user.origid.get(u).unwrap(), (u, a, i as i64 + 1))).collect());
    let by_uid: HashIdx<i64, (i64, (Id<User>, [i64; 3], i64))> = (&tr).map(|(o, _)| o).inv().select(&tr).collect();
    let phd = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 12])));
    let mut v = drain((&rp).select(origid.select(&by_uid).and(phd.opt())));
    v.sort_by_key(|&(p, ((_, (_, _, r)), _))| (r, Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((_, (u, a, _)), h))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [ostr(db.post_history.comment.get(h)), ostr(db.post_history.text.get(h))],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS TagRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.Tags, COUNT(c.Id) AS CommentCount, COUNT(pv.Id) AS ViewCount, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseVotes
//     FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN PostLinks pl ON rp.PostId = pl.PostId LEFT JOIN Posts pv ON rp.PostId = pv.Id
//     LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId WHERE rp.TagRank <= 5 GROUP BY rp.PostId, rp.Title, rp.Tags)
// SELECT ue.UserId, ue.DisplayName, pm.PostId, pm.Title, pm.Tags, pm.CommentCount, pm.ViewCount, pm.CloseVotes, ue.VoteCount, ue.UpVotes, ue.DownVotes, ue.BadgeCount
// FROM UserEngagement ue JOIN PostMetrics pm ON ue.UserId = pm.PostId ORDER BY pm.ViewCount DESC, ue.VoteCount DESC;
//
// ue.UserId = pm.PostId compares a user id with a post id, so it goes through origid; UserEngagement is folded only for the users that join.
fn q29585(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pm = (&rp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(links_of(db).opt()).and(Ident::<Post>::new()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, (((c, _), _), t)| [a[0] + c.is_some() as i64, a[1] + 1, a[2] + (t == Some(10)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let joined: MatSet<Id<User>> = (&pm).map(|_| ()).inv().map(|p: Id<Post>| p).select(origid).select(&uidx).collect();
    let ue = (&joined)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (t, b)| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + b.is_some() as i64]);
    let mut v = drain((&pm).and(origid.select(&uidx).select(Ident::<User>::new().and(&ue))));
    v.sort_by_key(|&(_, (a, (_, e)))| (Reverse(a[1]), Reverse(e[0])));
    rows(v.into_iter().map(|(p, (a, (u, e)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "tags"]));
        f.extend(a.map(V::I));
        f.extend(e.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN, p.OwnerUserId
//     FROM Posts p WHERE p.Score > 0),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT p.Id) AS TotalPosts
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS IsClosed FROM PostHistory ph GROUP BY ph.PostId, ph.CreationDate)
// SELECT us.UserId, us.DisplayName, us.GoldBadges, us.SilverBadges, us.BronzeBadges, us.TotalPosts, rp.PostId, rp.Title AS LatestPostTitle, rp.CreationDate AS LatestPostDate,
//        rp.Score AS LatestPostScore, rp.ViewCount AS LatestPostViews, cp.IsClosed
// FROM UserStats us LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.RN = 1 LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE (us.TotalPosts > 10 OR us.GoldBadges > 0) ORDER BY us.DisplayName, LatestPostDate DESC LIMIT 50;
fn q4176(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt()))
        .buf_fold(|v| {
            let mut ps: Vec<Id<Post>> = v.iter().filter_map(|x| x.1).collect();
            ps.sort_unstable();
            ps.dedup();
            let n = |c: i64| v.iter().filter(|x| x.0 == Some(c)).count() as i64;
            [n(1), n(2), n(3), ps.len() as i64]
        });
    let Post { owner_user, score, creation_date, .. } = &db.post;
    let latest = top_per(drain(db.post.with(score.gt(0)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let lv = rel(latest.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&lv).map(|(u, _)| u).inv().select(&lv).collect();
    let PostHistory { post, creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.group_by(post.and(hd)).select(post_history_type_id).fold(0i64, |m, t| m.max((t == 10) as i64));
    let cv = rel(drain(&cp));
    let cp_of: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let v = drain((&us).filt(|a| a[3] > 10 || a[0] > 0).and((&by_user).map(|(_, p)| p).select(Ident::<Post>::new().and((&cp_of).map(|(_, c)| c).opt())).opt()));
    let v = top_n(v, |&(u, (_, x))| (db.user.display_name.get(u).unwrap(), x.is_none(), Reverse(x.map(|(p, _)| creation_date.get(p).unwrap()))), 50);
    rows(v.into_iter().map(|(u, (a, x))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        match x {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
                f.push(oint(c));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Tags, p.CreationDate),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Tags, rp.CommentCount, rp.Upvotes, rp.Downvotes, rp.CreationDate FROM RankedPosts rp WHERE rp.Rank <= 5),
// AggregatedData AS (SELECT f.Tags, COUNT(f.PostId) AS TotalPosts, SUM(f.CommentCount) AS TotalComments, SUM(f.Upvotes) AS TotalUpvotes, SUM(f.Downvotes) AS TotalDownvotes
//     FROM FilteredPosts f GROUP BY f.Tags)
// SELECT a.Tags, a.TotalPosts, a.TotalComments, a.TotalUpvotes, a.TotalDownvotes, CASE WHEN a.TotalPosts > 0 THEN (a.TotalUpvotes + 1.0) / (a.TotalPosts + 1.0) ELSE 0 END AS UpvoteRatio,
//        CASE WHEN a.TotalPosts > 0 THEN (a.TotalDownvotes + 1.0) / (a.TotalPosts + 1.0) ELSE 0 END AS DownvoteRatio
// FROM AggregatedData a ORDER BY a.TotalPosts DESC, a.TotalUpvotes DESC;
//
// Rank reads only base columns, so the five newest questions per Tags are picked first and the comment x vote product is driven for those alone.
fn q28550(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let g = (&tp).group_by(tags_str.opt()).select(&s).fold([0i64; 4], |a, s| [a[0] + 1, a[1] + s[0], a[2] + s[1], a[3] + s[2]]);
    rows(drain(&g).into_iter().map(|(t, a)| {
        row(vec![ostr(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::F((a[2] as f64 + 1.0) / (a[0] as f64 + 1.0)), V::F((a[3] as f64 + 1.0) / (a[0] as f64 + 1.0))])
    }))
}

// WITH RecursiveTopUsers AS (SELECT Id, DisplayName, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users WHERE Reputation > 0),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostSummary AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId),
// JoinUserData AS (SELECT u.DisplayName, u.Reputation, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(ub.GoldBadges, 0) AS GoldBadges,
//        COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
//     FROM Users u LEFT JOIN PostSummary ps ON u.Id = ps.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE u.Reputation > 1000),
// TopTenUsers AS (SELECT *, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM JoinUserData)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalScore, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges,
//        CASE WHEN tu.TotalPosts > 0 THEN ROUND(tu.TotalScore / tu.TotalPosts, 2) ELSE 0 END AS AverageScorePerPost
// FROM TopTenUsers tu WHERE tu.UserRank <= 10 ORDER BY tu.Reputation DESC;
//
// RecursiveTopUsers is never read.
fn q34830(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let v = ranked(drain(rich().select(&db.user.reputation)), |&(_, r)| Reverse(r), true);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ub = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let ps = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score).opt()).fold([0i64; 2], |a, s| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0)]);
    rows(drain((&ps).and(&ub)).into_iter().map(|(u, (p, b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(p[0]), V::I(p[1])];
        f.extend(b.map(V::I));
        f.push(V::F(if p[0] > 0 { (p[1] as f64 / p[0] as f64 * 100.0).round() / 100.0 } else { 0.0 }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalQuestions, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT u.DisplayName, us.TotalQuestions, us.Upvotes, us.Downvotes, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, rp.Title AS LastPostTitle, rp.CreationDate AS LastPostDate
// FROM UserStats us JOIN Users u ON us.UserId = u.Id LEFT JOIN UserBadges ub ON ub.UserId = u.Id LEFT JOIN RankedPosts rp ON rp.OwnerUserId = u.Id AND rp.PostRank = 1
// WHERE us.TotalQuestions > 0 ORDER BY us.Upvotes - us.Downvotes DESC, LastPostDate DESC LIMIT 10;
fn q4043(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let asked = Ident::<Post>::new().with(post_type_id.eq(1));
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(asked).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let last = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let lv = rel(last.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&lv).map(|(u, _)| u).inv().select(&lv).collect();
    let v = drain((&us).and(&ub).and((&by_user).map(|(_, p)| p).opt()));
    let v = top_n(v, |&(_, ((a, _), p))| (Reverse(a[1] - a[2]), p.is_none(), Reverse(p.map(|p| creation_date.get(p).unwrap()))), 10);
    rows(v.into_iter().map(|(u, ((a, b), p))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserScores AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, (U.UpVotes - U.DownVotes) AS NetVotes, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U WHERE U.Reputation > 0),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.OwnerUserId, COUNT(COALESCE(C.Id, 0)) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.OwnerUserId),
// ClosedPostHistory AS (SELECT PH.PostId, COUNT(*) AS CloseCount, MAX(PH.CreationDate) AS LatestCloseDate FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId)
// SELECT U.DisplayName, U.Reputation, U.NetVotes, COALESCE(TP.PostId, 0) AS TopPostId, COALESCE(TP.Title, 'No Posts') AS TopPostTitle, COALESCE(TP.Score, 0) AS TopPostScore,
//        COALESCE(TP.ViewCount, 0) AS TopPostViewCount, CP.CloseCount, CP.LatestCloseDate
// FROM UserScores U LEFT JOIN TopPosts TP ON U.UserId = TP.OwnerUserId AND TP.PostRank = 1 LEFT JOIN ClosedPostHistory CP ON TP.PostId = CP.PostId
// WHERE U.ReputationRank <= 10 ORDER BY U.Reputation DESC, U.DisplayName;
fn q3774(db: &'static So) -> String {
    let rr = ranked(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(rr.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let lv = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&lv).map(|(u, _)| u).inv().select(&lv).collect();
    let PostHistory { post_history_type_id, post, creation_date: hd, .. } = &db.post_history;
    let cph = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tu).select(Ident::<User>::new().and((&by_user).map(|(_, p)| p).select(Ident::<Post>::new().and((&cph).opt())).opt())));
    rows(v.into_iter().map(|(u, (_, t))| {
        let (up, dn) = (db.user.up_votes.get(u).unwrap(), db.user.down_votes.get(u).unwrap());
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(up - dn)];
        match t {
            Some((p, c)) => {
                f.extend([V::I(db.post.origid.get(p).unwrap()), V::S(db.post.title.get(p).unwrap_or("No Posts")), V::I(score.get(p).unwrap()), V::I(db.post.view_count.get(p).unwrap_or(0))]);
                f.extend(match c {
                    Some((n, d)) => [V::I(n), V::T(d)],
                    None => [V::Null, V::Null],
                });
            }
            None => f.extend([V::I(0), V::S("No Posts"), V::I(0), V::I(0), V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount FROM RankedPosts rp WHERE rp.ScoreRank <= 10),
// UserPostCount AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount FROM Posts p GROUP BY p.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadge FROM Badges b GROUP BY b.UserId),
// Aggregated AS (SELECT upc.OwnerUserId, upc.PostCount, ub.BadgeCount, ub.HighestBadge, tp.ViewCount AS TopPostViewCount, tp.AnswerCount AS TopPostAnswerCount
//     FROM UserPostCount upc LEFT JOIN UserBadges ub ON upc.OwnerUserId = ub.UserId LEFT JOIN TopPosts tp ON upc.OwnerUserId = tp.PostId)
// SELECT u.Id AS UserId, u.DisplayName, agg.PostCount, COALESCE(agg.BadgeCount, 0) AS TotalBadges, COALESCE(agg.TopPostViewCount, 0) AS MostViewedPost,
//        COALESCE(agg.TopPostAnswerCount, 0) AS MostAnsweredPost
// FROM Users u LEFT JOIN Aggregated agg ON u.Id = agg.OwnerUserId WHERE u.Reputation > 1000 ORDER BY u.Reputation DESC, agg.PostCount DESC;
//
// upc.OwnerUserId = tp.PostId compares a user id with a post id, so it goes through origid.
fn q6430(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tp = rel(top.into_iter().map(|(p, _)| (origid.get(p).unwrap(), p)).collect());
    let by_oid: HashIdx<i64, (i64, Id<Post>)> = (&tp).map(|(o, _)| o).inv().select(&tp).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let agg = (&pc).and((&bc).opt()).and((&db.user.origid).select(&by_oid).map(|(_, p)| p).opt());
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select(agg.opt()));
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        match a {
            Some(((n, b), p)) => f.extend([
                V::I(n),
                V::I(b.unwrap_or(0)),
                V::I(p.and_then(|p| db.post.view_count.get(p)).unwrap_or(0)),
                V::I(p.and_then(|p| db.post.answer_count.get(p)).unwrap_or(0)),
            ]),
            None => f.extend([V::Null, V::I(0), V::I(0), V::I(0)]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.Score > 0),
// TagStatistics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// RecentPostHistory AS (SELECT ph.PostId, ph.UserDisplayName, ph.CreationDate, ph.Comment, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 month')
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, ts.TagName, ts.PostCount, ts.TotalScore, rph.UserDisplayName AS LastEditor,
//        rph.Comment AS LastEditComment, rph.CreationDate AS LastEditDate
// FROM RankedPosts rp LEFT JOIN TagStatistics ts ON ts.PostCount > 0 LEFT JOIN RecentPostHistory rph ON rph.PostId = rp.PostId AND rph.HistoryRank = 1
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// The first ON names only ts, so the ranked posts are crossed with the tags that pass it. tag_mentions has each (post, tag) once, so its row count is the distinct post count.
fn q30173(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(date(2023, 10, 1)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ts = tag_stats(db);
    let tsv = left_all(drain((&ts).filt(|a| a[0] > 0)));
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let ph = drain(db.post_history.with(hd.ge(add_months(date(2024, 10, 1), -1))).select(post));
    let ph = top_per(ph, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let ph = rel(ph.into_iter().map(|(h, p)| (p, h)).collect());
    let last: HashIdx<Id<Post>, (Id<Post>, Id<PostHistory>)> = (&ph).map(|(p, _)| p).inv().select(&ph).collect();
    let mut v = Vec::new();
    (&rp).select(Ident::<Post>::new().and((&last).map(|(_, h)| h).opt())).cross(&tsv).drive(|(p, _), ((_, h), t)| v.push((p, h, t)));
    rows(v.into_iter().map(|(p, h, t)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(match t {
            Some((t, a)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[3])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match h {
            Some(h) => [ostr(db.post_history.user_display_name.get(h)), ostr(db.post_history.comment.get(h)), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId AND v.VoteTypeId = 8 GROUP BY u.Id, u.Reputation),
// PostHistoryDetails AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.PostId),
// CommentStats AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, AVG(c.Score) AS AvgScore FROM Comments c GROUP BY c.PostId)
// SELECT rp.Title, rp.CreationDate, u.Id AS OwnerUserId, u.DisplayName, ur.Reputation, ur.TotalBounties, COALESCE(phd.EditCount, 0) AS EditCount, phd.LastEditedDate,
//        COALESCE(cs.CommentCount, 0) AS CommentCount, COALESCE(cs.AvgScore, 0) AS AvgScore,
//        CASE WHEN ur.Reputation > 1000 THEN 'High' WHEN ur.Reputation BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'Low' END AS ReputationLevel
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN PostHistoryDetails phd ON rp.Id = phd.PostId
// LEFT JOIN CommentStats cs ON rp.Id = cs.PostId WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC LIMIT 50;
fn q4771(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let last = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let last = top_n(last, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 50);
    let rp: MatSet<Id<Post>> = rel(last.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ur = db.user.group_by(Ident::<User>::new()).select(bounty.opt()).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5]))).select(&db.post_history.creation_date);
    let phd = (&rp).group_by(Ident::<Post>::new()).select(edits).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let cs = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let mut v = drain((&rp).select(owner_user.select(Ident::<User>::new().and(&ur)).and((&phd).opt()).and((&cs).opt())));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (((u, b), h), c))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.push(V::I(b));
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        f.extend(match c {
            Some(c) => [V::I(c[0]), avg(c[1], c[0])],
            None => [V::I(0), V::F(0.0)],
        });
        f.push(V::S(if r > 1000 { "High" } else if r >= 500 { "Medium" } else { "Low" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// HotQuestions AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount FROM RankedPosts WHERE Rank = 1 AND CommentCount > 5 AND Score > 10),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT h.PostId) AS PostsEdited FROM Users u LEFT JOIN Votes v ON v.UserId = u.Id LEFT JOIN PostHistory h ON h.UserId = u.Id GROUP BY u.Id, u.DisplayName)
// SELECT hq.Title, hq.CreationDate, hq.Score, hq.ViewCount, hq.OwnerDisplayName, ue.DisplayName AS EngagingUser, ue.UpVotes, ue.DownVotes, ue.PostsEdited
// FROM HotQuestions hq JOIN UserEngagement ue ON ue.UpVotes > 0 OR ue.DownVotes > 0 ORDER BY hq.Score DESC, hq.ViewCount DESC LIMIT 10;
//
// Rank partitions by p.Id, one row per group, so it is always 1. The ON names only ue, so the two are crossed.
fn q8017(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let hq = db
        .post
        .with(post_type_id.is_in([1, 2]).and(score.gt(10)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt())
        .fold(0i64, |n, c| n + c.is_some() as i64);
    let hist_by: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and((&hist_by).select(&db.post_history.post).opt()))
        .buf_fold(|v| {
            let mut ps: Vec<Id<Post>> = v.iter().filter_map(|x| x.1).collect();
            ps.sort_unstable();
            ps.dedup();
            [v.iter().filter(|x| x.0 == Some(2)).count() as i64, v.iter().filter(|x| x.0 == Some(3)).count() as i64, ps.len() as i64]
        });
    let uev = rel(drain((&ue).filt(|a| a[0] > 0 || a[1] > 0)));
    let mut v = Vec::new();
    (&hq).filt(|n| n > 5).cross(&uev).drive(|(p, _), (_, (u, a))| v.push((p, u, a)));
    let v = top_n(v, |&(p, _, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(p, u, a)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS BadgeCount, MAX(U.CreationDate) AS AccountCreationDate FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// PostActivity AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, MAX(P.LastActivityDate) AS LastPostActivity FROM Posts P
//     WHERE P.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') GROUP BY P.OwnerUserId),
// CommentStats AS (SELECT C.UserId, COUNT(C.Id) AS CommentCount, SUM(C.Score) AS TotalCommentScore FROM Comments C GROUP BY C.UserId),
// UserPostSummary AS (SELECT UR.UserId, UR.Reputation, COALESCE(PA.PostCount, 0) AS PostCount, COALESCE(PA.TotalScore, 0) AS TotalPostScore, COALESCE(CS.CommentCount, 0) AS CommentCount,
//        COALESCE(CS.TotalCommentScore, 0) AS TotalCommentScore, UR.BadgeCount, UR.AccountCreationDate
//     FROM UserReputation UR LEFT JOIN PostActivity PA ON UR.UserId = PA.OwnerUserId LEFT JOIN CommentStats CS ON UR.UserId = CS.UserId)
// SELECT UDS.UserId, UDS.Reputation, UDS.PostCount, UDS.TotalPostScore, UDS.CommentCount, UDS.TotalCommentScore, UDS.BadgeCount, UDS.AccountCreationDate,
//        RANK() OVER (ORDER BY UDS.Reputation DESC) AS ReputationRank
// FROM UserPostSummary UDS WHERE UDS.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY UDS.Reputation DESC, UDS.PostCount ASC;
fn q279(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let (s, n) = db.user.select(rep).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let mean = s as f64 / n as f64;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { creation_date, score, .. } = &db.post;
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(recent).select(score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let cs = db.user.group_by(Ident::<User>::new()).select(comments_by(db).select(&db.comment.score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = drain(db.user.with(rep.filt(|r| r as f64 > mean)).select((&bc).and((&pa).opt()).and((&cs).opt())));
    let v = ranked(v, |&(u, _)| Reverse(rep.get(u).unwrap()), false);
    rows(v.into_iter().map(|((u, ((b, p), c)), r)| {
        let p = p.unwrap_or([0, 0]);
        let c = c.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(p[0]), V::I(p[1]), V::I(c[0]), V::I(c[1]), V::I(b), user_col(db, u, "ucreated"), V::I(r)]);
        row(f)
    }))
}

// WITH UserStat AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score <= 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, PositivePosts, NegativePosts, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStat),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS Owner, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1)
// SELECT tu.DisplayName AS UserName, tu.Reputation, tu.PostCount, tu.PositivePosts, tu.NegativePosts, pp.PostId, pp.Title, pp.Score, pp.ViewCount
// FROM TopUsers tu LEFT JOIN PopularPosts pp ON tu.PostCount > 0 WHERE tu.ReputationRank <= 10 AND (pp.Score > 10 OR pp.ViewCount > 1000) ORDER BY tu.Reputation DESC, pp.Score DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone. The ON names only tu, so the
// users with posts are crossed with the questions; a user without posts gets the NULL row, which the WHERE then drops.
fn q2151(db: &'static So) -> String {
    let rr = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(rr.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(score).and(votes_of(db).opt())).opt())
        .buf_fold(|v| {
            let mut ps: Vec<Id<Post>> = v.iter().filter_map(|x| x.map(|x| x.0 .0)).collect();
            ps.sort_unstable();
            ps.dedup();
            [ps.len() as i64, v.iter().filter(|x| matches!(x, Some(((_, s), _)) if *s > 0)).count() as i64, v.iter().filter(|x| matches!(x, Some(((_, s), _)) if *s <= 0)).count() as i64]
        });
    let pp: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1)).with(owner_user).with(score.gt(10).or(view_count.gt(1000))).collect();
    let mut v = Vec::new();
    (&us).filt(|a| a[0] > 0).cross(&pp).drive(|(u, p), (a, _)| v.push((u, a, p)));
    v.sort_by_key(|&(u, _, p)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(u, a, p)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.Tags FROM RankedPosts rp WHERE rp.Rank <= 10),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.AnswerCount, trp.CommentCount, trp.Tags, ua.UserId, ua.DisplayName, ua.Reputation, ua.PostCount, ua.Upvotes, ua.Downvotes
// FROM TopRankedPosts trp JOIN UserActivity ua ON trp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = ua.UserId) ORDER BY trp.Score DESC, ua.Reputation DESC;
//
// The IN subquery says trp's post is owned by ua's user.
fn q6628(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let trp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ua = db
        .user
        .with((&db.user.creation_date).ge(add_months(current_date(), -6)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 3], |a, x| match x {
            Some(t) => [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let mut v = drain((&trp).select(owner_user.select(Ident::<User>::new().and(&ua))));
    v.sort_by_key(|&(p, (u, _))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(p, (u, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "tags"]);
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.OwnerPostRank <= 5),
// PostVotes AS (SELECT v.PostId, vt.Name AS VoteType, COUNT(v.Id) AS VoteCount FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//     WHERE v.PostId IN (SELECT PostId FROM TopPosts) GROUP BY v.PostId, vt.Name),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.OwnerDisplayName, COALESCE(SUM(CASE WHEN pv.VoteType = 'UpMod' THEN pv.VoteCount ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN pv.VoteType = 'DownMod' THEN pv.VoteCount ELSE 0 END), 0) AS DownVotes
//     FROM TopPosts tp LEFT JOIN PostVotes pv ON tp.PostId = pv.PostId GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.OwnerDisplayName)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.Score, pd.OwnerDisplayName, pd.UpVotes, pd.DownVotes, (pd.UpVotes - pd.DownVotes) AS NetVotes
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
fn q9692(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).select(votes_of(db)).map(|v| v).group_by(Same::<Id<Vote>>::new().select(&db.vote.post).and(vtype_name(db))).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let pvv = rel(drain(&pv));
    let pv_of: HashIdx<Id<Post>, ((Id<Post>, Str), i64)> = (&pvv).map(|((p, _), _)| p).inv().select(&pvv).collect();
    let pd = (&tp).group_by(Ident::<Post>::new()).select((&pv_of).opt()).fold([0i64; 2], |a, x| match x {
        Some(((_, n), c)) => [a[0] + if n == "UpMod" { c } else { 0 }, a[1] + if n == "DownMod" { c } else { 0 }],
        None => a,
    });
    rows(drain(&pd).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.Tags, p.Score, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// MostActiveUsers AS (SELECT OwnerUserId, COUNT(*) AS PostsCount FROM Posts WHERE CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY OwnerUserId)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.BadgeCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges, ra.PostId, ra.Title, ra.Body, ra.CreationDate, ra.Score, ra.AnswerCount, mau.PostsCount
// FROM UserStats us JOIN RecentPosts ra ON us.UserId = ra.OwnerUserId AND ra.rn = 1 JOIN MostActiveUsers mau ON us.UserId = mau.OwnerUserId
// WHERE us.Reputation >= 1000 ORDER BY mau.PostsCount DESC, us.Reputation DESC LIMIT 10;
fn q27824(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let last = top_per(drain(db.post.with(creation_date.ge(add_days(t0, -30))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let lv = rel(last.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&lv).map(|(u, _)| u).inv().select(&lv).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let mau = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with((&db.user.reputation).ge(1000)).select((&ub).and((&by_user).map(|(_, p)| p)).and(&mau)));
    let v = top_n(v, |&(u, (_, m))| (Reverse(m), Reverse(db.user.reputation.get(u).unwrap())), 10);
    rows(v.into_iter().map(|(u, ((b, p), m))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "body", "created", "score", "answers"]));
        f.push(V::I(m));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount FROM Posts p GROUP BY p.OwnerUserId),
// UsersWithPosts AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(bs.BadgeCount, 0) AS BadgeCount,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(ps.TotalPosts, 0) DESC, COALESCE(bs.BadgeCount, 0) DESC) AS Rank
//     FROM Users u LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId LEFT JOIN UserBadges bs ON u.Id = bs.UserId)
// SELECT uwp.UserId, uwp.DisplayName, uwp.TotalPosts, uwp.BadgeCount, COALESCE(ps.TotalScore, 0) AS TotalPostScore, ps.AvgViewCount,
//        CASE WHEN uwp.TotalPosts > 100 THEN 'Veteran' WHEN uwp.BadgeCount > 50 THEN 'Expert' ELSE 'Novice' END AS UserLevel
// FROM UsersWithPosts uwp LEFT JOIN PostStatistics ps ON uwp.UserId = ps.OwnerUserId WHERE uwp.TotalPosts > 0 ORDER BY uwp.Rank LIMIT 10;
fn q1737(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let v = top_n(drain((&ups).filt(|a| a[1] > 0).and(&bc)), |&(_, (a, b))| (Reverse(a[1]), Reverse(b)), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(b), V::I(a[4]), avg(a[6], a[5]), V::S(if a[1] > 100 { "Veteran" } else if b > 50 { "Expert" } else { "Novice" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, u.DisplayName AS OwnerDisplayName
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' AND p.ViewCount > 100),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, Rank, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments FROM Comments c GROUP BY c.PostId),
// PostBadges AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges FROM Badges b GROUP BY b.UserId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, COALESCE(pc.TotalComments, 0) AS TotalComments, tp.OwnerDisplayName, pb.TotalBadges
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId LEFT JOIN PostBadges pb ON tp.OwnerDisplayName = (SELECT u.DisplayName FROM Users u WHERE u.Id = pb.UserId)
// ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q6687(db: &'static So) -> String {
    let Post { post_type, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(view_count.gt(100)).with(owner_user).select(post_type.select(&db.post_type.name)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pb = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pbv = rel(drain(&pb));
    let by_name: HashIdx<Str, (i64, i64)> = (&pbv).map(|(u, _)| u).select(&uidx).select(&db.user.display_name).inv().select(&pbv).collect();
    let v = drain((&cc).and(owner_user.select(&db.user.display_name).select(&by_name).opt()));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([V::I(c), post_fields(db, p, &["owner"]).remove(0), b.map_or(V::Null, |b| V::I(b.1))]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(a.AnswerCount, 0) AS AnswerCount, COALESCE(c.CommentCount, 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.AnswerCount, ps.CommentCount, ur.Reputation, ur.ReputationRank,
//        CASE WHEN ps.ViewCount < 100 THEN 'Low Views' WHEN ps.ViewCount BETWEEN 100 AND 1000 THEN 'Moderate Views' ELSE 'High Views' END AS ViewCategory,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = ps.PostId AND v.VoteTypeId = 2) AS UpvoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = ps.PostId AND v.VoteTypeId = 3) AS DownvoteCount
// FROM PostStats ps JOIN UserReputation ur ON ps.PostId = ur.UserId WHERE ps.UserPostRank <= 5 ORDER BY ps.Score DESC, ps.CreationDate DESC;
//
// ps.PostId = ur.UserId compares a post id with a user id, so it goes through origid.
fn q679(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, origid, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let ps: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, r), k)| (db.user.origid.get(u).unwrap(), (r, k))).collect());
    let by_uid: HashIdx<i64, (i64, (i64, i64))> = (&rr).map(|(o, _)| o).inv().select(&rr).collect();
    let joined = || (&ps).with(origid.select(&by_uid));
    let ac = joined().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let cc = joined().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = joined().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&ac).and(&cc).and(&vc).and(origid.select(&by_uid)));
    rows(v.into_iter().map(|(p, (((a, c), u), (_, (r, k))))| {
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a), V::I(c), V::I(r), V::I(k)]);
        f.push(V::S(match w {
            Some(w) if w < 100 => "Low Views",
            Some(w) if w <= 1000 => "Moderate Views",
            _ => "High Views",
        }));
        f.extend([V::I(u[0]), V::I(u[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.OwnerDisplayName, pd.CommentCount, pd.UpVoteCount, pd.DownVoteCount, (pd.UpVoteCount - pd.DownVoteCount) AS NetVotes
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
//
// Each COUNT(DISTINCT) undoes its own fan-out, so each comes from a fold over one row per child.
fn q9193(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(u[0]), V::I(u[1]), V::I(u[0] - u[1])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews,
//        RANK() OVER (ORDER BY TotalUpVotes DESC) AS RankByUpVotes FROM UserPostStats WHERE PostCount > 0)
// SELECT u.UserId, u.DisplayName, u.PostCount, u.QuestionCount, u.AnswerCount, u.TotalViews, u.TotalUpVotes, u.TotalDownVotes, u.RankByViews, u.RankByUpVotes,
//        CASE WHEN u.RankByViews <= 10 THEN 'Top 10 by Views' ELSE 'Below Top 10 by Views' END AS ViewsRanking,
//        CASE WHEN u.RankByUpVotes <= 10 THEN 'Top 10 by Upvotes' ELSE 'Below Top 10 by Upvotes' END AS UpVotesRanking
// FROM TopUsers u WHERE u.QuestionCount > 5 AND u.TotalUpVotes > 10 ORDER BY u.TotalViews DESC, u.TotalUpVotes DESC;
fn q6647(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 6], |a, ((t, w), vt)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + (vt == Some(2)) as i64, a[5] + (vt == Some(3)) as i64]);
    let v = ranked(drain(&us), |&(_, a)| Reverse(a[3]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[4]), false);
    let v = drain(rel(v).filt(|(((_, a), _), _)| a[1] > 5 && a[4] > 10));
    rows(v.into_iter().map(|(_, (((u, a), rv), ru))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(rv), V::I(ru)]);
        f.push(V::S(if rv <= 10 { "Top 10 by Views" } else { "Below Top 10 by Views" }));
        f.push(V::S(if ru <= 10 { "Top 10 by Upvotes" } else { "Below Top 10 by Upvotes" }));
        row(f)
    }))
}

// WITH RECURSIVE UserPostScores AS (SELECT U.Id AS UserId, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, RANK() OVER (ORDER BY SUM(COALESCE(P.Score, 0)) DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// CommentStatistics AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount, AVG(C.Score) AS AvgCommentScore FROM Comments C GROUP BY C.PostId),
// UserBadgeCount AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B GROUP BY B.UserId)
// SELECT U.DisplayName, UPS.PostCount, UPS.TotalScore, UPS.UserRank, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate, CS.CommentCount, CS.AvgCommentScore,
//        COALESCE(UBC.BadgeCount, 0) AS BadgeCount
// FROM Users U LEFT JOIN UserPostScores UPS ON U.Id = UPS.UserId LEFT JOIN RecentPosts RP ON U.Id = RP.OwnerUserId AND RP.RecentRank = 1
// LEFT JOIN CommentStatistics CS ON RP.PostId = CS.PostId LEFT JOIN UserBadgeCount UBC ON U.Id = UBC.UserId WHERE UPS.TotalScore > 0 ORDER BY UPS.TotalScore DESC, UPS.PostCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q34823(db: &'static So) -> String {
    let ups = user_posts(db);
    let ur = rel(ranked(drain(&ups), |&(_, a)| Reverse(a[4]), false).into_iter().map(|((u, a), r)| (u, (a, r))).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, ([i64; 10], i64))> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let Post { owner_user, creation_date, .. } = &db.post;
    let recent = top_per(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let rv = rel(recent.into_iter().map(|(p, u)| (u, p)).collect());
    let rp_of: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rv).map(|(u, _)| u).inv().select(&rv).collect();
    let cs = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.select((&by_user).map(|(_, x)| x).filt(|(a, _)| a[4] > 0).and((&rp_of).map(|(_, p)| p).select(Ident::<Post>::new().and((&cs).opt())).opt()).and((&bc).opt())));
    rows(v.into_iter().map(|(u, (((a, r), p), b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[4]), V::I(r)];
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["title", "created"]));
                f.extend(match c {
                    Some(c) => [V::I(c[0]), avg(c[1], c[0])],
                    None => [V::Null, V::Null],
                });
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH RecursiveUserVotes AS (SELECT U.Id AS UserId, U.Reputation, COALESCE(V.VoteTypeId, 0) AS VoteTypeId, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY V.CreationDate DESC) AS VoteRank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId),
// ActiveUsers AS (SELECT UserId, Reputation, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM RecursiveUserVotes WHERE VoteRank = 1 GROUP BY UserId, Reputation),
// PostsWithComments AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, COALESCE(C.CommentCount, 0) AS CommentCount FROM Posts P
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId),
// EligiblePosts AS (SELECT PW.PostId, PW.Title, PW.OwnerUserId, COALESCE(U.Reputation, 0) AS OwnerReputation, PW.CommentCount FROM PostsWithComments PW
//     JOIN ActiveUsers U ON PW.OwnerUserId = U.UserId WHERE U.Reputation > 1000),
// RankedEligiblePosts AS (SELECT EP.*, RANK() OVER (ORDER BY EP.CommentCount DESC, EP.OwnerReputation DESC) AS Rank FROM EligiblePosts EP)
// SELECT REP.PostId, REP.Title, REP.OwnerUserId, REP.OwnerReputation, REP.CommentCount, REP.Rank FROM RankedEligiblePosts REP WHERE REP.Rank <= 10 ORDER BY REP.Rank;
//
// ActiveUsers has exactly one row per user (the VoteRank = 1 row of Users LEFT JOIN Votes), and its vote counts are never read.
fn q31052(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let ep = db.post.with(owner_user.select(rich)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&ep).and(owner_user.select(&db.user.reputation))), |&(_, (c, r))| (Reverse(c), Reverse(r)), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (c, r)), k)| {
        let mut f = post_fields(db, p, &["id", "title", "owner_id"]);
        f.extend([V::I(r), V::I(c), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, MAX(u.Reputation) AS Reputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Upvotes, rp.Downvotes, us.UserId, us.DisplayName, us.BadgeCount, us.Reputation,
//        CASE WHEN rp.Upvotes - rp.Downvotes > 0 THEN 'Positive' WHEN rp.Upvotes - rp.Downvotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteStatus
//     FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.rn <= 5)
// SELECT fp.PostId, fp.Title, fp.Upvotes, fp.Downvotes, fp.DisplayName, fp.BadgeCount, fp.Reputation, fp.VoteStatus,
//        CASE WHEN EXISTS (SELECT 1 FROM Comments c WHERE c.PostId = fp.PostId) THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus
// FROM FilteredPosts fp ORDER BY fp.Reputation DESC, fp.Upvotes DESC;
fn q1827(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let commented: MatSet<Id<Post>> = db.comment.select(&db.comment.post).collect();
    let v = drain((&vs).and(owner_user.select(Ident::<User>::new().and(&bc))).and(Ident::<Post>::new().with(&commented).opt()));
    rows(v.into_iter().map(|(p, ((a, (u, b)), c))| {
        let n = a[0] - a[1];
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), user_col(db, u, "name"), V::I(b), user_col(db, u, "rep")]);
        f.push(V::S(if n > 0 { "Positive" } else if n < 0 { "Negative" } else { "Neutral" }));
        f.push(V::S(if c.is_some() { "Has Comments" } else { "No Comments" }));
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, AVG(P.ViewCount) AS AverageViewCount, SUM(P.Score) AS TotalScore
//     FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%') WHERE P.PostTypeId = 1 GROUP BY T.TagName),
// HighScoringTags AS (SELECT TagName, PostCount, AverageViewCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM TagStatistics WHERE PostCount > 10),
// TopTags AS (SELECT TagName, PostCount, AverageViewCount, TotalScore FROM HighScoringTags WHERE ScoreRank <= 10),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionsAsked, COUNT(DISTINCT C.Id) AS CommentsMade, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Comments C ON U.Id = C.UserId
//     LEFT JOIN Votes V ON U.Id = V.UserId AND V.PostId IN (SELECT Id FROM Posts WHERE PostTypeId = 1) GROUP BY U.Id, U.DisplayName)
// SELECT T.TagName, T.PostCount, T.AverageViewCount, T.TotalScore, U.UserId, U.DisplayName, U.QuestionsAsked, U.CommentsMade, U.UpVotesReceived
// FROM TopTags T JOIN UserActivity U ON U.QuestionsAsked > 0 ORDER BY T.TotalScore DESC, U.UpVotesReceived DESC;
//
// The ON names only U, so the top tags are crossed with the users who asked something; UserActivity is folded only for those users.
fn q25434(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let asked = || Ident::<Post>::new().with(post_type_id.eq(1));
    let lt = tag_mentions(db);
    type R = (Id<Post>, Id<Tag>);
    let ts = (&lt)
        .group_by(Same::<R>::new().map(|(_, t): R| t).select(&db.tag.tag_name))
        .select(Same::<R>::new().map(|(p, _): R| p).select(asked()).select(view_count.opt().and(score)))
        .fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let v = ranked(drain((&ts).filt(|a| a[0] > 10)), |&(_, a)| Reverse(a[3]), false);
    let tt = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let askers: MatSet<Id<User>> = db.post.with(post_type_id.eq(1)).select(&db.post.owner_user).collect();
    let on_q = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.post).select(asked())));
    let ua = (&askers)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(asked()).and(comments_by(db).opt()).and(on_q.select(&db.vote.vote_type_id).opt()))
        .buf_fold(|v| {
            let mut ps: Vec<Id<Post>> = v.iter().map(|x| x.0 .0).collect();
            let mut cs: Vec<Id<Comment>> = v.iter().filter_map(|x| x.0 .1).collect();
            ps.sort_unstable();
            ps.dedup();
            cs.sort_unstable();
            cs.dedup();
            [ps.len() as i64, cs.len() as i64, v.iter().filter(|x| x.1 == Some(2)).count() as i64]
        });
    let mut v = Vec::new();
    (&tt).cross(&ua).drive(|(_, u), ((t, a), b)| v.push((t, a, u, b)));
    rows(v.into_iter().map(|(t, a, u, b)| {
        let mut f = vec![V::S(t), V::I(a[0]), avg(a[2], a[1]), V::I(a[3])];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, MAX(p.CreationDate) AS LastPostDate FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostVoteStats AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// RankedPosts AS (SELECT ps.PostId, ps.VoteCount, ps.UpVotes, ps.DownVotes, ROW_NUMBER() OVER (PARTITION BY ps.PostId ORDER BY ps.VoteCount DESC) AS rn FROM PostVoteStats ps)
// SELECT ups.UserId, ups.DisplayName, ups.Reputation, ups.TotalPosts, ups.Questions, ups.Answers, ups.LastPostDate, COALESCE(rp.PostId, -1) AS TopVotedPostId,
//        COALESCE(rp.VoteCount, 0) AS TopVotedPostVoteCount, COALESCE(rp.UpVotes, 0) AS TopVotedPostUpVotes, COALESCE(rp.DownVotes, 0) AS TopVotedPostDownVotes
// FROM UserPostStats ups LEFT JOIN RankedPosts rp ON ups.UserId = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = rp.PostId AND rp.rn = 1)
// WHERE ups.Reputation > 100 ORDER BY ups.Reputation DESC, ups.TotalPosts DESC;
//
// rn partitions by PostId, one row each, so it is always 1; the ON clause joins each user to the posts they own.
fn q3031(db: &'static So) -> String {
    let ups = user_posts(db);
    let pvs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select((&ups).and(posts_of(db).select(Ident::<Post>::new().and(&pvs)).opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), tmax(a[7])]);
        f.extend(match p {
            Some((p, s)) => [V::I(db.post.origid.get(p).unwrap()), V::I(s[0]), V::I(s[1]), V::I(s[2])],
            None => [V::I(-1), V::I(0), V::I(0), V::I(0)],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentActivity AS (SELECT U.Id AS UserId, MAX(P.CreationDate) AS LastPostDate, MAX(C.CreationDate) AS LastCommentDate FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id),
// CombinedStats AS (SELECT Us.UserId, Us.DisplayName, Us.Reputation, Us.TotalPosts, Us.TotalComments, Us.TotalUpVotes, Us.TotalDownVotes, Ra.LastPostDate, Ra.LastCommentDate,
//        DENSE_RANK() OVER (ORDER BY Us.Reputation DESC) AS ReputationRank FROM UserStats Us JOIN RecentActivity Ra ON Us.UserId = Ra.UserId)
// SELECT C.UserId, C.DisplayName, C.Reputation, C.TotalPosts, C.TotalComments, C.TotalUpVotes, C.TotalDownVotes, C.LastPostDate, C.LastCommentDate, C.ReputationRank
// FROM CombinedStats C WHERE C.TotalPosts > 10 AND C.Reputation > 1000 ORDER BY C.ReputationRank;
//
// ReputationRank reads only Reputation, so the ranks are taken over all users and the products are driven only for users over 1000.
fn q5634(db: &'static So) -> String {
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true).into_iter().map(|((u, _), k)| (u, k)).collect());
    let rank_of: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let us = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .buf_fold(|v| {
            let mut ps: Vec<Id<Post>> = v.iter().filter_map(|x| x.map(|x| x.0 .0)).collect();
            let mut cs: Vec<Id<Comment>> = v.iter().filter_map(|x| x.and_then(|x| x.0 .1)).collect();
            ps.sort_unstable();
            ps.dedup();
            cs.sort_unstable();
            cs.dedup();
            let n = |t: i64| v.iter().filter(|x| matches!(x, Some((_, Some(y))) if *y == t)).count() as i64;
            [ps.len() as i64, cs.len() as i64, n(2), n(3)]
        });
    let Post { creation_date, .. } = &db.post;
    let ra = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(creation_date.and(comments_of(db).select(&db.comment.creation_date).opt())).opt())
        .fold([i64::MIN; 2], |a, x| match x {
            Some((d, c)) => [a[0].max(d), c.map_or(a[1], |c| a[1].max(c))],
            None => a,
        });
    let mut v = drain((&us).filt(|a| a[0] > 10).and(&ra).and((&rank_of).map(|(_, k)| k)));
    v.sort_by_key(|&(_, (_, k))| k);
    rows(v.into_iter().map(|(u, ((a, d), k))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([tmax(d[0]), tmax(d[1]), V::I(k)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerPosts,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// ActiveUsers AS (SELECT UserId, TotalPosts, QuestionPosts, AnswerPosts, TotalScore FROM UserPostStats WHERE TotalPosts > 10 AND TotalScore > 50),
// TopBadges AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Class = 1 GROUP BY UserId),
// UserRanking AS (SELECT au.UserId, au.TotalPosts, au.QuestionPosts, au.AnswerPosts, au.TotalScore, COALESCE(tb.BadgeCount, 0) AS GoldBadges FROM ActiveUsers au LEFT JOIN TopBadges tb ON au.UserId = tb.UserId),
// RankedUsers AS (SELECT UserId, TotalPosts, QuestionPosts, AnswerPosts, TotalScore, GoldBadges, RANK() OVER (ORDER BY TotalScore DESC, TotalPosts DESC) AS UserRank FROM UserRanking)
// SELECT u.DisplayName, ru.TotalPosts, ru.QuestionPosts, ru.AnswerPosts, ru.TotalScore, ru.GoldBadges, ru.UserRank FROM RankedUsers ru JOIN Users u ON ru.UserId = u.Id
// WHERE ru.UserRank <= 10 ORDER BY ru.UserRank, ru.TotalScore DESC;
fn q6779(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score)).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }],
            None => [a[0] + 1, a[1], a[2], a[3]],
        });
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let gb = db.user.group_by(Ident::<User>::new()).select(gold).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&ups).filt(|a| a[0] > 10 && a[3] > 50).and((&gb).opt())), |&(_, (a, _))| (Reverse(a[3]), Reverse(a[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, g)), r)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([V::I(g.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.ViewCount, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank, p.PostTypeId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopQuestions AS (SELECT PostID, Title, OwnerDisplayName, CreationDate, ViewCount, Score FROM RankedPosts WHERE Rank <= 10 AND PostTypeId = 1),
// TopAnswers AS (SELECT p.Id AS AnswerID, p.Title, u.DisplayName AS AnswererDisplayName, p.CreationDate, p.ViewCount, p.Score,
//        (SELECT Title FROM Posts WHERE Id = p.AcceptedAnswerId) AS AcceptedQuestionTitle FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL)
// SELECT tq.Title AS QuestionTitle, tq.OwnerDisplayName AS QuestionOwner, tq.CreationDate AS QuestionCreationDate, tq.ViewCount AS QuestionViewCount, tq.Score AS QuestionScore,
//        ta.AnswererDisplayName AS AcceptedAnswerer, ta.CreationDate AS AnswerCreationDate, ta.ViewCount AS AnswerViewCount, ta.Score AS AnswerScore, ta.AcceptedQuestionTitle
// FROM TopQuestions tq LEFT JOIN TopAnswers ta ON tq.Title = ta.AcceptedQuestionTitle ORDER BY tq.Score DESC, tq.ViewCount DESC;
fn q7129(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, title, accepted_answer, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tq: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_title: HashIdx<Str, Id<Post>> = db.post.with(post_type_id.eq(2)).with(owner_user).select(accepted_answer.select(title)).inv().collect();
    let v = drain((&tq).with(post_type_id.eq(1)).select(title.select(&by_title).opt()));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "views", "score"]);
        f.extend(match a {
            Some(a) => {
                let mut g = post_fields(db, a, &["owner", "created", "views", "score"]);
                g.push(ostr(accepted_answer.get(a).and_then(|q| title.get(q))));
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopQuestions AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.RankByScore <= 10),
// PostStatistics AS (SELECT tq.PostId, tq.Title, tq.OwnerDisplayName, COUNT(c.Id) AS TotalComments, COALESCE(AVG(vote.UserVoteCount), 0) AS AverageVotes
//     FROM TopQuestions tq LEFT JOIN Comments c ON tq.PostId = c.PostId LEFT JOIN (SELECT v.PostId, COUNT(*) AS UserVoteCount FROM Votes v GROUP BY v.PostId) vote ON tq.PostId = vote.PostId
//     GROUP BY tq.PostId, tq.Title, tq.OwnerDisplayName)
// SELECT ps.PostId, ps.Title, ps.OwnerDisplayName, ps.TotalComments, ps.AverageVotes, COUNT(ph.Id) AS HistoricalEdits
// FROM PostStatistics ps LEFT JOIN PostHistory ph ON ps.PostId = ph.PostId WHERE ph.CreationDate >= CURRENT_DATE - INTERVAL '30 days'
// GROUP BY ps.PostId, ps.Title, ps.OwnerDisplayName, ps.TotalComments, ps.AverageVotes ORDER BY ps.AverageVotes DESC, ps.TotalComments DESC;
fn q8928(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10, false);
    let tq: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tq).group_by(Ident::<Post>::new()).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let ps = (&tq).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and((&vc).opt())).fold([0i64; 3], |a, (c, n)| [a[0] + c.is_some() as i64, a[1] + n.is_some() as i64, a[2] + n.unwrap_or(0)]);
    let recent = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).ge(add_days(current_date(), -30))));
    let h = (&tq).group_by(Ident::<Post>::new()).select(recent).fold(0i64, |n, _| n + 1);
    let mean = |a: [i64; 3]| if a[1] == 0 { 0.0 } else { a[2] as f64 / a[1] as f64 };
    let mut v = drain((&ps).and(&h));
    v.sort_by_key(|&(_, (a, _))| (Reverse(fkey(mean(a))), Reverse(a[0])));
    rows(v.into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::F(mean(a)), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ARRAY_LENGTH(string_to_array(p.Tags, '>'), 1) AS TagCount, u.DisplayName AS AuthorName,
//        COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, ARRAY_LENGTH(string_to_array(p.Tags, '>'), 1), u.DisplayName),
// AggregatedResults AS (SELECT RANK() OVER (ORDER BY Score DESC, ViewCount DESC, TagCount DESC) AS Rank, rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount,
//        rp.AuthorName, rp.UpvoteCount, rp.DownvoteCount, CASE WHEN rp.Score > 100 THEN 'High Score' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
//     FROM RankedPosts rp)
// SELECT *, CONCAT('Post Title: ', Title, ' | Author: ', AuthorName, ' | Views: ', ViewCount, ' | Score: ', Score, ' | Comments: ', CommentCount, ' | Score Category: ', ScoreCategory) AS PostSummary
// FROM AggregatedResults WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only base columns, so the top questions are picked first and the comment x vote product is driven for those alone.
fn q26472(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, tags_str, title, .. } = &db.post;
    let tc = |p: Id<Post>| tags_str.get(p).map(|t| t.split('>').count() as i64);
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), tc(p).is_none(), Reverse(tc(p)))
    }, false);
    let rk = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let rank_of: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let tp: MatSet<Id<Post>> = (&rk).map(|(p, _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&s).and((&rank_of).map(|(_, r)| r)));
    v.sort_by_key(|&(_, (_, r))| r);
    rows(v.into_iter().map(|(p, (a, r))| {
        let sc = score.get(p).unwrap();
        let cat = if sc > 100 { "High Score" } else if sc >= 50 { "Medium Score" } else { "Low Score" };
        let author = db.post.owner_user.get(p).map(|u| db.user.display_name.get(u).unwrap());
        let summary = format!(
            "Post Title: {} | Author: {} | Views: {} | Score: {} | Comments: {} | Score Category: {}",
            title.get(p).unwrap_or(""),
            author.unwrap_or(""),
            view_count.get(p).map_or(String::new(), |w| w.to_string()),
            sc,
            a[0],
            cat
        );
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(a[0]), ostr(author), V::I(a[1]), V::I(a[2]), V::S(cat), V::Owned(summary)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id, P.Title, P.ViewCount, P.CreationDate, P.Score, P.AnswerCount, DENSE_RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) as RankByUser
//     FROM Posts P WHERE P.PostTypeId = 1 AND P.ViewCount > 100),
// TopPosts AS (SELECT R.Id, R.Title, R.ViewCount, R.CreationDate, R.Score, R.AnswerCount FROM RankedPosts R WHERE R.RankByUser = 1),
// UserScores AS (SELECT U.Id AS UserId, (SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END)) AS NetScore
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT U.DisplayName, COALESCE(US.NetScore, 0) AS NetScore, COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges,
//        COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, TP.Title, TP.ViewCount, TP.CreationDate
// FROM Users U LEFT JOIN UserScores US ON U.Id = US.UserId LEFT JOIN UserBadges UB ON U.Id = UB.UserId RIGHT JOIN TopPosts TP ON U.Id = TP.Id
// WHERE U.Reputation > 500 ORDER BY NetScore DESC, TP.ViewCount DESC;
//
// U.Id = TP.Id compares a user id with a post id, so it goes through origid; the WHERE drops the unmatched posts the RIGHT JOIN kept.
fn q2192(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, creation_date, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(view_count.gt(100))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let uidx: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(500)).select(&db.user.origid).inv().collect();
    let v = drain((&tp).select(origid.select(&uidx).select(Ident::<User>::new().and(&us).and((&ub).opt()))));
    rows(v.into_iter().map(|(p, ((u, n), b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(post_fields(db, p, &["title", "views", "created"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, COUNT(DISTINCT c.Id) AS CommentCount, p.AcceptedAnswerId,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS Rank, pt.Name AS PostTypeName, u.DisplayName AS OwnerDisplayName,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, pt.Name, u.DisplayName, p.AcceptedAnswerId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.PostTypeName, rp.OwnerDisplayName, rp.UpVotes, rp.DownVotes
//     FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.AnswerCount, fp.CommentCount, fp.PostTypeName, fp.OwnerDisplayName, fp.UpVotes, fp.DownVotes,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = fp.PostId) AS EditCount
// FROM FilteredPosts fp WHERE fp.ViewCount > 1000 ORDER BY fp.ViewCount DESC, fp.Score DESC;
//
// Rank reads only base columns, so the top posts of each type are picked first and the comment x vote product is driven for those alone.
fn q7024(db: &'static So) -> String {
    let Post { post_type, owner_user, score, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(owner_user).select(post_type)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let fp = || (&tp).with(view_count.gt(1000));
    let s = fp()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = fp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ec = fp().group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    rows(drain((&s).and(&cc).and(&ec)).into_iter().map(|(p, ((a, c), e))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(c), tname(db, post_type.get(p).unwrap()), post_fields(db, p, &["owner"]).remove(0), V::I(a[0]), V::I(a[1]), V::I(e)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.ViewCount, p.Score, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') AND p.Score > 0),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.ViewCount, rp.Score, COUNT(c.Id) AS CommentCount FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId
//     WHERE rp.ScoreRank <= 10 GROUP BY rp.PostId, rp.Title, rp.OwnerDisplayName, rp.ViewCount, rp.Score),
// PostLinkStats AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS RelatedPostCount, SUM(CASE WHEN lt.Name = 'Duplicate' THEN 1 ELSE 0 END) AS DuplicateLinks
//     FROM PostLinks pl JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id GROUP BY pl.PostId)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.ViewCount, tp.Score, tp.CommentCount, pls.RelatedPostCount, pls.DuplicateLinks,
//        CASE WHEN tp.Score > 100 THEN 'High Engagement' WHEN tp.Score > 50 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM TopPosts tp LEFT JOIN PostLinkStats pls ON tp.PostId = pls.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q25342(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let lname = (&db.post_link.link_type).select(&db.link_type.name);
    let pls = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).select(lname)).fold([0i64; 2], |a, n| [a[0] + 1, a[1] + (n == "Duplicate") as i64]);
    rows(drain((&cc).and((&pls).opt())).into_iter().map(|(p, (c, l))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "views", "score"]);
        f.push(V::I(c));
        f.extend(match l {
            Some(l) => l.map(V::I),
            None => [V::Null, V::Null],
        });
        f.push(V::S(if s > 100 { "High Engagement" } else if s > 50 { "Moderate Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.ViewCount),
// TopPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, ViewCount, UpVotes, DownVotes, CommentCount FROM RankedPosts WHERE PostRank <= 10),
// UserVotes AS (SELECT v.UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Votes v GROUP BY v.UserId)
// SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.ViewCount, tp.UpVotes, tp.DownVotes, tp.CommentCount, u.DisplayName AS VoterDisplayName, uv.TotalUpVotes, uv.TotalDownVotes
// FROM TopPosts tp JOIN Votes v ON tp.PostId = v.PostId JOIN Users u ON v.UserId = u.Id JOIN UserVotes uv ON u.Id = uv.UserId ORDER BY tp.ViewCount DESC;
//
// PostRank reads only CreationDate, so the ten newest questions are picked first and the vote x comment product is driven for those alone.
fn q8425(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let Vote { user, vote_type_id, .. } = &db.vote;
    let uv = db.vote.group_by(user).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let mut v = drain((&s).and(votes_of(db).select(user.select(Ident::<User>::new().and(&uv)))));
    v.sort_by_key(|&(p, _)| {
        let w = db.post.view_count.get(p);
        (w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (a, (u, t)))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "views"]);
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "name"), V::I(t[0]), V::I(t[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// TopQuestions AS (SELECT PostId, Title, ViewCount, Score FROM RankedPosts WHERE Rank <= 10),
// PostEngagement AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id),
// EnhancedPosts AS (SELECT tp.PostId, tp.Title, tp.ViewCount, tp.Score, pe.CommentCount, pe.UpVoteCount, pe.DownVoteCount,
//        (CASE WHEN pe.UpVoteCount + pe.DownVoteCount > 0 THEN CAST(pe.UpVoteCount AS FLOAT) / (pe.UpVoteCount + pe.DownVoteCount) ELSE 0 END) AS UpvoteRatio
//     FROM TopQuestions tp JOIN PostEngagement pe ON tp.PostId = pe.PostId)
// SELECT ep.PostId, ep.Title, ep.ViewCount, ep.Score, ep.CommentCount, ep.UpVoteCount, ep.DownVoteCount, ep.UpvoteRatio FROM EnhancedPosts ep WHERE ep.UpvoteRatio > 0.5 ORDER BY ep.ViewCount DESC;
fn q31048(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ratio = |a: [i64; 3]| if a[1] + a[2] > 0 { a[1] as f32 / (a[1] + a[2]) as f32 } else { 0.0 };
    let pe = (&tp)
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&pe).filt(|a| ratio(a) as f64 > 0.5));
    v.sort_by_key(|&(p, _)| {
        let w = db.post.view_count.get(p);
        (w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend(a.map(V::I));
        f.push(V::F(ratio(a) as f64));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.Tags FROM RankedPosts rp WHERE rp.Rank <= 10),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COUNT(b.Id) AS TotalBadges, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostUserStats AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.Tags, us.UserId, us.DisplayName, us.TotalBounties, us.TotalBadges, us.TotalScore
//     FROM TopPosts tp JOIN Users u ON tp.Tags LIKE CONCAT('%<', u.DisplayName, '>%') JOIN UserStats us ON u.Id = us.UserId)
// SELECT pus.PostId, pus.Title, pus.Score, pus.ViewCount, pus.TotalBounties, pus.TotalBadges, pus.TotalScore FROM PostUserStats pus ORDER BY pus.Score DESC, pus.ViewCount DESC;
//
// The LIKE is a substring join, run over the top posts' Tags strings; UserStats is folded only for the users that join.
fn q6318(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, tags_str, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let strs: MatSet<Str> = (&tp).select(tags_str).collect();
    let bracketed = |t: Str, n: Str| t.match_indices(n).any(|(i, _)| i > 0 && t.as_bytes()[i - 1] == b'<' && t.as_bytes().get(i + n.len()) == Some(&b'>'));
    let hit: HashIdx<Str, Id<User>> = (&strs).select_where((&db.user.display_name).inv(), bracketed).collect();
    let users: MatSet<Id<User>> = (&strs).select(&hit).collect();
    let us = (&users)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()).and(posts_of(db).select(score).opt()))
        .fold([0i64; 3], |a, ((b, g), s)| [a[0] + b.flatten().unwrap_or(0), a[1] + g.is_some() as i64, a[2] + s.unwrap_or(0)]);
    let mut v = drain((&tp).select(tags_str.select(&hit).select(&us)));
    v.sort_by_key(|&(p, _)| {
        let w = db.post.view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.OwnerDisplayName, rp.Score, rp.ViewCount, rp.AnswerCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostEngagement AS (SELECT tp.Id, tp.Title, tp.OwnerDisplayName, tp.Score, tp.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(v.VoteCount, 0) AS UpvoteCount
//     FROM TopPosts tp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON tp.Id = c.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) v ON tp.Id = v.PostId)
// SELECT pe.Title, pe.OwnerDisplayName, pe.Score, pe.ViewCount, pe.CommentCount, pe.UpvoteCount, (CAST(pe.ViewCount AS FLOAT) / NULLIF(pe.Score, 0)) AS ViewScoreRatio
// FROM PostEngagement pe ORDER BY pe.Score DESC, pe.ViewCount DESC;
fn q6060(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let uc = (&tp).group_by(Ident::<Post>::new()).select(up.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&cc).and(&uc)).into_iter().map(|(p, (c, u))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.extend([V::I(c), V::I(u)]);
        f.push(match view_count.get(p) {
            Some(w) if s != 0 => V::F((w as f32 / s as f32) as f64),
            _ => V::Null,
        });
        row(f)
    }))
}

// WITH TagPostCounts AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.Reputation),
// TopTags AS (SELECT TagName, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagPostCounts),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserReputation)
// SELECT tt.TagName, tt.PostCount AS TotalPosts, tt.QuestionCount AS TotalQuestions, tt.AnswerCount AS TotalAnswers, tu.UserId AS TopUserId, tu.Reputation AS TopUserReputation,
//        tu.PostCount AS TopUserPostCount, tu.QuestionCount AS TopUserQuestionCount, tu.AnswerCount AS TopUserAnswerCount
// FROM TopTags tt JOIN TopUsers tu ON tt.TagRank = 1 AND tu.UserRank = 1 WHERE tt.PostCount > 0 ORDER BY tt.TagName;
//
// The ON names each side separately, so the rank-1 tags are crossed with the rank-1 users. tag_mentions has each (post, tag) once, so its row count is the distinct post count.
fn q9169(db: &'static So) -> String {
    let ts = tag_stats(db);
    let tt = ranked(drain(&ts), |&(_, a)| Reverse(a[0]), false);
    let tt = rel(tt.into_iter().take_while(|x| x.1 == 1).map(|x| x.0).collect());
    let ups = user_posts(db);
    let tu = ranked(drain(&ups), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 == 1).map(|x| x.0).collect());
    let mut v = Vec::new();
    (&tt).filt(|(_, a)| a[0] > 0).cross(&tu).drive(|_, ((t, a), (u, b))| v.push((t, a, u, b)));
    rows(v.into_iter().map(|(t, a, u, b)| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[4]), V::I(a[5])];
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Author, COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY COUNT(a.Id) DESC) AS RankByAnswers
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName AS UserName, COUNT(*) AS PostCount, SUM(CASE WHEN pts.Id = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN pts.Id = 2 THEN 1 ELSE 0 END) AS AnswersCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostTypes pts ON p.PostTypeId = pts.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Author, rp.AnswerCount, rp.UpVotes, rp.DownVotes, mau.UserName, mau.PostCount, mau.QuestionsCount, mau.AnswersCount
// FROM RankedPosts rp LEFT JOIN MostActiveUsers mau ON rp.Author = mau.UserName WHERE rp.RankByAnswers <= 10 ORDER BY rp.RankByAnswers;
fn q8117(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let rp = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(since)))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (x, t)| [a[0] + x.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let recent = Ident::<Post>::new().with(creation_date.ge(since));
    let mau = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(recent).select(&db.post.post_type)).fold([0i64; 3], |a, t| {
        let t = db.post_type.origid.get(t).unwrap();
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]
    });
    let mv = rel(drain(&mau));
    let by_name: HashIdx<Str, (Id<User>, [i64; 3])> = (&mv).map(|(u, _)| u).select(&db.user.display_name).inv().select(&mv).collect();
    let v = ranked(drain((&rp).and(owner_user.select(&db.user.display_name).select(&by_name).opt())), |&(_, (a, _))| Reverse(a[0]), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, (a, m)), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.extend(match m {
            Some((u, m)) => [user_col(db, u, "name"), V::I(m[0]), V::I(m[1]), V::I(m[2])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(Coalesce(c.CommentCount, 0)) AS TotalComments, SUM(Coalesce(v.UpVotes, 0)) AS TotalUpVotes,
//        SUM(Coalesce(v.DownVotes, 0)) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalComments, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY PostCount DESC) AS PostRank,
//        RANK() OVER (ORDER BY TotalUpVotes DESC) AS UpVoteRank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalComments, TotalUpVotes, TotalDownVotes, PostRank, UpVoteRank FROM TopUsers
// WHERE PostRank <= 10 OR UpVoteRank <= 10 ORDER BY PostRank, UpVoteRank;
fn q7849(db: &'static So) -> String {
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let vs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&cc).opt()).and((&vs).opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some(((t, c), v)) => {
                let v = v.unwrap_or([0, 0]);
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.unwrap_or(0), a[4] + v[0], a[5] + v[1]]
            }
            None => a,
        });
    let v = ranked(drain(&ua), |&(_, a)| Reverse(a[0]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[4]), false);
    let mut v = drain(rel(v).filt(|((_, p), u)| p <= 10 || u <= 10));
    v.sort_by_key(|&(_, ((_, p), u))| (p, u));
    rows(v.into_iter().map(|(_, (((u, a), pr), ur))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(pr), V::I(ur)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Users.Id AS UserId, Users.Reputation, COUNT(Badges.Id) AS BadgeCount, SUM(CASE WHEN Badges.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        MAX(Posts.CreationDate) AS LastPostDate FROM Users LEFT JOIN Badges ON Users.Id = Badges.UserId LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId GROUP BY Users.Id, Users.Reputation),
// ActiveUsers AS (SELECT UserId, Reputation, BadgeCount, GoldBadges, LastPostDate, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation WHERE BadgeCount > 0 OR Reputation > 100),
// RecentPosts AS (SELECT Posts.Id AS PostId, Posts.Title, Posts.CreationDate, Posts.OwnerUserId, EXTRACT(EPOCH FROM (CURRENT_TIMESTAMP - Posts.CreationDate)) / 60 AS PostAge,
//        COUNT(Comments.Id) AS CommentsCount FROM Posts LEFT JOIN Comments ON Posts.Id = Comments.PostId WHERE Posts.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'
//     GROUP BY Posts.Id, Posts.Title, Posts.CreationDate, Posts.OwnerUserId HAVING COUNT(Comments.Id) > 5)
// SELECT U.UserId, U.Reputation, U.BadgeCount, U.GoldBadges, RP.Title AS RecentPostTitle, RP.PostAge, RP.CommentsCount,
//        CASE WHEN U.LastPostDate < CURRENT_TIMESTAMP - INTERVAL '6 months' THEN 'Inactive' ELSE 'Active' END AS UserActivityStatus
// FROM ActiveUsers U LEFT JOIN RecentPosts RP ON U.UserId = RP.OwnerUserId WHERE U.ReputationRank < 11 ORDER BY U.Reputation DESC, RP.PostAge ASC NULLS LAST;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ: the TIMESTAMP columns are read as New York wall time, and the intervals are taken on that calendar.
fn q20334(db: &'static So) -> String {
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(&db.post.creation_date).opt()))
        .fold([0, 0, i64::MIN], |a, (b, d)| [a[0] + b.is_some() as i64, a[1] + (b == Some(1)) as i64, d.map_or(a[2], |d| a[2].max(d))]);
    let active = db.user.select((&db.user.reputation).and(&ur)).filt(|(r, a)| a[0] > 0 || r > 100);
    let v = ranked(drain(active), |&(_, (r, _))| Reverse(r), true);
    let top: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 < 11).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let now = utc_to_ny(now_utc());
    let Post { owner_user, creation_date, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(now, -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt())
        .fold(0i64, |n, c| n + c.is_some() as i64);
    let rp_of: HashIdx<Id<User>, Id<Post>> = db.post.with((&rp).filt(|n| n > 5)).select(owner_user).inv().collect();
    let age = |p: Id<Post>| secs(tz_sub(now, creation_date.get(p).unwrap())) / 60.0;
    let mut v = drain((&top).select(Ident::<User>::new().and(&ur).and((&rp_of).select(Ident::<Post>::new().and(&rp)).opt())));
    v.sort_by_key(|&(u, ((_, _), p))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), p.map(|(p, _)| fkey(age(p)))));
    rows(v.into_iter().map(|(u, ((_, a), p))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match p {
            Some((p, n)) => [post_fields(db, p, &["title"]).remove(0), V::F(age(p)), V::I(n)],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if a[2] != i64::MIN && a[2] < add_months(now, -6) { "Inactive" } else { "Active" }));
        row(f)
    }))
}

// WITH RecursiveTagStats AS (SELECT T.Id AS TagId, T.TagName, T.Count AS TagCount, P.Id AS PostId, P.Title, P.CreationDate, P.Score,
//        ROW_NUMBER() OVER (PARTITION BY T.Id ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' WHERE P.PostTypeId = 1),
// PostVoteStats AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVoteCount, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId),
// RecentActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostedQuestions, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentsMade, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Comments C ON C.UserId = U.Id GROUP BY U.Id, U.DisplayName HAVING COUNT(P.Id) > 0)
// SELECT R.TagId, R.TagName, R.TagCount, R.PostId, R.Title, R.CreationDate, R.Score, P.UpVoteCount, P.DownVoteCount, P.TotalVotes, A.UserId, A.DisplayName, A.PostedQuestions, A.CommentsMade, A.LastPostDate
// FROM RecursiveTagStats R LEFT JOIN PostVoteStats P ON R.PostId = P.PostId LEFT JOIN RecentActivity A ON R.PostId = A.PostedQuestions
// WHERE R.RecentPostRank = 1 ORDER BY R.TagCount DESC, R.Score DESC;
//
// The WHERE on P.PostTypeId makes the first LEFT JOIN inner. R.PostId = A.PostedQuestions compares a post id with a count, so it goes through origid.
fn q31760(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, .. } = &db.post;
    let asked = Ident::<Post>::new().with(post_type_id.eq(1));
    let lt = tag_mentions(db);
    let pairs = drain((&lt).map(|(p, t)| (p, t)).select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p).select(asked).map(|_| ())));
    let newest = top_per(pairs, |&((_, t), _)| t, |&((p, _), _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let r = rel(newest.into_iter().map(|x| x.0).collect());
    let pvs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let ra = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(creation_date).and(comments_by(db).opt()))
        .fold([0, 0, i64::MIN], |a, (d, c)| [a[0] + 1, a[1] + c.is_some() as i64, a[2].max(d)]);
    let rav = rel(drain(&ra));
    let by_count: HashIdx<i64, (Id<User>, [i64; 3])> = (&rav).map(|(_, a)| a[0]).inv().select(&rav).collect();
    type R = (Id<Post>, Id<Tag>);
    let v = drain((&r).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&pvs).opt())).and(Same::<R>::new().map(|(p, _): R| p).select(origid).select(&by_count).opt())));
    rows(v.into_iter().map(|(_, (((p, t), s), a))| {
        let mut f = vec![V::I(db.tag.origid.get(t).unwrap()), V::S(db.tag.tag_name.get(t).unwrap()), V::I(db.tag.count.get(t).unwrap())];
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend(match s {
            Some(s) => s.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        match a {
            Some((u, a)) => {
                f.extend(ucols(db, u, &["uid", "name"]));
                f.extend([V::I(a[0]), V::I(a[1]), V::T(a[2])]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RECURSIVE UserPostCTE AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// MostActiveUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostCTE WHERE TotalPosts > 0),
// PostScoreCTE AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Score, CASE WHEN p.Score > 10 THEN 'High' WHEN p.Score BETWEEN 1 AND 10 THEN 'Medium' ELSE 'Low' END AS ScoreCategory,
//        COALESCE((SELECT SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId = p.Id), 0) AS Upvotes FROM Posts p),
// TopPosts AS (SELECT p.PostId, p.OwnerUserId, p.ScoreCategory, p.Upvotes, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Upvotes DESC, p.Score DESC) as Rank FROM PostScoreCTE p)
// SELECT u.DisplayName, u.TotalPosts, u.QuestionCount, u.AnswerCount, tp.PostId, tp.ScoreCategory, tp.Upvotes
// FROM MostActiveUsers u LEFT JOIN TopPosts tp ON u.UserId = tp.OwnerUserId AND tp.Rank = 1 WHERE u.PostRank <= 10 ORDER BY u.PostRank;
//
// WITH RECURSIVE, but no CTE refers to itself. Upvotes is taken only for the posts of the ten most active users, whose rank is all TopPosts is read for.
fn q33870(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain((&ups).filt(|a| a[1] > 0)), |&(_, a)| Reverse(a[1]), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| (u, (a, r))).collect());
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let pc = (&tus).select(posts_of(db)).map(|p| p).group_by(Same::<Id<Post>>::new()).select(up.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let score = &db.post.score;
    let best = top_per(drain((&pc).and((&db.post.owner_user).and(score))), |&(_, (_, (u, _)))| u, |&(_, (n, (_, s)))| (Reverse(n), Reverse(s)), 1, true);
    let bv = rel(best.into_iter().map(|(p, (n, (u, _)))| (u, (p, n))).collect());
    let best_of: HashIdx<Id<User>, (Id<User>, (Id<Post>, i64))> = (&bv).map(|(u, _)| u).inv().select(&bv).collect();
    let mut v = drain((&tu).select(Same::<(Id<User>, ([i64; 10], i64))>::new().and(Same::<(Id<User>, ([i64; 10], i64))>::new().map(|(u, _)| u).select((&best_of).map(|(_, x)| x).opt()))));
    v.sort_by_key(|&(_, ((_, (_, r)), _))| r);
    rows(v.into_iter().map(|(_, ((u, (a, _)), b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(match b {
            Some((p, n)) => {
                let s = score.get(p).unwrap();
                [V::I(db.post.origid.get(p).unwrap()), V::S(if s > 10 { "High" } else if s >= 1 { "Medium" } else { "Low" }), V::I(n)]
            }
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, MAX(p.LastActivityDate) AS LastActivity, p.Score, p.CreationDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Score, p.CreationDate),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadges, SUM(ps.CommentCount) AS TotalComments, SUM(ps.AnswerCount) AS TotalAnswers,
//        SUM(ps.UpVoteCount) AS TotalUpVotes, SUM(ps.DownVoteCount) AS TotalDownVotes FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN PostStats ps ON u.Id = ps.PostId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalBadges, TotalComments, TotalAnswers, TotalUpVotes, TotalDownVotes,
//        RANK() OVER (ORDER BY TotalUpVotes DESC, TotalAnswers DESC, TotalComments DESC) AS UserRank FROM TopUsers)
// SELECT UserId, DisplayName, TotalBadges, TotalComments, TotalAnswers, TotalUpVotes, TotalDownVotes, UserRank FROM RankedUsers WHERE UserRank <= 10 ORDER BY UserRank;
//
// u.Id = ps.PostId compares a user id with a post id, so it goes through origid. The two COUNT(DISTINCT)s come from folds over one row per child.
fn q8106(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let ps0 = || db.post.with(post_type_id.is_in([1, 2]));
    let vs = ps0()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = ps0().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = ps0().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ps = (&vs).and(&cc).and(&ac);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and((&db.user.origid).select(&pidx).select(&ps).opt()))
        .fold([0i64; 7], |a, (b, s)| match s {
            Some(((v, c), n)) => [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + 1, a[3] + c, a[4] + n, a[5] + v[0], a[6] + v[1]],
            None => [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2], a[3], a[4], a[5], a[6]],
        });
    let o = |a: [i64; 7], i: usize| if a[2] == 0 { None } else { Some(a[i]) };
    let v = ranked(drain(&tu), |&(_, a)| (o(a, 5).is_none(), Reverse(o(a, 5)), o(a, 4).is_none(), Reverse(o(a, 4)), o(a, 3).is_none(), Reverse(o(a, 3))), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(nullable(a[1], a[0]));
        f.extend([3, 4, 5, 6].map(|i| oint(o(a, i))));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount,
//        SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE -1 END ELSE 0 END) AS NetVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostAnalytics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COUNT(C) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE -1 END) AS Score,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS UserPostRank, P.OwnerUserId
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId),
// FinalResults AS (SELECT UA.UserId, UA.DisplayName, PA.PostId, PA.Title, PA.CreationDate, PA.CommentCount, PA.UpVotes, PA.DownVotes, PA.Score, UA.NetVotes
//     FROM UserVoteCounts UA JOIN PostAnalytics PA ON UA.UserId = PA.OwnerUserId WHERE UA.NetVotes IS NOT NULL)
// SELECT UserId, DisplayName, PostId, Title, CreationDate, CommentCount, UpVotes, DownVotes, Score, NetVotes FROM FinalResults WHERE Score > 0 ORDER BY NetVotes DESC, CreationDate ASC LIMIT 10;
//
// COUNT(C) counts a whole row of C, which a LEFT JOIN never leaves NULL, so it counts every joined row (translation-failures.md 11).
fn q4498(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + match t {
        Some(2) => 1,
        Some(3) => -1,
        _ => 0,
    });
    let pa = db
        .post
        .with(&db.post.owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (_, t)| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + if t == Some(2) { 1 } else { -1 }]);
    let v = drain((&pa).filt(|a| a[3] > 0).and((&db.post.owner_user).select(Ident::<User>::new().and(&uv))));
    let v = top_n(v, |&(p, (_, (_, n)))| (Reverse(n), db.post.creation_date.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, (a, (u, n)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend(a.map(V::I));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT u.Id AS UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY u.Id),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS EditCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title),
// TopUsers AS (SELECT u.DisplayName, u.Reputation, u.CreationDate, uv.TotalVotes, uv.UpVotes, uv.DownVotes FROM Users u JOIN UserVoteCounts uv ON u.Id = uv.UserId
//     ORDER BY uv.TotalVotes DESC, u.Reputation DESC LIMIT 10),
// PopularPosts AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.EditCount, ps.UpVotes, ps.DownVotes, ROW_NUMBER() OVER (ORDER BY ps.UpVotes DESC) AS Rank FROM PostStatistics ps WHERE ps.UpVotes > 0)
// SELECT tu.DisplayName, tu.Reputation, pp.Title AS PopularPostTitle, pp.CommentCount, pp.EditCount, pp.UpVotes, pp.DownVotes
// FROM TopUsers tu JOIN PopularPosts pp ON pp.Rank <= 5 ORDER BY tu.Reputation DESC;
//
// The ON names only pp, so the ten users are crossed with the five posts. The two COUNT(DISTINCT)s come from folds over one row per child.
fn q5461(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let tu = rel(top_n(drain(&uv), |&(u, n)| (Reverse(n), Reverse(db.user.reputation.get(u).unwrap())), 10));
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pp = top_n(drain((&ps).filt(|a| a[0] > 0)), |&(p, a)| (Reverse(a[0]), p), 5);
    let ppm: MatSet<Id<Post>> = rel(pp.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&ppm).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ec = (&ppm).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ppr = rel(drain((&cc).and(&ec).and(&ps)).into_iter().map(|(p, ((c, e), a))| (p, ((a, c), e))).collect());
    let mut v = Vec::new();
    (&tu).cross(&ppr).drive(|_, ((u, _), (p, ((a, c), e)))| v.push((u, p, a, c, e)));
    v.sort_by_key(|&(u, ..)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, p, a, c, e)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(c), V::I(e), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews, AVG(p.AnswerCount) AS AverageAnswers, MAX(p.CreationDate) AS LastPostDate
//     FROM Posts p GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(bc.BadgeCount, 0) AS BadgeCount, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.TotalScore, 0) AS TotalScore,
//        COALESCE(ps.TotalViews, 0) AS TotalViews, COALESCE(ps.AverageAnswers, 0) AS AverageAnswers, COALESCE(ps.LastPostDate, '1900-01-01') AS LastPostDate
//     FROM Users u LEFT JOIN UserBadgeCounts bc ON u.Id = bc.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT up.UserId, up.DisplayName, up.BadgeCount, up.PostCount, up.TotalScore, up.TotalViews, up.AverageAnswers, up.LastPostDate,
//        DENSE_RANK() OVER (ORDER BY up.TotalScore DESC) AS RankByScore, DENSE_RANK() OVER (ORDER BY up.BadgeCount DESC) AS RankByBadges
// FROM UserPerformance up WHERE up.BadgeCount > 0 OR up.PostCount > 0 ORDER BY up.TotalScore DESC, up.LastPostDate DESC;
fn q9480(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { score, view_count, answer_count, creation_date, .. } = &db.post;
    let ps = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(answer_count.opt()).and(creation_date)).opt())
        .fold([0, 0, 0, 0, 0, i64::MIN], |a, x| match x {
            Some((((s, w), n), d)) => [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + n.is_some() as i64, a[4] + n.unwrap_or(0), a[5].max(d)],
            None => a,
        });
    let up = (&bc).and(&ps).filt(|(b, a)| b > 0 || a[0] > 0);
    let v = ranked(drain(up), |&(_, (_, a))| Reverse(a[1]), true);
    let v = ranked(v, |&((_, (b, _)), _)| Reverse(b), true);
    rows(v.into_iter().map(|(((u, (b, a)), rs), rb)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[3] == 0 { V::F(0.0) } else { avg(a[4], a[3]) }, V::T(if a[5] == i64::MIN { date(1900, 1, 1) } else { a[5] }), V::I(rs), V::I(rb)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(COUNT(c.Id), 0) AS CommentCount, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, ub.BadgeCount, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE u.Reputation > 1000)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, rp.CommentCount, tu.DisplayName AS TopUser, tu.Reputation AS UserReputation,
//        tu.BadgeCount AS UserBadgeCount, rp.Rank AS PostRank, tu.UserRank AS UserRank
// FROM RankedPosts rp JOIN TopUsers tu ON rp.PostId IN (SELECT AcceptedAnswerId FROM Posts WHERE OwnerUserId = tu.Id) WHERE rp.Rank <= 10 ORDER BY rp.Rank, tu.UserRank;
//
// Rank reads only base columns, so the ten top questions are picked first and the vote x comment product is driven for those alone.
// The IN is a semi join: a pair joins once whatever the number of the user's posts accepting that post.
fn q8396(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, accepted_answer, owner_user, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap()), p), 10);
    let rk = rel(v.into_iter().enumerate().map(|(i, (p, _))| (p, i as i64 + 1)).collect());
    let tp: MatSet<Id<Post>> = (&rk).map(|(p, _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let ur = rel(top_n(drain(rich().select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 0).into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let urank: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let bc = rich().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pairs: MatSet<(Id<Post>, Id<User>)> = db.post.select(accepted_answer.and(owner_user)).with(Same::<(Id<Post>, Id<User>)>::new().map(|(a, _)| a).select(&tp).map(|_| ())).map(|x| x).collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<User>)> = (&pairs).map(|(p, _)| p).inv().collect();
    let rank_of: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let mut v = drain((&s).and((&rank_of).map(|(_, r)| r)).and((&by_post).map(|(_, u)| u).select(Ident::<User>::new().and((&urank).map(|(_, r)| r)).and(&bc))));
    v.sort_by_key(|&(_, ((_, r), ((_, ur), _)))| (r, ur));
    rows(v.into_iter().map(|(p, ((a, r), ((u, ur), b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(r), V::I(ur)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Body, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalQuestionsAsked, SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedAnswers,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownvotedAnswers, AVG(p.ViewCount) AS AvgViewsPerQuestion FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName),
// CloseReasons AS (SELECT ph.PostId, ph.Comment AS CloseReason, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.Comment)
// SELECT u.DisplayName AS User, ra.PostId, ra.Title AS QuestionTitle, ra.ViewCount AS QuestionViews, ua.TotalQuestionsAsked, ua.TotalViews AS UserTotalViews, ua.UpvotedAnswers,
//        ua.DownvotedAnswers, ua.AvgViewsPerQuestion, cr.CloseReason, cr.CloseCount
// FROM RankedPosts ra JOIN Users u ON ra.OwnerUserId = u.Id JOIN UserActivity ua ON u.Id = ua.UserId LEFT JOIN CloseReasons cr ON ra.PostId = cr.PostId
// WHERE ra.rn = 1 ORDER BY ra.ViewCount DESC, ua.TotalQuestionsAsked DESC;
fn q27349(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let last = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let ra: MatSet<Id<Post>> = rel(last.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let asked = Ident::<Post>::new().with(post_type_id.eq(1));
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(asked).select(score.and(view_count.opt()))).fold([0i64; 5], |a, (s, w)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + (s > 0) as i64, a[4] + (s < 0) as i64]
    });
    let PostHistory { post, comment, post_history_type_id, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(comment.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let crv = rel(drain(&cr));
    let cr_of: HashIdx<Id<Post>, ((Id<Post>, Option<Str>), i64)> = (&crv).map(|((p, _), _)| p).inv().select(&crv).collect();
    let v = drain((&ra).select(owner_user.select(Ident::<User>::new().and(&ua)).and((&cr_of).opt())));
    rows(v.into_iter().map(|(p, ((u, a), c))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "views"]));
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4]), avg(a[2], a[1])]);
        f.extend(match c {
            Some(((_, r), n)) => [ostr(r), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostStats AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.AnswerCount, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.AnswerCount, tp.OwnerDisplayName)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.AnswerCount, ps.OwnerDisplayName, ps.CommentCount, ps.UpVotes, ps.DownVotes, (ps.UpVotes - ps.DownVotes) AS NetVotes
// FROM PostStats ps ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q6688(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.push(V::S(owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        row(f)
    }))
}

// Rewritten (rewrites/2655.sql): the float AVG of epoch seconds becomes the exact-integer mean SUM(epoch_us(..))::DOUBLE / COUNT(..) / 1e6.
// WITH MostActiveUsers AS (SELECT u.Id, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(v.BountyAmount) AS TotalBounty, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes,
//        ROW_NUMBER() OVER (ORDER BY COUNT(p.Id) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName HAVING COUNT(p.Id) > 5),
// CustomTagStats AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, AVG(Length(p.Body)) AS AvgPostLength, MAX(p.CreationDate) AS LastPostDate
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY t.TagName),
// ClosedPostReasons AS (SELECT ph.UserId, ph.Comment, COUNT(ph.Id) AS CloseCount,
//        SUM(epoch_us(cast('2024-10-01 12:34:56' as timestamp)) - epoch_us(ph.CreationDate))::DOUBLE / COUNT(ph.CreationDate) / 1e6 AS AvgCloseTime
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId, ph.Comment)
// SELECT aua.DisplayName, aua.PostCount, aua.TotalBounty, COALESCE(tas.PostCount, 0) AS TotalPostsForTags, COALESCE(tas.AvgPostLength, 0) AS AvgPostLength,
//        COALESCE(cpr.CloseCount, 0) AS TotalCloseVotes, COALESCE(cpr.AvgCloseTime, 0) AS AvgTimeToClose
// FROM MostActiveUsers aua LEFT JOIN CustomTagStats tas ON tas.PostCount > 0 LEFT JOIN ClosedPostReasons cpr ON cpr.UserId = aua.Id
// WHERE aua.UserRank <= 10 ORDER BY aua.TotalBounty DESC, aua.PostCount DESC;
//
// The first ON names only tas, so the users are crossed with the tag rows that pass it.
fn q2655(db: &'static So) -> String {
    let mau = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let b = b.flatten();
            [a[0] + p.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let top = top_n(drain((&mau).filt(|a| a[0] > 5)), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu = rel(top);
    let Post { creation_date, body, .. } = &db.post;
    let lt = tag_mentions(db);
    type R = (Id<Post>, Id<Tag>);
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let tas = (&lt)
        .group_by(Same::<R>::new().map(|(_, t): R| t).select(&db.tag.tag_name))
        .select(Same::<R>::new().map(|(p, _): R| p).select(recent).select(body))
        .fold([0i64; 2], |a, b| [a[0] + 1, a[1] + b.chars().count() as i64]);
    let tasv = left_all(drain((&tas).filt(|a| a[0] > 0)));
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let PostHistory { user, comment, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cpr = db.post_history.with(post_history_type_id.eq(10)).group_by(user.and(comment.opt())).select(hd).fold([0i64; 2], |a, d| [a[0] + 1, a[1] + (t0 - d)]);
    let cv = rel(drain(&cpr));
    let cpr_of: HashIdx<Id<User>, ((Id<User>, Option<Str>), [i64; 2])> = (&cv).map(|((u, _), _)| u).inv().select(&cv).collect();
    let mut v = Vec::new();
    (&tu).select(Same::<(Id<User>, [i64; 3])>::new().and(Same::<(Id<User>, [i64; 3])>::new().map(|(u, _)| u).select((&cpr_of).map(|(_, c)| c).opt())))
        .cross(&tasv)
        .drive(|_, (((u, a), c), t)| v.push((u, a, t, c)));
    v.sort_by_key(|&(_, a, _, _)| (a[1] == 0, Reverse(a[2]), Reverse(a[0])));
    rows(v.into_iter().map(|(u, a, t, c)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1])];
        f.extend(match t {
            Some((_, t)) => [V::I(t[0]), avg(t[1], t[0])],
            None => [V::I(0), V::F(0.0)],
        });
        f.extend(match c {
            Some(c) => [V::I(c[0]), V::F(c[1] as f64 / c[0] as f64 / 1e6)],
            None => [V::I(0), V::F(0.0)],
        });
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END), 0) AS TotalVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostWithComments AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount FROM Posts p
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId),
// PostHistoryAnalysis AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate, STRING_AGG(DISTINCT pht.Name, ', ') AS EditTypes
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT u.DisplayName AS User, u.Reputation, ups.UpVotes, ups.DownVotes, p.Title AS PostTitle, p.ViewCount, ph.LastEditDate, ph.EditCount, ph.EditTypes, p.CommentCount
// FROM Users u JOIN UserVoteStats ups ON u.Id = ups.UserId JOIN PostWithComments p ON p.PostId IN (SELECT DISTINCT PostId FROM Votes v WHERE v.UserId = u.Id)
// LEFT JOIN PostHistoryAnalysis ph ON p.PostId = ph.PostId WHERE ups.TotalVotes > 10 AND p.ViewCount > 200 AND ph.EditCount > 0
// ORDER BY u.Reputation DESC, p.ViewCount DESC LIMIT 50;
//
// The IN is a semi join on the posts a user voted on. STRING_AGG(DISTINCT ..) has no ORDER BY; the port joins the names sorted (the answer is empty, so it is not tested).
fn q1724(db: &'static So) -> String {
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(2 | 3)) as i64]
    });
    let voted: MatSet<(Id<User>, Id<Post>)> = db.vote.select((&db.vote.user).and(&db.vote.post)).map(|x| x).collect();
    let voted_by: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&voted).map(|(u, _)| u).inv().collect();
    let Post { view_count, .. } = &db.post;
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let pha = db.post.group_by(Ident::<Post>::new()).select(history_of(db).select((&db.post_history.creation_date).and(htype_name(db)))).buf_fold(|v| {
        let mut names: Vec<Str> = v.iter().map(|x| x.1).collect();
        names.sort_unstable();
        names.dedup();
        let s: &'static str = Box::leak(names.join(", ").into_boxed_str());
        (v.len() as i64, v.iter().map(|x| x.0).max().unwrap(), s)
    });
    let posts = Ident::<Post>::new().with(view_count.gt(200)).and((&pha).filt(|a| a.0 > 0)).and((&cc).opt());
    let v = drain((&uvs).filt(|a| a[2] > 10).and((&voted_by).map(|(_, p)| p).select(posts)));
    let v = top_n(v, |&(u, (_, ((p, _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(view_count.get(p))), 50);
    rows(v.into_iter().map(|(u, (a, ((p, h), c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::T(h.1), V::I(h.0), V::S(h.2), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, UserRank, BadgeCount FROM RankedUsers WHERE UserRank <= 10),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViews FROM Posts p GROUP BY p.OwnerUserId),
// UserPostStats AS (SELECT u.UserId, u.DisplayName, ps.PostCount, ps.TotalScore, ps.AvgViews, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM TopUsers u LEFT JOIN PostStats ps ON u.UserId = ps.OwnerUserId LEFT JOIN RankedUsers b ON u.UserId = b.UserId)
// SELECT ups.DisplayName, ups.PostCount, ups.TotalScore, ups.AvgViews, ups.BadgeCount,
//        CASE WHEN ups.BadgeCount > 5 THEN 'Highly Decorated' WHEN ups.BadgeCount > 0 THEN 'Moderately Decorated' ELSE 'No Badges' END AS BadgeStatus,
//        CASE WHEN ups.AvgViews > 1000 THEN 'Popular' WHEN ups.AvgViews BETWEEN 500 AND 1000 THEN 'Moderately Popular' ELSE 'Less Popular' END AS PopularityGrade
// FROM UserPostStats ups INNER JOIN Votes v ON ups.UserId = v.UserId WHERE v.VoteTypeId = 2 AND ups.PostCount > 10 ORDER BY ups.TotalScore DESC, ups.BadgeCount DESC;
fn q4979(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { score, view_count, .. } = &db.post;
    let ps = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt()))).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let up = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let mut v = drain((&ps).filt(|a| a[0] > 10).and(&bc).and(up));
    v.sort_by_key(|&(_, ((a, b), _))| (Reverse(a[1]), Reverse(b)));
    rows(v.into_iter().map(|(u, ((a, b), _))| {
        let av = a[2] as f64 / a[0] as f64;
        row(vec![
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[1]),
            V::F(av),
            V::I(b),
            V::S(if b > 5 { "Highly Decorated" } else if b > 0 { "Moderately Decorated" } else { "No Badges" }),
            V::S(if av > 1000.0 { "Popular" } else if av >= 500.0 { "Moderately Popular" } else { "Less Popular" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadges, AVG(u.Reputation) AS AvgReputation FROM Users u JOIN Badges b ON b.UserId = u.Id
//     WHERE u.LastAccessDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY u.Id, u.DisplayName),
// PostComments AS (SELECT p.Id AS PostId, COALESCE(SUM(c.Score), 0) AS TotalCommentScore FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.ViewCount > 1000 GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.RankScore, au.DisplayName AS TopUser, au.TotalBadges, au.AvgReputation, pc.TotalCommentScore
// FROM RankedPosts rp JOIN PostComments pc ON pc.PostId = rp.PostId JOIN ActiveUsers au ON au.UserId = (SELECT UserId FROM Votes v WHERE v.PostId = rp.PostId ORDER BY v.CreationDate DESC LIMIT 1)
// WHERE rp.RankScore <= 10 OR rp.ViewCount > 10000 ORDER BY rp.RankScore, rp.ViewCount DESC;
//
// The correlated LIMIT 1 is an arg-max fold over each post's votes; a tie on CreationDate keeps the larger vote id.
fn q3251(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let rp = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id));
    let rk = top_n(rp, |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), 0);
    let mut ranks = Vec::new();
    for i in 0..rk.len() {
        let r = if i > 0 && rk[i - 1].1 == rk[i].1 { ranks.last().map(|x: &(Id<Post>, i64)| x.1 + 1).unwrap() } else { 1 };
        ranks.push((rk[i].0, r));
    }
    let rr = rel(ranks);
    let rank_of: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rr).map(|(p, _)| p).inv().select(&rr).collect();
    let pc = db.post.with(view_count.gt(1000)).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    let au = db
        .user
        .with((&db.user.last_access_date).ge(add_months(current_date(), -6)))
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(badges_of(db).select(&db.badge.class)))
        .fold([0i64; 3], |a, (r, c)| [a[0] + 1, a[1] + c, a[2] + r]);
    let Vote { creation_date: vd, user, .. } = &db.vote;
    let last = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(vd.and(Ident::<Vote>::new()))).fold(None, |m: Option<(i64, Id<Vote>)>, x| Some(m.map_or(x, |m| m.max(x))));
    let lv = (&last).flat_map(|m| m.map(|x| x.1)).select(user);
    let kept = (&rank_of).map(|(_, r)| r).and(view_count.opt()).filt(|(r, w)| r <= 10 || w.map_or(false, |w| w > 10000));
    let mut v = drain(kept.and(&pc).and(lv.select(Ident::<User>::new().and(&au))));
    v.sort_by_key(|&(_, (((r, w), _), _))| (r, Reverse(w)));
    rows(v.into_iter().map(|(p, (((r, _), c), (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(r), user_col(db, u, "name"), V::I(a[1]), avg(a[2], a[0]), V::I(c)]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostDetails AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPostEdits AS (SELECT ph.UserId, ph.PostId, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.UserId, ph.PostId),
// EditedPostCount AS (SELECT rpd.UserId, COUNT(rpd.PostId) AS EditCount FROM RecentPostEdits rpd GROUP BY rpd.UserId),
// UserMetrics AS (SELECT up.UserId, up.DisplayName, up.Reputation, up.PostCount, up.TotalScore, COALESCE(ec.EditCount, 0) AS TotalEdits FROM UserPostDetails up LEFT JOIN EditedPostCount ec ON up.UserId = ec.UserId),
// TopUsers AS (SELECT um.*, ROW_NUMBER() OVER (ORDER BY um.Reputation DESC) AS UserRank FROM UserMetrics um),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.PostCount, tu.TotalScore, tu.TotalEdits, COALESCE(ub.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN COALESCE(ub.BadgeCount, 0) > 0 THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus
// FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId WHERE tu.UserRank <= 10 ORDER BY tu.Reputation DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q31791(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ups = user_posts(db);
    let PostHistory { user, post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let rpe = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(user.and(post)).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let ec = rel(drain(&rpe)).group_by(Same::<((Id<User>, Id<Post>), i64)>::new().map(|((u, _), _)| u)).select(Same::<((Id<User>, Id<Post>), i64)>::new()).fold(0i64, |n, _| n + 1);
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&tu).select(Ident::<User>::new().and(&ups).and((&ec).opt()).and(&bc)));
    rows(v.into_iter().map(|(u, (((_, a), e), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[4]), V::I(e.unwrap_or(0)), V::I(b), V::S(if b > 0 { "Has Badges" } else { "No Badges" })]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("32936", q32936),
    ("6049", q6049),
    ("6925", q6925),
    ("7785", q7785),
    ("27897", q27897),
    ("2582", q2582),
    ("27409", q27409),
    ("663", q663),
    ("833", q833),
    ("936", q936),
    ("6269", q6269),
    ("28435", q28435),
    ("7538", q7538),
    ("28874", q28874),
    ("5027", q5027),
    ("5976", q5976),
    ("9603", q9603),
    ("865", q865),
    ("3928", q3928),
    ("2141", q2141),
    ("4672", q4672),
    ("25628", q25628),
    ("4921", q4921),
    ("26771", q26771),
    ("2178", q2178),
    ("4866", q4866),
    ("27771", q27771),
    ("7142", q7142),
    ("28544", q28544),
    ("29944", q29944),
    ("5015", q5015),
    ("29414", q29414),
    ("4330", q4330),
    ("4135", q4135),
    ("2835", q2835),
    ("5158", q5158),
    ("51", q51),
    ("3764", q3764),
    ("8672", q8672),
    ("3532", q3532),
    ("5437", q5437),
    ("29585", q29585),
    ("4176", q4176),
    ("28550", q28550),
    ("34830", q34830),
    ("4043", q4043),
    ("3774", q3774),
    ("6430", q6430),
    ("30173", q30173),
    ("4771", q4771),
    ("8017", q8017),
    ("279", q279),
    ("2151", q2151),
    ("6628", q6628),
    ("9692", q9692),
    ("27824", q27824),
    ("1737", q1737),
    ("6687", q6687),
    ("679", q679),
    ("9193", q9193),
    ("6647", q6647),
    ("34823", q34823),
    ("31052", q31052),
    ("1827", q1827),
    ("25434", q25434),
    ("3031", q3031),
    ("5634", q5634),
    ("6779", q6779),
    ("7129", q7129),
    ("8928", q8928),
    ("26472", q26472),
    ("2192", q2192),
    ("7024", q7024),
    ("25342", q25342),
    ("8425", q8425),
    ("31048", q31048),
    ("6318", q6318),
    ("6060", q6060),
    ("9169", q9169),
    ("8117", q8117),
    ("7849", q7849),
    ("20334", q20334),
    ("31760", q31760),
    ("33870", q33870),
    ("8106", q8106),
    ("4498", q4498),
    ("5461", q5461),
    ("9480", q9480),
    ("8396", q8396),
    ("27349", q27349),
    ("6688", q6688),
    ("2655", q2655),
    ("1724", q1724),
    ("4979", q4979),
    ("3251", q3251),
    ("31791", q31791),
];
