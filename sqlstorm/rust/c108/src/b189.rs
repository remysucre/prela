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

fn tag_mentions_ci(db: &'static So) -> MatSet<(Id<Post>, Id<Tag>)> {
    let elems: MatSet<Str> = (&db.post.tags_str).flat_map(tag_list).collect();
    let contains: HashIdx<Str, Id<Tag>> =
        (&elems).select_where((&db.tag.tag_name).inv(), |e: Str, n: Str| e.to_lowercase().contains(&n.to_lowercase())).collect();
    db.post.select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&contains))).collect()
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
    let w = recent().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let rr: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), k)| (p, k)).collect();
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
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&up).and(&bc)).window(rank, |((_, a), _)| Reverse(a.2), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, (((u, a), b), _))| {
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
    let w = recent()
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(votes_of(db).opt()).and(view_count.opt()))
        .window(row_number, |((p, v), w)| (w.is_none(), Reverse(w), p, v), asc);
    let rr: MatSet<(Id<Post>, i64)> = (&w).map(|(((p, _), _), k)| (p, k)).collect();
    type R = (Id<Post>, i64);
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |a, c| (a.0 + c.is_some() as i64, a.1.max(c.unwrap_or(0))));
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let pht = db.post_history.with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).group_by(&db.post_history.post).select(htype_name(db)).buf_fold(|v| leak(v.join(", ")));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&rr).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&vc).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ub))).and(&pht)))));
    let v = top_n(v, |&(_, ((p, k), ((n, _), _)))| (Reverse(n), creation_date.get(p).unwrap(), p, k), 50);
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
    let first = |t: Option<Str>| {
        t.map(|t| {
            let mut a = ['\0'; 3];
            for (i, c) in t.chars().take(1).flat_map(|c| c.to_uppercase()).enumerate() {
                a[i] = c;
            }
            a
        })
    };
    let w = db.post.with(post_type_id.eq(1)).group_by(title.opt().map(first)).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
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
    let w = whole(db.user.with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(row_number, |(u, r)| (Reverse(r), u), asc);
    let tu: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 100).map(|((u, _), k)| (u, k)).collect();
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let us: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let pa = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(creation_date)).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, d)) => [a[0] + 1, a[1] + s, a[2] + (d < cut) as i64 * s, a[3] + (d < cut) as i64],
        None => a,
    });
    let ub = (&us).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    type U = (Id<User>, i64);
    let mut v = drain((&tu).select(Same::<U>::new().and(Same::<U>::new().map(|(u, _): U| u).select((&pa).and(&ub)))));
    v.sort_by_key(|&((_, k), _)| k);
    rows(v.into_iter().map(|(_, ((u, k), (a, b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::F(if a[3] == 0 { 0.0 } else { a[2] as f64 / a[3] as f64 })]);
        f.extend(b.map(V::I));
        f.push(V::I(k));
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
    let w = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let ps: MatSet<Id<Post>> = (&w).filt(|(((_, s), _), k)| k <= 5 || s > 100).map(|(((p, _), _), _)| p).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { comment, text, creation_date: hd, .. } = &db.post_history;
    let cah = (&ps).group_by(Ident::<Post>::new()).select(history_of(db).select(htype_name(db).and(comment.opt()).and(text.opt()).and(hd))).buf_fold(|v| {
        (
            agg_str(v.iter().filter(|x| x.0 .0 .0 == "Post Closed").flat_map(|x| x.0 .0 .1), "; "),
            agg_str(v.iter().filter(|x| x.0 .0 .0.starts_with("Edit ")).flat_map(|x| x.0 .1), "; "),
            v.iter().map(|x| x.1).max().unwrap(),
        )
    });
    let v = drain(
        (&ps).select(
            Ident::<Post>::new().and(
                comments_of(db)
                    .select(&db.comment.user)
                    .select(Ident::<User>::new().and((&db.user.reputation).and(&ur).filt(|(r, a): (i64, [i64; 2])| r <= 1000 || a[0] > a[1])))
                    .and((&cah).opt()),
            ),
        ),
    );
    let mut v = v;
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), view_count.get(p).is_none(), Reverse(view_count.get(p)), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, (p, ((u, (_, a)), c)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
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
    let w = rp().group_by(owner_user).select(Ident::<Post>::new().and(view_count.opt())).window(row_number, |(p, w)| (w.is_none(), Reverse(w), p), asc);
    let r5: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let pe = db
        .post
        .with(creation_date.ge(add_months(current_date(), -6)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&r5).select(Ident::<Post>::new().and(&pe).and(tags_str.flat_map(tag_list).select(Same::<Str>::new().with(&tt)))));
    let mut v = v;
    v.sort_by_key(|&(_, ((p, _), _))| (view_count.get(p).is_none(), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(_, ((p, a), t))| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "views", "created", "owner"]);
        f.push(V::S(t));
        f.extend(a.map(V::I));
        row(f)
    }))
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
    let w = recent()
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(votes_of(db).opt()).and(score))
        .window(row_number, |((p, v), s)| (Reverse(s), p, v), asc);
    let r1: MatSet<Id<Post>> = (&w).filt(|(((_, _), s), k)| k == 1 && s > 0).map(|(((p, _), _), _)| p).collect();
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let Comment { creation_date: cdt, .. } = &db.comment;
    let rc = db.comment.with(cdt.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(&db.comment.post).select(cdt).fold((0i64, i64::MIN), |a, d| (a.0 + 1, a.1.max(d)));
    let PostHistory { creation_date: hd, comment, .. } = &db.post_history;
    let phf = db.post_history.with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).group_by(&db.post_history.post).select(comment.opt()).buf_fold(|v| agg_str(v.iter().flatten().copied(), ", "));
    let v = drain((&r1).select(Ident::<Post>::new().and(&vc).and((&rc).opt()).and((&phf).opt())));
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
    v.sort_by_key(|&(_, ((_, ((a, _), _)), n))| (Reverse(a[0]), Reverse(n)));
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
    let w = db.post.with(creation_date.gt(cut)).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
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
    type DD = (i64, &'static [Str]);
    let rk: HashIdx<(), (Id<Post>, Option<DD>)> = whole(&rp).select(Ident::<Post>::new().and((&dd).opt())).collect();
    let v = drain((&nus).filt(|a| a[0] > 0).and(&pc).and(Ident::<User>::new().map(|_| ()).select(&rk).opt()));
    let v = top_n(v, |&(u, (_, x))| (Reverse(db.user.reputation.get(u).unwrap()), x.is_none(), x.map(|(p, _)| Reverse(creation_date.get(p).unwrap())), u, x.map(|(p, _)| p)), 50);
    rows(v.into_iter().map(|(u, ((s, n), x))| {
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
    let Post { creation_date, owner_user, owner_user_id, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30)));
    let j = drain(recent().select(votes_of(db).opt()));
    let j = top_n(j, |&(p, v)| (Reverse(creation_date.get(p).unwrap()), p, v), 10);
    let jr = rel(j.into_iter().map(|x| x.0).collect());
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user_id).select(&db.badge.name).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let pv = db.post_history.with((&db.post_history.post_history_type_id).eq(10).or((&db.post_history.post_history_type_id).eq(11))).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&jr).select(Ident::<Post>::new().and(&vc).and(owner_user.opt()).and(owner_user_id.select(&ub).opt()).and((&pv).opt())));
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
    let w = db
        .post
        .with(score.gt(0))
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let top = top_n(drain(db.user.select(&db.user.reputation)), |&(u, rp)| (Reverse(rp), u), 1);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let User { up_votes, down_votes, .. } = &db.user;
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(badges_of(db).opt()).and(votes_by(db).opt()).and(comments_by(db).opt()))
        .fold([0i64; 3], |a, ((((uv, dv), b), _), _)| [a[0] + b.is_some() as i64, a[1] + uv, a[2] + dv]);
    let ur: HashIdx<(), (Id<User>, [i64; 3])> = whole(&tu).select(Ident::<User>::new().and(&us)).collect();
    let Comment { score: cs, user_display_name, .. } = &db.comment;
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
    let v = drain((&rp).select(Ident::<Post>::new().and((&pwc).opt()).and(Ident::<Post>::new().map(|_| ()).select(&ur).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
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

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// ClosedAndEditedPosts AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseEvents, COUNT(CASE WHEN ph.PostHistoryTypeId IN (4, 5) THEN 1 END) AS EditEvents,
//        MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 4, 5) GROUP BY ph.PostId),
// UserBadges AS (SELECT UserId, COUNT(*) AS TotalBadges, STRING_AGG(Name, ', ') AS BadgeNames FROM Badges GROUP BY UserId),
// PostsWithTitleReplies AS (SELECT p.Id, p.Title, COALESCE((SELECT STRING_AGG(c.Text, ' | ') FROM Comments c WHERE c.PostId = p.Id), 'No comments') AS CommentSnippets FROM Posts p)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.OwnerDisplayName, ca.CloseEvents, ca.EditEvents, ca.LastEditDate, ub.TotalBadges, ub.BadgeNames, pwtr.CommentSnippets
// FROM RankedPosts rp LEFT JOIN ClosedAndEditedPosts ca ON rp.PostId = ca.PostId LEFT JOIN UserBadges ub ON rp.OwnerDisplayName = ub.UserId::varchar
// LEFT JOIN PostsWithTitleReplies pwtr ON rp.PostId = pwtr.Id WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC, COALESCE(ca.CloseEvents, 0) DESC LIMIT 10;
//
// `OwnerDisplayName = UserId::varchar` matches a display name that is exactly the decimal text of a user id, joined through the raw ids.
// Rank ties go to the smaller post id; badge names and comments are in id order.
fn q21004(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let t =&db.post_history.post_history_type_id;
    let ca = db
        .post_history
        .with(t.eq(10).or(t.eq(11)).or(t.eq(4)).or(t.eq(5)))
        .group_by(&db.post_history.post)
        .select(t.and(&db.post_history.creation_date))
        .fold((0i64, 0i64, i64::MIN), |a, (t, d)| (a.0 + (t == 10 || t == 11) as i64, a.1 + (t == 4 || t == 5) as i64, a.2.max(d)));
    let ub = db.badge.group_by(&db.badge.user_id).select(&db.badge.name).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let numeric = |s: Str| s.parse::<i64>().ok().filter(|n| n.to_string() == s);
    let cs = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.text)).buf_fold(|v| leak(v.join(" | ")));
    let v = drain((&rp).select(Ident::<Post>::new().and((&ca).opt()).and(owner_user.select(&db.user.display_name).flat_map(numeric).select(&ub).opt()).and((&cs).opt())));
    let v = top_n(v, |&(_, (((p, c), _), _))| (Reverse(score.get(p).unwrap()), view_count.get(p).is_none(), Reverse(view_count.get(p)), Reverse(c.map_or(0, |c| c.0))), 10);
    rows(v.into_iter().map(|(_, (((p, c), b), s))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        match c {
            Some((x, y, d)) => f.extend([V::I(x), V::I(y), V::T(d)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        match b {
            Some((n, s)) => f.extend([V::I(n), V::S(s)]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(V::S(s.unwrap_or("No comments")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.Score IS NOT NULL GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp
//     WHERE rp.rn = 1 AND (rp.UpVoteCount - rp.DownVoteCount) >= 5),
// PostHistoryData AS (SELECT ph.PostId, ph.CreationDate AS EditDate, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// AggregatedHistory AS (SELECT ph.PostId, MAX(ph.EditDate) AS LastEditDate, STRING_AGG(ph.CloseReason, ', ') AS CloseReasons FROM PostHistoryData ph GROUP BY ph.PostId)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.CommentCount, ah.LastEditDate, ah.CloseReasons
// FROM FilteredPosts fp LEFT JOIN AggregatedHistory ah ON fp.PostId = ah.PostId WHERE ah.CloseReasons IS NOT NULL OR ah.LastEditDate < fp.CreationDate
// ORDER BY fp.ViewCount DESC, fp.Score DESC LIMIT 100;
//
// rn reads only base columns, so the newest post of each type is picked first and the comment x vote product is driven for it alone.
// The WHERE needs an AggregatedHistory row, so that join is inner. Close reasons are in history id order.
fn q24054(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let e = (&fp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id: t, comment, creation_date: hd, .. } = &db.post_history;
    let ah = db
        .post_history
        .with(t.eq(10).or(t.eq(11)).or(t.eq(12)))
        .group_by(&db.post_history.post)
        .select(hd.and(comment.opt()))
        .buf_fold(|v| (v.iter().map(|x| x.0).max().unwrap(), agg_str(v.iter().flat_map(|x| x.1), ", ")));
    let v = drain((&fp).select(Ident::<Post>::new().and((&e).filt(|a| a[1] - a[2] >= 5)).and(&ah)).filt(|((p, _), (d, s)): ((Id<Post>, [i64; 3]), (i64, Option<Str>))| s.is_some() || d < creation_date.get(p).unwrap()));
    let v = top_n(v, |&(p, _)| (view_count.get(p).is_none(), Reverse(view_count.get(p)), Reverse(score.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(p, ((_, a), (d, s)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::T(d), ostr(s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, U.Reputation, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(RP.PostCount, 0) AS TotalPosts, COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(PC.CommentCount, 0) AS TotalComments
//     FROM Users U LEFT JOIN (SELECT OwnerUserId, COUNT(Id) AS PostCount FROM Posts GROUP BY OwnerUserId) RP ON U.Id = RP.OwnerUserId LEFT JOIN UserBadges UB ON U.Id = UB.UserId
//     LEFT JOIN PostComments PC ON U.Id = PC.PostId),
// TopUsers AS (SELECT UA.UserId, UA.DisplayName, UA.TotalPosts, UA.TotalBadges, UA.TotalComments, ROW_NUMBER() OVER (ORDER BY UA.TotalPosts DESC, UA.TotalBadges DESC) AS UserRank FROM UserActivity UA)
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalBadges, TU.TotalComments, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, TU.UserRank
// FROM TopUsers TU LEFT JOIN RankedPosts RP ON TU.UserId = RP.OwnerUserId WHERE TU.UserRank <= 10 AND (RP.Score IS NULL OR RP.Score > 10) ORDER BY TU.UserRank;
//
// `U.Id = PC.PostId` joins a user id to a post id, so it goes through the raw ids. PostRank is never read.
fn q3769(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let w = whole(db.user.iq())
        .select(Ident::<User>::new().and(&pc).and(&bc).and((&db.user.origid).select(&pidx).select(&cc).opt()))
        .window(row_number, |(((u, p), b), _)| (Reverse(p), Reverse(b), u), asc);
    type T = (Id<User>, i64, i64, i64, i64);
    let tu: MatSet<T> = (&w).filt(|(_, k)| k <= 10).map(|((((u, p), b), c), k)| (u, p, b, c.unwrap_or(0), k)).collect();
    let cd = add_years(current_date(), -1);
    let rp = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(cd)).and(score));
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select(rp.opt()))).filt(|(_, p): (T, Option<(Id<Post>, i64)>)| p.map_or(true, |(_, s)| s > 10)));
    let mut v = v;
    v.sort_by_key(|x| x.1 .0 .4);
    rows(v.into_iter().map(|(_, ((u, n, b, c, k), p))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(n), V::I(b), V::I(c)]);
        match p {
            Some((p, _)) =>f.extend(post_fields(db, p, &["title", "created", "score", "views"])),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        f.push(V::I(k));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserActivity),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, COALESCE(CAH.AnswerCount, 0) AS AnswerCount, COALESCE(PH.CloseReasonTypes, 'Not Closed') AS CloseReason, P.OwnerUserId
//     FROM Posts P LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) AS CAH ON P.Id = CAH.ParentId
//     LEFT JOIN (SELECT PostId, STRING_AGG(Comment, ', ') AS CloseReasonTypes FROM PostHistory WHERE PostHistoryTypeId = 10 GROUP BY PostId) AS PH ON P.Id = PH.PostId)
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, PD.Title, PD.CreationDate, PD.ViewCount, PD.AnswerCount, PD.CloseReason
// FROM TopUsers TU JOIN PostDetails PD ON TU.UserId = PD.OwnerUserId WHERE TU.ViewRank <= 10 ORDER BY TU.Reputation DESC, PD.CreationDate DESC;
//
// Close comments are in history id order.
fn q3242(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups)).window(rank, |(_, a)| (a[5] == 0, Reverse(a[6])), asc);
    type T = (Id<User>, [i64; 10]);
    let tu: MatSet<T> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let ac = db.post.group_by(Ident::<Post>::new()).select(answers_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.eq(10)).group_by(&db.post_history.post).select(comment.opt()).buf_fold(|v| agg_str(v.iter().flatten().copied(), ", "));
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select(posts_of(db).select(Ident::<Post>::new().and((&ac).opt()).and((&ph).opt()))))));
    let mut v = v;
    v.sort_by_key(|&(_, ((u, _), ((p, _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(db.post.creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((u, a), ((p, n), h)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.push(V::I(n.unwrap_or(0)));
        f.push(V::S(h.flatten().unwrap_or("Not Closed")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName, ph.Comment, ph.PostHistoryTypeId FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// TagData AS (SELECT p.Id AS PostId, SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2) AS Tags FROM Posts p)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, COALESCE(cp.UserDisplayName, 'N/A') AS ClosuredBy, CASE WHEN COUNT(DISTINCT cp.PostId) > 0 THEN 'Yes' ELSE 'No' END AS IsClosed,
//        STRING_AGG(td.Tags, ', ') AS AssociatedTags, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount
// FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId LEFT JOIN TagData td ON rp.PostId = td.PostId LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId
// WHERE rp.PostRank = 1 GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, cp.UserDisplayName
// HAVING COUNT(c.Id) > 5 OR SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) > 10 ORDER BY rp.CreationDate DESC LIMIT 50;
//
// PostRank reads only base columns, so each owner's newest question is picked first. The group is keyed by the post and the closer's name, so the
// joined (post, ClosedPosts row) pairs are materialised and grouped; the comment x vote product hangs off the post. Every row of a group carries the
// same TagData string, so the STRING_AGG is that string once per joined row.
fn q20256(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, tags_str, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let PostHistory { post_history_type_id: t, user_display_name, .. } = &db.post_history;
    let cph = history_of(db).select(Ident::<PostHistory>::new().with(t.eq(10).or(t.eq(11))));
    type J = (Id<Post>, Option<Id<PostHistory>>);
    let j: MatSet<J> = (&rp).select(Ident::<Post>::new().and(cph.opt())).collect();
    let g = (&j)
        .group_by(Same::<J>::new().map(|x: J| (x.0, x.1.map(|h| user_display_name.get(h)))))
        .select(Same::<J>::new().and(Same::<J>::new().map(|x: J| x.0).select(tags_str.opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))))
        .fold([0i64; 5], |a, ((_, h), ((tg, c), v))| [a[0] | h.is_some() as i64, a[1] + c.is_some() as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(2) || v == Some(3)) as i64, a[4] + tg.is_some() as i64]);
    let dc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&g).filt(|a| a[1] > 5 || a[2] > 10));
    type K = (Id<Post>, Option<Option<Str>>);
    let v = drain(rel(v).select(Same::<(K, [i64; 5])>::new().and(Same::<(K, [i64; 5])>::new().map(|x: (K, [i64; 5])| x.0 .0).select(&dc))));
    let v = top_n(v, |&(_, (((p, _), _), _))| Reverse(creation_date.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(_, (((p, n), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(V::S(n.flatten().unwrap_or("N/A")));
        f.push(V::S(if a[0] > 0 { "Yes" } else { "No" }));
        f.push(match tags_str.get(p) {
            Some(s) if a[4] > 0 => {
                let n = s.chars().count();
                let inner: String = s.chars().skip(1).take(n.saturating_sub(2)).collect();
                V::Owned(vec![inner; a[4] as usize].join(", "))
            }
            _ => V::Null,
        });
        f.extend([V::I(c), V::I(a[3])]);
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS TagName FROM Posts p WHERE p.PostTypeId = 1),
// TagCounts AS (SELECT TagName, COUNT(*) AS NumQuestions FROM PostTags GROUP BY TagName),
// TopTags AS (SELECT TagName, NumQuestions, ROW_NUMBER() OVER (ORDER BY NumQuestions DESC) AS Rank FROM TagCounts),
// UserVotes AS (SELECT v.UserId, COUNT(v.PostId) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(uv.VoteCount, 0) AS TotalVotes, COALESCE(uv.UpVoteCount, 0) AS TotalUpVotes, COALESCE(uv.DownVoteCount, 0) AS TotalDownVotes,
//        COALESCE(ub.BadgeCount, 0) AS TotalBadges, COALESCE(ub.GoldBadges, 0) AS TotalGoldBadges, COALESCE(ub.SilverBadges, 0) AS TotalSilverBadges, COALESCE(ub.BronzeBadges, 0) AS TotalBronzeBadges,
//        tt.TagName, tt.NumQuestions AS QuestionsTagged
// FROM Users u LEFT JOIN UserVotes uv ON u.Id = uv.UserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId JOIN TopTags tt ON tt.Rank <= 10
// ORDER BY tt.NumQuestions DESC, u.Reputation DESC LIMIT 50;
//
// `ON tt.Rank <= 10` names only TopTags, so it is a cross join with the ten most used tags.
fn q26014(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let tc = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10);
    let tt: MatSet<(Str, i64)> = rel(tt).map(|x| x).collect();
    let tti: HashIdx<(), (Str, i64)> = whole(&tt).collect();
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let top: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 50).map(|((u, _), _)| u).collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vtype_name(db)).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some("UpMod")) as i64, a[2] + (t == Some("DownMod")) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]);
    let v = drain((&top).select(Ident::<User>::new().and(&uv).and(&ub).and(Ident::<User>::new().map(|_| ()).select(&tti))));
    let v = top_n(v, |&(_, (((u, _), _), (t, n)))| (Reverse(n), Reverse(db.user.reputation.get(u).unwrap()), t, u), 50);
    rows(v.into_iter().map(|(_, (((u, a), b), (t, n)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Author, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= '2022-01-01' AND p.Score IS NOT NULL),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes v GROUP BY v.PostId),
// PostHistoryStats AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS RevisionCount FROM PostHistory ph WHERE ph.CreationDate >= '2023-01-01' GROUP BY ph.PostId, ph.PostHistoryTypeId)
// SELECT rp.PostId, rp.Title, rp.Author, rp.CreationDate, COALESCE(pv.Upvotes, 0) AS TotalUpvotes, COALESCE(pv.Downvotes, 0) AS TotalDownvotes, rp.Score, rp.CommentCount, rph.RevisionCount,
//        CASE WHEN rp.Score > 10 THEN 'Highly Scored' WHEN rp.Score BETWEEN 1 AND 10 THEN 'Moderately Scored' ELSE 'Low Scored' END AS ScoreCategory,
//        CASE WHEN rp.CommentCount > 0 THEN (SELECT STRING_AGG(c.Text, '; ') FROM Comments c WHERE c.PostId = rp.PostId LIMIT 3) ELSE 'No Comments' END AS RecentComments
// FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId LEFT JOIN PostHistoryStats rph ON rp.PostId = rph.PostId
// WHERE rp.Rank <= 5 AND (rph.PostHistoryTypeId IS NULL OR rph.RevisionCount > 1) ORDER BY rp.Score DESC, rp.CreationDate ASC;
//
// Rank numbers the joined comment rows. The LIMIT 3 cuts the one row STRING_AGG returns, so every comment is listed, in comment id order.
fn q25000(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(ts(2022, 1, 1, 0, 0, 0)));
    type J = (Id<Post>, Option<Id<Comment>>);
    let w = recent()
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(comments_of(db).opt()).and(creation_date))
        .window(row_number, |((p, c), d)| (Reverse(d), p, c), asc);
    let rp: MatSet<J> = (&w).filt(|(_, k)| k <= 5).map(|(((p, c), _), _)| (p, c)).collect();
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let rph = db.post_history.with(hd.ge(ts(2023, 1, 1, 0, 0, 0))).group_by(post.and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rr = rel(drain(&rph));
    type H = ((Id<Post>, i64), i64);
    let hidx: HashIdx<Id<Post>, H> = (&rr).map(|x: H| x.0 .0).inv().select(&rr).collect();
    let ct = recent().group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.text)).buf_fold(|v| leak(v.join("; ")));
    let v = drain((&rp).select(Same::<J>::new().map(|x: J| x.0).select(Ident::<Post>::new().and(&cc).and((&pv).opt()).and((&hidx).opt()).and((&ct).opt())).filt(|((_, h), _): ((((Id<Post>, i64), Option<[i64; 2]>), Option<H>), Option<Str>)| h.map_or(true, |h| h.1 > 1))));
    let mut v = v;
    v.sort_by_key(|&(_, ((((p, _), _), _), _))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(_, ((((p, c), a), h), t))| {
        let s = score.get(p).unwrap();
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(s), V::I(c), oint(h.map(|h| h.1))]);
        f.push(V::S(if s > 10 { "Highly Scored" } else if (1..=10).contains(&s) { "Moderately Scored" } else { "Low Scored" }));
        f.push(if c > 0 { ostr(t) } else { V::S("No Comments") });
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U WHERE U.Reputation >= 1000),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, P.ViewCount, P.CommentCount, STRING_AGG(T.TagName, ', ') AS Tags
//     FROM Posts P JOIN Tags T ON POSITION(T.TagName IN P.Tags) > 0 WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
//     GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, P.Score, P.ViewCount, P.CommentCount),
// PostStatistics AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.OwnerUserId, RU.DisplayName AS OwnerDisplayName, RP.Score, RP.ViewCount, RP.CommentCount, RP.Tags,
//        COALESCE(AVG(C.Score), 0) AS AverageCommentScore, COUNT(DISTINCT C.Id) AS TotalComments, COUNT(DISTINCT PH.Id) AS EditCount
//     FROM RecentPosts RP LEFT JOIN Comments C ON C.PostId = RP.PostId LEFT JOIN PostHistory PH ON PH.PostId = RP.PostId AND PH.PostHistoryTypeId IN (4, 5, 6)
//     LEFT JOIN RankedUsers RU ON RP.OwnerUserId = RU.Id
//     GROUP BY RP.PostId, RP.Title, RP.CreationDate, RP.OwnerUserId, RU.DisplayName, RP.Score, RP.ViewCount, RP.CommentCount, RP.Tags)
// SELECT PS.PostId, PS.Title, PS.CreationDate, PS.OwnerDisplayName, PS.Score, PS.ViewCount, PS.CommentCount, PS.Tags, PS.AverageCommentScore, PS.TotalComments, PS.EditCount
// FROM PostStatistics PS ORDER BY PS.Score DESC, PS.ViewCount DESC, PS.TotalComments DESC;
//
// A tag name has no '<' or '>', so POSITION in the Tags text is a match inside one tag, which is tag_mentions. The tags are listed in tag id order.
fn q27667(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let tm = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    let cut = add_months(ts(2024, 10, 1, 12, 34, 56), -1);
    let tg = (&tm)
        .group_by(Same::<M>::new().map(|x: M| x.0).with(creation_date.gt(cut)))
        .select(Same::<M>::new().map(|x: M| x.1))
        .buf_fold(|v| {
            let mut t: Vec<Id<Tag>> = v.iter().copied().collect();
            t.sort_unstable();
            leak(t.iter().map(|&t| db.tag.tag_name.get(t).unwrap()).collect::<Vec<_>>().join(", "))
        });
    let t = &db.post_history.post_history_type_id;
    let eh = || history_of(db).select(Ident::<PostHistory>::new().with(t.eq(4).or(t.eq(5)).or(t.eq(6))));
    let tps: MatSet<Id<Post>> = db.post.with(&tg).collect();
    let ps = (&tps).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt().and(eh().opt())).fold([0i64; 2], |a, (c, _)| [a[0] + c.unwrap_or(0), a[1] + c.is_some() as i64]);
    let dc = (&tps).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let de = (&tps).group_by(Ident::<Post>::new()).select(eh().opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ru = owner_user.select(Ident::<User>::new().with((&db.user.reputation).ge(1000)));
    let v = drain((&tg).and(&ps).and(&dc).and(&de).and(ru.opt()));
    let mut v = v;
    v.sort_by_key(|&(p, ((((_, _), c), _), _))| {
        let w = db.post.view_count.get(p);
        (Reverse(db.post.score.get(p).unwrap()), w.is_none(), Reverse(w), Reverse(c))
    });
    rows(v.into_iter().map(|(p, ((((t, a), c), e), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(ostr(u.map(|u| db.user.display_name.get(u).unwrap())));
        f.extend(post_fields(db, p, &["score", "views", "comments"]));
        f.extend([V::S(t), V::F(if a[1] == 0 { 0.0 } else { a[0] as f64 / a[1] as f64 }), V::I(c), V::I(e)]);
        row(f)
    }))
}

// Rewritten (rewrites/32480.sql): the original, with ph.Id carried through RecursivePostHistory and both of its orders refined with `, ph.Id DESC`:
// WITH RecursivePostHistory AS (SELECT ph.Id, ph.PostId, ph.CreationDate, ph.UserId, ph.Comment, ph.PostHistoryTypeId,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC, ph.Id DESC) as rn FROM PostHistory ph),
// UserBadgeCounts AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) as GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) as SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) as BronzeBadges FROM Badges b GROUP BY b.UserId),
// UserVoteSummary AS (SELECT v.UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.UserId),
// PostMetrics AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.OwnerUserId)
// SELECT u.Id AS UserId, u.DisplayName, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ups.UpVotes, ups.DownVotes, pm.PostId, pm.CommentCount, pm.VoteCount, pm.TotalViews, pm.TotalScore,
//        (SELECT ARRAY_AGG(ph.Comment ORDER BY ph.CreationDate DESC, ph.Id DESC) FROM RecursivePostHistory ph WHERE ph.PostId = pm.PostId) AS RecentHistoryComments
// FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN UserVoteSummary ups ON u.Id = ups.UserId LEFT JOIN PostMetrics pm ON u.Id = pm.OwnerUserId
// WHERE (ub.GoldBadges > 0 OR ub.SilverBadges > 0 OR ub.BronzeBadges > 0) AND pm.VoteCount > 10 ORDER BY pm.TotalScore DESC, u.Reputation DESC;
fn q32480(db: &'static So) -> String {
    let Post { creation_date, view_count, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pm = recent()
        .group_by(Ident::<Post>::new())
        .select(view_count.opt().and(score).and(comments_of(db).opt()).and(votes_of(db).opt()))
        .fold([0i64; 3], |a, (((w, s), _), _)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + s]);
    let nc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let nv = recent().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ups = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { creation_date: hd, comment, .. } = &db.post_history;
    let arr = recent().group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().and(hd).and(comment.opt()))).buf_fold(|v| {
        let mut h: Vec<_> = v.iter().copied().collect();
        h.sort_by_key(|&((h, d), _)| (Reverse(d), Reverse(h)));
        &*Box::leak(h.into_iter().map(|(_, c)| c).collect::<Vec<_>>().into_boxed_slice())
    });
    let v = drain(
        db.user
            .with((&ub).filt(|b| b[0] > 0 || b[1] > 0 || b[2] > 0))
            .select(Ident::<User>::new().and(&ub).and((&ups).opt()).and(posts_of(db).select(Ident::<Post>::new().and(&pm).and(&nc).and((&nv).filt(|n| n > 10)).and((&arr).opt())))),
    );
    let mut v = v;
    v.sort_by_key(|&(_, (((u, _), _), ((((_, a), _), _), _)))| (Reverse(a[2]), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(_, (((u, b), up), ((((p, a), c), n), h)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([oint(up.map(|x| x[0])), oint(up.map(|x| x[1]))]);
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(c), V::I(n), if a[0] == 0 { V::Null } else { V::I(a[1]) }, V::I(a[2])]);
        f.push(match h {
            Some(h) => V::L(h.iter().map(|&x| ostr(x)).collect()),
            None => V::Null,
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(V.BountyAmount) AS TotalBounties, DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName, U.Reputation),
// ClosedPosts AS (SELECT PH.PostId, COUNT(PH.PostId) AS CloseCount, STRING_AGG(DISTINCT CTR.Name, ', ') AS CloseReasons FROM PostHistory PH
//     JOIN CloseReasonTypes CTR ON CAST(PH.Comment AS INTEGER) = CTR.Id WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId),
// ActiveUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UP.PostCount, 0) AS TotalPosts, COALESCE(CP.CloseCount, 0) AS TotalClosedPosts,
//        COALESCE(CP.CloseReasons, 'None') AS CloseReasons FROM Users U LEFT JOIN UserStats UP ON U.Id = UP.UserId LEFT JOIN ClosedPosts CP ON U.Id = CP.PostId WHERE U.Reputation > 1000),
// TopPosters AS (SELECT UserId, SUM(TotalPosts) AS FollowerCount, COUNT(*) AS UniqueClosedPosts FROM ActiveUsers WHERE CloseReasons <> 'None' GROUP BY UserId)
// SELECT AU.DisplayName, AU.Reputation, AU.TotalPosts, AU.TotalClosedPosts, AU.CloseReasons, CASE WHEN AU.TotalClosedPosts > 0 THEN 'Active in Discussions' ELSE 'Lurker' END AS UserStatus,
//        COALESCE(T.FollowerCount, 0) AS Followers, R.ReputationRank
// FROM ActiveUsers AU JOIN UserStats R ON AU.UserId = R.UserId LEFT JOIN TopPosters T ON AU.UserId = T.UserId ORDER BY AU.Reputation DESC, AU.DisplayName ASC;
//
// `U.Id = CP.PostId` joins a user id to a post id, so it goes through the raw ids. The DISTINCT close reasons are listed sorted.
fn q21470(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).opt()).opt())
        .fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation).and(&us)).window(dense_rank, |((_, r), _)| Reverse(r), asc);
    type K = (Id<User>, i64, i64);
    let rk: MatSet<K> = (&w).map(|(((u, _), n), k)| (u, n, k)).collect();
    let kidx: HashIdx<Id<User>, K> = (&rk).map(|x: K| x.0).inv().collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(&db.post_history.post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().copied().collect();
            let n = t.len() as i64;
            t.sort_unstable();
            t.dedup();
            (n, leak(t.join(", ")))
        });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    type A = (Id<User>, (K, Option<(i64, Str)>));
    let au: MatSet<A> = db.user.with((&db.user.reputation).gt(1000)).select(Ident::<User>::new().and((&kidx).and((&db.user.origid).select(&pidx).select(&cp).opt()))).collect();
    let tp = (&au).filt(|x: A| x.1 .1.map_or("None", |c| c.1) != "None").group_by(Same::<A>::new().map(|x: A| x.0)).select(Same::<A>::new()).fold(0i64, |s, x| s + x.1 .0 .1);
    let v = drain((&au).select(Same::<A>::new().and(Same::<A>::new().map(|x: A| x.0).select((&tp).opt()))));
    rows(v.into_iter().map(|(_, ((u, ((_, n, k), c)), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        let cc = c.map_or(0, |c| c.0);
        f.extend([V::I(n), V::I(cc), V::S(c.map_or("None", |c| c.1))]);
        f.push(V::S(if cc > 0 { "Active in Discussions" } else { "Lurker" }));
        f.extend([V::I(t.unwrap_or(0)), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.Reputation) AS TotalReputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RecentPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, ur.DisplayName, ur.TotalReputation, ur.BadgeCount, COALESCE(COUNT(v.Id), 0) AS VoteCount
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN Votes v ON rp.PostId = v.PostId AND v.VoteTypeId IN (2, 3) WHERE rp.rn = 1
//     GROUP BY rp.PostId, rp.Title, rp.CreationDate, ur.DisplayName, ur.TotalReputation, ur.BadgeCount),
// PostHistoryWithReasons AS (SELECT ph.PostId, STRING_AGG(DISTINCT pht.Name, ', ') AS HistoryTypes, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons
//     FROM PostHistory ph LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id LEFT JOIN CloseReasonTypes cr ON ph.Comment = cr.Id::text
//     WHERE ph.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.DisplayName AS OwnerDisplayName, rp.TotalReputation, rp.BadgeCount, rp.VoteCount, phwr.HistoryTypes, phwr.CloseReasons
// FROM RecentPosts rp LEFT JOIN PostHistoryWithReasons phwr ON rp.PostId = phwr.PostId ORDER BY rp.CreationDate DESC LIMIT 50 OFFSET 0;
//
// rn reads only base columns, so each owner's newest post is picked first. The DISTINCT names are listed sorted.
fn q31981(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, .. } = &db.post;
    let cd = current_date();
    let w = db.post.with(creation_date.ge(add_years(cd, -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(badges_of(db).opt())).fold([0i64; 2], |a, (r, b)| [a[0] + r, a[1] + b.is_some() as i64]);
    let t = &db.vote.vote_type_id;
    let vc = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(t.eq(2).or(t.eq(3)))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let crt: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i: i64| leak(i.to_string())).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { creation_date: hd, comment, .. } = &db.post_history;
    let ph = db.post_history.with(hd.ge(add_months(cd, -6))).group_by(&db.post_history.post).select(htype_name(db).and(comment.select(&crt).opt())).buf_fold(|v| {
        let mut a: Vec<Str> = v.iter().map(|x| x.0).collect();
        let mut b: Vec<Str> = v.iter().flat_map(|x| x.1).collect();
        a.sort_unstable();
        a.dedup();
        b.sort_unstable();
        b.dedup();
        (agg_str(a, ", "), agg_str(b, ", "))
    });
    let v = drain((&rp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ur))).and(&vc).and((&ph).opt())));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(_, (((p, (u, a)), n), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        match h {
            Some((x, y)) => f.extend([ostr(x), ostr(y)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, (SELECT COUNT(*) FROM Posts P WHERE P.OwnerUserId = U.Id) AS PostCount,
//        (SELECT COUNT(*) FROM Comments C WHERE C.UserId = U.Id) AS CommentCount, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, COUNT(C.Id) AS TotalComments FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') AND P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, P.Score),
// TopUsers AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation FROM UserReputation UR WHERE UR.Reputation > 1000 AND UR.ReputationRank <= 10),
// PostDetails AS (SELECT RP.PostId, RP.Title, RP.CreationDate, UR.DisplayName AS OwnerName, RP.Score, RP.TotalComments FROM RecentPosts RP JOIN UserReputation UR ON RP.OwnerUserId = UR.UserId)
// SELECT PD.Title, PD.CreationDate, PD.OwnerName, PD.Score, PD.TotalComments,
//        CASE WHEN PD.TotalComments = 0 THEN 'No Comments' WHEN PD.TotalComments < 5 THEN 'Few Comments' ELSE 'Many Comments' END AS CommentLevel,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = PD.PostId AND V.VoteTypeId = 2) AS UpVotes, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = PD.PostId AND V.VoteTypeId = 3) AS DownVotes,
//        COALESCE((SELECT STRING_AGG(T.TagName, ', ') FROM Tags T JOIN Posts P ON T.ExcerptPostId = P.Id WHERE P.Id = PD.PostId), 'No Tags') AS Tags
// FROM PostDetails PD JOIN TopUsers TU ON PD.OwnerName = TU.DisplayName ORDER BY PD.CreationDate DESC LIMIT 50;
//
// The join to TopUsers is on the display name, so it goes through a name index. Tag names are in tag id order.
fn q2951(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|((_, rp), k)| rp > 1000 && k <= 10).map(|((u, _), _)| u).collect();
    let byname: HashIdx<Str, Id<User>> = (&tu).select(&db.user.display_name).inv().collect();
    let rp = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(post_type_id.eq(1));
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = rp().group_by(Ident::<Post>::new()).select((&ex).select(&db.tag.tag_name)).buf_fold(|v| leak(v.join(", ")));
    let v = drain((&cc).and(owner_user.select(&db.user.display_name).select(&byname)).and(&vc).and((&tg).opt()));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(p, (((c, _), a), t))| {
        let mut f = post_fields(db, p, &["title", "created", "owner", "score"]);
        f.push(V::I(c));
        f.push(V::S(if c == 0 { "No Comments" } else if c < 5 { "Few Comments" } else { "Many Comments" }));
        f.extend([V::I(a[0]), V::I(a[1]), V::S(t.unwrap_or("No Tags"))]);
        row(f)
    }))
}

// Rewritten (rewrites/23429.sql): the original, with the close reasons' STRING_AGG given `ORDER BY cr.Name` and the rn window `, p.Id DESC`:
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostConnections AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS RelatedPostCount FROM PostLinks pl GROUP BY pl.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(DISTINCT ph.Id) AS CloseCount, STRING_AGG(DISTINCT cr.Name, ', ' ORDER BY cr.Name) AS CloseReasons FROM PostHistory ph
//     INNER JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT up.UserId, up.Reputation, up.BadgeCount, rp.PostId, rp.Title, rp.CreationDate, COALESCE(pc.RelatedPostCount, 0) AS RelatedPostCount, COALESCE(cp.CloseCount, 0) AS CloseCount,
//        COALESCE(cp.CloseReasons, 'No close reasons') AS CloseReasons,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId AND c.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month') AS RecentComments
// FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN PostConnections pc ON rp.PostId = pc.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE up.Reputation > (SELECT AVG(Reputation) FROM UserReputation) AND rp.rn = 1 ORDER BY up.Reputation DESC, rp.Score DESC LIMIT 10;
//
// rn reads only base columns, so each owner's newest post is picked first. The average is a whole-table fold, compared exactly as sum over count.
fn q23429(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, post_type_id, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).or(post_type_id.eq(2)))
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), Reverse(p)), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let (s, n) = (&db.user.reputation).fold_flat((0i128, 0i128), |(s, n), r| (s + r as i128, n + 1));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pc = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id: t, comment, .. } = &db.post_history;
    let cp = db.post_history.with(t.eq(10).or(t.eq(11))).group_by(&db.post_history.post).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)).buf_fold(|v| {
        let mut t: Vec<Str> = v.iter().copied().collect();
        let n = t.len() as i64;
        t.sort_unstable();
        t.dedup();
        (n, leak(t.join(", ")))
    });
    let Comment { creation_date: cdt, .. } = &db.comment;
    let rc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().with(cdt.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)))).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hi = Ident::<User>::new().with((&db.user.reputation).filt(move |r| r as i128 * n > s));
    let v = drain((&rp).select(Ident::<Post>::new().and(owner_user.select(hi.and(&bc))).and((&pc).opt()).and((&cp).opt()).and(&rc)));
    let v = top_n(v, |&(_, ((((p, (u, _)), _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(_, ((((p, (u, b)), l), c), k))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(l.unwrap_or(0)), V::I(c.map_or(0, |c| c.0)), V::S(c.map_or("No close reasons", |c| c.1)), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, u.DisplayName AS AuthorName, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts AS p JOIN Users AS u ON p.OwnerUserId = u.Id LEFT JOIN Comments AS c ON p.Id = c.PostId LEFT JOIN Votes AS v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.Body),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Body, rp.AuthorName, rp.CommentCount, rp.UpVotes FROM RankedPosts AS rp WHERE rp.Rank = 1),
// MostDiscussedPosts AS (SELECT fp.PostId, fp.Title, fp.AuthorName, fp.CommentCount, fp.UpVotes FROM FilteredPosts AS fp WHERE fp.CommentCount > 10 ORDER BY fp.CommentCount DESC LIMIT 10),
// PostHistoryDetails AS (SELECT p.Id AS PostId, ph.CreationDate AS HistoryDate, pht.Name AS HistoryType, ph.Comment FROM PostHistory AS ph JOIN Posts AS p ON ph.PostId = p.Id
//     JOIN PostHistoryTypes AS pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.CreationDate BETWEEN (CAST('2024-10-01' AS DATE) - INTERVAL '30 days') AND CAST('2024-10-01' AS DATE))
// SELECT mdp.PostId, mdp.Title, mdp.AuthorName, mdp.CommentCount, mdp.UpVotes,
//        (SELECT STRING_AGG(HistoryType || ': ' || CAST(HistoryDate AS TEXT), '; ') FROM PostHistoryDetails WHERE PostId = mdp.PostId) AS HistoryDetails
// FROM MostDiscussedPosts AS mdp ORDER BY mdp.CommentCount DESC;
//
// Rank partitions by the post, so it is always 1. History entries are in history id order.
fn q28157(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let e = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let top = top_n(drain((&e).filt(|a| a[0] > 10)), |&(p, a)| (Reverse(a[0]), p), 10);
    let tp = rel(top);
    type T = (Id<Post>, [i64; 2]);
    let hd = &db.post_history.creation_date;
    let hx = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_days(date(2024, 10, 1), -30))).with(hd.le(date(2024, 10, 1))));
    let tps: MatSet<Id<Post>> = (&tp).map(|x: T| x.0).collect();
    let det = (&tps).group_by(Ident::<Post>::new()).select(hx.select(htype_name(db).and(hd))).buf_fold(|v| leak(v.iter().map(|&(n, d)| format!("{n}: {}", ts_text(d))).collect::<Vec<_>>().join("; ")));
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select((&det).opt()))));
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), ostr(h)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.Score, p.CreationDate, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank, p.Tags
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(pl.PostId) AS RelatedPostsCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostLinks pl ON p.Id = pl.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// FrequentTags AS (SELECT UNNEST(string_to_array(p.Tags, ',')) AS TagName FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year'),
// TagCounts AS (SELECT TagName, COUNT(*) AS TagUsageCount FROM FrequentTags GROUP BY TagName),
// TopTags AS (SELECT TagName FROM TagCounts ORDER BY TagUsageCount DESC LIMIT 10)
// SELECT ua.DisplayName, ua.UserId, rp.Title, rp.Score, rp.CreationDate, ta.TagName, ua.RelatedPostsCount, ua.UpVotesCount, ua.DownVotesCount,
//        (CASE WHEN rp.PostTypeId = 1 THEN 'Question' WHEN rp.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END) AS PostType, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount
// FROM RankedPosts rp JOIN UserActivity ua ON ua.UserId = rp.PostId JOIN TopTags ta ON ta.TagName = ANY (string_to_array(rp.Tags, ','))
// WHERE rp.ScoreRank <= 5 ORDER BY rp.Score DESC, ua.UpVotesCount DESC;
//
// The Tags text has no commas, so splitting it on ',' gives the whole string. `ua.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids,
// and the posts x links x votes product is driven for the users it reaches. A tie at the TopTags LIMIT goes to the smaller string.
fn q32846(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, tags_str, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let split = |s: Str| s.split(',');
    let tc = recent().group_by(tags_str.flat_map(split)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10);
    let tts: MatSet<Str> = rel(tt.into_iter().map(|x| x.0).collect()).map(|t| t).collect();
    let w = recent().group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let us: MatSet<Id<User>> = (&rp).select((&db.post.origid).select(&uidx)).collect();
    let ua = (&us)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(links_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, x| {
            let (l, t) = x.map_or((None, None), |x| x);
            [a[0] + l.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
        });
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&rp).select(Ident::<Post>::new().and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ua))).and(tags_str.flat_map(split).select(Same::<Str>::new().with(&tts))).and(&cc)));
    let mut v = v;
    v.sort_by_key(|&(_, (((p, (_, a)), _), _))| (Reverse(score.get(p).unwrap()), Reverse(a[1])));
    rows(v.into_iter().map(|(_, (((p, (u, a)), t), c))| {
        let mut f = ucols(db, u, &["name", "uid"]);
        f.extend(post_fields(db, p, &["title", "score", "created"]));
        f.extend([V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        let k = post_type_id.get(p).unwrap();
        f.push(V::S(if k == 1 { "Question" } else if k == 2 { "Answer" } else { "Other" }));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH RecursiveUserVotes AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId IN (2, 8) THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, COALESCE(P.AnswerCount, 0) AS AnswerCount, COALESCE(P.CommentCount, 0) AS CommentCount, COALESCE(P.FavoriteCount, 0) AS FavoriteCount,
//        P.CreationDate, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.ViewCount DESC) AS ViewRank, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.ViewCount, PS.AnswerCount, PS.CommentCount, PS.FavoriteCount, PS.CreationDate, U.DisplayName AS OwnerDisplayName
//     FROM PostStatistics PS JOIN Users U ON PS.PostId = U.Id WHERE PS.ViewRank <= 10 OR PS.ScoreRank <= 10),
// RecentBadgers AS (SELECT B.UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Badges B JOIN Users U ON B.UserId = U.Id
//     WHERE B.Date >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 MONTH' GROUP BY B.UserId, U.DisplayName)
// SELECT PS.Title, PS.ViewCount, PS.AnswerCount, PS.CommentCount, PS.FavoriteCount, RUV.TotalVotes, RUV.UpVotes, RUV.DownVotes, RB.BadgeCount, RB.BadgeNames
// FROM TopPosts PS LEFT JOIN RecursiveUserVotes RUV ON PS.OwnerDisplayName = RUV.DisplayName LEFT JOIN RecentBadgers RB ON RB.UserId = RUV.UserId ORDER BY PS.ViewCount DESC, RUV.TotalVotes DESC;
//
// Nothing recurses. `PS.PostId = U.Id` joins a post id to a user id, so it goes through the raw ids; RUV is joined through a display-name index. Badge names are in badge id order.
fn q30947(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(view_count.opt()).and(score))
        .window(rank, |((_, w), _)| (w.is_none(), Reverse(w)), asc);
    let w = (&w).window(rank, |(((_, _), s), _): (((Id<Post>, Option<i64>), i64), i64)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|((_, a), b)| a <= 10 || b <= 10).map(|((((p, _), _), _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ruv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2) || t == Some(8)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let byname: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let rb = db.badge.with((&db.badge.date).ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let v = drain((&tp).select(Ident::<Post>::new().and((&db.post.origid).select(&uidx).select(&db.user.display_name).select(byname.select(Ident::<User>::new().and(&ruv).and((&rb).opt())).opt()))));
    let mut v = v;
    v.sort_by_key(|&(_, (p, u))| {
        let t = u.map(|x| x.0 .1[0]);
        (view_count.get(p).is_none(), Reverse(view_count.get(p)), t.is_none(), Reverse(t))
    });
    rows(v.into_iter().map(|(_, (p, u))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::I(db.post.answer_count.get(p).unwrap_or(0)), V::I(db.post.comment_count.get(p).unwrap()), V::I(db.post.favorite_count.get(p).unwrap_or(0))]);
        match u {
            Some(((_, a), b)) => {
                f.extend(a.map(V::I));
                f.extend([oint(b.map(|b| b.0)), ostr(b.map(|b| b.1))]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// Rewritten (rewrites/32482.sql): the original, with the rn window refined with `, p.Id DESC`, UserRank with `, UserId`, and the change types' STRING_AGG given `ORDER BY pht.Name`:
// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id DESC) as rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserScores AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN voteType.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN voteType.VoteTypeId = 3 THEN 1 ELSE 0 END) AS ReputationScore,
//        COUNT(DISTINCT bh.Id) AS BadgeCount FROM Users u LEFT JOIN Votes voteType ON u.Id = voteType.UserId LEFT JOIN Badges bh ON u.Id = bh.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, ReputationScore, BadgeCount, ROW_NUMBER() OVER (ORDER BY ReputationScore DESC, UserId) AS UserRank FROM UserScores),
// PostHistoryAnalysis AS (SELECT ph.PostId, ph.UserId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate, STRING_AGG(DISTINCT pht.Name, ', ' ORDER BY pht.Name) AS ChangeTypes
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId, ph.UserId)
// SELECT rp.Title AS RecentPostTitle, rp.CreationDate AS PostCreationDate, rp.Score AS PostScore, rp.ViewCount AS PostViewCount, rp.AnswerCount AS NumberOfAnswers, tu.DisplayName AS TopUserDisplayName,
//        tu.ReputationScore AS UserReputationScore, ph.LastEditDate AS LastEditDate, ph.EditCount AS EditCount, ph.ChangeTypes AS TypesOfChanges
// FROM RecentPosts rp LEFT JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId LEFT JOIN PostHistoryAnalysis ph ON rp.Id = ph.PostId
// WHERE rp.rn = 1 AND tu.UserRank <= 10 ORDER BY rp.CreationDate DESC;
//
// rn reads only base columns, so each owner's newest post is picked first. PostHistoryAnalysis is keyed by (post, user), so its groups are re-keyed by post to be joined.
fn q32482(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt())).fold(0i64, |s, (t, _)| s + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let top = top_n(drain(&us), |&(u, s)| (Reverse(s), db.user.origid.get(u).unwrap()), 10);
    let tu = rel(top);
    type T = (Id<User>, i64);
    let tidx: HashIdx<Id<User>, T> = (&tu).map(|x: T| x.0).inv().select(&tu).collect();
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), Reverse(p)), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let PostHistory { post, user_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.group_by(post.and(user_id.opt())).select(htype_name(db).and(hd)).buf_fold(|v| {
        let mut t: Vec<Str> = v.iter().map(|x| x.0).collect();
        t.sort_unstable();
        t.dedup();
        (v.len() as i64, v.iter().map(|x| x.1).max().unwrap(), leak(t.join(", ")))
    });
    let pr = rel(drain(&ph));
    type H = ((Id<Post>, Option<i64>), (i64, i64, Str));
    let pidx: HashIdx<Id<Post>, H> = (&pr).map(|x: H| x.0 .0).inv().select(&pr).collect();
    let v = drain((&rp).select(Ident::<Post>::new().and(owner_user.select(&tidx)).and((&pidx).opt())));
    let mut v = v;
    v.sort_by_key(|&(_, ((p, _), _))| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(_, ((p, (u, s)), h))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers"]);
        f.extend(ucols(db, u, &["name"]));
        f.push(V::I(s));
        match h {
            Some((_, (n, d, t))) => f.extend([V::T(d), V::I(n), V::S(t)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.ViewCount > 100),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RecentCloseReasons AS (SELECT p.Id AS PostId, MAX(ph.CreationDate) AS LastClosed, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM PostHistory ph
//     JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id JOIN Posts p ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY p.Id),
// AggregatePostStats AS (SELECT p.Id AS PostId, AVG(COALESCE(c.Score, 0)) AS AverageCommentScore FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT rp.Id, rp.Title, rp.CreationDate, u.DisplayName, us.UpVotes, us.DownVotes, us.BadgeCount, rcr.CloseReasons, aps.AverageCommentScore
// FROM RankedPosts rp INNER JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserStats us ON u.Id = us.UserId LEFT JOIN RecentCloseReasons rcr ON rp.Id = rcr.PostId
// LEFT JOIN AggregatePostStats aps ON rp.Id = aps.PostId
// WHERE (rp.PostTypeId = 1 AND rp.rn <= 5) OR (rp.PostTypeId = 2 AND u.Reputation >= 100) ORDER BY COALESCE(rcr.LastClosed, '1900-01-01') DESC, rp.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// The DISTINCT close reasons are listed sorted.
fn q24381(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, owner_user, .. } = &db.post;
    let w = db.post.with(view_count.gt(100)).group_by(post_type_id).select(Ident::<Post>::new().and(post_type_id).and(creation_date)).window(row_number, |((p, _), d)| (Reverse(d), p), asc);
    type R = (Id<Post>, i64, i64);
    let rr: MatSet<R> = (&w).map(|(((p, t), _), k)| (p, t, k)).collect();
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id: t, comment, creation_date: hd, .. } = &db.post_history;
    let rcr = db
        .post_history
        .with(t.eq(10).or(t.eq(11)))
        .group_by(&db.post_history.post)
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|v| {
            let mut n: Vec<Str> = v.iter().map(|x| x.1).collect();
            n.sort_unstable();
            n.dedup();
            (v.iter().map(|x| x.0).max().unwrap(), leak(n.join(", ")))
        });
    let aps = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold((0i64, 0i64), |(s, n), c| (s + c.unwrap_or(0), n + 1));
    let v = drain(
        (&rr)
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(owner_user.select(Ident::<User>::new().and(&db.user.reputation).and(&us).and(&bc)).and((&rcr).opt()).and(&aps))))
            .filt(|((_, t, k), (((((_, r), _), _), _), _)): (R, (((((Id<User>, i64), [i64; 2]), i64), Option<(i64, Str)>), (i64, i64)))| (t == 1 && k <= 5) || (t == 2 && r >= 100)),
    );
    let v = top_n(v, |&(_, ((p, _, _), ((_, c), _)))| (Reverse(c.map_or(ts(1900, 1, 1, 0, 0, 0), |c| c.0)), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(_, ((p, _, _), (((((u, _), a), b), c), (s, n))))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b), ostr(c.map(|c| c.1)), V::F(s as f64 / n as f64)]);
        row(f)
    }))
}

// WITH RecursiveTagStats AS (SELECT t.Id AS TagId, t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, ROW_NUMBER() OVER (PARTITION BY t.Id ORDER BY COUNT(p.Id) DESC) AS Rank
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY t.Id, t.TagName),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS AuthorName, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// ClosedPostStats AS (SELECT ph.PostId, ph.CreationDate, COUNT(ph.Id) AS CloseCount, STRING_AGG(pr.Name, ', ') AS CloseReasons FROM PostHistory ph
//     JOIN CloseReasonTypes pr ON pr.Id = CAST(ph.Comment AS INTEGER) WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate)
// SELECT rtag.TagName, rtag.PostCount, rtag.TotalUpvotes, rtag.TotalDownvotes, COALESCE(cp.CloseCount, 0) AS NumberOfClosures, COALESCE(cp.CloseReasons, 'No Reasons') AS ClosureReasons,
//        rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate, rp.AuthorName AS RecentPostAuthor
// FROM RecursiveTagStats rtag LEFT JOIN ClosedPostStats cp ON rtag.TagId = cp.PostId LEFT JOIN RecentPosts rp ON rp.PostId = (SELECT PostId FROM RecentPosts WHERE RecentPostRank = 1)
// WHERE rtag.Rank <= 5 ORDER BY rtag.TotalUpvotes DESC, rtag.PostCount DESC;
//
// Nothing recurses, and Rank partitions by the tag, so it is always 1. The WHERE on p makes the LIKE join inner; it is tag_mentions. `rtag.TagId = cp.PostId`
// joins a tag id to a post id, so it goes through the raw ids. The ON of rp compares with an uncorrelated scalar, so it is a cross join with that one post.
fn q32224(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let tm = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    let ts_ = (&tm)
        .with(Same::<M>::new().map(|x: M| x.0).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(Same::<M>::new().map(|x: M| x.1))
        .select(Same::<M>::new().map(|x: M| x.0).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, post, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post.and(hd))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let cr = rel(drain(&cp));
    type C = ((Id<Post>, i64), (i64, Str));
    let cidx: HashIdx<Id<Post>, C> = (&cr).map(|x: C| x.0 .0).inv().select(&cr).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rp = top_n(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user)), |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rpi: HashIdx<(), Id<Post>> = whole(&rp).collect();
    let v = drain((&ts_).and((&db.tag.origid).select(&pidx).select(&cidx).opt()).and(Ident::<Tag>::new().map(|_| ()).select(&rpi).opt()));
    let mut v = v;
    v.sort_by_key(|&(_, ((a, _), _))| (Reverse(a[1]), Reverse(a[0])));
    rows(v.into_iter().map(|(t, ((a, c), p))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend([V::I(c.map_or(0, |c| c.1 .0)), V::S(c.map_or("No Reasons", |c| c.1 .1))]);
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "created", "owner"])),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Body, P.AnswerCount, P.Score, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS Rank
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(V.BountyAmount) AS TotalBounty FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId
//     LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostHistoryAnalysis AS (SELECT PH.PostId, COUNT(*) AS EditCount, ARRAY_AGG(DISTINCT PH.PostHistoryTypeId) AS EditTypes FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5, 6) GROUP BY PH.PostId),
// RecentClosures AS (SELECT P.Id AS PostId, PH.UserId AS CloserUserId, COUNT(*) AS CloseCount FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE PH.PostHistoryTypeId = 10 AND PH.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY P.Id, PH.UserId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Body, RP.AnswerCount, RP.Score, US.DisplayName AS UserName, US.Reputation, US.BadgeCount, US.TotalBounty, PH.EditCount,
//        COALESCE(ARRAY_TO_STRING(PH.EditTypes, ', '), 'No Edits') AS EditTypes, RC.CloserUserId AS ClosingUserId, COALESCE(RC.CloseCount, 0) AS CloseCount
// FROM RankedPosts RP JOIN UserStats US ON RP.PostId = (SELECT AcceptedAnswerId FROM Posts WHERE Id = RP.PostId) LEFT JOIN PostHistoryAnalysis PH ON RP.PostId = PH.PostId
// LEFT JOIN RecentClosures RC ON RP.PostId = RC.PostId WHERE RP.Rank <= 5 AND (COALESCE(RC.CloseCount, 0) = 0 OR US.Reputation > 1000)
// ORDER BY RP.Score DESC, RP.CreationDate DESC LIMIT 10;
//
// The ON never names US: it keeps the ranked posts whose AcceptedAnswerId is their own id and crosses them with every user. Rank ties go to the smaller post id.
fn q20758(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, accepted_answer_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let PostHistory { post_history_type_id: t, creation_date: hd, user_id, post, .. } = &db.post_history;
    let pha = db.post_history.with(t.eq(4).or(t.eq(5)).or(t.eq(6))).group_by(post).select(t).buf_fold(|v| {
        let mut x: Vec<i64> = v.iter().copied().collect();
        let n = x.len() as i64;
        x.sort_unstable();
        x.dedup();
        (n, leak(x.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ")))
    });
    let rc = db.post_history.with(t.eq(10)).with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(post.and(user_id.opt())).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rcr = rel(drain(&rc));
    type C = ((Id<Post>, Option<i64>), i64);
    let cidx: HashIdx<Id<Post>, C> = (&rcr).map(|x: C| x.0 .0).inv().select(&rcr).collect();
    let ra = (&rp).select(Ident::<Post>::new().with(origid.and(accepted_answer_id).filt(|(a, b)| a == b)).and((&pha).opt()).and((&cidx).opt()));
    let us = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold((0i64, 0i64), |(s, n), (_, b)| {
        let b = b.flatten();
        (s + b.unwrap_or(0), n + b.is_some() as i64)
    });
    let bd = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    type A = ((Id<Post>, Option<(i64, Str)>), Option<C>);
    type U = (((Id<User>, i64), (i64, i64)), i64);
    let v = drain(ra.cross(db.user.select(Ident::<User>::new().and(&db.user.reputation).and(&us).and(&bd))).filt(|(x, y): (A, U)| x.1.map_or(0, |c| c.1) == 0 || y.0 .0 .1 > 1000));
    let v = top_n(v, |&(_, (((p, _), _), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(_, (((p, h), c), (((u, _), (s, n)), b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "body", "answers", "score"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), nullable(s, n), oint(h.map(|h| h.0)), V::S(h.map_or("No Edits", |h| h.1)), oint(c.and_then(|c| c.0 .1)), V::I(c.map_or(0, |c| c.1))]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN VT.Name = 'Favorite' THEN 1 ELSE 0 END) AS Favorites
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN VoteTypes VT ON V.VoteTypeId = VT.Id GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostTagStats AS (SELECT P.Id AS PostId, P.Title, P.AcceptedAnswerId, ARRAY_AGG(DISTINCT TRIM(T.TagName)) AS Tags, P.CreationDate, P.ViewCount, P.AnswerCount, P.Score
//     FROM Posts P LEFT JOIN UNNEST(string_to_array(P.Tags, '>')) AS TagTag(TagName) ON true LEFT JOIN Tags T ON T.TagName = TRIM(TagTag.TagName)
//     GROUP BY P.Id, P.Title, P.AcceptedAnswerId, P.CreationDate, P.ViewCount, P.AnswerCount, P.Score),
// RecentActivity AS (SELECT P.Id AS PostId, MAX(CA.CreationDate) AS LastActivityDate, COUNT(CM.Id) AS CommentCount, COUNT(PH.Id) AS HistoryCount
//     FROM Posts P LEFT JOIN Comments CM ON P.Id = CM.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN Posts CA ON P.AcceptedAnswerId = CA.Id
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY P.Id)
// SELECT U.DisplayName AS User, U.Reputation, U.TotalVotes, U.UpVotes, U.DownVotes, U.Favorites, P.Title, P.Tags, P.ViewCount, P.AnswerCount, P.Score, R.LastActivityDate, R.CommentCount, R.HistoryCount
// FROM UserVoteStats U JOIN PostTagStats P ON U.UserId = P.AcceptedAnswerId JOIN RecentActivity R ON P.PostId = R.PostId
// ORDER BY U.Reputation DESC, P.ViewCount DESC, R.LastActivityDate DESC LIMIT 100;
//
// `U.UserId = P.AcceptedAnswerId` joins a user id to a post id, so it goes through the raw ids. The Tags elements split on '>' are matched against Tags by name; the
// DISTINCT list is sorted, NULL first.
fn q28038(db: &'static So) -> String {
    let Post { creation_date, accepted_answer_id, accepted_answer, tags_str, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let ra = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).opt())).fold([0i64; 2], |a, (c, h)| [a[0] + c.is_some() as i64, a[1] + h.is_some() as i64]);
    let tn: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tname = tn.select(&db.tag.tag_name);
    let tags = recent()
        .group_by(Ident::<Post>::new())
        .select(tags_str.flat_map(|s: Str| s.split('>')).select(Same::<Str>::new().map(|e: Str| e.trim()).select(&tname).opt()).opt())
        .buf_fold(|v| {
            let mut x: Vec<Option<Str>> = v.iter().map(|e| e.flatten().map(|t| t.trim())).collect();
            x.sort_unstable();
            x.dedup();
            &*Box::leak(x.into_boxed_slice())
        });
    let byacc: HashIdx<i64, Id<Post>> = accepted_answer_id.inv().collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vtype_name(db)).opt()).fold([0i64; 4], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some("UpMod")) as i64, a[2] + (t == Some("DownMod")) as i64, a[3] + (t == Some("Favorite")) as i64]
    });
    let v = drain((&uv).and((&db.user.origid).select(&byacc).select(Ident::<Post>::new().and(&ra).and(&tags).and(accepted_answer.select(creation_date).opt()))));
    let v = top_n(v, |&(u, (_, (((p, _), _), d)))| {
        let w = view_count.get(p);
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), d.is_none(), Reverse(d))
    }, 100);
    rows(v.into_iter().map(|(u, (a, (((p, r), t), d)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title"]));
        f.push(V::L(t.iter().map(|&x| ostr(x)).collect()));
        f.extend(post_fields(db, p, &["views", "answers", "score"]));
        f.extend([ots(d), V::I(r[0]), V::I(r[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) DESC) AS Ranking
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, PostCount, CommentCount FROM UserActivity WHERE Ranking <= 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS Author, COALESCE(pb.Body, 'No Body') AS PostBody, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS TotalComments,
//        COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN PostLinks pl ON p.Id = pl.PostId
//     LEFT JOIN (SELECT PostId, STRING_AGG(Text, ' ') AS Body FROM PostHistory WHERE PostHistoryTypeId = 2 GROUP BY PostId) pb ON p.Id = pb.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, pb.Body)
// SELECT tu.DisplayName AS TopUser, tu.UpVotes, tu.DownVotes, pd.Title AS PostTitle, pd.CreationDate AS PostDate, pd.PostBody, pd.TotalComments, pd.RelatedPostsCount
// FROM TopUsers tu JOIN Posts p ON tu.UserId = p.OwnerUserId JOIN PostDetails pd ON p.Id = pd.PostId ORDER BY tu.UpVotes DESC, pd.CreationDate DESC;
//
// The initial bodies of a post are joined in history id order.
fn q1702(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.and_then(|x| x.0);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let top = top_n(drain(&ua), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu = rel(top);
    type T = (Id<User>, [i64; 2]);
    let us: MatSet<Id<User>> = (&tu).map(|x: T| x.0).collect();
    let PostHistory { post_history_type_id, text, .. } = &db.post_history;
    let pb = db.post_history.with(post_history_type_id.eq(2)).group_by(&db.post_history.post).select(text.opt()).buf_fold(|v| agg_str(v.iter().flatten().copied(), " "));
    let ps = || (&us).select(posts_of(db));
    let pset: MatSet<Id<Post>> = ps().collect();
    let cc = (&pset).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rl = (&pset).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select(posts_of(db).select(Ident::<Post>::new().and((&pb).opt()).and(&cc).and((&rl).opt()))))));
    let mut v = v;
    v.sort_by_key(|&(_, ((_, a), (((p, _), _), _)))| (Reverse(a[0]), Reverse(db.post.creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((u, a), (((p, b), c), l)))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::S(b.flatten().unwrap_or("No Body")), V::I(c), V::I(l.unwrap_or(0))]);
        row(f)
    }))
}

// Rewritten (rewrites/30299.sql): the original, with the gold badge names' STRING_AGG given `ORDER BY Id`:
// WITH RECURSIVE UserReputationCTE AS (SELECT Id, Reputation, CreationDate, LastAccessDate, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, u.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(ph.Id) AS HistoryCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId),
// UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, MAX(Date) AS LastBadgeDate, STRING_AGG(Name, ', ' ORDER BY Id) AS BadgeNames FROM Badges WHERE Class = 1 GROUP BY UserId)
// SELECT us.Id AS UserId, us.DisplayName, us.Reputation, COALESCE(u.BadgeCount, 0) AS GoldBadgeCount, COALESCE(u.BadgeNames, '') AS GoldBadges, ps.PostId, ps.Title, ps.Score, ps.ViewCount,
//        ps.CommentCount, ps.TotalUpvotes, ps.TotalDownvotes, COALESCE(ph.HistoryCount, 0) AS EditHistoryCount, ph.LastEditDate, DENSE_RANK() OVER (ORDER BY us.Reputation DESC) AS UserRank
// FROM Users us LEFT JOIN UserBadges u ON us.Id = u.UserId LEFT JOIN PostStats ps ON us.DisplayName = ps.OwnerDisplayName LEFT JOIN PostHistoryStats ph ON ps.PostId = ph.PostId
// WHERE us.CreationDate > '2020-01-01' AND (u.BadgeCount IS NULL OR u.BadgeCount > 0) ORDER BY UserRank, ps.ViewCount DESC LIMIT 100;
//
// Nothing recurses. PostStats is joined on the owner's display name, through a name index. The order reads only Reputation and ViewCount, so the rows are
// ranked and cut before the comment x vote product is driven. Badge names are in badge id order.
fn q30299(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let byname: HashIdx<Str, Id<Post>> = db.post.with(post_type_id.eq(1)).select(owner_user.select(&db.user.display_name)).inv().collect();
    let ub = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    type J = (Id<User>, Option<Id<Post>>);
    let jm: MatSet<J> = db.user.with((&db.user.creation_date).gt(ts(2020, 1, 1, 0, 0, 0))).select(Ident::<User>::new().and((&db.user.display_name).select(&byname).opt())).collect();
    let w = whole(&jm).select(Same::<J>::new().and(Same::<J>::new().map(|x: J| x.0).select(&db.user.reputation))).window(dense_rank, |(_, r)| Reverse(r), asc);
    let r = top_n(drain(&w), |&(_, (((_, p), r), _))| {
        let w = p.and_then(|p| view_count.get(p));
        (Reverse(r), w.is_none(), Reverse(w))
    }, 100);
    type R = (Id<User>, Option<Id<Post>>, i64);
    let rr = rel(r.into_iter().map(|(_, (((u, p), _), k))| (u, p, k)).collect());
    let ps: MatSet<Id<Post>> = (&rr).flat_map(|x: R| x.1).collect();
    let st = (&ps).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let t = &db.post_history.post_history_type_id;
    let ph = db.post_history.with(t.eq(4).or(t.eq(5)).or(t.eq(6))).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |a, d| (a.0 + 1, a.1.max(d)));
    let v = drain((&rr).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&ub).opt())).and(Same::<R>::new().flat_map(|x: R| x.1).select(Ident::<Post>::new().and(&st).and((&ph).opt())).opt())));
    rows(v.into_iter().map(|(_, (((u, _, k), b), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b.map_or(0, |b| b.0)), V::S(b.map_or("", |b| b.1))]);
        match p {
            Some(((p, a), h)) => {
                f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
                f.extend(a.map(V::I));
                f.extend([V::I(h.map_or(0, |h| h.0)), ots(h.map(|h| h.1))]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::I(0), V::Null]),
        }
        f.push(V::I(k));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS TagRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.Tags),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.TagRank FROM RankedPosts rp WHERE rp.TagRank <= 5),
