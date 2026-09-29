use harness::prelude::*;
use std::cmp::Reverse;

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn snip(s: Str, n: usize) -> Str {
    &s[..s.char_indices().nth(n).map_or(s.len(), |x| x.0)]
}

fn agg_str<I: IntoIterator<Item = Str>>(it: I, sep: &str) -> Option<Str> {
    let v: Vec<Str> = it.into_iter().collect();
    if v.is_empty() { None } else { Some(leak(v.join(sep))) }
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.Score, p.ViewCount, p.AcceptedAnswerId, COALESCE(cnt.CommentCount, 0) AS CommentCount,
//        COALESCE(ans.AnswerCount, 0) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) cnt ON p.Id = cnt.PostId
//     LEFT JOIN (SELECT ParentId AS PostId, COUNT(Id) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) ans ON p.Id = ans.PostId WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, SUBSTRING(rp.Body, 1, 200) AS BodySnippet, rp.Tags, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.AnswerCount, pht.Name AS PostHistoryType
//     FROM RankedPosts rp LEFT JOIN PostHistory ph ON ph.PostId = rp.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.PostId IS NOT NULL)
// SELECT fp.Title, fp.BodySnippet, fp.ViewCount, fp.CommentCount, fp.AnswerCount, STRING_AGG(DISTINCT fp.Tags, ', ') AS AllTags, COUNT(fp.PostId) AS PostHistoryChangeCount,
//        MAX(fp.CreationDate) AS LastUpdatedDate
// FROM FilteredPosts fp GROUP BY fp.Title, fp.BodySnippet, fp.ViewCount, fp.CommentCount, fp.AnswerCount ORDER BY PostHistoryChangeCount DESC, LastUpdatedDate DESC;
//
// The DISTINCT tags inside one group are listed sorted.
fn q26632(db: &'static So) -> String {
    let Post { post_type_id, title, body, view_count, tags_str, creation_date, .. } = &db.post;
    let q = || db.post.with(post_type_id.eq(1));
    let hc = q().group_by(Ident::<Post>::new()).select(history_of(db)).fold(0i64, |n, _| n + 1);
    let cc = q().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = q().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let r = rel(drain((&hc).and(&cc).and(&ac)));
    type R = (Id<Post>, ((i64, i64), i64));
    type K = (Option<Str>, Str, Option<i64>, i64, i64);
    let g = (&r)
        .group_by(Same::<R>::new().map(move |(p, ((_, c), a)): R| -> K { (title.get(p), snip(body.get(p).unwrap(), 200), view_count.get(p), c, a) }))
        .select(Same::<R>::new())
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().flat_map(|x| tags_str.get(x.0)).collect();
            t.sort_unstable();
            t.dedup();
            (v.iter().map(|x| x.1 .0 .0).sum::<i64>(), v.iter().map(|x| creation_date.get(x.0).unwrap()).max().unwrap(), agg_str(t, ", "))
        });
    let v = drain(&g);
    rows(v.into_iter().map(|((t, s, w, c, a), (n, d, tg))| row(vec![ostr(t), V::S(s), oint(w), V::I(c), V::I(a), ostr(tg), V::I(n), V::T(d)])))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(COALESCE(NULLIF(p.ViewCount, 0), 0)) AS TotalViews,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY COUNT(DISTINCT p.Id) DESC) AS rn
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, PositivePosts, NegativePosts, TotalViews FROM UserStats WHERE rn = 1 ORDER BY Reputation DESC LIMIT 10),
// PostLinksAggregate AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS RelatedPostCount, STRING_AGG(DISTINCT pt.Name, ', ' ORDER BY pt.Name) AS LinkTypeNames
//     FROM PostLinks pl JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id JOIN Posts p ON pl.PostId = p.Id JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pl.PostId)
// SELECT u.DisplayName, u.Reputation, u.PostCount, u.PositivePosts, u.NegativePosts, u.TotalViews, p.Id AS PostId, p.Title, p.ViewCount,
//        COALESCE(link.RelatedPostCount, 0) AS RelatedPostCount, COALESCE(link.LinkTypeNames, 'None') AS LinkTypeNames
// FROM TopUsers u JOIN Posts p ON u.UserId = p.OwnerUserId LEFT JOIN PostLinksAggregate link ON p.Id = link.PostId
// WHERE p.CreationDate = (SELECT MAX(CreationDate) FROM Posts WHERE OwnerUserId = u.UserId) ORDER BY u.Reputation DESC, p.ViewCount DESC;
//
// rn partitions by the user, so it is always 1. The ten users are picked by Reputation first.
fn q22770(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let top = top_n(drain(db.user.select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let st = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(creation_date)).opt())
        .fold([0i64, 0, 0, 0, i64::MIN], |a, p| match p {
            Some(((s, w), d)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + w.unwrap_or(0), a[4].max(d)],
            None => a,
        });
    let links = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).select((&st).and(posts_of(db).select(Ident::<Post>::new().and(creation_date).and((&links).opt())))).filt(|(a, ((_, d), _))| d == a[4]));
    let mut v = v;
    v.sort_by_key(|&(u, (_, ((p, _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), view_count.get(p).is_none(), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(u, (a, ((p, _), l)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["id", "title", "views"]));
        f.push(V::I(l.unwrap_or(0)));
        f.push(if l.is_some() { post_fields(db, p, &["type"]).remove(0) } else { V::S("None") });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT up.DisplayName, up.Reputation, rp.Title AS RecentPostTitle, rp.CreationDate AS PostCreationDate, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        CASE WHEN rp.CommentCount > 0 THEN 'Has Comments' WHEN rp.OwnerRank = 1 THEN 'Latest Post' ELSE 'No Comments' END AS PostStatus, COALESCE(vs.VoteType, 'No Votes') AS VoteStatus
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id LEFT JOIN UserBadges ub ON up.Id = ub.UserId
// LEFT JOIN (SELECT PostId, STRING_AGG(CASE WHEN VoteTypeId = 2 THEN 'Upvote' WHEN VoteTypeId = 3 THEN 'Downvote' ELSE 'Other Vote' END, ', ') AS VoteType FROM Votes GROUP BY PostId) vs ON rp.Id = vs.PostId
// WHERE ub.BadgeCount > 0 ORDER BY up.Reputation DESC, rp.CreationDate DESC FETCH FIRST 10 ROWS ONLY;
//
// The votes of a post are listed in vote id order (the SQL leaves it open).
fn q4028(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let r = per_group(ranked(drain(recent().select(owner_user_id.opt())), |&(p, o)| (o, Reverse(creation_date.get(p).unwrap())), false), |x| x.1);
    let rr = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    type R = (Id<Post>, i64);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).buf_fold(|v| {
        leak(v.iter().map(|&t| if t == 2 { "Upvote" } else if t == 3 { "Downvote" } else { "Other Vote" }).collect::<Vec<_>>().join(", "))
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&rr).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&cc).and(owner_user.select(Ident::<User>::new().and(&ub))).and((&vs).opt())))));
    let v = top_n(v, |&(_, ((p, _), ((_, (u, _)), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(_, ((p, k), ((c, (u, b)), s)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend(b.map(V::I));
        f.push(V::S(if c > 0 { "Has Comments" } else if k == 1 { "Latest Post" } else { "No Comments" }));
        f.push(V::S(s.unwrap_or("No Votes")));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, STRING_AGG(Name, ', ') AS BadgeNames FROM Badges GROUP BY UserId),
// UserPosts AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, AVG(COALESCE(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) / 3600, 0)) AS AvgHourToActivity
//     FROM Posts p GROUP BY p.OwnerUserId),
// RankedUsers AS (SELECT u.Id, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(up.QuestionCount, 0) AS QuestionCount, COALESCE(up.AnswerCount, 0) AS AnswerCount,
//        COALESCE(up.TotalScore, 0) AS TotalScore, COALESCE(up.TotalViews, 0) AS TotalViews, COALESCE(up.AvgHourToActivity, 0) AS AvgHourToActivity,
//        RANK() OVER(ORDER BY COALESCE(up.TotalScore, 0) DESC) AS ScoreRank
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN UserPosts up ON u.Id = up.OwnerUserId)
// SELECT ru.DisplayName, ru.BadgeCount, ru.QuestionCount, ru.AnswerCount, ru.TotalScore, ru.TotalViews, ru.AvgHourToActivity,
//        CASE WHEN ru.BadgeCount >= 10 THEN 'Expert' WHEN ru.BadgeCount >= 5 THEN 'Intermediate' ELSE 'Beginner' END AS UserLevel,
//        CASE WHEN ru.AvgHourToActivity < 1 THEN 'Very Active' WHEN ru.AvgHourToActivity < 24 THEN 'Active' ELSE 'Less Active' END AS ActivityLevel
// FROM RankedUsers ru WHERE ru.ScoreRank <= 10 ORDER BY ru.TotalScore DESC;
fn q3240(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, last_activity_date, creation_date, .. } = &db.post;
    let up = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(last_activity_date).and(creation_date)).opt())
        .fold((0i64, 0i64, 0i64, 0i64, 0.0f64, 0i64), |a, p| match p {
            Some(((((t, s), w), l), d)) => (a.0 + (t == 1) as i64, a.1 + (t == 2) as i64, a.2 + s, a.3 + w.unwrap_or(0), a.4 + secs(l - d) / 3600.0, a.5 + 1),
            None => a,
        });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&up).and(&bc)), |&(_, (a, _))| Reverse(a.2), false);
    let v: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).collect();
    rows(v.into_iter().map(|((u, (a, b)), _)| {
        let h = if a.5 == 0 { 0.0 } else { a.4 / a.5 as f64 };
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(b), V::I(a.0), V::I(a.1), V::I(a.2), V::I(a.3), V::F(h)]);
        f.push(V::S(if b >= 10 { "Expert" } else if b >= 5 { "Intermediate" } else { "Beginner" }));
        f.push(V::S(if h < 1.0 { "Very Active" } else if h < 24.0 { "Active" } else { "Less Active" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS UserRanking,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS MaxBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryTags AS (SELECT ph.PostId, STRING_AGG(pt.Name, ', ') AS PostHistoryTypes FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id
//     WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY ph.PostId)
// SELECT u.DisplayName AS Author, up.PostId, up.Title, up.ViewCount, up.UserRanking, ub.BadgeCount, ub.MaxBadgeClass, pht.PostHistoryTypes, (up.UpVotes - up.DownVotes) AS NetVotes,
//        CASE WHEN up.UserRanking < 5 THEN 'New Contributor' WHEN ub.BadgeCount > 10 AND ub.MaxBadgeClass = 1 THEN 'Influencer' ELSE 'Regular User' END AS UserType
// FROM RankedPosts up JOIN Users u ON up.PostId = u.Id JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostHistoryTags pht ON up.PostId = pht.PostId
// WHERE pht.PostHistoryTypes IS NOT NULL ORDER BY NetVotes DESC, up.CreationDate ASC LIMIT 50;
//
// `up.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. UserRanking numbers the joined vote rows; ties in ViewCount are broken by
// post and vote id (the SQL leaves it open). History types are listed in history id order.
fn q33785(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let j = drain(recent().select(owner_user_id.opt().and(votes_of(db).opt())));
    let r = per_group(ranked(j, |&(p, (o, v))| (o, view_count.get(p).is_none(), Reverse(view_count.get(p)), p, v), false), |x| x.1 .0);
    let rr = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    type R = (Id<Post>, i64);
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |a, c| (a.0 + c.is_some() as i64, a.1.max(c.unwrap_or(0))));
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let pht = db.post_history.with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).group_by(&db.post_history.post).select(htype_name(db)).buf_fold(|v| leak(v.join(", ")));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&rr).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&vc).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ub))).and(&pht)))));
    let v = top_n(v, |&(i, ((p, _), ((n, _), _)))| (Reverse(n), creation_date.get(p).unwrap(), i), 50);
    rows(v.into_iter().map(|(_, ((p, k), ((n, (u, (bc, mc))), s)))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend(post_fields(db, p, &["id", "title", "views"]));
        f.extend([V::I(k), V::I(bc), if bc == 0 { V::Null } else { V::I(mc) }, V::S(s), V::I(n)]);
        f.push(V::S(if k < 5 { "New Contributor" } else if bc > 10 && mc == 1 { "Influencer" } else { "Regular User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(NULLIF(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0), 0) AS Upvotes,
//        COALESCE(NULLIF(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0), 0) AS Downvotes, ROW_NUMBER() OVER (PARTITION BY UPPER(SUBSTRING(p.Title, 1, 1)) ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// AggregatedPostData AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Upvotes, rp.Downvotes,
//        CASE WHEN rp.Upvotes > rp.Downvotes THEN 'Popular' WHEN rp.Upvotes < rp.Downvotes THEN 'Unpopular' ELSE 'Neutral' END AS Popularity FROM RankedPosts rp WHERE rp.Rank <= 5),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b JOIN Users u ON b.UserId = u.Id WHERE u.Reputation > 1000 GROUP BY b.UserId),
// FinalResults AS (SELECT ap.Title, ap.CreationDate, ap.ViewCount, ap.Upvotes, ap.Downvotes, ap.Popularity, ub.BadgeCount, ub.BadgeNames
//     FROM AggregatedPostData ap LEFT JOIN UserBadges ub ON ap.PostId = (SELECT MIN(p.Id) FROM Posts p WHERE p.OwnerUserId = ub.UserId))
// SELECT *, CASE WHEN BadgeCount IS NULL THEN 'No Badges' ELSE 'Has Badges' END AS UserBadgeStatus,
//        CONCAT('Post Title: "', Title, '" has ', Upvotes, ' upvotes and ', Downvotes, ' downvotes.') AS PostSummary
// FROM FinalResults ORDER BY ViewCount DESC LIMIT 10;
//
// Rank reads only base columns, so the questions are ranked first and the votes are counted for the survivors. Badge names are in badge id order.
fn q20128(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, view_count, origid, .. } = &db.post;
    let first = |p: Id<Post>| title.get(p).map(|t| t.chars().next().map_or(String::new(), |c| c.to_uppercase().collect::<String>()));
    let r = per_group(ranked(drain(db.post.with(post_type_id.eq(1))), |&(p, _)| (first(p), Reverse(creation_date.get(p).unwrap()), p), false), |x| first(x.0));
    let top: MatSet<Id<Post>> = rel(r.into_iter().map(|x| (x.0 .0, x.1)).collect()).filt(|x: (Id<Post>, i64)| x.1 <= 5).map(|x: (Id<Post>, i64)| x.0).collect();
    let vc = (&top).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let hi = || db.user.with((&db.user.reputation).gt(1000));
    let ub = hi().group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name)).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let mp = hi().group_by(Ident::<User>::new()).select(posts_of(db).select(origid)).fold(i64::MAX, |m, x| m.min(x));
    let bym: HashIdx<i64, Id<User>> = (&mp).inv().collect();
    let v = drain((&vc).and(origid.select(&bym).select(&ub).opt()));
    let v = top_n(v, |&(p, _)| (view_count.get(p).is_none(), Reverse(view_count.get(p))), 10);
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if a[0] > a[1] { "Popular" } else if a[0] < a[1] { "Unpopular" } else { "Neutral" }));
        match b {
            Some((n, s)) => f.extend([V::I(n), V::S(s), V::S("Has Badges")]),
            None => f.extend([V::Null, V::Null, V::S("No Badges")]),
        }
        f.push(V::Owned(format!("Post Title: \"{}\" has {} upvotes and {} downvotes.", title.get(p).unwrap_or(""), a[0], a[1])));
        row(f)
    }))
}

