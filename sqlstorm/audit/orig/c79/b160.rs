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


// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION
// BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01
// 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) ), TopRatedPosts AS ( SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount,
// rp.AnswerCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.PostRank <= 5 ), PostComments AS ( SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY
// c.PostId ), PostBadges AS ( SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId ) SELECT tr.PostId, tr.Title, tr.CreationDate, tr.Score,
// tr.ViewCount, tr.AnswerCount, tr.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pb.BadgeCount, 0) AS OwnerBadgeCount FROM TopRatedPosts tr LEFT
// JOIN PostComments pc ON tr.PostId = pc.PostId LEFT JOIN PostBadges pb ON tr.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = pb.UserId) ORDER BY tr.Score
// DESC, tr.CreationDate ASC;
//
// The PostBadges join matches on the owner's display name, so a post meets every badge holder of that name.
fn q6491(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = db.user.with(&bc).select(&db.user.display_name).inv().collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(&db.user.display_name).select((&by_name).select(&bc)).opt()));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        f.extend([V::I(c), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS ( SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges, COUNT(b.Id) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id ), PostActivity AS (
// SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS
// Answers, SUM(CASE WHEN p.PostTypeId IN (10, 11, 12) THEN 1 ELSE 0 END) AS ClosedPosts, SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END) AS TotalScore FROM Posts p GROUP
// BY p.OwnerUserId ), UserPostStats AS ( SELECT u.Id AS UserId, COALESCE(bc.GoldBadges, 0) AS GoldBadges, COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges, COALESCE(pa.TotalPosts, 0) AS TotalPosts, COALESCE(pa.Questions, 0) AS Questions, COALESCE(pa.Answers, 0) AS Answers,
// COALESCE(pa.ClosedPosts, 0) AS ClosedPosts, COALESCE(pa.TotalScore, 0) AS TotalScore FROM Users u LEFT JOIN UserBadgeCounts bc ON u.Id = bc.UserId LEFT JOIN PostActivity
// pa ON u.Id = pa.OwnerUserId ), RankedUsers AS ( SELECT us.*, RANK() OVER (ORDER BY us.TotalScore DESC, us.TotalPosts DESC) AS UserRank FROM UserPostStats us ) SELECT
// ru.UserId, ru.GoldBadges, ru.SilverBadges, ru.BronzeBadges, ru.TotalPosts, ru.Questions, ru.Answers, ru.ClosedPosts, ru.TotalScore, ru.UserRank FROM RankedUsers ru WHERE
// ru.TotalPosts > 0 OR ru.TotalScore > 0 ORDER BY ru.UserRank LIMIT 10;
fn q30785(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, score, .. } = &db.post;
    let ps = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 10 | 11 | 12) as i64, a[4] + s.max(0)],
        None => a,
    });
    let v = ranked(drain((&ub).and(&ps)), |&(_, (_, a))| (Reverse(a[4]), Reverse(a[0])), false);
    type R = ((Id<User>, ([i64; 3], [i64; 5])), i64);
    let ru = rel(v);
    let v = drain((&ru).filt(|((_, (_, a)), _): R| a[0] > 0 || a[4] > 0));
    let v = top_n(v, |&(_, (_, r))| r, 10);
    rows(v.into_iter().map(|(_, ((u, (b, a)), r))| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(b.map(V::I));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserVotingStats AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpvoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END)
// AS DownvoteCount, COUNT(CASE WHEN V.VoteTypeId IN (1, 6, 7) THEN 1 END) AS ActionableVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName
// ), TopQuestions AS ( SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(PH.CreationDate, DATE '1970-01-01') AS MostRecentEdit, COUNT(A.Id) AS AnswerCount,
// RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank FROM Posts P LEFT JOIN Posts A ON P.Id = A.ParentId LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId
// IN (4, 5, 6) WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.Score, PH.CreationDate HAVING COUNT(A.Id) > 0 ), EnhancedPostStats AS ( SELECT TQ.PostId,
// TQ.Title, TQ.CreationDate, TQ.Score, TQ.MostRecentEdit, TQ.AnswerCount, CASE WHEN TQ.ScoreRank <= 10 THEN 'Top' WHEN TQ.ScoreRank BETWEEN 11 AND 20 THEN 'Mid' ELSE 'Low'
// END AS ScoreCategory FROM TopQuestions TQ ) SELECT EPS.Title, EPS.Score, EPS.AnswerCount, UVS.DisplayName, UVS.UpvoteCount, UVS.DownvoteCount, EPS.ScoreCategory,
// COALESCE(EPS.MostRecentEdit, TIMESTAMP '1970-01-01 00:00:00') AS MostRecentEdit FROM EnhancedPostStats EPS JOIN UserVotingStats UVS ON EPS.PostId IN ( SELECT PostId FROM
// Votes WHERE UserId = UVS.UserId ) WHERE UVS.ActionableVotes >= 1 ORDER BY EPS.Score DESC, UVS.UpvoteCount DESC;
fn q808(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let edits = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6])));
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(edits.opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let hd_of = (&j).flat_map(|(_, h)| h).select(hd);
    let tq = (&j).group_by((&post_of).and(hd_of.opt())).select((&post_of).select(children_of(db))).fold(0i64, |n, _| n + 1);
    let v = ranked(drain(&tq), |&((p, _), _)| Reverse(score.get(p).unwrap()), false);
    let eps = rel(v);
    let eidx: HashIdx<Id<Post>, (((Id<Post>, Option<i64>), i64), i64)> = (&eps).map(|(((p, _), _), _)| p).inv().select(&eps).collect();
    let Vote { user, post, vote_type_id, .. } = &db.vote;
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(1 | 6 | 7)) as i64]
    });
    let pairs: MatSet<(Id<User>, Id<Post>)> = db.vote.select(user.and(post)).collect();
    let v = drain((&pairs).select(Same::<(Id<User>, Id<Post>)>::new().map(|(u, _)| u).select((&uvs).filt(|a| a[2] >= 1)).and(Same::<(Id<User>, Id<Post>)>::new().map(|(_, p)| p).select(&eidx))));
    rows(v.into_iter().map(|((u, p), (a, (((_, d), n), r)))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(n), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::S(if r <= 10 { "Top" } else if r <= 20 { "Mid" } else { "Low" }), V::T(d.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RecentPostActivity AS ( SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.LastActivityDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId =
// 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
// p.LastActivityDate DESC) AS UserPostRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP
// '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.LastActivityDate ), UserReputation AS ( SELECT u.Id AS UserId,
// u.Reputation, COUNT(b.Id) AS BadgeCount, AVG(v.BountyAmount) AS AverageBounty FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY u.Id, u.Reputation ), RankedPosts AS ( SELECT rpa.PostId, rpa.Title, rpa.CommentCount, rpa.UpVoteCount, rpa.DownVoteCount, rpa.OwnerUserId, ur.Reputation,
// ur.BadgeCount, ur.AverageBounty, rpa.UserPostRank FROM RecentPostActivity rpa JOIN UserReputation ur ON rpa.OwnerUserId = ur.UserId ) SELECT rp.PostId, rp.Title,
// rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, rp.Reputation, rp.BadgeCount, rp.AverageBounty, CASE WHEN rp.UserPostRank = 1 THEN 'Top Post' WHEN rp.CommentCount > 10
// THEN 'Popular Post' ELSE 'Normal Post' END AS PostCategory FROM RankedPosts rp WHERE rp.UpVoteCount - rp.DownVoteCount > 5 ORDER BY rp.UpVoteCount DESC, rp.CommentCount
// DESC;
//
// UserReputation is only read for the owners of the recent posts, so the badges x votes product is driven for those users alone.
fn q33082(db: &'static So) -> String {
    let Post { creation_date, owner_user, last_activity_date, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let rpa = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let first = top_per(drain(recent().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(last_activity_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let ur = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (b, v)| {
            let x = v.flatten();
            [a[0] + b.is_some() as i64, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0)]
        });
    let v = drain((&rpa).filt(|a| a[1] - a[2] > 5).and(owner_user.select(Ident::<User>::new().and(&ur))).and(Ident::<Post>::new().with(&first).opt()));
    rows(v.into_iter().map(|(p, ((a, (u, r)), top))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "rep"), V::I(r[0]), avg(r[2], r[1])]);
        f.push(V::S(if top.is_some() { "Top Post" } else if a[0] > 10 { "Popular Post" } else { "Normal Post" }));
        row(f)
    }))
}

// WITH UserStats AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u LEFT JOIN Votes v ON
// u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views ), PostMetrics AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(c.CommentCount,
// 0) AS CommentCount, COALESCE(u.Reputation, 0) AS UserReputation, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE
// 0 END) AS DownVotes, RANK() OVER (ORDER BY p.Score DESC) AS PostRank FROM Posts p LEFT JOIN ( SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId ) c ON
// p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN UserStats u ON p.OwnerUserId = u.UserId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, c.CommentCount,
// u.Reputation ), TopPosts AS ( SELECT pm.*, pt.Name AS PostTypeName FROM PostMetrics pm JOIN PostTypes pt ON pm.PostId = pt.Id WHERE pm.PostRank <= 10 ) SELECT tp.Title,
// tp.CreationDate, tp.UpVotes, tp.DownVotes, tp.CommentCount, tp.UserReputation, tp.PostTypeName, CASE WHEN tp.UpVotes > tp.DownVotes THEN 'Positive' WHEN tp.UpVotes <
// tp.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment FROM TopPosts tp WHERE tp.UserReputation > 100 ORDER BY tp.Score DESC
//
// PostRank reads only Score, so the top posts are picked first and the vote product is driven for those alone.
// `pm.PostId = pt.Id` joins a post id to a post-type id, so it goes through the raw ids.
fn q2665(db: &'static So) -> String {
    let Post { score, origid, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.select(score)), |&(_, s)| Reverse(s), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let tidx: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let base = (&tp).with(owner_user.select((&db.user.reputation).gt(100)));
    let s = base
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(&cc).and(origid.select(&tidx)));
    rows(v.into_iter().map(|(p, ((a, c), t))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), post_fields(db, p, &["rep"]).remove(0), V::S(db.post_type.name.get(t).unwrap())]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, array_length(string_to_array(p.Tags, '>'), 1) AS TagCount, u.DisplayName AS
// OwnerDisplayName, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByUser FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId
// = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' ), TopPosts AS ( SELECT PostId, Title, CreationDate, Score, ViewCount, TagCount,
// OwnerDisplayName FROM RankedPosts WHERE RankByUser = 1 ), PostStats AS ( SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.ViewCount, tp.TagCount, COUNT(c.Id)
// AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM TopPosts tp
// LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.ViewCount,
// tp.TagCount ) SELECT ps.PostId, ps.Title, ps.OwnerDisplayName, ps.Score, ps.ViewCount, ps.TagCount, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, CASE WHEN ps.Score
// > 0 THEN 'Positive' WHEN ps.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory, CASE WHEN ps.TagCount > 5 THEN 'Highly Tagged' WHEN ps.TagCount BETWEEN 3 AND 5
// THEN 'Moderately Tagged' ELSE 'Less Tagged' END AS TagCategory FROM PostStats ps ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q26462(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain(&s);
    rows(v.into_iter().map(|(p, a)| {
        let tc = tags_str.get(p).map(|t| t.split('>').count() as i64);
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.push(harness::fmt::oint(tc));
        f.extend(a.map(V::I));
        f.push(V::S(if sc > 0 { "Positive" } else if sc < 0 { "Negative" } else { "Neutral" }));
        f.push(V::S(match tc {
            Some(c) if c > 5 => "Highly Tagged",
            Some(c) if (3..=5).contains(&c) => "Moderately Tagged",
            _ => "Less Tagged",
        }));
        row(f)
    }))
}

// WITH RecursiveCTE AS ( SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC,
// p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1 ), PostHistoryDetails AS ( SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS
// LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId ), TopActiveUsers AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS
// QuestionCount, SUM(v.BountyAmount) AS TotalBounties FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY
// u.Id, u.DisplayName HAVING COUNT(p.Id) > 10 ), RankedPosts AS ( SELECT r.PostId, r.Title, r.Score, r.CreationDate, r.PostRank, COALESCE(pd.EditCount, 0) AS EditCount,
// pd.LastEditDate, tu.DisplayName AS OwnerDisplayName FROM RecursiveCTE r LEFT JOIN PostHistoryDetails pd ON r.PostId = pd.PostId JOIN Users u ON r.OwnerUserId = u.Id JOIN
// TopActiveUsers tu ON u.Id = tu.UserId ) SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.PostRank, rp.EditCount, rp.LastEditDate, rp.OwnerDisplayName, CASE WHEN
// rp.EditCount > 0 THEN 'Edited' ELSE 'Not Edited' END AS EditStatus, (SELECT COUNT(pl.Id) FROM PostLinks pl WHERE pl.PostId = rp.PostId) AS RelatedPostsCount FROM
// RankedPosts rp WHERE rp.Score > 10 ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 100;
fn q31401(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(p, u)| (u, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, u)| u);
    let rr = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rr).map(|(p, _)| p).inv().select(&rr).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pd = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8)));
    let tau = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt())).fold(0i64, |n, _| n + 1);
    let links = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let base = db.post.with(post_type_id.eq(1).and(score.gt(10))).with(owner_user.select((&tau).filt(|n| n > 10)));
    let v = drain(base.select((&rank).map(|(_, r)| r).and((&pd).opt()).and((&links).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((r, e), l))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        let (n, d) = e.unwrap_or((0, i64::MIN));
        f.extend([V::I(r), V::I(n), tmax(d)]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::S(if n > 0 { "Edited" } else { "Not Edited" }), V::I(l.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(p.Score) AS AveragePostScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id,
// u.DisplayName ), TopUsers AS ( SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AveragePostScore, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank
// FROM UserPostStats WHERE TotalPosts > 0 ), UserBadges AS ( SELECT b.UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId ), UserScores
// AS ( SELECT t.UserId, t.DisplayName, t.TotalPosts, t.TotalQuestions, t.TotalAnswers, t.AveragePostScore, COALESCE(ub.TotalBadges, 0) AS TotalBadges,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges FROM TopUsers t LEFT JOIN UserBadges
// ub ON t.UserId = ub.UserId ) SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AveragePostScore, TotalBadges, GoldBadges, SilverBadges, BronzeBadges,
// CASE WHEN TotalPosts > 100 THEN 'High Activity' WHEN TotalPosts BETWEEN 50 AND 100 THEN 'Moderate Activity' ELSE 'Low Activity' END AS ActivityLevel FROM UserScores WHERE
// TotalPosts > 0 ORDER BY TotalPosts DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q32550(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&ups).filt(|a| a[1] > 0).and((&ub).opt()));
    let v = top_n(v, |&(u, (a, _))| (Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1])]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.push(V::S(if a[1] > 100 { "High Activity" } else if a[1] >= 50 { "Moderate Activity" } else { "Low Activity" }));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score
// DESC, p.ViewCount DESC) AS Rank FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) ), UserActivity AS (
// SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN c.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' THEN 1 ELSE 0 END)
// AS RecentComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN
// Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName ), PostStatistics AS ( SELECT
// rp.PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS
// TotalDownVotes FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId GROUP BY rp.PostId ) SELECT rp.PostId, rp.Title,
// rp.CreationDate, rp.Score, rp.ViewCount, ua.DisplayName AS OwnerDisplayName, ua.BadgeCount, ua.RecentComments, ps.CommentCount, ps.TotalUpVotes, ps.TotalDownVotes, CASE
// WHEN rp.Rank <= 5 THEN 'Top 5' ELSE 'Others' END AS PostCategory FROM RankedPosts rp JOIN UserActivity ua ON rp.OwnerUserId = ua.UserId JOIN PostStatistics ps ON
// rp.PostId = ps.PostId ORDER BY rp.Rank;
//
// UserActivity is only read for the owners of the ranked posts, so its badges x comments x votes product is driven for those users alone.
fn q7683(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = || db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.is_in([1, 2])));
    let top = top_per(drain(rp().select(post_type_id)), |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let top5: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = rp().select(owner_user).collect();
    let since = add_months(t0, -1);
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(comments_by(db).select(&db.comment.creation_date).opt()).and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, ((b, c), t)| [a[0] + b.is_some() as i64, a[1] + c.map_or(false, |d| d >= since) as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let ps = rp()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&ps).and(owner_user.select(Ident::<User>::new().and(&ua))).and(Ident::<Post>::new().with(&top5).opt()));
    rows(v.into_iter().map(|(p, ((s, (u, a)), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1])]);
        f.extend(s.map(V::I));
        f.push(V::S(if t.is_some() { "Top 5" } else { "Others" }));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score
// DESC) AS RankScore, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) OVER (PARTITION BY p.Id) AS UpVoteCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) OVER (PARTITION BY
// p.Id) AS DownVoteCount FROM Posts p LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate > (cast('2024-10-01' as date) - INTERVAL '1 year') ), TopQuestions AS (
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.UpVoteCount, rp.DownVoteCount, COALESCE(b.Name, 'No Badge') AS BadgeName, (SELECT COUNT(*) FROM
// Comments c WHERE c.PostId = rp.PostId) AS CommentCount FROM RankedPosts rp LEFT JOIN Badges b ON b.UserId = rp.OwnerUserId AND b.Class = 1 WHERE rp.RankScore <= 10 ),
// RecentActivity AS ( SELECT p.Id AS PostId, ph.UserId, ph.CreationDate, ph.Comment, pt.Name AS HistoryType FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id JOIN
// PostHistoryTypes pt ON pt.Id = ph.PostHistoryTypeId WHERE ph.CreationDate > (cast('2024-10-01' as date) - INTERVAL '30 days') ) SELECT tq.Title, tq.CreationDate,
// tq.ViewCount, tq.Score, tq.UpVoteCount, tq.DownVoteCount, tq.BadgeName, tq.CommentCount, ra.UserId, ra.CreationDate AS RecentActivityDate, ra.Comment AS
// RecentActivityComment, ra.HistoryType FROM TopQuestions tq LEFT JOIN RecentActivity ra ON ra.PostId = tq.PostId ORDER BY tq.Score DESC, tq.ViewCount DESC NULLS LAST,
// ra.CreationDate DESC NULLS FIRST;
//
// RankScore numbers the post x vote rows, not the posts, so the joined rows are ranked; tied rows of one post are broken by vote id, which is not projected.
fn q22053(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let rp = || db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1)));
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = rp().select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    type J = (Id<Post>, Option<Id<Vote>>);
    let v = drain((&j).select(Same::<J>::new().and(Same::<J>::new().map(|(p, _)| p).select(post_type_id))));
    let top = top_per(v, |&(_, (_, t))| t, |&(_, ((p, v), _))| (Reverse(score.get(p).unwrap()), p, v), 10, false);
    let sel = rel(top.into_iter().map(|(_, (x, _))| x).collect());
    let picked: MatSet<Id<Post>> = (&sel).map(|(p, _)| p).collect();
    let ud = (&picked)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&picked).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let gold = owner_user.select(badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))));
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let recent = history_of(db).select(Ident::<PostHistory>::new().with(hd.gt(add_days(date(2024, 10, 1), -30))));
    let v = drain((&sel).select(Same::<J>::new().map(|(p, _)| p).select(Ident::<Post>::new().and(&ud).and(&cc).and(gold.opt()).and(recent.opt()))));
    rows(v.into_iter().map(|(_, ((((p, a), c), b), h))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())), V::I(c)]);
        f.extend(match h {
            Some(h) => [harness::fmt::oint(db.post_history.user_id.get(h)), V::T(hd.get(h).unwrap()), harness::fmt::ostr(db.post_history.comment.get(h)), V::S(htype_name(db).get(h).unwrap())],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, p.Title, p.Body, p.CreationDate, p.AcceptedAnswerId, RANK() OVER (PARTITION BY p.OwnerUserId
// ORDER BY p.CreationDate DESC) AS RankPerUser FROM Posts p ), UserStats AS ( SELECT u.Id AS UserId, (SELECT COUNT(*) FROM Votes v WHERE v.UserId = u.Id AND v.VoteTypeId =
// 2) AS UpVotes, (SELECT COUNT(*) FROM Votes v WHERE v.UserId = u.Id AND v.VoteTypeId = 3) AS DownVotes, (SELECT SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) FROM
// Posts p WHERE p.OwnerUserId = u.Id) AS QuestionCount, (SELECT SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) FROM Posts p WHERE p.OwnerUserId = u.Id) AS AnswerCount,
// u.Reputation FROM Users u ), PostHistoryStats AS ( SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN
// ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount, COUNT(CASE WHEN ph.PostHistoryTypeId IN (24, 25) THEN 1 END) AS EditCount FROM PostHistory ph GROUP BY ph.PostId )
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, u.DisplayName AS OwnerDisplayName, us.Reputation, us.UpVotes, us.DownVotes, us.QuestionCount, us.AnswerCount,
// COALESCE(phs.CloseCount, 0) AS CloseCount, COALESCE(phs.ReopenCount, 0) AS ReopenCount, COALESCE(phs.EditCount, 0) AS EditCount FROM RankedPosts rp JOIN Users u ON
// rp.OwnerUserId = u.Id JOIN UserStats us ON us.UserId = u.Id LEFT JOIN PostHistoryStats phs ON phs.PostId = rp.PostId WHERE (rp.RankPerUser = 1 OR us.Reputation > 1000)
// AND (rp.PostTypeId IN (1, 2) OR us.QuestionCount > 3) ORDER BY us.Reputation DESC, rp.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 100 ROWS ONLY;
fn q24616(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, .. } = &db.post;
    let top = top_per(drain(db.post.with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let first: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let uq = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64]);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + matches!(t, 24 | 25) as i64]);
    type R = ((((Id<User>, [i64; 2]), [i64; 2]), Option<Id<Post>>), i64);
    let v = drain(
        db.post
            .with(owner_user)
            .select(owner_user.select(Ident::<User>::new().and(&uv).and(&uq)).and(Ident::<Post>::new().with(&first).opt()).and(post_type_id))
            .filt(|((((u, _), q), f), t): R| (f.is_some() || db.user.reputation.get(u).unwrap() > 1000) && (t == 1 || t == 2 || q[0] > 3))
            .and((&phs).opt()),
    );
    let v = top_n(v, |&(p, (((((u, _), _), _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (((((u, a), q), _), _), h))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(q[0]), V::I(q[1])]);
        f.extend(h.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS RankByScore,
// COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, (SELECT COUNT(DISTINCT v.UserId) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount, (SELECT
// COUNT(DISTINCT v.UserId) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' ), TopRankedPosts AS ( SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount,
// rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.RankByScore = 1 ), PostHistories AS ( SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserId,
// ph.CreationDate, pht.Name AS HistoryTypeName FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.CreationDate >= cast('2024-10-01' as
// date) - INTERVAL '6 months' AND ph.PostHistoryTypeId IN (10, 12, 13) ) SELECT tp.PostId, tp.Title, tp.CreationDate AS PostCreationDate, tp.Score, tp.ViewCount,
// tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, ph.UserId AS HistorianUserId, ph.HistoryTypeName, ph.CreationDate AS HistoryDate FROM TopRankedPosts tp LEFT JOIN
// PostHistories ph ON tp.PostId = ph.PostId WHERE tp.UpVoteCount > tp.DownVoteCount ORDER BY tp.Score DESC, tp.ViewCount DESC, ph.CreationDate DESC NULLS LAST FETCH FIRST
// 100 ROWS ONLY;
//
// RankByScore numbers the post x comment rows; tied rows of one post are broken by comment id, which is not projected.
fn q20777(db: &'static So) -> String {
    let Post { creation_date, tags_str, score, view_count, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)));
    type J = (Id<Post>, Option<Id<Comment>>);
    let j: MatSet<J> = rp().select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let v = drain((&j).select(Same::<J>::new().map(|(p, _)| p).select(tags_str.opt())));
    let top = top_per(v, |&(_, t)| t, |&((p, c), _)| (Reverse(score.get(p).unwrap()), p, c), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|((p, _), _)| p).collect()).map(|p| p).collect();
    let Vote { vote_type_id, user, .. } = &db.vote;
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vote_type_id.and(user.opt())).opt()).buf_fold(|xs| {
        [distinct_some(xs.iter().map(|x| x.and_then(|(t, u)| u.filter(|_| t == 2)))), distinct_some(xs.iter().map(|x| x.and_then(|(t, u)| u.filter(|_| t == 3))))]
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let ph = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_months(date(2024, 10, 1), -6)).and(post_history_type_id.is_in([10, 12, 13]))));
    let v = drain((&ud).filt(|a| a[0] > a[1]).and(&cc).and(ph.opt()));
    let v = top_n(v, |&(p, (_, h))| {
        let w = view_count.get(p);
        let d = h.map(|h| hd.get(h).unwrap());
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), d.is_none(), Reverse(d), p, h)
    }, 100);
    rows(v.into_iter().map(|(p, ((a, c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [harness::fmt::oint(db.post_history.user_id.get(h)), V::S(htype_name(db).get(h).unwrap()), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeCounts AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN
// B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY
// U.Id, U.DisplayName ), PostStatistics AS ( SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN
// P.PostTypeId = 2 THEN 1 END) AS Answers, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId ), RecentEdits AS ( SELECT
// PH.UserId, PH.PostId, PH.CreationDate, PH.Comment, ROW_NUMBER() OVER (PARTITION BY PH.UserId ORDER BY PH.CreationDate DESC) AS EditRank FROM PostHistory PH WHERE
// PH.PostHistoryTypeId IN (4, 5, 6) ) SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.Questions, 0) AS
// Questions, COALESCE(PS.Answers, 0) AS Answers, COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.AverageScore, 0) AS AverageScore, RE.FirstEditComment AS
// MostRecentEditComment FROM Users U LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId LEFT JOIN (SELECT UserId, Comment
// AS FirstEditComment FROM RecentEdits WHERE EditRank = 1) RE ON U.Id = RE.UserId WHERE U.Reputation > 100 ORDER BY U.DisplayName;
fn q31177(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]
    });
    let PostHistory { user, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let re = top_per(drain(db.post_history.with(post_history_type_id.is_in([4, 5, 6])).select(user)), |&(_, u)| u, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let re = rel(re.into_iter().map(|(h, u)| (u, h)).collect());
    let last: HashIdx<Id<User>, (Id<User>, Id<PostHistory>)> = (&re).map(|(u, _)| u).inv().select(&re).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select((&ub).and((&ps).opt()).and((&last).map(|(_, h)| h).opt())));
    rows(v.into_iter().map(|(u, ((b, a), h))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        let a = a.unwrap_or([0; 6]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[4]), if a[0] == 0 { V::F(0.0) } else { avg(a[5], a[0]) }]);
        f.push(h.map_or(V::Null, |h| harness::fmt::ostr(db.post_history.comment.get(h))));
        row(f)
    }))
}

// WITH UserActivity AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE
// 0 END) AS DownVotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3
// THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT c.Id) AS CommentCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT
// JOIN Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON p.Id = c.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName ), UserScore AS ( SELECT ua.UserId,
// ua.DisplayName, ua.PostCount, ua.QuestionCount, ua.AnswerCount, ua.UpVotes, ua.DownVotes, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges, ua.CommentCount, (ua.UpVotes -
// ua.DownVotes) AS VoteScore, (ua.QuestionCount * 2 + ua.AnswerCount + ua.GoldBadges * 10 + ua.SilverBadges * 5 + ua.BronzeBadges * 2 + ua.CommentCount * 0.5) AS
// PrestigeScore FROM UserActivity ua ), Ranking AS ( SELECT *, RANK() OVER (ORDER BY PrestigeScore DESC) AS Rank FROM UserScore ) SELECT r.Rank, r.DisplayName, r.PostCount,
// r.QuestionCount, r.AnswerCount, r.UpVotes, r.DownVotes, r.GoldBadges, r.SilverBadges, r.BronzeBadges, r.CommentCount, r.VoteScore, r.PrestigeScore FROM Ranking r WHERE
// r.Rank <= 10 ORDER BY r.Rank;
fn q27359(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |((t, v), _)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64]
        });
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let prestige2 = |a: &[i64; 7], c: i64| 2 * (a[0] * 2 + a[1] + a[4] * 10 + a[5] * 5 + a[6] * 2) + c;
    let v = ranked(drain((&ua).and(&np).and(&nc)), |&(_, ((a, _), c))| Reverse(prestige2(&a, c)), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, n), c)), r)| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.extend([V::I(c), V::I(a[2] - a[3]), V::F(prestige2(&a, c) as f64 / 2.0)]);
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN
// B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY
// U.Id, U.DisplayName ), PostMetrics AS ( SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END)
// AS AnswerCount, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId ), UserStats AS ( SELECT U.Id AS UserId, U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PM.QuestionCount, 0) AS QuestionCount, COALESCE(PM.AnswerCount, 0) AS AnswerCount, COALESCE(PM.TotalScore, 0) AS
// TotalScore, COALESCE(PM.TotalViews, 0) AS TotalViews FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostMetrics PM ON U.Id = PM.OwnerUserId ),
// RankedUsers AS ( SELECT US.*, RANK() OVER (ORDER BY US.BadgeCount DESC, US.TotalScore DESC, US.TotalViews DESC) AS UserRank FROM UserStats US ) SELECT RU.UserId,
// RU.DisplayName, RU.BadgeCount, RU.QuestionCount, RU.AnswerCount, RU.TotalScore, RU.TotalViews, RU.UserRank, COALESCE(NULLIF(UB.GoldBadges, 0), NULL) AS
// GoldBadgeIndicator, COALESCE(NULLIF(UB.SilverBadges, 0), NULL) AS SilverBadgeIndicator, COALESCE(NULLIF(UB.BronzeBadges, 0), NULL) AS BronzeBadgeIndicator FROM
// RankedUsers RU LEFT JOIN UserBadges UB ON RU.UserId = UB.UserId WHERE RU.UserRank <= 10 ORDER BY RU.UserRank;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q31548(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let pm = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 4], |a, ((t, s), w)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.unwrap_or(0)]
    });
    let v = ranked(drain((&ub).and((&pm).opt())), |&(_, (b, m))| {
        let m = m.unwrap_or([0; 4]);
        (Reverse(b[0]), Reverse(m[2]), Reverse(m[3]))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (b, m)), r)| {
        let m = m.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b[0]), V::I(m[0]), V::I(m[1]), V::I(m[2]), V::I(m[3]), V::I(r)]);
        f.extend(b[1..].iter().map(|&x| if x == 0 { V::Null } else { V::I(x) }));
        row(f)
    }))
}

