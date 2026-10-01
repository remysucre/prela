use harness::prelude::*;
use std::cmp::Reverse;

// WITH RecursiveUserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, u.CreationDate, u.LastAccessDate, 0 AS TotalPosts, 0 AS TotalVotes, 0 AS UserLevel
//     FROM Users u WHERE u.Id IS NOT NULL
//     UNION ALL
//     SELECT u.Id, u.DisplayName, u.Reputation, u.Views, u.CreationDate, u.LastAccessDate, COUNT(p.Id) AS TotalPosts, SUM(v.BountyAmount) AS TotalVotes,
//        CASE WHEN COUNT(p.Id) > 50 THEN 3 WHEN COUNT(p.Id) > 20 THEN 2 ELSE 1 END AS UserLevel
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views, u.CreationDate, u.LastAccessDate),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, p.AcceptedAnswerId,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount FROM Posts p),
// EngagementStats AS (SELECT pd.*, us.DisplayName AS OwnerDisplayName, us.Reputation AS OwnerReputation, COALESCE(ub.BadgeCount, 0) AS UserBadgeCount,
//        COALESCE(ub.BadgeNames, 'No badges') AS UserBadges, pd.ViewCount * 1.0 / NULLIF(pd.CommentCount, 0) AS ViewPerComment,
//        ROW_NUMBER() OVER (PARTITION BY pd.OwnerUserId ORDER BY pd.Score DESC) AS PostRank
//     FROM PostDetails pd JOIN RecursiveUserStats us ON pd.OwnerUserId = us.UserId LEFT JOIN UserBadges ub ON us.UserId = ub.UserId),
// AggregatedData AS (SELECT OwnerDisplayName, COUNT(PostId) AS TotalPosts, SUM(ViewCount) AS TotalViews, SUM(Score) AS TotalScore,
//        MAX(ViewPerComment) AS MaxViewPerComment, MIN(PostRank) AS BestPostRank FROM EngagementStats GROUP BY OwnerDisplayName)
// SELECT ad.OwnerDisplayName, ad.TotalPosts, ad.TotalViews, ad.TotalScore, ad.MaxViewPerComment, ad.BestPostRank,
//        CASE WHEN ad.TotalPosts >= 100 THEN 'Expert' WHEN ad.TotalPosts >= 50 THEN 'Veteran' ELSE 'Novice' END AS UserExperienceLevel
// FROM AggregatedData ad ORDER BY ad.TotalScore DESC, ad.TotalViews ASC;
//
// Not recursive: the CTE is a UNION ALL of every user and every user with a post, so a post meets its owner twice. The UserBadges join
// is on its group key and nothing from it reaches the output.
fn q34241(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let us: HashIdx<Id<User>, i64> = db.user.map(|_| 0i64).union(db.user.with(posts_of(db)).map(|_| 1i64)).collect();
    type J = (Id<Post>, i64);
    let j: MatSet<J> = db.post.select(Ident::<Post>::new().and(owner_user.select(&us))).collect();
    let p = || Same::<J>::new().map(|x: J| x.0);
    let cpp = comments_per_post(db);
    let w = (&j)
        .group_by(p().select(owner_user))
        .select(Same::<J>::new().and(p().select(score)).and(p().select(view_count).opt()).and(p().select(&cpp)))
        .window(row_number, |(((_, s), _), _)| s, desc);
    type W = ((((J, i64), Option<i64>), i64), i64);
    let g = w
        .group_by(Same::<W>::new().map(|x: W| x.0 .0 .0 .0 .0).select(owner_user).select(&db.user.display_name))
        .fold((0i64, 0i64, 0i64, 0i64, None::<f64>, i64::MAX), |(n, vs, vn, ss, m, r), ((((_, s), v), c), k)| {
            let vpc = match (v, c) {
                (Some(v), c) if c != 0 => Some(v as f64 / c as f64),
                _ => None,
            };
            let m = match (m, vpc) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
            (n + 1, vs + v.unwrap_or(0), vn + v.is_some() as i64, ss + s, m, r.min(k))
        });
    rows(drain(&g).into_iter().map(|(name, (n, vs, vn, ss, m, r))| {
        let lvl = if n >= 100 { "Expert" } else if n >= 50 { "Veteran" } else { "Novice" };
        row(vec![V::S(name), V::I(n), nullable(vs, vn), V::I(ss), ofloat(m), V::I(r), V::S(lvl)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.ViewCount, p.Score, p.CreationDate,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS ScoreRank, COALESCE(NULLIF(p.Title, ''), '(No Title)') AS DisplayTitle,
//        ARRAY(SELECT DISTINCT UNNEST(string_to_array(p.Tags, '>')) ORDER BY 1) AS TagList
//     FROM Posts p WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' AND p.ViewCount IS NOT NULL),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldCount, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverCount,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ub.GoldCount, 0) AS GoldCount, COALESCE(ub.SilverCount, 0) AS SilverCount,
//        COALESCE(ub.BronzeCount, 0) AS BronzeCount, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE u.LastAccessDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days'),
// FilteredPosts AS (SELECT rp.PostId, rp.ViewCount, rp.Score, rp.DisplayTitle, rp.TagList, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        CASE WHEN u.Reputation >= 1000 THEN 'High Reputation' WHEN u.Reputation < 1000 AND u.Reputation > 100 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.ScoreRank <= 5),
// FinalOutput AS (SELECT fp.DisplayTitle, fp.ViewCount, fp.Score, fp.OwnerDisplayName, fp.OwnerReputation, fp.ReputationCategory, fp.TagList,
//        COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes
//     FROM FilteredPosts fp LEFT JOIN Comments c ON fp.PostId = c.PostId LEFT JOIN Votes v ON fp.PostId = v.PostId
//     GROUP BY fp.DisplayTitle, fp.ViewCount, fp.Score, fp.OwnerDisplayName, fp.OwnerReputation, fp.ReputationCategory, fp.TagList)
// SELECT fo.DisplayTitle, fo.ViewCount, fo.Score, fo.OwnerDisplayName, fo.OwnerReputation, fo.ReputationCategory, fo.TagList, fo.CommentCount, fo.TotalUpVotes
// FROM FinalOutput fo WHERE fo.CommentCount > (SELECT AVG(CommentCount) FROM FinalOutput) ORDER BY fo.Score DESC, fo.ViewCount DESC LIMIT 100;
//
// ScoreRank reads only base columns, so the posts are ranked first. ActiveUsers and UserBadges are never read.
fn q22668(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user_id, owner_user, score, title, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(view_count)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(view_count))
        .window(rank, |((_, s), w)| (s, w), desc)
        .filt(|(_, r)| r <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let dt = title.opt().map(|t: Option<Str>| match t {
        Some(t) if !t.is_empty() => t,
        _ => "(No Title)",
    });
    let tlf = (&tp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split('>'))).buf_fold(|mut v| -> &'static [Str] {
        v.sort();
        v.dedup();
        Box::leak(v.to_vec().into_boxed_slice())
    });
    let tl = (&tlf).opt().map(|o: Option<&'static [Str]>| o.unwrap_or(&[]));
    let g = (&tp)
        .group_by(dt.and(view_count).and(score).and(owner_user.select((&db.user.display_name).and(&db.user.reputation))).and(tl))
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .buf_fold(|v| {
            let mut c: Vec<Id<Comment>> = v.iter().filter_map(|x| x.0).collect();
            c.sort();
            c.dedup();
            (c.len() as i64, v.iter().filter(|x| x.1 == Some(2)).count() as i64)
        });
    let (s, n) = (&g).fold_flat((0i64, 0i64), |(s, n), (c, _)| (s + c, n + 1));
    let a = s as f64 / n as f64;
    let v = drain((&g).filt(|(c, _): (i64, i64)| c as f64 > a));
    let v = top_n(v, |&(((((_, w), s), _), _), _)| (Reverse(s), Reverse(w)), 100);
    rows(v.into_iter().map(|(((((t, w), s), (u, rep)), tl), (c, up))| {
        let cat = if rep >= 1000 { "High Reputation" } else if rep > 100 { "Medium Reputation" } else { "Low Reputation" };
        row(vec![V::S(t), V::I(w), V::I(s), V::S(u), V::I(rep), V::S(cat), V::L(tl.iter().map(|x| V::S(x)).collect()), V::I(c), V::I(up)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN (SELECT unnest(string_to_array(p.Tags, '<>')) AS tag FROM Posts p) AS tag ON TRUE JOIN Tags t ON tag = t.TagName
//     WHERE p.CreationDate > '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' AND p.Score >= 10
//     GROUP BY p.Id, pt.Name, p.Title, p.CreationDate, p.Score, p.ViewCount)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.PostRank, rp.Tags
// FROM RankedPosts rp WHERE rp.PostRank <= 5 ORDER BY rp.PostRank, rp.Score DESC;
//
// The tag subquery is uncorrelated, so every post meets the elements of every post's tag list; split on '<>' no element is a tag name.
fn q6783(db: &'static So) -> String {
    let Post { creation_date, score, view_count, tags_str, .. } = &db.post;
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let et: HashIdx<Id<Post>, Str> = tags_str.flat_map(|t: Str| t.split("<>")).select(&names).select(&db.tag.tag_name).collect();
    let base = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(score.ge(10));
    type X = ((Id<Post>, Option<Id<Comment>>), Str);
    let g = base
        .select(Ident::<Post>::new().and(comments_of(db).opt()))
        .cross(&et)
        .group_by(Same::<X>::new().map(|x: X| x.0 .0))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().map(|x| x.1).collect();
            t.sort();
            t.dedup();
            (v.iter().filter(|x| x.0 .1.is_some()).count() as i64, leak(t.join(", ")))
        });
    let w = db
        .post
        .with(&g)
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()).and(&g))
        .window(rank, |(((_, s), w), _)| (s, w), desc);
    let mut out = Vec::new();
    (&w).filt(|(_, k)| k <= 5).drive(|_, ((((p, _), _), (c, t)), k)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(k), V::S(t)]);
        out.push(row(f))
    });
    rows(out)
}

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn sorted_join(mut v: Vec<Str>, sep: &str) -> Option<Str> {
    v.sort();
    if v.is_empty() { None } else { Some(leak(v.join(sep))) }
}

fn distinct_list(v: impl IntoIterator<Item = Option<Str>>) -> V {
    let mut v: Vec<Option<Str>> = v.into_iter().collect();
    v.sort_by(|a, b| a.is_none().cmp(&b.is_none()).then(a.cmp(b)));
    v.dedup();
    V::L(v.into_iter().map(ostr).collect())
}

// WITH PopularQuestions AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, DENSE_RANK() OVER (ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// QuestionTags AS (SELECT p.Id AS PostId, STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags
//     FROM Posts p JOIN UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS t(TagName) ON TRUE WHERE p.PostTypeId = 1 GROUP BY p.Id),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 0)
// SELECT q.PostId, q.Title, q.Score, qt.Tags, u.DisplayName AS TopUser, u.Reputation
// FROM PopularQuestions q LEFT JOIN QuestionTags qt ON q.PostId = qt.PostId
// LEFT JOIN Users u ON u.Id = (SELECT UserId FROM Votes v WHERE v.PostId = q.PostId AND v.VoteTypeId = 2 ORDER BY v.CreationDate DESC LIMIT 1)
// WHERE q.RankByScore <= 10 ORDER BY q.Score DESC, q.CreationDate DESC;
//
// Ported from rewrites/249.sql (the STRING_AGG ordered by name). The correlated LIMIT 1 is an arg-max fold; a CreationDate tie goes to the larger vote id (the SQL leaves it open).
fn q249(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, creation_date, score, tags_str, .. } = &db.post;
    let pq = db.post.with(post_type_id.eq(1)).with(accepted_answer_id).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let tp: MatSet<Id<Post>> = whole(&pq)
        .select(Ident::<Post>::new().and(score))
        .window(dense_rank, |(_, s)| s, desc)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let qt = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(tags_str.flat_map(|t: Str| t[1..t.len() - 1].split("><")))
        .buf_fold(|v| sorted_join(v.to_vec(), ", ").unwrap());
    let Vote { vote_type_id, creation_date: vd, user, .. } = &db.vote;
    let lv = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2)).and(vd)))
        .fold(None, |a: Option<(i64, Id<Vote>)>, (v, d)| a.max(Some((d, v))));
    let lu = (&lv).map(|a: Option<(i64, Id<Vote>)>| a.unwrap().1).select(user.opt());
    let v = drain((&tp).select(Ident::<Post>::new().and((&qt).opt()).and(lu.opt())));
    rows(v.into_iter().map(|(_, ((p, t), u))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.push(ostr(t));
        match u.flatten() {
            Some(u) => f.extend(ucols(db, u, &["name", "rep"])),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, ARRAY_AGG(DISTINCT t.TagName) AS Tags,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(v.Id) DESC) AS VoteRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     LEFT JOIN LATERAL unnest(string_to_array(p.Tags, '><')) AS tag(tag) ON true LEFT JOIN Tags t ON t.TagName = tag.tag
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.OwnerUserId),
// RecentActivePosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.OwnerUserId, rp.CommentCount, rp.Tags, rp.VoteRank,
//        ROW_NUMBER() OVER (ORDER BY p.LastActivityDate DESC) AS RecentRank
//     FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id WHERE p.LastActivityDate >= CURRENT_DATE - INTERVAL '30 days')
// SELECT u.DisplayName, ra.PostId, ra.Title, ra.Body, ra.CreationDate, ra.CommentCount, ra.Tags, ra.VoteRank
// FROM RecentActivePosts ra JOIN Users u ON ra.OwnerUserId = u.Id WHERE ra.RecentRank <= 10 ORDER BY ra.CommentCount DESC, ra.VoteRank ASC;
fn q28267(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user_id, owner_user, last_activity_date, .. } = &db.post;
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tags = tags_str.flat_map(|t: Str| t.split("><")).select((&names).select(&db.tag.tag_name).opt()).opt();
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(up.opt()).and(tags))
        .buf_fold(|v| {
            let l: &'static [Option<Str>] = Box::leak(v.iter().map(|x| x.1.flatten()).collect::<Vec<_>>().into_boxed_slice());
            (v.iter().filter(|x| x.0 .0.is_some()).count() as i64, v.iter().filter(|x| x.0 .1.is_some()).count() as i64, l)
        });
    let recent = db.post.with(post_type_id.eq(1)).with(last_activity_date.ge(add_days(current_date(), -30)));
    let top: MatSet<Id<Post>> = whole(&recent)
        .select(Ident::<Post>::new().and(last_activity_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(&rp).and((&top).opt()).and(owner_user.opt()))
        .window(rank, |(((_, (_, n, _)), _), _)| n, desc);
    let mut out = Vec::new();
    (&w).filt(|(((_, t), u), _)| t.is_some() && u.is_some()).drive(|_, ((((p, (c, _, l)), _), u), k)| {
        let mut f = ucols(db, u.unwrap(), &["name"]);
        f.extend(post_fields(db, p, &["id", "title", "body", "created"]));
        f.extend([V::I(c), distinct_list(l.iter().copied()), V::I(k)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes, p.CreationDate, p.LastActivityDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, pt.Name AS PostType,
//        ARRAY_AGG(DISTINCT t.TagName) AS Tags
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN (SELECT UNNEST(string_to_array(p.Tags, '<>')) AS TagName) AS t ON TRUE
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, p.OwnerUserId, u.DisplayName, pt.Name),
// RankedPosts AS (SELECT *, ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostStats)
// SELECT PostId, Title, Score, ViewCount, CommentCount, UpVotes, DownVotes, CreationDate, LastActivityDate, OwnerUserId, OwnerDisplayName, PostType, Tags
// FROM RankedPosts WHERE Rank <= 10 ORDER BY Score DESC, ViewCount DESC;
//
// Rank reads only base columns (one PostStats row per post), so the ten posts are picked first.
fn q5919(db: &'static So) -> String {
    let Post { creation_date, score, view_count, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = whole(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (s, w, Reverse(p)), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let g = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(tags_str.flat_map(|t: Str| t.split("<>")).opt()))
        .buf_fold(|v| {
            let l: &'static [Option<Str>] = Box::leak(v.iter().map(|x| x.1).collect::<Vec<_>>().into_boxed_slice());
            (v.iter().filter(|x| x.0 .0.is_some()).count() as i64, v.iter().filter(|x| x.0 .1 == Some(2)).count() as i64, v.iter().filter(|x| x.0 .1 == Some(3)).count() as i64, l)
        });
    rows(drain(&g).into_iter().map(|(p, (c, u, d, l))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        f.extend(post_fields(db, p, &["created", "activity", "owner_id", "owner", "type"]));
        f.push(distinct_list(l.iter().copied()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoterCount,
//        STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN UNNEST(string_to_array(p.Tags, '>')) AS t(TagName) ON TRUE
//     WHERE p.PostTypeId = 1 AND p.Score > 0 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// TopPerformers AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CommentCount, rp.UniqueVoterCount, rp.ViewCount, rp.Tags FROM RankedPosts rp WHERE rp.UserPostRank = 1
//     ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10)
// SELECT u.DisplayName, u.Reputation, tp.Title, tp.Score, tp.CommentCount, tp.UniqueVoterCount, tp.ViewCount, tp.Tags
// FROM Users u JOIN TopPerformers tp ON u.Id = (SELECT DISTINCT p.OwnerUserId FROM Posts p WHERE p.Id = tp.PostId) ORDER BY u.Reputation DESC;
//
// Ported from rewrites/29390.sql (the STRING_AGG ordered by name). Both ranks read only base columns, so the ten posts are picked first;
// a CreationDate tie inside a user goes to the larger post id (the SQL leaves it open). The scalar subquery is the post's owner.
fn q29390(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user_id, owner_user, view_count, tags_str, .. } = &db.post;
    let first = db
        .post
        .with(post_type_id.eq(1))
        .with(score.gt(0))
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date).and(score).and(view_count.opt()))
        .window(row_number, |(((p, d), _), _)| (d, p), desc);
    let first = drain((&first).filt(|(_, n)| n == 1));
    let top = top_n(first, |&(_, ((((_, _), s), w), _))| (Reverse(s), w.is_none(), Reverse(w)), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.1 .0 .0 .0 .0).collect()).map(|p| p).collect();
    let g = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.user_id).opt()).opt()).and(tags_str.flat_map(|t: Str| t.split('>')).opt()))
        .buf_fold(|v| {
            let mut u: Vec<i64> = v.iter().filter_map(|x| x.0 .1.flatten()).collect();
            u.sort();
            u.dedup();
            (v.iter().filter(|x| x.0 .0.is_some()).count() as i64, u.len() as i64, sorted_join(v.iter().filter_map(|x| x.1).collect(), ", "))
        });
    let v = drain(db.post.select(Ident::<Post>::new().and(&g).and(owner_user)));
    rows(v.into_iter().map(|(_, ((p, (c, n, t)), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), V::I(n)]);
        f.extend(post_fields(db, p, &["views"]));
        f.push(ostr(t));
        row(f)
    }))
}

