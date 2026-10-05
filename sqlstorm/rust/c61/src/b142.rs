use harness::prelude::*;

fn unit() -> VecRel<usize, ()> {
    rel(vec![()])
}

fn kahan((s, err): (f64, f64), x: f64) -> (f64, f64) {
    let y = x - err;
    let t = s + y;
    (t, (t - s) - y)
}

fn tz_secs(a: i64, b: i64) -> f64 {
    tz_sub(a, b) as f64 / 1e6
}

fn fmean(s: (f64, f64), n: i64) -> V {
    if n == 0 { V::Null } else { V::F(s.0 / n as f64) }
}

fn tmin(x: i64) -> V {
    if x == i64::MAX { V::Null } else { V::T(x) }
}

fn tmax(x: i64) -> V {
    if x == i64::MIN { V::Null } else { V::T(x) }
}

fn type_name(db: &'static So) -> Compose<&'static Col<Post, Id<PostType>>, &'static Col<PostType, Str>> {
    (&db.post.post_type).select(&db.post_type.name)
}

// WITH PostStats AS (
//     SELECT p.Id AS PostId, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.CreationDate,
//            u.Reputation AS OwnerReputation, COUNT(c.Id) AS TotalComments,
//            AVG(EXTRACT(EPOCH FROM (v.CreationDate - p.CreationDate))) AS AvgVoteAge
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.CreationDate, u.Reputation)
// SELECT SUM(Score), SUM(ViewCount), AVG(OwnerReputation), SUM(AnswerCount), SUM(CommentCount),
//        COUNT(PostId), MAX(CreationDate), MIN(CreationDate), AVG(AvgVoteAge)
// FROM PostStats;
fn q12687(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, creation_date, owner_user, .. } = &db.post;
    let va = db
        .post
        .group_by(Ident::<Post>::new())
        .select(creation_date.and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.creation_date).opt()))
        .fold(((0.0f64, 0.0f64), 0i64), |(s, n), ((pc, _), v)| match v {
            Some(v) => (kahan(s, (v - pc) as f64 / 1e6), n + 1),
            None => (s, n),
        });
    let t = db
        .post
        .select(score.and(view_count.opt()).and(owner_user.select(&db.user.reputation).opt()).and(answer_count.opt()).and(comment_count).and(creation_date).and((&va).map(|(s, n)| if n == 0 { None } else { Some(s.0 / n as f64) })))
        .fold_flat(
            [0i64, 0, 0, 0, 0, 0, 0, 0, 0, i64::MIN, i64::MAX],
            |a, ((((((s, w), r), an), cc), cd), x)| {
                [
                    a[0] + s,
                    a[1] + w.unwrap_or(0),
                    a[2] + w.is_some() as i64,
                    a[3] + r.unwrap_or(0),
                    a[4] + r.is_some() as i64,
                    a[5] + an.unwrap_or(0),
                    a[6] + an.is_some() as i64,
                    a[7] + cc,
                    a[8] + 1,
                    a[9].max(cd),
                    a[10].min(cd),
                ]
            },
        );
    let (vs, vn) = db
        .post
        .select((&va).map(|(s, n)| if n == 0 { None } else { Some(s.0 / n as f64) }))
        .fold_flat(((0.0f64, 0.0f64), 0i64), |(s, n), x| match x {
            Some(x) => (kahan(s, x), n + 1),
            None => (s, n),
        });
    row(vec![
        nullable(t[0], t[8]),
        nullable(t[1], t[2]),
        avg(t[3], t[4]),
        nullable(t[5], t[6]),
        nullable(t[7], t[8]),
        V::I(t[8]),
        tmax(t[9]),
        tmin(t[10]),
        fmean(vs, vn),
    ])
}

// WITH PostStats AS (
//     SELECT p.PostTypeId, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//            SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts,
//            SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//            AVG(COALESCE(p.Score, 0)) AS AvgScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount,
//            AVG(EXTRACT(EPOCH FROM (COALESCE(p.LastActivityDate, CURRENT_TIMESTAMP) - p.CreationDate))) AS AvgTimeToActivity
//     FROM Posts p GROUP BY p.PostTypeId)
// SELECT pt.Name, ps.TotalPosts, ps.PositiveScorePosts, ps.NegativeScorePosts, ps.TotalViews,
//        ps.AvgScore, ps.AvgViewCount, ps.AvgTimeToActivity
// FROM PostTypes pt JOIN PostStats ps ON pt.Id = ps.PostTypeId
// ORDER BY ps.TotalPosts DESC;
//
// The COALESCE with CURRENT_TIMESTAMP makes both sides TIMESTAMPTZ in the
// session zone, so the difference is elapsed time across DST changes.
fn q12537(db: &'static So) -> String {
    let Post { post_type, score, view_count, last_activity_date, creation_date, .. } = &db.post;
    let mut out = Vec::new();
    db.post
        .group_by(post_type)
        .select(score.and(view_count.opt()).and(last_activity_date).and(creation_date))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, (0.0f64, 0.0f64)), |(n, pos, neg, w, s, t), (((sc, v), la), cd)| {
            (n + 1, pos + (sc > 0) as i64, neg + (sc < 0) as i64, w + v.unwrap_or(0), s + sc, kahan(t, tz_secs(la, cd)))
        })
        .drive(|pt, (n, pos, neg, w, s, t)| {
            out.push(row(vec![V::S(db.post_type.name.get(pt).unwrap()), V::I(n), V::I(pos), V::I(neg), V::I(w), avg(s, n), avg(w, n), fmean(t, n)]))
        });
    rows(out)
}

// WITH PostStats AS (
//     SELECT PostTypeId, COUNT(*) AS PostCount, SUM(ViewCount) AS TotalViews, AVG(Score) AS AverageScore,
//            COUNT(DISTINCT OwnerUserId) AS UniquePostOwners
//     FROM Posts GROUP BY PostTypeId),
// UserStats AS (SELECT COUNT(*) AS TotalUsers, AVG(Reputation) AS AverageReputation,
//                      SUM(UpVotes + DownVotes) AS TotalVotes FROM Users),
// VoteStats AS (SELECT VoteTypeId, COUNT(*) AS VoteCount FROM Votes GROUP BY VoteTypeId)
// SELECT PS.PostTypeId, PS.PostCount, PS.TotalViews, PS.AverageScore, PS.UniquePostOwners,
//        US.TotalUsers, US.AverageReputation, US.TotalVotes, VS.VoteTypeId, VS.VoteCount
// FROM PostStats PS CROSS JOIN UserStats US
// JOIN VoteStats VS ON VS.VoteTypeId IN (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16)
// ORDER BY PS.PostTypeId, VS.VoteTypeId;
fn q14231(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, owner_user_id, .. } = &db.post;
    let ps = db.post.group_by(post_type_id).select(view_count.opt().and(score)).fold((0i64, 0i64, 0i64, 0i64), |(n, wn, w, s), (v, sc)| {
        (n + 1, wn + v.is_some() as i64, w + v.unwrap_or(0), s + sc)
    });
    let owners = db.post.group_by(post_type_id).select(owner_user_id).count_distinct();
    let User { reputation, up_votes, down_votes, .. } = &db.user;
    let users: HashIdx<(), Id<User>> = whole(db.user.iq()).collect();
    let us = (&unit()).select((&users).select(reputation.and(up_votes).and(down_votes)).opt()).fold((0i64, 0i64, 0i64), |(n, r, v), x| match x {
        Some(((rep, u), d)) => (n + 1, r + rep, v + u + d),
        None => (n, r, v),
    });
    let vs = db
        .vote
        .with((&db.vote.vote_type_id).is_in([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16]))
        .group_by(&db.vote.vote_type_id)
        .fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&ps)
        .and((&owners).opt())
        .cross(&us)
        .cross(&vs)
        .drive(|((t, _), vt), ((((n, wn, w, s), o), (un, ur, uv)), vn)| {
            out.push(row(vec![V::I(t), V::I(n), nullable(w, wn), avg(s, n), V::I(o.unwrap_or(0)), V::I(un), avg(ur, un), nullable(uv, un), V::I(vt), V::I(vn)]))
        });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//            SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts,
