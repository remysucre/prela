use harness::prelude::*;
use std::cmp::Reverse;

fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|(a, _)| a).inv().map(|(_, b): (A, B)| b).collect()
}

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

fn runs(s: Str) -> Vec<Str> {
    s.split(['<', '>']).collect()
}

/// The (post, tag) pairs with `p.Tags LIKE '%' || t.TagName || '%'`. A name free of '<', '>', '%' and '_' can only occur inside one
/// run of Tags between angle brackets, so it is matched against those runs; any other name goes through a real LIKE over every distinct Tags string.
fn like_tags(db: &'static So) -> MatSet<(Id<Post>, Id<Tag>)> {
    let name = &db.tag.tag_name;
    let plain = |n: Str| !n.contains(['<', '>', '%', '_']);
    let named: HashIdx<Str, Id<Tag>> = db.tag.with(name.filt(plain)).select(name).inv().collect();
    let odd: HashIdx<Str, Id<Tag>> = db.tag.with(name.filt(move |n| !plain(n))).select(name).inv().collect();
    let tags = &db.post.tags_str;
    let rs: MatSet<Str> = tags.flat_map(runs).collect();
    let inside: HashIdx<Str, Id<Tag>> = (&rs).select_where(&named, |r: Str, n: Str| r.contains(n)).collect();
    let strs: MatSet<Str> = tags.collect();
    let hit: HashIdx<Str, Id<Tag>> = (&strs).select_where(&odd, |s: Str, n: Str| like(s, &format!("%{n}%"))).collect();
    db.post.select(Ident::<Post>::new().and(tags.flat_map(runs).select(&inside))).union(db.post.select(Ident::<Post>::new().and(tags.select(&hit)))).collect()
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        COUNT(c.Id) AS CommentCount, RANK() OVER (ORDER BY p.ViewCount DESC) AS RankByViews
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, u.DisplayName),
// RecentPostHistory AS (SELECT ph.PostId, STRING_AGG(ph.UserDisplayName || ' edited at ' || ph.CreationDate || ': ' || ph.Comment, ', ') AS EditHistory
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId),
// TopTags AS (SELECT t.TagName, COUNT(pt.Id) AS UseCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     GROUP BY t.TagName ORDER BY UseCount DESC LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName, rp.AnswerCount, rp.CommentCount, rp.RankByViews, rph.EditHistory,
//        tt.TagName AS TopTag, tt.UseCount AS TagUseCount
// FROM RankedPosts rp LEFT JOIN RecentPostHistory rph ON rp.PostId = rph.PostId CROSS JOIN TopTags tt ORDER BY rp.RankByViews ASC, rp.CreationDate DESC LIMIT 100;
//
// RankByViews reads only ViewCount, so the questions are ranked first and the answer x comment product is driven for the first hundred alone.
// The STRING_AGG has no ORDER BY; the port joins in PostHistory id order.
fn q27162(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, .. } = &db.post;
    let w = whole(db.post.with(post_type_id.eq(1))).select(Ident::<Post>::new().and(view_count.opt())).window(rank, |(_, w)| (w.is_none(), Reverse(w)), asc);
    let r = top_n(drain((&w).map(|((p, _), k)| (p, k))), |&(_, (p, k))| (k, Reverse(creation_date.get(p).unwrap()), p), 100);
    let rp: MatSet<(Id<Post>, i64)> = rel(r.into_iter().map(|x| x.1).collect()).map(|x| x).collect();
    let by_post = by_first(&rp);
    let tp: MatSet<Id<Post>> = (&rp).map(|(p, _)| p).collect();
    let cnt = (&tp)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (x, c)| [a[0] + x.is_some() as i64, a[1] + c.is_some() as i64]);
    let PostHistory { post_history_type_id, user_display_name, comment, creation_date: hd, .. } = &db.post_history;
    let edits = Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6])).with(user_display_name).with(comment);
    let eh = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(edits)).buf_fold(|it| {
        let mut h: Vec<Id<PostHistory>> = it.into_iter().collect();
        h.sort_unstable();
        let s: Vec<String> = h.into_iter().map(|h| format!("{} edited at {}: {}", user_display_name.get(h).unwrap(), ts_text(hd.get(h).unwrap()), comment.get(h).unwrap())).collect();
        &*Box::leak(s.join(", ").into_boxed_str())
    });
    let tm = like_tags(db);
    let uc = (&tm).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t).select(&db.tag.tag_name)).select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p).select(&db.post.post_type)).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&uc), |&(t, n)| (Reverse(n), t), 10));
    let rows_ = (&by_post).and(&cnt).and((&eh).opt());
    let mut v = Vec::new();
    rows_.cross(&tt).drive(|(p, _), (((k, a), e), (t, n))| v.push((p, k, a, e, t, n)));
    let v = top_n(v, |&(p, k, _, _, t, _)| (k, Reverse(creation_date.get(p).unwrap()), p, t), 100);
    rows(v.into_iter().map(|(p, k, a, e, t, n)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(k), ostr(e), V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank, COALESCE(b.Class, 0) AS BadgeClass,
//        p.OwnerUserId FROM Posts p LEFT JOIN Badges b ON p.OwnerUserId = b.UserId AND b.Class = 1 WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostStats AS (SELECT OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, AVG(COALESCE(ViewCount, 0)) AS AvgViews FROM Posts GROUP BY OwnerUserId),
// HighScorers AS (SELECT rp.OwnerUserId, rp.Title, rp.CreationDate, rp.Score, ps.TotalPosts, ps.PositivePosts, ps.AvgViews FROM RankedPosts rp JOIN PostStats ps ON rp.OwnerUserId = ps.OwnerUserId
//     WHERE rp.Rank <= 5)
// SELECT u.DisplayName, hs.Title, hs.CreationDate, hs.Score, hs.TotalPosts, hs.PositivePosts, hs.AvgViews,
//        CASE WHEN hs.AvgViews IS NULL THEN 'No Data' WHEN hs.AvgViews < 100 THEN 'Low Engagement' WHEN hs.AvgViews BETWEEN 100 AND 500 THEN 'Moderate Engagement' ELSE 'High Engagement' END AS EngagementLevel,
//        CONCAT_WS(' | ', COALESCE(NULLIF(u.Location, ''), 'Location Unknown'), COALESCE(NULLIF(u.WebsiteUrl, ''), 'No Website')) AS UserDetails
// FROM HighScorers hs JOIN Users u ON u.Id = hs.OwnerUserId ORDER BY hs.Score DESC, hs.TotalPosts DESC LIMIT 10;
//
// The rank numbers the post x gold-badge rows; rows of one post tie, and the port breaks ties by post then badge id.
fn q1892(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let j: MatSet<(Id<Post>, Option<Id<Badge>>)> =
        db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(Ident::<Post>::new().and(owner_user.select(gold).opt())).collect();
    type J = (Id<Post>, Option<Id<Badge>>);
    let post_of = Same::<J>::new().map(|(p, _): J| p);
    let w = (&j).group_by((&post_of).select(owner_user)).select(Same::<J>::new().and((&post_of).select(score))).window(row_number, |((p, b), s)| (Reverse(s), p, b), asc);
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + w.unwrap_or(0)]);
    let v = drain((&w).filt(|(_, r)| r <= 5).map(|((pb, _), _)| pb).and(&ps));
    let v = top_n(v, |&(_, ((p, b), a))| (Reverse(score.get(p).unwrap()), Reverse(a[0]), p, b), 10);
    let nz = |s: Option<Str>| s.filter(|s| !s.is_empty());
    rows(v.into_iter().map(|(u, ((p, _), a))| {
        let avgv = a[2] as f64 / a[0] as f64;
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[0])]);
        f.push(V::S(if avgv < 100.0 { "Low Engagement" } else if avgv <= 500.0 { "Moderate Engagement" } else { "High Engagement" }));
        f.push(V::Owned(format!("{} | {}", nz(db.user.location.get(u)).unwrap_or("Location Unknown"), nz(db.user.website_url.get(u)).unwrap_or("No Website"))));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        AVG(P.Score) AS AvgScore, SUM(P.ViewCount) AS TotalViews FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        ROW_NUMBER() OVER (PARTITION BY CASE WHEN TotalPosts = 0 THEN 'NoPosts' ELSE 'WithPosts' END ORDER BY Reputation DESC) AS PostRank FROM UserStats),
// RecentVotes AS (SELECT V.PostId, V.UserId, V.CreationDate, vt.Name AS VoteTypeName, COUNT(*) OVER (PARTITION BY V.PostId) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY V.PostId ORDER BY V.CreationDate DESC) AS RecentVoteRank
//     FROM Votes V JOIN VoteTypes vt ON V.VoteTypeId = vt.Id WHERE V.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'))
// SELECT T.UserId, T.DisplayName, T.Reputation, T.TotalPosts, T.Questions, T.Answers, T.AcceptedAnswers, T.AvgScore, T.TotalViews, R.VoteTypeName, R.VoteCount
// FROM TopUsers T LEFT JOIN RecentVotes R ON T.UserId = R.UserId AND R.RecentVoteRank <= 5
// WHERE T.ReputationRank <= 10 OR (T.TotalPosts = 0 AND T.Reputation > 100) ORDER BY T.ReputationRank, T.UserId;
//
// RecentVoteRank ties on votes cast the same day; the port breaks them by vote id. PostRank is never read.
fn q2043(db: &'static So) -> String {
    let ups = user_posts(db);
    let Post { accepted_answer_id, post_type_id, .. } = &db.post;
    let acc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)).with(accepted_answer_id)).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rw = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let rr: MatSet<(Id<User>, i64)> = (&rw).map(|((u, _), k)| (u, k)).collect();
    let rank = by_first(&rr);
    let Vote { creation_date: vd, user, .. } = &db.vote;
    let recent = || db.vote.with(vd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let vc = recent().group_by(&db.vote.post_id).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let vw = recent().group_by(&db.vote.post_id).select(Ident::<Vote>::new().and(vd)).window(row_number, |(v, d)| (Reverse(d), v), asc);
    let rv: MatSet<Id<Vote>> = (&vw).filt(|(_, n)| n <= 5).map(|((v, _), _)| v).collect();
    let by_user: HashIdx<Id<User>, Id<Vote>> = (&rv).select(user).inv().collect();
    let tu = db.user.with(
        (&ups)
            .and(&rank)
            .and(&db.user.reputation)
            .filt(|((a, k), r): (([i64; 10], i64), i64)| k <= 10 || (a[1] == 0 && r > 100)),
    );
    let v = drain(tu.select((&ups).and(&acc).and(&rank).and((&by_user).select(Ident::<Vote>::new().and((&db.vote.post_id).select(&vc))).opt())));
    rows(v.into_iter().map(|(u, (((a, c), _), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(c), avg(a[4], a[1]), nullable(a[6], a[5])]);
        f.extend(match r {
            Some((v, n)) => [V::S(vtype_name(db).get(v).unwrap()), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// MostActiveUsers AS (SELECT us.UserId, us.DisplayName, us.UpVotes, us.DownVotes, us.PostCount, RANK() OVER (ORDER BY us.PostCount DESC) AS UserRank FROM UserStats us WHERE us.PostCount > 10),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(c.CloseReason, 'Not Closed') AS CloseReason, mu.DisplayName AS TopUser, mu.UpVotes, mu.DownVotes, mu.PostCount
// FROM RankedPosts rp LEFT JOIN ClosedPosts c ON rp.PostId = c.PostId JOIN MostActiveUsers mu ON mu.UserId = rp.PostId % (SELECT COUNT(*) FROM Users)
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// `mu.UserId = rp.PostId % (SELECT COUNT(*) FROM Users)` joins a computed raw id, so it goes through the raw user ids. The Rank ties on score are broken by post id.
fn q3049(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 5).map(|((p, _), _)| p).collect();
    let n = count(&db.user.origid);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |k, p| k + p.is_some() as i64);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mu = Ident::<User>::new().and(&us).and((&pc).filt(|k| k > 10));
    let closed = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let mut v = drain((&rp).select(closed.opt().and(origid.map(move |o: i64| o % n).select(&uid).select(mu))));
    v.sort_by_key(|&(p, _)| {
        let w = db.post.view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (h, ((u, a), k)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("Not Closed")));
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(k)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.Score > 0 THEN P.Score ELSE 0 END) AS TotalScore,
//        COUNT(DISTINCT C.Id) AS CommentCount, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, AVG(COALESCE(P.ViewCount, 0)) AS AvgViewCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostHistoryDetails AS (SELECT PH.PostId, PH.UserId, PH.CreationDate, P.Title, P.Body, PH.Comment, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS HistoryRank
//     FROM PostHistory PH INNER JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId IN (10, 11, 12)),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, CommentCount, BadgeCount, AvgViewCount, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS UserRank FROM UserActivity)
// SELECT TU.UserId, TU.DisplayName, TU.PostCount, TU.TotalScore, TU.CommentCount, TU.BadgeCount, TU.AvgViewCount, PHD.PostId, PHD.Title, PHD.Body, PHD.CreationDate AS PostHistoryDate,
//        PHD.Comment AS HistoryComment
// FROM TopUsers TU LEFT JOIN PostHistoryDetails PHD ON TU.UserId = PHD.UserId WHERE TU.UserRank <= 5 AND PHD.HistoryRank = 1 ORDER BY TU.TotalScore DESC, PHD.PostId;
//
// HistoryRank ties when a post has two close/reopen/delete events at the same instant; the port breaks the tie by history id.
fn q2270(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (s, w) = p.map_or((0, 0), |((s, w), _)| (s.max(0), w.unwrap_or(0)));
            [a[0] + s, a[1] + b.is_some() as i64, a[2] + w, a[3] + 1]
        });
    let pc = user_posts(db);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tu = top_n(drain(&ua), |&(u, a)| (Reverse(a[0]), u), 5);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let w = db.post_history.with(post_history_type_id.is_in([10, 11, 12])).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (Reverse(d), h), asc);
    let first: MatSet<Id<PostHistory>> = (&w).filt(|(_, n)| n == 1).map(|((h, _), _)| h).collect();
    let by_user: HashIdx<Id<User>, Id<PostHistory>> = (&first).select(user).inv().collect();
    let v = drain((&tu).select((&ua).and(&pc).and(&cc).and(&by_user)));
    let mut v: Vec<_> = v.into_iter().map(|(u, (((a, p), c), h))| (u, a, p[1], c, h, post.get(h).unwrap())).collect();
    v.sort_by_key(|&(_, a, _, _, _, p)| (Reverse(a[0]), db.post.origid.get(p).unwrap()));
    rows(v.into_iter().map(|(u, a, pn, c, h, p)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(pn), V::I(a[0]), V::I(c), V::I(a[1]), avg(a[2], a[3])]);
        f.extend(post_fields(db, p, &["id", "title", "body"]));
        f.extend([V::T(hd.get(h).unwrap()), ostr(db.post_history.comment.get(h))]);
        row(f)
    }))
}

// WITH TagStats AS (SELECT TRIM(REGEXP_REPLACE(Tags, '<[^>]*>', '', 'g')) AS CleanTags, COUNT(*) AS PostCount, SUM(COALESCE(ViewCount, 0)) AS TotalViews, AVG(SCORE) AS AvgScore
//     FROM Posts WHERE PostTypeId = 1 GROUP BY CleanTags),
// TagCount AS (SELECT CleanTags, PostCount, TotalViews, AvgScore, ROW_NUMBER() OVER (ORDER BY TotalViews DESC, PostCount DESC, AvgScore DESC) AS Rank FROM TagStats WHERE PostCount > 5),
// UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN C.UserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// PopularUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, TotalComments, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS UserRank FROM UserEngagement)
// SELECT TU.CleanTags, TU.PostCount, TU.TotalViews, TU.AvgScore, PU.DisplayName, PU.TotalPosts, PU.TotalAnswers, PU.TotalQuestions, PU.TotalComments
// FROM TagCount TU JOIN PopularUsers PU ON PU.TotalPosts > 10 WHERE TU.Rank <= 10 ORDER BY TU.TotalViews DESC
//
// The ON clause names only PU, so the tag groups and the users with more than ten posts are crossed. UserRank is never read.
fn q26323(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, score, .. } = &db.post;
    let clean = |t: Option<Str>| -> Option<Str> {
        t.map(|t| {
            let mut out = String::new();
            let mut rest = t;
            while let Some(i) = rest.find('<') {
                match rest[i..].find('>') {
                    Some(j) => {
                        out.push_str(&rest[..i]);
                        rest = &rest[i + j + 1..];
                    }
                    None => break,
                }
            }
            out.push_str(rest);
            &*Box::leak(out.trim().to_string().into_boxed_str())
        })
    };
    let tsg = db
        .post
        .with(post_type_id.eq(1))
        .group_by(tags_str.opt().map(clean))
        .select(score.and(view_count.opt()))
        .fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let tc = top_n(drain((&tsg).filt(|a| a[0] > 5)), |&(_, a)| (Reverse(a[1]), Reverse(a[0]), Reverse(fkey(a[2] as f64 / a[0] as f64))), 10);
    let tc = rel(tc);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pu: MatSet<Id<User>> = db.user.with((&pc).filt(|n| n > 10)).collect();
    let ue = (&pu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(comments_of(db).select((&db.comment.user_id).opt()).opt())))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + c.flatten().is_some() as i64]);
    let mut v = Vec::new();
    (&tc).cross((&ue).and(&pc)).drive(|(_, u), ((t, a), (b, n))| v.push((t, a, u, b, n)));
    rows(v.into_iter().map(|(t, a, u, b, n)| row(vec![ostr(t), V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), user_col(db, u, "name"), V::I(n), V::I(b[0]), V::I(b[1]), V::I(b[2])])))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// QuestionStats AS (SELECT P.Id AS QuestionId, P.Title, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title),
