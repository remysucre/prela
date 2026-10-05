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

fn segments(s: Str) -> Vec<Str> {
    s.split('<').skip(1).filter_map(|x| x.find('>').map(|j| &x[..j])).collect()
}

/// The (post, tag) pairs whose Tags contains '<' || TagName || '>': POSITION(..) > 0, or `LIKE '%<' || TagName || '>%'` when
/// `pat`. A name free of '<' and '>' (and, under LIKE, of '%' and '_') can only occur as a whole bracketed segment; any other
/// name is matched against every distinct Tags string.
fn bracketed(db: &'static So, pat: bool) -> MatSet<(Id<Post>, Id<Tag>)> {
    let name = &db.tag.tag_name;
    let plain = move |n: Str| !n.contains(['<', '>']) && !(pat && n.contains(['%', '_']));
    let seg: HashIdx<Str, Id<Tag>> = db.tag.with(name.filt(plain)).select(name).inv().collect();
    let odd: HashIdx<Str, Id<Tag>> = db.tag.with(name.filt(move |n| !plain(n))).select(name).inv().collect();
    let strs: MatSet<Str> = (&db.post.tags_str).collect();
    let hit: HashIdx<Str, Id<Tag>> = (&strs)
        .select_where(&odd, move |s: Str, n: Str| if pat { like(s, &format!("%<{n}>%")) } else { s.contains(&format!("<{n}>")) })
        .collect();
    let tags = &db.post.tags_str;
    db.post.select(Ident::<Post>::new().and(tags.flat_map(segments).select(&seg))).union(db.post.select(Ident::<Post>::new().and(tags.select(&hit)))).collect()
}

/// COUNT(DISTINCT p.Id) per user over `Users LEFT JOIN Posts`; users with no post are absent (join it with `.opt()`).
fn distinct_posts(db: &'static So) -> Fold<Id<User>, i64> {
    db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct()
}

// WITH UserPostStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews,
//        SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers, AVG(COALESCE(p.Score, 0)) AS AverageScore
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, TotalAcceptedAnswers, AverageScore,
//        RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserPostStatistics),
// TopRanked AS (SELECT *, CASE WHEN ScoreRank <= 10 THEN 'Top 10 by Score' WHEN ViewRank <= 10 THEN 'Top 10 by Views' ELSE 'Other' END AS RankCategory FROM TopUsers)
// SELECT RankCategory, COUNT(UserId) AS UserCount, SUM(TotalPosts) AS CombinedPosts, SUM(TotalQuestions) AS CombinedQuestions, SUM(TotalAnswers) AS CombinedAnswers,
//        SUM(TotalScore) AS CombinedScore, SUM(TotalViews) AS CombinedViews, AVG(AverageScore) AS AverageScore
// FROM TopRanked GROUP BY RankCategory ORDER BY UserCount DESC;
fn q28039(db: &'static So) -> String {
    let ups = user_posts(db);
    let j = Ident::<User>::new().with((&db.user.reputation).gt(1000)).and((&ups).filt(|a| a[1] > 0));
    let w1 = whole(db.user.iq()).select(j).window(rank, |(_, a)| Reverse(a[4]), asc);
    let w2 = (&w1).window(rank, |((_, a), _)| (a[5] == 0, Reverse(a[6])), asc);
    type R = (((Id<User>, [i64; 10]), i64), i64);
    let g = (&w2)
        .group_by(Same::<R>::new().map(|((_, s), w): R| if s <= 10 { "Top 10 by Score" } else if w <= 10 { "Top 10 by Views" } else { "Other" }))
        .select(Same::<R>::new().map(|(((_, a), _), _): R| a))
        .fold(([0i64; 7], (0.0f64, 0.0f64)), |(s, k), a| {
            ([s[0] + 1, s[1] + a[1], s[2] + a[2], s[3] + a[3], s[4] + a[4], s[5] + a[5], s[6] + a[6]], kahan(k, a[4] as f64 / a[1] as f64))
        });
    let mut v = drain(&g);
    v.sort_by_key(|&(_, (s, _))| Reverse(s[0]));
    rows(v.into_iter().map(|(c, (s, k))| row(vec![V::S(c), V::I(s[0]), V::I(s[1]), V::I(s[2]), V::I(s[3]), V::I(s[4]), nullable(s[6], s[5]), fmean(k, s[0])])))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY BadgeCount DESC) AS BadgeRank FROM UserBadges),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, U.Reputation AS OwnerReputation,