// WITH PostTagCount AS (SELECT p.Id AS PostId, COUNT(DISTINCT t.TagName) AS TagCount, STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags
//     FROM Posts p INNER JOIN UNNEST(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) AS t(TagName) ON TRUE WHERE p.PostTypeId = 1 GROUP BY p.Id),
// PopularUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.ViewCount) AS TotalViews, COUNT(DISTINCT p.Id) AS QuestionsAnswered
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId IN (2, 1) GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5),
// RecentPostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(ph.Comment, 'No comments') AS RecentComment,
//        RANK() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS CommentRank
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.PostTypeId IN (1, 2))
// SELECT pt.PostId, ra.Title, pt.TagCount, pt.Tags, pu.DisplayName AS ActiveUser, pu.TotalViews, ra.RecentComment
// FROM PostTagCount pt JOIN RecentPostActivity ra ON pt.PostId = ra.PostId JOIN PopularUsers pu ON pu.QuestionsAnswered > 5 AND pu.UserId = ra.PostId
// WHERE ra.CommentRank = 1 ORDER BY pt.TagCount DESC, pu.TotalViews DESC;
//
// Ported from rewrites/28140.sql (the STRING_AGG ordered by name). `pu.UserId = ra.PostId` compares a user id with a post id, so it goes through the raw ids.
fn q28140(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, origid, .. } = &db.post;
    let ptc = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(tags_str.flat_map(|t: Str| t[1..t.len() - 1].split("><")))
        .buf_fold(|v| {
            let mut d: Vec<Str> = v.to_vec();
            d.sort();
            d.dedup();
            (d.len() as i64, sorted_join(v.to_vec(), ", ").unwrap())
        });
    let PostHistory { post, creation_date: hd, comment, .. } = &db.post_history;
    let w = db.post_history.group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(rank, |(_, d)| d, desc);
    let latest: HashIdx<Id<Post>, Id<PostHistory>> = (&w).filt(|(_, r): ((Id<PostHistory>, i64), i64)| r == 1).map(|((h, _), _)| h).collect();
    let ra = (&latest).select(comment.opt()).opt();
    let pu = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.is_in([1, 2])).and(view_count.opt())))
        .fold((0i64, 0i64, 0i64), |(n, s, k), (_, w)| (n + 1, s + w.unwrap_or(0), k + w.is_some() as i64));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pu5 = (&pu).filt(|a: (i64, i64, i64)| a.0 > 5);
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).select(Ident::<Post>::new().and(&ptc).and(ra).and(origid.select(&uidx).select(Ident::<User>::new().and(&pu5)))));
    rows(v.into_iter().map(|(_, (((p, (n, t)), c), (u, (_, s, k))))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(n), V::S(t), user_col(db, u, "name"), nullable(s, k), V::S(c.flatten().unwrap_or("No comments"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.UserId) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.Score, u.DisplayName),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.RowNum = 1)
// SELECT fp.PostId, fp.Title, fp.Score, fp.CommentCount, fp.VoteCount,
//        CASE WHEN fp.Score > 100 THEN 'High Score' WHEN fp.Score BETWEEN 50 AND 100 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory, STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags
// FROM FilteredPosts fp LEFT JOIN LATERAL (SELECT unnest(string_to_array(substring(fp.Body, 2, length(fp.Body)-2), '><')) AS TagName) AS t ON TRUE
// GROUP BY fp.PostId, fp.Title, fp.Score, fp.CommentCount, fp.VoteCount ORDER BY fp.Score DESC, fp.CommentCount DESC;
//
// Ported from rewrites/29901.sql (the STRING_AGG ordered by the piece). RowNum partitions by the post, one row each, so it is always 1.
fn q29901(db: &'static So) -> String {
    let Post { post_type_id, body, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).select((&db.vote.user_id).opt()).opt()))
        .buf_fold(|v| {
            let mut u: Vec<i64> = v.iter().filter_map(|x| x.1.flatten()).collect();
            u.sort();
            u.dedup();
            (v.iter().filter(|x| x.0.is_some()).count() as i64, u.len() as i64)
        });
    let pieces = body.flat_map(|b: Str| {
        let n = b.chars().count();
        let s: String = b.chars().skip(1).take(n.saturating_sub(2)).collect();
        leak(s).split("><")
    });
    let tg = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(pieces).buf_fold(|v| sorted_join(v.to_vec(), ", ").unwrap());
    let v = drain(db.post.select(Ident::<Post>::new().and(&rp).and((&tg).opt())));
    rows(v.into_iter().map(|(_, ((p, (c, n)), t))| {
        let s = db.post.score.get(p).unwrap();
        let cat = if s > 100 { "High Score" } else if (50..=100).contains(&s) { "Moderate Score" } else { "Low Score" };
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(c), V::I(n), V::S(cat), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.PostRank = 1),
// PostStats AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        ARRAY_AGG(DISTINCT tag.TagName) AS Tags
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     LEFT JOIN (SELECT UNNEST(STRING_TO_ARRAY(Tags, ',')) AS TagName, Id FROM Posts) AS tag ON tp.PostId = tag.Id
//     GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount)
// SELECT ps.*, (SELECT COUNT(*) FROM Badges b WHERE b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = ps.PostId)) AS BadgeCount
// FROM PostStats ps ORDER BY ps.Score DESC, ps.ViewCount DESC LIMIT 10;
//
// Both the rank and the LIMIT read only base columns (one PostStats row per post), so the ten posts are picked first. A CreationDate tie inside
// a user goes to the larger post id (the SQL leaves it open).
fn q9270(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, tags_str, .. } = &db.post;
    let first = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date).and(score).and(view_count.opt()))
        .window(row_number, |(((p, d), _), _)| (d, p), desc);
    let first = drain((&first).filt(|(_, n)| n == 1));
    let top = top_n(first, |&(_, ((((_, _), s), w), _))| (Reverse(s), w.is_none(), Reverse(w)), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.1 .0 .0 .0 .0).collect()).map(|p| p).collect();
    let bpu = badges_per_user(db);
    let g = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(tags_str.flat_map(|t: Str| t.split(',')).opt()))
        .buf_fold(|v| {
            let l: &'static [Option<Str>] = Box::leak(v.iter().map(|x| x.1).collect::<Vec<_>>().into_boxed_slice());
            (v.iter().filter(|x| x.0 .0.is_some()).count() as i64, v.iter().filter(|x| x.0 .1.is_some()).count() as i64, l)
        });
    let v = drain(db.post.select(Ident::<Post>::new().and(&g).and(owner_user.select(&bpu))));
    rows(v.into_iter().map(|(_, ((p, (c, n, l)), b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n), distinct_list(l.iter().copied()), V::I(b)]);
        row(f)
    }))
}