// ActivityHistory AS (SELECT ph.PostId, ph.UserDisplayName, ph.CreationDate, ph.Comment, ph.PostHistoryTypeId, STRING_AGG(DISTINCT pht.Name, ', ') AS HistoryTypeNames
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
//     GROUP BY ph.PostId, ph.UserDisplayName, ph.CreationDate, ph.Comment, ph.PostHistoryTypeId)
// SELECT tp.PostId, tp.Title, tp.Body, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes, ah.UserDisplayName AS EditorDisplayName,
//        ah.CreationDate AS EditDate, ah.Comment AS EditComment, ah.HistoryTypeNames
// FROM TopPosts tp LEFT JOIN ActivityHistory ah ON tp.PostId = ah.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// TagRank reads only base columns, so the questions are ranked first and the comment x vote product is driven for the survivors. ActivityHistory's groups
// are re-keyed by post to be joined. Rank ties go to the smaller post id.
fn q27474(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(tags_str.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let e = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let PostHistory { post, user_display_name, creation_date: hd, comment, post_history_type_id, .. } = &db.post_history;
    let ah = db
        .post_history
        .with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post.and(user_display_name.opt()).and(hd).and(comment.opt()).and(post_history_type_id))
        .select(htype_name(db))
        .buf_fold(|v| v[0]);
    let ar = rel(drain(&ah));
    type A = ((((Id<Post>, Option<Str>), i64), Option<Str>), i64);
    type H = (A, Str);
    let aidx: HashIdx<Id<Post>, H> = (&ar).map(|x: H| x.0 .0 .0 .0 .0).inv().select(&ar).collect();
    let v = drain((&e).and((&aidx).opt()));
    let mut v = v;
    v.sort_by_key(|&(p, _)| {
        let w = db.post.view_count.get(p);
        (Reverse(db.post.score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        match h {
            Some(((((( _, n), d), c), _), t)) => f.extend([ostr(n), V::T(d), ostr(c), V::S(t)]),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.LastActivityDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS Rank FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.Score > 0),
// PopularTags AS (SELECT T.TagName, COUNT(*) AS TagCount FROM Tags T JOIN Posts P ON P.Tags ILIKE CONCAT('%', T.TagName, '%')
//     WHERE P.PostTypeId = 1 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY T.TagName ORDER BY TagCount DESC LIMIT 5),
// PostAnalytics AS (SELECT P.Id AS PostId, P.Title, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseCount, COUNT(CM.Id) AS CommentCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments CM ON P.Id = CM.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title),
// FinalResults AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.LastActivityDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, PA.UpVotes, PA.DownVotes, PA.CloseCount, PA.CommentCount,
//        RANK() OVER (ORDER BY PA.UpVotes DESC, PA.CommentCount DESC) AS PopularityRank, (SELECT STRING_AGG(TagName, ', ') FROM PopularTags) AS TopTags
//     FROM RankedPosts RP LEFT JOIN PostAnalytics PA ON RP.PostId = PA.PostId)
// SELECT * FROM FinalResults WHERE PopularityRank <= 10 ORDER BY PopularityRank;
//
// The ILIKE is tag_mentions_ci. The five tags are listed most used first; the votes x comments x history product is
// driven for the RankedPosts questions.
fn q33063(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let tm = tag_mentions_ci(db);
    type M = (Id<Post>, Id<Tag>);
    let pt = (&tm)
        .with(Same::<M>::new().map(|x: M| x.0).with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(Same::<M>::new().map(|x: M| x.1).select(&db.tag.tag_name))
        .select(Same::<M>::new())
        .fold(0i64, |n, _| n + 1);
    let top = top_n(drain(&pt), |&(t, n)| (Reverse(n), t), 5);
    let tags = leak(top.iter().map(|x| x.0).collect::<Vec<_>>().join(", "));
    let rp: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1)).with(score.gt(0)).with(owner_user).collect();
    let pa = (&rp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 4], |a, ((v, c), h)| [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (h == Some(10)) as i64, a[3] + c.is_some() as i64]);
    let w = whole(&rp).select(Ident::<Post>::new().and(&pa)).window(rank, |(_, a)| (Reverse(a[0]), Reverse(a[3])), asc);
    let r = drain((&w).filt(|(_, k)| k <= 10));
    rows(r.into_iter().map(|(_, ((p, a), k))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        f.extend([V::I(k), V::S(tags)]);
        row(f)
    }))
}

// WITH FilteredPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Tags, p.OwnerUserId, LENGTH(STRING_AGG(p.Tags, ',')) AS TagCount, p.Body FROM Posts p
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.PostTypeId = 1 AND p.ViewCount > 10 GROUP BY p.Id, p.Title, p.CreationDate, p.Tags, p.OwnerUserId, p.Body),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(b.Class, 0)) AS TotalBadges FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '6 months' GROUP BY u.Id, u.DisplayName HAVING COUNT(p.Id) > 5),
// PostStatistics AS (SELECT p.PostId, p.Title, p.CreationDate, p.TagCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM FilteredPosts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.PostId = c.PostId LEFT JOIN Votes v ON p.PostId = v.PostId
//     GROUP BY p.PostId, p.Title, p.CreationDate, p.TagCount, u.DisplayName),
// RankedPosts AS (SELECT ps.*, RANK() OVER (ORDER BY ps.UpVotes - ps.DownVotes DESC) AS Rank FROM PostStatistics ps)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.TagCount, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.Rank, u.TotalBadges
// FROM RankedPosts rp JOIN MostActiveUsers u ON rp.OwnerDisplayName = u.DisplayName WHERE rp.Rank <= 10 ORDER BY rp.Rank;
//
// Each FilteredPosts group is one post, so TagCount is the length of its Tags. MostActiveUsers is joined on the display name, through a name index.
fn q28101(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, owner_user, tags_str, .. } = &db.post;
    let fp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(post_type_id.eq(1)).with(view_count.gt(10)).with(owner_user);
    let ps = fp().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let w = whole(fp()).select(Ident::<Post>::new().and(&ps)).window(rank, |(_, a)| Reverse(a[1] - a[2]), asc);
    type R = ((Id<Post>, [i64; 3]), i64);
    let rr: MatSet<R> = (&w).map(|x| x).collect();
    let mau = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 2], |a, (_, c)| [a[0] + 1, a[1] + c.unwrap_or(0)]);
    let byname: HashIdx<Str, Id<User>> = db.user.with((&mau).filt(|a| a[0] > 5)).select(&db.user.display_name).inv().collect();
    let v = drain((&rr).filt(|x: R| x.1 <= 10).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(owner_user.select(&db.user.display_name).select(byname.select(&mau))))));
    let mut v = v;
    v.sort_by_key(|x| x.1 .0 .1);
    rows(v.into_iter().map(|(_, (((p, a), k), m))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(oint(tags_str.get(p).map(|t| t.chars().count() as i64)));
        f.extend(post_fields(db, p, &["owner"]));
        f.extend(a.map(V::I));
        f.extend([V::I(k), V::I(m[1])]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, rp.CommentCount, rp.AnswerCount,
//        CASE WHEN rp.Score > 20 THEN 'High Score' WHEN rp.Score >= 10 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory, ROW_NUMBER() OVER (ORDER BY rp.ViewCount DESC) AS Rank FROM RecentPosts rp),
// BestPosts AS (SELECT ps.*, ROW_NUMBER() OVER (ORDER BY ps.UpVotes DESC, ps.ViewCount DESC) AS BestRank FROM PostStatistics ps)
// SELECT bp.PostId, bp.Title, bp.CreationDate, bp.Score, bp.ViewCount, bp.UpVotes, bp.DownVotes, bp.CommentCount, bp.AnswerCount, bp.ScoreCategory FROM BestPosts bp WHERE bp.BestRank <= 10
// UNION ALL
// SELECT -bp.PostId AS PostId, CONCAT('Unpopular Post: ', bp.Title) AS Title, bp.CreationDate, 0 AS Score, 0 AS ViewCount, 0 AS UpVotes, 0 AS DownVotes, 0 AS CommentCount, 0 AS AnswerCount,
//        'Unpopular' AS ScoreCategory FROM PostStatistics bp WHERE bp.Score <= 0 AND NOT EXISTS (SELECT 1 FROM BestPosts b WHERE b.PostId = bp.PostId)
// ORDER BY CreationDate DESC LIMIT 5;
//
// NOT EXISTS against BestPosts is every post (BestPosts is all of PostStatistics), so the second branch is empty. BestRank ties go to the smaller post id.
fn q2298(db: &'static So) -> String {
    let Post { creation_date, view_count, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let rp = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(answers_of(db).opt())).fold([0i64; 2], |a, ((t, _), _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let dc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let da = recent().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let best = top_n(drain((&rp).and(&dc).and(&da)), |&(p, ((a, _), _))| (Reverse(a[0]), view_count.get(p).is_none(), Reverse(view_count.get(p)), p), 10);
    let top10: MatSet<Id<Post>> = rel(best.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let b1 = (&top10).select(Ident::<Post>::new().and(&rp).and(&dc).and(&da)).map(|(((p, a), c), n)| (p, false, a, c, n));
    let b2 = recent().with(score.le(0)).minus(&rp).map(|p| (p, true, [0i64; 2], 0i64, 0i64));
    let out = top_n(drain(b1.union(b2)), |&(_, (p, ..))| Reverse(creation_date.get(p).unwrap()), 5);
    rows(out.into_iter().map(|(_, (p, un, a, c, n))| {
        let s = score.get(p).unwrap();
        if un {
            return row(vec![
                V::I(-db.post.origid.get(p).unwrap()),
                V::Owned(format!("Unpopular Post: {}", db.post.title.get(p).unwrap_or(""))),
                V::T(creation_date.get(p).unwrap()),
                V::I(0), V::I(0), V::I(0), V::I(0), V::I(0), V::I(0),
                V::S("Unpopular"),
            ]);
        }
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(n)]);
        f.push(V::S(if s > 20 { "High Score" } else if s >= 10 { "Moderate Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT U.Id AS UserId, U.Reputation, 0 AS Level FROM Users U WHERE U.Reputation IS NOT NULL
//     UNION ALL SELECT U.Id AS UserId, U.Reputation, UR.Level + 1 FROM Users U INNER JOIN UserReputation UR ON U.Id = UR.UserId WHERE UR.Level < 3),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Ranking FROM Users U),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, COUNT(C.Id) AS CommentCount, MAX(PH.CreationDate) AS LastEditDate, PT.Name AS PostType
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId INNER JOIN PostTypes PT ON P.PostTypeId = PT.Id
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.Score, P.ViewCount, PT.Name),
// AggregateVoteCounts AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN V.VoteTypeId = 10 THEN 1 ELSE 0 END) AS Deletions FROM Votes V GROUP BY V.PostId)
// SELECT TU.DisplayName, TU.Reputation, PS.PostId, PS.Title, PS.Score, PS.ViewCount, PS.CommentCount, PS.LastEditDate, PS.PostType, COALESCE(AVC.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(AVC.DownVotes, 0) AS TotalDownVotes, COALESCE(AVC.Deletions, 0) AS TotalDeletions
// FROM TopUsers TU JOIN Posts P ON P.OwnerUserId = TU.UserId JOIN PostStatistics PS ON PS.PostId = P.Id LEFT JOIN AggregateVoteCounts AVC ON AVC.PostId = P.Id
// WHERE TU.Ranking <= 10 ORDER BY TU.Reputation DESC, PS.Score DESC;
//
// The recursive CTE is never read. The ten users are picked by Reputation first.
fn q30679(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let top = top_n(drain(db.user.select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ps = (&tu)
        .select(posts_of(db))
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Same::<Id<Post>>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, i64::MIN), |a, (c, d)| (a.0 + c.is_some() as i64, a.1.max(d.unwrap_or(i64::MIN))));
    let avc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 10) as i64]);
    let v = drain((&tu).select(Ident::<User>::new().and(posts_of(db).select(Ident::<Post>::new().and(&ps).and((&avc).opt())))));
    let mut v = v;
    v.sort_by_key(|&(_, (u, ((p, _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(db.post.score.get(p).unwrap())));
    rows(v.into_iter().map(|(_, (u, ((p, (c, d)), a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(c), tmax(d)]);
        f.extend(post_fields(db, p, &["type"]));
        f.extend(a.unwrap_or([0; 3]).map(V::I));
        row(f)
    }))
}

// WITH RecursivePostCTE AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.Score, p.CreationDate, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteSummary AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, STRING_AGG(c.Name, ', ') AS CloseReasons FROM PostHistory ph INNER JOIN CloseReasonTypes c ON CAST(ph.Comment AS int) = c.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate)
// SELECT u.DisplayName AS User, up.Title AS RecentQuestion, up.Score AS QuestionScore, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, COALESCE(ps.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(ps.DownVotes, 0) AS TotalDownVotes, cp.CloseReasons AS CloseReasonsSummary, CASE WHEN cp.CloseReasons IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM Users u JOIN UserBadges ub ON u.Id = ub.UserId JOIN RecursivePostCTE up ON u.Id = up.OwnerUserId AND up.rn = 1 LEFT JOIN PostVoteSummary ps ON up.PostId = ps.PostId
// LEFT JOIN ClosedPosts cp ON up.PostId = cp.PostId WHERE u.Reputation > 1000 ORDER BY u.Reputation DESC, up.CreationDate DESC;
//
// Nothing recurses. rn reads only base columns, so each owner's newest question is picked first. ClosedPosts' groups are re-keyed by post to be joined.
fn q32522(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, owner_user, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let up: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let ub = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let pv = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, post, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post.and(hd)).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)).buf_fold(|v| leak(v.join(", ")));
    let cr = rel(drain(&cp));
    type C = ((Id<Post>, i64), Str);
    let cidx: HashIdx<Id<Post>, C> = (&cr).map(|x: C| x.0 .0).inv().select(&cr).collect();
    let v = drain((&up).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ub))).and(&pv).and((&cidx).opt())));
    let mut v = v;
    v.sort_by_key(|&(_, (((p, (u, _)), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, (((p, (u, b)), a), c))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), ostr(c.map(|c| c.1)), V::S(if c.is_some() { "Closed" } else { "Open" })]);
        row(f)
    }))
}

