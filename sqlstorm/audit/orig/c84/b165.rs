use harness::prelude::*;
use std::cmp::Reverse;

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') AND p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, FavoriteCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10)
// SELECT t.PostId, t.Title, t.CreationDate, t.Score, t.ViewCount, t.AnswerCount, t.CommentCount, t.FavoriteCount, t.OwnerDisplayName,
//        COALESCE(avg(c.Score), 0) AS AverageCommentScore, COUNT(distinct v.Id) AS VoteCount
// FROM TopPosts t LEFT JOIN Comments c ON t.PostId = c.PostId LEFT JOIN Votes v ON t.PostId = v.PostId AND v.VoteTypeId IN (2, 3)
// GROUP BY t.PostId, t.Title, t.CreationDate, t.Score, t.ViewCount, t.AnswerCount, t.CommentCount, t.FavoriteCount, t.OwnerDisplayName
// ORDER BY t.Score DESC, t.CreationDate DESC;
fn q6864(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ud = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(&db.comment.score).opt().and(ud().opt()))
        .fold([0i64; 2], |a, (c, _)| [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0)]);
    let d = (&tp).group_by(Ident::<Post>::new()).select(ud().opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&s).and(&d)).into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner"]);
        f.extend([if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }, V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE OwnerPostRank <= 5),
// VotesSummary AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS Downvotes, COUNT(*) AS TotalVotes
//     FROM Votes GROUP BY PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, ts.Upvotes, ts.Downvotes, ts.TotalVotes,
//        CASE WHEN ts.TotalVotes = 0 THEN 'No Votes' WHEN ts.Upvotes > ts.Downvotes THEN 'Positive' WHEN ts.Upvotes < ts.Downvotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM TopPosts tp LEFT JOIN VotesSummary ts ON tp.PostId = ts.PostId ORDER BY tp.CreationDate DESC LIMIT 50;
fn q1429(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = db.vote.with(&db.vote.post).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let v = top_n(drain((&tp).select(Ident::<Post>::new().and((&vs).opt()))), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (_, s))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(match s {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(if a[2] == 0 { "No Votes" } else if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" })],
            None => [V::Null, V::Null, V::Null, V::S("Neutral")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, U.DisplayName AS OwnerName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, OwnerName FROM RankedPosts WHERE rn <= 5),
// PostVotes AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId)
// SELECT TP.PostId, TP.Title, TP.Score, TP.ViewCount, TP.OwnerName, COALESCE(PV.UpVotes, 0) AS TotalUpVotes, COALESCE(PV.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(PC.CommentCount, 0) AS TotalComments
// FROM TopPosts TP LEFT JOIN PostVotes PV ON TP.PostId = PV.PostId LEFT JOIN PostComments PC ON TP.PostId = PC.PostId
// ORDER BY TP.Score DESC, TP.ViewCount DESC LIMIT 20;
fn q4082(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&pv).and(&pc)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 20);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, PFT.Name AS PostType,
//        ROW_NUMBER() OVER (PARTITION BY PFT.Name ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN PostTypes PFT ON p.PostTypeId = PFT.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// TopRanked AS (SELECT PostId, Title, CreationDate, Score, ViewCount, PostType FROM RankedPosts WHERE Rank <= 10),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT TR.PostId, TR.Title, TR.CreationDate, TR.Score, TR.ViewCount, TR.PostType, UE.UserId, UE.DisplayName, UE.VoteCount, UE.UpVotes, UE.DownVotes, PC.CommentCount
// FROM TopRanked TR JOIN UserEngagement UE ON TR.PostId = UE.UserId JOIN PostComments PC ON TR.PostId = PC.PostId ORDER BY TR.Score DESC, TR.ViewCount DESC;
//
// `TR.PostId = UE.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q8491(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tr: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ue = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let pc = (&tr).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&pc).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ue)))).into_iter().map(|(p, (c, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "type"]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)]);
        row(f)
    }))
}