//            AVG(p.ViewCount) AS AvgViewCount, AVG(p.AnswerCount) AS AvgAnswerCount,
//            AVG(p.CommentCount) AS AvgCommentCount,
//            MIN(p.CreationDate) AS FirstPostDate, MAX(p.CreationDate) AS LastPostDate
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (SELECT COUNT(u.Id) AS TotalUsers, AVG(u.Reputation) AS AvgReputation,
//                      MIN(u.CreationDate) AS FirstUserDate, MAX(u.CreationDate) AS LastUserDate FROM Users u),
// VoteStats AS (SELECT vt.Name AS VoteType, COUNT(v.Id) AS TotalVotes, AVG(v.BountyAmount) AS AvgBountyAmount
//               FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY vt.Name)
// SELECT ps.*, us.*, vs.* FROM PostStats ps, UserStats us, VoteStats vs
// ORDER BY ps.TotalPosts DESC;
fn q14922(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, creation_date, .. } = &db.post;
    let ps = db
        .post
        .group_by(type_name(db))
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(creation_date))
        .fold([0i64, 0, 0, 0, 0, 0, 0, 0, i64::MAX, i64::MIN], |a, ((((s, v), an), cc), cd)| {
            [
                a[0] + 1,
                a[1] + (s > 0) as i64,
                a[2] + (s < 0) as i64,
                a[3] + v.is_some() as i64,
                a[4] + v.unwrap_or(0),
                a[5] + an.is_some() as i64,
                a[6] + an.unwrap_or(0),
                a[7] + cc,
                a[8].min(cd),
                a[9].max(cd),
            ]
        });
    let User { reputation, creation_date: ucd, .. } = &db.user;
    let users: HashIdx<(), Id<User>> = whole(db.user.iq()).collect();
    let us = (&unit()).select((&users).select(reputation.and(ucd)).opt()).fold((0i64, 0i64, i64::MAX, i64::MIN), |(n, r, lo, hi), x| match x {
        Some((rep, d)) => (n + 1, r + rep, lo.min(d), hi.max(d)),
        None => (n, r, lo, hi),
    });
    let vs = db
        .vote
        .group_by((&db.vote.vote_type).select(&db.vote_type.name))
        .select((&db.vote.bounty_amount).opt())
        .fold((0i64, 0i64, 0i64), |(n, bn, b), x| (n + 1, bn + x.is_some() as i64, b + x.unwrap_or(0)));
    let mut out = Vec::new();
    (&ps).cross(&us).cross(&vs).drive(|((t, _), vt), ((a, (un, ur, lo, hi)), (vn, bn, b))| {
        out.push(row(vec![
            V::S(t),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            avg(a[4], a[3]),
            avg(a[6], a[5]),
            avg(a[7], a[0]),
            tmin(a[8]),
            tmax(a[9]),
            V::I(un),
            avg(ur, un),
            tmin(lo),
            tmax(hi),
            V::S(vt),
            V::I(vn),
            avg(b, bn),
        ]))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore,
//            SUM(p.ViewCount) AS TotalViews,
//            AVG(EXTRACT(EPOCH FROM (COALESCE(p.LastActivityDate, CURRENT_TIMESTAMP) - p.CreationDate))) AS AverageActiveDurationInSeconds
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name)
// SELECT p.PostType, p.TotalPosts, p.AverageScore, p.TotalViews, p.AverageActiveDurationInSeconds,
//        (SELECT COUNT(*) FROM Comments c
//         WHERE c.PostId IN (SELECT Id FROM Posts WHERE PostTypeId IN
//                            (SELECT Id FROM PostTypes WHERE Name = p.PostType))) AS TotalComments
// FROM PostStats p ORDER BY p.TotalPosts DESC;
fn q11688(db: &'static So) -> String {
    let Post { score, view_count, last_activity_date, creation_date, .. } = &db.post;
    let ps = db
        .post
        .group_by(type_name(db))
        .select(score.and(view_count.opt()).and(last_activity_date).and(creation_date))
        .fold((0i64, 0i64, 0i64, 0i64, (0.0f64, 0.0f64)), |(n, s, wn, w, t), (((sc, v), la), cd)| {
            (n + 1, s + sc, wn + v.is_some() as i64, w + v.unwrap_or(0), kahan(t, tz_secs(la, cd)))
        });
    let cc = db.comment.group_by((&db.comment.post).select(type_name(db))).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&ps).and((&cc).opt()).drive(|t, ((n, s, wn, w, d), c)| {
        out.push(row(vec![V::S(t), V::I(n), avg(s, n), nullable(w, wn), fmean(d, n), V::I(c.unwrap_or(0))]))
    });
    rows(out)
}

fn type_counts(db: &'static So) -> Fold<Str, [i64; 5]> {
    let Post { score, view_count, .. } = &db.post;
    db.post.group_by(ptype_name(db)).select(score.and(view_count.opt())).fold([0i64; 5], |a, (s, v)| {
        [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), 0]
    })
}

fn name_rep_badges(db: &'static So) -> Fold<(Str, i64), i64> {
    let User { display_name, reputation, .. } = &db.user;
    db.user
        .group_by(display_name.and(reputation))
        .select(badges_of(db).opt())
        .fold(0i64, |n, b| n + b.is_some() as i64)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AvgScore, SUM(p.ViewCount) AS TotalViews
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// TopUsers AS (
//     SELECT u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.DisplayName, u.Reputation ORDER BY u.Reputation DESC LIMIT 10)
// SELECT ps.PostType, ps.PostCount, ps.AvgScore, ps.TotalViews, tu.DisplayName AS TopUser,
//        tu.Reputation, tu.BadgeCount
// FROM PostStats ps CROSS JOIN TopUsers tu ORDER BY ps.PostType;
fn q11364(db: &'static So) -> String {
    types_x_top_users(db, 10)
}

// The same with LIMIT 5 and no final ORDER BY.
fn q12583(db: &'static So) -> String {
    types_x_top_users(db, 5)
}

fn types_x_top_users(db: &'static So, n: usize) -> String {
    let ps = type_counts(db);
    let tu = rel(top_n(drain(&name_rep_badges(db)), |&((_, r), _)| std::cmp::Reverse(r), n));
    let mut out = Vec::new();
    (&ps).cross(&tu).drive(|(t, _), (a, ((name, rep), b))| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::S(name), V::I(rep), V::I(b)]))
    });
    rows(out)
}