//        RANK() OVER (ORDER BY P.Score DESC) AS PostRank FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, OwnerReputation, PostRank FROM PostDetails WHERE PostRank <= 10)
// SELECT T.DisplayName AS TopUser, T.BadgeCount AS UserBadgeCount, T.GoldBadges, T.SilverBadges, T.BronzeBadges, P.Title AS TopPostTitle, P.ViewCount AS TopPostViews,
//        P.Score AS TopPostScore, P.CreationDate AS TopPostCreationDate
// FROM TopUsers T JOIN TopPosts P ON T.DisplayName = P.OwnerDisplayName ORDER BY T.BadgeRank, P.Score DESC;
fn q6524(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, score, .. } = &db.post;
    let w = whole(db.post.iq()).select(Ident::<Post>::new().with(owner_user).and(score)).window(rank, |(_, s)| s, desc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let by_name: HashIdx<Str, Id<Post>> = (&tp).select(owner_user.select(&db.user.display_name)).inv().collect();
    let mut v = drain((&ub).and((&db.user.display_name).select(&by_name)));
    v.sort_by_key(|&(_, (a, p))| (Reverse(a[0]), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "views", "score", "created"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// RecentUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, u.Views, u.UpVotes, u.DownVotes, u.LastAccessDate,
//        ROW_NUMBER() OVER (ORDER BY u.CreationDate DESC) AS UserRank FROM Users u WHERE u.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// TopPostsWithComments AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, pc.CommentCount
//     FROM RankedPosts rp LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE rp.Rank <= 10)
// SELECT tpp.Title, tpp.CreationDate, tpp.Score, tpp.ViewCount, tpp.AnswerCount, u.DisplayName AS TopUser, u.Reputation, u.Views
// FROM TopPostsWithComments tpp JOIN Users u ON tpp.AnswerCount > 0 AND u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tpp.PostId LIMIT 1)
// JOIN RecentUsers ru ON u.Id = ru.UserId ORDER BY tpp.Score DESC, tpp.ViewCount DESC FETCH FIRST 20 ROWS ONLY;
fn q7875(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, answer_count, owner_user, .. } = &db.post;
    let since = add_years(current_date(), -1);
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(post_type_id.is_in([1, 2]).and(creation_date.ge(since)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let recent = Ident::<User>::new().with((&db.user.creation_date).ge(since));
    let v = drain((&tp).with(answer_count.gt(0)).select(owner_user.select(recent)));
    let v = top_n(v, |&(p, _)| (key(p), p), 20);
    rows(v.into_iter().map(|(p, u)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers"]);
        f.extend(ucols(db, u, &["name", "rep", "uviews"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank, p.OwnerUserId FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1),
// MostActiveUsers AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount FROM Users U JOIN Posts p ON U.Id = p.OwnerUserId
//     WHERE p.PostTypeId = 1 GROUP BY U.Id, U.DisplayName HAVING COUNT(DISTINCT p.Id) > 5),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment AS CloseReason, ROW_NUMBER() OVER (ORDER BY ph.CreationDate DESC) AS CloseRank FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10),
// TopClosedPosts AS (SELECT cp.PostId, cp.CloseReason, cp.CreationDate AS CloseDate, ROW_NUMBER() OVER (PARTITION BY cp.PostId ORDER BY cp.CloseRank ASC) AS rn FROM ClosedPosts cp)
// SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.OwnerDisplayName, m.UserId AS ActiveUserId, m.DisplayName AS ActiveUserName, tc.CloseReason, tc.CloseDate
// FROM RankedPosts r LEFT JOIN MostActiveUsers m ON r.OwnerUserId = m.UserId LEFT JOIN TopClosedPosts tc ON r.PostId = tc.PostId AND tc.rn = 1
// WHERE r.ScoreRank <= 3 ORDER BY r.Score DESC, r.CreationDate ASC;
fn q33414(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let qc = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cl = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(hd))
        .window(row_number, |(h, d)| (Reverse(d), h), asc);
    let last = (&cl).filt(|(_, r)| r == 1).map(|((h, _), _)| h);
    let active = owner_user.select(Ident::<User>::new().with((&qc).filt(|n| n > 5)));
    let mut v = drain((&tp).select(active.opt().and(last.opt())));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (m, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(match m {
            Some(u) => ucols(db, u, &["uid", "name"]),
            None => vec![V::Null, V::Null],
        });
        f.extend(match h {
            Some(h) => [harness::fmt::ostr(db.post_history.comment.get(h)), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS rn,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpvoteCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownvoteCount FROM Posts p WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.UpvoteCount, rp.DownvoteCount FROM RankedPosts rp WHERE rp.rn <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserId, pht.Name AS HistoryType, ph.Comment,
//        DENSE_RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS history_rank FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.UpvoteCount, tp.DownvoteCount, COALESCE(pc.CommentCount, 0) AS TotalComments,
//        (SELECT COUNT(*) FROM PostLinks pl WHERE pl.PostId = tp.Id) AS TotalLinks, ph.HistoryType, ph.CreationDate as HistoryDate, ph.Comment AS HistoryComment
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.Id = pc.PostId LEFT JOIN PostHistoryDetails ph ON tp.Id = ph.PostId AND ph.history_rank = 1
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q2919(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let lc = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).opt()).fold(0i64, |n, l| n + l.is_some() as i64);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    let mut v = drain((&vc).and(&cc).and(&lc).and(Ident::<Post>::new().and(&md).select(&at).opt()));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (((u, c), l), h))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(u[0]), V::I(u[1]), V::I(c), V::I(l)]);
        f.extend(match h {
            Some(h) => [V::S(htype_name(db).get(h).unwrap()), V::T(hd.get(h).unwrap()), harness::fmt::ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// Rewritten (rewrites/6732.sql): the ORDER BY moved out of the FinalStats CTE and tie-broken on UserId.
// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u JOIN UserBadgeCounts ub ON u.Id = ub.UserId WHERE u.Reputation > 1000),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId),
// FinalStats AS (SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.BadgeCount, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, ps.PostCount, ps.QuestionCount,
//        ps.AnswerCount, ps.AverageScore FROM TopUsers tu LEFT JOIN PostStats ps ON tu.UserId = ps.OwnerUserId ORDER BY tu.Reputation DESC)
// SELECT UserId, DisplayName, Reputation, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, PostCount, QuestionCount, AnswerCount, AverageScore
// FROM FinalStats WHERE PostCount > 5 ORDER BY Reputation DESC, UserId LIMIT 10;
fn q6732(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ups = user_posts(db);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ub).and((&ups).filt(|a| a[1] > 5))));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), db.user.origid.get(u).unwrap()), 10);
    rows(v.into_iter().map(|(u, (b, a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(P.ViewCount) AS TotalViews, ROW_NUMBER() OVER(PARTITION BY P.OwnerUserId ORDER BY SUM(P.ViewCount) DESC) AS Rank FROM Posts P GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT U.DisplayName, U.Reputation, US.BadgeCount, US.GoldBadges, US.SilverBadges, US.BronzeBadges, PS.QuestionCount, PS.AnswerCount, PS.TotalViews
//     FROM UserStats US JOIN PostStats PS ON US.UserId = PS.OwnerUserId JOIN Users U ON U.Id = US.UserId)
// SELECT *, CASE WHEN Reputation > 1000 THEN 'High Reputation' WHEN Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationTier,
//        CASE WHEN TotalViews >= 10000 THEN 'Popular' ELSE 'Less Popular' END AS Popularity
// FROM CombinedStats WHERE QuestionCount > 0 ORDER BY Reputation DESC, TotalViews DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
fn q3584(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 4], |a, (t, w)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let v = drain((&ub).and((&ps).filt(|a| a[0] > 0)));
    let v = top_n(v, |&(u, (_, a))| (Reverse(db.user.reputation.get(u).unwrap()), a[2] == 0, Reverse(a[3]), u), 20);
    rows(v.into_iter().skip(10).map(|(u, (b, a))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), nullable(a[3], a[2])]);
        f.push(V::S(if rep > 1000 { "High Reputation" } else if rep >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        f.push(V::S(if a[2] > 0 && a[3] >= 10000 { "Popular" } else { "Less Popular" }));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// MostActivePosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, COUNT(C.Id) AS TotalComments, RANK() OVER (ORDER BY COUNT(C.Id) DESC) AS CommentRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title, P.OwnerUserId),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, U.DisplayName AS OwnerDisplayName, P.ViewCount,
//        CASE WHEN P.Score > 0 THEN 'Positive' WHEN P.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS PostSentiment, P.CreationDate,
//        SUM(V.BountyAmount) AS TotalBounty, COUNT(C.Id) AS CommentCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     GROUP BY P.Id, P.Title, P.OwnerUserId, U.DisplayName, P.ViewCount, P.Score, P.CreationDate)
// SELECT PU.ReputationRank, PU.DisplayName AS Poster, PS.Title AS PostTitle, PS.ViewCount, PS.PostSentiment, M.TotalComments, PS.TotalBounty, PS.CommentCount
// FROM PostStatistics PS JOIN RankedUsers PU ON PS.OwnerUserId = PU.UserId JOIN MostActivePosts M ON PS.PostId = M.PostId
// WHERE PS.ViewCount > 100 AND PS.CommentCount > 0 ORDER BY PU.ReputationRank ASC, M.TotalComments DESC;
fn q28654(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    let rr: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().collect();
    let Post { owner_user, view_count, score, .. } = &db.post;
    let posts = || db.post.with(view_count.gt(100)).with(owner_user);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ps = posts().group_by(Ident::<Post>::new()).select(bounty.opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (b, c)| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + c.is_some() as i64]
    });
    let tc = posts().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain((&ps).filt(|a| a[2] > 0).and(&tc).and(owner_user.select(&by_user)));
    v.sort_by_key(|&(_, ((_, t), (_, r)))| (r, Reverse(t)));
    rows(v.into_iter().map(|(p, ((a, t), (u, r)))| {
        let s = score.get(p).unwrap();
        let mut f = vec![V::I(r), user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }), V::I(t), nullable(a[1], a[0]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.Score, P.ViewCount, P.AnswerCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS Rnk
//     FROM Posts P WHERE P.Score > 0),
// PostMetrics AS (SELECT U.DisplayName, SUM(P.Score) AS TotalScore, COUNT(P.Id) AS TotalPosts, AVG(P.ViewCount) AS AvgViewCount, SUM(P.AnswerCount) AS TotalAnswers
//     FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.DisplayName),
// CombinedMetrics AS (SELECT UB.UserId, UB.DisplayName, PM.TotalScore, PM.TotalPosts, PM.AvgViewCount, PM.TotalAnswers, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges
//     FROM UserBadges UB JOIN PostMetrics PM ON UB.DisplayName = PM.DisplayName)
// SELECT CM.DisplayName, CM.TotalScore, CM.TotalPosts, CM.AvgViewCount, CM.TotalAnswers, CM.BadgeCount, CM.GoldBadges, CM.SilverBadges, CM.BronzeBadges, PP.Title AS PopularPostTitle
// FROM CombinedMetrics CM LEFT JOIN PopularPosts PP ON CM.UserId = PP.OwnerUserId AND PP.Rnk = 1 ORDER BY CM.TotalScore DESC, CM.BadgeCount DESC LIMIT 10;
fn q7211(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, score, view_count, answer_count, .. } = &db.post;
    let pm = db
        .post
        .with(owner_user)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()).and(answer_count.opt()))
        .fold([0i64; 6], |a, ((s, w), n)| [a[0] + s, a[1] + 1, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + n.is_some() as i64, a[5] + n.unwrap_or(0)]);
    let w = db.post.with(score.gt(0)).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let pp = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let v = drain((&ub).and((&db.user.display_name).select(&pm)).and(pp.opt()));
    let v = top_n(v, |&(u, ((b, m), _))| (Reverse(m[0]), Reverse(b[0]), u), 10);
    rows(v.into_iter().map(|(u, ((b, m), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(m[0]), V::I(m[1]), avg(m[3], m[2]), nullable(m[5], m[4])];
        f.extend(b.map(V::I));
        f.push(p.map_or(V::Null, |p| harness::fmt::ostr(db.post.title.get(p))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY EXTRACT(YEAR FROM p.CreationDate) ORDER BY p.ViewCount DESC) AS RankByViews,
//        ROW_NUMBER() OVER (PARTITION BY EXTRACT(YEAR FROM p.CreationDate) ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '5 years'),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, RankByViews, RankByScore FROM RankedPosts WHERE RankByViews <= 10 OR RankByScore <= 10),
// PostBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, pb.BadgeCount FROM Users u LEFT JOIN PostBadges pb ON u.Id = pb.UserId
//     WHERE EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = u.Id AND p.PostTypeId = 1) ORDER BY u.Reputation DESC LIMIT 5)
// SELECT trp.Title, trp.CreationDate, trp.ViewCount, trp.Score, trp.OwnerDisplayName, tu.DisplayName AS TopUserDisplayName, tu.Reputation AS TopUserReputation,
//        tu.BadgeCount AS TopUserBadgeCount
// FROM TopRankedPosts trp JOIN TopUsers tu ON trp.OwnerDisplayName = tu.DisplayName ORDER BY trp.CreationDate DESC;
fn q8758(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    let w1 = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -5))))
        .with(owner_user)
        .group_by(creation_date.map(year))
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (_, w))| (w.is_none(), Reverse(w), p), asc);
    let w2 = (&w1).window(row_number, |((p, (s, _)), _)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w2).filt(|((_, a), b)| a <= 10 || b <= 10).map(|(((p, _), _), _)| p).collect();
    let asks: MatSet<Id<User>> = db.post.with(post_type_id.eq(1)).select(owner_user).collect();
    let tu = top_n(drain((&asks).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 5);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let by_name: HashIdx<Str, Id<User>> = (&tu).select(&db.user.display_name).inv().collect();
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&tp).select(owner_user.select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&bc))));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (u, b))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score", "owner"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(if b == 0 { V::Null } else { V::I(b) });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 500 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopUsers AS (SELECT OwnerUserId, COUNT(*) AS PostCount, SUM(Score) AS TotalScore FROM RankedPosts GROUP BY OwnerUserId HAVING COUNT(*) > 5),
// CommentsWithVotes AS (SELECT c.PostId, COUNT(V.Id) AS VoteCount, AVG(c.Score) AS AvgCommentScore FROM Comments c LEFT JOIN Votes V ON c.PostId = V.PostId GROUP BY c.PostId),
// FinalResults AS (SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.AnswerCount, t.PostCount, t.TotalScore, cv.VoteCount, cv.AvgCommentScore
//     FROM RankedPosts r JOIN TopUsers t ON r.OwnerUserId = t.OwnerUserId JOIN CommentsWithVotes cv ON r.PostId = cv.PostId WHERE r.PostRank = 1)
// SELECT f.PostId, f.Title, f.CreationDate, f.Score, f.ViewCount, f.AnswerCount, f.PostCount, f.TotalScore, f.VoteCount, f.AvgCommentScore
// FROM FinalResults f ORDER BY f.TotalScore DESC, f.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
fn q7930(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(500))));
    let tu = rp().group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let w = rp().group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cv = (&first)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(&db.comment.score).and(votes_of(db).opt()))
        .fold([0i64; 3], |a, (s, v)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + s]);
    let v = drain((&cv).filt(|a| a[0] > 0).and(owner_user.select((&tu).filt(|a| a[0] > 5))));
    let v = top_n(v, |&(p, (_, t))| {
        let w = view_count.get(p);
        (Reverse(t[1]), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (c, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(t[0]), V::I(t[1]), V::I(c[1]), avg(c[2], c[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostID, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RN FROM Posts P WHERE P.PostTypeId = 1),
// UserActivity AS (SELECT U.Id AS UserID, U.DisplayName, U.Reputation, COUNT(P.Id) AS TotalQuestions, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounties,
//        SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 9
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentCloseReasons AS (SELECT PH.PostId, PH.PostHistoryTypeId, CR.Name AS CloseReason, PH.CreationDate AS CloseDate,
//        ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS CloseRank
//     FROM PostHistory PH JOIN CloseReasonTypes CR ON CAST(PH.Comment AS INTEGER) = CR.Id WHERE PH.PostHistoryTypeId IN (10, 11))
// SELECT R.Title, R.CreationDate, U.DisplayName AS UserName, U.TotalQuestions, U.TotalBounties, U.TotalScore, R.Score, COALESCE(R.ViewCount, 0) AS PostViewCount,
//        C.CloseReason AS RecentCloseReason, C.CloseDate AS LastClosedDate
// FROM RankedPosts R JOIN UserActivity U ON R.OwnerUserId = U.UserID LEFT JOIN RecentCloseReasons C ON R.PostID = C.PostId AND C.CloseRank = 1
// WHERE R.RN = 1 ORDER BY U.TotalScore DESC, R.CreationDate DESC LIMIT 50;
fn q2080(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(score.and(bounty.opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((s, b)) => [a[0] + 1, a[1] + b.flatten().unwrap_or(0), a[2] + s],
            None => a,
        });
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cl = db
        .post_history
        .with(post_history_type_id.in_v(vec![10, 11]))
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(hd).and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .window(row_number, |((h, d), _)| (Reverse(d), h), asc);
    let last = (&cl).filt(|(_, r)| r == 1).map(|(((h, _), r), _)| (h, r));
    let v = drain((&first).select(owner_user.select(Ident::<User>::new().and(&ua))).and(last.opt()));
    let v = top_n(v, |&(p, ((_, a), _))| (a[0] == 0, Reverse(a[2]), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((u, a), c))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), nullable(a[2], a[0]), V::I(score.get(p).unwrap()), V::I(view_count.get(p).unwrap_or(0))]);
        f.extend(match c {
            Some((h, r)) => [V::S(r), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS Author, p.CreationDate, p.Score, p.ViewCount,
//        array_length(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><'), 1) AS TagCount, COUNT(c.Id) AS CommentCount, COUNT(pa.Id) AS AnswerCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts pa ON p.Id = pa.ParentId AND pa.PostTypeId = 1
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName, p.CreationDate, p.Score, p.ViewCount),
// PopularTags AS (SELECT tag, COUNT(*) AS TagFrequency FROM (SELECT unnest(string_to_array(substring(Tags, 2, length(Tags) - 2), '><')) AS tag FROM Posts WHERE PostTypeId = 1) AS TagList
//     GROUP BY tag ORDER BY TagFrequency DESC LIMIT 10),
// BenchmarkData AS (SELECT pd.PostId, pd.Title, pd.Body, pd.Author, pd.CreationDate, pd.Score, pd.ViewCount, pd.TagCount, pd.CommentCount, pd.AnswerCount, pt.TagFrequency
//     FROM PostDetails pd JOIN PopularTags pt ON pd.Tags LIKE CONCAT('%', pt.tag, '%'))
// SELECT b.*, CASE WHEN b.TagCount > 5 THEN 'High Tag Count' WHEN b.AnswerCount > 10 THEN 'Highly Answered' ELSE 'Standard Post' END AS PostCategory
// FROM BenchmarkData b ORDER BY b.ViewCount DESC, b.Score DESC LIMIT 20;
fn q29819(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, score, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let freq = qs().select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let top = top_n(drain(&freq), |&(t, n)| (Reverse(n), t), 10);
    let tv = rel(top);
    let pt: HashIdx<Str, (Str, i64)> = (&tv).map(|(t, _)| t).inv().select(&tv).collect();
    let elems: MatSet<Str> = qs().select(tags_str.flat_map(tag_list)).collect();
    let contains: HashIdx<Str, (Str, i64)> = (&elems).select_where(&pt, |e: Str, t: Str| e.contains(t)).collect();
    let hits: MatSet<(Id<Post>, (Str, i64))> = qs().select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).select(&contains))).collect();
    let hit: MatSet<Id<Post>> = (&hits).map(|(p, _)| p).collect();
    let asked = Ident::<Post>::new().with(post_type_id.eq(1));
    let pd = (&hit)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).select(asked).opt()))
        .fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    type H = (Id<Post>, (Str, i64));
    let v = drain((&hits).select(Same::<H>::new().and(Same::<H>::new().map(|(p, _)| p).select(&pd))));
    let v = top_n(v, |&(_, ((p, (t, _)), _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()), p, t)
    }, 20);
    rows(v.into_iter().map(|(_, ((p, (_, n)), a))| {
        let tc = tags_str.get(p).map(|t| tag_list(t).count() as i64);
        let mut f = post_fields(db, p, &["id", "title", "body", "owner", "created", "score", "views"]);
        f.extend([harness::fmt::oint(tc), V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.push(V::S(if tc.map_or(false, |c| c > 5) { "High Tag Count" } else if a[1] > 10 { "Highly Answered" } else { "Standard Post" }));
        row(f)
    }))
}

// WITH RECURSIVE UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(p.Score) AS TotalScore, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// UserActivity AS (SELECT us.UserId, us.DisplayName, us.PostCount, us.QuestionsCount, us.AnswersCount, us.TotalScore, us.GoldBadges, us.SilverBadges, us.BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY us.TotalScore DESC) AS Rank, (SELECT COUNT(*) FROM Users) AS TotalUsers FROM UserPostStats us),
// RankedUserActivity AS (SELECT ua.*, CASE WHEN ua.Rank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType FROM UserActivity ua)
// SELECT rua.DisplayName, rua.PostCount, rua.QuestionsCount, rua.AnswersCount, rua.TotalScore, rua.GoldBadges, rua.SilverBadges, rua.BronzeBadges, rua.ContributorType,
//        CAST(100.0 * rua.Rank / rua.TotalUsers AS DECIMAL(5, 2)) AS ContributionPercentage
// FROM RankedUserActivity rua WHERE rua.TotalScore > 0 ORDER BY rua.TotalScore DESC OFFSET 0 ROWS FETCH NEXT 20 ROWS ONLY;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q30151(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score)).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (n, t, s) = p.map_or((0, 0, 0), |(t, s)| (1, t, s));
            [a[0] + n, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64]
        });
    let users = db.user.select(Ident::<User>::new()).fold_flat(0i64, |n, _| n + 1);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(row_number, |(u, a)| (a[0] == 0, Reverse(a[3]), u), asc);
    let v = top_n(drain((&w).filt(|((_, a), _)| a[0] > 0 && a[3] > 0)), |&(_, (_, r))| r, 20);
    rows(v.into_iter().map(|(_, ((u, a), r))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a[..7].iter().map(|&x| V::I(x)));
        f.push(V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" }));
        f.push(V::F((100.0 * r as f64 / users as f64 * 100.0).round() / 100.0));
        row(f)
    }))
}

// WITH RECURSIVE PostChain AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, 1 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT a.Id AS PostId, a.Title, a.OwnerUserId, a.CreationDate, a.Score, pc.Level + 1 FROM Posts a INNER JOIN Posts p ON a.ParentId = p.Id
//     INNER JOIN PostChain pc ON p.Id = pc.PostId)
// SELECT pc.PostId, pc.Title, u.DisplayName AS OwnerDisplayName, pc.CreationDate, pc.Score AS QuestionScore, COALESCE(SUM(sub.UpVoteCount), 0) AS TotalUpVotes,
//        COALESCE(SUM(sub.DownVoteCount), 0) AS TotalDownVotes, CASE WHEN nc.Clicks IS NOT NULL THEN 'Clicked' ELSE 'Not Clicked' END AS NotificationClickStatus
// FROM PostChain pc LEFT JOIN Users u ON pc.OwnerUserId = u.Id
// LEFT JOIN (SELECT PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVoteCount
//     FROM Votes v INNER JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId) sub ON pc.PostId = sub.PostId
// LEFT JOIN (SELECT PostId, COUNT(*) AS Clicks FROM PostLinks pl WHERE pl.LinkTypeId = 1 GROUP BY PostId) nc ON pc.PostId = nc.PostId
// WHERE pc.Level = 1 GROUP BY pc.PostId, pc.Title, u.DisplayName, pc.CreationDate, pc.Score, nc.Clicks ORDER BY TotalUpVotes DESC, pc.Score DESC LIMIT 100;
//
// Only the base case (Level = 1) is read, so the recursion is never needed.
fn q30419(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let clicked: MatSet<Id<Post>> = db.post_link.with((&db.post_link.link_type_id).eq(1)).select(&db.post_link.post).collect();
    let s = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(vtype_name(db)).opt())
        .fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    let v = drain((&s).and(Ident::<Post>::new().with(&clicked).opt()));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[0]), Reverse(score.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if c.is_some() { "Clicked" } else { "Not Clicked" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY SUBSTRING(P.Tags, 2, LENGTH(P.Tags) - 2) ORDER BY P.CreationDate DESC) AS Rank, P.Tags
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TagStatistics AS (SELECT SUBSTRING(P.Tags, 2, LENGTH(P.Tags) - 2) AS Tag, COUNT(*) AS TagCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AverageViews
//     FROM Posts P WHERE P.PostTypeId = 1 GROUP BY SUBSTRING(P.Tags, 2, LENGTH(P.Tags) - 2)),
// ClosedPostReasons AS (SELECT PH.PostId, COUNT(*) AS CloseCount, MAX(PH.CreationDate) AS LastCloseDate, CT.Name AS CloseReason
//     FROM PostHistory PH JOIN CloseReasonTypes CT ON PH.Comment = CAST(CT.Id AS VARCHAR) WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId, CT.Name)
// SELECT RP.PostId, RP.Title, RP.Body, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, TS.Tag, TS.TagCount, TS.TotalScore, TS.AverageViews,
//        CPR.CloseCount, CPR.LastCloseDate, CPR.CloseReason
// FROM RankedPosts RP LEFT JOIN TagStatistics TS ON RP.Tags LIKE '%' || TS.Tag || '%' LEFT JOIN ClosedPostReasons CPR ON RP.PostId = CPR.PostId
// WHERE RP.Rank = 1 ORDER BY TS.TagCount DESC, RP.CreationDate DESC;
//
// TS.Tag is a whole inner tag string (it can span several tags), so the LIKE is a raw substring test on the Tags text.
fn q26341(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, score, view_count, .. } = &db.post;
    let inner = |t: Str| -> Str { &t[1..t.len() - 1] };
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .with(owner_user)
        .group_by(tags_str.map(inner).opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let tsg = db
        .post
        .with(post_type_id.eq(1))
        .group_by(tags_str.map(inner))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let tv = rel(drain(&tsg));
    let tsk: HashIdx<Str, (Str, [i64; 4])> = (&tv).map(|(t, _)| t).inv().select(&tv).collect();
    let full: MatSet<Str> = (&rp).select(tags_str).collect();
    let like: HashIdx<Str, (Str, [i64; 4])> = (&full).select_where(&tsk, |f: Str, t: Str| f.contains(t)).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let exact = |s: Str| s.parse::<i64>().ok().filter(|i| i.to_string() == s);
    let cpr = db
        .post_history
        .with(post_history_type_id.in_v(vec![10, 11]))
        .group_by(post.and(comment.flat_map(exact).select(&reason)))
        .select(hd)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let cv = rel(drain(&cpr));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, Str), (i64, i64))> = (&cv).map(|((p, _), _)| p).inv().select(&cv).collect();
    let mut v = drain((&rp).select(tags_str.select(&like).opt().and((&by_post).opt())));
    v.sort_by_key(|&(p, (t, _))| (t.map_or(true, |_| false), Reverse(t.map(|(_, a)| a[0])), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (t, c))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "score", "views", "owner"]);
        f.extend(match t {
            Some((t, a)) => [V::S(t), V::I(a[0]), V::I(a[1]), avg(a[3], a[2])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some(((_, r), (n, d))) => [V::I(n), V::T(d), V::S(r)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.PostRank <= 10),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, pht.Name AS HistoryType, ph.CreationDate AS HistoryDate, ph.UserDisplayName, ph.Comment
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.PostId IN (SELECT PostId FROM TopPosts))
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes, phd.HistoryType, phd.HistoryDate,
//        phd.UserDisplayName AS EditorDisplayName, phd.Comment AS EditComment
// FROM TopPosts tp LEFT JOIN PostHistoryDetails phd ON tp.PostId = phd.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// PostRank reads only Score, so the top questions are picked first and the comment x vote product is driven for those alone.
fn q7222(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let w = whole(db.post.iq()).select(Ident::<Post>::new().with(post_type_id.eq(1)).and(score)).window(rank, |(_, s)| s, desc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&s).and(history_of(db).opt()));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(h) => [
                V::S(htype_name(db).get(h).unwrap()),
                V::T(db.post_history.creation_date.get(h).unwrap()),
                harness::fmt::ostr(db.post_history.user_display_name.get(h)),
                harness::fmt::ostr(db.post_history.comment.get(h)),
            ],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        RANK() OVER (ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS UserRank
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName HAVING COUNT(p.Id) > 10)
// SELECT r.PostId, r.Title, r.Body, r.CreationDate, r.ViewCount, r.CommentCount, r.UpVotes, r.DownVotes, tu.UserId, tu.DisplayName AS UserName, tu.TotalUpVotes, tu.TotalDownVotes
// FROM RankedPosts r JOIN TopUsers tu ON r.OwnerUserId = tu.UserId WHERE r.PostRank <= 5 ORDER BY tu.UserRank, r.CreationDate DESC;
//
// PostRank reads only base columns, so each owner's five newest questions are picked first and the comment x vote product is driven for those alone.
fn q27786(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&s).and(owner_user.select(Ident::<User>::new().and((&tu).filt(|a| a[0] > 10)))));
    v.sort_by_key(|&(p, (_, (_, t)))| (Reverse(t[1] - t[2]), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (a, (u, t)))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "views"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(t[1]), V::I(t[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.ClosedDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS OwnerRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopOwnerPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, ClosedDate, OwnerDisplayName FROM RankedPosts WHERE OwnerRank = 1),
// DetailedPostStats AS (SELECT t.PostId, t.Title, t.CreationDate, t.Score, t.ViewCount, t.AnswerCount, t.CommentCount, t.ClosedDate, t.OwnerDisplayName, COUNT(c.Id) AS TotalComments,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM TopOwnerPosts t LEFT JOIN Comments c ON t.PostId = c.PostId LEFT JOIN Votes v ON t.PostId = v.PostId
//     GROUP BY t.PostId, t.Title, t.CreationDate, t.Score, t.ViewCount, t.AnswerCount, t.CommentCount, t.ClosedDate, t.OwnerDisplayName)
// SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, ClosedDate, OwnerDisplayName, TotalComments, TotalUpvotes, TotalDownvotes
// FROM DetailedPostStats ORDER BY Score DESC, ViewCount DESC LIMIT 10;
fn q9690(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "closed", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// QuestionsStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS QuestionCount, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AverageViews, MAX(p.CreationDate) AS LastQuestionDate
//     FROM Posts p WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, qs.QuestionCount, qs.TotalScore, qs.AverageViews, qs.LastQuestionDate
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN QuestionsStatistics qs ON u.Id = qs.OwnerUserId),
// RankedUsers AS (SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, QuestionCount, TotalScore, AverageViews, LastQuestionDate,
//        RANK() OVER (ORDER BY QuestionCount DESC, TotalScore DESC) AS Rank FROM UserActivity)
// SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, QuestionCount, TotalScore, AverageViews, LastQuestionDate, Rank
// FROM RankedUsers WHERE Rank <= 10 ORDER BY Rank;
fn q6147(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, owner_user, score, view_count, creation_date, .. } = &db.post;
    let qs = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(score.and(view_count.opt()).and(creation_date))
        .fold([0, 0, 0, 0, i64::MIN], |a, ((s, w), d)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4].max(d)]);
    let w = whole(db.user.iq())
        .select(Ident::<User>::new().and((&ub).and((&qs).opt())))
        .window(rank, |(_, (_, q))| q.map_or((true, Reverse(0), Reverse(0)), |a| (false, Reverse(a[0]), Reverse(a[1]))), asc);
    let mut v: Vec<_> = drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(_, r)| r);
    rows(v.into_iter().map(|((u, (b, q)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(match q {
            Some(a) => [V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::T(a[4])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.ViewCount > 100),
// CommentedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Body, rp.Tags, rp.OwnerDisplayName, pc.CommentCount,
//        COALESCE(pc.AvgCommentLength, 0) AS AvgCommentLength
//     FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount, AVG(LENGTH(Text)) AS AvgCommentLength FROM Comments GROUP BY PostId) pc ON rp.PostId = pc.PostId
//     WHERE rp.TagRank = 1),
// FinalResults AS (SELECT cp.PostId, cp.Title, cp.CreationDate, cp.Score, cp.ViewCount, cp.Body, cp.Tags, cp.OwnerDisplayName, cp.CommentCount, cp.AvgCommentLength,
//        PH.Comment AS ClosedReason FROM CommentedPosts cp LEFT JOIN PostHistory PH ON cp.PostId = PH.PostId WHERE PH.PostHistoryTypeId = 10)
// SELECT PostId, Title, CreationDate, Score, ViewCount, Body, Tags, OwnerDisplayName, CommentCount, AvgCommentLength, ClosedReason
// FROM FinalResults ORDER BY Score DESC, ViewCount DESC LIMIT 50;
fn q28878(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, tags_str, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(view_count.gt(100)))
        .with(owner_user)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pc = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(&db.comment.text).opt())
        .fold([0i64; 2], |a, t| match t {
            Some(t) => [a[0] + 1, a[1] + t.chars().count() as i64],
            None => a,
        });
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&pc).and(closes));
    let v = top_n(v, |&(p, (_, h))| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p, h), 50);
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "body", "tags", "owner"]);
        f.extend([if a[0] == 0 { V::Null } else { V::I(a[0]) }, if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }, harness::fmt::ostr(db.post_history.comment.get(h))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerName,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= '2023-01-01'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerName FROM RankedPosts WHERE Rank <= 10),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.OwnerName, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, COUNT(c.Id) AS TotalComments,