// WITH UserReputation AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U ),
// PostStatistics AS ( SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN P.Score < 0 THEN 1 ELSE
// 0 END) AS NegativePosts, AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId ), TopUsers AS ( SELECT UR.UserId, UR.DisplayName, UR.Reputation,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.PositivePosts, 0) AS PositivePosts, COALESCE(PS.NegativePosts, 0) AS NegativePosts, COALESCE(PS.AverageScore, 0) AS
// AverageScore FROM UserReputation UR LEFT JOIN PostStatistics PS ON UR.UserId = PS.OwnerUserId WHERE UR.Reputation > 1000 ), ClosedPostCounts AS ( SELECT PH.UserId,
// COUNT(PH.Id) AS ClosedPostCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId ), FinalStatistics AS ( SELECT TU.UserId, TU.DisplayName,
// TU.Reputation, TU.TotalPosts, TU.PositivePosts, TU.NegativePosts, TU.AverageScore, COALESCE(CPC.ClosedPostCount, 0) AS ClosedPostCount, UR.ReputationRank FROM TopUsers TU
// LEFT JOIN ClosedPostCounts CPC ON TU.UserId = CPC.UserId JOIN UserReputation UR ON TU.UserId = UR.UserId ) SELECT UserId, DisplayName, Reputation, TotalPosts,
// PositivePosts, NegativePosts, ClosedPostCount, CASE WHEN ReputationRank <= 5 THEN 'Top Contributor' WHEN Reputation >= 2000 THEN 'Active Contributor' ELSE 'New
// Contributor' END AS ContributorCategory FROM FinalStatistics WHERE ReputationRank <= 10 ORDER BY Reputation DESC, TotalPosts DESC;
fn q2455(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score).fold([0i64; 4], |a, s| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + s]);
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cpc = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rr = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let rr = rel(rr.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let v = drain((&rr).select(Same::<(Id<User>, i64)>::new().and(Same::<(Id<User>, i64)>::new().map(|(u, _)| u).select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and((&ps).opt()).and((&cpc).opt())))));
    rows(v.into_iter().map(|(_, ((_, r), ((u, a), c)))| {
        let a = a.unwrap_or([0; 4]);
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c.unwrap_or(0))]);
        f.push(V::S(if r <= 5 { "Top Contributor" } else if rep >= 2000 { "Active Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH UserBadges AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class =
// 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id,
// U.DisplayName ), TopUsers AS ( SELECT UserId, DisplayName, TotalBadges, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY TotalBadges DESC) AS BadgeRank FROM
// UserBadges ), ActivePosts AS ( SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId
// = 2 THEN 1 ELSE 0 END) AS Answers, SUM(P.ViewCount) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId ), UserActivity AS ( SELECT U.Id AS UserId, U.DisplayName,
// COALESCE(AP.PostCount, 0) AS PostCount, COALESCE(AP.Questions, 0) AS Questions, COALESCE(AP.Answers, 0) AS Answers, COALESCE(AP.TotalViews, 0) AS TotalViews,
// COALESCE(UB.TotalBadges, 0) AS TotalBadges, RANK() OVER (ORDER BY COALESCE(AP.PostCount, 0) DESC) AS ActivityRank FROM Users U LEFT JOIN ActivePosts AP ON U.Id =
// AP.OwnerUserId LEFT JOIN UserBadges UB ON U.Id = UB.UserId ) SELECT UA.UserId, UA.DisplayName, UA.PostCount, UA.Questions, UA.Answers, UA.TotalViews, UA.TotalBadges,
// TB.GoldBadges, TB.SilverBadges, TB.BronzeBadges, UA.ActivityRank, TB.BadgeRank FROM UserActivity UA JOIN TopUsers TB ON UA.UserId = TB.UserId WHERE UA.TotalViews > 1000
// ORDER BY UA.ActivityRank, TB.BadgeRank LIMIT 10;
fn q26206(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let ap = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 4], |a, (t, w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0)]);
    let v = ranked(drain((&ub).and((&ap).opt())), |&(_, (b, _))| Reverse(b[0]), false);
    let v = ranked(v, |&((_, (_, a)), _)| Reverse(a.map_or(0, |a| a[0])), false);
    type R = (((Id<User>, ([i64; 4], Option<[i64; 4]>)), i64), i64);
    let r = rel(v);
    let v = drain((&r).filt(|(((_, (_, a)), _), _): R| a.map_or(0, |a| a[3]) > 1000));
    let v = top_n(v, |&(_, (((u, _), br), ar))| (ar, br, u), 10);
    rows(v.into_iter().map(|(_, (((u, (b, a)), br), ar))| {
        let a = a.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.extend([V::I(ar), V::I(br)]);
        row(f)
    }))
}