// WITH PostMetrics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.ViewCount) AS AvgViewCount, AVG(p.Score) AS AvgScore
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserMetrics AS (
//     SELECT u.Id AS UserId, u.Reputation, u.CreationDate, COUNT(p.Id) AS PostsCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.Reputation, u.CreationDate)
// SELECT pm.PostType, pm.PostCount, pm.AvgViewCount, pm.AvgScore, um.UserId, um.Reputation,
//        um.CreationDate, um.PostsCount
// FROM PostMetrics pm JOIN UserMetrics um ON um.PostsCount > 0 ORDER BY pm.PostType;
fn q12246(db: &'static So) -> String {
    let pm = type_counts(db);
    let um = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let mut out = Vec::new();
    (&pm).cross((&um).filt(|n| n > 0)).drive(|(t, u), (a, n)| {
        let mut f = vec![V::S(t), V::I(a[0]), avg(a[3], a[2]), avg(a[1], a[0])];
        f.extend(ucols(db, u, &["uid", "rep", "ucreated"]));
        f.push(V::I(n));
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT ps.PostType, ps.PostCount, ps.TotalScore, ps.TotalViews, us.DisplayName AS User,
//        us.Reputation, us.BadgeCount
// FROM PostStats ps JOIN UserStats us ON us.Reputation > 0
// ORDER BY ps.PostCount DESC, us.Reputation DESC;
fn q13570(db: &'static So) -> String {
    let ps = type_counts(db);
    let us = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt())
        .fold(0i64, |n, b| n + b.is_some() as i64);
    let mut out = Vec::new();
    (&ps).cross(&us).drive(|(t, u), (a, b)| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2])];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(b));
        out.push(row(f))
    });
    rows(out)
}

// WITH PostMetrics AS (
//     SELECT PT.Name AS PostType, COUNT(P.Id) AS PostCount, AVG(P.ViewCount) AS AvgViews, AVG(P.Score) AS AvgScore
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name),
// TopUsers AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS TotalPosts
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation ORDER BY U.Reputation DESC LIMIT 5)
// SELECT PM.PostType, PM.PostCount, PM.AvgViews, PM.AvgScore, TU.DisplayName AS TopUser,
//        TU.Reputation, TU.TotalPosts
// FROM PostMetrics PM CROSS JOIN TopUsers TU ORDER BY PM.PostCount DESC;
fn q10584(db: &'static So) -> String {
    let pm = type_counts(db);
    let per = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = rel(top_n(drain(&per), |&(u, _)| std::cmp::Reverse(db.user.reputation.get(u).unwrap()), 5));
    let mut out = Vec::new();
    (&pm).cross(&tu).drive(|(t, _), (a, (u, n))| {
        let mut f = vec![V::S(t), V::I(a[0]), avg(a[3], a[2]), avg(a[1], a[0])];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        out.push(row(f))
    });
    rows(out)
}

// The file holds two statements; DuckDB returns the result of the last one,
// which is what the oracle holds, so that is the one ported:
//
// SELECT (SELECT COUNT(*) FROM Posts) AS TotalPosts, ... (SELECT COUNT(*) FROM PostLinks);
// SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalAnswers,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 2
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName ORDER BY TotalAnswers DESC LIMIT 100;
fn q14612(db: &'static So) -> String {
    let answers = db.post.with((&db.post.post_type_id).eq(2)).select(&db.post.owner_user).inv().collect::<HashIdx<Id<User>, Id<Post>>>();
    let votes = db
        .user
        .group_by(Ident::<User>::new())
        .select((&answers).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold((0i64, 0i64), |(u, d), x| match x {
            Some(Some(t)) => (u + (t == 2) as i64, d + (t == 3) as i64),
            _ => (u, d),
        });
    let n = db.user.group_by(Ident::<User>::new()).select(&answers).count_distinct();
    let v = top_n(drain((&votes).and((&n).opt())), |&(u, (_, n))| (std::cmp::Reverse(n.unwrap_or(0)), u), 0);
    let v = top_n(v, |&(_, (_, n))| std::cmp::Reverse(n.unwrap_or(0)), 100);
    rows(v.iter().map(|&(u, ((up, down), n))| {
        let n = n.unwrap_or(0);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(up), V::I(down)]);
        row(f)
    }))
}

// WITH UserPostCounts AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount FROM UserPostCounts ORDER BY PostCount DESC LIMIT 10),
// RecentPosts AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Author
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id ORDER BY p.CreationDate DESC LIMIT 10)
// SELECT u.DisplayName AS TopUser, u.PostCount, r.PostId, r.Title AS RecentPostTitle,
//        r.CreationDate AS RecentPostDate, r.Author
// FROM TopUsers u FULL OUTER JOIN RecentPosts r ON u.DisplayName = r.Author
// ORDER BY u.PostCount DESC, r.CreationDate DESC;
fn q11134(db: &'static So) -> String {
    let per = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = rel(top_n(drain(&per), |&(_, n)| std::cmp::Reverse(n), 10));
    let rp: MatSet<Id<Post>> = rel(top_n(drain(owned(db).select(&db.post.creation_date)), |&(_, d)| std::cmp::Reverse(d), 10)).map(|(p, _)| p).collect();
    let author = (&db.post.owner_user).select(&db.user.display_name);
    let by_author: HashIdx<Str, Id<Post>> = db.post.with(&rp).select(&author).inv().collect();
    let names: MatSet<Str> = (&tu).map(|(u, _)| u).select(&db.user.display_name).collect();
    let recent = |p: Id<Post>| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(post_fields(db, p, &["owner"]).remove(0));
        f
    };
    type R = (Option<(Id<User>, i64)>, Option<Id<Post>>);
    let top_users = (&tu).map(|(u, _)| u).inv().map(|_| ()).inv();
    let left = top_users.select(Ident::<User>::new().and(&per).and((&db.user.display_name).select((&by_author).opt()))).map(|(u, p)| -> R { (Some(u), p) });
    let right = whole(db.post.with(&rp).minus((&author).with(&names))).map(|p| -> R { (None, Some(p)) });
    let mut out = Vec::new();
    left.union(right).drive(|_, (u, p): R| {
        let mut f = u.map_or(vec![V::Null, V::Null], |(u, n)| vec![user_col(db, u, "name"), V::I(n)]);
        f.extend(p.map_or_else(|| (0..4).map(|_| V::Null).collect(), recent));
        out.push(row(f))
    });
    rows(out)
}