// UserReputationChanges AS (SELECT H.UserId, COUNT(H.Id) AS EditsCount, MAX(H.CreationDate) AS LastEditDate FROM PostHistory H WHERE H.PostHistoryTypeId IN (4, 5, 6) GROUP BY H.UserId)
// SELECT U.DisplayName, U.Reputation, U.PostCount, U.AnswerCount, U.QuestionCount, QS.Title, QS.TotalBounty, QS.CommentCount, COALESCE(URC.EditsCount, 0) AS TotalEdits, URC.LastEditDate,
//        CASE WHEN U.Reputation > 1000 THEN 'Gold' WHEN U.Reputation > 500 THEN 'Silver' ELSE 'Bronze' END AS Badge
// FROM UserStats U JOIN QuestionStats QS ON U.QuestionCount > 0 LEFT JOIN UserReputationChanges URC ON U.UserId = URC.UserId
// WHERE U.UserRank <= 10 ORDER BY U.Reputation DESC, QS.TotalBounty DESC;
//
// The ON clause names only U, so the top users with a question are crossed with every question.
fn q3910(db: &'static So) -> String {
    let ups = user_posts(db);
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let PostHistory { user, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let urc = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(user).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let qs = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(bounty.opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (b, c)| [a[0] + b.flatten().unwrap_or(0), a[1] + c.is_some() as i64]);
    let mut v = Vec::new();
    (&tu).select((&ups).filt(|a| a[2] > 0).and((&urc).opt())).cross(&qs).drive(|(u, p), ((a, e), q)| v.push((u, a, e, p, q)));
    rows(v.into_iter().map(|(u, a, e, p, q)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[3]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(q[0]), V::I(q[1]), V::I(e.map_or(0, |e| e.0)), e.map_or(V::Null, |e| V::T(e.1))]);
        f.push(V::S(if r > 1000 { "Gold" } else if r > 500 { "Silver" } else { "Bronze" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes - DownVotes AS NetVotes, RANK() OVER (ORDER BY UpVotes DESC) AS UpVoteRank FROM UserVoteStats WHERE PostCount > 5 ORDER BY NetVotes DESC),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(PT.Name, 'General') AS PostType, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount
//     FROM Posts P LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title, P.CreationDate, PT.Name),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.PostType, pd.CommentCount, RANK() OVER (ORDER BY pd.CommentCount DESC) AS CommentRank FROM PostDetails pd)
// SELECT tu.UserId, tu.DisplayName, tu.NetVotes, tp.PostId, tp.Title, tp.CreationDate, tp.PostType, tp.CommentCount
// FROM TopUsers tu FULL OUTER JOIN TopPosts tp ON tu.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)
// WHERE tu.UpVoteRank <= 10 OR tp.CommentRank <= 10 ORDER BY tu.NetVotes DESC, tp.CommentCount DESC;
//
// The FULL OUTER JOIN is the matched (user, owned post) pairs, the posts whose owner is not a top user, and the top users who own no post.
// The badge fan-out does not change PostCount, and UpVotes/DownVotes are summed over the vote x badge rows as the SQL does.
fn q835(db: &'static So) -> String {
    let Vote { vote_type_id, post, .. } = &db.vote;
    let uvs = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let dp = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(post)).count_distinct();
    let uw = whole(db.user.with((&dp).filt(|n| n > 5))).select(Ident::<User>::new().and(&uvs)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let tu: MatSet<(Id<User>, i64, i64)> = (&uw).map(|((u, a), r)| (u, a[0] - a[1], r)).collect();
    let tidx: HashIdx<Id<User>, (Id<User>, i64, i64)> = (&tu).map(|(u, _, _)| u).inv().collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pw = whole(db.post.iq()).select(Ident::<Post>::new().and(&cc)).window(rank, |(_, n)| Reverse(n), asc);
    let tp: MatSet<(Id<Post>, i64, i64)> = (&pw).map(|((p, n), r)| (p, n, r)).collect();
    type P = (Id<Post>, i64, i64);
    let owner = Same::<P>::new().map(|(p, _, _): P| p).select(&db.post.owner_user).select(&tidx);
    let pairs = drain((&tp).select(Same::<P>::new().and(owner.opt())).filt(|((_, _, cr), t): (P, Option<(Id<User>, i64, i64)>)| cr <= 10 || t.map_or(false, |t| t.2 <= 10)));
    let owners: MatSet<Id<User>> = db.post.select(&db.post.owner_user).collect();
    type O = (Option<(Id<User>, i64)>, Option<(Id<Post>, i64)>);
    let lone = rel(drain((&tidx).filt(|(_, _, r): (Id<User>, i64, i64)| r <= 10).minus(&owners)));
    let left = rel(pairs).map(|(_, ((p, n, _), t))| -> O { (t.map(|t| (t.0, t.1)), Some((p, n))) });
    let right = (&lone).map(|(_, (u, net, _))| -> O { (Some((u, net)), None) });
    let mut v: Vec<O> = drain(left.map(|x| x).union(right)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(t, p)| (t.is_none(), Reverse(t.map(|t| t.1)), p.is_none(), Reverse(p.map(|p| p.1))));
    rows(v.into_iter().map(|(t, p)| {
        let mut f = match t {
            Some((u, net)) => vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(net)],
            None => vec![V::Null, V::Null, V::Null],
        };
        f.extend(match p {
            Some((p, n)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "type"]);
                g.push(V::I(n));
                g
            }
            None => (0..5).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(COALESCE(v.VoteCount, 0)) AS AvgVotes, RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// RecentActivity AS (SELECT PostId, COUNT(*) AS CommentCount, MAX(CreationDate) AS LastCommentDate FROM Comments GROUP BY PostId)
// SELECT up.UserId, up.DisplayName, up.PostCount, up.QuestionCount, up.AnswerCount, up.AvgVotes, cp.LastClosedDate, ra.CommentCount, ra.LastCommentDate,
//        CASE WHEN up.UserRank <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserCategory
// FROM UserPostStats up LEFT JOIN ClosedPosts cp ON up.UserId = cp.PostId LEFT JOIN RecentActivity ra ON up.UserId = ra.PostId
// WHERE up.PostCount > 0 AND (ra.LastCommentDate IS NULL OR ra.LastCommentDate > '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days')
// ORDER BY up.UserRank ASC, up.PostCount DESC;
//
// `up.UserId = cp.PostId` and `up.UserId = ra.PostId` join a user id to a post id, so they go through the raw ids. VoteCount keys on the raw Votes.PostId.
fn q3224(db: &'static So) -> String {
    let Post { post_type_id, origid, .. } = &db.post;
    let vc = db.vote.group_by(&db.vote.post_id).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(origid.select(&vc).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, n)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + n.unwrap_or(0)],
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let rk: MatSet<(Id<User>, [i64; 4], i64)> = (&w).map(|((u, a), r)| (u, a, r)).collect();
    let PostHistory { post_history_type_id, creation_date: hd, post_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post_id).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let Comment { post_id: cpid, creation_date: cd, .. } = &db.comment;
    let ra = db.comment.group_by(cpid).select(cd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    type R = (Id<User>, [i64; 4], i64);
    let uo = || Same::<R>::new().map(|(u, _, _): R| db.user.origid.get(u).unwrap());
    let v = drain(
        (&rk)
            .select(Same::<R>::new().and(uo().select(&cp).opt()).and(uo().select(&ra).opt()))
            .filt(|((r, _), a): ((R, Option<i64>), Option<(i64, i64)>)| r.1[0] > 0 && a.map_or(true, |a| a.1 > since)),
    );
    rows(v.into_iter().map(|(_, (((u, a, r), c), x))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), c.map_or(V::Null, V::T)]);
        f.extend(match x {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if r <= 10 { "Top User" } else { "Regular User" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount),
// RankedPosts AS (SELECT rp.*, DENSE_RANK() OVER (ORDER BY rp.Score DESC, rp.TotalBounty DESC) AS ScoreRank FROM RecentPosts rp),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END, 0)) AS GoldBadges, SUM(COALESCE(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END, 0)) AS SilverBadges,
//        SUM(COALESCE(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END, 0)) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.TotalBounty, rp.CommentCount, u.DisplayName AS TopUser, u.GoldBadges, u.SilverBadges, u.BronzeBadges,
//        CASE WHEN rp.ViewCount IS NULL THEN 'No views' WHEN rp.ViewCount > 1000 THEN 'Popular' ELSE 'Normal' END AS Popularity
// FROM RankedPosts rp JOIN TopUsers u ON rp.OwnerUserId = u.UserId WHERE rp.ScoreRank <= 10 ORDER BY rp.Score DESC, rp.TotalBounty DESC;
fn q3923(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(bounty.opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (b, c)| [a[0] + b.flatten().unwrap_or(0), a[1] + c.is_some() as i64]);
    let w = whole(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(Ident::<Post>::new().and(score).and(&rp)).window(dense_rank, |((_, s), a)| (Reverse(s), Reverse(a[0])), asc);
    let top = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), a), _)| (p, a));
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    type T = (Id<Post>, [i64; 2]);
    let mut v = drain((&top).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select(owner_user).select(Ident::<User>::new().and(&ub)))));
    v.sort_by_key(|&(_, ((p, a), _))| (Reverse(score.get(p).unwrap()), Reverse(a[0])));
    rows(v.into_iter().map(|(_, ((p, a), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), user_col(db, u, "name")]);
        f.extend(b.map(V::I));
        f.push(V::S(match view_count.get(p) {
            None => "No views",
            Some(w) if w > 1000 => "Popular",
            _ => "Normal",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        AVG(COALESCE(p.Score, 0)) AS AveragePostScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, rp.CommentCount, um.UserId, um.DisplayName, um.Reputation, um.TotalPosts,
//        um.PositivePosts, um.AveragePostScore
// FROM RankedPosts rp JOIN Users u ON u.Id = rp.PostId JOIN UserMetrics um ON u.Id = um.UserId WHERE rp.UserPostRank <= 5 ORDER BY um.Reputation DESC, rp.Score DESC;
//
// UserPostRank reads only base columns, so each owner's five newest posts are picked first (ties by post id) and the vote x comment x badge product is driven for those alone.
// `u.Id = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q8360(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 5).map(|((p, _), _)| p).collect();
    let bu: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user_id.select(&bu).opt()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let ups = user_posts(db);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mut v = drain((&s).and(origid.select(&uid).select(Ident::<User>::new().and(&ups))));
    v.sort_by_key(|&(p, (_, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(db.post.score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (a, (u, m)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.extend([V::I(m[1]), V::I(m[8]), avg(m[4], m[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostVoteStats AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, u.DisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, pvs.UpVotes, pvs.DownVotes, pvs.TotalVotes, CASE WHEN rp.RankByScore <= 10 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM RankedPosts rp LEFT JOIN Users u ON rp.PostId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId
// WHERE (u.Reputation IS NOT NULL AND u.Reputation > 100) AND (pvs.UpVotes IS NOT NULL OR pvs.DownVotes IS NOT NULL) ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 50;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids; PostVoteStats is keyed by the raw Votes.PostId. RankByScore ties are broken by post id.
fn q2102(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let w = recent().group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let pvs = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type K = (Id<Post>, i64);
    let pk = || Same::<K>::new().map(|(p, _): K| p).select(origid);
    let user = pk().select(&uid).select(Ident::<User>::new().with((&db.user.reputation).gt(100)).and((&ub).opt()));
    let v = drain((&rk).select(Same::<K>::new().and(user).and(pk().select(&pvs))));
    let v = top_n(v, |&(_, (((p, _), _), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(_, (((p, r), (u, b)), a))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(user_col(db, u, "name"));
        f.extend(b.map(V::I));
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankPerUser
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE u.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '3 months' GROUP BY u.Id, u.DisplayName HAVING SUM(p.Score) > 100),
// RecentActivity AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(*) AS ActivityCount FROM Votes v JOIN Posts p ON v.PostId = p.Id
//     WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 week' GROUP BY p.Id, p.OwnerUserId)
// SELECT u.DisplayName, u.Reputation, r.Title, r.CreationDate, r.Score, COALESCE(ac.ActivityCount, 0) AS RecentActivityCount, rt.TotalScore
// FROM Users u JOIN RankedPosts r ON u.Id = r.PostId LEFT JOIN RecentActivity ac ON r.PostId = ac.PostId JOIN TopUsers rt ON u.Id = rt.UserId
// WHERE r.RankPerUser = 1 AND (u.Location IS NOT NULL OR u.WebsiteUrl IS NOT NULL) ORDER BY rt.TotalScore DESC, u.Reputation DESC LIMIT 50;
//
// `u.Id = r.PostId` joins a user id to a post id, so it goes through the raw ids. RankPerUser ties are broken by post id. CommentCount is never projected.
fn q3655(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, score, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let tu = db.user.with((&db.user.creation_date).lt(add_months(t0, -3))).group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold(0i64, |s, x| s + x);
    let ra = db.vote.with((&db.vote.creation_date).ge(add_days(t0, -7))).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let located = Ident::<User>::new().with((&db.user.location).map(|_| ()).or((&db.user.website_url).map(|_| ())));
    let v = drain((&first).select(origid.select(&uid).select(located).select(Ident::<User>::new().and((&tu).filt(|s| s > 100))).and((&ra).opt())));
    let v = top_n(v, |&(p, ((u, t), _))| (Reverse(t), Reverse(db.user.reputation.get(u).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((u, t), n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(n.unwrap_or(0)), V::I(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.AnswerCount, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AnswerStats AS (SELECT p.Id AS PostId, COUNT(a.Id) AS AnswerCount, AVG(a.Score) AS AvgScore FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1 GROUP BY p.Id),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 'Closed/Reopened' ELSE 'Other' END AS ChangeType
//     FROM PostHistory ph WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months')
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.AnswerCount AS TotalAnswers, COALESCE(a.AnswerCount, 0) AS TotalAnswersFromStats,
//        COALESCE(a.AvgScore, 0) AS AvgAnswerScore, ph.ChangeType, ph.CreationDate AS ChangeDate
// FROM RankedPosts rp LEFT JOIN AnswerStats a ON rp.PostId = a.PostId LEFT JOIN PostHistoryDetails ph ON rp.PostId = ph.PostId
// WHERE rp.OwnerPostRank = 1 AND (ph.ChangeType IS NULL OR ph.ChangeType = 'Closed/Reopened') ORDER BY rp.CreationDate DESC;
//
// OwnerPostRank ties are broken by post id.
fn q4559(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let ast = (&first).group_by(Ident::<Post>::new()).select(children_of(db).select(score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_months(t0, -6))).and(post_history_type_id)).opt();
    let keep = |h: Option<(Id<PostHistory>, i64)>| h.map_or(true, |(_, t)| matches!(t, 10 | 11));
    let mut v = drain((&ast).and(ph.filt(keep).map(|h: Option<(Id<PostHistory>, i64)>| h.map(|(h, _)| h))));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "answers"]);
        f.extend([V::I(a[0]), if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }]);
        f.extend(match h {
            Some(h) => [V::S("Closed/Reopened"), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserVotes AS (SELECT U.Id AS UserId, U.DisplayName, V.PostId, V.VoteTypeId, V.CreationDate, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY V.CreationDate DESC) AS VoteRank
//     FROM Users U JOIN Votes V ON U.Id = V.UserId WHERE U.Reputation > 1000),
// PostStats AS (SELECT P.Id AS PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(COALESCE(P.Score, 0)) AS TotalScore, COUNT(DISTINCT C.UserId) AS UniqueCommenters
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id),
// RecentActivity AS (SELECT P.Id AS PostId, P.CreationDate, P.LastActivityDate, P.Title, COALESCE(H.Comment, 'No comments') AS LastEditComment,
//        ROW_NUMBER() OVER (ORDER BY P.LastActivityDate DESC) AS ActivityRank FROM Posts P LEFT JOIN PostHistory H ON P.Id = H.PostId AND H.PostHistoryTypeId = 24)
// SELECT PS.PostId, R.Title, R.CreationDate, R.LastActivityDate, PS.UpVotes, PS.DownVotes, PS.CommentCount, PS.TotalScore, PS.UniqueCommenters, UA.DisplayName AS LastVoter,
//        UA.VoteTypeId AS LastVoteType, R.LastEditComment
// FROM PostStats PS JOIN RecentActivity R ON PS.PostId = R.PostId LEFT JOIN UserVotes UA ON UA.PostId = PS.PostId AND UA.VoteRank = 1
// WHERE PS.UpVotes - PS.DownVotes > 5 ORDER BY PS.TotalScore DESC, R.LastActivityDate DESC LIMIT 100;
//
// WITH RECURSIVE, but no CTE refers to itself. ActivityRank is never read. VoteRank ties on votes cast the same day; the port breaks them by vote id.
fn q34951(db: &'static So) -> String {
    let Post { score, last_activity_date, .. } = &db.post;
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()))
        .fold([0i64; 4], |a, ((s, t), c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64, a[3] + s]);
    let ucd = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.user_id)).count_distinct();
    let uc = db.post.select((&ucd).opt().map(|n: Option<i64>| n.unwrap_or(0)));
    let Vote { user, creation_date: vd, .. } = &db.vote;
    let uw = db.vote.with(user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).group_by(user).select(Ident::<Vote>::new().and(vd)).window(row_number, |(v, d)| (Reverse(d), v), asc);
    let last: MatSet<Id<Vote>> = (&uw).filt(|(_, n)| n == 1).map(|((v, _), _)| v).collect();
    let by_post: HashIdx<Id<Post>, Id<Vote>> = (&last).select(&db.vote.post).inv().collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(24))).opt();
    let v = drain((&ps).filt(|a| a[0] - a[1] > 5).and(&uc).and(edits).and((&by_post).opt()));
    let v = top_n(v, |&(p, (((a, _), h), x))| (Reverse(a[3]), Reverse(last_activity_date.get(p).unwrap()), p, h, x), 100);
    rows(v.into_iter().map(|(p, (((a, n), h), x))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(n)]);
        f.extend(match x {
            Some(x) => [user_col(db, user.get(x).unwrap(), "name"), V::I(db.vote.vote_type_id.get(x).unwrap())],
            None => [V::Null, V::Null],
        });
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No comments")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
//        COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS UserRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// AggregatedUserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '2 years'
//     GROUP BY u.Id, u.DisplayName)
// SELECT p.Title, p.CreationDate, p.Score, p.CommentCount, p.UpVoteCount, p.DownVoteCount, u.DisplayName AS OwnerDisplayName, u.GoldBadges, u.SilverBadges, u.BronzeBadges, u.TotalPosts,
//        u.TotalScore, p.UserRank
// FROM RankedPosts p JOIN AggregatedUserStats u ON p.PostId = u.UserId WHERE p.UserRank <= 5 ORDER BY p.Score DESC, p.CreationDate DESC;
//
// UserRank reads only base columns, so each owner's top five questions are picked first (ties by post id) and the comment x vote product is driven for those alone.
// `p.PostId = u.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q8572(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, score, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1))))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<(Id<Post>, i64)> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), k)| (p, k)).collect();
    let by_post = by_first(&tp);
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let s = (&tps)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let recent = || db.user.with((&db.user.creation_date).ge(add_years(t0, -2)));
    let us = recent()
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(score).opt()))
        .fold([0i64; 5], |a, (c, s)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + s.is_some() as i64, a[4] + s.unwrap_or(0)]);
    let dp = recent().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mut v = drain((&s).and(&by_post).and(origid.select(&uid).select(Ident::<User>::new().and(&us).and(&dp))));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((a, k), ((u, b), n)))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend(a.map(V::I));
        f.push(user_col(db, u, "name"));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(n), nullable(b[4], b[3]), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.PostTypeId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id, u.Reputation, u.DisplayName),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount, MIN(ph.CreationDate) AS FirstClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.Title, rp.CreationDate, rp.Score, COALESCE(pc.CommentCount, 0) AS TotalComments, r.UserId, r.DisplayName AS UserDisplayName, r.Reputation, r.BadgeCount,
//        COALESCE(cp.CloseCount, 0) AS NumberOfClosures, cp.FirstClosedDate, CASE WHEN cp.CloseCount IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RankedPosts rp JOIN UserReputation r ON r.UserId = rp.OwnerUserId LEFT JOIN PostComments pc ON pc.PostId = rp.Id LEFT JOIN ClosedPosts cp ON cp.PostId = rp.Id
// WHERE rp.PostRank <= 5 ORDER BY rp.Score DESC, rp.CreationDate ASC;
fn q1507(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 5).map(|((p, _), _)| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    let mut v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&bc)).and((&pc).opt()).and((&cp).opt())));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (((u, b), c), x))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.push(V::I(c.unwrap_or(0)));
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.push(V::I(b));
        f.extend(match x {
            Some((n, d)) => [V::I(n), V::T(d), V::S("Closed")],
            None => [V::I(0), V::Null, V::S("Open")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes FROM Votes v GROUP BY v.PostId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, uv.Upvotes, uv.Downvotes, pc.CommentCount, COALESCE(rp.OwnerUserId, -1) AS OwnerUserId, rp.PostRank
//     FROM RankedPosts rp LEFT JOIN UserVotes uv ON rp.PostId = uv.PostId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId)
// SELECT pd.Title, pd.CreationDate, pd.ViewCount, pd.Score, pd.Upvotes, pd.Downvotes, pd.CommentCount,
//        CASE WHEN pd.Score IS NULL THEN 'No Score' WHEN pd.Score >= 10 THEN 'High Score' ELSE 'Low Score' END AS ScoreCategory, u.DisplayName AS OwnerDisplayName
// FROM PostDetails pd LEFT JOIN Users u ON pd.OwnerUserId = u.Id WHERE pd.PostRank = 1 AND pd.ViewCount > 100 AND u.Reputation IS NOT NULL
// ORDER BY pd.Score DESC, pd.ViewCount DESC LIMIT 50;
//
// `COALESCE(rp.OwnerUserId, -1)` sends ownerless posts to user -1, so the owner is looked up by raw id. UserVotes is keyed by the raw Votes.PostId.
fn q1900(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, owner_user_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let uv = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let owner = owner_user_id.opt().map(|o: Option<i64>| o.unwrap_or(-1)).select(&uid);
    let v = drain((&tp).with(view_count.gt(100)).select(origid.select(&uv).opt().and((&pc).opt()).and(owner)));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p), 50);
    rows(v.into_iter().map(|(p, ((a, c), u))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.extend([oint(c), V::S(if s >= 10 { "High Score" } else { "Low Score" }), user_col(db, u, "name")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) OVER (PARTITION BY p.Id) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) OVER (PARTITION BY p.Id) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentPostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.CreationDate >= (SELECT DATE_TRUNC('month', cast('2024-10-01' as date)) - INTERVAL '6 months'))
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, CASE WHEN rph.PostHistoryTypeId IS NOT NULL THEN 'Edited/Closed/Deleted' ELSE 'Active' END AS PostStatus,
//        CASE WHEN rp.Rank <= 3 THEN 'Top Post' WHEN rp.Rank BETWEEN 4 AND 10 THEN 'Middle Post' ELSE 'Bottom Post' END AS PostCategory,
//        COALESCE(CAST(rp.UpVotes AS VARCHAR), '0') || ' Upvotes' AS UpvoteString
// FROM RankedPosts rp LEFT JOIN RecentPostHistory rph ON rp.PostId = rph.PostId AND rph.HistoryRank = 1
// WHERE (rp.PostTypeId = 1 AND rp.Score > 0) OR (rp.PostTypeId = 2 AND rp.ViewCount > 10) ORDER BY rp.Score DESC, rp.ViewCount ASC LIMIT 100 OFFSET (SELECT COUNT(*) FROM RankedPosts) / 2;
//
// RankedPosts has no GROUP BY, so it is one row per post x vote; the ranks number those rows (ties broken by post and vote id), and the OFFSET counts them.
fn q24533(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> =
        db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    type J = (Id<Post>, Option<Id<Vote>>);
    let n = count(&j);
    let post_of = Same::<J>::new().map(|(p, _): J| p);
    let vote_of = Same::<J>::new().flat_map(|(_, v): J| v);
    let pv = (&j).group_by(&post_of).select((&vote_of).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let w = (&j).group_by((&post_of).select(post_type_id)).select(Same::<J>::new().and((&post_of).select(score))).window(row_number, |((pv, s), _)| (Reverse(s), pv), asc);
    type K = (Id<Post>, Option<Id<Vote>>, i64);
    let rk: MatSet<K> = (&w).map(|(((p, v), _), k)| (p, v, k)).collect();
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let since = add_months(trunc_month(date(2024, 10, 1)), -6);
    let hw = db.post_history.with(hd.ge(since)).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (Reverse(d), h), asc);
    let lh = (&hw).filt(|(_, n)| n == 1).map(|((h, _), _)| h);
    let pk = || Same::<K>::new().map(|(p, _, _): K| p);
    let kept = Ident::<Post>::new().with(post_type_id.and(score).and(view_count.opt()).filt(|((t, s), w): ((i64, i64), Option<i64>)| (t == 1 && s > 0) || (t == 2 && w.map_or(false, |w| w > 10))));
    let v = drain((&rk).with(pk().select(kept)).select(Same::<K>::new().and(pk().select(&pv).and(pk().select(&lh).opt()))));
    let v = top_n(v, |&((p, x, _), _)| (Reverse(score.get(p).unwrap()), view_count.get(p).is_none(), view_count.get(p), p, x), 0);
    rows(v.into_iter().skip((n / 2) as usize).take(100).map(|(_, ((p, _, k), (a, h)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if h.is_some() { "Edited/Closed/Deleted" } else { "Active" }));
        f.push(V::S(if k <= 3 { "Top Post" } else if k <= 10 { "Middle Post" } else { "Bottom Post" }));
        f.push(V::Owned(format!("{} Upvotes", a[0])));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS Upvotes,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS Downvotes, COALESCE(PLE.Likes, 0) AS RelatedLikes, CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END AS HasAcceptedAnswer
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS Likes FROM PostLinks pl WHERE pl.LinkTypeId = 1 GROUP BY PostId) PLE ON p.Id = PLE.PostId GROUP BY p.Id, HasAcceptedAnswer, PLE.Likes),
// TopPosts AS (SELECT ps.PostId, ps.CommentCount, ps.Upvotes, ps.Downvotes, ROW_NUMBER() OVER (ORDER BY ps.Upvotes DESC, ps.CommentCount DESC) AS Rank FROM PostStats ps WHERE ps.HasAcceptedAnswer = 1),
// Benchmark AS (SELECT tp.PostId, tp.CommentCount, tp.Upvotes, tp.Downvotes, COALESCE(NULLIF(tp.Upvotes - tp.Downvotes, 0), -1) AS VoteBalance FROM TopPosts tp WHERE tp.Rank <= 10)
// SELECT b.PostId, b.CommentCount, b.Upvotes, b.Downvotes, b.VoteBalance, CASE WHEN b.VoteBalance < 0 THEN 'Negative' WHEN b.VoteBalance = 0 THEN 'Neutral' ELSE 'Positive' END AS VoteStatus,
//        CONCAT('Post ID: ', b.PostId, ' - Vote Balance: ', b.VoteBalance) AS Summary
// FROM Benchmark b ORDER BY b.VoteBalance DESC;
//
// The distinct vote counts are a second fold over one row per vote. RelatedLikes is never read.
fn q2358(db: &'static So) -> String {
    let acc = || db.post.with(&db.post.accepted_answer_id);
    let cc = acc().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let dv = acc().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&cc).and(&dv)), |&(p, (c, a))| (Reverse(a[0]), Reverse(c), p), 10);
    let mut v: Vec<_> = v.into_iter().map(|(p, (c, a))| (p, c, a, if a[0] - a[1] == 0 { -1 } else { a[0] - a[1] })).collect();
    v.sort_by_key(|&(_, _, _, b)| Reverse(b));
    rows(v.into_iter().map(|(p, c, a, b)| {
        let id = db.post.origid.get(p).unwrap();
        row(vec![V::I(id), V::I(c), V::I(a[0]), V::I(a[1]), V::I(b), V::S(if b < 0 { "Negative" } else if b == 0 { "Neutral" } else { "Positive" }), V::Owned(format!("Post ID: {id} - Vote Balance: {b}"))])
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, COALESCE(P.AcceptedAnswerId, -1) AS AcceptedAnswer, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        P.ViewCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY P.Id, P.OwnerUserId, P.Title, P.CreationDate, P.AcceptedAnswerId, P.Score, P.ViewCount),
// PostStatistics AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.CommentCount, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = RP.PostId AND V.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = RP.PostId AND V.VoteTypeId = 3) AS DownVotes, RP.OwnerUserId FROM RecentPosts RP)
// SELECT UR.DisplayName, UR.Reputation, PS.Title, PS.CreationDate, PS.Score, PS.ViewCount, PS.CommentCount, PS.UpVotes, PS.DownVotes,
//        (CASE WHEN PS.Score > 0 THEN 'Positive' WHEN PS.Score < 0 THEN 'Negative' ELSE 'Neutral' END) AS PostSentiment
// FROM UserReputation UR JOIN PostStatistics PS ON UR.UserId = PS.OwnerUserId WHERE UR.ReputationRank <= 10 AND (PS.CommentCount > 5 OR PS.ViewCount > 50)
// ORDER BY UR.Reputation DESC, PS.CreationDate DESC;
fn q4815(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user.select(&tu));
    let ps = recent()
        .group_by(Ident::<Post>::new())
        .select(view_count.opt().and(comments_of(db).opt()))
        .fold((0i64, None::<i64>), |(n, _), (w, c)| (n + c.is_some() as i64, w));
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let mut v = drain((&ps).filt(|(c, w): (i64, Option<i64>)| c > 5 || w.map_or(false, |w| w > 50)).and(&vc));
    v.sort_by_key(|&(p, _)| (Reverse(db.user.reputation.get(owner_user.get(p).unwrap()).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((c, _), a))| {
        let s = score.get(p).unwrap();
        let mut f = ucols(db, owner_user.get(p).unwrap(), &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount, COUNT(DISTINCT rp.PostId) AS QuestionsAsked
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN RecentPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.BadgeCount, us.QuestionsAsked, RANK() OVER (ORDER BY us.Reputation DESC, us.BadgeCount DESC) AS Rank FROM UserStats us)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, tu.BadgeCount, tu.QuestionsAsked, COALESCE(rp.CommentCount, 0) AS LastMonthCommentCount, COALESCE(rp.UpVoteCount, 0) AS LastMonthUpVoteCount,
//        COALESCE(rp.DownVoteCount, 0) AS LastMonthDownVoteCount
// FROM TopUsers tu LEFT JOIN RecentPosts rp ON tu.UserId = rp.OwnerUserId WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// The distinct counts are one fold per user over badges and one over recent questions.
fn q30674(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let rq = || Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(rq()).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation).and(&bc).and(&qc)).window(rank, |(((_, r), b), _)| (Reverse(r), Reverse(b)), asc);
    let tu: MatSet<(Id<User>, i64, i64, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((((u, _), b), q), k)| (u, b, q, k)).collect();
    let rp = db
        .post
        .with(rq())
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    type T = (Id<User>, i64, i64, i64);
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _, _, _): T| u).select(posts_of(db).select(&rp)).opt())));
    v.sort_by_key(|&(_, ((_, _, _, k), _))| k);
    rows(v.into_iter().map(|(_, ((u, b, q, k), a))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = vec![V::I(k)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(q), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswer FROM Posts p
//     WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days' AND p.PostTypeId = 1),
// UserEngagement AS (SELECT u.Id AS UserId, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.ViewCount, ue.UserId, ue.CommentCount, ue.TotalUpVotes, ue.TotalDownVotes, ROW_NUMBER() OVER (PARTITION BY ue.UserId ORDER BY rp.ViewCount DESC) AS UserRank
//     FROM RecentPosts rp JOIN UserEngagement ue ON ue.UserId = rp.AcceptedAnswer),
// TopUsers AS (SELECT UserId, SUM(CommentCount) AS TotalComments, SUM(TotalUpVotes) AS TotalVotes FROM UserEngagement GROUP BY UserId HAVING SUM(CommentCount) > 5)
// SELECT ps.Title, ps.ViewCount, tu.TotalComments, tu.TotalVotes, CASE WHEN ps.UserRank = 1 THEN 'Top Post for User' ELSE 'Regular Post' END AS PostCategory
// FROM PostStatistics ps LEFT JOIN TopUsers tu ON ps.UserId = tu.UserId WHERE ps.ViewCount > 50 ORDER BY ps.ViewCount DESC, tu.TotalVotes DESC LIMIT 10;
//
// `ue.UserId = rp.AcceptedAnswer` joins a user id to a post id, so it goes through the raw ids, and UserEngagement is only computed for the users that join
// (TopUsers is read only through the same join). UserRank ties on ViewCount are broken by post id.
fn q3980(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, accepted_answer_id, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ans = || accepted_answer_id.opt().map(|a: Option<i64>| a.unwrap_or(0)).select(&uid);
    let ju: MatSet<Id<User>> = rp().select(ans()).collect();
    let ue = (&ju)
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 1], |a, (_, t)| [a[0] + (t == Some(2)) as i64]);
    let cc = (&ju).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = rp().group_by(ans()).select(Ident::<Post>::new().and(view_count.opt()).and(ans())).window(row_number, |((p, w), _)| (w.is_none(), Reverse(w), p), asc);
    type R = (Id<Post>, Id<User>, i64);
    let ur: MatSet<R> = (&w).map(|(((p, _), u), k)| (p, u, k)).collect();
    let tu = Same::<R>::new().map(|(_, u, _): R| u).select((&cc).filt(|n| n > 5).and(&ue)).opt();
    let v = drain((&ur).with(Same::<R>::new().map(|(p, _, _): R| p).select(view_count.gt(50))).select(Same::<R>::new().and(tu)));
    let v = top_n(v, |&(_, ((p, _, _), t))| (Reverse(view_count.get(p)), t.is_none(), Reverse(t.map(|t| t.1[0])), p), 10);
    rows(v.into_iter().map(|(_, ((p, _, k), t))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend(match t {
            Some((c, a)) => [V::I(c), V::I(a[0])],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if k == 1 { "Top Post for User" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank, COUNT(DISTINCT P.Id) AS PostCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, ReputationRank, PostCount FROM UserReputation WHERE ReputationRank <= 10),
// PostStats AS (SELECT P.Id AS PostId, P.OwnerUserId, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT PL.RelatedPostId) AS RelatedPostCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostLinks PL ON P.Id = PL.PostId GROUP BY P.Id, P.OwnerUserId),
// OverallPostStats AS (SELECT PS.OwnerUserId, COUNT(PS.PostId) AS TotalPosts, SUM(PS.UpVotes - PS.DownVotes) AS NetScore, SUM(PS.CommentCount) AS TotalComments,
//        SUM(PS.RelatedPostCount) AS TotalRelatedPosts FROM PostStats PS GROUP BY PS.OwnerUserId)
// SELECT U.DisplayName, U.Reputation, COALESCE(OPS.TotalPosts, 0) AS TotalPosts, COALESCE(OPS.NetScore, 0) AS NetScore, COALESCE(OPS.TotalComments, 0) AS TotalComments,
//        COALESCE(OPS.TotalRelatedPosts, 0) AS TotalRelatedPosts
// FROM TopUsers U LEFT JOIN OverallPostStats OPS ON U.UserId = OPS.OwnerUserId ORDER BY U.Reputation DESC;
//
// PostStats is read only for the posts of the top users, so the vote x comment x link product is driven for those alone.
fn q2500(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let tp: MatSet<Id<Post>> = (&tu).select(posts_of(db)).map(|p| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(links_of(db).opt()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let rl = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let ops = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&ps).and((&rl).opt())))
        .fold([0i64; 4], |a, (s, d)| [a[0] + 1, a[1] + s[0] - s[1], a[2] + s[2], a[3] + d.unwrap_or(0)]);
    let mut v = drain((&tu).select(Ident::<User>::new().and((&ops).opt())));
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(_, (u, a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(COALESCE(C.CommentsCount, 0)) AS TotalComments, SUM(U.UpVotes) AS TotalUpVotes, SUM(U.DownVotes) AS TotalDownVotes,
//        ROW_NUMBER() OVER (ORDER BY SUM(U.UpVotes) - SUM(U.DownVotes) DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN (SELECT C.PostId, COUNT(C.Id) AS CommentsCount FROM Comments C GROUP BY C.PostId) C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// PostRank AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, DENSE_RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS ScoreRank,
//        ROW_NUMBER() OVER (ORDER BY P.CreationDate DESC) AS RecentPostRank FROM Posts P WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days')
// SELECT UA.DisplayName, UA.TotalPosts, UA.TotalQuestions, UA.TotalAnswers, UA.TotalComments, UA.TotalUpVotes, UA.TotalDownVotes, PR.PostId, PR.Title, PR.CreationDate, PR.ViewCount, PR.Score,
//        PR.ScoreRank, PR.RecentPostRank
// FROM UserActivity UA LEFT JOIN PostRank PR ON UA.UserId = PR.PostId WHERE (UA.TotalQuestions > 0 OR UA.TotalAnswers > 0) AND UA.TotalUpVotes > 5 ORDER BY UA.Rank, PR.Score DESC LIMIT 100;
//
// `UA.UserId = PR.PostId` joins a user id to a post id, so it goes through the raw ids. Rank ties are broken by user id.
fn q1035(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user_id, origid, .. } = &db.post;
    let cp = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let User { up_votes, down_votes, .. } = &db.user;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).select(post_type_id.and((&cp).opt())).opt()))
        .fold([0i64; 6], |a, ((up, dn), p)| {
            let (t, c) = p.map_or((0, 0), |(t, c)| (t, c.unwrap_or(0)));
            [a[0] + p.is_some() as i64, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c, a[4] + up, a[5] + dn]
        });
    let uw = whole(db.user.iq()).select(Ident::<User>::new().and(&ua)).window(row_number, |(u, a)| (Reverse(a[4] - a[5]), u), asc);
    let r: MatSet<(Id<User>, [i64; 6], i64)> = (&uw).map(|((u, a), k)| (u, a, k)).collect();
    let recent = || db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30)));
    let sw = recent().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(dense_rank, |(_, s)| Reverse(s), asc);
    let sr: MatSet<(Id<Post>, i64)> = (&sw).map(|((p, _), s)| (p, s)).collect();
    type S = (Id<Post>, i64);
    let rw = whole(&sr).select(Same::<S>::new().and(Same::<S>::new().map(|(p, _): S| p).select(creation_date))).window(row_number, |((p, _), d)| (Reverse(d), p), asc);
    let pr: MatSet<(i64, Id<Post>, i64, i64)> = (&rw).map(|(((p, s), _), k)| (p, s, k)).select(Same::<(Id<Post>, i64, i64)>::new().and(Same::<(Id<Post>, i64, i64)>::new().map(|(p, _, _): (Id<Post>, i64, i64)| p).select(origid))).map(|((p, s, k), o)| (o, p, s, k)).collect();
    let by_raw: HashIdx<i64, (i64, Id<Post>, i64, i64)> = (&pr).map(|(o, _, _, _)| o).inv().collect();
    type U = (Id<User>, [i64; 6], i64);
    let keep = |(_, a, _): U| (a[1] > 0 || a[2] > 0) && a[4] > 5;
    let v = drain((&r).filt(keep).select(Same::<U>::new().and(Same::<U>::new().map(|(u, _, _): U| db.user.origid.get(u).unwrap()).select(&by_raw).opt())));
    let v = top_n(v, |&(_, ((_, _, k), p))| (k, p.is_none(), Reverse(p.map(|p| score.get(p.1).unwrap()))), 100);
    rows(v.into_iter().map(|(_, ((u, a, _), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[5])];
        f.extend(match p {
            Some((_, p, s, k)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "views", "score"]);
                g.extend([V::I(s), V::I(k)]);
                g
            }
            None => (0..7).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AggregatedVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId),
// CombinedData AS (SELECT rp.PostId, rp.Title, u.DisplayName AS OwnerName, ab.UpVotes, ab.DownVotes, ub.TotalBadges, ub.HighestBadgeClass
//     FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN AggregatedVotes ab ON rp.PostId = ab.PostId LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE rp.PostRank <= 5)
// SELECT cd.PostId, cd.Title, cd.OwnerName, COALESCE(cd.UpVotes, 0) AS UpVotesCount, COALESCE(cd.DownVotes, 0) AS DownVotesCount, COALESCE(cd.TotalBadges, 0) AS UserBadgesCount,
//        CASE WHEN cd.HighestBadgeClass IS NULL THEN 'None' WHEN cd.HighestBadgeClass = 1 THEN 'Gold' WHEN cd.HighestBadgeClass = 2 THEN 'Silver' ELSE 'Bronze' END AS HighestBadge
// FROM CombinedData cd ORDER BY cd.OwnerName ASC, cd.PostId DESC LIMIT 100;
//
// PostRank ties are broken by post id; the ownerless posts rank among themselves. AggregatedVotes is keyed by the raw Votes.PostId.
fn q24020(db: &'static So) -> String {
    let Post { owner_user, owner_user_id, creation_date, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 5).map(|((p, _), _)| p).collect();
    let av = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and((&ub).opt())).opt().and(origid.select(&av).opt())));
    let name = |u: Option<(Id<User>, Option<(i64, i64)>)>| u.map(|u| db.user.display_name.get(u.0).unwrap());
    let v = top_n(v, |&(p, (u, _))| (name(u).is_none(), name(u), Reverse(origid.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(p, (u, a))| {
        let a = a.unwrap_or([0; 2]);
        let b = u.and_then(|u| u.1);
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([ostr(name(u)), V::I(a[0]), V::I(a[1]), V::I(b.map_or(0, |b| b.0))]);
        f.push(V::S(match b.map(|b| b.1) {
            None => "None",
            Some(1) => "Gold",
            Some(2) => "Silver",
            _ => "Bronze",
        }));
        row(f)
    }))
}

// WITH UserVotes AS (SELECT u.Id AS UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Answered' ELSE 'Unanswered' END AS PostStatus, u.Id AS OwnerUserId,
//        u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - interval '1 year'),
// ClosedPosts AS (SELECT ph.PostId, MIN(ph.CreationDate) AS FirstClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.PostStatus, pd.OwnerUserId, pd.OwnerDisplayName, COALESCE(up.TotalVotes, 0) AS UserTotalVotes, COALESCE(up.UpVotes, 0) AS UserUpVotes,
//        COALESCE(up.DownVotes, 0) AS UserDownVotes, cp.FirstClosedDate, EXTRACT(EPOCH FROM (cp.FirstClosedDate - pd.CreationDate)) AS TimeToClose
// FROM PostDetails pd LEFT JOIN UserVotes up ON pd.OwnerUserId = up.UserId LEFT JOIN ClosedPosts cp ON pd.PostId = cp.PostId
// WHERE pd.Rank = 1 AND (pd.PostStatus = 'Answered' OR pd.PostStatus = 'Unanswered') ORDER BY pd.CreationDate DESC;
//
// Rank is partitioned by the post itself, so every post is its own rank 1.
fn q3319(db: &'static So) -> String {
    let Post { owner_user, creation_date, accepted_answer_id, .. } = &db.post;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold(i64::MAX, |m, d| m.min(d));
    let mut v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(owner_user.select(Ident::<User>::new().and(&uv)).opt().and((&cp).opt())));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (u, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Answered" } else { "Unanswered" }));
        f.extend(match u {
            Some((u, a)) => {
                let mut g = ucols(db, u, &["uid", "name"]);
                g.extend(a.map(V::I));
                g
            }
            None => vec![V::Null, V::Null, V::I(0), V::I(0), V::I(0)],
        });
        f.extend(match c {
            Some(c) => [V::T(c), V::F(secs(c - creation_date.get(p).unwrap()))],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY SUM(COALESCE(v.BountyAmount, 0)) DESC) AS BountyRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalBounties, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats WHERE PostCount > 0)
// SELECT t.UserId, t.DisplayName, t.Reputation, t.PostCount, t.QuestionCount, t.AnswerCount, t.TotalBounties, COALESCE(ROUND((CAST(t.TotalBounties AS decimal) / NULLIF(t.PostCount, 0)), 2), 0) AS AvgBountyPerPost,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId = t.UserId AND b.Class = 1) AS GoldBadges, (SELECT COUNT(*) FROM Badges b WHERE b.UserId = t.UserId AND b.Class = 2) AS SilverBadges,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId = t.UserId AND b.Class = 3) AS BronzeBadges,
//        CASE WHEN t.ReputationRank <= 10 THEN 'Top Contributor' WHEN t.ReputationRank <= 50 THEN 'Valued Member' ELSE 'Novice' END AS UserCategory
// FROM TopUsers t WHERE t.TotalBounties IS NOT NULL ORDER BY t.Reputation DESC, t.TotalBounties DESC LIMIT 50;
//
// BountyRank is never read. TotalBounties is a SUM of COALESCEs over at least one row, so it is never NULL.
fn q4757(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(bounty.opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let w = whole(db.user.with(&pc)).select(Ident::<User>::new().and(&db.user.reputation)).window(dense_rank, |(_, r)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), k)| (u, k)).collect();
    let v = top_n(drain((&us).and(&pc).and(by_first(&rk))), |&(u, ((a, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[2]), u), 50);
    let tu = rel(v.into_iter().map(|(u, ((a, n), k))| (u, a, n, k)).collect());
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    type T = (Id<User>, [i64; 3], i64, i64);
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _, _, _): T| u).select(&bc))));
    rows(v.into_iter().map(|(_, ((u, a, n, k), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F((a[2] as f64 / n as f64 * 100.0).round() / 100.0)]);
        f.extend(b.map(V::I));
        f.push(V::S(if k <= 10 { "Top Contributor" } else if k <= 50 { "Valued Member" } else { "Novice" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistoryStats AS (SELECT ph.UserId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS ClosedPosts, COUNT(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeletedPosts
//     FROM PostHistory ph GROUP BY ph.UserId),
// CombinedStats AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.Questions, us.Answers, us.AcceptedAnswers, COALESCE(phs.ClosedPosts, 0) AS ClosedPosts,
//        COALESCE(phs.DeletedPosts, 0) AS DeletedPosts, CASE WHEN us.Reputation >= 1000 THEN 'Expert' WHEN us.Reputation >= 100 THEN 'Experienced' ELSE 'Novice' END AS UserLevel
//     FROM UserStats us LEFT JOIN PostHistoryStats phs ON us.UserId = phs.UserId)
// SELECT c.DisplayName, c.Reputation, c.TotalPosts, c.Questions, c.Answers, c.AcceptedAnswers, c.ClosedPosts, c.DeletedPosts, c.UserLevel, ROW_NUMBER() OVER (ORDER BY c.Reputation DESC) AS Rank
// FROM CombinedStats c WHERE c.TotalPosts > 0 ORDER BY c.Reputation DESC LIMIT 10;
fn q4661(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 2 && x.is_some()) as i64],
            None => a,
        });
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let phs = db.post_history.group_by(user).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + matches!(t, 12 | 13) as i64]);
    let w = whole(db.user.with((&us).filt(|a| a[0] > 0))).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let v = drain(by_first(&rk).and(&us).and((&phs).opt()));
    rows(v.into_iter().map(|(u, ((i, a), h))| {
        let r = db.user.reputation.get(u).unwrap();
        let h = h.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(h[0]), V::I(h[1]), V::S(if r >= 1000 { "Expert" } else if r >= 100 { "Experienced" } else { "Novice" }), V::I(i)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// CommentsStatistics AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, AVG(c.Score) AS AverageCommentScore FROM Comments c GROUP BY c.PostId)
// SELECT u.Id AS UserId, u.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, rp.PostId, rp.Title, rp.CreationDate AS PostCreationDate, COALESCE(cs.CommentCount, 0) AS CommentCount,
//        COALESCE(cs.AverageCommentScore, 0) AS AverageCommentScore,
//        CASE WHEN u.Reputation > 1000 THEN 'High Reputation' WHEN u.Reputation IS NULL THEN 'No Reputation Data' ELSE 'Low Reputation' END AS ReputationCategory
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN RecentPosts rp ON u.Id = rp.OwnerUserId AND rp.rn = 1 LEFT JOIN CommentsStatistics cs ON rp.PostId = cs.PostId
// WHERE u.LastAccessDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '90 days' ORDER BY u.Reputation DESC NULLS LAST, ub.BadgeCount DESC;
//
// rn ties are broken by post id.
fn q528(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(t0, -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let cs = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let lp = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p).select(Ident::<Post>::new().and((&cs).opt()));
    let v = drain(db.user.with((&db.user.last_access_date).lt(add_days(t0, -90))).select((&ub).and(lp.opt())));
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some((p, c)) => {
                let c = c.unwrap_or([0; 2]);
                let mut g = post_fields(db, p, &["id", "title", "created"]);
                g.extend([V::I(c[0]), if c[0] == 0 { V::F(0.0) } else { avg(c[1], c[0]) }]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::I(0), V::F(0.0)],
        });
        f.push(V::S(if db.user.reputation.get(u).unwrap() > 1000 { "High Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, RANK() OVER(PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserPostStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, AVG(COALESCE(p.Score, 0)) AS AverageScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// PostClosureDetails AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, MIN(ph.CreationDate) AS FirstCloseDate FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// PopularTags AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS TaggedPosts FROM Tags t JOIN Posts p ON t.Id = p.Id GROUP BY t.TagName HAVING COUNT(DISTINCT p.Id) > 5)
// SELECT R.PostId, R.Title, COALESCE(ups.PostCount, 0) AS PostCount, ups.TotalViews, ups.AverageScore, R.ViewCount AS RecentViews, COALESCE(pc.CloseCount, 0) AS ClosureCount, pt.TagName
// FROM RankedPosts R LEFT JOIN UserPostStats ups ON R.PostId = ups.UserId LEFT JOIN PostClosureDetails pc ON R.PostId = pc.PostId LEFT JOIN PopularTags pt ON pt.TaggedPosts = R.PostId
// WHERE R.RankScore <= 10 ORDER BY RecentViews DESC, ClosureCount DESC LIMIT 50 OFFSET 0;
//
// `R.PostId = ups.UserId`, `t.Id = p.Id` and `pt.TaggedPosts = R.PostId` join ids of different tables (and a count), so they go through the raw ids.
fn q23128(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 10).map(|(((p, _), _), _)| p).collect();
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, w)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + 1],
            None => [a[0], a[1], a[2], a[3] + 1],
        });
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let pc = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(post_history_type_id).fold(0i64, |n, t| n + (t == 10) as i64);
    let pid: HashIdx<i64, Id<Post>> = origid.inv().collect();
    let tagged = db.tag.group_by(&db.tag.tag_name).select((&db.tag.origid).select(&pid)).fold(0i64, |n, _| n + 1);
    let pt = rel(drain((&tagged).filt(|n| n > 5)));
    let by_count: HashIdx<i64, (Str, i64)> = (&pt).map(|(_, n)| n).inv().select(&pt).collect();
    let v = drain((&tp).select(origid.select(&uid).select(&ups).opt().and((&pc).opt()).and(origid.select(&by_count).opt())));
    let v = top_n(v, |&(p, ((_, c), _))| (view_count.get(p).is_none(), Reverse(view_count.get(p)), Reverse(c.unwrap_or(0)), p), 50);
    rows(v.into_iter().map(|(p, ((u, c), t))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(match u {
            Some(a) => [V::I(a[0]), V::I(a[1]), avg(a[2], a[3])],
            None => [V::I(0), V::Null, V::Null],
        });
        f.extend([oint(view_count.get(p)), V::I(c.unwrap_or(0)), ostr(t.map(|t| t.0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(DISTINCT p.Id) AS TotalPosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistoryAggregates AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeletionCount
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalUpVotes, us.TotalDownVotes, us.TotalPosts, rp.PostId, rp.Title, rp.CreationDate, pah.CloseReopenCount, pah.DeletionCount,
//        (us.TotalUpVotes - us.TotalDownVotes) AS VoteNet
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.PostId LEFT JOIN PostHistoryAggregates pah ON rp.PostId = pah.PostId
// WHERE us.TotalPosts > 5 AND (SELECT COUNT(*) FROM Votes v WHERE v.UserId = us.UserId AND v.VoteTypeId = 2) > 10 ORDER BY VoteNet DESC, rp.CreationDate DESC LIMIT 50;
//
// `us.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids, and UserStats is only computed for the users that join. PostRank is never read.
fn q2070(db: &'static So) -> String {
    let Post { creation_date, origid, .. } = &db.post;
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ju: MatSet<Id<User>> = rp().select(origid.select(&uid)).collect();
    let us = (&ju)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let pc = (&ju).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cast = (&ju).group_by(Ident::<User>::new()).select(votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let pah = db.post_history.group_by(post).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + (t == 12) as i64]);
    let good = Ident::<User>::new().and(&us).and((&pc).filt(|n| n > 5)).and((&cast).filt(|n| n > 10));
    let v = drain(rp().select(origid.select(&uid).select(good).and((&pah).opt())));
    let v = top_n(v, |&(p, ((((_, a), _), _), _))| (Reverse(a[0] - a[1]), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((((u, a), n), _), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend(match h {
            Some(h) => [V::I(h[0]), V::I(h[1])],
            None => [V::Null, V::Null],
        });
        f.push(V::I(a[0] - a[1]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpVotePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownVotePosts, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        AVG(EXTRACT(EPOCH FROM COALESCE(p.LastActivityDate, p.CreationDate) - p.CreationDate)) AS AvgPostActiveTime FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpVotePosts, DownVotePosts, AcceptedAnswers, AvgPostActiveTime, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats),
// PostVoteSummary AS (SELECT p.OwnerUserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY p.OwnerUserId)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.PostCount, u.UpVotePosts, u.DownVotePosts, u.AcceptedAnswers, u.AvgPostActiveTime, pvs.TotalVotes, pvs.UpVotes, pvs.DownVotes, u.ReputationRank
// FROM TopUsers u JOIN PostVoteSummary pvs ON u.UserId = pvs.OwnerUserId WHERE u.ReputationRank <= 10 ORDER BY u.Reputation DESC, pvs.TotalVotes DESC;
//
// AvgPostActiveTime averages a float per row over thousands of posts, so it is a Kahan sum (translation-failures.md 10).
fn q8658(db: &'static So) -> String {
    let Post { score, accepted_answer_id, last_activity_date, creation_date, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let us = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(accepted_answer_id.opt()).and(last_activity_date).and(creation_date)).opt())
        .fold(([0i64; 4], (0.0f64, 0.0f64)), |(a, t), p| match p {
            Some((((s, x), l), c)) => ([a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + x.is_some() as i64], kahan(t, secs(l - c))),
            None => (a, t),
        });
    let pvs = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(vtype_name(db)).opt()))
        .fold([0i64; 3], |a, n| [a[0] + n.is_some() as i64, a[1] + (n == Some("UpMod")) as i64, a[2] + (n == Some("DownMod")) as i64]);
    type T = (Id<User>, i64);
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select((&us).and(&pvs)))));
    v.sort_by_key(|&(_, ((u, _), (_, p)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(p[0])));
    rows(v.into_iter().map(|(_, ((u, k), ((a, t), p)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.push(if a[0] == 0 { V::Null } else { V::F(t.0 / a[0] as f64) });
        f.extend(p.map(V::I));
        f.push(V::I(k));
        row(f)
    }))
}

// Rewritten (rewrites/8730.sql): the final ORDER BY tie-broken on PostId.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(v.Id) AS VoteCount,
//        ROW_NUMBER() OVER(PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// PostHistories AS (SELECT ph.PostId, ph.CreationDate AS HistoryCreationDate, pht.Name AS HistoryType, COUNT(*) AS HistoryCount FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
//     GROUP BY ph.PostId, ph.CreationDate, pht.Name),
// AggregatedResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.VoteCount,
//        COALESCE(SUM(CASE WHEN ph.HistoryCount IS NOT NULL THEN ph.HistoryCount ELSE 0 END), 0) AS TotalHistories
//     FROM RankedPosts rp LEFT JOIN PostHistories ph ON rp.PostId = ph.PostId GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.VoteCount)
// SELECT ar.PostId, ar.Title, ar.CreationDate, ar.Score, ar.ViewCount, ar.OwnerDisplayName, ar.VoteCount, ar.TotalHistories
// FROM AggregatedResults ar ORDER BY ar.Score DESC, ar.ViewCount DESC, ar.PostId LIMIT 100;
//
// The groups of PostHistories partition a post's history rows, so the sum of their counts is the post's history row count. rn is never read.
fn q8730(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let rp = || db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let vc = rp().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let hc = rp().group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = top_n(drain((&vc).and(&hc)), |&(p, _)| (Reverse(score.get(p).unwrap()), view_count.get(p).is_none(), Reverse(view_count.get(p)), origid.get(p).unwrap()), 100);
    rows(v.into_iter().map(|(p, (n, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(n), V::I(h)]);
        row(f)
    }))
}

// Rewritten (rewrites/868.sql): the float AVG of epoch seconds became the exact-integer mean.
// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadgeCount, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// RecentPostStats AS (SELECT p.OwnerUserId, SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(*) / 1e6 AS AvgActivityDuration FROM Posts p
//     WHERE p.LastActivityDate IS NOT NULL GROUP BY p.OwnerUserId),
// PostVoteStats AS (SELECT p.OwnerUserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.OwnerUserId)
// SELECT us.UserId, us.DisplayName, us.TotalBadgeCount, us.TotalPosts, us.QuestionCount, us.AnswerCount, COALESCE(rps.AvgActivityDuration, 0) AS AvgActivityDuration, COALESCE(pvs.TotalVotes, 0) AS TotalVotes,
//        COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes, RANK() OVER (ORDER BY us.TotalPosts DESC) AS UserRank
// FROM UserStats us LEFT JOIN RecentPostStats rps ON us.UserId = rps.OwnerUserId LEFT JOIN PostVoteStats pvs ON us.UserId = pvs.OwnerUserId
// WHERE us.TotalPosts > 10 ORDER BY UserRank, us.DisplayName;
fn q868(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, creation_date, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(post_type_id).opt()))
        .fold([0i64; 5], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0), a[2] + t.is_some() as i64, a[3] + (t == Some(1)) as i64, a[4] + (t == Some(2)) as i64]);
    let ru = || db.user.with((&us).filt(|a| a[2] > 10));
    let rps = ru().group_by(Ident::<User>::new()).select(posts_of(db).select(last_activity_date.and(creation_date))).fold((0i64, 0i128), |(n, s), (l, c)| (n + 1, s + (l - c) as i128));
    let pvs = ru()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = whole(ru()).select(Ident::<User>::new().and((&us).and((&rps).opt()).and((&pvs).opt()))).window(rank, |(_, ((a, _), _))| Reverse(a[2]), asc);
    let v = drain(&w);
    rows(v.into_iter().map(|(_, ((u, ((a, r), p)), k))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([nullable(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.push(V::F(r.map_or(0.0, |(n, s)| s as f64 / n as f64 / 1e6)));
        f.extend(p.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(k));
        row(f)
    }))
}

// WITH PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN PH.Id END) AS CloseReopenCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE P.CreationDate >= '2023-01-01' GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.OwnerUserId),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 1000),
// Combined AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.TotalBounty, PS.CommentCount, PS.CloseReopenCount, UR.DisplayName AS UserName, UR.Reputation, UR.ReputationRank
//     FROM PostStats PS JOIN Users U ON PS.PostId = U.Id JOIN UserReputation UR ON U.Id = UR.UserId WHERE PS.RecentPostRank <= 10)
// SELECT C.*, CASE WHEN C.CloseReopenCount > 0 THEN 'Closed/Reopened' ELSE 'Active' END AS PostStatus,
//        CASE WHEN C.ReputationRank BETWEEN 1 AND 10 THEN 'Top User' WHEN C.ReputationRank BETWEEN 11 AND 50 THEN 'Mid User' ELSE 'New User' END AS UserCategory
// FROM Combined C ORDER BY C.Reputation DESC, C.CreationDate DESC LIMIT 100;
//
// RecentPostRank reads only base columns, so each owner's ten newest posts are picked first (ties by post id), and the bounty x comment x history product is driven
// only for those whose raw id is a reputable user's id (`PS.PostId = U.Id` joins a post id to a user id). ReputationRank ties are broken by user id.
fn q953(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, origid, .. } = &db.post;
    let pw = db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&pw).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let rw = whole(db.user.with((&db.user.reputation).gt(1000))).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let rr: MatSet<(Id<User>, (Id<User>, i64))> = (&rw).map(|((u, _), k)| (u, (u, k))).collect();
    let rank = by_first(&rr);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ur = origid.select(&uid).select(&rank);
    let jp: MatSet<Id<Post>> = (&tp).with(ur).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ps = (&jp)
        .group_by(Ident::<Post>::new())
        .select(bounty.opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 2], |a, ((b, c), _)| [a[0] + b.flatten().unwrap_or(0), a[1] + c.is_some() as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let cr = (&jp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&ps).and(&cr).and(origid.select(&uid).select(&rank)));
    let v = top_n(v, |&(p, (_, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((a, c), (u, k)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(k), V::S(if c > 0 { "Closed/Reopened" } else { "Active" }), V::S(if k <= 10 { "Top User" } else if k <= 50 { "Mid User" } else { "New User" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        COALESCE(u.DisplayName, 'Deleted User') AS OwnerDisplayName FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.Score IS NOT NULL AND p.Score > 0),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// BadgesCount AS (SELECT b.UserId, COUNT(DISTINCT b.Id) AS BadgeCount FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId)
// SELECT rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, COALESCE(pv.UpVotes, 0) AS TotalUpVotes, COALESCE(pv.DownVotes, 0) AS TotalDownVotes, COALESCE(bc.BadgeCount, 0) AS GoldBadges,
//        CASE WHEN rp.Score > 100 THEN 'Hot Post' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Trending Post' ELSE 'Regular Post' END AS PostCategory,
//        CASE WHEN rp.Score IS NULL THEN 'No Score' ELSE 'Scored' END AS ScoreStatus
// FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.Id = pv.PostId LEFT JOIN Users u ON rp.OwnerDisplayName = u.DisplayName LEFT JOIN BadgesCount bc ON u.Id = bc.UserId
// WHERE rp.rn = 1 AND (rp.ViewCount > 50 OR bc.BadgeCount > 2) ORDER BY rp.ViewCount DESC, rp.Score DESC OFFSET 10 ROWS FETCH NEXT 20 ROWS ONLY;
//
// rn ties are broken by post id. PostVotes is keyed by the raw Votes.PostId.
fn q21922(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, origid, .. } = &db.post;
    let w = db.post.with(score.gt(0)).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let pv = db.vote.group_by(&db.vote.post_id).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let bc = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let name = owner_user.opt().map(move |u: Option<Id<User>>| u.map_or("Deleted User", |u| db.user.display_name.get(u).unwrap()));
    type R = (Option<i64>, Option<(Id<User>, Option<i64>)>);
    let keep = |(w, u): R| w.map_or(false, |w| w > 50) || u.and_then(|u| u.1).map_or(false, |b| b > 2);
    let v = drain((&first).select(view_count.opt().and(name.select(&by_name).select(Ident::<User>::new().and((&bc).opt())).opt())).filt(keep).and(origid.select(&pv).opt()));
    let v = top_n(v, |&(p, _)| (view_count.get(p).is_none(), Reverse(view_count.get(p)), Reverse(score.get(p).unwrap()), p), 30);
    rows(v.into_iter().skip(10).map(|(p, ((_, u), a))| {
        let s = score.get(p).unwrap();
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(u.and_then(|u| u.1).unwrap_or(0))]);
        f.extend([V::S(if s > 100 { "Hot Post" } else if s >= 50 { "Trending Post" } else { "Regular Post" }), V::S("Scored")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerName, ARRAY_LENGTH(STRING_TO_ARRAY(p.Tags, '><'), 1) AS TagCount,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostWithVotes AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.OwnerUserId, rp.OwnerName, rp.TagCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 1 THEN 1 ELSE 0 END), 0) AS AcceptedByOriginatorVotes,
//        COUNT(DISTINCT c.Id) AS CommentCount FROM RankedPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId LEFT JOIN Comments c ON rp.PostId = c.PostId
//     GROUP BY rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.OwnerUserId, rp.OwnerName, rp.TagCount),
// TopPosts AS (SELECT pw.*, ROW_NUMBER() OVER (ORDER BY UpVotes DESC, CreationDate DESC) AS OverallRank FROM PostWithVotes pw)
// SELECT tp.PostId, tp.Title, tp.Body, tp.CreationDate, tp.OwnerName, tp.TagCount, tp.UpVotes, tp.DownVotes, tp.AcceptedByOriginatorVotes, tp.CommentCount
// FROM TopPosts tp WHERE tp.OverallRank <= 10 ORDER BY tp.UpVotes DESC, tp.CreationDate DESC;
//
// PostRank is never read. The distinct comment count is a second fold over one row per comment.
fn q27843(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -1)))).with(owner_user);
    let pw = rp()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(1)) as i64]);
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&pw).and(&cc)), |&(p, (a, _))| (Reverse(a[0]), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner"]);
        f.push(oint(tags_str.get(p).map(|t| t.split("><").count() as i64)));
        f.extend(a.map(V::I));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostRankings AS (SELECT p.Id AS PostId, p.Title, RANK() OVER (ORDER BY p.LastActivityDate DESC) AS PostRank FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT us.UserId, us.DisplayName, us.TotalPosts, us.TotalQuestions, us.TotalAnswers, us.GoldBadges, us.SilverBadges, us.BronzeBadges, pr.PostId, pr.Title, pr.PostRank,
//        COALESCE(rc.CommentCount, 0) AS CommentCount, rc.LastCommentDate
// FROM UserStats us JOIN PostRankings pr ON pr.PostId IN (SELECT p.Id FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Id = us.UserId)
// LEFT JOIN RecentComments rc ON pr.PostId = rc.PostId WHERE us.TotalPosts > 0 ORDER BY us.TotalPosts DESC, pr.PostRank ASC LIMIT 10;
//
// The IN subquery pairs each ranked post with its owner. UserStats is read only for owners of a ranked post, so the post x badge product is driven for those alone.
fn q1962(db: &'static So) -> String {
    let Post { creation_date, last_activity_date, owner_user, post_type_id, .. } = &db.post;
    let w = whole(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(Ident::<Post>::new().and(last_activity_date)).window(rank, |(_, d)| Reverse(d), asc);
    let pr: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), k)| (p, k)).collect();
    let owners: MatSet<Id<User>> = (&pr).map(|(p, _)| p).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (t, c)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]);
    let dp = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rc = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type P = (Id<Post>, i64);
    let v = drain((&pr).select(Same::<P>::new().and(Same::<P>::new().map(|(p, _): P| p).select(owner_user).select(Ident::<User>::new().and(&us).and((&dp).filt(|n| n > 0))))));
    let v = top_n(v, |&(_, ((p, k), ((_, _), n)))| (Reverse(n), k, p), 10);
    let v = rel(v.into_iter().map(|x| x.1).collect());
    type W = ((Id<Post>, i64), ((Id<User>, [i64; 5]), i64));
    let v = drain((&v).select(Same::<W>::new().and(Same::<W>::new().map(|((p, _), _): W| p).select(&rc).opt())));
    rows(v.into_iter().map(|(_, (((p, k), ((u, a), n)), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.push(V::I(k));
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH UserScores AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Upvotes, Downvotes, PostCount, CommentCount, BadgeCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC, Upvotes DESC) AS Rank FROM UserScores),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'))
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.CommentCount, tu.BadgeCount, rp.Title AS MostRecentPostTitle, rp.CreationDate AS MostRecentPostDate, rp.Score AS MostRecentPostScore
// FROM TopUsers tu LEFT JOIN RecentPosts rp ON tu.UserId = rp.OwnerUserId AND rp.rn = 1 WHERE tu.Rank <= 10 ORDER BY tu.Reputation DESC, tu.Upvotes DESC;
//
// Rank leads with Reputation, so only users whose RANK() by Reputation is at most ten can rank in the top ten; the post x comment x own-vote x badge product is
// driven for those alone. `v.PostId = p.Id AND v.UserId = u.Id` on a post of u is a vote by the post's owner (`own_votes`). Rank and rn ties are broken by id.
fn q4744(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let rw = whole(db.user.iq()).select(Ident::<User>::new().and(rep)).window(rank, |(_, r)| Reverse(r), asc);
    let cand: MatSet<Id<User>> = (&rw).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let own = own_votes(db);
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let dp = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let dc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let db_ = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tu = top_n(drain((&us).and(&dp).and(&dc).and(&db_)), |&(u, (((a, _), _), _))| (Reverse(rep.get(u).unwrap()), Reverse(a[0]), u), 10);
    let tu = rel(tu);
    let Post { owner_user, creation_date, last_activity_date, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(last_activity_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let latest = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p);
    type T = (Id<User>, ((([i64; 2], i64), i64), i64));
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(&latest).opt())));
    v.sort_by_key(|&(_, ((u, (((a, _), _), _)), _))| (Reverse(rep.get(u).unwrap()), Reverse(a[0])));
    rows(v.into_iter().map(|(_, ((u, (((_, n), c), b)), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(c), V::I(b)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(v.UpVotes, 0)) AS TotalUpVotes, SUM(COALESCE(v.DownVotes, 0)) AS TotalDownVotes,
//        ROW_NUMBER() OVER (ORDER BY COUNT(p.Id) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days')
// SELECT ups.UserId, ups.DisplayName, ups.PostCount, ups.TotalUpVotes, ups.TotalDownVotes, rp.PostId, rp.Title, rp.CreationDate AS RecentPostDate, rp.UserPostRank,
//        COALESCE(ph.Comment, 'No comments available') AS PostHistoryComment
// FROM UserPostStats ups LEFT JOIN RecentPosts rp ON ups.UserId = rp.OwnerUserId AND rp.UserPostRank = 1
// LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory ph_sub WHERE ph_sub.PostId = rp.PostId AND ph_sub.UserId IS NOT NULL)
// WHERE ups.Rank <= 10 ORDER BY ups.Rank;
//
// The correlated MAX is a per-post fold over the history rows with a user, and the history rows at that instant are joined back. Rank ties are broken by user id.
fn q30260(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ups = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&vs).opt()).opt())
        .fold([0i64; 3], |a, p| match p {
            Some(v) => {
                let v = v.unwrap_or([0; 2]);
                [a[0] + 1, a[1] + v[0], a[2] + v[1]]
            }
            None => a,
        });
    let tu = rel(top_n(drain(&ups), |&(u, a)| (Reverse(a[0]), u), 10));
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -7))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let latest = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p);
    let PostHistory { post, creation_date: hd, user_id, .. } = &db.post_history;
    let md = db.post_history.with(user_id).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    let ph = Ident::<Post>::new().and(&md).select(&at);
    type T = (Id<User>, [i64; 3]);
    let lp = Same::<T>::new().map(|(u, _): T| u).select((&latest).select(Ident::<Post>::new().and(ph.opt()))).opt();
    let v = drain((&tu).select(Same::<T>::new().and(lp)));
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(match p {
            Some((p, h)) => {
                let mut g = post_fields(db, p, &["id", "title", "created"]);
                g.extend([V::I(1), V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No comments available"))]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::S("No comments available")],
        });
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS AnswerCount, ROW_NUMBER() OVER (ORDER BY SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS VoteRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 2 LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// ClosedPosts AS (SELECT P.Id AS PostId, P.Title, PH.CreationDate, PH.Comment FROM Posts P INNER JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE PH.PostHistoryTypeId = 10 AND PH.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// RecentQuestions AS (SELECT P.Id AS QuestionId, P.Title, COUNT(A.Id) AS AnswerCount FROM Posts P LEFT JOIN Posts A ON P.Id = A.ParentId
//     WHERE P.PostTypeId = 1 AND P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY P.Id, P.Title)
// SELECT U.DisplayName, U.UpVotes, U.DownVotes, RQ.Title AS RecentQuestionTitle, RQ.AnswerCount AS RecentAnswerCount, CP.Comment AS CloseComment
// FROM UserVoteStats U FULL OUTER JOIN RecentQuestions RQ ON U.UserId = RQ.QuestionId LEFT JOIN ClosedPosts CP ON RQ.QuestionId = CP.PostId
// WHERE U.VoteRank <= 10 OR RQ.AnswerCount > 0 ORDER BY COALESCE(U.UpVotes, 0) DESC, COALESCE(RQ.AnswerCount, 0) DESC;
//
// `U.UserId = RQ.QuestionId` joins a user id to a post id, so it goes through the raw ids. The FULL OUTER JOIN is the questions (each with its matching user, if any)
// together with the users no question matches. VoteRank ties are broken by user id.
fn q2623(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let answer = Ident::<Post>::new().with(post_type_id.eq(2));
    let uvs = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(answer).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let w = whole(&uvs).select(Ident::<User>::new().and(&uvs)).window(row_number, |(u, a)| (Reverse(a[0]), u), asc);
    type U = (Id<User>, [i64; 2], i64);
    let ur: MatSet<U> = (&w).map(|((u, a), k)| (u, a, k)).collect();
    let by_raw: HashIdx<i64, U> = (&ur).map(|(u, _, _): U| db.user.origid.get(u).unwrap()).inv().select(&ur).collect();
    let rq = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(t0, -30)))).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10).and(hd.ge(add_years(t0, -1)))));
    type O = (Option<U>, Option<(Id<Post>, i64)>);
    let left = rel(drain((&rq).and(origid.select(&by_raw).opt())).into_iter().map(|(p, (n, u)): (Id<Post>, (i64, Option<U>))| -> O { (u, Some((p, n))) }).collect());
    let qraw: MatSet<i64> = (&rq).and(origid).map(|(_, o)| o).collect();
    let matched = (&ur).select(Same::<U>::new().map(|(u, _, _): U| db.user.origid.get(u).unwrap()).select(&qraw));
    let lone = rel(drain((&ur).minus(matched)));
    let right = (&lone).map(|(_, u)| -> O { (Some(u), None) });
    let both = rel(drain((&left).map(|x| x).union(right)).into_iter().map(|x| x.1).collect());
    let keep = |(u, q): O| u.map_or(false, |u| u.2 <= 10) || q.map_or(false, |q| q.1 > 0);
    let v = drain((&both).filt(keep).select(Same::<O>::new().and(Same::<O>::new().flat_map(|(_, q): O| q.map(|q| q.0)).select(cp.opt()).opt())));
    let v = top_n(v, |&(_, ((u, q), _))| (Reverse(u.map_or(0, |u| u.1[0])), Reverse(q.map_or(0, |q| q.1))), 0);
    rows(v.into_iter().map(|(_, ((u, q), c))| {
        let mut f = match u {
            Some((u, a, _)) => vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])],
            None => vec![V::Null, V::Null, V::Null],
        };
        f.extend(match q {
            Some((p, n)) => [ostr(db.post.title.get(p)), V::I(n)],
            None => [V::Null, V::Null],
        });
        f.push(ostr(c.flatten().and_then(|h| db.post_history.comment.get(h))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.PostTypeId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankByViews,
//        DENSE_RANK() OVER (ORDER BY COALESCE(ph.UserId, -1)) AS UserRank
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (10, 11) WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 YEAR' AND p.ViewCount IS NOT NULL),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.RankByViews, rp.UserRank, pt.Name AS PostType FROM RankedPosts rp JOIN PostTypes pt ON rp.PostTypeId = pt.Id WHERE rp.RankByViews <= 10),
// FilteredPosts AS (SELECT trp.*, COALESCE(v.ReceivedVotes, 0) AS TotalVotes, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = trp.PostId) AS CommentCount FROM TopRankedPosts trp
//     LEFT JOIN (SELECT p.Id AS PostId, COUNT(v.Id) AS ReceivedVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 6) GROUP BY p.Id) v ON trp.PostId = v.PostId)
// SELECT fp.PostId, fp.Title, fp.ViewCount, fp.TotalVotes, fp.CommentCount, fp.UserRank, CASE WHEN fp.UserRank = 1 THEN 'This Post is the Most Engaging!' ELSE 'Engaging Post' END AS EngagementMessage
// FROM FilteredPosts fp WHERE fp.TotalVotes > 0 OR fp.CommentCount > 0 ORDER BY fp.TotalVotes DESC, fp.ViewCount DESC;
//
// The ranks number the post x close-history rows; RankByViews ties are broken by post and history id.
fn q23874(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11]))).opt();
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1))).with(view_count).select(Ident::<Post>::new().and(closes)).collect();
    type J = (Id<Post>, Option<Id<PostHistory>>);
    let uid_of = Same::<J>::new().flat_map(|(_, h): J| h).select(&db.post_history.user_id);
    let w1 = whole(&j).select(Same::<J>::new().and(uid_of.opt())).window(dense_rank, |(_, u)| u.unwrap_or(-1), asc);
    let ur: MatSet<(J, i64)> = (&w1).map(|((x, _), k)| (x, k)).collect();
    type T = (J, i64);
    let tp_of = Same::<T>::new().map(|((p, _), _): T| p);
    let w2 = (&ur).group_by((&tp_of).select(post_type_id)).select(Same::<T>::new().and((&tp_of).select(view_count.opt()))).window(row_number, |(((p, h), _), w)| (w.is_none(), Reverse(w), p, h), asc);
    let top = (&w2).filt(|(_, n)| n <= 10).map(|((x, _), _)| x);
    let rv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 6])));
    let tv = db.post.group_by(Ident::<Post>::new()).select(rv.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pk = || Same::<T>::new().map(|((p, _), _): T| p);
    let v = drain((&top).select(Same::<T>::new().and(pk().select(&tv)).and(pk().select(&cc))).filt(|((_, t), c): ((T, i64), i64)| t > 0 || c > 0));
    let mut v: Vec<_> = v.into_iter().map(|(_, ((((p, _), k), t), c))| (p, k, t, c)).collect();
    v.sort_by_key(|&(p, _, t, _)| (Reverse(t), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(p, k, t, c)| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(t), V::I(c), V::I(k), V::S(if k == 1 { "This Post is the Most Engaging!" } else { "Engaging Post" })]);
        row(f)
    }))
}

// WITH RECURSIVE UserVoteCounts AS (SELECT UserId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS Downvotes FROM Votes GROUP BY UserId),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, COALESCE(SUM(CASE WHEN c.UserId IS NOT NULL THEN 1 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END), 0) AS CloseCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(uv.Upvotes, 0) - COALESCE(uv.Downvotes, 0) AS NetVotes, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN UserVoteCounts uv ON u.Id = uv.UserId)
// SELECT us.DisplayName, us.Reputation, ps.Title, ps.Score, ps.CommentCount, ps.CloseCount, ps.CreationDate, CASE WHEN ps.PostRank = 1 THEN 'Latest Post' ELSE NULL END AS PostStatus,
//        CASE WHEN us.Reputation >= 1000 THEN 'Gold' WHEN us.Reputation >= 500 THEN 'Silver' ELSE 'Bronze' END AS Badge
// FROM UserReputation us JOIN PostStats ps ON us.UserId = ps.OwnerUserId WHERE us.NetVotes >= 5 ORDER BY us.Reputation DESC, ps.CreationDate DESC LIMIT 10;
//
// WITH RECURSIVE, but no CTE refers to itself. PostRank ties are broken by post id; ReputationRank is never read.
fn q34915(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold(0i64, |n, t| n + (t == 2) as i64 - (t == 3) as i64);
    let good: MatSet<Id<User>> = db.user.with((&uv).filt(|n| n >= 5)).collect();
    let rp = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user.select(&good));
    let w = rp().group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let ps = rp()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select((&db.comment.user_id).opt()).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.flatten().is_some() as i64, a[1] + (t == Some(10)) as i64]);
    let v = drain((&ps).and(Ident::<Post>::new().with(&first).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(db.user.reputation.get(owner_user.get(p).unwrap()).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, l))| {
        let u = owner_user.get(p).unwrap();
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["created"]));
        f.push(if l.is_some() { V::S("Latest Post") } else { V::Null });
        f.push(V::S(if r >= 1000 { "Gold" } else if r >= 500 { "Silver" } else { "Bronze" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, u.DisplayName AS Owner, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopVoters AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// PostHistoryAggregate AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.Owner, COALESCE(tv.VoteCount, 0) AS TotalVotes, COALESCE(tv.UpVotes, 0) AS UpVotes,
//        COALESCE(tv.DownVotes, 0) AS DownVotes, COALESCE(pha.EditCount, 0) AS EditCount, pha.LastEditDate,
//        CASE WHEN rp.Score IS NULL THEN 'No score yet' WHEN rp.Score > 10 THEN 'Popular' ELSE 'Needs more engagement' END AS EngagementLevel
// FROM RecentPosts rp LEFT JOIN TopVoters tv ON rp.PostId = tv.PostId LEFT JOIN PostHistoryAggregate pha ON rp.PostId = pha.PostId WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC LIMIT 100;
//
// rn ties are broken by post id. TopVoters is keyed by the raw Votes.PostId.
fn q32970(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let tv = db.vote.group_by(&db.vote.post_id).select(vtype_name(db)).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&first).select(origid.select(&tv).opt().and((&pha).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (t, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "owner"]);
        f.extend(t.unwrap_or([0; 3]).map(V::I));
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        f.push(V::S(if score.get(p).unwrap() > 10 { "Popular" } else { "Needs more engagement" }));
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT Id, Reputation, CreationDate, DisplayName,
//        (Reputation + COALESCE((SELECT SUM(BountyAmount) FROM Votes WHERE UserId = U.Id AND BountyAmount IS NOT NULL), 0)) AS TotalReputation,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Ranking FROM Users U),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.TotalReputation, RANK() OVER (ORDER BY U.TotalReputation DESC) AS Rank FROM UserReputation U WHERE U.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year')
// SELECT U.Id AS UserId, U.DisplayName, U.TotalReputation, COALESCE(BadgeCount.BadgeCount, 0) AS BadgeCount, COALESCE(PostStats.PostCount, 0) AS PostCount, COALESCE(VoteStats.UpVoteCount, 0) AS UpVoteCount,
//        COALESCE(VoteStats.DownVoteCount, 0) AS DownVoteCount,
//        CASE WHEN U.TotalReputation >= 5000 THEN 'High Reputation User' WHEN U.TotalReputation >= 1000 THEN 'Moderate Reputation User' ELSE 'New User' END AS UserCategory
// FROM TopUsers U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) BadgeCount ON U.Id = BadgeCount.UserId
// LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS PostCount FROM Posts WHERE PostTypeId = 1 GROUP BY OwnerUserId) PostStats ON U.Id = PostStats.OwnerUserId
// LEFT JOIN (SELECT UserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Votes GROUP BY UserId) VoteStats
//     ON U.Id = VoteStats.UserId
// WHERE U.Rank <= 10 ORDER BY U.TotalReputation DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. Ranking is never read.
fn q31283(db: &'static So) -> String {
    let User { reputation, creation_date, .. } = &db.user;
    let recent = || db.user.with(creation_date.gt(add_years(date(2024, 10, 1), -1)));
    let tr = recent().group_by(Ident::<User>::new()).select(reputation.and(votes_by(db).select(&db.vote.bounty_amount).opt())).fold((0i64, 0i64), |(_, b), (x, y)| (x, b + y.unwrap_or(0)));
    let w = whole(&tr).select(Ident::<User>::new().and(&tr)).window(rank, |(_, (r, b))| Reverse(r + b), asc);
    let tu: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, (r, b)), _)| (u, r + b)).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let pc = db.post.with((&db.post.post_type_id).eq(1)).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type T = (Id<User>, i64);
    let uk = || Same::<T>::new().map(|(u, _): T| u);
    let mut v = drain((&tu).select(Same::<T>::new().and(uk().select(&bc).opt()).and(uk().select(&pc).opt()).and(uk().select(&vs).opt())));
    v.sort_by_key(|&(_, ((((_, t), _), _), _))| Reverse(t));
    rows(v.into_iter().map(|(_, ((((u, t), b), p), s))| {
        let s = s.unwrap_or([0; 2]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(t), V::I(b.unwrap_or(0)), V::I(p.unwrap_or(0)), V::I(s[0]), V::I(s[1])]);
        f.push(V::S(if t >= 5000 { "High Reputation User" } else if t >= 1000 { "Moderate Reputation User" } else { "New User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore,
//        SUM(v.BountyAmount) OVER (PARTITION BY p.OwnerUserId) AS TotalBounty
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT b.Id) AS TotalBadges, COALESCE(SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END), 0) AS PositiveScoreCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT p.Id, hp.PostId, hp.Comment FROM PostHistory hp JOIN Posts p ON hp.PostId = p.Id WHERE hp.PostHistoryTypeId = 10),
// TopPosts AS (SELECT rp.Id, rp.Title, us.DisplayName, rp.CreationDate, rp.Score, rp.ViewCount, rp.TotalBounty FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.RankByScore <= 5)
// SELECT tp.Title, tp.DisplayName, tp.CreationDate, tp.Score, tp.ViewCount, tp.TotalBounty, COALESCE(cp.Comment, 'No closure comment') AS ClosureComment
// FROM TopPosts tp LEFT JOIN ClosedPosts cp ON tp.Id = cp.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// RankedPosts has no GROUP BY: it is one row per post x bounty-start vote, ranked and summed per owner over those rows. UserStats has one row per user and only
// DisplayName is read from it, so it is the join to Users.
fn q3222(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let start = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8)));
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> =
        db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(Ident::<Post>::new().and(start.opt())).collect();
    type J = (Id<Post>, Option<Id<Vote>>);
    let owner = || Same::<J>::new().map(|(p, _): J| p).select(owner_user);
    let tb = (&j).group_by(owner()).select(Same::<J>::new().flat_map(|(_, v): J| v).select(&db.vote.bounty_amount)).fold(0i64, |s, b| s + b);
    let w = (&j).group_by(owner()).select(Same::<J>::new().and(Same::<J>::new().map(|(p, _): J| p).select(score))).window(rank, |(_, s)| Reverse(s), asc);
    let top = (&w).filt(|(_, n)| n <= 5).map(|((x, _), _)| x);
    let closed = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&top).select(Same::<J>::new().and(Same::<J>::new().map(|(p, _): J| p).select(closed.opt()))).and((&tb).opt()));
    let mut v: Vec<_> = v.into_iter().map(|(u, (((p, _), h), b))| (p, u, b, h)).collect();
    v.sort_by_key(|&(p, _, _, _)| (Reverse(score.get(p).unwrap()), db.post.view_count.get(p).is_none(), Reverse(db.post.view_count.get(p))));
    rows(v.into_iter().map(|(p, u, b, h)| {
        let mut f = post_fields(db, p, &["title"]);
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["created", "score", "views"]));
        f.extend([oint(b), V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No closure comment"))]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 1000),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserPostDetails AS (SELECT ru.DisplayName, ru.UserId, ps.TotalPosts, ps.TotalQuestions, ps.TotalAnswers FROM RankedUsers ru LEFT JOIN PostStats ps ON ru.UserId = ps.OwnerUserId)
// SELECT u.DisplayName, COALESCE(upd.TotalPosts, 0) AS TotalPosts, COALESCE(upd.TotalQuestions, 0) AS TotalQuestions, COALESCE(upd.TotalAnswers, 0) AS TotalAnswers,
//        CASE WHEN COALESCE(upd.TotalPosts, 0) > 0 THEN ROUND(COALESCE(upd.TotalAnswers, 0) * 1.0 / upd.TotalPosts * 100, 2) ELSE 0 END AS AnswerRate,
//        CASE WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = u.Id AND b.Class = 1) THEN 'Gold' WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = u.Id AND b.Class = 2) THEN 'Silver'
//        WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = u.Id AND b.Class = 3) THEN 'Bronze' ELSE 'No Badge' END AS BadgeStatus
// FROM Users u LEFT JOIN UserPostDetails upd ON u.Id = upd.UserId WHERE u.Id IS NOT NULL ORDER BY AnswerRate DESC NULLS LAST, TotalPosts DESC;
//
// UserRank is never read.
fn q3913(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let ps = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let ru = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let best = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold(i64::MAX, |m, c| m.min(c));
    let v = drain(db.user.select(ru.select(&ps).opt().and((&best).opt())));
    rows(v.into_iter().map(|(u, (a, b))| {
        let a = a.unwrap_or([0; 3]);
        let rate = if a[0] > 0 { (a[2] as f64 * 1.0 / a[0] as f64 * 100.0 * 100.0).round() / 100.0 } else { 0.0 };
        row(vec![
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::F(rate),
            V::S(match b {
                Some(1) => "Gold",
                Some(2) => "Silver",
                Some(3) => "Bronze",
                _ => "No Badge",
            }),
        ])
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON V.UserId = U.Id AND V.PostId = P.Id GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalQuestions, TotalBounties, RANK() OVER (ORDER BY Reputation DESC) AS RankReputation,
//        RANK() OVER (ORDER BY TotalPosts DESC) AS RankPosts FROM UserStatistics),
// RecentActivity AS (SELECT U.Id AS UserId, U.DisplayName, p.Title, p.CreationDate, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Users U JOIN Posts p ON U.Id = p.OwnerUserId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalAnswers, TU.TotalQuestions, TU.TotalBounties, RA.Title AS RecentPostTitle, RA.CreationDate AS RecentPostDate
// FROM TopUsers TU LEFT JOIN RecentActivity RA ON TU.UserId = RA.UserId AND RA.RecentPostRank = 1 WHERE TU.RankReputation <= 10 OR TU.RankPosts <= 10
// ORDER BY TU.RankReputation, TU.RankPosts LIMIT 50;
//
// `V.UserId = U.Id AND V.PostId = P.Id` on a post of U is a vote by the post's owner (`own_votes`). RecentPostRank ties are broken by post id.
fn q3135(db: &'static So) -> String {
    let own = own_votes(db);
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and((&own).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w1 = whole(db.user.iq()).select(Ident::<User>::new().and(&us).and(&dp).and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let w2 = (&w1).window(rank, |((((_, _), n), _), _)| Reverse(n), asc);
    type T = (Id<User>, [i64; 3], i64, i64, i64);
    let tu: MatSet<T> = (&w2).filt(|((_, rr), rp)| rr <= 10 || rp <= 10).map(|(((((u, a), n), _), rr), rp)| (u, a, n, rr, rp)).collect();
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let latest = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p);
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _, _, _, _): T| u).select(&latest).opt())));
    let v = top_n(v, |&(_, ((u, _, _, a, b), _))| (a, b, u), 50);
    rows(v.into_iter().map(|(_, ((u, a, n, _, _), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.CreationDate, p.Score, p.Title, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId,
//        COUNT(c.Id) AS CommentCount, MAX(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN ph.CreationDate END) AS ClosedOrReopenedDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.CreationDate, p.Score, p.Title, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.PostId, rp.CreationDate, rp.Score, rp.Title, rp.OwnerUserId, COALESCE(rp.CommentCount, 0) AS CommentCount,
//        CASE WHEN rp.ClosedOrReopenedDate IS NOT NULL AND rp.ClosedOrReopenedDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 'Closed Recently' ELSE 'Active' END AS PostStatus
//     FROM RankedPosts rp WHERE rp.rn = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.CommentCount, fp.PostStatus, ur.Reputation, ur.BadgeCount
// FROM FilteredPosts fp JOIN UserReputation ur ON fp.OwnerUserId = ur.UserId WHERE (fp.Score > 10 OR fp.CommentCount > 5) AND ur.BadgeCount > 0 AND ur.Reputation > 100
// ORDER BY fp.CreationDate DESC LIMIT 100;
//
// rn reads only base columns, so each owner's newest post is picked first (ties by post id) and the comment x history product is driven for those alone.
fn q21028(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(post_type_id.is_in([1, 2])).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let rp = (&first)
        .group_by(Ident::<Post>::new())
        .select(score.and(comments_of(db).opt().and(history_of(db).select(post_history_type_id.and(hd)).opt())))
        .fold((0i64, 0i64, i64::MIN), |(_, n, m), (s, (c, h))| {
            (s, n + c.is_some() as i64, match h {
                Some((t, d)) if t == 10 || t == 11 => m.max(d),
                _ => m,
            })
        });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ur = Ident::<User>::new().with((&db.user.reputation).gt(100)).and((&bc).filt(|n| n > 0));
    let v = drain((&rp).filt(|(s, n, _): (i64, i64, i64)| s > 10 || n > 5).and(owner_user.select(ur)));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    rows(v.into_iter().map(|(p, ((_, n, m), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(n), V::S(if m != i64::MIN && m > since { "Closed Recently" } else { "Active" }), user_col(db, u, "rep"), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvotesReceived
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE(cp.CloseCount, 0) AS TotalCloseCount, us.DisplayName AS OwnerDisplayName, us.Reputation AS OwnerReputation,
//        CASE WHEN p.OwnerUserId IS NULL THEN 'Deleted User' ELSE us.DisplayName END AS EffectiveOwner
//     FROM Posts p LEFT JOIN UserStats us ON p.OwnerUserId = us.UserId LEFT JOIN ClosedPosts cp ON p.Id = cp.PostId WHERE p.PostTypeId = 1 AND p.Score > 10)
// SELECT pd.PostId, pd.Title, pd.Score, pd.TotalCloseCount, pd.OwnerDisplayName, pd.OwnerReputation, pd.EffectiveOwner FROM PostDetails pd WHERE pd.TotalCloseCount = 0 ORDER BY pd.Score DESC, pd.Title ASC;
//
// RankedPosts is never referenced, and no UserStats aggregate is read, so UserStats is the join to Users.
fn q3562(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, owner_user_id, .. } = &db.post;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(10))).minus(closes).select(owner_user.opt()));
    rows(v.into_iter().map(|(p, u)| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.push(V::I(0));
        f.extend(match u {
            Some(u) => ucols(db, u, &["name", "rep", "name"]),
            None => vec![V::Null, V::Null, if owner_user_id.get(p).is_none() { V::S("Deleted User") } else { V::Null }],
        });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, AVG(v.BountyAmount) AS AverageBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id, u.DisplayName, SUM(u.UpVotes) AS TotalUpvotes, SUM(u.DownVotes) AS TotalDownvotes, RANK() OVER (ORDER BY SUM(u.UpVotes) DESC) AS UserRank
//     FROM Users u GROUP BY u.Id, u.DisplayName HAVING SUM(u.UpVotes) > 0),
// PostsWithOwnerInfo AS (SELECT rp.Id, rp.Title, rp.CreationDate, ru.DisplayName AS OwnerDisplayName, rp.CommentCount, rp.AverageBounty, tu.TotalUpvotes, tu.TotalDownvotes, tu.UserRank
//     FROM RecentPosts rp JOIN Users ru ON rp.OwnerUserId = ru.Id LEFT JOIN TopUsers tu ON ru.Id = tu.Id)
// SELECT pwi.*, CASE WHEN pwi.AverageBounty IS NULL THEN 'No Bounty' ELSE CONCAT('Average Bounty: $', COALESCE(CAST(pwi.AverageBounty AS TEXT), '0')) END AS BountyInfo,
//        CASE WHEN pwi.CommentCount > 10 THEN 'Highly Engaged' ELSE 'Less Engaged' END AS EngagementLevel
// FROM PostsWithOwnerInfo pwi WHERE EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = pwi.Id AND v.VoteTypeId IN (2, 3)) ORDER BY pwi.CreationDate DESC LIMIT 50;
fn q2765(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ud = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(ud)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let User { up_votes, .. } = &db.user;
    let w = whole(db.user.with(up_votes.gt(0))).select(Ident::<User>::new().and(up_votes)).window(rank, |(_, n)| Reverse(n), asc);
    let tr: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), k)| (u, k)).collect();
    let rank = by_first(&tr);
    let v = drain((&rp).and(owner_user.select(Ident::<User>::new().and((&rank).opt()))));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (a, (u, k)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(a[0])]);
        let ab = if a[1] == 0 { None } else { Some(a[2] as f64 / a[1] as f64) };
        f.push(ab.map_or(V::Null, V::F));
        f.extend(match k {
            Some(k) => [user_col(db, u, "uup"), user_col(db, u, "udown"), V::I(k)],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(match ab {
            None => V::S("No Bounty"),
            Some(x) => V::Owned(format!("Average Bounty: ${x:?}")),
        });
        f.push(V::S(if a[0] > 10 { "Highly Engaged" } else { "Less Engaged" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation IS NOT NULL),
// PostActivity AS (SELECT P.Id AS PostId, P.OwnerUserId, COUNT(C.ID) AS CommentCount, COALESCE(SUM(V.UserId), 0) AS VoteCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (2, 3) GROUP BY P.Id, P.OwnerUserId),
// ActivePostOwners AS (SELECT P.OwnerUserId, COUNT(*) AS PostCount, SUM(PA.CommentCount) AS TotalComments, SUM(PA.VoteCount) AS TotalVotes FROM Posts P JOIN PostActivity PA ON P.Id = PA.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.DisplayName, U.Reputation, UR.ReputationRank, A.PostCount, A.TotalComments, A.TotalVotes FROM ActivePostOwners A JOIN UserReputation UR ON A.OwnerUserId = UR.UserId
//     JOIN Users U ON A.OwnerUserId = U.Id WHERE UR.ReputationRank <= 10)
// SELECT U.DisplayName, U.Reputation, COALESCE(A.PostCount, 0) AS ActivePosts, COALESCE(A.TotalComments, 0) AS TotalComments, COALESCE(A.TotalVotes, 0) AS TotalVotes,
//        CONCAT(U.DisplayName, ': ', CASE WHEN U.Reputation > 1000 THEN 'Expert' WHEN U.Reputation BETWEEN 500 AND 1000 THEN 'Intermediate' ELSE 'Beginner' END) AS ReputationCategory
// FROM Users U LEFT JOIN ActivePostOwners A ON U.Id = A.OwnerUserId WHERE U.LastAccessDate IS NOT NULL ORDER BY U.Reputation DESC;
//
// TopUsers is never referenced. `SUM(V.UserId)` sums the raw voter ids over the comment x vote rows. PostActivity is read only for the recent posts, so the product is driven for those alone.
fn q20838(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let ud = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select((&db.vote.user_id).opt());
    let pa = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(ud.opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.flatten().unwrap_or(0)]);
    let apo = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&pa)).fold([0i64; 3], |a, x| [a[0] + 1, a[1] + x[0], a[2] + x[1]]);
    let v = drain(db.user.select((&apo).opt()));
    rows(v.into_iter().map(|(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let n = db.user.display_name.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.unwrap_or([0; 3]).map(V::I));
        f.push(V::Owned(format!("{n}: {}", if r > 1000 { "Expert" } else if r >= 500 { "Intermediate" } else { "Beginner" })));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, ROW_NUMBER() OVER (ORDER BY P.Score DESC) AS Rank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.Score
//     HAVING COUNT(DISTINCT C.Id) > 5),
// PostDetails AS (SELECT PH.PostId, PH.UserId, PH.Comment AS EditComment, PH.CreationDate AS EditDate, P.Title FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId IN (4, 5, 6))
// SELECT UA.UserId, UA.DisplayName, UA.Reputation, PP.Title AS PopularPostTitle, PP.CommentCount, PD.EditComment, PD.EditDate
// FROM UserActivity UA JOIN PopularPosts PP ON UA.PostCount > 10 LEFT JOIN PostDetails PD ON PP.PostId = PD.PostId
// WHERE (UA.TotalVotes > 20 OR (UA.Reputation >= 1000 AND PP.CommentCount > 2)) ORDER BY UA.Reputation DESC, PP.CommentCount DESC;
//
// The ON clause names only UA, so the users with more than ten posts are crossed with the popular posts. Rank is never read.
fn q4927(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold(0i64, |n, t| n + matches!(t.flatten(), Some(2 | 3)) as i64);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let us: HashIdx<Id<User>, (i64, i64)> = (&ua).and((&pc).filt(|n| n > 10)).map(|(t, _)| t).and(&db.user.reputation).collect();
    let pp = db
        .post
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt())
        .fold(0i64, |n, c| n + c.is_some() as i64);
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let pd: HashIdx<Id<Post>, (i64, Option<Id<PostHistory>>)> = (&pp).filt(|n| n > 5).and(edits.opt()).collect();
    let mut out = Vec::new();
    (&us).cross(&pd).filt(|((t, r), (n, _)): ((i64, i64), (i64, Option<Id<PostHistory>>))| t > 20 || (r >= 1000 && n > 2)).drive(|(u, p), (_, (n, h))| out.push((u, p, n, h)));
    rows(out.into_iter().map(|(u, p, n, h)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.push(V::I(n));
        f.extend(match h {
            Some(h) => [ostr(db.post_history.comment.get(h)), V::T(db.post_history.creation_date.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank, p.OwnerUserId,
//        u.Reputation FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' AND p.PostTypeId IN (1, 2)),
// DistinctTags AS (SELECT DISTINCT TRIM(REGEXP_SPLIT_TO_TABLE(p.Tags, '>')) AS TagName, p.Id AS PostId FROM Posts p WHERE p.Tags IS NOT NULL),
// PostScores AS (SELECT rp.PostId, COALESCE(v.TotalVotes, 0) AS TotalVotes, COALESCE(b.BadgeCount, 0) AS BadgeCount, rp.Reputation, (rp.Score + COALESCE(v.TotalVotes, 0) + COALESCE(b.BadgeCount, 0)) AS TotalScore
//     FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId) v ON rp.PostId = v.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON rp.OwnerUserId = b.UserId)
// SELECT pt.TagName, COUNT(ps.PostId) AS PostsCount, AVG(ps.TotalScore) AS AverageScore
// FROM DistinctTags dt JOIN PostScores ps ON dt.PostId = ps.PostId JOIN Tags pt ON pt.TagName = dt.TagName GROUP BY pt.TagName
// HAVING AVG(ps.TotalScore) > (SELECT AVG(TotalScore) FROM PostScores) * 0.75 ORDER BY AverageScore DESC LIMIT 10;
//
// REGEXP_SPLIT_TO_TABLE on '>' leaves a '<' on every piece, which is then joined to Tags by name.
fn q22949(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, tags_str, origid, .. } = &db.post;
    let vc = db.vote.group_by(&db.vote.post_id).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let rp = || db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user);
    let ps = rp().group_by(Ident::<Post>::new()).select(score.and(origid.select(&vc).opt()).and(owner_user.select(&bc).opt())).fold(0i64, |_, ((s, v), b)| s + v.unwrap_or(0) + b.unwrap_or(0));
    let all = (&ps).map(|_| ()).inv().select(&ps).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let (n, s) = (&all).fold_flat((0i64, 0i64), |_, x| x);
    let overall = s as f64 / n as f64;
    let pieces = |t: Str| t.split('>').map(|x| x.trim()).collect::<Vec<_>>();
    let by_name: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let dt: MatSet<(Str, Id<Post>)> = db.post.select(tags_str.flat_map(pieces).and(Ident::<Post>::new())).map(|x| x).collect();
    type D = (Str, Id<Post>);
    let g = (&dt)
        .group_by(Same::<D>::new().map(|(t, _): D| t))
        .select(Same::<D>::new().map(|(t, _): D| t).select(&by_name).and(Same::<D>::new().map(|(_, p): D| p).select(&ps)))
        .fold((0i64, 0i64), |(n, s), (_, x)| (n + 1, s + x));
    let v = top_n(drain((&g).filt(|(n, s): (i64, i64)| s as f64 / n as f64 > overall * 0.75)), |&(t, (n, s))| (Reverse(fkey(s as f64 / n as f64)), t), 10);
    rows(v.into_iter().map(|(t, (n, s))| row(vec![V::S(t), V::I(n), V::F(s as f64 / n as f64)])))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        COALESCE(COUNT(DISTINCT C.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalVotes, AVG(COALESCE(P.Score, 0)) AS AverageScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (2, 3) GROUP BY U.Id, U.DisplayName),
// PopularUsers AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, CommentCount, TotalVotes, AverageScore, RANK() OVER (ORDER BY TotalVotes DESC) AS VoteRank,
//        RANK() OVER (ORDER BY AnswerCount DESC) AS AnswerRank FROM UserPostStats WHERE AverageScore > 0 AND QuestionCount > 5),
// BannedUsers AS (SELECT DISTINCT UserId FROM Votes WHERE VoteTypeId = 10),
// FinalResults AS (SELECT PU.UserId, PU.DisplayName, PU.QuestionCount, PU.AnswerCount, PU.CommentCount, PU.TotalVotes, PU.AverageScore, CASE WHEN BU.UserId IS NOT NULL THEN 'Banned' ELSE 'Active' END AS UserStatus
//     FROM PopularUsers PU LEFT JOIN BannedUsers BU ON PU.UserId = BU.UserId)
// SELECT UserId, DisplayName, QuestionCount, AnswerCount, CommentCount, TotalVotes, AverageScore, UserStatus FROM FinalResults WHERE UserStatus = 'Active' ORDER BY AverageScore DESC, TotalVotes DESC;
//
// The ranks are never read. The distinct comment count is a second fold over one row per comment.
fn q23994(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let ud = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(comments_of(db).opt().and(ud.opt()))).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, s), (_, v))) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + v.is_some() as i64, a[3] + s, a[4] + 1],
            None => [a[0], a[1], a[2], a[3], a[4] + 1],
        });
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let banned: MatSet<Id<User>> = db.vote.with((&db.vote.vote_type_id).eq(10)).select(&db.vote.user).collect();
    let v = drain(db.user.minus(&banned).select((&ups).filt(|a| a[3] > 0 && a[0] > 5).and(&cc)));
    rows(v.into_iter().map(|(u, (a, c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(a[2]), avg(a[3], a[4]), V::S("Active")]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Ranking, COUNT(c.Id) AS TotalComments
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId)
// SELECT us.UserId, us.DisplayName, us.Upvotes, us.Downvotes, us.PostCount, us.CommentCount, us.BadgeCount, pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.Ranking, pd.TotalComments
// FROM UserStats us LEFT JOIN PostDetails pd ON us.UserId = pd.PostId
// WHERE us.Upvotes > us.Downvotes AND us.PostCount > 5 AND EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = us.UserId AND b.Class = 1)
// ORDER BY us.Upvotes DESC, us.PostCount DESC, us.BadgeCount DESC LIMIT 100;
//
// The EXISTS keeps only users with a gold badge, so the post x comment x vote x badge product is driven for those alone. `us.UserId = pd.PostId` joins a user id to a
// post id, so it goes through the raw ids. Ranking ties are broken by post id.
fn q2479(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, origid, .. } = &db.post;
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let us = (&gold)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let dp = (&gold).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let dc = (&gold).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let dbg = (&gold).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let w = recent().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    type K = (Id<Post>, i64);
    let rk: MatSet<K> = (&w).map(|((p, _), k)| (p, k)).collect();
    let by_raw: HashIdx<i64, K> = (&rk).map(|(p, _): K| p).select(origid).inv().collect();
    let tc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pd = (&by_raw).select(Same::<K>::new().and(Same::<K>::new().map(|(p, _): K| p).select(&tc)));
    let uo = (&db.user.origid).select(pd).opt();
    let v = drain((&us).filt(|a| a[0] > a[1]).and((&dp).filt(|n| n > 5)).and(&dc).and(&dbg).and(uo));
    let v = top_n(v, |&(u, ((((a, n), _), b), _))| (Reverse(a[0]), Reverse(n), Reverse(b), u), 100);
    rows(v.into_iter().map(|(u, ((((a, n), c), b), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(c), V::I(b)]);
        f.extend(match p {
            Some(((p, k), t)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "score", "views"]);
                g.extend([V::I(k), V::I(t)]);
                g
            }
            None => (0..7).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate ASC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days' GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.Score, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.CommentCount FROM RankedPosts rp WHERE rp.PostRank <= 10),
// PostDetails AS (SELECT tp.Title, tp.OwnerDisplayName, tp.ViewCount, tp.CommentCount, COALESCE(SUM(CASE WHEN v.UserId IS NOT NULL AND vt.Id IN (2, 6) THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.UserId IS NOT NULL AND vt.Id = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN v.UserId IS NOT NULL AND vt.Id = 10 THEN 1 ELSE 0 END), 0) AS CloseVotes
//     FROM TopPosts tp LEFT JOIN Votes v ON tp.PostId = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY tp.Title, tp.OwnerDisplayName, tp.ViewCount, tp.CommentCount)
// SELECT pd.Title, pd.OwnerDisplayName, pd.ViewCount, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.CloseVotes FROM PostDetails pd ORDER BY pd.ViewCount DESC, pd.CommentCount DESC;
//
// PostDetails groups by the column tuple, not by the post.
fn q5444(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, title, view_count, .. } = &db.post;
    let recent = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -7)))).with(owner_user);
    let w = whole(recent()).select(Ident::<Post>::new().and(score).and(creation_date)).window(rank, |((_, s), d)| (Reverse(s), d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let key = title.opt().and(owner_user.select(&db.user.display_name)).and(view_count.opt()).and(&cc);
    let voted = votes_of(db).select(Ident::<Vote>::new().with(&db.vote.user_id)).select(&db.vote.vote_type_id);
    let g = (&tp).group_by(key).select(voted.opt()).fold([0i64; 3], |a, t| [a[0] + matches!(t, Some(2 | 6)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(10)) as i64]);
    let mut v = drain(&g);
    v.sort_by_key(|&((((_, _), w), c), _)| (w.is_none(), Reverse(w), Reverse(c)));
    rows(v.into_iter().map(|((((t, n), w), c), a)| {
        let mut f = vec![ostr(t), V::S(n), oint(w), V::I(c)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, BadgeCount, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats),
// ActiveUsers AS (SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.PostCount, TU.AnswerCount, TU.QuestionCount, TU.BadgeCount, TU.ReputationRank, COALESCE(COM.TotalComments, 0) AS TotalComments
//     FROM TopUsers TU LEFT JOIN (SELECT C.UserId, COUNT(C.Id) AS TotalComments FROM Comments C GROUP BY C.UserId) COM ON TU.UserId = COM.UserId)
// SELECT AU.DisplayName, AU.Reputation, AU.PostCount, AU.AnswerCount, AU.QuestionCount, AU.BadgeCount, AU.ReputationRank, AU.TotalComments,
//        (AU.AnswerCount::FLOAT / NULLIF(AU.QuestionCount, 0)) * 100 AS AnswerToQuestionRatio, (AU.BadgeCount::FLOAT / NULLIF(AU.PostCount, 0)) * 100 AS BadgeToPostRatio
// FROM ActiveUsers AU WHERE AU.ReputationRank <= 10 ORDER BY AU.Reputation DESC, AU.TotalComments DESC;
//
// FLOAT is single precision, so the ratios are computed in f32. ReputationRank reads only Reputation, so the top users are picked first and the post x badge product is
// driven for those alone.
fn q9363(db: &'static So) -> String {
    let w = whole(db.user.with((&db.user.reputation).gt(1000))).select(Ident::<User>::new().and(&db.user.reputation)).window(dense_rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let us = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(1)) as i64, a[2] + b.is_some() as i64]);
    let dp = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let com = db.comment.group_by(&db.comment.user).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    type T = (Id<User>, i64);
    let uk = || Same::<T>::new().map(|(u, _): T| u);
    let mut v = drain((&tu).select(Same::<T>::new().and(uk().select(&us)).and(uk().select(&dp)).and(uk().select(&com).opt())));
    v.sort_by_key(|&(_, ((((u, _), _), _), c))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(c.unwrap_or(0))));
    let ratio = |x: i64, y: i64| if y == 0 { V::Null } else { V::F(((x as f32 / y as f32) * 100.0f32) as f64) };
    rows(v.into_iter().map(|(_, ((((u, k), a), n), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(k), V::I(c.unwrap_or(0)), ratio(a[0], a[1]), ratio(a[2], n)]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN v.VoteTypeId = 1 THEN 1 END) AS AcceptedAnswers FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END), 0) AS CloseCount,
//        COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 ELSE 0 END), 0) AS DeleteCount
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.Score, p.ViewCount),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ROW_NUMBER() OVER (PARTITION BY UPPER(ps.Title) ORDER BY ps.Score DESC, ps.ViewCount DESC) AS Rank, ps.CloseCount, ps.DeleteCount
//     FROM PostStatistics ps)
// SELECT ups.UserId, ups.DisplayName, rp.Title AS PostTitle, rp.Score AS PostScore, rp.ViewCount AS PostViews, ups.UpVotes, ups.DownVotes, ups.AcceptedAnswers, rp.CloseCount, rp.DeleteCount
// FROM UserVoteSummary ups JOIN RankedPosts rp ON ups.UserId IN (SELECT DISTINCT p.OwnerUserId FROM Posts p WHERE p.Id = rp.PostId)
// WHERE ups.UpVotes > ups.DownVotes AND rp.Rank <= 5 ORDER BY ups.UpVotes DESC, rp.Score DESC;
//
// The IN subquery pairs each ranked post with its owner. Rank ties are broken by post id.
fn q1500(db: &'static So) -> String {
    let Post { creation_date, title, score, view_count, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let up = |t: Option<Str>| -> Option<Str> { t.map(|t| &*Box::leak(t.to_uppercase().into_boxed_str())) };
    let w = recent()
        .group_by(title.opt().map(up))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 5).map(|(((p, _), _), _)| p).collect();
    let ps = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.post_history_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + matches!(t, Some(10 | 11)) as i64, a[1] + matches!(t, Some(12 | 13)) as i64]
    });
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(1)) as i64]
    });
    let v = drain((&ps).and(owner_user.select(Ident::<User>::new().and((&uv).filt(|a| a[0] > a[1])))));
    let v = top_n(v, |&(p, (_, (_, a)))| (Reverse(a[0]), Reverse(score.get(p).unwrap()), p), 0);
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend(a.map(V::I));
        f.extend(c.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(CASE WHEN up.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN down.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
//     FROM Users u LEFT JOIN Votes up ON u.Id = up.UserId AND up.VoteTypeId = 2 LEFT JOIN Votes down ON u.Id = down.UserId AND down.VoteTypeId = 3 GROUP BY u.Id, u.Reputation),
// InterestingPosts AS (SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.CommentCount, ur.Reputation, (ur.TotalUpVotes - ur.TotalDownVotes) AS VoteBalance
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.CommentCount > 5 AND ur.Reputation > 100)
// SELECT ip.PostId, ip.Title, ip.CreationDate, ip.CommentCount, ip.Reputation, ip.VoteBalance,
//        CASE WHEN ip.VoteBalance > 0 THEN 'Popular' WHEN ip.VoteBalance < 0 THEN 'Unpopular' ELSE 'Neutral' END AS PopularityStatus
// FROM InterestingPosts ip WHERE ip.VoteBalance IS NOT NULL ORDER BY ip.VoteBalance DESC, ip.CommentCount DESC LIMIT 10;
//
// Only CommentCount is read from RankedPosts, and a COUNT(DISTINCT) is not changed by the vote join, so it is one fold over the comments. The up x down vote
// product of UserReputation is driven.
fn q4635(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let cc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let of = |t: i64| votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(t)));
    let ur = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(of(2).opt().and(of(3).opt()))
        .fold([0i64; 2], |a, (u, d)| [a[0] + u.is_some() as i64, a[1] + d.is_some() as i64]);
    let v = drain((&cc).filt(|n| n > 5).and(owner_user.select(Ident::<User>::new().and(&ur))));
    let v = top_n(v, |&(p, (n, (_, a)))| (Reverse(a[0] - a[1]), Reverse(n), p), 10);
    rows(v.into_iter().map(|(p, (n, (u, a)))| {
        let b = a[0] - a[1];
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(n), user_col(db, u, "rep"), V::I(b), V::S(if b > 0 { "Popular" } else if b < 0 { "Unpopular" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.OwnerUserId, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.OwnerUserId),
// PostDetails AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.CommentCount, COALESCE(UR.DisplayName, 'Anonymous') AS OwnerName, UR.Reputation AS OwnerReputation, R.ReputationRank
//     FROM RecentPosts RP LEFT JOIN Users U ON RP.OwnerUserId = U.Id LEFT JOIN UserReputation UR ON U.Id = UR.UserId
//     JOIN (SELECT DISTINCT UserId, ReputationRank FROM UserReputation WHERE ReputationRank <= 10) R ON U.Id = R.UserId)
// SELECT PD.PostId, PD.Title, PD.CreationDate, PD.Score, PD.CommentCount, PD.OwnerName, PD.OwnerReputation, PD.ReputationRank,
//        CASE WHEN PD.Score > 0 THEN 'High' WHEN PD.Score = 0 THEN 'Neutral' ELSE 'Low' END AS ScoreCategory
// FROM PostDetails PD WHERE PD.CommentCount > 5 ORDER BY PD.Score DESC, PD.CreationDate DESC FETCH FIRST 10 ROWS ONLY;
fn q4988(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tr: MatSet<(Id<User>, (Id<User>, i64))> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, (u, k))).collect();
    let rank = by_first(&tr);
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let v = drain((&rp).filt(|n| n > 5).and(owner_user.select(&rank)));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (n, (u, k)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(n));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(k), V::S(if s > 0 { "High" } else if s == 0 { "Neutral" } else { "Low" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(EXTRACT(EPOCH FROM (COALESCE(p.LastActivityDate, TIMESTAMP '2024-10-01 12:34:56') - p.CreationDate))) AS AvgPostAgeSeconds
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, QuestionCount, AnswerCount, AvgPostAgeSeconds, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserActivity),
// StackOverflowStats AS (SELECT COUNT(*) AS TotalPosts, COUNT(DISTINCT u.Id) AS TotalUsers, COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 1) AS TotalQuestions,
//        COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 2) AS TotalAnswers, AVG(p.Score) AS AvgPostScore FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id)
// SELECT tu.UserId, tu.DisplayName, tu.PostCount, tu.TotalScore, tu.QuestionCount, tu.AnswerCount, (SELECT TotalPosts FROM StackOverflowStats) AS TotalPosts, (SELECT TotalUsers FROM StackOverflowStats) AS TotalUsers,
//        (SELECT TotalQuestions FROM StackOverflowStats) AS TotalQuestions, (SELECT TotalAnswers FROM StackOverflowStats) AS TotalAnswers, (SELECT AvgPostScore FROM StackOverflowStats) AS AvgPostScore
// FROM TopUsers tu WHERE tu.ScoreRank <= 10 ORDER BY tu.TotalScore DESC;
//
// StackOverflowStats is read only through scalar subqueries, so its aggregates are computed once. AvgPostAgeSeconds is never projected.
fn q9465(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ups = user_posts(db);
    let w = whole(&ups).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| Reverse(a[4]), asc);
    let tu: MatSet<(Id<User>, [i64; 10])> = (&w).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let s = db.post.with(owner_user).select(post_type_id.and(score)).fold_flat([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let n = (&whole(db.post.with(owner_user)).select(owner_user).count_distinct()).fold_flat(0i64, |_, x| x);
    let mut v = drain(&tu);
    v.sort_by_key(|&((_, a), _)| Reverse(a[4]));
    rows(v.into_iter().map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[4]), V::I(a[2]), V::I(a[3]), V::I(s[0]), V::I(n), V::I(s[1]), V::I(s[2]), avg(s[3], s[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount, ROW_NUMBER() OVER(PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS RankByScore,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER(PARTITION BY P.Id) AS UpvoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER(PARTITION BY P.Id) AS DownvoteCount,
//        COALESCE(PH.Comment, 'No close reason provided') AS CloseReason
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId = 10
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.RankByScore, RP.UpvoteCount, RP.DownvoteCount,
//        CASE WHEN RP.UpvoteCount IS NULL THEN 'None' WHEN RP.UpvoteCount - RP.DownvoteCount >= 0 THEN 'Positive' ELSE 'Negative' END AS VoteSentiment FROM RankedPosts RP WHERE RP.RankByScore <= 10)
// SELECT FP.PostId, FP.Title, FP.CreationDate, FP.Score, FP.ViewCount, FP.VoteSentiment, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM FilteredPosts FP LEFT JOIN Badges B ON FP.PostId = B.UserId GROUP BY FP.PostId, FP.Title, FP.CreationDate, FP.Score, FP.ViewCount, FP.VoteSentiment
// ORDER BY FP.Score DESC, FP.CreationDate DESC;
//
// RankedPosts has no GROUP BY: it is one row per post x vote x close-history row, and the ranks and window counts are over those rows (RankByScore ties broken by
// post, vote and history id). The GROUP BY columns are all fixed by the post. `FP.PostId = B.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q2540(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).opt();
    type J = ((Id<Post>, Option<Id<Vote>>), Option<Id<PostHistory>>);
    let j: MatSet<J> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(Ident::<Post>::new().and(votes_of(db).opt()).and(closes)).collect();
    let pk = || Same::<J>::new().map(|((p, _), _): J| p);
    let wc = (&j).group_by(pk()).select(Same::<J>::new().flat_map(|((_, v), _): J| v).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let w = (&j).group_by(pk().select(post_type_id)).select(Same::<J>::new().and(pk().select(score))).window(row_number, |(x, s)| (Reverse(s), x), asc);
    let fp = (&w).filt(|(_, n)| n <= 10).map(|((x, _), _)| x);
    let bu: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let bdg = origid.select(&bu).select(&db.badge.class);
    let g = (&fp).group_by(pk()).select(pk().select(bdg.opt())).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let mut v = drain((&g).and(&wc));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (b, w))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::S(if w[0] - w[1] >= 0 { "Positive" } else { "Negative" }));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id LEFT JOIN Comments C ON P.Id = C.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COALESCE(SUM(CASE WHEN C.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalComments
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate BETWEEN '2022-01-01' AND '2023-12-31' GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.TotalUpVotes - PS.TotalDownVotes AS NetVotes,
//        ROW_NUMBER() OVER (ORDER BY PS.Score DESC, (PS.TotalUpVotes - PS.TotalDownVotes) DESC) AS Rank FROM PostStats PS WHERE PS.TotalComments > 10)
// SELECT UPS.UserId, UPS.DisplayName, UPS.UpVotes, UPS.DownVotes, UPS.TotalPosts, UPS.CommentCount, TP.Title, TP.CreationDate, TP.Score, TP.NetVotes
// FROM UserVoteStats UPS JOIN TopPosts TP ON UPS.UpVotes > 10 OR UPS.DownVotes > 5 WHERE TP.Rank <= 50 ORDER BY UPS.UpVotes DESC, TP.Score DESC;
//
// The ON clause names only UPS, so the qualifying users are crossed with the top posts. Rank ties are broken by post id.
fn q4922(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let Vote { post, vote_type_id, .. } = &db.vote;
    let us = || db.user.with((&db.user.reputation).gt(1000));
    let uvs = us()
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id.and(post.select(comments_of(db).opt()).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.map(|x| x.0);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let dp = us().group_by(Ident::<User>::new()).select(votes_by(db).select(post)).count_distinct();
    let dc = us().group_by(Ident::<User>::new()).select(votes_by(db).select(post).select(comments_of(db))).count_distinct();
    let ups: HashIdx<Id<User>, (([i64; 2], i64), i64)> = (&uvs)
        .filt(|a| a[0] > 10 || a[1] > 5)
        .and((&dp).opt().map(|n: Option<i64>| n.unwrap_or(0)))
        .and((&dc).opt().map(|n: Option<i64>| n.unwrap_or(0)))
        .collect();
    let ps = db
        .post
        .with(creation_date.between(ts(2022, 1, 1, 0, 0, 0), ts(2023, 12, 31, 0, 0, 0)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(vote_type_id).opt().and(comments_of(db).select((&db.comment.user_id).opt()).opt()))
        .fold([0i64; 2], |a, (t, c)| [a[0] + (t == Some(2)) as i64 - (t == Some(3)) as i64, a[1] + c.flatten().is_some() as i64]);
    let tp = rel(top_n(drain((&ps).filt(|a| a[1] > 10)), |&(p, a)| (Reverse(score.get(p).unwrap()), Reverse(a[0]), p), 50));
    let mut v = Vec::new();
    (&ups).cross(&tp).drive(|(u, _), (((a, n), c), (p, t))| v.push((u, a, n, c, p, t[0])));
    v.sort_by_key(|&(_, a, _, _, p, _)| (Reverse(a[0]), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(u, a, n, c, p, t)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(c)]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.push(V::I(t));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.UpVotes, u.DownVotes, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(p.Score) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.UpVotes, u.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStatistics),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, COALESCE(ph.Comment, 'No comment') AS HistoryComment, pt.Name AS PostType
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10 LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= NOW() - INTERVAL '1 month')
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, pd.Title, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.PostType, tu.ScoreRank,
//        CASE WHEN pd.CommentCount > 0 THEN 'Active Discussion' ELSE 'No Activity' END AS DiscussionStatus
// FROM TopUsers tu JOIN PostDetails pd ON tu.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pd.PostId) WHERE tu.ScoreRank <= 10 ORDER BY tu.ScoreRank, pd.ViewCount DESC;
//
// NOW() is a TIMESTAMPTZ, so the cut is taken on the New York wall clock.
fn q4420(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let us = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt()).fold((0i64, 0i64), |(n, s), x| (n + x.is_some() as i64, s + x.unwrap_or(0)));
    let w = whole(&us).select(Ident::<User>::new().and(&us)).window(rank, |(_, (n, s))| (n == 0, Reverse(s)), asc);
    let tr: MatSet<(Id<User>, (Id<User>, i64, i64))> = (&w).filt(|(_, k)| k <= 10).map(|((u, (n, _)), k)| (u, (u, n, k))).collect();
    let rank = by_first(&tr);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).opt();
    let v = drain(db.post.with(creation_date.ge(add_months(utc_to_ny(now_utc()), -1))).select(closes.and(owner_user.select(&rank))));
    let v = top_n(v, |&(p, (_, (_, _, k)))| (k, db.post.view_count.get(p).is_none(), Reverse(db.post.view_count.get(p))), 0);
    rows(v.into_iter().map(|(p, (_, (u, n, k)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["title", "views", "answers", "comments", "type"]));
        f.extend([V::I(k), V::S(if db.post.comment_count.get(p).unwrap() > 0 { "Active Discussion" } else { "No Activity" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score >= 10),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteDetails AS (SELECT p.Id AS PostId, COUNT(v.Id) AS TotalVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, pvd.TotalVotes, pvd.UpVotes, pvd.DownVotes
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostVoteDetails pvd ON rp.PostId = pvd.PostId
// WHERE (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) > 5 AND (SELECT COUNT(DISTINCT c.UserId) FROM Comments c WHERE c.PostId = rp.PostId) > 3
// ORDER BY rp.Score DESC, ub.BadgeCount DESC LIMIT 100;
//
// Rank is never read.
fn q21608(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.ge(10))).with(owner_user);
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let du = rp().group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.user_id)).count_distinct();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let pvd = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain((&cc).filt(|n| n > 5).and((&du).filt(|n| n > 3)).and(owner_user.select(&ub)).and(&pvd));
    let v = top_n(v, |&(p, ((_, b), _))| (Reverse(score.get(p).unwrap()), Reverse(b[0]), p), 100);
    rows(v.into_iter().map(|(p, ((_, b), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(b.map(V::I));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, p.AcceptedAnswerId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, p.AcceptedAnswerId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopContributors AS (SELECT ua.UserId, ua.DisplayName, ua.TotalViews, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges, ROW_NUMBER() OVER (ORDER BY ua.TotalViews DESC) AS Rank FROM UserActivity ua WHERE ua.TotalViews > 0)
// SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, uc.DisplayName, uc.TotalViews, uc.GoldBadges, uc.SilverBadges, uc.BronzeBadges
// FROM RecentPosts rp JOIN TopContributors uc ON rp.OwnerUserId = uc.UserId WHERE rp.AcceptedAnswerId IS NOT NULL AND uc.Rank <= 10 ORDER BY rp.CreationDate DESC;
//
// rn is never read. Rank ties are broken by user id.
fn q3205(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, accepted_answer_id, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (w, c)| [a[0] + w.flatten().unwrap_or(0), a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]);
    let tc = rel(top_n(drain((&ua).filt(|a| a[0] > 0)), |&(u, a)| (Reverse(a[0]), u), 10));
    let tci: HashIdx<Id<User>, (Id<User>, [i64; 4])> = (&tc).map(|(u, _)| u).inv().select(&tc).collect();
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(accepted_answer_id)
        .with(owner_user.select(&tci))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&rp).and(owner_user.select(&tci)));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(a.map(V::I));
        f.push(user_col(db, u, "name"));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.LastActivityDate, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.LastActivityDate DESC) AS ActivityRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.PostTypeId, p.LastActivityDate),
// VoteStatistics AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CommentCount, CASE WHEN cps.CloseCount IS NULL THEN 'Open' ELSE 'Closed' END AS PostStatus, vs.UpVotes, vs.DownVotes,
//        (COALESCE(vs.UpVotes, 0) - COALESCE(vs.DownVotes, 0)) AS NetVotes, rp.ActivityRank
//     FROM RankedPosts rp LEFT JOIN VoteStatistics vs ON rp.PostId = vs.PostId LEFT JOIN ClosedPosts cps ON rp.PostId = cps.PostId WHERE rp.ActivityRank <= 5)
// SELECT PostId, Title, CommentCount, PostStatus, UpVotes, DownVotes, NetVotes FROM FinalResults
// WHERE (NetVotes > 0 AND PostStatus = 'Open') OR (PostStatus = 'Closed' AND CommentCount > 5) ORDER BY NetVotes DESC, CommentCount DESC;
//
// VoteStatistics is keyed by the raw Votes.PostId.
fn q24277(db: &'static So) -> String {
    let Post { post_type_id, creation_date, last_activity_date, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(last_activity_date)).window(rank, |(_, d)| Reverse(d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    type R = ((i64, Option<[i64; 2]>), Option<Id<Post>>);
    let keep = |((c, a), cl): R| {
        let a = a.unwrap_or([0; 2]);
        (a[0] - a[1] > 0 && cl.is_none()) || (cl.is_some() && c > 5)
    };
    let mut v = drain((&cc).and(origid.select(&vs).opt()).and(Ident::<Post>::new().with(&closed).opt()).filt(keep));
    v.sort_by_key(|&(_, ((c, a), _))| {
        let a = a.unwrap_or([0; 2]);
        (Reverse(a[0] - a[1]), Reverse(c))
    });
    rows(v.into_iter().map(|(p, ((c, a), cl))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c), V::S(if cl.is_some() { "Closed" } else { "Open" })]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])],
            None => [V::Null, V::Null, V::I(0)],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, B.Name AS BadgeName, B.Class, B.Date AS AwardDate, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY B.Date DESC) AS BadgeRank
//     FROM Users U JOIN Badges B ON U.Id = B.UserId),
// PostAnalytics AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.AnswerCount, COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount,
//        DENSE_RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank, DENSE_RANK() OVER (ORDER BY P.ViewCount DESC) AS ViewRank, MAX(Ph.CreationDate) AS LastUpdateDate
//     FROM Posts P LEFT JOIN PostHistory Ph ON P.Id = Ph.PostId GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.AnswerCount),
// TopPosts AS (SELECT PostId, Title, ViewCount, CommentCount, AnswerCount, Score, LastUpdateDate, CTE1.BadgeName FROM PostAnalytics P
//     LEFT JOIN UserBadges CTE1 ON CTE1.BadgeName IN ('Gold', 'Silver') AND P.ScoreRank <= 5 WHERE P.ViewCount > 100)
// SELECT U.DisplayName, COUNT(DISTINCT B.Id) AS BadgeCount, COUNT(DISTINCT TP.PostId) AS PostCount, AVG(TP.ViewCount) AS AvgViews, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, MAX(TP.LastUpdateDate) AS LatestPostUpdate
// FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN TopPosts TP ON TP.BadgeName IS NOT NULL WHERE U.Reputation > 5000 GROUP BY U.Id, U.DisplayName HAVING AVG(TP.ViewCount) > 50;
//
// WITH RECURSIVE, but no CTE refers to itself. The TopPosts rows with a BadgeName are the top-five-score posts with more than 100 views crossed with the badges named
// Gold or Silver; the final LEFT JOIN names only TP, so every user is crossed with those rows (a LEFT JOIN on a relation keyed by `()`).
fn q31144(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let w = whole(db.post.iq()).select(Ident::<Post>::new().and(score)).window(dense_rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let named: MatSet<Id<Badge>> = db.badge.with((&db.badge.name).is_in(["Gold", "Silver"])).with(&db.badge.user).collect();
    let lu = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.creation_date)).fold(i64::MIN, |m, d| m.max(d));
    let tpr: HashIdx<(), (Id<Post>, Id<Badge>)> = (&tp).with(view_count.gt(100)).cross(&named).map(|_| ()).inv().collect();
    let tpost = (&tpr).map(|(p, _): (Id<Post>, Id<Badge>)| p);
    let tpv = (&tpost).select(view_count.and((&lu).opt()));
    let users = || db.user.with((&db.user.reputation).gt(5000));
    let g = users()
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(Ident::<User>::new().map(|_| ()).select(&tpv).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, i64::MIN), |(n, s, g, sv, m), (c, t)| match t {
            Some((w, l)) => (n + 1, s + w, g + (c == Some(1)) as i64, sv + (c == Some(2)) as i64, m.max(l.unwrap_or(i64::MIN))),
            None => (n, s, g + (c == Some(1)) as i64, sv + (c == Some(2)) as i64, m),
        });
    let nb = users().group_by(Ident::<User>::new()).select(badges_of(db)).count_distinct();
    let np = users().group_by(Ident::<User>::new()).select(Ident::<User>::new().map(|_| ()).select(&tpost)).count_distinct();
    let v = drain((&g).filt(|(n, s, _, _, _): (i64, i64, i64, i64, i64)| n > 0 && s as f64 / n as f64 > 50.0).and((&nb).opt()).and((&np).opt()));
    rows(v.into_iter().map(|(u, (((n, s, g, sv, m), nb), np))| row(vec![user_col(db, u, "name"), V::I(nb.unwrap_or(0)), V::I(np.unwrap_or(0)), avg(s, n), V::I(g), V::I(sv), tmax(m)])))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
//        COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalUpVotes, TotalDownVotes, TotalPosts, TotalBadges, RANK() OVER (ORDER BY TotalUpVotes - TotalDownVotes DESC, TotalPosts DESC) AS UserRank FROM UserVoteStats),
// RecentPopularPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS PopularityRank FROM Posts p
//     WHERE p.Score > 10 AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// EligibleBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b WHERE b.Class = 2 GROUP BY b.UserId)
// SELECT ru.DisplayName, ru.TotalUpVotes, ru.TotalDownVotes, ru.TotalPosts, COALESCE(eb.BadgeCount, 0) AS SilverBadges, rp.PostId, rp.Title, rp.CreationDate, rp.Score
// FROM RankedUsers ru LEFT JOIN EligibleBadges eb ON ru.UserId = eb.UserId LEFT JOIN RecentPopularPosts rp ON ru.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId LIMIT 1)
// WHERE ru.UserRank <= 10 ORDER BY ru.UserRank, rp.Score DESC;
//
// The correlated `LIMIT 1` looks a post up by its id, so it is the post's owner.
fn q34967(db: &'static So) -> String {
    let Post { score, creation_date, owner_user, .. } = &db.post;
    let us = || db.user.with((&db.user.reputation).gt(100));
    let ups = us()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let dp = us().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(us()).select(Ident::<User>::new().and(&ups).and(&dp)).window(rank, |((_, a), n)| (Reverse(a[0] - a[1]), Reverse(n)), asc);
    let ru: MatSet<(Id<User>, [i64; 2], i64, i64)> = (&w).filt(|(_, k)| k <= 10).map(|(((u, a), n), k)| (u, a, n, k)).collect();
    let eb = db.badge.with((&db.badge.class).eq(2)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let rpp: HashIdx<Id<User>, Id<Post>> = db.post.with(score.gt(10).and(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(owner_user).inv().collect();
    type T = (Id<User>, [i64; 2], i64, i64);
    let uk = || Same::<T>::new().map(|(u, _, _, _): T| u);
    let v = drain((&ru).select(Same::<T>::new().and(uk().select(&eb).opt()).and(uk().select(&rpp).opt())));
    let v = top_n(v, |&(_, (((_, _, _, k), _), p))| (k, p.is_none(), Reverse(p.map(|p| score.get(p).unwrap()))), 0);
    rows(v.into_iter().map(|(_, (((u, a, n, _), e), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n), V::I(e.unwrap_or(0))];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created", "score"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentRank FROM Posts p
//     WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.UpVotes, us.DownVotes, us.GoldBadges, us.SilverBadges, us.BronzeBadges FROM UserStatistics us WHERE us.Rank <= 10)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, COALESCE(rp.Title, 'No Recent Posts') AS LastPostTitle, COALESCE(rp.CreationDate, NULL) AS LastPostDate,
//        CASE WHEN tu.UpVotes > tu.DownVotes THEN 'Positive' ELSE 'Negative' END AS VoteSentiment
// FROM TopUsers tu LEFT JOIN RecentPosts rp ON tu.UserId = rp.OwnerUserId AND rp.RecentRank = 1 ORDER BY tu.Reputation DESC;
//
// Rank reads only the distinct post count, so the top users are picked first and the post x vote x badge product is driven for those alone.
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so the cut is taken on the New York wall clock.
fn q199(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, .. } = &db.post;
    let pc = db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&pc).select(Ident::<User>::new().and(&pc)).window(rank, |(_, n)| Reverse(n), asc);
    let tu: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let tus: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let s = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let since = add_days(utc_to_ny(now_utc()), -30);
    let rw = db.post.with(creation_date.ge(since)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let latest = (&rw).filt(|(_, k)| k == 1).map(|((p, _), _)| p);
    type T = (Id<User>, i64);
    let uk = || Same::<T>::new().map(|(u, _): T| u);
    let mut v = drain((&tu).select(Same::<T>::new().and(uk().select(&s)).and(uk().select(&latest).opt())));
    v.sort_by_key(|&(_, (((u, _), _), _))| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(_, (((u, n), a), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(match p {
            Some(p) => [V::S(title.get(p).unwrap_or("No Recent Posts")), V::T(creation_date.get(p).unwrap())],
            None => [V::S("No Recent Posts"), V::Null],
        });
        f.push(V::S(if a[0] > a[1] { "Positive" } else { "Negative" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostsWithAnswers AS (SELECT P.Id AS PostId, P.Title, P.AcceptedAnswerId, P.CreationDate, COUNT(DISTINCT P2.Id) AS AnswerCount, COALESCE(MAX(P2.ViewCount), 0) AS MaxViewCount
//     FROM Posts P LEFT JOIN Posts P2 ON P.Id = P2.ParentId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.AcceptedAnswerId, P.CreationDate),
// PostsHistoryClosure AS (SELECT PH.PostId, COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 END) AS ClosureCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId),
// RankedPosts AS (SELECT PWA.PostId, UBD.UserId, ROW_NUMBER() OVER (PARTITION BY UBD.UserId ORDER BY PWA.MaxViewCount DESC, PWA.AnswerCount DESC) AS Rank
//     FROM PostsWithAnswers PWA JOIN UserBadges UBD ON PWA.AcceptedAnswerId IS NOT NULL JOIN Users U ON U.Id = PWA.AcceptedAnswerId WHERE UBD.BadgeCount >= 5 AND PWA.MaxViewCount > 0)
// SELECT UBD.DisplayName, PWA.Title, PWA.CreationDate, COALESCE(PHC.ClosureCount, 0) AS ClosureCount, RP.Rank
// FROM RankedPosts RP JOIN PostsWithAnswers PWA ON RP.PostId = PWA.PostId JOIN UserBadges UBD ON RP.UserId = UBD.UserId LEFT JOIN PostsHistoryClosure PHC ON PWA.PostId = PHC.PostId
// WHERE RP.Rank <= 10 ORDER BY RP.Rank, PWA.CreationDate DESC;
//
// `JOIN UserBadges UBD ON PWA.AcceptedAnswerId IS NOT NULL` names only PWA, so the qualifying questions are crossed with the users with five or more badges;
// `U.Id = PWA.AcceptedAnswerId` joins a user id to a post id through the raw ids. Rank ties are broken by post id.
fn q24431(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, view_count, creation_date, .. } = &db.post;
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pwa = db
        .post
        .with(post_type_id.eq(1))
        .with(accepted_answer_id.select(&uid))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).select(view_count.opt()).opt())
        .fold((0i64, 0i64), |(n, m), c| match c {
            Some(w) => (n + 1, m.max(w.unwrap_or(0))),
            None => (n, m),
        });
    let q: HashIdx<(), (Id<Post>, (i64, i64))> = (&pwa).filt(|(_, m): (i64, i64)| m > 0).map(|_| ()).inv().select(Ident::<Post>::new().and(&pwa)).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = db
        .user
        .with((&bc).filt(|n| n >= 5))
        .group_by(Ident::<User>::new())
        .select(Ident::<User>::new().map(|_| ()).select(&q))
        .window(row_number, |(p, (n, m))| (Reverse(m), Reverse(n), p), asc);
    let hc = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    type R = (Id<Post>, i64);
    let rp = (&w).filt(|(_, k)| k <= 10).map(|((p, _), k)| (p, k));
    let mut v = drain(rp.select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(&hc).opt())));
    v.sort_by_key(|&(_, ((p, k), _))| (k, Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(u, ((p, k), c))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(k)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
//        SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName, u.Reputation),
// HighlyActiveUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalBounties, Rank FROM UserStats WHERE TotalPosts > 10),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.OwnerUserId, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentRank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT u.DisplayName, u.Reputation, ha.TotalPosts, ha.TotalAnswers, ha.TotalBounties, rp.PostId, rp.Title, rp.CreationDate,
//        CASE WHEN rp.PostTypeId = 1 THEN 'Question' WHEN rp.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount
// FROM HighlyActiveUsers ha JOIN RecentPosts rp ON ha.UserId = rp.OwnerUserId LEFT JOIN Users u ON ha.UserId = u.Id WHERE ha.Rank <= 50 AND rp.RecentRank = 1
// ORDER BY ha.TotalBounties DESC, u.Reputation DESC;
//
// Rank reads only Reputation, so the top fifty users are picked first (ties by user id) and the post x bounty product is driven for those alone.
fn q3592(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 50);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(bounty.opt()).opt())
        .fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let rw = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(dense_rank, |(_, d)| Reverse(d), asc);
    let latest = (&rw).filt(|(_, k)| k == 1).map(|((p, _), _)| p);
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&us).and((&pc).filt(|a| a[0] > 10)).and((&latest).select(Ident::<Post>::new().and((&cc).opt()))));
    let v = top_n(v, |&(u, ((b, _), _))| (Reverse(b), Reverse(db.user.reputation.get(u).unwrap())), 0);
    rows(v.into_iter().map(|(u, ((b, a), (p, c)))| {
        let t = post_type_id.get(p).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b)]);
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::S(if t == 1 { "Question" } else if t == 2 { "Answer" } else { "Other" }), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 2) AS UpvoteCount,
//        COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 3) AS DownvoteCount, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title),
// UserBadgeCounts AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldCount, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverCount, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeCount
//     FROM Badges b GROUP BY b.UserId),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount, ps.CloseCount, ROW_NUMBER() OVER (ORDER BY ps.UpvoteCount DESC) AS Rank FROM PostStats ps
//     WHERE ps.UpvoteCount - ps.DownvoteCount > 0)
// SELECT t.Title, t.CommentCount, t.UpvoteCount, t.DownvoteCount, t.CloseCount, u.DisplayName, COALESCE(ub.GoldCount, 0) AS GoldBadges, COALESCE(ub.SilverCount, 0) AS SilverBadges,
//        COALESCE(ub.BronzeCount, 0) AS BronzeBadges
// FROM TopPosts t JOIN Posts p ON t.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserBadgeCounts ub ON ub.UserId = u.Id WHERE t.Rank <= 10 ORDER BY t.UpvoteCount DESC;
//
// The distinct voter counts are a second fold over one row per vote. Rank ties are broken by post id.
fn q271(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps = rp()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 2], |a, ((c, _), t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(10)) as i64]);
    let voters = |t: i64| votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(t))).select(&db.vote.user_id);
    let upd = rp().group_by(Ident::<Post>::new()).select(voters(2)).count_distinct();
    let dnd = rp().group_by(Ident::<Post>::new()).select(voters(3)).count_distinct();
    let up = rp().select((&upd).opt().map(|n: Option<i64>| n.unwrap_or(0)));
    let dn = rp().select((&dnd).opt().map(|n: Option<i64>| n.unwrap_or(0)));
    let tp = top_n(drain((&ps).and(&up).and(&dn).filt(|((_, u), d): (([i64; 2], i64), i64)| u - d > 0)), |&(p, ((_, u), _))| (Reverse(u), p), 10);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let tp = rel(tp);
    type T = (Id<Post>, (([i64; 2], i64), i64));
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select(owner_user.select(Ident::<User>::new().and((&ub).opt()))).opt())));
    rows(v.into_iter().map(|(_, ((p, ((a, u), d)), o))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(a[0]), V::I(u), V::I(d), V::I(a[1])]);
        f.extend(match o {
            Some((x, b)) => {
                let b = b.unwrap_or([0; 3]);
                [user_col(db, x, "name"), V::I(b[0]), V::I(b[1]), V::I(b[2])]
            }
            None => [V::Null, V::I(0), V::I(0), V::I(0)],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT B.Id) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Upvotes, Downvotes, TotalPosts, TotalBadges, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats),
// PostAnalytics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT PL.RelatedPostId) AS RelatedPostsCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostLinks PL ON P.Id = PL.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount)
// SELECT TU.DisplayName, TU.Reputation, TU.Upvotes, TU.Downvotes, PA.Title, PA.CreationDate, PA.Score, PA.ViewCount, PA.CommentCount, PA.RelatedPostsCount
// FROM TopUsers TU JOIN PostAnalytics PA ON PA.PostId IN (SELECT AcceptedAnswerId FROM Posts WHERE OwnerUserId = TU.UserId)
// WHERE TU.Rank <= 10 ORDER BY TU.Reputation DESC, PA.Score DESC FETCH FIRST 50 ROWS ONLY;
//
// Rank reads only Reputation, so the top ten users are picked first (ties by user id) and the post x vote x badge product is driven for those alone. The IN pairs
// each top user with the questions that are an accepted answer of one of their posts; PostAnalytics is computed for those questions alone.
fn q854(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, score, .. } = &db.post;
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let acc = posts_of(db).select(accepted_answer).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let pairs: MatSet<(Id<User>, Id<Post>)> = (&tu).select(Ident::<User>::new().and(acc)).map(|x| x).collect();
    let ap: MatSet<Id<Post>> = (&pairs).map(|(_, p)| p).collect();
    let pa = (&ap).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(links_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let rld = (&ap).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let rl = (&ap).select((&rld).opt().map(|n: Option<i64>| n.unwrap_or(0)));
    type P = (Id<User>, Id<Post>);
    let v = drain((&pairs).select(Same::<P>::new().and(Same::<P>::new().map(|(u, _): P| u).select(&us)).and(Same::<P>::new().map(|(_, p): P| p).select((&pa).and(&rl)))));
    let v = top_n(v, |&(_, (((u, p), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(_, (((u, p), a), (c, r)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn FROM Posts p
//     WHERE p.PostTypeId IN (1, 2)),
// RecentPosts AS (SELECT rp.*, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.rn <= 10),
// ClosedReasonCounts AS (SELECT p.Id AS PostId, COUNT(ph.Id) AS CloseReasonCount FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10 GROUP BY p.Id),
// VotesSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.OwnerReputation, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes,
//        COALESCE(rc.CloseReasonCount, 0) AS CloseReasonCount, CASE WHEN vs.UpVotes IS NOT NULL AND vs.DownVotes IS NOT NULL THEN (vs.UpVotes + vs.DownVotes) ELSE 0 END AS TotalVotes,
//        CASE WHEN rc.CloseReasonCount > 0 THEN 'Closed' ELSE 'Open' END AS PostStatus, DATE_TRUNC('day', rp.CreationDate) AS CreationDateTruncated
// FROM RecentPosts rp LEFT JOIN VotesSummary vs ON rp.PostId = vs.PostId LEFT JOIN ClosedReasonCounts rc ON rp.PostId = rc.PostId ORDER BY rp.CreationDate DESC;
//
// rn ties are broken by post id. VotesSummary is keyed by the raw Votes.PostId.
fn q32813(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, .. } = &db.post;
    let w = db.post.with(post_type_id.is_in([1, 2])).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let vs = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let rc = (&tp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let mut v = drain((&rc).and(origid.select(&vs).opt()));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "rep"]);
        let b = a.unwrap_or([0; 2]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(c), V::I(a.map_or(0, |a| a[0] + a[1])), V::S(if c > 0 { "Closed" } else { "Open" }), V::T(trunc_day(creation_date.get(p).unwrap()))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS RankScore,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, pt.Name),
// ClosedPosts AS (SELECT ph.PostId, MIN(ph.CreationDate) AS FirstCloseDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, COALESCE(cp.FirstCloseDate, DATE '1970-01-01') AS FirstCloseDate,
//        CASE WHEN cp.FirstCloseDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.UpVotes, ps.DownVotes, ps.FirstCloseDate, ps.PostStatus,
//        CASE WHEN ps.Score > 10 THEN 'High Score' WHEN ps.Score BETWEEN 5 AND 10 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory, COUNT(*) OVER() AS TotalPosts
// FROM PostStatistics ps WHERE ps.PostStatus = 'Open' AND ps.UpVotes > ps.DownVotes ORDER BY ps.Score DESC LIMIT 10;
//
// Open means no close history, so FirstCloseDate is always the COALESCE default. RankScore is never read.
fn q4013(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .minus(&closed)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let open = (&rp).filt(|a| a[0] > a[1]);
    let total = count(&open);
    let v = top_n(drain(open), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::T(0), V::S("Open"), V::S(if s > 10 { "High Score" } else if s >= 5 { "Medium Score" } else { "Low Score" }), V::I(total)]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(SUM(c.Score), 0) AS TotalCommentScore, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01' AS DATE) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.TotalCommentScore, ps.UpvoteCount, ps.DownvoteCount, ROW_NUMBER() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC) AS Rank
//     FROM PostStatistics ps)
// SELECT ue.DisplayName, tp.Title, tp.ViewCount, tp.Score, tp.UpvoteCount, tp.DownvoteCount, tp.TotalCommentScore
// FROM UserEngagement ue JOIN Posts p ON ue.UserId = p.OwnerUserId JOIN TopPosts tp ON p.Id = tp.PostId WHERE ue.PostCount >= 5 ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Only PostCount is read from UserEngagement, and a COUNT(DISTINCT p.Id) is not changed by the vote and comment joins, so it is one fold over the posts. Rank is never read.
fn q7698(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let ps = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(&db.comment.score).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&ps).and(owner_user.select(Ident::<User>::new().with((&pc).filt(|n| n >= 5)))));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS ViewRank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.PostTypeId),
// RecentVotes AS (SELECT v.PostId, v.VoteTypeId, COUNT(*) AS VoteCount FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId, v.VoteTypeId),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CommentCount, COALESCE(SUM(rv.VoteCount) FILTER (WHERE rv.VoteTypeId = 2), 0) AS UpVotes,
//        COALESCE(SUM(rv.VoteCount) FILTER (WHERE rv.VoteTypeId = 3), 0) AS DownVotes FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId GROUP BY rp.PostId, rp.Title, rp.ViewCount, rp.CommentCount)
// SELECT pm.PostId, pm.Title, pm.ViewCount, pm.CommentCount, pm.UpVotes, pm.DownVotes,
//        CASE WHEN pm.UpVotes - pm.DownVotes > 0 THEN 'Positive' WHEN pm.UpVotes - pm.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment,
//        CASE WHEN pm.CommentCount > 5 THEN 'Highly Discussed' WHEN pm.CommentCount BETWEEN 1 AND 5 THEN 'Moderately Discussed' ELSE 'Not Discussed' END AS DiscussionLevel
// FROM PostMetrics pm WHERE pm.ViewCount > 50 AND (pm.UpVotes + pm.DownVotes) > 0 AND pm.CommentCount IS NOT NULL ORDER BY pm.ViewCount DESC, pm.UpVotes DESC LIMIT 10;
//
// The RecentVotes groups of a post partition its recent votes by type, so their summed counts are the post's recent up and down votes. ViewRank is never read.
fn q22076(db: &'static So) -> String {
    let Post { creation_date, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_days(t0, -30)))).select(&db.vote.vote_type_id);
    let pm = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .with(view_count.gt(50))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt())
        .fold(0i64, |n, c| n + c.is_some() as i64);
    let rv = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .with(view_count.gt(50))
        .group_by(Ident::<Post>::new())
        .select(recent.opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&pm).and((&rv).filt(|a| a[0] + a[1] > 0))), |&(p, (_, a))| (Reverse(view_count.get(p)), Reverse(a[0]), p), 10);
    rows(v.into_iter().map(|(p, (c, a))| {
        let d = a[0] - a[1];
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if d > 0 { "Positive" } else if d < 0 { "Negative" } else { "Neutral" }));
        f.push(V::S(if c > 5 { "Highly Discussed" } else if c >= 1 { "Moderately Discussed" } else { "Not Discussed" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// MostVotedPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, ub.BadgeCount, pvc.UpVotes, pvc.DownVotes,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(pvc.UpVotes - pvc.DownVotes, 0) DESC, rp.CreationDate DESC) AS MostVotedRank
//     FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostVoteCounts pvc ON rp.Id = pvc.PostId WHERE rp.PostRank = 1)
// SELECT mvp.Title, mvp.CreationDate, mvp.BadgeCount, mvp.UpVotes, mvp.DownVotes, CASE WHEN mvp.BadgeCount IS NULL THEN 'No Badges' ELSE 'Has Badges' END AS BadgeStatus
// FROM MostVotedPosts mvp WHERE mvp.MostVotedRank <= 10 ORDER BY mvp.UpVotes DESC, mvp.CreationDate DESC;
//
// PostRank ties are broken by post id; the ownerless posts rank among themselves. PostVoteCounts is keyed by the raw Votes.PostId.
fn q529(db: &'static So) -> String {
    let Post { owner_user, owner_user_id, creation_date, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pvc = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&first).select(owner_user.select(&bc).opt().and(origid.select(&pvc).opt())));
    let v = top_n(v, |&(p, (_, a))| (Reverse(a.map_or(0, |a| a[0] - a[1])), Reverse(creation_date.get(p).unwrap()), p), 10);
    let mut v = v;
    v.sort_by_key(|&(p, (_, a))| (a.is_none(), Reverse(a.map(|a| a[0])), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (b, a))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.push(oint(b));
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if b.is_none() { "No Badges" } else { "Has Badges" }));
        row(f)
    }))
}

// WITH UserRankings AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS Rank, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT P2.Id) AS TotalAnswers
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Posts P2 ON U.Id = P2.OwnerUserId AND P2.PostTypeId = 2 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Rank, TotalPosts, TotalAnswers FROM UserRankings WHERE Rank <= 10),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT V.Id) AS VoteCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 month'
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount),
// UserPostInteraction AS (SELECT U.Id AS UserId, U.DisplayName, P.Id AS PostId, PS.Title, PS.Score, PS.ViewCount, PS.CommentCount, PS.VoteCount FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId
//     JOIN PostStatistics PS ON P.Id = PS.PostId)
// SELECT TU.DisplayName AS TopUser, COUNT(DISTINCT UPI.PostId) AS UserPostCount, SUM(UPI.Score) AS TotalScore, AVG(UPI.ViewCount) AS AverageViews, AVG(UPI.CommentCount) AS AverageComments,
//        AVG(UPI.VoteCount) AS AverageVotes
// FROM TopUsers TU JOIN UserPostInteraction UPI ON TU.UserId = UPI.UserId GROUP BY TU.DisplayName ORDER BY TotalScore DESC;
//
// Rank reads only Reputation, so the top users are picked first; TotalPosts and TotalAnswers are never read. The final GROUP BY is on the display name.
fn q9161(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let recent = Ident::<Post>::new().with(creation_date.ge(add_months(date(2024, 10, 1), -1)));
    let tp: MatSet<Id<Post>> = (&tu).select(posts_of(db).select(recent)).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let ps = (&cc).and(&vc);
    type Row = (((Id<Post>, i64), Option<i64>), (i64, i64));
    let upi: MatSet<(Id<User>, Row)> = (&tu).select(Ident::<User>::new().and(posts_of(db).select(Ident::<Post>::new().and(score).and(view_count.opt()).and(&ps)))).map(|x| x).collect();
    type U = (Id<User>, Row);
    let g = (&upi)
        .group_by(Same::<U>::new().map(|(u, _): U| u).select(&db.user.display_name))
        .select(Same::<U>::new().map(|(_, r): U| r))
        .fold([0i64; 6], |a, (((_, s), w), (c, v))| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c, a[5] + v]);
    let mut v = drain(&g);
    v.sort_by_key(|&(_, a)| Reverse(a[1]));
    rows(v.into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), avg(a[4], a[0]), avg(a[5], a[0])])))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, PostCount, CommentCount, RANK() OVER (ORDER BY (PostCount - DownVotes + UpVotes) DESC) AS UserRank FROM UserActivity),
// PostInfo AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, C.UserDisplayName AS LastCommenter, P.LastActivityDate, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY C.CreationDate DESC) AS CommentRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// PostStats AS (SELECT PostId, Title, CreationDate, ViewCount, LastCommenter, LastActivityDate FROM PostInfo WHERE CommentRank = 1)
// SELECT TU.DisplayName, TU.UpVotes, TU.DownVotes, TU.PostCount, TU.CommentCount, PS.Title, PS.CreationDate, PS.ViewCount, PS.LastCommenter, PS.LastActivityDate
// FROM TopUsers TU JOIN PostStats PS ON TU.UserId = PS.PostId WHERE TU.UserRank <= 10 ORDER BY TU.UserRank;
//
// `TU.UserId = PS.PostId` joins a user id to a post id, so it goes through the raw ids. CommentRank ties are broken by comment id.
fn q3880(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, p| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ua).and(&pc).and(&cc)).window(rank, |(((_, a), n), _)| Reverse(n - a[1] + a[0]), asc);
    type T = (Id<User>, [i64; 2], i64, i64, i64);
    let tu: MatSet<T> = (&w).filt(|(_, k)| k <= 10).map(|((((u, a), n), c), k)| (u, a, n, c, k)).collect();
    let Post { creation_date, origid, .. } = &db.post;
    let Comment { creation_date: cd, .. } = &db.comment;
    let rp = || db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30)));
    let lw = rp()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(Ident::<Comment>::new().and(cd)).opt())
        .window(row_number, |x: Option<(Id<Comment>, i64)>| (x.is_none(), Reverse(x.map(|(_, d)| d)), x.map(|(c, _)| c)), asc);
    let lc = (&lw).filt(|(_, k)| k == 1).map(|(x, _)| x.map(|(c, _): (Id<Comment>, i64)| c));
    let pid: HashIdx<i64, Id<Post>> = origid.inv().collect();
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _, _, _, _): T| u).select(&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&lc)))));
    v.sort_by_key(|&(_, ((_, _, _, _, k), _))| k);
    rows(v.into_iter().map(|(_, ((u, a, n, c, _), (p, x)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n), V::I(c)];
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend([ostr(x.and_then(|x| db.comment.user_display_name.get(x))), V::T(db.post.last_activity_date.get(p).unwrap())]);
        row(f)
    }))
}