fn json_int_field(s: Str, key: &str) -> Option<i64> {
    let s = s.trim();
    if !s.starts_with('{') {
        return None;
    }
    let k = format!("\"{key}\"");
    let i = s.find(&k)? + k.len();
    let rest = s[i..].trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.trim_start_matches('"');
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '-')).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostAggregate AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, COALESCE(PH.TotalHistoryChanges, 0) AS HistoryChanges, COALESCE(CR.ReasonName, 'N/A') AS CloseReason
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS TotalHistoryChanges FROM PostHistory GROUP BY PostId) PH ON P.Id = PH.PostId
//     LEFT JOIN (SELECT PH.PostId, CRT.Name AS ReasonName FROM PostHistory PH JOIN CloseReasonTypes CRT ON CAST(PH.Comment AS INTEGER) = CRT.Id WHERE PH.PostHistoryTypeId IN (10, 11)
//                GROUP BY PH.PostId, CRT.Name) CR ON P.Id = CR.PostId),
// UserPostDetails AS (SELECT US.UserId, US.DisplayName, SUM(PA.ViewCount) AS TotalPostViews, SUM(PA.Score) AS TotalPostScore, AVG(PA.HistoryChanges) AS AvgPostHistoryChanges,
//        ARRAY_AGG(DISTINCT PA.CloseReason) AS CloseReasons FROM UserStatistics US JOIN PostAggregate PA ON US.UserId = PA.PostId GROUP BY US.UserId, US.DisplayName)
// SELECT U.DisplayName, U.Reputation, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalUpvotes, U.TotalDownvotes, UPD.TotalPostViews, UPD.TotalPostScore, UPD.AvgPostHistoryChanges,
//        UNNEST(UPD.CloseReasons) AS CloseReason
// FROM UserStatistics U JOIN UserPostDetails UPD ON U.UserId = UPD.UserId ORDER BY U.Reputation DESC, U.TotalPosts DESC LIMIT 100;
//
// `US.UserId = PA.PostId` joins a user id to a post id, so it goes through the raw ids. The UNNEST is a flat_map over the distinct reasons.
fn q8118(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]);
    let phc = db.post_history.group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id: t, comment, post, .. } = &db.post_history;
    let crs: MatSet<(Id<Post>, Str)> = db.post_history.with(t.eq(10).or(t.eq(11))).select(post.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))).collect();
    let cri: HashIdx<Id<Post>, (Id<Post>, Str)> = (&crs).map(|x: (Id<Post>, Str)| x.0).inv().select(&crs).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let upd = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.origid).select(&pidx).select(view_count.opt().and(score).and((&phc).opt()).and((&cri).opt())))
        .buf_fold(|v| {
            let mut r: Vec<Str> = v.iter().map(|x| x.1.map_or("N/A", |c| c.1)).collect();
            r.sort_unstable();
            r.dedup();
            let w: Vec<i64> = v.iter().flat_map(|x| x.0 .0 .0).collect();
            let vs = if w.is_empty() { None } else { Some(w.iter().sum::<i64>()) };
            (vs, v.iter().map(|x| x.0 .0 .1).sum::<i64>(), v.iter().map(|x| x.0 .1.unwrap_or(0)).sum::<i64>(), v.len() as i64, &*Box::leak(r.into_boxed_slice()))
        });
    type U = (Option<i64>, i64, i64, i64, &'static [Str]);
    let v = drain((&us).and(&dp).and((&upd).flat_map(|a: U| a.4.iter().map(move |&r| (a.0, a.1, a.2, a.3, r)))));
    let v = top_n(v, |&(u, ((_, d), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(d[0])), 100);
    rows(v.into_iter().map(|(u, ((a, d), (w, s, h, n, r)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(d[0]), V::I(d[1]), V::I(d[2]), V::I(a[0]), V::I(a[1]), oint(w), V::I(s), avg(h, n), V::S(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName, pht.Name AS PostHistoryTypeName, ph.Comment, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = rp.PostId)
//     LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId
//     WHERE rp.Rank <= 5 GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, u.DisplayName, pht.Name, ph.Comment),
// PostMetrics AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, Score, PostHistoryTypeName, CommentCount, UpVotes, DownVotes,
//        CASE WHEN UpVotes = 0 AND DownVotes = 0 THEN 'No Votes' WHEN UpVotes > DownVotes THEN 'Positive' WHEN UpVotes < DownVotes THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment FROM TopPosts)
// SELECT pm.*, (SELECT STRING_AGG(t.TagName, ', ') FROM Tags t JOIN Posts p ON p.Id = pm.PostId WHERE t.ExcerptPostId = p.Id) AS Tags
// FROM PostMetrics pm WHERE pm.CommentCount > 0 ORDER BY Score DESC, CreationDate DESC;
//
// Rank ties go to the smaller post id. The group is keyed by the latest history rows' type and comment, so the (post, latest history row) pairs are
// materialised and grouped, with the comment x vote product hanging off the post.
fn q21045(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let PostHistory { post, creation_date: hd, comment, .. } = &db.post_history;
    let hmax = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let latest: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post.select(&hmax).and(hd).filt(|(m, d)| m == d)).select(post).inv().collect();
    type J = (Id<Post>, Option<((Id<PostHistory>, Str), Option<Str>)>);
    let j: MatSet<J> = (&rp).select(Ident::<Post>::new().and((&latest).select(Ident::<PostHistory>::new().and(htype_name(db)).and(comment.opt())).opt())).collect();
    let g = (&j)
        .group_by(Same::<J>::new().map(|x: J| (x.0, x.1.map(|((_, n), c)| (n, c)))))
        .select(Same::<J>::new().map(|x: J| x.0).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&rp).group_by(Ident::<Post>::new()).select((&ex).select(&db.tag.tag_name)).buf_fold(|v| leak(v.join(", ")));
    type K = (Id<Post>, Option<(Str, Option<Str>)>);
    let v = drain(rel(drain((&g).filt(|a| a[0] > 0))).select(Same::<(K, [i64; 3])>::new().and(Same::<(K, [i64; 3])>::new().map(|x: (K, [i64; 3])| x.0 .0).select((&tg).opt()))));
    let mut v = v;
    v.sort_by_key(|&(_, (((p, _), _), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, (((p, h), a), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::S(db.post.owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap())));
        f.extend(post_fields(db, p, &["score"]));
        f.push(ostr(h.map(|h| h.0)));
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] == 0 && a[2] == 0 { "No Votes" } else if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        f.push(ostr(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY p.Id), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY p.Id), 0) AS DownVotes, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 AND p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId),
// PostHistoryDetails AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS CloseDate, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenDate,
//        COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount FROM PostHistory ph GROUP BY ph.PostId),
// FinalPostStats AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, ph.CloseDate, ph.ReopenDate, ub.BadgeCount, ub.BadgeNames, ph.CloseReopenCount
//     FROM RankedPosts rp LEFT JOIN PostHistoryDetails ph ON rp.PostId = ph.PostId LEFT JOIN UserBadges ub ON ub.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId LIMIT 1) WHERE rp.rn <= 10)
// SELECT Title, Score, ViewCount, UpVotes, DownVotes, CloseDate, ReopenDate, COALESCE(BadgeCount, 0) AS BadgeCount, COALESCE(BadgeNames, 'None') AS BadgeNames
// FROM FinalPostStats WHERE CloseReopenCount = 0 ORDER BY Score DESC, ViewCount DESC;
//
// rn numbers the joined vote rows. `CloseReopenCount = 0` needs a PostHistoryDetails row. Badge names are in badge id order.
fn q21565(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user_id, .. } = &db.post;
    let recent = || db.post.with(post_type_id.eq(1)).with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let j = top_n(drain(recent().select(votes_of(db).opt())), |&(p, v)| (Reverse(creation_date.get(p).unwrap()), p, v), 10);
    let jr = rel(j.into_iter().map(|x| x.0).collect());
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id: t, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.group_by(&db.post_history.post).select(t.and(hd)).fold((i64::MIN, i64::MIN, 0i64), |a, (t, d)| {
        (if t == 10 { a.0.max(d) } else { a.0 }, if t == 11 { a.1.max(d) } else { a.1 }, a.2 + (t == 10 || t == 11) as i64)
    });
    let ub = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user_id).select(&db.badge.name).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let v = drain((&jr).select(Ident::<Post>::new().and(&vc).and((&ph).filt(|a| a.2 == 0)).and(owner_user_id.select(&ub).opt())));
    let mut v = v;
    v.sort_by_key(|&(_, (((p, _), _), _))| {
        let w = db.post.view_count.get(p);
        (Reverse(db.post.score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, (((p, a), h), b))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), tmax(h.0), tmax(h.1), V::I(b.map_or(0, |b| b.0)), V::S(b.map_or("None", |b| b.1))]);
        row(f)
    }))
}