fn stats_by_type(db: &'static So) -> Fold<Str, [i64; 10]> {
    let Post { score, view_count, answer_count, comment_count, favorite_count, .. } = &db.post;
    db.post
        .group_by(ptype_name(db))
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()))
        .fold([0i64; 10], |a, ((((s, v), an), cc), f)| {
            [
                a[0] + 1,
                a[1] + s,
                a[2] + v.is_some() as i64,
                a[3] + v.unwrap_or(0),
                a[4] + an.is_some() as i64,
                a[5] + an.unwrap_or(0),
                a[6] + cc,
                a[7] + f.is_some() as i64,
                a[8] + f.unwrap_or(0),
                0,
            ]
        })
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore,
//            AVG(p.ViewCount) AS AverageViewCount, AVG(p.AnswerCount) AS AverageAnswerCount,
//            AVG(p.CommentCount) AS AverageCommentCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount,
//            SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT ps.PostType, ps.PostCount, ps.TotalScore, ps.AverageViewCount, ps.AverageAnswerCount,
//        ps.AverageCommentCount, us.UserId, us.DisplayName, us.BadgeCount, us.TotalUpVotes, us.TotalDownVotes
// FROM PostStats ps JOIN UserStats us ON us.BadgeCount > 0
// ORDER BY ps.PostCount DESC, us.TotalUpVotes DESC;
fn q14810(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(badges_of(db).opt()))
        .fold((0i64, 0i64, 0i64), |(n, u, d), ((up, dn), b)| (n + b.is_some() as i64, u + up, d + dn));
    let mut out = Vec::new();
    (&ps).cross((&us).filt(|(n, _, _)| n > 0)).drive(|(t, u), (a, (n, up, dn))| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0])];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(n), V::I(up), V::I(dn)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStatistics AS (
//     SELECT PT.Name AS PostType, COUNT(P.Id) AS TotalPosts, AVG(P.Score) AS AvgScore,
//            SUM(P.ViewCount) AS TotalViews, SUM(P.AnswerCount) AS TotalAnswers,
//            SUM(P.CommentCount) AS TotalComments, SUM(P.FavoriteCount) AS TotalFavorites
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name),
// UserStatistics AS (
//     SELECT U.DisplayName AS UserName, COUNT(P.Id) AS TotalPosts, AVG(U.Reputation) AS AvgReputation,
//            SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.DisplayName)
// SELECT PS.PostType, PS.TotalPosts, PS.AvgScore, PS.TotalViews, PS.TotalAnswers, PS.TotalComments,
//        PS.TotalFavorites, US.UserName, US.TotalPosts AS UserTotalPosts, US.AvgReputation,
//        US.TotalViews AS UserTotalViews
// FROM PostStatistics PS LEFT JOIN UserStatistics US ON US.TotalPosts > 0
// ORDER BY PS.TotalPosts DESC, US.TotalPosts DESC;
fn q13038(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select((&db.user.reputation).and(posts_of(db).select((&db.post.view_count).opt()).opt()))
        .fold([0i64; 5], |a, (r, p)| [a[0] + 1, a[1] + p.is_some() as i64, a[2] + r, a[3] + p.flatten().is_some() as i64, a[4] + p.flatten().unwrap_or(0)]);
    let us: HashIdx<(), (Str, [i64; 5])> = (&us).filt(|a| a[1] > 0).map(|_| ()).inv().select(Same::<Str>::new().and(&us)).collect();
    let mut out = Vec::new();
    (&ps).and((&ps).map(|_| ()).select(&us).opt()).drive(|t, (a, u)| {
        let mut f = vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), nullable(a[5], a[4]), V::I(a[6]), nullable(a[8], a[7])];
        f.extend(match u {
            Some((name, u)) => [V::S(name), V::I(u[1]), avg(u[2], u[0]), nullable(u[4], u[3])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f));
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT P.Id AS PostId, P.PostTypeId, P.Score, P.ViewCount,
//            COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount,
//            COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id), 0) AS VoteCount,
//            P.CreationDate, P.LastActivityDate
//     FROM Posts P),
// UserStats AS (
//     SELECT U.Id AS UserId, COUNT(DISTINCT P.Id) AS PostCount, SUM(U.UpVotes) AS TotalUpVotes,
//            SUM(U.DownVotes) AS TotalDownVotes
//     FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id),
// Summary AS (
//     SELECT P.PostTypeId, AVG(P.Score) AS AvgScore, AVG(P.ViewCount) AS AvgViewCount,
//            SUM(P.CommentCount) AS TotalComments, SUM(P.VoteCount) AS TotalVotes,
//            COUNT(DISTINCT P.PostId) AS NumberOfPosts
//     FROM PostStats P GROUP BY P.PostTypeId)
// SELECT S.PostTypeId, S.AvgScore, S.AvgViewCount, S.TotalComments, S.TotalVotes,
//        U.PostCount AS TotalUsers, U.TotalUpVotes, U.TotalDownVotes
// FROM Summary S JOIN UserStats U ON U.PostCount > 0 ORDER BY S.PostTypeId;
fn q11544(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let (cp, vp) = (comments_per_post(db), votes_per_post(db));
    let s = db
        .post
        .group_by(post_type_id)
        .select(score.and(view_count.opt()).and(&cp).and(&vp))
        .fold([0i64; 6], |a, (((sc, v), c), x)| [a[0] + 1, a[1] + sc, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4] + c, a[5] + x]);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db)))
        .fold((0i64, 0i64), |(u, d), ((up, dn), _)| (u + up, d + dn));
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let mut out = Vec::new();
    (&s).cross((&dp).filt(|n| n > 0).and(&us)).drive(|(t, _), (a, (n, (up, dn)))| {
        out.push(row(vec![V::I(t), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::I(n), V::I(up), V::I(dn)]))
    });
    rows(out)
}

// WITH PostDetails AS (
//     SELECT pt.Name AS PostTypeName, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AvgScore,
//            COUNT(DISTINCT v.UserId) AS UserCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY pt.Name),
// VoteDetails AS (
//     SELECT vt.Name AS VoteTypeName, COUNT(v.Id) AS VoteCount
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY vt.Name)
// SELECT pd.PostTypeName, pd.PostCount, pd.AvgScore, pd.UserCount, vd.VoteTypeName, vd.VoteCount
// FROM PostDetails pd CROSS JOIN VoteDetails vd ORDER BY pd.PostTypeName, vd.VoteTypeName;
fn q11510(db: &'static So) -> String {
    let pd = db
        .post
        .group_by(ptype_name(db))
        .select((&db.post.score).and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, s), (sc, _)| (n + 1, s + sc));
    let uc = db.post.group_by(ptype_name(db)).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let vd = db.vote.group_by(vtype_name(db)).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&pd).and((&uc).opt()).cross(&vd).drive(|(t, vt), (((n, s), u), vn)| {
        out.push(row(vec![V::S(t), V::I(n), avg(s, n), V::I(u.unwrap_or(0)), V::S(vt), V::I(vn)]))
    });
    rows(out)
}

