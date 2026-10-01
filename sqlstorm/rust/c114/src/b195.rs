use harness::prelude::*;
use std::cmp::Reverse;

// SELECT u.Id AS UserId, u.DisplayName, COUNT(a.Id) AS TotalAnswers, COUNT(c.Id) AS TotalComments, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
// FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName ORDER BY TotalAnswers DESC, TotalViews DESC;
fn q10328(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ans = Ident::<Post>::new().with(post_type_id.eq(1)).select(children_of(db));
    let g = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(score).and(ans.opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((((v, s), an), c)) => [a[0] + an.is_some() as i64, a[1] + c.is_some() as i64, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4] + 1, a[5] + s],
            None => a,
        });
    rows(drain(&g).into_iter().map(|(u, a)| row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), nullable(a[5], a[4])])))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, pt.Name AS PostType, COUNT(DISTINCT a.Id) AS AnswerCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        p.ViewCount, p.Score, MAX(h.CreationDate) AS LastEditDate
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN PostHistory h ON p.Id = h.PostId JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     GROUP BY p.Id, p.Title, pt.Name, p.ViewCount, p.Score)
// SELECT ps.PostId, ps.Title, ps.PostType, ps.AnswerCount, ps.CommentCount, ps.ViewCount, ps.Score, ps.LastEditDate
// FROM PostStats ps ORDER BY ps.Score DESC, ps.ViewCount DESC;
//
// Every aggregate is DISTINCT or a MAX, so each child is folded on its own rather than through the product.
fn q14506(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, title, origid, .. } = &db.post;
    let ans = Ident::<Post>::new().with(post_type_id.eq(1)).select(children_of(db));
    let g = db
        .post
        .group_by(Ident::<Post>::new())
        .select(ans.opt())
        .fold(0i64, |n, a| n + a.is_some() as i64);
    let cpp = comments_per_post(db);
    let hmax = history_max_date(db);
    let v = drain(db.post.select(Ident::<Post>::new().and(origid).and(title.opt()).and(ptype_name(db)).and(&g).and(&cpp).and(view_count.opt()).and(score).and(&hmax)));
    rows(v.into_iter().map(|(_, ((((((((_, id), t), pt), a), c), w), s), h))| {
        row(vec![V::I(id), ostr(t), V::S(pt), V::I(a), V::I(c), oint(w), V::I(s), if h == i64::MIN { V::Null } else { V::T(h) }])
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, MAX(p.CreationDate) AS LastPostDate
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE CONCAT('%<', t.TagName, '>%') GROUP BY t.TagName),
// UserReputation AS (SELECT u.Id, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// PopularTags AS (SELECT ts.TagName, ts.PostCount, ts.QuestionCount, ts.AnswerCount, ts.LastPostDate, ur.DisplayName AS MostActiveUser,
//        (ur.Upvotes - ur.Downvotes) AS ReputationChange
//     FROM TagStats ts JOIN UserReputation ur ON ur.Upvotes > ur.Downvotes WHERE ts.PostCount > 10 ORDER BY ts.PostCount DESC LIMIT 5)
// SELECT pt.TagName, pt.PostCount, pt.QuestionCount, pt.AnswerCount, pt.LastPostDate, pt.MostActiveUser, pt.ReputationChange
// FROM PopularTags pt WHERE pt.ReputationChange > 5 ORDER BY pt.ReputationChange DESC;
//
// No tag name holds a LIKE wildcard, '<' or '>', so the LIKE is the exploded tag edge.
fn q29016(db: &'static So) -> String {
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&db.post.tags).inv().collect();
    let ts = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).select((&db.post.post_type_id).and(&db.post.creation_date)).opt())
        .fold((0i64, 0i64, 0i64, i64::MIN), |(n, q, a, d), p| match p {
            Some((t, c)) => (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, d.max(c)),
            None => (n, q, a, d),
        });
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt())
        .fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64))
        .filt(|(u, d)| u > d);
    let v = drain((&ts).filt(|x: (i64, i64, i64, i64)| x.0 > 10).cross(&ur));
    let v: Vec<((Str, Id<User>), ((i64, i64, i64, i64), (i64, i64)))> = top_n(v, |x| Reverse(x.1 .0 .0), 5);
    let v = drain(rel(v).filt(|(_, (_, (u, d)))| u - d > 5));
    rows(v.into_iter().map(|(_, ((t, u), ((n, q, a, d), (up, dn))))| {
        row(vec![V::S(t), V::I(n), V::I(q), V::I(a), tmax(d), V::S(db.user.display_name.get(u).unwrap()), V::I(up - dn)])
    }))
}

// WITH PostStats AS (SELECT p.PostTypeId, COUNT(p.Id) AS TotalPosts, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore, AVG(p.Score) AS AvgScore,
//        AVG(p.ViewCount) AS AvgViews, MAX(p.CreationDate) AS LastPostDate FROM Posts p GROUP BY p.PostTypeId),
// UserStats AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostsCreated, SUM(v.BountyAmount) AS TotalBounty, SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// VoteStats AS (SELECT vt.Id AS VoteTypeId, COUNT(v.Id) AS VoteCount FROM VoteTypes vt LEFT JOIN Votes v ON vt.Id = v.VoteTypeId GROUP BY vt.Id)
// SELECT ps.PostTypeId, ps.TotalPosts, ps.TotalViews, ps.TotalScore, ps.AvgScore, ps.AvgViews, ps.LastPostDate, us.UserId, us.PostsCreated, us.TotalBounty,
//        us.TotalVotes, vs.VoteTypeId, vs.VoteCount
// FROM PostStats ps JOIN UserStats us ON us.PostsCreated > 0 JOIN VoteStats vs ON vs.VoteCount > 0
// ORDER BY ps.TotalPosts DESC, us.PostsCreated DESC, vs.VoteCount DESC;
fn q12962(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let ps = db
        .post
        .group_by(&db.post.post_type_id)
        .select(score.and(view_count.opt()).and(creation_date))
        .fold([0i64, 0, 0, 0, i64::MIN], |a, ((s, v), d)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4].max(d)]);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(v) => [a[0] + 1, a[1] + v.flatten().unwrap_or(0), a[2] + v.flatten().is_some() as i64, a[3] + v.is_some() as i64],
            None => a,
        })
        .filt(|a: [i64; 4]| a[0] > 0);
    let of_type: HashIdx<Id<VoteType>, Id<Vote>> = (&db.vote.vote_type).inv().collect();
    let vs = db.vote_type.group_by(&db.vote_type.origid).select((&of_type).opt()).fold(0i64, |n, v| n + v.is_some() as i64).filt(|n| n > 0);
    let mut out = Vec::new();
    (&ps).cross(&us).cross(&vs).drive(|((t, u), vt), ((p, a), n)| {
        out.push(row(vec![
            V::I(t),
            V::I(p[0]),
            nullable(p[3], p[2]),
            V::I(p[1]),
            avg(p[1], p[0]),
            avg(p[3], p[2]),
            tmax(p[4]),
            V::I(db.user.origid.get(u).unwrap()),
            V::I(a[0]),
            nullable(a[1], a[2]),
            V::I(a[3]),
            V::I(vt),
            V::I(n),
        ]))
    });
    rows(out)
}