// WITH RECURSIVE UserReputationCTE AS (SELECT U.Id, U.DisplayName, U.Reputation, CAST(0 AS int) AS Level FROM Users U WHERE U.Reputation IS NOT NULL
//     UNION ALL SELECT U.Id, U.DisplayName, U.Reputation, C.Level + 1 FROM Users U JOIN UserReputationCTE C ON U.Reputation >= C.Reputation + 1000),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, Dense_Rank() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 1000),
// PostActivity AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        AVG(CASE WHEN P.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN P.Score END) AS AvgScoreLast30Days FROM Posts P GROUP BY P.OwnerUserId),
// UserBadges AS (SELECT B.UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT U.DisplayName, U.Reputation, COALESCE(PA.PostCount, 0) AS TotalPosts, COALESCE(PA.TotalScore, 0) AS TotalScore, COALESCE(PA.AvgScoreLast30Days, 0) AS AvgScoreLast30Days,
//        COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS OverallRank
// FROM Users U LEFT JOIN PostActivity PA ON U.Id = PA.OwnerUserId LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// WHERE U.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' ORDER BY U.Reputation DESC LIMIT 100;
//
// Neither recursive CTE nor TopUsers is read. The hundred users are picked by Reputation first.
fn q30308(db: &'static So) -> String {
    let Post { score, creation_date, .. } = &db.post;
    let top = top_n(drain(db.user.with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 100);
    let tu = rel(top.into_iter().map(|x| x.0).collect());
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let us: MatSet<Id<User>> = (&tu).map(|u| u).collect();
    let pa = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(creation_date)).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, d)) => [a[0] + 1, a[1] + s, a[2] + (d < cut) as i64 * s, a[3] + (d < cut) as i64],
        None => a,
    });
    let ub = (&us).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&tu).select(Ident::<User>::new().and(&pa).and(&ub)));
    rows(v.into_iter().map(|(i, ((u, a), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::F(if a[3] == 0 { 0.0 } else { a[2] as f64 / a[3] as f64 })]);
        f.extend(b.map(V::I));
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// CloseAndEditHistory AS (SELECT ph.PostId, STRING_AGG(CASE WHEN pt.Name = 'Post Closed' THEN ph.Comment ELSE NULL END, '; ') AS CloseReasons,
//        STRING_AGG(CASE WHEN pt.Name LIKE 'Edit %' THEN ph.Text ELSE NULL END, '; ') AS EditHistory, MAX(ph.CreationDate) AS LastActivityDate
//     FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id GROUP BY ph.PostId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvotesGiven,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvotesGiven FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, uch.UserId, uch.DisplayName, uch.Reputation, uch.UpvotesGiven, uch.DownvotesGiven, cah.CloseReasons, cah.EditHistory, cah.LastActivityDate
// FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId = rp.PostId LEFT JOIN UserReputation uch ON c.UserId = uch.UserId LEFT JOIN CloseAndEditHistory cah ON rp.PostId = cah.PostId
// WHERE (rp.Rank <= 5 OR rp.Score > 100) AND (uch.Reputation <= 1000 OR uch.UpvotesGiven > uch.DownvotesGiven)
// ORDER BY rp.Score DESC, rp.ViewCount DESC, rp.CreationDate DESC;
//
// The WHERE on uch drops every row without a commenting user, so those joins are inner. The history strings are in history id order.
fn q21718(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let r = per_group(
        ranked(drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), false),
        |x| x.1,
    );
    let rr = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    type R = (Id<Post>, i64);
    let keep = || (&rr).filt(|(p, k): R| k <= 5 || score.get(p).unwrap() > 100);
    let ps: MatSet<Id<Post>> = keep().map(|x: R| x.0).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { comment, text, creation_date: hd, .. } = &db.post_history;
    let cah = (&ps).group_by(Ident::<Post>::new()).select(history_of(db)).buf_fold(|v| {
        let nm = |h: Id<PostHistory>| db.post_history_type.name.get(db.post_history.post_history_type.get(h).unwrap()).unwrap();
        (
            agg_str(v.iter().filter(|&&h| nm(h) == "Post Closed").flat_map(|&h| comment.get(h)), "; "),
            agg_str(v.iter().filter(|&&h| nm(h).starts_with("Edit ")).flat_map(|&h| text.get(h)), "; "),
            v.iter().map(|&h| hd.get(h).unwrap()).max().unwrap(),
        )
    });
    let v = drain(
        keep()
            .select(Same::<R>::new().map(|x: R| x.0).select(comments_of(db).select(&db.comment.user).select(Ident::<User>::new().and(&ur)).and((&cah).opt())))
            .filt(|((u, a), _): ((Id<User>, [i64; 2]), _)| db.user.reputation.get(u).unwrap() <= 1000 || a[0] > a[1]),
    );
    let mut v = v;
    v.sort_by_key(|&(i, _)| {
        let p = rr.v[i].0;
        (Reverse(score.get(p).unwrap()), view_count.get(p).is_none(), Reverse(view_count.get(p)), Reverse(creation_date.get(p).unwrap()))
    });
    rows(v.into_iter().map(|(i, ((u, a), c))| {
        let mut f = post_fields(db, rr.v[i].0, &["id", "title", "score", "views"]);
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        match c {
            Some((x, y, d)) => f.extend([ostr(x), ostr(y), V::T(d)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.ViewCount, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.PostTypeId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// AggregatedTags AS (SELECT unnest(string_to_array(substring(Tags, 2, length(Tags)-2), '><')) AS Tag FROM RankedPosts),
// TagCounts AS (SELECT Tag, COUNT(*) AS TagFrequency FROM AggregatedTags GROUP BY Tag),
// TopTags AS (SELECT Tag, TagFrequency, ROW_NUMBER() OVER (ORDER BY TagFrequency DESC) AS TagRank FROM TagCounts WHERE TagFrequency > 5),
// PostEngagement AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.Tags, rp.ViewCount, rp.CreationDate, rp.OwnerDisplayName, te.Tag AS MostFrequentTag, pe.CommentCount, pe.UpVotes, pe.DownVotes
// FROM RankedPosts rp JOIN PostEngagement pe ON rp.PostId = pe.PostId JOIN TopTags te ON te.Tag = ANY(string_to_array(substring(rp.Tags, 2, length(rp.Tags)-2), '><'))
// WHERE rp.Rank <= 5 ORDER BY rp.ViewCount DESC;
//
// Rank ties in ViewCount are broken by post id (the SQL leaves it open).
fn q25050(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, tags_str, .. } = &db.post;
    let rp = || db.post.with(owner_user).with(creation_date.ge(add_years(current_date(), -1)));
    let tc = rp().group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tt = (&tc).filt(|n| n > 5);
    let r = per_group(ranked(drain(rp().select(owner_user)), |&(p, u)| (u, view_count.get(p).is_none(), Reverse(view_count.get(p)), p), false), |x| x.1);
    let rr = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    type R = (Id<Post>, i64);
    let pe = db
        .post
        .with(creation_date.ge(add_months(current_date(), -6)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&rr).filt(|x: R| x.1 <= 5).select(Same::<R>::new().map(|x: R| x.0).select(Ident::<Post>::new().and(&pe).and(tags_str.flat_map(tag_list).select(Same::<Str>::new().with(&tt))))));
    let mut v = v;
    v.sort_by_key(|&(_, ((p, _), _))| (view_count.get(p).is_none(), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(_, ((p, a), t))| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "views", "created", "owner"]);
        f.push(V::S(t));
        f.extend(a.map(V::I));
        row(f)
    }))
}

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

// WITH RECURSIVE UserHierarchy AS (SELECT Id, DisplayName, Reputation, CreationDate, LastAccessDate, 0 AS Level FROM Users WHERE Reputation > 1000
//     UNION ALL SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate, uh.Level + 1 FROM Users u INNER JOIN UserHierarchy uh ON u.Id = uh.Id WHERE u.Reputation < uh.Reputation),
// PostSummary AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(p.AnswerCount, 0) AS AnswerCount, COALESCE(p.ViewCount, 0) AS ViewCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1),
// VoteDetails AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// CommentDetails AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT u.DisplayName, SUM(ps.ViewCount) AS TotalViews, COUNT(DISTINCT ps.PostId) AS TotalPosts, COALESCE(SUM(v.UpVotes), 0) - COALESCE(SUM(v.DownVotes), 0) AS NetVotes,
//        COALESCE(SUM(cd.CommentCount), 0) AS TotalComments, u.Reputation, CASE WHEN u.Reputation > 10000 THEN 'Elite' WHEN u.Reputation > 5000 THEN 'Expert' ELSE 'Novice' END AS UserLevel
// FROM UserHierarchy u LEFT JOIN PostSummary ps ON ps.OwnerUserId = u.Id LEFT JOIN VoteDetails v ON v.PostId = ps.PostId LEFT JOIN CommentDetails cd ON cd.PostId = ps.PostId
// WHERE u.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(DISTINCT ps.PostId) > 5 ORDER BY TotalViews DESC;
//
// The recursive step joins a user to itself and asks for a lower reputation than its own, so it never adds a row: only the base case exists.
// Each post meets at most one VoteDetails and one CommentDetails row, so the joined rows are the questions.
fn q34314(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let q = || db.post.with(post_type_id.eq(1));
    let vd = q().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cd = q().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let User { reputation, last_access_date, .. } = &db.user;
    let us = db
        .user
        .with(reputation.gt(1000))
        .with(last_access_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(view_count.opt().and((&vd).opt()).and((&cd).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((w, v), c)) => {
                let v = v.unwrap_or([0, 0]);
                [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + v[0], a[3] + v[1], a[4] + c.unwrap_or(0)]
            }
            None => a,
        });
    let mut v = drain((&us).filt(|a| a[0] > 5));
    v.sort_by_key(|&(_, a)| Reverse(a[1]));
    rows(v.into_iter().map(|(u, a)| {
        let r = reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(a[1]), V::I(a[0]), V::I(a[2] - a[3]), V::I(a[4]), V::I(r)]);
        f.push(V::S(if r > 10000 { "Elite" } else if r > 5000 { "Expert" } else { "Novice" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, COALESCE(u.DisplayName, 'Anonymous') AS Author,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS rn, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS Downvotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year')),
// RecentComments AS (SELECT c.PostId, COUNT(*) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c
//     WHERE c.CreationDate >= (CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 month') GROUP BY c.PostId),
// PostHistoryFiltered AS (SELECT ph.PostId, STRING_AGG(ph.Comment, ', ') AS HistoryComments FROM PostHistory ph
//     WHERE ph.CreationDate >= (CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '6 months') GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.Author, rp.Upvotes, rp.Downvotes, COALESCE(rc.CommentCount, 0) AS RecentCommentCount,
//        rc.LastCommentDate, COALESCE(phf.HistoryComments, 'No history') AS PostHistory
// FROM RankedPosts rp LEFT JOIN RecentComments rc ON rp.PostId = rc.PostId LEFT JOIN PostHistoryFiltered phf ON rp.PostId = phf.PostId
// WHERE rp.rn = 1 AND rp.Score > 0 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 100;
//
// rn numbers the joined vote rows; a tie in Score goes to the smaller post id. History comments are in history id order.
fn q3722(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let j = drain(recent().select(post_type_id.and(votes_of(db).opt())));
    let r = per_group(ranked(j, |&(p, (t, v))| (t, Reverse(score.get(p).unwrap()), p, v), false), |x| x.1 .0);
    let rr = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    type R = (Id<Post>, i64);
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let Comment { creation_date: cdt, .. } = &db.comment;
    let rc = db.comment.with(cdt.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.comment.post).select(cdt).fold((0i64, i64::MIN), |a, d| (a.0 + 1, a.1.max(d)));
    let PostHistory { creation_date: hd, comment, .. } = &db.post_history;
    let phf = db.post_history.with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).group_by(&db.post_history.post).select(comment.opt()).buf_fold(|v| agg_str(v.iter().flatten().copied(), ", "));
    let v = drain(
        (&rr)
            .filt(|(p, k): R| k == 1 && score.get(p).unwrap() > 0)
            .select(Same::<R>::new().map(|x: R| x.0).select(Ident::<Post>::new().and(&vc).and((&rc).opt()).and((&phf).opt()))),
    );
    let v = top_n(v, |&(_, (((p, _), _), _))| (Reverse(score.get(p).unwrap()), view_count.get(p).is_none(), Reverse(view_count.get(p))), 100);
    rows(v.into_iter().map(|(_, (((p, a), c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::S(owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.map_or(0, |c| c.0)), ots(c.map(|c| c.1))]);
        f.push(V::S(h.flatten().unwrap_or("No history")));
        row(f)
    }))
}

// WITH ParsedTags AS (SELECT p.Id AS PostId, TRIM(UNNEST(string_to_array(SUBSTRING(p.Tags, 2, LENGTH(p.Tags) - 2), '><'))) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagCounts AS (SELECT Tag, COUNT(*) AS TagFrequency FROM ParsedTags GROUP BY Tag),
// UserEngagement AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS QuestionsAsked, COUNT(DISTINCT c.Id) AS CommentsMade, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// TopUsers AS (SELECT ue.UserId, ue.QuestionsAsked, ue.CommentsMade, ue.UpVotesReceived, ue.DownVotesReceived, u.Reputation FROM UserEngagement ue JOIN Users u ON ue.UserId = u.Id
//     WHERE u.Reputation > 1000 ORDER BY ue.UpVotesReceived DESC LIMIT 10),
// TagPopularity AS (SELECT tc.Tag, tc.TagFrequency, ROW_NUMBER() OVER (ORDER BY tc.TagFrequency DESC) AS Rank FROM TagCounts tc WHERE tc.TagFrequency > 1)
// SELECT tu.UserId, tu.QuestionsAsked, tu.CommentsMade, tu.UpVotesReceived, tu.DownVotesReceived, tu.Reputation, tp.Tag, tp.TagFrequency
// FROM TopUsers tu JOIN TagPopularity tp ON tu.UserId IN (SELECT DISTINCT p.OwnerUserId FROM Posts p WHERE p.Tags LIKE '%' || tp.Tag || '%')
// ORDER BY tu.UpVotesReceived DESC, tp.TagFrequency DESC;
//
// The LIKE is tested against the distinct tag elements, as in tag_mentions: a TagPopularity tag has no '<' or '>', so it can only match inside one element.
fn q26870(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let pairs: MatSet<(Id<Post>, Str)> = db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(tags_str.flat_map(|t: Str| tag_list(t).map(|x| x.trim())))).collect();
    let tc = (&pairs).group_by(Same::<(Id<Post>, Str)>::new().map(|(_, t): (Id<Post>, Str)| t)).select(Same::<(Id<Post>, Str)>::new()).fold(0i64, |n, _| n + 1);
    let tp = (&tc).filt(|n| n > 1);
    let hi = || db.user.with((&db.user.reputation).gt(1000));
    let ue = hi()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.and_then(|x| x.1);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let top = top_n(drain(&ue), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu = rel(top);
    type T = (Id<User>, [i64; 2]);
    let us: MatSet<Id<User>> = (&tu).map(|x: T| x.0).collect();
    let pc = (&us).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cm = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let elems: MatSet<Str> = (&db.post.tags_str).flat_map(tag_list).collect();
    let tps: MatSet<Str> = (&pairs).map(|(_, t): (Id<Post>, Str)| t).with(&tp).collect();
    let like: HashIdx<Str, Str> = (&elems).select_where(&tps, |e: Str, t: Str| e.contains(t)).collect();
    let ut: MatSet<(Id<User>, Str)> = (&us).select(Ident::<User>::new().and(posts_of(db).select(tags_str.flat_map(tag_list).select(&like)))).collect();
    let v = drain((&ut).select(Same::<(Id<User>, Str)>::new().and(Same::<(Id<User>, Str)>::new().map(|x: (Id<User>, Str)| x.0).select((&ue).and(&pc).and(&cm))).and(Same::<(Id<User>, Str)>::new().map(|x: (Id<User>, Str)| x.1).select(&tc))));
    let mut v = v;
    v.sort_by_key(|&(_, (((_, ((a, _), _)), n)))| (Reverse(a[0]), Reverse(n)));
    rows(v.into_iter().map(|(_, (((u, t), ((a, p), c)), n))| {
        let mut f = ucols(db, u, &["uid"]);
        f.extend([V::I(p), V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(ucols(db, u, &["rep"]));
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// RecentUserStats AS (SELECT u.Id AS UserId, u.Reputation, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties, COUNT(DISTINCT p.Id) AS PostsCount, SUM(COALESCE(c.Score, 0)) AS TotalCommentScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE u.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseReasonCount, ARRAY_AGG(DISTINCT crt.Name) AS CloseReasons FROM PostHistory ph
//     INNER JOIN CloseReasonTypes crt ON CAST(ph.Comment AS INTEGER) = crt.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT up.Id AS UserId, up.DisplayName, up.Reputation, rps.PostId, rps.Title AS RecentPostTitle, rps.CreationDate AS RecentPostDate, COALESCE(dd.CloseReasonCount, 0) AS ClosedPostCount,
//        COALESCE(dd.CloseReasons, ARRAY[]::VARCHAR[]) AS CloseReasons, rps.Score AS PostScore, rps.ViewCount AS PostViews, nus.PostsCount AS UserPostsCount, nus.TotalBounties, nus.TotalCommentScore
// FROM Users up LEFT JOIN RecentUserStats nus ON up.Id = nus.UserId LEFT JOIN RankedPosts rps ON rps.Rank <= 5 LEFT JOIN ClosedPosts dd ON dd.PostId = rps.PostId
// WHERE up.Reputation > 100 AND nus.TotalBounties > 0 ORDER BY up.Reputation DESC, rps.CreationDate DESC LIMIT 50;
//
// `ON rps.Rank <= 5` names only RankedPosts, so it is a cross join with the top five posts of each type. The close reasons are listed sorted.
fn q22021(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let cut = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let ru = || db.user.with((&db.user.creation_date).gt(cut)).with((&db.user.reputation).gt(100));
    let nus = ru()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt()).opt().and(comments_of(db).select(&db.comment.score).opt())).opt())
        .fold([0i64; 2], |a, x| match x {
            Some((b, c)) => [a[0] + b.flatten().unwrap_or(0), a[1] + c.unwrap_or(0)],
            None => a,
        });
    let pc = ru().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let a = drain((&nus).filt(|a| a[0] > 0).and(&pc));
    let r = top_per(drain(db.post.with(creation_date.gt(cut)).select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let dd = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(&db.post_history.post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().copied().collect();
            let n = t.len() as i64;
            t.sort_unstable();
            t.dedup();
            (n, &*Box::leak(t.into_boxed_slice()))
        });
    let rp = rel(r.into_iter().map(|x| x.0).collect());
    let b = drain((&rp).select(Ident::<Post>::new().and((&dd).opt()))).into_iter().map(|x| x.1).collect::<Vec<_>>();
    let b = left_all(b).v;
    let v = cross_top(a, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), b, |x| x.map(|(p, _)| (Reverse(creation_date.get(p).unwrap()), p)), 50);
    rows(v.into_iter().map(|((u, (s, n)), x)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        match x {
            Some((p, d)) => {
                f.extend(post_fields(db, p, &["id", "title", "created"]));
                f.push(V::I(d.map_or(0, |d| d.0)));
                f.push(V::L(d.map_or(&[][..], |d| d.1).iter().map(|&s| V::S(s)).collect()));
                f.extend(post_fields(db, p, &["score", "views"]));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::I(0), V::L(vec![]), V::Null, V::Null]),
        }
        f.extend([V::I(n), V::I(s[0]), V::I(s[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS Downvotes, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '30 days')),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId),
// PostHistoryVotes AS (SELECT ph.PostId, COUNT(*) AS CloseVotes FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, u.DisplayName AS Owner, rp.ViewCount, rp.Upvotes, rp.Downvotes, COALESCE(ub.BadgeCount, 0) AS GoldBadgeCount,
//        COALESCE(ub.BadgeNames, 'None') AS GoldBadges, COALESCE(pv.CloseVotes, 0) AS CloseVoteCount,
//        CASE WHEN rp.Upvotes - rp.Downvotes > 0 THEN 'Positive' WHEN rp.Upvotes - rp.Downvotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment,
//        CASE WHEN rp.CreationDate <= (cast('2024-10-01' as date) - INTERVAL '15 days') THEN 'Old' ELSE 'New' END AS PostAge
// FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostHistoryVotes pv ON rp.Id = pv.PostId
// WHERE rp.PostRank <= 10 ORDER BY rp.ViewCount DESC, rp.CreationDate DESC;
//
// PostRank numbers the joined vote rows, so a post with several votes fills several of the ten places. Badge names are in badge id order.
fn q21116(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30)));
    let j = drain(recent().select(votes_of(db).opt()));
    let j = top_n(j, |&(p, v)| (Reverse(creation_date.get(p).unwrap()), p, v), 10);
    let jr = rel(j.into_iter().map(|x| x.0).collect());
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let pv = db.post_history.with((&db.post_history.post_history_type_id).eq(10).or((&db.post_history.post_history_type_id).eq(11))).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&jr).select(Ident::<Post>::new().and(&vc).and(owner_user.opt()).and(owner_user.select(&ub).opt()).and((&pv).opt())));
    let mut v = v;
    v.sort_by_key(|&(_, ((((p, _), _), _), _))| (view_count.get(p).is_none(), Reverse(view_count.get(p)), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((((p, a), u), b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(ostr(u.map(|u| db.user.display_name.get(u).unwrap())));
        f.extend(post_fields(db, p, &["views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b.map_or(0, |b| b.0)), V::S(b.map_or("None", |b| b.1)), V::I(c.unwrap_or(0))]);
        f.push(V::S(if a[0] - a[1] > 0 { "Positive" } else if a[0] - a[1] < 0 { "Negative" } else { "Neutral" }));
        f.push(V::S(if creation_date.get(p).unwrap() <= add_days(date(2024, 10, 1), -15) { "Old" } else { "New" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.Score > 0 AND p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        COUNT(DISTINCT c.Id) AS CommentCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Comments c ON u.Id = c.UserId GROUP BY u.Id, u.DisplayName),
// PostWithComments AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(c.Score) AS TotalCommentScore, STRING_AGG(DISTINCT c.UserDisplayName, ', ') AS CommentUsers
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title)
// SELECT r.PostId, r.Title, r.Score, r.ViewCount, r.CreationDate, u.DisplayName AS UserWithMostBadges, u.BadgeCount, u.TotalUpVotes, u.TotalDownVotes, p.CommentCount AS PostCommentCount,
//        p.TotalCommentScore, p.CommentUsers
// FROM RankedPosts r LEFT JOIN UserStatistics u ON u.UserId = (SELECT u2.Id FROM Users u2 ORDER BY u2.Reputation DESC LIMIT 1) LEFT JOIN PostWithComments p ON p.PostId = r.PostId
// WHERE r.Rank <= 10 ORDER BY r.Score DESC, r.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// The ON compares u.UserId with an uncorrelated scalar, so UserStatistics is cut to that one user and crossed with every ranked post; its
// badge x vote x comment product is driven for that user alone. The DISTINCT commenter names are listed sorted.
fn q1222(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let r = top_per(
        drain(db.post.with(score.gt(0)).with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id)),
        |&(_, t)| t,
        |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p),
        10,
        false,
    );
    let r = top_n(r, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    let top = top_n(drain(db.user.select(&db.user.reputation)), |&(u, rp)| (Reverse(rp), u), 1);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let User { up_votes, down_votes, .. } = &db.user;
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(badges_of(db).opt()).and(votes_by(db).opt()).and(comments_by(db).opt()))
        .fold([0i64; 3], |a, ((((uv, dv), b), _), _)| [a[0] + b.is_some() as i64, a[1] + uv, a[2] + dv]);
    let ur = rel(drain(&us));
    let Comment { score: cs, user_display_name, .. } = &db.comment;
    let rp = rel(r.into_iter().map(|x| x.0).collect());
    let pwc = (&rp)
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select(cs.and(user_display_name.opt())).opt())
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().flatten().flat_map(|x| x.1).collect();
            t.sort_unstable();
            t.dedup();
            let n = v.iter().flatten().count() as i64;
            (n, v.iter().flatten().map(|x| x.0).sum::<i64>(), agg_str(t, ", "))
        });
    let v = drain((&rp).select(Ident::<Post>::new().and((&pwc).opt())).cross(left_all(ur.v.clone())));
    let v = top_n(v, |&((_, _), ((p, _), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(_, ((p, c), u))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        match u {
            Some((u, a)) => {
                f.extend(ucols(db, u, &["name"]));
                f.extend(a.map(V::I));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        match c {
            Some((n, s, t)) => f.extend([V::I(n), if n == 0 { V::Null } else { V::I(s) }, ostr(t)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("26632", q26632),
    ("22770", q22770),
    ("4028", q4028),
    ("3240", q3240),
    ("33785", q33785),
    ("20128", q20128),
    ("30308", q30308),
    ("21718", q21718),
    ("25050", q25050),
    ("34314", q34314),
    ("3722", q3722),
    ("26870", q26870),
    ("22021", q22021),
    ("21116", q21116),
    ("1222", q1222),
];