// WITH PostTypeStats AS (
//     SELECT pt.Name AS PostTypeName, COUNT(p.Id) AS PostCount, AVG(u.Reputation) AS AverageUserReputation
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id
//     GROUP BY pt.Name),
// PostHistoryStats AS (
//     SELECT pht.Name AS PostHistoryTypeName, COUNT(ph.Id) AS HistoryCount
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY pht.Name)
// SELECT pts.PostTypeName, pts.PostCount, pts.AverageUserReputation, phs.PostHistoryTypeName, phs.HistoryCount
// FROM PostTypeStats pts LEFT JOIN PostHistoryStats phs ON phs.PostHistoryTypeName IS NOT NULL
// ORDER BY pts.PostCount DESC;
fn q11488(db: &'static So) -> String {
    let pts = owned(db).group_by(ptype_name(db)).select((&db.post.owner_user).select(&db.user.reputation)).fold((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let phs = db.post_history.group_by(htype_name(db)).fold(0i64, |n, _| n + 1);
    let phs: HashIdx<(), (Str, i64)> = (&phs).map(|_| ()).inv().select(Same::<Str>::new().and(&phs)).collect();
    let mut out = Vec::new();
    (&pts).and((&pts).map(|_| ()).select(&phs).opt()).drive(|t, ((n, s), h)| {
        out.push(row(vec![V::S(t), V::I(n), avg(s, n), h.map_or(V::Null, |h| V::S(h.0)), h.map_or(V::Null, |h| V::I(h.1))]))
    });
    rows(out)
}

// WITH PostMetrics AS (
//     SELECT pt.Name AS PostTypeName, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore,
//            SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     GROUP BY pt.Name),
// UserReputation AS (SELECT u.DisplayName, u.Reputation FROM Users u ORDER BY u.Reputation DESC LIMIT 10)
// SELECT pm.PostTypeName, pm.PostCount, pm.AverageScore, pm.TotalComments, ur.DisplayName AS TopUser, ur.Reputation
// FROM PostMetrics pm CROSS JOIN UserReputation ur ORDER BY pm.PostCount DESC;
fn q13344(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let pm = db.post.group_by(ptype_name(db)).select((&db.post.score).and((&cc).opt())).fold((0i64, 0i64, 0i64), |(n, s, c), (sc, x)| (n + 1, s + sc, c + x.unwrap_or(0)));
    let ur = rel(top_n(drain(&db.user.reputation), |&(_, r)| std::cmp::Reverse(r), 10));
    let mut out = Vec::new();
    (&pm).cross(&ur).drive(|(t, _), ((n, s, c), (u, r))| out.push(row(vec![V::S(t), V::I(n), avg(s, n), V::I(c), user_col(db, u, "name"), V::I(r)])));
    rows(out)
}

// WITH PostSummary AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AvgScore,
//            AVG(p.ViewCount) AS AvgViewCount, AVG(COALESCE(p.AnswerCount, 0)) AS AvgAnswerCount
//     FROM Posts p INNER JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// ActiveUsers AS (
//     SELECT u.Id, u.DisplayName, COUNT(p.Id) AS PostCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.DisplayName ORDER BY PostCount DESC LIMIT 10)
// SELECT ps.PostType, ps.PostCount, ps.AvgScore, ps.AvgViewCount, ps.AvgAnswerCount,
//        au.DisplayName AS ActiveUser, au.PostCount AS UserPostCount
// FROM PostSummary ps LEFT JOIN ActiveUsers au ON true ORDER BY ps.PostType;
fn q14547(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let per = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let au = rel(top_n(drain(&per), |&(_, n)| std::cmp::Reverse(n), 10));
    let au: HashIdx<(), (Id<User>, i64)> = (&au).map(|_| ()).inv().select(&au).collect();
    let mut out = Vec::new();
    (&ps).and((&ps).map(|_| ()).select(&au).opt()).drive(|t, (a, x)| {
        let (name, n) = x.map_or((V::Null, V::Null), |(u, n)| (user_col(db, u, "name"), V::I(n)));
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), avg(a[5], a[0]), name, n]))
    });
    rows(out)
}