// WITH ProcessedTags AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS TagName FROM Posts p WHERE p.PostTypeId = 1),
// TagStatistics AS (SELECT TagName, COUNT(*) AS TagFrequency, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM ProcessedTags JOIN Posts p ON ProcessedTags.PostId = p.Id GROUP BY TagName),
// TopTags AS (SELECT TagName, TagFrequency, TotalViews, TotalScore, ROW_NUMBER() OVER (ORDER BY TagFrequency DESC, TotalViews DESC) AS Rank FROM TagStatistics),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Comments c ON c.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// ActiveUsers AS (SELECT UserId, DisplayName, QuestionCount, CommentCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY QuestionCount DESC, UpVotes DESC) AS UserRank FROM UserActivity)
// SELECT t.TagName, t.TagFrequency, t.TotalViews, t.TotalScore, u.DisplayName AS TopUser, u.QuestionCount, u.CommentCount, u.UpVotes AS UserUpVotes, u.DownVotes AS UserDownVotes
// FROM TopTags t JOIN ActiveUsers u ON t.TagName IN (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) FROM Posts p WHERE p.OwnerUserId = u.UserId AND p.PostTypeId = 1)
// WHERE t.Rank <= 10 AND u.UserRank <= 5 ORDER BY t.TagFrequency DESC, u.UpVotes DESC;
//
// UserRank leads with QuestionCount, a per-user count, so only the users with RANK() <= 5 on it can rank in the top five; the
// questions x comments x votes product is driven for those alone.
fn q29566(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, tags_str, .. } = &db.post;
    let q = || db.post.with(post_type_id.eq(1));
    let ts_ = q().group_by(tags_str.flat_map(tag_list)).select(view_count.opt().and(score)).fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let tt = top_n(drain(&ts_), |&(t, a)| (Reverse(a[0]), a[1] == 0, Reverse(a[2]), t), 10);
    let tq = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let dq = db.user.group_by(Ident::<User>::new()).select(tq().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&dq)).window(rank, |(_, n)| Reverse(n), asc);
    let cand: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 5).map(|((u, _), _)| u).collect();
    let ua = (&cand).group_by(Ident::<User>::new()).select(tq().opt().and(comments_by(db).opt()).and(votes_by(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, ((_, _), t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let dc = (&cand).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let top = top_n(drain((&ua).and(&dq).and(&dc)), |&(u, ((a, n), _))| (Reverse(n), Reverse(a[0]), u), 5);
    let tu = rel(top);
    let ttr = rel(tt);
    type T = (Str, [i64; 4]);
    type U = (Id<User>, (([i64; 2], i64), i64));
    let ut: MatSet<(Id<User>, Str)> = (&tu).map(|x: U| x.0).select(Ident::<User>::new().and(tq().select(tags_str.flat_map(tag_list)))).collect();
    let tti: HashIdx<Str, T> = (&ttr).map(|x: T| x.0).inv().select(&ttr).collect();
    let tui: HashIdx<Id<User>, U> = (&tu).map(|x: U| x.0).inv().select(&tu).collect();
    type P = (Id<User>, Str);
    let v = drain((&ut).select(Same::<P>::new().map(|x: P| x.1).select(&tti).and(Same::<P>::new().map(|x: P| x.0).select(&tui))));
    let mut v = v;
    v.sort_by_key(|&(_, (t, u))| (Reverse(t.1[0]), Reverse(u.1 .0 .0[0])));
    rows(v.into_iter().map(|(_, ((t, a), (u, ((b, n), c))))| {
        let mut f = vec![V::S(t), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3])];
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(n), V::I(c), V::I(b[0]), V::I(b[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.PostTypeId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COUNT(c.Id) DESC) AS Rank FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY p.Id, p.PostTypeId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id WHERE u.Reputation > 1000 GROUP BY u.Id),
// PostDetails AS (SELECT r.PostId, r.PostTypeId, r.CommentCount, r.UpVotes, r.DownVotes, u.AboutMe, ub.BadgeCount, ub.HighestBadgeClass,
//        CASE WHEN p.AcceptedAnswerId IS NULL THEN 'No Accepted Answer' ELSE 'Accepted Answer Exists' END AS AcceptedAnswerStatus
//     FROM RankedPosts r JOIN Posts p ON p.Id = r.PostId JOIN Users u ON u.Id = p.OwnerUserId LEFT JOIN UserBadges ub ON ub.UserId = p.OwnerUserId)
// SELECT pd.PostId, pd.PostTypeId, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.AboutMe, pd.BadgeCount, pd.HighestBadgeClass, pd.AcceptedAnswerStatus,
//        CASE WHEN pd.CommentCount > 10 THEN 'Highly Interactive' WHEN pd.UpVotes - pd.DownVotes > 0 THEN 'Positive Engagement' ELSE 'Low Engagement' END AS EngagementLevel,
//        STRING_AGG(DISTINCT pt.Name, ', ') AS PostType
// FROM PostDetails pd JOIN PostTypes pt ON pt.Id = pd.PostTypeId
// GROUP BY pd.PostId, pd.PostTypeId, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.AboutMe, pd.BadgeCount, pd.HighestBadgeClass, pd.AcceptedAnswerStatus
// HAVING pd.CommentCount > 5 AND (pd.UpVotes - pd.DownVotes) > 2 ORDER BY pd.UpVotes DESC, pd.CommentCount DESC LIMIT 100;
//
// Rank is never read. Each group is one post with its one type name.
fn q20809(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let rp = db.post.with(owner_user).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let ub = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |a, c| (a.0 + c.is_some() as i64, a.1.max(c.unwrap_or(0))));
    let v = drain((&rp).filt(|a| a[0] > 5 && a[1] - a[2] > 2).and(owner_user.select(Ident::<User>::new().and((&db.user.about_me).opt()).and((&ub).opt()))));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[1]), Reverse(a[0]), p), 100);
    rows(v.into_iter().map(|(p, (a, ((_, am), b)))| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend(a.map(V::I));
        f.push(ostr(am));
        f.extend([oint(b.map(|b| b.0)), b.map_or(V::Null, |b| if b.0 == 0 { V::Null } else { V::I(b.1) })]);
        f.push(V::S(if db.post.accepted_answer_id.get(p).is_none() { "No Accepted Answer" } else { "Accepted Answer Exists" }));
        f.push(V::S(if a[0] > 10 { "Highly Interactive" } else if a[1] - a[2] > 0 { "Positive Engagement" } else { "Low Engagement" }));
        f.extend(post_fields(db, p, &["type"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, SUM(p.ViewCount) AS TotalViews, ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.OwnerUserId),
// ClosedQuestions AS (SELECT p.Id AS PostId, p.Title, ph.CreationDate AS ClosedDate, STRING_AGG(DISTINCT ctr.Name, ', ') AS CloseReason
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10 LEFT JOIN CloseReasonTypes ctr ON (ph.Comment::json->>'ReasonId')::int = ctr.Id GROUP BY p.Id, p.Title, ph.CreationDate)
// SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalComments, ua.TotalUpVotes, ua.TotalDownVotes, ua.TotalViews, ps.PostId, ps.Title AS PostTitle, ps.CommentCount, ps.UpVotes, ps.DownVotes,
//        ps.TotalBounty, cq.ClosedDate, cq.CloseReason
// FROM UserActivity ua LEFT JOIN PostStats ps ON ua.UserId = ps.OwnerUserId LEFT JOIN ClosedQuestions cq ON ps.PostId = cq.PostId
// WHERE ua.TotalPosts > 0 ORDER BY ua.Rank, ps.PostId OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
//
// Rank reads only the distinct post count, so the users are ranked by it first; every kept user yields at least one row, so the twenty rows
// the OFFSET/FETCH reads come from Rank <= 20, and the posts x comments x votes product is driven for those. `->>'ReasonId'` of a Comment that is not a JSON
// object is NULL.
fn q3348(db: &'static So) -> String {
    let Post { view_count, .. } = &db.post;
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(db.user.with((&dp).filt(|n| n > 0))).select(Ident::<User>::new().and(&dp)).window(row_number, |(u, n)| (Reverse(n), u), asc);
    type T = (Id<User>, i64, i64);
    let tu: MatSet<T> = (&w).filt(|(_, k)| k <= 20).map(|((u, n), k)| (u, n, k)).collect();
    let us: MatSet<Id<User>> = (&tu).map(|x: T| x.0).collect();
    let ua = (&us)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some(((w, _), t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
            None => a,
        });
    let dc = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ps: MatSet<Id<Post>> = (&us).select(posts_of(db)).collect();
    let pst = (&ps).group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (v, _)| {
        let t = v.map(|v| v.0);
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + v.and_then(|v| v.1).unwrap_or(0)]
    });
    let pdc = (&ps).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, post, .. } = &db.post_history;
    let cq = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post.and(hd))
        .select(comment.flat_map(|s: Str| json_int_field(s, "ReasonId")).select(&reason).opt())
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().flatten().copied().collect();
            t.sort_unstable();
            t.dedup();
            agg_str(t, ", ")
        });
    let cr = rel(drain(&cq));
    type C = ((Id<Post>, i64), Option<Str>);
    let cidx: HashIdx<Id<Post>, C> = (&cr).map(|x: C| x.0 .0).inv().select(&cr).collect();
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select((&ua).and(&dc).and(posts_of(db).select(Ident::<Post>::new().and(&pst).and(&pdc).and((&cidx).opt())).opt())))));
    let v = top_n(v, |&(_, ((_, _, k), ((_, _), p)))| (k, p.map(|p| db.post.origid.get(p.0 .0 .0).unwrap())), 20);
    rows(v.into_iter().skip(10).map(|(_, ((u, n, _), ((a, c), p)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2])]);
        match p {
            Some((((p, s), dc), q)) => {
                f.extend(post_fields(db, p, &["id", "title"]));
                f.extend([V::I(dc), V::I(s[0]), V::I(s[1]), V::I(s[2])]);
                match q {
                    Some(((_, d), r)) => f.extend([V::T(d), ostr(r)]),
                    None => f.extend([V::Null, V::Null]),
                }
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, pt.Name AS PostType, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount
//     FROM Posts p INNER JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days') GROUP BY p.Id, p.Title, p.CreationDate, pt.Name, u.DisplayName),
// VoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes FROM Votes v GROUP BY v.PostId),
// PostHistoryDetails AS (SELECT ph.PostId, STRING_AGG(CONCAT(pt.Name, ' by ', ph.UserDisplayName), ', ') AS History, MAX(ph.CreationDate) AS LastModified
//     FROM PostHistory ph INNER JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id GROUP BY ph.PostId),
// FinalData AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.PostType, rp.OwnerDisplayName, rp.CommentCount, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes,
//        ph.History, ph.LastModified, ru.DisplayName AS TopUser, ru.UserRank
//     FROM RecentPosts rp LEFT JOIN VoteSummary vs ON rp.PostId = vs.PostId LEFT JOIN PostHistoryDetails ph ON rp.PostId = ph.PostId CROSS JOIN (SELECT * FROM RankedUsers WHERE UserRank = 1) ru)
// SELECT * FROM FinalData WHERE (UpVotes - DownVotes) > 10 OR (CommentCount > 5 AND LastModified > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days')) ORDER BY CreationDate DESC;
//
// History entries are in history id order.
fn q22862(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vs = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { user_display_name, creation_date: hd, .. } = &db.post_history;
    let ph = recent().group_by(Ident::<Post>::new()).select(history_of(db).select(htype_name(db).and(user_display_name.opt()).and(hd))).buf_fold(|v| {
        (leak(v.iter().map(|&((n, u), _)| format!("{n} by {}", u.unwrap_or(""))).collect::<Vec<_>>().join(", ")), v.iter().map(|x| x.1).max().unwrap())
    });
    let ru = top_n(drain(db.user.select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 1);
    let ru = rel(ru.into_iter().map(|x| x.0).collect());
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -7);
    let v = drain(
        (&cc)
            .and((&vs).opt())
            .and((&ph).opt())
            .filt(|((c, a), h): ((i64, Option<[i64; 2]>), Option<(Str, i64)>)| {
                let a = a.unwrap_or([0, 0]);
                a[0] - a[1] > 10 || (c > 5 && h.map_or(false, |h| h.1 > cut))
            })
            .cross(&ru),
    );
    rows(v.into_iter().map(|((p, _), (((c, a), h), u))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "type", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), ostr(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
        f.extend(ucols(db, u, &["name"]));
        f.push(V::I(1));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        STRING_AGG(t.TagName, ', ') AS Tags FROM Posts p LEFT JOIN Tags t ON t.WikiPostId = p.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 YEAR'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.PostTypeId),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5),
// UserInteraction AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users u LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// PostHistoryDetails AS (SELECT ph.PostId, ph.CreationDate AS EditDate, ph.UserDisplayName, ph.Comment AS EditComment, ph.Text AS NewBody FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6)),
// AggregatedPostHistory AS (SELECT p.PostId, MAX(ph.EditDate) AS LastEditDate, STRING_AGG(CONCAT(ph.UserDisplayName, ' - ', ph.EditComment), '; ') AS EditsDetails
//     FROM TopPosts p LEFT JOIN PostHistoryDetails ph ON ph.PostId = p.PostId GROUP BY p.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.Tags, up.DisplayName AS TopUser, up.TotalUpVotes, up.TotalDownVotes, aph.LastEditDate, aph.EditsDetails
// FROM TopPosts tp LEFT JOIN UserInteraction up ON up.UserId = tp.PostId LEFT JOIN AggregatedPostHistory aph ON aph.PostId = tp.PostId
// WHERE (COALESCE(up.TotalUpVotes, 0) - COALESCE(up.TotalDownVotes, 0)) > 10 ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// `up.UserId = tp.PostId` joins a user id to a post id, so it goes through the raw ids; the WHERE needs a user row with more than ten net votes, so
// that join is inner. A post with no edit still has its one LEFT JOIN row, whose CONCAT is ' - '. Rank ties go to the smaller post id.
fn q32181(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let wk: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let tg = (&tp).group_by(Ident::<Post>::new()).select((&wk).select(&db.tag.tag_name)).buf_fold(|v| leak(v.join(", ")));
    let ui = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post_history_type_id: t, user_display_name, comment, creation_date: hd, .. } = &db.post_history;
    let eh = history_of(db).select(Ident::<PostHistory>::new().with(t.eq(4).or(t.eq(5)).or(t.eq(6))));
    let aph = (&tp).group_by(Ident::<Post>::new()).select(eh.select(hd.and(user_display_name.opt()).and(comment.opt())).opt()).buf_fold(|v| {
        let d = v.iter().flatten().map(|x| x.0 .0).max();
        let s = v.iter().map(|x| match x {
            Some(((_, u), c)) => format!("{} - {}", u.unwrap_or(""), c.unwrap_or("")),
            None => " - ".to_string(),
        });
        (d, leak(s.collect::<Vec<_>>().join("; ")))
    });
    let v = drain((&tp).select(Ident::<Post>::new().and((&tg).opt()).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and((&ui).filt(|a| a[0] - a[1] > 10)))).and(&aph)));
    let mut v = v;
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, (((p, t), (u, a)), (d, s)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.push(ostr(t));
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(a[0]), V::I(a[1]), ots(d), V::S(s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankByCreation,
//        RANK() OVER (ORDER BY p.Score DESC) AS RankByScore FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// UserVoteStats AS (SELECT v.UserId, COUNT(CASE WHEN v.VoteTypeId IN (2, 6) THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId IN (3, 10) THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.UserId),
// PostCommentCounts AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostBadges AS (SELECT b.UserId, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId),
// CompositeVotes AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        COUNT(CASE WHEN VoteTypeId = 6 THEN 1 ELSE 0 END) AS CloseVotes FROM Votes GROUP BY PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, up.UpVotes, down.DownVotes, COALESCE(cc.CommentCount, 0) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 6) AS CloseVoteCount, pb.BadgeNames,
//        CASE WHEN rp.RankByCreation <= 10 THEN 'New' WHEN rp.RankByScore <= 10 THEN 'Popular' ELSE 'Regular' END AS PostCategory
// FROM RankedPosts rp LEFT JOIN UserVoteStats up ON up.UserId = rp.PostId LEFT JOIN UserVoteStats down ON down.UserId = rp.PostId LEFT JOIN PostCommentCounts cc ON cc.PostId = rp.PostId
// LEFT JOIN PostBadges pb ON pb.UserId = rp.PostId LEFT JOIN CompositeVotes cv ON cv.PostId = rp.PostId
// WHERE rp.RankByCreation <= 20 OR rp.RankByScore <= 20 ORDER BY rp.CreationDate DESC, rp.Score DESC;
//
// UserVoteStats and PostBadges are keyed by a user id and joined to a post id, so they go through the raw ids. CompositeVotes is never read.
// RankByCreation ties go to the smaller post id; badge names are in badge id order.
fn q24057(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    type A = (Id<Post>, i64);
    let ra: MatSet<A> = (&w).map(|((p, _), a)| (p, a)).collect();
    let w = whole(&ra).select(Same::<A>::new().and(Same::<A>::new().map(|x: A| x.0).select(score))).window(rank, |(_, s)| Reverse(s), asc);
    type R = (Id<Post>, i64, i64);
    let rr: MatSet<R> = (&w).map(|(((p, a), _), b)| (p, a, b)).collect();
    let Vote { vote_type_id, user_id, .. } = &db.vote;
    let uvs = db.vote.group_by(user_id).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2 || t == 6) as i64, a[1] + (t == 3 || t == 10) as i64]);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let c6 = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(6)))).fold(0i64, |n, _| n + 1);
    let pb = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user_id).select(&db.badge.name).buf_fold(|v| leak(v.join(", ")));
    let origid = &db.post.origid;
    let v = drain(
        (&rr)
            .filt(|x: R| x.1 <= 20 || x.2 <= 20)
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(origid.select((&uvs).opt()).and((&cc).opt()).and((&c6).opt()).and(origid.select((&pb).opt()))))),
    );
    let mut v = v;
    v.sort_by_key(|&(_, ((p, _, _), _))| (Reverse(creation_date.get(p).unwrap()), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((p, a, b), (((u, c), k), n)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([oint(u.map(|u| u[0])), oint(u.map(|u| u[1])), V::I(c.unwrap_or(0)), V::I(k.unwrap_or(0)), ostr(n)]);
        f.push(V::S(if a <= 10 { "New" } else if b <= 10 { "Popular" } else { "Regular" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(CAST(NULLIF(p.OwnerUserId, -1) AS INT), -1) AS ActualOwnerUserId, ARRAY_AGG(t.TagName) AS TagsArray
//     FROM Posts p LEFT JOIN Tags t ON POSITION(t.TagName IN p.Tags) > 0 WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// VoteStatistics AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId IN (2, 1) THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v GROUP BY v.PostId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// BadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges FROM Badges b GROUP BY b.UserId),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserId, MAX(ph.CreationDate) AS LastModifiedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (5, 11, 12, 14)
//     GROUP BY ph.PostId, ph.PostHistoryTypeId, ph.UserId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes,
//        u.Reputation, bc.TotalBadges, ph.LastModifiedDate, CASE WHEN ph.PostHistoryTypeId = 10 THEN 'Closed' WHEN ph.PostHistoryTypeId = 11 THEN 'Reopened' ELSE 'Open' END AS PostStatus,
//        ARRAY_TO_STRING(rp.TagsArray, ', ') AS Tags
// FROM RecentPosts rp LEFT JOIN Users u ON u.Id = rp.ActualOwnerUserId LEFT JOIN VoteStatistics vs ON vs.PostId = rp.PostId LEFT JOIN BadgeCounts bc ON bc.UserId = u.Id
// LEFT JOIN PostHistoryDetails ph ON ph.PostId = rp.PostId WHERE rp.ViewCount > (SELECT AVG(ViewCount) FROM RecentPosts) ORDER BY rp.CreationDate DESC LIMIT 50;
//
// A NULL owner becomes -1, which is a real user (Community), so the owner goes through the raw ids. A tag name has no '<' or '>', so POSITION is
// tag_mentions; the tags are listed in tag id order. The average is a whole-table fold, compared exactly. History groups are re-keyed by post.
fn q21981(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user_id, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)));
    let (s, n) = recent().select(view_count).fold_flat((0i128, 0i128), |(s, n), w| (s + w as i128, n + 1));
    let tm = tag_mentions(db);
    let tg: HashIdx<Id<Post>, Id<Tag>> = (&tm).map(|x: (Id<Post>, Id<Tag>)| x.0).inv().select((&tm).map(|x: (Id<Post>, Id<Tag>)| x.1)).collect();
    let ta = recent().group_by(Ident::<Post>::new()).select((&tg).opt()).buf_fold(|v| {
        let mut t: Vec<Id<Tag>> = v.iter().flatten().copied().collect();
        t.sort_unstable();
        leak(t.iter().map(|&t| db.tag.tag_name.get(t).unwrap()).collect::<Vec<_>>().join(", "))
    });
    let vs = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2 || t == 1) as i64, a[1] + (t == 3) as i64]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post, post_history_type_id: t, user_id, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.with(t.eq(5).or(t.eq(11)).or(t.eq(12)).or(t.eq(14))).group_by(post.and(t).and(user_id.opt())).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let pr = rel(drain(&phd));
    type H = (((Id<Post>, i64), Option<i64>), i64);
    let hidx: HashIdx<Id<Post>, H> = (&pr).map(|x: H| x.0 .0 .0).inv().select(&pr).collect();
    let own = owner_user_id.opt().map(|o: Option<i64>| o.filter(|&x| x != -1).unwrap_or(-1));
    let v = drain(
        recent()
            .with(view_count.filt(move |w| w as i128 * n > s))
            .select(Ident::<Post>::new().and(&ta).and(own.select(&uidx).select(Ident::<User>::new().and((&bc).opt())).opt()).and((&vs).opt()).and((&hidx).opt())),
    );
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(_, ((((p, t), u), a), h))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(match u {
            Some((u, _)) => ucols(db, u, &["name"]).remove(0),
            None => V::S("Anonymous"),
        });
        f.extend([V::I(a[0]), V::I(a[1])]);
        match u {
            Some((u, b)) => f.extend([ucols(db, u, &["rep"]).remove(0), oint(b)]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(ots(h.map(|h| h.1)));
        f.push(V::S(match h.map(|h| h.0 .0 .1) {
            Some(10) => "Closed",
            Some(11) => "Reopened",
            _ => "Open",
        }));
        f.push(V::S(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS RankByScore,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.Id) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.Id) AS DownVotes,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) OVER (PARTITION BY P.Id) AS CommentCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, STRING_AGG(CT.Name, ', ') AS CloseReasons FROM PostHistory PH JOIN CloseReasonTypes CT ON PH.Comment::INTEGER = CT.Id WHERE PH.PostHistoryTypeId = 10
//     GROUP BY PH.PostId, PH.CreationDate),
// ActiveBadges AS (SELECT U.Id AS UserId, B.Name AS BadgeName, COUNT(B.Id) AS BadgeCount FROM Badges B JOIN Users U ON B.UserId = U.Id
//     WHERE B.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY U.Id, B.Name),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U WHERE U.Reputation > 1000)
// SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.UpVotes, RP.DownVotes, RP.CommentCount, CP.CloseReasons, TB.UserId AS TopUserId, TB.DisplayName AS TopUserName, TB.Reputation AS TopUserReputation,
//        AB.BadgeName, AB.BadgeCount
// FROM RankedPosts RP LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId LEFT JOIN ActiveBadges AB ON RP.PostId = AB.UserId LEFT JOIN TopUsers TB ON AB.UserId = TB.UserId
// WHERE RP.RankByScore <= 5 AND (CP.CloseReasons IS NOT NULL OR AB.BadgeCount > 0) ORDER BY RP.Score DESC, RP.ViewCount DESC;
//
// RankByScore numbers the joined vote x comment rows; ties go to the smaller post, vote and comment id. `RP.PostId = AB.UserId` joins a post id to a user
// id, so it goes through the raw ids. ClosedPosts and ActiveBadges are re-keyed by post and user to be joined.
fn q21999(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    type J = ((Id<Post>, Option<Id<Vote>>), Option<Id<Comment>>);
    let w = recent()
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(votes_of(db).opt()).and(comments_of(db).opt()).and(score))
        .window(row_number, |(((p, v), c), s)| (Reverse(s), p, v, c), asc);
    let rp: MatSet<J> = (&w).filt(|(_, k)| k <= 5).map(|((j, _), _)| j).collect();
    let e =recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]
    });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, post, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post.and(hd)).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)).buf_fold(|v| leak(v.join(", ")));
    let cr = rel(drain(&cp));
    type C = ((Id<Post>, i64), Str);
    let cidx: HashIdx<Id<Post>, C> = (&cr).map(|x: C| x.0 .0).inv().select(&cr).collect();
    let ab = db.badge.with((&db.badge.date).ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).group_by((&db.badge.user).and(&db.badge.name)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ar = rel(drain(&ab));
    type A = ((Id<User>, Str), i64);
    let aidx: HashIdx<Id<User>, A> = (&ar).map(|x: A| x.0 .0).inv().select(&ar).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let hi = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain(
        (&rp)
            .select(Same::<J>::new().map(|x: J| x.0 .0).select(Ident::<Post>::new().and(&e).and((&cidx).opt()).and((&db.post.origid).select(&uidx).select((&aidx).and(hi.opt())).opt())))
            .filt(|((_, c), b): (((Id<Post>, [i64; 3]), Option<C>), Option<(A, Option<Id<User>>)>)| c.is_some() || b.map_or(false, |x| x.0 .1 > 0)),
    );
    let mut v = v;
    v.sort_by_key(|&(_, (((p, _), _), _))| {
        let w = db.post.view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, (((p, a), c), b))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(ostr(c.map(|c| c.1)));
        match b.and_then(|b| b.1) {
            Some(u) => f.extend(ucols(db, u, &["uid", "name", "rep"])),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.extend([ostr(b.map(|b| b.0 .0 .1)), oint(b.map(|b| b.0 .1))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankUser FROM Posts p WHERE p.Score IS NOT NULL AND p.Score > 0),
// UserVotes AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosedDate, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// CTE_AggregatedData AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.RankScore, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(cp.LastClosedDate, NULL) AS LastClosedDate,
//        COALESCE(cp.CloseReasons, 'No close reasons') AS CloseReasons, COALESCE(uv.VoteCount, 0) AS TotalVotes, COALESCE((uv.UpVotes - uv.DownVotes), 0) AS NetVotes
//     FROM RankedPosts rp LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId LEFT JOIN UserVotes uv ON rp.PostId = uv.UserId)
// SELECT PostId, Title, Score, CreationDate, RankScore, CommentCount, LastClosedDate, CloseReasons, TotalVotes, NetVotes, CASE WHEN LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM CTE_AggregatedData WHERE RankScore <= 5 ORDER BY Score DESC, CreationDate DESC LIMIT 100;
//
// `rp.PostId = uv.UserId` joins a post id to a user id, so it goes through the raw ids. RankScore ties go to the smaller post id; close reasons are in history id order.
fn q22810(db: &'static So) -> String {
    let Post { score, post_type_id, creation_date, origid, .. } = &db.post;
    let w = db.post.with(score.gt(0)).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    type R = (Id<Post>, i64);
    let rr: MatSet<R> = (&w).map(|((p, _), k)| (p, k)).collect();
    let uv = db.vote.group_by(&db.vote.user_id).select(vtype_name(db)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == "UpMod") as i64, a[2] + (t == "DownMod") as i64]);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(&db.post_history.post)
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|v| (v.iter().map(|x| x.0).max().unwrap(), leak(v.iter().map(|x| x.1).collect::<Vec<_>>().join(", "))));
    let v = drain((&rr).filt(|x: R| x.1 <= 5).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&cc).opt().and((&cp).opt()).and(origid.select((&uv).opt()))))));
    let v = top_n(v, |&(_, ((p, _), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(_, ((p, k), ((c, cl), u)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend([V::I(k), V::I(c.unwrap_or(0)), ots(cl.map(|x| x.0)), V::S(cl.map_or("No close reasons", |x| x.1))]);
        f.extend([V::I(u.map_or(0, |u| u[0])), V::I(u.map_or(0, |u| u[1] - u[2]))]);
        f.push(V::S(if cl.is_some() { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostDetails AS (SELECT rp.PostID, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostID) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostID AND v.VoteTypeId IN (2, 10)), 0) AS UpvoteCount FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, STRING_AGG(ph.Comment, ', ') AS Comments, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId IN (10, 11, 12)) AS ChangeCount
//     FROM PostHistory ph WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years' GROUP BY ph.PostId, ph.PostHistoryTypeId, ph.CreationDate),
// FinalResults AS (SELECT pd.PostID, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.UpvoteCount, COALESCE(phd.Comments, 'No changes') AS HistoryComments,
//        COALESCE(phd.ChangeCount, 0) AS HistoryChangeCount FROM PostDetails pd LEFT JOIN PostHistoryDetails phd ON pd.PostID = phd.PostId)
// SELECT FR.PostID, FR.Title, FR.CreationDate, FR.Score, FR.ViewCount, FR.AnswerCount, FR.CommentCount, FR.UpvoteCount, FR.HistoryComments, FR.HistoryChangeCount,
//        CASE WHEN FR.HistoryChangeCount > 0 THEN 'Modified' WHEN FR.CommentCount < 5 THEN 'Low Engagement' WHEN FR.Score = 0 THEN 'Neutral' ELSE 'Popular' END AS EngagementStatus
// FROM FinalResults FR WHERE FR.ViewCount > 100 ORDER BY FR.Score DESC, FR.ViewCount DESC;
//
// Rank ties go to the smaller post id. History groups are re-keyed by post; a group's comments are in history id order.
fn q21649(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let pd: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let cc = (&pd).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let t = &db.vote.vote_type_id;
    let uc = (&pd).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(t.eq(2).or(t.eq(10)))).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id: ht, creation_date: hd, comment, .. } = &db.post_history;
    let phd = db.post_history.with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -2))).group_by(post.and(ht).and(hd)).select(comment.opt()).buf_fold(|v| agg_str(v.iter().flatten().copied(), ", "));
    let pr = rel(drain(&phd));
    type H = (((Id<Post>, i64), i64), Option<Str>);
    let hidx: HashIdx<Id<Post>, H> = (&pr).map(|x: H| x.0 .0 .0).inv().select(&pr).collect();
    let phc = db.post_history.with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -2))).group_by(post.and(ht).and(hd)).select(ht).fold(0i64, |n, t| n + (t == 10 || t == 11 || t == 12) as i64);
    let v = drain((&pd).with(view_count.gt(100)).select(Ident::<Post>::new().and(&cc).and(&uc).and((&hidx).select(Same::<H>::new().and(Same::<H>::new().map(|x: H| x.0).select(&phc))).opt())));
    let mut v = v;
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(_, (((p, c), u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        let n = h.map_or(0, |h| h.1);
        f.extend([V::I(c), V::I(u), V::S(h.and_then(|h| h.0 .1).unwrap_or("No changes")), V::I(n)]);
        let s = score.get(p).unwrap();
        f.push(V::S(if n > 0 { "Modified" } else if c < 5 { "Low Engagement" } else if s == 0 { "Neutral" } else { "Popular" }));
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
    ("21004", q21004),
    ("24054", q24054),
    ("3769", q3769),
    ("3242", q3242),
    ("20256", q20256),
    ("26014", q26014),
    ("25000", q25000),
    ("27667", q27667),
    ("32480", q32480),
    ("21470", q21470),
    ("31981", q31981),
    ("2951", q2951),
    ("23429", q23429),
    ("28157", q28157),
    ("32846", q32846),
    ("30947", q30947),
    ("32482", q32482),
    ("24381", q24381),
    ("32224", q32224),
    ("20758", q20758),
    ("28038", q28038),
    ("1702", q1702),
    ("30299", q30299),
    ("27474", q27474),
    ("33063", q33063),
    ("28101", q28101),
    ("2298", q2298),
    ("30679", q30679),
    ("32522", q32522),
    ("8118", q8118),
    ("21045", q21045),
    ("21565", q21565),
    ("29566", q29566),
    ("20809", q20809),
    ("3348", q3348),
    ("22862", q22862),
    ("32181", q32181),
    ("24057", q24057),
    ("21981", q21981),
    ("21999", q21999),
    ("22810", q22810),
    ("21649", q21649),
];