// WITH RecentPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN
// v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT
// JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY
// p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName ), TopUsers AS ( SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS
// TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.CreationDate >=
// cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '90 days' GROUP BY u.Id, u.DisplayName ORDER BY TotalUpVotes DESC LIMIT 10 ), PostStats AS ( SELECT rp.PostId,
// rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes, ROW_NUMBER() OVER (ORDER BY rp.CreationDate DESC) AS Ranking, (SELECT COUNT(*)
// FROM RecentPosts) AS TotalPosts FROM RecentPosts rp ) SELECT ps.PostId, ps.Title, ps.CreationDate, ps.OwnerDisplayName, ps.CommentCount, ps.UpVotes, ps.DownVotes,
// ps.Ranking, ps.TotalPosts, tu.DisplayName AS TopVoter, tu.TotalUpVotes, tu.TotalDownVotes FROM PostStats ps LEFT JOIN TopUsers tu ON ps.OwnerDisplayName = tu.DisplayName
// WHERE ps.UpVotes > 0 ORDER BY ps.Ranking;
fn q5556(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = db
        .post
        .with(creation_date.ge(add_days(t0, -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let total = count(&rp);
    let rk = rel(top_n(drain(&rp), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 0).into_iter().enumerate().map(|(i, (p, _))| (p, i as i64 + 1)).collect());
    let ranking: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let tu = db
        .user
        .with((&db.user.creation_date).ge(add_days(t0, -90)))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let tu = rel(top_n(drain(&tu), |&(u, a)| (Reverse(a[0]), u), 10));
    let by_name: HashIdx<Str, (Id<User>, [i64; 2])> = (&tu).map(|(u, _)| u).select(&db.user.display_name).inv().select(&tu).collect();
    let v = drain((&rp).filt(|a| a[1] > 0).and((&ranking).map(|(_, r)| r)).and(owner_user.select(&db.user.display_name).select(&by_name).opt()));
    rows(v.into_iter().map(|(p, ((a, r), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(total)]);
        f.extend(match t {
            Some((u, b)) => [user_col(db, u, "name"), V::I(b[0]), V::I(b[1])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserVoteStatistics AS ( SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1
// ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT UserId,
// DisplayName, UpVotes, DownVotes, TotalVotes, RANK() OVER (ORDER BY UpVotes DESC) AS Rank FROM UserVoteStatistics ), PostStatistics AS ( SELECT p.Id AS PostId, p.Title,
// p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN
// 1 ELSE 0 END), 0) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.Score, p.ViewCount ),
// TopPosts AS ( SELECT PostId, Title, Score, ViewCount, CommentCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Score DESC) AS Rank FROM PostStatistics ), RecentActivity AS
// ( SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Author, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS ActivityRank FROM
// Posts p JOIN Users u ON p.OwnerUserId = u.Id ) SELECT tu.Rank AS UserRank, tu.DisplayName AS UserName, tp.Rank AS PostRank, tp.Title AS PostTitle, tp.Score AS PostScore,
// tp.ViewCount AS PostViews, ra.CreationDate AS RecentPostDate, ra.Author AS PostAuthor FROM TopUsers tu JOIN TopPosts tp ON tu.UpVotes > 0 JOIN RecentActivity ra ON
// tp.PostId = ra.PostId WHERE tu.Rank <= 10 AND tp.Rank <= 10 AND ra.ActivityRank = 1 ORDER BY tu.Rank, tp.Rank;
//
// TopPosts is ranked on Score alone and the comment x vote aggregates beside it are never read, so they are not computed.
fn q7749(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64);
    let tu = ranked(drain(&uv), |&(_, n)| Reverse(n), false);
    let tu = rel(tu.into_iter().map(|((u, n), r)| (u, n, r)).collect());
    let tu: Vec<_> = drain((&tu).filt(|(_, n, r)| r <= 10 && n > 0));
    let Post { score, owner_user, creation_date, .. } = &db.post;
    let tp = ranked(drain(db.post.select(score)), |&(_, s)| Reverse(s), false);
    let tp = rel(tp.into_iter().map(|((p, _), r)| (p, r)).collect());
    let first = top_per(drain(db.post.with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tp: Vec<_> = drain((&tp).filt(|(_, r)| r <= 10).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _)| p).with(&first))));
    let (a, b) = (rel(tu.into_iter().map(|x| x.1).collect()), rel(tp.into_iter().map(|x| x.1 .0).collect()));
    let v = drain((&a).cross(&b));
    rows(v.into_iter().map(|(_, ((u, _, ur), (p, pr)))| {
        let mut f = vec![V::I(ur), user_col(db, u, "name"), V::I(pr)];
        f.extend(post_fields(db, p, &["title", "score", "views", "created", "owner"]));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
// COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS UpVoteCount, SUM(CASE WHEN
// v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS DownVoteCount FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
// WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' ), UserStatistics AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(CASE
// WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, COUNT(b.Id) AS BadgeCount, AVG(p.ViewCount) AS AvgViews FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id
// LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id, u.DisplayName, u.Reputation ), ClosedPosts AS ( SELECT ph.PostId, ph.CreationDate, PHT.Name AS CloseReason,
// ph.UserDisplayName AS CloserUser FROM PostHistory ph JOIN PostHistoryTypes PHT ON PHT.Id = ph.PostHistoryTypeId WHERE ph.PostHistoryTypeId IN (10, 11) ) SELECT up.UserId,
// up.DisplayName, up.Reputation, up.QuestionCount, up.BadgeCount, up.AvgViews, rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.PostRank, rp.CommentCount,
// rp.UpVoteCount, rp.DownVoteCount, cp.CloseReason, cp.CloserUser FROM UserStatistics up LEFT JOIN RankedPosts rp ON up.UserId = rp.PostId LEFT JOIN ClosedPosts cp ON
// rp.PostId = cp.PostId WHERE NOT EXISTS ( SELECT 1 FROM Votes v WHERE v.UserId = up.UserId AND v.PostId = rp.PostId AND v.VoteTypeId = 3 ) ORDER BY up.Reputation DESC,
// rp.ViewCount DESC NULLS LAST LIMIT 100;
//
// `up.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. RankedPosts is the post x comment x vote rows;
// PostRank numbers them per owner by Score, and rows of one post tie, so the port breaks those ties by post, comment and vote id.
fn q24154(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, origid, view_count, post_type_id, .. } = &db.post;
    type J = ((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>);
    let j: MatSet<J> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt())).collect();
    let v = drain((&j).select(Same::<J>::new().map(|((p, _), _)| p).select(owner_user.opt())));
    let v = ranked(v, |&(((p, c), v), u)| (u, Reverse(score.get(p).unwrap()), p, c, v), false);
    let v = per_group(v, |&(_, u)| u);
    let rr = rel(v.into_iter().map(|((x, _), r)| (x, r)).collect());
    let jp: MatSet<Id<Post>> = (&j).map(|((p, _), _)| p).collect();
    let pc = (&jp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let byorig: HashIdx<i64, (J, i64)> = (&rr).map(|(((p, _), _), _)| p).select(origid).inv().select(&rr).collect();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, w) = p.map_or((0, None), |(t, w)| (t, w));
            [a[0] + (t == 1) as i64, a[1] + b.is_some() as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
        });
    let Vote { user, post, vote_type_id, .. } = &db.vote;
    let down: MatSet<(Id<User>, Option<Id<Post>>)> = db.vote.with(vote_type_id.eq(3)).select(user.and(post.map(Some))).collect();
    let PostHistory { post_history_type_id, .. } = &db.post_history;
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11])));
    type X = (Id<User>, Option<(J, i64)>);
    let joined: MatSet<X> = db.user.select(Ident::<User>::new().and((&db.user.origid).select(&byorig).opt())).collect();
    let key = Same::<X>::new().map(|(u, r): X| (u, r.map(|(((p, _), _), _)| p)));
    let kept = (&joined).select(Same::<X>::new().minus(key.select(&down)));
    let v = drain(kept.select(Same::<X>::new().map(|(u, _): X| u).select(&us).and(Same::<X>::new().map(|(_, r): X| r.map(|(((p, _), _), _)| p)).flat_map(|p: Option<Id<Post>>| p).select((&pc).and(closed.opt())).opt())));
    let v = top_n(v, |&((u, r), (_, x))| {
        let w = r.and_then(|(((p, _), _), _)| view_count.get(p));
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), u, r, x.and_then(|(_, h)| h))
    }, 100);
    rows(v.into_iter().map(|((u, r), (a, x))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[3], a[2])]);
        match (r, x) {
            (Some((((p, _), _), rk)), Some((s, h))) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
                f.push(V::I(rk));
                f.extend(s.map(V::I));
                f.extend(match h {
                    Some(h) => [V::S(htype_name(db).get(h).unwrap()), harness::fmt::ostr(db.post_history.user_display_name.get(h))],
                    None => [V::Null, V::Null],
                });
            }
            _ => f.extend((0..10).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH UserActivity AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN p.PostTypeId = 1 THEN 1
// ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount FROM Users u LEFT
// JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName ), ModeratedPosts AS (
// SELECT p.Id AS PostId, p.Title, ph.CreationDate, pt.Name AS PostType, COUNT(v.Id) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS
// RevisionRank FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (10, 11) JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Votes v ON p.Id
// = v.PostId AND v.VoteTypeId = 2 GROUP BY p.Id, p.Title, ph.CreationDate, pt.Name ), RankedUserActivity AS ( SELECT ua.UserId, ua.DisplayName, ua.PostCount, ua.TotalViews,
// ua.QuestionCount, ua.AnswerCount, ua.CommentCount, ua.BadgeCount, RANK() OVER (ORDER BY ua.PostCount DESC, ua.TotalViews DESC) AS UserRank FROM UserActivity ua ) SELECT
// rua.UserId, rua.DisplayName, rua.PostCount, rua.TotalViews, rua.QuestionCount, rua.AnswerCount, rua.CommentCount, rua.BadgeCount, mp.Title AS LatestModerationTitle,
// mp.PostId AS ModeratedPostId, mp.PostType, mp.VoteCount AS TotalUpvotes, mp.CreationDate AS LastModerationDate FROM RankedUserActivity rua LEFT JOIN ModeratedPosts mp ON
// rua.UserId = mp.PostId WHERE rua.BadgeCount > 0 AND (rua.CommentCount > 5 OR rua.TotalViews > 1000) ORDER BY rua.UserRank, mp.VoteCount DESC NULLS LAST;
//
// `rua.UserId = mp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q222(db: &'static So) -> String {
    let Post { post_type_id, view_count, origid, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, b)| match p {
            Some(((t, w), c)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + c.is_some() as i64, a[5] + b.is_some() as i64],
            None => [a[0], a[1], a[2], a[3], a[4], a[5] + b.is_some() as i64],
        });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let mp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(hd)).select(post.select(up.opt())).fold(0i64, |n, v| n + v.is_some() as i64);
    let mv = rel(drain(&mp));
    let byorig: HashIdx<i64, ((Id<Post>, i64), i64)> = (&mv).map(|((p, _), _)| p).select(origid).inv().select(&mv).collect();
    let v = drain((&ua).filt(|a| a[5] > 0 && (a[4] > 5 || a[1] > 1000)).and((&db.user.origid).select(&byorig).opt()));
    rows(v.into_iter().map(|(u, (a, m))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(match m {
            Some(((p, d), n)) => [post_fields(db, p, &["title"]).remove(0), post_fields(db, p, &["id"]).remove(0), post_fields(db, p, &["type"]).remove(0), V::I(n), V::T(d)],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1
// ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON
// u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate ), PostsStats AS ( SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE
// WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(p.Score) AS AvgScore FROM Posts p GROUP
// BY p.OwnerUserId ), CombinedStats AS ( SELECT us.UserId, us.DisplayName, us.Reputation, us.CreationDate, us.LastAccessDate, us.BadgeCount, ps.TotalPosts,
// ps.TotalQuestions, ps.TotalAnswers, ps.AvgScore, ROW_NUMBER() OVER (ORDER BY us.Reputation DESC) AS Rank FROM UserStats us LEFT JOIN PostsStats ps ON us.UserId =
// ps.OwnerUserId ) SELECT cs.UserId, cs.DisplayName, cs.Reputation, cs.CreationDate, cs.LastAccessDate, cs.BadgeCount, COALESCE(cs.TotalPosts, 0) AS TotalPosts,
// COALESCE(cs.TotalQuestions, 0) AS TotalQuestions, COALESCE(cs.TotalAnswers, 0) AS TotalAnswers, COALESCE(cs.AvgScore, 0) AS AvgScore, cs.Rank, CASE WHEN cs.BadgeCount = 0
// THEN 'No Badges' WHEN cs.BadgeCount <= 3 THEN 'Novice' WHEN cs.BadgeCount <= 10 THEN 'Intermediate' ELSE 'Expert' END AS BadgeLevel FROM CombinedStats cs WHERE
// cs.Reputation > 50 OR cs.TotalPosts > 10 ORDER BY cs.Rank LIMIT 100 OFFSET 0;
fn q4610(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).opt()))
        .fold(0i64, |n, (b, _)| n + b.is_some() as i64);
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let v = top_n(drain((&us).and((&ps).opt())), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 0);
    type R = (i64, (Id<User>, (i64, Option<[i64; 4]>)));
    let cs = rel(v.into_iter().enumerate().map(|(i, x)| (i as i64 + 1, x)).collect());
    let v = drain((&cs).filt(|(_, (u, (_, a))): R| db.user.reputation.get(u).unwrap() > 50 || a.map_or(false, |a| a[0] > 10)));
    let v = top_n(v, |&(_, (r, _))| r, 100);
    rows(v.into_iter().map(|(_, (r, (u, (b, a))))| {
        let a = a.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated", "last_access"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[0] == 0 { V::F(0.0) } else { avg(a[3], a[0]) }, V::I(r)]);
        f.push(V::S(if b == 0 { "No Badges" } else if b <= 3 { "Novice" } else if b <= 10 { "Intermediate" } else { "Expert" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS ( SELECT U.Id AS UserId, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges, COUNT(B.Id)
// FILTER (WHERE B.Class = 3) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id ), PostDetails AS ( SELECT P.Id AS PostId, P.Title,
// P.CreationDate, P.OwnerUserId, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
// FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId ), ClosedPosts
// AS ( SELECT PH.PostId, PH.CreationDate, C.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS INT) = C.Id WHERE PH.PostHistoryTypeId =
// 10 ) SELECT U.DisplayName, U.Reputation, U.CreationDate, U.Location, UBadge.GoldBadges, UBadge.SilverBadges, UBadge.BronzeBadges, PD.Title AS RecentPostTitle,
// PD.CreationDate AS RecentPostDate, PD.TotalBounty, CP.CloseReason AS RecentlyClosedReason, COUNT(DISTINCT CP.PostId) AS TotalClosedPosts FROM Users U LEFT JOIN
// UserBadgeCounts UBadge ON U.Id = UBadge.UserId LEFT JOIN PostDetails PD ON U.Id = PD.OwnerUserId AND PD.PostRank = 1 LEFT JOIN ClosedPosts CP ON PD.PostId = CP.PostId
// WHERE U.Reputation >= 1000 GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.Location, UBadge.GoldBadges, UBadge.SilverBadges, UBadge.BronzeBadges, PD.Title,
// PD.CreationDate, PD.TotalBounty, CP.CloseReason HAVING COUNT(DISTINCT CP.PostId) > 0 ORDER BY U.Reputation DESC, U.CreationDate ASC;
//
// COUNT(DISTINCT CP.PostId) counts the one PostDetails post of the group, so every surviving group has 1.
fn q3014(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let pd = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let pd_of: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&pd).map(|(u, _)| u).inv().select(&pd).collect();
    let pdp: MatSet<Id<Post>> = (&pd).map(|(_, p)| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let tb = (&pdp).group_by(Ident::<Post>::new()).select(bounty.opt()).fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .select(Ident::<PostHistory>::new())
        .fold(0i64, |n, _| n + 1);
    let cv = rel(drain(&cp));
    let cp_of: HashIdx<Id<Post>, ((Id<Post>, Str), i64)> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let v = drain(db.user.with((&db.user.reputation).ge(1000)).select((&ub).and((&pd_of).map(|(_, p)| p).select(Ident::<Post>::new().and(&tb).and((&cp_of).map(|((_, r), _)| r))))));
    rows(v.into_iter().map(|(u, (b, ((p, t), r)))| {
        let mut f = ucols(db, u, &["name", "rep", "ucreated"]);
        f.push(harness::fmt::ostr(db.user.location.get(u)));
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(t), V::S(r), V::I(1)]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostID, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, ARRAY_LENGTH(STRING_TO_ARRAY(p.Tags, '>'), 1) AS TagCount,
// u.Reputation, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY p.CreationDate DESC) AS UserPostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1
// ), TopPosts AS ( SELECT rp.PostID, rp.Title, rp.Body, rp.CreationDate, rp.Score, rp.ViewCount, rp.TagCount, rp.Reputation FROM RankedPosts rp WHERE rp.UserPostRank = 1
// ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10 ), PostVoteCounts AS ( SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId
// = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId ), PostStatistics AS ( SELECT tp.PostID, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, pvc.UpVotes,
// pvc.DownVotes, tp.Reputation / (EXTRACT(EPOCH FROM cast('2024-10-01 12:34:56' as timestamp) - tp.CreationDate)/3600) AS ReputationPerHour FROM TopPosts tp LEFT JOIN
// PostVoteCounts pvc ON tp.PostID = pvc.PostId ) SELECT ps.PostID, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.UpVotes, ps.DownVotes, ps.ReputationPerHour, CASE
// WHEN ps.UpVotes > ps.DownVotes THEN 'Positive' WHEN ps.UpVotes < ps.DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment, CASE WHEN ps.ReputationPerHour > 5 THEN
// 'High' WHEN ps.ReputationPerHour BETWEEN 2 AND 5 THEN 'Medium' ELSE 'Low' END AS ReputationGrowth FROM PostStatistics ps ORDER BY ps.ReputationPerHour DESC;
fn q25522(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = top_n(first, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pvc = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain((&tp).select(owner_user.select(&db.user.reputation).and((&pvc).opt())));
    let rph = |p: Id<Post>, r: i64| r as f64 / (secs(t0 - creation_date.get(p).unwrap()) / 3600.0);
    rows(v.into_iter().map(|(p, (r, a))| {
        let x = rph(p, r);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.push(V::F(x));
        f.push(V::S(match a {
            Some(a) if a[0] > a[1] => "Positive",
            Some(a) if a[0] < a[1] => "Negative",
            _ => "Neutral",
        }));
        f.push(V::S(if x > 5.0 { "High" } else if (2.0..=5.0).contains(&x) { "Medium" } else { "Low" }));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.AnswerCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC)
// AS RankScore, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPost FROM Posts p WHERE p.CreationDate > CURRENT_DATE - INTERVAL '5
// years' ), UserVoteStats AS ( SELECT v.UserId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS
// Downvotes, COUNT(v.Id) AS TotalVotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId ), PostHistoryAgg AS ( SELECT ph.PostId, MAX(CASE WHEN
// pht.Name = 'Post Closed' THEN ph.CreationDate END) AS LastClosedDate, COUNT(*) AS EditCount FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// GROUP BY ph.PostId ), PostComments AS ( SELECT c.PostId, COUNT(c.Id) AS CommentsCount FROM Comments c GROUP BY c.PostId ) SELECT p.PostId, p.Title, p.CreationDate,
// u.DisplayName AS OwnerDisplayName, p.Score, p.AnswerCount, pu.Upvotes, pu.Downvotes, pa.LastClosedDate, pc.CommentsCount, CASE WHEN pa.EditCount > 5 THEN 'Highly Edited'
// WHEN pa.EditCount BETWEEN 2 AND 5 THEN 'Moderately Edited' ELSE 'Rarely Edited' END AS EditFrequencyLabel FROM RankedPosts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN UserVoteStats pu ON u.Id = pu.UserId LEFT JOIN PostHistoryAgg pa ON p.PostId = pa.PostId LEFT JOIN PostComments pc ON p.PostId = pc.PostId WHERE p.RankScore = 1
// AND (p.AnswerCount > 0 OR pu.Upvotes > 10) ORDER BY p.CreationDate DESC;
fn q20865(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, answer_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(creation_date.gt(add_years(current_date(), -5))).select(post_type_id)), |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pu = db.vote.group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let pa = db.post_history.group_by(post).select(htype_name(db).and(hd)).fold((i64::MIN, 0i64), |(m, n), (t, d)| (if t == "Post Closed" { m.max(d) } else { m }, n + 1));
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    type R = (Option<i64>, Option<[i64; 2]>);
    let v = drain((&tp).select(answer_count.opt().and(owner_user.select(&pu).opt())).filt(|(n, u): R| n.map_or(false, |n| n > 0) || u.map_or(false, |u| u[0] > 10)).and((&pa).opt()).and((&pc).opt()));
    rows(v.into_iter().map(|(p, (((_, u), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "answers"]);
        f.extend(match u {
            Some(u) => [V::I(u[0]), V::I(u[1])],
            None => [V::Null, V::Null],
        });
        f.extend([a.map_or(V::Null, |(m, _)| tmax(m)), harness::fmt::oint(c)]);
        f.push(V::S(match a {
            Some((_, n)) if n > 5 => "Highly Edited",
            Some((_, n)) if n >= 2 => "Moderately Edited",
            _ => "Rarely Edited",
        }));
        row(f)
    }))
}

// WITH RecursiveUserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN
// 1 ELSE 0 END) AS TotalDownVotes, ROW_NUMBER() OVER (ORDER BY COUNT(p.Id) DESC) AS UserRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON
// p.Id = v.PostId GROUP BY u.Id, u.DisplayName ), UserRanking AS ( SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, TotalUpVotes, TotalDownVotes,
// UserRank, CASE WHEN PostCount > 100 THEN 'Expert' WHEN PostCount BETWEEN 50 AND 100 THEN 'Pro' ELSE 'Novice' END AS UserLevel FROM RecursiveUserPostStats ), PopularTags
// AS ( SELECT t.TagName, COUNT(p.Id) AS TagPostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(p.Id) > 10 ),
// TopClosedPosts AS ( SELECT p.Id AS PostId, p.Title, COUNT(ph.Id) AS CloseCount FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10 WHERE
// p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY p.Id, p.Title ORDER BY CloseCount DESC LIMIT 5 ) SELECT ur.UserId, ur.DisplayName, ur.PostCount,
// ur.QuestionCount, ur.AnswerCount, ur.TotalUpVotes, ur.TotalDownVotes, ur.UserLevel, tt.TagName, tt.TagPostCount, cp.Title AS ClosedPostTitle, cp.CloseCount AS CloseCount
// FROM UserRanking ur LEFT JOIN PopularTags tt ON ur.UserRank = 1 LEFT JOIN TopClosedPosts cp ON ur.UserRank = 1 ORDER BY ur.UserRank;
//
// The LEFT JOINs on `ur.UserRank = 1` name only ur, so the top user is crossed with PopularTags and TopClosedPosts.
// CURRENT_TIMESTAMP is when the query runs and the data ends in 2024, so TopClosedPosts is empty. The comparison is
// TIMESTAMPTZ, so CreationDate is read as New York local time.
fn q32236(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let rs = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let ranked_users = top_n(drain(&rs), |&(u, a)| (Reverse(a[0]), u), 0);
    let ur = rel(ranked_users.into_iter().enumerate().map(|(i, (u, a))| (i as i64 + 1, u, a)).collect());
    let ts_ = tag_stats(db);
    let pt = rel(drain((&ts_).filt(|a| a[0] > 10)));
    let pt_all: HashIdx<(), (Id<Tag>, [i64; 6])> = (&pt).map(|_| ()).inv().select(&pt).collect();
    let now = now_utc();
    let cc = db
        .post
        .with(creation_date.filt(move |d| ny_to_utc(d) >= now - 30 * DAY_US))
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))))
        .fold(0i64, |n, _| n + 1);
    let cp = rel(top_n(drain(&cc), |&(p, n)| (Reverse(n), p), 5));
    let cp_all: HashIdx<(), (Id<Post>, i64)> = (&cp).map(|_| ()).inv().select(&cp).collect();
    type U = (i64, Id<User>, [i64; 5]);
    let first = || Same::<U>::new().filt(|(r, _, _): U| r == 1).map(|_: U| ());
    let v = drain((&ur).select(Same::<U>::new().and(first().select(&pt_all).opt()).and(first().select(&cp_all).opt())));
    let level = |n: i64| if n > 100 { "Expert" } else if n >= 50 { "Pro" } else { "Novice" };
    rows(v.into_iter().map(|(_, (((_, u, a), t), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[1]), V::I(a[3]), V::I(a[4]), V::S(level(a[0]))]);
        f.extend(match t {
            Some((t, s)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(s[0])],
            None => [V::Null, V::Null],
        });
        f.extend(match c {
            Some((p, n)) => [post_fields(db, p, &["title"]).remove(0), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecursivePostHistory AS ( SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate
// DESC) AS RevisionRank FROM PostHistory ph JOIN Posts p ON p.Id = ph.PostId WHERE ph.CreationDate < cast('2024-10-01 12:34:56' as timestamp) ), AggregatedVotes AS ( SELECT
// v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes FROM Votes v JOIN VoteTypes vt
// ON v.VoteTypeId = vt.Id GROUP BY v.PostId ), ClosedPosts AS ( SELECT p.Id AS PostId, ph.CreationDate AS ClosedDate, u.DisplayName AS ModeratorName FROM Posts p JOIN
// PostHistory ph ON ph.PostId = p.Id AND ph.PostHistoryTypeId IN (10, 11) JOIN Users u ON ph.UserId = u.Id WHERE ph.PostHistoryTypeId = 10 ), RankedPosts AS ( SELECT p.Id,
// p.Title, p.ViewCount, COALESCE(av.UpVotes, 0) AS TotalUpVotes, COALESCE(av.DownVotes, 0) AS TotalDownVotes, COALESCE(cp.ClosedDate, '1970-01-01') AS ClosedDate,
// ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC, p.CreationDate ASC) AS Rank FROM Posts p LEFT JOIN AggregatedVotes av ON p.Id = av.PostId LEFT JOIN ClosedPosts cp ON p.Id =
// cp.PostId ) SELECT rp.Id AS PostId, rp.Title, rp.ViewCount, rp.TotalUpVotes, rp.TotalDownVotes, rp.ClosedDate, CASE WHEN rp.ClosedDate > '1970-01-01' THEN 'Closed' ELSE
// 'Open' END AS PostStatus, CASE WHEN rp.Rank <= 10 THEN 'Top 10' ELSE 'Other' END AS RankGroup FROM RankedPosts rp WHERE rp.TotalDownVotes < rp.TotalUpVotes AND rp.Rank <=
// 20 ORDER BY rp.Rank, rp.ViewCount DESC;
fn q21896(db: &'static So) -> String {
    let Post { view_count, creation_date, .. } = &db.post;
    let av = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let PostHistory { post_history_type_id, user, creation_date: hd, .. } = &db.post_history;
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).with(user));
    let v = drain(db.post.select(closed.opt()));
    let v = top_n(v, |&(p, h)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), creation_date.get(p).unwrap(), p, h)
    }, 20);
    type R = (i64, (Id<Post>, Option<Id<PostHistory>>));
    let rp = rel(v.into_iter().enumerate().map(|(i, x)| (i as i64 + 1, x)).collect());
    let v = drain((&rp).select(Same::<R>::new().and(Same::<R>::new().map(|(_, (p, _)): R| p).select((&av).opt()))).filt(|(_, a): (R, Option<[i64; 2]>)| {
        let a = a.unwrap_or([0; 2]);
        a[1] < a[0]
    }));
    rows(v.into_iter().map(|(_, ((r, (p, h)), a))| {
        let a = a.unwrap_or([0; 2]);
        let d = h.map_or(0, |h| hd.get(h).unwrap());
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::T(d), V::S(if d > 0 { "Closed" } else { "Open" }), V::S(if r <= 10 { "Top 10" } else { "Other" })]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY
// p.PostTypeId ORDER BY p.Score DESC) AS Rank, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY
// p.Id) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS DownVoteCount, MAX(CASE WHEN v.VoteTypeId = 10 THEN 1 ELSE 0 END) OVER
// (PARTITION BY p.Id) AS IsDeleted FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE
// p.CreationDate >= '2023-01-01' AND (p.Score > 10 OR p.ViewCount > 100) ), PostActivity AS ( SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.Score,
// rp.ViewCount, rp.Rank, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END), 0) AS
// CloseReopenCount, MAX(rp.IsDeleted) AS IsDeleted FROM RankedPosts rp LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId GROUP BY rp.PostId, rp.Title, rp.CreationDate,
// rp.OwnerDisplayName, rp.Score, rp.ViewCount, rp.Rank, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount ) SELECT pa.PostId, pa.Title, pa.CreationDate,
// pa.OwnerDisplayName, pa.Score, pa.ViewCount, pa.Rank, pa.CommentCount, pa.UpVoteCount, pa.DownVoteCount, pa.CloseReopenCount, CASE WHEN pa.IsDeleted = 1 THEN 'Deleted'
// ELSE 'Active' END AS PostStatus FROM PostActivity pa WHERE pa.Rank <= 10 ORDER BY pa.Score DESC, pa.ViewCount DESC;
//
// Rank numbers the post x comment x vote rows, and rows of one post tie on Score, so the port breaks those ties by post,
// comment and vote id; the tied rows differ only in the rank.
fn q30982(db: &'static So) -> String {
    let Post { creation_date, score, view_count, post_type_id, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).with(score.gt(10).or(view_count.gt(100)));
    type J = ((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>);
    let j: MatSet<J> = base().select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt())).collect();
    let v = drain((&j).select(Same::<J>::new().map(|((p, _), _)| p).select(post_type_id)));
    let v = ranked(v, |&(((p, c), v), t)| (t, Reverse(score.get(p).unwrap()), p, c, v), false);
    let v = per_group(v, |&(_, t)| t);
    type R = ((J, i64), i64);
    let rr = rel(v);
    let top: Vec<_> = drain((&rr).filt(|(_, r): R| r <= 10));
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.1 .0 .0 .0 .0).collect()).map(|p| p).collect();
    let pw = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3].max((t == Some(10)) as i64)]);
    let cr = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold(0i64, |n, t| n + matches!(t, Some(10 | 11)) as i64);
    let sel = rel(top.into_iter().map(|(_, ((((p, _), _), _), r))| (p, r)).collect());
    let v = drain((&sel).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _)| p).select((&pw).and(&cr)))));
    rows(v.into_iter().map(|(_, ((p, r), (a, c)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.extend([V::I(r), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c), V::S(if a[3] == 1 { "Deleted" } else { "Active" })]);
        row(f)
    }))
}

// WITH UserStats AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(P.ViewCount, 0)) AS
// TotalViews, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT BA.Id) AS BadgeCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id
// = C.PostId LEFT JOIN Badges BA ON U.Id = BA.UserId GROUP BY U.Id, U.DisplayName ), PostStats AS ( SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount,
// COUNT(C.Id) AS CommentCount, COUNT(DISTINCT V.Id) AS VoteCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id,
// P.Title, P.CreationDate, P.Score, P.ViewCount ), TopUsers AS ( SELECT UserId, DisplayName, PostCount, TotalScore, TotalViews, CommentCount, BadgeCount, RANK() OVER (ORDER
// BY TotalScore DESC) AS ScoreRank FROM UserStats ), TopPosts AS ( SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, VoteCount, RANK() OVER (ORDER BY
// Score DESC) AS ScoreRank FROM PostStats ) SELECT TU.UserId, TU.DisplayName, TU.PostCount, TU.TotalScore, TU.TotalViews, TU.CommentCount, TU.BadgeCount, TP.PostId,
// TP.Title AS PostTitle, TP.CreationDate AS PostCreationDate, TP.Score AS PostScore, TP.ViewCount AS PostViewCount, TP.CommentCount AS PostCommentCount, TP.VoteCount AS
// PostVoteCount FROM TopUsers TU JOIN TopPosts TP ON TU.UserId = (SELECT OwnerUserId FROM Posts ORDER BY Score DESC LIMIT 1) WHERE TU.ScoreRank <= 10 AND TP.ScoreRank <=
// 10;
//
// TopPosts' ScoreRank reads only Score, so the top posts are picked first and their comment x vote aggregates computed for those alone.
fn q10502(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| match p {
            Some(((s, w), _)) => [a[0] + s, a[1] + w.unwrap_or(0)],
            None => a,
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let nb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tu = ranked(drain((&us).and(&np).and(&nc).and(&nb)), |&(_, (((a, _), _), _))| Reverse(a[0]), false);
    let tu = rel(tu.into_iter().map(|(x, r)| (x, r)).collect());
    let owner = top_n(drain(db.post.select(owner_user.opt())), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1).first().and_then(|x| x.1);
    type T = ((Id<User>, ((([i64; 2], i64), i64), i64)), i64);
    let tu = drain((&tu).filt(move |((u, _), r): T| r <= 10 && Some(u) == owner));
    let tp = ranked(drain(db.post.select(score)), |&(_, s)| Reverse(s), false);
    let tp: MatSet<Id<Post>> = rel(tp.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let ps = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let tpv = rel(drain((&ps).and(&pv)));
    let tur = rel(tu.into_iter().map(|x| x.1).collect());
    let v = drain((&tur).cross(&tpv));
    rows(v.into_iter().map(|(_, (((u, (((a, n), c), b)), _), (p, (pc, pv))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(c), V::I(b)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(pc), V::I(pv)]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u
// ), TopUsers AS ( SELECT UserId, DisplayName, Reputation FROM UserReputation WHERE Rank <= 10 ), PostVotes AS ( SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN
// 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY p.Id ), FilteredPosts AS ( SELECT p.Id, p.Title, p.CreationDate, p.Score, pv.UpVotes, pv.DownVotes, u.DisplayName AS OwnerDisplayName FROM Posts p JOIN PostVotes
// pv ON p.Id = pv.PostId JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND pv.TotalVotes > 0 ),
// PostDetails AS ( SELECT fp.Id, fp.Title, fp.CreationDate, fp.Score, fp.UpVotes, fp.DownVotes, fp.OwnerDisplayName, CASE WHEN fp.UpVotes - fp.DownVotes < 0 THEN 'Negative'
// WHEN fp.UpVotes - fp.DownVotes = 0 THEN 'Neutral' ELSE 'Positive' END AS VoteSentiment FROM FilteredPosts fp ) SELECT tu.DisplayName AS TopUser, pd.Title AS PostTitle,
// pd.Score, pd.UpVotes, pd.DownVotes, pd.VoteSentiment, CASE WHEN pd.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 'Archived' ELSE
// 'Active' END AS PostStatus FROM PostDetails pd CROSS JOIN TopUsers tu WHERE pd.Score > 10 ORDER BY tu.Reputation DESC, pd.Score DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q31216(db: &'static So) -> String {
    let tu = rel(top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10));
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let pv = db
        .post
        .with(creation_date.ge(add_years(t0, -1)).and(score.gt(10)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]);
    let pd = rel(drain((&pv).filt(|a| a[2] > 0)));
    let v = drain((&pd).cross(&tu));
    rows(v.into_iter().map(|(_, ((p, a), (u, _)))| {
        let d = a[0] - a[1];
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if d < 0 { "Negative" } else if d == 0 { "Neutral" } else { "Positive" })]);
        f.push(V::S(if creation_date.get(p).unwrap() < add_days(t0, -30) { "Archived" } else { "Active" }));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, COALESCE(A.AnswerCount, 0) AS AnswerCount, COALESCE(V.VoteCount, 0) AS VoteCount,
// ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId FROM Posts p LEFT JOIN ( SELECT ParentId, COUNT(*) AS
// AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId ) A ON p.Id = A.ParentId LEFT JOIN ( SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId ) V ON
// p.Id = V.PostId WHERE p.PostTypeId = 1 ), UserStats AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(COALESCE(A.AnswerCount, 0)) AS TotalAnswers, COALESCE(SUM(V.VoteCount), 0) AS TotalVotes FROM Users U LEFT JOIN Posts p ON U.Id = p.OwnerUserId LEFT JOIN ( SELECT
// ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId ) A ON p.Id = A.ParentId LEFT JOIN ( SELECT PostId, COUNT(*) AS VoteCount FROM Votes
// GROUP BY PostId ) V ON p.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation ) SELECT R.PostId, R.Title, R.CreationDate, R.Body, R.AnswerCount, R.VoteCount,
// U.DisplayName AS OwnerDisplayName, U.Reputation AS OwnerReputation, U.TotalPosts, U.TotalAnswers, U.TotalVotes FROM RankedPosts R JOIN UserStats U ON R.OwnerUserId =
// U.UserId WHERE R.UserPostRank <= 5 ORDER BY U.TotalVotes DESC, R.CreationDate DESC;
fn q27768(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let ac = db.post.group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&ac).and(&vc)).opt()).fold([0i64; 3], |a, x| match x {
        Some((n, v)) => [a[0] + 1, a[1] + n, a[2] + v],
        None => a,
    });
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&tp).select((&ac).and(&vc).and(owner_user.select(Ident::<User>::new().and(&us)))));
    rows(v.into_iter().map(|(p, ((n, c), (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "body"]);
        f.extend([V::I(n), V::I(c)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE
// WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN
// v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id,
// u.DisplayName ), UserBadges AS ( SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId ), UserPerformance AS ( SELECT ups.UserId, ups.DisplayName, ups.TotalPosts,
// ups.TotalQuestions, ups.TotalAnswers, ups.TotalUpvotes, ups.TotalDownvotes, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, ROW_NUMBER() OVER (ORDER BY ups.TotalUpvotes DESC) AS Rank FROM UserPostStats ups LEFT JOIN UserBadges ub ON ups.UserId =
// ub.UserId ) SELECT up.UserId, up.DisplayName, up.TotalPosts, up.TotalQuestions, up.TotalAnswers, up.TotalUpvotes, up.TotalDownvotes, up.GoldBadges, up.SilverBadges,
// up.BronzeBadges, up.Rank, ROUND((up.TotalUpvotes::numeric / NULLIF(up.TotalPosts, 0)) * 100, 2) AS UpvotePercentage, CASE WHEN up.Rank <= 10 THEN 'Top Contributor' ELSE
// 'Regular Contributor' END AS ContributorStatus FROM UserPerformance up WHERE up.TotalPosts > 0 ORDER BY up.Rank FETCH FIRST 50 ROWS ONLY;
fn q1619(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = top_n(drain((&ups).and((&ub).opt())), |&(u, (a, _))| (Reverse(a[3]), u), 0);
    type R = (i64, (Id<User>, ([i64; 5], Option<[i64; 3]>)));
    let up = rel(v.into_iter().enumerate().map(|(i, x)| (i as i64 + 1, x)).collect());
    let v = top_n(drain((&up).filt(|(_, (_, (a, _))): R| a[0] > 0)), |&(_, (r, _))| r, 50);
    rows(v.into_iter().map(|(_, (r, (u, (a, b))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend([V::I(r), V::F((a[3] as f64 / a[0] as f64 * 100.0 * 100.0).round() / 100.0), V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" })]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName AS Author, COUNT(a.Id) AS AnswerCount, COALESCE(SUM(CASE WHEN
// v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, ROW_NUMBER() OVER (PARTITION BY p.Tags
// ORDER BY COUNT(a.Id) DESC) AS TagRank FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v
// ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName ), FilteredPosts AS ( SELECT PostId, Title, Body, Tags,
// CreationDate, Author, AnswerCount, Upvotes, Downvotes FROM RankedPosts WHERE TagRank <= 5 ), PostDetails AS ( SELECT fp.*, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE
// WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseCount, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END), 0) AS ReopenCount FROM
// FilteredPosts fp LEFT JOIN Comments c ON fp.PostId = c.PostId LEFT JOIN PostHistory ph ON fp.PostId = ph.PostId GROUP BY fp.PostId, fp.Title, fp.Body, fp.Tags,
// fp.CreationDate, fp.Author, fp.AnswerCount, fp.Upvotes, fp.Downvotes ) SELECT pd.PostId, pd.Title, pd.Body, pd.Tags, pd.CreationDate, pd.Author, pd.AnswerCount,
// pd.Upvotes, pd.Downvotes, pd.CommentCount, pd.CloseCount, pd.ReopenCount, CASE WHEN pd.CloseCount > 0 THEN 'Closed' ELSE 'Open' END AS PostStatus FROM PostDetails pd
// ORDER BY pd.Upvotes DESC, pd.AnswerCount DESC FETCH FIRST 10 ROWS ONLY;
fn q25005(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (x, t)| [a[0] + x.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let top = top_per(drain((&rp).and(tags_str.opt())), |&(_, (_, t))| t, |&(p, (a, _))| (Reverse(a[0]), p), 5, false);
    let fp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pd = (&fp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(10)) as i64, a[2] + (t == Some(11)) as i64]);
    let v = top_n(drain((&pd).and(&rp)), |&(p, (_, a))| (Reverse(a[1]), Reverse(a[0]), p), 10);
    rows(v.into_iter().map(|(p, (d, a))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "owner"]);
        f.extend(a.map(V::I));
        f.extend(d.map(V::I));
        f.push(V::S(if d[1] > 0 { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN
// b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY
// u.Id, u.DisplayName ), TopPosts AS ( SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS PostRank FROM Posts p
// WHERE p.PostTypeId = 1 ), UserPostMetrics AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS QuestionCount, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS
// AverageViewCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND
// p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName ), RankedUsers AS ( SELECT ub.UserId, ub.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
// up.QuestionCount, up.TotalScore, up.AverageViewCount, up.AcceptedAnswerCount, RANK() OVER (ORDER BY ub.BadgeCount DESC, up.TotalScore DESC) AS UserRank FROM
// UserBadgeCounts ub JOIN UserPostMetrics up ON ub.UserId = up.UserId ) SELECT ru.UserRank, ru.DisplayName, ru.BadgeCount, ru.GoldBadges, ru.SilverBadges, ru.BronzeBadges,
// ru.QuestionCount, ru.TotalScore, ru.AverageViewCount, ru.AcceptedAnswerCount, tp.Title AS TopPostTitle, tp.CreationDate AS TopPostDate FROM RankedUsers ru LEFT JOIN
// TopPosts tp ON ru.UserId = tp.OwnerUserId WHERE ru.UserRank <= 10 ORDER BY ru.UserRank;
fn q29230(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, score, view_count, accepted_answer, .. } = &db.post;
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let up = db.user.group_by(Ident::<User>::new()).select(qs().select(score.and(view_count.opt()).and(accepted_answer.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((s, w), x)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x.is_some() as i64],
        None => a,
    });
    let v = ranked(drain((&ub).and(&up)), |&(_, (b, a))| (Reverse(b[0]), a[0] == 0, Reverse(a[1])), false);
    type R = ((Id<User>, ([i64; 4], [i64; 5])), i64);
    let ru = rel(v);
    let v = drain((&ru).filt(|(_, r): R| r <= 10).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| u).select(qs().opt()))));
    rows(v.into_iter().map(|(_, (((u, (b, a)), r), p))| {
        let mut f = vec![V::I(r), user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), nullable(a[1], a[0]), avg(a[3], a[2]), V::I(a[4])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class =
// 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id,
// u.DisplayName ), TopUsers AS ( SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY BadgeCount DESC) AS UserRank FROM
// UserBadges WHERE BadgeCount > 0 ), PostStats AS ( SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
// AVG(p.ViewCount) AS AverageViews, SUM(COALESCE(c.CommentCount, 0)) AS TotalComments FROM Posts p LEFT JOIN ( SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP
// BY PostId ) c ON p.Id = c.PostId GROUP BY p.OwnerUserId ) SELECT u.Id AS UserId, u.DisplayName, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.PositivePosts, 0) AS
// PositivePosts, COALESCE(ps.AverageViews, 0) AS AverageViews, COALESCE(ps.TotalComments, 0) AS TotalComments, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
// COALESCE(ub.BadgeCount, 0) AS TotalBadges, tu.UserRank FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN
// TopUsers tu ON u.Id = tu.UserId WHERE (u.Reputation > 100 OR u.Views > 1000) AND (EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = u.Id HAVING COUNT(p.Id) > 5) OR NOT
// EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = u.Id)) ORDER BY tu.UserRank, u.DisplayName LIMIT 100 OFFSET 0;
fn q3296(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let tu = ranked(drain((&ub).filt(|a| a[0] > 0)), |&(_, a)| Reverse(a[0]), false);
    let tu = rel(tu.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let Post { owner_user, score, view_count, .. } = &db.post;
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt()).and((&cc).opt())).fold([0i64; 5], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c.unwrap_or(0)]
    });
    let User { reputation, views, .. } = &db.user;
    type R = (Option<[i64; 5]>, [i64; 4]);
    let v = drain(
        db.user
            .with(reputation.gt(100).or(views.gt(1000)))
            .select((&ps).opt().and(&ub))
            .filt(|(a, b): R| a.map_or(false, |a| a[0] > 5) || b[0] == 0)
            .and((&rank).map(|(_, r)| r).opt()),
    );
    let v = top_n(v, |&(u, (_, r))| (r.is_none(), r, db.user.display_name.get(u).unwrap(), u), 100);
    rows(v.into_iter().map(|(u, ((a, b), r))| {
        let a = a.unwrap_or([0; 5]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), if a[2] == 0 { V::F(0.0) } else { avg(a[3], a[2]) }, V::I(a[4]), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(b[0]), harness::fmt::oint(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC)
// AS PostRank FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' ), UserBadges AS ( SELECT U.Id AS UserId, COUNT(B.Id) FILTER (WHERE
// B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges, COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges FROM Users U LEFT JOIN Badges B
// ON U.Id = B.UserId GROUP BY U.Id ), ClosedPosts AS ( SELECT PH.PostId, PH.CreationDate, PH.UserDisplayName, C.Name AS CloseReason FROM PostHistory PH JOIN
// CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id WHERE PH.PostHistoryTypeId IN (10, 11) ), AggregatedStats AS ( SELECT U.Id AS UserId, U.DisplayName,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT
// JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName ) SELECT R.PostId, R.Title, R.CreationDate, U.DisplayName AS PostOwner, UGold.GoldBadges,
// USilver.SilverBadges, UBronze.BronzeBadges, A.TotalBounty, A.TotalViews, A.AverageScore, CP.CloseReason FROM RankedPosts R JOIN Users U ON R.OwnerUserId = U.Id LEFT JOIN
// UserBadges UGold ON U.Id = UGold.UserId LEFT JOIN UserBadges USilver ON U.Id = USilver.UserId LEFT JOIN UserBadges UBronze ON U.Id = UBronze.UserId LEFT JOIN
// AggregatedStats A ON U.Id = A.UserId LEFT JOIN ClosedPosts CP ON R.PostId = CP.PostId WHERE R.PostRank = 1 ORDER BY R.CreationDate DESC LIMIT 100;
//
// AggregatedStats is only read for the owners of the ranked posts, so its posts x votes product is driven for those users alone.
fn q2546(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let first = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&first).select(owner_user).collect();
    let ub = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64],
        None => a,
    });
    let agg = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((s, w), b)) => [a[0] + b.flatten().unwrap_or(0), a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + 1, a[4] + s],
            None => a,
        });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason));
    let v = drain((&first).select(owner_user.select((&ub).and(&agg))).and(cp.opt()));
    let v = top_n(v, |&(p, (_, r))| (Reverse(creation_date.get(p).unwrap()), p, r), 100);
    rows(v.into_iter().map(|(p, ((b, a), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), nullable(a[2], a[1]), avg(a[4], a[3]), harness::fmt::ostr(r)]);
        row(f)
    }))
}

// WITH UserReputation AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS
// AnswerCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 100 GROUP BY
// U.Id, U.DisplayName, U.Reputation ), TopUsers AS ( SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, ROW_NUMBER() OVER (ORDER BY Reputation
// DESC) AS Rank FROM UserReputation WHERE PostCount > 0 ), PopularTags AS ( SELECT T.TagName, COUNT(P.Id) AS TagPostCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' ||
// T.TagName || '%' GROUP BY T.TagName HAVING COUNT(P.Id) > 5 ), PostDetails AS ( SELECT P.Id AS PostId, P.Title, P.CreationDate, PT.Name AS PostType, U.DisplayName AS
// OwnerName, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id JOIN Users
// U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, PT.Name, U.DisplayName )
// SELECT TU.DisplayName AS TopUser, TU.Reputation, PT.Title AS PopularPostTitle, PT.PostId, PT.CreationDate, PT.PostType, PT.CommentCount, PT.UpvoteCount, Tags.TagName AS
// PopularTag FROM TopUsers TU JOIN PostDetails PT ON PT.OwnerName = TU.DisplayName JOIN PopularTags Tags ON PT.Title LIKE '%' || Tags.TagName || '%' WHERE TU.Rank <= 10
// ORDER BY TU.Reputation DESC, PT.UpvoteCount DESC, PT.CreationDate DESC;
fn q7515(db: &'static So) -> String {
    let Post { owner_user, title, .. } = &db.post;
    let np = db.post.group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tu = rel(top_n(drain(db.user.with((&db.user.reputation).gt(100)).with(&np).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10));
    let by_name: HashIdx<Str, Id<User>> = (&tu).map(|(u, _)| u).select(&db.user.display_name).inv().select((&tu).map(|(u, _)| u)).collect();
    let pd: MatSet<Id<Post>> = db.post.with(owner_user.select(&db.user.display_name).select(&by_name)).select(Ident::<Post>::new()).collect();
    let agg = (&pd)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let ts_ = tag_stats(db);
    let pt: HashIdx<Str, Id<Tag>> = db.tag.with((&ts_).filt(|a| a[0] > 5)).select(&db.tag.tag_name).inv().collect();
    let titles: MatSet<Str> = (&pd).select(title).collect();
    let like: HashIdx<Str, Id<Tag>> = (&titles).select_where(&pt, |t: Str, n: Str| t.contains(n)).collect();
    let v = drain((&agg).and(owner_user.select(&db.user.display_name).select(&by_name)).and(title.select(&like)));
    rows(v.into_iter().map(|(p, ((a, u), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "id", "created", "type"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::S(db.tag.tag_name.get(t).unwrap())]);
        row(f)
    }))
}

// WITH UserStatistics AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS
// QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId IN (4,5) THEN 1 ELSE 0 END) AS TagWikiCount,
// AVG(VB.BountyAmount) AS AvgBountyAmount, MAX(U.CreationDate) AS AccountCreationDate FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes VB ON P.Id =
// VB.PostId AND VB.VoteTypeId IN (8, 9) WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation ), PostEngagement AS ( SELECT P.Id AS PostId, P.Title,
// P.CreationDate, P.LastActivityDate, COUNT(C.ID) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1
// ELSE 0 END) AS DownVoteCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.Score > 0 GROUP BY P.Id, P.Title,
// P.CreationDate, P.LastActivityDate ), TopUsers AS ( SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TagWikiCount, AvgBountyAmount,
// AccountCreationDate, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserStatistics ) SELECT U.DisplayName AS TopUser, U.Reputation AS UserReputation, P.Title AS
// PopularPost, P.CommentCount AS PostCommentCount, P.UpVoteCount AS PostUpVoteCount, P.DownVoteCount AS PostDownVoteCount, U.AccountCreationDate AS UserAccountCreationDate
// FROM TopUsers U JOIN PostEngagement P ON U.PostCount > 5 WHERE U.UserRank <= 10 ORDER BY U.Reputation DESC, P.UpVoteCount DESC FETCH FIRST 50 ROWS ONLY;
//
// The JOIN is on `U.PostCount > 5` alone, so the top users are crossed with every PostEngagement row; the ORDER BY leads with
// the user side, so `lex_top` cuts the product without building it.
fn q9732(db: &'static So) -> String {
    let Post { score, .. } = &db.post;
    let np = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = ranked(drain(&np), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    type T = ((Id<User>, i64), i64);
    let tur = rel(tu);
    let tu: Vec<Id<User>> = drain((&tur).filt(|((_, n), r): T| r <= 10 && n > 5)).into_iter().map(|(_, ((u, _), _))| u).collect();
    let pe = db
        .post
        .with(score.gt(0))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = cross_top(tu, |&u| Reverse(db.user.reputation.get(u).unwrap()), drain(&pe), |&(p, a)| (Reverse(a[1]), p), 50);
    rows(v.into_iter().map(|(u, (p, a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend(a.map(V::I));
        f.push(user_col(db, u, "ucreated"));
        row(f)
    }))
}

// WITH PostTags AS ( SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1 ),
// UserPostStats AS ( SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.AnswerCount, 0)) AS
// TotalAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id ), TagPopularity AS ( SELECT Tag, COUNT(*) AS
// TagCount FROM PostTags GROUP BY Tag ), TopTags AS ( SELECT Tag, TagCount FROM TagPopularity ORDER BY TagCount DESC LIMIT 10 ), UserReputationStats AS ( SELECT u.Id AS
// UserId, u.DisplayName, up.QuestionCount, up.TotalViews, up.TotalAnswers, up.TotalScore, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u JOIN UserPostStats
// up ON u.Id = up.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, up.QuestionCount, up.TotalViews, up.TotalAnswers, up.TotalScore, u.DisplayName, u.Reputation
// ), OverallStats AS ( SELECT AVG(QuestionCount) AS AvgQuestions, AVG(TotalViews) AS AvgViews, AVG(TotalAnswers) AS AvgAnswers, AVG(TotalScore) AS AvgScore, SUM(BadgeCount)
// AS TotalBadges FROM UserReputationStats ) SELECT u.DisplayName, u.Reputation, u.QuestionCount, u.TotalViews, u.TotalAnswers, u.TotalScore, (u.TotalScore - os.AvgScore) AS
// ScoreDifference, (u.TotalViews - os.AvgViews) AS ViewDifference, COALESCE(tt.Tag, 'General') AS PopularTag FROM UserReputationStats u CROSS JOIN OverallStats os LEFT JOIN
// TopTags tt ON u.QuestionCount > os.AvgQuestions ORDER BY u.TotalScore DESC;
//
// BadgeCount only feeds TotalBadges, which nothing reads, so the badges join is not driven.
fn q25981(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, answer_count, score, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(answer_count.opt()).and(score)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((w, n), s)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + n.unwrap_or(0), a[3] + s],
            None => a,
        });
    let tot = (&ups).fold_flat([0i64; 5], |t, a| [t[0] + a[0], t[1] + a[1], t[2] + a[2], t[3] + a[3], t[4] + 1]);
    let n = tot[4] as f64;
    let (aq, av, as_) = (tot[0] as f64 / n, tot[1] as f64 / n, tot[3] as f64 / n);
    let freq = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&freq), |&(t, n)| (Reverse(n), t), 10));
    let tt_all: HashIdx<(), (Str, i64)> = (&tt).map(|_| ()).inv().select(&tt).collect();
    let v = drain((&ups).select(Same::<[i64; 4]>::new().and(Same::<[i64; 4]>::new().filt(move |a: [i64; 4]| a[0] as f64 > aq).map(|_| ()).select(&tt_all).opt())));
    rows(v.into_iter().map(|(u, (a, t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::F(a[3] as f64 - as_), V::F(a[1] as f64 - av), V::S(t.map_or("General", |(t, _)| t))]);
        row(f)
    }))
}

// WITH UserPostStatistics AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalWikiPosts, SUM(CASE WHEN p.Score > 0
// THEN 1 ELSE 0 END) AS PositiveScorePosts FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT UserId, DisplayName,
// TotalPosts, TotalQuestions, TotalAnswers, TotalWikiPosts, PositiveScorePosts, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStatistics WHERE TotalPosts
// > 0 ), UserBadges AS ( SELECT b.UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0
// END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId ), UserStatistics AS ( SELECT tu.UserId,
// tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalWikiPosts, tu.PositiveScorePosts, ub.TotalBadges, ub.GoldBadges, ub.SilverBadges,
// ub.BronzeBadges FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId ) SELECT us.DisplayName, us.TotalPosts, us.TotalQuestions, us.TotalAnswers,
// us.TotalWikiPosts, us.PositiveScorePosts, COALESCE(us.TotalBadges, 0) AS TotalBadges, COALESCE(us.GoldBadges, 0) AS GoldBadges, COALESCE(us.SilverBadges, 0) AS
// SilverBadges, COALESCE(us.BronzeBadges, 0) AS BronzeBadges FROM UserStatistics us WHERE us.TotalPosts > 10 ORDER BY us.TotalPosts DESC LIMIT 10;
fn q29320(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + (s > 0) as i64],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = top_n(drain((&ups).filt(|a| a[0] > 10).and((&ub).opt())), |&(u, (a, _))| (Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.Score DESC)
// AS Rank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate,
// p.Score, p.ViewCount ), TopQuestions AS ( SELECT rp.Id, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank = 1 ORDER BY rp.Score DESC
// LIMIT 10 ), UserStatistics AS ( SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0
// END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, MAX(u.Reputation) AS MaxReputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName ), PostVoteCounts AS ( SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3
// THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id )
// SELECT tq.Title, tq.Score, tq.ViewCount, tq.CommentCount, u.DisplayName AS TopUser, us.GoldBadges, us.SilverBadges, us.BronzeBadges, us.MaxReputation, pvc.UpVotes,
// pvc.DownVotes, pvc.TotalVotes FROM TopQuestions tq JOIN Posts post ON tq.Id = post.Id JOIN Users u ON post.OwnerUserId = u.Id JOIN UserStatistics us ON u.Id = us.UserId
// JOIN PostVoteCounts pvc ON tq.Id = pvc.PostId ORDER BY tq.Score DESC, us.MaxReputation DESC;
//
// CURRENT_TIMESTAMP is when the query runs and the data ends in 2024, so RankedPosts is empty. The comparison is
// TIMESTAMPTZ, so CreationDate is read as New York local time.
fn q1266(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let since = ny_to_utc(add_years(utc_to_ny(now_utc()), -1));
    let rp = || db.post.with(creation_date.filt(move |d| ny_to_utc(d) >= since));
    let tq = rel(top_n(drain(rp().select(score)), |&(p, s)| (Reverse(s), p), 10));
    let tq: MatSet<Id<Post>> = (&tq).map(|(p, _)| p).collect();
    let cc = (&tq).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pvc = (&tq).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(2 | 3)) as i64]
    });
    let us = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64],
        None => a,
    });
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&us))).and(&pvc));
    rows(v.into_iter().map(|(p, ((c, (u, b)), a))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), user_col(db, u, "name")]);
        f.extend(b.map(V::I));
        f.push(user_col(db, u, "rep"));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE
// WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.PostTypeId IN (1, 2) THEN P.Score ELSE 0 END) AS TotalScore, SUM(CASE WHEN P.PostTypeId = 1 AND
// P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName ), UserBadges AS
// ( SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END)
// AS BronzeBadges FROM Badges B GROUP BY B.UserId ), UserVoteStats AS ( SELECT V.UserId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS TotalUpvotes, COUNT(CASE WHEN
// V.VoteTypeId = 3 THEN 1 END) AS TotalDownvotes FROM Votes V GROUP BY V.UserId ), TopUsers AS ( SELECT UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.TotalQuestions,
// UPS.TotalAnswers, UPS.TotalScore, UPS.AcceptedAnswers, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, UVS.TotalUpvotes, UVS.TotalDownvotes, RANK() OVER (ORDER BY
// UPS.TotalScore DESC) AS ScoreRank FROM UserPostStats UPS LEFT JOIN UserBadges UB ON UPS.UserId = UB.UserId LEFT JOIN UserVoteStats UVS ON UPS.UserId = UVS.UserId ) SELECT
// TU.DisplayName, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalScore, TU.AcceptedAnswers, TU.GoldBadges, TU.SilverBadges, TU.BronzeBadges, TU.TotalUpvotes,
// TU.TotalDownvotes, TU.ScoreRank FROM TopUsers TU WHERE TU.ScoreRank <= 10 ORDER BY TU.TotalScore DESC, TU.DisplayName;
fn q6004(db: &'static So) -> String {
    let Post { post_type_id, score, accepted_answer_id, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(accepted_answer_id.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, s), x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 || t == 2 { s } else { 0 }, a[4] + (t == 1 && x.is_some()) as i64],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = ranked(drain((&ups).and((&ub).opt()).and((&uv).opt())), |&(_, ((a, _), _))| (a[0] == 0, Reverse(a[3])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, b), c)), r)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), V::I(a[4])]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some(c) => c.map(V::I),
            None => [V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RecursivePostStats AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY
// p.OwnerUserId ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p WHERE p.PostTypeId = 1 ), UserStats AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation,
// COUNT(b.Id) AS BadgeCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId
// = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName,
// u.Reputation ), TopUsers AS ( SELECT us.UserId, us.DisplayName, us.Reputation, us.BadgeCount, us.TotalBounty, us.TotalUpVotes, us.TotalDownVotes, ROW_NUMBER() OVER (ORDER
// BY us.Reputation DESC) AS UserRank FROM UserStats us WHERE us.Reputation > 0 ), PostHistoryAggregates AS ( SELECT ph.PostId, MAX(ph.CreationDate) AS LastEditDate,
// COUNT(ph.Id) AS EditCount, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseOpenCount FROM PostHistory ph GROUP BY ph.PostId ) SELECT p.PostId,
// p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName, us.Reputation, us.BadgeCount, us.TotalBounty, ph.LastEditDate,
// ph.EditCount, ph.CloseOpenCount FROM RecursivePostStats p JOIN Users u ON p.OwnerUserId = u.Id JOIN UserStats us ON u.Id = us.UserId JOIN PostHistoryAggregates ph ON
// p.PostId = ph.PostId WHERE us.Reputation > 100 AND p.RowNum = 1 ORDER BY us.Reputation DESC, p.CreationDate ASC LIMIT 50 OFFSET 0;
//
// UserStats is only read for the owners of the picked questions, so its badges x votes product is driven for those users alone.
fn q31064(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(100));
    let owners: MatSet<Id<User>> = (&first).select(owner_user.select(rich)).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, (b, v)| [a[0] + b.is_some() as i64, a[1] + v.flatten().unwrap_or(0)]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.group_by(post).select(hd.and(post_history_type_id)).fold((i64::MIN, 0i64, 0i64), |(m, n, c), (d, t)| (m.max(d), n + 1, c + matches!(t, 10 | 11) as i64));
    let v = drain((&first).select(owner_user.select(Ident::<User>::new().and(&us))).and(&pha));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), creation_date.get(p).unwrap(), p), 50);
    rows(v.into_iter().map(|(p, ((u, a), (m, n, c)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::T(m), V::I(n), V::I(c)]);
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(P.Score) AS TotalScore, COUNT(DISTINCT C.Id)
// AS CommentCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId WHERE U.CreationDate <= cast('2024-10-01 12:34:56' as
// timestamp) - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName, U.Reputation ), PostMetrics AS ( SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score,
// COUNT(DISTINCT C.Id) AS TotalComments, COUNT(DISTINCT V.Id) AS TotalVotes, COUNT(DISTINCT PH.Id) AS EditCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT
// JOIN Votes V ON P.Id = V.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId IN (4, 5) WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as
// timestamp) - INTERVAL '1 month' GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score ), UserPostSummary AS ( SELECT UA.UserId, UA.DisplayName, UA.Reputation,
// UA.PostCount, UA.TotalScore, UA.CommentCount, PM.PostId, PM.Title, PM.CreationDate, PM.ViewCount, PM.Score, PM.TotalComments, PM.TotalVotes, RANK() OVER (PARTITION BY
// UA.UserId ORDER BY PM.CreationDate DESC) AS PostRank FROM UserActivity UA JOIN PostMetrics PM ON UA.UserId = PM.PostId ) SELECT UPS.UserId, UPS.DisplayName,
// UPS.Reputation, UPS.PostCount, UPS.TotalScore, UPS.CommentCount, UPS.PostId AS RecentPostId, UPS.Title AS RecentPostTitle, UPS.CreationDate AS RecentPostCreationDate,
// UPS.ViewCount AS RecentPostViewCount, UPS.Score AS RecentPostScore, UPS.TotalComments AS RecentPostTotalComments, UPS.TotalVotes AS RecentPostTotalVotes FROM
// UserPostSummary UPS WHERE UPS.PostRank = 1 ORDER BY UPS.Reputation DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. `UA.UserId = PM.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q32648(db: &'static So) -> String {
    let Post { creation_date, origid, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let recent = || db.post.with(creation_date.ge(add_months(t0, -1)));
    let tc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tv = recent().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let byorig: HashIdx<i64, Id<Post>> = recent().select(origid).inv().collect();
    let old = || db.user.with((&db.user.creation_date).le(add_years(t0, -1)));
    let ua = old().group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(comments_of(db).opt())).opt()).fold(0i64, |s, p| s + p.map_or(0, |(s, _)| s));
    let npc = old().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ncc = old().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&ua).and(&npc).and(&ncc).and((&db.user.origid).select(&byorig).select(Ident::<Post>::new().and(&tc).and(&tv))));
    rows(v.into_iter().map(|(u, (((s, n), c), ((p, pc), pv)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), nullable(s, n), V::I(c)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(pc), V::I(pv)]);
        row(f)
    }))
}

