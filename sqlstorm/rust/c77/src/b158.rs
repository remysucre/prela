use harness::prelude::*;
use std::cmp::Reverse;

fn like(s: &str, p: &str) -> bool {
    let (s, p): (Vec<char>, Vec<char>) = (s.chars().collect(), p.chars().collect());
    let (mut i, mut j, mut star, mut mark) = (0, 0, None, 0);
    while i < s.len() {
        if j < p.len() && p[j] == '%' {
            star = Some(j);
            mark = i;
            j += 1;
        } else if j < p.len() && (p[j] == '_' || p[j] == s[i]) {
            i += 1;
            j += 1;
        } else if let Some(st) = star {
            j = st + 1;
            mark += 1;
            i = mark;
        } else {
            return false;
        }
    }
    p[j..].iter().all(|&c| c == '%')
}

fn like_in(s: &str, pre: &str, n: &str, post: &str) -> bool {
    if n.contains(['%', '_']) {
        like(s, &format!("%{pre}{n}{post}%"))
    } else {
        s.match_indices(n).any(|(i, _)| s[..i].ends_with(pre) && s[i + n.len()..].starts_with(post))
    }
}

fn win<D, S, K, O, C, W: Copy>(s: &Fold<D, S>, f: fn(&[(K, (D, S))], &mut Vec<W>), order: O, cmp: C) -> Window<(), (D, S), W>
where
    D: Copy + Eq + std::hash::Hash + 'static,
    S: Copy + Eq + std::hash::Hash,
    K: Copy,
    O: Fn((D, S)) -> K,
    C: Fn(&K, &K) -> std::cmp::Ordering,
{
    whole(s).select(Same::<D>::new().and(s)).window(f, order, cmp)
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount,
//        COALESCE(MAX(b.Name), 'No Badge') AS HighestBadge, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY p.CreationDate DESC) AS UserLatestPostRank
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id LEFT JOIN Users u ON u.Id = p.OwnerUserId
//     LEFT JOIN Badges b ON b.UserId = u.Id WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, u.Id),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, ViewCount, CommentCount, UpVoteCount, DownVoteCount, HighestBadge FROM PostStats WHERE UserLatestPostRank = 1)
// SELECT pp.PostId, pp.Title, pp.Score, pp.CreationDate, pp.ViewCount, pp.CommentCount, pp.UpVoteCount, pp.DownVoteCount, pp.HighestBadge,
//        COUNT(DISTINCT ph.UserId) AS EditHistoryCount, MAX(ph.CreationDate) AS LastEditDate
// FROM TopPosts pp LEFT JOIN PostHistory ph ON ph.PostId = pp.PostId
// GROUP BY pp.PostId, pp.Title, pp.Score, pp.CreationDate, pp.ViewCount, pp.CommentCount, pp.UpVoteCount, pp.DownVoteCount, pp.HighestBadge
// ORDER BY pp.Score DESC, pp.CreationDate ASC LIMIT 10;
//
// Every aggregate is DISTINCT or a MAX, so each child is folded on its own; the rank and the cut read only base columns, so the ten posts are picked first.
fn q9070(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first = top_n(drain((&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p)), |&(_, p)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()), 10);
    let tp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.1).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let hb = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).select(&db.badge.name).opt()).fold(None, |m: Option<Str>, n| match (m, n) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    });
    let PostHistory { user_id, creation_date: hd, .. } = &db.post_history;
    let eu = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(user_id)).count_distinct();
    let md = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(hd).opt()).fold(i64::MIN, |m, d| d.map_or(m, |d| m.max(d)));
    let mut v = drain((&cc).and(&vc).and(&hb).and((&eu).opt()).and(&md));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, ((((c, u), b), e), m))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "views"]);
        f.extend([V::I(c), V::I(u[0]), V::I(u[1]), V::S(b.unwrap_or("No Badge")), V::I(e.unwrap_or(0)), tmax(m)]);
        row(f)
    }))
}