// WITH PostCounts AS (SELECT CAST(CreationDate AS DATE) AS PostDate, COUNT(*) AS TotalPosts
//                     FROM Posts GROUP BY CAST(CreationDate AS DATE)),
// UserCounts AS (... the same over Users ...), VoteCounts AS (... the same over Votes ...)
// SELECT COALESCE(p.PostDate, u.UserDate, v.VoteDate) AS ActivityDate,
//        COALESCE(p.TotalPosts, 0) AS Posts, COALESCE(u.TotalUsers, 0) AS Users,
//        COALESCE(v.TotalVotes, 0) AS Votes
// FROM PostCounts p FULL OUTER JOIN UserCounts u ON p.PostDate = u.UserDate
// FULL OUTER JOIN VoteCounts v ON COALESCE(p.PostDate, u.UserDate) = v.VoteDate
// ORDER BY ActivityDate;
//
// Both joins are on the one date, so the rows are the union of the three
// date sets, each probed for its count.
fn q11388(db: &'static So) -> String {
    let pc = db.post.group_by((&db.post.creation_date).map(trunc_day)).fold(0i64, |n, _| n + 1);
    let uc = db.user.group_by((&db.user.creation_date).map(trunc_day)).fold(0i64, |n, _| n + 1);
    let vc = db.vote.group_by((&db.vote.creation_date).map(trunc_day)).fold(0i64, |n, _| n + 1);
    let days: MatSet<i64> = (&pc).map(|_| ()).inv().union((&uc).map(|_| ()).inv()).union((&vc).map(|_| ()).inv()).collect();
    let mut out = Vec::new();
    (&days).select(Same::new().and((&pc).opt()).and((&uc).opt()).and((&vc).opt())).drive(|_, (((d, p), u), v)| {
        out.push(row(vec![V::D(d), V::I(p.unwrap_or(0)), V::I(u.unwrap_or(0)), V::I(v.unwrap_or(0))]))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserActivity AS (
//     SELECT u.DisplayName, COUNT(p.Id) AS PostsCreated,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCreated,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCreated,
//            MAX(p.CreationDate) AS LastActivityDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.DisplayName)
// SELECT pc.PostType, pc.TotalPosts, ua.DisplayName, ua.PostsCreated, ua.QuestionsCreated,
//        ua.AnswersCreated, ua.LastActivityDate
// FROM PostCounts pc JOIN UserActivity ua ON ua.PostsCreated > 0
// ORDER BY pc.TotalPosts DESC, ua.PostsCreated DESC;
fn q10879(db: &'static So) -> String {
    let pc = db.post.group_by(ptype_name(db)).fold(0i64, |n, _| n + 1);
    let ua = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.creation_date)).opt())
        .fold([0i64, 0, 0, i64::MIN], |a, p| match p {
            Some((t, d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(d)],
            None => a,
        });
    let mut out = Vec::new();
    (&pc).cross((&ua).filt(|a| a[0] > 0)).drive(|(t, name), (n, a)| {
        out.push(row(vec![V::S(t), V::I(n), V::S(name), V::I(a[0]), V::I(a[1]), V::I(a[2]), tmax(a[3])]))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, SUM(p.ViewCount) AS TotalViews,
//            AVG(p.Score) AS AvgScore,
//            AVG(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate)) / 3600) AS AvgAgeHours
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.DisplayName, COUNT(b.Id) AS TotalBadges, SUM(b.Class) AS BadgeScore,
//            AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT ps.PostType, ps.TotalPosts, ps.TotalViews, ps.AvgScore, ps.AvgAgeHours, us.DisplayName,
//        us.TotalBadges, us.BadgeScore, us.AvgReputation
// FROM PostStats ps JOIN UserStats us ON us.AvgReputation > 1000
// ORDER BY ps.TotalPosts DESC, us.AvgReputation DESC;
fn q13670(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let Post { view_count, score, creation_date, .. } = &db.post;
    let ps = db
        .post
        .group_by(ptype_name(db))
        .select(view_count.opt().and(score).and(creation_date))
        .fold((0i64, 0i64, 0i64, 0i64, (0.0f64, 0.0f64)), |(n, wn, w, s, h), ((v, sc), cd)| {
            (n + 1, wn + v.is_some() as i64, w + v.unwrap_or(0), s + sc, kahan(h, secs(now - cd) / 3600.0))
        });
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (r, c)| [a[0] + 1, a[1] + c.is_some() as i64, a[2] + c.unwrap_or(0), a[3] + r]);
    let mut out = Vec::new();
    (&ps).cross((&us).filt(|a| a[3] as f64 / a[0] as f64 > 1000.0)).drive(|(t, name), ((n, wn, w, s, h), a)| {
        out.push(row(vec![V::S(t), V::I(n), nullable(w, wn), avg(s, n), fmean(h, n), V::S(name), V::I(a[1]), nullable(a[2], a[1]), avg(a[3], a[0])]))
    });
    rows(out)
}

// WITH PostCount AS (
//     SELECT PT.Name AS PostType, COUNT(P.Id) AS TotalPosts
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name),
// UserEngagement AS (
//     SELECT U.DisplayName, COUNT(DISTINCT C.Id) AS TotalComments, COUNT(DISTINCT V.Id) AS TotalVotes
//     FROM Users U LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Votes V ON U.Id = V.UserId
//     GROUP BY U.DisplayName),
// BadgeDistribution AS (SELECT B.Class, COUNT(B.Id) AS TotalBadges FROM Badges B GROUP BY B.Class)
// SELECT PC.PostType, PC.TotalPosts, UE.DisplayName, UE.TotalComments, UE.TotalVotes, BD.Class, BD.TotalBadges
// FROM PostCount PC JOIN UserEngagement UE ON UE.TotalComments > 0 OR UE.TotalVotes > 0
// JOIN BadgeDistribution BD ON BD.TotalBadges > 0
// ORDER BY PC.TotalPosts DESC, UE.TotalVotes DESC;
//
// A DISTINCT count over the comments x votes product is the distinct count
// of each side, so each is counted over its own join.
fn q12374(db: &'static So) -> String {
    let pc = db.post.group_by(ptype_name(db)).fold(0i64, |n, _| n + 1);
    let dc = db.user.group_by(&db.user.display_name).select(comments_by(db)).count_distinct();
    let dv = db.user.group_by(&db.user.display_name).select(votes_by(db)).count_distinct();
    let names: MatSet<Str> = (&db.user.display_name).collect();
    let ue = drain((&names).select(Same::new().and((&dc).opt()).and((&dv).opt())).filt(|((_, c), v)| c.unwrap_or(0) > 0 || v.unwrap_or(0) > 0));
    let bd = db.badge.group_by(&db.badge.class).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&pc).cross(rel(ue)).cross((&bd).filt(|n| n > 0)).drive(|((t, _), cl), ((n, (_, ((name, c), v))), bn)| {
        out.push(row(vec![V::S(t), V::I(n), V::S(name), V::I(c.unwrap_or(0)), V::I(v.unwrap_or(0)), V::I(cl), V::I(bn)]))
    });
    rows(out)
}

// WITH PostCounts AS (SELECT PostTypeId, COUNT(*) AS TotalPosts FROM Posts GROUP BY PostTypeId),
// UserCounts AS (SELECT COUNT(*) AS TotalUsers FROM Users),
// BadgeCounts AS (SELECT COUNT(*) AS TotalBadges FROM Badges),
// VoteCounts AS (SELECT VoteTypeId, COUNT(*) AS TotalVotes FROM Votes GROUP BY VoteTypeId),
// CommentCounts AS (SELECT PostId, COUNT(*) AS TotalComments FROM Comments GROUP BY PostId)
// SELECT pc.PostTypeId, pc.TotalPosts, uc.TotalUsers, bc.TotalBadges,
//        COALESCE(v.TotalVotes, 0) AS TotalVotes, COALESCE(cc.TotalComments, 0) AS TotalComments
// FROM PostCounts pc CROSS JOIN UserCounts uc CROSS JOIN BadgeCounts bc
// LEFT JOIN VoteCounts v ON v.VoteTypeId IN (1, 2, 3, 4, 5)
// LEFT JOIN CommentCounts cc ON cc.PostId = pc.PostTypeId
// ORDER BY pc.PostTypeId;
fn q13467(db: &'static So) -> String {
    let pc = db.post.group_by(&db.post.post_type_id).fold(0i64, |n, _| n + 1);
    let uc = count(&db.user.id);
    let bc = count(&db.badge.id);
    let vc = db.vote.with((&db.vote.vote_type_id).is_in([1, 2, 3, 4, 5])).group_by(&db.vote.vote_type_id).fold(0i64, |n, _| n + 1);
    let cc = db.comment.group_by(&db.comment.post_id).fold(0i64, |n, _| n + 1);
    let vc: HashIdx<(), (i64, i64)> = (&vc).map(|_| ()).inv().select(Same::<i64>::new().and(&vc)).collect();
    let mut out = Vec::new();
    (&pc).and((&cc).opt()).and((&pc).map(|_| ()).select(&vc).opt()).drive(|t, ((n, c), v)| {
        out.push(row(vec![V::I(t), V::I(n), V::I(uc), V::I(bc), V::I(v.map_or(0, |v| v.1)), V::I(c.unwrap_or(0))]))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//            COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
//            COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//            SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostTypeCounts AS (
//     SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(p.Id) AS NumberOfPosts
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Id, pt.Name)
// SELECT ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalScore,
//        ups.TotalViews, ptc.PostTypeName, ptc.NumberOfPosts
// FROM UserPostStats ups JOIN PostTypeCounts ptc ON ptc.NumberOfPosts > 0
// ORDER BY ups.TotalScore DESC, ups.TotalPosts DESC;
fn q14934(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0)],
            None => a,
        });
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let ptc = db
        .post_type
        .group_by(Ident::<PostType>::new())
        .select((&of_type).opt())
        .fold(0i64, |n, p| n + p.is_some() as i64);
    let mut out = Vec::new();
    (&ups).cross((&ptc).filt(|n| n > 0)).drive(|(u, t), (a, n)| {
        out.push(row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), nullable(a[5], a[4]), V::S(db.post_type.name.get(t).unwrap()), V::I(n)]))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AvgScore,