fn inner_tags(t: Str) -> std::str::Split<'static, &'static str> {
    t[1..t.len() - 1].split("><")
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostTags AS (SELECT p.Id AS PostId, STRING_AGG(t.TagName, ', ') AS Tags FROM Posts p
//     JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags FROM 2 FOR length(p.Tags) - 2), '><')) AS tag) AS tag ON TRUE JOIN Tags t ON t.TagName = tag
//     WHERE p.PostTypeId = 1 GROUP BY p.Id)
// SELECT p.PostId, p.Title, p.CreationDate, p.Score, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName, u.Upvotes, u.Downvotes, pt.Tags,
//        CASE WHEN p.Score >= 10 THEN 'High Score' WHEN p.Score >= 5 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM RankedPosts p LEFT JOIN UserStats u ON p.OwnerUserId = u.UserId LEFT JOIN PostTags pt ON p.PostId = pt.PostId
// WHERE p.rn = 1 AND (u.Upvotes - u.Downvotes) > 0 ORDER BY p.Score DESC, p.CreationDate ASC LIMIT 100;
//
// rn reads only base columns, so the posts are ranked first; a CreationDate tie inside a user goes to the larger post id. The STRING_AGG
// order is left open by the SQL; the port joins in tag-list order.
fn q3347(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, owner_user, score, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, p), desc)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .collect();
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pt = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(tags_str.flat_map(inner_tags).select(&names).select(&db.tag.tag_name)).buf_fold(|v| leak(v.join(", ")));
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&us)).filt(|(_, a): (Id<User>, [i64; 2])| a[0] - a[1] > 0)).and((&pt).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()), 100);
    rows(v.into_iter().map(|(_, ((p, (u, a)), t))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), ostr(t)]);
        f.push(V::S(if s >= 10 { "High Score" } else if s >= 5 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        COUNT(v.Id) AS VoteCount, RANK() OVER (ORDER BY COUNT(v.Id) DESC) AS VoteRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName, u.Reputation),
// PostTagCounts AS (SELECT p.Id AS PostId, UNNEST(STRING_TO_ARRAY(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.Tags IS NOT NULL),
// TagUsage AS (SELECT Tag, COUNT(PostId) AS TagCount FROM PostTagCounts GROUP BY Tag ORDER BY TagCount DESC LIMIT 10),
// TopPosts AS (SELECT rp.*, STRING_AGG(t.Tag, ', ') AS UsedTags FROM RankedPosts rp JOIN PostTagCounts pt ON rp.PostId = pt.PostId JOIN TagUsage t ON pt.Tag = t.Tag
//     GROUP BY rp.PostId, rp.Title, rp.Body, rp.Tags, rp.CreationDate, rp.OwnerDisplayName, rp.OwnerReputation, rp.VoteCount, rp.VoteRank)
// SELECT tp.PostId, tp.Title, tp.Body, tp.CreationDate, tp.OwnerDisplayName, tp.OwnerReputation, tp.VoteCount, tp.VoteRank, tp.UsedTags
// FROM TopPosts tp WHERE tp.VoteRank <= 5 ORDER BY tp.VoteCount DESC;
//
// The STRING_AGG order is left open by the SQL; the port joins in tag-list order.
fn q26925(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, .. } = &db.post;
    let vpp = votes_per_post(db);
    let rv: MatSet<R> = whole(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user))
        .select(Ident::<Post>::new().and(&vpp))
        .window(rank, |(_, n)| n, desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, n), k)| (p, n, k))
        .collect();
    type R = (Id<Post>, i64, i64);
    let tu = db.post.select(tags_str.flat_map(inner_tags)).group_by(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tu: MatSet<Str> = rel(top_n(drain(&tu), |&(_, n)| Reverse(n), 10).into_iter().map(|x| x.0).collect()).map(|t| t).collect();
    let g = (&rv).group_by(Same::<R>::new()).select(Same::<R>::new().map(|x: R| x.0).select(tags_str.flat_map(inner_tags).with(&tu))).buf_fold(|v| leak(v.join(", ")));
    rows(drain(&g).into_iter().map(|((p, n, k), t)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner", "rep"]);
        f.extend([V::I(n), V::I(k), V::S(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN pb.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN pb.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts pb ON u.Id = pb.OwnerUserId LEFT JOIN Votes v ON pb.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PostTags AS (SELECT p.Id AS PostId, STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS TagNames FROM Posts p JOIN UNNEST(string_to_array(p.Tags, '>')) AS t(TagName) ON TRUE GROUP BY p.Id)
// SELECT us.DisplayName, us.TotalQuestions, us.TotalAnswers, us.TotalUpvotes, us.TotalDownvotes, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount,
//        rp.CommentCount, pt.TagNames
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId JOIN PostTags pt ON rp.PostId = pt.PostId WHERE rp.PostRank = 1 ORDER BY us.TotalUpvotes DESC, us.DisplayName;
//
// Ported from rewrites/28042.sql (the STRING_AGG ordered by name). PostRank reads only base columns, so the posts are ranked first; a
// CreationDate tie inside a user goes to the larger post id (the SQL leaves it open).
fn q28042(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, p), desc)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pt = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split('>'))).buf_fold(|v| sorted_join(v.to_vec(), ", ").unwrap());
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&us))).and(&pt)));
    rows(v.into_iter().map(|(_, ((p, (u, a)), t))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]));
        f.push(V::S(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, ARRAY_AGG(t.TagName ORDER BY t.TagName) AS TagsArray,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount, RANK() OVER (ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId
//     LEFT JOIN LATERAL (SELECT unnest(string_to_array(SUBSTRING(p.Tags, 2, LENGTH(p.Tags) - 2), '><')) AS TagName) AS t ON TRUE
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.Score, rp.TagsArray, rp.CommentCount, rp.AnswerCount,
//        ROW_NUMBER() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS ScoreRank FROM RankedPosts rp WHERE rp.PostRank <= 50)
// SELECT tp.Title, tp.Body, tp.CreationDate, tp.ViewCount, tp.Score, tp.TagsArray, tp.CommentCount, tp.AnswerCount, u.DisplayName AS OwnerDisplayName, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM TopPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Class = 1 GROUP BY UserId) b ON b.UserId = u.Id
// WHERE EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = tp.PostId AND v.VoteTypeId = 2) ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Ported from rewrites/27466.sql (the ARRAY_AGG ordered by name). PostRank reads only CreationDate, so the posts are ranked first; ScoreRank is never read.
fn q27466(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = whole(db.post.with(post_type_id.eq(1)))
        .select(Ident::<Post>::new().and(creation_date))
        .window(rank, |(_, d)| d, desc)
        .filt(|(_, k)| k <= 50)
        .map(|((p, _), _)| p)
        .collect();
    let g = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()).and(tags_str.flat_map(inner_tags).opt()))
        .buf_fold(|v| {
            let mut c: Vec<Id<Comment>> = v.iter().filter_map(|x| x.0 .0).collect();
            c.sort();
            c.dedup();
            let mut a: Vec<Id<Post>> = v.iter().filter_map(|x| x.0 .1).collect();
            a.sort();
            a.dedup();
            let mut t: Vec<Option<Str>> = v.iter().map(|x| x.1).collect();
            t.sort_by(|a, b| a.is_none().cmp(&b.is_none()).then(a.cmp(b)));
            let t: &'static [Option<Str>] = Box::leak(t.into_boxed_slice());
            (t, c.len() as i64, a.len() as i64)
        });
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let b = (&owners).group_by(Ident::<User>::new()).select(gold).fold(0i64, |n, _| n + 1);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let v = drain((&tp).with(up).select(Ident::<Post>::new().and(&g).and(owner_user.select(Ident::<User>::new().and((&b).opt())))));
    rows(v.into_iter().map(|(_, ((p, (t, c, a)), (u, n)))| {
        let mut f = post_fields(db, p, &["title", "body", "created", "views", "score"]);
        f.extend([V::L(t.iter().map(|&x| ostr(x)).collect()), V::I(c), V::I(a), user_col(db, u, "name"), V::I(n.unwrap_or(0))]);
        row(f)
    }))
}

fn text_f64(x: f64) -> String {
    if x.fract() == 0.0 { format!("{x:.1}") } else { format!("{x}") }
}

// WITH RECURSIVE UserScoreCTE AS (SELECT Id, Reputation, 0 AS Level FROM Users WHERE Reputation > 1000
//     UNION ALL SELECT u.Id, u.Reputation, us.Level + 1 FROM Users u JOIN UserScoreCTE us ON u.Reputation > us.Reputation AND us.Level < 3),
// PostRankings AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// UserWithTopPost AS (SELECT u.DisplayName, up.PostId, up.Title, ps.CloseCount FROM Users u JOIN PostRankings up ON u.Id = up.OwnerUserId
//     LEFT JOIN ClosedPosts ps ON up.PostId = ps.PostId WHERE up.Rank = 1)
// SELECT u.DisplayName, u.Reputation, COALESCE(ps.CloseCount, 0) AS CloseCount, COUNT(b.Id) AS BadgeCount, STRING_AGG(DISTINCT p.Title, ', ') AS TopPosts
// FROM Users u LEFT JOIN UserWithTopPost ps ON u.DisplayName = ps.DisplayName LEFT JOIN Badges b ON b.UserId = u.Id
// LEFT JOIN Posts p ON p.OwnerUserId = u.Id AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '60 days'
// WHERE u.Reputation IS NOT NULL AND u.Location IS NOT NULL GROUP BY u.DisplayName, u.Reputation, ps.CloseCount HAVING AVG(u.Reputation) >= 1000
// ORDER BY u.Reputation DESC LIMIT 10;
//
// The recursive CTE is never read. Rank reads only base columns, so the posts are ranked first; a tie goes to the larger post id. The distinct
// titles are joined in sorted order (the SQL leaves it open).
fn q33973(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, title, .. } = &db.post;
    let User { display_name, reputation, location, .. } = &db.user;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, p), desc)
        .filt(|(_, n)| n == 1)
        .map(|(((p, _), _), _)| p)
        .collect();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, (Id<Post>, Option<i64>)> =
        (&tp).select((&db.post.owner_user).select(display_name)).inv().select(Ident::<Post>::new().and((&cp).opt())).collect();
    type J = (Id<User>, Option<(Id<Post>, Option<i64>)>);
    let j: MatSet<J> = db.user.with(location).select(Ident::<User>::new().and(display_name.select(&by_name).opt())).collect();
    let u = || Same::<J>::new().map(|x: J| x.0);
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(date(2024, 10, 1), -60)))).select(title.opt());
    let g = (&j)
        .group_by(u().select(display_name.and(reputation)).and(Same::<J>::new().map(|x: J| x.1.and_then(|y| y.1))))
        .select(u().select(reputation.and(badges_of(db).opt()).and(recent.opt())))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1.flatten()).collect();
            t.sort();
            t.dedup();
            let rs: i64 = v.iter().map(|x| x.0 .0).sum();
            (v.iter().filter(|x| x.0 .1.is_some()).count() as i64, rs as f64 / v.len() as f64, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let v = drain((&g).filt(|(_, a, _): (i64, f64, Option<Str>)| a >= 1000.0));
    let v = top_n(v, |&(((_, r), _), _)| Reverse(r), 10);
    rows(v.into_iter().map(|(((n, r), c), (b, _, t))| row(vec![V::S(n), V::I(r), V::I(c.unwrap_or(0)), V::I(b), ostr(t)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, ARRAY_AGG(DISTINCT t.TagName) AS Tags, COUNT(c.Id) AS CommentCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN UNNEST(string_to_array(p.Tags, '>')) AS t(TagName) ON t.TagName IS NOT NULL WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// PopularTags AS (SELECT t.TagName AS PopularTag FROM Posts p JOIN UNNEST(string_to_array(p.Tags, '>')) AS t(TagName) ON t.TagName IS NOT NULL
//     GROUP BY t.TagName ORDER BY COUNT(*) DESC LIMIT 10),
// TagStatistics AS (SELECT pt.PopularTag, COUNT(rp.PostId) AS PostsCount, SUM(rp.UpVoteCount) AS TotalUpVotes, SUM(rp.DownVoteCount) AS TotalDownVotes,
//        AVG(rp.CommentCount) AS AverageComments FROM PopularTags pt LEFT JOIN RankedPosts rp ON rp.Tags @> ARRAY[pt.PopularTag] GROUP BY pt.PopularTag)
// SELECT ts.PopularTag, ts.PostsCount, ts.TotalUpVotes, ts.TotalDownVotes, ts.AverageComments,
//        CONCAT('Tag: ', ts.PopularTag, ' | Total Posts: ', ts.PostsCount, ' | UpVotes: ', ts.TotalUpVotes, ' | DownVotes: ', ts.TotalDownVotes,
//        ' | Average Comments: ', ROUND(ts.AverageComments, 2)) AS Summary
// FROM TagStatistics ts ORDER BY ts.PostsCount DESC;
//
// `rp.Tags @> ARRAY[tag]` is membership in the post's distinct tag pieces, so the posts are joined to the tags through those pieces.
fn q27947(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let pieces = || tags_str.flat_map(|t: Str| t.split('>'));
    type A = (&'static [Str], i64, i64, i64);
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(pieces().opt()))
        .buf_fold(|v| -> A {
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            t.sort();
            t.dedup();
            let t: &'static [Str] = Box::leak(t.into_boxed_slice());
            (t, v.iter().filter(|x| x.0 .0.is_some()).count() as i64, v.iter().filter(|x| x.0 .1 == Some(2)).count() as i64, v.iter().filter(|x| x.0 .1 == Some(3)).count() as i64)
        });
    let pc = db.post.select(pieces()).group_by(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let pop: MatSet<Str> = rel(top_n(drain(&pc), |&(_, n)| Reverse(n), 10).into_iter().map(|x| x.0).collect()).map(|t| t).collect();
    type T = (Id<Post>, (Str, A));
    let pt = rel(drain((&rp).flat_map(|a: A| a.0.iter().map(move |&t| (t, a)))));
    let by_tag: HashIdx<Str, A> = (&pt).map(|x: T| x.1 .0).inv().select((&pt).map(|x: T| x.1 .1)).collect();
    let ts = (&pop).group_by(Same::<Str>::new()).select((&by_tag).opt()).fold((0i64, 0i64, 0i64, 0i64), |(n, u, d, c), a| match a {
        Some(a) => (n + 1, u + a.2, d + a.3, c + a.1),
        None => (n, u, d, c),
    });
    rows(drain(&ts).into_iter().map(|(t, (n, u, d, c))| {
        let (uv, dv, av) = if n == 0 { (V::Null, V::Null, V::Null) } else { (V::I(u), V::I(d), V::F(c as f64 / n as f64)) };
        let txt = |x: String| if n == 0 { String::new() } else { x };
        let s = format!(
            "Tag: {t} | Total Posts: {n} | UpVotes: {} | DownVotes: {} | Average Comments: {}",
            txt(u.to_string()),
            txt(d.to_string()),
            txt(text_f64(((c as f64 / n as f64) * 100.0).round() / 100.0))
        );
        row(vec![V::S(t), V::I(n), uv, dv, av, V::Owned(s)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.Tags, COUNT(a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.Tags),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, u.DisplayName AS OwnerDisplayName, rp.AnswerCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END), 0) AS NetVotes, STRING_AGG(t.TagName, ', ') AS TagsAggregated
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN Votes v ON rp.PostId = v.PostId LEFT JOIN UNNEST(string_to_array(rp.Tags, ',')) AS t(TagName) ON TRUE
//     WHERE rp.rn = 1 GROUP BY rp.PostId, rp.Title, rp.Body, rp.CreationDate, u.DisplayName, rp.AnswerCount),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.OwnerDisplayName, ps.AnswerCount, ps.TotalUpVotes, ps.TotalDownVotes, ps.NetVotes, ps.TagsAggregated,
//        ROW_NUMBER() OVER (ORDER BY ps.NetVotes DESC, ps.TotalUpVotes DESC) AS PostRank FROM PostStatistics ps)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.AnswerCount, tp.TotalUpVotes, tp.TotalDownVotes, tp.NetVotes, tp.TagsAggregated
// FROM TopPosts tp WHERE tp.PostRank <= 10 ORDER BY tp.NetVotes DESC;
//
// rn partitions by the post, one row each, so it is always 1. Splitting on ',' leaves the tag string whole, one piece per post.
fn q26912(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, .. } = &db.post;
    let ac = answers_per_post(db);
    let ps = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(tags_str.flat_map(|t: Str| t.split(',')).opt()))
        .buf_fold(|v| {
            let u = v.iter().filter(|x| x.0 == Some(2)).count() as i64;
            let d = v.iter().filter(|x| x.0 == Some(3)).count() as i64;
            let t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            (u, d, u - d, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let w = whole(&ps).select(Ident::<Post>::new().and(&ps).and(&ac)).window(row_number, |((p, (u, _, n, _)), _)| (n, u, Reverse(p)), desc);
    let mut out = Vec::new();
    (&w).filt(|(_, k)| k <= 10).drive(|_, (((p, (u, d, n, t)), a), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a), V::I(u), V::I(d), V::I(n), ostr(t)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, COUNT(a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVotes,
//        STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS TagsList
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     LEFT JOIN LATERAL (SELECT STRING_TO_ARRAY(p.Tags, '>') AS TagArray) AS ta ON TRUE LEFT JOIN UNNEST(ta.TagArray) AS t(TagName) ON TRUE
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, '; ') AS Badges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// RecentActivity AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.AnswerCount, rp.NetVotes, rb.BadgeCount, ra.CommentCount, ra.LastCommentDate, rp.TagsList,
//        CASE WHEN ra.CommentCount > 0 THEN 'Active' ELSE 'Inactive' END AS PostActivityStatus,
//        CASE WHEN rp.OwnerPostRank = 1 AND rb.BadgeCount > 0 THEN 'Active Top User with Badges' ELSE 'Regular Post' END AS UserPostStatus
// FROM RankedPosts rp LEFT JOIN UserBadges rb ON rp.OwnerUserId = rb.UserId LEFT JOIN RecentActivity ra ON rp.PostId = ra.PostId
// WHERE rp.Score > 10 ORDER BY rp.Score DESC, rp.PostId LIMIT 50;
//
// Ported from rewrites/24653.sql (the STRING_AGG ordered by name). The rank and the LIMIT read only base columns, so the fifty posts are picked
// first; a CreationDate tie inside a user goes to the larger post id (the SQL leaves it open).
fn q24653(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, owner_user, score, origid, tags_str, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).with(score.gt(10)).select(score.and(origid))), |&(_, (s, o))| (Reverse(s), o), 50);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let g = (&tp)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).filt(|t| t == 2 || t == 3).opt()).and(tags_str.flat_map(|t: Str| t.split('>')).opt()))
        .buf_fold(|v| {
            let a = v.iter().filter(|x| x.0 .0.is_some()).count() as i64;
            let n = v.iter().filter(|x| x.0 .1 == Some(2)).count() as i64 - v.iter().filter(|x| x.0 .1 == Some(3)).count() as i64;
            (a, n, sorted_join(v.iter().filter_map(|x| x.1).collect(), ", "))
        });
    let bpu = badges_per_user(db);
    let Comment { post, creation_date: cd, .. } = &db.comment;
    let ra = db.comment.group_by(post).select(cd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date).and((&tp).opt()))
        .window(row_number, |((p, d), _)| (d, p), desc);
    type K = (Id<Post>, i64);
    let rk: MatSet<K> = (&w).filt(|((_, t), _)| t.is_some()).map(|(((p, _), _), k)| (p, k)).collect();
    let v = drain((&rk).select(Same::<K>::new().and(Same::<K>::new().map(|x: K| x.0).select((&g).and(owner_user.select(&bpu).opt()).and((&ra).opt())))));
    rows(v.into_iter().map(|(_, ((p, k), (((a, n, t), b), c)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a), V::I(n), oint(b), oint(c.map(|c| c.0)), ots(c.map(|c| c.1)), ostr(t)]);
        f.push(V::S(if c.map_or(false, |c| c.0 > 0) { "Active" } else { "Inactive" }));
        f.push(V::S(if k == 1 && b.map_or(false, |b| b > 0) { "Active Top User with Badges" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ARRAY_AGG(t.TagName) AS Tags, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT a.Id) AS AnswerCount, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     LEFT JOIN LATERAL UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS t(TagName) ON TRUE
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// PopularTags AS (SELECT tags.TagName, COUNT(*) AS PopularityCount FROM RankedPosts r CROSS JOIN UNNEST(r.Tags) AS tags(TagName) GROUP BY tags.TagName
//     ORDER BY PopularityCount DESC LIMIT 10),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.Tags, rp.CommentCount, rp.AnswerCount, pt.TagName AS PopularTag, ur.UserId,
//        ur.DisplayName AS UserDisplayName, ur.Reputation, ur.QuestionCount, ur.BadgeCount
// FROM RankedPosts rp JOIN UserReputation ur ON rp.PostId = (SELECT p.Id FROM Posts p WHERE p.OwnerUserId IS NOT NULL ORDER BY p.CreationDate DESC LIMIT 1)
// JOIN PopularTags pt ON pt.TagName = ANY(rp.Tags) WHERE rp.RankScore <= 50 ORDER BY rp.RankScore LIMIT 100;
//
// The UserReputation join names only rp, so it is a cross join of the one matching post with every user. Each COUNT(DISTINCT) there is its own
// fold over one row per child. The ARRAY_AGG order is left open by the SQL; the port keeps the joined-row order.
fn q28477(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user_id, creation_date, tags_str, .. } = &db.post;
    type A = (&'static [Option<Str>], &'static [Str], i64, i64);
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(tags_str.flat_map(inner_tags).opt()))
        .buf_fold(|v| -> A {
            let mut c: Vec<Id<Comment>> = v.iter().filter_map(|x| x.0 .0).collect();
            c.sort();
            c.dedup();
            let mut a: Vec<Id<Post>> = v.iter().filter_map(|x| x.0 .1).collect();
            a.sort();
            a.dedup();
            let mut d: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            d.sort();
            d.dedup();
            (Box::leak(v.iter().map(|x| x.1).collect::<Vec<_>>().into_boxed_slice()), Box::leak(d.into_boxed_slice()), c.len() as i64, a.len() as i64)
        });
    let pc = (&rp).flat_map(|a: A| a.0.iter().copied()).group_by(Same::<Option<Str>>::new()).fold(0i64, |n, _| n + 1);
    let pop: MatSet<Option<Str>> = rel(top_n(drain(&pc), |&(_, n)| Reverse(n), 10).into_iter().map(|x| x.0).collect()).map(|t| t).collect();
    let latest = top_n(drain(db.post.with(owner_user_id).select(creation_date)), |&(_, d)| Reverse(d), 1);
    let ls: MatSet<Id<Post>> = rel(latest.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    type K = (Id<Post>, i64);
    let rk: MatSet<K> = whole(db.post.with(post_type_id.eq(1)))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (s, w), desc)
        .filt(|(_, k)| k <= 50)
        .map(|(((p, _), _), k)| (p, k))
        .collect();
    let rs = (&rk).select(Same::<K>::new().with(Same::<K>::new().map(|x: K| x.0).with(&ls)).and(Same::<K>::new().map(|x: K| x.0).select(&rp)));
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)))).fold(0i64, |n, _| n + 1);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let ur = db.user.select(Ident::<User>::new().and((&qc).opt()).and((&bc).opt()));
    type R = (K, A);
    let v = drain(rs.cross(ur).select(Same::<(R, ((Id<User>, Option<i64>), Option<i64>))>::new().and(Same::<(R, ((Id<User>, Option<i64>), Option<i64>))>::new().map(|x: (R, ((Id<User>, Option<i64>), Option<i64>))| x.0 .1 .1).flat_map(|d: &'static [Str]| d.iter().copied().map(Some)).with(&pop))));
    let v = top_n(v, |&(_, ((((_, k), _), _), _))| k, 100);
    rows(v.into_iter().map(|(_, ((((p, _), (l, _, c, a)), ((u, q), b)), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::L(l.iter().map(|&x| ostr(x)).collect()), V::I(c), V::I(a), ostr(t)]);
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.extend([V::I(q.unwrap_or(0)), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RECURSIVE PostCTE AS (SELECT p.Id, p.Title, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, 0 AS Level
//     FROM Posts p WHERE p.PostTypeId = 1
//     UNION ALL SELECT p.Id, p.Title, COALESCE(p.AcceptedAnswerId, -1), p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, Level + 1 FROM Posts p INNER JOIN PostCTE cte ON p.ParentId = cte.Id),
// PostMetrics AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags,
//        MAX(v.CreationDate) AS LastVoteDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Tags t ON t.ExcerptPostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score),
// ClosingHistory AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastCloseDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// RankedPosts AS (SELECT pm.*, RANK() OVER (ORDER BY pm.Score DESC, pm.ViewCount DESC) AS Rank FROM PostMetrics pm)
// SELECT rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.Tags, CASE WHEN ch.LastCloseDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus, rp.Rank,
//        u.DisplayName AS MostActiveUser
// FROM RankedPosts rp LEFT JOIN Users u ON u.Id = (SELECT v.UserId FROM Votes v WHERE v.PostId = rp.Id ORDER BY v.CreationDate DESC LIMIT 1)
// LEFT JOIN ClosingHistory ch ON ch.PostId = rp.Id WHERE rp.Rank <= 100 ORDER BY rp.Rank;
//
// The recursive CTE is never read. Rank reads only base columns (one PostMetrics row per post), so the posts are ranked first. The correlated
// LIMIT 1 is an arg-max fold; a CreationDate tie goes to the larger vote id (the SQL leaves it open). The distinct tag names are joined in name order.
fn q34078(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    type K = (Id<Post>, i64);
    let rk: MatSet<K> = whole(&db.post.id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (s, w), desc)
        .filt(|(_, k)| k <= 100)
        .map(|(((p, _), _), k)| (p, k))
        .collect();
    let tp: MatSet<Id<Post>> = (&rk).map(|x: K| x.0).collect();
    let excerpts: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let pm = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&excerpts).select(&db.tag.tag_name).opt()).and(votes_of(db).opt()))
        .buf_fold(|v| {
            let mut c: Vec<Id<Comment>> = v.iter().filter_map(|x| x.0 .0).collect();
            c.sort();
            c.dedup();
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.0 .1).collect();
            t.sort();
            t.dedup();
            (c.len() as i64, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let Vote { creation_date: vd, user, .. } = &db.vote;
    let lv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().and(vd))).fold(None, |a: Option<(i64, Id<Vote>)>, (v, d)| a.max(Some((d, v))));
    let mu = (&lv).map(|a: Option<(i64, Id<Vote>)>| a.unwrap().1).select(user.select(&db.user.display_name));
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ch = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let v = drain((&rk).select(Same::<K>::new().and(Same::<K>::new().map(|x: K| x.0).select((&pm).and(mu.opt()).and((&ch).opt())))));
    rows(v.into_iter().map(|(_, ((p, k), (((c, t), u), h)))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), ostr(t), V::S(if h.is_some() { "Closed" } else { "Active" }), V::I(k), ostr(u)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName, COUNT(DISTINCT com.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes, ARRAY_AGG(DISTINCT t.TagName) AS Tags
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments com ON p.Id = com.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN unnest(string_to_array(p.Tags, ',')) AS tag_elements(tag) ON TRUE LEFT JOIN Tags t ON t.TagName = TRIM(tag_elements.tag)
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.ViewCount, p.Score, u.DisplayName),
// RankedPosts AS (SELECT ps.*, RANK() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC) AS Rank FROM PostStats ps)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.Tags, rp.Rank FROM RankedPosts rp WHERE rp.Rank <= 10 ORDER BY rp.Rank;
//
// Rank reads only base columns (one PostStats row per post), so the posts are ranked first.
fn q9693(db: &'static So) -> String {
    let Post { creation_date, score, view_count, tags_str, owner_user, .. } = &db.post;
    type K = (Id<Post>, i64);
    let rk: MatSet<K> = whole(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (s, w), desc)
        .filt(|(_, k)| k <= 10)
        .map(|(((p, _), _), k)| (p, k))
        .collect();
    let tp: MatSet<Id<Post>> = (&rk).map(|x: K| x.0).collect();
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tags = tags_str.flat_map(|t: Str| t.split(',')).map(|e: Str| e.trim_matches(' ')).select((&names).select(&db.tag.tag_name).opt()).opt();
    let g = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().and(&db.vote.vote_type_id)).opt()).and(tags))
        .buf_fold(|v| {
            let mut c: Vec<Id<Comment>> = v.iter().filter_map(|x| x.0 .0).collect();
            c.sort();
            c.dedup();
            let mut u: Vec<Id<Vote>> = v.iter().filter_map(|x| x.0 .1).filter(|x| x.1 == 2).map(|x| x.0).collect();
            u.sort();
            u.dedup();
            let mut d: Vec<Id<Vote>> = v.iter().filter_map(|x| x.0 .1).filter(|x| x.1 == 3).map(|x| x.0).collect();
            d.sort();
            d.dedup();
            let t: &'static [Option<Str>] = Box::leak(v.iter().map(|x| x.1.flatten()).collect::<Vec<_>>().into_boxed_slice());
            (c.len() as i64, u.len() as i64, d.len() as i64, t)
        });
    let v = drain((&rk).select(Same::<K>::new().and(Same::<K>::new().map(|x: K| x.0).select((&g).and(owner_user.select(&db.user.display_name).opt())))));
    rows(v.into_iter().map(|(_, ((p, k), ((c, u, d, t), n)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::S(n.unwrap_or("Community")), V::I(c), V::I(u), V::I(d), distinct_list(t.iter().copied()), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= '2023-10-01 12:34:56'::timestamp - INTERVAL '1 year'),
// PostTags AS (SELECT P.Id AS PostId, STRING_AGG(T.TagName, ', ') AS Tags FROM Posts P JOIN UNNEST(string_to_array(P.Tags, '<>')) AS T(TagName) ON T.TagName IS NOT NULL
//     WHERE P.PostTypeId = 1 GROUP BY P.Id),
// PostVotes AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes GROUP BY PostId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.AnswerCount, RP.OwnerDisplayName, PT.Tags, PV.UpVotes, PV.DownVotes, PV.TotalVotes
// FROM RankedPosts RP LEFT JOIN PostTags PT ON RP.PostId = PT.PostId LEFT JOIN PostVotes PV ON RP.PostId = PV.PostId WHERE RP.PostRank <= 5 ORDER BY RP.PostId, RP.PostRank;
//
// Splitting on '<>' leaves the tag string whole, one piece per post.
fn q6681(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2023, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (s, d), desc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let pt = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split("<>"))).buf_fold(|v| leak(v.join(", ")));
    let pv = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let v = drain((&tp).select(Ident::<Post>::new().and((&pt).opt()).and((&db.post.origid).select(&pv).opt())));
    rows(v.into_iter().map(|(_, ((p, t), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        f.push(ostr(t));
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

fn latest_badges(db: &'static So) -> HashIdx<Id<User>, Id<Badge>> {
    let Badge { user, date, .. } = &db.badge;
    let w = db.badge.group_by(user).select(Ident::<Badge>::new().and(date)).window(rank, |(_, d)| d, desc);
    (&w).filt(|(_, r): ((Id<Badge>, i64), i64)| r == 1).map(|((b, _), _)| b).collect()
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id, c.Id) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Rank, rp.CommentCount, COALESCE(b.Name, 'No Badge') AS BadgeName,
//        CASE WHEN rp.ViewCount IS NULL THEN 'No Views' WHEN rp.ViewCount > 100 THEN 'High View Count' ELSE 'Moderate View Count' END AS ViewStatus,
//        STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags
// FROM RankedPosts rp LEFT JOIN Users u ON rp.PostId = u.Id
// LEFT JOIN Badges b ON u.Id = b.UserId AND b.Date = (SELECT MAX(b2.Date) FROM Badges b2 WHERE b2.UserId = u.Id)
// LEFT JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS TagName FROM Posts p WHERE p.Id = rp.PostId) t ON TRUE
// WHERE rp.Rank <= 5 AND (rp.Score > 10 OR rp.ViewCount IS NOT NULL)
// GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Rank, rp.CommentCount, b.Name ORDER BY rp.CreationDate DESC;
//
// Ported from rewrites/21822.sql (Rank ordered totally by post and comment id, the STRING_AGG by name). Rank numbers the post x comment rows.
// `rp.PostId = u.Id` compares a post id with a user id, so it goes through the raw ids; the badges at the user's latest date are a rank-1 window.
fn q21822(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, origid, tags_str, .. } = &db.post;
    type R = (Id<Post>, i64);
    let rr: MatSet<R> = db
        .post
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(comments_of(db).opt()))
        .window(row_number, |((p, s), c)| (Reverse(s), p, c.is_none(), c), asc)
        .filt(|(_, k)| k <= 5)
        .map(|(((p, _), _), k)| (p, k))
        .collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let lb = latest_badges(db);
    let wh = Ident::<Post>::new().with(score.gt(10)).or(Ident::<Post>::new().with(view_count));
    type J = (R, Option<(Id<Badge>, Str)>);
    let j: MatSet<J> = (&rr)
        .with(Same::<R>::new().map(|x: R| x.0).with(wh))
        .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(origid.select(&uidx).select((&lb).select(Ident::<Badge>::new().and(&db.badge.name)))).opt()))
        .collect();
    let g = (&j)
        .group_by(Same::<J>::new().map(|x: J| (x.0, x.1.map(|b| b.1))))
        .select(Same::<J>::new().map(|x: J| x.0 .0).select(tags_str.flat_map(inner_tags).opt()))
        .buf_fold(|v| sorted_join(v.iter().filter_map(|x| *x).collect(), ", "));
    let cpp = comments_per_post(db);
    type G = (R, Option<Str>);
    let v = drain((&g).and(Same::<G>::new().map(|x: G| x.0 .0).select(&cpp)));
    rows(v.into_iter().map(|(((p, k), b), (t, c))| {
        let w = view_count.get(p);
        let st = match w {
            None => "No Views",
            Some(w) if w > 100 => "High View Count",
            _ => "Moderate View Count",
        };
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(k), V::I(c), V::S(b.unwrap_or("No Badge")), V::S(st), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.LastActivityDate, p.ViewCount, p.Score, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS RankByScore, COUNT(c.Id) AS CommentCount, ARRAY_AGG(DISTINCT t.TagName) AS TagList
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN UNNEST(string_to_array(SUBSTRING(p.Tags, 2, LENGTH(p.Tags) - 2), '><')) AS t(TagName) ON t.TagName IS NOT NULL
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR'
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.LastActivityDate, p.ViewCount, p.Score, p.Tags, pt.Name),
// TopPosts AS (SELECT PostId, Title, Body, CreationDate, LastActivityDate, ViewCount, Score, CommentCount, TagList FROM RankedPosts WHERE RankByScore <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.LastActivityDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.TagList, u.DisplayName AS AuthorName,
//        b.Name AS BadgeName, b.Class AS BadgeClass
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Date >= tp.CreationDate ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// RankByScore reads only base columns (one row per post), so the posts are ranked first; a Score tie goes to the smaller post id (the SQL leaves it
// open). `tp.PostId = u.Id` compares a post id with a user id, so it goes through the raw ids.
fn q25913(db: &'static So) -> String {
    let Post { creation_date, score, origid, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let g = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(tags_str.flat_map(inner_tags).opt()))
        .buf_fold(|v| -> (i64, &'static [Option<Str>]) { (v.iter().filter(|x| x.0.is_some()).count() as i64, Box::leak(v.iter().map(|x| x.1).collect::<Vec<_>>().into_boxed_slice())) });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type R = (Id<Post>, (Id<User>, i64));
    let ru: MatSet<R> = (&tp).select(Ident::<Post>::new().and(origid.select(&uidx).and(creation_date))).collect();
    let Badge { date, name, class, .. } = &db.badge;
    let late = Same::<R>::new()
        .and(Same::<R>::new().map(|x: R| x.1 .0).select(badges_of(db).select(date.and(name).and(class))))
        .filt(|(r, ((d, _), _)): (R, ((i64, Str), i64))| d >= r.1 .1)
        .map(|x: (R, ((i64, Str), i64))| x.1);
    let v = drain((&ru).select(Same::<R>::new().and(late.opt()).and(Same::<R>::new().map(|x: R| x.0).select(&g))));
    rows(v.into_iter().map(|(_, (((p, (u, _)), b), (c, t)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "activity", "views", "score"]);
        f.extend([V::I(c), distinct_list(t.iter().copied()), user_col(db, u, "name")]);
        f.extend(match b {
            Some(((_, n), k)) => [V::S(n), V::I(k)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ps.Name AS PostType, t.TagName,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY v.CreationDate DESC, v.Id DESC, t.TagName) AS RecentVoteRank
//     FROM Posts p LEFT JOIN PostTypes ps ON p.PostTypeId = ps.Id LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS TagName) t ON TRUE
//     WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year' AND p.ViewCount > 100),
// AggregatedPostMetrics AS (SELECT rp.PostId, rp.Title, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, MIN(rp.CreationDate) AS FirstSeen, MAX(rp.CreationDate) AS LastActive,
//        STRING_AGG(DISTINCT rp.TagName, ', ') AS Tags
//     FROM RankedPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId WHERE rp.RecentVoteRank = 1 GROUP BY rp.PostId, rp.Title)
// SELECT apm.PostId, apm.Title, apm.VoteCount, apm.UpvoteCount, apm.DownvoteCount, apm.FirstSeen, apm.LastActive, apm.Tags,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = apm.PostId) AS CommentCount
// FROM AggregatedPostMetrics apm ORDER BY apm.VoteCount DESC, apm.LastActive DESC LIMIT 100;
//
// Ported from rewrites/29577.sql (RecentVoteRank ordered totally by vote id and tag).
fn q29577(db: &'static So) -> String {
    let Post { creation_date, view_count, tags_str, .. } = &db.post;
    let base = || db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1))).with(view_count.gt(100));
    let vd = &db.vote.creation_date;
    let first = base()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(Ident::<Vote>::new().and(vd)).opt().and(tags_str.flat_map(inner_tags).opt()))
        .window(row_number, |(v, t): (Option<(Id<Vote>, i64)>, Option<Str>)| (v.is_none(), Reverse(v.map(|x| x.1)), Reverse(v.map(|x| x.0)), t.is_none(), t), asc);
    let apm = base().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let cpp = comments_per_post(db);
    let v = drain(base().select(Ident::<Post>::new().and(&apm).and((&first).filt(|(_, n)| n == 1).map(|((_, t), _)| t)).and(&cpp)));
    let v = top_n(v, |&(p, (((_, a), _), _))| (Reverse(a[0]), Reverse(creation_date.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(_, (((p, a), t), c))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["created", "created"]));
        f.extend([ostr(t), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankByType
//     FROM Posts p WHERE p.ViewCount IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.Reputation, u.DisplayName),
// PostTagData AS (SELECT p.Id AS PostId, STRING_AGG(t.TagName, ', ') AS Tags FROM Posts p
//     JOIN LATERAL (SELECT UNNEST(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) AS tag) AS tag ON TRUE JOIN Tags t ON t.TagName = tag GROUP BY p.Id)
// SELECT p.PostId, p.Title, CONCAT(u.DisplayName, ' (Reputation: ', u.Reputation + COALESCE(u.TotalBounties, 0), ')') AS UserProfile, p.CreationDate,
//        COALESCE(pt.Tags, 'No Tags') AS AssociatedTags, CASE WHEN p.RankByType = 1 THEN 'Latest' WHEN p.RankByType <= 3 THEN 'Popular' ELSE 'Other' END AS PostRank
// FROM RankedPosts p LEFT JOIN UserReputation u ON p.OwnerUserId = u.UserId LEFT JOIN PostTagData pt ON p.PostId = pt.PostId
// WHERE p.RankByType <= 10 AND (u.Reputation > 1000 OR u.Reputation IS NULL) ORDER BY p.CreationDate DESC, u.Reputation DESC NULLS LAST OFFSET 0 ROWS FETCH NEXT 50 ROWS ONLY;
//
// RankByType reads only base columns, so the posts are ranked first. The STRING_AGG order is left open by the SQL; the port joins in tag-list order.
fn q24008(db: &'static So) -> String {
    let Post { view_count, post_type_id, creation_date, owner_user, tags_str, .. } = &db.post;
    type R = (Id<Post>, i64);
    let r: MatSet<R> = db
        .post
        .with(view_count)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(rank, |(_, d)| d, desc)
        .filt(|(_, k)| k <= 10)
        .map(|((p, _), k)| (p, k))
        .collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pt = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(inner_tags).select(&names).select(&db.tag.tag_name)).buf_fold(|v| leak(v.join(", ")));
    let u = owner_user.select(Ident::<User>::new().and(&ur).and(&db.user.reputation)).opt();
    let v = drain(
        (&r)
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(u.and((&pt).opt()))))
            .filt(|(_, (u, _)): (R, (Option<((Id<User>, i64), i64)>, Option<Str>))| u.map_or(true, |(_, r)| r > 1000)),
    );
    let v = top_n(v, |&(_, ((p, _), (u, _)))| {
        let r = u.map(|(_, r)| r);
        (Reverse(creation_date.get(p).unwrap()), r.is_none(), Reverse(r))
    }, 50);
    rows(v.into_iter().map(|(_, ((p, k), (u, t)))| {
        let prof = match u {
            Some(((u, b), r)) => format!("{} (Reputation: {})", db.user.display_name.get(u).unwrap(), r + b),
            None => " (Reputation: )".to_string(),
        };
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::Owned(prof));
        f.extend(post_fields(db, p, &["created"]));
        f.extend([V::S(t.unwrap_or("No Tags")), V::S(if k == 1 { "Latest" } else if k <= 3 { "Popular" } else { "Other" })]);
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS TagName) t ON TRUE
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount),
// HighEngagementPosts AS (SELECT pm.PostId, pm.Title, pm.ViewCount, pm.AnswerCount, pm.UpVotes, pm.DownVotes, pm.Tags, (pm.UpVotes - pm.DownVotes) AS NetVotes,
//        RANK() OVER (ORDER BY pm.ViewCount DESC, (pm.UpVotes - pm.DownVotes) DESC) AS EngagementRank FROM PostMetrics pm WHERE pm.ViewCount > 50)
// SELECT he.PostId, he.Title, he.ViewCount, he.AnswerCount, he.UpVotes, he.DownVotes, he.NetVotes, he.Tags, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, p.LastEditDate
// FROM HighEngagementPosts he JOIN Posts p ON he.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id WHERE he.EngagementRank <= 10 ORDER BY he.EngagementRank;
//
// Ported from rewrites/27402.sql (the STRING_AGG ordered by name).
fn q27402(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, tags_str, owner_user, .. } = &db.post;
    let pm = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(post_type_id.eq(1))
        .with(view_count.gt(50))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(tags_str.flat_map(inner_tags).opt()))
        .buf_fold(|v| {
            let u = v.iter().filter(|x| x.0 == Some(2)).count() as i64;
            let d = v.iter().filter(|x| x.0 == Some(3)).count() as i64;
            (u, d, sorted_join(v.iter().filter_map(|x| x.1).collect(), ", "))
        });
    type R = (((Id<Post>, i64), (i64, i64, Option<Str>)), i64);
    let r: MatSet<R> = whole(&pm)
        .select(Ident::<Post>::new().and(view_count).and(&pm))
        .window(rank, |((_, w), (u, d, _))| (w, u - d), desc)
        .filt(|(_, k)| k <= 10)
        .collect();
    let v = drain((&r).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0 .0).select(owner_user))));
    rows(v.into_iter().map(|(_, ((((p, w), (u, d, t)), _), o))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::I(w));
        f.extend(post_fields(db, p, &["answers"]));
        f.extend([V::I(u), V::I(d), V::I(u - d), ostr(t)]);
        f.extend(ucols(db, o, &["name", "rep"]));
        f.extend(post_fields(db, p, &["edited"]));
        row(f)
    }))
}

// WITH RecursiveUserContribution AS (SELECT U.Id AS UserId, COUNT(P.Id) AS PostCount, SUM(V.BountyAmount) AS TotalBounties, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY SUM(P.Score) DESC) AS Rank,
//        COALESCE(U.LastAccessDate, U.CreationDate) AS LastActiveDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
//     WHERE U.Reputation > 0 GROUP BY U.Id, U.LastAccessDate, U.CreationDate),
// UserBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B GROUP BY B.UserId),
// PostTags AS (SELECT P.Id AS PostId, STRING_AGG(T.TagName, ', ') AS TagsList FROM Posts P
//     CROSS JOIN UNNEST(STRING_TO_ARRAY(SUBSTRING(P.Tags FROM 2 FOR LENGTH(P.Tags) - 2), '> <')) AS T(TagName) GROUP BY P.Id)
// SELECT U.DisplayName, U.Reputation, COALESCE(UC.PostCount, 0) AS TotalPosts, COALESCE(UC.Questions, 0) AS TotalQuestions, COALESCE(UC.Answers, 0) AS TotalAnswers,
//        COALESCE(UB.BadgeCount, 0) AS TotalBadges, UC.TotalBounties, UC.LastActiveDate, PT.TagsList
// FROM Users U LEFT JOIN RecursiveUserContribution UC ON U.Id = UC.UserId LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostTags PT ON P.Id = PT.PostId
// WHERE UC.Rank IS NULL OR UC.Rank <= 10 ORDER BY U.Reputation DESC, UC.TotalBounties DESC;
//
// Not recursive. Rank partitions by the user, one row each, so it is always 1 and the WHERE keeps every row. Splitting on '> <' leaves the
// inner tag string whole, one piece per post.
fn q30685(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let uc = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(bv.opt())).opt())
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, bs, bn, q, a), x| match x {
            Some((t, b)) => (n + 1, bs + b.flatten().unwrap_or(0), bn + b.flatten().is_some() as i64, q + (t == 1) as i64, a + (t == 2) as i64),
            None => (n, bs, bn, q, a),
        });
    let ub = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let pt = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t[1..t.len() - 1].split("> <"))).buf_fold(|v| leak(v.join(", ")));
    let v = drain(db.user.select(Ident::<User>::new().and((&uc).opt()).and((&ub).opt()).and(posts_of(db).select((&pt).opt()).opt())));
    rows(v.into_iter().map(|(_, (((u, c), b), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        match c {
            Some((n, bs, bn, q, a)) => f.extend([V::I(n), V::I(q), V::I(a), V::I(b.unwrap_or(0)), nullable(bs, bn), user_col(db, u, "last_access")]),
            None => f.extend([V::I(0), V::I(0), V::I(0), V::I(b.unwrap_or(0)), V::Null, V::Null]),
        }
        f.push(ostr(t.flatten()));
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCount AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, RANK() OVER (ORDER BY COUNT(B.Id) DESC) AS BadgeRank
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (2, 3)
//     WHERE P.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, P.Score),
// PostMetadata AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.CommentCount, RP.VoteCount, U.DisplayName, U.Reputation, UB.BadgeCount
//     FROM RecentPosts RP LEFT JOIN Users U ON RP.OwnerUserId = U.Id LEFT JOIN UserBadgeCount UB ON U.Id = UB.UserId),
// PostsWithTag AS (SELECT P.Id, P.Title, STRING_AGG(T.TagName, ', ') AS Tags FROM Posts P LEFT JOIN LATERAL UNNEST(STRING_TO_ARRAY(P.Tags, '<>')) AS T(TagName) ON TRUE GROUP BY P.Id, P.Title)
// SELECT PM.Title, PM.CreationDate, PM.Score, PM.CommentCount, PM.VoteCount, PM.DisplayName, PM.Reputation, PM.BadgeCount, PT.Tags
// FROM PostMetadata PM LEFT JOIN PostsWithTag PT ON PM.PostId = PT.Id WHERE PM.Reputation IS NOT NULL AND PM.VoteCount > 0 AND PM.BadgeCount > 0
// ORDER BY PM.Score DESC, PM.CreationDate DESC LIMIT 50;
//
// Not recursive: no CTE refers to itself. Splitting on '<>' leaves the tag string whole, one piece per post.
fn q33434(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, tags_str, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).opt()))
        .fold((0i64, 0i64), |(c, v), (ci, vi)| (c + ci.is_some() as i64, v + vi.is_some() as i64));
    let bpu = badges_per_user(db);
    let pwt = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split("<>")).opt()).buf_fold(|v| {
        let t: Vec<Str> = v.iter().filter_map(|x| *x).collect();
        if t.is_empty() { None } else { Some(leak(t.join(", "))) }
    });
    let v = drain(
        (&rp)
            .filt(|(_, n): (i64, i64)| n > 0)
            .and(owner_user.select(Ident::<User>::new().and(&bpu)).filt(|(_, b): (Id<User>, i64)| b > 0))
            .and(&pwt),
    );
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(p, (((c, n), (u, b)), t))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT r.PostId, r.Title, r.OwnerDisplayName, r.Score, r.CreationDate FROM RankedPosts r WHERE r.PostRank <= 5),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.CreationDate, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
//     FROM TopPosts tp LEFT JOIN (SELECT unnest(string_to_array(Posts.Tags, ', ')) AS TagName, Posts.Id AS PostId FROM Posts) t ON t.PostId = tp.PostId
//     GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.CreationDate),
// VotesSummary AS (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT pd.PostId, pd.Title, pd.OwnerDisplayName, pd.Score, pd.CreationDate, pd.Tags, COALESCE(vs.Upvotes, 0) AS Upvotes, COALESCE(vs.Downvotes, 0) AS Downvotes
// FROM PostDetails pd LEFT JOIN VotesSummary vs ON pd.PostId = vs.PostId ORDER BY pd.Score DESC, pd.CreationDate DESC;
//
// PostRank reads only base columns, so the posts are ranked first; a tie goes to the larger post id (the SQL leaves it open). Splitting on ', '
// leaves the tag string whole, one piece per post.
fn q5689(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (s, d, p), desc)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let pd = (&tp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split(", ")).opt()).buf_fold(|v| {
        let mut t: Vec<Str> = v.iter().filter_map(|x| *x).collect();
        t.sort();
        t.dedup();
        if t.is_empty() { None } else { Some(leak(t.join(", "))) }
    });
    let vs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let g = (&pd).and(&vs).map(|(t, a): (Option<Str>, [i64; 2])| (t, a[0], a[1]));
    rows(drain(&g).into_iter().map(|(p, (t, u, d))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "created"]);
        f.extend([ostr(t), V::I(u), V::I(d)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, p.Tags, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Body, p.Tags),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Body, rp.Tags, rp.CommentCount, rp.UpVotes, rp.DownVotes,
//        ROW_NUMBER() OVER (ORDER BY rp.UpVotes - rp.DownVotes DESC, rp.CommentCount DESC) AS Rank FROM RankedPosts rp WHERE rp.RowNum = 1)
// SELECT tp.Title, tp.CreationDate, tp.Body, tp.Tags, tp.CommentCount, tp.UpVotes, tp.DownVotes,
//        CASE WHEN tp.UpVotes - tp.DownVotes > 0 THEN 'Popular' WHEN tp.UpVotes - tp.DownVotes < 0 THEN 'Unpopular' ELSE 'Neutral' END AS Sentiment,
//        U.DisplayName AS AuthorName, COUNT(b.Id) AS BadgeCount, ARRAY_AGG(DISTINCT t.TagName) AS RelatedTags
// FROM TopPosts tp JOIN Users U ON tp.PostId = U.Id LEFT JOIN Badges b ON U.Id = b.UserId LEFT JOIN LATERAL (SELECT UNNEST(string_to_array(tp.Tags, ',')) AS TagName) AS tag ON true
// LEFT JOIN Tags t ON t.TagName = tag.TagName WHERE tp.Rank <= 10
// GROUP BY tp.Title, tp.CreationDate, tp.Body, tp.Tags, tp.CommentCount, tp.UpVotes, tp.DownVotes, U.DisplayName ORDER BY tp.UpVotes - tp.DownVotes DESC, tp.CommentCount DESC;
//
// RowNum partitions by the post, one row each, so it is always 1. `tp.PostId = U.Id` compares a post id with a user id, so it goes through the raw ids.
fn q28579(db: &'static So) -> String {
    let Post { post_type_id, origid, tags_str, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    type R = (Id<Post>, [i64; 3]);
    let tv: MatSet<R> = whole(&rp)
        .select(Ident::<Post>::new().and(&rp))
        .window(row_number, |(p, a)| (a[1] - a[2], a[0], Reverse(p)), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(x, _)| x)
        .collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tags = tags_str.flat_map(|t: Str| t.split(',')).select((&names).select(&db.tag.tag_name).opt()).opt();
    let g = (&tv)
        .group_by(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(origid.select(&uidx).select(&db.user.display_name))))
        .select(Same::<R>::new().map(|x: R| x.0).select(origid.select(&uidx).select(badges_of(db).opt()).and(tags)))
        .buf_fold(|v| -> (i64, &'static [Option<Str>]) {
            (v.iter().filter(|x| x.0.is_some()).count() as i64, Box::leak(v.iter().map(|x| x.1.flatten()).collect::<Vec<_>>().into_boxed_slice()))
        });
    rows(drain(&g).into_iter().map(|(((p, a), n), (b, t))| {
        let s = a[1] - a[2];
        let mut f = post_fields(db, p, &["title", "created", "body", "tags"]);
        f.extend(a.map(V::I));
        f.extend([V::S(if s > 0 { "Popular" } else if s < 0 { "Unpopular" } else { "Neutral" }), V::S(n), V::I(b), distinct_list(t.iter().copied())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.Id, V.Id) AS PostRank,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.Id) AS UpVotesCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.Id) AS DownVotesCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, PH.Comment, C.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id
//     WHERE PH.PostHistoryTypeId IN (10, 11)),
// PostTags AS (SELECT P.Id AS PostId, ARRAY_AGG(T.TagName ORDER BY T.TagName) AS Tags FROM Posts P LEFT JOIN LATERAL UNNEST(STRING_TO_ARRAY(P.Tags, '>')) AS T(TagName) ON TRUE GROUP BY P.Id),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ' ORDER BY B.Name) AS BadgeNames FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT RP.Title, RP.Score, RP.ViewCount, RP.CreationDate, RP.UpVotesCount, RP.DownVotesCount, COALESCE(CP.CloseReason, 'Not Closed') AS CloseReason, PT.Tags, UB.BadgeCount, UB.BadgeNames
// FROM RankedPosts RP LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId LEFT JOIN PostTags PT ON RP.PostId = PT.PostId LEFT JOIN UserBadges UB ON RP.PostId = UB.UserId
// WHERE RP.PostRank <= 5 AND (RP.UpVotesCount - RP.DownVotesCount) > 0 ORDER BY RP.Score DESC, RP.ViewCount DESC;
//
// Ported from rewrites/23574.sql (PostRank ordered totally by post and vote id, both aggregates ordered by name). PostRank numbers the post x vote
// rows. `RP.PostId = UB.UserId` compares a post id with a user id, so it goes through the raw ids.
fn q23574(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, tags_str, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rr = base()
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(votes_of(db).opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), p, w.is_none(), w), asc);
    let rr = (&rr).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p);
    let ud = base().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cr: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name));
    let pt = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split('>')).opt()).buf_fold(|v| -> &'static [Option<Str>] {
        let mut t: Vec<Option<Str>> = v.to_vec();
        t.sort_by(|a, b| a.is_none().cmp(&b.is_none()).then(a.cmp(b)));
        Box::leak(t.into_boxed_slice())
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name).opt()).buf_fold(|v| {
        let t: Vec<Str> = v.iter().filter_map(|x| *x).collect();
        (t.len() as i64, sorted_join(t, ", "))
    });
    let v = drain(
        (&rr)
            .select(Ident::<Post>::new().and(&ud))
            .filt(|(_, a): (Id<Post>, [i64; 2])| a[0] - a[1] > 0)
            .select(Same::<(Id<Post>, [i64; 2])>::new().and(Same::<(Id<Post>, [i64; 2])>::new().map(|x: (Id<Post>, [i64; 2])| x.0).select(cp.opt().and(&pt).and(origid.select(&uidx).select(&ub).opt())))),
    );
    rows(v.into_iter().map(|(_, ((p, a), ((c, t), b)))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("Not Closed")), V::L(t.iter().map(|&x| ostr(x)).collect())]);
        f.extend(match b {
            Some((n, s)) => [V::I(n), ostr(s)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserTagStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        STRING_AGG(DISTINCT t.TagName, ', ') AS TagsContributed
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT pt.Id, UNNEST(STRING_TO_ARRAY(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) AS TagName FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id
//                WHERE p.PostTypeId = 1) t ON p.Id = t.Id
//     GROUP BY u.Id, u.DisplayName),
// RecentlyClosedPosts AS (SELECT p.Id AS PostId, p.Title, ph.CreationDate AS ClosedDate, ph.UserDisplayName AS ClosedBy, ph.Comment AS CloseReason,
//        ROW_NUMBER() OVER (ORDER BY ph.CreationDate DESC) AS CloseRank FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10),
// TopUsers AS (SELECT UserId, COUNT(*) AS TotalVotes, SUM(CASE WHEN VoteTypeId IN (2, 4) THEN 1 ELSE 0 END) AS PositiveVotes FROM Votes GROUP BY UserId ORDER BY TotalVotes DESC LIMIT 10)
// SELECT u.DisplayName AS UserName, u.Reputation, ut.TotalPosts, ut.TotalQuestions, ut.TotalAnswers, ut.AcceptedAnswers, ut.TagsContributed, rcp.PostId,
//        rcp.Title AS ClosedPostTitle, rcp.ClosedDate, rcp.ClosedBy, rcp.CloseReason, tu.TotalVotes, tu.PositiveVotes
// FROM UserTagStats ut JOIN Users u ON u.Id = ut.UserId LEFT JOIN RecentlyClosedPosts rcp ON TRUE JOIN TopUsers tu ON u.Id = tu.UserId
// ORDER BY rcp.ClosedDate DESC, u.Reputation DESC;
//
// The tag subquery projects `pt.Id`, the PostTypes id (always 1), so `p.Id = t.Id` gives every question's tag pieces to the post whose id is 1,
// through the raw ids. `LEFT JOIN ... ON TRUE` is a cross join with the closings. TopUsers groups the NULL UserId too, which then joins no user.
// The distinct tags are joined in name order (the SQL leaves it open).
fn q27216(db: &'static So) -> String {
    let Post { post_type_id, post_type, origid, tags_str, accepted_answer_id, title, .. } = &db.post;
    let Vote { user_id, vote_type_id, .. } = &db.vote;
    let tu = db.vote.group_by(user_id.opt()).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + (t == 2 || t == 4) as i64]);
    let tu = top_n(drain(&tu), |&(_, a)| Reverse(a[0]), 10);
    type U = (Option<i64>, [i64; 2]);
    let tv = rel(tu);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type T = (Id<Post>, (i64, Str));
    let tr = rel(drain(db.post.with(post_type_id.eq(1)).select(post_type.select(&db.post_type.origid).and(tags_str.flat_map(inner_tags)))));
    let tidx: HashIdx<i64, Str> = (&tr).map(|x: T| x.1 .0).inv().select((&tr).map(|x: T| x.1 .1)).collect();
    let tus = (&tv).flat_map(|x: U| x.0).select(&uidx);
    let ut = tus
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(post_type_id).and(accepted_answer_id.opt()).and(origid.select(&tidx).opt())).opt())
        .buf_fold(|v| {
            let mut ps: Vec<Id<Post>> = v.iter().filter_map(|x| x.map(|x| x.0 .0 .0)).collect();
            ps.sort();
            ps.dedup();
            let q = v.iter().filter(|x| x.map_or(false, |x| x.0 .0 .1 == 1)).count() as i64;
            let a = v.iter().filter(|x| x.map_or(false, |x| x.0 .0 .1 == 2)).count() as i64;
            let acc = v.iter().filter(|x| x.map_or(false, |x| x.0 .0 .1 == 1 && x.0 .1.is_some())).count() as i64;
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.and_then(|x| x.1)).collect();
            t.sort();
            t.dedup();
            (ps.len() as i64, q, a, acc, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let PostHistory { post, post_history_type_id, creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    type C = (((Id<Post>, i64), Option<Str>), Option<Str>);
    let rcp: HashIdx<(), C> = whole(db.post_history.with(post_history_type_id.eq(10))).select(post.and(hd).and(user_display_name.opt()).and(comment.opt())).collect();
    let v = drain((&tv).select(
        Same::<U>::new()
            .and(Same::<U>::new().flat_map(|x: U| x.0).select(&uidx).select(Ident::<User>::new().and(&ut)))
            .and(Same::<U>::new().map(|_| ()).select((&rcp).opt())),
    ));
    rows(v.into_iter().map(|(_, (((_, a), (u, (n, q, an, acc, t))), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(q), V::I(an), V::I(acc), ostr(t)]);
        match c {
            Some((((p, d), by), cm)) => f.extend([V::I(origid.get(p).unwrap()), ostr(title.get(p)), V::T(d), ostr(by), ostr(cm)]),
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        f.extend([V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ARRAY_AGG(DISTINCT t.TagName) AS Tags, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS t(TagName) ON true WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// PopularPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.Tags, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.Rank <= 100),
// PostAnalyses AS (SELECT pp.PostId, pp.Title, pp.UpVoteCount, pp.DownVoteCount, pp.CommentCount,
//        COALESCE((SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = pp.PostId AND ph.PostHistoryTypeId IN (10, 11)), 0) AS CloseReopenCount,
//        COALESCE((SELECT COUNT(*) FROM PostLinks pl WHERE pl.PostId = pp.PostId AND pl.LinkTypeId = 3), 0) AS DuplicateCount FROM PopularPosts pp)
// SELECT pa.PostId, pa.Title, pa.UpVoteCount, pa.DownVoteCount, pa.CommentCount, pa.CloseReopenCount, pa.DuplicateCount,
//        (0.2 * pa.UpVoteCount + 0.1 * pa.CommentCount - 0.1 * pa.DownVoteCount + 0.3 * pa.CloseReopenCount - 0.5 * pa.DuplicateCount) AS EngagementScore
// FROM PostAnalyses pa ORDER BY EngagementScore DESC LIMIT 50;
//
// Rank reads only CreationDate, so the hundred posts are picked first. The score is DECIMAL arithmetic, so it is summed in exact tenths.
fn q29260(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = whole(db.post.with(post_type_id.eq(1)))
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, n)| n <= 100)
        .map(|((p, _), _)| p)
        .collect();
    let g = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(tags_str.flat_map(inner_tags).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let cr = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])))).fold(0i64, |n, _| n + 1);
    let du = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).select(Ident::<PostLink>::new().with((&db.post_link.link_type_id).eq(3)))).fold(0i64, |n, _| n + 1);
    let v = drain((&g).and((&cr).opt()).and((&du).opt()).map(|((a, c), d): (([i64; 3], Option<i64>), Option<i64>)| {
        let (c, d) = (c.unwrap_or(0), d.unwrap_or(0));
        (a, c, d, 2 * a[1] + a[0] - a[2] + 3 * c - 5 * d)
    }));
    let v = top_n(v, |x| Reverse(x.1 .3), 50);
    rows(v.into_iter().map(|(p, (a, c, d, e))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[0]), V::I(c), V::I(d), V::F(e as f64 / 10.0)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS BadgeCount,
//        SUM(CASE WHEN P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 1 ELSE 0 END) AS RecentPostCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvotesReceived
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, COUNT(C.Id) AS CommentCount, ARRAY_AGG(DISTINCT TAG.TagName) AS Tags
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN LATERAL UNNEST(string_to_array(P.Tags, '><')) AS TAG(TagName) ON TRUE GROUP BY P.Id, P.Title, P.Score, P.ViewCount),
// TopUsers AS (SELECT Us.UserId, Us.Reputation, Us.BadgeCount, Us.RecentPostCount, Us.UpvotesReceived, Us.DownvotesReceived, ROW_NUMBER() OVER (ORDER BY Us.Reputation DESC) AS Rank
//     FROM UserStats Us WHERE Us.Reputation > 0),
// RankedPosts AS (SELECT Pd.PostId, Pd.Title, Pd.Score, Pd.ViewCount, Pd.CommentCount, Pd.Tags, ROW_NUMBER() OVER (ORDER BY Pd.Score DESC, Pd.ViewCount DESC) AS PostRank FROM PostDetails Pd)
// SELECT Tu.UserId, Tu.Reputation, Tu.BadgeCount, Tu.RecentPostCount, Tu.UpvotesReceived, Tu.DownvotesReceived, Rp.PostId, Rp.Title AS PostTitle, Rp.Score AS PostScore,
//        Rp.ViewCount AS PostViewCount, Rp.CommentCount AS PostCommentCount, Rp.Tags AS PostTags
// FROM TopUsers Tu JOIN RankedPosts Rp ON Tu.UserId = Rp.PostId WHERE Tu.Rank <= 10 AND Rp.PostRank <= 20 ORDER BY Tu.Reputation DESC, Rp.Score DESC;
//
// Both ranks read only base columns (one row per user, one per post), so the ten users and twenty posts are picked first, joined through the raw
// ids (`Tu.UserId = Rp.PostId` compares a user id with a post id), and the aggregates are taken for the pairs that meet. The ARRAY_AGG order is left open.
fn q6507(db: &'static So) -> String {
    let Post { score, view_count, origid, creation_date, tags_str, .. } = &db.post;
    let User { reputation, .. } = &db.user;
    let tus: MatSet<Id<User>> = whole(db.user.with(reputation.gt(0)))
        .select(Ident::<User>::new().and(reputation))
        .window(row_number, |(u, r)| (Reverse(r), u), asc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let rps: MatSet<Id<Post>> = whole(&db.post.id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (s, w, Reverse(p)), desc)
        .filt(|(_, n)| n <= 20)
        .map(|(((p, _), _), _)| p)
        .collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type P = (Id<Post>, Id<User>);
    let pairs: MatSet<P> = (&rps).select(Ident::<Post>::new().and(origid.select(&uidx).with(&tus))).collect();
    let cut = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let us = (&pairs)
        .map(|x: P| x.1)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(creation_date.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 4], |a, (b, p)| {
            [a[0] + b.is_some() as i64, a[1] + p.map_or(0, |(d, _)| (d >= cut) as i64), a[2] + p.map_or(0, |(_, t)| (t == Some(2)) as i64), a[3] + p.map_or(0, |(_, t)| (t == Some(3)) as i64)]
        });
    let pd = (&pairs).map(|x: P| x.0).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(tags_str.flat_map(|t: Str| t.split("><")).opt())).buf_fold(|v| -> (i64, &'static [Option<Str>]) {
        (v.iter().filter(|x| x.0.is_some()).count() as i64, Box::leak(v.iter().map(|x| x.1).collect::<Vec<_>>().into_boxed_slice()))
    });
    let v = drain((&pairs).select(Same::<P>::new().and(Same::<P>::new().map(|x: P| x.1).select(&us)).and(Same::<P>::new().map(|x: P| x.0).select(&pd))));
    rows(v.into_iter().map(|(_, (((p, u), a), (c, t)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(c), distinct_list(t.iter().copied())]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, DENSE_RANK() OVER (ORDER BY COUNT(V.Id) DESC) AS VoteRank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostInfo AS (SELECT P.Id AS PostId, P.Title, P.Score, PH.PostHistoryTypeId, PH.CreationDate, PH.UserId AS EditorId, U.DisplayName AS EditorName,
//        ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY PH.CreationDate DESC) AS EditorHistoryRank
//     FROM Posts P INNER JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN Users U ON PH.UserId = U.Id WHERE PH.PostHistoryTypeId IN (4, 5, 6)),
// FilteredPostInfo AS (SELECT PI.PostId, PI.Title, PI.Score, PI.PostHistoryTypeId, PI.CreationDate, PI.EditorId, PI.EditorName, PI.EditorHistoryRank,
//        COALESCE(B.TotalVotes, 0) AS TotalUserVotes FROM PostInfo PI LEFT JOIN UserVoteSummary B ON PI.EditorId = B.UserId WHERE PI.Score > 10)
// SELECT FPI.PostId, FPI.Title, MAX(FPI.CreationDate) AS LastEditDate, FPI.EditorName, FPI.TotalUserVotes,
//        CASE WHEN FPI.TotalUserVotes BETWEEN 1 AND 5 THEN 'Novice Editor' WHEN FPI.TotalUserVotes BETWEEN 6 AND 15 THEN 'Intermediate Editor' ELSE 'Veteran Editor' END AS EditorExperienceLevel,
//        STRING_AGG(DISTINCT T.TagName, ', ') AS RelatedTags
// FROM FilteredPostInfo FPI LEFT JOIN Posts P ON FPI.PostId = P.Id LEFT JOIN LATERAL (SELECT unnest(string_to_array(P.Tags, ',')) AS TagName) AS T ON TRUE
// GROUP BY FPI.PostId, FPI.Title, FPI.EditorName, FPI.TotalUserVotes HAVING COUNT(FPI.EditorHistoryRank) > 2 ORDER BY LastEditDate DESC LIMIT 100;
//
// The group key is taken per history row (the editor varies within a post). Splitting on ',' leaves the tag string whole, one piece per post.
fn q20055(db: &'static So) -> String {
    let PostHistory { post, post_history_type_id, creation_date: hd, user, .. } = &db.post_history;
    let Post { score, tags_str, .. } = &db.post;
    let vpu = votes_per_user(db);
    let g = db
        .post_history
        .with(post_history_type_id.is_in([4, 5, 6]))
        .with(post.select(score.gt(10)))
        .group_by(post.and(user.select(&db.user.display_name).opt()).and(user.select(&vpu).opt().map(|n: Option<i64>| n.unwrap_or(0))))
        .select(hd.and(post.select(tags_str.flat_map(|t: Str| t.split(',')).opt())))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            t.sort();
            t.dedup();
            (v.len() as i64, v.iter().map(|x| x.0).max().unwrap(), if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let v = drain((&g).filt(|(n, _, _): (i64, i64, Option<Str>)| n > 2));
    let v = top_n(v, |&(_, (_, d, _))| Reverse(d), 100);
    rows(v.into_iter().map(|(((p, e), n), (_, d, t))| {
        let lvl = if (1..=5).contains(&n) { "Novice Editor" } else if (6..=15).contains(&n) { "Intermediate Editor" } else { "Veteran Editor" };
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::T(d), ostr(e), V::I(n), V::S(lvl), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.LastActivityDate, p.Score, p.ViewCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS DownVotes,
//        STRING_AGG(t.TagName, ', ') OVER (PARTITION BY p.Id) AS Tags
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN UNNEST(STRING_TO_ARRAY(p.Tags, ',')) AS t(TagName) ON TRUE
//     WHERE p.PostTypeId = 1 AND p.LastActivityDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'),
// TopUsers AS (SELECT OwnerUserId, SUM(Score) AS TotalScore FROM RankedPosts WHERE PostRank <= 5 GROUP BY OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadge, MAX(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadge,
//        MAX(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadge FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id HAVING COUNT(b.Id) > 0)
// SELECT u.DisplayName, u.Reputation, u.Location, COALESCE(b.BadgeCount, 0) AS BadgeCount, COALESCE(b.GoldBadge, 0) AS GoldBadge, COALESCE(b.SilverBadge, 0) AS SilverBadge,
//        COALESCE(b.BronzeBadge, 0) AS BronzeBadge, COALESCE(t.TotalScore, 0) AS TotalScore, COUNT(r.PostId) AS TotalPosts, SUM(r.UpVotes) AS TotalUpVotes, SUM(r.DownVotes) AS TotalDownVotes
// FROM Users u LEFT JOIN UserBadges b ON u.Id = b.UserId LEFT JOIN TopUsers t ON u.Id = t.OwnerUserId LEFT JOIN RankedPosts r ON r.OwnerUserId = u.Id
// GROUP BY u.Id, u.DisplayName, u.Reputation, u.Location, b.BadgeCount, b.GoldBadge, b.SilverBadge, b.BronzeBadge, t.TotalScore ORDER BY TotalScore DESC, u.Reputation DESC LIMIT 10;
//
// PostRank numbers each user's post x comment x vote x tag rows by Score; ties there only swap rows of equal Score, so the top-five sum is the
// same whichever wins. The per-post window sums ride on every joined row of the post, as in the SQL.
fn q31472(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, owner_user, score, tags_str, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1)).with(last_activity_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let joined = || comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(tags_str.flat_map(|t: Str| t.split(',')).opt());
    let win = rp().group_by(Ident::<Post>::new()).select(joined()).fold([0i64; 2], |a, ((_, t), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let rn = rp().group_by(owner_user).select(score.and(joined())).window(row_number, |(s, _)| s, desc);
    let ts_ = (&rn).filt(|(_, n)| n <= 5).fold(0i64, |a, ((s, _), _)| a + s);
    let Badge { class, .. } = &db.badge;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1].max((c == 1) as i64), a[2].max((c == 2) as i64), a[3].max((c == 3) as i64)]);
    let r = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)).with(last_activity_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select((&win).and(joined())).opt())
        .fold((0i64, 0i64, 0i64), |(n, u, d), x| match x {
            Some((w, _)) => (n + 1, u + w[0], d + w[1]),
            None => (n, u, d),
        });
    let v = drain(db.user.select(Ident::<User>::new().and((&ub).opt()).and((&ts_).opt()).and(&r)));
    let v = top_n(v, |&(u, (((_, _), t), _))| (Reverse(t.unwrap_or(0)), Reverse(db.user.reputation.get(u).unwrap())), 10);
    rows(v.into_iter().map(|(_, (((u, b), t), (n, up, dn)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(ostr(db.user.location.get(u)));
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        f.extend([V::I(t.unwrap_or(0)), V::I(n), nullable(up, n), nullable(dn, n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankByDate,
//        COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVotes,
//        STRING_AGG(DISTINCT t.TagName, ', ') AS TagsList
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN LATERAL (SELECT unnest(string_to_array(p.Tags, '<>,>')) AS TagName) t ON TRUE
//     WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.PostTypeId),
// ClosedPosts AS (SELECT p.Id AS PostId, ph.CreationDate AS ClosedDate, ph.UserDisplayName AS ClosedBy, ph.Comment AS CloseReason FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId = 10)
// SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.RankByDate, rp.CommentCount, rp.NetVotes, rp.TagsList, cp.ClosedDate, cp.ClosedBy, cp.CloseReason
// FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE (rp.RankByDate <= 5 AND cp.ClosedDate IS NOT NULL) OR (rp.NetVotes > 10 AND cp.ClosedDate IS NULL)
// ORDER BY CASE WHEN cp.ClosedDate IS NOT NULL THEN 0 ELSE 1 END, rp.Score DESC, rp.CreationDate DESC FETCH FIRST 50 ROWS ONLY;
//
// Splitting on '<>,>' leaves the tag string whole, one piece per post. A CreationDate tie inside a type goes to the larger post id (the SQL leaves it open).
fn q23800(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, tags_str, .. } = &db.post;
    let g = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(tags_str.flat_map(|t: Str| t.split("<>,>")).opt()))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            t.sort();
            t.dedup();
            let n = v.iter().filter(|x| x.0 .1 == Some(2)).count() as i64 - v.iter().filter(|x| x.0 .1 == Some(3)).count() as i64;
            (v.iter().filter(|x| x.0 .0.is_some()).count() as i64, n, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let PostHistory { post_history_type_id, creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(hd.and(user_display_name.opt()).and(comment.opt()));
    let w = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date).and(&g))
        .window(row_number, |((p, d), _)| (d, p), desc);
    type W = (((Id<Post>, i64), (i64, i64, Option<Str>)), i64);
    type J = (W, Option<((i64, Option<Str>), Option<Str>)>);
    let v = drain(
        (&w).select(Same::<W>::new().and(Same::<W>::new().map(|x: W| x.0 .0 .0).select(cp.opt())))
            .filt(|((((_, _), (_, n, _)), k), c): J| (k <= 5 && c.is_some()) || (n > 10 && c.is_none())),
    );
    let v = top_n(v, |&(_, ((((p, d), _), _), c))| (c.is_none(), Reverse(score.get(p).unwrap()), Reverse(d)), 50);
    rows(v.into_iter().map(|(_, ((((p, _), (c, n, t)), k), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend([V::I(k), V::I(c), V::I(n), ostr(t)]);
        match h {
            Some(((d, by), cm)) => f.extend([V::T(d), ostr(by), ostr(cm)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH PostScore AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//                             FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.UpVotes, ps.DownVotes FROM PostScore ps WHERE ps.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.UpVotes, tp.DownVotes, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = tp.PostId) AS CommentCount,
//        (SELECT AVG(COALESCE(Owner.Reputation, 0)) FROM Users Owner JOIN Posts p ON p.OwnerUserId = Owner.Id WHERE p.Id = tp.PostId) AS AverageOwnerReputation,
//        (SELECT STRING_AGG(DISTINCT t.TagName, ', ') FROM Tags t JOIN LATERAL (SELECT UNNEST(string_to_array(SUBSTRING(p.Tags FROM 2 FOR length(p.Tags) - 2), '><')) AS tag
//                FROM Posts p WHERE p.Id = tp.PostId) tags ON t.TagName = tags.tag) AS TagsList
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11) ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the posts are ranked first; a Score tie goes to the smaller post id (the SQL leaves it open). The distinct
// tag names are joined in name order.
fn q3695(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tl = (&tp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(inner_tags).select(&names).select(&db.tag.tag_name)).buf_fold(|v| {
        let mut t = v.to_vec();
        t.sort();
        t.dedup();
        leak(t.join(", "))
    });
    let ar = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(&db.user.reputation)).fold((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let cpp = comments_per_post(db);
    let closings = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let v = drain((&tp).select(Ident::<Post>::new().and(closings).and((&ud).opt()).and(&cpp).and((&ar).opt()).and((&tl).opt())));
    rows(v.into_iter().map(|(_, (((((p, _), a), c), r), t))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), r.map_or(V::Null, |(s, n)| V::F(s as f64 / n as f64)), ostr(t)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY SUM(COALESCE(p.ViewCount, 0)) DESC) AS ViewRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 WHERE u.Reputation > 50 GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags,
//        CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END AS AnswerStatus, COALESCE(SUM(c.Score), 0) AS CommentScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN LATERAL (SELECT unnest(string_to_array(p.Tags, '><')) AS TagName) t ON TRUE
//     GROUP BY p.Id, p.Title, p.CreationDate, p.AcceptedAnswerId),
// Benchmarking AS (SELECT ua.UserId, ua.DisplayName, pd.PostId, pd.Title, pd.CreationDate, pd.Tags, pd.AnswerStatus, pd.CommentScore, ua.TotalViews, ua.TotalBounties
//     FROM UserActivity ua JOIN PostDetails pd ON ua.UserId = pd.PostId WHERE ua.ViewRank <= 10)
// SELECT b.UserId, b.DisplayName, COUNT(b.PostId) AS TotalPosts, SUM(b.CommentScore) AS TotalCommentScore, AVG(b.TotalViews) AS AvgViewsPerPost, MAX(b.TotalBounties) AS HighestBounty
// FROM Benchmarking b GROUP BY b.UserId, b.DisplayName ORDER BY AVG(b.TotalViews) DESC, COUNT(b.PostId) DESC LIMIT 5;
//
// ViewRank partitions by the user, one row each, so it is always 1. `ua.UserId = pd.PostId` compares a user id with a post id, so it goes through the raw ids.
fn q20498(db: &'static So) -> String {
    let Post { view_count, tags_str, .. } = &db.post;
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let ua = db
        .user
        .with((&db.user.reputation).gt(50))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(bv.opt())).opt())
        .fold((0i64, 0i64), |(w, b), x| match x {
            Some((v, bb)) => (w + v.unwrap_or(0), b + bb.flatten().unwrap_or(0)),
            None => (w, b),
        });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let pd = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt().and(tags_str.flat_map(|t: Str| t.split("><")).opt())).fold((0i64, 0i64), |(s, n), (c, _)| (s + c.unwrap_or(0), n + c.is_some() as i64));
    let g = db.user.select((&ua).and((&db.user.origid).select(&pidx).select(&pd))).fold((0i64, 0i64, 0i64, 0i64), |(n, cs, tv, hb), ((w, b), (s, _))| (n + 1, cs + s, tv + w, hb.max(b)));
    let mut v = drain(&g);
    v.sort_by(|a, b| ((b.1 .2 as f64) / (b.1 .0 as f64)).partial_cmp(&((a.1 .2 as f64) / (a.1 .0 as f64))).unwrap().then(b.1 .0.cmp(&a.1 .0)));
    v.truncate(5);
    rows(v.into_iter().map(|(u, (n, cs, tv, hb))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(cs), V::F(tv as f64 / n as f64), V::I(hb)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// ClosePostHistory AS (SELECT ph.PostId, ph.CreationDate AS CloseDate, ph.UserId AS CloserUserId, EXTRACT(EPOCH FROM (CURRENT_TIMESTAMP - ph.CreationDate)) / 60 AS MinutesSinceClose
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(p.Score, 0) AS PostScore, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id AND v.VoteTypeId IN (2, 3)
//     LEFT JOIN unnest(string_to_array(p.Tags, ',')) AS t(TagName) ON t.TagName IS NOT NULL WHERE p.CreationDate > CURRENT_DATE - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.ViewCount, p.Score),
// RankedPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.PostScore, rp.CommentCount, rp.VoteCount, rp.Tags, ROW_NUMBER() OVER (ORDER BY rp.PostScore DESC, rp.ViewCount DESC) AS PostRank
//     FROM RecentPosts rp)
// SELECT ur.DisplayName, ur.Reputation, ur.ReputationRank, p.PostId, p.Title, p.ViewCount, p.PostScore, p.CommentCount, p.VoteCount, p.PostRank, cp.CloseDate, cp.MinutesSinceClose
// FROM UserReputation ur LEFT JOIN RankedPosts p ON p.PostId = ur.UserId LEFT JOIN ClosePostHistory cp ON p.PostId = cp.PostId
// WHERE ur.Reputation > 1000 ORDER BY ur.ReputationRank, p.PostRank;
//
// `p.PostId = ur.UserId` compares a post id with a user id, so it goes through the raw ids. A PostRank tie goes to the smaller post id (the SQL leaves it open).
fn q33160(db: &'static So) -> String {
    let Post { creation_date, score, view_count, origid, tags_str, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.gt(add_days(current_date(), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).opt()).and(tags_str.flat_map(|t: Str| t.split(',')).opt()))
        .fold([0i64; 2], |a, ((c, v), _)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    type R = ((Id<Post>, [i64; 2]), i64);
    let rk: MatSet<R> = whole(&rp)
        .select(Ident::<Post>::new().and(&rp).and(score.and(view_count.opt())))
        .window(row_number, |((p, _), (s, w))| (s, w, Reverse(p)), desc)
        .map(|((x, _), k)| (x, k))
        .collect();
    let byo: HashIdx<i64, R> = (&rk).map(|x: R| x.0 .0).select(origid).inv().collect();
    let now = utc_to_ny(now_utc());
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]))).select(hd);
    let ur = whole(&db.user.id).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    type U = ((Id<User>, i64), i64);
    let v = drain(
        (&ur)
            .filt(|((_, r), _): U| r > 1000)
            .select(Same::<U>::new().and(Same::<U>::new().map(|x: U| x.0 .0).select((&db.user.origid).select((&byo).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(cp.opt())))).opt()))),
    );
    rows(v.into_iter().map(|(_, (((u, _), k), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(k));
        match p {
            Some((((p, a), r), d)) => {
                f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
                f.extend([V::I(a[0]), V::I(a[1]), V::I(r), ots(d), d.map_or(V::Null, |d| V::F(secs(tz_sub(now, d)) / 60.0))]);
            }
            None => f.extend((0..9).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS Comments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, Questions, Answers, UpVotes, DownVotes, Comments, DENSE_RANK() OVER (ORDER BY PostCount DESC) AS EngagementRank FROM UserEngagement)
// SELECT U.DisplayName, U.PostCount, U.Questions, U.Answers, U.UpVotes, U.DownVotes, U.Comments, PT.Name AS PostType, COALESCE(T.TagName, 'No Tags') AS TagName,
//        CASE WHEN U.PostCount > 50 THEN 'High Engagement' WHEN U.PostCount BETWEEN 20 AND 50 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM TopUsers U LEFT JOIN Posts P ON U.UserId = P.OwnerUserId LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN LATERAL (SELECT string_agg(T.TagName, ', ' ORDER BY T.TagName) AS TagName FROM Tags T WHERE P.Tags LIKE '%' || T.TagName || '%') T ON TRUE
// WHERE U.EngagementRank <= 10 ORDER BY U.EngagementRank;
//
// Ported from rewrites/8317.sql (the string_agg ordered by name). EngagementRank reads only COUNT(DISTINCT P.Id), a fold of its own, so the users
// are ranked on it first and the joined-row sums are taken for them alone.
fn q8317(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu: MatSet<Id<User>> = whole(&pc)
        .select(Ident::<User>::new().and(&pc))
        .window(dense_rank, |(_, n)| n, desc)
        .filt(|(_, k)| k <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let ue = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, v), c)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + c.is_some() as i64],
            None => a,
        });
    let tm = tag_mentions(db);
    type M = (Id<Post>, Id<Tag>);
    let tmi: HashIdx<Id<Post>, Id<Tag>> = (&tm).map(|x: M| x.0).inv().select(Same::<M>::new().map(|x: M| x.1)).collect();
    let tn = db.post.group_by(Ident::<Post>::new()).select((&tmi).select(&db.tag.tag_name)).buf_fold(|v| sorted_join(v.to_vec(), ", ").unwrap());
    let v = drain((&tu).select(Ident::<User>::new().and(&pc).and(&ue).and(posts_of(db).select(ptype_name(db).and((&tn).opt())).opt())));
    rows(v.into_iter().map(|(_, (((u, n), a), p))| {
        let mut f = ucols(db, u, &["name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        match p {
            Some((t, g)) => f.extend([V::S(t), V::S(g.unwrap_or("No Tags"))]),
            None => f.extend([V::Null, V::S("No Tags")]),
        }
        f.push(V::S(if n > 50 { "High Engagement" } else if n >= 20 { "Moderate Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U),
// ActivePosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END AS HasAcceptedAnswer,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title, P.ViewCount, P.CreationDate, P.AcceptedAnswerId),
// PostTags AS (SELECT P.Id AS PostId, STRING_AGG(T.TagName, ', ') AS Tags FROM Posts P
//     LEFT JOIN LATERAL (SELECT UNNEST(STRING_TO_ARRAY(SUBSTRING(P.Tags, 2, LENGTH(P.Tags) - 2), '><')) AS TagName) AS TagArray ON TRUE
//     LEFT JOIN Tags T ON T.TagName = TagArray.TagName GROUP BY P.Id),
// PostSummary AS (SELECT AP.PostId, AP.Title, AP.ViewCount, AP.CreationDate, AP.HasAcceptedAnswer, PT.Tags, R.UserRank
//     FROM ActivePosts AP JOIN RankedUsers R ON AP.HasAcceptedAnswer = R.Id LEFT JOIN PostTags PT ON AP.PostId = PT.PostId)
// SELECT PS.Title, PS.ViewCount, PS.CreationDate, PS.HasAcceptedAnswer, COALESCE(PS.Tags, 'No Tags') AS Tags, U.DisplayName,
//        CASE WHEN PS.UserRank IS NOT NULL THEN 'Active User' ELSE 'Inactive User' END AS UserStatus
// FROM PostSummary PS LEFT JOIN Users U ON U.Id = PS.HasAcceptedAnswer
// WHERE PS.ViewCount > 100 AND (PS.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '3 months' OR U.Id IS NULL) ORDER BY PS.CreationDate DESC LIMIT 50;
//
// `AP.HasAcceptedAnswer = R.Id` compares a 0/1 flag with a user id, so it goes through the raw ids. The STRING_AGG order is left open by the SQL;
// the port joins in tag-list order.
fn q4233(db: &'static So) -> String {
    let Post { creation_date, view_count, accepted_answer_id, tags_str, .. } = &db.post;
    type U = ((Id<User>, i64), i64);
    let rr: MatSet<U> = whole(&db.user.id).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| r, desc).collect();
    let rank: HashIdx<Id<User>, i64> = (&rr).map(|x: U| x.0 .0).inv().map(|x: U| x.1).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pt = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(inner_tags).select((&names).select(&db.tag.tag_name).opt()).opt()).buf_fold(|v| {
        let t: Vec<Str> = v.iter().filter_map(|x| x.flatten()).collect();
        if t.is_empty() { None } else { Some(leak(t.join(", "))) }
    });
    let flag = || accepted_answer_id.opt().map(|a: Option<i64>| a.is_some() as i64);
    let cut = add_months(ts(2024, 10, 1, 12, 34, 56), -3);
    type J = ((((i64, i64), i64), Option<Str>), Option<Id<User>>);
    let v = drain(
        db.post
            .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
            .with(view_count.gt(100))
            .select(creation_date.and(flag()).and(flag().select(&uidx).select(&rank)).and((&pt).opt().map(|t: Option<Option<Str>>| t.flatten())).and(flag().select(&uidx).opt()))
            .filt(move |((((d, _), _), _), u): J| d < cut || u.is_none()),
    );
    let v = top_n(v, |&(_, ((((d, _), _), _), _))| Reverse(d), 50);
    rows(v.into_iter().map(|(p, ((((_, h), k), t), u))| {
        let mut f = post_fields(db, p, &["title", "views", "created"]);
        f.extend([V::I(h), V::S(t.unwrap_or("No Tags")), u.map_or(V::Null, |u| user_col(db, u, "name")), V::S(if k > 0 { "Active User" } else { "Inactive User" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, PostCount, TotalBounty, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalBounty DESC, UpVotes DESC) AS UserRank FROM UserActivity),
// ActivePosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.AnswerCount, MAX(PH.CreationDate) AS LastEditDate, STRING_AGG(DISTINCT T.TagName, ', ') AS Tags
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN LATERAL (SELECT unnest(string_to_array(P.Tags, '<>')) AS TagName) T ON TRUE
//     WHERE P.CreationDate > CURRENT_TIMESTAMP - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.ViewCount, P.AnswerCount)
// SELECT U.DisplayName, U.PostCount, U.TotalBounty, U.UpVotes, U.DownVotes, R.UserRank, A.PostId, A.Title, A.ViewCount, A.AnswerCount, A.LastEditDate, A.Tags
// FROM RankedUsers R JOIN UserActivity U ON R.UserId = U.UserId
// LEFT JOIN ActivePosts A ON A.LastEditDate = (SELECT MAX(LastEditDate) FROM ActivePosts AP WHERE AP.PostId IN (SELECT P.Id FROM Posts P WHERE P.OwnerUserId = U.UserId))
// WHERE R.UserRank <= 10 ORDER BY U.TotalBounty DESC, U.UpVotes DESC;
//
// The scalar subquery is the latest edit among the user's active posts, a per-user fold; the join then matches any active post edited at that
// instant. CURRENT_TIMESTAMP is a TIMESTAMPTZ, so CreationDate is compared as a New York instant.
fn q268(db: &'static So) -> String {
    let Post { creation_date, tags_str, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt())).opt())
        .buf_fold(|v| {
            let mut ps: Vec<Id<Post>> = v.iter().filter_map(|x| x.map(|x| x.0)).collect();
            ps.sort();
            ps.dedup();
            let vs = || v.iter().filter_map(|x| x.and_then(|x| x.1));
            (ps.len() as i64, vs().map(|x| x.1.unwrap_or(0)).sum::<i64>(), vs().filter(|x| x.0 == 2).count() as i64, vs().filter(|x| x.0 == 3).count() as i64)
        });
    type R = ((Id<User>, (i64, i64, i64, i64)), i64);
    let ru: MatSet<R> = whole(&ua).select(Ident::<User>::new().and(&ua)).window(rank, |(_, (_, b, u, _))| (b, u), desc).filt(|(_, k)| k <= 10).collect();
    let cut = add_years(utc_to_ny(now_utc()), -1);
    let active = || Ident::<Post>::new().with(creation_date.filt(move |d| ny_to_utc(d) > ny_to_utc(cut)));
    let ap = db
        .post
        .select(active())
        .group_by(Ident::<Post>::new())
        .select((history_of(db).select(&db.post_history.creation_date)).opt().and(tags_str.flat_map(|t: Str| t.split("<>")).opt()))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            t.sort();
            t.dedup();
            (v.iter().filter_map(|x| x.0).max(), if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let by_date: HashIdx<i64, Id<Post>> = (&ap).flat_map(|a: (Option<i64>, Option<Str>)| a.0).inv().collect();
    let um = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&ap).flat_map(|a: (Option<i64>, Option<Str>)| a.0)).fold(i64::MIN, |m, d| m.max(d));
    let v = drain((&ru).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select((&um).select(&by_date).select(Ident::<Post>::new().and(&ap)).opt()))));
    rows(v.into_iter().map(|(_, (((u, (n, b, up, dn)), k), a))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(n), V::I(b), V::I(up), V::I(dn), V::I(k)]);
        match a {
            Some((p, (d, t))) => {
                f.extend(post_fields(db, p, &["id", "title", "views", "answers"]));
                f.extend([ots(d), ostr(t)]);
            }
            None => f.extend((0..6).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        COALESCE(u.Reputation, 0) AS UserReputation
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostTags AS (SELECT p.Id AS PostId, t.TagName, COUNT(*) AS TagCount FROM Posts p
//     INNER JOIN LATERAL unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '> <')) AS t(TagName) ON true GROUP BY p.Id, t.TagName),
// TopPostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.UserReputation, cp.LastClosedDate, STRING_AGG(pt.TagName, ', ') AS Tags
//     FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId LEFT JOIN PostTags pt ON rp.PostId = pt.PostId WHERE rp.RankScore <= 5
//     GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.UserReputation, cp.LastClosedDate)
// SELECT tps.*, CASE WHEN tps.LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus,
//        CASE WHEN tps.UserReputation IS NULL THEN 'No Reputation' WHEN tps.UserReputation < 1000 THEN 'Low Reputation' ELSE 'High Reputation' END AS ReputationCategory
// FROM TopPostStatistics tps WHERE tps.Tags IS NOT NULL ORDER BY tps.Score DESC, tps.CreationDate ASC LIMIT 20;
//
// RankScore reads only base columns, so the posts are ranked first. Splitting on '> <' leaves the inner tag string whole, one piece per post.
fn q20788(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| s, desc)
        .filt(|(_, k)| k <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    type PT = (Id<Post>, Str);
    let pr: MatSet<PT> = db.post.select(Ident::<Post>::new().and(tags_str.flat_map(|t: Str| t[1..t.len() - 1].split("> <")))).collect();
    let pti: HashIdx<Id<Post>, Str> = (&pr).map(|x: PT| x.0).inv().map(|x: PT| x.1).collect();
    let g = (&tp).group_by(Ident::<Post>::new()).select((&pti).opt()).buf_fold(|v| {
        let t: Vec<Str> = v.iter().filter_map(|x| *x).collect();
        if t.is_empty() { None } else { Some(leak(t.join(", "))) }
    });
    let v = drain((&g).filt(|t: Option<Str>| t.is_some()).and(owner_user.select(&db.user.reputation).opt()).and((&cp).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()), 20);
    rows(v.into_iter().map(|(p, ((t, r), c))| {
        let r = r.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(r), ots(c), ostr(t), V::S(if c.is_some() { "Closed" } else { "Open" }), V::S(if r < 1000 { "Low Reputation" } else { "High Reputation" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id) AS RankByScore, COUNT(v.Id) AS VoteCount,
//        STRING_AGG(DISTINCT t.TagName, ', ') AS TagsAggregated, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosedDate
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN unnest(string_to_array(p.Tags, '>')) AS t(TagName) ON TRUE
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Score, p.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId),
// HighScoringUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, rp.PostId, rp.Score, rp.RankByScore, ub.TotalBadges, ub.HighestBadgeClass,
//        CASE WHEN ub.TotalBadges > 10 THEN 'Super User' WHEN ub.TotalBadges BETWEEN 5 AND 10 THEN 'Regular User' ELSE 'New User' END AS UserCategory
//     FROM Users u INNER JOIN RankedPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId
//     WHERE rp.RankByScore = 1 AND (rp.LastClosedDate IS NULL OR rp.LastClosedDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'))
// SELECT hsu.UserId, hsu.DisplayName, hsu.Reputation, hsu.UserCategory, hsu.PostId, hsu.Score, hsu.TotalBadges AS UserBadges, hsu.HighestBadgeClass,
//        CASE WHEN hsu.HighestBadgeClass IS NULL THEN 'No badges earned' WHEN hsu.HighestBadgeClass = 1 THEN 'Gold Badge Holder' WHEN hsu.HighestBadgeClass = 2 THEN 'Silver Badge Holder'
//        ELSE 'Bronze Badge Holder' END AS BadgeDescriptor, CASE WHEN hsu.Reputation IS NULL THEN 'Reputation Hidden' ELSE 'Active User with Reputation' END AS UserStatus
// FROM HighScoringUsers hsu WHERE hsu.Reputation >= 100 ORDER BY hsu.Reputation DESC, hsu.Score DESC LIMIT 10;
//
// Ported from rewrites/24261.sql (RankByScore ordered totally by post id). RankByScore reads only base columns, so the posts are ranked first;
// VoteCount and TagsAggregated are never read.
fn q24261(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, score, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .collect();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let lc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(hd)).fold(i64::MIN, |m, d| m.max(d));
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let v = drain(
        (&tp)
            .with(Ident::<Post>::new().minus(&lc).or((&lc).filt(move |d| d < cut)))
            .select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).ge(100)).and((&ub).opt())))),
    );
    let v = top_n(v, |&(_, (p, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(_, (p, (u, b)))| {
        let cat = match b {
            Some((n, _)) if n > 10 => "Super User",
            Some((n, _)) if n >= 5 => "Regular User",
            _ => "New User",
        };
        let desc = match b {
            None => "No badges earned",
            Some((_, 1)) => "Gold Badge Holder",
            Some((_, 2)) => "Silver Badge Holder",
            _ => "Bronze Badge Holder",
        };
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::S(cat));
        f.extend(post_fields(db, p, &["id", "score"]));
        f.extend([oint(b.map(|b| b.0)), oint(b.map(|b| b.1)), V::S(desc), V::S("Active User with Reputation")]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(V.BountyAmount), 0) AS TotalBountyAmount, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.Score, P.AnswerCount, P.ViewCount, P.CreationDate, P.ClosedDate, L.LinkTypeId, PH.PostHistoryTypeId, PH.Comment,
//        STRING_AGG(T.TagName, ', ') AS Tags
//     FROM Posts P LEFT JOIN PostLinks L ON P.Id = L.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN LATERAL (SELECT UNNEST(STRING_TO_ARRAY(P.Tags, '>')) AS TagName) T ON TRUE
//     WHERE P.ViewCount > 100 AND P.CreationDate < CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title, P.Score, P.AnswerCount, P.ViewCount, P.CreationDate, P.ClosedDate, L.LinkTypeId, PH.PostHistoryTypeId, PH.Comment),
// PostStatistics AS (SELECT PD.PostId, PD.Title, PD.Score, PD.AnswerCount, PD.ViewCount, PD.ClosedDate, U.DisplayName AS MostActiveUser,
//        COUNT(CASE WHEN C.UserId IS NOT NULL THEN 1 END) AS CommentCount, DENSE_RANK() OVER (PARTITION BY PD.PostId ORDER BY PD.ViewCount DESC) AS ViewRank
//     FROM PostDetails PD LEFT JOIN Comments C ON PD.PostId = C.PostId LEFT JOIN Users U ON C.UserId = U.Id
//     GROUP BY PD.PostId, PD.Title, PD.Score, PD.AnswerCount, PD.ViewCount, PD.ClosedDate, U.DisplayName)
// SELECT PS.PostId, PS.Title, PS.Score, PS.AnswerCount, PS.ViewCount, PS.ClosedDate, PS.MostActiveUser,
//        (SELECT COUNT(*) FROM PostHistory WHERE PostId = PS.PostId AND PostHistoryTypeId = 10) AS CloseVotes,
//        (SELECT COUNT(*) FROM PostHistory WHERE PostId = PS.PostId AND PostHistoryTypeId = 13) AS UndeleteVotes, UR.Reputation, UR.TotalBountyAmount
// FROM PostStatistics PS JOIN UserReputation UR ON PS.MostActiveUser = UR.DisplayName WHERE UR.Reputation > 100 ORDER BY PS.ViewCount DESC, PS.ClosedDate DESC LIMIT 10;
//
// PostDetails has one row per distinct (link type, history type, comment) of a post, collected as a set; its Tags are never read. PostStatistics
// groups those rows joined to the post's comments by the commenter's name; its CommentCount and ViewRank are never read.
fn q23548(db: &'static So) -> String {
    let Post { view_count, creation_date, closed_date, .. } = &db.post;
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    type D = (Id<Post>, Option<i64>, Option<(i64, Option<Str>)>);
    let pd: MatSet<D> = db
        .post
        .with(view_count.gt(100))
        .with(creation_date.lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .select(Ident::<Post>::new().and(links_of(db).select(&db.post_link.link_type_id).opt()).and(history_of(db).select(post_history_type_id.and(comment.opt())).opt()))
        .map(|((p, l), h)| (p, l, h))
        .collect();
    type X = (D, Option<Option<Str>>);
    type K = (Id<Post>, Option<Str>);
    let ps: MatSet<K> = (&pd)
        .select(Same::<D>::new().and(Same::<D>::new().map(|x: D| x.0).select(comments_of(db).select((&db.comment.user).select(&db.user.display_name).opt()).opt())))
        .map(|x: X| (x.0 .0, x.1.flatten()))
        .collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let names: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let ph = |t: i64| db.post.group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(t)))).fold(0i64, |n, _| n + 1);
    let (c10, c13) = (ph(10), ph(13));
    let v = drain((&ps).select(
        Same::<K>::new()
            .and(Same::<K>::new().flat_map(|k: K| k.1).select(&names).select(Ident::<User>::new().with((&db.user.reputation).gt(100)).and(&ur)))
            .and(Same::<K>::new().map(|k: K| k.0).select((&c10).opt().and((&c13).opt()))),
    ));
    let v = top_n(v, |&(_, (((p, _), _), _))| {
        let c = closed_date.get(p);
        (Reverse(view_count.get(p).unwrap()), c.is_none(), Reverse(c))
    }, 10);
    rows(v.into_iter().map(|(_, (((p, n), (u, b)), (a, d)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "answers", "views", "closed"]);
        f.extend([ostr(n), V::I(a.unwrap_or(0)), V::I(d.unwrap_or(0)), user_col(db, u, "rep"), V::I(b)]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, p.CommentCount, p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS RN
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// PostTags AS (SELECT p.Id AS PostId, UNNEST(string_to_array(substring(p.Tags, 2, LENGTH(p.Tags) - 2), '> <')) AS Tag FROM Posts p WHERE p.Tags IS NOT NULL),
// UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT u.DisplayName, u.Reputation, uvs.UpVotes, uvs.DownVotes, ubc.GoldBadges, ubc.SilverBadges, ubc.BronzeBadges, tp.Title AS TopPostTitle, tp.ViewCount AS TopPostViewCount,
//        tp.AnswerCount AS TopPostAnswerCount, tp.CommentCount AS TopPostCommentCount, tp.CreationDate AS TopPostCreationDate, STRING_AGG(pt.Tag, ', ') AS AssociatedTags
// FROM UserVoteSummary uvs JOIN Users u ON u.Id = uvs.UserId LEFT JOIN TopPosts tp ON tp.RN = 1 LEFT JOIN UserBadgeCounts ubc ON ubc.UserId = u.Id
// LEFT JOIN PostTags pt ON pt.PostId = tp.PostId WHERE uvs.PostCount > 5
// GROUP BY u.DisplayName, u.Reputation, uvs.UpVotes, uvs.DownVotes, ubc.GoldBadges, ubc.SilverBadges, ubc.BronzeBadges, tp.Title, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.CreationDate
// ORDER BY u.Reputation DESC;
//
// `LEFT JOIN TopPosts tp ON tp.RN = 1` names only tp, so every user meets the one top post (a cross join with the RN = 1 row). Splitting on '> <'
// leaves the inner tag string whole, one piece per post.
fn q25471(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, tags_str, .. } = &db.post;
    let uvs = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).opt())).opt())
        .buf_fold(|v| {
            let mut ps: Vec<Id<Post>> = v.iter().filter_map(|x| x.and_then(|x| x.1)).collect();
            ps.sort();
            ps.dedup();
            (v.iter().filter(|x| x.map_or(false, |x| x.0 == 2)).count() as i64, v.iter().filter(|x| x.map_or(false, |x| x.0 == 3)).count() as i64, ps.len() as i64)
        });
    let pt = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t[1..t.len() - 1].split("> <"))).buf_fold(|v| leak(v.join(", ")));
    let top: HashIdx<(), (Id<Post>, Option<Str>)> = whole(db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(_, s)| s, desc)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and((&pt).opt()))
        .collect();
    let ubc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + c.is_some() as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&uvs).filt(|a: (i64, i64, i64)| a.2 > 5).and(&ubc).and(Ident::<User>::new().map(|_| ()).select((&top).opt())));
    rows(v.into_iter().map(|(u, (((up, dn, _), b), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(up), V::I(dn), V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        match t {
            Some((p, g)) => {
                f.extend(post_fields(db, p, &["title", "views", "answers", "comments", "created"]));
                f.push(ostr(g));
            }
            None => f.extend((0..6).map(|_| V::Null)),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS RN,
//        STRING_AGG(t.TagName, ', ') AS TagList FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN UNNEST(STRING_TO_ARRAY(p.Tags, '>><<')) AS t(TagName) ON TRUE
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, pt.Name, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.TagList FROM RankedPosts rp WHERE rp.RN = 1),
// UserVotes AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS Upvotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS Downvotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// PostWithVotes AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.TagList, COALESCE(uv.Upvotes, 0) AS Upvotes, COALESCE(uv.Downvotes, 0) AS Downvotes
//     FROM TopPosts tp LEFT JOIN UserVotes uv ON tp.PostId = uv.PostId),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(bp.Upvotes, 0)) AS TotalUpvotes, SUM(COALESCE(bp.Downvotes, 0)) AS TotalDownvotes, COUNT(DISTINCT bp.PostId) AS PostsEngaged
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN UserVotes bp ON v.PostId = bp.PostId GROUP BY u.Id, u.DisplayName)
// SELECT ue.UserId, ue.DisplayName, ue.TotalUpvotes, ue.TotalDownvotes, ue.PostsEngaged, (ue.TotalUpvotes - ue.TotalDownvotes) AS NetEngagement,
//        (SELECT COUNT(DISTINCT p.Id) FROM Posts p WHERE p.OwnerUserId = ue.UserId) AS TotalUserPosts
// FROM UserEngagement ue WHERE ue.TotalUpvotes > 10 ORDER BY NetEngagement DESC LIMIT 10;
//
// RankedPosts, TopPosts and PostWithVotes are never read. UserVotes is keyed by the raw Votes.PostId.
fn q31602(db: &'static So) -> String {
    let Vote { post_id, .. } = &db.vote;
    let uv = db.vote.group_by(post_id).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(post_id.select(Same::<i64>::new().and(&uv)).opt()).opt())
        .buf_fold(|v| {
            let b: Vec<(i64, [i64; 2])> = v.iter().filter_map(|x| x.flatten()).collect();
            let mut ps: Vec<i64> = b.iter().map(|x| x.0).collect();
            ps.sort();
            ps.dedup();
            (b.iter().map(|x| x.1[0]).sum::<i64>(), b.iter().map(|x| x.1[1]).sum::<i64>(), ps.len() as i64)
        });
    let tup = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&ue).filt(|a: (i64, i64, i64)| a.0 > 10).and((&tup).opt()));
    let v = top_n(v, |&(_, ((u, d, _), _))| Reverse(u - d), 10);
    rows(v.into_iter().map(|(u, ((up, dn, n), t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(up), V::I(dn), V::I(n), V::I(up - dn), V::I(t.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank, COUNT(DISTINCT B.Id) AS BadgeCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.Reputation),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT COALESCE(P.AcceptedAnswerId, -1)) AS AcceptedAnswerCount, SUM(P.Score) AS TotalScore,
//        STRING_AGG(DISTINCT T.TagName, ', ') AS AssociatedTags FROM Posts P LEFT JOIN UNNEST(STRING_TO_ARRAY(P.Tags, ',')) AS T(TagName) ON TRUE GROUP BY P.OwnerUserId),
// UserEngagement AS (SELECT U.Id AS UserId, COALESCE(UR.ReputationRank, 0) AS ReputationRank, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.AcceptedAnswerCount, 0) AS AcceptedAnswerCount,
//        COALESCE(PS.TotalScore, 0) AS TotalScore, COALESCE(PS.AssociatedTags, 'No Tags') AS AssociatedTags
//     FROM Users U LEFT JOIN UserReputation UR ON U.Id = UR.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId)
// SELECT UEng.UserId, U.DisplayName, UEng.ReputationRank, UEng.PostCount, UEng.AcceptedAnswerCount, UEng.TotalScore,
//        CASE WHEN UEng.ReputationRank <= 10 THEN 'Top User' WHEN UEng.ReputationRank <= 50 THEN 'Average User'
//        WHEN EXTRACT(YEAR FROM CURRENT_DATE) - EXTRACT(YEAR FROM U.CreationDate) < 1 THEN 'New User' ELSE 'Experienced User' END AS UserCategory,
//        UEng.AssociatedTags, CASE WHEN UEng.TotalScore > 100 AND UEng.PostCount >= 10 THEN 'Highly Engaged' WHEN UEng.AcceptedAnswerCount >= 5 THEN 'Helpful User' ELSE 'Low Engagement' END AS EngagementLevel
// FROM Users U JOIN UserEngagement UEng ON U.Id = UEng.UserId WHERE U.LastAccessDate > CURRENT_TIMESTAMP - INTERVAL '30 days' ORDER BY UEng.ReputationRank ASC, UEng.TotalScore DESC LIMIT 50;
//
// ReputationRank reads only Reputation (one UserReputation row per user), and the WHERE reads only the user, so the users are filtered first and
// the rest is taken for them. CURRENT_TIMESTAMP is a TIMESTAMPTZ, so LastAccessDate is compared as a New York instant. The distinct tags are joined in name order.
fn q21754(db: &'static So) -> String {
    let User { last_access_date, reputation, creation_date, .. } = &db.user;
    let Post { accepted_answer_id, score, tags_str, .. } = &db.post;
    let cut = add_days(utc_to_ny(now_utc()), -30);
    let us: MatSet<Id<User>> = db.user.with(last_access_date.filt(move |d| ny_to_utc(d) > ny_to_utc(cut))).collect();
    let w = whole(&db.user.id).select(Ident::<User>::new().and(reputation).and((&us).opt())).window(rank, |((_, r), _)| r, desc);
    type U = (Id<User>, i64);
    let ru: MatSet<U> = (&w).filt(|((_, a), _)| a.is_some()).map(|(((u, _), _), k)| (u, k)).collect();
    let ps = (&us)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(accepted_answer_id.opt()).and(score).and(tags_str.flat_map(|t: Str| t.split(',')).opt())))
        .buf_fold(|v| {
            let mut p: Vec<Id<Post>> = v.iter().map(|x| x.0 .0 .0).collect();
            p.sort();
            p.dedup();
            let mut a: Vec<i64> = v.iter().map(|x| x.0 .0 .1.unwrap_or(-1)).collect();
            a.sort();
            a.dedup();
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            t.sort();
            t.dedup();
            (p.len() as i64, a.len() as i64, v.iter().map(|x| x.0 .1).sum::<i64>(), if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let yr = year(current_date());
    let v = drain((&ru).select(Same::<U>::new().and(Same::<U>::new().map(|x: U| x.0).select((&ps).opt()))));
    let v = top_n(v, |&(_, ((_, k), p))| (k, Reverse(p.map_or(0, |p| p.2))), 50);
    rows(v.into_iter().map(|(_, ((u, k), p))| {
        let (n, a, s, t) = p.unwrap_or((0, 0, 0, None));
        let cat = if k <= 10 { "Top User" } else if k <= 50 { "Average User" } else if yr - year(creation_date.get(u).unwrap()) < 1 { "New User" } else { "Experienced User" };
        let eng = if s > 100 && n >= 10 { "Highly Engaged" } else if a >= 5 { "Helpful User" } else { "Low Engagement" };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(k), V::I(n), V::I(a), V::I(s), V::S(cat), V::S(t.unwrap_or("No Tags")), V::S(eng)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS NetVotes,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostWithTags AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.NetVotes, rp.CommentCount, STRING_AGG(t.TagName, ', ') AS Tags
//     FROM RankedPosts rp LEFT JOIN LATERAL (SELECT UNNEST(string_to_array(p.Tags, ',')) AS TagName FROM Posts p WHERE p.Id = rp.PostId) t ON true
//     GROUP BY rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.NetVotes, rp.CommentCount),
// BadgeCounts AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldCount, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverCount,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeCount FROM Badges b GROUP BY b.UserId),
// PostsAndBadges AS (SELECT pwt.PostId, pwt.Title, pwt.Score, pwt.CreationDate, pwt.NetVotes, pwt.CommentCount, COALESCE(bc.GoldCount, 0) AS GoldCount,
//        COALESCE(bc.SilverCount, 0) AS SilverCount, COALESCE(bc.BronzeCount, 0) AS BronzeCount FROM PostWithTags pwt LEFT JOIN BadgeCounts bc ON pwt.PostId = bc.UserId)
// SELECT pab.PostId, pab.Title, pab.Score, pab.CreationDate, pab.NetVotes, pab.CommentCount, COALESCE(pab.GoldCount + pab.SilverCount + pab.BronzeCount, 0) AS TotalBadges,
//        CASE WHEN pab.NetVotes > 10 THEN 'Highly Popular' WHEN pab.NetVotes BETWEEN 1 AND 10 THEN 'Moderately Popular' ELSE 'Less Popular' END AS PopularityStatus
// FROM PostsAndBadges pab WHERE pab.CommentCount IS NOT NULL ORDER BY pab.Score DESC, pab.CreationDate DESC LIMIT 100;
//
// PostWithTags collapses the vote rows back to one row per post, and the LIMIT reads only base columns, so the hundred posts are picked first.
// `pwt.PostId = bc.UserId` compares a post id with a user id, so it goes through the raw ids. Rank and Tags are never read.
fn q20318(db: &'static So) -> String {
    let Post { creation_date, score, origid, .. } = &db.post;
    let top = top_n(drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score.and(creation_date))), |&(_, (s, d))| (Reverse(s), Reverse(d)), 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let nv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let bc = db.badge.group_by(&db.badge.user_id).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let cpp = comments_per_post(db);
    let v = drain((&tp).select(Ident::<Post>::new().and(&nv).and(&cpp).and(origid.select(&bc).opt())));
    rows(v.into_iter().map(|(_, (((p, n), c), b))| {
        let b = b.unwrap_or([0; 3]);
        let st = if n > 10 { "Highly Popular" } else if (1..=10).contains(&n) { "Moderately Popular" } else { "Less Popular" };
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend([V::I(n), V::I(c), V::I(b[0] + b[1] + b[2]), V::S(st)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(voteCounts.UpVotes, 0) AS UpVotes, COALESCE(voteCounts.DownVotes, 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//                             FROM Votes GROUP BY PostId) voteCounts ON p.Id = voteCounts.PostId),
// TopRatedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.UpVotes, tp.DownVotes, COALESCE(u.DisplayName, 'Anonymous') AS UserDisplayName,
//        SUM(CASE WHEN c.PostId IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
//     FROM TopRatedPosts tp LEFT JOIN Posts p ON tp.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN UNNEST(string_to_array(p.Tags, ',')) AS t(TagName) ON TRUE
//     GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.UpVotes, tp.DownVotes, u.DisplayName),
// FinalOutput AS (SELECT pd.Title, pd.UserDisplayName, pd.Score, pd.ViewCount, pd.UpVotes, pd.DownVotes, pd.CommentCount, pd.Tags,
//        CASE WHEN pd.UpVotes > pd.DownVotes THEN 'Positive Post' WHEN pd.UpVotes < pd.DownVotes THEN 'Negative Post' ELSE 'Neutral Post' END AS Sentiment FROM PostDetails pd)
// SELECT *, CASE WHEN Sentiment = 'Positive Post' AND Score >= 10 THEN 'Hot Content' WHEN Sentiment = 'Negative Post' AND Score < 0 THEN 'Needs Attention' ELSE 'Standard Post' END AS ContentStatus
// FROM FinalOutput WHERE ViewCount IS NOT NULL ORDER BY Score DESC, ViewCount DESC LIMIT 50;
//
// Rank reads only base columns, so the posts are ranked first; a tie goes to the smaller post id (the SQL leaves it open). Splitting on ','
// leaves the tag string whole, one piece per post.
fn q24509(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (s, w, Reverse(p)), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pd = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(tags_str.flat_map(|t: Str| t.split(',')).opt())).buf_fold(|v| {
        let mut t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
        t.sort();
        t.dedup();
        (v.iter().filter(|x| x.0.is_some()).count() as i64, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
    });
    let v = drain((&tp).with(view_count).select(Ident::<Post>::new().and((&ud).opt()).and(owner_user.select(&db.user.display_name).opt()).and(&pd)));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(_, (((p, a), n), (c, t)))| {
        let a = a.unwrap_or([0; 2]);
        let s = score.get(p).unwrap();
        let sen = if a[0] > a[1] { "Positive Post" } else if a[0] < a[1] { "Negative Post" } else { "Neutral Post" };
        let cs = if sen == "Positive Post" && s >= 10 { "Hot Content" } else if sen == "Negative Post" && s < 0 { "Needs Attention" } else { "Standard Post" };
        let mut f = post_fields(db, p, &["title"]);
        f.push(V::S(n.unwrap_or("Anonymous")));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), ostr(t), V::S(sen), V::S(cs)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId IN (2, 4) THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT ph.PostId) AS PostHistoryCount,
//        SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS PostClosedCount, STRING_AGG(DISTINCT t.TagName, ',') AS Tags
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN PostHistory ph ON ph.UserId = u.Id LEFT JOIN Posts p ON ph.PostId = p.Id
//     LEFT JOIN LATERAL UNNEST(string_to_array(p.Tags, ',')) AS t(TagName) ON TRUE GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, (SELECT COUNT(c.Id) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
//        ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// QualifiedUsers AS (SELECT ua.UserId, ua.DisplayName, ua.VoteCount, ua.Upvotes, ua.Downvotes, ua.PostHistoryCount, ua.PostClosedCount, ua.Tags, ps.PostId, ps.Title, ps.Score,
//        ps.ViewCount, ps.CommentCount FROM UserActivity ua INNER JOIN PostStatistics ps ON ua.UserId = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = ps.PostId)
//     WHERE ua.Upvotes > ua.Downvotes)
// SELECT q.UserId, q.DisplayName, q.Tags, p.PostId, p.Title, p.Score, p.ViewCount, p.CommentCount,
//        CASE WHEN p.Score > 100 THEN 'High Score Post' WHEN p.Score BETWEEN 50 AND 100 THEN 'Medium Score Post' ELSE 'Low Score Post' END AS ScoreCategory,
//        CASE WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = q.UserId AND b.Class = 1) THEN 'Gold Badge Holder'
//        WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = q.UserId AND b.Class = 2) THEN 'Silver Badge Holder'
//        WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = q.UserId AND b.Class = 3) THEN 'Bronze Badge Holder' ELSE 'No Badges' END AS BadgeStatus
// FROM QualifiedUsers q JOIN PostStatistics p ON q.PostId = p.PostId WHERE q.PostClosedCount = 0 ORDER BY p.ViewCount DESC, q.Upvotes DESC LIMIT 50;
//
// The scalar subquery is the post's owner, so UserActivity is taken for the owners of the recent posts alone. RowNum is never read.
fn q24051(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, score, view_count, .. } = &db.post;
    let ps: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).collect();
    let owners: MatSet<Id<User>> = (&ps).select(owner_user).collect();
    let hb: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and((&hb).select(post.and(post_history_type_id).and(post.select(tags_str.flat_map(|t: Str| t.split(',')).opt()))).opt()))
        .buf_fold(|v| {
            let up = v.iter().filter(|x| matches!(x.0, Some(2) | Some(4))).count() as i64;
            let dn = v.iter().filter(|x| x.0 == Some(3)).count() as i64;
            let cl = v.iter().filter(|x| x.1.map_or(false, |h| h.0 .1 == 10 || h.0 .1 == 11)).count() as i64;
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1.and_then(|h| h.1)).collect();
            t.sort();
            t.dedup();
            (up, dn, cl, if t.is_empty() { None } else { Some(leak(t.join(","))) })
        });
    let Badge { class, .. } = &db.badge;
    let has = |c: i64| badges_of(db).select(Ident::<Badge>::new().with(class.eq(c)));
    let cpp = comments_per_post(db);
    type A = (i64, i64, i64, Option<Str>);
    type B = ((Option<Id<User>>, Option<Id<User>>), Option<Id<User>>);
    type Q = (Id<Post>, ((Id<User>, A), B));
    let badges = Ident::<User>::new().with(has(1)).opt().and(Ident::<User>::new().with(has(2)).opt()).and(Ident::<User>::new().with(has(3)).opt());
    let v = drain(
        (&ps)
            .select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ua).and(badges))))
            .filt(|(_, ((_, a), _)): Q| a.0 > a.1 && a.2 == 0)
            .select(Same::<Q>::new().and(Same::<Q>::new().map(|x: Q| x.0).select(&cpp))),
    );
    let v = top_n(v, |&(_, ((p, ((_, a), _)), _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(a.0))
    }, 50);
    rows(v.into_iter().map(|(_, ((p, ((u, (_, _, _, t)), ((g, sv), b))), c))| {
        let s = score.get(p).unwrap();
        let cat = if s > 100 { "High Score Post" } else if (50..=100).contains(&s) { "Medium Score Post" } else { "Low Score Post" };
        let bs = if g.is_some() { "Gold Badge Holder" } else if sv.is_some() { "Silver Badge Holder" } else if b.is_some() { "Bronze Badge Holder" } else { "No Badges" };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(ostr(t));
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        f.extend([V::I(c), V::S(cat), V::S(bs)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, (UpVotes - DownVotes) AS Score, RANK() OVER (ORDER BY (UpVotes - DownVotes) DESC) AS Rank FROM UserActivity)
// SELECT T.DisplayName, T.Score, T.Rank,
//        (SELECT STRING_AGG(TT.TagName, ', ') FROM Tags TT WHERE TT.Id IN (SELECT DISTINCT CAST(UNNEST(string_to_array(P.Tags, '<>')) AS INTEGER))
//         AND P.OwnerUserId IS NOT NULL AND P.AnswerCount > 0) AS Tags
// FROM TopUsers T JOIN Posts P ON T.UserId = P.OwnerUserId
// WHERE T.Rank <= 10 AND EXISTS (SELECT 1 FROM PostHistory PH WHERE PH.PostId = P.Id AND PH.PostHistoryTypeId IN (10, 12) AND PH.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
// ORDER BY T.Score DESC OFFSET 5 ROWS FETCH NEXT 5 ROWS ONLY;
//
// PostCount and CommentCount are never read. The CAST to INTEGER keeps only the pieces that parse (none of '<tag>' does); the matching tags are
// joined in Tags id order. One row qualifies, so the OFFSET leaves none.
fn q1421(db: &'static So) -> String {
    let Post { owner_user_id, answer_count, tags_str, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold(0i64, |s, x| s + x.map_or(0, |(_, t)| (t == Some(2)) as i64 - (t == Some(3)) as i64));
    type R = ((Id<User>, i64), i64);
    let r: MatSet<R> = whole(&ua).select(Ident::<User>::new().and(&ua)).window(rank, |(_, s)| s, desc).filt(|(_, k)| k <= 10).collect();
    let tidx: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let tn = db
        .post
        .group_by(Ident::<Post>::new())
        .select(tags_str.flat_map(|t: Str| t.split("<>")).flat_map(|e: Str| e.trim().parse::<i64>().ok()).select(&tidx).select(Ident::<Tag>::new().and(&db.tag.tag_name)))
        .buf_fold(|mut v| {
            v.sort();
            v.dedup();
            leak(v.iter().map(|x| x.1).collect::<Vec<_>>().join(", "))
        });
    let recent = history_of(db).select(
        Ident::<PostHistory>::new()
            .with((&db.post_history.post_history_type_id).is_in([10, 12]))
            .with((&db.post_history.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))),
    );
    let tags = Ident::<Post>::new().with(owner_user_id).with(answer_count.gt(0)).select(&tn);
    let v = drain((&r).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(posts_of(db).select(Ident::<Post>::new().with(recent).and(tags.opt()))))));
    let mut v = top_n(v, |&(_, (((_, s), _), _))| Reverse(s), 0);
    let v: Vec<_> = v.drain(..).skip(5).take(5).collect();
    rows(v.into_iter().map(|(_, (((u, s), k), (_, t)))| row(vec![user_col(db, u, "name"), V::I(s), V::I(k), ostr(t)])))
}

// WITH RECURSIVE UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// PostScore AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Score, COALESCE(MAX(b.Class), 0) AS MaxBadgeClass FROM Posts p LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId, p.Score),
// PostWithUserStats AS (SELECT p.*, COALESCE(u.Reputation, 0) AS UserReputation, COALESCE(ub.BadgeCount, 0) AS UserBadgeCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId),
// RankedPosts AS (SELECT pwus.*, RANK() OVER (PARTITION BY pwus.PostTypeId ORDER BY pwus.Score DESC, pwus.ViewCount DESC) AS PostRank FROM PostWithUserStats pwus)
// SELECT rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UserReputation, rp.UserBadgeCount, CASE WHEN rp.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Not Accepted' END AS AnswerStatus,
//        STRING_AGG(DISTINCT tag.TagName, ', ') AS TagsList
// FROM RankedPosts rp LEFT JOIN LATERAL (SELECT unnest(string_to_array(rp.Tags, '>,<')) AS TagName) AS tag ON TRUE WHERE rp.PostRank <= 10
// GROUP BY rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UserReputation, rp.UserBadgeCount, rp.AcceptedAnswerId, rp.PostTypeId ORDER BY rp.PostTypeId, rp.Score DESC;
//
// Not recursive: no CTE refers to itself, and PostScore is never read. PostRank reads only base columns, so the posts are ranked first.
// Splitting on '>,<' leaves the tag string whole, one piece per post.
fn q33447(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (s, w), desc)
        .filt(|(_, k)| k <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let bpu = badges_per_user(db);
    let tl = (&tp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split(">,<")).opt()).buf_fold(|v| {
        let mut t: Vec<Str> = v.iter().filter_map(|x| *x).collect();
        t.sort();
        t.dedup();
        if t.is_empty() { None } else { Some(leak(t.join(", "))) }
    });
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select((&db.user.reputation).and(&bpu))).and(&tl)));
    rows(v.into_iter().map(|(_, ((p, (r, b)), t))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(r), V::I(b), V::S(if db.post.accepted_answer_id.get(p).is_some() { "Accepted" } else { "Not Accepted" }), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserStats AS (SELECT U.Id AS UserId, COUNT(DISTINCT B.Id) AS TotalBadges, SUM(CASE WHEN U.Reputation > 1000 THEN 1 ELSE 0 END) AS HighReputationCount
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// TaggedPosts AS (SELECT P.Id AS PostId, unnest(string_to_array(substring(P.Tags, 2, length(P.Tags)-2), '><')) AS Tag FROM Posts P WHERE P.Tags IS NOT NULL),
// PostVoteCounts AS (SELECT PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes V GROUP BY PostId)
// SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, US.TotalBadges,
//        (SELECT COALESCE(SUM(UpVotes), 0) - COALESCE(SUM(DownVotes), 0) FROM PostVoteCounts PVC WHERE PVC.PostId = RP.PostId) AS NetVotes, PG.PosterGroup
// FROM RankedPosts RP JOIN UserStats US ON US.UserId = RP.PostId
// LEFT JOIN (SELECT TP.PostId, STRING_AGG(TP.Tag, ', ') AS PosterGroup FROM TaggedPosts TP GROUP BY TP.PostId) PG ON PG.PostId = RP.PostId
// WHERE RP.Rank <= 10 ORDER BY NetVotes DESC, RP.CreationDate DESC;
//
// Rank reads only base columns, so the posts are ranked first; a tie goes to the smaller post id (the SQL leaves it open). `US.UserId = RP.PostId`
// compares a user id with a post id, so it goes through the raw ids. The STRING_AGG order is left open; the port joins in tag-list order.
fn q24945(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, tags_str, .. } = &db.post;
    let tp: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let bpu = badges_per_user(db);
    let pvc = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold(0i64, |n, t| n + (t == 2) as i64 - (t == 3) as i64);
    let pg = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(inner_tags)).buf_fold(|v| leak(v.join(", ")));
    let v = drain((&tp).select(Ident::<Post>::new().and(origid.select(&uidx).select(&bpu)).and(origid.select(&pvc).opt()).and((&pg).opt())));
    rows(v.into_iter().map(|(_, (((p, b), n), g))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(b), V::I(n.unwrap_or(0)), ostr(g)]);
        row(f)
    }))
}

fn ntile(i: usize, n: usize, k: usize) -> i64 {
    let (q, r) = (n / k, n % k);
    (if i < r * (q + 1) { i / (q + 1) } else { r + (i - r * (q + 1)) / q }) as i64 + 1
}

fn ntile5<O, R>(g: &[(O, R)], out: &mut Vec<i64>) {
    out.extend((0..g.len()).map(|i| ntile(i, g.len(), 5)));
}

fn count_all<O, R>(g: &[(O, R)], out: &mut Vec<i64>) {
    out.extend(g.iter().map(|_| g.len() as i64));
}

// WITH UserReputation AS (SELECT Id, Reputation, LastAccessDate, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// TopPosts AS (SELECT P.Id, P.Title, P.ViewCount, P.Score, COALESCE(PL.RelatedPostId, -1) AS RelatedPostId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        STRING_AGG(DISTINCT T.TagName, ', ') AS Tags
//     FROM Posts P LEFT JOIN PostLinks PL ON P.Id = PL.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN LATERAL (SELECT unnest(string_to_array(P.Tags, ',')) AS TagName) T ON TRUE
//     WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.ViewCount, P.Score, PL.RelatedPostId HAVING COUNT(DISTINCT T.TagName) > 2),
// TopUsers AS (SELECT Id, Reputation FROM UserReputation WHERE Reputation > 1000 AND LastAccessDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'),
// PostAnalytics AS (SELECT TP.Title, TP.ViewCount, TP.Score, CASE WHEN U.Id IS NOT NULL THEN 'Active User' ELSE 'Inactive User' END AS UserStatus, TP.Tags
//     FROM TopPosts TP LEFT JOIN TopUsers U ON TP.RelatedPostId = U.Id),
// FinalResults AS (SELECT *, NTILE(5) OVER (ORDER BY Score DESC) AS ScoreQuartile, COUNT(*) OVER () AS TotalPosts FROM PostAnalytics)
// SELECT *, CASE WHEN ScoreQuartile = 1 THEN 'Top Performer' WHEN ScoreQuartile = 2 THEN 'High Performer' WHEN ScoreQuartile = 3 THEN 'Medium Performer'
//        WHEN ScoreQuartile = 4 THEN 'Low Performer' ELSE 'Bottom Performer' END AS PerformanceCategory
// FROM FinalResults WHERE Score > 10 AND UserStatus = 'Active User' ORDER BY ViewCount DESC;
//
// The group key is taken per link row (RelatedPostId varies within a post). `TP.RelatedPostId = U.Id` compares a post id with a user id, so it
// goes through the raw ids. A Score tie inside NTILE goes to the smaller post id (the SQL leaves it open).
fn q3449(db: &'static So) -> String {
    let Post { creation_date, score, tags_str, .. } = &db.post;
    let tp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .select(Ident::<Post>::new().and(links_of(db).select(&db.post_link.related_post_id).opt()))
        .group_by(Same::<(Id<Post>, Option<i64>)>::new().map(|(p, l): (Id<Post>, Option<i64>)| (p, l.unwrap_or(-1))))
        .select(Same::<(Id<Post>, Option<i64>)>::new().map(|x: (Id<Post>, Option<i64>)| x.0).select(comments_of(db).opt().and(tags_str.flat_map(|t: Str| t.split(',')).opt())))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            t.sort();
            t.dedup();
            (t.len() as i64, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let User { reputation, last_access_date, origid, .. } = &db.user;
    let tu: HashIdx<i64, Id<User>> = db.user.with(reputation.gt(1000)).with(last_access_date.gt(cut)).select(origid).inv().collect();
    type K = (Id<Post>, i64);
    type A = (((i64, Option<Str>), Option<Id<User>>), i64);
    let pa: HashIdx<K, A> = (&tp)
        .filt(|(n, _): (i64, Option<Str>)| n > 2)
        .and(Same::<K>::new().map(|k: K| k.1).select(&tu).opt())
        .and(Same::<K>::new().map(|k: K| k.0).select(score))
        .collect();
    let fr = whole(&pa).select(Same::<K>::new().and(&pa)).window(ntile5, |(k, (_, s))| (Reverse(s), k), asc).window(count_all, |_| (), asc);
    type F = (((K, A), i64), i64);
    let mut out = Vec::new();
    (&fr).filt(|(((_, ((_, u), s)), _), _): F| s > 10 && u.is_some()).drive(|_, (((k, (((_, t), _), _)), q), n)| {
        let cat = ["Top Performer", "High Performer", "Medium Performer", "Low Performer", "Bottom Performer"][(q - 1) as usize];
        let mut f = post_fields(db, k.0, &["title", "views", "score"]);
        f.extend([V::S("Active User"), ostr(t), V::I(q), V::I(n), V::S(cat)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, P.ViewCount, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, ARRAY_AGG(DISTINCT T.TagName) AS Tags,
//        RANK() OVER (ORDER BY COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) - COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) DESC) AS VoteRank
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN unnest(string_to_array(P.Tags, '><')) AS T(TagName) ON TRUE
//     WHERE P.CreationDate >= DATE '2024-10-01' - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.Body, P.CreationDate, P.ViewCount),
// PostActivity AS (SELECT RH.PostId, RH.Title, RH.ViewCount, RH.UpVotes, RH.DownVotes, RH.CommentCount, RANK() OVER (ORDER BY RH.ViewCount DESC) AS ViewRank,
//        RANK() OVER (ORDER BY RH.UpVotes DESC) AS UpVoteRank, RANK() OVER (ORDER BY RH.CommentCount DESC) AS CommentRank FROM RankedPosts RH)
// SELECT PA.PostId, PA.Title, PA.ViewCount, PA.UpVotes, PA.DownVotes, PA.CommentCount, PA.ViewRank, PA.UpVoteRank, PA.CommentRank,
//        COALESCE(ARRAY_AGG(DISTINCT T.TagName ORDER BY T.TagName), ARRAY[]::text[]) AS Tags
// FROM PostActivity PA LEFT JOIN Posts P ON PA.PostId = P.Id LEFT JOIN unnest(string_to_array(P.Tags, '><')) AS T(TagName) ON TRUE
// GROUP BY PA.PostId, PA.Title, PA.ViewCount, PA.UpVotes, PA.DownVotes, PA.CommentCount, PA.ViewRank, PA.UpVoteRank, PA.CommentRank
// ORDER BY PA.ViewRank, PA.UpVoteRank, PA.CommentRank, PA.PostId LIMIT 10;
//
// Ported from rewrites/26707.sql (the final ARRAY_AGG ordered by name, the LIMIT ordered totally by post id). VoteRank and the inner Tags are never read.
fn q26707(db: &'static So) -> String {
    let Post { creation_date, view_count, tags_str, origid, .. } = &db.post;
    let pieces = || tags_str.flat_map(|t: Str| t.split("><")).opt();
    let rp = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(pieces()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let tl = db.post.group_by(Ident::<Post>::new()).select(pieces()).buf_fold(|v| -> &'static [Option<Str>] {
        let mut t = v.to_vec();
        t.sort_by(|a, b| a.is_none().cmp(&b.is_none()).then(a.cmp(b)));
        t.dedup();
        Box::leak(t.into_boxed_slice())
    });
    let w = whole(&rp)
        .select(Ident::<Post>::new().and(view_count.opt()).and(&rp).and(&tl))
        .window(rank, |(((_, w), _), _)| w, desc)
        .window(rank, |((((_, _), a), _), _)| a[0], desc)
        .window(rank, |(((((_, _), a), _), _), _)| a[2], desc);
    let v = top_n(drain(&w), |&(_, ((((((p, _), _), _), vr), ur), cr))| (vr, ur, cr, origid.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(_, ((((((p, _), a), t), vr), ur), cr))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(vr), V::I(ur), V::I(cr), V::L(t.iter().map(|&x| ostr(x)).collect())]);
        row(f)
    }))
}

// WITH LatestPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate AS PostCreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        COALESCE(AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE NULL END), 0) AS AverageUpVotes, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        STRING_AGG(DISTINCT t.TagName, ', ' ORDER BY t.TagName) AS Tags
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN UNNEST(STRING_TO_ARRAY(p.Tags, '><')) AS tag_name(tag) ON tag_name.tag IS NOT NULL LEFT JOIN Tags t ON t.TagName = tag_name.tag
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.ViewCount),
// PostHistoryStats AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS EditCount FROM PostHistory ph WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY ph.PostId, ph.PostHistoryTypeId),
// PostScores AS (SELECT lp.PostId, lp.Title, lp.OwnerDisplayName, lp.PostCreationDate, lp.ViewCount, lp.AverageUpVotes, lp.CommentCount, lp.Tags,
//        COALESCE(SUM(CASE WHEN phs.EditCount > 0 THEN phs.EditCount END), 0) AS TotalEdits
//     FROM LatestPosts lp LEFT JOIN PostHistoryStats phs ON lp.PostId = phs.PostId
//     GROUP BY lp.PostId, lp.Title, lp.OwnerDisplayName, lp.PostCreationDate, lp.ViewCount, lp.AverageUpVotes, lp.CommentCount, lp.Tags)
// SELECT ps.*, RANK() OVER (ORDER BY ps.AverageUpVotes DESC, ps.ViewCount DESC, ps.TotalEdits DESC) AS Rank FROM PostScores ps ORDER BY Rank, ps.PostId LIMIT 10;
//
// Ported from rewrites/8283.sql (the STRING_AGG ordered by name, the LIMIT ordered totally by post id). AVG of 1-or-NULL is 1 wherever an
// upvote row exists.
fn q8283(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, tags_str, origid, .. } = &db.post;
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let lp = db
        .post
        .with(creation_date.ge(cut))
        .group_by(Ident::<Post>::new())
        .select(up.opt().and(comments_of(db).opt()).and(tags_str.flat_map(|t: Str| t.split("><")).select((&names).select(&db.tag.tag_name).opt()).opt()))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1.flatten()).collect();
            t.sort();
            t.dedup();
            (v.iter().any(|x| x.0 .0.is_some()) as i64, v.iter().filter(|x| x.0 .1.is_some()).count() as i64, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let te = db.post_history.with(hd.ge(cut)).group_by(post).fold(0i64, |n, _| n + 1);
    let w = whole(&lp)
        .select(Ident::<Post>::new().and(&lp).and(view_count.opt()).and((&te).opt()).and(owner_user.select(&db.user.display_name).opt()))
        .window(rank, |((((_, (a, _, _)), w), e), _)| (a, w, e.unwrap_or(0)), desc);
    let v = top_n(drain(&w), |&(_, (((((p, _), _), _), _), k))| (k, origid.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(_, (((((p, (a, c, t)), _), e), n), k))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(ostr(n));
        f.extend(post_fields(db, p, &["created", "views"]));
        f.extend([V::F(a as f64), V::I(c), ostr(t), V::I(e.unwrap_or(0)), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(v.Id) OVER (PARTITION BY p.Id) AS VoteCount, (SELECT COUNT(c.Id) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.PostTypeId, rp.CreationDate, rp.Score, rp.OwnerUserId, rp.VoteCount, rp.CommentCount FROM RankedPosts rp
//     WHERE rp.rn = 1 AND rp.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = rp.PostTypeId)),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT fp.PostId, fp.Title, fp.Score, CASE WHEN ub.BadgeCount IS NULL THEN 'No Badges' WHEN ub.BadgeCount >= 5 THEN 'Expert' WHEN ub.BadgeCount BETWEEN 1 AND 4 THEN 'Novice' ELSE 'Unknown' END AS UserExperience,
//        ub.HighestBadgeClass, COALESCE(u.Reputation, 0) AS UserReputation, CASE WHEN fp.CommentCount > 0 THEN 'Engaged' ELSE 'Silent' END AS UserEngagement,
//        STRING_AGG(DISTINCT t.TagName, ', ') AS RelatedTags
// FROM FilteredPosts fp LEFT JOIN Users u ON fp.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN (SELECT Id, UNNEST(STRING_TO_ARRAY(Tags, '>')) AS TagName FROM Posts) t ON fp.PostId = t.Id
// GROUP BY fp.PostId, fp.Title, fp.Score, ub.BadgeCount, ub.HighestBadgeClass, u.Reputation, fp.CommentCount ORDER BY fp.Score DESC, UserEngagement DESC LIMIT 50;
//
// rn = 1 keeps one joined row of each type's newest post (its vote rows differ in nothing that is read), so the newest posts are picked first; a
// CreationDate tie goes to the larger post id (the SQL leaves it open). The correlated AVG is a per-type fold.
fn q24299(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, tags_str, .. } = &db.post;
    let first: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (d, p), desc)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .collect();
    let av = db.post.group_by(post_type_id).select(score).fold((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let above = score.and(post_type_id.select(&av)).filt(|(s, (t, n)): (i64, (i64, i64))| s as f64 > t as f64 / n as f64);
    let fp: MatSet<Id<Post>> = (&first).with(above).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, i64::MIN), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let tg = (&fp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split('>')).opt()).buf_fold(|v| {
        let mut t: Vec<Str> = v.iter().filter_map(|x| *x).collect();
        t.sort();
        t.dedup();
        if t.is_empty() { None } else { Some(leak(t.join(", "))) }
    });
    let cpp = comments_per_post(db);
    let v = drain((&fp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ub)).opt()).and(&cpp).and(&tg)));
    let v = top_n(v, |&(p, (((_, _), c), _))| (Reverse(score.get(p).unwrap()), c > 0), 50);
    rows(v.into_iter().map(|(_, (((p, u), c), t))| {
        let (ex, hc, rep) = match u {
            Some((u, (n, m))) => (if n >= 5 { "Expert" } else if n >= 1 { "Novice" } else { "Unknown" }, omax(m, n), db.user.reputation.get(u).unwrap()),
            None => ("No Badges", V::Null, 0),
        };
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::S(ex), hc, V::I(rep), V::S(if c > 0 { "Engaged" } else { "Silent" }), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseOpenCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeletionCount,
//        MAX(ph.CreationDate) AS LastActivityDate FROM PostHistory ph WHERE ph.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '6 months' GROUP BY ph.PostId),
// EnhancedPostInfo AS (SELECT rp.PostId, rp.OwnerUserId, rp.Title, rp.CreationDate, rp.Score, COALESCE(ps.CloseOpenCount, 0) AS CloseOpenCount, COALESCE(ps.DeletionCount, 0) AS DeletionCount,
//        ps.LastActivityDate, ur.Reputation AS OwnerReputation, ur.TotalBounties
//     FROM RankedPosts rp LEFT JOIN PostHistorySummary ps ON rp.PostId = ps.PostId LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId)
// SELECT epi.PostId, epi.Title, epi.CreationDate, epi.Score, epi.CloseOpenCount, epi.DeletionCount, epi.OwnerReputation,
//        CASE WHEN epi.OwnerReputation >= 1000 THEN 'Elite' WHEN epi.OwnerReputation BETWEEN 500 AND 999 THEN 'Experienced' ELSE 'Novice' END AS UserCategory,
//        STRING_AGG(DISTINCT t.TagName, ', ' ORDER BY t.TagName) AS Tags
// FROM EnhancedPostInfo epi LEFT JOIN Posts p ON epi.PostId = p.Id LEFT JOIN LATERAL (SELECT UNNEST(STRING_TO_ARRAY(p.Tags, ', ')) AS TagName) t ON TRUE
// GROUP BY epi.PostId, epi.Title, epi.CreationDate, epi.Score, epi.CloseOpenCount, epi.DeletionCount, epi.OwnerReputation ORDER BY epi.Score DESC, epi.CloseOpenCount DESC;
//
// ScoreRank and the bounty totals are never read. Splitting on ', ' leaves the tag string whole, one piece per post.
fn q23584(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ps = db.post_history.with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).group_by(post).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10 || t == 11) as i64, a[1] + (t == 12) as i64]);
    let tg = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split(", "))).buf_fold(|v| {
        let mut t = v.to_vec();
        t.sort();
        t.dedup();
        leak(t.join(", "))
    });
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(Ident::<Post>::new().and((&ps).opt()).and(owner_user.select(&db.user.reputation).opt()).and((&tg).opt())));
    rows(v.into_iter().map(|(_, (((p, a), r), t))| {
        let a = a.unwrap_or([0; 2]);
        let cat = match r {
            Some(r) if r >= 1000 => "Elite",
            Some(r) if (500..=999).contains(&r) => "Experienced",
            _ => "Novice",
        };
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), oint(r), V::S(cat), ostr(t)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("34241", q34241),
    ("22668", q22668),
    ("6783", q6783),
    ("249", q249),
    ("28267", q28267),
    ("5919", q5919),
    ("29390", q29390),
    ("28140", q28140),
    ("29901", q29901),
    ("9270", q9270),
    ("3347", q3347),
    ("26925", q26925),
    ("28042", q28042),
    ("27466", q27466),
    ("33973", q33973),
    ("27947", q27947),
    ("26912", q26912),
    ("24653", q24653),
    ("28477", q28477),
    ("34078", q34078),
    ("9693", q9693),
    ("6681", q6681),
    ("21822", q21822),
    ("25913", q25913),
    ("29577", q29577),
    ("24008", q24008),
    ("27402", q27402),
    ("30685", q30685),
    ("33434", q33434),
    ("5689", q5689),
    ("28579", q28579),
    ("23574", q23574),
    ("27216", q27216),
    ("29260", q29260),
    ("6507", q6507),
    ("20055", q20055),
    ("31472", q31472),
    ("23800", q23800),
    ("3695", q3695),
    ("20498", q20498),
    ("33160", q33160),
    ("8317", q8317),
    ("4233", q4233),
    ("268", q268),
    ("20788", q20788),
    ("24261", q24261),
    ("23548", q23548),
    ("25471", q25471),
    ("31602", q31602),
    ("21754", q21754),
    ("20318", q20318),
    ("24509", q24509),
    ("24051", q24051),
    ("1421", q1421),
    ("33447", q33447),
    ("24945", q24945),
    ("3449", q3449),
    ("26707", q26707),
    ("8283", q8283),
    ("24299", q24299),
    ("23584", q23584),
];