// WITH PostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate AS PostCreationDate, p.Score, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// RankedPosts AS (SELECT pa.*, RANK() OVER (ORDER BY pa.Score DESC, pa.CommentCount DESC) AS RankScore FROM PostActivity pa),
// RecentPostHistory AS (SELECT ph.PostId, ph.CreationDate,
//        MAX(ph.CreationDate) OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) AS LastEditDate,
//        ph.UserId, u.DisplayName AS LastEditedBy, COUNT(ph.Id) AS EditCount
//     FROM PostHistory ph JOIN Users u ON ph.UserId = u.Id WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId, ph.UserId, u.DisplayName, ph.CreationDate)
// SELECT rp.RankScore, rp.Title, rp.PostCreationDate, rp.Score, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount, rph.LastEditDate, rph.LastEditedBy, rph.EditCount
// FROM RankedPosts rp LEFT JOIN RecentPostHistory rph ON rp.PostId = rph.PostId WHERE rp.RankScore <= 10 ORDER BY rp.RankScore, rp.PostCreationDate DESC;
fn q33911(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let pa = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = win(&pa, rank, |(p, a)| (Reverse(score.get(p).unwrap()), Reverse(a[0])), asc);
    let rp: MatSet<(Id<Post>, [i64; 3], i64)> = (&w).filt(|(_, r)| r <= 10).map(|((p, a), r)| (p, a, r)).collect();
    let PostHistory { post, user, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let edits = || db.post_history.with(post_history_type_id.is_in([4, 5, 6])).with(user);
    let g = edits().group_by(post.and(user).and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let gv = rel(drain(&g));
    let by_post: HashIdx<Id<Post>, (((Id<Post>, Id<User>), i64), i64)> = (&gv).map(|(((p, _), _), _)| p).inv().select(&gv).collect();
    let last = edits().group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    type R = (Id<Post>, [i64; 3], i64);
    let v = drain((&rp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _): R| p).select((&by_post).and(&last)).opt())));
    rows(v.into_iter().map(|(_, ((p, a, r), h))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(((((_, u), _), n), m)) => [V::T(m), user_col(db, u, "name"), V::I(n)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankRecent
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName, RankScore, RankRecent FROM RankedPosts WHERE RankScore <= 10 OR RankRecent <= 10),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEdited FROM PostHistory ph GROUP BY ph.PostId),
// FinalResults AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName, phs.EditCount, phs.LastEdited
//     FROM TopPosts tp LEFT JOIN PostHistorySummary phs ON tp.PostId = phs.PostId)
// SELECT *, CASE WHEN AnswerCount > 0 THEN 'Has Answers' ELSE 'No Answers' END AS Status FROM FinalResults ORDER BY Score DESC, ViewCount DESC;
fn q9697(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, answer_count, .. } = &db.post;
    type T = (((Id<Post>, i64), Option<i64>), i64);
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()).and(creation_date))
        .window(rank, |(((_, s), w), _)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let w = (&w).window(rank, |((_, d), _): (T, i64)| Reverse(d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|((_, a), b)| a <= 10 || b <= 10).map(|(((((p, _), _), _), _), _)| p).collect();
    let hs = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    rows(drain(&hs).into_iter().map(|(p, (n, m))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([if n == 0 { V::Null } else { V::I(n) }, tmax(m)]);
        f.push(V::S(if answer_count.get(p).map_or(false, |a| a > 0) { "Has Answers" } else { "No Answers" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
//        COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.TotalPosts, 0) AS TotalPosts,
//        COALESCE(PS.TotalQuestions, 0) AS TotalQuestions, COALESCE(PS.TotalAnswers, 0) AS TotalAnswers, COALESCE(PS.TotalViews, 0) AS TotalViews,
//        COALESCE(PS.AverageScore, 0) AS AverageScore
//     FROM Users U LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId)
// SELECT U.DisplayName, U.Reputation, U.BadgeCount, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalViews, U.AverageScore,
//        RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY U.BadgeCount DESC) AS BadgeRank, RANK() OVER (ORDER BY U.TotalPosts DESC) AS PostRank
// FROM UserPerformance U WHERE U.TotalPosts > 0 ORDER BY U.Reputation DESC, U.BadgeCount DESC, U.TotalPosts DESC LIMIT 10;
fn q25866(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    type T = (Id<User>, ([i64; 10], i64));
    let w = whole((&ups).filt(|a| a[1] > 0)).select(Ident::<User>::new().and((&ups).and(&bc))).window(rank, |(u, _): T| Reverse(rep(u)), asc);
    let w = (&w).window(rank, |((_, (_, b)), _): (T, i64)| Reverse(b), asc);
    let w = (&w).window(rank, |(((_, (a, _)), _), _): ((T, i64), i64)| Reverse(a[1]), asc);
    let v = top_n(drain(&w), |&(_, ((((u, (a, b)), _), _), _))| (Reverse(rep(u)), Reverse(b), Reverse(a[1])), 10);
    rows(v.into_iter().map(|(_, ((((u, (a, b)), r1), r2), r3))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), avg(a[4], a[1]), V::I(r1), V::I(r2), V::I(r3)]);
        row(f)
    }))
}

// WITH TagCounts AS (SELECT T.Id AS TagId, T.TagName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.Id, T.TagName),
// UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopTags AS (SELECT TagId, TagName, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagCounts WHERE PostCount > 0),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserReputation WHERE PostCount > 0)
// SELECT T.TagName, T.PostCount AS TotalPosts, T.QuestionCount, T.AnswerCount, U.DisplayName AS TopUser, U.Reputation AS UserReputation
// FROM TopTags T JOIN TopUsers U ON U.QuestionCount > 0 WHERE T.TagRank <= 10 AND U.UserRank <= 10 ORDER BY T.PostCount DESC, U.Reputation DESC LIMIT 10;
//
// The ON clause names only U, so the top tags and the top users are crossed.
fn q28704(db: &'static So) -> String {
    let ts = tag_stats(db);
    let tt = whole((&ts).filt(|a| a[0] > 0)).select(Ident::<Tag>::new().and(&ts)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let ups = user_posts(db);
    let tu = whole((&ups).filt(|a| a[1] > 0)).select(Ident::<User>::new().and(&ups)).window(rank, |(u, _)| Reverse(rep(u)), asc);
    let mut v = Vec::new();
    (&tt).filt(|(_, r)| r <= 10).cross((&tu).filt(|((_, a), r)| r <= 10 && a[2] > 0)).drive(|_, (((t, a), _), ((u, _), _))| v.push((t, a, u)));
    let v = top_n(v, |&(t, a, u)| (Reverse(a[0]), Reverse(rep(u)), t, u), 10);
    rows(v.into_iter().map(|(t, a, u)| row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[4]), V::I(a[5]), user_col(db, u, "name"), V::I(rep(u))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopPosts AS (SELECT rp.*, (UpVotes - DownVotes) AS Score FROM RankedPosts rp WHERE rp.PostRank = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges B ON u.Id = B.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.GoldBadges, us.SilverBadges, us.BronzeBadges, tp.PostId, tp.Title AS LatestPostTitle, tp.CreationDate AS LatestPostDate,
//        tp.Score AS LatestPostScore, tp.CommentCount
// FROM UserStats us LEFT JOIN TopPosts tp ON us.UserId = tp.OwnerUserId ORDER BY us.Reputation DESC, tp.Score DESC NULLS LAST LIMIT 100;
//
// PostRank reads only base columns, so each owner's newest question is picked first and the comment x vote product is driven for those alone.
fn q395(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let own: HashIdx<Id<User>, Id<Post>> = (&tp).select(owner_user).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let v = drain((&ub).and((&own).select(Ident::<Post>::new().and(&pa)).opt()));
    let v = top_n(v, |&(u, (_, t))| {
        let s = t.map(|(_, a)| a[1] - a[2]);
        (Reverse(db.user.reputation.get(u).unwrap()), s.is_none(), Reverse(s))
    }, 100);
    rows(v.into_iter().map(|(u, (b, t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(match t {
            Some((p, a)) => {
                let mut g = post_fields(db, p, &["id", "title", "created"]);
                g.extend([V::I(a[1] - a[2]), V::I(a[0])]);
                g
            }
            None => (0..5).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH TagCounts AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvotesReceived FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     GROUP BY u.Id, u.DisplayName),
// TopTags AS (SELECT TagName, PostCount, QuestionCount, AnswerCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagCounts WHERE PostCount > 0),
// TopUsers AS (SELECT UserId, DisplayName, PostsCreated, UpvotesReceived, DownvotesReceived, ROW_NUMBER() OVER (ORDER BY PostsCreated DESC) AS UserRank FROM UserEngagement WHERE PostsCreated > 0)
// SELECT tt.TagName, tt.PostCount, tt.QuestionCount, tt.AnswerCount, tu.DisplayName AS TopUser, tu.PostsCreated, tu.UpvotesReceived, tu.DownvotesReceived
// FROM TopTags tt JOIN TopUsers tu ON tt.TagRank = tu.UserRank WHERE tt.TagRank <= 10 ORDER BY tt.PostCount DESC, tu.PostsCreated DESC;
//
// The ten top tags and users are re-keyed by their ROW_NUMBER, so the join on the rank is `.and`.
fn q25956(db: &'static So) -> String {
    type R = (Id<Post>, Id<Tag>);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t): R| t).inv().select(Same::<R>::new().map(|(p, _): R| p)).collect();
    let Tag { tag_name, .. } = &db.tag;
    let qa = db.tag.group_by(tag_name).select((&by_tag).select(&db.post.post_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64]);
    let pc = db.tag.group_by(tag_name).select(&by_tag).count_distinct();
    let tw = whole(&pc).select(Same::<Str>::new().and((&pc).and(&qa))).window(row_number, |(t, (n, _))| (Reverse(n), t), asc);
    type TT = ((Str, (i64, [i64; 2])), i64);
    let tm: MatSet<TT> = (&tw).filt(|(_, r)| r <= 10).map(|x| x).collect();
    let tk: HashIdx<i64, TT> = (&tm).map(|(_, r): TT| r).inv().select(&tm).collect();
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(Some(2))) as i64, a[1] + (t == Some(Some(3))) as i64]);
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let uw = whole(&dp).select(Ident::<User>::new().and((&dp).and(&ue))).window(row_number, |(u, (n, _))| (Reverse(n), u), asc);
    type UT = ((Id<User>, (i64, [i64; 2])), i64);
    let um: MatSet<UT> = (&uw).filt(|(_, r)| r <= 10).map(|x| x).collect();
    let uk: HashIdx<i64, UT> = (&um).map(|(_, r): UT| r).inv().select(&um).collect();
    rows(drain((&tk).and(&uk)).into_iter().map(|(_, (((t, (n, a)), _), ((u, (m, e)), _)))| {
        row(vec![V::S(t), V::I(n), V::I(a[0]), V::I(a[1]), user_col(db, u, "name"), V::I(m), V::I(e[0]), V::I(e[1])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= '2023-01-01' AND p.Score IS NOT NULL GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 500 GROUP BY u.Id, u.Reputation),
// PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, ur.UserId, ur.Reputation, ur.BadgeCount, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges, pv.UpVotes, pv.DownVotes
// FROM RankedPosts rp JOIN Users u ON u.Id = rp.PostId LEFT JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId
// WHERE rp.PostRank <= 10 ORDER BY rp.CreationDate DESC, rp.Score DESC;
//
// `u.Id = rp.PostId` joins a user id to a post id, so it goes through the raw ids. PostRank reads only Score, so the ranked posts are picked first.
fn q842(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ur = db.user.with((&db.user.reputation).gt(500)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let mut v = drain((&cc).and(origid.select(&uidx).select(Ident::<User>::new().and(&ur).opt())).and((&pv).opt()));
    v.sort_by_key(|&(p, _)| (Reverse(creation_date.get(p).unwrap()), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((c, u), w))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(c));
        match u {
            Some((u, a)) => {
                f.extend(ucols(db, u, &["uid", "rep"]));
                f.extend(a.map(V::I));
            }
            None => f.extend((0..6).map(|_| V::Null)),
        }
        match w {
            Some(a) => f.extend(a.map(V::I)),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        COALESCE(a.AnswerCount, 0) AS AnswerCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v
//     ON p.Id = v.PostId),
// RankedPosts AS (SELECT pd.*, RANK() OVER (ORDER BY pd.Score DESC, pd.ViewCount DESC, pd.CreationDate DESC) AS Rank FROM PostDetails pd)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.LastActivityDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.UpVotes, rp.DownVotes
// FROM RankedPosts rp WHERE rp.Rank <= 100 ORDER BY rp.Rank;
//
// Rank reads only base columns, so the top posts are picked first.
fn q8446(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let w = whole(db.post.iq()).select(Ident::<Post>::new().and(score).and(view_count.opt()).and(creation_date)).window(rank, |(((_, s), w), d)| (Reverse(s), w.is_none(), Reverse(w), Reverse(d)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 100).map(|((((p, _), _), _), _)| p).collect();
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&ac).and(&cc).and(&vc)).into_iter().map(|(p, ((a, c), u))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "activity", "score", "views"]);
        f.extend([V::I(a), V::I(c), V::I(u[0]), V::I(u[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId IN (3, 4, 5, 6) THEN 1 ELSE 0 END) AS Wikis,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName, u.Reputation),
// CloseReasons AS (SELECT ph.PostId, MIN(CASE WHEN ph.PostHistoryTypeId = 10 THEN cr.Name END) AS CloseReason FROM PostHistory ph
//     LEFT JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// UserStats AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.PostCount, ua.Questions, ua.Answers, ua.Wikis, COALESCE(cr.CloseReason, 'No Close Reason') AS CloseReason,
//        ROW_NUMBER() OVER (ORDER BY ua.Reputation DESC) AS Rank FROM UserActivity ua LEFT JOIN CloseReasons cr ON ua.UserId = cr.PostId)
// SELECT us.DisplayName, us.Reputation, us.PostCount, us.Questions, us.Answers, us.Wikis, us.CloseReason, CASE WHEN us.Rank <= 10 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributorStatus
// FROM UserStats us WHERE us.PostCount > 10 ORDER BY us.Rank;
//
// `ua.UserId = cr.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q386(db: &'static So) -> String {
    let ua = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, _)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 3..=6) as i64],
            None => a,
        });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt())
        .fold(None, |m: Option<Str>, n| match (m, n) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let w = whole(&ua).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let v = drain((&w).map(|((u, _), i)| (u, i)).select(Same::<(Id<User>, i64)>::new().and(Same::<(Id<User>, i64)>::new().map(|(u, _)| u).select((&ua).filt(|a| a[0] > 10).and((&db.user.origid).select(&pidx).select(&cr).opt())))));
    rows(v.into_iter().map(|(_, ((u, i), (a, r)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::S(r.flatten().unwrap_or("No Close Reason")));
        f.push(V::S(if i <= 10 { "Top Contributor" } else { "Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN c.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.OwnerUserId, u.DisplayName),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes, us.DisplayName AS BadgeOwner, us.GoldBadges,
//        us.SilverBadges, us.BronzeBadges, RANK() OVER (ORDER BY rp.UpVotes DESC) AS VoteRank
// FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.PostRank = 1 AND rp.CommentCount > 0 ORDER BY VoteRank, rp.CreationDate DESC;
//
// PostRank reads only base columns, so each owner's newest question is picked first and the comment x vote product is driven for those alone.
fn q27519(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [0, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let fr = || (&pa).filt(|a| a[0] > 0).and(owner_user.select(Ident::<User>::new().and(&ub)));
    let w = whole(fr()).select(Ident::<Post>::new().and(fr())).window(rank, |(_, (a, _))| Reverse(a[1]), asc);
    rows(drain(&w).into_iter().map(|(_, ((p, (a, (u, b))), r))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(user_col(db, u, "name"));
        f.extend(b[1..].iter().map(|&x| V::I(x)));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// UserAchievements AS (SELECT U.DisplayName, COALESCE(P.QuestionCount, 0) AS QuestionCount, COALESCE(P.AnswerCount, 0) AS AnswerCount, COALESCE(P.TotalViews, 0) AS TotalViews,
//        COALESCE(P.TotalScore, 0) AS TotalScore, COALESCE(B.BadgeCount, 0) AS BadgeCount, COALESCE(B.GoldBadges, 0) AS GoldBadges, COALESCE(B.SilverBadges, 0) AS SilverBadges,
//        COALESCE(B.BronzeBadges, 0) AS BronzeBadges FROM Users U LEFT JOIN PostStatistics P ON U.Id = P.OwnerUserId LEFT JOIN UserBadgeStats B ON U.Id = B.UserId)
// SELECT UA.DisplayName, UA.QuestionCount, UA.AnswerCount, UA.TotalViews, UA.TotalScore, UA.BadgeCount, UA.GoldBadges, UA.SilverBadges, UA.BronzeBadges,
//        RANK() OVER (ORDER BY UA.TotalScore DESC) AS ScoreRank
// FROM UserAchievements UA WHERE UA.BadgeCount > 0 ORDER BY UA.TotalScore DESC, UA.DisplayName ASC LIMIT 10;
fn q26285(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let w = whole((&ub).filt(|b| b[0] > 0)).select(Ident::<User>::new().and((&ub).and(&ups)).and(&db.user.display_name)).window(rank, |((_, (_, a)), _)| Reverse(a[4]), asc);
    let v = top_n(drain(&w), |&(_, (((_, (_, a)), n), _))| (Reverse(a[4]), n), 10);
    rows(v.into_iter().map(|(_, (((u, (b, a)), _), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4])];
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes, COALESCE(BR.BadgeCount, 0) AS BronzeCount,
//        COALESCE(SR.BadgeCount, 0) AS SilverCount, COALESCE(GR.BadgeCount, 0) AS GoldCount, RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS RankPosition
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Class = 3 GROUP BY UserId) BR ON p.OwnerUserId = BR.UserId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Class = 2 GROUP BY UserId) SR ON p.OwnerUserId = SR.UserId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Class = 1 GROUP BY UserId) GR ON p.OwnerUserId = GR.UserId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.BronzeCount, rp.SilverCount, rp.GoldCount, rp.RankPosition
// FROM RankedPosts rp WHERE rp.RankPosition <= 100 ORDER BY rp.RankPosition;
//
// There is no GROUP BY, so the rank numbers the post x vote rows; the whole product is driven and ranked.
fn q9545(db: &'static So) -> String {
    let Post { score, creation_date, owner_user_id, .. } = &db.post;
    let w = whole(db.post.iq()).select(Ident::<Post>::new().and(score).and(creation_date).and(votes_of(db).opt())).window(rank, |(((_, s), d), _)| (Reverse(s), Reverse(d)), asc);
    let top = (&w).filt(|(_, r)| r <= 100).map(|((((p, _), _), _), r)| (p, r));
    type R = (Id<Post>, i64);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = db.badge.group_by(&db.badge.user_id).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 3) as i64, a[1] + (c == 2) as i64, a[2] + (c == 1) as i64]);
    let v = drain(top.select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select((&vc).and(owner_user_id.select(&bc).opt())))));
    rows(v.into_iter().map(|(_, ((p, r), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers", "comments"]);
        f.extend(u.map(V::I));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COALESCE(v.UpVotes, 0) AS UpVotes,
//        COALESCE(v.DownVotes, 0) AS DownVotes, p.AnswerCount, p.CommentCount, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v
//     ON p.Id = v.PostId WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT r.PostId, r.Title, r.Body, r.Tags, r.CreationDate, r.OwnerUserId, r.OwnerDisplayName, r.UpVotes, r.DownVotes, (r.UpVotes - r.DownVotes) AS NetScore,
//        COUNT(c.Id) AS CommentCount FROM RankedPosts r LEFT JOIN Comments c ON r.PostId = c.PostId WHERE r.Rank <= 5
//     GROUP BY r.PostId, r.Title, r.Body, r.Tags, r.CreationDate, r.OwnerUserId, r.OwnerDisplayName, r.UpVotes, r.DownVotes, r.Rank)
// SELECT fp.PostId, fp.Title, fp.Body, fp.Tags, fp.CreationDate, fp.OwnerDisplayName, fp.UpVotes, fp.DownVotes, fp.NetScore, fp.CommentCount
// FROM FilteredPosts fp ORDER BY fp.NetScore DESC, fp.CreationDate DESC;
fn q25134(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(tags_str.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&vc).and(&cc)).into_iter().map(|(p, (u, c))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "owner"]);
        f.extend([V::I(u[0]), V::I(u[1]), V::I(u[0] - u[1]), V::I(c)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotesCount,
//        SUM(CASE WHEN v.VoteTypeId = 8 THEN v.BountyAmount ELSE 0 END) AS TotalBountySpent FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentsCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.Score),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.UpVotes, ps.DownVotes, ps.CommentsCount, RANK() OVER (ORDER BY ps.Score DESC, ps.UpVotes DESC) AS ScoreRank FROM PostStats ps)
// SELECT ups.UserId, ups.DisplayName, ups.UpVotesCount, ups.DownVotesCount, ups.TotalBountySpent, rp.PostId, rp.Title, rp.Score, rp.UpVotes, rp.DownVotes, rp.CommentsCount
// FROM UserVoteStats ups CROSS JOIN (SELECT PostId, Title, Score, UpVotes, DownVotes, CommentsCount FROM RankedPosts WHERE ScoreRank <= 10) rp
// WHERE ups.UpVotesCount > ups.DownVotesCount AND (ups.TotalBountySpent > 0 OR ups.UpVotesCount > 10) ORDER BY ups.DisplayName, rp.Score DESC;
fn q3709(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt()).fold([0i64; 4], |a, v| match v {
        Some((t, b)) => {
            let x = if t == 8 { b } else { Some(0) };
            [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + x.is_some() as i64, a[3] + x.unwrap_or(0)]
        }
        None => a,
    });
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let rk = whole(&ps).select(Ident::<Post>::new().and(&db.post.score).and(&ps)).window(rank, |((_, s), a)| (Reverse(s), Reverse(a[0])), asc);
    let rp: MatSet<(Id<Post>, [i64; 3])> = (&rk).filt(|(_, r)| r <= 10).map(|(((p, _), a), _)| (p, a)).collect();
    let uq = (&us).filt(|a: [i64; 4]| a[0] > a[1] && ((a[2] > 0 && a[3] > 0) || a[0] > 10));
    let mut v = Vec::new();
    uq.cross(&rp).drive(|(u, _), (a, (p, s))| v.push((u, a, p, s)));
    rows(v.into_iter().map(|(u, a, p, s)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), nullable(a[3], a[2])]);
        f.extend(post_fields(db, p, &["id", "title", "score"]));
        f.extend(s.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswer,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId, ARRAY_LENGTH(string_to_array(p.Tags, '<>'), 1) AS TagCount
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStatistics AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopQuestions AS (SELECT rp.Id AS PostId, rp.Title, rp.Score, rp.ViewCount, us.UserId, us.GoldBadges, us.SilverBadges, us.BronzeBadges
//     FROM RankedPosts rp JOIN Users u ON u.Id = rp.OwnerUserId JOIN UserStatistics us ON us.UserId = u.Id WHERE rp.rn = 1 AND rp.Score > 0)
// SELECT tq.PostId, tq.Title, tq.Score, tq.ViewCount, tq.GoldBadges, tq.SilverBadges, tq.BronzeBadges, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = tq.PostId) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = tq.PostId AND v.VoteTypeId = 2) AS UpVoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = tq.PostId AND v.VoteTypeId = 3) AS DownVoteCount
// FROM TopQuestions tq ORDER BY tq.Score DESC, tq.ViewCount DESC LIMIT 10;
fn q33518(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let top = top_n(drain((&first).with(score.gt(0)).with(owner_user)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc).and(owner_user.select(&ub))).into_iter().map(|(p, ((c, u), b))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(b.map(V::I));
        f.extend([V::I(c), V::I(u[0]), V::I(u[1])]);
        row(f)
    }))
}

// WITH RecursiveCTE AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COALESCE(SUM(p.Score), 0) AS TotalScore, COUNT(DISTINCT p.Id) AS QuestionCount
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id AND p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName),
// CloseReasons AS (SELECT ph.UserId, COUNT(*) AS TotalCloseVotes FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId),
// RankedUsers AS (SELECT ua.UserId, ua.DisplayName, ua.TotalViews, ua.TotalScore, ua.QuestionCount, cr.TotalCloseVotes, RANK() OVER (ORDER BY ua.TotalScore DESC, ua.TotalViews DESC) AS UserRank
//     FROM UserActivity ua LEFT JOIN CloseReasons cr ON ua.UserId = cr.UserId)
// SELECT ru.UserId, ru.DisplayName, ru.TotalViews, ru.TotalScore, ru.QuestionCount, COALESCE(ru.TotalCloseVotes, 0) AS TotalCloseVotes, ru.UserRank, rcte.PostId, rcte.Title,
//        rcte.CreationDate, rcte.Score
// FROM RankedUsers ru LEFT JOIN RecursiveCTE rcte ON ru.UserId = rcte.OwnerUserId WHERE ru.QuestionCount > 0 ORDER BY ru.UserRank, rcte.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// Named RecursiveCTE, but it is a plain CTE; its UserPostRank is never read.
fn q33949(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, score, creation_date, .. } = &db.post;
    let asked = || Ident::<Post>::new().with(post_type_id.eq(1));
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(asked()).select(view_count.opt().and(score)).opt()).fold([0i64; 3], |a, p| match p {
        Some((w, s)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s],
        None => a,
    });
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let ru = win(&ua, rank, |(_, a)| (Reverse(a[2]), Reverse(a[1])), asc);
    type R = ((Id<User>, [i64; 3]), i64);
    let qidx: HashIdx<Id<User>, Id<Post>> = db.post.with(post_type_id.eq(1)).select(owner_user).inv().collect();
    let v = drain((&ru).filt(|((_, a), _): R| a[0] > 0).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| u).select((&cr).opt().and(&qidx)))));
    let v = top_n(v, |&(_, ((_, r), (_, p)))| (r, Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(_, (((u, a), r), (c, p)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[0]), V::I(c.unwrap_or(0)), V::I(r)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastCloseDate,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS LastReopenDate FROM PostHistory ph GROUP BY ph.PostId),
// CommentsAggregate AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate AS PostCreationDate, rp.Score, rp.ViewCount, ur.DisplayName AS OwnerName, ur.Reputation AS OwnerReputation, ur.BadgeCount,
//        COALESCE(ps.EditCount, 0) AS PostEditCount, ps.LastCloseDate, ps.LastReopenDate, COALESCE(ca.CommentCount, 0) AS TotalComments, ca.LastCommentDate
// FROM RankedPosts rp JOIN UserReputation ur ON ur.UserId = rp.OwnerUserId LEFT JOIN PostHistorySummary ps ON ps.PostId = rp.PostId LEFT JOIN CommentsAggregate ca ON ca.PostId = rp.PostId
// WHERE rp.PostRank = 1 ORDER BY ur.Reputation DESC, rp.ViewCount DESC LIMIT 100;
fn q30042(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let top = top_n(drain((&first).select(owner_user)), |&(p, u)| {
        let w = view_count.get(p);
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w))
    }, 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let hs = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd)).opt()).fold((0i64, i64::MIN, i64::MIN), |(n, c, r), h| match h {
        Some((t, d)) => (n + 1, if t == 10 { c.max(d) } else { c }, if t == 11 { r.max(d) } else { r }),
        None => (n, c, r),
    });
    let ca = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    rows(drain((&hs).and(&ca).and(owner_user.select(&bc))).into_iter().map(|(p, (((n, c, r), (k, m)), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep"]);
        f.extend([V::I(b), V::I(n), tmax(c), tmax(r), V::I(k), tmax(m)]);
        row(f)
    }))
}

// WITH FilteredPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.Tags, p.AnswerCount, p.ViewCount, p.CommentCount,
//        ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><'), 1) AS TagCount
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.AnswerCount > 0 AND p.ViewCount > 100),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(c.Score, 0)) AS TotalComments, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties,
//        AVG(COALESCE(c.Score, 0)) AS AvgCommentScore, SUM(fp.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN FilteredPosts fp ON u.Id = fp.OwnerUserId
//     GROUP BY u.Id, u.DisplayName HAVING SUM(COALESCE(c.Score, 0)) > 0 OR SUM(COALESCE(v.BountyAmount, 0)) > 0),
// TopUsers AS (SELECT ue.UserId, ue.DisplayName, ue.TotalComments, ue.TotalBounties, ue.AvgCommentScore, ue.TotalViews,
//        ROW_NUMBER() OVER (ORDER BY ue.TotalComments DESC, ue.TotalBounties DESC) AS UserRank FROM UserEngagement ue)
// SELECT tu.UserId, tu.DisplayName, tu.TotalComments, tu.TotalBounties, tu.AvgCommentScore, tu.TotalViews FROM TopUsers tu WHERE tu.UserRank <= 10
// ORDER BY tu.TotalComments DESC, tu.TotalBounties DESC;
fn q25885(db: &'static So) -> String {
    let Post { creation_date, answer_count, view_count, .. } = &db.post;
    let fp = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(answer_count.gt(0)).with(view_count.gt(100));
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(
            comments_by(db)
                .select(&db.comment.score)
                .opt()
                .and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())
                .and(posts_of(db).select(fp).select(view_count).opt()),
        )
        .fold([0i64; 5], |a, ((c, b), w)| {
            [a[0] + c.unwrap_or(0), a[1] + b.flatten().unwrap_or(0), a[2] + 1, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
        });
    let v = top_n(drain((&ue).filt(|a| a[0] > 0 || a[1] > 0)), |&(_, a)| (Reverse(a[0]), Reverse(a[1])), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[0], a[2]), nullable(a[4], a[3])]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount FROM Posts p GROUP BY p.OwnerUserId),
// RankedUsers AS (SELECT ub.UserId, ub.DisplayName, COALESCE(ps.QuestionCount, 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount, COALESCE(ps.TotalScore, 0) AS TotalScore,
//        COALESCE(ps.AvgViewCount, 0) AS AvgViewCount, ub.BadgeCount, ROW_NUMBER() OVER (ORDER BY COALESCE(ps.TotalScore, 0) DESC, ub.BadgeCount DESC) AS UserRank
//     FROM UserBadges ub LEFT JOIN PostStats ps ON ub.UserId = ps.OwnerUserId)
// SELECT ru.UserId, ru.DisplayName, ru.QuestionCount, ru.AnswerCount, ru.TotalScore, ru.AvgViewCount, ru.BadgeCount,
//        CASE WHEN ru.BadgeCount > 10 THEN 'High Achiever' WHEN ru.BadgeCount BETWEEN 5 AND 10 THEN 'Achiever' ELSE 'Novice' END AS AchievementLevel
// FROM RankedUsers ru WHERE ru.AnswerCount > 0 ORDER BY ru.UserRank LIMIT 100;
fn q3486(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&ups).filt(|a| a[3] > 0).and(&bc)), |&(_, (a, b))| (Reverse(a[4]), Reverse(b)), 100);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[2]), V::I(a[3]), V::I(a[4]), if a[5] == 0 { V::F(0.0) } else { avg(a[6], a[5]) }, V::I(b)]);
        f.push(V::S(if b > 10 { "High Achiever" } else if b >= 5 { "Achiever" } else { "Novice" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(P.Score) AS AvgScore FROM Posts P GROUP BY P.OwnerUserId),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UB.TotalBadges, 0) AS TotalBadges, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.Questions, 0) AS Questions,
//        COALESCE(PS.Answers, 0) AS Answers, COALESCE(PS.AvgScore, 0) AS AvgScore FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId),
// RankedUsers AS (SELECT UA.*, ROW_NUMBER() OVER (ORDER BY UA.Reputation DESC, UA.TotalBadges DESC) AS UserRank FROM UserActivity UA)
// SELECT R.UserId, R.DisplayName, R.Reputation, R.TotalBadges, R.TotalPosts, R.Questions, R.Answers, R.AvgScore,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.UserId = R.UserId AND V.VoteTypeId IN (2, 3)), 0) AS TotalVotes
// FROM RankedUsers R WHERE R.UserRank <= 10 ORDER BY R.UserRank;
fn q1738(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + matches!(t, Some(2 | 3)) as i64);
    let v = top_n(drain((&ups).and(&bc).and(&tv)), |&(u, ((_, b), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b)), 10);
    rows(v.into_iter().map(|(u, ((a, b), t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[1] == 0 { V::F(0.0) } else { avg(a[4], a[1]) }, V::I(t)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, AVG(COALESCE(p.Score, 0)) AS AverageScore, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName),
// RankedUserStats AS (SELECT UserId, DisplayName, TotalPosts, AnswerCount, QuestionCount, AverageScore, TotalBounties, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank,
//        RANK() OVER (ORDER BY AverageScore DESC) AS ScoreRank FROM UserPostStats),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS UsageCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' WHERE p.PostTypeId = 1 GROUP BY t.TagName HAVING COUNT(p.Id) > 10),
// TopTags AS (SELECT TagName, UsageCount, RANK() OVER (ORDER BY UsageCount DESC) AS TagRank FROM PopularTags)
// SELECT r.DisplayName, r.TotalPosts, r.AnswerCount, r.QuestionCount, r.AverageScore, r.TotalBounties, t.TagName, t.UsageCount
// FROM RankedUserStats r LEFT JOIN TopTags t ON r.PostRank <= 10 AND t.TagRank <= 5 WHERE r.TotalPosts > 0 ORDER BY r.TotalPosts DESC, r.AverageScore DESC;
//
// The ON clause is a predicate on each side, so the users with PostRank <= 10 are crossed with the top tags and the rest with the one NULL row.
fn q4297(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score).and(bounty.opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, s), b)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + s, a[4] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let ur = win(&us, rank, |(_, a)| Reverse(a[0]), asc);
    type P = (Id<Post>, Id<Tag>);
    let lt = tag_mentions(db);
    let asked = Ident::<Post>::new().with((&db.post.post_type_id).eq(1));
    let uc = (&lt).group_by(Same::<P>::new().map(|(_, t): P| t).select(&db.tag.tag_name)).select(Same::<P>::new().map(|(p, _): P| p).select(asked)).fold(0i64, |n, _| n + 1);
    let tw = whole((&uc).filt(|n| n > 10)).select(Same::<Str>::new().and(&uc)).window(rank, |(_, n)| Reverse(n), asc);
    let tt: HashIdx<(), (Str, i64)> = (&tw).filt(|(_, r)| r <= 5).map(|(x, _)| x).collect();
    type R = ((Id<User>, [i64; 5]), i64);
    let v = drain((&ur).filt(|((_, a), _)| a[0] > 0).select(Same::<R>::new().and(Same::<R>::new().filt(|(_, r): R| r <= 10).map(|_| ()).select(&tt).opt())));
    rows(v.into_iter().map(|(_, (((u, a), _), t))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(a[4])];
        f.extend(match t {
            Some((t, n)) => [V::S(t), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.PostRank = 1),
// PostStatistics AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount)
// SELECT ps.PostId, ps.Title, ps.OwnerDisplayName, ps.CreationDate, ps.Score, ps.ViewCount, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount,
//        CASE WHEN ps.UpvoteCount - ps.DownvoteCount > 0 THEN 'Positive' WHEN ps.UpvoteCount - ps.DownvoteCount < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment
// FROM PostStatistics ps ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q9202(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&pa).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, MAX(ph.CreationDate) AS LastEditDate
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId
//     GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.OwnerDisplayName)
// SELECT pd.PostId, pd.Title, pd.OwnerDisplayName, pd.CreationDate, pd.ViewCount, pd.Score, pd.CommentCount, pd.UpVoteCount, pd.DownVoteCount, pd.LastEditDate
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
fn q9905(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let pd = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold([0, 0, 0, i64::MIN], |a, ((c, t), d)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, d.map_or(a[3], |d| a[3].max(d))]);
    rows(drain(&pd).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), tmax(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.ViewCount IS NOT NULL GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// TopQuestions AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.CommentCount,
//        CASE WHEN rp.Score >= 10 THEN 'High Score' WHEN rp.Score BETWEEN 5 AND 9 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory FROM RankedPosts rp WHERE rp.PostRank <= 5),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId AND v.PostId IN (SELECT PostId FROM TopQuestions) GROUP BY u.Id, u.DisplayName)
// SELECT ua.UserId, ua.DisplayName, ua.BadgeCount, ua.TotalBounties, tq.Title, tq.ViewCount, tq.Score, tq.CommentCount,
//        CASE WHEN ua.BadgeCount > 5 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributionLevel
// FROM UserActivity ua INNER JOIN TopQuestions tq ON ua.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tq.PostId LIMIT 1) ORDER BY ua.TotalBounties DESC, tq.Score DESC;
fn q3750(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, owner_user, .. } = &db.post;
    let w = db.post.with(view_count).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tq: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tq).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let on_tq = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.post).select(&tq)));
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(on_tq.select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (b, v)| {
            let v = v.flatten();
            [a[0] + b.is_some() as i64, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0)]
        });
    rows(drain((&cc).and(owner_user.select(Ident::<User>::new().and(&ua)))).into_iter().map(|(p, (c, (u, a)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1])]);
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(c), V::S(if a[0] > 5 { "Top Contributor" } else { "Contributor" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE(COUNT(a.Id), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        RANK() OVER (ORDER BY COALESCE(SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes vt ON p.Id = vt.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// RecentActivity AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.AnswerCount, rp.UpVotes, rp.DownVotes, ra.CommentCount, ra.LastCommentDate, rp.Rank
//     FROM RankedPosts rp LEFT JOIN RecentActivity ra ON rp.PostId = ra.PostId)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.OwnerDisplayName, pd.AnswerCount, pd.UpVotes, pd.DownVotes, pd.CommentCount, pd.LastCommentDate, pd.Rank
// FROM PostDetails pd WHERE pd.Rank <= 10 ORDER BY pd.Rank;
fn q7390(db: &'static So) -> String {
    let rp = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = win(&rp, rank, |(_, a)| Reverse(a[1] - a[2]), asc);
    type R = (Id<Post>, [i64; 3], i64);
    let tp: MatSet<R> = (&w).filt(|(_, r)| r <= 10).map(|((p, a), r)| (p, a, r)).collect();
    let ra = (&tp).map(|(p, _, _): R| p).group_by(Same::<Id<Post>>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    rows(drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _): R| p).select(&ra)))).into_iter().map(|(_, ((p, a, r), (n, m)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.extend([V::I(n), tmax(m), V::I(r)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 4 THEN 1 ELSE 0 END) AS TotalTagWikis,
//        SUM(CASE WHEN p.PostTypeId = 5 THEN 1 ELSE 0 END) AS TotalExcerpts, SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END) AS TotalScore, AVG(v.BountyAmount) AS AvgBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE u.Reputation > 50 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalQuestions, ua.TotalAnswers, ua.TotalTagWikis, ua.TotalExcerpts, ua.TotalScore, ua.AvgBounty,
//        DENSE_RANK() OVER (ORDER BY ua.TotalScore DESC) AS RankByScore, DENSE_RANK() OVER (ORDER BY ua.TotalPosts DESC) AS RankByPosts FROM UserActivity ua)
// SELECT tu.UserId, tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalTagWikis, tu.TotalExcerpts, tu.TotalScore, tu.AvgBounty, tu.RankByScore, tu.RankByPosts,
//        CASE WHEN tu.RankByScore = tu.RankByPosts THEN 'Equal Ranking' WHEN tu.RankByScore < tu.RankByPosts THEN 'Higher Score Rank' ELSE 'Higher Post Rank' END AS RankingComparison
// FROM TopUsers tu WHERE tu.RankByScore <= 10 OR tu.RankByPosts <= 10 ORDER BY tu.TotalScore DESC, tu.TotalPosts DESC;
fn q27578(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ua = db
        .user
        .with((&db.user.reputation).gt(50))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score).and(bounty.opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((t, s), b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 4) as i64, a[3] + (t == 5) as i64, a[4] + s.max(0), a[5] + b.is_some() as i64, a[6] + b.unwrap_or(0)]
            }
            None => a,
        });
    let dp = db.user.with((&db.user.reputation).gt(50)).group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    type T = (Id<User>, ([i64; 7], Option<i64>));
    let w = whole(&ua).select(Ident::<User>::new().and((&ua).and((&dp).opt()))).window(dense_rank, |(_, (a, _)): T| Reverse(a[4]), asc);
    let w = (&w).window(dense_rank, |((_, (_, n)), _): (T, i64)| Reverse(n.unwrap_or(0)), asc);
    let mut v: Vec<_> = drain((&w).filt(|((_, s), p)| s <= 10 || p <= 10).map(|(((u, (a, n)), s), p)| (((u, (a, n.unwrap_or(0))), s), p)));
    v.sort_by_key(|&(_, (((_, (a, n)), _), _))| (Reverse(a[4]), Reverse(n)));
    rows(v.into_iter().map(|(_, (((u, (a, n)), s), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[6], a[5]), V::I(s), V::I(p)]);
        f.push(V::S(if s == p { "Equal Ranking" } else if s < p { "Higher Score Rank" } else { "Higher Post Rank" }));
        row(f)
    }))
}

// WITH TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 1000),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, RANK() OVER (ORDER BY P.ViewCount DESC) AS PopularityRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND P.PostTypeId IN (1, 2)),
// TopBadges AS (SELECT B.UserId, B.Name AS BadgeName, COUNT(B.Id) AS BadgeCount, RANK() OVER (PARTITION BY B.UserId ORDER BY COUNT(B.Id) DESC) AS BadgeRank FROM Badges B GROUP BY B.UserId, B.Name),
// ActiveComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount, RANK() OVER (PARTITION BY C.PostId ORDER BY COUNT(C.Id) DESC) AS CommentRank FROM Comments C GROUP BY C.PostId)
// SELECT TU.DisplayName AS TopUser, TU.Reputation, PP.Title AS PopularPost, PP.ViewCount, AB.BadgeName, AB.BadgeCount, COALESCE(AC.CommentCount, 0) AS ActiveComments
// FROM TopUsers TU JOIN PopularPosts PP ON PP.OwnerDisplayName = TU.DisplayName LEFT JOIN TopBadges AB ON AB.UserId = TU.UserId AND AB.BadgeRank = 1
// LEFT JOIN ActiveComments AC ON AC.PostId = PP.PostId WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC, PP.ViewCount DESC;
fn q9279(db: &'static So) -> String {
    let User { reputation, display_name, .. } = &db.user;
    let tw = whole(db.user.with(reputation.gt(1000))).select(Ident::<User>::new().and(reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&tw).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let pp = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])));
    let by_name: HashIdx<Str, Id<Post>> = pp.select(owner_user.select(display_name)).inv().collect();
    let Badge { user, name, .. } = &db.badge;
    let tb = rel(drain(db.badge.group_by(user.and(name)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1)));
    let by_user: HashIdx<Id<User>, ((Id<User>, Str), i64)> = (&tb).map(|((u, _), _)| u).inv().select(&tb).collect();
    let tw = (&by_user).window(rank, |(_, k)| Reverse(k), asc);
    let tbu = (&tw).filt(|(_, r)| r == 1).map(|(x, _)| x);
    let ac = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).select(Ident::<User>::new().and(display_name.select(&by_name).select(Ident::<Post>::new().and((&ac).opt()))).and(tbu.opt())));
    rows(v.into_iter().map(|(_, ((u, (p, c)), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend(match b {
            Some(((_, n), k)) => [V::S(n), V::I(k)],
            None => [V::Null, V::Null],
        });
        f.push(V::I(c.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UBC.BadgeCount, 0) AS BadgeCount, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.Questions, 0) AS Questions,
//        COALESCE(PS.Answers, 0) AS Answers, COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.TotalScore, 0) AS TotalScore
//     FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN PostSummary PS ON U.Id = PS.OwnerUserId)
// SELECT UE.DisplayName, UE.Reputation, UE.BadgeCount, UE.TotalPosts, UE.Questions, UE.Answers, UE.TotalViews, UE.TotalScore, RANK() OVER (ORDER BY UE.TotalScore DESC) AS ScoreRank
// FROM UserEngagement UE WHERE UE.Reputation > 1000 ORDER BY UE.TotalScore DESC, UE.BadgeCount DESC;
fn q25032(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = whole(db.user.with((&db.user.reputation).gt(1000))).select(Ident::<User>::new().and((&ups).and(&bc))).window(rank, |(_, (a, _))| Reverse(a[4]), asc);
    rows(drain(&w).into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days'),
// PostSummary AS (SELECT u.UserId, u.DisplayName, COUNT(p.PostId) AS TotalPosts, COUNT(c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM RankedUsers u LEFT JOIN RecentPosts p ON u.UserId = p.OwnerUserId LEFT JOIN Comments c ON p.PostId = c.PostId LEFT JOIN Votes v ON p.PostId = v.PostId GROUP BY u.UserId, u.DisplayName)
// SELECT ps.UserId, ps.DisplayName, ps.TotalPosts, ps.TotalComments, ps.TotalUpVotes, ps.TotalDownVotes,
//        CASE WHEN ps.TotalPosts > 0 THEN ROUND((ps.TotalUpVotes * 1.0 / GREATEST(ps.TotalPosts, 1)) * 100, 2) ELSE 0 END AS UpVotePercentage,
//        CASE WHEN ps.TotalDownVotes > 0 THEN ROUND((ps.TotalDownVotes * 1.0 / GREATEST(ps.TotalPosts, 1)) * 100, 2) ELSE 0 END AS DownVotePercentage, ur.ReputationRank
// FROM PostSummary ps JOIN RankedUsers ur ON ps.UserId = ur.UserId WHERE ur.ReputationRank <= 50 ORDER BY ur.ReputationRank, ps.TotalPosts DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x comment x vote product is driven for those alone.
fn q6586(db: &'static So) -> String {
    let uw = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let ur = (&uw).filt(|(_, r)| r <= 50).map(|((u, _), r)| (u, r));
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let ps = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(recent).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((c, t)) => [a[0] + 1, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let pct = |x: i64, n: i64| V::F((x as f64 / n.max(1) as f64 * 100.0 * 100.0).round() / 100.0);
    type R = (Id<User>, i64);
    rows(drain(ur.select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&ps)))).into_iter().map(|(_, ((u, r), a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(if a[0] > 0 { pct(a[2], a[0]) } else { V::F(0.0) });
        f.push(if a[3] > 0 { pct(a[3], a[0]) } else { V::F(0.0) });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p WHERE p.PostTypeId IN (1, 2)),
// RecentBadges AS (SELECT u.Id AS UserId, u.DisplayName, b.Name AS BadgeName, b.Date AS BadgeDate, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY b.Date DESC) AS BadgeRank
//     FROM Users u JOIN Badges b ON u.Id = b.UserId WHERE b.Class = 1),
// CommentStatistics AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN c.Score > 0 THEN 1 ELSE 0 END) AS PositiveComments FROM Comments c GROUP BY c.PostId),
// VoteStatistics AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, CASE WHEN rs.UserId IS NOT NULL THEN 'Yes' ELSE 'No' END AS HasRecentGoldBadge, cs.CommentCount, cs.PositiveComments,
//        vs.UpVotes, vs.DownVotes, vs.TotalVotes
// FROM RankedPosts rp LEFT JOIN RecentBadges rs ON rp.PostId = rs.UserId AND rs.BadgeRank = 1 LEFT JOIN CommentStatistics cs ON rp.PostId = cs.PostId
// LEFT JOIN VoteStatistics vs ON rp.PostId = vs.PostId WHERE rp.RankByScore <= 10 ORDER BY rp.PostId;
//
// `rp.PostId = rs.UserId` joins a post id to a user id, so it goes through the raw ids; BadgeRank = 1 keeps one row per user with a gold badge.
fn q33551(db: &'static So) -> String {
    let Post { post_type_id, score, origid, .. } = &db.post;
    let w = db.post.with(post_type_id.is_in([1, 2])).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let cs = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + (s > 0) as i64]);
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let v = drain((&tp).select(Ident::<Post>::new().and(origid.select(&uidx).select(Ident::<User>::new().with(&gold)).opt()).and((&cs).opt()).and((&vs).opt())));
    rows(v.into_iter().map(|(_, (((p, g), c), w))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.push(V::S(if g.is_some() { "Yes" } else { "No" }));
        f.extend(match c {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null],
        });
        f.extend(match w {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers, SUM(p.Score) AS TotalScore,
//        SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// UserAggregates AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(ps.Questions, 0) AS Questions, COALESCE(ps.Answers, 0) AS Answers, COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(ps.TotalViews, 0) AS TotalViews
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId),
// RankedUsers AS (SELECT ua.*, RANK() OVER (ORDER BY ua.TotalScore DESC, ua.TotalViews DESC) AS ScoreRank FROM UserAggregates ua)
// SELECT ru.DisplayName, ru.GoldBadges, ru.SilverBadges, ru.BronzeBadges, ru.Questions, ru.Answers, ru.TotalScore, ru.TotalViews,
//        CASE WHEN ru.ScoreRank <= 10 THEN 'Top User' WHEN ru.ScoreRank <= 50 THEN 'Moderate User' ELSE 'New User' END AS UserCategory
// FROM RankedUsers ru WHERE ru.TotalScore > 0 ORDER BY ru.ScoreRank FETCH FIRST 20 ROWS ONLY;
fn q2961(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let w = whole(&ups).select(Ident::<User>::new().and((&ups).and(&ub))).window(rank, |(_, (a, _))| (Reverse(a[4]), Reverse(a[6])), asc);
    let v = top_n(drain((&w).filt(|((_, (a, _)), _)| a[4] > 0)), |&(_, (_, r))| r, 20);
    rows(v.into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend([V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6])]);
        f.push(V::S(if r <= 10 { "Top User" } else if r <= 50 { "Moderate User" } else { "New User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// UserScores AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(pc.CommentCount, 0) AS TotalComments, COALESCE(lp.LikeCount, 0) AS TotalLikes, (u.UpVotes - u.DownVotes) AS NetVotes
//     FROM Users u LEFT JOIN PostComments pc ON u.Id = pc.PostId
//     LEFT JOIN (SELECT p.OwnerUserId, COUNT(v.Id) AS LikeCount FROM Votes v INNER JOIN Posts p ON p.Id = v.PostId WHERE v.VoteTypeId = 2 GROUP BY p.OwnerUserId) lp ON u.Id = lp.OwnerUserId),
// FinalScores AS (SELECT rs.PostId, rs.Title, rs.CreationDate, us.Reputation, us.TotalComments, us.TotalLikes, us.NetVotes,
//        (us.Reputation * 0.5 + us.TotalComments * 0.3 + us.TotalLikes * 0.2) AS WeightedScore FROM RankedPosts rs JOIN UserScores us ON rs.PostId = us.UserId WHERE us.Reputation > 100)
// SELECT PostId, Title, CreationDate, Reputation, TotalComments, TotalLikes, WeightedScore FROM FinalScores WHERE WeightedScore > 10 ORDER BY WeightedScore DESC;
//
// `u.Id = pc.PostId` and `rs.PostId = us.UserId` join user ids to post ids, so both go through the raw ids.
fn q3044(db: &'static So) -> String {
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let lp = db.vote.with((&db.vote.vote_type_id).eq(2)).group_by((&db.vote.post).select(&db.post.owner_user)).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let us = Ident::<User>::new()
        .with((&db.user.reputation).gt(100))
        .select(&db.user.reputation)
        .and((&db.user.origid).select(&pidx).select(&pc).opt())
        .and((&lp).opt())
        .map(|((r, c), l): ((i64, Option<i64>), Option<i64>)| (r, c.unwrap_or(0), l.unwrap_or(0), r * 5 + c.unwrap_or(0) * 3 + l.unwrap_or(0) * 2));
    let recent = db.post.with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let v = drain(recent.select((&db.post.origid).select(&uidx).select(us)).filt(|(_, _, _, w)| w > 100));
    rows(v.into_iter().map(|(p, (r, c, l, w))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(r), V::I(c), V::I(l), V::F(w as f64 / 10.0)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.AcceptedAnswerId,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p WHERE p.CreationDate >= '2023-01-01' AND p.Score > 10),
// UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryAggregate AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseCount, MAX(ph.CreationDate) AS LastChangeDate
//     FROM PostHistory ph WHERE ph.CreationDate >= '2023-01-01' GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, pha.CloseCount, pha.LastChangeDate
// FROM RankedPosts rp LEFT JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId
// LEFT JOIN PostHistoryAggregate pha ON rp.PostId = pha.PostId
// WHERE (pha.CloseCount IS NULL OR pha.CloseCount < 2) AND (ub.BadgeCount IS NOT NULL AND ub.BadgeCount >= 1) AND rp.Rank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC, rp.PostId;
fn q22621(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let since = ts(2023, 1, 1, 0, 0, 0);
    let w = db
        .post
        .with(creation_date.ge(since).and(score.gt(10)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.with(hd.ge(since)).group_by(post).select(post_history_type_id.and(hd)).fold((0i64, i64::MIN), |(n, m), (t, d)| (n + matches!(t, 10 | 11) as i64, m.max(d)));
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select((&ub).filt(|a| a[0] >= 1))).and((&pha).opt())).filt(|(_, h): ((Id<Post>, [i64; 4]), Option<(i64, i64)>)| h.map_or(true, |(n, _)| n < 2)));
    rows(v.into_iter().map(|(_, ((p, b), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(b.map(V::I));
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT P.Id, P.Title, P.CreationDate, P.OwnerUserId, COUNT(A.Id) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RN
//     FROM Posts P LEFT JOIN Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2 LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.PostTypeId = 1 AND P.CreationDate >= CURRENT_DATE - INTERVAL '6 MONTH' GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, P.Score),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, MAX(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS HasGold, MAX(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS HasSilver,
//        MAX(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS HasBronze FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT U.DisplayName, U.Reputation, RP.Title, RP.CreationDate, RP.AnswerCount, RP.UpVotes, RP.DownVotes, RP.Score, UB.BadgeCount,
//        CASE WHEN UB.HasGold = 1 THEN 'Gold Badge' WHEN UB.HasSilver = 1 THEN 'Silver Badge' WHEN UB.HasBronze = 1 THEN 'Bronze Badge' ELSE 'No Badge' END AS BadgeStatus
// FROM RecentPosts RP JOIN Users U ON RP.OwnerUserId = U.Id LEFT JOIN UserBadges UB ON U.Id = UB.UserId WHERE RP.RN = 1 ORDER BY RP.UpVotes DESC, RP.Score DESC, RP.CreationDate DESC;
//
// RN reads only base columns, so each owner's newest question is picked first and the answer x vote product is driven for those alone.
fn q31482(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_months(current_date(), -6))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1].max((c == 1) as i64), a[2].max((c == 2) as i64), a[3].max((c == 3) as i64)],
        None => a,
    });
    rows(drain((&rp).and(owner_user.select(Ident::<User>::new().and(&ub)))).into_iter().map(|(p, (a, (u, b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend(a.map(V::I));
        f.extend([V::I(db.post.score.get(p).unwrap()), V::I(b[0])]);
        f.push(V::S(if b[1] == 1 { "Gold Badge" } else if b[2] == 1 { "Silver Badge" } else if b[3] == 1 { "Bronze Badge" } else { "No Badge" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p
//     WHERE p.Score > 0 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS TotalComments,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM TopPosts tp LEFT JOIN Users u ON tp.PostId = u.Id LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, u.DisplayName)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.OwnerDisplayName, pd.TotalComments, pd.TotalUpvotes, pd.TotalDownvotes
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
//
// `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q7649(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(score.gt(0).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pd = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&pd).and(origid.select(&uidx).opt())).into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.push(u.map_or(V::Null, |u| user_col(db, u, "name")));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        p.OwnerUserId FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// UsersWithBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadge, MAX(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadge,
//        MAX(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadge FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, ub.BadgeCount, ub.GoldBadge, ub.SilverBadge, ub.BronzeBadge, ps.CommentCount, ps.UpVotes, ps.DownVotes,
//        COALESCE(ps.UpVotes, 0) AS TotalUpVotes, COALESCE(ps.DownVotes, 0) AS TotalDownVotes
// FROM RankedPosts rp LEFT JOIN UsersWithBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostStats ps ON rp.PostId = ps.PostId WHERE rp.Rank <= 5 ORDER BY rp.CreationDate DESC;
//
// PostStats is joined per post, so it is computed for the ranked posts alone.
fn q30838(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1].max((c == 1) as i64), a[2].max((c == 2) as i64), a[3].max((c == 3) as i64)],
        None => a,
    });
    rows(drain((&ps).and(owner_user.select(&ub))).into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend(b.map(V::I));
        f.extend(a.map(V::I));
        f.extend([V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(A.AnswerCount), 0) AS TotalAnswers, COALESCE(SUM(Q.QuestionCount), 0) AS TotalQuestions,
//        COALESCE(SUM(C.CommentCount), 0) AS TotalComments, RANK() OVER (ORDER BY COALESCE(SUM(A.AnswerCount), 0) DESC) AS AnswerRank
//     FROM Users U LEFT JOIN (SELECT OwnerUserId, COUNT(Id) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY OwnerUserId) A ON U.Id = A.OwnerUserId
//     LEFT JOIN (SELECT OwnerUserId, COUNT(Id) AS QuestionCount FROM Posts WHERE PostTypeId = 1 GROUP BY OwnerUserId) Q ON U.Id = Q.OwnerUserId
//     LEFT JOIN (SELECT UserId, COUNT(Id) AS CommentCount FROM Comments GROUP BY UserId) C ON U.Id = C.UserId GROUP BY U.Id, U.DisplayName),
// HighActivityUsers AS (SELECT UserId, DisplayName, TotalAnswers, TotalQuestions, TotalComments, AnswerRank FROM UserActivity WHERE TotalAnswers > 10 OR TotalQuestions > 10),
// RecentPosts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS RecentPostCount FROM Posts P WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY P.OwnerUserId)
// SELECT U.DisplayName, U.TotalAnswers, U.TotalQuestions, U.TotalComments, COALESCE(RP.RecentPostCount, 0) AS RecentPosts
// FROM HighActivityUsers U LEFT JOIN RecentPosts RP ON U.UserId = RP.OwnerUserId ORDER BY U.TotalAnswers DESC, U.TotalQuestions DESC LIMIT 100;
fn q1280(db: &'static So) -> String {
    let ups = user_posts(db);
    let cc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let recent = Ident::<Post>::new().with((&db.post.creation_date).gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let rp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(recent).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&ups).filt(|a| a[3] > 10 || a[2] > 10).and(&cc).and(&rp));
    let v = top_n(v, |&(_, ((a, _), _))| (Reverse(a[3]), Reverse(a[2])), 100);
    rows(v.into_iter().map(|(u, ((a, c), r))| row(vec![user_col(db, u, "name"), V::I(a[3]), V::I(a[2]), V::I(c), V::I(r)])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// TotalVotes AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostStats AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, COALESCE(tv.UpVotes, 0) AS UpVotes, COALESCE(tv.DownVotes, 0) AS DownVotes
//     FROM RankedPosts rp LEFT JOIN TotalVotes tv ON rp.Id = tv.PostId WHERE rp.rn = 1),
// TopUsers AS (SELECT u.Id, u.DisplayName, SUM(ps.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId JOIN PostStats ps ON p.Id = ps.Id
//     GROUP BY u.Id, u.DisplayName ORDER BY TotalScore DESC LIMIT 5)
// SELECT pu.DisplayName AS TopUser, COUNT(ps.Id) AS PostCount, AVG(ps.Score) AS AverageScore, SUM(ps.ViewCount) AS TotalViews, SUM(ps.UpVotes) AS TotalUpVotes, SUM(ps.DownVotes) AS TotalDownVotes
// FROM TopUsers pu JOIN PostStats ps ON ps.Id IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = pu.Id) GROUP BY pu.DisplayName ORDER BY AverageScore DESC;
fn q2548(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(score.gt(0))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let ps: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(&ps)).select(score)).fold(0i64, |s, x| s + x);
    let tu = top_n(drain(&tu), |&(_, s)| Reverse(s), 5);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let tv = (&ps).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let g = (&tu)
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(score.and(view_count.opt()).and(&tv)))
        .fold([0i64; 6], |a, ((s, w), t)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + t[0], a[5] + t[1]]);
    rows(drain(&g).into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5])])))
}

// WITH TagStats AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, SUM(P.AnswerCount) AS TotalAnswers, SUM(P.CommentCount) AS TotalComments
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' WHERE P.PostTypeId = 1 GROUP BY T.TagName),
// UserEngagement AS (SELECT U.Id AS UserId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived,
//        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentsReceived FROM Users U LEFT JOIN Votes V ON V.UserId = U.Id LEFT JOIN Comments C ON C.UserId = U.Id GROUP BY U.Id),
// PostHistorySummary AS (SELECT PH.UserDisplayName, COUNT(PH.Id) AS EditCount, STRING_AGG(DISTINCT P.Title, ', ') AS EditedPostTitles, STRING_AGG(DISTINCT PH.Comment, ', ') AS EditComments
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId IN (4, 5, 6) GROUP BY PH.UserDisplayName)
// SELECT TS.TagName, TS.PostCount, TS.TotalViews, TS.TotalAnswers, TS.TotalComments, UE.UserId, UE.UpVotesReceived, UE.DownVotesReceived, UE.CommentsReceived, PHS.EditCount,
//        PHS.EditedPostTitles, PHS.EditComments
// FROM TagStats TS JOIN UserEngagement UE ON UE.UpVotesReceived + UE.DownVotesReceived > 10 JOIN PostHistorySummary PHS ON PHS.EditCount > 5
// ORDER BY TS.TotalViews DESC, TS.PostCount DESC, UE.UpVotesReceived DESC;
//
// Both ON clauses name one side only, so the three CTEs are crossed. The STRING_AGGs have no ORDER BY; the port sorts the distinct values.
fn q26076(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let Post { post_type_id, view_count, answer_count, comment_count, .. } = &db.post;
    type P = (Id<Post>, Id<Tag>);
    let qs = || (&lt).with(Same::<P>::new().map(|(p, _)| p).select(post_type_id.eq(1))).group_by(Same::<P>::new().map(|(_, t)| t).select(&db.tag.tag_name));
    let ts = qs()
        .select(Same::<P>::new().map(|(p, _)| p).select(view_count.opt().and(answer_count.opt()).and(comment_count)))
        .fold([0i64; 6], |a, ((w, n), c)| [a[0], a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + n.is_some() as i64, a[4] + n.unwrap_or(0), a[5] + c]);
    let pc = qs().select(Same::<P>::new().map(|(p, _)| p)).count_distinct();
    let ts = rel(drain((&pc).and(&ts).map(|(n, a)| [n, a[1], a[2], a[3], a[4], a[5]])));
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(comments_by(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let ue = rel(drain((&ue).filt(|a| a[0] + a[1] > 10)));
    let agg = |v: Vec<Str>| -> Option<Str> {
        let mut v = v;
        v.sort_unstable();
        v.dedup();
        if v.is_empty() { None } else { Some(Box::leak(v.join(", ").into_boxed_str())) }
    };
    let PostHistory { post_history_type_id, user_display_name, post, comment, .. } = &db.post_history;
    let phs = db
        .post_history
        .with(post_history_type_id.is_in([4, 5, 6]))
        .group_by(user_display_name.opt())
        .select(post.select((&db.post.title).opt()).and(comment.opt()))
        .buf_fold(|v| (v.len() as i64, agg(v.iter().flat_map(|x| x.0).collect()), agg(v.iter().flat_map(|x| x.1).collect())));
    let phs = rel(drain((&phs).filt(|(n, _, _)| n > 5)));
    let mut v = Vec::new();
    (&ts).cross((&ue).cross(&phs)).drive(|_, ((t, a), ((u, e), (_, (n, pt, pc))))| v.push((t, a, u, e, n, pt, pc)));
    v.sort_by_key(|x| (x.1[1] == 0, Reverse(x.1[2]), Reverse(x.1[0]), Reverse(x.3[0])));
    rows(v.into_iter().map(|(t, a, u, e, n, pt, pc)| {
        row(vec![V::S(t), V::I(a[0]), nullable(a[2], a[1]), nullable(a[4], a[3]), V::I(a[5]), user_col(db, u, "uid"), V::I(e[0]), V::I(e[1]), V::I(e[2]), V::I(n), ostr(pt), ostr(pc)])
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UserPostMetrics AS (SELECT U.Id AS UserId, U.DisplayName, UB.BadgeCount, PS.PostCount, PS.QuestionCount, PS.AnswerCount, PS.TotalViews, PS.TotalScore
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT UPM.UserId, UPM.DisplayName, COALESCE(UPM.BadgeCount, 0) AS BadgeCount, COALESCE(UPM.PostCount, 0) AS PostCount, COALESCE(UPM.QuestionCount, 0) AS QuestionCount,
//        COALESCE(UPM.AnswerCount, 0) AS AnswerCount, COALESCE(UPM.TotalViews, 0) AS TotalViews, COALESCE(UPM.TotalScore, 0) AS TotalScore,
//        CASE WHEN UPM.BadgeCount >= 10 THEN 'High Achiever' WHEN UPM.BadgeCount >= 5 THEN 'Active Contributor' ELSE 'New User' END AS UserTier
// FROM UserPostMetrics UPM ORDER BY UPM.TotalScore DESC, UPM.BadgeCount DESC;
//
// BadgeNames is never read, so it is not built.
fn q29243(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, creation_date, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(recent).select(post_type_id.and(view_count.opt()).and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, w), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s],
        None => a,
    });
    rows(drain((&bc).and(&ps)).into_iter().map(|(u, (b, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(b));
        f.extend(a.map(V::I));
        f.push(V::S(if b >= 10 { "High Achiever" } else if b >= 5 { "Active Contributor" } else { "New User" }));
        row(f)
    }))
}

// WITH UserBadgeCount AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        AVG(p.Score) AS AvgScore FROM Posts p GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS TotalBadges, COALESCE(ps.PostCount, 0) AS TotalPosts, COALESCE(ps.QuestionCount, 0) AS TotalQuestions,
//        COALESCE(ps.AnswerCount, 0) AS TotalAnswers, COALESCE(ps.AvgScore, 0) AS AvgPostScore FROM Users u LEFT JOIN UserBadgeCount ub ON u.Id = ub.UserId LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId),
// Ranking AS (SELECT UserId, DisplayName, TotalBadges, TotalPosts, TotalQuestions, TotalAnswers, AvgPostScore, RANK() OVER (ORDER BY TotalBadges DESC, TotalPosts DESC, AvgPostScore DESC) AS PerformanceRank
//     FROM UserPerformance)
// SELECT r.UserId, r.DisplayName, r.TotalBadges, r.TotalPosts, r.TotalQuestions, r.TotalAnswers, r.AvgPostScore, r.PerformanceRank FROM Ranking r WHERE r.PerformanceRank <= 10 ORDER BY r.PerformanceRank;
fn q2648(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mean = |a: [i64; 10]| if a[1] == 0 { 0.0 } else { a[4] as f64 / a[1] as f64 };
    let w = whole(&ups).select(Ident::<User>::new().and((&ups).and(&bc))).window(rank, |(_, (a, b))| (Reverse(b), Reverse(a[1]), Reverse(fkey(mean(a)))), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::F(mean(a)), V::I(r)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalUpVotes, TotalDownVotes, CommentCount, RANK() OVER (ORDER BY PostCount DESC) AS PostCountRank,
//        RANK() OVER (ORDER BY TotalUpVotes DESC) AS UpVotesRank, RANK() OVER (ORDER BY TotalDownVotes DESC) AS DownVotesRank FROM UserActivity),
// UserHighlights AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalUpVotes, TotalDownVotes, CommentCount,
//        CASE WHEN PostCountRank <= 10 THEN 'Top Posters' WHEN UpVotesRank <= 10 THEN 'Top Upvoted' WHEN DownVotesRank <= 10 THEN 'Top Downvoted' ELSE 'Regular Users' END AS UserGroup FROM TopUsers)
// SELECT UserGroup, COUNT(UserId) AS UserCount, AVG(PostCount) AS AvgPosts, AVG(TotalUpVotes) AS AvgUpVotes, AVG(TotalDownVotes) AS AvgDownVotes, AVG(CommentCount) AS AvgComments
// FROM UserHighlights GROUP BY UserGroup ORDER BY UserCount DESC;
//
// The two COUNT(DISTINCT) come from one row per post; the vote sums are folded over the post x vote x comment product.
fn q8849(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt())
        .fold([0i64; 2], |a, x| match x {
            Some((t, _)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
            None => a,
        });
    let dc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).opt()).opt()).fold([0i64; 2], |a, x| match x {
        Some(c) => [a[0] + 1, a[1] + c.is_some() as i64],
        None => a,
    });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type T = (Id<User>, (([i64; 2], [i64; 2]), i64));
    let w = whole(&ua).select(Ident::<User>::new().and((&ua).and(&dc).and(&dp))).window(rank, |(_, (_, n)): T| Reverse(n), asc);
    let w = (&w).window(rank, |((_, ((a, _), _)), _): (T, i64)| Reverse(a[0]), asc);
    let w = (&w).window(rank, |(((_, ((a, _), _)), _), _): ((T, i64), i64)| Reverse(a[1]), asc);
    type R = ((((Id<User>, (([i64; 2], [i64; 2]), i64)), i64), i64), i64);
    let g = (&w)
        .group_by(Same::<R>::new().map(|(((_, p), u), d): R| if p <= 10 { "Top Posters" } else if u <= 10 { "Top Upvoted" } else if d <= 10 { "Top Downvoted" } else { "Regular Users" }))
        .select(Same::<R>::new().map(|(((x, _), _), _): R| x))
        .fold([0i64; 5], |s, (_, ((a, c), n))| [s[0] + 1, s[1] + n, s[2] + a[0], s[3] + a[1], s[4] + c[1]]);
    rows(drain(&g).into_iter().map(|(k, s)| row(vec![V::S(k), V::I(s[0]), avg(s[1], s[0]), avg(s[2], s[0]), avg(s[3], s[0]), avg(s[4], s[0])])))
}

// WITH TagPopularity AS (SELECT TagName, COUNT(*) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY TagName),
// UserEngagement AS (SELECT U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpvotesGiven, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownvotesGiven,
//        SUM(COALESCE(P.Score, 0)) AS TotalScore FROM Users U LEFT JOIN Votes V ON V.UserId = U.Id LEFT JOIN Posts P ON P.OwnerUserId = U.Id GROUP BY U.DisplayName),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, PH.CreationDate AS LastEditDate, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY PH.CreationDate DESC) AS EditRank
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId)
// SELECT TP.TagName, TP.TotalPosts, TP.QuestionCount, TP.AnswerCount, U.DisplayName, U.UpvotesGiven, U.DownvotesGiven, U.TotalScore, PA.PostId, PA.Title, PA.CreationDate, PA.ViewCount, PA.LastEditDate
// FROM TagPopularity TP JOIN UserEngagement U ON U.UpvotesGiven > 5 /* Interested in active users */ JOIN PostActivity PA ON PA.EditRank = 1 /* Most recent edit for each post */
// WHERE TP.TotalPosts > 10 /* Only tags with more than 10 associated posts */ ORDER BY TP.TotalPosts DESC, U.TotalScore DESC, PA.ViewCount DESC;
//
// Both ON clauses name one side only, so the three CTEs are crossed. EditRank = 1 keeps each post's latest history row, whose date is the maximum.
fn q29565(db: &'static So) -> String {
    type P = (Id<Post>, Id<Tag>);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t): P| t).inv().select(Same::<P>::new().map(|(p, _): P| p)).collect();
    let tpop = db.tag.group_by(&db.tag.tag_name).select((&by_tag).select(&db.post.post_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let tp = rel(drain((&tpop).filt(|a| a[0] > 10)));
    let ue = db
        .user
        .group_by(&db.user.display_name)
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(&db.post.score).opt()))
        .fold([0i64; 3], |a, (t, s)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + s.unwrap_or(0)]);
    let ue = rel(drain((&ue).filt(|a| a[0] > 5)));
    let pa = db.post.group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.creation_date).opt()).fold(i64::MIN, |m, d| d.map_or(m, |d| m.max(d)));
    let pa = rel(drain(&pa));
    let mut v = Vec::new();
    (&tp).cross((&ue).cross(&pa)).drive(|_, ((t, a), ((n, e), (p, d)))| v.push((t, a, n, e, p, d)));
    let vc = &db.post.view_count;
    v.sort_by_key(|x| (Reverse(x.1[0]), Reverse(x.3[2]), vc.get(x.4).is_none(), Reverse(vc.get(x.4))));
    rows(v.into_iter().map(|(t, a, n, e, p, d)| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(n), V::I(e[0]), V::I(e[1]), V::I(e[2])];
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.push(tmax(d));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(p.Score) AS TotalScore,
//        SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ubc.GoldBadges, 0) AS GoldBadges, COALESCE(ubc.SilverBadges, 0) AS SilverBadges, COALESCE(ubc.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(ps.QuestionCount, 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount, COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(ps.TotalViews, 0) AS TotalViews,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(ps.TotalScore, 0) DESC) AS Rank FROM Users u LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId),
// TopPerformers AS (SELECT * FROM UserPerformance WHERE Rank <= 10)
// SELECT tp.UserId, tp.DisplayName, tp.GoldBadges, tp.SilverBadges, tp.BronzeBadges, tp.QuestionCount, tp.AnswerCount, tp.TotalScore, tp.TotalViews,
//        (SELECT COUNT(*) FROM Votes v WHERE v.UserId = tp.UserId AND v.VoteTypeId IN (2, 3)) AS TotalVotes FROM TopPerformers tp ORDER BY tp.Rank;
fn q4122(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let tv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + matches!(t, Some(2 | 3)) as i64);
    let v = top_n(drain((&ups).and(&ub).and(&tv)), |&(_, ((a, _), _))| Reverse(a[4]), 10);
    rows(v.into_iter().map(|(u, ((a, b), t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6]), V::I(t)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(PS.PostCount, 0) AS TotalPosts, COALESCE(PS.Questions, 0) AS TotalQuestions,
//        COALESCE(PS.Answers, 0) AS TotalAnswers, COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(PS.AverageScore, 0) AS AverageScore
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT UserId, DisplayName, TotalBadges, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, AverageScore, RANK() OVER (ORDER BY TotalBadges DESC, TotalPosts DESC) AS BadgePostRank
// FROM UserEngagement WHERE TotalPosts > 0 ORDER BY BadgePostRank ASC, TotalBadges DESC, TotalPosts DESC FETCH FIRST 10 ROWS ONLY;
fn q27788(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = whole((&ups).filt(|a| a[1] > 0)).select(Ident::<User>::new().and((&ups).and(&bc))).window(rank, |(_, (a, b))| (Reverse(b), Reverse(a[1])), asc);
    let v = top_n(drain(&w), |&(_, (_, r))| r, 10);
    rows(v.into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), avg(a[4], a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN c.Score > 0 THEN 1 ELSE 0 END) AS PositiveComments FROM Comments c GROUP BY c.PostId),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName, pc.CommentCount AS TotalComments, pc.PositiveComments
//     FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId)
// SELECT pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.TotalComments, pd.PositiveComments,
//        CASE WHEN pd.Score >= 10 THEN 'High' WHEN pd.Score BETWEEN 4 AND 9 THEN 'Medium' ELSE 'Low' END AS ScoreCategory FROM PostDetails pd ORDER BY pd.Score DESC, pd.Title;
fn q6513(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let pc = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + (s > 0) as i64]);
    rows(drain((&tp).select(Ident::<Post>::new().and((&pc).opt()))).into_iter().map(|(_, (p, c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "comments"]);
        f.extend(match c {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null],
        });
        f.push(V::S(if s >= 10 { "High" } else if s >= 4 { "Medium" } else { "Low" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.OwnerDisplayName, rp.Title, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.rn = 1),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// BadgedUsers AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// JoinedData AS (SELECT tp.OwnerDisplayName, tp.Title, tp.Score, tp.ViewCount, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pc.LastCommentDate, DATE '1970-01-01') AS LastCommentDate, bu.BadgeCount
//     FROM TopPosts tp LEFT JOIN PostComments pc ON tp.Title = (SELECT Title FROM Posts WHERE Id = pc.PostId)
//     LEFT JOIN BadgedUsers bu ON tp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = bu.UserId))
// SELECT OwnerDisplayName, Title, Score, ViewCount, CommentCount, LastCommentDate, BadgeCount FROM JoinedData WHERE Score > 10 AND ViewCount > 100 ORDER BY Score DESC, CommentCount DESC;
//
// PostComments joins on the title of its post and BadgedUsers on the display name of its user, so both are indexed by those strings.
fn q3156(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, title, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pc = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let by_title: HashIdx<Str, Id<Post>> = db.post.with(&pc).select(title).inv().collect();
    let bu = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = db.user.with(&bu).select(&db.user.display_name).inv().collect();
    let v = drain((&tp).with(score.gt(10)).with(view_count.gt(100)).select(Ident::<Post>::new().and(title.select(&by_title).select(&pc).opt()).and(owner_user.select(&db.user.display_name).select(&by_name).select(&bu).opt())));
    rows(v.into_iter().map(|(_, ((p, c), b))| {
        let (n, m) = c.unwrap_or((0, 0));
        let mut f = post_fields(db, p, &["owner", "title", "score", "views"]);
        f.extend([V::I(n), V::T(m), oint(b)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(COALESCE(P.Score, 0)) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// BadgesCount AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// FinalStats AS (SELECT UR.UserId, UR.DisplayName, COALESCE(PS.QuestionCount, 0) AS QuestionCount, COALESCE(PS.AnswerCount, 0) AS AnswerCount, COALESCE(PS.TotalScore, 0) AS TotalScore,
//        COALESCE(BC.BadgeCount, 0) AS BadgeCount, COALESCE(BC.GoldBadges, 0) AS GoldBadges, COALESCE(BC.SilverBadges, 0) AS SilverBadges, COALESCE(BC.BronzeBadges, 0) AS BronzeBadges,
//        UR.UserRank, UR.Reputation FROM UserReputation UR LEFT JOIN PostStats PS ON UR.UserId = PS.OwnerUserId LEFT JOIN BadgesCount BC ON UR.UserId = BC.UserId)
// SELECT F.DisplayName, F.QuestionCount, F.AnswerCount, F.TotalScore, F.BadgeCount, F.GoldBadges, F.SilverBadges, F.BronzeBadges, F.UserRank
// FROM FinalStats F WHERE F.Reputation > 1000 ORDER BY F.TotalScore DESC, F.BadgeCount DESC FETCH FIRST 10 ROWS ONLY;
fn q4721(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let rep = &db.user.reputation;
    let ur = whole(db.user.iq()).select(Ident::<User>::new().and(rep)).window(rank, |(_, r)| Reverse(r), asc);
    type R = ((Id<User>, i64), i64);
    let v = drain((&ur).filt(|((_, r), _): R| r > 1000).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| u).select((&ups).and(&ub)))));
    let v = top_n(v, |&(_, (_, (a, b)))| (Reverse(a[4]), Reverse(b[0])), 10);
    rows(v.into_iter().map(|(_, (((u, _), r), (a, b)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[2]), V::I(a[3]), V::I(a[4])];
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, CommentCount, VoteCount, BadgeCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserStats),
// TopQuestions AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score ORDER BY p.Score DESC LIMIT 10)
// SELECT u.DisplayName AS UserName, u.Reputation, q.Title AS TopQuestionTitle, q.Score AS QuestionScore, q.ViewCount AS QuestionViews, q.CommentCount AS QuestionComments, q.VoteCount AS QuestionVotes
// FROM TopUsers u JOIN TopQuestions q ON u.UserId = (SELECT OwnerUserId FROM Posts WHERE Title = q.Title LIMIT 1) WHERE u.UserRank <= 10 ORDER BY u.Reputation DESC;
//
// None of UserStats' aggregates is read, and it has one row per user, so only UserRank is computed. The unordered
// LIMIT 1 over the posts with q.Title takes the smallest post id.
fn q7494(db: &'static So) -> String {
    let Post { post_type_id, score, title, owner_user, .. } = &db.post;
    let tu = top_n(drain(&db.user.reputation), |&(_, r)| Reverse(r), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let tq = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(_, s)| Reverse(s), 10);
    let tq: MatSet<Id<Post>> = rel(tq.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tq).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tq).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let one = db.post.group_by(title).select(Ident::<Post>::new()).fold(None, |m: Option<Id<Post>>, p| Some(m.map_or(p, |m| m.min(p))));
    let v = drain((&cc).and(&vc).and(title.select(&one).flat_map(|p: Option<Id<Post>>| p).select(owner_user).select(Ident::<User>::new().with(&tu))));
    rows(v.into_iter().map(|(p, ((c, n), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH RECURSIVE UserRankings AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u),
// PostStatistics AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.OwnerUserId, p.PostTypeId),
// FilteredPosts AS (SELECT ps.PostId, ps.OwnerUserId, ps.CommentCount, ps.UpVotes, ps.DownVotes, ur.DisplayName AS OwnerDisplayName, ur.Reputation AS OwnerReputation, ps.CloseCount, ps.ReopenCount
//     FROM PostStatistics ps JOIN UserRankings ur ON ps.OwnerUserId = ur.UserId WHERE ps.CommentCount > 5 AND (ps.UpVotes - ps.DownVotes) > 10),
// TopPosts AS (SELECT fp.*, ROW_NUMBER() OVER (ORDER BY (fp.UpVotes - fp.DownVotes) DESC) AS PostRank FROM FilteredPosts fp)
// SELECT tp.PostId, tp.OwnerDisplayName, tp.OwnerReputation, tp.CommentCount, tp.UpVotes, tp.DownVotes, tp.CloseCount, tp.ReopenCount FROM TopPosts tp WHERE tp.PostRank <= 100
// ORDER BY (tp.UpVotes - tp.DownVotes) DESC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q31127(db: &'static So) -> String {
    let ps = db
        .post
        .with(&db.post.owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 5], |a, ((c, t), h)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (h == Some(10)) as i64, a[4] + (h == Some(11)) as i64]);
    let v = top_n(drain((&ps).filt(|a| a[0] > 5 && a[1] - a[2] > 10)), |&(_, a)| Reverse(a[1] - a[2]), 100);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "owner", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Posts p LEFT JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, U.DisplayName, p.PostTypeId),
// HighScorePosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, TotalBounties FROM RankedPosts WHERE Rank <= 5),
// PostHistoryDetails AS (SELECT ph.PostId, ph.CreationDate AS HistoryDate, ph.UserDisplayName AS Editor, ph.Comment, p.Title FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.PostHistoryTypeId IN (4, 5, 10))
// SELECT h.PostId, h.Title, h.OwnerDisplayName, h.ViewCount, h.Score, h.CommentCount, h.TotalBounties, p.HistoryDate, p.Editor, p.Comment AS EditComment, COALESCE(h.Score * 0.1, 0) AS WeightedScore
// FROM HighScorePosts h LEFT JOIN PostHistoryDetails p ON h.PostId = p.PostId ORDER BY h.Score DESC, h.ViewCount DESC;
//
// Rank reads only base columns, so the ranked posts are picked first and the comment x vote product is driven for those alone.
fn q34363(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let rp = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let PostHistory { post_history_type_id, creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    let phd = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 10])));
    rows(drain((&rp).and(phd.opt())).into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views", "score"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), ostr(user_display_name.get(h)), ostr(comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::F(score.get(p).unwrap() as f64 / 10.0));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostHistorySummary AS (SELECT ph.UserId, COUNT(ph.Id) AS TotalChanges, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph GROUP BY ph.UserId)
// SELECT ur.UserId, ur.DisplayName, ur.Reputation, us.QuestionCount, us.AnswerCount, us.TotalScore, us.TotalViews, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        COALESCE(phs.TotalChanges, 0) AS TotalPostHistoryChanges, phs.LastEditDate, RANK() OVER (ORDER BY ur.Reputation DESC) AS RankByReputation
// FROM UserReputation ur LEFT JOIN PostStats us ON ur.UserId = us.OwnerUserId LEFT JOIN UserBadges ub ON ur.UserId = ub.UserId LEFT JOIN PostHistorySummary phs ON ur.UserId = phs.UserId
// WHERE ur.Reputation > 1000 AND (ub.GoldBadges + ub.SilverBadges + ub.BronzeBadges) >= 5 ORDER BY ur.Reputation DESC, TotalPostHistoryChanges DESC;
fn q3432(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let PostHistory { user, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(user).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let fr = || db.user.with((&db.user.reputation).gt(1000)).select((&ups).and((&ub).filt(|b| b[0] + b[1] + b[2] >= 5)).and((&phs).opt()));
    let w = whole(fr()).select(Ident::<User>::new().and(&db.user.reputation).and(fr())).window(rank, |((_, r), _)| Reverse(r), asc);
    rows(drain(&w).into_iter().map(|(_, (((u, _), ((a, b), h)), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(if a[1] == 0 { [V::Null, V::Null, V::Null, V::Null] } else { [V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6])] });
        f.extend(b.map(V::I));
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::I(0), V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS Owner,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.Owner FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Class = 1 OR b.Class = 2 GROUP BY b.UserId),
// FinalMetrics AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, COALESCE(pc.CommentCount, 0) AS CommentCount, pb.BadgeCount
//     FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId LEFT JOIN PostBadges pb ON tp.Owner = (SELECT DisplayName FROM Users WHERE Id = pb.UserId))
// SELECT fm.PostId, fm.Title, fm.CreationDate, fm.Score, fm.ViewCount, fm.AnswerCount, fm.CommentCount, fm.BadgeCount FROM FinalMetrics fm ORDER BY fm.Score DESC, fm.ViewCount DESC;
//
// PostBadges joins on the display name of its user, so it is indexed by that string.
fn q7062(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pb = db.badge.with((&db.badge.class).is_in([1, 2])).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = db.user.with(&pb).select(&db.user.display_name).inv().collect();
    rows(drain((&cc).and(owner_user.select(&db.user.display_name).select(&by_name).select(&pb).opt())).into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(c), oint(b)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(b.Class) AS TotalBadgeClass, ROW_NUMBER() OVER (ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS RN
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// PostAnalytics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.CommentCount, p.AnswerCount, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenedDate, p.OwnerUserId
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.CommentCount, p.AnswerCount, p.OwnerUserId)
// SELECT us.DisplayName, us.PostCount, us.QuestionCount, us.AnswerCount, us.UpVotes, us.DownVotes, pa.Title, pa.CreationDate, pa.ViewCount, pa.CommentCount, pa.ClosedDate, pa.ReopenedDate
// FROM UserStats us JOIN PostAnalytics pa ON us.UserId = pa.OwnerUserId WHERE us.RN <= 50 ORDER BY us.UpVotes - us.DownVotes DESC;
fn q7481(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, _)| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let dp = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let tu = top_n(drain((&us).and((&dp).opt()).map(|(a, n)| (a, n.unwrap_or(0)))), |&(_, (a, _))| Reverse(a[2] - a[3]), 50);
    let tu = rel(tu);
    type R = (Id<User>, ([i64; 4], i64));
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pa = db.post.group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd)).opt()).fold([i64::MIN; 2], |m, h| match h {
        Some((10, d)) => [m[0].max(d), m[1]],
        Some((11, d)) => [m[0], m[1].max(d)],
        _ => m,
    });
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(posts_of(db)).select(Ident::<Post>::new().and(&pa)))));
    rows(v.into_iter().map(|(_, ((u, (a, n)), (p, m)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(post_fields(db, p, &["title", "created", "views", "comments"]));
        f.extend([tmax(m[0]), tmax(m[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.Score, p.ViewCount, ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><'), 1) AS TagCount,
//        COALESCE(u.DisplayName, 'Deleted User') AS OwnerDisplayName, COALESCE(u.Reputation, 0) AS OwnerReputation, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Tags, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation),
// TopPosts AS (SELECT *, RANK() OVER (ORDER BY Score DESC, ViewCount DESC, TagCount DESC) AS PostRank FROM RankedPosts),
// FinalStats AS (SELECT PostId, Title, OwnerDisplayName, OwnerReputation, Score, ViewCount, TagCount, CommentCount, UpVoteCount, DownVoteCount, PostRank FROM TopPosts WHERE PostRank <= 10)
// SELECT PostId, Title, OwnerDisplayName, OwnerReputation, Score, ViewCount, TagCount, CommentCount, UpVoteCount, DownVoteCount,
//        (CAST(UpVoteCount AS FLOAT) / NULLIF(CommentCount, 0)) AS UpVoteToCommentRatio, (CAST(DownVoteCount AS FLOAT) / NULLIF(CommentCount, 0)) AS DownVoteToCommentRatio
// FROM FinalStats ORDER BY PostRank;
//
// PostRank reads only base columns, so the top questions are picked first and the comment x vote product is driven for those alone.
fn q26152(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, tags_str, owner_user, .. } = &db.post;
    let tc = |p: Id<Post>| tags_str.get(p).map(|t| tag_list(t).count() as i64);
    let w = whole(db.post.with(post_type_id.eq(1)))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()).and(tags_str.opt()))
        .window(rank, |(((_, s), w), t)| {
            let t = t.map(|t| tag_list(t).count() as i64);
            (Reverse(s), w.is_none(), Reverse(w), t.is_none(), Reverse(t))
        }, asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((((p, _), _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ratio = |x: i64, n: i64| if n == 0 { V::Null } else { V::F((x as f32 / n as f32) as f64) };
    rows(drain(&s).into_iter().map(|(p, a)| {
        let u = owner_user.get(p);
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(u.map_or(V::S("Deleted User"), |u| user_col(db, u, "name")));
        f.push(u.map_or(V::I(0), |u| user_col(db, u, "rep")));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.push(oint(tc(p)));
        f.extend(a.map(V::I));
        f.extend([ratio(a[1], a[0]), ratio(a[2], a[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, q.OwnerDisplayName AS QuestionOwner, p.CreationDate,
//        RANK() OVER (PARTITION BY p.Id ORDER BY COUNT(DISTINCT v.Id) DESC) AS RankByVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Posts q ON p.ParentId = q.Id WHERE p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.Body, q.OwnerDisplayName, p.CreationDate),
// TopPosts AS (SELECT PostId, Title, Body, CommentCount, VoteCount, QuestionOwner, CreationDate FROM RankedPosts WHERE RankByVotes = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(v.BountyAmount) AS TotalBounty FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName)
// SELECT tp.PostId, tp.Title AS PostTitle, tp.Body AS PostBody, tp.CommentCount, tp.VoteCount, tp.QuestionOwner, tp.CreationDate, us.UserId, us.DisplayName AS OwnerDisplayName,
//        us.QuestionCount, us.AnswerCount, us.TotalBounty
// FROM TopPosts tp JOIN UserStats us ON tp.QuestionOwner = us.DisplayName ORDER BY tp.VoteCount DESC, tp.CommentCount DESC;
//
// RankByVotes partitions by the post itself, so it is always 1. The join is on the parent's OwnerDisplayName, through an index of users by name.
fn q27253(db: &'static So) -> String {
    let Post { post_type_id, parent, owner_display_name, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
            }
            None => a,
        });
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let tp = || db.post.with(post_type_id.is_in([1, 2])).with(parent.select(owner_display_name).select(&by_name));
    let cc = tp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = tp().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = drain((&cc).and(&vc).and(parent.select(owner_display_name.and(owner_display_name.select(&by_name).select(Ident::<User>::new().and(&us))))));
    rows(v.into_iter().map(|(p, ((c, n), (q, (u, a))))| {
        let mut f = post_fields(db, p, &["id", "title", "body"]);
        f.extend([V::I(c), V::I(n), V::S(q)]);
        f.push(post_fields(db, p, &["created"]).remove(0));
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(a[0]), V::I(a[1]), nullable(a[3], a[2])]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, U.Views, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U JOIN UserBadges UB ON U.Id = UB.UserId WHERE U.Reputation > 1000),
// PopularPosts AS (SELECT P.Id, P.Title, P.Score, P.ViewCount, P.OwnerUserId, COUNT(V.Id) AS VoteCount FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '30 DAY' GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.OwnerUserId HAVING COUNT(V.Id) > 10),
// UserTopPosts AS (SELECT U.DisplayName, PP.Title, PP.Score, PP.VoteCount, RANK() OVER (PARTITION BY U.Id ORDER BY PP.VoteCount DESC) AS PostRank FROM TopUsers U JOIN PopularPosts PP ON U.Id = PP.OwnerUserId)
// SELECT U.DisplayName, U.Reputation, U.BadgeCount, U.GoldBadges, U.SilverBadges, U.BronzeBadges, P.Title AS TopPostTitle, P.Score AS PostScore, P.VoteCount AS PostVoteCount
// FROM TopUsers U JOIN UserTopPosts P ON U.DisplayName = P.DisplayName WHERE P.PostRank = 1 ORDER BY U.Reputation DESC;
//
// UserTopPosts joins back on the display name, so its posts are indexed by their owner's name.
fn q5106(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let tu = || Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let Post { creation_date, owner_user, .. } = &db.post;
    let pp = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    type R = (Id<Post>, (i64, Id<User>));
    let utp = rel(drain((&pp).filt(|n| n > 10).and(owner_user.select(tu()))));
    let by_user: HashIdx<Id<User>, R> = (&utp).map(|(_, (_, u)): R| u).inv().select(&utp).collect();
    let w = (&by_user).window(rank, |(_, (n, _))| Reverse(n), asc);
    let top: MatSet<R> = (&w).filt(|(_, r)| r == 1).map(|(x, _)| x).collect();
    let by_name: HashIdx<Str, R> = (&top).map(|(_, (_, u)): R| u).select(&db.user.display_name).inv().select(&top).collect();
    let v = drain(db.user.select(tu().and(&ub).and((&db.user.display_name).select(&by_name))));
    rows(v.into_iter().map(|(_, ((u, b), (p, (n, _))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["title", "score"]));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostsCount, COUNT(DISTINCT C.Id) AS CommentsCount, SUM(V.BountyAmount) AS TotalBountyAmount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.CreationDate >= '2020-01-01'
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostsCount, CommentsCount, TotalBountyAmount, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY PostsCount DESC) AS PostsRank, RANK() OVER (ORDER BY CommentsCount DESC) AS CommentsRank FROM UserActivity)
// SELECT UserId, DisplayName, Reputation, PostsCount, CommentsCount, TotalBountyAmount, TotalUpVotes, TotalDownVotes, CASE WHEN ReputationRank <= 10 THEN 'Top Reputation' ELSE 'Normal' END AS ReputationCategory,
//        CASE WHEN PostsRank <= 5 THEN 'Top Posts' ELSE 'Normal' END AS PostsCategory, CASE WHEN CommentsRank <= 5 THEN 'Top Comments' ELSE 'Normal' END AS CommentsCategory
// FROM TopUsers WHERE TotalUpVotes > TotalDownVotes ORDER BY Reputation DESC, PostsCount DESC, CommentsCount DESC;
//
// The two COUNT(DISTINCT) are the plain per-user counts; the vote sums are folded over the post x comment x vote product.
fn q8601(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let users = || db.user.with((&db.user.creation_date).ge(ts(2020, 1, 1, 0, 0, 0)));
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt()).opt().and(comments_by(db).opt()))
        .fold([0i64; 4], |a, (p, _)| match p.flatten() {
            Some((t, b)) => [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
            None => a,
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = users().group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    type T = (Id<User>, (([i64; 4], i64), i64));
    let w = whole(&ua).select(Ident::<User>::new().and((&ua).and(&pc).and(&cc))).window(rank, |(u, _): T| Reverse(rep(u)), asc);
    let w = (&w).window(rank, |((_, ((_, p), _)), _): (T, i64)| Reverse(p), asc);
    let w = (&w).window(rank, |(((_, (_, c)), _), _): ((T, i64), i64)| Reverse(c), asc);
    type R = ((((Id<User>, (([i64; 4], i64), i64)), i64), i64), i64);
    let mut v = drain((&w).filt(|((((_, ((a, _), _)), _), _), _): R| a[2] > a[3]));
    v.sort_by_key(|&(_, ((((u, ((_, p), c)), _), _), _))| (Reverse(rep(u)), Reverse(p), Reverse(c)));
    rows(v.into_iter().map(|(_, ((((u, ((a, p), c)), r1), r2), r3))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(p), V::I(c), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
        f.push(V::S(if r1 <= 10 { "Top Reputation" } else { "Normal" }));
        f.push(V::S(if r2 <= 5 { "Top Posts" } else { "Normal" }));
        f.push(V::S(if r3 <= 5 { "Top Comments" } else { "Normal" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        RANK() OVER (ORDER BY SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalUpvotes, TotalDownvotes, Rank FROM UserStats WHERE Rank <= 10),
// ActiveUsers AS (SELECT U.Id AS UserId, COUNT(DISTINCT C.Id) AS TotalComments, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalAnswers, TU.TotalQuestions, TU.TotalUpvotes, TU.TotalDownvotes, AU.TotalComments, AU.GoldBadges, AU.SilverBadges, AU.BronzeBadges
// FROM TopUsers TU JOIN ActiveUsers AU ON TU.UserId = AU.UserId ORDER BY TU.Rank;
//
// ActiveUsers is joined per user, so it is computed for the ten top users alone.
fn q6939(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let w = win(&us, rank, |(_, a)| Reverse(a[2]), asc);
    type R = ((Id<User>, [i64; 4]), i64);
    let tu: MatSet<R> = (&w).filt(|(_, r)| r <= 10).map(|x| x).collect();
    let tus: MatSet<Id<User>> = (&tu).map(|((u, _), _): R| u).collect();
    let au = (&tus).group_by(Ident::<User>::new()).select(comments_by(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, (_, c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let ac = (&tus).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let dp = (&tus).group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    rows(drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| u).select((&dp).opt().and(&ac).and(&au))))).into_iter().map(|(_, (((u, a), _), ((n, c), b)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n.unwrap_or(0))];
        f.extend(a.map(V::I));
        f.push(V::I(c));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(P.Score) AS TotalScore,
//        AVG(P.ViewCount) AS AvgViewCount FROM Posts P GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT U.Id, U.DisplayName, COALESCE(UB.TotalBadges, 0) AS BadgeCount, COALESCE(PS.QuestionCount, 0) AS QuestionsCreated, COALESCE(PS.AnswerCount, 0) AS AnswersGiven,
//        COALESCE(PS.TotalScore, 0) AS TotalScore, COALESCE(PS.AvgViewCount, 0) AS AverageViewCount, ROW_NUMBER() OVER (ORDER BY COALESCE(PS.TotalScore, 0) DESC) AS RankScore
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT U.DisplayName, U.BadgeCount, U.QuestionsCreated, U.AnswersGiven, U.TotalScore, U.AverageViewCount, CASE WHEN U.RankScore <= 10 THEN 'Top Performer' ELSE 'Contributor' END AS PerformanceCategory
// FROM UserPerformance U WHERE U.QuestionsCreated > 5 OR U.AnswersGiven > 10 ORDER BY U.TotalScore DESC NULLS LAST;
fn q2378(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = whole(&ups).select(Ident::<User>::new().and((&ups).and(&bc))).window(row_number, |(u, (a, _))| (Reverse(a[4]), u), asc);
    rows(drain((&w).filt(|((_, (a, _)), _)| a[2] > 5 || a[3] > 10)).into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(b), V::I(a[2]), V::I(a[3]), V::I(a[4]), if a[5] == 0 { V::F(0.0) } else { avg(a[6], a[5]) }];
        f.push(V::S(if r <= 10 { "Top Performer" } else { "Contributor" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldCount, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeCount FROM Badges GROUP BY UserId),
// UserPosts AS (SELECT OwnerUserId, COUNT(*) AS PostCount, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        AVG(ViewCount) AS AvgViewCount FROM Posts GROUP BY OwnerUserId),
// ActiveUsers AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(UP.PostCount, 0) AS TotalPosts, COALESCE(UP.QuestionsCount, 0) AS TotalQuestions,
//        COALESCE(UP.AnswersCount, 0) AS TotalAnswers, COALESCE(UP.AvgViewCount, 0) AS AvgViewCount, DENSE_RANK() OVER (ORDER BY COALESCE(UP.PostCount, 0) DESC) AS PostRank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN UserPosts UP ON U.Id = UP.OwnerUserId WHERE U.Reputation > 0),
// TopActiveUsers AS (SELECT UserId, DisplayName, TotalBadges, TotalPosts, TotalQuestions, TotalAnswers, AvgViewCount FROM ActiveUsers WHERE PostRank <= 10)
// SELECT A.DisplayName, A.TotalBadges, A.TotalPosts, A.TotalQuestions, A.TotalAnswers, A.AvgViewCount,
//        CASE WHEN A.TotalBadges = 0 THEN 'No Badges' WHEN A.TotalBadges > 5 THEN 'Very Active' ELSE 'Moderately Active' END AS ActivityLevel
// FROM TopActiveUsers A ORDER BY A.TotalPosts DESC, A.TotalBadges DESC;
fn q3766(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = whole(db.user.with((&db.user.reputation).gt(0))).select(Ident::<User>::new().and((&ups).and(&bc))).window(dense_rank, |(_, (a, _))| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, (a, b)), _))| {
        let mut f = vec![user_col(db, u, "name"), V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[5] == 0 { V::F(0.0) } else { avg(a[6], a[5]) }];
        f.push(V::S(if b == 0 { "No Badges" } else if b > 5 { "Very Active" } else { "Moderately Active" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, u.DisplayName AS OwnerDisplayName
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, pt.Name),
// PostHistoryDetails AS (SELECT ph.PostId, MAX(CASE WHEN pht.Name = 'Post Closed' THEN ph.CreationDate END) AS LastClosedDate, MAX(CASE WHEN pht.Name = 'Post Reopened' THEN ph.CreationDate END) AS LastReopenedDate,
//        MAX(CASE WHEN pht.Name = 'Initial Body' THEN ph.CreationDate END) AS InitialBodyEditDate FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount, hp.LastClosedDate, hp.LastReopenedDate, hp.InitialBodyEditDate, rp.OwnerDisplayName,
//        DENSE_RANK() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS OverallRank
// FROM RankedPosts rp LEFT JOIN PostHistoryDetails hp ON rp.PostId = hp.PostId WHERE rp.Rank <= 5 ORDER BY rp.Rank, rp.Score DESC;
//
// Rank reads only base columns, so the ranked posts are picked first and the comment x vote product is driven for those alone.
fn q7491(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let rp = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let hp = db.post_history.group_by(&db.post_history.post).select(htype_name(db).and(&db.post_history.creation_date)).fold([i64::MIN; 3], |m, (n, d)| match n {
        "Post Closed" => [m[0].max(d), m[1], m[2]],
        "Post Reopened" => [m[0], m[1].max(d), m[2]],
        "Initial Body" => [m[0], m[1], m[2].max(d)],
        _ => m,
    });
    let ow = whole(&rp).select(Ident::<Post>::new().and(score).and(view_count.opt()).and((&rp).and((&hp).opt()))).window(dense_rank, |(((_, s), w), _)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    rows(drain(&ow).into_iter().map(|(_, ((((p, _), _), (a, h)), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend(h.unwrap_or([i64::MIN; 3]).map(tmax));
        f.push(post_fields(db, p, &["owner"]).remove(0));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.AcceptedAnswerId, p.OwnerUserId, u.DisplayName AS OwnerUserDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.AcceptedAnswerId, p.OwnerUserId, u.DisplayName),
// RecentPosts AS (SELECT PostId, Title, Body, CreationDate, OwnerUserDisplayName, CommentCount, UpVoteCount, DownVoteCount, (UpVoteCount - DownVoteCount) AS Score, UserPostRank
//     FROM RankedPosts WHERE CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')),
// PostStatistics AS (SELECT RP.OwnerUserDisplayName, AVG(Score) AS AverageScore, COUNT(RP.PostId) AS TotalPosts, SUM(RP.CommentCount) AS TotalComments FROM RecentPosts RP GROUP BY RP.OwnerUserDisplayName)
// SELECT PS.OwnerUserDisplayName, PS.TotalPosts, PS.TotalComments, PS.AverageScore, RANK() OVER (ORDER BY PS.AverageScore DESC) AS Rank FROM PostStatistics PS WHERE PS.TotalPosts > 0 ORDER BY Rank;
//
// UserPostRank is never read, so RecentPosts' date filter is applied before the comment x vote product.
fn q25928(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]);
    let ps = db.post.with(&rp).group_by(owner_user.select(&db.user.display_name).opt()).select(&rp).fold([0i64; 3], |s, a| [s[0] + 1, s[1] + a[0], s[2] + a[1]]);
    let w = win(&ps, rank, |(_, s)| Reverse(fkey(s[2] as f64 / s[0] as f64)), asc);
    rows(drain(&w).into_iter().map(|(_, ((n, s), r))| row(vec![ostr(n), V::I(s[0]), V::I(s[1]), avg(s[2], s[0]), V::I(r)])))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers, SUM(P.Score) AS TotalScore,
//        SUM(P.ViewCount) AS TotalViews, COALESCE(MAX(P.AcceptedAnswerId), -1) AS LastAcceptedAnswerId FROM Posts P GROUP BY P.OwnerUserId),
// RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
//        PS.Questions, PS.Answers, PS.TotalScore, PS.TotalViews, RANK() OVER (ORDER BY PS.TotalScore DESC) AS ScoreRank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId WHERE U.Reputation > 1000)
// SELECT R.UserId, R.DisplayName, R.GoldBadges, R.SilverBadges, R.BronzeBadges, R.Questions, R.Answers, R.TotalScore, R.TotalViews,
//        CASE WHEN R.GoldBadges >= 5 THEN 'Elite' WHEN R.GoldBadges >= 1 THEN 'Gold Member' ELSE 'Regular Member' END AS MembershipLevel
// FROM RankedUsers R WHERE R.ScoreRank <= 10 ORDER BY R.TotalScore DESC, R.UserId;
fn q2339(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let w = whole(db.user.with((&db.user.reputation).gt(1000))).select(Ident::<User>::new().and((&ups).and(&ub))).window(rank, |(_, (a, _))| (a[1] == 0, Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, (a, b)), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(if a[1] == 0 { [V::Null, V::Null, V::Null, V::Null] } else { [V::I(a[2]), V::I(a[3]), V::I(a[4]), nullable(a[6], a[5])] });
        f.push(V::S(if b[0] >= 5 { "Elite" } else if b[0] >= 1 { "Gold Member" } else { "Regular Member" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS ViewRank,
//        RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges,
//        SUM(u.Reputation) AS TotalReputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostCommentCounts AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.Score, COALESCE(pcc.CommentCount, 0) AS TotalComments, COALESCE(cp.CloseCount, 0) AS TotalClosed, ub.TotalReputation,
//        ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM RankedPosts rp LEFT JOIN PostCommentCounts pcc ON pcc.PostId = rp.PostId LEFT JOIN ClosedPosts cp ON cp.PostId = rp.PostId JOIN Users u ON u.Id = rp.PostId JOIN UserBadges ub ON ub.UserId = u.Id
// WHERE rp.ViewRank <= 10 ORDER BY rp.ViewCount DESC, rp.Score DESC;
//
// `u.Id = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q32899(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(p, w)| (w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 4], |a, (r, c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + r]
    });
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let cl = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    rows(drain((&cc).and(&cl).and(origid.select(&uidx).select(&ub))).into_iter().map(|(p, ((c, k), b))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score"]);
        f.extend([V::I(c), V::I(k), V::I(b[3]), V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserStats WHERE PostCount > 10 AND Reputation > 100),
// TopPostStats AS (SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AverageViewCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Id, pt.Name)
// SELECT tu.DisplayName AS TopUser, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.TotalUpVotes, tu.TotalDownVotes, tps.PostTypeName, tps.TotalPosts, tps.TotalScore, tps.AverageViewCount
// FROM TopUsers tu JOIN TopPostStats tps ON tu.QuestionCount > 0 WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC, tps.TotalPosts DESC;
//
// The rank reads Reputation and the distinct post count, so the ten users are picked first and the post x vote product is driven for those alone;
// the ON clause names only tu, so they are crossed with the post types.
fn q6112(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let dp = db.user.with(rep.gt(100)).group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let w = whole((&dp).filt(|n| n > 10)).select(Ident::<User>::new().and(rep)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (t, v)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]);
    let tps = db.post.group_by(&db.post.post_type).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let tps = rel(drain(&tps));
    let mut v = Vec::new();
    (&us).filt(|a: [i64; 4]| a[0] > 0).and(&dp).cross(&tps).drive(|(u, _), ((a, n), (t, s))| v.push((u, a, n, t, s)));
    rows(v.into_iter().map(|(u, a, n, t, s)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(n)];
        f.extend(a.map(V::I));
        f.extend([tname(db, t), V::I(s[0]), V::I(s[1]), avg(s[3], s[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName FROM RankedPosts WHERE PostRank <= 5),
// PostsWithBadges AS (SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.AnswerCount, trp.CommentCount, trp.OwnerDisplayName, COUNT(b.Id) AS BadgeCount
//     FROM TopRankedPosts trp LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = trp.PostId)
//     GROUP BY trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.AnswerCount, trp.CommentCount, trp.OwnerDisplayName)
// SELECT p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerDisplayName, p.BadgeCount,
//        CASE WHEN p.Score > 10 THEN 'High Engagement' WHEN p.Score BETWEEN 5 AND 10 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM PostsWithBadges p ORDER BY p.Score DESC, p.CreationDate DESC LIMIT 50;
fn q7803(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top = top_n(drain((&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p)), |&(_, p)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 50);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.1).collect()).map(|p| p).collect();
    let bc = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain(&bc).into_iter().map(|(p, b)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "comments", "owner"]);
        f.push(V::I(b));
        f.push(V::S(if s > 10 { "High Engagement" } else if s >= 5 { "Moderate Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerName, p.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        DENSE_RANK() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS VoteRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName, p.CreationDate),
// TagDetails AS (SELECT t.TagName, COUNT(pt.Id) AS PostCount FROM Tags t JOIN Posts pt ON pt.Tags LIKE '%' || t.TagName || '%' WHERE pt.PostTypeId = 1 GROUP BY t.TagName),
// HighVotePosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.OwnerName, rp.CreationDate, rp.UpVotes, rp.DownVotes, rp.CommentCount FROM RankedPosts rp WHERE rp.VoteRank <= 10)
// SELECT h.PostId, h.Title, h.Body, h.OwnerName, h.CreationDate, h.UpVotes, h.DownVotes, h.CommentCount, td.TagName, td.PostCount
// FROM HighVotePosts h LEFT JOIN TagDetails td ON h.Title LIKE '%' || td.TagName || '%' ORDER BY h.UpVotes - h.DownVotes DESC, h.CreationDate DESC;
//
// The title LIKE is a substring join: select_where scans the tag names for each distinct title of the ranked posts.
fn q28920(db: &'static So) -> String {
    let Post { post_type_id, title, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let w = win(&rp, dense_rank, |(_, a)| Reverse(a[0] - a[1]), asc);
    type R = (Id<Post>, [i64; 3]);
    let hv: MatSet<R> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    type P = (Id<Post>, Id<Tag>);
    let lt = tag_mentions(db);
    let asked = Ident::<Post>::new().with(post_type_id.eq(1));
    let td = (&lt).group_by(Same::<P>::new().map(|(_, t): P| t).select(&db.tag.tag_name)).select(Same::<P>::new().map(|(p, _): P| p).select(asked)).fold(0i64, |n, _| n + 1);
    let tdm: MatSet<(Str, i64)> = whole(&td).select(Same::<Str>::new().and(&td)).map(|x| x).collect();
    let by_name: HashIdx<Str, (Str, i64)> = (&tdm).map(|(n, _): (Str, i64)| n).inv().select(&tdm).collect();
    let titles: MatSet<Str> = (&hv).map(|(p, _): R| p).select(title).collect();
    let like: HashIdx<Str, (Str, i64)> = (&titles).select_where(&by_name, |t: Str, n: Str| like_in(t, "", n, "")).collect();
    let v = drain((&hv).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(title.select(&like).opt()))));
    rows(v.into_iter().map(|(_, ((p, a), t))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "owner", "created"]);
        f.extend(a.map(V::I));
        f.extend(match t {
            Some((n, c)) => [V::S(n), V::I(c)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.Tags, u.DisplayName AS OwnerName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankByViews, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostActivities AS (SELECT rp.PostId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount,
//        SUM(CASE WHEN bh.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount, SUM(CASE WHEN bh.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount
//     FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId = rp.PostId LEFT JOIN Votes v ON v.PostId = rp.PostId LEFT JOIN PostHistory bh ON bh.PostId = rp.PostId GROUP BY rp.PostId)
// SELECT rp.PostId, rp.Title, rp.OwnerName, rp.CreationDate, rp.ViewCount, rp.Score, rp.Tags, pa.CommentCount, pa.UpVoteCount, pa.DownVoteCount, pa.CloseCount, pa.ReopenCount,
//        CASE WHEN rp.RankByViews <= 5 THEN 'Top Views' WHEN rp.RankByScore <= 5 THEN 'Top Scores' ELSE 'Others' END AS PostCategory
// FROM RankedPosts rp JOIN PostActivities pa ON rp.PostId = pa.PostId WHERE rp.RankByViews <= 10 OR rp.RankByScore <= 10 ORDER BY rp.RankByViews, rp.RankByScore DESC;
//
// Both ranks read only base columns, so the ranked posts are picked first and the comment x vote x history product is driven for those alone.
fn q29381(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    type T = ((Id<Post>, Option<i64>), i64);
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(view_count.opt()).and(score))
        .window(rank, |((_, w), _)| (w.is_none(), Reverse(w)), asc);
    let w = (&w).window(rank, |((_, s), _): (T, i64)| Reverse(s), asc);
    let rk: MatSet<(Id<Post>, (i64, i64))> = (&w).filt(|((_, a), b)| a <= 10 || b <= 10).map(|((((p, _), _), a), b)| (p, (a, b))).collect();
    let ranks: HashIdx<Id<Post>, (i64, i64)> = (&rk).map(|(p, _)| p).inv().select((&rk).map(|(_, r)| r)).collect();
    let tp: MatSet<Id<Post>> = (&rk).map(|(p, _)| p).collect();
    let pa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 5], |a, ((c, t), h)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (h == Some(10)) as i64, a[4] + (h == Some(11)) as i64]);
    rows(drain((&pa).and(&ranks)).into_iter().map(|(p, (a, (r1, r2)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score", "tags"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r1 <= 5 { "Top Views" } else if r2 <= 5 { "Top Scores" } else { "Others" }));
        row(f)
    }))
}

// WITH UserVotes AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (ORDER BY COUNT(V.Id) DESC) AS VoteRank FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COUNT(C.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS OwnerPostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId),
// ClosedPosts AS (SELECT PH.PostId, PH.Comment AS CloseReason, COUNT(P.Id) AS RelatedCount FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId, PH.Comment)
// SELECT U.DisplayName AS UserName, U.VoteCount, U.UpVotes, U.DownVotes, PP.PostId, PP.Title, PP.CreationDate, PP.Score, PP.ViewCount, COALESCE(CP.CloseReason, 'Not Closed') AS CloseReason,
//        COALESCE(CP.RelatedCount, 0) AS RelatedCount
// FROM UserVotes U JOIN PopularPosts PP ON U.UserId = PP.OwnerPostRank LEFT JOIN ClosedPosts CP ON PP.PostId = CP.PostId WHERE U.VoteRank <= 10 AND (U.UpVotes - U.DownVotes) > 5
// ORDER BY U.VoteCount DESC, PP.Score DESC;
//
// `U.UserId = PP.OwnerPostRank` joins a user id to a row number, so it goes through the raw user id.
fn q22188(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let uw = win(&uv, rank, |(_, a)| Reverse(a[0]), asc);
    type U = (Id<User>, [i64; 3]);
    let tu: MatSet<U> = (&uw).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let Post { post_type_id, owner_user_id, score, .. } = &db.post;
    let rn = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let pp: MatSet<(Id<Post>, i64)> = (&rn).map(|((p, _), r)| (p, r)).collect();
    let by_rank: HashIdx<i64, (Id<Post>, i64)> = (&pp).map(|(_, r)| r).inv().select(&pp).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post.and(comment.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cp = rel(drain(&cp));
    let cpp: HashIdx<Id<Post>, ((Id<Post>, Option<Str>), i64)> = (&cp).map(|((p, _), _)| p).inv().select(&cp).collect();
    let v = drain((&tu).filt(|(_, a): U| a[1] - a[2] > 5).select(Same::<U>::new().and(Same::<U>::new().map(|(u, _): U| u).select(&db.user.origid).select(&by_rank).select(Same::<(Id<Post>, i64)>::new().map(|(p, _)| p).select(Ident::<Post>::new().and((&cpp).opt()))))));
    rows(v.into_iter().map(|(_, ((u, a), (p, c)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend(match c {
            Some(((_, r), n)) => [V::S(r.unwrap_or("Not Closed")), V::I(n)],
            None => [V::S("Not Closed"), V::I(0)],
        });
        row(f)
    }))
}

// WITH RECURSIVE RecursivePosts AS (SELECT p.Id AS PostID, p.Title, p.OwnerUserId, p.AcceptedAnswerId, p.CreationDate, p.Score, p.ViewCount, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT a.Id, a.Title, a.OwnerUserId, a.AcceptedAnswerId, a.CreationDate, a.Score, a.ViewCount, rp.Level + 1 FROM Posts a JOIN RecursivePosts rp ON a.ParentId = rp.PostID)
// SELECT u.Id AS UserID, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN a.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount, SUM(COALESCE(COMMENT_COUNT.comment_count, 0)) AS TotalCommentCount,
//        SUM(COALESCE(VOTE_COUNT.upvote_count, 0)) AS TotalUpVotes, SUM(COALESCE(VOTE_COUNT.downvote_count, 0)) AS TotalDownVotes
// FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Posts a ON p.Id = a.ParentId
// LEFT JOIN (SELECT c.PostId, COUNT(c.Id) AS comment_count FROM Comments c GROUP BY c.PostId) AS COMMENT_COUNT ON p.Id = COMMENT_COUNT.PostId
// LEFT JOIN (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS upvote_count, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS downvote_count
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId) AS VOTE_COUNT ON p.Id = VOTE_COUNT.PostId
// WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(DISTINCT p.Id) > 0 ORDER BY u.Reputation DESC LIMIT 10;
//
// The recursive CTE is never referenced by the final SELECT, so it is not computed. The two COUNT(DISTINCT) come from one row per question.
fn q31562(db: &'static So) -> String {
    let asked = || posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(1)));
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let vc = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(asked().select(children_of(db).select((&db.post.accepted_answer_id).opt()).opt().and((&cc).opt()).and((&vc).opt())).opt())
        .fold([0i64; 4], |s, x| match x {
            Some(((a, c), v)) => {
                let v = v.unwrap_or([0; 2]);
                [s[0] + a.flatten().is_some() as i64, s[1] + c.unwrap_or(0), s[2] + v[0], s[3] + v[1]]
            }
            None => s,
        });
    let d = users().group_by(Ident::<User>::new()).select(asked().select(children_of(db).opt()).opt()).fold([0i64; 2], |s, x| match x {
        Some(a) => [s[0] + 1, s[1] + a.is_some() as i64],
        None => s,
    });
    let qd = users().group_by(Ident::<User>::new()).select(asked().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = top_n(drain((&s).and(&d).and((&qd).filt(|n| n > 0))), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), 10);
    rows(v.into_iter().map(|(u, ((s, d), q))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(q), V::I(d[1])]);
        f.extend(s.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'),
// PostVoteStats AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes, COUNT(v.Id) AS TotalVotes FROM Votes v GROUP BY v.PostId),
// PostHistoryAggregated AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseVotes, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenVotes,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteVotes, MIN(ph.CreationDate) AS FirstHistoryDate, MAX(ph.CreationDate) AS LastHistoryDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.Score, pvs.Upvotes, pvs.Downvotes, pvs.TotalVotes, pha.CloseVotes, pha.ReopenVotes,
//        pha.DeleteVotes, pha.FirstHistoryDate, pha.LastHistoryDate
// FROM RankedPosts rp LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId LEFT JOIN PostHistoryAggregated pha ON rp.PostId = pha.PostId WHERE rp.Rank <= 5 ORDER BY rp.PostId, rp.Score DESC;
fn q6747(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([0, 0, 0, i64::MAX, i64::MIN], |a, (t, d)| {
        [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + matches!(t, 12 | 13) as i64, a[3].min(d), a[4].max(d)]
    });
    rows(drain((&tp).select(Ident::<Post>::new().and((&pvs).opt()).and((&pha).opt()))).into_iter().map(|(_, ((p, v), h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "answers", "score"]);
        f.extend(match v {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match h {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(a[3]), V::T(a[4])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(badge.BadgeCount, 0) AS BadgeCount, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.QuestionCount, 0) AS QuestionCount,
//        COALESCE(ps.AnswerCount, 0) AS AnswerCount, COALESCE(ps.TotalViews, 0) AS TotalViews, COALESCE(ps.TotalScore, 0) AS TotalScore, RANK() OVER (ORDER BY COALESCE(ps.TotalScore, 0) DESC) AS ScoreRank
//     FROM Users u LEFT JOIN UserBadgeStats badge ON u.Id = badge.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT up.UserId, up.DisplayName, up.BadgeCount, up.PostCount, up.QuestionCount, up.AnswerCount, up.TotalViews, up.TotalScore, up.ScoreRank FROM UserPerformance up
// WHERE up.TotalViews > 0 ORDER BY up.ScoreRank, up.TotalScore DESC LIMIT 10;
fn q5192(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = whole(&ups).select(Ident::<User>::new().and((&ups).and(&bc))).window(rank, |(_, (a, _))| Reverse(a[4]), asc);
    let v = top_n(drain((&w).filt(|((_, (a, _)), _)| a[6] > 0)), |&(_, (_, r))| r, 10);
    rows(v.into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalVotes, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalVotes DESC) AS UserRank FROM UserVoteSummary),
// PostSummary AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS HasAcceptedAnswer
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId),
// ResultSet AS (SELECT Pu.DisplayName AS UserDisplayName, Pu.TotalVotes AS UserTotalVotes, Ps.Title AS PostTitle, Ps.CommentCount, Ps.TotalUpVotes, Ps.TotalDownVotes, Ps.HasAcceptedAnswer,
//        RANK() OVER (PARTITION BY Pu.UserId ORDER BY Ps.TotalUpVotes DESC) AS PostRank FROM TopUsers Pu JOIN PostSummary Ps ON Pu.UserId = Ps.OwnerUserId)
// SELECT * FROM ResultSet WHERE PostRank = 1 ORDER BY UserTotalVotes DESC, UserDisplayName;
fn q9443(db: &'static So) -> String {
    let tv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let acc = &db.post.accepted_answer_id;
    let ps = db
        .post
        .with(&db.post.owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(acc.opt()))
        .fold([0i64; 4], |a, ((c, t), x)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + x.is_some() as i64]);
    let w = db.post.group_by(&db.post.owner_user).select(Ident::<Post>::new().and(&ps)).window(rank, |(_, a)| Reverse(a[1]), asc);
    let v = drain((&w).filt(|(_, r)| r == 1).map(|(x, _)| x).and(&tv));
    rows(v.into_iter().map(|(_, ((p, a), n))| {
        let mut f = vec![post_fields(db, p, &["owner"]).remove(0), V::I(n)];
        f.push(post_fields(db, p, &["title"]).remove(0));
        f.extend(a.map(V::I));
        f.push(V::I(1));
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT Id, Reputation, CAST(CreationDate AS DATE) AS CreationDate, 1 AS Level FROM Users WHERE Reputation > 1000
//     UNION ALL SELECT U.Id, U.Reputation, CAST(U.CreationDate AS DATE), Level + 1 FROM Users U INNER JOIN UserReputation UR ON U.Id = UR.Id WHERE UR.Reputation < U.Reputation),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(NULLIF(P.AcceptedAnswerId, -1), 0) AS AcceptedAnswerId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        SUM(V.BountyAmount) AS TotalBounty FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (2, 3)
//     WHERE P.CreationDate > '2020-01-01' GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.AcceptedAnswerId),
// TopUsers AS (SELECT U.DisplayName, U.Id, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews, COUNT(DISTINCT P.Id) AS TotalPosts FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     WHERE U.Reputation > 1000 GROUP BY U.DisplayName, U.Id HAVING COUNT(DISTINCT P.Id) > 10 ORDER BY TotalScore DESC LIMIT 10)
// SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.CommentCount, PS.TotalBounty, TU.DisplayName AS TopUser, UR.Reputation AS UserReputation
// FROM PostStatistics PS LEFT JOIN TopUsers TU ON PS.AcceptedAnswerId = TU.Id LEFT JOIN UserReputation UR ON TU.Id = UR.Id WHERE PS.Score > 10 ORDER BY PS.CommentCount DESC, PS.TotalBounty DESC;
//
// The recursive step joins a user to itself and keeps it only when its reputation is below its own, which is never, so UserReputation is its base case.
// `PS.AcceptedAnswerId = TU.Id` joins a post id to a user id, so it goes through the raw ids.
fn q32973(db: &'static So) -> String {
    let Post { creation_date, score, accepted_answer_id, .. } = &db.post;
    let votes23 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select((&db.vote.bounty_amount).opt());
    let ps = db
        .post
        .with(creation_date.gt(ts(2020, 1, 1, 0, 0, 0)).and(score.gt(10)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes23.opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let ups = user_posts(db);
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select((&ups).filt(|a| a[1] > 10))), |&(_, a)| Reverse(a[4]), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ur = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain((&ps).and(accepted_answer_id.opt().map(|a: Option<i64>| a.filter(|&x| x != -1).unwrap_or(0)).select(&uidx).select(Ident::<User>::new().with(&tu).and(ur.opt())).opt()));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1])]);
        f.extend(match t {
            Some((u, r)) => [user_col(db, u, "name"), r.map_or(V::Null, |u| user_col(db, u, "rep"))],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecursivePosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN rp.RowNum = 1 THEN 1 END), 0) AS RecentPostsCount, COALESCE(SUM(rp.CommentCount), 0) AS TotalComments,
//        COALESCE(SUM(rp.UpVotes) - SUM(rp.DownVotes), 0) AS VotesNet FROM Users u LEFT JOIN RecursivePosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, RecentPostsCount, TotalComments, VotesNet, RANK() OVER (ORDER BY VotesNet DESC, Reputation DESC) AS UserRank FROM UserStats)
// SELECT tu.DisplayName, tu.Reputation, tu.RecentPostsCount, tu.TotalComments, tu.VotesNet,
//        CASE WHEN tu.UserRank <= 10 THEN 'Top Contributor' WHEN tu.UserRank > 10 AND tu.UserRank <= 50 THEN 'Contributor' ELSE 'New User' END AS UserCategory
// FROM TopUsers tu WHERE tu.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY tu.VotesNet DESC, tu.Reputation DESC;
//
// A user's posts have exactly one RowNum = 1 among them, so RecentPostsCount is 1 for a user with posts and 0 otherwise.
fn q33999(db: &'static So) -> String {
    let rp = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&rp).opt()).fold([0i64; 4], |s, a| match a {
        Some(a) => [1, s[1] + a[0], s[2] + a[1] - a[2], s[3] + 1],
        None => s,
    });
    let rep = &db.user.reputation;
    let (sum, n) = db.user.select(rep).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let mean = sum as f64 / n as f64;
    let w = whole(&us).select(Ident::<User>::new().and(rep).and(&us)).window(rank, |((_, r), a)| (Reverse(a[2]), Reverse(r)), asc);
    rows(drain((&w).filt(move |(((_, r), _), _)| r as f64 > mean)).into_iter().map(|(_, (((u, _), a), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Contributor" } else { "New User" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN V.VoteTypeId = 5 THEN 1 END) AS Favorites, COUNT(V.Id) AS TotalVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount, MAX(P.CreationDate) AS LastPostDate FROM Posts P GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UV.UpVotes, 0) AS UpVotes, COALESCE(UV.DownVotes, 0) AS DownVotes, COALESCE(UV.Favorites, 0) AS Favorites,
//        COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.TotalScore, 0) AS TotalScore, COALESCE(PS.AvgViewCount, 0) AS AvgViewCount, PS.LastPostDate
//     FROM Users U LEFT JOIN UserVoteStats UV ON U.Id = UV.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT C.UserId, C.DisplayName, C.UpVotes, C.DownVotes, C.Favorites, C.PostCount, C.TotalScore, C.AvgViewCount, C.LastPostDate,
//        CASE WHEN C.PostCount = 0 THEN 'No Posts' ELSE CONCAT('Posts: ', C.PostCount, ', Score: ', C.TotalScore) END AS PostSummary, RANK() OVER (ORDER BY C.TotalScore DESC) AS ScoreRank
// FROM CombinedStats C WHERE C.UpVotes - C.DownVotes > 5 ORDER BY C.TotalScore DESC LIMIT 10 OFFSET 0;
fn q4379(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(5)) as i64]
    });
    let ups = user_posts(db);
    let w = whole((&uv).filt(|a| a[0] - a[1] > 5)).select(Ident::<User>::new().and((&uv).and(&ups))).window(rank, |(_, (_, a))| Reverse(a[4]), asc);
    let v = top_n(drain(&w), |&(_, (_, r))| r, 10);
    rows(v.into_iter().map(|(_, ((u, (b, a)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[1]), V::I(a[4]), if a[5] == 0 { V::F(0.0) } else { avg(a[6], a[5]) }, tmax(a[7])]);
        f.push(if a[1] == 0 { V::S("No Posts") } else { V::Owned(format!("Posts: {}, Score: {}", a[1], a[4])) });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount, COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.LastActivityDate, U.DisplayName AS OwnerDisplayName, COUNT(C.CreationDate) AS CommentCount, COUNT(DISTINCT PH.Id) AS EditHistoryCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     GROUP BY P.Id, P.Title, P.CreationDate, P.LastActivityDate, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY UpVotes - DownVotes DESC) AS Rank FROM UserStats)
// SELECT T.UserId, T.DisplayName, T.UpVotes, T.DownVotes, P.PostId, P.Title, P.CreationDate, P.LastActivityDate, P.CommentCount, P.EditHistoryCount
// FROM TopUsers T JOIN PostActivity P ON P.OwnerDisplayName = T.DisplayName WHERE T.Rank <= 10 ORDER BY T.Rank, P.LastActivityDate DESC;
//
// The distinct counts are never read. PostActivity joins on its owner's display name, so the posts are indexed by that name.
fn q6921(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(Some(2))) as i64, a[1] + (t == Some(Some(3))) as i64]);
    let tu = top_n(drain(&us), |&(_, a)| Reverse(a[0] - a[1]), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { owner_user, .. } = &db.post;
    let names: MatSet<Str> = (&tu).select(&db.user.display_name).collect();
    let pa = db
        .post
        .with(owner_user.select(&db.user.display_name).select(&names))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).opt()))
        .fold([0i64; 1], |a, (c, _)| [a[0] + c.is_some() as i64]);
    let hc = db.post.with(&pa).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let by_name: HashIdx<Str, Id<Post>> = db.post.with(&pa).select(owner_user.select(&db.user.display_name)).inv().collect();
    let v = drain((&tu).select(Ident::<User>::new().and(&us).and((&db.user.display_name).select(&by_name).select(Ident::<Post>::new().and(&pa).and(&hc)))));
    rows(v.into_iter().map(|(_, ((u, a), ((p, c), h)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "activity"]));
        f.extend([V::I(c[0]), V::I(h)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS VoteRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// TopVotedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.AnswerCount, rp.UpVotes, rp.DownVotes, rp.VoteRank FROM RankedPosts rp WHERE rp.VoteRank <= 10)
// SELECT tvp.PostId, tvp.Title, tvp.CreationDate, tvp.OwnerDisplayName, tvp.CommentCount, tvp.AnswerCount, tvp.UpVotes, tvp.DownVotes, (tvp.UpVotes - tvp.DownVotes) AS NetVotes,
//        COUNT(DISTINCT ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate
// FROM TopVotedPosts tvp LEFT JOIN PostHistory ph ON tvp.PostId = ph.PostId
// GROUP BY tvp.PostId, tvp.Title, tvp.CreationDate, tvp.OwnerDisplayName, tvp.CommentCount, tvp.AnswerCount, tvp.UpVotes, tvp.DownVotes ORDER BY NetVotes DESC, tvp.CreationDate DESC;
//
// The vote sums are folded over the comment x answer x vote product; the two COUNT(DISTINCT) come from one row per question.
fn q8585(db: &'static So) -> String {
    let asked = || db.post.with((&db.post.post_type_id).eq(1));
    let rp = asked()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let w = win(&rp, rank, |(_, a)| Reverse(a[0] - a[1]), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let eh = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    rows(drain((&cc).and(&ac).and(&rp).and(&eh)).into_iter().map(|(p, (((c, a), v), (n, m)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(a), V::I(v[0]), V::I(v[1]), V::I(v[0] - v[1]), V::I(n), tmax(m)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, WikiCount, TotalUpvotes, TotalDownvotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserStats WHERE Reputation > 1000),
// PopularQuestions AS (SELECT p.Id AS QuestionId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.ViewCount HAVING COUNT(c.Id) > 5 AND COUNT(DISTINCT v.UserId) > 10)
// SELECT tu.DisplayName AS TopUser, tu.Reputation, tu.QuestionCount, pu.Title AS PopularQuestion, pu.Score, pu.ViewCount, pu.CommentCount, pu.VoteCount
// FROM TopUsers tu JOIN PopularQuestions pu ON tu.QuestionCount > 0 ORDER BY tu.ReputationRank, pu.Score DESC LIMIT 10;
//
// The ON clause names only tu, so the users are crossed with the popular questions.
fn q6523(db: &'static So) -> String {
    let user_id = &db.vote.user_id;
    let tu = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).opt())).opt())
        .fold(0i64, |n, p| n + (p.map(|x| x.0) == Some(1)) as i64);
    let tw = whole(&tu).select(Ident::<User>::new().and(&db.user.reputation).and(&tu)).window(rank, |((_, r), _)| Reverse(r), asc);
    let tu = (&tw).filt(|((_, n), _)| n > 0).map(|(((u, _), n), r)| ((u, n), r));
    let asked = || db.post.with((&db.post.post_type_id).eq(1));
    let cc = asked().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let dv = asked().group_by(Ident::<Post>::new()).select(votes_of(db).select(user_id)).count_distinct();
    let pq = rel(drain((&cc).filt(|n| n > 5).and((&dv).filt(|n| n > 10))));
    let mut v = Vec::new();
    tu.cross(&pq).drive(|_, (((u, q), r), (p, (c, n)))| v.push((u, q, r, p, c, n)));
    let v = top_n(v, |&(_, _, r, p, _, _)| (r, Reverse(db.post.score.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(u, q, _, p, c, n)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(q));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.ViewCount > 100),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBounty FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN Votes v ON u.Id = v.UserId AND v.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// PostInteractions AS (SELECT p.Id AS PostId, SUM(c.Score) AS TotalCommentScore, COUNT(DISTINCT v.UserId) AS UniqueVoters, COUNT(DISTINCT c.Id) AS TotalComments FROM Posts p
//     LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id)
// SELECT rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, au.DisplayName AS ActiveUser, au.BadgeCount, au.TotalBounty, pi.TotalCommentScore, pi.UniqueVoters, pi.TotalComments
// FROM RankedPosts rp JOIN ActiveUsers au ON au.UserId = rp.PostId JOIN PostInteractions pi ON pi.PostId = rp.PostId WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// `au.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q8802(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { post_type_id, creation_date, view_count, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).with(view_count.gt(100)).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let Vote { creation_date: vd, bounty_amount, user_id, .. } = &db.vote;
    let recent = || Ident::<Vote>::new().with(vd.ge(add_years(t0, -1)));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let au: MatSet<Id<User>> = (&tp).select(origid.select(&uidx)).collect();
    let ab = (&au).group_by(Ident::<User>::new()).select(badges_of(db).opt().and(votes_by(db).select(recent()).select(bounty_amount.opt()).opt())).fold((0i64, 0i64), |(n, s), (_, b)| {
        let b = b.flatten();
        (n + b.is_some() as i64, s + b.unwrap_or(0))
    });
    let bd = (&au).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pi = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt().and(votes_of(db).select(recent()).opt())).fold((0i64, 0i64), |(n, s), (c, _)| {
        (n + c.is_some() as i64, s + c.unwrap_or(0))
    });
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(recent()).select(user_id)).count_distinct();
    let tc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&pi).and((&uv).opt()).and(&tc).and(origid.select(&uidx).select(Ident::<User>::new().and(&ab).and(&bd))));
    rows(v.into_iter().map(|(p, ((((n, s), u), c), ((w, (bn, bs)), b)))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score", "answers", "comments"]);
        f.extend([user_col(db, w, "name"), V::I(b), nullable(bs, bn), nullable(s, n), V::I(u.unwrap_or(0)), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.TagRank = 1),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpvoteCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownvoteCount FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName),
// FinalResults AS (SELECT pd.*, CASE WHEN pd.Score >= 50 THEN 'High' WHEN pd.Score >= 20 THEN 'Medium' ELSE 'Low' END AS ScoreCategory FROM PostDetails pd)
// SELECT fr.OwnerDisplayName, fr.Title, fr.CreationDate, fr.Score, fr.ViewCount, fr.CommentCount, fr.UpvoteCount, fr.DownvoteCount, fr.ScoreCategory FROM FinalResults fr
// ORDER BY fr.Score DESC, fr.ViewCount DESC LIMIT 10;
//
// TagRank and the cut read only base columns, so the ten posts are picked first and the comment x vote product is driven for those alone.
fn q6227(db: &'static So) -> String {
    let Post { owner_user, creation_date, tags_str, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| Reverse(s), asc);
    let top = top_n(drain((&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p)).into_iter().map(|x| (x.1, ())).collect(), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pd = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&pd).into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["owner", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(if s >= 50 { "High" } else if s >= 20 { "Medium" } else { "Low" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, p.AnswerCount, p.CommentCount, p.Tags,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 100),
// TagStats AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AvgScore FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
//     WHERE p.PostTypeId = 1 GROUP BY t.TagName),
// PostHistories AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, p.Title, pt.Name AS PostHistoryTypeName, COUNT(*) OVER (PARTITION BY ph.PostId) AS EditCount
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id WHERE pt.Id IN (4, 5, 6))
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.ViewCount, rp.Score, ts.TagName, ts.PostCount, ts.TotalViews, ts.AvgScore, phs.EditCount
// FROM RankedPosts rp LEFT JOIN TagStats ts ON rp.Tags LIKE '%' || ts.TagName || '%' LEFT JOIN PostHistories phs ON rp.PostId = phs.PostId WHERE rp.PostRank = 1 ORDER BY rp.CreationDate DESC;
//
// Both LIKEs are the tag substring join, through tag_mentions; a question mentions only tags that have a question, so every mention finds its TagStats row.
fn q29245(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, view_count, score, .. } = &db.post;
    let lt = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    type N = (Id<Post>, Str);
    let names: MatSet<N> = (&lt).map(|(p, t): M| (p, t)).select(Same::<M>::new().map(|(p, _): M| p).and(Same::<M>::new().map(|(_, t): M| t).select(&db.tag.tag_name))).map(|x| x).collect();
    let qm = || (&lt).with(Same::<M>::new().map(|(p, _): M| p).select(post_type_id.eq(1))).group_by(Same::<M>::new().map(|(_, t): M| t).select(&db.tag.tag_name));
    let tst = qm().select(Same::<M>::new().map(|(p, _): M| p).select(view_count.opt().and(score))).fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let pc = qm().select(Same::<M>::new().map(|(p, _): M| p)).count_distinct();
    let by_post: HashIdx<Id<Post>, Str> = (&names).map(|(p, _): N| p).inv().select(Same::<N>::new().map(|(_, n): N| n)).collect();
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .with(view_count.gt(100))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let edits = || history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let ec = (&tp).group_by(Ident::<Post>::new()).select(edits().opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&tp).select(Ident::<Post>::new().and((&by_post).select(Same::<Str>::new().and((&pc).and(&tst))).opt()).and(edits().opt()).and(&ec)));
    rows(v.into_iter().map(|(_, (((p, t), h), n))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score"]);
        f.extend(match t {
            Some((t, (c, a))) => [V::S(t), V::I(c), nullable(a[2], a[1]), avg(a[3], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(if h.is_some() { V::I(n) } else { V::Null });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AvgScore, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN pp.LinkTypeId = 3 THEN 1 ELSE 0 END), 0) AS DuplicatePosts
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostLinks pp ON p.Id = pp.PostId GROUP BY p.OwnerUserId),
// FinalStats AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.UpVotes - us.DownVotes AS NetVotes, ps.PostCount, ps.AvgScore, ps.CommentCount, ps.DuplicatePosts,
//        CASE WHEN ps.PostCount > 10 THEN 'Active' WHEN ps.PostCount BETWEEN 1 AND 10 THEN 'Moderate' ELSE 'Inactive' END AS ActivityStatus
//     FROM UserStats us LEFT JOIN PostStats ps ON us.UserId = ps.OwnerUserId)
// SELECT fs.DisplayName, fs.Reputation, fs.NetVotes, fs.AvgScore, fs.CommentCount, fs.DuplicatePosts, fs.ActivityStatus, RANK() OVER (ORDER BY fs.Reputation DESC) AS ReputationRank
// FROM FinalStats fs WHERE fs.Reputation >= (SELECT AVG(Reputation) FROM Users) ORDER BY fs.Reputation DESC;
fn q4019(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let (sum, n) = db.user.select(rep).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let mean = sum as f64 / n as f64;
    let users = || db.user.with(rep.filt(move |r| r as f64 >= mean));
    let nv = users().group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let ps = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).opt()).and(links_of(db).select(&db.post_link.link_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((s, c), l)) => [a[0] + 1, a[1] + s, a[2] + c.is_some() as i64, a[3] + (l == Some(3)) as i64],
            None => a,
        });
    let w = whole(&nv).select(Ident::<User>::new().and(rep).and((&nv).and(&ps))).window(rank, |((_, r), _)| Reverse(r), asc);
    rows(drain(&w).into_iter().map(|(_, (((u, _), (n, a)), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(if a[0] == 0 { [V::Null, V::Null, V::Null] } else { [avg(a[1], a[0]), V::I(a[2]), V::I(a[3])] });
        f.push(V::S(if a[0] > 10 { "Active" } else if a[0] >= 1 { "Moderate" } else { "Inactive" }));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankUserReputation AS (SELECT Id AS UserId, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// RecentPostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, (SELECT COUNT(*) FROM Posts a WHERE a.ParentId = p.Id) AS AnswerCount,
//        CASE WHEN EXISTS (SELECT 1 FROM Posts WHERE Id = p.AcceptedAnswerId) THEN 1 ELSE 0 END AS HasAcceptedAnswer
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostHistorySummary AS (SELECT PostId, COUNT(CASE WHEN PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
//        COUNT(CASE WHEN PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteUndeleteCount, COUNT(*) AS TotalEdits FROM PostHistory GROUP BY PostId)
// SELECT p.PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.CommentCount, p.AnswerCount, p.HasAcceptedAnswer, ph.CloseCount, ph.ReopenCount, ph.DeleteUndeleteCount, ph.TotalEdits,
//        r.UserId AS TopUserId, r.Reputation, r.ReputationRank
// FROM RecentPostStats p LEFT JOIN PostHistorySummary ph ON p.PostId = ph.PostId JOIN RankUserReputation r ON p.OwnerUserId = r.UserId ORDER BY p.CreationDate DESC, p.ViewCount DESC LIMIT 100;
fn q8035(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, accepted_answer, .. } = &db.post;
    let rw = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rr: MatSet<(Id<User>, i64)> = (&rw).map(|((u, _), r)| (u, r)).collect();
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().select(&rr).collect();
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.select(&by_user)));
    let top = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(creation_date.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 100);
    let tp = rel(top);
    type R = (Id<Post>, (Id<User>, i64));
    let pid = || Same::<R>::new().map(|(p, _): R| p);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let ac = db.post.group_by(&db.post.parent).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let ph = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 4], |a, t| {
        [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + matches!(t, 12 | 13) as i64, a[3] + 1]
    });
    let v = drain((&tp).select(Same::<R>::new().and(pid().select((&cc).opt()).and(pid().select((&ac).opt())).and(pid().select(accepted_answer.opt())).and(pid().select((&ph).opt())))));
    rows(v.into_iter().map(|(_, ((p, (u, r)), (((c, a), x), h)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0)), V::I(x.is_some() as i64)]);
        f.extend(match h {
            Some(h) => h.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS PostRank FROM Posts P WHERE P.PostTypeId = 1),
// UserVotes AS (SELECT V.UserId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount FROM Votes V GROUP BY V.UserId),
// PostHistoryAggregates AS (SELECT PH.PostId, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount
//     FROM PostHistory PH GROUP BY PH.PostId),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.OwnerUserId, RP.Score, U.DisplayName AS OwnerDisplayName, COALESCE(UV.UpVoteCount, 0) AS UpVotes, COALESCE(UV.DownVoteCount, 0) AS DownVotes,
//        COALESCE(PHA.CloseCount, 0) AS CloseCount, COALESCE(PHA.ReopenCount, 0) AS ReopenCount
//     FROM RankedPosts RP LEFT JOIN Users U ON RP.OwnerUserId = U.Id LEFT JOIN UserVotes UV ON RP.OwnerUserId = UV.UserId LEFT JOIN PostHistoryAggregates PHA ON RP.PostId = PHA.PostId WHERE RP.PostRank = 1)
// SELECT TP.Title, TP.CreationDate, TP.OwnerDisplayName, TP.UpVotes, TP.DownVotes, TP.CloseCount, TP.ReopenCount FROM TopPosts TP INNER JOIN Users U ON TP.OwnerUserId = U.Id
// WHERE U.Reputation > 1000 ORDER BY TP.Score DESC, TP.CloseCount DESC LIMIT 50;
fn q34367(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pha = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let high = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain((&first).select(Ident::<Post>::new().and(owner_user.select(high).select(Ident::<User>::new().and((&uv).opt()))).and((&pha).opt())));
    let v = top_n(v, |&(_, ((p, _), h))| (Reverse(score.get(p).unwrap()), Reverse(h.map_or(0, |h| h[0]))), 50);
    rows(v.into_iter().map(|(_, ((p, (_, u)), h))| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend(u.unwrap_or([0; 2]).map(V::I));
        f.extend(h.unwrap_or([0; 2]).map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(P.Score) AS TotalScore,
//        SUM(P.ViewCount) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(B.BadgeCount, 0) AS BadgeCount, COALESCE(B.GoldBadges, 0) AS GoldBadges, COALESCE(B.SilverBadges, 0) AS SilverBadges,
//        COALESCE(B.BronzeBadges, 0) AS BronzeBadges, COALESCE(P.QuestionCount, 0) AS QuestionCount, COALESCE(P.AnswerCount, 0) AS AnswerCount, COALESCE(P.TotalScore, 0) AS TotalScore,
//        COALESCE(P.TotalViews, 0) AS TotalViews FROM Users U LEFT JOIN UserBadges B ON U.Id = B.UserId LEFT JOIN PostStatistics P ON U.Id = P.OwnerUserId)
// SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, QuestionCount, AnswerCount, TotalScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC, TotalViews DESC) AS PerformanceRank
// FROM UserPerformance ORDER BY PerformanceRank FETCH FIRST 50 ROWS ONLY;
fn q5355(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let w = whole(&ub).select(Ident::<User>::new().and((&ub).and(&ups))).window(rank, |(_, (_, a))| (Reverse(a[4]), Reverse(a[6])), asc);
    let v = top_n(drain(&w), |&(_, (_, r))| r, 50);
    rows(v.into_iter().map(|(_, ((u, (b, a)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoredPosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, PositiveScoredPosts, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank,
//        RANK() OVER (ORDER BY TotalUpVotes DESC) AS UpVoteRank FROM UserPostStats)
// SELECT t.UserId, t.DisplayName, t.TotalPosts, t.TotalQuestions, t.TotalAnswers, t.TotalUpVotes, t.TotalDownVotes, t.PositiveScoredPosts, p.PostRank, u.UpVoteRank,
//        CASE WHEN p.PostRank <= 10 THEN 'Top Posters' ELSE 'Regular Posters' END AS PostCategory, CASE WHEN u.UpVoteRank <= 10 THEN 'Top Upvoted Users' ELSE 'Regular Upvoted Users' END AS VoteCategory
// FROM TopUsers t JOIN TopUsers p ON t.UserId = p.UserId JOIN TopUsers u ON t.UserId = u.UserId WHERE t.TotalPosts > 0 OR t.TotalUpVotes > 0 ORDER BY t.TotalPosts DESC, t.TotalUpVotes DESC;
//
// The two self-joins are on the key, so each row meets itself.
fn q25453(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + (s > 0) as i64],
            None => a,
        });
    let w = win(&us, rank, |(_, a)| Reverse(a[0]), asc);
    let w = (&w).window(rank, |((_, a), _): ((Id<User>, [i64; 6]), i64)| Reverse(a[3]), asc);
    rows(drain((&w).filt(|(((_, a), _), _)| a[0] > 0 || a[3] > 0)).into_iter().map(|(_, (((u, a), p), w))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(p), V::I(w)]);
        f.push(V::S(if p <= 10 { "Top Posters" } else { "Regular Posters" }));
        f.push(V::S(if w <= 10 { "Top Upvoted Users" } else { "Regular Upvoted Users" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')),
// RecentActivity AS (SELECT p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.OwnerUserId)
// SELECT ub.UserId, ub.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, pa.PostId, pa.Title AS PopularPostTitle, pa.Score AS PopularPostScore, pa.ViewCount AS PopularPostViews,
//        ra.CommentCount AS RecentCommentCount, ra.VoteCount AS RecentVoteCount
// FROM UserBadges ub JOIN RecentActivity ra ON ub.UserId = ra.OwnerUserId JOIN PopularPosts pa ON ra.OwnerUserId = pa.PostId WHERE ub.BadgeCount > 0 ORDER BY ub.BadgeCount DESC, pa.Score DESC;
//
// `ra.OwnerUserId = pa.PostId` joins a user id to a post id, so it goes through the raw ids; the ROW_NUMBER is never read.
fn q7201(db: &'static So) -> String {
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let ra = db
        .post
        .with(creation_date.ge(since))
        .group_by(owner_user)
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let pa = Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(since)));
    let v = drain((&ub).filt(|b| b[0] > 0).and(&ra).and((&db.user.origid).select(&pidx).select(pa)));
    rows(v.into_iter().map(|(u, ((b, a), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT u.Id, COALESCE(ub.TotalBadges, 0) AS BadgeCount, COALESCE(ps.TotalPosts, 0) AS UserPostCount, COALESCE(ps.TotalQuestions, 0) AS UserQuestionCount,
//        COALESCE(ps.TotalAnswers, 0) AS UserAnswerCount, COALESCE(ps.TotalScore, 0) AS UserTotalScore, COALESCE(ps.TotalViews, 0) AS UserTotalViews, u.Reputation, u.CreationDate, u.DisplayName
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId),
// RankedUsers AS (SELECT ua.*, RANK() OVER (ORDER BY ua.Reputation DESC, ua.BadgeCount DESC, ua.UserTotalScore DESC) AS UserRank FROM UserActivity ua)
// SELECT r.UserRank, r.DisplayName, r.Reputation, r.BadgeCount, r.UserPostCount, r.UserQuestionCount, r.UserAnswerCount, r.UserTotalScore, r.UserTotalViews, r.CreationDate
// FROM RankedUsers r WHERE r.UserRank <= 100 ORDER BY r.UserRank;
fn q8805(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = whole(&ups).select(Ident::<User>::new().and(&db.user.reputation).and((&ups).and(&bc))).window(rank, |((_, r), (a, b))| (Reverse(r), Reverse(b), Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 100)).into_iter().map(|(_, (((u, _), (a, b)), r))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6]), user_col(db, u, "ucreated")]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(*) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionsCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswersCount,
//        AVG(P.Score) AS AverageScore, SUM(P.ViewCount) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT UB.UserId, UB.DisplayName, COALESCE(PS.PostCount, 0) AS TotalPosts, COALESCE(PS.QuestionsCount, 0) AS TotalQuestions, COALESCE(PS.AnswersCount, 0) AS TotalAnswers,
//        COALESCE(PS.AverageScore, 0) AS AvgPostScore, COALESCE(PS.TotalViews, 0) AS TotalPostViews, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges
//     FROM UserBadges UB LEFT JOIN PostStatistics PS ON UB.UserId = PS.OwnerUserId)
// SELECT U.DisplayName, U.BadgeCount, U.GoldBadges, U.SilverBadges, U.BronzeBadges, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.AvgPostScore, U.TotalPostViews,
//        RANK() OVER (ORDER BY U.BadgeCount DESC, U.TotalPostViews DESC) AS Rank
// FROM UserPerformance U WHERE U.BadgeCount > 0 OR U.TotalPosts > 0 ORDER BY Rank, U.DisplayName;
//
// COUNT(*) over the LEFT JOIN counts the one unmatched row too, so every user has BadgeCount >= 1.
fn q27243(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + 1, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let up = (&ub).and(&ups).filt(|(b, a)| b[0] > 0 || a[1] > 0);
    let w = whole(up).select(Ident::<User>::new().and((&ub).and(&ups))).window(rank, |(_, (b, a))| (Reverse(b[0]), Reverse(a[6])), asc);
    rows(drain(&w).into_iter().map(|(_, ((u, (b, a)), r))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[1] == 0 { V::F(0.0) } else { avg(a[4], a[1]) }, V::I(a[6]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis, COUNT(DISTINCT c.Id) AS TotalComments,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, Wikis, TotalComments, GoldBadges, SilverBadges, BronzeBadges, UpVotes, DownVotes,
//        RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, Wikis, TotalComments, GoldBadges, SilverBadges, BronzeBadges, UpVotes, DownVotes, ReputationRank, PostRank
// FROM RankedUsers WHERE ReputationRank <= 50 OR PostRank <= 50 ORDER BY ReputationRank, PostRank;
//
// Both ranks read only Reputation and the distinct post count, so the users are picked first and the post x comment x badge x vote product is driven for those alone.
fn q8863(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    type T = ((Id<User>, i64), Option<i64>);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(rep).and((&dp).opt())).window(rank, |((_, r), _): T| Reverse(r), asc);
    let w = (&w).window(rank, |((_, n), _): (T, i64)| Reverse(n.unwrap_or(0)), asc);
    type R = (Id<User>, i64, i64, i64);
    let pick: MatSet<R> = (&w).filt(|((_, r), p)| r <= 50 || p <= 50).map(|((((u, _), n), r), p)| (u, n.unwrap_or(0), r, p)).collect();
    let tu: MatSet<Id<User>> = (&pick).map(|(u, _, _, _): R| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 8], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |((t, _), v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (b == Some(1)) as i64, a[4] + (b == Some(2)) as i64, a[5] + (b == Some(3)) as i64, a[6] + (v == Some(2)) as i64, a[7] + (v == Some(3)) as i64]
        });
    let dc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&pick).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _, _, _): R| u).select((&us).and(&dc))))).into_iter().map(|(_, ((u, n, r, p), (a, c)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)]);
        f.extend(a[3..].iter().map(|&x| V::I(x)));
        f.extend([V::I(r), V::I(p)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId),
// TopUsers AS (SELECT u.Id, u.DisplayName, COALESCE(ubc.BadgeCount, 0) AS BadgeCount, u.Reputation, ROW_NUMBER() OVER (ORDER BY COALESCE(ubc.BadgeCount, 0) DESC, u.Reputation DESC) AS UserRank
//     FROM Users u LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId WHERE u.Reputation > 100),
// PostVoteSummary AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS Downvotes, COUNT(CASE WHEN VoteTypeId = 10 THEN 1 END) AS Deletions
//     FROM Votes GROUP BY PostId),
// PostsWithVoteInfo AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, COALESCE(pvs.Upvotes, 0) AS Upvotes, COALESCE(pvs.Downvotes, 0) AS Downvotes, COALESCE(pvs.Deletions, 0) AS Deletions,
//        ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RecentPostRank FROM Posts p LEFT JOIN PostVoteSummary pvs ON p.Id = pvs.PostId)
// SELECT tu.DisplayName, tu.BadgeCount, tu.Reputation, COUNT(pwv.PostId) AS PostsContributed, SUM(pwv.Upvotes) AS TotalUpvotes, SUM(pwv.Downvotes) AS TotalDownvotes
// FROM TopUsers tu LEFT JOIN PostsWithVoteInfo pwv ON tu.Id = pwv.OwnerUserId WHERE tu.UserRank <= 10 GROUP BY tu.DisplayName, tu.BadgeCount, tu.Reputation ORDER BY TotalUpvotes DESC, TotalDownvotes ASC;
fn q1154(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let bc = db.user.with(rep.gt(100)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tu = top_n(drain(&bc), |&(u, b)| (Reverse(b), Reverse(rep.get(u).unwrap())), 10);
    let tu = rel(tu);
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type R = (Id<User>, i64);
    let g = (&tu)
        .group_by(Same::<R>::new().map(|(u, _): R| u).select(&db.user.display_name).and(Same::<R>::new().map(|(_, b): R| b)).and(Same::<R>::new().map(|(u, _): R| u).select(rep)))
        .select(Same::<R>::new().map(|(u, _): R| u).select(posts_of(db).select((&pvs).opt()).opt()))
        .fold([0i64; 4], |a, p| match p {
            Some(v) => {
                let v = v.unwrap_or([0; 2]);
                [a[0] + 1, 1, a[2] + v[0], a[3] + v[1]]
            }
            None => a,
        });
    rows(drain(&g).into_iter().map(|(((n, b), r), a)| row(vec![V::S(n), V::I(b), V::I(r), V::I(a[0]), nullable(a[2], a[1]), nullable(a[3], a[1])])))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COALESCE(AVG(score), 0) AS AverageScore,
//        DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY COALESCE(AVG(score), 0) DESC) AS UserPostRank FROM Posts p GROUP BY p.OwnerUserId),
// ClosedPosts AS (SELECT ph.UserId, COUNT(ph.Id) AS ClosedPostCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 OR ph.PostHistoryTypeId = 11 GROUP BY ph.UserId)
// SELECT u.DisplayName, u.Reputation, ubc.TotalBadges, ubc.GoldBadges, ubc.SilverBadges, ubc.BronzeBadges, ps.TotalPosts, ps.TotalViews, ps.AverageScore, COALESCE(cp.ClosedPostCount, 0) AS ClosedCount,
//        CASE WHEN ps.UserPostRank = 1 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributorStatus
// FROM Users u LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN ClosedPosts cp ON u.Id = cp.UserId
// WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users) AND (ubc.TotalBadges > 5 OR ps.TotalPosts > 10) AND COALESCE(ps.AverageScore, 0) > 0 ORDER BY u.Reputation DESC, ubc.TotalBadges DESC;
//
// UserPostRank partitions PostStats by its own group key, so it is 1 on every row.
fn q23401(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let (sum, n) = db.user.select(rep).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let mean = sum as f64 / n as f64;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ups = user_posts(db);
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with(rep.filt(move |r| r as f64 > mean)).select((&ub).and(&ups).and((&cp).opt())).filt(|((b, a), _)| (b[0] > 5 || a[1] > 10) && a[1] > 0 && a[4] > 0));
    rows(v.into_iter().map(|(u, ((b, a), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[1]), V::I(a[6]), avg(a[4], a[1]), V::I(c.unwrap_or(0)), V::S("Top Contributor")]);
        row(f)
    }))
}

// WITH TagsCTE AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, t.TagName, COUNT(t.TagName) AS TagCount FROM Posts p
//     JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS tag) AS tag ON TRUE JOIN Tags t ON t.TagName = tag
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, t.TagName),
// MostPopularTags AS (SELECT TagName, SUM(TagCount) AS TotalUsage FROM TagsCTE GROUP BY TagName ORDER BY TotalUsage DESC LIMIT 10),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName AS UserName, COUNT(DISTINCT p.Id) AS QuestionCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id AND p.PostTypeId = 1 LEFT JOIN Comments c ON c.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// TopContributors AS (SELECT ue.UserId, ue.UserName, ue.QuestionCount, ue.CommentCount, ue.UpvoteCount, (ue.QuestionCount + ue.CommentCount + ue.UpvoteCount) AS TotalEngagement
//     FROM UserEngagement ue ORDER BY TotalEngagement DESC LIMIT 5)
// SELECT t.TagName, tc.UserName, tc.QuestionCount, tc.CommentCount, tc.UpvoteCount FROM MostPopularTags t JOIN TopContributors tc ON t.TotalUsage > 10 ORDER BY t.TagName, tc.TotalEngagement DESC;
//
// The exploded tag list is joined to Tags through an index of tag names. The ON clause names only t, so the tags are crossed with the contributors.
fn q26314(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let by_name: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tu = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list).select(&by_name).select(&db.tag.tag_name)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&tu), |&(_, n)| Reverse(n), 10));
    let users = || db.user.with((&db.user.reputation).gt(100));
    let asked = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let up = users()
        .group_by(Ident::<User>::new())
        .select(asked().opt().and(comments_by(db).opt()).and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold(0i64, |n, (_, t)| n + (t == Some(2)) as i64);
    let qc = users().group_by(Ident::<User>::new()).select(asked().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = users().group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tc = rel(top_n(drain((&up).and(&qc).and(&cc)), |&(_, ((u, q), c))| Reverse(q + c + u), 5));
    let mut v = Vec::new();
    (&tt).filt(|(_, n): (Str, i64)| n > 10).cross(&tc).drive(|_, ((t, _), (u, ((w, q), c)))| v.push((t, u, w, q, c)));
    rows(v.into_iter().map(|(t, u, w, q, c)| row(vec![V::S(t), user_col(db, u, "name"), V::I(q), V::I(c), V::I(w)])))
}

// WITH UserBadgeCount AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViewCount FROM Posts P GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UBC.BadgeCount, 0) AS BadgeCount, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.TotalViewCount, 0) AS TotalViewCount,
//        COALESCE(PS.QuestionCount, 0) AS QuestionCount, COALESCE(PS.AnswerCount, 0) AS AnswerCount, U.Reputation FROM Users U LEFT JOIN UserBadgeCount UBC ON U.Id = UBC.UserId
//     LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId)
// SELECT U.DisplayName, U.Reputation, U.BadgeCount, U.PostCount, U.TotalViewCount, U.QuestionCount, U.AnswerCount, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY U.BadgeCount DESC) AS BadgeRank, RANK() OVER (ORDER BY U.PostCount DESC) AS PostRank
// FROM UserPerformance U WHERE U.Reputation > 1000 ORDER BY U.Reputation DESC, U.BadgeCount DESC LIMIT 10;
fn q26028(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type T = ((Id<User>, i64), ([i64; 10], i64));
    let w = whole(db.user.with(rep.gt(1000))).select(Ident::<User>::new().and(rep).and((&ups).and(&bc))).window(rank, |((_, r), _): T| Reverse(r), asc);
    let w = (&w).window(rank, |((_, (_, b)), _): (T, i64)| Reverse(b), asc);
    let w = (&w).window(rank, |(((_, (a, _)), _), _): ((T, i64), i64)| Reverse(a[1]), asc);
    let v = top_n(drain(&w), |&(_, (((((_, r), (_, b)), _), _), _))| (Reverse(r), Reverse(b)), 10);
    rows(v.into_iter().map(|(_, ((((((u, _), (a, b)), r1), r2), r3)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[6]), V::I(a[2]), V::I(a[3]), V::I(r1), V::I(r2), V::I(r3)]);
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(B.Id) AS TotalBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId),
// UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(P.TotalPosts, 0) AS TotalPosts, COALESCE(P.Questions, 0) AS TotalQuestions, COALESCE(P.Answers, 0) AS TotalAnswers,
//        COALESCE(P.TotalScore, 0) AS TotalScore, COALESCE(P.TotalViews, 0) AS TotalViews, COALESCE(B.GoldBadges, 0) AS GoldBadges, COALESCE(B.SilverBadges, 0) AS SilverBadges,
//        COALESCE(B.BronzeBadges, 0) AS BronzeBadges, COALESCE(B.TotalBadges, 0) AS TotalBadges FROM Users U LEFT JOIN PostStats P ON U.Id = P.OwnerUserId LEFT JOIN UserBadgeStats B ON U.Id = B.UserId)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, GoldBadges, SilverBadges, BronzeBadges, TotalBadges, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
// FROM UserEngagement WHERE TotalPosts > 0 ORDER BY TotalScore DESC, TotalPosts DESC LIMIT 10;
fn q7212(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64, a[3] + 1],
        None => a,
    });
    let v = ranked(drain((&ups).filt(|a| a[1] > 0).and(&ub)), |&(_, (a, _))| Reverse(a[4]), false);
    let v = top_n(v, |&((_, (a, _)), _)| (Reverse(a[4]), Reverse(a[1])), 10);
    rows(v.into_iter().map(|((u, (a, b)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6])]);
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, p.Tags, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= '2020-01-01'),
// TagStatistics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%')
//     WHERE p.PostTypeId = 1 GROUP BY t.TagName),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionsCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Votes v ON v.PostId = p.Id AND v.VoteTypeId IN (8, 9) WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, ts.PostCount AS TagPostCount, ts.TotalViews AS TagTotalViews, ts.AverageScore AS TagAverageScore, ur.DisplayName AS OwnerDisplayName,
//        ur.Reputation AS OwnerReputation, ur.QuestionsCount AS OwnerQuestionsCount, ur.TotalBounty AS OwnerTotalBounty
// FROM RankedPosts rp JOIN TagStatistics ts ON rp.Tags LIKE CONCAT('%', ts.TagName, '%') JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.ScoreRank = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// Both LIKEs are the tag substring join, through tag_mentions; a question mentions only tags that have a question, so every mention finds its TagStatistics row.
fn q25960(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    let lt = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    let tst = (&lt)
        .with(Same::<M>::new().map(|(p, _): M| p).select(post_type_id.eq(1)))
        .group_by(Same::<M>::new().map(|(_, t): M| t))
        .select(Same::<M>::new().map(|(p, _): M| p).select(view_count.opt().and(score)))
        .fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let by_post: HashIdx<Id<Post>, Id<Tag>> = (&lt).map(|(p, _): M| p).inv().select((&lt).map(|(_, t): M| t)).collect();
    let asked = || Ident::<Post>::new().with(post_type_id.eq(1));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ur = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(asked()).select(bounty.opt())).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let uq = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(asked())).fold(0i64, |n, _| n + 1);
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(ts(2020, 1, 1, 0, 0, 0)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and((&by_post).select(Ident::<Tag>::new().and(&tst))).and(owner_user.select(Ident::<User>::new().and(&uq).and(&ur)))));
    rows(v.into_iter().map(|(_, ((p, (_, a)), ((u, q), b)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(q), V::I(b)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostMetrics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        AVG(p.Score) AS AverageScore, MAX(p.CreationDate) AS LastPostDate FROM Posts p GROUP BY p.OwnerUserId),
// UserScores AS (SELECT u.Id, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(pm.PostCount, 0) AS PostCount, COALESCE(pm.QuestionCount, 0) AS QuestionCount,
//        COALESCE(pm.AnswerCount, 0) AS AnswerCount, COALESCE(pm.AverageScore, 0) AS AverageScore, COALESCE(pm.LastPostDate, '1900-01-01') AS LastPostDate
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostMetrics pm ON u.Id = pm.OwnerUserId)
// SELECT u.DisplayName, u.BadgeCount, u.PostCount, u.QuestionCount, u.AnswerCount, u.AverageScore, DENSE_RANK() OVER (ORDER BY u.AverageScore DESC) AS ScoreRank,
//        CASE WHEN u.PostCount = 0 THEN 'No Posts' WHEN u.AverageScore > 10 THEN 'Highly Active' ELSE 'Moderately Active' END AS ActivityLevel
// FROM UserScores u WHERE u.BadgeCount > 0 OR u.PostCount > 0 ORDER BY u.AverageScore DESC, u.BadgeCount DESC LIMIT 10;
fn q3550(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mean = |a: [i64; 10]| if a[1] == 0 { 0.0 } else { a[4] as f64 / a[1] as f64 };
    type R = (Id<User>, ([i64; 10], i64));
    let v = drain(rel(drain((&ups).and(&bc))).filt(|(_, (a, b)): R| b > 0 || a[1] > 0));
    let v = ranked(v, |&(_, (_, (a, _)))| Reverse(fkey(mean(a))), true);
    let v = top_n(v, |&((_, (_, (a, b))), _)| (Reverse(fkey(mean(a))), Reverse(b)), 10);
    rows(v.into_iter().map(|((_, (u, (a, b))), r)| {
        let m = mean(a);
        let mut f = vec![user_col(db, u, "name"), V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::F(m), V::I(r)];
        f.push(V::S(if a[1] == 0 { "No Posts" } else if m > 10.0 { "Highly Active" } else { "Moderately Active" }));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("9070", q9070),
    ("33911", q33911),
    ("9697", q9697),
    ("25866", q25866),
    ("28704", q28704),
    ("395", q395),
    ("25956", q25956),
    ("842", q842),
    ("8446", q8446),
    ("386", q386),
    ("27519", q27519),
    ("26285", q26285),
    ("9545", q9545),
    ("25134", q25134),
    ("3709", q3709),
    ("33518", q33518),
    ("33949", q33949),
    ("30042", q30042),
    ("25885", q25885),
    ("3486", q3486),
    ("1738", q1738),
    ("4297", q4297),
    ("9202", q9202),
    ("9905", q9905),
    ("3750", q3750),
    ("7390", q7390),
    ("27578", q27578),
    ("9279", q9279),
    ("25032", q25032),
    ("6586", q6586),
    ("33551", q33551),
    ("2961", q2961),
    ("3044", q3044),
    ("22621", q22621),
    ("31482", q31482),
    ("7649", q7649),
    ("30838", q30838),
    ("1280", q1280),
    ("2548", q2548),
    ("26076", q26076),
    ("29243", q29243),
    ("2648", q2648),
    ("8849", q8849),
    ("29565", q29565),
    ("4122", q4122),
    ("27788", q27788),
    ("6513", q6513),
    ("3156", q3156),
    ("4721", q4721),
    ("7494", q7494),
    ("31127", q31127),
    ("34363", q34363),
    ("3432", q3432),
    ("7062", q7062),
    ("7481", q7481),
    ("26152", q26152),
    ("27253", q27253),
    ("5106", q5106),
    ("8601", q8601),
    ("6939", q6939),
    ("2378", q2378),
    ("3766", q3766),
    ("7491", q7491),
    ("25928", q25928),
    ("2339", q2339),
    ("32899", q32899),
    ("6112", q6112),
    ("7803", q7803),
    ("28920", q28920),
    ("29381", q29381),
    ("22188", q22188),
    ("31562", q31562),
    ("6747", q6747),
    ("5192", q5192),
    ("9443", q9443),
    ("32973", q32973),
    ("33999", q33999),
    ("4379", q4379),
    ("6921", q6921),
    ("8585", q8585),
    ("6523", q6523),
    ("8802", q8802),
    ("6227", q6227),
    ("29245", q29245),
    ("4019", q4019),
    ("8035", q8035),
    ("34367", q34367),
    ("5355", q5355),
    ("25453", q25453),
    ("7201", q7201),
    ("8805", q8805),
    ("27243", q27243),
    ("8863", q8863),
    ("1154", q1154),
    ("23401", q23401),
    ("26314", q26314),
    ("26028", q26028),
    ("7212", q7212),
    ("25960", q25960),
    ("3550", q3550),
];