//            SUM(p.ViewCount) AS TotalViews, SUM(p.AnswerCount) AS TotalAnswers,
//            SUM(p.FavoriteCount) AS TotalFavorites
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS TotalBadges, SUM(v.BountyAmount) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.DisplayName)
// SELECT ps.PostType, ps.TotalPosts, ps.AvgScore, ps.TotalViews, ps.TotalAnswers, ps.TotalFavorites,
//        us.UserId, us.DisplayName, us.TotalBadges, us.TotalBounties
// FROM PostStats ps LEFT JOIN UserStats us ON us.UserId IS NOT NULL
// ORDER BY ps.TotalPosts DESC, us.TotalBadges DESC;
fn q11681(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64, 0i64), |(b, bn, bs), (x, v)| {
            let v = v.flatten();
            (b + x.is_some() as i64, bn + v.is_some() as i64, bs + v.unwrap_or(0))
        });
    let us: HashIdx<(), (Id<User>, (i64, i64, i64))> = (&us).map(|_| ()).inv().select(Ident::<User>::new().and(&us)).collect();
    let mut out = Vec::new();
    (&ps).and((&ps).map(|_| ()).select(&us).opt()).drive(|t, (a, x)| {
        let mut f = vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), nullable(a[5], a[4]), nullable(a[8], a[7])];
        match x {
            Some((u, (b, bn, bs))) => {
                f.extend(ucols(db, u, &["uid", "name"]));
                f.extend([V::I(b), nullable(bs, bn)]);
            }
            None => f.extend((0..4).map(|_| V::Null)),
        }
        out.push(row(f))
    });
    rows(out)
}

/// Per user, over `Users LEFT JOIN Posts`: [joined rows, posts, questions,
/// answers, score sum, views present, views sum, latest creation, score > 0,
/// reputation summed over the rows].
fn user_posts(db: &'static So) -> Fold<Id<User>, [i64; 10]> {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    db.user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).opt()))
        .fold([0, 0, 0, 0, 0, 0, 0, i64::MIN, 0, 0], |a, (r, p)| match p {
            Some((((t, s), v), d)) => [
                a[0] + 1,
                a[1] + 1,
                a[2] + (t == 1) as i64,
                a[3] + (t == 2) as i64,
                a[4] + s,
                a[5] + v.is_some() as i64,
                a[6] + v.unwrap_or(0),
                a[7].max(d),
                a[8] + (s > 0) as i64,
                a[9] + r,
            ],
            None => {
                let mut a = a;
                a[0] += 1;
                a[9] += r;
                a
            }
        })
}

/// `PostTypes LEFT JOIN Posts` grouped by the type: [posts, score sum, views
/// present, views sum], every type included.
fn type_left_posts(db: &'static So) -> Fold<Id<PostType>, [i64; 4]> {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    db.post_type
        .group_by(Ident::<PostType>::new())
        .select(of_type.select((&db.post.score).and((&db.post.view_count).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, v)) => [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)],
            None => a,
        })
}

/// `type_left_posts` grouped by the type's name.
fn type_left_posts_by_name(db: &'static So) -> Fold<Str, [i64; 4]> {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    db.post_type
        .group_by(&db.post_type.name)
        .select(of_type.select((&db.post.score).and((&db.post.view_count).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, v)) => [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)],
            None => a,
        })
}

// WITH PostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//            SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//            SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, COUNT(c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// PostTypesStats AS (
//     SELECT pt.Name AS PostTypeName, COUNT(p.Id) AS TotalPosts
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Name)
// SELECT ps.UserId, ps.DisplayName, ps.PostCount, ps.UpvoteCount, ps.DownvoteCount, ps.CommentCount,
//        pts.PostTypeName, pts.TotalPosts
// FROM PostStats ps JOIN PostTypesStats pts ON ps.PostCount > 0
// ORDER BY ps.PostCount DESC, ps.UpvoteCount DESC;
fn q13115(db: &'static So) -> String {
    let ps = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((v, c)) => [a[0] + 1, a[1] + (v == Some(2)) as i64, a[2] + (v == Some(3)) as i64, a[3] + c.is_some() as i64],
            None => a,
        });
    let pts = type_left_posts_by_name(db);
    let mut out = Vec::new();
    (&ps).filt(|a| a[0] > 0).cross(&pts).drive(|(u, t), (a, b)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::S(t), V::I(b[0])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT p.OwnerUserId) AS UniquePosters,
//            SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AverageViews, AVG(p.AnswerCount) AS AverageAnswers
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.DisplayName, u.Reputation, COUNT(b.Id) AS TotalBadges, SUM(v.BountyAmount) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT ps.PostType, ps.TotalPosts, ps.UniquePosters, ps.TotalScore, ps.AverageViews, ps.AverageAnswers,
//        us.DisplayName, us.Reputation, us.TotalBadges, us.TotalBounties
// FROM PostStats ps JOIN UserStats us ON us.Reputation > 1000
// ORDER BY ps.TotalPosts DESC, us.Reputation DESC;
fn q14777(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let up = db.post.group_by(ptype_name(db)).select(&db.post.owner_user_id).count_distinct();
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64, 0i64), |(b, bn, bs), (x, v)| {
            let v = v.flatten();
            (b + x.is_some() as i64, bn + v.is_some() as i64, bs + v.unwrap_or(0))
        });
    let mut out = Vec::new();
    (&ps).and((&up).opt()).cross(&us).drive(|(t, u), ((a, n), (b, bn, bs))| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(n.unwrap_or(0)), V::I(a[1]), avg(a[3], a[2]), avg(a[5], a[4])];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), nullable(bs, bn)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//            SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore, MAX(p.CreationDate) AS LatestPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostTypesCount AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Id, pt.Name)