// WITH RECURSIVE ActiveUsers AS (SELECT Id, DisplayName, Reputation, CreationDate, LastAccessDate, Rank() OVER (ORDER BY Reputation DESC) as UserRank FROM Users WHERE Reputation > 1000),
// PostsWithScores AS (SELECT P.Id AS PostId, P.Title, COALESCE(P.Score, 0) AS PostScore, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(V.BountyAmount) AS TotalBounty, P.OwnerUserId
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.Score, P.OwnerUserId),
// TopPosts AS (SELECT PWS.PostId, PWS.Title, PWS.PostScore, PWS.CommentCount, PWS.TotalBounty, AU.DisplayName AS OwnerName, AU.Reputation AS OwnerReputation,
//        DENSE_RANK() OVER (ORDER BY (PWS.PostScore + PWS.TotalBounty) DESC) AS Rank FROM PostsWithScores PWS JOIN ActiveUsers AU ON PWS.OwnerUserId = AU.Id),
// ClosedPosts AS (SELECT P.Id AS PostId, PH.CreationDate, PH.Comment AS CloseReason, P.Title, RANK() OVER (PARTITION BY P.Id ORDER BY PH.CreationDate DESC) AS RecentClose
//     FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId WHERE PH.PostHistoryTypeId = 10)
// SELECT TP.Title, TP.PostScore, TP.CommentCount, TP.TotalBounty, TP.OwnerName, TP.OwnerReputation, COALESCE(CP.CloseReason, 'Not Closed') AS LastCloseReason
// FROM TopPosts TP LEFT JOIN ClosedPosts CP ON TP.PostId = CP.PostId AND CP.RecentClose = 1 WHERE TP.Rank <= 10 ORDER BY TP.PostScore DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. A post with no bounty has a NULL rank key, which sorts after every other key.
fn q34366(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let pws = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let key = |s: i64, a: [i64; 3]| if a[1] == 0 { None } else { Some(s + a[2]) };
    let w = whole(&pws).select(Ident::<Post>::new().and(score).and(&pws)).window(dense_rank, |((_, s), a)| (key(s, a).is_none(), Reverse(key(s, a))), asc);
    let tp: MatSet<(Id<Post>, [i64; 3])> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), a), _)| (p, a)).collect();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cw = db.post_history.with(post_history_type_id.eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new().and(hd)).window(rank, |(_, d)| Reverse(d), asc);
    let last = (&cw).filt(|(_, k)| k == 1).map(|((h, _), _)| h);
    type T = (Id<Post>, [i64; 3]);
    let mut v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select(&last).opt())));
    v.sort_by_key(|&(_, ((p, _), _))| Reverse(score.get(p).unwrap()));
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1])]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("Not Closed")));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName),
// BadgeRanking AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, RANK() OVER (ORDER BY COUNT(b.Id) DESC) AS BadgeRank FROM Badges b GROUP BY b.UserId),
// TopUsers AS (SELECT u.Id, u.DisplayName, COALESCE(r.BadgeCount, 0) AS BadgeCount, r.BadgeRank FROM Users u LEFT JOIN BadgeRanking r ON u.Id = r.UserId)
// SELECT p.Title AS PostTitle, p.CreationDate AS Created_at, p.ViewCount, p.UpvoteCount, p.DownvoteCount, t.DisplayName AS TopUser, t.BadgeCount, t.BadgeRank,
//        CASE WHEN p.Score IS NULL THEN 'No Score' ELSE COALESCE(CAST(p.Score AS VARCHAR), 'Unknown') END AS Score, CASE WHEN p.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS Comments_Status
// FROM RecentPosts p LEFT JOIN TopUsers t ON p.OwnerName = t.DisplayName WHERE p.PostRank <= 10 ORDER BY p.ViewCount DESC;
//
// PostRank reads only CreationDate, so the ten newest posts are picked first (ties by post id) and the comment x vote product is driven for those alone.
fn q202(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let top = top_n(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bcnt = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let bw = whole(&bcnt).select(Ident::<User>::new().and(&bcnt)).window(rank, |(_, n)| Reverse(n), asc);
    let brm: MatSet<(Id<User>, (Id<User>, i64, i64))> = (&bw).map(|((u, n), k)| (u, (u, n, k))).collect();
    let bri = by_first(&brm);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let tu = owner_user.select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and((&bri).opt()));
    let mut v = drain((&rp).and(tu.opt()));
    v.sort_by_key(|&(p, _)| {
        let w = db.post.view_count.get(p);
        (w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        f.extend([V::I(a[1]), V::I(a[2])]);
        f.extend(match t {
            Some((u, b)) => [user_col(db, u, "name"), V::I(b.map_or(0, |b| b.1)), b.map_or(V::Null, |b| V::I(b.2))],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::Owned(score.get(p).unwrap().to_string()), V::S(if a[0] > 0 { "Has Comments" } else { "No Comments" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank,
//        COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostScoreAnalysis AS (SELECT rp.PostId, COALESCE(rp.Score, 0) AS AdjustedScore, COUNT(c.Id) AS CommentCount FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId = rp.PostId GROUP BY rp.PostId, rp.Score),
// AcceptedAnswerInfo AS (SELECT p.Id AS QuestionId, a.Id AS AcceptedAnswerId, a.Score AS AcceptedAnswerScore FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id WHERE p.PostTypeId = 1)
// SELECT rp.PostId, rp.Title, sa.AcceptedAnswerId, sa.AcceptedAnswerScore, psa.CommentCount,
//        CASE WHEN psa.AdjustedScore IS NULL THEN 'No Score' WHEN psa.AdjustedScore > 0 THEN 'Scored Post' ELSE 'Unscored or Negative' END AS ScoreCategory,
//        CASE WHEN rp.UserPostRank = 1 THEN 'Most Recent Post by User' ELSE 'Not Most Recent' END AS UserPostStatus
// FROM RankedPosts rp LEFT JOIN PostScoreAnalysis psa ON rp.PostId = psa.PostId LEFT JOIN AcceptedAnswerInfo sa ON rp.PostId = sa.QuestionId
// WHERE (rp.ViewCount > 100 OR psa.CommentCount > 5) AND (rp.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' OR sa.AcceptedAnswerId IS NOT NULL)
// ORDER BY rp.UserPostRank DESC, psa.AdjustedScore DESC NULLS LAST LIMIT 50;
//
// UserPostRank ties are broken by post id; the ownerless posts rank among themselves.
fn q21219(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, score, view_count, post_type_id, accepted_answer, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let recent = || db.post.with(creation_date.ge(add_years(t0, -1)));
    let w = recent().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), k)| (p, k)).collect();
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let aa = Ident::<Post>::new().with(post_type_id.eq(1)).select(accepted_answer.opt());
    type K = (Id<Post>, i64);
    let pk = || Same::<K>::new().map(|(p, _): K| p);
    let six = add_months(t0, -6);
    let keep = |((((_, _), c), a), (w, d)): (((K, i64), Option<Option<Id<Post>>>), (Option<i64>, i64))| (w.map_or(false, |w| w > 100) || c > 5) && (d < six || a.flatten().is_some());
    let v = drain((&rk).select(Same::<K>::new().and(pk().select(&cc)).and(pk().select(aa).opt()).and(pk().select(view_count.opt().and(creation_date)))).filt(keep));
    let v = top_n(v, |&(_, ((((p, k), _), _), _))| (Reverse(k), Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(_, ((((p, k), c), a), _))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(match a.flatten() {
            Some(a) => post_fields(db, a, &["id", "score"]),
            None => vec![V::Null, V::Null],
        });
        f.extend([V::I(c), V::S(if s > 0 { "Scored Post" } else { "Unscored or Negative" }), V::S(if k == 1 { "Most Recent Post by User" } else { "Not Most Recent" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(CASE WHEN b.Id IS NOT NULL THEN 1 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS HistoryCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, BadgeCount, RANK() OVER (ORDER BY PostCount DESC, UpVotes DESC) AS UserRank FROM UserStats)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes, tu.BadgeCount, pa.Title AS RecentPostTitle, pa.CommentCount, pa.HistoryCount
// FROM TopUsers tu LEFT JOIN PostActivity pa ON tu.UserId = pa.PostId WHERE tu.UserRank <= 10 ORDER BY tu.UserRank;
//
// UserRank leads with the distinct post count, so only users whose RANK() by that count is at most ten can rank in the top ten; the post x vote x badge product is driven
// for those alone. `tu.UserId = pa.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q6134(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let cw = whole(db.user.with(&pc)).select(Ident::<User>::new().and(&pc)).window(rank, |(_, n)| Reverse(n), asc);
    let cand: MatSet<Id<User>> = (&cw).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let w = whole(&cand).select(Ident::<User>::new().and(&us).and(&pc)).window(rank, |((_, a), n)| (Reverse(n), Reverse(a[2])), asc);
    let tu: MatSet<(Id<User>, [i64; 5], i64, i64)> = (&w).filt(|(_, k)| k <= 10).map(|(((u, a), n), k)| (u, a, n, k)).collect();
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pa = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let hc = rp().group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let pid: HashIdx<i64, Id<Post>> = origid.inv().collect();
    type T = (Id<User>, [i64; 5], i64, i64);
    let pk = Same::<T>::new().map(|(u, _, _, _): T| u).select(&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&pa).and(&hc));
    let mut v = drain((&tu).select(Same::<T>::new().and(pk.opt())));
    v.sort_by_key(|&(_, ((_, _, _, k), _))| k);
    rows(v.into_iter().map(|(_, ((u, a, n, _), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])];
        f.extend(match p {
            Some(((p, c), h)) => [ostr(db.post.title.get(p)), V::I(c), V::I(h)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// CombinedData AS (SELECT rp.PostId, rp.Title, rp.Score, rb.BadgeCount, rb.HighestBadgeClass, pc.CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVotes
//     FROM RankedPosts rp LEFT JOIN UserBadges rb ON rp.OwnerUserId = rb.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId
//     GROUP BY rp.PostId, rp.Title, rp.Score, rb.BadgeCount, rb.HighestBadgeClass, pc.CommentCount)
// SELECT pg.PostId, pg.Title, pg.Score, pg.BadgeCount, pg.HighestBadgeClass, pg.CommentCount, pg.NetVotes, PHT.Name AS PostHistoryTypeName
// FROM CombinedData pg LEFT JOIN PostHistory ph ON pg.PostId = ph.PostId LEFT JOIN PostHistoryTypes PHT ON ph.PostHistoryTypeId = PHT.Id
// WHERE pg.Score > 0 AND (pg.BadgeCount IS NULL OR pg.BadgeCount > 5) ORDER BY pg.Score DESC, pg.CommentCount DESC;
//
// PostRank is never read.
fn q1451(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, i64::MIN), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let cd = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let keep = |(_, b): (i64, Option<(i64, i64)>)| b.map_or(true, |b| b.0 > 5);
    let v = drain((&cd).and(owner_user.select(&ub).opt()).filt(keep).and((&pc).opt()).and(history_of(db).opt()));
    rows(v.into_iter().map(|(p, (((n, b), c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend(match b {
            Some((k, m)) => [V::I(k), omax(m, k)],
            None => [V::Null, V::Null],
        });
        f.extend([oint(c), V::I(n), h.map_or(V::Null, |h| V::S(htype_name(db).get(h).unwrap()))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) as PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c WHERE c.CreationDate >= cast('2024-10-01' as date) - INTERVAL '6 months'
//     GROUP BY c.PostId)
// SELECT p.Title, p.Score, p.ViewCount, p.CreationDate AS PostDate, us.DisplayName AS Author, us.Reputation, us.TotalUpvotes, us.TotalDownvotes, us.TotalBadges, rpc.CommentCount, rpc.LastCommentDate,
//        CASE WHEN p.Score > 10 THEN 'High Score' WHEN p.Score BETWEEN 5 AND 10 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM RankedPosts p JOIN UserStats us ON p.OwnerUserId = us.UserId LEFT JOIN RecentPostComments rpc ON p.Id = rpc.PostId WHERE p.PostRank = 1 ORDER BY p.CreationDate DESC LIMIT 100;
//
// PostRank and the LIMIT read only base columns, so the hundred newest latest-posts are picked first (ties by post id) and the post x vote x badge product of UserStats
// is driven for their owners alone.
fn q1734(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first = top_n(drain((&w).filt(|(_, n)| n == 1).map(|(x, _)| x)), |&(_, (p, d))| (Reverse(d), p), 100);
    let fp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.1 .0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&fp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let bc = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Comment { post, creation_date: cd, .. } = &db.comment;
    let rpc = db.comment.with(cd.ge(add_months(date(2024, 10, 1), -6))).group_by(post).select(cd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&fp).select(owner_user.select(Ident::<User>::new().and(&us).and(&bc)).and((&rpc).opt())));
    v.sort_by_key(|&(p, _)| (Reverse(creation_date.get(p).unwrap()), p));
    rows(v.into_iter().map(|(p, (((u, a), b), c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "score", "views", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b)]);
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if s > 10 { "High Score" } else if s >= 5 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RECURSIVE PopularPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS PopularityRank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// RecentUserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(COALESCE(c.Score, 0)) AS TotalCommentScore, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '60 days' LEFT JOIN Comments c ON c.UserId = u.Id
//     LEFT JOIN Votes v ON v.UserId = u.Id AND v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '60 days' GROUP BY u.Id, u.DisplayName),
// HighRankingPosts AS (SELECT pp.Id, pp.Title, pp.ViewCount, pp.Score, ua.UserId, ua.DisplayName, ROW_NUMBER() OVER (PARTITION BY ua.UserId ORDER BY pp.Score DESC) AS UserPostRank
//     FROM PopularPosts pp JOIN RecentUserActivity ua ON pp.Score > 10)
// SELECT h.Id AS HighRankingPostId, h.Title AS HighRankingPostTitle, h.ViewCount AS HighRankingPostViewCount, h.Score AS HighRankingPostScore, u.DisplayName AS UserDisplayName,
//        CASE WHEN ua.PostsCreated > 0 THEN 'Active User' ELSE 'Inactive User' END AS UserStatus
// FROM HighRankingPosts h JOIN RecentUserActivity ua ON h.UserId = ua.UserId JOIN Users u ON u.Id = h.UserId WHERE UserPostRank <= 5 ORDER BY h.Score DESC, u.Reputation DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. `JOIN RecentUserActivity ua ON pp.Score > 10` names only pp, so the popular posts are crossed with every user; UserPostRank
// ties are broken by post id. Only PostsCreated is read from RecentUserActivity, and a COUNT(DISTINCT p.Id) is not changed by the comment and vote joins. PopularityRank is never read.
fn q32195(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let pp: HashIdx<(), (Id<Post>, i64)> = db.post.with(creation_date.ge(add_days(t0, -30))).with(score.gt(10)).map(|_| ()).inv().select(Ident::<Post>::new().and(score)).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(t0, -60)))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = db.user.group_by(Ident::<User>::new()).select(Ident::<User>::new().map(|_| ()).select(&pp)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let mut v = drain((&w).filt(|(_, k)| k <= 5).map(|(x, _)| x).and(&pc));
    v.sort_by_key(|&(u, ((_, s), _))| (Reverse(s), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(u, ((p, _), n))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([user_col(db, u, "name"), V::S(if n > 0 { "Active User" } else { "Inactive User" })]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("27162", q27162),
    ("1892", q1892),
    ("2043", q2043),
    ("3049", q3049),
    ("2270", q2270),
    ("26323", q26323),
    ("3910", q3910),
    ("835", q835),
    ("3224", q3224),
    ("3923", q3923),
    ("8360", q8360),
    ("2102", q2102),
    ("3655", q3655),
    ("4559", q4559),
    ("34951", q34951),
    ("8572", q8572),
    ("1507", q1507),
    ("1900", q1900),
    ("24533", q24533),
    ("2358", q2358),
    ("4815", q4815),
    ("30674", q30674),
    ("3980", q3980),
    ("2500", q2500),
    ("1035", q1035),
    ("24020", q24020),
    ("3319", q3319),
    ("4757", q4757),
    ("4661", q4661),
    ("528", q528),
    ("23128", q23128),
    ("2070", q2070),
    ("8658", q8658),
    ("8730", q8730),
    ("868", q868),
    ("953", q953),
    ("21922", q21922),
    ("27843", q27843),
    ("1962", q1962),
    ("4744", q4744),
    ("30260", q30260),
    ("2623", q2623),
    ("23874", q23874),
    ("34915", q34915),
    ("32970", q32970),
    ("31283", q31283),
    ("3222", q3222),
    ("3913", q3913),
    ("3135", q3135),
    ("21028", q21028),
    ("3562", q3562),
    ("2765", q2765),
    ("20838", q20838),
    ("4927", q4927),
    ("22949", q22949),
    ("23994", q23994),
    ("2479", q2479),
    ("5444", q5444),
    ("9363", q9363),
    ("1500", q1500),
    ("4635", q4635),
    ("4988", q4988),
    ("9465", q9465),
    ("2540", q2540),
    ("4922", q4922),
    ("4420", q4420),
    ("21608", q21608),
    ("3205", q3205),
    ("24277", q24277),
    ("31144", q31144),
    ("34967", q34967),
    ("199", q199),
    ("24431", q24431),
    ("3592", q3592),
    ("271", q271),
    ("854", q854),
    ("32813", q32813),
    ("4013", q4013),
    ("7698", q7698),
    ("22076", q22076),
    ("529", q529),
    ("9161", q9161),
    ("3880", q3880),
    ("34366", q34366),
    ("202", q202),
    ("21219", q21219),
    ("6134", q6134),
    ("1451", q1451),
    ("1734", q1734),
    ("32195", q32195),
];
