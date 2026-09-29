use harness::prelude::*;
use std::cmp::Reverse;

fn type_name(db: &'static So) -> Compose<&'static Col<Post, Id<PostType>>, &'static Col<PostType, Str>> {
    (&db.post.post_type).select(&db.post_type.name)
}

fn vote_type_name(db: &'static So) -> Compose<&'static Col<Vote, Id<VoteType>>, &'static Col<VoteType, Str>> {
    (&db.vote.vote_type).select(&db.vote_type.name)
}

fn top<Q: Drive, K: Ord>(q: Q, key: impl Fn(&(Q::D, Q::R)) -> K, n: usize) -> VecRel<usize, (Q::D, Q::R)> {
    let mut v = Vec::new();
    q.drive(|d, t| v.push((d, t)));
    v.sort_by_key(|t| key(t));
    if n < v.len() && key(&v[n - 1]) == key(&v[n]) {
        eprintln!("tie at the LIMIT cut");
    }
    v.truncate(n);
    VecRel::new(v)
}

// WITH PostCounts AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts,
//            AVG(p.Score) AS AvgScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     GROUP BY pt.Name),
// UserReputation AS (
//     SELECT u.Reputation, COUNT(*) AS UserPostCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     GROUP BY u.Reputation)
// SELECT pc.PostType, pc.TotalPosts, pc.AvgScore, pc.AvgViewCount,
//        ur.Reputation, ur.UserPostCount
// FROM PostCounts pc JOIN UserReputation ur ON ur.UserPostCount > 0
// ORDER BY pc.TotalPosts DESC, ur.Reputation DESC;
fn q14979(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let pc = db
        .post
        .group_by(type_name(db))
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (sc, v)| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        });
    let ur = db.post.group_by(owner_user.select(&db.user.reputation)).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&pc).cross((&ur).filt(|m| m > 0)).drive(|(t, rep), ((n, s, vn, vs), m)| {
        out.push(row(vec![V::S(t), V::I(n), avg(s, n), avg(vs, vn), V::I(rep), V::I(m)]))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT PostTypeId, COUNT(*) AS TotalPosts, AVG(ViewCount) AS AvgViewCount,
//            AVG(Score) AS AvgScore
//     FROM Posts GROUP BY PostTypeId),
// UserReputation AS (
//     SELECT AVG(Reputation) AS AvgReputation, COUNT(*) AS TotalUsers FROM Users),
// PopularTags AS (
//     SELECT Tags, COUNT(*) AS TagCount FROM Posts WHERE PostTypeId = 1
//     GROUP BY Tags ORDER BY TagCount DESC LIMIT 10)
// SELECT p.PostTypeId, p.TotalPosts, p.AvgViewCount, p.AvgScore,
//        u.AvgReputation, u.TotalUsers, t.Tags, t.TagCount
// FROM PostCounts p, UserReputation u, PopularTags t;
fn q11485(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, tags_str, .. } = &db.post;
    let pc = db
        .post
        .group_by(post_type_id)
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (sc, v)| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        });
    let ur = whole(&db.user.id).select(&db.user.reputation).fold((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let tags = db.post.with(post_type_id.eq(1)).group_by(tags_str.opt()).fold(0i64, |n, _| n + 1);
    let pt = top(&tags, |&(_, n)| Reverse(n), 10);
    let mut out = Vec::new();
    (&pc).cross(&ur).cross(&pt).drive(|((t, ()), _), (((n, s, vn, vs), (un, us)), (tg, tc))| {
        out.push(row(vec![V::I(t), V::I(n), avg(vs, vn), avg(s, n), avg(us, un), V::I(un), ostr(tg), V::I(tc)]))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// VoteCounts AS (
//     SELECT vt.Name AS VoteType, COUNT(v.Id) AS TotalVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY vt.Name),
// UserStats AS (
//     SELECT COUNT(u.Id) AS TotalUsers, AVG(u.Reputation) AS AverageReputation FROM Users u)
// SELECT pc.PostType, pc.TotalPosts, pc.AverageScore, vc.VoteType, vc.TotalVotes,
//        us.TotalUsers, us.AverageReputation
// FROM PostCounts pc CROSS JOIN UserStats us CROSS JOIN VoteCounts vc
// ORDER BY pc.TotalPosts DESC;
fn q13476(db: &'static So) -> String {
    let pc = db.post.group_by(type_name(db)).select(&db.post.score).fold((0i64, 0i64), |(n, s), sc| (n + 1, s + sc));
    let vc = db.vote.group_by(vote_type_name(db)).fold(0i64, |n, _| n + 1);
    let us = whole(&db.user.id).select(&db.user.reputation).fold((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let mut out = Vec::new();
    (&pc).cross(&us).cross(&vc).drive(|((t, ()), vt), (((n, s), (un, us)), vn)| {
        out.push(row(vec![V::S(t), V::I(n), avg(s, n), V::S(vt), V::I(vn), V::I(un), avg(us, un)]))
    });
    rows(out)
}

// WITH TotalPosts AS (SELECT COUNT(*) AS TotalPostCount FROM Posts),
// PostsByType AS (
//     SELECT pt.Name AS PostTypeName, COUNT(p.Id) AS PostCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// TopUsers AS (
//     SELECT u.DisplayName, u.Reputation, COUNT(p.Id) AS PostsCount
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation
//     ORDER BY PostsCount DESC LIMIT 10),
// VotesStatistics AS (
//     SELECT vt.Name AS VoteTypeName, COUNT(v.Id) AS VoteCount
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY vt.Name)
// SELECT (SELECT TotalPostCount FROM TotalPosts) AS TotalPosts,
//        pp.PostTypeName, pp.PostCount, tu.DisplayName, tu.Reputation, tu.PostsCount,
//        vs.VoteTypeName, vs.VoteCount
// FROM PostsByType pp CROSS JOIN TopUsers tu CROSS JOIN VotesStatistics vs;
fn q11785(db: &'static So) -> String {
    let total = whole(&db.post.id).fold(0i64, |n, _| n + 1);
    let pp = db.post.group_by(type_name(db)).fold(0i64, |n, _| n + 1);
    let per_user = (&db.post.owner_user).inv().fold(0i64, |n, _| n + 1);
    let tu = top(&per_user, |&(_, n)| Reverse(n), 10);
    let vs = db.vote.group_by(vote_type_name(db)).fold(0i64, |n, _| n + 1);
    let User { display_name, reputation, .. } = &db.user;
    let mut out = Vec::new();
    (&total).cross(&pp).cross(&tu).cross(&vs).drive(|(((_, t), _), vt), (((tp, n), (u, un)), vn)| {
        out.push(row(vec![
            V::I(tp),
            V::S(t),
            V::I(n),
            V::S(display_name.get(u).unwrap()),
            V::I(reputation.get(u).unwrap()),
            V::I(un),
            V::S(vt),
            V::I(vn),
        ]))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT p.PostTypeId, COUNT(*) AS TotalPosts, SUM(p.Score) AS TotalScore,
//            AVG(p.ViewCount) AS AverageViewCount, AVG(p.AnswerCount) AS AverageAnswerCount,
//            AVG(p.CommentCount) AS AverageCommentCount
//     FROM Posts p GROUP BY p.PostTypeId),
// UserStats AS (
//     SELECT COUNT(*) AS TotalUsers, AVG(u.Reputation) AS AverageReputation,
//            MAX(u.LastAccessDate) AS MostRecentAccessDate
//     FROM Users u),
// VoteStats AS (
//     SELECT v.VoteTypeId, COUNT(*) AS TotalVotes FROM Votes v GROUP BY v.VoteTypeId)
// SELECT ps.PostTypeId, ps.TotalPosts, ps.TotalScore, ps.AverageViewCount,
//        ps.AverageAnswerCount, ps.AverageCommentCount, us.TotalUsers,
//        us.AverageReputation, us.MostRecentAccessDate, vs.VoteTypeId, vs.TotalVotes
// FROM PostStats ps JOIN UserStats us ON TRUE JOIN VoteStats vs ON TRUE
// ORDER BY ps.PostTypeId, vs.VoteTypeId;
fn q14747(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, answer_count, comment_count, .. } = &db.post;
    let ps = db
        .post
        .group_by(post_type_id)
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, an, asum, cs), (((sc, v), a), c)| {
            (
                n + 1,
                s + sc,
                vn + v.is_some() as i64,
                vs + v.unwrap_or(0),
                an + a.is_some() as i64,
                asum + a.unwrap_or(0),
                cs + c,
            )
        });
    let us = whole(&db.user.id)
        .select((&db.user.reputation).and(&db.user.last_access_date))
        .fold((0i64, 0i64, i64::MIN), |(n, s, m), (r, la)| (n + 1, s + r, m.max(la)));
    let vs = db.vote.group_by(&db.vote.vote_type_id).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&ps).cross(&us).cross(&vs).drive(|((t, ()), vt), (((n, s, vn, vsum, an, asum, cs), (un, rs, la)), vn2)| {
        out.push(row(vec![
            V::I(t),
            V::I(n),
            V::I(s),
            avg(vsum, vn),
            avg(asum, an),
            avg(cs, n),
            V::I(un),
            avg(rs, un),
            V::T(la),
            V::I(vt),
            V::I(vn2),
        ]))
    });
    rows(out)
}

// WITH TopUsers AS (
//     SELECT u.Id, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount,
//            SUM(COALESCE(vUp.VoteCount, 0)) AS TotalUpVotes,
//            SUM(COALESCE(vDown.VoteCount, 0)) AS TotalDownVotes
//     FROM Users u
//     LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes
//                WHERE VoteTypeId = 2 GROUP BY PostId) vUp ON p.Id = vUp.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes
//                WHERE VoteTypeId = 3 GROUP BY PostId) vDown ON p.Id = vDown.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopPostTypes AS (
//     SELECT pt.Name AS PostTypeName, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Name),
// ClosedPosts AS (
//     SELECT COUNT(*) AS ClosedPostCount, SUM(v.Score) AS TotalClosedScore
//     FROM Posts v WHERE v.ClosedDate IS NOT NULL)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.TotalUpVotes, tu.TotalDownVotes,
//        tpt.PostTypeName, tpt.PostCount AS PostTypeCount, tpt.TotalScore,
//        cp.ClosedPostCount, cp.TotalClosedScore
// FROM TopUsers tu CROSS JOIN TopPostTypes tpt CROSS JOIN ClosedPosts cp
// ORDER BY tu.Reputation DESC, tpt.PostCount DESC;
fn q10056(db: &'static So) -> String {
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let tu = (&db.post.owner_user)
        .inv()
        .select((&up).and(&down))
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64), |(n, a, b), (u, d)| (n + 1, a + u, b + d));
    let tpt = (&db.post.post_type)
        .inv()
        .select(&db.post.score)
        .dense_fold_outer(db.post_type.id.n, (0i64, 0i64), |(n, s), sc| (n + 1, s + sc));
    let cp = whole(db.post.with(&db.post.closed_date))
        .select(&db.post.score)
        .fold((0i64, 0i64), |(n, s), sc| (n + 1, s + sc));
    let User { display_name, reputation, .. } = &db.user;
    let mut out = Vec::new();
    (&tu).cross(&tpt).cross(&cp).drive(|((u, t), ()), (((n, a, b), (tn, ts)), (cn, cs))| {
        out.push(row(vec![
            V::S(display_name.get(u).unwrap()),
            V::I(reputation.get(u).unwrap()),
            V::I(n),
            V::I(a),
            V::I(b),
            V::S(db.post_type.name.get(t).unwrap()),
            V::I(tn),
            nullable(ts, tn),
            V::I(cn),
            nullable(cs, cn),
        ]))
    });
    rows(out)
}

// WITH PostMetrics AS (
//     SELECT P.PostTypeId, COUNT(*) AS TotalPosts,
//            SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//            SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews
//     FROM Posts P GROUP BY P.PostTypeId),
// UserMetrics AS (
//     SELECT U.Reputation, COUNT(DISTINCT U.Id) AS TotalUsers,
//            SUM(U.UpVotes) AS TotalUpVotes, SUM(U.DownVotes) AS TotalDownVotes
//     FROM Users U GROUP BY U.Reputation),
// VoteMetrics AS (
//     SELECT V.VoteTypeId, COUNT(*) AS TotalVotes FROM Votes V GROUP BY V.VoteTypeId)
// SELECT PM.PostTypeId, PM.TotalPosts, PM.AcceptedAnswers, PM.TotalScore, PM.TotalViews,
//        UM.TotalUsers, UM.TotalUpVotes, UM.TotalDownVotes, VM.VoteTypeId, VM.TotalVotes
// FROM PostMetrics PM CROSS JOIN UserMetrics UM CROSS JOIN VoteMetrics VM
// ORDER BY PM.PostTypeId, VM.VoteTypeId;
fn q10239(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, accepted_answer_id, .. } = &db.post;
    let pm = db
        .post
        .group_by(post_type_id)
        .select(score.and(view_count.opt()).and(accepted_answer_id.opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, acc, s, vn, vs), ((sc, v), a)| {
            (n + 1, acc + a.is_some() as i64, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        });
    let User { reputation, up_votes, down_votes, .. } = &db.user;
    let um = db
        .user
        .group_by(reputation)
        .select(up_votes.and(down_votes))
        .fold((0i64, 0i64), |(u, d), (uv, dv)| (u + uv, d + dv));
    let um_distinct = db.user.group_by(reputation).select(Ident::<User>::new()).count_distinct();
    let vm = db.vote.group_by(&db.vote.vote_type_id).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&pm).cross((&um).and(&um_distinct)).cross(&vm).drive(|((t, _), vt), (((n, acc, s, vn, vs), ((u, d), nd)), vc)| {
        out.push(row(vec![
            V::I(t),
            V::I(n),
            V::I(acc),
            V::I(s),
            nullable(vs, vn),
            V::I(nd),
            V::I(u),
            V::I(d),
            V::I(vt),
            V::I(vc),
        ]))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT p.PostTypeId, COUNT(*) AS TotalPosts,
//            SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//            AVG(v.BountyAmount) AS AverageBountyAmount, COUNT(c.Id) AS TotalComments
//     FROM Posts p
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
//     LEFT JOIN Comments c ON p.Id = c.PostId
//     GROUP BY p.PostTypeId),
// UserStats AS (
//     SELECT u.Id AS UserId, COUNT(b.Id) AS TotalBadges, AVG(u.Reputation) AS AverageReputation
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT ps.PostTypeId, ps.TotalPosts, ps.PositiveScorePosts, ps.AverageBountyAmount,
//        ps.TotalComments, COUNT(DISTINCT us.UserId) AS ActiveUsers,
//        AVG(us.AverageReputation) AS AverageUserReputation
// FROM PostStats ps LEFT JOIN UserStats us ON ps.TotalPosts > 0
// GROUP BY ps.PostTypeId, ps.TotalPosts, ps.PositiveScorePosts, ps.AverageBountyAmount, ps.TotalComments
// ORDER BY ps.PostTypeId;
fn q10384(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty_votes = votes_of(db).with(vote_type_id.filt(|t| t == 8 || t == 9));
    let ps = db
        .post
        .group_by(post_type_id)
        .select(score.and((&bounty_votes).opt()).and(comments_of(db).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, pos, bn, bs, nc), ((sc, v), c)| {
            let b = v.and_then(|v| bounty_amount.get(v));
            (n + 1, pos + (sc > 0) as i64, bn + b.is_some() as i64, bs + b.unwrap_or(0), nc + c.is_some() as i64)
        });
    let (un, rs) = (&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let mut out = Vec::new();
    (&ps).drive(|t, (n, pos, bn, bs, nc)| {
        let (users, rep) = if n > 0 { (V::I(un), avg(rs, un)) } else { (V::I(0), V::Null) };
        out.push(row(vec![V::I(t), V::I(n), V::I(pos), avg(bs, bn), V::I(nc), users, rep]))
    });
    rows(out)
}

// WITH PostStatistics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AvgScore,
//            SUM(CASE WHEN p.LastActivityDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//                THEN 1 ELSE 0 END) AS RecentPosts,
//            SUM(p.ViewCount) AS TotalViews
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStatistics AS (
//     SELECT u.DisplayName, COUNT(b.Id) AS TotalBadges,
//            SUM(CASE WHEN c.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '365 days'
//                THEN 1 ELSE 0 END) AS ActiveComments
//     FROM Users u
//     LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN Comments c ON u.Id = c.UserId
//     GROUP BY u.DisplayName)
// SELECT ps.PostType, ps.TotalPosts, ps.AvgScore, ps.RecentPosts, ps.TotalViews,
//        us.DisplayName, us.TotalBadges, us.ActiveComments
// FROM PostStatistics ps JOIN UserStatistics us ON us.TotalBadges > 0
// ORDER BY ps.TotalPosts DESC;
fn q10393(db: &'static So) -> String {
    let Post { score, last_activity_date, view_count, .. } = &db.post;
    let now = ts(2024, 10, 1, 12, 34, 56);
    let (month, year) = (now - 30 * DAY_US, now - 365 * DAY_US);
    let ps = db
        .post
        .group_by(type_name(db))
        .select(score.and(last_activity_date).and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, r, vn, vs), ((sc, la), v)| {
            (n + 1, s + sc, r + (la > month) as i64, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        });
    let cd = &db.comment.creation_date;
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select(badges_of(db).opt().and(comments_by(db).opt()))
        .fold((0i64, 0i64), |(nb, a), (b, c)| {
            (nb + b.is_some() as i64, a + c.is_some_and(|c| cd.get(c).unwrap() > year) as i64)
        });
    let mut out = Vec::new();
    (&ps).cross((&us).filt(|(nb, _)| nb > 0)).drive(|(t, name), ((n, s, r, vn, vs), (nb, a))| {
        out.push(row(vec![V::S(t), V::I(n), avg(s, n), V::I(r), nullable(vs, vn), V::S(name), V::I(nb), V::I(a)]))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT PostTypeId, COUNT(*) AS TotalPosts FROM Posts GROUP BY PostTypeId),
// TopUsers AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount,
//            SUM(COALESCE(P.Score, 0)) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     GROUP BY U.Id, U.DisplayName),
// BadgeCounts AS (
//     SELECT UserId, COUNT(*) AS TotalBadges FROM Badges GROUP BY UserId)
// SELECT PCT.PostTypeId, PCT.TotalPosts, TU.DisplayName AS TopUser,
//        TU.PostCount AS UserPostCount, TU.TotalScore AS UserTotalScore,
//        BC.TotalBadges AS UserTotalBadges
// FROM PostCounts PCT
// JOIN TopUsers TU ON TU.PostCount = (SELECT MAX(PostCount) FROM TopUsers)
// JOIN BadgeCounts BC ON TU.UserId = BC.UserId
// ORDER BY PCT.TotalPosts DESC;
fn q10441(db: &'static So) -> String {
    let pct = db.post.group_by(&db.post.post_type_id).fold(0i64, |n, _| n + 1);
    let tu = (&db.post.owner_user)
        .inv()
        .select(&db.post.score)
        .dense_fold_outer(db.user.id.n, (0i64, 0i64), |(n, s), sc| (n + 1, s + sc));
    let max = (&tu).fold_flat(0i64, |m, (n, _)| m.max(n));
    let bc = (&db.badge.user).inv().fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&pct).cross((&tu).filt(|(n, _)| n == max).and(&bc)).drive(|(t, u), (tn, ((n, s), b))| {
        out.push(row(vec![
            V::I(t),
            V::I(tn),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(n),
            V::I(s),
            V::I(b),
        ]))
    });
    rows(out)
}

pub static ENTRIES: &[harness::Entry] = &[
    ("14979", q14979),
    ("11485", q11485),
    ("13476", q13476),
    ("11785", q11785),
    ("14747", q14747),
    ("10056", q10056),
    ("10239", q10239),
    ("10384", q10384),
    ("10393", q10393),
    ("10441", q10441),
];