// SELECT u.UserId, u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalViews,
//        u.TotalScore, u.LatestPostDate, pt.PostType, pt.PostCount
// FROM UserPostStats u LEFT JOIN PostTypesCount pt ON pt.PostCount > 0
// ORDER BY u.TotalScore DESC, u.TotalPosts DESC, u.DisplayName;
fn q13234(db: &'static So) -> String {
    let ups = user_posts(db);
    let pt = type_left_posts(db);
    let pt: HashIdx<(), (Id<PostType>, [i64; 4])> = (&pt).filt(|b| b[0] > 0).map(|_| ()).inv().select(Ident::<PostType>::new().and(&pt)).collect();
    let mut out = Vec::new();
    (&ups).and((&ups).map(|_| ()).select(&pt).opt()).drive(|u, (a, x)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), tmax(a[7])]);
        f.extend(x.map_or([V::Null, V::Null], |(t, b)| [V::S(db.post_type.name.get(t).unwrap()), V::I(b[0])]));
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (
//     SELECT PT.Name AS PostType, COUNT(P.Id) AS TotalPosts, AVG(P.ViewCount) AS AverageViews,
//            AVG(P.Score) AS AverageScore
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name)
// SELECT U.UserId, U.DisplayName, U.PostCount, U.QuestionCount, U.AnswerCount, U.TotalViews,
//        U.AverageScore, PS.PostType, PS.TotalPosts, PS.AverageViews, PS.AverageScore
// FROM UserPostStats U JOIN PostStatistics PS ON U.PostCount > 0
// ORDER BY U.TotalViews DESC, U.AverageScore DESC;
fn q12391(db: &'static So) -> String {
    let ups = user_posts(db);
    let ps = stats_by_type(db);
    let mut out = Vec::new();
    (&ups).filt(|a| a[1] > 0).cross(&ps).drive(|(u, t), (a, b)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), avg(a[4], a[1]), V::S(t), V::I(b[0]), avg(b[3], b[2]), avg(b[1], b[0])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, COUNT(DISTINCT P.Id) AS PostCount,
//            SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     GROUP BY U.Id, U.Reputation),
// PostTypesStats AS (
//     SELECT PT.Id AS PostTypeId, PT.Name AS PostTypeName, COUNT(P.Id) AS PostCount,
//            SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews
//     FROM PostTypes PT LEFT JOIN Posts P ON PT.Id = P.PostTypeId GROUP BY PT.Id, PT.Name)
// SELECT US.UserId, US.Reputation, US.BadgeCount, US.PostCount, US.TotalScore, US.TotalViews,
//        US.LastPostDate, PTS.PostTypeId, PTS.PostTypeName, PTS.PostCount AS PostsPerType,
//        PTS.TotalScore AS ScorePerType, PTS.TotalViews AS ViewsPerType
// FROM UserStats US JOIN PostTypesStats PTS ON US.PostCount > 0
// ORDER BY US.Reputation DESC, PTS.PostCount DESC;
fn q13800(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(score.and(view_count.opt()).and(creation_date)).opt()))
        .fold([0, 0, 0, 0, i64::MIN], |a, (_, p)| match p {
            Some(((s, v), d)) => [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4].max(d)],
            None => a,
        });
    let db_ = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).count_distinct();
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let pts = type_left_posts(db);
    let mut out = Vec::new();
    (&dp).filt(|n| n > 0).and(&us).and((&db_).opt()).cross(&pts).drive(|(u, t), (((n, a), b), p)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(n), nullable(a[1], a[0]), nullable(a[3], a[2]), tmax(a[4])]);
        f.extend([V::I(db.post_type.origid.get(t).unwrap()), V::S(db.post_type.name.get(t).unwrap()), V::I(p[0]), nullable(p[1], p[0]), nullable(p[3], p[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserVoteCounts AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS VoteCount,
//            SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (
//     SELECT P.Id AS PostId, P.Title, P.PostTypeId, COUNT(C.Id) AS CommentCount,
//            SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title, P.PostTypeId)
// SELECT U.DisplayName, UV.VoteCount, UV.UpVoteCount, UV.DownVoteCount, PS.PostId, PS.Title,
//        PS.PostTypeId, PS.CommentCount, PS.TotalScore, PS.TotalViews
// FROM UserVoteCounts UV
// JOIN PostStatistics PS ON PS.PostId = (SELECT P.Id FROM Posts P ORDER BY P.Score DESC LIMIT 1)
// JOIN Users U ON U.Id = UV.UserId
// ORDER BY PS.TotalScore DESC;
fn q14904(db: &'static So) -> String {
    let best: MatSet<Id<Post>> = rel(top_n(drain(&db.post.score), |&(_, s)| std::cmp::Reverse(s), 1)).map(|(p, _)| p).collect();
    let Post { score, view_count, .. } = &db.post;
    let ps = db
        .post
        .with(&best)
        .group_by(Ident::<Post>::new())
        .select(score.and(view_count.opt()).and(comments_of(db).opt()))
        .fold([0i64; 4], |a, ((s, v), c)| [a[0] + c.is_some() as i64, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]);
    let uv = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| match t {
            Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let mut out = Vec::new();
    (&uv).cross(&ps).drive(|(u, p), (a, b)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["id", "title", "type_id"]));
        f.extend([V::I(b[0]), V::I(b[1]), nullable(b[3], b[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostTypeCounts AS (
//     SELECT pt.Name AS PostTypeName, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//            AVG(p.ViewCount) AS AvgViewCount, AVG(p.Score) AS AvgScore
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserActivity AS (
//     SELECT u.DisplayName AS UserName, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//            AVG(p.ViewCount) AS AvgViewCount, AVG(p.Score) AS AvgScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.DisplayName)
// SELECT ptc.PostTypeName, ptc.TotalPosts, ptc.PositiveScorePosts, ptc.AvgViewCount, ptc.AvgScore,
//        ua.UserName, ua.TotalPosts AS UserTotalPosts, ua.PositiveScorePosts AS UserPositiveScorePosts,
//        ua.AvgViewCount AS UserAvgViewCount, ua.AvgScore AS UserAvgScore
// FROM PostTypeCounts ptc CROSS JOIN UserActivity ua ORDER BY ptc.PostTypeName, ua.UserName;
fn q11336(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let ptc = db.post.group_by(ptype_name(db)).select(score.and(view_count.opt())).fold([0i64; 5], |a, (s, v)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4] + s]
    });
    let ua = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(score.and(view_count.opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((s, v)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4] + s],
            None => a,
        });
    let mut out = Vec::new();
    (&ptc).cross(&ua).drive(|(t, name), (a, b)| {
        out.push(row(vec![V::S(t), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), avg(a[4], a[0]), V::S(name), V::I(b[0]), V::I(b[1]), avg(b[3], b[2]), avg(b[4], b[0])]))
    });
    rows(out)
}

// WITH UserPostStatistics AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//            SUM(p.Score) AS TotalScore, AVG(u.Reputation) AS AverageReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostTypeBreakdown AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViewCount,
//            AVG(p.Score) AS AverageScore
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Name)
// SELECT ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalScore,
//        ups.AverageReputation, ptb.PostType, ptb.PostCount, ptb.TotalViewCount, ptb.AverageScore
// FROM UserPostStatistics ups JOIN PostTypeBreakdown ptb ON ups.TotalPosts > 0
// ORDER BY ups.TotalPosts DESC, ptb.PostCount DESC;
fn q11182(db: &'static So) -> String {
    let ups = user_posts(db);
    let ptb = type_left_posts_by_name(db);
    let mut out = Vec::new();
    (&ups).filt(|a| a[1] > 0).cross(&ptb).drive(|(u, t), (a, b)| {
        out.push(row(vec![
            user_col(db, u, "name"),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            nullable(a[4], a[1]),
            avg(a[9], a[0]),
            V::S(t),
            V::I(b[0]),
            nullable(b[3], b[2]),
            avg(b[1], b[0]),
        ]))
    });
    rows(out)
}

pub static ENTRIES: &[harness::Entry] = &[
    ("12687", q12687),
    ("12537", q12537),
    ("14231", q14231),
    ("14922", q14922),
    ("11688", q11688),
    ("11364", q11364),
    ("12583", q12583),
    ("12246", q12246),
    ("13570", q13570),
    ("10584", q10584),
    ("14612", q14612),
    ("11134", q11134),
    ("14810", q14810),
    ("13038", q13038),
    ("11544", q11544),
    ("11510", q11510),
    ("11488", q11488),
    ("13344", q13344),
    ("14547", q14547),
    ("11388", q11388),
    ("10879", q10879),
    ("13670", q13670),
    ("12374", q12374),
    ("13467", q13467),
    ("14934", q14934),
    ("11681", q11681),
    ("13115", q13115),
    ("14777", q14777),
    ("13234", q13234),
    ("12391", q12391),
    ("13800", q13800),
    ("14904", q14904),
    ("11336", q11336),
    ("11182", q11182),
];