// WITH PostStatistics AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN
// v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id
// ORDER BY p.CreationDate DESC) AS rn FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate,
// p.Score, p.ViewCount ), TopPosts AS ( SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.CommentCount, ps.UpVotes, ps.DownVotes, DENSE_RANK() OVER
// (ORDER BY ps.Score DESC) AS ScoreRank FROM PostStatistics ps ), UserBadges AS ( SELECT b.UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE
// b.Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Badges b GROUP BY b.UserId ), TopUsers AS ( SELECT u.Id AS UserId, u.DisplayName,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges FROM Users u LEFT JOIN UserBadges ub
// ON u.Id = ub.UserId ) SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpVotes, tp.DownVotes, tu.DisplayName AS TopUser, tu.GoldBadges,
// tu.SilverBadges, tu.BronzeBadges FROM TopPosts tp JOIN TopUsers tu ON tp.PostId = (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = tu.UserId ORDER BY p.Score DESC LIMIT 1)
// WHERE tp.ScoreRank <= 10 ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// The correlated `ORDER BY p.Score DESC LIMIT 1` picks each user's highest-scoring post; among tied posts the port takes the smallest id.
fn q2736(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.select(score)), |&(_, s)| Reverse(s), true);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let best = top_per(drain(db.post.with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let best: MatSet<Id<Post>> = rel(best.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .with(&best)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&ps).and(owner_user.select(Ident::<User>::new().and((&ub).opt()))));
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(user_col(db, u, "name"));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U
// WHERE U.Reputation > 0 ), TopPosts AS ( SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, P.OwnerUserId, COUNT(C.ID) AS CommentCount, SUM(CASE WHEN V.VoteTypeId =
// 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON
// P.Id = V.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.Score, P.CreationDate, P.OwnerUserId ), ClosedPosts AS ( SELECT PH.PostId, PH.PostHistoryTypeId, COUNT(*)
// AS CloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId, PH.PostHistoryTypeId ), PostMetrics AS ( SELECT TP.PostId, TP.Title,
// TP.Score, TP.CommentCount, COALESCE(CP.CloseCount, 0) AS CloseCount, U.DisplayName AS Owner, U.Reputation AS OwnerReputation, U.ReputationRank AS OwnerRank FROM TopPosts
// TP JOIN UserReputation U ON TP.OwnerUserId = U.UserId LEFT JOIN ClosedPosts CP ON TP.PostId = CP.PostId ), FinalMetrics AS ( SELECT PM.*, CASE WHEN PM.CloseCount > 0 THEN
// 'Closed' ELSE 'Active' END AS Status, CASE WHEN PM.OwnerReputation > 5000 THEN 'Veteran' WHEN PM.OwnerReputation BETWEEN 1000 AND 5000 THEN 'Established' ELSE 'Newcomer'
// END AS UserCategory FROM PostMetrics PM ) SELECT F.UserCategory, F.Status, F.Title, F.Score, F.CommentCount, F.CloseCount, F.Owner, F.OwnerReputation FROM FinalMetrics F
// WHERE F.Score > ( SELECT AVG(Score) FROM TopPosts ) ORDER BY F.Score DESC, F.CommentCount DESC FETCH FIRST 10 ROWS ONLY;
fn q2965(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let tp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let (ss, sn) = db.post.with(post_type_id.eq(1)).select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let above = move |s: i64| s as i128 * sn as i128 > ss as i128;
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cv = rel(drain(&cp));
    let cp_of: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let v = drain(db.post.with(score.filt(above)).select((&tp).and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(0)))).and((&cp_of).map(|(_, n)| n).opt())));
    let v = top_n(v, |&(p, ((c, _), n))| (Reverse(score.get(p).unwrap()), Reverse(c), p, n), 10);
    rows(v.into_iter().map(|(p, ((c, u), n))| {
        let n = n.unwrap_or(0);
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = vec![V::S(if rep > 5000 { "Veteran" } else if rep >= 1000 { "Established" } else { "Newcomer" }), V::S(if n > 0 { "Closed" } else { "Active" })];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), V::I(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION
// BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) -
// INTERVAL '6 months' AND p.ViewCount > 100 ), TopPosts AS ( SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, OwnerDisplayName FROM RankedPosts WHERE Rank
// <= 5 ), PostStatistics AS ( SELECT th.PostId, th.Title, th.CreationDate, th.Score, th.ViewCount, th.AnswerCount, COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount FROM TopPosts th LEFT JOIN ( SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId ) c ON th.PostId = c.PostId
// LEFT JOIN ( SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId ) b ON b.UserId = ( SELECT OwnerUserId FROM Posts WHERE Id = th.PostId ) ) SELECT ps.PostId,
// ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.AnswerCount, ps.CommentCount, ps.BadgeCount, pht.Name AS PostHistoryType, COUNT(phe.Id) AS RevisionCount FROM
// PostStatistics ps LEFT JOIN PostHistory phe ON ps.PostId = phe.PostId LEFT JOIN PostHistoryTypes pht ON phe.PostHistoryTypeId = pht.Id GROUP BY ps.PostId, ps.Title,
// ps.CreationDate, ps.Score, ps.ViewCount, ps.AnswerCount, ps.CommentCount, ps.BadgeCount, pht.Name ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q8947(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(date(2024, 10, 1), -6)).and(view_count.gt(100))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, .. } = &db.post_history;
    let ht = db.post_history.group_by(post.and(htype_name(db))).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let hv = rel(drain(&ht));
    let ht_of: HashIdx<Id<Post>, ((Id<Post>, Str), i64)> = (&hv).map(|((p, _), _)| p).inv().select(&hv).collect();
    let v = drain((&tp).select((&cc).opt().and(owner_user.select(&bc).opt()).and((&ht_of).opt())));
    rows(v.into_iter().map(|(p, ((c, b), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(b.unwrap_or(0))]);
        f.extend(match h {
            Some(((_, t), n)) => [V::S(t), V::I(n)],
            None => [V::Null, V::I(0)],
        });
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS
// Rank, U.DisplayName AS Author, COALESCE(( SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2 ), 0) AS Upvotes, COALESCE(( SELECT COUNT(*) FROM Votes
// v WHERE v.PostId = p.Id AND v.VoteTypeId = 3 ), 0) AS Downvotes, ( SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id ) AS CommentCount FROM Posts p JOIN Users U ON
// p.OwnerUserId = U.Id WHERE p.CreationDate >= DATE '2023-01-01' ), ClosedPosts AS ( SELECT ph.PostId, ph.CreationDate, ph.Comment, ph.UserDisplayName, P.Title FROM
// PostHistory ph JOIN Posts P ON ph.PostId = P.Id WHERE ph.PostHistoryTypeId = 10 ), FilteredPosts AS ( SELECT rp.Title, rp.PostId, rp.CreationDate, rp.ViewCount, rp.Score,
// rp.Upvotes, rp.Downvotes, rp.CommentCount, COALESCE(cp.Comment, 'Not closed') AS CloseComment, COALESCE(cp.UserDisplayName, 'N/A') AS CloseUser FROM RankedPosts rp LEFT
// JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.Rank <= 5 ) SELECT p.PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.Upvotes, p.Downvotes, p.CommentCount,
// (p.Upvotes - p.Downvotes) AS NetVotes, CASE WHEN p.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus, CONCAT('Closed Comment: ',
// p.CloseComment, ' by ', p.CloseUser) AS CloseDetails FROM FilteredPosts p WHERE (p.Upvotes - p.Downvotes) > 0 ORDER BY p.Score DESC, p.CreationDate DESC;
fn q33427(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(date(2023, 1, 1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post_history_type_id, comment, user_display_name, .. } = &db.post_history;
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)));
    let v = drain((&ud).filt(|a| a[0] - a[1] > 0).and(&cc).and(closed.opt()));
    rows(v.into_iter().map(|(p, ((a, c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(a[0] - a[1]), V::S(if c > 0 { "Has Comments" } else { "No Comments" })]);
        let cm = h.and_then(|h| comment.get(h)).unwrap_or("Not closed");
        let cu = h.and_then(|h| user_display_name.get(h)).unwrap_or("N/A");
        f.push(V::Owned(format!("Closed Comment: {cm} by {cu}")));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY
// p.CreationDate DESC) AS Rank FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 10 ), UserActivity AS ( SELECT u.Id AS
// UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, COUNT(DISTINCT c.Id) AS CommentsCount, SUM(v.BountyAmount) AS TotalBounties FROM Users u LEFT JOIN Posts p ON
// u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id WHERE u.Reputation > 1000 GROUP BY u.Id,
// u.DisplayName ), RecentPostHistory AS ( SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserId, p.Title, p.OwnerDisplayName FROM PostHistory ph JOIN Posts p
// ON ph.PostId = p.Id WHERE ph.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND ph.PostHistoryTypeId IN (10, 11, 12) ), TopUserPosts AS ( SELECT
// rp.PostId, COUNT(DISTINCT u.Id) AS ActiveUsers, AVG(rp.ViewCount) AS AverageViews FROM RankedPosts rp JOIN Votes v ON rp.PostId = v.PostId JOIN Users u ON v.UserId = u.Id
// GROUP BY rp.PostId ) SELECT up.UserId, up.DisplayName, up.PostsCount, up.CommentsCount, up.TotalBounties, pp.PostId, pp.AverageViews, rph.CreationDate AS
// RecentActionDate, rph.PostHistoryTypeId, rph.OwnerDisplayName, rp.Rank FROM UserActivity up JOIN TopUserPosts pp ON up.PostsCount > 5 JOIN RecentPostHistory rph ON
// pp.PostId = rph.PostId JOIN RankedPosts rp ON pp.PostId = rp.PostId ORDER BY up.TotalBounties DESC, rp.Rank ASC;
//
// The JOIN on `up.PostsCount > 5` names only up, so those users are crossed with TopUserPosts.
fn q8757(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = || db.post.with(creation_date.gt(add_years(t0, -1)).and(score.gt(10)));
    let rk = ranked(drain(rp().select(post_type_id)), |&(p, t)| (t, Reverse(creation_date.get(p).unwrap()), p), false);
    let rk = rel(per_group(rk, |&(_, t)| t).into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let bounty = own_votes(db);
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(bounty.select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 2], |a, p| match p.and_then(|(_, b)| b.flatten()) {
            Some(b) => [a[0] + 1, a[1] + b],
            None => a,
        });
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tup = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.user)).count_distinct();
    let PostHistory { creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let rph = history_of(db).select(Ident::<PostHistory>::new().with(hd.gt(add_days(t0, -30)).and(post_history_type_id.is_in([10, 11, 12]))));
    let ur = rel(drain((&ua).and((&np).filt(|n| n > 5)).and(&nc)));
    let pr = rel(drain((&tup).and(rph).and((&rank).map(|(_, r)| r))));
    let v = drain((&ur).cross(&pr));
    rows(v.into_iter().map(|(_, ((u, ((a, n), c)), (p, ((_, h), r))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c), nullable(a[1], a[0]), post_fields(db, p, &["id"]).remove(0)]);
        f.push(harness::fmt::ofloat(view_count.get(p).map(|w| w as f64)));
        f.extend([V::T(hd.get(h).unwrap()), V::I(post_history_type_id.get(h).unwrap()), post_fields(db, p, &["owner_name"]).remove(0), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC)
// AS Rank FROM Posts p WHERE p.PostTypeId = 1 ), TopUsers AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS QuestionCount,
// SUM(v.BountyAmount) AS TotalBounties FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN
// (8, 9) GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate HAVING COUNT(DISTINCT p.Id) > 5 ), RecentActivity AS ( SELECT p.OwnerUserId, COUNT(DISTINCT c.Id) AS
// CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.OwnerUserId ), UserStats AS ( SELECT tu.UserId,
// tu.DisplayName, tu.Reputation, COALESCE(ra.CommentCount, 0) AS CommentCount, ra.LastCommentDate, tu.QuestionCount, tu.TotalBounties FROM TopUsers tu LEFT JOIN
// RecentActivity ra ON tu.UserId = ra.OwnerUserId ), PostHistoryStats AS ( SELECT ph.UserId, COUNT(*) AS EditCount, COUNT(DISTINCT ph.PostId) AS UniquePostEdits FROM
// PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.UserId ) SELECT us.DisplayName, us.Reputation, us.QuestionCount, us.TotalBounties, us.CommentCount,
// us.LastCommentDate, COALESCE(ph.EditCount, 0) AS TotalEdits, COALESCE(ph.UniquePostEdits, 0) AS UniquePostsEdited FROM UserStats us LEFT JOIN PostHistoryStats ph ON
// us.UserId = ph.UserId WHERE us.Reputation > 1000 ORDER BY us.Reputation DESC, us.QuestionCount DESC;
fn q30227(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let tb = users().group_by(Ident::<User>::new()).select(qs().select(bounty.opt()).opt()).fold([0i64; 2], |a, x| match x.flatten().flatten() {
        Some(b) => [a[0] + 1, a[1] + b],
        None => a,
    });
    let nq = users().group_by(Ident::<User>::new()).select(qs().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let Comment { creation_date: cd, .. } = &db.comment;
    let ra = users().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).select(cd))).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let PostHistory { user, post, post_history_type_id, .. } = &db.post_history;
    let edits = || db.post_history.with(post_history_type_id.is_in([4, 5, 6]));
    let ec = edits().group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let ep = edits().group_by(user).select(post).count_distinct();
    let v = drain((&tb).and((&nq).filt(|n| n > 5)).and((&ra).opt()).and((&ec).opt()).and((&ep).opt()));
    rows(v.into_iter().map(|(u, ((((b, q), r), e), d))| {
        let (n, m) = r.unwrap_or((0, i64::MIN));
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(q), nullable(b[1], b[0]), V::I(n), tmax(m), V::I(e.unwrap_or(0)), V::I(d.unwrap_or(0))]);
        row(f)
    }))
}

// WITH TagFrequency AS ( SELECT tag, COUNT(*) AS tag_count FROM ( SELECT TRIM(UNNEST(string_to_array(SUBSTRING(Tags, 2, LENGTH(Tags) - 2), '><'))) AS tag FROM Posts WHERE
// PostTypeId = 1 ) AS UnnestedTags GROUP BY tag ), TopTags AS ( SELECT tag, tag_count FROM TagFrequency ORDER BY tag_count DESC LIMIT 5 ), UserActivity AS ( SELECT u.Id AS
// user_id, u.DisplayName, COUNT(DISTINCT p.Id) AS question_count, SUM(p.ViewCount) AS total_views, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS total_upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS total_downvotes FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE
// p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName ), TagContributions AS ( SELECT u.Id AS user_id, COUNT(DISTINCT p.Id) AS contributions, t.tag FROM Users u JOIN Posts p ON
// u.Id = p.OwnerUserId JOIN (SELECT DISTINCT TRIM(UNNEST(string_to_array(SUBSTRING(Tags, 2, LENGTH(Tags) - 2), '><'))) AS tag FROM Posts WHERE PostTypeId = 1) t ON
// POSITION(t.tag IN SUBSTRING(p.Tags, 2, LENGTH(p.Tags) - 2)) > 0 GROUP BY u.Id, t.tag ), FinalResults AS ( SELECT ua.DisplayName, ua.question_count, ua.total_views,
// ua.total_upvotes, ua.total_downvotes, tc.tag, tc.contributions FROM UserActivity ua LEFT JOIN TagContributions tc ON ua.user_id = tc.user_id WHERE tc.tag IN (SELECT tag
// FROM TopTags) ) SELECT DisplayName, question_count, total_views, total_upvotes, total_downvotes, tag, contributions FROM FinalResults ORDER BY question_count DESC,
// contributions DESC;
//
// TagContributions matches a tag anywhere inside the post's stripped Tags string, so the tag test is a substring test on that string.
fn q25177(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, view_count, .. } = &db.post;
    let inner = |t: Str| -> Str { &t[1..t.len() - 1] };
    let qs = || db.post.with(post_type_id.eq(1));
    let freq = qs().select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&freq), |&(t, n)| (Reverse(n), t), 5));
    let top: HashIdx<Str, Str> = (&tt).map(|(t, _)| t).inv().select((&tt).map(|(t, _)| t)).collect();
    let strs: MatSet<Str> = db.post.select(tags_str.map(inner)).collect();
    let like: HashIdx<Str, Str> = (&strs).select_where(&top, |s: Str, t: Str| s.contains(t)).collect();
    let tc = db.post.with(owner_user).group_by(owner_user.and(tags_str.map(inner).select(&like))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (w, t)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let nq = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)))).fold(0i64, |n, _| n + 1);
    let tv = rel(drain(&tc));
    let tc_of: HashIdx<Id<User>, ((Id<User>, Str), i64)> = (&tv).map(|((u, _), _)| u).inv().select(&tv).collect();
    let v = drain((&ua).and(&nq).and(&tc_of));
    rows(v.into_iter().map(|(u, ((a, q), ((_, t), n)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(q), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3])];
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN
// p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1
// ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentsCount FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName ), TopUsers AS (
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, AcceptedAnswers, UpVotes, DownVotes, CommentsCount, RANK() OVER (ORDER BY TotalPosts DESC) AS Rank FROM
// UserPostStats ), TopQuestions AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentsCount FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE
// p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName ORDER BY VoteCount DESC LIMIT 5 ) SELECT tu.DisplayName AS TopUser,
// tu.TotalPosts, tu.Questions, tu.Answers, tu.AcceptedAnswers, tu.UpVotes, tu.DownVotes, tu.CommentsCount, tq.Title AS TopQuestionTitle, tq.ViewCount AS TopQuestionViews,
// tq.Score AS TopQuestionScore FROM TopUsers tu CROSS JOIN TopQuestions tq WHERE tu.Rank <= 10 ORDER BY tu.TotalPosts DESC;
fn q9094(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, owner_user, .. } = &db.post;
    let vt = || votes_of(db).select(&db.vote.vote_type_id).opt();
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer.opt()).and(comments_of(db).opt()).and(vt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some((((t, x), c), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + x.is_some() as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64, a[6] + c.is_some() as i64],
            None => a,
        });
    let tu = ranked(drain(&ups), |&(_, a)| Reverse(a[0]), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let tq = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(Ident::<Post>::new()).select(vt().and(comments_of(db).opt())).fold(0i64, |n, (v, _)| n + v.is_some() as i64);
    let tq = rel(top_n(drain(&tq), |&(p, n)| (Reverse(n), p), 5));
    let v = drain((&tu).cross(&tq));
    rows(v.into_iter().map(|(_, ((u, a), (p, _)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        row(f)
    }))
}

// WITH UserBadges AS ( SELECT U.Id AS UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE
// WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id ), PostDetails AS ( SELECT P.Id AS PostId, P.Title,
// P.CreationDate, P.OwnerUserId, COUNT(C) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END)
// AS DownVotes, DENSE_RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS UserPostRank FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN
// Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId ), PopularPosts AS ( SELECT PD.PostId, PD.Title, PD.CreationDate, PD.CommentCount,
// PD.UpVotes, PD.DownVotes, U.DisplayName, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges FROM PostDetails PD JOIN Users U ON PD.OwnerUserId = U.Id LEFT JOIN UserBadges UB
// ON U.Id = UB.UserId WHERE PD.UpVotes - PD.DownVotes > 5 ORDER BY PD.UpVotes DESC ), OverallPerformance AS ( SELECT PostId, Title, CommentCount, UpVotes - DownVotes AS
// NetScore, GoldBadges, SilverBadges, BronzeBadges, CASE WHEN GoldBadges > 0 THEN 'High Contributor' WHEN SilverBadges > 0 THEN 'Contributor' ELSE 'New Member' END AS
// MemberType FROM PopularPosts ) SELECT *, CASE WHEN MemberType = 'High Contributor' AND NetScore > 20 THEN 'Top Performer' WHEN MemberType = 'Contributor' AND NetScore
// BETWEEN 10 AND 20 THEN 'Regular Performer' ELSE 'Needs Improvement' END AS PerformanceCategory FROM OverallPerformance WHERE NetScore IS NOT NULL ORDER BY NetScore DESC,
// CommentCount DESC;
//
// COUNT(C) counts the whole row of C, which is never NULL, so it is the joined row count (translation-failures.md 11).
fn q4773(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64],
        None => a,
    });
    let pd = db
        .post
        .with(&db.post.owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (_, t)| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&pd).filt(|a| a[1] - a[2] > 5).and((&db.post.owner_user).select(&ub)));
    rows(v.into_iter().map(|(p, (a, b))| {
        let net = a[1] - a[2];
        let mt = if b[0] > 0 { "High Contributor" } else if b[1] > 0 { "Contributor" } else { "New Member" };
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(net)]);
        f.extend(b.map(V::I));
        f.push(V::S(mt));
        f.push(V::S(if mt == "High Contributor" && net > 20 {
            "Top Performer"
        } else if mt == "Contributor" && (10..=20).contains(&net) {
            "Regular Performer"
        } else {
            "Needs Improvement"
        }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS ( SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount, SUM(CASE WHEN b.Class = 2 THEN 1
// ELSE 0 END) AS SilverCount, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id ), TopUsers AS (
// SELECT UserId, Reputation, BadgeCount, GoldCount, SilverCount, BronzeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserBadgeCounts ub JOIN Users u
// ON ub.UserId = u.Id ), PostStatistics AS ( SELECT OwnerUserId, COUNT(*) AS PostCount, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN
// PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CommentCount) AS TotalComments, SUM(ViewCount) AS TotalViews, RANK() OVER (ORDER BY SUM(ViewCount) DESC) AS ViewRank
// FROM Posts GROUP BY OwnerUserId ), CombinedStats AS ( SELECT tu.UserId, tu.Reputation, tu.BadgeCount, tu.GoldCount, tu.SilverCount, tu.BronzeCount, COALESCE(ps.PostCount,
// 0) AS PostCount, COALESCE(ps.QuestionCount, 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount, COALESCE(ps.TotalComments, 0) AS TotalComments,
// COALESCE(ps.TotalViews, 0) AS TotalViews, tu.ReputationRank, ps.ViewRank FROM TopUsers tu LEFT JOIN PostStatistics ps ON tu.UserId = ps.OwnerUserId ) SELECT c.UserId,
// c.Reputation, c.BadgeCount, c.GoldCount, c.SilverCount, c.BronzeCount, c.PostCount, c.QuestionCount, c.AnswerCount, c.TotalComments, c.TotalViews, c.ReputationRank,
// c.ViewRank FROM CombinedStats c WHERE c.ReputationRank <= 10 OR c.ViewRank <= 10 ORDER BY c.ReputationRank, c.ViewRank;
//
// PostStatistics groups by OwnerUserId, so the ownerless posts are a group of their own and take a ViewRank, even though it never joins a user.
fn q9766(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let rr = ranked(drain(&ub), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let Post { owner_user, post_type_id, comment_count, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user.opt()).select(post_type_id.and(comment_count).and(view_count.opt())).fold([0i64; 6], |a, ((t, c), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let vr = ranked(drain(&ps), |&(_, a)| (a[4] == 0, Reverse(a[5])), false);
    let vr = rel(vr.into_iter().map(|((u, a), r)| (u, (a, r))).collect());
    let vr_of: HashIdx<Option<Id<User>>, (Option<Id<User>>, ([i64; 6], i64))> = (&vr).map(|(u, _)| u).inv().select(&vr).collect();
    type R = ((Id<User>, [i64; 4]), i64);
    let rr = rel(rr);
    let v = drain((&rr).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| Some(u)).select((&vr_of).map(|(_, x)| x)).opt())).filt(|(((_, _), r), p): (R, Option<([i64; 6], i64)>)| {
        r <= 10 || p.map_or(false, |(_, w)| w <= 10)
    }));
    rows(v.into_iter().map(|(_, (((u, b), r), p))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(b.map(V::I));
        match p {
            Some((a, w)) => {
                f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[5]), V::I(r), V::I(w)]);
            }
            None => f.extend([V::I(0), V::I(0), V::I(0), V::I(0), V::I(0), V::I(r), V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
// p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0 ), UserStats AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3
// THEN 1 ELSE 0 END), 0) AS TotalDownvotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation ), PostDetails AS ( SELECT
// rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, us.DisplayName, us.Reputation, us.TotalBounties, us.TotalUpvotes, us.TotalDownvotes, CASE WHEN us.Reputation
// >= 1000 THEN 'High Repute' WHEN us.Reputation >= 500 THEN 'Medium Repute' ELSE 'Low Repute' END AS UserReputationCategory FROM RankedPosts rp JOIN UserStats us ON
// rp.OwnerUserId = us.UserId WHERE rp.Rank = 1 ) SELECT pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.DisplayName, pd.Reputation, pd.TotalBounties, pd.TotalUpvotes,
// pd.TotalDownvotes, pd.UserReputationCategory, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pd.PostId) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId =
// pd.PostId AND v.VoteTypeId = 2) AS UpvoteCount, CASE WHEN pd.Score >= 10 THEN 'Popular' WHEN pd.Score BETWEEN 1 AND 9 THEN 'Moderately Popular' ELSE 'Unpopular' END AS
// PopularityCategory FROM PostDetails pd WHERE pd.TotalBounties > 0 OR pd.TotalUpvotes > 0 ORDER BY pd.Score DESC, pd.CreationDate ASC LIMIT 100;
fn q3081(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt()).fold([0i64; 3], |a, v| match v {
        Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let cc = (&first).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uc = (&first).group_by(Ident::<Post>::new()).select(votes_of(db).select(vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64);
    let v = drain((&first).select(owner_user.select(Ident::<User>::new().and((&us).filt(|a| a[0] > 0 || a[1] > 0)))).and(&cc).and(&uc));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 100);
    rows(v.into_iter().map(|(p, (((u, a), c), n))| {
        let rep = db.user.reputation.get(u).unwrap();
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(a.map(V::I));
        f.push(V::S(if rep >= 1000 { "High Repute" } else if rep >= 500 { "Medium Repute" } else { "Low Repute" }));
        f.extend([V::I(c), V::I(n), V::S(if s >= 10 { "Popular" } else if (1..=9).contains(&s) { "Moderately Popular" } else { "Unpopular" })]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
// p.CreationDate DESC) AS UserPostRank FROM Posts p WHERE p.PostTypeId = 1 ), PostStatistics AS ( SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount,
// u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COUNT(c.Id) AS CommentCount, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes
// FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN ( SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod'
// THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId
// ) v ON rp.PostId = v.PostId WHERE rp.UserPostRank <= 5 GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, u.DisplayName, u.Reputation, v.UpVotes,
// v.DownVotes ), TopPostStats AS ( SELECT ps.*, RANK() OVER (ORDER BY ps.Score DESC) AS ScoreRank, RANK() OVER (ORDER BY ps.ViewCount DESC) AS ViewRank FROM PostStatistics
// ps ) SELECT tps.PostId, tps.Title, tps.CreationDate, tps.Score, tps.ViewCount, tps.OwnerDisplayName, tps.OwnerReputation, tps.CommentCount, tps.UpVotes, tps.DownVotes,
// CASE WHEN tps.ScoreRank = 1 THEN 'Top Scored' WHEN tps.ViewRank = 1 THEN 'Most Viewed' ELSE 'Regular Post' END AS PostTypeIndicator FROM TopPostStats tps WHERE
// tps.OwnerReputation > 1000 ORDER BY tps.Score DESC, tps.ViewCount DESC LIMIT 10;
fn q34696(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let sr = ranked(drain((&rp).select(score)), |&(_, s)| Reverse(s), false);
    let sr = rel(sr.into_iter().map(|((p, _), r)| (p, r)).collect());
    let vr = ranked(drain((&rp).select(view_count.opt())), |&(_, w)| (w.is_none(), Reverse(w)), false);
    let vr = rel(vr.into_iter().map(|((p, _), r)| (p, r)).collect());
    let sr_of: HashIdx<Id<Post>, (Id<Post>, i64)> = (&sr).map(|(p, _)| p).inv().select(&sr).collect();
    let vr_of: HashIdx<Id<Post>, (Id<Post>, i64)> = (&vr).map(|(p, _)| p).inv().select(&vr).collect();
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain((&rp).with(owner_user.select(rich)).select((&ps).and((&pv).opt()).and((&sr_of).map(|(_, r)| r)).and((&vr_of).map(|(_, r)| r))));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (((c, a), s), w))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(if s == 1 { "Top Scored" } else if w == 1 { "Most Viewed" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, u.DisplayName AS Author, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0
// END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM
// Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 AND p.Score > 0
// GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.OwnerUserId ), ClosedPosts AS ( SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName, ph.Comment, COUNT(*) AS
// CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate, ph.UserDisplayName, ph.Comment ), PostWithClosure AS ( SELECT
// rp.PostId, rp.Title, rp.Author, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, COALESCE(cp.CloseCount, 0) AS CloseCount, CASE WHEN COALESCE(cp.CloseCount, 0)
// > 0 THEN 'Closed' ELSE 'Active' END AS PostStatus FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId ) SELECT pwc.PostId, pwc.Title, pwc.Author,
// pwc.CreationDate, pwc.CommentCount, pwc.UpVotes, pwc.DownVotes, pwc.CloseCount, pwc.PostStatus, CASE WHEN pwc.PostStatus = 'Closed' THEN 'This post is closed with ' ||
// CAST(pwc.CloseCount AS VARCHAR) || ' close votes' ELSE 'This post is active and has received ' || CAST(pwc.CommentCount AS VARCHAR) || ' comments' END AS StatusMessage
// FROM PostWithClosure pwc WHERE pwc.UpVotes > pwc.DownVotes ORDER BY pwc.CreationDate DESC, pwc.UpVotes DESC;
fn q20039(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post.and(hd).and(user_display_name.opt()).and(comment.opt()))
        .select(Ident::<PostHistory>::new())
        .fold(0i64, |n, _| n + 1);
    let cv = rel(drain(&cp));
    type K = (((Id<Post>, i64), Option<Str>), Option<Str>);
    let cp_of: HashIdx<Id<Post>, (K, i64)> = (&cv).map(|((((p, _), _), _), _)| p).inv().select(&cv).collect();
    let v = drain((&rp).filt(|a| a[1] > a[2]).and((&cp_of).map(|(_, n)| n).opt()));
    rows(v.into_iter().map(|(p, (a, n))| {
        let n = n.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend(a.map(V::I));
        f.extend([V::I(n), V::S(if n > 0 { "Closed" } else { "Active" })]);
        f.push(V::Owned(if n > 0 { format!("This post is closed with {n} close votes") } else { format!("This post is active and has received {} comments", a[0]) }));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC)
// AS ViewRank, COUNT(c.Id) AS CommentsCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.PostTypeId
// ), PostHistoryAggregates AS ( SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount, COUNT(CASE WHEN ph.PostHistoryTypeId IN
// (12, 13) THEN 1 END) AS DeleteUndeleteCount, MAX(ph.CreationDate) AS LastHistoryDate FROM PostHistory ph GROUP BY ph.PostId ), UserReputations AS ( SELECT u.Id AS UserId,
// AVG(u.Reputation) AS AverageReputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id ) SELECT rp.PostId, rp.Title,
// rp.CreationDate, rp.ViewCount, rp.CommentsCount, hp.CloseReopenCount, hp.DeleteUndeleteCount, ur.AverageReputation, ur.BadgeCount, CASE WHEN ur.AverageReputation IS NULL
// THEN 'No Reputation Data' WHEN ur.AverageReputation BETWEEN 0 AND 100 THEN 'New User' WHEN ur.AverageReputation BETWEEN 101 AND 500 THEN 'Intermediate User' WHEN
// ur.AverageReputation > 500 THEN 'Experienced User' END AS UserReputationCategory FROM RankedPosts rp LEFT JOIN PostHistoryAggregates hp ON rp.PostId = hp.PostId LEFT JOIN
// Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserReputations ur ON u.Id = ur.UserId WHERE (rp.ViewRank <= 5 AND rp.ViewCount > 100) OR (hp.CloseReopenCount > 0) ORDER BY
// rp.ViewCount DESC, rp.Title ASC LIMIT 20 OFFSET 10;
//
// The WHERE reads only ViewRank, ViewCount and CloseReopenCount, so the qualifying posts are picked first and the comment x vote product is driven for those alone.
fn q24297(db: &'static So) -> String {
    let Post { post_type_id, view_count, owner_user, title, .. } = &db.post;
    let top = top_per(drain(db.post.select(post_type_id)), |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 5, false);
    let top5: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let hp = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([0i64, 0, i64::MIN], |a, (t, d)| [a[0] + matches!(t, 10 | 11) as i64, a[1] + matches!(t, 12 | 13) as i64, a[2].max(d)]);
    let picked: MatSet<Id<Post>> = (&top5).with(view_count.gt(100)).select(Ident::<Post>::new()).union(db.post.with((&hp).filt(|a| a[0] > 0)).select(Ident::<Post>::new())).collect();
    let cc = (&picked).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&cc).and((&hp).opt()).and(owner_user.select(Ident::<User>::new().and(&ur)).opt()));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        let t = title.get(p);
        (w.is_none(), Reverse(w), t.is_none(), t, p)
    }, 30);
    rows(v.into_iter().skip(10).map(|(p, ((c, h), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.push(V::I(c));
        f.extend(match h {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.extend(match u {
            Some((u, b)) => {
                let r = db.user.reputation.get(u).unwrap();
                [V::F(r as f64), V::I(b), V::S(if (0..=100).contains(&r) { "New User" } else if (101..=500).contains(&r) { "Intermediate User" } else if r > 500 { "Experienced User" } else { "" })]
            }
            None => [V::Null, V::Null, V::S("No Reputation Data")],
        });
        row(f)
    }))
}

// WITH RECURSIVE RecursivePostHierarchy AS ( SELECT p.Id AS PostId, p.Title AS PostTitle, p.OwnerUserId, p.PostTypeId, 0 AS Level FROM Posts p WHERE p.PostTypeId = 1 UNION
// ALL SELECT a.Id AS PostId, a.Title AS PostTitle, a.OwnerUserId, a.PostTypeId, Level + 1 FROM Posts a INNER JOIN RecursivePostHierarchy q ON a.ParentId = q.PostId WHERE
// a.PostTypeId = 2 ), PostStats AS ( SELECT p.Id, p.Title, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId =
// 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(v.VoteTypeId), 0) AS TotalVotes, p.ViewCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title,
// p.ViewCount ), UserBadgeCounts AS ( SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId ), PostHistoryDetails AS ( SELECT ph.PostId,
// MAX(ph.CreationDate) AS LatestEditDate, MAX(ph.PostHistoryTypeId) AS LatestEditTypeId FROM PostHistory ph GROUP BY ph.PostId ) SELECT p.Id AS PostId, p.Title,
// u.DisplayName AS OwnerDisplayName, ph.LatestEditDate, ph.LatestEditTypeId, ps.UpVotes, ps.DownVotes, ps.TotalVotes, ps.ViewCount, ub.BadgeCount, r.Level AS AnswerLevel
// FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN PostStats ps ON p.Id = ps.Id LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN
// PostHistoryDetails ph ON p.Id = ph.PostId LEFT JOIN RecursivePostHierarchy r ON r.PostId = p.Id WHERE p.PostTypeId = 1 AND (ph.LatestEditTypeId IS NULL OR
// ph.LatestEditDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month') AND (ps.ViewCount > 100 OR r.Level > 0) ORDER BY ps.TotalVotes DESC, ps.ViewCount DESC LIMIT 50;
//
// The recursive step only adds answers (`a.PostTypeId = 2`) and r is joined to questions (`p.PostTypeId = 1`), so only the base case (Level 0) is ever read.
fn q33356(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let ps = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.unwrap_or(0)]
    });
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.group_by(post).select(hd.and(post_history_type_id)).fold((i64::MIN, i64::MIN), |(d, t), (x, y)| (d.max(x), t.max(y)));
    let since = add_months(ts(2024, 10, 1, 12, 34, 56), -1);
    type R = ([i64; 3], Option<(i64, i64)>);
    let v = drain(db.post.with(post_type_id.eq(1)).with(view_count.gt(100)).select((&ps).and((&ph).opt())).filt(move |(_, h): R| h.map_or(true, |(d, _)| d > since)).and(owner_user.select(&ub).opt()));
    let v = top_n(v, |&(p, ((a, _), _))| {
        let w = view_count.get(p);
        (Reverse(a[2]), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, ((a, h), b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend(match h {
            Some((d, t)) => [V::T(d), V::I(t)],
            None => [V::Null, V::Null],
        });
        f.extend(a.map(V::I));
        f.extend([post_fields(db, p, &["views"]).remove(0), harness::fmt::oint(b), V::I(0)]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId
// ORDER BY p.CreationDate DESC) AS RN, p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1 ), AggregatedUserStats AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation,
// SUM(COALESCE(b.Class, 0)) AS TotalBadges, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews FROM Users u LEFT JOIN Badges b ON u.Id =
// b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation ), RecentVotes AS ( SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1
// END) AS TotalUpvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS TotalDownvotes FROM Votes v GROUP BY v.PostId ), PostStatusHistory AS ( SELECT ph.PostId, MAX(CASE
// WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenedDate FROM PostHistory
// ph GROUP BY ph.PostId ) SELECT up.UserId, up.DisplayName, up.Reputation, up.TotalBadges, up.TotalScore, up.TotalViews, rp.PostId, rp.Title, rp.CreationDate AS
// QuestionDate, rp.Score AS QuestionScore, rp.ViewCount AS QuestionViewCount, rp.AnswerCount, rp.CommentCount, COALESCE(rv.TotalUpvotes, 0) AS TotalUpvotes,
// COALESCE(rv.TotalDownvotes, 0) AS TotalDownvotes, ph.ClosedDate, ph.ReopenedDate FROM AggregatedUserStats up LEFT JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId AND
// rp.RN <= 3 LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN PostStatusHistory ph ON rp.PostId = ph.PostId WHERE up.Reputation > 1000 ORDER BY up.Reputation
// DESC, rp.CreationDate DESC;
fn q32751(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let up = users()
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(score.and(view_count.opt())).opt()))
        .fold([0i64; 3], |a, (b, p)| {
            let (s, w) = p.map_or((0, None), |(s, w)| (s, w));
            [a[0] + b.unwrap_or(0), a[1] + s, a[2] + w.unwrap_or(0)]
        });
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold((i64::MIN, i64::MIN), |(c, r), (t, d)| (if t == 10 { c.max(d) } else { c }, if t == 11 { r.max(d) } else { r }));
    let v = drain((&up).and(posts_of(db).select(Ident::<Post>::new().with(&rp).and((&rv).opt()).and((&ph).opt())).opt()));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        match p {
            Some(((p, r), h)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]));
                let r = r.unwrap_or([0; 2]);
                f.extend([V::I(r[0]), V::I(r[1])]);
                let (c, o) = h.unwrap_or((i64::MIN, i64::MIN));
                f.extend([tmax(c), tmax(o)]);
            }
            None => {
                f.extend((0..7).map(|_| V::Null));
                f.extend([V::I(0), V::I(0), V::Null, V::Null]);
            }
        }
        row(f)
    }))
}

// Rewritten (rewrites/26510.sql): the ROW_NUMBER order is tie-broken on p.Id.
// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC,
// p.ViewCount DESC, p.Id) AS Rank, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId =
// 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' ), TopPosts AS ( SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.OwnerReputation,
// rp.Score, rp.ViewCount, rp.Tags FROM RankedPosts rp WHERE rp.Rank <= 5 ), TagStatistics AS ( SELECT t.TagName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(p.ViewCount) AS
// TotalViews, AVG(p.Score) AS AverageScore FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' WHERE p.PostTypeId = 1 GROUP BY t.TagName ), UserStatistics AS (
// SELECT u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AverageViews FROM Users u JOIN Posts p ON u.Id =
// p.OwnerUserId WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.DisplayName ) SELECT tp.Title,
// tp.OwnerDisplayName, tp.OwnerReputation, tp.Score, tp.ViewCount, ts.TagName, ts.QuestionCount AS TagQuestionCount, ts.TotalViews AS TagTotalViews, ts.AverageScore AS
// TagAverageScore, us.QuestionCount AS UserQuestionCount, us.TotalScore AS UserTotalScore, us.AverageViews AS UserAverageViews FROM TopPosts tp LEFT JOIN TagStatistics ts
// ON tp.Tags LIKE '%' || ts.TagName || '%' LEFT JOIN UserStatistics us ON tp.OwnerDisplayName = us.DisplayName ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q26510(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let recent = || db.post.with(post_type_id.eq(1).and(creation_date.ge(since))).with(owner_user);
    let top = top_per(drain(recent().select(owner_user)), |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let lt = tag_mentions(db);
    let q_by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).with(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p).select(post_type_id.eq(1))).map(|(_, t)| t).inv().collect();
    let ts_ = db.tag.group_by(Ident::<Tag>::new()).select((&q_by_tag).map(|(p, _)| p).select(view_count.opt().and(score))).fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let tags_of: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&lt).map(|(p, _)| p).inv().collect();
    let us = recent().group_by(owner_user.select(&db.user.display_name)).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let v = drain((&tp).select((&tags_of).map(|(_, t)| t).select(Ident::<Tag>::new().and(&ts_)).opt().and(owner_user.select(&db.user.display_name).select(&us).opt())));
    rows(v.into_iter().map(|(p, (t, u))| {
        let mut f = post_fields(db, p, &["title", "owner", "rep", "score", "views"]);
        f.extend(match t {
            Some((t, a)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match u {
            Some(a) => [V::I(a[0]), V::I(a[1]), avg(a[3], a[2])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeCounts AS ( SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS
// SilverBadges, SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId ), PostActivity AS ( SELECT OwnerUserId, COUNT(CASE WHEN PostTypeId =
// 1 THEN 1 END) AS QuestionsAsked, COUNT(CASE WHEN PostTypeId = 2 THEN 1 END) AS AnswersGiven, SUM(ViewCount) AS TotalViews, SUM(Score) AS TotalScore FROM Posts GROUP BY
// OwnerUserId ), RecentPostHistory AS ( SELECT PostId, UserId, MAX(CreationDate) AS MostRecentEdit FROM PostHistory GROUP BY PostId, UserId ), TopUsers AS ( SELECT U.Id,
// U.DisplayName, U.Reputation, COALESCE(UBC.BadgeCount, 0) AS BadgeCount, COALESCE(PA.QuestionsAsked, 0) AS QuestionsAsked, COALESCE(PA.AnswersGiven, 0) AS AnswersGiven,
// COALESCE(PA.TotalViews, 0) AS TotalViews, COALESCE(PA.TotalScore, 0) AS TotalScore, CASE WHEN U.LastAccessDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1
// year' THEN 'Inactive' ELSE 'Active' END AS UserStatus FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN PostActivity PA ON U.Id = PA.OwnerUserId )
// SELECT TU.DisplayName, TU.Reputation, TU.BadgeCount, TU.QuestionsAsked, TU.AnswersGiven, TU.TotalViews, TU.TotalScore, TU.UserStatus, ARRAY_AGG(RPH.UserId) AS
// RecentEditors FROM TopUsers TU LEFT JOIN RecentPostHistory RPH ON TU.Id = RPH.UserId GROUP BY TU.DisplayName, TU.Reputation, TU.BadgeCount, TU.QuestionsAsked,
// TU.AnswersGiven, TU.TotalViews, TU.TotalScore, TU.UserStatus ORDER BY TU.Reputation DESC, TU.QuestionsAsked DESC LIMIT 10;
fn q3468(db: &'static So) -> String {
    let ubc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let pa = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 4], |a, ((t, w), s)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    type T = (Option<i64>, Option<[i64; 4]>);
    let tu = db.user.select((&ubc).opt().and((&pa).opt()).and((&db.user.last_access_date).map(move |d| d < since))).map(|((b, a), s): (T, bool)| (b.unwrap_or(0), a.unwrap_or([0; 4]), s));
    let PostHistory { post, user, .. } = &db.post_history;
    let rph = db.post_history.with(user).group_by(post.and(user)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rv = rel(drain(&rph));
    let rph_of: HashIdx<Id<User>, ((Id<Post>, Id<User>), i64)> = (&rv).map(|((_, u), _)| u).inv().select(&rv).collect();
    let g = db
        .user
        .group_by((&db.user.display_name).and(&db.user.reputation).and(tu))
        .select((&rph_of).map(|((_, u), _)| u).select(&db.user.origid).opt())
        .buf_fold(|xs| -> &'static [Option<i64>] { Box::leak(xs.iter().copied().collect::<Vec<_>>().into_boxed_slice()) });
    let v = top_n(drain(&g), |&(((n, r), (_, a, _)), _)| (Reverse(r), Reverse(a[0]), n), 10);
    rows(v.into_iter().map(|(((n, r), (b, a, s)), l)| {
        let mut f = vec![V::S(n), V::I(r), V::I(b)];
        f.extend(a.map(V::I));
        f.push(V::S(if s { "Inactive" } else { "Active" }));
        f.push(V::L(l.iter().map(|&x| harness::fmt::oint(x)).collect()));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
// COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(co.FavoriteCount, 0) AS FavoriteCount, p.OwnerUserId FROM Posts p LEFT JOIN ( SELECT PostId, COUNT(*) AS
// CommentCount FROM Comments GROUP BY PostId ) c ON p.Id = c.PostId LEFT JOIN ( SELECT PostId, COUNT(*) AS FavoriteCount FROM Votes WHERE VoteTypeId = 5 GROUP BY PostId )
// co ON p.Id = co.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' ), FilteredPosts AS ( SELECT rp.PostId, rp.Title, rp.CreationDate,
// rp.Score, rp.CommentCount, rp.FavoriteCount, rp.OwnerUserId FROM RankedPosts rp WHERE rp.PostRank <= 5 ), UserStatistics AS ( SELECT u.Id AS UserId, u.DisplayName,
// u.Reputation, SUM(fp.CommentCount) AS TotalComments, SUM(fp.FavoriteCount) AS TotalFavorites, COUNT(fp.PostId) AS TotalPosts FROM Users u LEFT JOIN FilteredPosts fp ON
// u.Id = fp.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation HAVING SUM(fp.CommentCount) > 10 OR SUM(fp.FavoriteCount) > 10 ) SELECT us.UserId, us.DisplayName,
// us.Reputation, us.TotalComments, us.TotalFavorites, us.TotalPosts, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty FROM UserStatistics us LEFT JOIN Votes v ON us.UserId =
// v.UserId WHERE v.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY us.UserId, us.DisplayName, us.Reputation, us.TotalComments,
// us.TotalFavorites, us.TotalPosts ORDER BY us.Reputation DESC, us.TotalFavorites DESC LIMIT 10;
fn q23619(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let top = top_per(drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let fp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let fav = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(5)));
    let fc = (&fp).group_by(Ident::<Post>::new()).select(fav.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&cc).and(&fc))).fold([0i64; 3], |a, (c, f)| [a[0] + c, a[1] + f, a[2] + 1]);
    let Vote { creation_date: vd, bounty_amount, .. } = &db.vote;
    let bounty = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(Ident::<Vote>::new().with(vd.ge(add_months(t0, -6)))).select(bounty_amount.opt())).fold(0i64, |s, b| s + b.unwrap_or(0));
    let v = drain((&us).filt(|a| a[0] > 10 || a[1] > 10).and(&bounty));
    let v = top_n(v, |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b)]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputationChanges AS ( SELECT U.Id AS UserId, U.Reputation, PH.CreationDate, PH.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY
// PH.CreationDate DESC) AS RowNum FROM Users U JOIN PostHistory PH ON U.Id = PH.UserId WHERE PH.PostHistoryTypeId IN (10, 11, 12, 13) ), AggregatedReputation AS ( SELECT
// U.Id AS UserId, U.DisplayName, SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN -U.Reputation * 0.1 WHEN PH.PostHistoryTypeId = 11 THEN U.Reputation * 0.15 ELSE 0 END) AS
// TotalReputationChange FROM Users U JOIN PostHistory PH ON U.Id = PH.UserId WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY U.Id, U.DisplayName HAVING SUM(CASE WHEN
// PH.PostHistoryTypeId = 10 THEN -1 WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) > 0 ), UserBadges AS ( SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN
// B.Class = 1 THEN 1 WHEN B.Class = 2 THEN 2 WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BadgeScore FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id ),
// UserActivity AS ( SELECT U.Id AS UserId, COUNT(P.Id) AS PostCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties FROM Users U LEFT JOIN Posts P ON U.Id =
// P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id ), FinalResults AS ( SELECT U.Id AS UserId, U.DisplayName, COALESCE(URC.TotalReputationChange, 0) +
// COALESCE(UB.BadgeScore, 0) AS FinalScore, UA.PostCount, UA.TotalBounties FROM Users U LEFT JOIN AggregatedReputation URC ON U.Id = URC.UserId LEFT JOIN UserBadges UB ON
// U.Id = UB.UserId LEFT JOIN UserActivity UA ON U.Id = UA.UserId ) SELECT UserId, DisplayName, FinalScore, PostCount, TotalBounties FROM FinalResults WHERE FinalScore > 0
// ORDER BY FinalScore DESC, TotalBounties DESC LIMIT 10;
//
// WITH RECURSIVE, but no CTE refers to itself. The DECIMAL reputation change is summed in hundredths.
fn q31145(db: &'static So) -> String {
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let rep = &db.user.reputation;
    let ar = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(user)
        .select(post_history_type_id.and(user.select(rep)))
        .fold([0i64; 2], |a, (t, r)| [a[0] + if t == 10 { -10 * r } else { 15 * r }, a[1] + if t == 10 { -1 } else { 1 }]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + match c {
        Some(1) | Some(3) => 1,
        Some(2) => 2,
        _ => 0,
    });
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).opt())
        .fold([0i64; 2], |a, p| match p {
            Some(v) => [a[0] + 1, a[1] + v.flatten().unwrap_or(0)],
            None => a,
        });
    type R = ((Option<[i64; 2]>, i64), [i64; 2]);
    let score = |(a, b): (Option<[i64; 2]>, i64)| a.filter(|a| a[1] > 0).map_or(0, |a| a[0]) + 100 * b;
    let v = drain(db.user.select((&ar).opt().and(&ub).and(&ua)).filt(move |(x, _): R| score(x) > 0));
    let v = top_n(v, |&(u, (x, a))| (Reverse(score(x)), Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (x, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::F(score(x) as f64 / 100.0), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserVoteSummary AS ( SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN
// v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END), 0) AS TotalVotes FROM Users u LEFT JOIN Votes
// v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName ), PostSummary AS ( SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, COALESCE(pc.CommentCount, 0) AS CommentCount,
// COALESCE(pe.EditCount, 0) AS EditCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p LEFT JOIN ( SELECT PostId, COUNT(*) AS
// CommentCount FROM Comments GROUP BY PostId ) pc ON p.Id = pc.PostId LEFT JOIN ( SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE PostHistoryTypeId IN ('5',
// '24') GROUP BY PostId ) pe ON p.Id = pe.PostId ), TopPosts AS ( SELECT ps.PostId, ps.Title, ps.ViewCount, ps.Score, ps.CommentCount, ps.EditCount, RANK() OVER (ORDER BY
// ps.ViewCount DESC) AS ViewRank FROM PostSummary ps WHERE ps.RowNum = 1 ) SELECT ups.UserId, ups.DisplayName, tp.Title, tp.ViewCount, tp.Score, ups.TotalVotes, CASE WHEN
// ups.TotalVotes > 100 THEN 'Popular' WHEN ups.TotalVotes BETWEEN 51 AND 100 THEN 'Moderate' ELSE 'Less Popular' END AS PopularityCategory FROM UserVoteSummary ups JOIN
// Votes v ON ups.UserId = v.UserId JOIN TopPosts tp ON tp.PostId = v.PostId WHERE (tp.EditCount > 5 OR tp.CommentCount > 10) AND (SELECT COUNT(*) FROM Votes WHERE PostId =
// tp.PostId AND VoteTypeId = 2) > 10 ORDER BY ups.TotalVotes DESC, tp.ViewCount DESC;
fn q2197(db: &'static So) -> String {
    let Vote { user, post, vote_type_id, .. } = &db.vote;
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id).opt()).fold(0i64, |n, t| n + matches!(t, Some(2 | 3)) as i64);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post: hp, post_history_type_id, .. } = &db.post_history;
    let ec = db.post_history.with(post_history_type_id.is_in([5, 24])).group_by(hp).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let up = db.vote.with(vote_type_id.eq(2)).group_by(post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    type R = ((Option<i64>, Option<i64>), Option<i64>);
    let tp: MatSet<Id<Post>> = db.post.with(Ident::<Post>::new().select((&ec).opt().and((&cc).opt()).and((&up).opt())).filt(|((e, c), u): R| (e.unwrap_or(0) > 5 || c.unwrap_or(0) > 10) && u.unwrap_or(0) > 10)).select(Ident::<Post>::new()).collect();
    let v = drain(db.vote.select(user.select(Ident::<User>::new().and(&uvs)).and(post.select(Ident::<Post>::new().with(&tp)))));
    rows(v.into_iter().map(|(_, ((u, n), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(n), V::S(if n > 100 { "Popular" } else if (51..=100).contains(&n) { "Moderate" } else { "Less Popular" })]);
        row(f)
    }))
}

// WITH UserScore AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN
// 1 ELSE 0 END) AS Downvotes, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes
// V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation ), TopUsers AS ( SELECT UserId, DisplayName, Reputation, Upvotes,
// Downvotes, PostCount, CommentCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC, Upvotes DESC) AS Rank FROM UserScore ), PostStats AS ( SELECT P.Id AS PostId, P.Title,
// P.CreationDate, P.Score, COUNT(C.Id) AS TotalComments, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0
// END) AS TotalDownvotes FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.Score ),
// TopPosts AS ( SELECT PostId, Title, CreationDate, Score, TotalComments, TotalUpvotes, TotalDownvotes, ROW_NUMBER() OVER (ORDER BY Score DESC, TotalUpvotes DESC) AS Rank
// FROM PostStats ) SELECT U.DisplayName AS TopUser, U.Reputation, U.Upvotes AS UserUpvotes, U.Downvotes AS UserDownvotes, U.PostCount AS UserPostCount, U.CommentCount AS
// UserCommentCount, P.Title AS TopPost, P.CreationDate AS PostCreationDate, P.Score AS PostScore, P.TotalComments AS PostCommentCount, P.TotalUpvotes AS PostTotalUpvotes,
// P.TotalDownvotes AS PostTotalDownvotes FROM TopUsers U JOIN TopPosts P ON U.Rank = 1 AND P.Rank = 1 WHERE U.UserId = P.PostId ORDER BY U.Reputation DESC;
//
// `U.UserId = P.PostId` compares a user id with a post id, so it goes through the raw ids.
fn q6591(db: &'static So) -> String {
    let vt = || votes_of(db).select(&db.vote.vote_type_id).opt();
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(vt().and(comments_of(db).opt())).opt()).fold([0i64; 2], |a, p| match p {
        Some((t, _)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
        None => a,
    });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tu = top_n(drain((&us).and(&np).and(&nc)), |&(u, ((a, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), u), 1);
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(vt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tp = top_n(drain(&ps), |&(p, a)| (Reverse(db.post.score.get(p).unwrap()), Reverse(a[1]), p), 1);
    let (tu, tp) = (rel(tu), rel(tp));
    let v = drain((&tu).cross(&tp).filt(|((u, _), (p, _))| db.user.origid.get(u) == db.post.origid.get(p)));
    rows(v.into_iter().map(|(_, ((u, ((a, n), c)), (p, s)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(c)]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend(s.map(V::I));
        row(f)
    }))
}

// WITH TagStats AS ( SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount, COUNT(DISTINCT CASE
// WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount, SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN
// v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY
// t.TagName ), UserStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY u.Id,
// u.DisplayName ), TopTags AS ( SELECT ts.TagName, ts.PostCount, ts.QuestionCount, ts.AnswerCount, ts.TotalViews, ts.TotalUpVotes, ts.TotalDownVotes, ROW_NUMBER() OVER
// (ORDER BY ts.PostCount DESC) AS TagRank FROM TagStats ts ), TopUsers AS ( SELECT us.UserId, us.DisplayName, us.TotalPosts, us.UpVotes, us.DownVotes, ROW_NUMBER() OVER
// (ORDER BY us.UpVotes DESC) AS UserRank FROM UserStats us ) SELECT tt.TagName, tt.PostCount AS TagPostCount, tt.QuestionCount AS TagQuestionCount, tt.AnswerCount AS
// TagAnswerCount, tt.TotalViews AS TagTotalViews, tt.TotalUpVotes AS TagTotalUpVotes, tt.TotalDownVotes AS TagTotalDownVotes, tu.DisplayName AS TopUserDisplayName,
// tu.TotalPosts AS UserTotalPosts, tu.UpVotes AS UserUpVotes, tu.DownVotes AS UserDownVotes FROM TopTags tt JOIN TopUsers tu ON tt.TagRank = 1 AND tu.UserRank = 1 WHERE
// tt.PostCount > 0 ORDER BY tt.TotalViews DESC, tt.TagName;
//
// TagRank reads only PostCount, so the top tag is picked first and its posts x votes product is driven for it alone.
fn q29971(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let pc = db.tag.group_by(Ident::<Tag>::new()).select((&by_tag).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tt = rel(top_n(drain(&pc), |&(t, n)| (Reverse(n), t), 1));
    let Post { post_type_id, view_count, .. } = &db.post;
    let vt = || votes_of(db).select(&db.vote.vote_type_id).opt();
    let top: MatSet<Id<Tag>> = (&tt).map(|(t, _)| t).collect();
    let agg = (&top)
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select(post_type_id.and(view_count.opt()).and(vt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((_, w), v)) => [a[0], a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let qa = (&top).group_by(Ident::<Tag>::new()).select((&by_tag).map(|(p, _)| p).select(post_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64]);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(vt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(t) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
        None => a,
    });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = rel(top_n(drain((&us).and(&np)), |&(u, (a, _))| (Reverse(a[0]), u), 1));
    let tg = rel(drain((&pc).filt(|n| n > 0).and(&agg).and((&qa).opt())));
    let v = drain((&tg).cross(&tu));
    rows(v.into_iter().map(|(_, ((t, ((n, a), q)), (u, (b, m))))| {
        let q = q.unwrap_or([0; 2]);
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(n), V::I(q[0]), V::I(q[1]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4])];
        f.extend([user_col(db, u, "name"), V::I(m), V::I(b[0]), V::I(b[1])]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE
// WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(p.Score) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ),
// BadgeCounts AS ( SELECT UserId, COUNT(*) AS TotalBadges, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS
// SilverBadges, SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId ), RecentPostHistory AS ( SELECT p.Id AS PostId, p.Title,
// ph.CreationDate, ph.Comment, ph.UserDisplayName, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS rn, p.OwnerUserId AS UserId FROM Posts p JOIN
// PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11) ), TopUsers AS ( SELECT ups.UserId, ups.DisplayName, ups.PostCount, ups.Questions, ups.Answers,
// ups.TotalScore, COALESCE(bc.TotalBadges, 0) AS BadgeCount, COALESCE(bc.GoldBadges, 0) AS GoldBadges, COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges FROM UserPostStats ups LEFT JOIN BadgeCounts bc ON ups.UserId = bc.UserId WHERE ups.TotalScore > 100 ) SELECT tu.DisplayName,
// tu.PostCount, tu.Questions, tu.Answers, tu.TotalScore, tu.BadgeCount, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, rp.Title AS LastPostTitle, COALESCE(rp.Comment, 'No
// recent activity') AS RecentComment, rp.CreationDate AS RecentActivityDate FROM TopUsers tu LEFT JOIN RecentPostHistory rp ON tu.UserId = rp.UserId AND rp.rn = 1 ORDER BY
// tu.TotalScore DESC, tu.PostCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q32058(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s],
        None => a,
    });
    let bc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let last = top_per(drain(db.post_history.with(post_history_type_id.is_in([10, 11])).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let lr = rel(last.into_iter().map(|(h, _)| h).collect());
    let by_owner: HashIdx<Id<User>, Id<PostHistory>> = (&lr).select(post.select(owner_user)).inv().select(&lr).collect();
    let v = drain((&ups).filt(|a| a[3] > 100).and((&bc).opt()).and((&by_owner).opt()));
    rows(v.into_iter().map(|(u, ((a, b), h))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        match h {
            Some(h) => {
                f.extend(post_fields(db, post.get(h).unwrap(), &["title"]));
                f.extend([V::S(db.post_history.comment.get(h).unwrap_or("No recent activity")), V::T(hd.get(h).unwrap())]);
            }
            None => f.extend([V::Null, V::S("No recent activity"), V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
// p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1 ), UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(rp.PostId) AS QuestionCount,
// SUM(rp.Score) AS TotalScore, SUM(rp.ViewCount) AS TotalViews, AVG(rp.Score) AS AverageScore, MAX(rp.CreationDate) AS LastQuestionDate FROM Users u LEFT JOIN RankedPosts
// rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName ), BadgeStatistics AS ( SELECT b.UserId, COUNT(b.Id) AS BadgeCount, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS
// GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId ),
// CombinedStats AS ( SELECT ups.UserId, ups.DisplayName, COALESCE(ups.QuestionCount, 0) AS QuestionCount, COALESCE(ups.TotalScore, 0) AS TotalScore,
// COALESCE(ups.TotalViews, 0) AS TotalViews, COALESCE(ups.AverageScore, 0.0) AS AverageScore, COALESCE(bs.BadgeCount, 0) AS BadgeCount, COALESCE(bs.GoldBadges, 0) AS
// GoldBadges, COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges, DENSE_RANK() OVER (ORDER BY COALESCE(ups.QuestionCount, 0) DESC,
// COALESCE(ups.TotalScore, 0) DESC) AS UserRank FROM UserPostStats ups FULL OUTER JOIN BadgeStatistics bs ON ups.UserId = bs.UserId ), TopUsers AS ( SELECT * FROM
// CombinedStats WHERE UserRank <= 10 ) SELECT cu.DisplayName, cu.QuestionCount, cu.TotalScore, cu.TotalViews, cu.AverageScore, cu.BadgeCount, cu.GoldBadges,
// cu.SilverBadges, cu.BronzeBadges FROM TopUsers cu ORDER BY cu.QuestionCount DESC, cu.TotalScore DESC;
//
// Every badge's UserId is a user, so the FULL OUTER JOIN adds no rows beyond the LEFT JOIN from Users.
fn q31004(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(score.and(view_count.opt()).and(creation_date)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((s, w), d)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4].max(d)],
            None => a,
        });
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = ranked(drain((&ups).and((&bs).opt())), |&(_, (a, _))| (Reverse(a[0]), Reverse(a[1])), true);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, b)), _)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[3]), if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }];
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id =
// B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation ), TopUsers AS ( SELECT UserId, DisplayName, Reputation, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, RANK()
// OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation ), PopularPosts AS ( SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, P.CreationDate,
// P.OwnerUserId, U.DisplayName AS OwnerDisplayName, RANK() OVER (ORDER BY P.ViewCount DESC) AS PopularityRank FROM Posts P INNER JOIN Users U ON P.OwnerUserId = U.Id WHERE
// P.PostTypeId = 1 ), UserPostStats AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS
// AcceptedAnswers, SUM(CASE WHEN V.UserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id =
// V.PostId AND V.VoteTypeId = 2 GROUP BY U.Id, U.DisplayName ) SELECT TU.DisplayName AS TopUser, TU.Reputation AS Reputation, TU.BadgeCount AS TotalBadges, PU.Title AS
// PopularPost, PU.ViewCount AS PopularityViewCount, PU.Score AS PopularPostScore, UPS.TotalPosts AS UserTotalPosts, UPS.AcceptedAnswers AS UserAcceptedAnswers,
// UPS.TotalVotes AS UserTotalVotes FROM TopUsers TU JOIN PopularPosts PU ON PU.PopularityRank <= 10 JOIN UserPostStats UPS ON UPS.UserId = TU.UserId WHERE TU.ReputationRank
// <= 10 ORDER BY TU.Reputation DESC, PU.ViewCount DESC;
//
// The JOIN on `PU.PopularityRank <= 10` names only PU, so the top users are crossed with the popular questions.
fn q29780(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, owner_user, view_count, accepted_answer_id, .. } = &db.post;
    let pp = ranked(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(view_count.opt())), |&(_, w)| (w.is_none(), Reverse(w)), false);
    let pp = rel(pp.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect());
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).select((&db.vote.user).opt());
    let ups = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(accepted_answer_id.opt().and(up.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((x, v)) => [a[0] + 1, a[1] + x.is_some() as i64, a[2] + v.flatten().is_some() as i64],
        None => a,
    });
    let tur = rel(drain((&ub).and(&ups)));
    let v = drain((&tur).cross(&pp));
    rows(v.into_iter().map(|(_, ((u, (b, a)), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN
// p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount,
// AVG(COALESCE(p.Score, 0)) AS AvgScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT UserId, DisplayName,
// TotalPosts, QuestionCount, AnswerCount, WikiCount, AvgViewCount, AvgScore, RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts, RANK() OVER (ORDER BY AvgScore DESC) AS
// RankByScore FROM UserPostStats ), BadgeStats AS ( SELECT b.UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN
// b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId ), UserRankings AS ( SELECT
// tu.UserId, tu.DisplayName, tu.TotalPosts, tu.QuestionCount, tu.AnswerCount, tu.WikiCount, tu.AvgViewCount, tu.AvgScore, COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges, COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges, RANK() OVER (ORDER BY
// tu.RankByPosts, tu.RankByScore DESC) AS OverallRanking FROM TopUsers tu LEFT JOIN BadgeStats bs ON tu.UserId = bs.UserId ) SELECT UserId, DisplayName, TotalPosts,
// QuestionCount, AnswerCount, WikiCount, AvgViewCount, AvgScore, TotalBadges, GoldBadges, SilverBadges, BronzeBadges, OverallRanking FROM UserRankings WHERE TotalPosts > 0
// ORDER BY OverallRanking LIMIT 10;
//
// COALESCE inside AVG counts the one row a user with no posts keeps, so those users average 0.0 (translation-failures.md 12).
fn q29799(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, w), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + w.unwrap_or(0), a[5] + s],
        None => a,
    });
    let mean = |s: i64, n: i64| if n == 0 { 0.0 } else { s as f64 / n as f64 };
    let v = ranked(drain(&ups), |&(_, a)| Reverse(a[0]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(fkey(mean(a[5], a[0]))), false);
    let v = ranked(v, |&((_, rp), rs)| (rp, Reverse(rs)), false);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    type R = ((((Id<User>, [i64; 6]), i64), i64), i64);
    let ur = rel(v);
    let v = drain((&ur).filt(|((((_, a), _), _), _): R| a[0] > 0).select(Same::<R>::new().and(Same::<R>::new().map(|((((u, _), _), _), _): R| u).select((&bs).opt()))));
    let v = top_n(v, |&(_, (((_, _), o), _))| o, 10);
    rows(v.into_iter().map(|(_, (((((u, a), _), _), o), b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::F(mean(a[4], a[0])), V::F(mean(a[5], a[0]))]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.push(V::I(o));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.LastActivityDate, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY
// pt.Name ORDER BY p.CreationDate DESC) AS PostRank, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(v.BountyAmount) OVER (PARTITION BY p.Id) AS TotalBounty FROM
// Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
// AND v.VoteTypeId IN (8, 9) WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' ), RecentActivePosts AS ( SELECT PostId, Title, Score, CreationDate,
// LastActivityDate, OwnerDisplayName, PostRank, CommentCount, TotalBounty, (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = rp.PostId AND ph.CreationDate >=
// cast('2024-10-01' as date) - INTERVAL '1 year') AS EditCount FROM RankedPosts rp WHERE PostRank = 1 ), SelectedPosts AS ( SELECT PostId, Title, Score, CreationDate,
// LastActivityDate, OwnerDisplayName, CommentCount, TotalBounty, EditCount, CASE WHEN TotalBounty > 0 THEN 'Has Bounty' ELSE 'No Bounty' END AS BountyStatus FROM
// RecentActivePosts WHERE Score > 10 OR CommentCount > 5 ) SELECT sp.PostId, sp.Title, sp.Score, sp.CreationDate, sp.LastActivityDate, sp.OwnerDisplayName, sp.CommentCount,
// sp.TotalBounty, sp.EditCount, sp.BountyStatus, CASE WHEN EXISTS ( SELECT 1 FROM PostHistory ph WHERE ph.PostId = sp.PostId AND ph.PostHistoryTypeId IN (10, 11) HAVING
// COUNT(*) > 0 ) THEN 'Closed/Reopened' ELSE 'Open' END AS PostStatus FROM SelectedPosts sp ORDER BY sp.Score DESC, sp.LastActivityDate DESC LIMIT 100;
//
// PostRank numbers the post x comment x vote rows per post type by CreationDate, so rank 1 is one row of the newest post of the type;
// every row of a post carries the same window values, so which of its rows it is cannot be observed.
fn q31317(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, last_activity_date, .. } = &db.post;
    let since = add_years(date(2024, 10, 1), -1);
    let newest = top_per(drain(db.post.with(creation_date.ge(since)).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let np: MatSet<Id<Post>> = rel(newest.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let w = (&np).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 3], |a, (c, b)| {
        let b = b.flatten();
        [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
    });
    let PostHistory { creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let ec = (&np).group_by(Ident::<Post>::new()).select(history_of(db).select(hd.and(post_history_type_id)).opt()).fold([0i64; 2], |a, h| match h {
        Some((d, t)) => [a[0] + (d >= since) as i64, a[1] + matches!(t, 10 | 11) as i64],
        None => a,
    });
    type R = ((i64, [i64; 3]), [i64; 2]);
    let v = drain((&np).select(score.and(&w).and(&ec)).filt(|((s, a), _): R| s > 10 || a[0] > 5));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(last_activity_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((_, a), e))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "activity", "owner"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(e[0]), V::S(if a[1] > 0 && a[2] > 0 { "Has Bounty" } else { "No Bounty" }), V::S(if e[1] > 0 { "Closed/Reopened" } else { "Open" })]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, P.OwnerUserId, U.Reputation AS OwnerReputation, ROW_NUMBER() OVER (PARTITION
// BY P.PostTypeId ORDER BY P.Score DESC) AS Rank FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id ), TopScoringPosts AS ( SELECT PostId, Title, Score, ViewCount,
// CreationDate, OwnerReputation FROM RankedPosts WHERE Rank <= 5 ), PostAnalytics AS ( SELECT PS.PostId, PS.Title, PS.Score, PS.ViewCount, PS.OwnerReputation, COUNT(CM.Id)
// AS CommentCount, COUNT(V.Id) AS VoteCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1
// ELSE 0 END), 0) AS TotalDownvotes FROM TopScoringPosts PS LEFT JOIN Comments CM ON PS.PostId = CM.PostId LEFT JOIN Votes V ON PS.PostId = V.PostId GROUP BY PS.PostId,
// PS.Title, PS.Score, PS.ViewCount, PS.OwnerReputation ), UserBadges AS ( SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, MAX(B.Class) AS MaxBadgeClass FROM Users U LEFT
// JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id ), PostBadges AS ( SELECT PA.PostId, CASE WHEN UB.BadgeCount >= 5 THEN 'Gold Badge Holder' WHEN UB.BadgeCount >= 3 THEN
// 'Silver Badge Holder' WHEN UB.BadgeCount >= 1 THEN 'Bronze Badge Holder' ELSE 'No Badges' END AS BadgeStatus FROM PostAnalytics PA JOIN Users U ON PA.OwnerReputation =
// U.Reputation JOIN UserBadges UB ON U.Id = UB.UserId ) SELECT PA.PostId, PA.Title, PA.Score, PA.ViewCount, PA.CommentCount, PA.VoteCount, PA.TotalUpvotes,
// PA.TotalDownvotes, PB.BadgeStatus FROM PostAnalytics PA LEFT JOIN PostBadges PB ON PA.PostId = PB.PostId WHERE PA.Score > 0 ORDER BY PA.Score DESC, PA.ViewCount DESC;
//
// PostBadges joins on `PA.OwnerReputation = U.Reputation`, so a post meets every user with its owner's reputation.
fn q30576(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(owner_user).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pa = (&tp)
        .with(score.gt(0))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let by_rep: HashIdx<i64, Id<User>> = (&db.user.reputation).inv().collect();
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&pa).and(owner_user.select(&db.user.reputation).select(&by_rep).select(&ubc).opt()));
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(match b {
            Some(b) => V::S(if b >= 5 { "Gold Badge Holder" } else if b >= 3 { "Silver Badge Holder" } else if b >= 1 { "Bronze Badge Holder" } else { "No Badges" }),
            None => V::Null,
        });
        row(f)
    }))
}

// WITH UserBadgeCounts AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS
// SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName ), PostMetrics AS (
// SELECT P.Id AS PostId, P.OwnerUserId, P.PostTypeId, COUNT(C) AS CommentCount, SUM(V.BountyAmount) AS TotalBounty, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS
// UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id, P.OwnerUserId, P.PostTypeId ), TopUsersByPosts AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1
// ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(PM.UpVoteCount), 0) AS TotalUpVotes,
// COALESCE(SUM(PM.DownVoteCount), 0) AS TotalDownVotes, COALESCE(SUM(PM.CommentCount), 0) AS TotalComments, COALESCE(SUM(PM.TotalBounty), 0) AS TotalBountyEarned FROM Users
// U JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostMetrics PM ON P.Id = PM.PostId GROUP BY U.Id, U.DisplayName ), RankedUsers AS ( SELECT UserId, DisplayName,
// PostCount, QuestionCount, AnswerCount, TotalUpVotes, TotalDownVotes, TotalComments, TotalBountyEarned, ROW_NUMBER() OVER (ORDER BY PostCount DESC, TotalUpVotes DESC) AS
// UserRank FROM TopUsersByPosts ) SELECT RU.UserId, RU.DisplayName, RU.PostCount, RU.QuestionCount, RU.AnswerCount, RU.TotalUpVotes, RU.TotalDownVotes, RU.TotalComments,
// RU.TotalBountyEarned, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges FROM RankedUsers RU LEFT JOIN UserBadgeCounts UB ON RU.UserId = UB.UserId WHERE RU.UserRank <= 10
// ORDER BY RU.UserRank;
//
// COUNT(C) counts the whole row of C, which is never NULL, so it is the joined row count (translation-failures.md 11).
fn q25737(db: &'static So) -> String {
    let pm = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()))
        .fold([0i64; 5], |a, (_, v)| {
            let (t, b) = v.map_or((0, None), |(t, b)| (t, b));
            [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0), a[3] + (t == 2) as i64, a[4] + (t == 3) as i64]
        });
    let Post { post_type_id, .. } = &db.post;
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(&pm))).fold([0i64; 7], |a, (t, m)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + m[3], a[4] + m[4], a[5] + m[0], a[6] + if m[1] > 0 { m[2] } else { 0 }]
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64],
        None => a,
    });
    let v = top_n(drain((&tu).and(&ub)), |&(u, (a, _))| (Reverse(a[0]), Reverse(a[3]), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerUserId, DENSE_RANK() OVER (PARTITION BY
// p.OwnerUserId ORDER BY p.Score DESC) AS Rank, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank FROM Posts p WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0 ), UserBadges AS ( SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN
// b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id ), PostHistorySummary AS ( SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END)
// AS ClosureActions, COUNT(CASE WHEN ph.PostHistoryTypeId IN (1, 4, 6) THEN 1 END) AS EditActions FROM PostHistory ph GROUP BY ph.PostId ), MostActiveUsers AS ( SELECT u.Id
// AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, AVG(u.Reputation) AS AvgReputation FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName
// HAVING COUNT(v.Id) > 5 ) SELECT p.Title, p.CreationDate, r.Rank, b.BadgeCount, b.GoldBadges, b.SilverBadges, b.BronzeBadges, p.Score, p.ViewCount, p.AnswerCount,
// p.CommentCount, pah.ClosureActions, pah.EditActions, u.DisplayName AS OwnerDisplayName, mu.VoteCount, mu.AvgReputation FROM RankedPosts r JOIN Posts p ON r.PostId = p.Id
// JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserBadges b ON u.Id = b.UserId LEFT JOIN PostHistorySummary pah ON p.Id = pah.PostId LEFT JOIN MostActiveUsers mu ON u.Id
// = mu.UserId WHERE r.RecentPostRank <= 3 ORDER BY r.Rank, p.CreationDate DESC;
fn q34399(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user);
    let dr = ranked(drain(rp().select(owner_user)), |&(p, u)| (u, Reverse(score.get(p).unwrap())), true);
    let dr = rel(per_group(dr, |&(_, u)| u).into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&dr).map(|(p, _)| p).inv().select(&dr).collect();
    let recent = top_per(drain(rp().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let recent: MatSet<Id<Post>> = rel(recent.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + matches!(t, 1 | 4 | 6) as i64]);
    let mu = db.vote.group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&recent).select((&rank).map(|(_, r)| r).and(owner_user.select(Ident::<User>::new().and(&ub).and((&mu).filt(|n| n > 5).opt()))).and((&phs).opt())));
    rows(v.into_iter().map(|(p, ((r, ((u, b), m)), h))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.push(V::I(r));
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["score", "views", "answers", "comments"]));
        f.extend(match h {
            Some(h) => h.map(V::I),
            None => [V::Null, V::Null],
        });
        f.push(user_col(db, u, "name"));
        f.extend(match m {
            Some(n) => [V::I(n), V::F(db.user.reputation.get(u).unwrap() as f64)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN
// b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY
// u.Id, u.DisplayName ), PostStats AS ( SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN
// p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId ), UserActivity AS ( SELECT us.Id AS UserId,
// us.DisplayName, COALESCE(ps.PostCount, 0) AS TotalPosts, COALESCE(ps.QuestionsCount, 0) AS TotalQuestions, COALESCE(ps.AnswersCount, 0) AS TotalAnswers,
// COALESCE(bs.BadgeCount, 0) AS TotalBadges, COALESCE(bs.GoldBadges, 0) AS TotalGoldBadges, COALESCE(bs.SilverBadges, 0) AS TotalSilverBadges, COALESCE(bs.BronzeBadges, 0)
// AS TotalBronzeBadges FROM UserBadgeStats bs FULL OUTER JOIN PostStats ps ON bs.UserId = ps.OwnerUserId FULL OUTER JOIN Users us ON us.Id = COALESCE(bs.UserId,
// ps.OwnerUserId) ), RecentPostActivity AS ( SELECT p.OwnerUserId, p.Title, p.CreationDate, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS
// RecencyRank FROM Posts p ) SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalQuestions, ua.TotalAnswers, ua.TotalBadges, ua.TotalGoldBadges, ua.TotalSilverBadges,
// ua.TotalBronzeBadges, rpa.Title AS MostRecentPostTitle, rpa.CreationDate AS MostRecentPostDate FROM UserActivity ua LEFT JOIN RecentPostActivity rpa ON ua.UserId =
// rpa.OwnerUserId AND rpa.RecencyRank = 1 WHERE ua.TotalBadges >= 5 OR (ua.TotalPosts = 0 AND ua.TotalQuestions > 2) ORDER BY ua.TotalBadges DESC, ua.TotalPosts DESC LIMIT
// 100;
//
// The FULL OUTER JOINs add one row beyond the users: PostStats' ownerless group, which matches no user and no badges, and the
// WHERE removes it (0 badges and TotalPosts > 0), so the port reads the users alone.
fn q23266(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, creation_date, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let top = top_per(drain(db.post.with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let tr = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let recent: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&tr).map(|(u, _)| u).inv().select(&tr).collect();
    type R = ([i64; 4], Option<[i64; 3]>);
    let v = drain((&ub).and((&ps).opt()).filt(|(b, a): R| {
        let a = a.unwrap_or([0; 3]);
        b[0] >= 5 || (a[0] == 0 && a[1] > 2)
    }).and((&recent).map(|(_, p)| p).opt()));
    let v = top_n(v, |&(u, ((b, a), p))| (Reverse(b[0]), Reverse(a.map_or(0, |a| a[0])), u, p), 100);
    rows(v.into_iter().map(|(u, ((b, a), p))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecursivePostHistory AS ( SELECT ph.Id, ph.PostId, ph.CreationDate, ph.UserId, ph.UserDisplayName, pt.Name AS PostType, RANK() OVER (PARTITION BY ph.PostId ORDER BY
// ph.CreationDate DESC) AS HistoryRank FROM PostHistory ph INNER JOIN Posts p ON ph.PostId = p.Id INNER JOIN PostTypes pt ON p.PostTypeId = pt.Id ), MaxComments AS ( SELECT
// PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId ), UserReputation AS ( SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY
// u.Reputation DESC) AS RepRank FROM Users u ), UserBadges AS ( SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId ), PostsStatistics AS ( SELECT p.Id AS
// PostId, p.Title, COALESCE(mc.CommentCount, 0) AS TotalComments, COALESCE(ub.BadgeCount, 0) AS UserBadgeCount, pp.HistoryRank, p.Score, p.ViewCount, p.CreationDate FROM
// Posts p LEFT JOIN MaxComments mc ON p.Id = mc.PostId JOIN RecursivePostHistory pp ON p.Id = pp.PostId LEFT JOIN UserBadges ub ON p.OwnerUserId = ub.UserId WHERE
// pp.HistoryRank = 1 ), TopPosts AS ( SELECT ps.PostId, ps.Title, ps.TotalComments, ps.UserBadgeCount, ps.Score, ps.ViewCount, DENSE_RANK() OVER (ORDER BY ps.Score DESC) AS
// PostRank FROM PostsStatistics ps ) SELECT tp.PostId, tp.Title, tp.TotalComments, tp.UserBadgeCount, tp.Score, CASE WHEN tp.ViewCount > 1000 THEN 'High Views' WHEN
// tp.ViewCount BETWEEN 500 AND 1000 THEN 'Moderate Views' ELSE 'Low Views' END AS ViewCategory, CASE WHEN tr.RepRank <= 10 THEN 'Top Reputation' ELSE 'Regular' END AS
// OwnerReputationCategory FROM TopPosts tp LEFT JOIN UserReputation tr ON tp.UserBadgeCount = tr.UserId WHERE tp.PostRank <= 10 ORDER BY tp.Score DESC, tp.TotalComments
// DESC;
//
// `tp.UserBadgeCount = tr.UserId` compares a badge count with a user id, so it goes through the raw ids.
fn q30920(db: &'static So) -> String {
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let top = top_per(drain(db.post_history.select(post)), |&(_, p)| p, |&(h, _)| Reverse(hd.get(h).unwrap()), 1, true);
    let tr = rel(top);
    let latest: HashIdx<Id<Post>, Id<PostHistory>> = (&tr).map(|(_, p)| p).inv().select((&tr).map(|(h, _)| h)).collect();
    let Post { score, owner_user, view_count, .. } = &db.post;
    let with_hist: MatSet<Id<Post>> = (&tr).map(|(_, p)| p).collect();
    let v = ranked(drain((&with_hist).select(score)), |&(_, s)| Reverse(s), true);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let mc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ubc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let tu = rel(top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10));
    let top_ids: HashIdx<i64, i64> = (&tu).map(|(u, _)| u).select(&db.user.origid).inv().map(|_| 1i64).collect();
    let bc = || owner_user.select(&ubc).opt().map(|b: Option<i64>| b.unwrap_or(0));
    let v = drain((&tp).select((&latest).and((&mc).opt()).and(bc().and(bc().select(&top_ids).opt()))));
    rows(v.into_iter().map(|(p, ((_, c), (b, t)))| {
        let w = view_count.get(p);
        let top = t.is_some();
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(b), V::I(score.get(p).unwrap())]);
        f.push(V::S(match w {
            Some(w) if w > 1000 => "High Views",
            Some(w) if (500..=1000).contains(&w) => "Moderate Views",
            _ => "Low Views",
        }));
        f.push(V::S(if top { "Top Reputation" } else { "Regular" }));
        row(f)
    }))
}

// WITH BadgeStats AS ( SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN Class = 3
// THEN 1 END) AS BronzeBadges FROM Badges GROUP BY UserId ), PostStats AS ( SELECT OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN Score > 0 THEN 1 ELSE 0 END) AS
// PositivePosts, SUM(CASE WHEN Score < 0 THEN 1 ELSE 0 END) AS NegativePosts FROM Posts GROUP BY OwnerUserId ), UserPerformance AS ( SELECT u.Id AS UserId, u.DisplayName,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges, COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges, COALESCE(ps.TotalPosts, 0) AS
// TotalPosts, COALESCE(ps.PositivePosts, 0) AS PositivePosts, COALESCE(ps.NegativePosts, 0) AS NegativePosts, ROW_NUMBER() OVER (ORDER BY COALESCE(ps.PositivePosts, 0)
// DESC) AS PerformanceRank FROM Users u LEFT JOIN BadgeStats bs ON u.Id = bs.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId ), TopUsers AS ( SELECT DisplayName,
// GoldBadges, SilverBadges, BronzeBadges, TotalPosts, PositivePosts, NegativePosts, PerformanceRank FROM UserPerformance WHERE PerformanceRank <= 10 ), DetailedPostInfo AS
// ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, (SELECT COUNT(*) FROM
// Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVoteCount FROM Posts
// p ) SELECT tu.DisplayName, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, d.PostId, d.Title, d.CreationDate, d.Score, d.ViewCount, d.CommentCount, d.UpVoteCount,
// d.DownVoteCount FROM TopUsers tu JOIN DetailedPostInfo d ON tu.DisplayName = (SELECT OwnerDisplayName FROM Posts WHERE Id = d.PostId) ORDER BY tu.PerformanceRank, d.Score
// DESC;
//
// DetailedPostInfo joins on Posts.OwnerDisplayName, the name column kept for posts whose owner account is gone.
fn q4560(db: &'static So) -> String {
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let Post { owner_user, score, owner_display_name, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score).fold(0i64, |n, s| n + (s > 0) as i64);
    let tu = rel(top_n(drain(db.user.select((&ps).opt())), |&(u, n)| (Reverse(n.unwrap_or(0)), u), 10));
    let by_name: HashIdx<Str, Id<Post>> = owner_display_name.inv().collect();
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let v = drain((&tus).select((&bs).opt().and((&db.user.display_name).select(&by_name))));
    let posts: MatSet<Id<Post>> = rel(v.iter().map(|x| x.1 .1).collect()).map(|p| p).collect();
    let cc = (&posts).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ud = (&posts).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&tus).select((&bs).opt().and((&db.user.display_name).select(&by_name).select(Ident::<Post>::new().and(&cc).and(&ud)))));
    rows(v.into_iter().map(|(u, (b, ((p, c), a)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserVoteCounts AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpvoteCount, COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS
// DownvoteCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName ), PostStatistics AS ( SELECT P.Id AS PostId, P.Title, P.Score,
// COALESCE(P.ViewCount, 0) AS ViewCount, COALESCE(B.Class, 0) AS BadgeCount, COALESCE(PH.EditsCount, 0) AS EditsCount, COUNT(C.Id) AS CommentCount FROM Posts P LEFT JOIN
// Badges B ON P.OwnerUserId = B.UserId AND B.Date > P.CreationDate LEFT JOIN ( SELECT PostId, COUNT(*) AS EditsCount FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6)
// GROUP BY PostId ) PH ON P.Id = PH.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title, P.Score, P.ViewCount, B.Class, PH.EditsCount ),
// FilteredPostStatistics AS ( SELECT PS.PostId, PS.Title, PS.Score, PS.ViewCount, PS.BadgeCount, PS.EditsCount, PS.CommentCount, CASE WHEN PS.Score > 10 THEN 'Hot' WHEN
// PS.Score BETWEEN 5 AND 10 THEN 'Warm' ELSE 'Cold' END AS PostHeat FROM PostStatistics PS WHERE PS.CommentCount > 0 AND PS.ViewCount > 0 AND PS.BadgeCount < 3 ),
// RankedPosts AS ( SELECT FPS.*, RANK() OVER (ORDER BY FPS.Score DESC) AS ScoreRank, ROW_NUMBER() OVER (PARTITION BY FPS.PostHeat ORDER BY FPS.ViewCount DESC) AS HeatRank
// FROM FilteredPostStatistics FPS ) SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.EditsCount, RP.CommentCount, RP.PostHeat, UVC.UpvoteCount, UVC.DownvoteCount FROM
// RankedPosts RP LEFT JOIN UserVoteCounts UVC ON RP.PostId = UVC.UserId WHERE RP.ScoreRank <= 10 AND (RP.PostHeat = 'Hot' OR RP.PostHeat = 'Warm') ORDER BY RP.Score DESC,
// RP.ViewCount DESC;
//
// `RP.PostId = UVC.UserId` joins a post id to a user id, so it goes through the raw ids. The WHERE keeps only groups with a
// comment and a view count, so only such posts are joined to their owners' badges.
fn q21193(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, origid, .. } = &db.post;
    let base: MatSet<Id<Post>> = db.post.with(view_count.gt(0)).with(comments_of(db)).select(Ident::<Post>::new()).collect();
    type PB = (Id<Post>, Id<Badge>);
    let pairs: HashIdx<Id<Post>, Id<Badge>> = (&base).select(Ident::<Post>::new().and(owner_user.select(badges_of(db)))).filt(|(p, b): PB| db.badge.date.get(b).unwrap() > creation_date.get(p).unwrap()).map(|(_, b): PB| b).collect();
    type J = (Id<Post>, Option<Id<Badge>>);
    let j: MatSet<J> = (&base).select(Ident::<Post>::new().and((&pairs).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let class_of = (&j).flat_map(|(_, b)| b).select(&db.badge.class);
    let g = (&j).group_by((&post_of).and(class_of.opt())).select((&post_of).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let ec = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    type G = ((Id<Post>, Option<i64>), i64);
    let gv = rel(drain(&g));
    let kept: Vec<_> = drain((&gv).filt(|((_, c), _): G| c.unwrap_or(0) < 3));
    let heat = |s: i64| if s > 10 { "Hot" } else if (5..=10).contains(&s) { "Warm" } else { "Cold" };
    let r = ranked(kept, |&(_, ((p, _), _))| Reverse(score.get(p).unwrap()), false);
    type RR = ((usize, G), i64);
    let rr = rel(r);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let uvc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&rr).filt(move |((_, ((p, _), _)), r): RR| r <= 10 && heat(score.get(p).unwrap()) != "Cold").select(Same::<RR>::new().and(Same::<RR>::new().map(|((_, ((p, _), _)), _): RR| p).select((&ec).opt().and(origid.select(&uid).select(&uvc).opt())))));
    rows(v.into_iter().map(|(_, (((_, ((p, _), c)), _), (e, u)))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(view_count.get(p).unwrap_or(0)), V::I(e.unwrap_or(0)), V::I(c), V::S(heat(score.get(p).unwrap()))]);
        f.extend(match u {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeCounts AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id =
// B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation ), UserPostMetrics AS ( SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END)
// AS Questions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId ),
// CombinedMetrics AS ( SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UBC.BadgeCount, 0) AS BadgeCount, COALESCE(UBC.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBC.SilverBadges, 0) AS SilverBadges, COALESCE(UBC.BronzeBadges, 0) AS BronzeBadges, COALESCE(UPM.TotalPosts, 0) AS TotalPosts, COALESCE(UPM.Questions, 0) AS
// Questions, COALESCE(UPM.Answers, 0) AS Answers, COALESCE(UPM.TotalViews, 0) AS TotalViews, COALESCE(UPM.AverageScore, 0) AS AverageScore FROM Users U LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN UserPostMetrics UPM ON U.Id = UPM.OwnerUserId ), RankedUsers AS ( SELECT UserId, DisplayName, Reputation, BadgeCount,
// GoldBadges, SilverBadges, BronzeBadges, TotalPosts, Questions, Answers, TotalViews, AverageScore, ROW_NUMBER() OVER (ORDER BY Reputation DESC, BadgeCount DESC, TotalViews
// DESC) AS Rank FROM CombinedMetrics ) SELECT UserId, DisplayName, Reputation, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, TotalPosts, Questions, Answers,
// TotalViews, AverageScore, Rank FROM RankedUsers WHERE Rank <= 100 ORDER BY Rank;
fn q25114(db: &'static So) -> String {
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let upm = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, w), s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s]);
    let v = top_n(drain((&ubc).and((&upm).opt())), |&(u, (b, a))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b[0]), Reverse(a.map_or(0, |a| a[3])), u), 100);
    rows(v.into_iter().enumerate().map(|(i, (u, (b, a)))| {
        let a = a.unwrap_or([0; 5]);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[0] == 0 { V::F(0.0) } else { avg(a[4], a[0]) }, V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.Score, p.CreationDate, u.DisplayName AS Author, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER
// BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01
// 12:34:56' - INTERVAL '1 year' ), TagStatistics AS ( SELECT t.TagName, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore FROM Tags t JOIN Posts p ON p.Tags LIKE
// CONCAT('%<', t.TagName, '>%') WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY t.TagName ), UserStatistics AS ( SELECT u.Id AS UserId,
// u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ), RecentComments AS ( SELECT c.Id AS CommentId, c.Text, c.CreationDate, p.Title AS PostTitle, u.DisplayName
// AS Author FROM Comments c JOIN Posts p ON c.PostId = p.Id JOIN Users u ON c.UserId = u.Id WHERE c.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' )
// SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.Score, rp.CreationDate, rp.Author, ts.TagName, ts.PostCount, ts.AverageScore, us.DisplayName AS UserWithMostBadges,
// us.BadgeCount, us.QuestionsAsked, rc.CommentId, rc.Text AS RecentComment, rc.CreationDate AS CommentDate, rc.Author AS CommentAuthor FROM RankedPosts rp LEFT JOIN
// TagStatistics ts ON ts.PostCount > 5 LEFT JOIN UserStatistics us ON us.BadgeCount = ( SELECT MAX(BadgeCount) FROM UserStatistics ) LEFT JOIN RecentComments rc ON
// rc.PostTitle = rp.Title WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rc.CreationDate DESC;
//
// TagStatistics and UserStatistics are joined on conditions that name only their own side, so each is crossed with the top questions.
fn q29499(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, title, tags, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let top = top_per(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(since))).with(owner_user).select(post_type_id)), |&(_, t)| t, |&(p, _)| {
        (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p)
    }, 10, false);
    let tp = rel(top.into_iter().map(|x| x.0).collect());
    let ts_ = db.post.with(creation_date.ge(since)).group_by(tags).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let tsv = left_all(drain((&ts_).filt(|a| a[0] > 5)));
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id).opt()))
        .fold([0i64; 2], |a, (b, t)| [a[0] + b.is_some() as i64, a[1] + (t == Some(1)) as i64]);
    let mx = (&us).fold_flat(i64::MIN, |m, a| m.max(a[0]));
    let usv = left_all(drain((&us).filt(move |a| a[0] == mx)));
    let Comment { creation_date: cd, user, .. } = &db.comment;
    let rc: HashIdx<Str, Id<Comment>> = db.comment.with(cd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).with(user).select((&db.comment.post).select(title)).inv().collect();
    type X = ((Id<Post>, Option<(Id<Tag>, [i64; 2])>), Option<(Id<User>, [i64; 2])>);
    let v = drain((&tp).cross(&tsv).cross(&usv).select(Same::<X>::new().and(Same::<X>::new().map(|((p, _), _): X| p).select(title).select(&rc).opt())));
    rows(v.into_iter().map(|(_, (((p, t), u), c))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "score", "created", "owner"]);
        f.extend(match t {
            Some((t, a)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), avg(a[1], a[0])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match u {
            Some((u, a)) => [user_col(db, u, "name"), V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some(c) => [V::I(db.comment.origid.get(c).unwrap()), V::S(db.comment.text.get(c).unwrap()), V::T(cd.get(c).unwrap()), V::S(user.get(c).map(|u| db.user.display_name.get(u).unwrap()).unwrap())],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS ( SELECT u.Id AS UserId, u.Reputation, u.CreationDate, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS
// GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON
// u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.CreationDate, u.DisplayName ), PostsStatistics AS ( SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN
// p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(p.Score, 0)) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount FROM Posts p WHERE p.OwnerUserId IS NOT NULL GROUP BY p.OwnerUserId ), UserPerformance AS ( SELECT ur.UserId, ur.DisplayName,
// ur.Reputation, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.QuestionCount, 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount, COALESCE(ps.TotalScore,
// 0) AS TotalScore, COALESCE(ps.AvgViewCount, 0) AS AvgViewCount, ur.BadgeCount, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges FROM UserReputation ur LEFT JOIN
// PostsStatistics ps ON ur.UserId = ps.OwnerUserId ) SELECT up.UserId, up.DisplayName, up.Reputation, up.PostCount, up.QuestionCount, up.AnswerCount, up.TotalScore,
// up.AvgViewCount, up.BadgeCount, STRING_AGG(CASE WHEN up.GoldBadges > 0 THEN 'Gold' WHEN up.SilverBadges > 0 THEN 'Silver' WHEN up.BronzeBadges > 0 THEN 'Bronze' ELSE NULL
// END, ', ') AS BadgeTypes FROM UserPerformance up WHERE up.Reputation IS NOT NULL AND up.PostCount > (SELECT AVG(PostCount) FROM PostsStatistics) GROUP BY up.UserId,
// up.DisplayName, up.Reputation, up.PostCount, up.QuestionCount, up.AnswerCount, up.TotalScore, up.AvgViewCount, up.BadgeCount ORDER BY up.Reputation DESC, up.TotalScore
// DESC, up.PostCount DESC FETCH FIRST 10 ROWS ONLY;
//
// The GROUP BY keeps one row per user, so each STRING_AGG sees a single value.
fn q23252(db: &'static So) -> String {
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let (sum, n) = (&ps).fold_flat((0i64, 0i64), |(s, n), a| (s + a[0], n + 1));
    let v = drain((&ur).and(&ps).filt(move |(_, a): ([i64; 4], [i64; 6])| a[0] as i128 * n as i128 > sum as i128));
    let v = top_n(v, |&(u, (_, a))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[3]), Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (b, a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[4] == 0 { V::F(0.0) } else { avg(a[5], a[4]) }, V::I(b[0])]);
        f.push(if b[1] > 0 { V::S("Gold") } else if b[2] > 0 { V::S("Silver") } else if b[3] > 0 { V::S("Bronze") } else { V::Null });
        row(f)
    }))
}

// WITH UserBadges AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class =
// 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id,
// U.DisplayName ), PostStatistics AS ( SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS
// AnswerCount, COUNT(CASE WHEN P.Score >= 0 THEN 1 END) AS NonNegativePosts FROM Posts P GROUP BY P.OwnerUserId ), UserActivity AS ( SELECT U.Id AS UserId, U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(PS.QuestionCount, 0) AS TotalQuestions, COALESCE(PS.AnswerCount, 0) AS TotalAnswers, COALESCE(PS.NonNegativePosts, 0)
// AS TotalNonNegativePosts, RANK() OVER (ORDER BY COALESCE(UB.BadgeCount, 0) DESC, COALESCE(PS.QuestionCount, 0) DESC) AS Ranking FROM Users U LEFT JOIN UserBadges UB ON
// U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId ), TopUsers AS ( SELECT UA.UserId, UA.DisplayName, UA.TotalBadges, UA.TotalQuestions,
// UA.TotalAnswers, UA.TotalNonNegativePosts, UA.Ranking FROM UserActivity UA WHERE UA.TotalBadges > 0 OR UA.TotalQuestions > 0 ORDER BY UA.Ranking LIMIT 10 ) SELECT
// TU.DisplayName, TU.TotalBadges, TU.TotalQuestions, TU.TotalAnswers, TU.TotalNonNegativePosts, CASE WHEN TU.TotalQuestions = 0 THEN NULL ELSE ROUND((TU.TotalAnswers * 1.0
// / NULLIF(TU.TotalQuestions, 0)) * 100, 2) END AS AnswerRate, CASE WHEN TU.TotalBadges = 0 THEN 'No Badges' ELSE CASE WHEN TU.TotalBadges >= 5 THEN 'Active Contributor'
// ELSE 'Emerging Contributor' END END AS ContributorStatus FROM TopUsers TU WHERE TU.TotalNonNegativePosts > 5 AND TU.TotalBadges + TU.TotalQuestions > (SELECT
// AVG(TotalBadges + TotalQuestions) FROM UserActivity) ORDER BY TU.TotalBadges DESC, TU.TotalQuestions DESC;
fn q24045(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 3], |a, (t, s)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (s >= 0) as i64]);
    let ua = ranked(drain((&ub).and((&ps).opt())), |&(_, (b, a))| (Reverse(b), Reverse(a.map_or(0, |a| a[0]))), false);
    let (sum, n) = (&ub).and((&ps).opt()).fold_flat((0i64, 0i64), |(s, n), (b, a)| (s + b + a.map_or(0, |a| a[0]), n + 1));
    type R = ((Id<User>, (i64, Option<[i64; 3]>)), i64);
    let uar = rel(ua);
    let tu = top_n(drain((&uar).filt(|((_, (b, a)), _): R| b > 0 || a.map_or(0, |a| a[0]) > 0)), |&(_, (_, r))| r, 10);
    let tur = rel(tu.into_iter().map(|x| x.1).collect());
    let v = drain((&tur).filt(move |((_, (b, a)), _): R| {
        let a = a.unwrap_or([0; 3]);
        a[2] > 5 && (b + a[0]) as i128 * n as i128 > sum as i128
    }));
    rows(v.into_iter().map(|(_, ((u, (b, a)), _))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.push(if a[0] == 0 { V::Null } else { V::F((a[1] as f64 / a[0] as f64 * 100.0 * 100.0).round() / 100.0) });
        f.push(V::S(if b == 0 { "No Badges" } else if b >= 5 { "Active Contributor" } else { "Emerging Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
// p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1 ), UserBadges AS ( SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id =
// b.UserId GROUP BY u.Id ), PostStatistics AS ( SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, MAX(p.LastActivityDate) AS LastActivityDate
// FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id ), DeletedPosts AS ( SELECT p.Id AS PostId,
// ph.CreationDate, ph.Comment AS DeleteReason FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 12 ), ActiveThreadStats AS ( SELECT
// r.PostId, r.Title, r.CreationDate, r.Score, ps.CommentCount, ps.VoteCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, r.OwnerUserId, CASE WHEN dp.PostId IS NOT NULL
// THEN 'Deleted' ELSE 'Active' END AS Status FROM RankedPosts r LEFT JOIN PostStatistics ps ON r.PostId = ps.PostId LEFT JOIN UserBadges ub ON r.OwnerUserId = ub.UserId
// LEFT JOIN DeletedPosts dp ON r.PostId = dp.PostId WHERE r.Rank = 1 ) SELECT a.PostId, a.Title, a.CreationDate, a.Score, a.CommentCount, a.VoteCount, a.GoldBadges,
// a.SilverBadges, a.BronzeBadges, CASE WHEN a.Status = 'Deleted' THEN 'This post has been deleted' ELSE 'This post is active' END AS PostStatus FROM ActiveThreadStats a
// ORDER BY a.Score DESC, a.CreationDate ASC;
fn q33690(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&first).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64],
        None => a,
    });
    let deleted = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(12)));
    let v = drain((&ps).and(owner_user.select(&ub).opt()).and(deleted.opt()));
    rows(v.into_iter().map(|(p, ((a, b), d))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(a.map(V::I));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if d.is_some() { "This post has been deleted" } else { "This post is active" }));
        row(f)
    }))
}

// WITH RecursivePostStats AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS
// UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE
// p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score ), TopPosts AS ( SELECT PostId, Title, CreationDate, Score, CommentCount, UpVotes, DownVotes,
// ROW_NUMBER() OVER (ORDER BY Score DESC) AS RankScore FROM RecursivePostStats WHERE Score IS NOT NULL ), TopUserBadges AS ( SELECT u.Id AS UserId, u.DisplayName,
// COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS BestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName ), FilteredUsers AS ( SELECT
// UserId, DisplayName, BadgeCount, BestBadgeClass FROM TopUserBadges WHERE BadgeCount > 5 ), PostHistoryStats AS ( SELECT p.Id AS PostId, p.Title, ph.Comment,
// ph.CreationDate, ph.UserDisplayName, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS EditRanking FROM Posts p JOIN
// PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (4, 5) ) SELECT tp.PostId, tp.Title, tp.CreationDate AS QuestionDate, tp.Score, tp.CommentCount,
// tp.UpVotes, tp.DownVotes, fu.DisplayName AS UserWithBadges, fu.BadgeCount, CASE WHEN phs.EditRanking = 1 THEN 'Most Recent Edit' ELSE 'Earlier Edit' END AS EditStatus,
// phs.Comment AS EditComment FROM TopPosts tp LEFT JOIN Posts pp ON pp.Id = tp.PostId LEFT JOIN FilteredUsers fu ON pp.OwnerUserId = fu.UserId LEFT JOIN PostHistoryStats
// phs ON tp.PostId = phs.PostId WHERE tp.RankScore <= 10 ORDER BY tp.Score DESC, tp.CreationDate ASC;
//
// RankScore reads only Score, so the top questions are picked first and the comment x vote product is driven for those alone.
fn q30689(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let fu = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let er = ranked(drain(db.post_history.with(post_history_type_id.is_in([4, 5])).select(post)), |&(h, p)| (p, Reverse(hd.get(h).unwrap()), h), false);
    let er = rel(per_group(er, |&(_, p)| p).into_iter().map(|((h, p), r)| (p, (h, r))).collect());
    let phs: HashIdx<Id<Post>, (Id<Post>, (Id<PostHistory>, i64))> = (&er).map(|(p, _)| p).inv().select(&er).collect();
    let v = drain((&s).and(owner_user.select(Ident::<User>::new().and((&fu).filt(|n| n > 5))).opt()).and((&phs).map(|(_, x)| x).opt()));
    rows(v.into_iter().map(|(p, ((a, u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(a.map(V::I));
        f.extend(match u {
            Some((u, n)) => [user_col(db, u, "name"), V::I(n)],
            None => [V::Null, V::Null],
        });
        f.extend(match h {
            Some((h, r)) => [V::S(if r == 1 { "Most Recent Edit" } else { "Earlier Edit" }), harness::fmt::ostr(db.post_history.comment.get(h))],
            None => [V::S("Earlier Edit"), V::Null],
        });
        row(f)
    }))
}

// WITH UserPostStats AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE
// WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(CASE WHEN p.PostTypeId = 1 THEN p.Score END) AS AvgQuestionScore, AVG(CASE WHEN p.PostTypeId = 2 THEN
// p.Score END) AS AvgAnswerScore, SUM(CASE WHEN p.PostTypeId = 1 THEN p.ViewCount ELSE 0 END) AS TotalQuestionViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName ), TopUsers AS ( SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AvgQuestionScore, AvgAnswerScore, TotalQuestionViews,
// RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts, RANK() OVER (ORDER BY AvgQuestionScore DESC) AS RankByQuestionScore, RANK() OVER (ORDER BY AvgAnswerScore DESC) AS
// RankByAnswerScore FROM UserPostStats ), UserBadges AS ( SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE
// WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId ), FinalResults AS (
// SELECT u.DisplayName, COALESCE(up.TotalPosts, 0) AS TotalPosts, COALESCE(up.TotalQuestions, 0) AS TotalQuestions, COALESCE(up.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(up.AvgQuestionScore, 0) AS AvgQuestionScore, COALESCE(up.AvgAnswerScore, 0) AS AvgAnswerScore, COALESCE(ub.BadgeCount, 0) AS TotalBadges, COALESCE(ub.GoldBadges,
// 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges FROM Users u LEFT JOIN UserPostStats up ON u.Id = up.UserId
// LEFT JOIN UserBadges ub ON u.Id = ub.UserId ) SELECT DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AvgQuestionScore, AvgAnswerScore, TotalBadges, GoldBadges,
// SilverBadges, BronzeBadges FROM FinalResults WHERE TotalPosts > 10 ORDER BY AvgQuestionScore DESC, TotalPosts DESC LIMIT 10;
fn q28380(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }, a[4] + if t == 2 { s } else { 0 }],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let mean = |s: i64, n: i64| if n == 0 { 0.0 } else { s as f64 / n as f64 };
    let v = top_n(drain((&ups).filt(|a| a[0] > 10).and((&ub).opt())), |&(u, (a, _))| (Reverse(fkey(mean(a[3], a[1]))), Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(mean(a[3], a[1])), V::F(mean(a[4], a[2]))];
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class =
// 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id,
// U.DisplayName ), RecentPostDetails AS ( SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, P.Body FROM Posts P JOIN
// Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' ), TaggedPostStatistics AS ( SELECT PT.TagName, COUNT(P.Id) AS PostCount,
// AVG(P.ViewCount) AS AverageViews, AVG(P.Score) AS AverageScore FROM Posts P JOIN LATERAL (SELECT unnest(string_to_array(P.Tags, ',')) AS tag) tag ON TRUE JOIN Tags PT ON
// tag = PT.TagName GROUP BY PT.TagName ORDER BY PostCount DESC ), UserEngagementStats AS ( SELECT U.Id AS UserId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS
// UpVotesGiven, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesGiven, COUNT(C.Id) AS CommentsGiven FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT
// JOIN Comments C ON U.Id = C.UserId GROUP BY U.Id ) SELECT U.DisplayName, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, RPD.PostId, RPD.Title,
// RPD.CreationDate AS PostCreationDate, RPD.ViewCount, RPD.Score AS PostScore, UES.UpVotesGiven, UES.DownVotesGiven, UES.CommentsGiven, TPS.TagName, TPS.PostCount,
// TPS.AverageViews, TPS.AverageScore FROM UserBadges UB JOIN Users U ON UB.UserId = U.Id LEFT JOIN RecentPostDetails RPD ON U.DisplayName = RPD.OwnerDisplayName LEFT JOIN
// UserEngagementStats UES ON U.Id = UES.UserId LEFT JOIN (SELECT TagName, PostCount, AverageViews, AverageScore FROM TaggedPostStatistics) TPS ON TRUE ORDER BY
// UB.BadgeCount DESC, RPD.ViewCount DESC, TPS.PostCount DESC;
//
// CURRENT_TIMESTAMP is when the query runs and the data ends in 2024, so RecentPostDetails is empty; the comparison is
// TIMESTAMPTZ, so CreationDate is read as New York local time. TaggedPostStatistics splits Tags on ',', which never occurs,
// so no piece names a tag and it is empty too.
fn q26497(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { creation_date, owner_user, tags_str, view_count, score, .. } = &db.post;
    let now = now_utc();
    let by_name: HashIdx<Str, Id<Post>> = db.post.with(creation_date.filt(move |d| ny_to_utc(d) >= now - 30 * DAY_US)).with(owner_user).select(owner_user.select(&db.user.display_name)).inv().collect();
    let tag_idx: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tps = db.post.group_by(tags_str.flat_map(|t: Str| t.split(',')).select(&tag_idx)).select(view_count.opt().and(score)).fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let tpsv = left_all(drain(&tps));
    let ues = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(comments_by(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let urows = rel(drain((&ub).and((&db.user.display_name).select(&by_name).opt()).and(&ues)));
    let v = drain((&urows).cross(&tpsv));
    rows(v.into_iter().map(|(_, ((u, ((b, p), e)), t))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created", "views", "score"]),
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(e.map(V::I));
        f.extend(match t {
            Some((t, a)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH TagStats AS ( SELECT t.Id AS TagId, t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.ViewCount > 0 THEN p.ViewCount ELSE 0 END) AS TotalViews,
// AVG(p.Score) AS AverageScore FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%') WHERE p.PostTypeId = 1 GROUP BY t.Id, t.TagName ), TopTags AS ( SELECT
// TagId, TagName, PostCount, TotalViews, AverageScore, ROW_NUMBER() OVER (ORDER BY TotalViews DESC) AS TagRank FROM TagStats ), UserBadges AS ( SELECT u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id ), PopularUsers AS ( SELECT u.Id AS UserId,
// u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived, (SUM(CASE
// WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END)) AS NetVotes FROM Users u JOIN Posts p ON p.OwnerUserId = u.Id JOIN Votes v
// ON v.PostId = p.Id WHERE p.PostTypeId IN (1, 2) GROUP BY u.Id, u.DisplayName ), Benchmark AS ( SELECT t.TagId, t.TagName, t.PostCount, t.TotalViews, t.AverageScore,
// u.UserId, u.DisplayName, u.UpVotesReceived, u.DownVotesReceived, u.NetVotes, b.BadgeCount, b.HighestBadgeClass FROM TopTags t JOIN PopularUsers u ON u.UpVotesReceived >
// 10 JOIN UserBadges b ON b.UserId = u.UserId WHERE t.TagRank <= 10 ) SELECT b.TagId, b.TagName, b.PostCount, b.TotalViews, b.AverageScore, b.UserId, b.DisplayName,
// b.UpVotesReceived, b.DownVotesReceived, b.NetVotes, b.BadgeCount, CASE WHEN b.HighestBadgeClass = 1 THEN 'Gold' WHEN b.HighestBadgeClass = 2 THEN 'Silver' WHEN
// b.HighestBadgeClass = 3 THEN 'Bronze' ELSE 'None' END AS HighestBadge FROM Benchmark b ORDER BY b.TotalViews DESC, b.AverageScore DESC;
//
// The JOIN on `u.UpVotesReceived > 10` names only u, so the top tags are crossed with those users.
fn q25238(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let lt = tag_mentions(db);
    let q_by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).with(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p).select(post_type_id.eq(1))).map(|(_, t)| t).inv().collect();
    let ts_ = db.tag.group_by(Ident::<Tag>::new()).select((&q_by_tag).map(|(p, _)| p).select(view_count.opt().and(score))).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.filter(|&w| w > 0).unwrap_or(0), a[2] + s]);
    let tt = rel(top_n(drain(&ts_), |&(t, a)| (Reverse(a[1]), t), 10));
    let pu = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.is_in([1, 2]))).select(votes_of(db).select(&db.vote.vote_type_id)))
        .fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let pur = rel(drain((&pu).filt(|a| a[0] > 10).and(&ub)));
    let v = drain((&tt).cross(&pur));
    rows(v.into_iter().map(|(_, ((t, a), (u, (x, (n, m)))))| {
        let mut f = vec![V::I(db.tag.origid.get(t).unwrap()), V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), avg(a[2], a[0])];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(x[0]), V::I(x[1]), V::I(x[0] - x[1]), V::I(n)]);
        f.push(V::S(match m {
            1 => "Gold",
            2 => "Silver",
            3 => "Bronze",
            _ => "None",
        }));
        row(f)
    }))
}

// WITH UserStats AS ( SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1
// ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments
// C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id GROUP BY U.Id, U.Reputation ), PostStatistics AS ( SELECT P.Id AS PostId, P.OwnerUserId,
// P.PostTypeId, COUNT(DISTINCT C.Id) AS TotalComments, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END)
// AS TotalDownVotes, AVG(P.Score) AS AvgScore, COUNT(DISTINCT PH.Id) AS TotalHistoryChanges FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id =
// V.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id, P.OwnerUserId, P.PostTypeId ), TopUsers AS ( SELECT UserId, Reputation, TotalPosts, TotalComments,
// TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats ), TopPosts AS ( SELECT PostId, OwnerUserId, PostTypeId,
// TotalComments, TotalUpVotes, TotalDownVotes, AvgScore, TotalHistoryChanges, RANK() OVER (ORDER BY AvgScore DESC) AS AvgScoreRank FROM PostStatistics ) SELECT U.UserId,
// U.Reputation, U.TotalPosts, U.TotalComments, U.TotalUpVotes, U.TotalDownVotes, P.PostId, P.PostTypeId, P.TotalComments AS PostComments, P.TotalUpVotes AS PostUpVotes,
// P.TotalDownVotes AS PostDownVotes, P.AvgScore, P.TotalHistoryChanges, TU.ReputationRank, TP.AvgScoreRank FROM TopUsers U JOIN TopPosts P ON U.UserId = P.OwnerUserId JOIN
// (SELECT UserId, COUNT(*) AS ReputationRank FROM TopUsers GROUP BY UserId) TU ON U.UserId = TU.UserId JOIN (SELECT PostId, COUNT(*) AS AvgScoreRank FROM TopPosts GROUP BY
// PostId) TP ON P.PostId = TP.PostId ORDER BY U.Reputation DESC, P.AvgScore DESC LIMIT 10;
//
// The rows are ordered by the owner's Reputation and then the post's Score (AvgScore is the post's own score), both base columns,
// so the ten (user, post) pairs are picked first and the products are driven for those alone. TU and TP count one row per id.
fn q11656(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let pairs = top_n(drain(db.post.with(owner_user).select(owner_user)), |&(p, u)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(pairs.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu: MatSet<Id<User>> = rel(pairs.iter().map(|x| x.1).collect()).map(|u| u).collect();
    let own = own_votes(db);
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((_, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
            None => a,
        });
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).opt()))
        .fold([0i64; 2], |a, ((_, t), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ph = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&ps).and(&pc).and(&ph).and(owner_user.select(Ident::<User>::new().and(&us).and(&np).and(&nc))));
    let v = top_n(v, |&(p, (_, (((u, _), _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (((a, c), h), (((u, b), n), k)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(k), V::I(b[0]), V::I(b[1])]);
        f.extend(post_fields(db, p, &["id", "type_id"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::F(score.get(p).unwrap() as f64), V::I(h), V::I(1), V::I(1)]);
        row(f)
    }))
}

// WITH RECURSIVE UserHierarchy AS ( SELECT U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, U.Location, 1 AS Level FROM Users U WHERE U.Reputation >
// 5000 UNION ALL SELECT U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, U.Location, UH.Level + 1 FROM Users U JOIN UserHierarchy UH ON U.Id = UH.Id + 1
// WHERE U.Reputation > UH.Reputation ), PostVoteDetails AS ( SELECT P.Id AS PostId, P.Title, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS
// UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate > (CAST('2024-10-01
// 12:34:56' AS timestamp) - INTERVAL '30 days') GROUP BY P.Id, P.Title ), RecentPosts AS ( SELECT P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId,
// U.DisplayName AS OwnerDisplayName, PV.VoteCount, PV.UpVotes, PV.DownVotes FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN PostVoteDetails PV ON P.Id =
// PV.PostId WHERE P.PostTypeId = 1 ), TopUsers AS ( SELECT U.Id, U.DisplayName, U.Reputation FROM Users U WHERE U.Reputation >= ( SELECT AVG(Reputation) FROM Users WHERE
// LastAccessDate > (CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year') ) ORDER BY U.Reputation DESC LIMIT 10 ) SELECT RP.Title, RP.CreationDate, RP.Score,
// RP.ViewCount, RP.OwnerDisplayName, TU.DisplayName AS TopUser, TU.Reputation AS TopUserReputation, COALESCE(RP.VoteCount, 0) AS TotalVotes, COALESCE(RP.UpVotes, 0) AS
// TotalUpVotes, COALESCE(RP.DownVotes, 0) AS TotalDownVotes, CASE WHEN RP.OwnerUserId IS NULL THEN 'Unknown' ELSE 'Known User' END AS UserStatus FROM RecentPosts RP LEFT
// JOIN TopUsers TU ON RP.OwnerUserId = TU.Id ORDER BY RP.Score DESC, RP.ViewCount DESC;
//
// UserHierarchy is never referenced by the final SELECT, so its recursion is never run.
fn q30476(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let pv = db
        .post
        .with(creation_date.gt(add_days(t0, -30)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let (sum, n) = db.user.with((&db.user.last_access_date).gt(add_years(t0, -1))).select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let tu = rel(top_n(drain((&db.user.reputation).filt(move |r| r as i128 * n as i128 >= sum as i128)), |&(u, r)| (Reverse(r), u), 10));
    let tus: HashIdx<Id<User>, Id<User>> = (&tu).map(|(u, _)| u).inv().select((&tu).map(|(u, _)| u)).collect();
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select((&pv).opt().and(owner_user.select(&tus).opt())));
    rows(v.into_iter().map(|(p, (a, t))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend(match t {
            Some(u) => [user_col(db, u, "name"), user_col(db, u, "rep")],
            None => [V::Null, V::Null],
        });
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S("Known User")]);
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS
// UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
// FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title,
// p.CreationDate, p.OwnerUserId ), UserBadges AS ( SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS
// SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId ), UserStatistics AS ( SELECT u.Id AS UserId, u.Reputation,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, COUNT(p.Id) AS QuestionCount,
// SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND
// p.PostTypeId = 1 GROUP BY u.Id, u.Reputation, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges ), FinalResults AS ( SELECT ups.UserId, ups.Reputation, ups.GoldBadges,
// ups.SilverBadges, ups.BronzeBadges, ups.QuestionCount, ups.TotalViews, ups.TotalScore, r.PostRank, MAX(r.AnswerCount) AS MaxAnswerCount, MAX(r.UpvoteCount +
// r.DownvoteCount) AS TotalVotes FROM UserStatistics ups LEFT JOIN RankedPosts r ON ups.UserId = r.OwnerUserId GROUP BY ups.UserId, ups.Reputation, ups.GoldBadges,
// ups.SilverBadges, ups.BronzeBadges, ups.QuestionCount, ups.TotalViews, ups.TotalScore, r.PostRank ) SELECT UserId, Reputation, GoldBadges, SilverBadges, BronzeBadges,
// QuestionCount, TotalViews, TotalScore, PostRank, MaxAnswerCount, TotalVotes FROM FinalResults WHERE QuestionCount > 10 ORDER BY Reputation DESC, TotalVotes DESC;
//
// RankedPosts is only read for owners with more than ten questions, so its answers x votes product is driven for their questions alone.
fn q34055(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let us = db.user.group_by(Ident::<User>::new()).select(qs().select(view_count.opt().and(score))).fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let busy = || Ident::<User>::new().with((&us).filt(|a| a[0] > 10));
    let rp = || db.post.with(post_type_id.eq(1)).with(owner_user.select(busy()));
    let rk = ranked(drain(rp().select(owner_user)), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false);
    let rk = rel(per_group(rk, |&(_, u)| u).into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let pa = rp()
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (x, t)| [a[0] + x.is_some() as i64, a[1] + matches!(t, Some(2 | 3)) as i64]);
    let v = drain((&us).filt(|a| a[0] > 10).and((&ub).opt()).and(qs().select((&rank).map(|(_, r)| r).and(&pa))));
    rows(v.into_iter().map(|(u, ((a, b), (r, x)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(r), V::I(x[0]), V::I(x[1])]);
        row(f)
    }))
}

// WITH UserPostStats AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE
// WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName ), UserBadges AS ( SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId ), PostTypeSummary AS (
// SELECT P.OwnerUserId, PT.Name AS PostType, COUNT(P.Id) AS PostCount FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY P.OwnerUserId, PT.Name ),
// PostsWithClosure AS ( SELECT PH.UserId, PH.PostId, PH.CreationDate, PH.Comment, CASE WHEN PH.PostHistoryTypeId = 10 THEN 'Closed' WHEN PH.PostHistoryTypeId = 11 THEN
// 'Reopened' ELSE 'Other' END AS ClosureType FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) ), RankedUserPosts AS ( SELECT UPS.UserId, UPS.DisplayName,
// UPS.TotalPosts, UPS.TotalQuestions, UPS.TotalAnswers, UPS.TotalBounty, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, ROW_NUMBER() OVER (ORDER BY UPS.TotalPosts DESC)
// AS Rank FROM UserPostStats UPS LEFT JOIN UserBadges UB ON UPS.UserId = UB.UserId ) SELECT R.UserId, R.DisplayName, R.TotalPosts, R.TotalQuestions, R.TotalAnswers,
// R.TotalBounty, R.GoldBadges, R.SilverBadges, R.BronzeBadges, COALESCE(PTS.PostType, 'No Posts') AS PostType, COALESCE(PTS.PostCount, 0) AS PostCount, PWC.CreationDate AS
// ClosureDate, PWC.Comment AS ClosureComment, PWC.ClosureType FROM RankedUserPosts R LEFT JOIN PostTypeSummary PTS ON R.UserId = PTS.OwnerUserId LEFT JOIN PostsWithClosure
// PWC ON R.UserId = PWC.UserId WHERE R.Rank <= 10 ORDER BY R.TotalPosts DESC, R.DisplayName;
fn q30090(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(bounty.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, b)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let tu = top_n(drain(&ups), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pts = db.post.group_by(owner_user.and(ptype_name(db))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let pv = rel(drain(&pts));
    let pts_of: HashIdx<Id<User>, ((Id<User>, Str), i64)> = (&pv).map(|((u, _), _)| u).inv().select(&pv).collect();
    let PostHistory { user, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let pwc: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(post_history_type_id.is_in([10, 11])).select(user).inv().collect();
    let v = drain((&tu).select((&ups).and((&ub).opt()).and((&pts_of).opt()).and((&pwc).opt())));
    rows(v.into_iter().map(|(u, (((a, b), t), h))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match t {
            Some(((_, n), c)) => [V::S(n), V::I(c)],
            None => [V::S("No Posts"), V::I(0)],
        });
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), harness::fmt::ostr(comment.get(h)), V::S(if post_history_type_id.get(h).unwrap() == 10 { "Closed" } else { "Reopened" })],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS ( SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2
// THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY SUBSTRING(p.Tags,
// 2, LENGTH(p.Tags) - 2) ORDER BY p.CreationDate DESC) AS TagGroupRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE
// p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Tags, p.CreationDate, p.ViewCount, p.Score ), TopPostsByTag AS ( SELECT rp.PostId, rp.Title, rp.Tags, rp.CreationDate,
// rp.ViewCount, rp.Score, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.TagGroupRank <= 3 ), PostHistorySummary AS ( SELECT ph.PostId,
// ph.PostHistoryTypeId, ph.CreationDate, COUNT(*) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId, ph.PostHistoryTypeId,
// ph.CreationDate ), FinalBenchmark AS ( SELECT tpt.PostId, tpt.Title, tpt.Tags, tpt.CreationDate, tpt.ViewCount, tpt.Score, tpt.CommentCount, tpt.UpVoteCount,
// tpt.DownVoteCount, SUM(COALESCE(phs.EditCount, 0)) AS TotalEdits, COALESCE(SUM(CASE WHEN phs.PostHistoryTypeId = 4 THEN phs.EditCount END), 0) AS TitleEdits,
// COALESCE(SUM(CASE WHEN phs.PostHistoryTypeId = 5 THEN phs.EditCount END), 0) AS BodyEdits, COALESCE(SUM(CASE WHEN phs.PostHistoryTypeId = 6 THEN phs.EditCount END), 0) AS
// TagEdits FROM TopPostsByTag tpt LEFT JOIN PostHistorySummary phs ON tpt.PostId = phs.PostId GROUP BY tpt.PostId, tpt.Title, tpt.Tags, tpt.CreationDate, tpt.ViewCount,
// tpt.Score, tpt.CommentCount, tpt.UpVoteCount, tpt.DownVoteCount ) SELECT PostId, Title, Tags, CreationDate, ViewCount, Score, CommentCount, UpVoteCount, DownVoteCount,
// TotalEdits, TitleEdits, BodyEdits, TagEdits FROM FinalBenchmark ORDER BY Score DESC, UpVoteCount DESC, ViewCount DESC;
//
// TagGroupRank reads only base columns, so the three newest questions of each tag list are picked first and the comment x vote product is driven for those alone.
fn q25848(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, .. } = &db.post;
    let inner = |t: Str| -> Str { &t[1..t.len() - 1] };
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(tags_str.map(inner).opt())), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post.and(post_history_type_id).and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let hv = rel(drain(&phs));
    let phs_of: HashIdx<Id<Post>, (((Id<Post>, i64), i64), i64)> = (&hv).map(|(((p, _), _), _)| p).inv().select(&hv).collect();
    let fb = (&tp).group_by(Ident::<Post>::new()).select((&phs_of).opt()).fold([0i64; 4], |a, g| match g {
        Some((((_, t), _), n)) => [a[0] + n, a[1] + if t == 4 { n } else { 0 }, a[2] + if t == 5 { n } else { 0 }, a[3] + if t == 6 { n } else { 0 }],
        None => a,
    });
    let v = drain((&s).and(&fb));
    rows(v.into_iter().map(|(p, (a, e))| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "created", "views", "score"]);
        f.extend(a.map(V::I));
        f.extend(e.map(V::I));
        row(f)
    }))
}

// WITH UserVoteSummary AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN
// VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN VoteTypes VT ON V.VoteTypeId = VT.Id GROUP BY U.Id,
// U.DisplayName ), PostActivity AS ( SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.CommentCount, P.AnswerCount, PH.CreationDate AS LastHistoryDate,
// P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY
// PH.CreationDate DESC) AS LastHistoryRank FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId ), UserPostSummary AS ( SELECT U.Id AS UserId, U.DisplayName,
// COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// MAX(P.LastActivityDate) AS LastActivity, MAX(P.CreationDate) AS FirstPostDate FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName ),
// CombinedSummary AS ( SELECT UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.QuestionCount, UPS.AnswerCount, CONCAT_WS(' | ', COALESCE(CAST(UPS.FirstPostDate AS TEXT),
// 'No Posts'), COALESCE(CAST(UPS.LastActivity AS TEXT), 'Inactive')) AS ActivitySummary, COALESCE(UVS.TotalVotes, 0) AS TotalVotes, COALESCE(UVS.UpVotes, 0) AS UpVotes,
// COALESCE(UVS.DownVotes, 0) AS DownVotes FROM UserPostSummary UPS LEFT JOIN UserVoteSummary UVS ON UPS.UserId = UVS.UserId ) SELECT CS.DisplayName, CS.TotalPosts,
// CS.QuestionCount, CS.AnswerCount, CS.TotalVotes, CS.UpVotes, CS.DownVotes, CS.ActivitySummary FROM CombinedSummary CS WHERE CS.TotalPosts > 0 AND (CS.UpVotes -
// CS.DownVotes) > 10 AND NOT EXISTS ( SELECT 1 FROM Users U WHERE U.Id = CS.UserId AND (U.Reputation < 50 OR U.CreationDate > '2023-01-01') ) ORDER BY CS.TotalVotes DESC,
// CS.DisplayName;
//
// PostActivity is never read by the final SELECT, so it is not computed.
fn q23893(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, creation_date, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(last_activity_date).and(creation_date)).opt())
        .fold([0i64, 0, 0, i64::MIN, i64::MIN], |a, p| match p {
            Some(((t, l), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(l), a[4].max(c)],
            None => a,
        });
    let uvs = db.vote.group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let User { reputation, creation_date: uc, .. } = &db.user;
    let bad = Ident::<User>::new().with(reputation.lt(50).or(uc.gt(ts(2023, 1, 1, 0, 0, 0))));
    type R = ([i64; 5], Option<[i64; 3]>);
    let v = drain(db.user.minus(bad).select((&ups).and((&uvs).opt())).filt(|(a, b): R| {
        let b = b.unwrap_or([0; 3]);
        a[0] > 0 && b[1] - b[2] > 10
    }));
    rows(v.into_iter().map(|(u, (a, b))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b[0]), V::I(b[1]), V::I(b[2])];
        f.push(V::Owned(format!("{} | {}", ts_text(a[4]), ts_text(a[3]))));
        row(f)
    }))
}

// WITH UserBadgeSummary AS ( SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN
// B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY
// U.Id, U.DisplayName ), PostStatistics AS ( SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE
// WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers, SUM(P.ViewCount) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId ), UserPerformance AS ( SELECT U.Id AS UserId,
// U.DisplayName, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.TotalQuestions, 0) AS TotalQuestions, COALESCE(PS.TotalAnswers, 0) AS TotalAnswers, U.Reputation FROM
// Users U LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId ), BadgePerformance AS ( SELECT U.UserId, U.DisplayName, U.TotalBadges, U.GoldBadges, U.SilverBadges,
// U.BronzeBadges, UP.TotalPosts, UP.Reputation, RANK() OVER (ORDER BY UP.Reputation DESC) AS ReputationRank FROM UserBadgeSummary U JOIN UserPerformance UP ON U.UserId =
// UP.UserId ), ReputationCategories AS ( SELECT CASE WHEN Reputation >= 1000 THEN 'High' WHEN Reputation >= 100 THEN 'Medium' ELSE 'Low' END AS ReputationCategory,
// AVG(Reputation) AS AvgReputation FROM Users GROUP BY CASE WHEN Reputation >= 1000 THEN 'High' WHEN Reputation >= 100 THEN 'Medium' ELSE 'Low' END ) SELECT BP.DisplayName,
// BP.TotalBadges, BP.GoldBadges, BP.SilverBadges, BP.BronzeBadges, BP.TotalPosts, BP.Reputation, RP.ReputationCategory, RP.AvgReputation, CASE WHEN BP.TotalPosts > 10 THEN
// 'Active User' ELSE 'Less Active User' END AS UserActivityLevel FROM BadgePerformance BP JOIN ReputationCategories RP ON BP.Reputation BETWEEN RP.AvgReputation - 500 AND
// RP.AvgReputation + 500 WHERE (BP.GoldBadges > 0 OR BP.SilverBadges > 0) AND BP.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY BP.Reputation DESC, BP.TotalPosts
// DESC;
//
// The JOIN is on a range around each category's average, which names BP and RP but no key, so it is a filtered cross join.
fn q20719(db: &'static So) -> String {
    let ubs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ps = db.post.group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let rep = &db.user.reputation;
    let cat = |r: i64| if r >= 1000 { "High" } else if r >= 100 { "Medium" } else { "Low" };
    let rc = db.user.group_by(rep.map(cat)).select(rep).fold((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let rcv = rel(drain(&rc).into_iter().map(|(c, (s, n))| (c, s as f64 / n as f64)).collect());
    let (sum, n) = db.user.select(rep).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let bp = rel(drain(db.user.with(rep.filt(move |r| r as i128 * n as i128 > sum as i128)).select((&ubs).filt(|b| b[1] > 0 || b[2] > 0).and((&ps).opt()))));
    let v = drain((&bp).cross(&rcv).filt(|((u, _), (_, a)): ((Id<User>, ([i64; 4], Option<i64>)), (&'static str, f64))| {
        let r = db.user.reputation.get(u).unwrap() as f64;
        r >= a - 500.0 && r <= a + 500.0
    }));
    rows(v.into_iter().map(|(_, ((u, (b, p)), (c, a)))| {
        let p = p.unwrap_or(0);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend([V::I(p), user_col(db, u, "rep"), V::S(c), V::F(a), V::S(if p > 10 { "Active User" } else { "Less Active User" })]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostCounts AS ( SELECT OwnerUserId, COUNT(*) AS PostCount FROM Posts WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY OwnerUserId UNION ALL SELECT UserId, COUNT(*) FROM Comments WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY UserId ),
// UserBadges AS ( SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS
// SilverBadges, SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId ), PostHistoryDetails AS ( SELECT ph.UserId, ph.PostId,
// ph.CreationDate, p.Title, p.Score, COUNT(DISTINCT c.Id) AS CommentCount FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id LEFT JOIN Comments c ON p.Id = c.PostId GROUP
// BY ph.UserId, ph.PostId, ph.CreationDate, p.Title, p.Score ), VoteSummary AS ( SELECT p.OwnerUserId, SUM(CASE WHEN v.VoteTypeId IN (2) THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId IN (3) THEN 1 ELSE 0 END) AS TotalDownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId ) SELECT u.Id AS UserId,
// u.DisplayName, u.Reputation, COALESCE(UPC.PostCount, 0) AS PostsInLastYear, COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, COALESCE(PS.TotalUpVotes, 0) AS UpVotes, COALESCE(PS.TotalDownVotes, 0) AS
// DownVotes, COUNT(DISTINCT PHD.PostId) AS PostsWithHistory FROM Users u LEFT JOIN UserPostCounts UPC ON u.Id = UPC.OwnerUserId LEFT JOIN UserBadges UB ON u.Id = UB.UserId
// LEFT JOIN VoteSummary PS ON u.Id = PS.OwnerUserId LEFT JOIN PostHistoryDetails PHD ON u.Id = PHD.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, UPC.PostCount,
// UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, PS.TotalUpVotes, PS.TotalDownVotes ORDER BY u.Reputation DESC LIMIT 20;
//
// WITH RECURSIVE, but no CTE refers to itself. UserPostCounts is a UNION ALL, so a user can match one row from Posts and one from Comments.
fn q33373(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let since = add_years(t0, -1);
    let pc = db.post.with((&db.post.creation_date).ge(since)).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let cc = db.comment.with((&db.comment.creation_date).ge(since)).group_by(&db.comment.user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let upc: HashIdx<Id<User>, i64> = (&pc).union(&cc).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let vs = db.post.group_by(&db.post.owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { user, post, .. } = &db.post_history;
    let phd = db.post_history.group_by(user).select(post).count_distinct();
    let v = drain(db.user.select((&upc).opt().and((&ub).opt()).and((&vs).opt()).and((&phd).opt())));
    let v = top_n(v, |&(u, (((n, _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), u, n), 20);
    rows(v.into_iter().map(|(u, (((n, b), s), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n.unwrap_or(0)));
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.extend(s.unwrap_or([0; 2]).map(V::I));
        f.push(V::I(h.unwrap_or(0)));
        row(f)
    }))
}

// WITH RecursivePostHistory AS ( SELECT ph.PostId, ph.UserId, ph.CreationDate, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate
// DESC) AS RevisionNumber FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (1, 4, 10) ), UserVoteStatistics AS ( SELECT v.UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN
// v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(CASE WHEN v.VoteTypeId = 4 THEN 1 END) AS
// OffensiveVotes FROM Votes v GROUP BY v.UserId ), PostStats AS ( SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COALESCE(SUM(v.BountyAmount), 0) AS
// TotalBountyAmount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS
// TotalDownvotes, COUNT(c.Id) AS CommentCount, COUNT(ph.Id) FILTER (WHERE ph.PostHistoryTypeId = 10) AS CloseCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT
// JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate ), UserBadgeCounts AS ( SELECT u.Id
// AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE
// WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id ) SELECT ps.PostId, ps.Title, ps.CreationDate,
// ps.TotalBountyAmount, ps.TotalUpvotes, ps.TotalDownvotes, ps.CommentCount, ps.CloseCount, u.DisplayName AS OwnerDisplayName, ubc.BadgeCount AS OwnerBadgeCount,
// ubc.GoldBadges AS OwnerGoldBadges, ubc.SilverBadges AS OwnerSilverBadges, ubc.BronzeBadges AS OwnerBronzeBadges, ups.TotalVotes AS OwnerTotalVotes, ups.UpVotes AS
// OwnerUpVotes, ups.DownVotes AS OwnerDownVotes, ups.OffensiveVotes AS OwnerOffensiveVotes FROM PostStats ps JOIN Users u ON ps.OwnerUserId = u.Id JOIN UserBadgeCounts ubc
// ON u.Id = ubc.UserId JOIN UserVoteStatistics ups ON u.Id = ups.UserId WHERE (ps.TotalUpvotes - ps.TotalDownvotes) > 0 ORDER BY ps.TotalBountyAmount DESC, ps.CommentCount
// DESC, ps.CloseCount ASC LIMIT 10;
//
// The JOIN with UserVoteStatistics keeps only owners who cast a vote, so the votes x comments x history product is driven for their posts alone.
fn q34801(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let uvs = db.vote.group_by(&db.vote.user).select(vote_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (t == 4) as i64]);
    let Post { owner_user, .. } = &db.post;
    let voters = Ident::<User>::new().with(&uvs);
    let ps = db
        .post
        .with(owner_user.select(voters))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt().and(comments_of(db).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 5], |a, ((v, c), h)| {
            let (t, b) = v.map_or((0, None), |(t, b)| (t, b));
            [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + c.is_some() as i64, a[4] + (h == Some(10)) as i64]
        });
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let v = drain((&ps).filt(|a| a[1] - a[2] > 0).and(owner_user.select(Ident::<User>::new().and(&ubc).and(&uvs))));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[0]), Reverse(a[3]), a[4], p), 10);
    rows(v.into_iter().map(|(p, (a, ((u, b), x)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(a.map(V::I));
        f.push(user_col(db, u, "name"));
        f.extend(b.map(V::I));
        f.extend(x.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS ( SELECT Id AS UserId, Reputation, CASE WHEN Reputation >= 1000 THEN 'Gold' WHEN Reputation >= 500 THEN 'Silver' WHEN Reputation >= 0 THEN 'Bronze'
// ELSE 'Negative' END AS ReputationCategory FROM Users ), PostScoreDetails AS ( SELECT p.Id AS PostId, p.Title, COALESCE(p.Score, 0) AS PostScore, COALESCE(votes.UpVotes,
// 0) AS UpVotes, COALESCE(votes.DownVotes, 0) AS DownVotes, CASE WHEN COALESCE(p.Score, 0) > 0 THEN 'Positive' WHEN COALESCE(p.Score, 0) < 0 THEN 'Negative' ELSE 'Neutral'
// END AS ScoreSentiment FROM Posts p LEFT JOIN ( SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END)
// AS DownVotes FROM Votes GROUP BY PostId ) votes ON p.Id = votes.PostId ), ClosedPosts AS ( SELECT ph.PostId, ph.CreationDate AS ClosedDate, STRING_AGG(DISTINCT cr.Name,
// ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS int) = cr.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId,
// ph.CreationDate ), UserActivity AS ( SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName ), AggregatedData AS ( SELECT ur.UserId, ur.ReputationCategory, COUNT(DISTINCT ps.PostId) AS
// TotalPosts, SUM(ps.PostScore) AS TotalPostScore, SUM(ua.PostsCreated) AS PostsCreatedByOwner, SUM(ua.TotalViews) AS TotalViewsByOwner, COUNT(cp.PostId) AS ClosedPostCount
// FROM UserReputation ur LEFT JOIN PostScoreDetails ps ON ur.UserId = ps.PostId LEFT JOIN UserActivity ua ON ur.UserId = ua.UserId LEFT JOIN ClosedPosts cp ON ps.PostId =
// cp.PostId GROUP BY ur.UserId, ur.ReputationCategory ) SELECT ad.UserId, ad.ReputationCategory, ad.TotalPosts, ad.TotalPostScore, ad.PostsCreatedByOwner,
// ad.TotalViewsByOwner, ad.ClosedPostCount, CASE WHEN ad.TotalPostScore > 0 THEN 'This user has a positive influence.' WHEN ad.TotalPostScore < 0 THEN 'This user has a
// negative influence.' ELSE 'This user has a neutral presence.' END AS InfluenceDescription FROM AggregatedData ad WHERE ad.TotalPosts > 5 ORDER BY ad.TotalPostScore DESC,
// ad.ReputationCategory;
//
// `ur.UserId = ps.PostId` joins a user id to a post id, so it goes through the raw ids; a user meets at most one post that way,
// so TotalPosts is never above 1 and the WHERE leaves nothing.
fn q22117(db: &'static So) -> String {
    let Post { origid, score, view_count, .. } = &db.post;
    let byorig: HashIdx<i64, Id<Post>> = origid.inv().collect();
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(w) => [a[0] + 1, a[1] + w.unwrap_or(0)],
        None => a,
    });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .with(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .group_by(post.and(hd))
        .select(Ident::<PostHistory>::new())
        .fold(0i64, |n, _| n + 1);
    let cv = rel(drain(&cp));
    let cp_of: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let ad = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.origid).select(&byorig).select(Ident::<Post>::new().and(score).and((&cp_of).opt())).opt().and(&ua))
        .fold(([0i64; 5], None::<Id<Post>>), |(a, d), (p, u)| match p {
            Some(((p, s), c)) => ([a[0], a[1] + s, a[2] + u[0], a[3] + u[1], a[4] + c.is_some() as i64], Some(p)),
            None => ([a[0], a[1], a[2] + u[0], a[3] + u[1], a[4]], d),
        });
    let v = drain((&ad).filt(|(_, d)| d.map_or(0, |_| 1) > 5));
    let cat = |r: i64| if r >= 1000 { "Gold" } else if r >= 500 { "Silver" } else if r >= 0 { "Bronze" } else { "Negative" };
    rows(v.into_iter().map(|(u, (a, d))| {
        let mut f = vec![user_col(db, u, "uid"), V::S(cat(db.user.reputation.get(u).unwrap())), V::I(d.map_or(0, |_| 1))];
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.push(V::S(if a[1] > 0 { "This user has a positive influence." } else if a[1] < 0 { "This user has a negative influence." } else { "This user has a neutral presence." }));
        row(f)
    }))
}

// WITH RecursiveBadgeCounts AS ( SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2
// THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id ),
// UserPostActivity AS ( SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN p.PostTypeId = 2
// THEN 1 END) AS TotalAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId ), ActiveUsers AS (
// SELECT u.Id, u.DisplayName, COALESCE(rbc.BadgeCount, 0) AS BadgeCount, COALESCE(upa.TotalPosts, 0) AS TotalPosts, COALESCE(upa.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(upa.TotalAnswers, 0) AS TotalAnswers, COALESCE(upa.TotalScore, 0) AS TotalScore, COALESCE(upa.TotalViews, 0) AS TotalViews FROM Users u LEFT JOIN
// RecursiveBadgeCounts rbc ON u.Id = rbc.UserId LEFT JOIN UserPostActivity upa ON u.Id = upa.OwnerUserId WHERE u.Reputation > 1000 ), RankedUsers AS ( SELECT *,
// ROW_NUMBER() OVER (ORDER BY TotalScore DESC, TotalPosts DESC) AS ScoreRank, RANK() OVER (ORDER BY BadgeCount DESC) AS BadgeRank FROM ActiveUsers ) SELECT au.DisplayName,
// au.BadgeCount, au.TotalPosts, au.TotalQuestions, au.TotalAnswers, au.TotalScore, au.TotalViews, CASE WHEN au.BadgeCount > 0 THEN 'Badge Holder' ELSE 'Novice' END AS
// UserType, CASE WHEN ScoreRank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributionLevel FROM RankedUsers au WHERE au.ScoreRank <= 50 AND
// au.BadgeRank <= 50 ORDER BY au.TotalScore DESC, au.BadgeCount DESC;
fn q20091(db: &'static So) -> String {
    let rbc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let upa = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 5], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0)]
    });
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&rbc).and((&upa).opt())));
    let v: Vec<_> = top_n(v, |&(u, (_, a))| {
        let a = a.unwrap_or([0; 5]);
        (Reverse(a[3]), Reverse(a[0]), u)
    }, 0)
    .into_iter()
    .enumerate()
    .map(|(i, x)| (x, i as i64 + 1))
    .collect();
    let v = ranked(v, |&((_, (b, _)), _)| Reverse(b), false);
    type R = (((Id<User>, (i64, Option<[i64; 5]>)), i64), i64);
    let rr = rel(v);
    let v = drain((&rr).filt(|((_, s), b): R| s <= 50 && b <= 50));
    rows(v.into_iter().map(|(_, (((u, (b, a)), s), _))| {
        let a = a.unwrap_or([0; 5]);
        let mut f = vec![user_col(db, u, "name"), V::I(b)];
        f.extend(a.map(V::I));
        f.extend([V::S(if b > 0 { "Badge Holder" } else { "Novice" }), V::S(if s <= 10 { "Top Contributor" } else { "Regular Contributor" })]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("6491", q6491),
    ("30785", q30785),
    ("808", q808),
    ("33082", q33082),
    ("2665", q2665),
    ("26462", q26462),
    ("31401", q31401),
    ("32550", q32550),
    ("7683", q7683),
    ("22053", q22053),
    ("24616", q24616),
    ("20777", q20777),
    ("31177", q31177),
    ("27359", q27359),
    ("31548", q31548),
    ("2455", q2455),
    ("26206", q26206),
    ("5556", q5556),
    ("7749", q7749),
    ("24154", q24154),
    ("222", q222),
    ("4610", q4610),
    ("3014", q3014),
    ("25522", q25522),
    ("20865", q20865),
    ("32236", q32236),
    ("21896", q21896),
    ("30982", q30982),
    ("10502", q10502),
    ("31216", q31216),
    ("27768", q27768),
    ("1619", q1619),
    ("25005", q25005),
    ("29230", q29230),
    ("3296", q3296),
    ("2546", q2546),
    ("7515", q7515),
    ("9732", q9732),
    ("25981", q25981),
    ("29320", q29320),
    ("1266", q1266),
    ("6004", q6004),
    ("31064", q31064),
    ("32648", q32648),
    ("2736", q2736),
    ("2965", q2965),
    ("8947", q8947),
    ("33427", q33427),
    ("8757", q8757),
    ("30227", q30227),
    ("25177", q25177),
    ("9094", q9094),
    ("4773", q4773),
    ("9766", q9766),
    ("3081", q3081),
    ("34696", q34696),
    ("20039", q20039),
    ("24297", q24297),
    ("33356", q33356),
    ("32751", q32751),
    ("26510", q26510),
    ("3468", q3468),
    ("23619", q23619),
    ("31145", q31145),
    ("2197", q2197),
    ("6591", q6591),
    ("29971", q29971),
    ("32058", q32058),
    ("31004", q31004),
    ("29780", q29780),
    ("29799", q29799),
    ("31317", q31317),
    ("30576", q30576),
    ("25737", q25737),
    ("34399", q34399),
    ("23266", q23266),
    ("30920", q30920),
    ("4560", q4560),
    ("21193", q21193),
    ("25114", q25114),
    ("29499", q29499),
    ("23252", q23252),
    ("24045", q24045),
    ("33690", q33690),
    ("30689", q30689),
    ("28380", q28380),
    ("26497", q26497),
    ("25238", q25238),
    ("11656", q11656),
    ("30476", q30476),
    ("34055", q34055),
    ("30090", q30090),
    ("25848", q25848),
    ("23893", q23893),
    ("20719", q20719),
    ("33373", q33373),
    ("34801", q34801),
    ("22117", q22117),
    ("20091", q20091),
];