//        SUM(CASE WHEN v.VoteTypeId IN (2, 4) THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.OwnerName, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount)
// SELECT pd.PostId, pd.Title, pd.OwnerName, pd.CreationDate, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.TotalComments, pd.UpVotes, pd.DownVotes,
//        CASE WHEN pd.UpVotes + pd.DownVotes > 0 THEN ROUND((pd.UpVotes * 1.0 / (pd.UpVotes + pd.DownVotes)) * 100, 2) ELSE 0 END AS ApprovalRatio
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
fn q9683(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = whole(db.post.iq())
        .select(Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)))).with(owner_user).and(score.and(view_count.opt())))
        .window(dense_rank, |(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + matches!(t, Some(2 | 4)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain(&s);
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views", "answers", "comments"]);
        f.extend(a.map(V::I));
        f.push(V::F(if a[1] + a[2] > 0 { (a[1] as f64 / (a[1] + a[2]) as f64 * 100.0 * 100.0).round() / 100.0 } else { 0.0 }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Body, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Location, (SELECT COUNT(*) FROM Posts WHERE OwnerUserId = u.Id AND PostTypeId = 1) AS QuestionCount,
//        (SELECT COUNT(*) FROM Badges WHERE UserId = u.Id) AS BadgeCount FROM Users u),
// RecentPostActivity AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 days' GROUP BY p.Id, p.OwnerUserId),
// CombinedResults AS (SELECT ur.DisplayName, ur.Reputation, ur.Location, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rpa.CommentCount, rpa.VoteCount, ur.QuestionCount, ur.BadgeCount
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId JOIN RecentPostActivity rpa ON rp.PostId = rpa.PostId WHERE rp.RN = 1)
// SELECT DisplayName, Reputation, Location, Title, CreationDate, ViewCount, Score, CommentCount, VoteCount, QuestionCount, BadgeCount
// FROM CombinedResults ORDER BY Reputation DESC, ViewCount DESC LIMIT 10;
fn q27949(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let recent = (&tp).with(creation_date.ge(add_days(current_date(), -30)));
    let rpa = recent
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&rpa).and(owner_user.select(Ident::<User>::new().and(&qc).and(&bc))));
    let v = top_n(v, |&(p, (_, ((u, _), _)))| {
        let w = view_count.get(p);
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (a, ((u, q), b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(harness::fmt::ostr(db.user.location.get(u)));
        f.extend(post_fields(db, p, &["title", "created", "views", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(q), V::I(b)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.ViewCount, p.Score HAVING COUNT(c.Id) > 5),
// RecentHistory AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, ph.Comment, p.Title AS PostTitle, RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS VersionRank
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month')
// SELECT ur.UserId, ur.DisplayName, ur.Reputation, pp.PostId, pp.Title AS PostTitle, pp.ViewCount, pp.Score, pp.CommentCount, rh.CreationDate AS RecentChangeDate,
//        rh.Comment AS RecentComment, rh.VersionRank
// FROM UserReputation ur JOIN PopularPosts pp ON ur.UserId IN (SELECT OwnerUserId FROM Posts WHERE Id IN (SELECT PostId FROM PostHistory WHERE PostHistoryTypeId IN (10, 11)))
// LEFT JOIN RecentHistory rh ON pp.PostId = rh.PostId AND rh.VersionRank = 1
// WHERE ur.Reputation > 1000 ORDER BY pp.Score DESC, ur.Reputation DESC LIMIT 100;
//
// The ON clause names only ur, so users and popular posts are crossed.
fn q2161(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).in_v(vec![10, 11])).select(&db.post_history.post).collect();
    let ur: MatSet<Id<User>> = (&closed).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).collect();
    let pp = db
        .post
        .with(creation_date.ge(add_days(t0, -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt())
        .fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let recent = || db.post_history.with(hd.ge(add_months(t0, -1)));
    let md = recent().group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = recent().select(post.and(hd)).inv().collect();
    let pr = (&pp).filt(|n| n > 5).and(Ident::<Post>::new().and(&md).select(&at).opt());
    let mut v = Vec::new();
    (&ur).cross(pr).drive(|(u, p), (_, (n, h))| v.push((u, p, n, h)));
    let v = top_n(v, |&(u, p, _, h)| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), u, p, h), 100);
    rows(v.into_iter().map(|(u, p, n, h)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
        f.push(V::I(n));
        f.extend(match h {
            Some(h) => [V::T(hd.get(h).unwrap()), harness::fmt::ostr(db.post_history.comment.get(h)), V::I(1)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.OwnerUserId AS UserId, u.DisplayName, u.Reputation
//     FROM RankedPosts rp INNER JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.PostRank <= 5),
// PostStatistics AS (SELECT tp.UserId, tp.DisplayName, SUM(tp.Score) AS TotalScore, COUNT(tp.PostId) AS PostCount, AVG(tp.ViewCount) AS AvgViewCount
//     FROM TopPosts tp GROUP BY tp.UserId, tp.DisplayName),
// TopVoters AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount FROM Votes v INNER JOIN Posts p ON v.PostId = p.Id
//     WHERE p.PostTypeId = 1 AND v.VoteTypeId IN (2, 3) /* Only upvotes and downvotes */ GROUP BY v.UserId),
// CombinedStats AS (SELECT ps.UserId, ps.DisplayName, ps.TotalScore, ps.PostCount, ps.AvgViewCount, COALESCE(tv.VoteCount, 0) AS VoteCount
//     FROM PostStatistics ps LEFT JOIN TopVoters tv ON ps.UserId = tv.UserId)
// SELECT cs.DisplayName, cs.TotalScore, cs.PostCount, cs.AvgViewCount, cs.VoteCount FROM CombinedStats cs WHERE cs.TotalScore > 1000
// ORDER BY cs.TotalScore DESC, cs.PostCount DESC;
fn q34265(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let ps = (&tp).group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + s, a[1] + 1, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let Vote { user, post, vote_type_id, .. } = &db.vote;
    let tv = db.vote.with(vote_type_id.is_in([2, 3])).with(post.select(Ident::<Post>::new().with(post_type_id.eq(1)))).group_by(user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&ps).filt(|a| a[0] > 1000).and((&tv).opt()));
    let v = top_n(v, |&(_, (a, _))| (Reverse(a[0]), Reverse(a[1])), 0);
    rows(v.into_iter().map(|(u, (a, n))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(n.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.Tags, p.CreationDate, p.ViewCount, u.DisplayName),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Tags, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount,
//        CASE WHEN rp.ViewCount > 1000 THEN 'High Engagement' WHEN rp.ViewCount BETWEEN 500 AND 1000 THEN 'Medium Engagement' ELSE 'Low Engagement' END AS EngagementLevel,
//        STRING_AGG(t.TagName, ', ') AS TagsList
//     FROM RankedPosts rp LEFT JOIN Tags t ON POSITION(CONCAT('<', t.TagName, '>') IN rp.Tags) > 0
//     GROUP BY rp.PostId, rp.Title, rp.Tags, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount)
// SELECT fp.PostId, fp.Title, fp.TagsList, fp.CreationDate, fp.ViewCount, fp.OwnerDisplayName, fp.CommentCount, fp.UpvoteCount, fp.DownvoteCount, fp.EngagementLevel
// FROM FilteredPosts fp WHERE fp.CommentCount > 5 OR fp.UpvoteCount > 10 ORDER BY fp.ViewCount DESC, fp.CreationDate DESC;
//
// STRING_AGG has no ORDER BY; DuckDB's is in Tags.Id order (every row of the oracle), which is the order kept here.
fn q28032(db: &'static So) -> String {
    type R = (Id<Post>, Id<Tag>);
    let bt = bracketed(db, false);
    let by_post: HashIdx<Id<Post>, R> = (&bt).map(|(p, _): R| p).inv().collect();
    let s = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let names = db.post.group_by(Ident::<Post>::new()).select((&by_post).map(|(_, t): R| t).opt()).buf_fold(|it| {
        let mut t: Vec<Id<Tag>> = it.into_iter().flatten().collect();
        t.sort_unstable();
        let n: Vec<Str> = t.into_iter().map(|t| db.tag.tag_name.get(t).unwrap()).collect();
        if n.is_empty() { None } else { Some(&*Box::leak(n.join(", ").into_boxed_str())) }
    });
    let v = drain((&s).filt(|a| a[0] > 5 || a[1] > 10).and(&names));
    rows(v.into_iter().map(|(p, (a, n))| {
        let w = db.post.view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(ostr(n));
        f.extend(post_fields(db, p, &["created", "views", "owner"]));
        f.extend(a.map(V::I));
        f.push(V::S(match w {
            Some(w) if w > 1000 => "High Engagement",
            Some(w) if w >= 500 => "Medium Engagement",
            _ => "Low Engagement",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, pt.Name AS PostType, COUNT(a.Id) AS AnswerCount, COUNT(c.Id) AS CommentCount,
//        SUM(v.BountyAmount) AS TotalBounties, RANK() OVER (PARTITION BY pt.Name ORDER BY COUNT(a.Id) DESC, COUNT(c.Id) DESC) AS RankByEngagement
//     FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2 LEFT JOIN Comments c ON c.PostId = p.Id
//     LEFT JOIN Votes v ON v.PostId = p.Id AND v.VoteTypeId = 8 WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '365 days'
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, pt.Name),
// TopEngagedPosts AS (SELECT PostId, Title, Body, CreationDate, PostType, AnswerCount, CommentCount, TotalBounties FROM RankedPosts WHERE RankByEngagement <= 5),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(c.Id) AS TotalComments, SUM(v.BountyAmount) AS TotalBounties
//     FROM Users u LEFT JOIN Comments c ON c.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName)
// SELECT tep.Title, tep.PostType, tep.AnswerCount, tep.CommentCount, tep.TotalBounties, ue.DisplayName AS EngagingUser, ue.TotalComments AS UserCommentCount,
//        ue.TotalBounties AS UserTotalBounties
// FROM TopEngagedPosts tep JOIN UserEngagement ue ON ue.TotalComments > 0 OR ue.TotalBounties > 0 ORDER BY tep.TotalBounties DESC, tep.AnswerCount DESC, tep.CommentCount DESC;
//
// The ON clause names only ue, so the top posts and the engaged users are crossed.
fn q26175(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = || Ident::<Vote>::new().with(vote_type_id.eq(8)).select(bounty_amount.opt());
    let recent = || db.post.with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -365)));
    let rp = recent()
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(comments_of(db).opt()).and(votes_of(db).select(bounty()).opt()))
        .fold([0i64; 4], |a, ((x, c), b)| {
            let b = b.flatten();
            [a[0] + x.is_some() as i64, a[1] + c.is_some() as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
        });
    let w = recent()
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and((&rp).and(ptype_name(db))))
        .window(rank, |(_, (a, _))| (Reverse(a[0]), Reverse(a[1])), asc);
    let tep: MatSet<(Id<Post>, ([i64; 4], Str))> = (&w).filt(|(_, r)| r <= 5).map(|(x, _)| x).collect();
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).select(bounty()).opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let ue: HashIdx<Id<User>, [i64; 3]> = (&ue).filt(|a| a[0] > 0 || (a[1] > 0 && a[2] > 0)).collect();
    let mut rs = Vec::new();
    (&tep).cross(&ue).drive(|(_, u), ((p, (a, t)), b)| {
        rs.push(row(vec![harness::fmt::ostr(db.post.title.get(p)), V::S(t), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), user_col(db, u, "name"), V::I(b[0]), nullable(b[2], b[1])]));
    });
    rows(rs)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount,
//        u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY substring(Tags, 2, length(Tags)-2) ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// LatestPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank = 1),
// PostStatistics AS (SELECT lp.PostId, lp.Title, lp.Tags, lp.CreationDate, lp.ViewCount, lp.Score, lp.AnswerCount, lp.CommentCount, lp.FavoriteCount, COUNT(c.Id) AS CommentCountTotal,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM LatestPosts lp LEFT JOIN Comments c ON lp.PostId = c.PostId LEFT JOIN Votes v ON lp.PostId = v.PostId
//     GROUP BY lp.PostId, lp.Title, lp.Tags, lp.CreationDate, lp.ViewCount, lp.Score, lp.AnswerCount, lp.CommentCount, lp.FavoriteCount)
// SELECT ps.PostId, ps.Title, ps.Tags, ps.CreationDate, ps.ViewCount, ps.Score, ps.AnswerCount, ps.CommentCount, ps.FavoriteCount, ps.CommentCountTotal, ps.UpVotes, ps.DownVotes,
//        CASE WHEN ps.Score >= 10 AND ps.CommentCountTotal >= 5 THEN 'Hot' WHEN ps.Score >= 5 THEN 'Trending' ELSE 'Regular' END AS Popularity
// FROM PostStatistics ps WHERE ps.ViewCount > 100 ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q27950(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, view_count, score, .. } = &db.post;
    let inner = |t: Str| -> Str { &t[1..t.len() - 1] };
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(tags_str.map(inner).opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let lp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let s = (&lp)
        .with(view_count.gt(100))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "tags", "created", "views", "score", "answers", "comments", "favorites"]);
        f.extend(a.map(V::I));
        f.push(V::S(if sc >= 10 && a[0] >= 5 { "Hot" } else if sc >= 5 { "Trending" } else { "Regular" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId)
// SELECT u.DisplayName, rp.Title, rp.CreationDate, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.UpVotes IS NULL AND rp.DownVotes IS NULL THEN 'No votes yet'
//             ELSE CONCAT_WS(' ', COALESCE(CONCAT('Upvotes:', rp.UpVotes), 'No upvotes'), COALESCE(CONCAT('Downvotes:', rp.DownVotes), 'No downvotes')) END AS VoteSummary,
//        COALESCE(b.Name, 'No badge') AS BadgeName,
//        CASE WHEN (rp.UpVotes - rp.DownVotes) > 0 THEN 'Positive Engagement' WHEN (rp.UpVotes - rp.DownVotes) < 0 THEN 'Negative Engagement' ELSE 'Neutral Engagement' END AS EngagementStatus,
//        CASE WHEN rl.PostId IS NOT NULL THEN 'Has Related Post' ELSE 'No Related Post' END AS RelatedPostStatus
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Class = 1
// LEFT JOIN PostLinks rl ON rp.PostId = rl.PostId AND rl.LinkTypeId = 1 WHERE rp.RN = 1 ORDER BY rp.ViewCount DESC LIMIT 10;
//
// RN reads only base columns, so each owner's newest question is picked first and the comment x vote product is driven for those alone.
fn q20338(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, owner_user, creation_date, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let dup = links_of(db).select(Ident::<PostLink>::new().with((&db.post_link.link_type_id).eq(1)));
    let v = drain((&s).and(owner_user.select(Ident::<User>::new().and(gold.opt()))).and(dup.opt()));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(p, ((a, (u, b)), l))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend(a.map(V::I));
        f.push(V::Owned(format!("Upvotes:{} Downvotes:{}", a[1], a[2])));
        f.push(V::S(b.map_or("No badge", |b| db.badge.name.get(b).unwrap())));
        f.push(V::S(if a[1] - a[2] > 0 { "Positive Engagement" } else if a[1] - a[2] < 0 { "Negative Engagement" } else { "Neutral Engagement" }));
        f.push(V::S(if l.is_some() { "Has Related Post" } else { "No Related Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId AND b.Class = 1 GROUP BY u.Id, u.DisplayName),
// TopPostWithBadges AS (SELECT r.PostId, r.Title, r.Score, r.CreationDate, u.DisplayName AS OwnerName, ub.BadgeCount,
//        CASE WHEN r.Score > 100 THEN 'Highly Scored' WHEN r.Score > 50 THEN 'Moderately Scored' ELSE 'Low Scores' END AS ScoreCategory
//     FROM RankedPosts r INNER JOIN Users u ON r.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE r.Rank <= 10),
// PostActivity AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id)
// SELECT tp.PostId, tp.Title, tp.Score, tp.CreationDate, tp.OwnerName, tp.BadgeCount, pa.CommentCount, pa.Upvotes, pa.Downvotes, tp.ScoreCategory
// FROM TopPostWithBadges tp JOIN PostActivity pa ON tp.PostId = pa.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// PostActivity is joined only to the top ten, so its comment x vote product is driven for those alone.
fn q30714(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let ub = db.user.group_by(Ident::<User>::new()).select(gold.opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&s).and(owner_user.select(&ub)));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (a, b))| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.push(V::I(b));
        f.extend(a.map(V::I));
        f.push(V::S(if sc > 100 { "Highly Scored" } else if sc > 50 { "Moderately Scored" } else { "Low Scores" }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS TotalScore,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN p.ViewCount ELSE 0 END) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TagUsage AS (SELECT t.TagName, COUNT(p.Id) AS UsageCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionUsageCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerUsageCount FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, TotalScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
//     FROM UserPostStats WHERE TotalPosts > 0),
// TopTags AS (SELECT TagName, UsageCount, QuestionUsageCount, AnswerUsageCount, RANK() OVER (ORDER BY UsageCount DESC) AS TagRank FROM TagUsage WHERE UsageCount > 0)
// SELECT u.UserId, u.DisplayName, u.TotalPosts, u.QuestionCount, u.AnswerCount, u.TotalScore, u.TotalViews, t.TagName, t.UsageCount, t.QuestionUsageCount, t.AnswerUsageCount
// FROM TopUsers u JOIN TopTags t ON u.QuestionCount > 10 AND t.UsageCount > 10 WHERE u.ScoreRank <= 10 AND t.TagRank <= 10 ORDER BY u.TotalScore DESC, t.UsageCount DESC;
//
// The ON clause names each side separately, so the ranked users and tags are filtered and crossed.
fn q29036(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), w)) => {
                let q = t == 1;
                let w = if q { w } else { Some(0) };
                [a[0] + 1, a[1] + q as i64, a[2] + (t == 2) as i64, a[3] + if q { s } else { 0 }, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
            }
            None => a,
        });
    let wu = whole(db.user.iq()).select(Ident::<User>::new().and((&ups).filt(|a| a[0] > 0))).window(rank, |(_, a)| Reverse(a[3]), asc);
    let tu = (&wu).filt(|((_, a), r)| r <= 10 && a[1] > 10).map(|(x, _)| x);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tg = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select(post_type_id).opt())
        .fold([0i64; 3], |a, t| match t {
            Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
            None => a,
        });
    let names: MatSet<Str> = (&db.tag.tag_name).collect();
    let wt = whole(&names).select(Same::<Str>::new().and((&tg).filt(|a| a[0] > 0))).window(rank, |(_, b)| Reverse(b[0]), asc);
    let tt = (&wt).filt(|((_, b), r)| r <= 10 && b[0] > 10).map(|(x, _)| x);
    let mut v = Vec::new();
    tu.cross(tt).drive(|_, (u, t)| v.push((u, t)));
    v.sort_by_key(|&((_, a), (_, b))| (Reverse(a[3]), Reverse(b[0])));
    rows(v.into_iter().map(|((u, a), (t, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4])]);
        f.extend([V::S(t), V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COALESCE(P.PostCount, 0) AS PostCount, COALESCE(C.CommentCount, 0) AS CommentCount,
//        COALESCE(V.VoteCount, 0) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY U.CreationDate DESC) AS rn
//     FROM Users U LEFT JOIN (SELECT OwnerUserId, COUNT(Id) AS PostCount FROM Posts GROUP BY OwnerUserId) P ON U.Id = P.OwnerUserId
//     LEFT JOIN (SELECT UserId, COUNT(Id) AS CommentCount FROM Comments GROUP BY UserId) C ON U.Id = C.UserId
//     LEFT JOIN (SELECT UserId, COUNT(Id) AS VoteCount FROM Votes GROUP BY UserId) V ON U.Id = V.UserId WHERE U.Reputation > 100),
// RecentUserActivity AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserActivity WHERE rn = 1),
// MostCommentedPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, C.CommentCount, ROW_NUMBER() OVER (ORDER BY C.CommentCount DESC) AS rn
//     FROM Posts P JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId WHERE P.PostTypeId = 1)
// SELECT U.DisplayName, U.Reputation, U.PostCount, U.CommentCount, P.Title, P.CommentCount AS PostCommentCount
// FROM RecentUserActivity U LEFT JOIN MostCommentedPosts P ON U.Rank = 1 WHERE U.Rank <= 10 ORDER BY U.Reputation DESC, P.CommentCount DESC;
//
// rn partitions by the user's own id, so it is 1 for every user. The ON clause names only U: it is a join of U.Rank against the constant 1.
fn q1119(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new());
    let pc = users().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = users().select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(db.user.iq())
        .select(Ident::<User>::new().and((&pc).and(&cc).and(&db.user.reputation)))
        .window(rank, |(_, ((p, _), r))| (Reverse(r), Reverse(p)), asc);
    let mcp = db.post.with((&db.post.post_type_id).eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let at_one: HashIdx<i64, (Id<Post>, i64)> = (&mcp).map(|_| 1i64).inv().select(Ident::<Post>::new().and(&mcp)).collect();
    type R = ((Id<User>, ((i64, i64), i64)), i64);
    let v = drain((&w).filt(|(_, r)| r <= 10).select(Same::<R>::new().and(Same::<R>::new().map(|(_, r): R| r).select(&at_one).opt())));
    rows(v.into_iter().map(|(_, (((u, ((p, c), _)), _), m))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(c)]);
        f.extend(match m {
            Some((q, n)) => [ostr(db.post.title.get(q)), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecursiveTagExcerpts AS (SELECT T.Id AS TagId, T.TagName, T.Count, T.ExcerptPostId, P.Title AS ExcerptTitle, P.OwnerUserId, P.CreationDate
//     FROM Tags T LEFT JOIN Posts P ON T.ExcerptPostId = P.Id
//     UNION ALL SELECT T.Id, T.TagName, T.Count, T.ExcerptPostId, P.Title, P.OwnerUserId, P.CreationDate FROM Tags T JOIN Posts P ON T.WikiPostId = P.Id),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// UserPosts AS (SELECT U.Id AS UserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, MAX(P.CreationDate) AS LastPostDate FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id)
// SELECT U.DisplayName, U.Reputation, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, UP.TotalPosts, UP.TotalQuestions, UP.TotalAnswers, UP.LastPostDate,
//        RTE.TagName, RTE.ExcerptTitle, RTE.Count AS TagCount
// FROM Users U JOIN UserBadges UB ON U.Id = UB.UserId JOIN UserPosts UP ON U.Id = UP.UserId LEFT JOIN RecursiveTagExcerpts RTE ON U.Id = RTE.OwnerUserId
// WHERE U.Reputation > 1000 AND (RTE.TagName IS NOT NULL OR UP.TotalPosts > 5) ORDER BY U.Reputation DESC, RTE.Count DESC;
//
// Not recursive: the CTE is a plain UNION ALL. An excerpt with no post has no owner, so it never meets a user.
fn q32636(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, creation_date, owner_user, title, .. } = &db.post;
    let up = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(creation_date)).opt()).fold([0, 0, 0, i64::MIN], |a, p| match p {
        Some((t, d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(d)],
        None => a,
    });
    type T = (Id<Tag>, Id<Post>, i64);
    let ex = db.tag.select(Ident::<Tag>::new().and(&db.tag.excerpt_post)).map(|(t, p)| (t, p, 0i64));
    let wk = db.tag.select(Ident::<Tag>::new().and(&db.tag.wiki_post)).map(|(t, p)| (t, p, 1i64));
    let both: MatSet<T> = ex.union(wk).collect();
    let rte: HashIdx<Id<User>, T> = (&both).select(Same::<T>::new().map(|(_, p, _): T| p).select(owner_user)).inv().collect();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ub).and(&up).and((&rte).opt())).filt(|((_, a), r): (([i64; 4], [i64; 4]), Option<T>)| r.is_some() || a[0] > 5));
    rows(v.into_iter().map(|(u, ((b, a), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), tmax(a[3])]);
        f.extend(match r {
            Some((t, p, _)) => [V::S(db.tag.tag_name.get(t).unwrap()), ostr(title.get(p)), V::I(db.tag.count.get(t).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.Score IS NOT NULL),
// FilteredPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerName FROM RankedPosts WHERE RankScore <= 5),
// PostMetrics AS (SELECT fp.PostId, fp.Title, fp.OwnerName, fp.CreationDate, COALESCE(NULLIF(fp.ViewCount, 0), 1) AS ViewCount, CASE WHEN fp.Score IS NULL THEN 0 ELSE fp.Score END AS AdjustedScore,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = fp.PostId) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = fp.PostId AND v.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = fp.PostId AND v.VoteTypeId = 3) AS DownVotes FROM FilteredPosts fp),
// AggregatedMetrics AS (SELECT OwnerName, SUM(AdjustedScore) AS TotalScore, SUM(ViewCount) AS TotalViews, SUM(CommentCount) AS TotalComments, SUM(UpVotes) AS TotalUpVotes,
//        SUM(DownVotes) AS TotalDownVotes FROM PostMetrics GROUP BY OwnerName)
// SELECT OwnerName, TotalScore, TotalViews, TotalComments, TotalUpVotes, TotalDownVotes, CASE WHEN TotalViews > 0 THEN (TotalUpVotes * 100.0 / TotalViews) ELSE NULL END AS UpVotePercentage,
//        CASE WHEN TotalComments > 0 THEN (TotalScore * 1.0 / TotalComments) ELSE NULL END AS ScorePerComment
// FROM AggregatedMetrics WHERE TotalScore > 0 ORDER BY TotalScore DESC OFFSET 10 ROWS FETCH NEXT 5 ROWS ONLY;
fn q21931(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let w = db.post.with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let pm = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let am = (&fp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()).and(&pm).and(&pv))
        .fold([0i64; 5], |a, (((s, w), c), v)| [a[0] + s, a[1] + w.filter(|&w| w != 0).unwrap_or(1), a[2] + c, a[3] + v[0], a[4] + v[1]]);
    let v = top_n(drain((&am).filt(|a| a[0] > 0)), |&(n, a)| (Reverse(a[0]), n), 15);
    rows(v.into_iter().skip(10).map(|(n, a)| {
        let mut f = vec![V::S(n)];
        f.extend(a.map(V::I));
        f.push(if a[1] > 0 { V::F(a[3] as f64 * 100.0 / a[1] as f64) } else { V::Null });
        f.push(if a[2] > 0 { V::F(a[0] as f64 / a[2] as f64) } else { V::Null });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopRankedUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalWikis, TotalScore, RANK() OVER (ORDER BY TotalScore DESC, TotalPosts DESC) AS Rank FROM UserPostStats),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS TotalComments, MAX(v.CreationDate) AS LastVoteDate, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title),
// PopularPosts AS (SELECT pa.PostId, pa.Title, pa.TotalComments, pa.LastVoteDate, pa.UpVotes, pa.DownVotes,
//        RANK() OVER (ORDER BY (pa.UpVotes - pa.DownVotes) DESC, pa.TotalComments DESC) AS PostRank FROM PostActivity pa)
// SELECT tru.DisplayName, tru.TotalPosts, tru.TotalQuestions, tru.TotalAnswers, tru.TotalWikis, tru.TotalScore, pp.Title AS PopularPostTitle, pp.TotalComments, pp.LastVoteDate,
//        pp.UpVotes, pp.DownVotes
// FROM TopRankedUsers tru JOIN PopularPosts pp ON tru.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pp.PostId) WHERE tru.Rank <= 10 AND pp.PostRank <= 10
// ORDER BY tru.TotalScore DESC, pp.UpVotes DESC;
fn q9136(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + s],
        None => a,
    });
    let wu = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| (a[0] == 0, Reverse(a[4]), Reverse(a[0])), asc);
    let tr: MatSet<(Id<User>, [i64; 5])> = (&wu).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let tu: HashIdx<Id<User>, (Id<User>, [i64; 5])> = (&tr).map(|(u, _)| u).inv().collect();
    let pa = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).and(&db.vote.creation_date)).opt()))
        .fold([0, i64::MIN, 0, 0], |a, (c, v)| [a[0] + c.is_some() as i64, v.map_or(a[1], |(_, d)| a[1].max(d)), a[2] + (v.map(|x| x.0) == Some(2)) as i64, a[3] + (v.map(|x| x.0) == Some(3)) as i64]);
    let wp = whole(db.post.iq()).select(Ident::<Post>::new().and(&pa)).window(rank, |(_, a)| (Reverse(a[2] - a[3]), Reverse(a[0])), asc);
    let pp: MatSet<(Id<Post>, [i64; 4])> = (&wp).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let mut v = drain((&pp).select(Same::<(Id<Post>, [i64; 4])>::new().and(Same::<(Id<Post>, [i64; 4])>::new().map(|(p, _)| p).select(owner_user).select(&tu))));
    v.sort_by_key(|&(_, ((_, b), (_, a)))| (Reverse(a[4]), Reverse(b[2])));
    rows(v.into_iter().map(|(_, ((p, b), (u, a)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([ostr(db.post.title.get(p)), V::I(b[0]), tmax(b[1]), V::I(b[2]), V::I(b[3])]);
        row(f)
    }))
}

// WITH RankedQuestions AS (SELECT p.Id AS QuestionId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName),
// HighlightedAnswers AS (SELECT a.Id AS AnswerId, a.Body, a.Score AS AnswerScore, a.CreationDate AS AnswerCreationDate, aq.QuestionId,
//        ROW_NUMBER() OVER (PARTITION BY aq.QuestionId ORDER BY a.Score DESC) AS rn FROM Posts a JOIN RankedQuestions aq ON a.ParentId = aq.QuestionId WHERE a.PostTypeId = 2),
// TopAnswers AS (SELECT QuestionId, AnswerId, Body, AnswerScore, AnswerCreationDate FROM HighlightedAnswers WHERE rn = 1),
// CombinedResults AS (SELECT rq.QuestionId, rq.Title, rq.CreationDate AS QuestionCreationDate, rq.ViewCount, rq.Score AS QuestionScore, rq.OwnerDisplayName, ta.AnswerId,
//        ta.Body AS TopAnswerBody, ta.AnswerScore, ta.AnswerCreationDate FROM RankedQuestions rq LEFT JOIN TopAnswers ta ON rq.QuestionId = ta.QuestionId)
// SELECT cr.QuestionId, cr.Title AS QuestionTitle, cr.QuestionCreationDate, cr.ViewCount AS QuestionViewCount, cr.QuestionScore AS QuestionScore, cr.OwnerDisplayName,
//        COALESCE(cr.TopAnswerBody, 'No answers yet') AS TopAnswerBody, COALESCE(cr.AnswerScore, 0) AS TopAnswerScore, cr.AnswerCreationDate AS TopAnswerCreationDate
// FROM CombinedResults cr ORDER BY cr.QuestionCreationDate DESC LIMIT 20;
//
// AnswerCount is never read, and rn partitions by the question's own id, so RankedQuestions is the owned questions.
fn q26219(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let rq = top_n(drain(qs().select(creation_date)), |&(p, d)| (Reverse(d), p), 20);
    let rq: MatSet<Id<Post>> = rel(rq.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let w = (&rq).group_by(Ident::<Post>::new()).select(answers_of(db).select(Ident::<Post>::new().and(score))).window(row_number, |(a, s)| (Reverse(s), a), asc);
    let top = (&w).filt(|(_, r)| r == 1).map(|((a, _), _)| a);
    let v = drain((&rq).select(top.opt()));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend(match a {
            Some(a) => [V::S(db.post.body.get(a).unwrap()), V::I(score.get(a).unwrap()), V::T(creation_date.get(a).unwrap())],
            None => [V::S("No answers yet"), V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.ViewCount > 1000 THEN 1 ELSE 0 END) AS PopularPosts, AVG(P.Score) AS AvgScore,
//        MAX(P.CreationDate) AS LastPostDate FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, PopularPosts, AvgScore, LastPostDate, RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts,
//        RANK() OVER (ORDER BY AvgScore DESC) AS RankByScore FROM UserPostStats),
// ActiveBadges AS (SELECT B.UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// BenchmarkResults AS (SELECT U.DisplayName, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.PopularPosts, U.AvgScore, U.LastPostDate, AB.TotalBadges, AB.GoldBadges, AB.SilverBadges,
//        AB.BronzeBadges, U.RankByPosts, U.RankByScore FROM TopUsers U LEFT JOIN ActiveBadges AB ON U.UserId = AB.UserId)
// SELECT DisplayName, TotalPosts, TotalQuestions, TotalAnswers, PopularPosts, AvgScore, LastPostDate, TotalBadges, GoldBadges, SilverBadges, BronzeBadges, RankByPosts, RankByScore
// FROM BenchmarkResults WHERE TotalPosts > 10 ORDER BY RankByPosts, RankByScore;
fn q26539(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).opt())
        .fold([0, 0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some((((t, s), w), d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (w.map_or(false, |w| w > 1000)) as i64, a[4] + s, a[5].max(d)],
            None => a,
        });
    let ab = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let w1 = whole(db.user.iq()).select(Ident::<User>::new().and((&ups).and((&ab).opt()))).window(rank, |(_, (a, _))| Reverse(a[0]), asc);
    let w2 = (&w1).window(rank, |((_, (a, _)), _)| if a[0] == 0 { (true, Reverse(0)) } else { (false, Reverse(fkey(a[4] as f64 / a[0] as f64))) }, asc);
    let v = drain((&w2).filt(|(((_, (a, _)), _), _)| a[0] > 10));
    rows(v.into_iter().map(|(_, (((u, (a, b)), r1), r2))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), tmax(a[5])]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([V::I(r1), V::I(r2)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY SUBSTRING(p.Tags, 2, LENGTH(p.Tags) - 2) ORDER BY p.ViewCount DESC) AS TagRank, COUNT(c.Id) AS CommentCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// AggregatedUserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Tags, rp.CreationDate, rp.ViewCount, rp.Score, au.DisplayName AS OwnerDisplayName, au.TotalPosts, au.UpVotes, au.DownVotes, rp.CommentCount
//     FROM RankedPosts rp JOIN AggregatedUserStats au ON rp.OwnerUserId = au.UserId WHERE rp.TagRank <= 5 ORDER BY rp.TagRank, rp.ViewCount DESC)
// SELECT tp.PostId, tp.Title, tp.Tags, tp.CreationDate, tp.ViewCount, tp.Score, tp.OwnerDisplayName, tp.TotalPosts, tp.UpVotes, tp.DownVotes, tp.CommentCount,
//        CASE WHEN tp.Score > 100 THEN 'Highly Voted' WHEN tp.Score BETWEEN 50 AND 100 THEN 'Moderately Voted' ELSE 'Low Votes' END AS VoteCategory
// FROM TopPosts tp ORDER BY tp.ViewCount DESC LIMIT 10;
//
// TagRank and the final ORDER BY read only base columns, and the join to AggregatedUserStats keeps exactly the posts with an owner, so the ten posts
// are picked first and the badge x post x vote product is driven only for their owners.
fn q27966(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, view_count, score, .. } = &db.post;
    let inner = |t: Str| -> Str { &t[1..t.len() - 1] };
    let vk = |p: Id<Post>| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(tags_str.map(inner).opt())
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(p, w)| (w.is_none(), Reverse(w), p), asc);
    let r5: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let t10 = top_n(drain((&r5).select(owner_user)), |&(p, _)| vk(p), 10);
    let tp: MatSet<Id<Post>> = rel(t10.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let au = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()))
        .fold([0i64; 3], |a, (_, p)| [a[0] + p.is_some() as i64, a[1] + (p.flatten() == Some(2)) as i64, a[2] + (p.flatten() == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&au))));
    let v = top_n(v, |&(p, _)| vk(p), 10);
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "tags", "created", "views", "score"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        f.push(V::I(c));
        f.push(V::S(if s > 100 { "Highly Voted" } else if s >= 50 { "Moderately Voted" } else { "Low Votes" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(pb.TotalPosts, 0) AS TotalPosts, COALESCE(ub.TotalBadges, 0) AS TotalBadges, pb.AverageScore,
//        CASE WHEN COALESCE(pb.TotalPosts, 0) = 0 THEN 'No Posts' WHEN COALESCE(ub.TotalBadges, 0) > 10 THEN 'Highly Acclaimed' ELSE 'Average Contributor' END AS UserContributionLevel
//     FROM Users u LEFT JOIN PostStatistics pb ON u.Id = pb.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId),
// PostVoteStats AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN v.VoteTypeId IN (10, 12) THEN 1 ELSE 0 END) AS DeleteVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT u.UserId, u.DisplayName, u.TotalPosts, u.TotalBadges, u.UserContributionLevel, p.Title AS PostTitle, p.CreationDate AS PostCreationDate, ps.VoteCount, ps.UpVotes, ps.DownVotes,
//        ps.DeleteVotes, CASE WHEN ps.DeleteVotes > 0 THEN 'Deleted' WHEN ps.UpVotes > ps.DownVotes THEN 'Favorably Received' ELSE 'Unfavorably Received' END AS PostReception
// FROM UserPerformance u LEFT JOIN Posts p ON u.UserId = p.OwnerUserId LEFT JOIN PostVoteStats ps ON p.Id = ps.PostId WHERE u.TotalPosts > 0 ORDER BY u.UserId, p.CreationDate DESC;
fn q21013(db: &'static So) -> String {
    let pc = db.post.group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 4], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + matches!(t, Some(10 | 12)) as i64]
    });
    let v = drain(db.user.select((&pc).filt(|n| n > 0).and((&bc).opt()).and(posts_of(db).select(Ident::<Post>::new().and(&pv)).opt())));
    rows(v.into_iter().map(|(u, ((n, b), p))| {
        let b = b.unwrap_or(0);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n), V::I(b), V::S(if b > 10 { "Highly Acclaimed" } else { "Average Contributor" })];
        match p {
            Some((p, a)) => {
                f.extend(post_fields(db, p, &["title", "created"]));
                f.extend(a.map(V::I));
                f.push(V::S(if a[3] > 0 { "Deleted" } else if a[1] > a[2] { "Favorably Received" } else { "Unfavorably Received" }));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::S("Unfavorably Received")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation > 1000 THEN 'High Reputation' WHEN u.Reputation BETWEEN 500 AND 1000 THEN 'Moderate Reputation'
//        ELSE 'Low Reputation' END AS ReputationCategory FROM Users u WHERE u.Reputation IS NOT NULL),
// PostAndUserStats AS (SELECT rp.PostId, rp.OwnerUserId, ur.Reputation, ur.ReputationCategory, rp.Score AS PostScore, rp.CreationDate,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 3) AS DownVotes FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId)
// SELECT puas.PostId, puas.Reputation, puas.ReputationCategory, puas.PostScore, puas.CommentCount, puas.UpVotes, puas.DownVotes,
//        CASE WHEN puas.CommentCount > 10 THEN 'Highly Engaged' WHEN puas.CommentCount BETWEEN 1 AND 10 THEN 'Moderately Engaged' ELSE 'Not Engaged' END AS EngagementLevel
// FROM PostAndUserStats puas WHERE puas.PostScore > 5 AND EXISTS (SELECT 1 FROM Posts p WHERE p.AcceptedAnswerId = puas.PostId AND p.OwnerUserId IS NOT NULL AND p.PostTypeId = 2)
// ORDER BY puas.Reputation DESC, puas.CommentCount DESC LIMIT 50;
//
// Neither rank is read.
fn q24700(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, owner_user_id, post_type_id, accepted_answer, .. } = &db.post;
    let acc: MatSet<Id<Post>> = db.post.with(post_type_id.eq(2)).with(owner_user_id).select(accepted_answer).collect();
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(5))).with(owner_user).with(&acc).group_by(Ident::<Post>::new());
    let cc = base().select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = base().select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(&vc).and(owner_user));
    let v = top_n(v, |&(p, ((c, _), u))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(c), p), 50);
    rows(v.into_iter().map(|(p, ((c, a), u))| {
        let r = db.user.reputation.get(u).unwrap();
        row(vec![
            post_fields(db, p, &["id"]).remove(0),
            V::I(r),
            V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Moderate Reputation" } else { "Low Reputation" }),
            V::I(score.get(p).unwrap()),
            V::I(c),
            V::I(a[0]),
            V::I(a[1]),
            V::S(if c > 10 { "Highly Engaged" } else if c >= 1 { "Moderately Engaged" } else { "Not Engaged" }),
        ])
    }))
}

// WITH RECURSIVE PostHierarchy AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, p.AcceptedAnswerId, 0 AS Level FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId, p.AcceptedAnswerId, ph.Level + 1 AS Level FROM Posts p JOIN PostHierarchy ph ON p.ParentId = ph.PostId
//     WHERE p.PostTypeId = 2),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(p.Score), 0) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// VoterStats AS (SELECT v.UserId, COUNT(*) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// BadgesEarned AS (SELECT b.UserId, COUNT(*) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId)
// SELECT us.UserId, us.DisplayName, us.TotalPosts, us.TotalQuestions, us.TotalAnswers, us.TotalScore, COALESCE(vs.TotalVotes, 0) AS TotalVotes, COALESCE(vs.UpVotes, 0) AS UpVotes,
//        COALESCE(vs.DownVotes, 0) AS DownVotes, COALESCE(be.BadgeCount, 0) AS BadgeCount, COALESCE(be.HighestBadgeClass, 0) AS HighestBadgeClass
// FROM UserStats us LEFT JOIN VoterStats vs ON us.UserId = vs.UserId LEFT JOIN BadgesEarned be ON us.UserId = be.UserId WHERE us.TotalPosts > 10
// ORDER BY us.TotalScore DESC, us.TotalPosts DESC LIMIT 50;
//
// PostHierarchy is never referenced, so its recursion is never run.
fn q30692(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s],
        None => a,
    });
    let vs = db.vote.group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let be = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 2], |a, c| [a[0] + 1, a[1].max(c)]);
    let v = drain((&us).filt(|a| a[0] > 10).and((&vs).opt()).and((&be).opt()));
    let v = top_n(v, |&(u, ((a, _), _))| (Reverse(a[3]), Reverse(a[0]), u), 50);
    rows(v.into_iter().map(|(u, ((a, w), b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(w.unwrap_or([0; 3]).map(V::I));
        f.extend(b.unwrap_or([0; 2]).map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts, SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, Questions, Answers, TagWikis, PositiveScorePosts, NegativeScorePosts, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserReputation),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// UserPostHistory AS (SELECT p.OwnerUserId, COUNT(ph.Id) AS EditCount, SUM(CASE WHEN ph.PostHistoryTypeId BETWEEN 4 AND 6 THEN 1 ELSE 0 END) AS TotalEdits, MAX(ph.CreationDate) AS LastEditDate
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.OwnerUserId)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.Questions, tu.Answers, tu.TagWikis, tu.PositiveScorePosts, tu.NegativeScorePosts, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges,
//        ub.BronzeBadges, uph.EditCount, uph.TotalEdits, uph.LastEditDate
// FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId LEFT JOIN UserPostHistory uph ON tu.UserId = uph.OwnerUserId WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC;
//
// ReputationRank reads only Reputation, so the ten users are picked first.
fn q29791(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let g = || (&tu).group_by(Ident::<User>::new());
    let ur = g().select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt()).fold([0i64; 6], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + (s > 0) as i64, a[5] + (s < 0) as i64],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let PostHistory { post_history_type_id, creation_date, .. } = &db.post_history;
    let uph = g().select(posts_of(db).select(history_of(db).select(post_history_type_id.and(creation_date)))).fold([0, 0, i64::MIN], |a, (t, d)| {
        [a[0] + 1, a[1] + (4..=6).contains(&t) as i64, a[2].max(d)]
    });
    let mut v = drain((&ur).and((&ub).opt()).and((&uph).opt()));
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, ((a, b), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match h {
            Some(h) => [V::I(h[0]), V::I(h[1]), V::T(h[2])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.Body, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        RANK() OVER (ORDER BY COUNT(c.Id) DESC, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, u.DisplayName, p.Title, p.Body, p.CreationDate),
// MostActiveUsers AS (SELECT u.Id AS UserID, u.DisplayName, COUNT(v.Id) AS VoteCount, COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName
//     HAVING COUNT(v.Id) + COUNT(c.Id) > 10),
// PostHistoryAggregates AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MIN(ph.CreationDate) AS FirstEditDate, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT rp.PostID, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.AnswerCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, pha.EditCount, pha.FirstEditDate, pha.LastEditDate,
//        mau.DisplayName AS MostActiveUser, mau.VoteCount, mau.CommentCount, mau.BadgeCount
// FROM RankedPosts rp LEFT JOIN PostHistoryAggregates pha ON rp.PostID = pha.PostId
// LEFT JOIN MostActiveUsers mau ON mau.UserID = (SELECT UserID FROM MostActiveUsers ORDER BY VoteCount DESC LIMIT 1) WHERE rp.Rank <= 10 ORDER BY rp.Rank;
//
// The ON clause names only mau, against an uncorrelated scalar: every ranked post meets the one most active user.
fn q25940(db: &'static So) -> String {
    let rp = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, ((x, c), t)| [a[0] + x.is_some() as i64, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let w = whole(db.post.iq()).select(Ident::<Post>::new().and(&rp)).window(rank, |(_, a)| (Reverse(a[1]), Reverse(a[2])), asc);
    let PostHistory { post, post_history_type_id, creation_date, .. } = &db.post_history;
    let pha = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(creation_date).fold([0, i64::MAX, i64::MIN], |a, d| [a[0] + 1, a[1].min(d), a[2].max(d)]);
    let mau = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).opt().and(comments_by(db).opt()).and(badges_of(db).opt()))
        .fold([0i64; 3], |a, ((v, c), b)| [a[0] + v.is_some() as i64, a[1] + c.is_some() as i64, a[2] + b.is_some() as i64]);
    let best = rel(top_n(drain((&mau).filt(|a| a[0] + a[1] > 10)), |&(_, a)| Reverse(a[0]), 1));
    let bi: HashIdx<(), (Id<User>, [i64; 3])> = (&best).map(|_| ()).inv().select(&best).collect();
    type R = ((Id<Post>, [i64; 4]), i64);
    let mut v: Vec<_> = drain((&w).filt(|(_, r)| r <= 10).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select(&pha).opt())).and((&bi).opt()))
        .into_iter()
        .map(|x| x.1)
        .collect();
    v.sort_by_key(|&(((_, r), _), _)| r);
    rows(v.into_iter().map(|((((p, a), _), h), m)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(h) => [V::I(h[0]), V::T(h[1]), V::T(h[2])],
            None => [V::Null, V::Null, V::Null],
        });
        match m {
            Some((u, m)) => {
                f.push(user_col(db, u, "name"));
                f.extend(m.map(V::I));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE(p.ViewCount, 0) AS ViewCount, COALESCE(p.Score, 0) AS Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 days'),
// PostScoreCTE AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.ViewCount, rp.Score, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.Id) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.Id AND v.VoteTypeId = 2) AS UpvoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.Id AND v.VoteTypeId = 3) AS DownvoteCount
//     FROM RecentPosts rp WHERE rp.rn = 1),
// AverageVotes AS (SELECT AVG(CommentCount) AS AvgComments, AVG(UpvoteCount) AS AvgUpvotes, AVG(DownvoteCount) AS AvgDownvotes FROM PostScoreCTE),
// PostAnalytics AS (SELECT ps.Id, ps.Title, ps.OwnerDisplayName, ps.ViewCount, ps.Score, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount,
//        CASE WHEN ps.UpvoteCount > ps.DownvoteCount THEN 'Positive' WHEN ps.UpvoteCount < ps.DownvoteCount THEN 'Negative' ELSE 'Neutral' END AS Sentiment,
//        CASE WHEN ps.ViewCount > (SELECT AVG(ViewCount) FROM PostScoreCTE) THEN 1 ELSE 0 END AS IsAboveAverageViews FROM PostScoreCTE ps)
// SELECT pa.OwnerDisplayName, pa.Title, pa.ViewCount, pa.Score, pa.CommentCount, pa.UpvoteCount, pa.DownvoteCount, pa.Sentiment, (SELECT AVG(AvgComments) FROM AverageVotes) AS OverallAvgComments,
//        (SELECT AVG(AvgUpvotes) FROM AverageVotes) AS OverallAvgUpvotes, (SELECT AVG(AvgDownvotes) FROM AverageVotes) AS OverallAvgDownvotes
// FROM PostAnalytics pa WHERE pa.IsAboveAverageViews = 1 ORDER BY pa.Score DESC;
fn q32815(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(current_date(), -30)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let ps: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&ps).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&ps).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let stats = (&cc).and(&vc).and(view_count.opt());
    let tot = whole(&stats).select(&stats).fold([0i64; 5], |a, ((c, u), w)| [a[0] + 1, a[1] + c, a[2] + u[0], a[3] + u[1], a[4] + w.unwrap_or(0)]);
    let v = drain((&stats).cross(&tot).filt(|(((_, _), w), a): (((i64, [i64; 2]), Option<i64>), [i64; 5])| (w.unwrap_or(0) as f64) > a[4] as f64 / a[0] as f64));
    let mut v = v;
    v.sort_by_key(|&((p, _), _)| Reverse(db.post.score.get(p).unwrap()));
    rows(v.into_iter().map(|((p, _), (((c, u), w), a))| {
        let mut f = post_fields(db, p, &["owner", "title"]);
        f.extend([V::I(w.unwrap_or(0)), V::I(db.post.score.get(p).unwrap()), V::I(c), V::I(u[0]), V::I(u[1])]);
        f.push(V::S(if u[0] > u[1] { "Positive" } else if u[0] < u[1] { "Negative" } else { "Neutral" }));
        f.extend([avg(a[1], a[0]), avg(a[2], a[0]), avg(a[3], a[0])]);
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// ActiveUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserPostCounts)
// SELECT UserId, DisplayName, Reputation, PostCount, BadgeCount, Rank FROM ActiveUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Every user ranked above one of the top ten by (Reputation, PostCount) has at least its reputation, so the users with RANK() <= 10 by Reputation alone
// are picked first; the post x badge product is driven for those, and their ranks among themselves are their ranks among all users.
fn q13447(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let s = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (p, b)| [a[0] + p.is_some() as i64, a[1] + b.is_some() as i64]);
    let w2 = whole(&s).select(Ident::<User>::new().and((&s).and(&db.user.reputation))).window(rank, |(_, (a, r))| (Reverse(r), Reverse(a[0])), asc);
    rows(drain((&w2).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, (a, _)), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT u.UserId, u.DisplayName, u.PostCount, u.VoteCount, RANK() OVER (ORDER BY us.Reputation DESC) AS ReputationRank FROM UserPostCounts u JOIN Users us ON u.UserId = us.Id)
// SELECT UserId, DisplayName, PostCount, VoteCount, ReputationRank FROM RankedUsers WHERE ReputationRank <= 10 ORDER BY ReputationRank;
//
// ReputationRank reads only Reputation, so the ten users are picked first and the post x vote product is driven for those.
fn q12725(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    let rr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().collect();
    let s = (&tu).map(|(u, _)| u).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).opt()).opt()).fold([0i64; 2], |a, p| [a[0] + p.is_some() as i64, a[1] + p.flatten().is_some() as i64]);
    let v = drain((&s).and((&tu).map(|(_, r)| r)));
    rows(v.into_iter().map(|(u, (a, r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// TopUsers AS (SELECT UserId, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS RankByPostCount FROM UserPostCounts)
// SELECT u.DisplayName, u.Reputation, u.CreationDate, tuc.PostCount, tuc.QuestionCount, tuc.AnswerCount FROM TopUsers tuc JOIN Users u ON u.Id = tuc.UserId
// WHERE tuc.RankByPostCount <= 10 ORDER BY tuc.RankByPostCount;
fn q11951(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["name", "rep", "ucreated"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH TagStats AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers,
//        SUM(COALESCE(P.CommentCount, 0)) AS TotalComments FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName),
// TopTags AS (SELECT TagName, TotalViews, ROW_NUMBER() OVER (ORDER BY TotalViews DESC) AS RN FROM TagStats)
// SELECT T.TagName, T.TotalViews, P.Title AS MostViewedPost, P.ViewCount AS PostViewCount FROM TopTags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%'
// WHERE T.RN <= 10 ORDER BY T.TotalViews DESC;
fn q27530(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts = db.tag.group_by(&db.tag.tag_name).select((&by_tag).map(|(p, _)| p).select((&db.post.view_count).opt()).opt()).fold(0i64, |s, w| s + w.flatten().unwrap_or(0));
    let top = top_n(drain(&ts), |&(t, s)| (Reverse(s), t), 10);
    let tt = rel(top);
    type PT = (Id<Post>, Id<Tag>);
    let named: MatSet<(Id<Post>, Str)> = (&lt).select(Same::<PT>::new().map(|(p, _): PT| p).and(Same::<PT>::new().map(|(_, t): PT| t).select(&db.tag.tag_name))).collect();
    let by_name: HashIdx<Str, (Id<Post>, Str)> = (&named).map(|(_, n)| n).inv().collect();
    let v = drain((&tt).select(Same::<(Str, i64)>::new().and(Same::<(Str, i64)>::new().map(|(t, _)| t).select(&by_name))));
    rows(v.into_iter().map(|(_, ((t, s), (p, _)))| {
        let mut f = vec![V::S(t), V::I(s)];
        f.extend(post_fields(db, p, &["title", "views"]));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(V.Id, 0)) AS TotalVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, TotalScore, TotalVotes, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserReputation)
// SELECT U.DisplayName, U.Reputation, TU.PostCount, TU.TotalScore, TU.TotalVotes FROM TopUsers TU INNER JOIN Users U ON TU.UserId = U.Id WHERE TU.Rank <= 10;
fn q11233(db: &'static So) -> String {
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(&db.vote.origid).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((s, v)) => [a[0] + s, a[1] + v.unwrap_or(0)],
            None => a,
        });
    let pc = distinct_posts(db);
    let v = top_n(drain((&ur).and((&pc).opt())), |&(u, (a, _))| (Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, u.Views, u.UpVotes, u.DownVotes, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation, u.Views, u.UpVotes, u.DownVotes),
// TopUsers AS (SELECT UserId, Reputation, Views, UpVotes, DownVotes, PostCount, TotalBountyAmount, ROW_NUMBER() OVER (ORDER BY PostCount DESC, Reputation DESC) AS Rank FROM UserStatistics)
// SELECT UserId, Reputation, Views, UpVotes, DownVotes, PostCount, TotalBountyAmount FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only the distinct post count and Reputation, so the ten users are picked first and the post x vote product is driven for those.
fn q11228(db: &'static So) -> String {
    let pc = distinct_posts(db);
    let top = top_n(drain(db.user.select((&pc).opt())), |&(u, n)| (Reverse(n.unwrap_or(0)), Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let b = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold(0i64, |s, (_, b)| s + b.flatten().unwrap_or(0));
    let v = drain((&b).and((&pc).opt()));
    rows(v.into_iter().map(|(u, (b, n))| {
        let mut f = ucols(db, u, &["uid", "rep", "uviews", "uup", "udown"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(b)]);
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, AnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS PostRank, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
//     FROM UserPostCounts WHERE PostCount > 0)
// SELECT UserId, DisplayName, PostCount, TotalScore, AnswerCount, PostRank, ScoreRank FROM TopUsers WHERE PostRank <= 10 OR ScoreRank <= 10 ORDER BY PostRank, ScoreRank;
fn q12709(db: &'static So) -> String {
    let ups = user_posts(db);
    let w1 = whole(db.user.iq()).select(Ident::<User>::new().and((&ups).filt(|a| a[1] > 0))).window(rank, |(_, a)| Reverse(a[1]), asc);
    let w2 = (&w1).window(rank, |((_, a), _)| Reverse(a[4]), asc);
    rows(drain((&w2).filt(|((_, p), s)| p <= 10 || s <= 10)).into_iter().map(|(_, (((u, a), p), s))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[4]), V::I(a[3]), V::I(p), V::I(s)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, ReputationRank FROM TopUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
fn q12504(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and((&ups).and(&db.user.reputation))).window(rank, |(_, (_, r))| r, desc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, (a, _)), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, TotalScore, UpVotes, DownVotes FROM TopUsers WHERE ScoreRank <= 10;
fn q10789(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, t)) => [a[0] + 1, a[1] + s, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&us)).window(rank, |(_, a)| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(P.Score) AS TotalScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, TotalScore, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q12817(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[3]), V::I(a[2]), nullable(a[4], a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        SUM(COALESCE(B.Id, 0)) AS TotalBadges FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalViews, TotalScore, TotalBadges, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStats)
// SELECT UserId, DisplayName, TotalPosts, TotalViews, TotalScore, TotalBadges, ScoreRank FROM TopUsers WHERE ScoreRank <= 10 ORDER BY ScoreRank;
fn q12664(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)).opt().and(badges_of(db).select(&db.badge.origid).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let (w, s) = p.map_or((0, 0), |(w, s)| (w.unwrap_or(0), s));
            [a[0] + w, a[1] + s, a[2] + b.unwrap_or(0)]
        });
    let pc = distinct_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and((&us).and((&pc).opt()))).window(rank, |(_, (a, _))| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, (a, n)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n.unwrap_or(0)));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews FROM TopUsers WHERE ScoreRank <= 10;
fn q12714(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(case when p.PostTypeId = 1 then 1 end) AS TotalQuestions,
//        COUNT(case when p.PostTypeId = 2 then 1 end) AS TotalAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, AvgViewCount, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, AvgViewCount FROM TopUsers WHERE ScoreRank <= 10;
fn q12705(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = top_n(drain(&ups), |&(u, a)| (Reverse(a[4]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[6], a[5])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalBounty, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalBounty, Rank FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only the distinct post count, so the ten users are picked first and the post x vote product is driven for those.
fn q13153(db: &'static So) -> String {
    let pc = distinct_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and((&pc).opt())).window(row_number, |(u, n)| (Reverse(n.unwrap_or(0)), u), asc);
    let tr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&tr).map(|(u, _)| u).inv().collect();
    let s = (&tu)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
            }
            None => a,
        });
    let v = drain((&s).and((&pc).opt()).and((&tu).map(|(_, r)| r)));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, PositiveScorePosts, NegativeScorePosts, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStatistics)
// SELECT UserId, DisplayName, Reputation, PostCount, PositiveScorePosts, NegativeScorePosts FROM TopUsers WHERE Rank <= 10 ORDER BY Reputation DESC;
fn q12728(db: &'static So) -> String {
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score).opt()).fold([0i64; 3], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64],
        None => a,
    });
    rows(drain(&s).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT rp.*, ROW_NUMBER() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS Rank FROM RankedPosts rp)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.VoteCount FROM TopPosts tp WHERE tp.Rank <= 10;
//
// Rank reads only base columns, so the ten questions are picked first and the comment x upvote product is driven for those.
fn q6527(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(date(2024, 10, 1), -30)))).select(score));
    let top = top_n(v, |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let dv = (&tp).group_by(Ident::<Post>::new()).select(up().select(&db.vote.user_id)).count_distinct();
    rows(drain((&cc).and((&dv).opt())).into_iter().map(|(p, (c, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(n.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore FROM TopUsers WHERE ScoreRank <= 10 ORDER BY TotalScore DESC;
fn q10464(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| Reverse(a[4]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        row(f)
    }))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, TotalBounty, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserMetrics)
// SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, TotalBounty FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the post x vote product is driven for those.
fn q13760(db: &'static So) -> String {
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 4], |a, (t, b)| {
            let b = b.flatten();
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
        });
    let pc = distinct_posts(db);
    rows(drain((&s).and((&pc).opt())).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score ELSE 0 END) AS TotalScore,
//        SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT * FROM TopUsers WHERE ScoreRank <= 10;
fn q11612(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 || t == 2 { s } else { 0 }, a[4] + w.unwrap_or(0)],
        None => a,
    });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[3]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Ranking FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, Ranking FROM TopUsers WHERE Ranking <= 10 ORDER BY Ranking;
fn q13056(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(row_number, |(u, a)| (Reverse(a[1]), u), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, Rank FROM TopUsers WHERE Rank <= 10;
fn q13739(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), nullable(a[4], a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, BadgeCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT u.DisplayName, tu.Reputation, tu.PostCount, tu.BadgeCount, tu.UpVotes, tu.DownVotes FROM TopUsers tu JOIN Users u ON tu.UserId = u.Id WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the post x badge x vote product is driven for those.
fn q14672(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let g = || (&tu).group_by(Ident::<User>::new());
    let s = g()
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let pc = g().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = g().select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&s).and(&pc).and(&bc)).into_iter().map(|(u, ((a, p), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(b), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserPosts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
//        COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPosts)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews FROM TopUsers WHERE Rank <= 10;
fn q11345(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = top_n(drain(&ups), |&(u, a)| (Reverse(a[4]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6])]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(b.Id, 0)) AS BadgeCount,
//        COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, TotalScore, BadgeCount, CommentCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics)
// SELECT UserId, Reputation, PostCount, TotalScore, BadgeCount, CommentCount, ReputationRank FROM TopUsers WHERE ReputationRank <= 10;
//
// ReputationRank reads only Reputation, so the ten users are picked first and the post x badge x comment product is driven for those.
fn q10197(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    let rr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().collect();
    let g = || (&tu).map(|(u, _)| u).group_by(Ident::<User>::new());
    let s = g()
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.origid).opt()))
        .fold([0i64; 2], |a, (p, b)| [a[0] + p.map_or(0, |(s, _)| s), a[1] + b.unwrap_or(0)]);
    let pc = g().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = g().select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    rows(drain((&s).and(&pc).and((&cc).opt()).and((&tu).map(|(_, r)| r))).into_iter().map(|(u, (((a, p), c), r))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(p), V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0)), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, ScoreRank FROM TopUsers WHERE ScoreRank <= 10 ORDER BY TotalScore DESC;
fn q10851(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| Reverse(a[4]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

/// Users LEFT JOIN Posts LEFT JOIN Votes, for the given users: [upvotes, downvotes, questions, answers] over the joined rows.
fn post_votes_of(db: &'static So, tu: &MatSet<Id<User>>) -> Fold<Id<User>, [i64; 4]> {
    tu.group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64],
            None => a,
        })
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpVoteCount, DownVoteCount, QuestionCount, AnswerCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStatistics)
// SELECT * FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only Reputation, so the ten users are picked first and the post x vote product is driven for those.
fn q11386(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let rr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().collect();
    let tu: MatSet<Id<User>> = (&rr).map(|(u, _)| u).collect();
    let s = post_votes_of(db, &tu);
    let pc = distinct_posts(db);
    let v = drain((&s).and((&pc).opt()).and((&rank).map(|(_, r)| r)));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpvoteCount, DownvoteCount, ReputationRank FROM TopUsers WHERE ReputationRank <= 10;
//
// ReputationRank reads only Reputation, so the ten users are picked first and the post x vote product is driven for those.
fn q12847(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    let rr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().collect();
    let tu: MatSet<Id<User>> = (&rr).map(|(u, _)| u).collect();
    let s = post_votes_of(db, &tu);
    let pc = distinct_posts(db);
    let v = drain((&s).and((&pc).opt()).and((&rank).map(|(_, r)| r)));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[2]), V::I(a[3]), V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers, COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COALESCE(SUM(p.Score), 0) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, ScoreRank FROM TopUsers WHERE ScoreRank <= 10;
fn q10102(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| Reverse(a[4]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostCount,
//        SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, BadgeCount, PostCount, TotalViews, TotalScore, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStatistics)
// SELECT u.UserId, u.Reputation, u.BadgeCount, u.PostCount, u.TotalViews, u.TotalScore, t.Rank FROM UserStatistics u JOIN TopUsers t ON u.UserId = t.UserId WHERE t.Rank <= 10 ORDER BY t.Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the badge x post product is driven for those.
fn q13086(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let rr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().collect();
    let s = (&rank)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)).opt()))
        .fold([0i64; 6], |a, (b, p)| match p {
            Some((w, s)) => [a[0] + b.is_some() as i64, a[1] + 1, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s, 0],
            None => [a[0] + b.is_some() as i64, a[1], a[2], a[3], a[4], 0],
        });
    rows(drain((&s).and((&rank).map(|(_, r)| r))).into_iter().map(|(u, (a, r))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), nullable(a[4], a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH user_stats AS (SELECT Users.Id AS UserId, Users.Reputation, COUNT(DISTINCT Posts.Id) AS PostCount, COUNT(DISTINCT Badges.Id) AS BadgeCount,
//        SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId LEFT JOIN Badges ON Users.Id = Badges.UserId LEFT JOIN Votes ON Posts.Id = Votes.PostId GROUP BY Users.Id, Users.Reputation),
// top_users AS (SELECT UserId, Reputation, PostCount, BadgeCount, UpvoteCount, DownvoteCount, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM user_stats)
// SELECT UserId, Reputation, PostCount, BadgeCount, UpvoteCount, DownvoteCount, Rank FROM top_users WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the post x badge x vote product is driven for those.
fn q12322(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    let rr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().collect();
    let g = || (&tu).map(|(u, _)| u).group_by(Ident::<User>::new());
    let s = g()
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let pc = g().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = g().select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&s).and(&pc).and(&bc).and((&tu).map(|(_, r)| r))).into_iter().map(|(u, (((a, p), b), r))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(p), V::I(b), V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, BadgeCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS Ranking FROM UserStats)
// SELECT UserId, Reputation, PostCount, BadgeCount, UpVotes, DownVotes, Ranking FROM TopUsers WHERE Ranking <= 100 ORDER BY Ranking;
//
// Ranking reads only Reputation, so the hundred users are picked first and the post x badge x vote product is driven for those.
fn q14801(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    let rr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 100).map(|((u, _), r)| (u, r)).collect();
    let tu: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().collect();
    let g = || (&tu).map(|(u, _)| u).group_by(Ident::<User>::new());
    let s = g()
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let pc = g().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = g().select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&s).and(&pc).and(&bc).and((&tu).map(|(_, r)| r))).into_iter().map(|(u, (((a, p), b), r))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(p), V::I(b), V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(c.Id, 0)) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, QuestionCount, AnswerCount, CommentCount, ROW_NUMBER() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, TotalViews, QuestionCount, AnswerCount, CommentCount, ViewRank FROM TopUsers WHERE ViewRank <= 10;
fn q14370(db: &'static So) -> String {
    let Post { view_count, post_type_id, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(post_type_id).and(comments_of(db).select(&db.comment.origid).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((w, t), c)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + c.unwrap_or(0)],
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(row_number, |(u, a)| (Reverse(a[1]), u), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS VerifiedPosts, AVG(p.Score) AS AvgScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, VerifiedPosts, AvgScore, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, VerifiedPosts, AvgScore FROM TopUsers WHERE Rank <= 10 ORDER BY TotalPosts DESC;
fn q10098(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = top_n(drain(&ups), |&(u, a)| (Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[1]), avg(a[4], a[1])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AverageViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, AverageViews, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, AverageViews FROM TopUsers WHERE ScoreRank <= 10 ORDER BY ScoreRank;
fn q14352(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), avg(a[6], a[5])]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// RankedPosts AS (SELECT rp.*, RANK() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS Rank FROM RecentPosts rp)
// SELECT r.Title, r.OwnerDisplayName, r.CreationDate, r.Score, r.ViewCount, r.CommentCount, r.UpVotes, r.DownVotes, r.Rank FROM RankedPosts r WHERE r.Rank <= 10 ORDER BY r.Rank;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those.
fn q7720(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let w = whole(db.post.iq())
        .select(Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).and(score.and(view_count.opt())))
        .window(rank, |(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let rr: MatSet<(Id<Post>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), r)| (p, r)).collect();
    let rank: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rr).map(|(p, _)| p).inv().collect();
    let s = (&rank)
        .map(|(p, _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).and((&rank).map(|(_, r)| r))).into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(vs.VoteCount), 0) AS TotalVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) vs ON p.Id = vs.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalVotes, RANK() OVER (ORDER BY TotalVotes DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalVotes FROM TopUsers WHERE Rank <= 10 ORDER BY TotalVotes DESC;
fn q11120(db: &'static So) -> String {
    let vs = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&vs).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, n)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + n.unwrap_or(0)],
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[3]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(COALESCE(p.VIEWCOUNT, 0)) AS AvgViewCount, AVG(COALESCE(p.Score, 0)) AS AvgScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AvgViewCount, AvgScore, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AvgViewCount, AvgScore, PostRank FROM TopUsers WHERE PostRank <= 10 ORDER BY PostRank;
fn q12844(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[6], a[0]), avg(a[4], a[0]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
//        AVG(COALESCE(P.AnswerCount, 0)) AS AvgAnswers, AVG(COALESCE(P.CommentCount, 0)) AS AvgComments FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, TotalScore, TotalViews, AvgAnswers, AvgComments, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStats)
// SELECT U.DisplayName, U.Reputation, TU.PostCount, TU.TotalScore, TU.TotalViews, TU.AvgAnswers, TU.AvgComments FROM TopUsers TU JOIN Users U ON TU.UserId = U.Id
// WHERE TU.ScoreRank <= 10 ORDER BY TU.ScoreRank;
fn q10882(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count)).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((((s, w), n), c)) => [a[0] + 1, a[1] + 1, a[2] + s, a[3] + w.unwrap_or(0), a[4] + n.unwrap_or(0), a[5] + c],
            None => [a[0] + 1, a[1], a[2], a[3], a[4], a[5]],
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[2]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), avg(a[5], a[0])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(C.CommentCount, 0)) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, TotalScore, TotalComments, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStats)
// SELECT U.DisplayName, U.Reputation, T.PostCount, T.TotalScore, T.TotalComments, T.ScoreRank FROM TopUsers T JOIN Users U ON T.UserId = U.Id WHERE T.ScoreRank <= 10 ORDER BY T.ScoreRank;
fn q12169(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and((&cc).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((s, c)) => [a[0] + 1, a[1] + s, a[2] + c.unwrap_or(0)],
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[1]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalUpVotes, TotalDownVotes FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only the distinct post count, so the ten users are picked first and the post x vote product is driven for those.
fn q12757(db: &'static So) -> String {
    let pc = distinct_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and((&pc).opt())).window(rank, |(_, n)| Reverse(n.unwrap_or(0)), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let s = post_votes_of(db, &tu);
    rows(drain((&s).and((&pc).opt())).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[2]), V::I(a[3]), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(c.Id) AS TotalComments, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalScore, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY TotalScore DESC) AS UserRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalScore, TotalUpVotes, TotalDownVotes FROM TopUsers WHERE UserRank <= 10;
fn q12479(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((s, c), t)) => [a[0] + 1, a[1] + c.is_some() as i64, a[2] + s, a[3] + (t == Some(2)) as i64, a[4] + (t == Some(3)) as i64],
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[2]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// RankedUserStats AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalViews, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalViews, TotalScore, ScoreRank FROM RankedUserStats WHERE ScoreRank <= 10 ORDER BY ScoreRank;
fn q10926(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), nullable(a[4], a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, TotalScore, TotalViews, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, TotalScore, TotalViews FROM TopUsers WHERE ScoreRank <= 10 ORDER BY TotalScore DESC;
fn q14521(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), _))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), nullable(a[6], a[5])]);
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount, COUNT(DISTINCT c.Id) AS CommentCount, AVG(u.Reputation) AS AvgUserReputation
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON c.PostId = p.Id
//     LEFT JOIN Users u ON p.OwnerUserId = u.Id GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, QuestionCount, AnswerCount, CommentCount, AvgUserReputation, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagStatistics)
// SELECT TagName, PostCount, QuestionCount, AnswerCount, CommentCount, AvgUserReputation FROM TopTags WHERE TagRank <= 10 ORDER BY TagRank;
//
// TagRank reads only the distinct post count, so the ten tags are picked first and the post x comment product is driven for those.
fn q29008(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t)| t).inv().map(|(p, _)| p).collect();
    let of_name: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pc = db.tag.group_by(&db.tag.tag_name).select(&by_tag).count_distinct();
    let w = whole(&pc).select(Same::<Str>::new().and(&pc)).window(rank, |(_, n)| n, desc);
    let tt: MatSet<Str> = (&w).filt(|(_, r)| r <= 10).map(|((t, _), _)| t).collect();
    let g = || (&tt).group_by(Same::<Str>::new());
    let s = g()
        .select((&of_name).select(&by_tag).select(ptype_name(db).and(comments_of(db).opt()).and((&db.post.owner_user).select(&db.user.reputation).opt())))
        .fold([0i64; 4], |a, ((t, _), r)| [a[0] + (t == "Question") as i64, a[1] + (t == "Answer") as i64, a[2] + r.is_some() as i64, a[3] + r.unwrap_or(0)]);
    let cc = g().select((&of_name).select(&by_tag).select(comments_of(db))).count_distinct();
    let mut v = drain((&s).and(&pc).and((&cc).opt()));
    v.sort_by_key(|&(_, ((_, n), _))| Reverse(n));
    rows(v.into_iter().map(|(t, ((a, n), c))| row(vec![V::S(t), V::I(n), V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0)), avg(a[3], a[2])])))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, TotalViews, TotalScore, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserMetrics)
// SELECT U.DisplayName, MU.Reputation, MU.PostCount, MU.QuestionCount, MU.AnswerCount, MU.TotalViews, MU.TotalScore FROM TopUsers MU JOIN Users U ON MU.UserId = U.Id
// WHERE MU.Rank <= 10 ORDER BY MU.Rank;
fn q14623(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = top_n(drain(&ups), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1])]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.Reputation AS OwnerReputation,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.Reputation),
// TopPosts AS (SELECT *, ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostStats)
// SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerReputation, UpVotes, DownVotes FROM TopPosts WHERE Rank <= 10;
//
// Rank reads only base columns, so the ten posts are picked first and their votes are counted.
fn q14966(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let top = top_n(drain(score), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

/// For the ten users by Reputation (ROW_NUMBER): Users LEFT JOIN Posts LEFT JOIN Comments (on the post) LEFT JOIN Votes (by the user),
/// [distinct posts, distinct comments, upvotes, downvotes, rank].
fn rep10_post_comment_votes(db: &'static So) -> Vec<(Id<User>, [i64; 5])> {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let rr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().collect();
    let g = || (&rank).map(|(u, _)| u).group_by(Ident::<User>::new());
    let s = g()
        .select(posts_of(db).select(comments_of(db).opt()).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = g().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = g().select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    drain((&s).and(&pc).and(&cc).and((&rank).map(|(_, r)| r))).into_iter().map(|(u, (((a, p), c), r))| (u, [p, c, a[0], a[1], r])).collect()
}

// WITH UsersStats AS (SELECT U.Id AS UserId, U.Reputation, U.CreationDate, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON U.Id = V.UserId
//     GROUP BY U.Id, U.Reputation, U.CreationDate, U.DisplayName),
// TopUsers AS (SELECT UserId, Reputation, PostCount, CommentCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UsersStats)
// SELECT UserId, Reputation, PostCount, CommentCount, UpVotes, DownVotes FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only Reputation, so the ten users are picked first and the post x comment x vote product is driven for those.
fn q13495(db: &'static So) -> String {
    rows(rep10_post_comment_votes(db).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a[..4].iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalViews, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStatistics)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalViews, TotalScore, ScoreRank FROM TopUsers WHERE ScoreRank <= 10;
fn q10979(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, a), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), nullable(a[4], a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation WHERE Reputation > 1000)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.UpVotes, TU.DownVotes, (TU.UpVotes - TU.DownVotes) AS NetVotes FROM TopUsers TU WHERE TU.Rank <= 10
// ORDER BY TU.Reputation DESC, NetVotes DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the post x vote product is driven for those.
fn q9557(db: &'static So) -> String {
    let top = top_n(drain((&db.user.reputation).gt(1000)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = post_votes_of(db, &tu);
    let pc = distinct_posts(db);
    rows(drain((&s).and((&pc).opt())).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, CommentCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, CommentCount, UpVotes, DownVotes, Rank FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only Reputation, so the ten users are picked first and the post x comment x vote product is driven for those.
fn q13170(db: &'static So) -> String {
    rows(rep10_post_comment_votes(db).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(V.BountyAmount) AS TotalBounty, AVG(U.Reputation) AS AverageReputation
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalBounty, AverageReputation, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalBounty, AverageReputation, Rank FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only the distinct post count, so the ten users are picked first and the post x vote product is driven for those.
fn q14608(db: &'static So) -> String {
    let pc = distinct_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and((&pc).opt())).window(rank, |(_, n)| Reverse(n.unwrap_or(0)), asc);
    let rr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&rr).map(|(u, _)| u).inv().collect();
    let s = (&rank)
        .map(|(u, _)| u)
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt()))
        .fold([0i64; 6], |a, (r, p)| {
            let (t, b) = p.map_or((0, None), |(t, b)| (t, b.flatten()));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0), a[4] + 1, a[5] + r]
        });
    rows(drain((&s).and((&pc).opt()).and((&rank).map(|(_, r)| r))).into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::F(a[5] as f64 / a[4] as f64), V::I(r)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("28039", q28039),
    ("6524", q6524),
    ("7875", q7875),
    ("33414", q33414),
    ("2919", q2919),
    ("6732", q6732),
    ("3584", q3584),
    ("28654", q28654),
    ("7211", q7211),
    ("8758", q8758),
    ("7930", q7930),
    ("2080", q2080),
    ("29819", q29819),
    ("30151", q30151),
    ("30419", q30419),
    ("26341", q26341),
    ("7222", q7222),
    ("27786", q27786),
    ("9690", q9690),
    ("6147", q6147),
    ("28878", q28878),
    ("9683", q9683),
    ("27949", q27949),
    ("2161", q2161),
    ("34265", q34265),
    ("28032", q28032),
    ("26175", q26175),
    ("27950", q27950),
    ("20338", q20338),
    ("30714", q30714),
    ("29036", q29036),
    ("1119", q1119),
    ("32636", q32636),
    ("21931", q21931),
    ("9136", q9136),
    ("26219", q26219),
    ("26539", q26539),
    ("27966", q27966),
    ("21013", q21013),
    ("24700", q24700),
    ("30692", q30692),
    ("29791", q29791),
    ("25940", q25940),
    ("32815", q32815),
    ("13447", q13447),
    ("12725", q12725),
    ("11951", q11951),
    ("27530", q27530),
    ("11233", q11233),
    ("11228", q11228),
    ("12709", q12709),
    ("12504", q12504),
    ("10789", q10789),
    ("12817", q12817),
    ("12664", q12664),
    ("12714", q12714),
    ("12705", q12705),
    ("13153", q13153),
    ("12728", q12728),
    ("6527", q6527),
    ("10464", q10464),
    ("13760", q13760),
    ("11612", q11612),
    ("13056", q13056),
    ("13739", q13739),
    ("14672", q14672),
    ("11345", q11345),
    ("10197", q10197),
    ("10851", q10851),
    ("11386", q11386),
    ("12847", q12847),
    ("10102", q10102),
    ("13086", q13086),
    ("12322", q12322),
    ("14801", q14801),
    ("14370", q14370),
    ("10098", q10098),
    ("14352", q14352),
    ("7720", q7720),
    ("11120", q11120),
    ("12844", q12844),
    ("10882", q10882),
    ("12169", q12169),
    ("12757", q12757),
    ("12479", q12479),
    ("10926", q10926),
    ("14521", q14521),
    ("29008", q29008),
    ("14623", q14623),
    ("14966", q14966),
    ("13495", q13495),
    ("10979", q10979),
    ("9557", q9557),
    ("13170", q13170),
    ("14608", q14608),
];