// WITH RECURSIVE UserVotes AS (SELECT U.Id AS UserId, U.Reputation, V.VoteTypeId, COUNT(V.Id) AS VoteCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.Reputation, V.VoteTypeId),
// PostInfo AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END) AS CommentCount,
//        SUM(V.BountyAmount) AS TotalBounty FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.ViewCount, P.Score, P.AcceptedAnswerId),
// TopPosts AS (SELECT PI.PostId, PI.Title, PI.ViewCount, PI.Score, PI.AcceptedAnswerId, PI.CommentCount, PI.TotalBounty,
//        ROW_NUMBER() OVER (PARTITION BY PI.AcceptedAnswerId ORDER BY PI.Score DESC) AS RowNum FROM PostInfo PI)
// SELECT U.DisplayName, U.Reputation, TP.Title, TP.ViewCount, TP.Score, TP.CommentCount, TP.TotalBounty
// FROM Users U INNER JOIN Votes V ON U.Id = V.UserId INNER JOIN TopPosts TP ON V.PostId = TP.PostId
// WHERE TP.RowNum = 1 AND U.Reputation > 1000 AND (TP.TotalBounty IS NULL OR TP.TotalBounty > 0) ORDER BY U.Reputation DESC, TP.Score DESC;
//
// WITH RECURSIVE, but no CTE refers to itself (and UserVotes is never read). RowNum reads only base columns, so the winners are picked first
// and the comment x vote product is driven for those alone.
fn q32341(db: &'static So) -> String {
    let Post { accepted_answer_id, score, .. } = &db.post;
    let v = drain(db.post.select(accepted_answer_id.opt()));
    let top = top_per(v, |&(_, a)| a.unwrap_or(0), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pi = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let Vote { user, post, .. } = &db.vote;
    let v = drain(db.vote.select(user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))).and(post.select(Ident::<Post>::new().and((&pi).filt(|a| a[1] == 0 || a[2] > 0))))));
    rows(v.into_iter().map(|(_, (u, (p, a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(a[0]), nullable(a[2], a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     LEFT JOIN Votes vote ON p.Id = vote.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.CommentCount, rp.AnswerCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.*, (tp.UpVotes - tp.DownVotes) AS NetVotes, CASE WHEN tp.Score > 100 THEN 'High' WHEN tp.Score BETWEEN 50 AND 100 THEN 'Medium' ELSE 'Low' END AS ScoreCategory
// FROM TopPosts tp ORDER BY tp.Score DESC;
//
// Rank reads only Score and CreationDate, so the top questions are picked first and the comment x answer x vote product is driven for those alone.
fn q5812(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap())), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&s).and(&cc).and(&ac)).into_iter().map(|(p, ((a, c), n))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::S(if s > 100 { "High" } else if s >= 50 { "Medium" } else { "Low" })]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(c.Score, 0)) AS TotalCommentScore, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty,
//        MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName),
// PostHistoryDetails AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount, MAX(ph.CreationDate) AS LastHistoryDate FROM PostHistory ph GROUP BY ph.PostId),
// RankedUsers AS (SELECT ue.*, ROW_NUMBER() OVER (ORDER BY ue.PostCount DESC) AS Rank FROM UserEngagement ue)
// SELECT ru.UserId, ru.DisplayName, ru.PostCount, ru.TotalCommentScore, ru.TotalBounty, ru.LastPostDate, COALESCE(ph.CloseReopenCount, 0) AS CloseReopenCount, ph.LastHistoryDate,
//        CASE WHEN ru.PostCount > 100 THEN 'Experienced' WHEN ru.PostCount BETWEEN 50 AND 100 THEN 'Moderate' ELSE 'Novice' END AS ExperienceLevel
// FROM RankedUsers ru LEFT JOIN PostHistoryDetails ph ON ru.UserId = ph.PostId WHERE ru.Rank <= 10 ORDER BY ru.TotalBounty DESC, ru.TotalCommentScore DESC LIMIT 10 OFFSET 0;
//
// Rank reads only the post count, so the ten users are picked first and the post x comment x vote product is driven for those alone.
// `ru.UserId = ph.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q3522(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let top = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ue = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.creation_date).and(comments_of(db).select(&db.comment.score).opt()).and(bounty.opt())).opt())
        .fold([0i64, 0, i64::MIN, 0, 0], |a, x| match x {
            Some(((d, c), b)) => {
                let b = b.flatten();
                [a[0] + c.unwrap_or(0), a[1] + b.unwrap_or(0), a[2].max(d), a[3] + c.is_some() as i64, a[4] + b.is_some() as i64]
            }
            None => a,
        });
    let PostHistory { post, post_history_type_id, creation_date, .. } = &db.post_history;
    let phd = db.post_history.group_by(post).select(post_history_type_id.and(creation_date)).fold((0i64, i64::MIN), |(n, m), (t, d)| (n + (t == 10 || t == 11) as i64, m.max(d)));
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain((&ue).and(&pc).and((&db.user.origid).select(&pidx).select(&phd).opt()));
    let v = top_n(v, |&(u, ((a, _), _))| (Reverse(a[1]), Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, ((a, n), h))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), nullable(a[0], a[3]), nullable(a[1], a[4]), tmax(a[2])]);
        f.extend(match h {
            Some((c, d)) => [V::I(c), V::T(d)],
            None => [V::I(0), V::Null],
        });
        f.push(V::S(if n > 100 { "Experienced" } else if n >= 50 { "Moderate" } else { "Novice" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS Owner, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, Body, CreationDate, Owner, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.Title, tp.CreationDate, tp.Owner, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, pt.Name AS PostTypeName, ph.Comment AS EditComment
// FROM TopPosts tp JOIN PostTypes pt ON tp.PostId = pt.Id LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId AND ph.PostHistoryTypeId = 24 ORDER BY tp.CreationDate DESC;
//
// Rank reads only base columns, so each type's five newest posts are picked first and the product is driven for those alone.
// `tp.PostId = pt.Id` joins a post id to a post type id, so it goes through the raw ids.
fn q27420(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tidx: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(24)));
    let v = drain((&s).and((&db.post.origid).select(&tidx)).and(edits.opt()));
    rows(v.into_iter().map(|(p, ((a, t), h))| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(db.post_type.name.get(t).unwrap()));
        f.push(h.map_or(V::Null, |h| ostr(db.post_history.comment.get(h))));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(B.Id) AS BadgeCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PopularPosts AS (SELECT PD.PostId, PD.Title, PD.CreationDate, PD.Score, PD.ViewCount, PD.OwnerDisplayName, ROW_NUMBER() OVER (ORDER BY PD.Score DESC, PD.ViewCount DESC) AS Rank FROM PostDetails PD)
// SELECT US.UserId, US.DisplayName, US.Reputation, US.BadgeCount, US.TotalUpVotes, US.TotalDownVotes, PP.Title, PP.Score, PP.ViewCount, PP.OwnerDisplayName
// FROM UserStats US JOIN PopularPosts PP ON PP.Rank <= 10 ORDER BY US.Reputation DESC, PP.Score DESC;
//
// The ON clause names only PP, so users are crossed with the ten popular posts.
fn q5544(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (b, t)| [a[0] + b.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let pp = top_n(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(Ident::<Post>::new())), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let pp = rel(pp.into_iter().map(|x| x.0).collect());
    let mut v = Vec::new();
    (&us).cross(&pp).drive(|(u, _), (a, p)| v.push((u, a, p)));
    rows(v.into_iter().map(|(u, a, p)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "score", "views", "owner"]));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalBadges, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation)
// SELECT tu.UserId, u.DisplayName, tu.Reputation, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalBadges, COALESCE(AVG(vs.VoteCount), 0) AS AverageVotes
// FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id
// LEFT JOIN (SELECT p.OwnerUserId, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId) vs ON tu.UserId = vs.OwnerUserId
// WHERE tu.Rank <= 10 GROUP BY tu.UserId, u.DisplayName, tu.Reputation, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalBadges ORDER BY tu.Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first. Every COUNT(DISTINCT) undoes its own fan-out, so each is a fold over one child.
fn q8253(db: &'static So) -> String {
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]);
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let vs = db.post.with(&db.post.owner_user).group_by(&db.post.owner_user).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&pc).and(&bc).and((&vs).opt())).into_iter().map(|(u, ((a, b), n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b), V::F(n.unwrap_or(0) as f64)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, COALESCE(p.AcceptedAnswerId, 0) AS HasAcceptedAnswer, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS TagRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.AcceptedAnswerId, p.Tags),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.HasAcceptedAnswer, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.TagRank = 1)
// SELECT fp.PostId, fp.Title, fp.ViewCount, fp.CreationDate, fp.HasAcceptedAnswer, fp.CommentCount, fp.UpVotes, fp.DownVotes,
//        CASE WHEN fp.HasAcceptedAnswer = 1 THEN 'Yes' ELSE 'No' END AS AcceptedAnswerStatus
// FROM FilteredPosts fp ORDER BY fp.ViewCount DESC, fp.UpVotes DESC LIMIT 50;
//
// TagRank reads only base columns, so each tag list's newest question is picked first and the comment x vote product is driven for those alone.
fn q8525(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, view_count, accepted_answer_id, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, a)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(a[1]), p)
    }, 50);
    rows(v.into_iter().map(|(p, a)| {
        let h = accepted_answer_id.get(p).unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "views", "created"]);
        f.extend([V::I(h), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(if h == 1 { "Yes" } else { "No" })]);
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopPosts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UBS.TotalBadges, 0) AS BadgeCount, COALESCE(TP.PostCount, 0) AS PostCount, COALESCE(TP.TotalScore, 0) AS TotalScore,
//        COALESCE(TP.AvgViewCount, 0) AS AvgViewCount FROM Users U LEFT JOIN UserBadgeStats UBS ON U.Id = UBS.UserId LEFT JOIN TopPosts TP ON U.Id = TP.OwnerUserId)
// SELECT U.DisplayName, U.BadgeCount, U.PostCount, U.TotalScore, U.AvgViewCount, RANK() OVER (ORDER BY U.TotalScore DESC, U.PostCount DESC) AS Rank
// FROM UserPerformance U ORDER BY Rank LIMIT 10;
fn q9140(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let v = drain((&bc).and((&tp).opt()));
    let key = |a: &Option<[i64; 4]>| {
        let a = a.unwrap_or([0; 4]);
        (Reverse(a[1]), Reverse(a[0]))
    };
    let v = ranked(v, |&(_, (_, a))| key(&a), false);
    let v = top_n(v, |&((u, (_, a)), r)| (r, key(&a), u), 10);
    rows(v.into_iter().map(|((u, (b, a)), r)| {
        let a = a.unwrap_or([0; 4]);
        row(vec![user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[1]), if a[2] == 0 { V::F(0.0) } else { avg(a[3], a[2]) }, V::I(r)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT a.Id) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        RANK() OVER (ORDER BY COUNT(DISTINCT a.Id) DESC, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// CommentedPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.AnswerCount, rp.UpVotes, rp.DownVotes, COUNT(c.Id) AS CommentCount
//     FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId = rp.Id GROUP BY rp.Id, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.AnswerCount, rp.UpVotes, rp.DownVotes)
// SELECT cp.Title, cp.OwnerDisplayName, cp.CreationDate, cp.AnswerCount, cp.UpVotes, cp.DownVotes, cp.CommentCount, RANK() OVER (ORDER BY cp.UpVotes - cp.DownVotes DESC) AS PopularityRank
// FROM CommentedPosts cp WHERE cp.AnswerCount > 0 ORDER BY PopularityRank LIMIT 10;
fn q9327(db: &'static So) -> String {
    let qs = || db.post.with((&db.post.post_type_id).eq(1));
    let rp = qs()
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ac = qs().group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&rp).and((&ac).filt(|n| n > 0)).and(&cc)), |&(_, ((a, _), _))| Reverse(a[0] - a[1]), false);
    let v = top_n(v, |&((p, _), r)| (r, p), 10);
    rows(v.into_iter().map(|((p, ((a, n), c)), r)| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(c), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE u.Reputation > 0 GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT ur.UserId, ur.Reputation, ur.PostCount, ur.TotalBounties, ROW_NUMBER() OVER (ORDER BY ur.Reputation DESC, ur.PostCount DESC) AS Ranking FROM UserReputation ur)
// SELECT tu.UserId, u.DisplayName, tu.Reputation, tu.PostCount, tu.TotalBounties, COALESCE(rp.Title, 'No Posts') AS RecentPostTitle, COALESCE(rp.CreationDate, NULL) AS RecentPostDate
// FROM TopUsers tu LEFT JOIN Users u ON tu.UserId = u.Id LEFT JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId AND rp.PostRank = 1
// WHERE tu.Ranking <= 10 ORDER BY tu.Ranking;
//
// Ranking reads only Reputation and the post count, so the ten users are picked first and the post x bounty-vote product is driven for those alone.
fn q311(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let pc = db.user.with((&db.user.reputation).gt(0)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let top = top_n(drain(&pc), |&(u, n)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let tb = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |n, b| n + b.flatten().flatten().unwrap_or(0));
    let recent = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let first = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = rel(first.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&first).map(|(u, _)| u).inv().select(&first).collect();
    let v = drain((&pc).and(&tb).and((&by_user).map(|(_, p)| p).opt()));
    let v = top_n(v, |&(u, ((n, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u), 10);
    rows(v.into_iter().map(|(u, ((n, b), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(b)]);
        f.push(match p.and_then(|p| db.post.title.get(p)) {
            Some(t) => V::S(t),
            None => V::S("No Posts"),
        });
        f.push(p.map_or(V::Null, |p| V::T(creation_date.get(p).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, U.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PopularTags AS (SELECT T.TagName, COUNT(P.Id) AS TagCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName HAVING COUNT(P.Id) > 10),
// ClosedPosts AS (SELECT PostId, COUNT(*) AS CloseVoteCount FROM PostHistory WHERE PostHistoryTypeId = 10 GROUP BY PostId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.ViewCount, RP.Score, RP.AnswerCount, PT.TagName, COALESCE(CP.CloseVoteCount, 0) AS CloseVoteCount,
//        CASE WHEN RP.OwnerReputation > 1000 THEN 'High Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM RankedPosts RP LEFT JOIN PopularTags PT ON RP.Title ILIKE '%' || PT.TagName || '%' LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId
// WHERE RP.RowNum = 1 ORDER BY RP.Score DESC, RP.ViewCount DESC LIMIT 50;
fn q1214(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, title, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let pt = db.tag.group_by(&db.tag.tag_name).select(&by_tag).fold(0i64, |n, _| n + 1);
    let names: HashIdx<Str, (i64, Str)> = (&pt).filt(|n| n > 10).and(Same::<Str>::new()).collect();
    let titles: MatSet<Str> = (&rp).select(title).collect();
    let hit: HashIdx<Str, (i64, Str)> = (&titles).select_where(&names, |t: Str, n: Str| t.to_lowercase().contains(&n.to_lowercase())).collect();
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&rp).select(title.select(&hit).opt().and((&cp).opt())));
    rows(v.into_iter().map(|(p, (t, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.extend([t.map_or(V::Null, |(_, n)| V::S(n)), V::I(c.unwrap_or(0))]);
        f.push(V::S(if db.user.reputation.get(owner_user.get(p).unwrap()).unwrap() > 1000 { "High Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(B.Id) AS TotalBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// HighScorePosts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, AVG(P.Score) AS AvgScore, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY AVG(P.Score) DESC) AS Rank
//     FROM Posts P WHERE P.Score IS NOT NULL GROUP BY P.OwnerUserId)
// SELECT U.DisplayName, UBadge.GoldBadges, UBadge.SilverBadges, UBadge.BronzeBadges, HScore.TotalPosts, HScore.AvgScore,
//        CASE WHEN HScore.AvgScore IS NULL THEN 'No Posts' WHEN HScore.AvgScore > 50 THEN 'High Performer' WHEN HScore.AvgScore BETWEEN 20 AND 50 THEN 'Moderate Performer' ELSE 'Needs Improvement' END AS PerformanceCategory
// FROM Users U LEFT JOIN UserBadgeStats UBadge ON U.Id = UBadge.UserId LEFT JOIN HighScorePosts HScore ON U.Id = HScore.OwnerUserId
// WHERE U.LastAccessDate >= cast('2024-10-01' as date) - INTERVAL '1 YEAR' AND (UBadge.TotalBadges IS NULL OR UBadge.TotalBadges > 0) ORDER BY U.DisplayName ASC, HScore.AvgScore DESC NULLS LAST;
fn q705(db: &'static So) -> String {
    let users = || db.user.with((&db.user.last_access_date).ge(add_years(date(2024, 10, 1), -1)));
    let ub = users().group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64, a[3] + 1],
        None => a,
    });
    let hs = db.post.group_by(&db.post.owner_user).select(&db.post.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    rows(drain((&ub).filt(|a| a[3] > 0).and((&hs).opt())).into_iter().map(|(u, (b, h))| {
        let mut f = vec![user_col(db, u, "name"), V::I(b[0]), V::I(b[1]), V::I(b[2])];
        match h {
            Some(h) => {
                let m = h[1] as f64 / h[0] as f64;
                f.extend([V::I(h[0]), V::F(m), V::S(if m > 50.0 { "High Performer" } else if m >= 20.0 { "Moderate Performer" } else { "Needs Improvement" })]);
            }
            None => f.extend([V::Null, V::Null, V::S("No Posts")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT ur.UserId, ur.Reputation, ur.PostCount, RANK() OVER (ORDER BY ur.Reputation DESC) AS ReputationRank FROM UserReputation ur WHERE ur.Reputation > 1000)
// SELECT u.DisplayName, p.Title, p.Score, COALESCE(ph.Comment, 'No History') AS PostHistoryComment, t.TagName
// FROM RankedPosts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN PostHistory ph ON ph.PostId = p.PostId AND ph.PostHistoryTypeId = 10
// LEFT JOIN PostLinks pl ON pl.PostId = p.PostId LEFT JOIN Posts rp ON pl.RelatedPostId = rp.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.PostId
// WHERE p.PostRank = 1 AND u.Id IN (SELECT UserId FROM TopUsers WHERE ReputationRank <= 10) ORDER BY p.Score DESC, p.CreationDate DESC;
//
// `LEFT JOIN Posts rp ON pl.RelatedPostId = rp.Id` matches at most one row per link and rp is never read, so it is left out.
fn q2379(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let posters: MatSet<Id<User>> = db.post.select(owner_user).collect();
    let tu = ranked(drain((&posters).with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let recent = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(score.gt(0))).select(owner_user));
    let first = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let v = drain((&first).with(owner_user.select(&tu)).select(closes.opt().and(links_of(db).opt()).and(excerpt.opt())));
    rows(v.into_iter().map(|(p, ((h, _), t))| {
        let mut f = post_fields(db, p, &["owner", "title", "score"]);
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No History")));
        f.push(t.map_or(V::Null, |t| V::S(db.tag.tag_name.get(t).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT up.DisplayName, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(pc.CommentCount, 0) AS CommentCount, pc.LastCommentDate, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM RankedPosts rp JOIN Users up ON rp.PostId = up.Id LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN UserBadges ub ON up.Id = ub.UserId
// WHERE rp.UserPostRank <= 5 ORDER BY rp.Score DESC, up.DisplayName LIMIT 50;
//
// `rp.PostId = up.Id` joins a post id to a user id, so it goes through the raw ids.
fn q3718(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pc = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&rp).select((&db.post.origid).select(&uidx).select(Ident::<User>::new().and((&ub).opt())).and((&pc).opt())));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(score.get(p).unwrap()), db.user.display_name.get(u).unwrap(), p), 50);
    rows(v.into_iter().map(|(p, ((u, b), c))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        f.extend(match b {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Owner, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT v.Id) DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Owner, CommentCount, AnswerCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10)
// SELECT t.PostId, t.Title, t.CreationDate, t.Owner, t.CommentCount, t.AnswerCount, t.UpVotes, t.DownVotes, pht.Name AS PostHistoryType
// FROM TopPosts t LEFT JOIN PostHistory ph ON t.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// WHERE ph.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') ORDER BY t.UpVotes DESC, t.CreationDate DESC;
//
// COUNT(DISTINCT v.Id) is the question's own vote count, so the ten are picked first (a tie at the tenth is broken by post) and the product is driven for those alone.
fn q9218(db: &'static So) -> String {
    let qs = db.post.with((&db.post.post_type_id).eq(1));
    let vc = qs.group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let top = top_n(drain(&vc), |&(p, n)| (Reverse(n), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let recent = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    rows(drain((&s).and(&cc).and(&ac).and(recent)).into_iter().map(|(p, (((a, c), n), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1]), V::S(htype_name(db).get(h).unwrap())]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAsked, COUNT(DISTINCT a.Id) AS AnswersGiven,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes,
//        RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS QuestionRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Posts a ON u.Id = a.OwnerUserId AND a.PostTypeId = 2
//     LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId IN (SELECT Id FROM Posts) GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionsAsked, AnswersGiven, TotalUpvotes, TotalDownvotes, QuestionRank FROM UserEngagement WHERE QuestionsAsked > 0)
// SELECT tu.DisplayName, tu.QuestionsAsked, tu.AnswersGiven, tu.TotalUpvotes, tu.TotalDownvotes, (tu.TotalUpvotes - tu.TotalDownvotes) AS NetVotes,
//        CASE WHEN tu.QuestionRank <= 5 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId WHERE b.Class = 1 OR b.Class = 2 ORDER BY tu.QuestionsAsked DESC, NetVotes DESC;
//
// QuestionRank reads only the question count, and a user without questions ranks below every user with one, so the question x answer x vote
// product is driven only for the users with a question.
fn q2649(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let asks = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let answers = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2)));
    let qc = db.user.group_by(Ident::<User>::new()).select(asks().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let qr = ranked(drain(&qc), |&(_, n)| Reverse(n), false);
    let qr = rel(drain(rel(qr).filt(|((_, n), _)| n > 0)).into_iter().map(|(_, ((u, n), r))| (u, (n, r))).collect());
    let tu: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&qr).map(|(u, _)| u).inv().select(&qr).collect();
    let ac = (&tu).map(|(u, _)| u).group_by(Ident::<User>::new()).select(answers.opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let votes = votes_by(db).select(Ident::<Vote>::new().with(&db.vote.post)).select(&db.vote.vote_type_id);
    let ue = (&tu).map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(asks().and(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2))).opt()).and(votes.opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let gs = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).is_in([1, 2])));
    rows(drain((&ue).and(&ac).and((&tu).map(|(_, x)| x)).and(gs)).into_iter().map(|(u, (((a, n), (q, r)), _))| {
        row(vec![user_col(db, u, "name"), V::I(q), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::S(if r <= 5 { "Top Contributor" } else { "Regular Contributor" })])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, u.Reputation, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.DisplayName, us.Reputation, us.GoldBadges, us.SilverBadges, us.BronzeBadges, rp.Title, rp.CreationDate, rp.CommentCount
// FROM UserStatistics us JOIN RankedPosts rp ON us.UserId = rp.PostId WHERE rp.PostRank = 1 AND us.Reputation >= 1000 AND (us.GoldBadges > 0 OR us.SilverBadges > 5)
// ORDER BY us.Reputation DESC, rp.CreationDate ASC LIMIT 10;
//
// `us.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q3295(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let us = db.user.with((&db.user.reputation).ge(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&cc).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and((&us).filt(|a| a[0] > 0 || a[1] > 5)))));
    let v = top_n(v, |&(p, (_, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), creation_date.get(p).unwrap(), p), 10);
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "created"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostScores AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetScore,
//        COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoters
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// RankedPosts AS (SELECT ps.PostId, ps.NetScore, ps.CommentCount, ROW_NUMBER() OVER (ORDER BY ps.NetScore DESC, ps.CommentCount DESC) AS Rank FROM PostScores ps),
// TopUsers AS (SELECT ub.UserId, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY (ub.GoldBadges + ub.SilverBadges * 0.5 + ub.BronzeBadges * 0.25) DESC) AS UserRank FROM UserBadges ub)
// SELECT pu.UserId, pu.UserRank, rp.PostId, rp.NetScore, rp.CommentCount FROM TopUsers pu JOIN RankedPosts rp ON pu.UserRank <= 10 WHERE rp.NetScore > 0
// ORDER BY pu.UserRank, rp.NetScore DESC LIMIT 5;
//
// The ON clause names only pu, so the ten top users are crossed with the posts.
fn q543(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |w, c| w + match c {
        Some(1) => 4,
        Some(2) => 2,
        Some(3) => 1,
        _ => 0,
    });
    let tu = top_n(drain(&ub), |&(u, w)| (Reverse(w), u), 10);
    let tu = rel(tu.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let ud = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select(&db.vote.vote_type_id);
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(ud.opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (t, c)| [a[0] + (t == Some(2)) as i64 - (t == Some(3)) as i64, a[1] + c.is_some() as i64]);
    let psv = rel(drain((&ps).filt(|a| a[0] > 0)));
    let mut v = Vec::new();
    (&tu).cross(&psv).drive(|_, ((u, r), (p, a))| v.push((u, r, p, a)));
    let v = top_n(v, |&(_, r, p, a)| (r, Reverse(a[0]), p), 5);
    rows(v.into_iter().map(|(u, r, p, a)| row(vec![user_col(db, u, "uid"), V::I(r), V::I(db.post.origid.get(p).unwrap()), V::I(a[0]), V::I(a[1])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS ScoreRank
//     FROM Posts p WHERE p.OwnerUserId IS NOT NULL AND p.Score > 0),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBounties FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users) GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, us.DisplayName AS TopUser, us.BadgeCount, us.TotalBounties FROM RankedPosts rp
//     JOIN UserStats us ON rp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = us.UserId) WHERE rp.ScoreRank <= 5)
// SELECT tp.Title, tp.Score, tp.TopUser, tp.BadgeCount, tp.TotalBounties,
//        COALESCE(GREATEST((SELECT COUNT(c.Id) FROM Comments c WHERE c.PostId = tp.PostId), (SELECT COUNT(pl.RelatedPostId) FROM PostLinks pl WHERE pl.PostId = tp.PostId)), 0) AS EngagementCount,
//        CASE WHEN tp.TotalBounties > 0 THEN 'Has Bounties' ELSE 'No Bounties' END AS BountyStatus
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.BadgeCount DESC;
//
// `rp.PostId IN (posts of us.UserId)` is the post's owner. UserStats is read only through that join, so it is folded for the owners of the ranked posts alone.
// `Reputation > AVG(Reputation)` is compared exactly as `rep * n > sum`.
fn q4638(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let v = ranked(drain(db.post.with(owner_user).with(score.gt(0)).select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()), false);
    let v = per_group(v, |&(_, t)| t);
    let rp: MatSet<Id<Post>> = rel(v.into_iter().filter(|x| x.1 <= 5).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let (n, sum) = (&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let us = (&owners)
        .with((&db.user.reputation).filt(move |r| r * n > sum))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (b, v)| {
            let v = v.flatten();
            [a[0] + b.is_some() as i64, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0)]
        });
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |k, c| k + c.is_some() as i64);
    let lc = (&rp).group_by(Ident::<Post>::new()).select(links_of(db).opt()).fold(0i64, |k, c| k + c.is_some() as i64);
    rows(drain((&cc).and(&lc).and(owner_user.select(Ident::<User>::new().and(&us)))).into_iter().map(|(p, ((c, l), (u, a)))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), V::I(c.max(l)), V::S(if a[1] > 0 && a[2] > 0 { "Has Bounties" } else { "No Bounties" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COUNT(DISTINCT P.Id) AS PostCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty, COUNT(B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, CreationDate, LastAccessDate, PostCount, QuestionCount, AnswerCount, TotalBounty, BadgeCount,
//        RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, CreationDate, LastAccessDate, PostCount, QuestionCount, AnswerCount, TotalBounty, BadgeCount, ReputationRank, PostCountRank
// FROM RankedUsers WHERE ReputationRank <= 10 OR PostCountRank <= 10 ORDER BY Reputation DESC, PostCount DESC;
//
// Both ranks read only Reputation and the distinct post count, so the users are picked first and the post x vote x badge product is driven for those alone.
fn q7734(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain(&pc), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, n), _)| Reverse(n), false);
    let v: Vec<_> = v.into_iter().filter(|&((_, r), c)| r <= 10 || c <= 10).collect();
    let pick = rel(v.iter().map(|&(((u, n), r), c)| (u, (n, r, c))).collect());
    let tu: HashIdx<Id<User>, (Id<User>, (i64, i64, i64))> = (&pick).map(|(u, _)| u).inv().select(&pick).collect();
    let prod = posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt().and(badges_of(db).opt());
    let us = (&tu).map(|(u, _)| u).group_by(Ident::<User>::new()).select(prod).fold([0i64; 4], |a, (p, b)| {
        let (t, w) = p.map_or((0, 0), |(t, v)| (t, v.flatten().unwrap_or(0)));
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w, a[3] + b.is_some() as i64]
    });
    rows(drain((&us).and((&tu).map(|(_, x)| x))).into_iter().map(|(u, (a, (n, r, c)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated", "last_access"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(r), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// PostCategories AS (SELECT pt.Name AS PostType, r.PostId, r.Title, r.CreationDate, r.ViewCount, r.Score, r.CommentCount, r.UpVoteCount, r.DownVoteCount, r.Rank
//     FROM RankedPosts r JOIN PostTypes pt ON r.PostId = pt.Id)
// SELECT pc.PostType, COUNT(*) AS TotalPosts, AVG(pc.ViewCount) AS AvgViews, SUM(pc.Score) AS TotalScore, SUM(pc.CommentCount) AS TotalComments, SUM(pc.UpVoteCount) AS TotalUpVotes,
//        SUM(pc.DownVoteCount) AS TotalDownVotes FROM PostCategories pc GROUP BY pc.PostType HAVING COUNT(*) > 5 ORDER BY TotalScore DESC;
//
// `r.PostId = pt.Id` joins a post id to a post type id, so it goes through the raw ids. Rank is never read.
fn q8077(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let tidx: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with((&db.post.origid).select(&tidx))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let g = db
        .post
        .with(&rp)
        .group_by((&db.post.origid).select(&tidx).select(&db.post_type.name))
        .select(Ident::<Post>::new().and(&rp))
        .fold([0i64; 7], |s, (p, a)| {
            let w = view_count.get(p);
            [s[0] + 1, s[1] + w.is_some() as i64, s[2] + w.unwrap_or(0), s[3] + score.get(p).unwrap(), s[4] + a[0], s[5] + a[1], s[6] + a[2]]
        });
    let mut v = drain((&g).filt(|s| s[0] > 5));
    v.sort_by_key(|&(_, s)| Reverse(s[3]));
    rows(v.into_iter().map(|(t, s)| row(vec![V::S(t), V::I(s[0]), avg(s[2], s[1]), V::I(s[3]), V::I(s[4]), V::I(s[5]), V::I(s[6])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS RankByScore, COUNT(c.Id) AS TotalComments
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, pt.Name),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, RP.TotalComments FROM RankedPosts RP WHERE RP.RankByScore <= 5)
// SELECT TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.ViewCount, TP.OwnerDisplayName, TP.TotalComments, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
// FROM TopPosts TP LEFT JOIN Votes v ON TP.PostId = v.PostId GROUP BY TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.ViewCount, TP.OwnerDisplayName, TP.TotalComments
// ORDER BY TP.Score DESC, TP.ViewCount DESC;
fn q9498(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let mut v = drain((&cc).and(&vc));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.PostRank, rp.CommentCount, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.UpVotes > rp.DownVotes THEN 'Positive' WHEN rp.UpVotes < rp.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
//     FROM RankedPosts rp WHERE rp.CommentCount > 5 AND rp.PostRank <= 3)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.VoteSentiment, COALESCE(NULLIF(fp.UpVotes - fp.DownVotes, 0), NULL) AS EffectiveVotes,
//        CONCAT('Post ID: ', fp.PostId, ' — Title: ', fp.Title) AS PostSummary
// FROM FilteredPosts fp ORDER BY fp.CreationDate DESC LIMIT 10;
//
// PostRank reads only base columns, so each owner's three newest posts are picked first and the comment x vote product is driven for those alone.
fn q3644(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain((&s).filt(|a| a[0] > 5)), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let d = a[1] - a[2];
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        f.push(if d == 0 { V::Null } else { V::I(d) });
        f.push(V::Owned(format!("Post ID: {} — Title: {}", db.post.origid.get(p).unwrap(), db.post.title.get(p).unwrap_or(""))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId),
// PostVotes AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY V.PostId)
// SELECT TP.PostId, TP.Title, TP.Score, TP.ViewCount, COALESCE(PC.CommentCount, 0) AS CommentCount, COALESCE(PV.UpVotes, 0) AS UpVotes, COALESCE(PV.DownVotes, 0) AS DownVotes, TP.OwnerDisplayName
// FROM TopPosts TP LEFT JOIN PostComments PC ON TP.PostId = PC.PostId LEFT JOIN PostVotes PV ON TP.PostId = PV.PostId ORDER BY TP.Score DESC, TP.ViewCount DESC;
fn q7056(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), post_fields(db, p, &["owner"]).remove(0)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT B.Id) AS BadgeCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, CommentCount, BadgeCount, TotalBounty, LastPostDate,
//        ROW_NUMBER() OVER (ORDER BY (QuestionCount * 2 + AnswerCount + CommentCount + BadgeCount + TotalBounty) DESC) AS Rank FROM UserActivity)
// SELECT TU.DisplayName, TU.QuestionCount, TU.AnswerCount, TU.CommentCount, TU.BadgeCount, TU.TotalBounty, TU.LastPostDate,
//        CASE WHEN TU.BadgeCount > 10 THEN 'Expert' WHEN TU.BadgeCount BETWEEN 5 AND 10 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel
// FROM TopUsers TU WHERE TU.Rank <= 10 ORDER BY TU.TotalBounty DESC;
fn q3496(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let per_post = post_type_id.and(creation_date).and(comments_of(db).opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt());
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(per_post).opt().and(badges_of(db).opt()))
        .fold([0i64, 0, 0, i64::MIN], |a, (p, _)| match p {
            Some((((t, d), _), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + v.flatten().unwrap_or(0), a[3].max(d)],
            None => a,
        });
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&ua).and(&cc).and(&bc)), |&(u, ((a, c), b))| (Reverse(a[0] * 2 + a[1] + c + b + a[2]), u), 10);
    let mut v = v;
    v.sort_by_key(|&(_, ((a, _), _))| Reverse(a[2]));
    rows(v.into_iter().map(|(u, ((a, c), b))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(c), V::I(b), V::I(a[2]), tmax(a[3]), V::S(if b > 10 { "Expert" } else if b >= 5 { "Intermediate" } else { "Novice" })])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, bh.BadgeCount, bc.CloseCount, COALESCE(v.UpVoteCount, 0) AS UpVoteCount,
//        COALESCE(v.DownVoteCount, 0) AS DownVoteCount
// FROM RankedPosts rp LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) bh ON bh.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId)
// LEFT JOIN (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId) bc ON bc.PostId = rp.PostId
// LEFT JOIN (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVoteCount FROM Votes GROUP BY PostId) v ON v.PostId = rp.PostId
// WHERE rp.rn <= 5 ORDER BY rp.CreationDate DESC;
fn q5021(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bh = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let vc = db.vote.with(&db.vote.post).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    rows(drain((&tp).select(owner_user.select(&bh).opt().and((&bc).opt()).and((&vc).opt()))).into_iter().map(|(p, ((b, c), a))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([oint(b), oint(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT Users.Id AS UserId, Users.DisplayName, Users.Reputation, Users.CreationDate, Users.LastAccessDate, Users.UpVotes, Users.DownVotes,
//        (Users.UpVotes - Users.DownVotes) AS NetVotes, COUNT(DISTINCT Posts.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN Posts.PostTypeId = 1 THEN Posts.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN Posts.PostTypeId = 2 THEN Posts.Id END) AS TotalAnswers, SUM(COALESCE(Posts.ViewCount, 0)) AS TotalViews
//     FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId GROUP BY Users.Id, Users.DisplayName, Users.Reputation, Users.CreationDate, Users.LastAccessDate, Users.UpVotes, Users.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, NetVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, NetVotes, ReputationRank, PostRank, ViewRank
// FROM TopUsers WHERE TotalPosts > 0 ORDER BY Reputation DESC, TotalPosts DESC, TotalViews DESC LIMIT 10;
fn q10368(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain(&ups), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[1]), false);
    let v = ranked(v, |&(((_, a), _), _)| Reverse(a[6]), false);
    let v = drain(rel(v).filt(|x: (((( Id<User>, [i64; 10]), i64), i64), i64)| x.0 .0 .0 .1[1] > 0)).into_iter().map(|x| x.1).collect();
    let v = top_n(v, |&((((u, a), _), _), _)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1]), Reverse(a[6]), u), 10);
    rows(v.into_iter().map(|((((u, a), r), p), w)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6])]);
        f.push(V::I(db.user.up_votes.get(u).unwrap() - db.user.down_votes.get(u).unwrap()));
        f.extend([V::I(r), V::I(p), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q5767(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.Score, p.CreationDate, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS TotalPosts FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PostEngagement AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COUNT(pl.RelatedPostId) AS RelatedCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN PostLinks pl ON p.Id = pl.PostId GROUP BY p.Id)
// SELECT up.DisplayName, rp.Title, rp.Score, up.UpVotes, up.DownVotes, pe.CommentCount, pe.RelatedCount
// FROM RankedPosts rp JOIN UserStats up ON rp.OwnerUserId = up.UserId JOIN PostEngagement pe ON rp.PostId = pe.PostId WHERE rp.Rank = 1
// ORDER BY up.TotalPosts DESC, rp.Score DESC FETCH FIRST 10 ROWS ONLY;
//
// UserStats and PostEngagement are read only through the joins, so they are folded for the owners and posts that reach them.
fn q4327(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let tp = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pe = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(links_of(db).opt())).fold([0i64; 2], |a, (c, l)| [a[0] + c.is_some() as i64, a[1] + l.is_some() as i64]);
    let v = drain((&pe).and(owner_user.select(Ident::<User>::new().and(&us).and(&tp))));
    let v = top_n(v, |&(p, (_, ((_, _), n)))| (Reverse(n), Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (e, ((u, a), _)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(e[0]), V::I(e[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(b.Id) AS BadgeCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount, BadgeCount FROM RankedPosts WHERE Rank <= 5)
// SELECT t.OwnerDisplayName, COUNT(DISTINCT t.PostId) AS TotalPosts, AVG(t.Score) AS AverageScore, SUM(t.ViewCount) AS TotalViews, SUM(t.CommentCount) AS TotalComments,
//        SUM(t.BadgeCount) AS TotalBadges
// FROM TopPosts t JOIN Users u ON t.OwnerDisplayName = u.DisplayName GROUP BY t.OwnerDisplayName ORDER BY TotalPosts DESC, AverageScore DESC FETCH FIRST 10 ROWS ONLY;
//
// Rank reads only base columns, so each owner's five best questions are picked first and the comment x badge product is driven for those alone.
fn q6363(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64]);
    let name = owner_user.select(&db.user.display_name).opt().map(|n: Option<Str>| n.unwrap_or("Community User"));
    let uname: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let g = (&tp)
        .group_by(name)
        .select(Ident::<Post>::new().and(&rp).and(owner_user.select(&db.user.display_name).opt().map(|n: Option<Str>| n.unwrap_or("Community User")).select(&uname)))
        .fold([0i64; 7], |s, ((p, a), _)| {
            let w = view_count.get(p);
            [s[0] + 1, s[1] + score.get(p).unwrap(), s[2] + w.is_some() as i64, s[3] + w.unwrap_or(0), s[4] + a[0], s[5] + a[1], 0]
        });
    let d = (&tp).group_by(owner_user.select(&db.user.display_name).opt().map(|n: Option<Str>| n.unwrap_or("Community User"))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&g).and(&d)), |&(n, (s, k))| (Reverse(k), Reverse(fkey(s[1] as f64 / s[0] as f64)), n), 10);
    rows(v.into_iter().map(|(n, (s, k))| row(vec![V::S(n), V::I(k), avg(s[1], s[0]), nullable(s[3], s[2]), V::I(s[4]), V::I(s[5])])))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Pos
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(rp.Upvotes) AS TotalUpvotes, SUM(rp.Downvotes) AS TotalDownvotes, COUNT(rp.PostId) AS PostCount
//     FROM Users u JOIN RecentPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName HAVING COUNT(rp.PostId) > 5 ORDER BY TotalUpvotes DESC LIMIT 10)
// SELECT u.UserId, u.DisplayName, u.TotalUpvotes, u.TotalDownvotes, u.PostCount, COALESCE(rp.Title, 'No Title') AS RecentlyPostedTitle, rp.CreationDate AS RecentPostDate,
//        CASE WHEN rp.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus
// FROM TopUsers u LEFT JOIN RecentPosts rp ON u.UserId = rp.OwnerUserId AND rp.Pos = 1 ORDER BY u.TotalUpvotes DESC;
fn q913(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30)));
    let rp = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tu = recent().group_by(owner_user).select(&rp).fold([0i64; 3], |s, a| [s[0] + a[1], s[1] + a[2], s[2] + 1]);
    let tu = top_n(drain((&tu).filt(|s| s[2] > 5)), |&(u, s)| (Reverse(s[0]), u), 10);
    let first = top_per(drain(recent().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = rel(first.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&first).map(|(u, _)| u).inv().select(&first).collect();
    let tv = rel(tu);
    let v = drain((&tv).select(Same::<(Id<User>, [i64; 3])>::new().and(Same::<(Id<User>, [i64; 3])>::new().map(|(u, _)| u).select((&by_user).map(|(_, p)| p).select(Ident::<Post>::new().and(&rp))).opt())));
    rows(v.into_iter().map(|(_, ((u, s), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(s[0]), V::I(s[1]), V::I(s[2])]);
        match p {
            Some((p, a)) => f.extend([V::S(db.post.title.get(p).unwrap_or("No Title")), V::T(creation_date.get(p).unwrap()), V::S(if a[0] > 0 { "Has Comments" } else { "No Comments" })]),
            None => f.extend([V::S("No Title"), V::Null, V::S("No Comments")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.rn = 1),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v
//     WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId)
// SELECT tp.Title, tp.CreationDate, tp.OwnerDisplayName, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes,
//        CASE WHEN COALESCE(rv.UpVotes, 0) > COALESCE(rv.DownVotes, 0) THEN 'Positive Feedback' WHEN COALESCE(rv.UpVotes, 0) < COALESCE(rv.DownVotes, 0) THEN 'Negative Feedback' ELSE 'No Feedback' END AS FeedbackStatus
// FROM TopPosts tp LEFT JOIN RecentVotes rv ON tp.PostId = rv.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC LIMIT 10;
fn q448(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let top = top_n(top, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { post, vote_type_id, creation_date: vd, .. } = &db.vote;
    let rv = db.vote.with(vd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(post).group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    rows(drain((&tp).select(Ident::<Post>::new().and((&rv).opt()))).into_iter().map(|(p, (_, a))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "Positive Feedback" } else if a[0] < a[1] { "Negative Feedback" } else { "No Feedback" })]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges GROUP BY UserId),
// PostWithDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, OwnerDisplayName, CommentCount, PostRank, OwnerUserId FROM PostWithDetails WHERE PostRank = 1)
// SELECT tb.OwnerDisplayName, COUNT(*) AS TopPostCount, SUM(COALESCE(ubc.GoldBadges, 0)) AS TotalGoldBadges, SUM(COALESCE(ubc.SilverBadges, 0)) AS TotalSilverBadges,
//        SUM(COALESCE(ubc.BronzeBadges, 0)) AS TotalBronzeBadges
// FROM TopPosts tb LEFT JOIN UserBadgeCounts ubc ON tb.OwnerUserId = ubc.UserId GROUP BY tb.OwnerDisplayName HAVING COUNT(*) > 5 ORDER BY TopPostCount DESC LIMIT 10;
fn q1525(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ubc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let g = (&tp)
        .group_by(owner_user.select(&db.user.display_name).opt().map(|n: Option<Str>| n.unwrap_or("Community")))
        .select(owner_user.select(&ubc).opt())
        .fold([0i64; 4], |s, b| {
            let b = b.unwrap_or([0; 3]);
            [s[0] + 1, s[1] + b[0], s[2] + b[1], s[3] + b[2]]
        });
    let v = top_n(drain((&g).filt(|s| s[0] > 5)), |&(n, s)| (Reverse(s[0]), n), 10);
    rows(v.into_iter().map(|(n, s)| row(vec![V::S(n), V::I(s[0]), V::I(s[1]), V::I(s[2]), V::I(s[3])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount,
//        ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.UpVotes, tp.DownVotes, tp.CommentCount, u.DisplayName AS OwnerDisplayName, COUNT(b.Id) AS BadgeCount
// FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.UpVotes, tp.DownVotes, tp.CommentCount, u.DisplayName ORDER BY tp.Score DESC;
//
// RankedPosts has no GROUP BY, so Rank numbers the post x vote x comment rows and TopPosts is ten of those rows, not ten posts. The rows are
// materialised and the ten kept; the vote/comment ids break ties inside one post, where every projected column agrees.
fn q7474(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let joined: MatSet<(Id<Post>, Option<Id<Vote>>, Option<Id<Comment>>)> = qs().select(Ident::<Post>::new().and(votes_of(db).opt()).and(comments_of(db).opt())).map(|((p, v), c)| (p, v, c)).collect();
    let wins = qs()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let top = top_n(drain(&joined), |&(r, _)| {
        let w = view_count.get(r.0);
        (Reverse(score.get(r.0).unwrap()), w.is_none(), Reverse(w), r)
    }, 10);
    let tp = rel(top.into_iter().map(|x| x.0).collect());
    type Row = (Id<Post>, Option<Id<Vote>>, Option<Id<Comment>>);
    let g = (&tp)
        .group_by(Same::<Row>::new().map(|r: Row| r.0))
        .select(Same::<Row>::new().map(|r: Row| r.0).select(owner_user).select(badges_of(db).opt()))
        .fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&g).and(&wins).and(owner_user));
    v.sort_by_key(|&(p, _)| Reverse(score.get(p).unwrap()));
    rows(v.into_iter().map(|(p, ((b, a), u))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), user_col(db, u, "name"), V::I(b)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(c.Score, 0)) AS CommentScore, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.Tags, COALESCE(ph.Comment, 'No Comment') AS LastEditComment,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS LastPostRank, p.OwnerUserId FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId)
// SELECT us.UserId, us.DisplayName, us.QuestionCount, us.AnswerCount, us.TotalViews, us.TotalScore, us.CommentScore, us.AvgReputation, pm.PostId, pm.Title, pm.CreationDate,
//        pm.ViewCount, pm.Score, pm.Tags, pm.LastEditComment
// FROM UserStats us LEFT JOIN PostMetrics pm ON us.UserId = pm.OwnerUserId WHERE us.QuestionCount > 0 ORDER BY us.TotalScore DESC, us.QuestionCount DESC;
//
// LastPostRank is never read.
fn q10399(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(comments_of(db).select(&db.comment.score).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((((t, w), s), c)) => [a[0] + (t == 2) as i64, a[1] + w.unwrap_or(0), a[2] + s, a[3] + c.unwrap_or(0)],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&us).and((&pc).filt(|n| n > 0)).and(posts_of(db).select(Ident::<Post>::new().and(history_of(db).opt()))));
    rows(v.into_iter().map(|(u, ((a, n), (p, h)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::F(db.user.reputation.get(u).unwrap() as f64)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "tags"]));
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No Comment")));
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 5 THEN 1 ELSE 0 END) AS TagWikiCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, tags.TagName AS RelatedTag, p.OwnerUserId
//     FROM Posts p LEFT JOIN LATERAL (SELECT unnest(string_to_array(p.Tags, ',')) AS TagName) AS tags ON TRUE WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'))
// SELECT um.UserId, um.Reputation, um.PostCount, um.QuestionCount, um.AnswerCount, um.TagWikiCount, ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount,
//        ps.AnswerCount AS PostAnswerCount, ps.CommentCount AS PostCommentCount, ps.FavoriteCount AS PostFavoriteCount, ps.RelatedTag
// FROM UserMetrics um JOIN PostStatistics ps ON um.UserId = ps.OwnerUserId ORDER BY um.Reputation DESC, ps.Score DESC;
//
// The LATERAL unnest splits Tags on ','; a NULL Tags gives no element, which the LEFT JOIN keeps as one NULL tag.
fn q13107(db: &'static So) -> String {
    let Post { owner_user, creation_date, tags_str, post_type_id, .. } = &db.post;
    let um = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 4], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(5)) as i64]
    });
    let split = tags_str.opt().flat_map(|t: Option<Str>| match t {
        Some(t) => t.split(',').map(Some).collect::<Vec<Option<Str>>>(),
        None => vec![None],
    });
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.select(Ident::<User>::new().and(&um)).and(split)));
    rows(v.into_iter().map(|(p, ((u, a), t))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites"]));
        f.push(ostr(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount, COALESCE(MAX(v.VoteTypeId), 0) AS MaxVoteType,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, p.PostTypeId),
// TopRankedPosts AS (SELECT * FROM RankedPosts WHERE ScoreRank <= 10),
// PostHistoryAggregated AS (SELECT h.PostId, COUNT(h.Id) AS EditCount, COUNT(CASE WHEN h.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
//        COUNT(CASE WHEN h.PostHistoryTypeId = 19 THEN 1 END) AS ProtectCount FROM PostHistory h GROUP BY h.PostId)
// SELECT tr.PostId, tr.Title, tr.CreationDate, tr.ViewCount, tr.Score, tr.Author, tr.CommentCount, tr.MaxVoteType, pha.EditCount, pha.CloseCount, pha.ProtectCount
// FROM TopRankedPosts tr LEFT JOIN PostHistoryAggregated pha ON tr.PostId = pha.PostId ORDER BY tr.Score DESC, tr.ViewCount DESC;
//
// ScoreRank reads only Score, so the ranked posts are picked first and the comment x vote product is driven for those alone.
fn q7681(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1].max(t.unwrap_or(0))]);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let pha = db.post_history.group_by(post).select(post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 10) as i64, a[2] + (t == 19) as i64]);
    let mut v = drain((&s).and((&pha).opt()));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => h.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionsAsked,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswersProvided, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN P.ViewCount ELSE 0 END), 0) AS TotalViewsOnQuestions,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN P.ViewCount ELSE 0 END), 0) AS TotalViewsOnAnswers, COUNT(DISTINCT C.Id) AS TotalComments, COUNT(DISTINCT B.Id) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionsAsked, AnswersProvided, TotalViewsOnQuestions, TotalViewsOnAnswers, TotalComments, TotalBadges,
//        RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics)
// SELECT UserId, DisplayName, Reputation, QuestionsAsked, AnswersProvided, TotalViewsOnQuestions, TotalViewsOnAnswers, TotalComments, TotalBadges, ReputationRank
// FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Reputation, so the users are picked first and the post x comment x badge product is driven for those alone.
fn q5827(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).collect();
    let tr = rel(v.iter().map(|&((u, _), r)| (u, r)).collect());
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let keys = (&tu).map(|(u, _)| u);
    let s = (&keys)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, _)| match p {
            Some(((t, w), _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + if t == 1 { w.unwrap_or(0) } else { 0 }, a[3] + if t == 2 { w.unwrap_or(0) } else { 0 }],
            None => a,
        });
    let cc = (&keys).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = (&keys).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&s).and(&cc).and(&bc).and((&tu).map(|(_, r)| r))).into_iter().map(|(u, (((a, c), b), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(c), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS RankByScore, COUNT(c.Id) AS CommentCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.Score > 0 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, pt.Name, p.Title, p.Score, p.CreationDate),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.CommentCount FROM RankedPosts rp WHERE rp.RankByScore <= 5),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT tp.Title, tp.Score, tp.CommentCount, ua.DisplayName, ua.UpVotes, ua.DownVotes FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id LEFT JOIN UserActivity ua ON p.OwnerUserId = ua.UserId
// WHERE COALESCE(ua.UpVotes, 0) > COALESCE(ua.DownVotes, 0) ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// RankByScore breaks score ties by post id here; the query's answer is empty whichever posts win.
fn q294(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(score.gt(0).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ua = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&ua)).opt()).filt(|(_, u): (i64, Option<(Id<User>, [i64; 2])>)| u.map_or(false, |(_, a)| a[0] > a[1])));
    rows(v.into_iter().map(|(p, (c, u))| {
        let (u, a) = u.unwrap();
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, u.DisplayName AS OwnerName, p.CreationDate, p.Score, COUNT(a.Id) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score),
// TopRankedPosts AS (SELECT Id, Title, OwnerName, CreationDate, Score, AnswerCount FROM RankedPosts WHERE Rank <= 5),
// VotesSummary AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Votes WHERE PostId IN (SELECT Id FROM TopRankedPosts) GROUP BY PostId)
// SELECT trp.Title, trp.OwnerName, trp.CreationDate, trp.Score, trp.AnswerCount, COALESCE(vs.TotalUpvotes, 0) AS TotalUpvotes, COALESCE(vs.TotalDownvotes, 0) AS TotalDownvotes
// FROM TopRankedPosts trp LEFT JOIN VotesSummary vs ON trp.Id = vs.PostId ORDER BY trp.Score DESC, trp.AnswerCount DESC;
//
// Rank partitions by the post's own id, so it is 1 for every post and the cut keeps them all.
fn q5007(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let ac = qs().group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = qs().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&ac).and(&vs)).into_iter().map(|(p, (n, a))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(b.Id) FILTER (WHERE b.UserId IS NOT NULL) AS BadgeCount, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, ViewCount, Score, CommentCount, UpVoteCount, BadgeCount, Rank FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.*, pt.Name AS PostType, CASE WHEN tp.Score > 100 THEN 'High Performer' WHEN tp.Score BETWEEN 50 AND 100 THEN 'Moderate Performer' ELSE 'Low Performer' END AS PerformanceCategory
// FROM TopPosts tp JOIN PostTypes pt ON EXISTS (SELECT 1 FROM Posts p WHERE p.Id = tp.PostId AND p.PostTypeId = pt.Id);
//
// Rank reads only Score and ViewCount, so the ten questions are picked first and the comment x vote x badge product is driven for those alone.
// The EXISTS is the post's own type.
fn q6017(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(Ident::<Post>::new()));
    let top = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let tp = rel(top.into_iter().enumerate().map(|(i, (p, _))| (p, i as i64 + 1)).collect());
    let ranks: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tp).map(|(p, _)| p).inv().select(&tp).collect();
    let s = (&ranks)
        .map(|(p, _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 3], |a, ((c, t), b)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + b.is_some() as i64]);
    rows(drain((&s).and((&ranks).map(|(_, r)| r)).and(ptype_name(db))).into_iter().map(|(p, ((a, r), t))| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r), V::S(t), V::S(if sc > 100 { "High Performer" } else if sc >= 50 { "Moderate Performer" } else { "Low Performer" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        COUNT(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswerCount FROM Posts P GROUP BY P.OwnerUserId),
// UserBadgeCounts AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT UR.UserId, UR.DisplayName, UR.Reputation, PS.QuestionCount, PS.TotalViews, PS.TotalScore, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(UB.GoldBadges, 0) AS GoldBadges,
//        COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges
// FROM UserReputation UR LEFT JOIN PostStatistics PS ON UR.UserId = PS.OwnerUserId LEFT JOIN UserBadgeCounts UB ON UR.UserId = UB.UserId
// WHERE UR.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY UR.Reputation DESC, PS.TotalViews DESC LIMIT 10;
//
// `Reputation > AVG(Reputation)` is compared exactly as `rep * n > sum`.
fn q1110(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let (n, sum) = (&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 3], |a, ((t, w), s)| [a[0] + (t == 1) as i64, a[1] + w.unwrap_or(0), a[2] + s]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain(db.user.with((&db.user.reputation).filt(move |r| r * n > sum)).select((&ps).opt().and((&ub).opt())));
    let v = top_n(v, |&(u, (p, _))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|a| a[1])), u), 10);
    rows(v.into_iter().map(|(u, (p, b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(match p {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.PostTypeId, p.CreationDate),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.rn <= 10)
// SELECT f.Title, f.Score, f.ViewCount, f.CommentCount, f.VoteCount, f.UpVotes, f.DownVotes,
//        CONCAT('{', '"Score": ', f.Score, ', ', '"ViewCount": ', f.ViewCount, ', ', '"CommentCount": ', f.CommentCount, ', ', '"VoteCount": ', f.VoteCount, ', ', '"UpVotes": ', f.UpVotes, ', ', '"DownVotes": ', f.DownVotes, '}') AS Stats
// FROM FilteredPosts f ORDER BY f.Score DESC;
//
// rn reads only base columns, so each type's ten newest posts are picked first and the comment x vote product is driven for those alone.
fn q5745(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&s).and(&vc)).into_iter().map(|(p, (a, n))| {
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2])]);
        f.push(V::Owned(format!(
            "{{\"Score\": {}, \"ViewCount\": {}, \"CommentCount\": {}, \"VoteCount\": {}, \"UpVotes\": {}, \"DownVotes\": {}}}",
            score.get(p).unwrap(),
            w.map_or(String::new(), |w| w.to_string()),
            a[0],
            n,
            a[1],
            a[2]
        )));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, rp.Rank FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        CASE WHEN tp.Score > 10 THEN 'High Score' ELSE 'Low Score' END AS ScoreCategory
// FROM TopPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// Rank reads only base columns, so each type's ten best posts are picked first and the comment x vote product is driven for those alone.
fn q8882(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).and(owner_user)).into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::S(if score.get(p).unwrap() > 10 { "High Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 AND p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN bh.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges bh ON u.Id = bh.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT up.DisplayName, up.Reputation, COUNT(rp.PostId) AS QuestionsAnswered, SUM(rp.Score) AS TotalScore, SUM(rp.ViewCount) AS TotalViews, us.BadgeCount
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserStats us ON up.Id = us.UserId WHERE rp.Rank <= 5
// GROUP BY up.DisplayName, up.Reputation, us.BadgeCount ORDER BY TotalScore DESC, TotalViews DESC LIMIT 10;
//
// UserStats is read only through the join, so its post x badge product is driven for the owners of the ranked questions alone.
fn q6287(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).opt())).fold(0i64, |n, (_, b)| n + b.is_some() as i64);
    let g = (&tp)
        .group_by(owner_user.select((&db.user.display_name).and(&db.user.reputation).and(&us)))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let v = top_n(drain(&g), |&(k, a)| (Reverse(a[1]), a[2] == 0, Reverse(a[3]), k), 10);
    rows(v.into_iter().map(|(((n, r), b), a)| row(vec![V::S(n), V::I(r), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(b)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, ph.UserId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount FROM PostHistory ph GROUP BY ph.PostId, ph.UserId)
// SELECT us.UserId, u.DisplayName, us.Reputation, us.QuestionCount, us.TotalBounties, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, cp.CloseReopenCount
// FROM UserStats us JOIN Users u ON us.UserId = u.Id LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.OwnerPostRank = 1 LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE us.Reputation >= 1000 ORDER BY us.TotalBounties DESC, us.Reputation DESC;
fn q1634(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let asks = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let users = || db.user.with((&db.user.reputation).ge(1000));
    let us = users().group_by(Ident::<User>::new()).select(asks.select(bounty.opt()).opt()).fold(0i64, |n, b| n + b.flatten().flatten().unwrap_or(0));
    let qc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = rel(first.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&first).map(|(u, _)| u).inv().select(&first).collect();
    let PostHistory { post, user, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.group_by(post.and(user.opt())).select(post_history_type_id).fold(0i64, |n, t| n + (t == 10 || t == 11) as i64);
    let cpv = rel(drain(&cp));
    let cp_of: HashIdx<Id<Post>, ((Id<Post>, Option<Id<User>>), i64)> = (&cpv).map(|((p, _), _)| p).inv().select(&cpv).collect();
    let v = drain((&us).and(&qc).and((&by_user).map(|(_, p)| p).select(Ident::<Post>::new().and((&cp_of).map(|(_, n)| n).opt())).opt()));
    rows(v.into_iter().map(|(u, ((b, q), x))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(q), V::I(b)]);
        match x {
            Some((p, n)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
                f.push(oint(n));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(b.Class) AS TotalBadges FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName ORDER BY TotalBadges DESC LIMIT 10)
// SELECT ru.DisplayName, COUNT(DISTINCT rp.PostId) AS RecentPostsCount, SUM(rp.ViewCount) AS TotalPostViews, SUM(rp.Score) AS TotalScore, SUM(tp.QuestionCount) AS TotalQuestions,
//        SUM(tp.AnswerCount) AS TotalAnswers, AVG(tp.TotalBadges) AS AverageBadges
// FROM RankedPosts rp JOIN TopUsers tp ON rp.PostId = ANY (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = tp.UserId) JOIN Users ru ON tp.UserId = ru.Id
// GROUP BY ru.DisplayName HAVING COUNT(DISTINCT rp.PostId) > 5 ORDER BY TotalPostViews DESC;
//
// `rp.PostId = ANY (posts of tp.UserId)` is the post's owner. UserPostRank is never read.
fn q8468(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (t, c)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + c.is_some() as i64, a[3] + c.unwrap_or(0)]);
    let top = top_n(drain(&tu), |&(u, a)| (a[2] == 0, Reverse(a[3]), u), 10);
    let tv = rel(top);
    let tidx: HashIdx<Id<User>, (Id<User>, [i64; 4])> = (&tv).map(|(u, _)| u).inv().select(&tv).collect();
    let g = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user.select(&tidx))
        .group_by(owner_user.select(&db.user.display_name))
        .select(view_count.opt().and(score).and(owner_user.select(&tidx)))
        .fold([0i64; 7], |s, ((w, sc), (_, a))| [s[0] + 1, s[1] + w.is_some() as i64, s[2] + w.unwrap_or(0), s[3] + sc, s[4] + a[0], s[5] + a[1], s[6] + a[3]]);
    let d = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user.select(&tidx))
        .group_by(owner_user.select(&db.user.display_name))
        .select(Ident::<Post>::new())
        .count_distinct();
    let mut v = drain((&g).and((&d).filt(|n| n > 5)));
    v.sort_by_key(|&(_, (s, _))| (s[1] == 0, Reverse(s[2])));
    rows(v.into_iter().map(|(n, (s, k))| row(vec![V::S(n), V::I(k), nullable(s[2], s[1]), V::I(s[3]), V::I(s[4]), V::I(s[5]), avg(s[6], s[0])])))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(P.ViewCount) AS TotalViews FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.OwnerUserId),
// UserWithPosts AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation, PS.PostCount, PS.UpVotes, PS.DownVotes, PS.TotalViews FROM UserReputation UR LEFT JOIN PostStats PS ON UR.UserId = PS.OwnerUserId)
// SELECT UWP.UserId, UWP.DisplayName, UWP.Reputation, COALESCE(UWP.PostCount, 0) AS TotalPosts, COALESCE(UWP.UpVotes, 0) AS TotalUpVotes, COALESCE(UWP.DownVotes, 0) AS TotalDownVotes,
//        UWP.TotalViews, (COALESCE(UWP.UpVotes, 0) - COALESCE(UWP.DownVotes, 0)) AS NetVotes,
//        CASE WHEN UWP.Reputation >= 1000 THEN 'High Reputation' WHEN UWP.Reputation >= 500 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
// FROM UserWithPosts UWP WHERE UWP.TotalViews >= 1000 AND UWP.UserId IS NOT NULL ORDER BY UWP.Reputation DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q3104(db: &'static So) -> String {
    let Post { owner_user, view_count, .. } = &db.post;
    let ps = db
        .post
        .group_by(owner_user)
        .select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 5], |a, (w, t)| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]);
    let v = drain((&ps).filt(|a| a[3] > 0 && a[4] >= 1000));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[4]), V::I(a[1] - a[2])]);
        f.push(V::S(if r >= 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END), 0) AS TagWikiCount,
//        COALESCE(SUM(P.ViewCount), 0) AS TotalViewCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, QuestionCount, AnswerCount, TotalViewCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY TotalViewCount DESC) AS ViewCountRank FROM UserStats)
// SELECT TU.DisplayName, TU.Reputation, TU.BadgeCount, TU.QuestionCount, TU.AnswerCount, TU.TotalViewCount,
//        CASE WHEN TU.ReputationRank <= 10 THEN 'Top Reputation User' WHEN TU.ViewCountRank <= 10 THEN 'Top Viewed User' ELSE 'Regular User' END AS UserType
// FROM TopUsers TU WHERE TU.QuestionCount > 10 OR TU.AnswerCount > 10 ORDER BY TU.Reputation DESC, TU.TotalViewCount DESC LIMIT 50;
fn q5052(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(view_count.opt())).opt()))
        .fold([0i64; 3], |a, (_, p)| match p {
            Some((t, w)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0)],
            None => a,
        });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&us).and(&bc)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[2]), false);
    let v = drain(rel(v).filt(|x: (((Id<User>, ([i64; 3], i64)), i64), i64)| x.0 .0 .1 .0[0] > 10 || x.0 .0 .1 .0[1] > 10)).into_iter().map(|x| x.1).collect();
    let v = top_n(v, |&(((u, (a, _)), _), _)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[2]), u), 50);
    rows(v.into_iter().map(|(((u, (a, b)), r), w)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if r <= 10 { "Top Reputation User" } else if w <= 10 { "Top Viewed User" } else { "Regular User" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// TopActiveUsers AS (SELECT UserId, DisplayName, PostsCount, QuestionsCount, AnswersCount, UpVotesCount, DownVotesCount, LastPostDate, RANK() OVER (ORDER BY PostsCount DESC) AS UserRank
//     FROM UserActivity WHERE PostsCount > 0)
// SELECT t.UserId, t.DisplayName, t.PostsCount, t.QuestionsCount, t.AnswersCount, t.UpVotesCount, t.DownVotesCount, t.LastPostDate,
//        ROUND(COALESCE(NULLIF(t.UpVotesCount, 0), 1) / NULLIF(t.AnswersCount, 0), 2) AS UpVoteToAnswerRatio, ROUND(COALESCE(NULLIF(t.DownVotesCount, 0), 1) / NULLIF(t.QuestionsCount, 0), 2) AS DownVoteToQuestionRatio
// FROM TopActiveUsers t WHERE t.UserRank <= 10 ORDER BY t.UserRank;
//
// UserRank reads only the distinct post count, so the users are picked first and the post x own-vote product is driven for those alone.
// `v.UserId = u.Id` with `u.Id = p.OwnerUserId` is a vote by the post's owner: `own_votes`.
fn q8696(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&pc).filt(|n| n > 0)), |&(_, n)| Reverse(n), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).collect();
    let tr = rel(v.into_iter().map(|((u, n), r)| (u, (n, r))).collect());
    let tu: HashIdx<Id<User>, (Id<User>, (i64, i64))> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    let ov = own_votes(db);
    let ua = (&tu)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(creation_date).and((&ov).select(&db.vote.vote_type_id).opt())))
        .fold([0i64, 0, 0, 0, i64::MIN], |a, ((t, d), vt)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (vt == Some(2)) as i64, a[3] + (vt == Some(3)) as i64, a[4].max(d)]);
    let round2 = |x: f64| (x * 100.0).round() / 100.0;
    rows(drain((&ua).and((&tu).map(|(_, x)| x))).into_iter().map(|(u, (a, (n, _)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), tmax(a[4])]);
        let up = if a[2] == 0 { 1 } else { a[2] };
        let down = if a[3] == 0 { 1 } else { a[3] };
        f.push(if a[1] == 0 { V::Null } else { V::F(round2(up as f64 / a[1] as f64)) });
        f.push(if a[0] == 0 { V::Null } else { V::F(round2(down as f64 / a[0] as f64)) });
        row(f)
    }))
}

// Rewritten (rewrites/1251.sql): the ROW_NUMBER order is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id) AS Rank,
//        u.Reputation AS UserReputation FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostsWithComments AS (SELECT r.PostId, r.Title, r.CreationDate, r.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, r.UserReputation
//     FROM RankedPosts r LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON r.PostId = c.PostId WHERE r.Rank <= 10),
// TopPosts AS (SELECT p.*, CASE WHEN p.UserReputation >= 1000 THEN 'Expert' WHEN p.UserReputation >= 100 THEN 'Contributor' ELSE 'Novice' END AS UserLevel FROM PostsWithComments p WHERE p.CommentCount > 5)
// SELECT t.Title, t.CreationDate, t.ViewCount, t.CommentCount, t.UserLevel,
//        CASE WHEN t.UserLevel = 'Expert' THEN 'Highly recommended' WHEN t.UserLevel = 'Contributor' THEN 'Moderate recommendation' ELSE 'Not recommended' END AS Recommendation
// FROM TopPosts t ORDER BY t.ViewCount DESC;
fn q1251(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select((&cc).filt(|n| n > 5).and(owner_user.select(&db.user.reputation).opt())));
    rows(v.into_iter().map(|(p, (n, r))| {
        let lvl = match r {
            Some(r) if r >= 1000 => "Expert",
            Some(r) if r >= 100 => "Contributor",
            _ => "Novice",
        };
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        f.extend([V::I(n), V::S(lvl), V::S(match lvl { "Expert" => "Highly recommended", "Contributor" => "Moderate recommendation", _ => "Not recommended" })]);
        row(f)
    }))
}

// Rewritten (rewrites/25228.sql): the final ORDER BY is tie-broken on fp.PostId.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, p.Tags, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Body, p.Tags),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Body, rp.Tags, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.rn = 1)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.CommentCount, fp.UpVoteCount, fp.DownVoteCount, (fp.UpVoteCount - fp.DownVoteCount) AS NetVoteScore,
//        ARRAY_LENGTH(string_to_array(fp.Tags, '><'), 1) AS TagCount,
//        CASE WHEN fp.UpVoteCount > fp.DownVoteCount THEN 'Positive' WHEN fp.UpVoteCount < fp.DownVoteCount THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM FilteredPosts fp ORDER BY NetVoteScore DESC, fp.PostId LIMIT 50;
//
// rn partitions by the post's own id, so it is 1 for every post.
fn q25228(db: &'static So) -> String {
    let Post { creation_date, tags_str, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let voter = |t: i64| votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(t))).select(&db.vote.user);
    let up = recent().group_by(Ident::<Post>::new()).select(voter(2).opt()).buf_fold(distinct_some);
    let down = recent().group_by(Ident::<Post>::new()).select(voter(3).opt()).buf_fold(distinct_some);
    let v = top_n(drain((&cc).and(&up).and(&down)), |&(p, ((_, u), d))| (Reverse(u - d), db.post.origid.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(p, ((c, u), d))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::I(u - d), oint(tags_str.get(p).map(|t| t.split("><").count() as i64))]);
        f.push(V::S(if u > d { "Positive" } else if u < d { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseVotes, RANK() OVER (ORDER BY u.Reputation DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, AcceptedAnswers, CloseVotes, Rank FROM UserStats WHERE Rank <= 10)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.Questions, tu.Answers, tu.AcceptedAnswers, tu.CloseVotes, AVG(v.BountyAmount) AS AverageBountyAmount
// FROM TopUsers tu LEFT JOIN Votes v ON tu.UserId = v.UserId GROUP BY tu.UserId, tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.Questions, tu.Answers, tu.AcceptedAnswers, tu.CloseVotes
// ORDER BY tu.Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the post x history product is driven for those alone.
fn q6859(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some(((t, acc), h)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 1 && acc.is_some()) as i64, a[3] + (h == Some(10)) as i64],
            None => a,
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let vb = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold([0i64; 2], |a, b| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]
    });
    rows(drain((&us).and(&pc).and(&vb)).into_iter().map(|(u, ((a, n), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(avg(b[1], b[0]));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS Downvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS CloseVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > CURRENT_TIMESTAMP - INTERVAL '30 days'),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Upvotes, rp.Downvotes, rp.CloseVotes, (rp.Upvotes - rp.Downvotes) AS NetVotes,
//        RANK() OVER (ORDER BY (rp.Upvotes - rp.Downvotes) DESC) AS VoteRank FROM RecentPosts rp),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment AS ClosureReason FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Upvotes, ps.Downvotes, ps.CloseVotes, ps.NetVotes, ps.VoteRank, cp.ClosureReason
// FROM PostStatistics ps LEFT JOIN ClosedPosts cp ON ps.PostId = cp.PostId WHERE ps.NetVotes > 0 ORDER BY ps.VoteRank LIMIT 10;
//
// RecentPosts has no GROUP BY, so it is one row per post x vote, and VoteRank ranks those rows. The rows are materialised to be ranked.
// CreationDate is compared with CURRENT_TIMESTAMP as a TIMESTAMPTZ in the session zone.
fn q4801(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let since = now_utc() - 30 * DAY_US;
    let recent = || db.post.with(creation_date.filt(move |d| ny_to_utc(d) > since));
    let joined: MatSet<(Id<Post>, Option<Id<Vote>>)> = recent().select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let sums = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(6)) as i64]
    });
    type Row = (Id<Post>, Option<Id<Vote>>);
    let rs = ranked(drain((&joined).select(Same::<Row>::new().map(|r: Row| r.0).select(&sums))), |&(_, a)| Reverse(a[0] - a[1]), false);
    let rs = rel(rs);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    type R2 = ((Row, [i64; 3]), i64);
    let v = drain((&rs).filt(|((_, a), _): R2| a[0] - a[1] > 0).select(Same::<R2>::new().and(Same::<R2>::new().map(|(((p, _), _), _): R2| p).select(closes.opt()))));
    let v = top_n(v, |&(_, ((((p, x), _), r), h))| (r, p, x, h), 10);
    rows(v.into_iter().map(|(_, ((((p, _), a), r), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[0] - a[1]), V::I(r)]);
        f.push(h.map_or(V::Null, |h| ostr(db.post_history.comment.get(h))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Author, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.Author, RP.CreationDate, RP.Score FROM RankedPosts RP WHERE RP.Rank <= 5),
// PostComments AS (SELECT pc.PostId, COUNT(pc.Id) AS CommentCount FROM Comments pc GROUP BY pc.PostId),
// PostVotes AS (SELECT pv.PostId, SUM(CASE WHEN pv.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN pv.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes pv GROUP BY pv.PostId)
// SELECT TP.Title, TP.Author, TP.CreationDate, COALESCE(PC.CommentCount, 0) AS TotalComments, COALESCE(PV.UpVotes, 0) AS TotalUpVotes, COALESCE(PV.DownVotes, 0) AS TotalDownVotes,
//        (COALESCE(PV.UpVotes, 0) - COALESCE(PV.DownVotes, 0)) AS NetScore
// FROM TopPosts TP LEFT JOIN PostComments PC ON TP.PostId = PC.PostId LEFT JOIN PostVotes PV ON TP.PostId = PV.PostId ORDER BY NetScore DESC, TP.CreationDate DESC;
fn q7293(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, p.Body, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND pt.Name = 'Question'),
// TopQuestions AS (SELECT PostRank, PostId, Title, CreationDate, ViewCount, AnswerCount, CommentCount, Body, OwnerDisplayName FROM RankedPosts WHERE PostRank <= 10)
// SELECT tq.PostId, tq.Title, tq.CreationDate, tq.ViewCount, tq.AnswerCount, tq.CommentCount, tq.OwnerDisplayName, COUNT(c.Id) AS TotalComments,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
// FROM TopQuestions tq LEFT JOIN Comments c ON tq.PostId = c.PostId LEFT JOIN Votes v ON tq.PostId = v.PostId
// GROUP BY tq.PostId, tq.Title, tq.CreationDate, tq.ViewCount, tq.AnswerCount, tq.CommentCount, tq.OwnerDisplayName ORDER BY tq.ViewCount DESC;
fn q28393(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(ptype_name(db).filt(|n: Str| n == "Question")).with(owner_user).select(Ident::<Post>::new()));
    let top = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers", "comments", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// Rewritten (rewrites/6630.sql): the final ORDER BY is tie-broken on UserId.
// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(P.Score) AS TotalScore FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UsersWithStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UBC.BadgeCount, 0) AS BadgeCount, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.QuestionCount, 0) AS QuestionCount,
//        COALESCE(PS.AnswerCount, 0) AS AnswerCount, COALESCE(PS.TotalScore, 0) AS TotalScore FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId)
// SELECT UserId, DisplayName, BadgeCount, PostCount, QuestionCount, AnswerCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
// FROM UsersWithStats WHERE BadgeCount > 0 OR PostCount > 0 ORDER BY ScoreRank, TotalScore DESC, UserId LIMIT 50;
fn q6630(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, score, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(post_type_id.and(score))
        .fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let v = drain((&bc).and((&ps).opt()).filt(|(b, p): (i64, Option<[i64; 4]>)| b > 0 || p.map_or(false, |a| a[0] > 0)));
    let v = ranked(v, |&(_, (_, p))| Reverse(p.map_or(0, |a| a[3])), false);
    let v = top_n(v, |&((u, (_, p)), r)| (r, Reverse(p.map_or(0, |a| a[3])), db.user.origid.get(u).unwrap()), 50);
    rows(v.into_iter().map(|((u, (b, p)), r)| {
        let a = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.Score),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(SUM(b.Class), 0) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// TopScoringPosts AS (SELECT rp.Title, ur.DisplayName, rp.Score, rp.CommentCount, RANK() OVER (ORDER BY rp.Score DESC) AS ScoreRank
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.PostRank = 1 AND ur.Reputation > 1000)
// SELECT t.Title AS TopPostTitle, t.DisplayName AS PostOwner, t.Score AS PostScore, t.CommentCount,
//        CASE WHEN t.Score > 100 THEN 'High Score' WHEN t.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM TopScoringPosts t WHERE t.ScoreRank <= 10 ORDER BY t.Score DESC;
//
// PostRank breaks a score tie within an owner by post id; DuckDB gives the same rows at every thread count.
fn q425(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tsp = drain((&first).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).select(score));
    let tsp = ranked(tsp, |&(_, s)| Reverse(s), false);
    let tsp: MatSet<Id<Post>> = rel(tsp.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tsp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain(&cc).into_iter().map(|(p, c)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "score"]);
        f.extend([V::I(c), V::S(if s > 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(b.Class), 0) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT up.DisplayName, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, ur.Reputation, ur.BadgeCount
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserReputation ur ON up.Id = ur.UserId
// WHERE rp.RN = 1 AND (rp.CommentCount > 5 OR rp.UpVoteCount - rp.DownVoteCount > 10) ORDER BY rp.CreationDate DESC LIMIT 10 OFFSET 0;
//
// RN reads only base columns, so each owner's newest post is picked first and the comment x vote product is driven for those alone.
fn q1953(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_months(t0, -1))).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&first)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ur = db
        .user
        .with((&db.user.creation_date).ge(add_months(t0, -6)))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt())
        .fold(0i64, |n, c| n + c.unwrap_or(0));
    let v = drain((&s).filt(|a| a[0] > 5 || a[1] - a[2] > 10).and(owner_user.select(Ident::<User>::new().and(&ur))));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), user_col(db, u, "rep"), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, p.CreationDate, u.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v
//     WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY v.PostId),
// PostHistories AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes, rp.CreationDate, rp.OwnerDisplayName, php.EditCount, php.LastEditDate
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN PostHistories php ON rp.PostId = php.PostId
// WHERE rp.UserRank = 1 ORDER BY rp.ViewCount DESC, rp.CreationDate DESC LIMIT 50;
fn q4675(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { post, vote_type_id, creation_date: vd, .. } = &db.vote;
    let rv = db.vote.with(vd.ge(add_months(t0, -1))).with(post).group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post: hp, creation_date: hd, .. } = &db.post_history;
    let php = db.post_history.group_by(hp).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tp).select((&rv).opt().and((&php).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()), p)
    }, 50);
    rows(v.into_iter().map(|(p, (r, h))| {
        let r = r.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(r[0]), V::I(r[1])]);
        f.extend(post_fields(db, p, &["created", "owner"]));
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.AnswerCount, COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 100),
// PostVotes AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// PopularPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.AnswerCount, rp.OwnerDisplayName, pv.VoteCount, pv.UpVotes, pv.DownVotes
//     FROM RankedPosts rp JOIN PostVotes pv ON rp.PostId = pv.PostId WHERE rp.Rank <= 10)
// SELECT pp.Title, pp.OwnerDisplayName, pp.ViewCount, pp.AnswerCount, pp.VoteCount, pp.UpVotes, pp.DownVotes, EXTRACT(YEAR FROM pp.CreationDate) AS PostYear
// FROM PopularPosts pp ORDER BY pp.ViewCount DESC, pp.AnswerCount DESC;
fn q5755(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(view_count.gt(100)).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(view_count.get(p)), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&pv).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title"]);
        f.push(V::S(owner_user.get(p).map_or("Community", |u| db.user.display_name.get(u).unwrap())));
        f.extend(post_fields(db, p, &["views", "answers"]));
        f.extend(a.map(V::I));
        f.push(V::I(year(creation_date.get(p).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= '2022-01-01' AND p.Score IS NOT NULL),
// CommentCounts AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments FROM Comments c GROUP BY c.PostId),
// TopPostsComments AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(cc.TotalComments, 0) AS CommentCount FROM RankedPosts rp LEFT JOIN CommentCounts cc ON rp.PostId = cc.PostId
//     WHERE rp.Rank <= 5)
// SELECT p.PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.CommentCount,
//        CASE WHEN p.CommentCount = 0 THEN 'No Comments' WHEN p.CommentCount BETWEEN 1 AND 5 THEN 'Few Comments' ELSE 'Many Comments' END AS CommentAvailability,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.PostId AND v.VoteTypeId = 2) AS UpVotes, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.PostId AND v.VoteTypeId = 3) AS DownVotes
// FROM TopPostsComments p ORDER BY p.Score DESC, p.ViewCount DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q21979(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&cc).and(&vc)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::S(if c == 0 { "No Comments" } else if c <= 5 { "Few Comments" } else { "Many Comments" }), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostHistories AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS RevisionCount, MAX(ph.CreationDate) AS LastRevisionDate FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, COALESCE(pv.UpVotes, 0) AS UpVotes, COALESCE(pv.DownVotes, 0) AS DownVotes, ph.RevisionCount, ph.LastRevisionDate,
//        CASE WHEN rp.ScoreRank = 1 THEN 'Top Post' WHEN rp.ScoreRank <= 5 THEN 'High Rank' ELSE 'Normal Rank' END AS RankCategory
// FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId LEFT JOIN PostHistories ph ON rp.PostId = ph.PostId AND ph.PostHistoryTypeId = 24
// WHERE rp.ViewCount > 100 AND (rp.Score > 10 OR ph.RevisionCount > 3) ORDER BY rp.ViewCount DESC, rp.Score DESC LIMIT 50;
fn q34586(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let v = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let rp = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let pv = db.vote.with(&db.vote.post).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.eq(24)).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type R = (Id<Post>, i64);
    let p_of = || Same::<R>::new().map(|(p, _): R| p);
    let v = drain(
        (&rp)
            .with(p_of().select(view_count.gt(100)))
            .select(Same::<R>::new().and(p_of().select((&pv).opt())).and(p_of().select((&ph).opt())))
            .filt(|(((p, _), _), h): ((R, Option<[i64; 2]>), Option<(i64, i64)>)| score.get(p).unwrap() > 10 || h.map_or(false, |(n, _)| n > 3)),
    );
    let v = top_n(v, |&(_, (((p, _), _), _))| (Reverse(view_count.get(p)), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(_, (((p, r), a), h))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if r == 1 { "Top Post" } else if r <= 5 { "High Rank" } else { "Normal Rank" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBounties FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, u.DisplayName AS OwnerDisplayName, rp.CreationDate, rp.Score, us.BadgeCount, us.TotalBounties, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        CASE WHEN rp.Score > 100 THEN 'High Score' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserStats us ON u.Id = us.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId
// WHERE rp.ScoreRank = 1 ORDER BY rp.Score DESC LIMIT 10;
//
// UserStats is read only through the join, so its badge x vote product is driven for the owners of the ranked posts alone.
fn q1268(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&first).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (b, v)| {
            let v = v.flatten();
            [a[0] + b.is_some() as i64, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0)]
        });
    let cc = (&first).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&cc).and(owner_user.select(&us))), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (c, a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(c), V::S(if s > 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ue.UserId, ue.DisplayName, ue.TotalPosts, ue.TotalQuestions, ue.TotalAnswers, ue.TotalUpvotes, ue.TotalDownvotes, ue.TotalBadges,
//        RANK() OVER (ORDER BY ue.TotalPosts DESC) AS PostRank, RANK() OVER (ORDER BY ue.TotalUpvotes DESC) AS UpvoteRank FROM UserEngagement ue)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalUpvotes, tu.TotalDownvotes, tu.TotalBadges, tu.PostRank, tu.UpvoteRank
// FROM TopUsers tu WHERE tu.PostRank <= 10 OR tu.UpvoteRank <= 10 ORDER BY tu.PostRank, tu.UpvoteRank;
fn q9064(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let ue = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, vt) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (vt == Some(2)) as i64, a[3] + (vt == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&ue).and(&pc)), |&(_, (_, n))| Reverse(n), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[2]), false);
    let mut v: Vec<_> = v.into_iter().filter(|&(((_, _), p), u)| p <= 10 || u <= 10).collect();
    v.sort_by_key(|&(((_, _), p), u)| (p, u));
    rows(v.into_iter().map(|(((u, (a, n)), p), r)| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.extend([V::I(p), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.LastActivityDate, p.OwnerUserId, U.DisplayName AS OwnerDisplayName,
//        COUNT(CASE WHEN c.PostId IS NOT NULL THEN 1 END) AS CommentCount, COUNT(a.Id) AS AnswerCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.LastActivityDate, p.OwnerUserId, U.DisplayName),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.PostRank <= 5),
// VoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT tp.PostId, tp.Title, tp.Body, tp.CreationDate, tp.LastActivityDate, tp.OwnerDisplayName, COALESCE(vs.UpVotes, 0) AS TotalUpVotes, COALESCE(vs.DownVotes, 0) AS TotalDownVotes,
//        tp.CommentCount, tp.AnswerCount
// FROM TopPosts tp LEFT JOIN VoteSummary vs ON tp.PostId = vs.PostId ORDER BY tp.CreationDate DESC;
//
// PostRank reads only base columns, so each owner's newest questions are picked first and the comment x answer product is driven for those alone.
fn q28349(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(children_of(db).opt())).fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let vs = db.vote.with(&db.vote.post).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    rows(drain((&s).and((&vs).opt())).into_iter().map(|(p, (a, v))| {
        let v = v.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "activity", "owner"]);
        f.extend([V::I(v[0]), V::I(v[1]), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT u.Id AS UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.UpVotes, ps.DownVotes, RANK() OVER (ORDER BY ps.UpVotes - ps.DownVotes DESC, ps.CreationDate ASC) AS PostRank FROM PostStats ps)
// SELECT rp.Title, rp.CommentCount, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.UpVotes IS NULL THEN 'No Votes Yet' WHEN rp.UpVotes = 0 THEN 'Neutral' ELSE 'Vote Difference: ' || (rp.UpVotes - rp.DownVotes) END AS VoteStatus,
//        (SELECT COUNT(DISTINCT v.UserId) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId IN (2, 3)) AS UniqueVoterCount
// FROM RankedPosts rp WHERE rp.PostRank <= 10 ORDER BY rp.PostRank;
//
// UserVoteCounts is never read.
fn q2369(db: &'static So) -> String {
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = ranked(drain(&ps), |&(p, a)| (Reverse(a[1] - a[2]), db.post.creation_date.get(p).unwrap()), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).collect();
    let tp = rel(v.into_iter().map(|((p, a), r)| (p, (a, r))).collect());
    let top: HashIdx<Id<Post>, (Id<Post>, ([i64; 3], i64))> = (&tp).map(|(p, _)| p).inv().select(&tp).collect();
    let voters = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select(&db.vote.user);
    let uv = (&top).map(|(p, _)| p).group_by(Ident::<Post>::new()).select(voters.opt()).buf_fold(distinct_some);
    rows(drain((&uv).and((&top).map(|(_, x)| x))).into_iter().map(|(p, (n, (a, _)))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(if a[1] == 0 { V::S("Neutral") } else { V::Owned(format!("Vote Difference: {}", a[1] - a[2])) });
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT au.UserId, au.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, au.PostCount, au.TotalScore, au.AvgViewCount,
//        RANK() OVER (ORDER BY au.TotalScore DESC, ub.BadgeCount DESC) AS UserRank FROM ActiveUsers au JOIN UserBadges ub ON au.UserId = ub.UserId)
// SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, PostCount, TotalScore, AvgViewCount, UserRank FROM TopUsers WHERE UserRank <= 10 ORDER BY UserRank;
fn q7948(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let au = db
        .post
        .with(owner_user.select(Ident::<User>::new().with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let v = ranked(drain((&au).and(&ub)), |&(_, (a, b))| (Reverse(a[1]), Reverse(b[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, b)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), V::I(r)]);
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, SUM(CASE WHEN P.Score IS NOT NULL THEN 1 ELSE 0 END) AS TotalPosts, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, AcceptedAnswers, TotalViews, TotalUpvotes, TotalDownvotes, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserMetrics)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.AcceptedAnswers, TU.TotalViews, TU.TotalUpvotes, TU.TotalDownvotes, THH.HistoricalRevisions
// FROM TopUsers TU JOIN (SELECT P.OwnerUserId, COUNT(PH.Id) AS HistoricalRevisions FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.OwnerUserId) THH ON TU.UserId = THH.OwnerUserId
// WHERE TU.UserRank <= 10 ORDER BY TU.UserRank;
//
// UserRank reads only Reputation, so the users are picked first and the post x vote product is driven for those alone.
fn q9210(db: &'static So) -> String {
    let Post { owner_user, accepted_answer_id, view_count, .. } = &db.post;
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let um = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(accepted_answer_id.opt().and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((acc, w), t)) => [a[0] + 1, a[1] + acc.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + (t == Some(2)) as i64, a[4] + (t == Some(3)) as i64],
            None => a,
        });
    let thh = db.post.with(owner_user).group_by(owner_user).select(history_of(db)).fold(0i64, |n, _| n + 1);
    let mut v = drain((&um).and(&thh));
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, (a, h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::I(h));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS Rank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= '2023-01-01'
//     GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName, P.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, UpVotes, DownVotes, CommentCount FROM RankedPosts WHERE Rank <= 5)
// SELECT T.OwnerDisplayName, T.Title, T.CreationDate, T.UpVotes, T.DownVotes, T.CommentCount, COALESCE(PH.Comment, 'No edits made') AS RecentEditComment
// FROM TopPosts T LEFT JOIN PostHistory PH ON T.PostId = PH.PostId AND PH.CreationDate = (SELECT MAX(PH2.CreationDate) FROM PostHistory PH2 WHERE PH2.PostId = T.PostId)
// ORDER BY T.UpVotes DESC, T.CommentCount DESC;
//
// Rank reads only base columns, so each type's five newest posts are picked first and the vote x comment product is driven for those alone.
fn q7044(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    rows(drain((&s).and(Ident::<Post>::new().and(&md).select(&at).opt())).into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["owner", "title", "created"]);
        f.extend(a.map(V::I));
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No edits made")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COALESCE(b.Name, 'No Badge') AS UserBadge, U.Reputation
//     FROM Posts p LEFT JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Badges b ON U.Id = b.UserId AND b.Class = 1 WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteCounts AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Rank, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes, COALESCE(c.CloseCount, 0) AS CloseCount,
//        rp.UserBadge, rp.Reputation
// FROM RankedPosts rp LEFT JOIN PostVoteCounts v ON rp.PostId = v.PostId LEFT JOIN ClosedPosts c ON rp.PostId = c.PostId WHERE rp.Rank <= 5 ORDER BY rp.Rank, rp.CreationDate DESC;
//
// RankedPosts has no GROUP BY, so Rank ranks the post x gold-badge rows: a post with three gold badges takes three places. The rows are materialised to be ranked.
fn q1467(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let joined: MatSet<(Id<Post>, Option<Id<Badge>>)> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(Ident::<Post>::new().and(owner_user.select(gold).opt())).collect();
    let v = ranked(drain(&joined), |&((p, _), _)| (post_type_id.get(p).unwrap(), Reverse(score.get(p).unwrap())), false);
    let v = per_group(v, |&((p, _), _)| post_type_id.get(p).unwrap());
    let rp = rel(v.into_iter().filter(|x| x.1 <= 5).map(|((r, _), k)| (r, k)).collect());
    let pv = db.vote.with(&db.vote.post).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    type R = ((Id<Post>, Option<Id<Badge>>), i64);
    let p_of = || Same::<R>::new().map(|((p, _), _): R| p);
    rows(drain((&rp).select(Same::<R>::new().and(p_of().select((&pv).opt())).and(p_of().select((&cp).opt())))).into_iter().map(|(_, ((((p, b), k), v), c))| {
        let v = v.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(k), V::I(v[0]), V::I(v[1]), V::I(c.unwrap_or(0))]);
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        f.extend(post_fields(db, p, &["rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// UserEngagement AS (SELECT u.Id AS UserId, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, ue.UpVoteCount, ue.DownVoteCount, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u JOIN UserEngagement ue ON u.Id = ue.UserId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.Score, tu.DisplayName AS TopUser, tu.Reputation AS UserReputation
// FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id JOIN TopUsers tu ON p.OwnerUserId = tu.Id WHERE rp.RankScore <= 5 ORDER BY p.PostTypeId, rp.RankScore;
//
// UserEngagement groups Users LEFT JOIN ..., so it has exactly one row per user: the join to it neither filters nor multiplies, and none of its
// columns reach the output, so its comment x vote product is not computed. TopUsers is every user.
fn q5490(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    rows(drain((&tp).select(owner_user)).into_iter().map(|(p, u)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers", "score"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostVoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes FROM Votes v GROUP BY v.PostId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COALESCE(pvc.Upvotes, 0) AS Upvotes, COALESCE(pvc.Downvotes, 0) AS Downvotes, COALESCE(pc.CommentCount, 0) AS CommentCount
// FROM TopPosts tp LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId LEFT JOIN PostComments pc ON tp.PostId = pc.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q8467(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// TopRankedPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, Rank FROM RankedPosts WHERE Rank <= 10),
// PostVoteSummary AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes WHERE PostId IN (SELECT PostId FROM TopRankedPosts) GROUP BY PostId)
// SELECT trp.PostId, trp.Title, trp.OwnerDisplayName, trp.Score, pvs.UpVotes, pvs.DownVotes, pvs.TotalVotes, COALESCE(ROUND(100.0 * pvs.UpVotes / NULLIF(pvs.TotalVotes, 0), 2), 0) AS UpvotePercentage
// FROM TopRankedPosts trp LEFT JOIN PostVoteSummary pvs ON trp.PostId = pvs.PostId ORDER BY trp.Score DESC, trp.PostId;
fn q6938(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvs = db.vote.with((&db.vote.post).select(&tp)).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    rows(drain((&tp).select(Ident::<Post>::new().and((&pvs).opt()))).into_iter().map(|(p, (_, a))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score"]);
        match a {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F((100.0 * a[0] as f64 / a[2] as f64 * 100.0).round() / 100.0)]),
            None => f.extend([V::Null, V::Null, V::Null, V::F(0.0)]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS Downvotes,
//        DENSE_RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS Rank
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - interval '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.OwnerUserId),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT U.DisplayName, RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.Upvotes, RP.Downvotes, UBadges.BadgeCount, UBadges.GoldBadges, UBadges.SilverBadges, UBadges.BronzeBadges
// FROM RankedPosts RP JOIN Users U ON RP.PostId = U.Id LEFT JOIN UserBadges UBadges ON U.Id = UBadges.UserId
// WHERE (RP.Upvotes - RP.Downvotes) > 0 AND UBadges.BadgeCount IS NOT NULL ORDER BY RP.Rank, RP.Score DESC FETCH FIRST 100 ROWS ONLY;
//
// `RP.PostId = U.Id` joins a post id to a user id, so it goes through the raw ids.
fn q4216(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let v = per_group(ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap())), true), |&(_, u)| u);
    let rp = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rk: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let keys = (&rk).map(|(p, _)| p);
    let vc = (&keys).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&vc).filt(|a| a[0] - a[1] > 0).and((&rk).map(|(_, r)| r)).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ub))));
    let v = top_n(v, |&(p, ((_, r), _))| (r, Reverse(score.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((a, _), (u, b)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerName, u.Reputation AS OwnerReputation, COALESCE(v.UpVotes, 0) AS UpVotes,
//        COALESCE(v.DownVotes, 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT bh.Id) AS HistoryCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory bh ON p.Id = bh.PostId WHERE p.ViewCount > 100
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation, v.UpVotes, v.DownVotes),
// Ranking AS (SELECT pd.*, RANK() OVER (ORDER BY pd.Score DESC, pd.ViewCount DESC) AS Rank FROM PostDetails pd)
// SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.OwnerName, r.OwnerReputation, r.UpVotes, r.DownVotes, r.CommentCount, r.HistoryCount, r.Rank
// FROM Ranking r WHERE r.Rank <= 100 ORDER BY r.Rank;
//
// Rank reads only Score and ViewCount, so the posts are ranked first. Both COUNT(DISTINCT) undo their own fan-out, so each is a fold over one child.
fn q5674(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let v = ranked(drain(db.post.with(view_count.gt(100)).select(score.and(view_count))), |&(_, (s, w))| (Reverse(s), Reverse(w)), false);
    let tr = rel(v.into_iter().take_while(|x| x.1 <= 100).map(|((p, _), r)| (p, r)).collect());
    let top: HashIdx<Id<Post>, (Id<Post>, i64)> = (&tr).map(|(p, _)| p).inv().select(&tr).collect();
    let keys = (&top).map(|(p, _)| p);
    let vv = db.vote.with(&db.vote.post).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = (&keys).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&keys).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cc).and(&hc).and((&vv).opt()).and((&top).map(|(_, r)| r))).into_iter().map(|(p, (((c, h), v), r))| {
        let v = v.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep"]);
        f.extend([V::I(v[0]), V::I(v[1]), V::I(c), V::I(h), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '6 months' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5),
// VoteStats AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS Upvotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS Downvotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COALESCE(vs.Upvotes, 0) AS TotalUpvotes, COALESCE(vs.Downvotes, 0) AS TotalDownvotes,
//        CASE WHEN tp.Score > 10 THEN 'Popular' WHEN tp.Score <= 10 AND tp.Score > 5 THEN 'Moderately Popular' ELSE 'Less Popular' END AS PopularityCategory
// FROM TopPosts tp LEFT JOIN VoteStats vs ON tp.PostId = vs.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q7215(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_months(current_date(), -6)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    rows(drain(&vs).into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if s > 10 { "Popular" } else if s > 5 { "Moderately Popular" } else { "Less Popular" })]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 1000),
// ActivePosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate),
// TopPosts AS (SELECT ap.PostId, ap.Title, ap.CreationDate, ap.CommentCount, ap.VoteCount, ap.UpVotes, ap.DownVotes, RANK() OVER (ORDER BY ap.VoteCount DESC) AS PostRank FROM ActivePosts ap WHERE ap.VoteCount > 0)
// SELECT ru.DisplayName, ru.Reputation, tp.Title, tp.CommentCount, tp.VoteCount, tp.UpVotes, tp.DownVotes
// FROM RankedUsers ru JOIN TopPosts tp ON ru.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) WHERE tp.PostRank <= 10 ORDER BY ru.Reputation DESC, tp.VoteCount DESC;
//
// PostRank reads only the distinct vote count, which is the post's own vote count, so the posts are ranked first and the comment x vote product is driven for those alone.
fn q8420(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let vc = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(Ident::<Post>::new()).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let v = ranked(drain(&vc), |&(_, n)| Reverse(n), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&s).and(&vc).and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))));
    rows(v.into_iter().map(|(p, ((a, n), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount, MAX(P.CreationDate) AS LastPostDate FROM Posts P GROUP BY P.OwnerUserId),
// RankedUsers AS (SELECT U.Id, U.DisplayName, B.BadgeCount, P.TotalPosts, P.TotalScore, P.AvgViewCount, ROW_NUMBER() OVER (ORDER BY B.BadgeCount DESC, P.TotalScore DESC) AS UserRank
//     FROM Users U LEFT JOIN UserBadges B ON U.Id = B.UserId LEFT JOIN PostStatistics P ON U.Id = P.OwnerUserId)
// SELECT R.UserRank, R.DisplayName, COALESCE(R.BadgeCount, 0) AS BadgeCount, COALESCE(R.TotalPosts, 0) AS TotalPosts, COALESCE(R.TotalScore, 0) AS TotalScore, COALESCE(R.AvgViewCount, 0) AS AvgViewCount,
//        CASE WHEN R.BadgeCount IS NULL THEN 'No Badges' WHEN R.BadgeCount > 10 THEN 'Veteran User' ELSE 'New User' END AS UserType
// FROM RankedUsers R WHERE R.UserRank <= 10 ORDER BY R.UserRank;
fn q1081(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let v = top_n(drain((&bc).and((&ps).opt())), |&(u, (b, p))| (Reverse(b), p.is_none(), Reverse(p.map(|a| a[1])), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (b, p)))| {
        let a = p.unwrap_or([0; 4]);
        row(vec![V::I(i as i64 + 1), user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[1]), if a[2] == 0 { V::F(0.0) } else { avg(a[3], a[2]) }, V::S(if b > 10 { "Veteran User" } else { "New User" })])
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, UpVotes, DownVotes, CommentCount, BadgeCount, TotalViews, ROW_NUMBER() OVER (ORDER BY TotalViews DESC) AS ViewRank,
//        RANK() OVER (ORDER BY UpVotes DESC) AS UpVoteRank FROM UserActivity)
// SELECT ru.DisplayName, ru.QuestionCount, ru.AnswerCount, ru.UpVotes, ru.DownVotes, ru.CommentCount, ru.BadgeCount, ru.TotalViews
// FROM RankedUsers ru WHERE ru.ViewRank <= 10 OR (ru.UpVoteRank <= 5 AND ru.BadgeCount > 3) ORDER BY ru.TotalViews DESC, ru.UpVotes DESC;
fn q3101(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let per_post = post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt());
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(per_post).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some((((t, w), vt), _)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (vt == Some(2)) as i64, a[3] + (vt == Some(3)) as i64, a[4] + w.unwrap_or(0)],
            None => a,
        });
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&ua).and(&cc).and(&bc));
    let v = ranked(v, |&(u, ((a, _), _))| (Reverse(a[4]), u), false);
    let v = ranked(v, |&((_, ((a, _), _)), _)| Reverse(a[2]), false);
    type R = (((Id<User>, (([i64; 5], i64), i64)), i64), i64);
    let mut v: Vec<R> = drain(rel(v).filt(|(((_, (_, b)), w), r): R| w <= 10 || (r <= 5 && b > 3))).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(((u, ((a, _), _)), _), _)| (Reverse(a[4]), Reverse(a[2]), u));
    rows(v.into_iter().map(|(((u, ((a, c), b)), _), _)| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(c), V::I(b), V::I(a[4])])))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.CreationDate >= '2020-01-01'
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.PostCount, tu.UpVotes, tu.DownVotes, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, p.Title AS LatestPostTitle, p.CreationDate AS LatestPostDate
// FROM TopUsers tu LEFT JOIN Posts p ON tu.UserId = p.OwnerUserId WHERE tu.ReputationRank <= 10 ORDER BY tu.ReputationRank;
//
// ReputationRank reads only Reputation, so the users are picked first and the post x vote x badge product is driven for those alone.
fn q8522(db: &'static So) -> String {
    let v = ranked(drain(db.user.with((&db.user.creation_date).ge(ts(2020, 1, 1, 0, 0, 0))).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, c)| {
            let t = p.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&us).and(&pc).and(posts_of(db).opt())).into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "created"])),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(V.Id) AS TotalVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COALESCE(PH.EditCount, 0) AS EditCount FROM Posts P
//     LEFT JOIN (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId) PH ON P.Id = PH.PostId),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.ViewCount, ROW_NUMBER() OVER (ORDER BY PS.Score DESC, PS.ViewCount DESC) AS Rank FROM PostStats PS
//     WHERE PS.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT UPS.UserId, UPS.DisplayName, TP.Title, TP.Score, TP.ViewCount, UPS.Upvotes, UPS.Downvotes, TP.Rank
// FROM UserVoteSummary UPS JOIN TopPosts TP ON UPS.TotalVotes > 5 WHERE TP.Rank <= 10 ORDER BY UPS.Upvotes DESC, TP.Score DESC;
//
// The ON clause names only UPS, so the voting users are crossed with the ten top posts. EditCount is never read.
fn q1337(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]);
    let top = top_n(drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(Ident::<Post>::new())), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let tp = rel(top.into_iter().enumerate().map(|(i, (p, _))| (p, i as i64 + 1)).collect());
    let uv = rel(drain((&uvs).filt(|a| a[2] > 5)));
    let mut v = Vec::new();
    (&uv).cross(&tp).drive(|_, ((u, a), (p, r))| v.push((u, a, p, r)));
    rows(v.into_iter().map(|(u, a, p, r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.VoteTypeId) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(DISTINCT v.VoteTypeId) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount, MAX(b.Date) AS LastBadgeDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.ViewCount),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.ViewCount, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, RANK() OVER (ORDER BY ps.ViewCount DESC) AS ViewRank,
//        RANK() OVER (ORDER BY ps.UpVoteCount DESC) AS UpVoteRank, RANK() OVER (ORDER BY ps.CommentCount DESC) AS CommentRank FROM PostStats ps)
// SELECT tp.PostId, tp.Title, tp.ViewCount, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount,
//        CASE WHEN tp.ViewRank <= 10 THEN 'Top Viewed' WHEN tp.UpVoteRank <= 10 THEN 'Top Upvoted' WHEN tp.CommentRank <= 10 THEN 'Top Commented' ELSE 'Regular Post' END AS PostCategory
// FROM TopPosts tp ORDER BY tp.ViewRank, tp.UpVoteRank, tp.CommentRank;
fn q9324(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1].max((t == Some(2)) as i64), a[2].max((t == Some(3)) as i64)]);
    let v = ranked(drain(&ps), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[1]), false);
    let v = ranked(v, |&(((_, a), _), _)| Reverse(a[0]), false);
    rows(v.into_iter().map(|((((p, a), w), u), c)| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(if w <= 10 { "Top Viewed" } else if u <= 10 { "Top Upvoted" } else if c <= 10 { "Top Commented" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Date) AS LastBadgeDate FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostMetrics AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId),
// RankedPosts AS (SELECT pm.PostId, pm.OwnerUserId, pm.CommentCount, pm.UpVotes, pm.DownVotes, RANK() OVER (PARTITION BY pm.OwnerUserId ORDER BY pm.UpVotes DESC) AS PostRank FROM PostMetrics pm)
// SELECT u.DisplayName AS UserName, ub.BadgeCount, rp.PostId, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.PostRank,
//        CASE WHEN rp.PostRank <= 3 THEN 'Top Post' WHEN rp.CommentCount > 10 THEN 'Popular Post' ELSE 'Regular Post' END AS PostCategory
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId
// WHERE (ub.BadgeCount IS NULL OR ub.BadgeCount > 0) AND (rp.UpVotes > 5 OR rp.CommentCount > 5) ORDER BY UserName, PostCategory;
fn q930(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let pm = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&pm).and(owner_user.opt()));
    let v = per_group(ranked(v, |&(_, (a, u))| (u, Reverse(a[1])), false), |&(_, (_, u))| u);
    let rp = rel(v.into_iter().map(|((p, (a, _)), r)| (p, (a, r))).collect());
    let by_user: HashIdx<Id<User>, (Id<Post>, ([i64; 3], i64))> = (&rp).map(|(p, _)| p).select(owner_user).inv().select(&rp).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&ub).filt(|n| n > 0).and((&by_user).filt(|(_, (a, _)): (Id<Post>, ([i64; 3], i64))| a[1] > 5 || a[0] > 5)));
    rows(v.into_iter().map(|(u, (b, (p, (a, r))))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(b),
            V::I(db.post.origid.get(p).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(r),
            V::S(if r <= 3 { "Top Post" } else if a[0] > 10 { "Popular Post" } else { "Regular Post" }),
        ])
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, u.Reputation, u.CreationDate,
//        DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// PostSummary AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, p.CommentCount, p.Score, CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Unaccepted' END AS AnswerStatus,
//        p.OwnerUserId FROM Posts p)
// SELECT ua.UserId, ua.DisplayName, ua.PostCount, ua.AnswerCount, ua.Upvotes, ua.Downvotes, ua.Reputation, ua.CreationDate, ps.PostId, ps.Title, ps.ViewCount, ps.AnswerCount AS PostAnswerCount,
//        ps.CommentCount, ps.Score, ps.AnswerStatus
// FROM UserActivity ua JOIN PostSummary ps ON ua.UserId = ps.OwnerUserId WHERE ua.ReputationRank <= 100 ORDER BY ua.Reputation DESC, ps.ViewCount DESC;
//
// ReputationRank reads only Reputation, so the users are picked first and the post x vote product is driven for those alone.
fn q5912(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 100).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((t, vt)) => [a[0] + (t == 2) as i64, a[1] + (vt == Some(2)) as i64, a[2] + (vt == Some(3)) as i64],
            None => a,
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&ua).and(&pc).and(posts_of(db))).into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(ucols(db, u, &["rep", "ucreated"]));
        f.extend(post_fields(db, p, &["id", "title", "views", "answers", "comments", "score"]));
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Accepted" } else { "Unaccepted" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBountyAmount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalQuestions, TotalBountyAmount, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation)
// SELECT U.DisplayName, U.Reputation, U.TotalPosts, U.TotalAnswers, U.TotalQuestions, U.TotalBountyAmount, T.Name AS MostVotedPostType, COUNT(P2.Id) AS RelatedPostsCount
// FROM TopUsers U LEFT JOIN Posts P ON U.UserId = P.OwnerUserId LEFT JOIN PostTypes T ON P.PostTypeId = T.Id LEFT JOIN PostLinks PL ON P.Id = PL.PostId LEFT JOIN Posts P2 ON PL.RelatedPostId = P2.Id
// WHERE U.Rank <= 10 GROUP BY U.UserId, U.DisplayName, U.Reputation, U.TotalPosts, U.TotalAnswers, U.TotalQuestions, U.TotalBountyAmount, T.Name ORDER BY U.Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the post x vote product is driven for those alone.
fn q6980(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let v = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let tb = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).opt()).fold(0i64, |n, b| n + b.flatten().flatten().unwrap_or(0));
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(1)) as i64]);
    let owned = || db.post.with((&db.post.owner_user).select(&tu));
    let g = owned().group_by((&db.post.owner_user).and(ptype_name(db))).select(links_of(db).select(&db.post_link.related_post).opt()).fold(0i64, |n, r| n + r.is_some() as i64);
    let mut v: Vec<(Id<User>, Option<Str>, i64)> = drain(&g).into_iter().map(|((u, t), n)| (u, Some(t), n)).collect();
    v.extend(drain((&tu).minus(posts_of(db))).into_iter().map(|(u, _)| (u, None, 0)));
    let stats = rel(v);
    type R = (Id<User>, Option<Str>, i64);
    let mut v = drain((&stats).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _, _): R| u).select((&tb).and(&pc)))));
    v.sort_by_key(|&(_, ((u, _, _), _))| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(_, ((u, t, n), (b, a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b), ostr(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.Score, p.ViewCount),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount, u.DisplayName AS AuthorDisplayName, u.Reputation, COALESCE(b.GoldCount, 0) AS GoldBadges,
//        COALESCE(b.SilverCount, 0) AS SilverBadges, COALESCE(b.BronzeCount, 0) AS BronzeBadges
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id
// LEFT JOIN (SELECT UserId, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldCount, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverCount, SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
//     FROM Badges GROUP BY UserId) b ON u.Id = b.UserId ORDER BY tp.Score DESC;
//
// Rank reads only Score and ViewCount, so the ten posts are picked first. `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q9003(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let top = top_n(drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).select(Ident::<Post>::new())), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ud = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(ud.opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let b = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    rows(drain((&s).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and((&b).opt())))).into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalScore, ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalScore DESC) AS Rank FROM UserStats),
// ActivePosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month')
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, ap.PostId, ap.Title, ap.CreationDate, ap.CommentCount, ap.VoteCount
// FROM TopUsers tu JOIN ActivePosts ap ON tu.UserId = ap.OwnerUserId WHERE tu.Rank <= 10 ORDER BY tu.Rank, ap.VoteCount DESC;
fn q6214(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let ups = user_posts(db);
    let top = top_n(drain(&ups), |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), a[1] == 0, Reverse(a[4]), u), 10);
    let tu = rel(top.into_iter().enumerate().map(|(i, (u, a))| (u, (a, i as i64 + 1))).collect());
    let tidx: HashIdx<Id<User>, (Id<User>, ([i64; 10], i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let active = || db.post.with(creation_date.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user.select(&tidx));
    let cc = active().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = active().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.user).opt()).buf_fold(distinct_some);
    rows(drain((&cc).and(&vc).and(owner_user.select(&tidx))).into_iter().map(|(p, ((c, n), (u, (a, r))))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("6864", q6864),
    ("1429", q1429),
    ("4082", q4082),
    ("8491", q8491),
    ("32341", q32341),
    ("5812", q5812),
    ("3522", q3522),
    ("27420", q27420),
    ("5544", q5544),
    ("8253", q8253),
    ("8525", q8525),
    ("9140", q9140),
    ("9327", q9327),
    ("311", q311),
    ("1214", q1214),
    ("705", q705),
    ("2379", q2379),
    ("3718", q3718),
    ("9218", q9218),
    ("2649", q2649),
    ("3295", q3295),
    ("543", q543),
    ("4638", q4638),
    ("7734", q7734),
    ("8077", q8077),
    ("9498", q9498),
    ("3644", q3644),
    ("7056", q7056),
    ("3496", q3496),
    ("5021", q5021),
    ("10368", q10368),
    ("5767", q5767),
    ("4327", q4327),
    ("6363", q6363),
    ("913", q913),
    ("448", q448),
    ("1525", q1525),
    ("7474", q7474),
    ("10399", q10399),
    ("13107", q13107),
    ("7681", q7681),
    ("5827", q5827),
    ("294", q294),
    ("5007", q5007),
    ("6017", q6017),
    ("1110", q1110),
    ("5745", q5745),
    ("8882", q8882),
    ("6287", q6287),
    ("1634", q1634),
    ("8468", q8468),
    ("3104", q3104),
    ("5052", q5052),
    ("8696", q8696),
    ("1251", q1251),
    ("25228", q25228),
    ("6859", q6859),
    ("4801", q4801),
    ("7293", q7293),
    ("28393", q28393),
    ("6630", q6630),
    ("425", q425),
    ("1953", q1953),
    ("4675", q4675),
    ("5755", q5755),
    ("21979", q21979),
    ("34586", q34586),
    ("1268", q1268),
    ("9064", q9064),
    ("28349", q28349),
    ("2369", q2369),
    ("7948", q7948),
    ("9210", q9210),
    ("7044", q7044),
    ("1467", q1467),
    ("5490", q5490),
    ("8467", q8467),
    ("6938", q6938),
    ("4216", q4216),
    ("5674", q5674),
    ("7215", q7215),
    ("8420", q8420),
    ("1081", q1081),
    ("3101", q3101),
    ("8522", q8522),
    ("1337", q1337),
    ("9324", q9324),
    ("930", q930),
    ("5912", q5912),
    ("6980", q6980),
    ("9003", q9003),
    ("6214", q6214),
];