// WITH PostEngagement AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoterCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 4 THEN 1 ELSE 0 END) AS OffensiveCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON u.Id = c.UserId
//     WHERE u.CreationDate >= '2023-01-01' GROUP BY u.Id, u.DisplayName)
// SELECT pe.PostId, pe.Title, pe.CreationDate, pe.CommentCount, pe.UniqueVoterCount, pe.UpVoteCount, pe.DownVoteCount, pe.OffensiveCount,
//        ue.UserId, ue.DisplayName, ue.PostCount, ue.CommentCount AS UserCommentCount, ue.TotalUpVotes, ue.TotalDownVotes
// FROM PostEngagement pe JOIN UserEngagement ue ON pe.UniqueVoterCount > 0 ORDER BY pe.CreationDate DESC, pe.UpVoteCount DESC;
fn q11814(db: &'static So) -> String {
    let User { creation_date: ucd, up_votes, down_votes, .. } = &db.user;
    let pe = db
        .post
        .with((&db.post.creation_date).ge(date(2023, 1, 1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.user_id).opt().and(&db.vote.vote_type_id)).opt()))
        .buf_fold(|v| {
            let mut u: Vec<i64> = v.iter().filter_map(|x| x.1.and_then(|y| y.0)).collect();
            u.sort();
            u.dedup();
            let t = |k: i64| v.iter().filter(|x| x.1.map(|y| y.1) == Some(k)).count() as i64;
            [v.iter().filter(|x| x.0.is_some()).count() as i64, u.len() as i64, t(2), t(3), t(4)]
        })
        .filt(|a: [i64; 5]| a[1] > 0);
    let ue = db
        .user
        .with(ucd.ge(date(2023, 1, 1)))
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).opt()).and(comments_by(db).opt()))
        .buf_fold(|v| {
            let mut p: Vec<Id<Post>> = v.iter().filter_map(|x| x.0 .1).collect();
            p.sort();
            p.dedup();
            let mut c: Vec<Id<Comment>> = v.iter().filter_map(|x| x.1).collect();
            c.sort();
            c.dedup();
            [p.len() as i64, c.len() as i64, v.iter().map(|x| x.0 .0 .0).sum(), v.iter().map(|x| x.0 .0 .1).sum()]
        });
    let mut out = Vec::new();
    (&pe).cross(&ue).drive(|(p, u), (a, b)| {
        out.push(row(vec![
            V::I(db.post.origid.get(p).unwrap()),
            ostr(db.post.title.get(p)),
            V::T(db.post.creation_date.get(p).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            V::I(a[4]),
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(b[0]),
            V::I(b[1]),
            V::I(b[2]),
            V::I(b[3]),
        ]))
    });
    rows(out)
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2020-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, COUNT(DISTINCT b.Id) AS BadgesEarned, SUM(v.BountyAmount) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     WHERE u.CreationDate >= '2020-01-01' GROUP BY u.Id, u.DisplayName)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.CommentCount, ps.AnswerCount, ps.UpVoteCount, ps.DownVoteCount,
//        us.UserId, us.DisplayName AS UserDisplayName, us.PostsCreated, us.BadgesEarned, us.TotalBountyAmount
// FROM PostStats ps JOIN UserStats us ON ps.PostId = us.UserId ORDER BY ps.Score DESC, ps.ViewCount DESC;
//
// The join on the raw ids is done first, so only the posts whose Id is a new user's Id are aggregated.
fn q13791(db: &'static So) -> String {
    let Post { creation_date, post_type_id, origid, title, score, view_count, .. } = &db.post;
    let uid: HashIdx<i64, Id<User>> = db.user.with((&db.user.creation_date).ge(date(2020, 1, 1))).select(&db.user.origid).inv().collect();
    let tp: MatSet<Id<Post>> = db.post.with(creation_date.ge(date(2020, 1, 1))).with(origid.select(&uid)).collect();
    let ans = Ident::<Post>::new().with(post_type_id.eq(1)).select(children_of(db));
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(ans.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .buf_fold(|v| {
            let mut c: Vec<Id<Comment>> = v.iter().filter_map(|x| x.0 .0).collect();
            c.sort();
            c.dedup();
            let mut a: Vec<Id<Post>> = v.iter().filter_map(|x| x.0 .1).collect();
            a.sort();
            a.dedup();
            [c.len() as i64, a.len() as i64, v.iter().filter(|x| x.1 == Some(2)).count() as i64, v.iter().filter(|x| x.1 == Some(3)).count() as i64]
        });
    let tu: MatSet<Id<User>> = (&tp).select(origid).select(&uid).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .buf_fold(|v| {
            let mut p: Vec<Id<Post>> = v.iter().filter_map(|x| x.0 .0).collect();
            p.sort();
            p.dedup();
            let mut b: Vec<Id<Badge>> = v.iter().filter_map(|x| x.0 .1).collect();
            b.sort();
            b.dedup();
            let bs: Vec<i64> = v.iter().filter_map(|x| x.1.flatten()).collect();
            [p.len() as i64, b.len() as i64, bs.iter().sum(), bs.len() as i64]
        });
    let v = drain(db.post.select(Ident::<Post>::new().and(&ps).and(origid.select(&uid).select(Ident::<User>::new().and(&us)))));
    rows(v.into_iter().map(|(_, ((p, a), (u, b)))| {
        row(vec![
            V::I(origid.get(p).unwrap()),
            ostr(title.get(p)),
            V::T(creation_date.get(p).unwrap()),
            V::I(score.get(p).unwrap()),
            oint(view_count.get(p)),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(b[0]),
            V::I(b[1]),
            nullable(b[2], b[3]),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p INNER JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1 AND p.Score > 5),
// TopUsers AS (SELECT R.OwnerDisplayName, COUNT(R.PostId) AS TotalPosts, SUM(R.ViewCount) AS TotalViews, SUM(R.Score) AS TotalScore
//     FROM RankedPosts R WHERE R.PostRank <= 5 GROUP BY R.OwnerDisplayName),
// TopTags AS (SELECT tag.TagName, COUNT(p.Id) AS TagUsage FROM Tags tag INNER JOIN Posts p ON p.Tags LIKE CONCAT('%<', tag.TagName, '>%')
//     GROUP BY tag.TagName ORDER BY TagUsage DESC LIMIT 10)
// SELECT U.DisplayName AS TopUser, U.Reputation, T.TotalPosts, T.TotalViews, T.TotalScore, Tag.TagName AS FrequentTag, Tag.TagUsage
// FROM TopUsers T INNER JOIN Users U ON T.OwnerDisplayName = U.DisplayName CROSS JOIN TopTags Tag
// ORDER BY T.TotalPosts DESC, T.TotalViews DESC;
//
// No tag name holds a LIKE wildcard, '<' or '>', so the LIKE is the exploded tag edge.
fn q6872(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, view_count, .. } = &db.post;
    let rp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1))
        .with(score.gt(5))
        .with(owner_user)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, r)| r <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let tu = (&rp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(view_count.opt().and(score))
        .fold([0i64; 4], |a, (v, s)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s]);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&db.post.tags).inv().collect();
    let tt = db.tag.group_by(&db.tag.tag_name).select(&by_tag).fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&tt), |&(_, n)| Reverse(n), 10);
    let tt = rel(tt);
    let v = drain((&tu).and(&by_name).cross(&tt));
    rows(v.into_iter().map(|(_, ((a, u), (t, n)))| {
        row(vec![V::S(db.user.display_name.get(u).unwrap()), V::I(db.user.reputation.get(u).unwrap()), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::S(t), V::I(n)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, COUNT(a.Id) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        RANK() OVER (ORDER BY COUNT(a.Id) DESC) AS Rank, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS NewestPosts
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate)
// SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.AnswerCount, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.Rank <= 10 THEN 'Top 10 Questions' ELSE 'Other Questions' END AS Category, rp.NewestPosts
// FROM RankedPosts rp WHERE rp.NewestPosts <= 50 ORDER BY rp.Rank ASC, rp.PostId DESC;
fn q25550(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, title, body, tags_str, .. } = &db.post;
    let g = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = ranked(drain(&g), |&(_, a)| Reverse(a[0]), false);
    type X = ((Id<Post>, [i64; 3]), i64);
    let v = top_n(drain(rel(v).select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0 .0).select(creation_date)))), |&(_, (_, d))| Reverse(d), 50);
    rows(v.into_iter().enumerate().map(|(i, (_, (((p, a), r), _)))| {
        let cat = if r <= 10 { "Top 10 Questions" } else { "Other Questions" };
        row(vec![V::I(origid.get(p).unwrap()), ostr(title.get(p)), V::S(body.get(p).unwrap()), ostr(tags_str.get(p)), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(cat), V::I(i as i64 + 1)])
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TagStats AS (SELECT t.Id AS TagId, t.TagName, COUNT(p.Id) AS PostsWithTag, SUM(p.ViewCount) AS TotalViews, SUM(p.AnswerCount) AS TotalAnswers
//     FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%<', t.TagName, '>%') GROUP BY t.Id, t.TagName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalViews, TotalScore, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank
//     FROM UserPostStats WHERE TotalPosts > 0),
// TopTags AS (SELECT TagId, TagName, PostsWithTag, TotalViews, TotalAnswers, ROW_NUMBER() OVER (ORDER BY TotalViews DESC) AS Rank FROM TagStats WHERE PostsWithTag > 0)
// SELECT tu.Rank AS UserRank, tu.DisplayName AS TopUser, tu.TotalPosts, tu.Questions, tu.Answers, tu.TotalViews AS UserTotalViews, tu.TotalScore AS UserTotalScore,
//        tt.Rank AS TagRank, tt.TagName AS TopTag, tt.PostsWithTag, tt.TotalViews AS TagTotalViews, tt.TotalAnswers AS TagTotalAnswers
// FROM TopUsers tu JOIN TopTags tt ON tu.TotalViews > 1000 AND tt.TotalViews > 1000
// WHERE tu.Rank <= 10 AND tt.Rank <= 10 ORDER BY tu.Rank, tt.Rank;
//
// No tag name holds a LIKE wildcard, '<' or '>', so the LIKE is the exploded tag edge.
fn q26829(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, answer_count, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)))
        .fold([0i64; 6], |a, ((t, v), s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + v.is_some() as i64, a[4] + v.unwrap_or(0), a[5] + s]);
    let tu = top_n(drain(&ups), |&(_, a)| Reverse(a[5]), 10);
    let tu: Vec<(i64, (Id<User>, [i64; 6]))> = tu.into_iter().enumerate().map(|(i, x)| (i as i64 + 1, x)).collect();
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&db.post.tags).inv().collect();
    let ts = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).select(view_count.opt().and(answer_count.opt())))
        .fold([0i64; 5], |a, (v, n)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + n.is_some() as i64, a[4] + n.unwrap_or(0)]);
    let tt = top_n(drain(&ts), |&(_, a)| (a[1] > 0, Reverse(a[2])), 10);
    let tt: Vec<(i64, (Id<Tag>, [i64; 5]))> = tt.into_iter().enumerate().map(|(i, x)| (i as i64 + 1, x)).collect();
    let v = drain(rel(tu).filt(|(_, (_, a))| a[3] > 0 && a[4] > 1000).cross(rel(tt).filt(|(_, (_, a))| a[1] > 0 && a[2] > 1000)));
    rows(v.into_iter().map(|(_, ((ur, (u, a)), (tr, (t, b))))| {
        row(vec![
            V::I(ur),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            nullable(a[4], a[3]),
            V::I(a[5]),
            V::I(tr),
            V::S(db.tag.tag_name.get(t).unwrap()),
            V::I(b[0]),
            nullable(b[2], b[1]),
            nullable(b[4], b[3]),
        ])
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT A.Id) AS TotalAnswers,
//        SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN Posts A ON A.ParentId = P.Id AND P.PostTypeId = 1 AND A.PostTypeId = 2
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopQuestions AS (SELECT P.OwnerUserId, P.Title, P.CreationDate, P.Score, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS Rank
//     FROM Posts P WHERE P.PostTypeId = 1)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.TotalPosts, U.TotalAnswers, U.TotalViews, TQ.Title AS TopQuestion, TQ.Score
// FROM UserStats U LEFT JOIN TopQuestions TQ ON U.UserId = TQ.OwnerUserId AND TQ.Rank = 1
// ORDER BY U.Reputation DESC LIMIT 10;
//
// The LIMIT orders by reputation alone, so only the users at or above the tenth reputation are aggregated.
fn q12487(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, title, .. } = &db.post;
    let rep = &db.user.reputation;
    let top = top_n(drain(db.user.select(rep)), |&(_, r)| Reverse(r), 10);
    let cut = top.last().map_or(i64::MAX, |x| x.1);
    let tu: MatSet<Id<User>> = db.user.with(rep.ge(cut)).collect();
    let ans = Ident::<Post>::new().with(post_type_id.eq(1)).select(answers_of(db));
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(view_count.opt()).and(ans.opt())).opt())
        .buf_fold(|v| {
            let mut p: Vec<Id<Post>> = v.iter().filter_map(|x| x.map(|y| y.0 .0)).collect();
            p.sort();
            p.dedup();
            let mut a: Vec<Id<Post>> = v.iter().filter_map(|x| x.and_then(|y| y.1)).collect();
            a.sort();
            a.dedup();
            let w: Vec<i64> = v.iter().filter_map(|x| x.and_then(|y| y.0 .1)).collect();
            [p.len() as i64, a.len() as i64, w.iter().sum(), w.len() as i64]
        });
    let tq: MatSet<Id<Post>> = (&tu)
        .select(posts_of(db))
        .with(post_type_id.eq(1))
        .group_by(&db.post.owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| s, desc)
        .filt(|(_, r)| r == 1)
        .map(|((p, _), _)| p)
        .collect();
    let tq_of: HashIdx<Id<User>, Id<Post>> = (&tq).select(&db.post.owner_user).inv().collect();
    let v = drain((&tu).select(Ident::<User>::new().and(rep).and(&us).and((&tq_of).opt())));
    let v = top_n(v, |&(_, (((_, r), _), _))| Reverse(r), 10);
    rows(v.into_iter().map(|(_, (((u, r), a), q))| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(r),
            V::I(a[0]),
            V::I(a[1]),
            nullable(a[2], a[3]),
            q.map_or(V::Null, |q| ostr(title.get(q))),
            q.map_or(V::Null, |q| V::I(score.get(q).unwrap())),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.LastActivityDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.OwnerUserId IS NOT NULL),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%')
//     GROUP BY t.TagName HAVING COUNT(p.Id) > 100),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u JOIN Votes v ON v.UserId = u.Id WHERE u.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years'
//     GROUP BY u.Id, u.DisplayName)
// SELECT rp.Title AS PopularPostTitle, rp.Score AS PopularPostScore, rp.ViewCount AS PopularPostViews, pt.TagName AS PopularTag,
//        au.DisplayName AS ActiveUser, au.UpVotes AS UserUpVotes, au.DownVotes AS UserDownVotes
// FROM RankedPosts rp JOIN PopularTags pt ON pt.PostCount > 50 JOIN ActiveUsers au ON au.UpVotes > 10
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, au.UpVotes DESC;
fn q9095(db: &'static So) -> String {
    let Post { last_activity_date, owner_user_id, score, view_count, title, .. } = &db.post;
    let rp = db
        .post
        .with(last_activity_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user_id)
        .group_by(&db.post.post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(_, s)| s, desc)
        .filt(|(_, r)| r <= 5)
        .map(|((p, _), _)| p);
    let lt = tag_mentions(db);
    let pt = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|x: (Id<Post>, Id<Tag>)| x.1).select(&db.tag.tag_name)).fold(0i64, |n, _| n + 1).filt(|n| n > 100).filt(|n| n > 50);
    let au = db
        .user
        .with((&db.user.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -2)))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id))
        .fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64))
        .filt(|(u, _)| u > 10);
    let v = drain(rp.cross(&pt).cross(&au));
    rows(v.into_iter().map(|(((_, t), u), ((p, _), (up, dn)))| {
        row(vec![ostr(title.get(p)), V::I(score.get(p).unwrap()), oint(view_count.get(p)), V::S(t), V::S(db.user.display_name.get(u).unwrap()), V::I(up), V::I(dn)])
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT A.Id) AS TotalAnswers,
//        COUNT(DISTINCT C.Id) AS TotalComments, SUM(V.BountyAmount) AS TotalBounties
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Posts A ON P.Id = A.ParentId AND P.PostTypeId = 1
//     LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.FavoriteCount, PT.Name AS PostType
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id),
// TopUsers AS (SELECT U.UserId, U.Reputation, U.TotalPosts, U.TotalAnswers, U.TotalComments, U.TotalBounties, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM UserStats U),
// TopPosts AS (SELECT P.PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank FROM PostStats P)
// SELECT TU.ReputationRank, TU.UserId, TU.Reputation, TU.TotalPosts, TU.TotalAnswers, TU.TotalComments, TU.TotalBounties,
//        TP.ScoreRank, TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.ViewCount
// FROM TopUsers TU JOIN TopPosts TP ON TU.TotalPosts > 0 WHERE TU.ReputationRank <= 10 AND TP.ScoreRank <= 10
// ORDER BY TU.ReputationRank, TP.ScoreRank;
//
// ReputationRank reads only a base column, so the users are ranked first and only those with rank <= 10 are aggregated.
fn q10331(db: &'static So) -> String {
    let Post { post_type_id, score, title, creation_date, view_count, origid, .. } = &db.post;
    let ru = ranked(drain(db.user.select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let ru = rel(drain(rel(ru).filt(|(_, k)| k <= 10)).into_iter().map(|x| x.1).collect());
    let tu: HashIdx<Id<User>, i64> = (&ru).map(|((u, _), _)| u).inv().select((&ru).map(|(_, k)| k)).collect();
    let tus: MatSet<Id<User>> = (&ru).map(|((u, _), _)| u).collect();
    let ans = Ident::<Post>::new().with(post_type_id.eq(1)).select(children_of(db));
    let us = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(ans.opt()).and(comments_of(db).opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .buf_fold(|v| {
            let rs: Vec<_> = v.iter().flatten().copied().collect();
            let mut p: Vec<Id<Post>> = rs.iter().map(|x| x.0 .0 .0).collect();
            p.sort();
            p.dedup();
            let mut a: Vec<Id<Post>> = rs.iter().filter_map(|x| x.0 .0 .1).collect();
            a.sort();
            a.dedup();
            let mut c: Vec<Id<Comment>> = rs.iter().filter_map(|x| x.0 .1).collect();
            c.sort();
            c.dedup();
            let b: Vec<i64> = rs.iter().filter_map(|x| x.1.flatten()).collect();
            [p.len() as i64, a.len() as i64, c.len() as i64, b.iter().sum(), b.len() as i64]
        })
        .filt(|a: [i64; 5]| a[0] > 0);
    let rp = ranked(drain(db.post.select(score)), |&(_, s)| Reverse(s), false);
    let rp = rel(rp).filt(|(_, k)| k <= 10);
    let v = drain((&tu).and(&us).cross(rp));
    rows(v.into_iter().map(|((u, _), ((k, a), ((p, s), r)))| {
        row(vec![
            V::I(k),
            V::I(db.user.origid.get(u).unwrap()),
            V::I(db.user.reputation.get(u).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            nullable(a[3], a[4]),
            V::I(r),
            V::I(origid.get(p).unwrap()),
            ostr(title.get(p)),
            V::T(creation_date.get(p).unwrap()),
            V::I(s),
            oint(view_count.get(p)),
        ])
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT P.Id) AS PostCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, PostCount, ROW_NUMBER() OVER (ORDER BY UpVotes - DownVotes DESC, UserId) AS UserRank
//     FROM UserVoteStats),
// PopularTags AS (SELECT Tags.TagName, COUNT(P.Id) AS Popularity FROM Tags JOIN Posts P ON P.Tags LIKE CONCAT('%', Tags.TagName, '%')
//     GROUP BY Tags.TagName ORDER BY Popularity DESC LIMIT 5)
// SELECT T.TagName, U.DisplayName, U.UpVotes, U.DownVotes, U.PostCount, (U.UpVotes - U.DownVotes) AS VoteBalance
// FROM TopUsers U JOIN PopularTags T ON U.PostCount > 5 WHERE U.UserRank <= 10 ORDER BY T.Popularity DESC, VoteBalance DESC;
//
// rewrites/8893.sql: the original orders the ROW_NUMBER by the vote balance alone, which is 0 for every user.
fn q8893(db: &'static So) -> String {
    let uv = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).opt())).opt())
        .buf_fold(|v| {
            let mut p: Vec<Id<Post>> = v.iter().filter_map(|x| x.and_then(|y| y.1)).collect();
            p.sort();
            p.dedup();
            let t = |k: i64| v.iter().filter(|x| x.map(|y| y.0) == Some(k)).count() as i64;
            [t(2), t(3), p.len() as i64]
        });
    let tu = top_n(drain((&uv).and(&db.user.origid)), |&(_, (a, id))| (Reverse(a[0] - a[1]), id), 10);
    let lt = tag_mentions(db);
    let pt = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|x: (Id<Post>, Id<Tag>)| x.1).select(&db.tag.tag_name)).fold(0i64, |n, _| n + 1);
    let pt = top_n(drain(&pt), |&(_, n)| Reverse(n), 5);
    let v = drain(rel(tu).filt(|(_, (a, _))| a[2] > 5).cross(rel(pt)));
    rows(v.into_iter().map(|(_, ((u, (a, _)), (t, _)))| {
        row(vec![V::S(t), V::S(db.user.display_name.get(u).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[0] - a[1])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, COALESCE(NULLIF(p.AcceptedAnswerId, -1), 0) AS ApprovedAnswer,
//        COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Posts a ON a.ParentId = p.Id AND p.PostTypeId = 1 LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, p.AcceptedAnswerId),
// TaggedPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.CreationDate, rp.ViewCount, rp.ApprovedAnswer, rp.AnswerCount, rp.UpVotes, rp.DownVotes,
//        SPLIT_PART(rp.Tags, '>', 2) AS PrimaryTag FROM RankedPosts rp)
// SELECT tp.PrimaryTag, COUNT(tp.PostId) AS TagPostCount, AVG(tp.ViewCount) AS AvgViewCount, AVG(tp.AnswerCount) AS AvgAnswerCount,
//        SUM(tp.UpVotes) AS TotalUpVotes, SUM(tp.DownVotes) AS TotalDownVotes, MAX(tp.CreationDate) AS LastPostDate
// FROM TaggedPosts tp WHERE tp.ViewCount > 100 GROUP BY tp.PrimaryTag ORDER BY TagPostCount DESC;
fn q29010(db: &'static So) -> String {
    let Post { post_type_id, view_count, tags_str, creation_date, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1)).with(view_count.gt(100));
    let rp = base
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let primary = tags_str.opt().map(|t: Option<Str>| t.map(|t| t.split('>').nth(1).unwrap_or("")));
    let g = db
        .post
        .with(&rp)
        .group_by(primary)
        .select(view_count.and(&rp).and(creation_date))
        .fold([0i64, 0, 0, 0, 0, i64::MIN], |a, ((v, r), d)| [a[0] + 1, a[1] + v, a[2] + r[0], a[3] + r[1], a[4] + r[2], a[5].max(d)]);
    rows(drain(&g).into_iter().map(|(k, a)| {
        row(vec![ostr(k), V::I(a[0]), avg(a[1], a[0]), avg(a[2], a[0]), V::F(a[3] as f64), V::F(a[4] as f64), tmax(a[5])])
    }))
}

// WITH PostScore AS (SELECT p.Id AS PostId, p.Score, p.AnswerCount, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS TagCount FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%')
//     JOIN PostScore ps ON ps.PostId = p.Id GROUP BY t.TagName HAVING COUNT(p.Id) > 10),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.DisplayName HAVING SUM(COALESCE(v.BountyAmount, 0)) > 100)
// SELECT p.Title, p.Score, pt.TagName, u.DisplayName AS TopUser, ps.AnswerCount, ps.ViewCount, COALESCE(cp.CloseCount, 0) AS CloseCount
// FROM Posts p LEFT JOIN PostScore ps ON p.Id = ps.PostId JOIN PopularTags pt ON p.Tags LIKE CONCAT('%', pt.TagName, '%')
// LEFT JOIN ClosedPosts cp ON p.Id = cp.PostId JOIN TopUsers u ON u.TotalBounty = (SELECT MAX(TotalBounty) FROM TopUsers)
// WHERE ps.RankScore <= 10 ORDER BY p.Score DESC, pt.TagName;
//
// The RankScore tie at 10 is among answers, which have no Tags and so never reach the output.
fn q3542(db: &'static So) -> String {
    let Post { creation_date, score, title, answer_count, view_count, .. } = &db.post;
    let recent: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).collect();
    let ps: MatSet<Id<Post>> = (&recent)
        .group_by(&db.post.post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(_, s)| s, desc)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let lt = tag_mentions(db);
    let ment: HashIdx<Id<Post>, Id<Tag>> = (&lt).map(|(p, _)| p).inv().map(|(_, t): (Id<Post>, Id<Tag>)| t).collect();
    let pt = (&recent).select(&ment).group_by(&db.tag.tag_name).fold(0i64, |n, _| n + 1).filt(|n| n > 10);
    let closed = db
        .post_history
        .with((&db.post_history.post_history_type_id).eq(10))
        .select(&db.post_history.post)
        .inv()
        .dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())
        .fold(0i64, |s, b| s + b.flatten().unwrap_or(0))
        .filt(|s| s > 100);
    let m = (&tu).fold_flat(i64::MIN, |a, s| a.max(s));
    let v = drain((&ps).select(Ident::<Post>::new().and((&ment).select(&db.tag.tag_name).with(&pt)).and(&closed)).cross((&tu).filt(move |s| s == m)));
    rows(v.into_iter().map(|((_, u), (((p, t), c), _))| {
        row(vec![ostr(title.get(p)), V::I(score.get(p).unwrap()), V::S(t), V::S(db.user.display_name.get(u).unwrap()), oint(answer_count.get(p)), oint(view_count.get(p)), V::I(c)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2023-01-01 00:00:00' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// AggregatedData AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.UpVotes > rp.DownVotes THEN 'Positive' WHEN rp.DownVotes > rp.UpVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
//     FROM RankedPosts rp WHERE rp.rn <= 10)
// SELECT ad.PostId, ad.Title, ad.OwnerDisplayName, ad.CommentCount, ad.UpVotes, ad.DownVotes, ad.VoteSentiment, pt.Name AS PostTypeName,
//        COALESCE(cf.Count, 0) AS TagUsageCount
// FROM AggregatedData ad JOIN PostTypes pt ON ad.PostId = pt.Id
// LEFT JOIN (SELECT p.Id AS PostId, COUNT(t.Id) AS Count FROM Posts p JOIN Tags t ON p.Tags LIKE CONCAT('%', t.TagName, '%') GROUP BY p.Id) cf ON ad.PostId = cf.PostId
// ORDER BY ad.CommentCount DESC, ad.UpVotes DESC;
//
// rn reads only base columns, so the posts are ranked first and only the newest ten per type are aggregated.
fn q8326(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, origid, title, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(date(2023, 1, 1)))
        .with(post_type_id.is_in([1, 2]))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let g = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let types: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let lt = tag_mentions(db);
    let cf: HashIdx<Id<Post>, i64> = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|x: (Id<Post>, Id<Tag>)| x.0)).fold(0i64, |n, _| n + 1).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and(&g).and(origid.select(&types).select(&db.post_type.name)).and((&cf).opt())));
    rows(v.into_iter().map(|(_, (((p, a), n), c))| {
        let s = if a[1] > a[2] { "Positive" } else if a[2] > a[1] { "Negative" } else { "Neutral" };
        row(vec![
            V::I(origid.get(p).unwrap()),
            ostr(title.get(p)),
            V::S(owner_user.select(&db.user.display_name).get(p).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::S(s),
            V::S(n),
            V::I(c.unwrap_or(0)),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.Tags, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.ViewCount DESC) AS TagRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.PostID, rp.Title, rp.Tags, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, rp.CreationDate FROM RankedPosts rp WHERE rp.TagRank <= 5),
// UsersWithBadges AS (SELECT u.Id AS UserID, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostsWithModerationHistory AS (SELECT p.Id AS PostID, p.Title, COALESCE(ph.Comment, '') AS ModerationComment, COUNT(ph.Id) AS HistoryCount
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, ph.Comment),
// FinalResult AS (SELECT fp.PostID, fp.Title, fp.Tags, fp.ViewCount, fp.Score, fp.AnswerCount, fp.CommentCount, ub.DisplayName AS UserDisplayName, ub.BadgeCount,
//        pmh.ModerationComment, pmh.HistoryCount
//     FROM FilteredPosts fp JOIN UsersWithBadges ub ON fp.PostID = (SELECT OwnerUserId FROM Posts WHERE Id = fp.PostID)
//     LEFT JOIN PostsWithModerationHistory pmh ON fp.PostID = pmh.PostID)
// SELECT PostID, Title, Tags, ViewCount, Score, AnswerCount, CommentCount, UserDisplayName, BadgeCount, ModerationComment, HistoryCount
// FROM FinalResult ORDER BY Score DESC, ViewCount DESC;
//
// The ub join condition does not mention ub: it keeps the posts whose Id equals their own OwnerUserId, crossed with every user.
fn q25128(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, view_count, origid, owner_user_id, title, score, answer_count, comment_count, .. } = &db.post;
    let fp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1))
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(_, v)| (v.is_none(), v), desc)
        .filt(|(_, r)| r <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let own: MatSet<Id<Post>> = (&fp).with(origid.and(owner_user_id).filt(|(a, b)| a == b)).collect();
    type H = (Id<Post>, Option<Option<Str>>);
    let pmh = (&own)
        .select(Ident::<Post>::new().and(history_of(db).select((&db.post_history.comment).opt()).opt()))
        .group_by(Same::<H>::new().map(|(p, c): H| (p, c.flatten())))
        .fold(0i64, |n, (_, h): H| n + h.is_some() as i64);
    let r = rel(drain(&pmh));
    let pm: HashIdx<Id<Post>, ((Id<Post>, Option<Str>), i64)> = (&r).map(|((p, _), _)| p).inv().select(&r).collect();
    let bpu = badges_per_user(db);
    let v = drain((&own).select(Ident::<Post>::new().and((&pm).opt())).cross(&bpu));
    rows(v.into_iter().map(|((_, u), ((p, h), b))| {
        row(vec![
            V::I(origid.get(p).unwrap()),
            ostr(title.get(p)),
            ostr(tags_str.get(p)),
            oint(view_count.get(p)),
            V::I(score.get(p).unwrap()),
            oint(answer_count.get(p)),
            V::I(comment_count.get(p).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(b),
            h.map_or(V::Null, |((_, c), _)| V::S(c.unwrap_or(""))),
            h.map_or(V::Null, |(_, n)| V::I(n)),
        ])
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(coalesce(p.ViewCount, 0)) AS TotalViews, SUM(coalesce(p.Score, 0)) AS TotalScore,
//        AVG(p.ViewCount) AS AvgViewsPerPost
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopTags AS (SELECT t.Id AS TagId, t.TagName, SUM(p.ViewCount) AS TotalViews, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.Id, t.TagName ORDER BY TotalViews DESC LIMIT 10),
// UserRankings AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, RANK() OVER (ORDER BY ua.Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY ua.TotalViews DESC) AS ViewsRank, RANK() OVER (ORDER BY ua.PostCount DESC) AS PostsRank FROM UserActivity ua)
// SELECT ur.DisplayName, ur.Reputation, ur.ReputationRank, ur.ViewsRank, ur.PostsRank, tt.TagName, tt.TotalViews AS TagTotalViews, tt.PostCount AS TagPostCount
// FROM UserRankings ur JOIN TopTags tt ON tt.PostCount > 5 ORDER BY ur.Reputation DESC, tt.TotalViews DESC;
fn q29675(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt()).opt())
        .fold((0i64, 0i64), |(n, w), p| (n + p.is_some() as i64, w + p.flatten().unwrap_or(0)));
    let v = drain((&ua).and(&db.user.reputation));
    let v = ranked(v, |&(_, (_, r))| Reverse(r), false);
    let v = ranked(v, |&((_, ((_, w), _)), _)| Reverse(w), false);
    let v = ranked(v, |&(((_, ((n, _), _)), _), _)| Reverse(n), false);
    let lt = tag_mentions(db);
    let tt = (&lt)
        .group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|x: (Id<Post>, Id<Tag>)| x.1))
        .select(Same::<(Id<Post>, Id<Tag>)>::new().map(|x: (Id<Post>, Id<Tag>)| x.0).select((&db.post.view_count).opt()))
        .fold((0i64, 0i64, 0i64), |(n, k, s), w| (n + 1, k + w.is_some() as i64, s + w.unwrap_or(0)));
    let tt = top_n(drain(&tt), |&(_, (_, k, s))| (k > 0, Reverse(s)), 10);
    let mut out = Vec::new();
    rel(v).cross(rel(tt).filt(|(_, (n, _, _))| n > 5)).drive(|_, (((((u, (_, r)), rr), vr), pr), (t, (n, k, s)))| {
        out.push(row(vec![V::S(db.user.display_name.get(u).unwrap()), V::I(r), V::I(rr), V::I(vr), V::I(pr), V::S(db.tag.tag_name.get(t).unwrap()), nullable(s, k), V::I(n)]))
    });
    rows(out)
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation,
//        CASE WHEN u.Reputation > 1000 THEN 'High Reputation' WHEN u.Reputation BETWEEN 100 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
//     FROM Users u),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END AS AnswerStatus
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AcceptedAnswerId),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v GROUP BY v.PostId),
// PostsWithVotes AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.CommentCount, COALESCE(pvc.UpVotes, 0) AS UpVotes,
//        COALESCE(pvc.DownVotes, 0) AS DownVotes, pd.AnswerStatus FROM PostDetails pd LEFT JOIN PostVoteCounts pvc ON pd.PostId = pvc.PostId),
// RankedPosts AS (SELECT pwv.*, ROW_NUMBER() OVER (PARTITION BY pwv.AnswerStatus ORDER BY pwv.Score DESC, pwv.ViewCount DESC) AS Rank FROM PostsWithVotes pwv)
// SELECT ur.DisplayName, ur.ReputationCategory, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.AnswerStatus,
//        CASE WHEN rp.Rank <= 5 THEN 'Top 5 Posts' ELSE 'Outside Top 5' END AS TopPostIndicator
// FROM RankedPosts rp JOIN UserReputation ur ON ur.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId)
// WHERE ur.Reputation IS NOT NULL AND rp.AnswerStatus = 'Accepted' ORDER BY ur.Reputation DESC, rp.Score DESC;
fn q23503(db: &'static So) -> String {
    let Post { accepted_answer_id, score, view_count, owner_user, title, creation_date, .. } = &db.post;
    let acc = || db.post.with(accepted_answer_id);
    let top = top_n(drain(acc().select(score.and(view_count.opt()))), |&(_, (s, w))| (Reverse(s), w.is_some(), Reverse(w)), 5);
    let top: MatSet<Id<Post>> = rel(top).map(|(p, _)| p).collect();
    let vc = acc()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let cpp = comments_per_post(db);
    let v = drain(acc().select(Ident::<Post>::new().and(owner_user).and(&cpp).and(&vc).and(Ident::<Post>::new().with(&top).opt())));
    rows(v.into_iter().map(|(_, ((((p, u), c), (up, dn)), t))| {
        let r = db.user.reputation.get(u).unwrap();
        let cat = if r > 1000 { "High Reputation" } else if r >= 100 { "Medium Reputation" } else { "Low Reputation" };
        row(vec![
            V::S(db.user.display_name.get(u).unwrap()),
            V::S(cat),
            ostr(title.get(p)),
            V::T(creation_date.get(p).unwrap()),
            V::I(score.get(p).unwrap()),
            oint(view_count.get(p)),
            V::I(c),
            V::I(up),
            V::I(dn),
            V::S("Accepted"),
            V::S(if t.is_some() { "Top 5 Posts" } else { "Outside Top 5" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, p.AcceptedAnswerId, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.Score > 0 AND p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// RecentVotes AS (SELECT v.PostId, v.VoteTypeId, COUNT(*) AS VoteCount FROM Votes v WHERE v.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
//     GROUP BY v.PostId, v.VoteTypeId),
// PostStatistics AS (SELECT rp.PostId, rp.Title, COALESCE(rv.VoteCount, 0) AS RecentVoteCount, rp.ViewCount, rp.Score, rp.Tags
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId WHERE rp.PostRank = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes,
//        ROW_NUMBER() OVER (ORDER BY SUM(u.UpVotes) DESC) AS UserRank FROM Users u GROUP BY u.Id, u.DisplayName)
// SELECT pu.DisplayName, ps.Title, ps.RecentVoteCount, ps.ViewCount, ps.Score, ps.Tags,
//        CASE WHEN ps.RecentVoteCount = 0 THEN 'No recent votes' ELSE 'Votes recorded' END AS VoteStatus,
//        CASE WHEN ps.Score > 100 THEN 'Highly Rated' WHEN ps.Score BETWEEN 50 AND 100 THEN 'Moderately Rated' ELSE 'Low Rated' END AS RatingStatus
// FROM PostStatistics ps JOIN Users pu ON ps.PostId IN (SELECT DISTINCT PostId FROM Posts WHERE OwnerUserId = pu.Id)
// WHERE pu.Id IN (SELECT UserId FROM TopUsers WHERE UserRank <= 10) ORDER BY ps.Score DESC, ps.RecentVoteCount DESC LIMIT 50;
//
// Posts has no PostId column, so the IN subquery's PostId is ps.PostId: the join keeps every user who owns a post, crossed with every row of ps.
fn q451(db: &'static So) -> String {
    let Post { score, creation_date, owner_user_id, title, view_count, tags_str, .. } = &db.post;
    let rp: MatSet<Id<Post>> = db
        .post
        .with(score.gt(0))
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, r)| r == 1)
        .map(|((p, _), _)| p)
        .collect();
    let Vote { creation_date: vcd, post, vote_type_id, .. } = &db.vote;
    let rv = db.vote.with(vcd.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post.and(vote_type_id)).fold(0i64, |n, _| n + 1);
    let r = rel(drain(&rv));
    let rvi: HashIdx<Id<Post>, i64> = (&r).map(|((p, _), _)| p).inv().select((&r).map(|(_, n)| n)).collect();
    let tu = top_n(drain(db.user.select(&db.user.up_votes)), |&(_, u)| Reverse(u), 10);
    let tu: MatSet<Id<User>> = rel(tu).map(|(u, _)| u).collect();
    let v = drain((&rp).select(Ident::<Post>::new().and(score).and((&rvi).opt())).cross((&tu).with(posts_of(db))));
    let v = top_n(v, |&(_, (((_, s), c), _))| (Reverse(s), Reverse(c.unwrap_or(0))), 50);
    rows(v.into_iter().map(|(_, (((p, s), c), u))| {
        let c = c.unwrap_or(0);
        let rs = if s > 100 { "Highly Rated" } else if s >= 50 { "Moderately Rated" } else { "Low Rated" };
        row(vec![
            V::S(db.user.display_name.get(u).unwrap()),
            ostr(title.get(p)),
            V::I(c),
            oint(view_count.get(p)),
            V::I(s),
            ostr(tags_str.get(p)),
            V::S(if c == 0 { "No recent votes" } else { "Votes recorded" }),
            V::S(rs),
        ])
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostsCount, COUNT(DISTINCT C.Id) AS CommentsCount,
//        SUM(COALESCE(V.VoteCount, 0)) AS TotalVotes, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON U.Id = C.UserId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'),
// CloseReasonSummary AS (SELECT PH.PostId, COUNT(*) AS CloseReasonCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId)
// SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.PostsCount, UA.CommentsCount, UA.TotalVotes, UA.UserRank, RP.PostId, RP.Title,
//        RP.CreationDate AS RecentPostDate, RP.Score, RP.ViewCount, CR.CloseReasonCount
// FROM UserActivity UA LEFT JOIN RecentPosts RP ON UA.UserId = RP.OwnerUserId AND RP.RecentPostRank = 1
// LEFT JOIN CloseReasonSummary CR ON RP.PostId = CR.PostId
// WHERE UA.Reputation > 1000 AND (RP.Score IS NOT NULL OR RP.ViewCount > 100) ORDER BY UA.UserRank;
//
// The WHERE needs an RP row (Score is never NULL, so any RP row passes), so only the users with a recent post are aggregated.
fn q4025(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, origid, title, .. } = &db.post;
    let rp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_days(utc_to_ny(now_utc()), -30)))
        .group_by((&db.post.owner_user_id).opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, r)| r == 1)
        .map(|((p, _), _)| p)
        .collect();
    let rp_of: HashIdx<Id<User>, Id<Post>> = (&rp).select(owner_user).inv().collect();
    let ranks = top_n(drain(db.user.select(&db.user.reputation)), |&(_, r)| Reverse(r), 0);
    let ranks: Vec<(Id<User>, i64)> = ranks.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect();
    let r = rel(ranks);
    let rank: HashIdx<Id<User>, i64> = (&r).map(|(u, _)| u).inv().select((&r).map(|(_, k)| k)).collect();
    let tu: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(1000)).with(&rp_of).collect();
    let vpp = votes_per_post(db);
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(&vpp)).opt().and(comments_by(db).opt()))
        .buf_fold(|v| {
            let mut p: Vec<Id<Post>> = v.iter().filter_map(|x| x.0.map(|y| y.0)).collect();
            p.sort();
            p.dedup();
            let mut c: Vec<Id<Comment>> = v.iter().filter_map(|x| x.1).collect();
            c.sort();
            c.dedup();
            [p.len() as i64, c.len() as i64, v.iter().map(|x| x.0.map_or(0, |y| y.1)).sum()]
        });
    let cr = db
        .post_history
        .with((&db.post_history.post_history_type_id).is_in([10, 11]))
        .select(&db.post_history.post)
        .inv()
        .dense_fold(db.post.id.n, 0i64, |a, _| a + 1);
    let v = drain((&tu).select(Ident::<User>::new().and(&ua).and(&rank).and((&rp_of).select(Ident::<Post>::new().and((&cr).opt())))));
    rows(v.into_iter().map(|(_, (((u, a), k), (p, c)))| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(db.user.reputation.get(u).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(k),
            V::I(origid.get(p).unwrap()),
            ostr(title.get(p)),
            V::T(creation_date.get(p).unwrap()),
            V::I(score.get(p).unwrap()),
            oint(view_count.get(p)),
            oint(c),
        ])
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions,
//        COALESCE(SUM(V.BountyAmount), 0) AS TotalBountyEarned, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// QuestionStats AS (SELECT P.Id AS QuestionId, P.Title, COUNT(A.Id) AS AnswerCount, AVG(COALESCE(CAST(A.Score AS FLOAT), 0)) AS AvgAnswerScore,
//        COUNT(DISTINCT C.Id) AS TotalComments, MAX(P.LastActivityDate) AS LastActiveDate,
//        CASE WHEN MAX(P.ClosedDate) IS NOT NULL THEN 'Closed' ELSE 'Open' END AS Status
//     FROM Posts P LEFT JOIN Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2 LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title),
// PostHistoryData AS (SELECT PH.PostId, DENSE_RANK() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS RevisionRank, PH.PostHistoryTypeId,
//        PH.CreationDate, COALESCE(PH.Comment, 'No comment') AS Comment
//     FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (6, 10, 14))
// SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.TotalPosts, UA.TotalQuestions, UA.TotalAnswers, UA.TotalBountyEarned, Q.questionId, Q.Title AS QuestionTitle,
//        Q.AnswerCount, Q.AvgAnswerScore, Q.TotalComments, PH.RevisionRank, PH.PostHistoryTypeId, PH.CreationDate AS HistoryDate, PH.Comment,
//        CASE WHEN (UA.TotalPosts > 100 AND UA.Reputation >= 1000) THEN 'Veteran' ELSE 'Regular' END AS UserType,
//        CASE WHEN Q.Status = 'Closed' THEN 'This question is closed, cannot accept new answers.' ELSE 'Open for answers.' END AS StatusMessage
// FROM UserActivity UA JOIN QuestionStats Q ON UA.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = Q.QuestionId)
// LEFT JOIN PostHistoryData PH ON Q.QuestionId = PH.PostId AND PH.RevisionRank = 1
// WHERE UA.Reputation > 0 AND UA.Reputation IS NOT NULL ORDER BY UA.Reputation DESC, Q.AvgAnswerScore DESC;
fn q24454(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, closed_date, origid, title, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date: hcd, comment, .. } = &db.post_history;
    let qs: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1)).with(owner_user.select((&db.user.reputation).gt(0))).collect();
    let tu: MatSet<Id<User>> = (&qs).select(owner_user).collect();
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(post_type_id).and(votes_of(db).with((&db.vote.vote_type_id).is_in([8, 9])).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .buf_fold(|v| {
            let rs: Vec<_> = v.iter().flatten().copied().collect();
            let mut p: Vec<Id<Post>> = rs.iter().map(|x| x.0 .0).collect();
            p.sort();
            p.dedup();
            [p.len() as i64, rs.iter().filter(|x| x.0 .1 == 2).count() as i64, rs.iter().filter(|x| x.0 .1 == 1).count() as i64, rs.iter().map(|x| x.1.flatten().unwrap_or(0)).sum()]
        });
    let q = (&qs)
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).select(score).opt().and(comments_of(db).opt()))
        .buf_fold(|v| {
            let mut c: Vec<Id<Comment>> = v.iter().filter_map(|x| x.1).collect();
            c.sort();
            c.dedup();
            let s: i64 = v.iter().map(|x| x.0.unwrap_or(0)).sum();
            (v.iter().filter(|x| x.0.is_some()).count() as i64, s as f64 / v.len() as f64, c.len() as i64)
        });
    let ph: HashIdx<Id<Post>, Id<PostHistory>> = db
        .post_history
        .with(post_history_type_id.is_in([6, 10, 14]))
        .group_by(&db.post_history.post)
        .select(Ident::<PostHistory>::new().and(hcd))
        .window(dense_rank, |(_, d)| d, desc)
        .filt(|(_, r)| r == 1)
        .map(|((h, _), _)| h)
        .collect();
    let mut out = Vec::new();
    (&qs).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ua))).and(&q).and((&ph).opt())).drive(|_, (((p, (u, a)), (n, avg_s, c)), h)| {
        let rep = db.user.reputation.get(u).unwrap();
        out.push(row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(rep),
            V::I(a[0]),
            V::I(a[2]),
            V::I(a[1]),
            V::I(a[3]),
            V::I(origid.get(p).unwrap()),
            ostr(title.get(p)),
            V::I(n),
            V::F(avg_s),
            V::I(c),
            h.map_or(V::Null, |_| V::I(1)),
            oint(h.map(|h| post_history_type_id.get(h).unwrap())),
            ots(h.map(|h| hcd.get(h).unwrap())),
            h.map_or(V::Null, |h| V::S(comment.get(h).unwrap_or("No comment"))),
            V::S(if a[0] > 100 && rep >= 1000 { "Veteran" } else { "Regular" }),
            V::S(if closed_date.get(p).is_some() { "This question is closed, cannot accept new answers." } else { "Open for answers." }),
        ]))
    });
    rows(out)
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT b.Id) AS BadgesCount, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN ph.PostId END) AS ClosedPostsCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY u.Id, u.DisplayName),
// UserRanked AS (SELECT ua.*, RANK() OVER (ORDER BY (Upvotes - Downvotes) DESC, PostCount DESC) AS ActivityRank FROM UserActivity ua),
// FilteredUsers AS (SELECT r.UserId, r.DisplayName, r.Upvotes, r.Downvotes, r.PostCount, r.CommentCount, r.BadgesCount, r.ClosedPostsCount, r.ActivityRank
//     FROM UserRanked r WHERE r.PostCount > 5 AND r.BadgesCount > 2),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS TotalComments, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes,
//        ROW_NUMBER() OVER (ORDER BY (COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0)) DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY p.Id, p.Title HAVING COUNT(c.Id) > 10)
// SELECT fu.DisplayName, fu.Upvotes, fu.Downvotes, fu.PostCount, fu.CommentCount, p.Title AS PopularPostTitle, p.TotalComments, p.TotalUpvotes, p.TotalDownvotes, fu.ActivityRank
// FROM FilteredUsers fu JOIN TopPosts p ON fu.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = p.PostId)
// WHERE fu.ActivityRank <= 10 AND (fu.ClosedPostsCount IS NULL OR fu.ClosedPostsCount = 0) ORDER BY fu.ActivityRank;
//
// The vote sums are folded over the whole product; the DISTINCT counts each from their own child.
fn q24043(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, .. } = &db.post;
    let vt = votes_by(db).select(&db.vote.vote_type_id).opt();
    let rest = posts_of(db).select(comments_of(db).opt().and(history_of(db).opt())).opt().and(badges_of(db).opt());
    let ud = db.user.group_by(Ident::<User>::new()).select(vt.and(rest)).fold((0i64, 0i64), |(u, d), (t, _)| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = badges_per_user(db);
    let closed = Ident::<Post>::new().with(history_of(db).select(&db.post_history.post_history_type_id).filt(|t| t == 10 || t == 11));
    let xc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(closed).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain(db.user.select((&ud).and(&pc).and(&cc).and(&bc).and(&xc)));
    let v = ranked(v, |&(_, ((((ud, p), _), _), _))| (Reverse(ud.0 - ud.1), Reverse(p)), false);
    let fu = drain(rel(v).filt(|((_, ((((_, p), _), b), _)), r)| p > 5 && b > 2 && r <= 10).filt(|((_, (_, x)), _)| x == 0));
    let fr = rel(fu.into_iter().map(|x| x.1).collect());
    let fi: HashIdx<Id<User>, _> = (&fr).map(|((u, _), _)| u).inv().select(&fr).collect();
    let tp = db
        .post
        .with(creation_date.gt(add_days(utc_to_ny(now_utc()), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64])
        .filt(|a: [i64; 3]| a[0] > 10);
    let v = drain(db.post.select(Ident::<Post>::new().and(&tp).and(owner_user.select(&fi))));
    rows(v.into_iter().map(|(_, ((p, a), ((u, ((((ud, n), c), _), _)), r)))| {
        row(vec![V::S(db.user.display_name.get(u).unwrap()), V::I(ud.0), V::I(ud.1), V::I(n), V::I(c), ostr(title.get(p)), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS Author, ARRAY_AGG(DISTINCT t.TagName) AS Tags
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS t(TagName) ON TRUE
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName ORDER BY p.CreationDate DESC LIMIT 10),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.Author, rp.CreationDate, rp.ViewCount, rp.Score, ARRAY_LENGTH(rp.Tags, 1) AS TagCount,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 2) AS UpVoteCount
//     FROM RankedPosts rp)
// SELECT ps.PostId, ps.Title, ps.Author, ps.CreationDate, ps.ViewCount, ps.Score, ps.TagCount, ps.CommentCount, ps.UpVoteCount,
//        ps.Score * 1.0 / NULLIF(ps.TagCount, 0) AS ScorePerTag, ps.Score * 1.0 / NULLIF(ps.CommentCount, 0) AS ScorePerComment,
//        ps.ViewCount * 1.0 / NULLIF(ps.CommentCount, 0) AS ViewPerComment
// FROM PostStats ps WHERE ps.Score > 0 ORDER BY ScorePerTag DESC, ScorePerComment DESC;
//
// The LIMIT reads only base columns, so the ten newest questions are picked first. A post with no Tags still aggregates one NULL, so its TagCount is 1.
fn q28029(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, origid, title, view_count, score, tags_str, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(creation_date)), |&(_, d)| Reverse(d), 10);
    let tp: MatSet<Id<Post>> = rel(top).map(|(p, _)| p).collect();
    let tc = (&tp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t[1..t.len() - 1].split("><")).opt()).buf_fold(|v| {
        let mut t: Vec<Option<Str>> = v.to_vec();
        t.sort();
        t.dedup();
        t.len() as i64
    });
    let cpp = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let v = drain((&tp).with(score.gt(0)).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name)).and(&tc).and(&cpp).and(&up)));
    let div = |a: Option<i64>, b: i64| if b == 0 { V::Null } else { ofloat(a.map(|a| a as f64 / b as f64)) };
    rows(v.into_iter().map(|(_, ((((p, a), t), c), u))| {
        let s = score.get(p).unwrap();
        let w = view_count.get(p);
        row(vec![
            V::I(origid.get(p).unwrap()),
            ostr(title.get(p)),
            V::S(a),
            V::T(creation_date.get(p).unwrap()),
            oint(w),
            V::I(s),
            V::I(t),
            V::I(c),
            V::I(u),
            div(Some(s), t),
            div(Some(s), c),
            div(w, c),
        ])
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.PostTypeId = 1 AND P.Score > 0),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostVoteHistory AS (SELECT V.PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN V.VoteTypeId = 6 THEN 1 END) AS CloseVotes, COUNT(CASE WHEN V.VoteTypeId = 7 THEN 1 END) AS ReopenVotes, COUNT(*) AS TotalVotes
//     FROM Votes V GROUP BY V.PostId)
// SELECT U.DisplayName AS UserName, U.Reputation, RB.BadgeCount, RB.BadgeNames, RP.PostId, RP.Title AS QuestionTitle, RP.CreationDate AS QuestionDate,
//        PH.PostHistoryTypeId, PH.CreationDate AS HistoryDate, COALESCE(PVH.UpVotes, 0) AS UpVotes, COALESCE(PVH.DownVotes, 0) AS DownVotes,
//        COALESCE(PVH.CloseVotes, 0) AS CloseVotes, COALESCE(PVH.ReopenVotes, 0) AS ReopenVotes, PH.Comment
// FROM RankedPosts RP JOIN Users U ON RP.PostRank = 1 AND RP.PostId IN (SELECT P.Id FROM Posts P WHERE P.OwnerUserId = U.Id)
// LEFT JOIN UserBadges RB ON U.Id = RB.UserId LEFT JOIN PostHistory PH ON RP.PostId = PH.PostId LEFT JOIN PostVoteHistory PVH ON RP.PostId = PVH.PostId
// WHERE PH.CreationDate < RP.CreationDate AND PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY U.Reputation DESC, RP.Score DESC LIMIT 100;
//
// No history row predates its post, so the answer is empty; the unordered STRING_AGG is built in badge id order.
fn q23269(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, origid, title, .. } = &db.post;
    let PostHistory { creation_date: hcd, post_history_type_id, comment, .. } = &db.post_history;
    let rp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1))
        .with(score.gt(0))
        .group_by((&db.post.owner_user_id).opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, r)| r == 1)
        .map(|((p, _), _)| p)
        .collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(Ident::<Badge>::new().and(&db.badge.name)).opt()).buf_fold(|v| {
        let mut b: Vec<(Id<Badge>, Str)> = v.iter().flatten().copied().collect();
        b.sort();
        let names: Vec<Str> = b.iter().map(|x| x.1).collect();
        (b.len() as i64, if b.is_empty() { None } else { Some(leak(names.join(", "))) })
    });
    let pvh = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 4], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 6) as i64, a[3] + (t == 7) as i64]);
    let lo = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let early = history_of(db).select(Ident::<PostHistory>::new().and(hcd)).filt(move |(_, d)| d >= lo);
    let v = drain(
        (&rp)
            .select(Ident::<Post>::new().and(creation_date).and(score).and(owner_user.select(Ident::<User>::new().and(&db.user.reputation).and(&ub))).and(early).and(origid.select(&pvh).opt()))
            .filt(|(((((_, d), _), _), (_, hd)), _)| hd < d),
    );
    let v = top_n(v, |&(_, (((((_, _), s), (_, r)), _), _))| (Reverse(r), Reverse(s)), 100);
    rows(v.into_iter().map(|(_, (((((p, d), _), ((u, r), (bc, bn))), (h, hd)), pv))| {
        let pv = pv.unwrap_or([0; 4]);
        row(vec![
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(r),
            V::I(bc),
            ostr(bn),
            V::I(origid.get(p).unwrap()),
            ostr(title.get(p)),
            V::T(d),
            V::I(post_history_type_id.get(h).unwrap()),
            V::T(hd),
            V::I(pv[0]),
            V::I(pv[1]),
            V::I(pv[2]),
            V::I(pv[3]),
            ostr(comment.get(h)),
        ])
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalQuestions, TotalAnswers, UpVotes - DownVotes AS NetVotes FROM UserStats ORDER BY NetVotes DESC LIMIT 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, pt.Name AS PostType, array_agg(DISTINCT t.TagName) AS Tags
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     LEFT JOIN unnest(string_to_array(p.Tags, '>')) AS t(TagName) ON t.TagName IS NOT NULL
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.Score, pt.Name)
// SELECT tu.DisplayName AS TopUser, pd.Title AS PostTitle, pd.CreationDate AS PostDate, pd.Score AS PostScore, pd.PostType AS TypeOfPost, pd.Tags AS AssociatedTags
// FROM TopUsers tu JOIN PostDetails pd ON tu.DisplayName = pd.OwnerDisplayName ORDER BY tu.NetVotes DESC, pd.Score DESC;
fn q9600(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, title, score, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold(0i64, |n, p| n + (p.flatten() == Some(2)) as i64 - (p.flatten() == Some(3)) as i64);
    let tu = top_n(drain(&us), |&(_, n)| Reverse(n), 10);
    let r = rel(tu);
    let tn: HashIdx<Str, (Id<User>, i64)> = (&r).map(|(u, _)| u).select(&db.user.display_name).inv().select(&r).collect();
    let tg = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(tags_str.flat_map(|t: Str| t.split('>')).opt())
        .buf_fold(|v| -> &'static [Option<Str>] {
            let mut t: Vec<Option<Str>> = v.to_vec();
            t.sort();
            t.dedup();
            Box::leak(t.into_boxed_slice())
        });
    let v = drain(db.post.with(&tg).select(Ident::<Post>::new().and(&tg).and(ptype_name(db)).and(owner_user.select(&db.user.display_name).select(&tn))));
    rows(v.into_iter().map(|(_, (((p, t), pt), (u, _)))| {
        row(vec![V::S(db.user.display_name.get(u).unwrap()), ostr(title.get(p)), V::T(creation_date.get(p).unwrap()), V::I(score.get(p).unwrap()), V::S(pt), V::L(t.iter().map(|&x| ostr(x)).collect())])
    }))
}

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, U.DisplayName AS Author, U.Reputation AS AuthorReputation, P.ViewCount, P.Score,
//        ROW_NUMBER() OVER (PARTITION BY P.Tags ORDER BY P.Score DESC) AS TagRank, ARRAY_AGG(DISTINCT T.TagName) AS TagList
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     LEFT JOIN LATERAL unnest(string_to_array(substring(P.Tags, 2, length(P.Tags) - 2), '><')) AS T(TagName) ON true
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.Body, P.CreationDate, U.DisplayName, U.Reputation, P.ViewCount, P.Score, P.Tags),
// MostVotedTags AS (SELECT TagList, COUNT(*) AS PostCount FROM RankedPosts WHERE TagRank = 1 GROUP BY TagList HAVING COUNT(*) > 5),
// FinalOutput AS (SELECT RP.PostId, RP.Title, RP.Author, RP.AuthorReputation, RP.ViewCount, RP.Score, MT.TagList, RANK() OVER (ORDER BY RP.Score DESC) AS PostRank
//     FROM RankedPosts RP JOIN MostVotedTags MT ON MT.TagList @> RP.TagList)
// SELECT FO.PostId, FO.Title, FO.Author, FO.AuthorReputation, FO.ViewCount, FO.Score, FO.TagList, FO.PostRank
// FROM FinalOutput FO WHERE FO.PostRank <= 10 ORDER BY FO.Score DESC, FO.ViewCount DESC;
//
// Each Tags partition has exactly one TagRank = 1 row and its TagList depends only on Tags, so the ROW_NUMBER tie cannot change MostVotedTags.
fn q29126(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, score, origid, title, view_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let tl = qs().group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t[1..t.len() - 1].split("><")).opt()).buf_fold(|v| -> &'static [Option<Str>] {
        let mut t: Vec<Option<Str>> = v.to_vec();
        t.sort();
        t.dedup();
        Box::leak(t.into_boxed_slice())
    });
    let mt = qs()
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(_, s)| s, desc)
        .filt(|(_, r)| r == 1)
        .map(|((p, _), _)| p)
        .group_by(&tl)
        .fold(0i64, |n, _| n + 1)
        .filt(|n| n > 5);
    let within = |a: &[Option<Str>], b: &[Option<Str>]| a.iter().all(|x| x.is_some() && b.contains(x));
    let j = drain(db.post.with(&tl).select(Ident::<Post>::new().and(&tl).and(score)).cross(rel(drain(&mt))).filt(move |(((_, a), _), (b, _))| within(a, b)));
    let j = ranked(j, |&(_, ((_, s), _))| Reverse(s), false);
    let v = drain(rel(j).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, ((_, (((p, _), s), (b, _))), r))| {
        row(vec![
            V::I(origid.get(p).unwrap()),
            ostr(title.get(p)),
            V::S(owner_user.select(&db.user.display_name).get(p).unwrap()),
            V::I(owner_user.select(&db.user.reputation).get(p).unwrap()),
            oint(view_count.get(p)),
            V::I(s),
            V::L(b.iter().map(|&x| ostr(x)).collect()),
            V::I(r),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN,
//        COALESCE(u.Reputation, 0) AS UserReputation
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopUsers AS (SELECT OwnerUserId, COUNT(*) AS PostCount, SUM(Score) AS TotalScore, AVG(ViewCount) AS AvgViewCount FROM Posts GROUP BY OwnerUserId HAVING COUNT(*) > 5),
// PostHistoryTags AS (SELECT ph.PostId, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id LEFT JOIN UNNEST(string_to_array(p.Tags, ',')) AS tag(tagName) ON TRUE
//     JOIN Tags t ON t.TagName = TRIM(tag.tagName) WHERE ph.CreationDate >= cast('2024-10-01' as date) - INTERVAL '6 months' GROUP BY ph.PostId)
// SELECT up.OwnerUserId, u.DisplayName AS UserDisplayName, up.TotalScore, up.PostCount, up.AvgViewCount, pp.PostId, pp.Title, pp.CreationDate, pp.ViewCount, pp.Score,
//        COALESCE(pht.Tags, 'No Tags') AS PostTags, CASE WHEN pp.Score >= 0 THEN 'Non-negative Score' ELSE 'Negative Score' END AS ScoreCategory
// FROM TopUsers up JOIN RankedPosts pp ON up.OwnerUserId = pp.PostId LEFT JOIN PostHistoryTags pht ON pp.PostId = pht.PostId
// JOIN Users u ON u.Id = up.OwnerUserId WHERE pp.RN = 1 AND up.PostCount > 10
// ORDER BY up.TotalScore DESC, pp.ViewCount DESC LIMIT 50;
fn q22105(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, view_count, origid, title, tags_str, .. } = &db.post;
    let pp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, r)| r == 1)
        .map(|((p, _), _)| p)
        .collect();
    let pp: HashIdx<i64, Id<Post>> = (&pp).select(origid).inv().collect();
    let up = db
        .post
        .group_by(owner_user_id)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, v)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)])
        .filt(|a: [i64; 4]| a[0] > 5);
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pht = db
        .post_history
        .with((&db.post_history.creation_date).ge(add_months(date(2024, 10, 1), -6)))
        .group_by(&db.post_history.post)
        .select((&db.post_history.post).select(tags_str.flat_map(|t: Str| t.split(',')).map(|t: Str| t.trim()).select(&names).select(&db.tag.tag_name)))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.to_vec();
            t.sort();
            t.dedup();
            leak(t.join(", "))
        });
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&up).filt(|a: [i64; 4]| a[0] > 10).and(&pp).and(&uid));
    let v = drain(rel(v).select(Same::<(i64, (([i64; 4], Id<Post>), Id<User>))>::new().and(Same::<(i64, (([i64; 4], Id<Post>), Id<User>))>::new().map(|x: (i64, (([i64; 4], Id<Post>), Id<User>))| x.1 .0 .1).select((&pht).opt()))));
    let v = top_n(v, |&(_, ((_, ((a, p), _)), _))| (Reverse(a[1]), view_count.get(p).is_some(), Reverse(view_count.get(p))), 50);
    rows(v.into_iter().map(|(_, ((o, ((a, p), u)), t))| {
        let s = score.get(p).unwrap();
        row(vec![
            V::I(o),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(a[1]),
            V::I(a[0]),
            avg(a[3], a[2]),
            V::I(origid.get(p).unwrap()),
            ostr(title.get(p)),
            V::T(creation_date.get(p).unwrap()),
            oint(view_count.get(p)),
            V::I(s),
            V::S(t.unwrap_or("No Tags")),
            V::S(if s >= 0 { "Non-negative Score" } else { "Negative Score" }),
        ])
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostActivity AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT ph.CreationDate) AS EditCount, MAX(p.CreationDate) AS PostCreationDate,
//        MAX(ph.CreationDate) AS LastEditDate, SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS ClosureChanges
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id),
// TagStats AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
//     FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%') GROUP BY t.TagName),
// RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.ViewCount > 1000)
// SELECT ups.DisplayName, ups.UpVotes, uvs.DownVotes, ta.TagName, ps.Title AS PopularPost, ps.ViewCount, ps.Score, pa.CommentCount, pa.EditCount, pa.ClosureChanges
// FROM UserVoteStats ups JOIN UserVoteStats uvs ON ups.UserId = uvs.UserId JOIN TagStats ta ON ups.UpVotes > 10 AND ta.PostCount > 1
// JOIN RankedPosts ps ON ps.PostRank <= 5 JOIN PostActivity pa ON ps.Id = pa.PostId
// WHERE ups.UpVotes > uvs.DownVotes AND pa.EditCount > 0 AND ta.TotalViews > 50 AND ps.Score IS NOT NULL
// ORDER BY ups.UpVotes DESC, ps.ViewCount DESC;
//
// PostRank reads only base columns, so PostActivity is folded only for the ranked posts.
fn q20533(db: &'static So) -> String {
    let Post { view_count, score, title, .. } = &db.post;
    let uvs = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt())
        .fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64))
        .filt(|(u, d)| u > 10 && u > d);
    let lt = tag_mentions(db);
    let ta = (&lt)
        .group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|x: (Id<Post>, Id<Tag>)| x.1).select(&db.tag.tag_name))
        .select(Same::<(Id<Post>, Id<Tag>)>::new().map(|x: (Id<Post>, Id<Tag>)| x.0).select(view_count.opt()))
        .fold((0i64, 0i64), |(n, s), w| (n + 1, s + w.unwrap_or(0)))
        .filt(|(n, s)| n > 1 && s > 50);
    let ps: MatSet<Id<Post>> = db
        .post
        .with(view_count.gt(1000))
        .group_by(&db.post.post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(_, s)| s, desc)
        .filt(|(_, r)| r <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let pa = (&ps)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select((&db.post_history.creation_date).and(&db.post_history.post_history_type_id)).opt()))
        .buf_fold(|v| {
            let mut d: Vec<i64> = v.iter().filter_map(|x| x.1.map(|h| h.0)).collect();
            d.sort();
            d.dedup();
            [v.iter().filter(|x| x.0.is_some()).count() as i64, d.len() as i64, v.iter().filter(|x| matches!(x.1, Some((_, 10 | 11)))).count() as i64]
        })
        .filt(|a: [i64; 3]| a[1] > 0);
    let v = drain((&uvs).cross(&ta).cross((&ps).select(Ident::<Post>::new().and(&pa))));
    rows(v.into_iter().map(|(((u, t), _), (((up, dn), _), (p, a)))| {
        row(vec![
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(up),
            V::I(dn),
            V::S(t),
            ostr(title.get(p)),
            oint(view_count.get(p)),
            V::I(score.get(p).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
        ])
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10328", q10328),
    ("14506", q14506),
    ("29016", q29016),
    ("12962", q12962),
    ("11814", q11814),
    ("13791", q13791),
    ("6872", q6872),
    ("25550", q25550),
    ("26829", q26829),
    ("12487", q12487),
    ("9095", q9095),
    ("10331", q10331),
    ("8893", q8893),
    ("29010", q29010),
    ("3542", q3542),
    ("8326", q8326),
    ("25128", q25128),
    ("29675", q29675),
    ("23503", q23503),
    ("451", q451),
    ("4025", q4025),
    ("24454", q24454),
    ("24043", q24043),
    ("28029", q28029),
    ("23269", q23269),
    ("9600", q9600),
    ("29126", q29126),
    ("22105", q22105),
    ("20533", q20533),
];
