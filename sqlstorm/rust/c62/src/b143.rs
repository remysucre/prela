use harness::prelude::*;
use std::cmp::Reverse;

// ORDER BY (a key, b key) LIMIT n over a CROSS JOIN. An a row of RANK r by its key has (r - 1) * |b| rows ahead of it, so only
// a rows with (r - 1) * |b| < n can reach the first n: those are picked by a window first and only they are crossed with b.
fn cross_top<A: Copy, B: Copy, KA: Ord + Copy, KB: Ord>(a: Vec<A>, ka: impl Fn(&A) -> KA, b: Vec<B>, kb: impl Fn(&B) -> KB, n: usize) -> Vec<(A, B)> {
    let ra = rel(a);
    let rb = rel(b);
    let ranked = (&ra).map(|_| ()).inv().select(&ra).window(rank, |x: A| ka(&x), asc);
    let nb = count(&rb).max(1);
    let picked = (&ranked).filt(move |(_, r)| (r - 1) * nb < n as i64).map(|(x, _)| x);
    top_n(drain(picked.cross(&rb)), |(_, (x, y))| (ka(x), kb(y)), n).into_iter().map(|x| x.1).collect()
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

fn stats_by_type(db: &'static So) -> Fold<Str, [i64; 12]> {
    stats_by(db, ptype_name(db))
}

fn stats_by_type_id(db: &'static So) -> Fold<Id<PostType>, [i64; 12]> {
    stats_by(db, &db.post.post_type)
}

fn stats_by<K: IntoQuery>(db: &'static So, key: K) -> Fold<ROf<K>, [i64; 12]>
where
    K::Q: Probe<D = Id<Post>>,
    ROf<K>: Copy + Eq + std::hash::Hash,
{
    let Post { score, view_count, answer_count, comment_count, favorite_count, creation_date, .. } = &db.post;
    db.post
        .group_by(key)
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()).and(creation_date))
        .fold([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, i64::MIN, 0], |a, (((((s, v), an), cc), f), d)| {
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
                a[9] + (s > 0) as i64,
                a[10].max(d),
                0,
            ]
        })
}

fn tname(db: &'static So, t: Id<PostType>) -> V {
    V::S(db.post_type.name.get(t).unwrap())
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// PostTypeStats AS (
//     SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(p.Id) AS TotalPosts,
//            SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AverageViewCount
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Id, pt.Name)
// SELECT u.DisplayName, u.Reputation, ups.PostCount, ups.QuestionCount, ups.AnswerCount,
//        ups.AverageScore, ups.AverageViewCount, pts.PostTypeName, pts.TotalPosts, pts.TotalScore,
//        pts.AverageViewCount
// FROM UserPostStats ups JOIN Users u ON ups.UserId = u.Id JOIN PostTypeStats pts ON ups.PostCount > 0
// ORDER BY ups.PostCount DESC, ups.AverageScore DESC;
fn q13405(db: &'static So) -> String {
    let ups = user_posts(db);
    let pts = type_left_posts(db);
    let mut out = Vec::new();
    (&ups).filt(|a| a[1] > 0).cross(&pts).drive(|(u, t), (a, b)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1]), avg(a[6], a[5]), tname(db, t), V::I(b[0]), nullable(b[1], b[0]), avg(b[3], b[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT PT.Name AS PostType, COUNT(P.Id) AS TotalPosts, AVG(P.Score) AS AvgScore,
//            SUM(P.ViewCount) AS TotalViews, SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers,
//            SUM(COALESCE(P.CommentCount, 0)) AS TotalComments,
//            SUM(COALESCE(P.FavoriteCount, 0)) AS TotalFavorites, MAX(P.CreationDate) AS LatestPostDate
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name),
// UserStats AS (
//     SELECT U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(U.Reputation) AS TotalReputation,
//            MAX(U.LastAccessDate) AS LastActivity
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.DisplayName)
// SELECT PS.PostType, PS.TotalPosts, PS.AvgScore, PS.TotalViews, PS.TotalAnswers, PS.TotalComments,
//        PS.TotalFavorites, PS.LatestPostDate, US.DisplayName AS TopUser, US.TotalBadges,
//        US.TotalReputation, US.LastActivity
// FROM PostStats PS JOIN UserStats US ON US.TotalBadges = (SELECT MAX(TotalBadges) FROM UserStats)
// ORDER BY PS.TotalPosts DESC;
fn q11680(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let User { reputation, last_access_date, .. } = &db.user;
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select(reputation.and(last_access_date).and(badges_of(db).opt()))
        .fold((0i64, 0i64, i64::MIN), |(b, r, la), ((rep, l), x)| (b + x.is_some() as i64, r + rep, la.max(l)));
    let most = (&us).fold_flat(i64::MIN, |m, (b, _, _)| m.max(b));
    let mut out = Vec::new();
    (&ps).cross((&us).filt(|(b, _, _)| b == most)).drive(|(t, name), (a, (b, r, la))| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[5]), V::I(a[6]), V::I(a[8]), tmax(a[10]), V::S(name), V::I(b), V::I(r), tmax(la)]))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//            COUNT(DISTINCT p.PostTypeId) AS UniquePostTypes, SUM(p.ViewCount) AS TotalViews,
//            SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewsPerPost, AVG(p.Score) AS AvgScorePerPost
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostTypeStats AS (
//     SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(p.Id) AS TotalPosts,
//            SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AvgScore
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Id, pt.Name)
// SELECT ups.DisplayName, ups.TotalPosts, ups.UniquePostTypes, ups.TotalViews, ups.TotalScore,
//        ups.AvgViewsPerPost, ups.AvgScorePerPost, pts.PostTypeName, pts.TotalPosts AS PostTypeTotalPosts,
//        pts.TotalViews AS PostTypeTotalViews, pts.AvgScore AS PostTypeAvgScore
// FROM UserPostStats ups CROSS JOIN PostTypeStats pts
// ORDER BY ups.TotalPosts DESC, pts.TotalViews DESC;
fn q10579(db: &'static So) -> String {
    let ups = user_posts(db);
    let ut = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).count_distinct();
    let pts = type_left_posts(db);
    let mut out = Vec::new();
    (&ups).and((&ut).opt()).cross(&pts).drive(|(u, t), ((a, n), b)| {
        out.push(row(vec![
            user_col(db, u, "name"),
            V::I(a[1]),
            V::I(n.unwrap_or(0)),
            nullable(a[6], a[5]),
            nullable(a[4], a[1]),
            avg(a[6], a[5]),
            avg(a[4], a[1]),
            tname(db, t),
            V::I(b[0]),
            nullable(b[3], b[2]),
            avg(b[1], b[0]),
        ]))
    });
    rows(out)
}

// WITH TagCounts AS (
//     SELECT t.Id AS TagId, t.TagName, COUNT(p.Id) AS PostCount
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.Id, t.TagName),
// MostActiveUsers AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostsCount,
//            SUM(COALESCE(CommentsCount, 0)) AS TotalComments, SUM(u.UpVotes) AS TotalUpVotes,
//            SUM(u.DownVotes) AS TotalDownVotes
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentsCount FROM Comments GROUP BY PostId) c ON c.PostId = p.Id
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopTags AS (SELECT TagId, TagName, PostCount FROM TagCounts WHERE PostCount > 0
//             ORDER BY PostCount DESC LIMIT 10)
// SELECT u.DisplayName AS ActiveUser, u.PostsCount, u.TotalComments, u.TotalUpVotes, u.TotalDownVotes,
//        t.TagName, t.PostCount
// FROM MostActiveUsers u JOIN TopTags t ON u.PostsCount > 5
// ORDER BY u.TotalUpVotes DESC, t.PostCount DESC;
//
// A tag name cannot contain `<` or `>`, so it is a substring of the Tags
// string exactly when it is a substring of one of its elements, which is
// what `tag_mentions` pairs.
fn q25217(db: &'static So) -> String {
    let tm = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&tm).map(|(_, t)| t).inv().collect();
    let tc = db.tag.group_by(Ident::<Tag>::new()).select((&by_tag).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tt = rel(top_n(drain((&tc).filt(|n| n > 0)), |&(_, n)| Reverse(n), 10));
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let User { up_votes, down_votes, .. } = &db.user;
    let mu = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).select((&cc).opt()).opt()))
        .fold([0i64; 4], |a, ((u, d), p)| [a[0] + p.is_some() as i64, a[1] + p.flatten().unwrap_or(0), a[2] + u, a[3] + d]);
    let mut out = Vec::new();
    (&mu).filt(|a| a[0] > 5).cross(&tt).drive(|(u, _), (a, (t, n))| {
        out.push(row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::S(db.tag.tag_name.get(t).unwrap()), V::I(n)]))
    });
    rows(out)
}

// WITH PostMetrics AS (
//     SELECT p.PostTypeId, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalQuestionsWithScore,
//            AVG(p.ViewCount) AS AvgViewCount, AVG(p.AnswerCount) AS AvgAnswerCount,
//            AVG(p.CommentCount) AS AvgCommentCount, AVG(p.FavoriteCount) AS AvgFavoriteCount
//     FROM Posts p GROUP BY p.PostTypeId),
// UserMetrics AS (
//     SELECT u.Id AS UserId, COUNT(b.Id) AS TotalBadges, SUM(u.UpVotes) AS TotalUpVotes,
//            AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT pt.Name AS PostType, pm.TotalPosts, pm.TotalQuestionsWithScore, pm.AvgViewCount,
//        pm.AvgAnswerCount, pm.AvgCommentCount, pm.AvgFavoriteCount,
//        SUM(um.TotalBadges) AS TotalBadgesAwarded, SUM(um.TotalUpVotes) AS TotalUpVotes,
//        AVG(um.AvgReputation) AS AvgUserReputation
// FROM PostTypes pt JOIN PostMetrics pm ON pt.Id = pm.PostTypeId JOIN UserMetrics um ON 1=1
// GROUP BY pt.Name, pm.TotalPosts, ... ORDER BY pm.TotalPosts DESC;
//
// Each user's AvgReputation is its reputation exactly (a mean of copies), so
// the outer AVG adds whole numbers.
fn q12369(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, favorite_count, .. } = &db.post;
    let pm = db
        .post
        .group_by(&db.post.post_type)
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()))
        .fold([0i64; 9], |a, ((((s, v), an), cc), f)| {
            [a[0] + 1, a[1] + (s > 0) as i64, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + cc, a[7] + f.is_some() as i64, a[8] + f.unwrap_or(0)]
        });
    let User { up_votes, reputation, .. } = &db.user;
    let um = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(reputation).and(badges_of(db).opt()))
        .fold((0i64, 0i64, 0i64), |(b, u, _), ((up, r), x)| (b + x.is_some() as i64, u + up, r));
    let tot = whole(&um).select(&um).fold([0i64; 4], |a, (b, u, r)| [a[0] + b, a[1] + u, a[2] + r, a[3] + 1]);
    let mut out = Vec::new();
    (&pm).cross(&tot).drive(|(t, _), (a, tot)| {
        out.push(row(vec![tname(db, t), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0]), avg(a[8], a[7]), V::I(tot[0]), V::I(tot[1]), avg(tot[2], tot[3])]))
    });
    rows(out)
}

// WITH TagCounts AS (
//     SELECT t.TagName, COUNT(p.Id) AS PostCount
//     FROM Tags t LEFT JOIN Posts p ON p.Tags ILIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// PopularUsers AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostsCreated, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.DisplayName
//     ORDER BY PostsCreated DESC LIMIT 10),
// MostActivePosts AS (
//     SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.ViewCount > 1000 GROUP BY p.Id, p.Title
//     ORDER BY CommentCount DESC, TotalVotes DESC LIMIT 5)
// SELECT tc.TagName, tc.PostCount, pu.DisplayName AS PopularUser, pu.PostsCreated, pu.TotalScore,
//        mp.Title AS ActivePostTitle, mp.CommentCount, mp.TotalVotes
// FROM TagCounts tc CROSS JOIN PopularUsers pu CROSS JOIN MostActivePosts mp
// ORDER BY tc.PostCount DESC, pu.PostsCreated DESC, mp.CommentCount DESC;
fn q25883(db: &'static So) -> String {
    let elems: MatSet<Str> = (&db.post.tags_str).flat_map(tag_list).collect();
    let lower = |s: Str| -> Str { Box::leak(s.to_lowercase().into_boxed_str()) };
    let contains: HashIdx<Str, Id<Tag>> = (&elems).select_where((&db.tag.tag_name).inv(), move |e: Str, n: Str| lower(e).contains(lower(n))).collect();
    let tm: MatSet<(Id<Post>, Id<Tag>)> = db.post.select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&contains))).collect();
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&tm).map(|(_, t)| t).inv().collect();
    let tc = db.tag.group_by(&db.tag.tag_name).select((&by_tag).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pu = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let pu = rel(top_n(drain(&pu), |&(_, (n, _))| Reverse(n), 10));
    let mp = db
        .post
        .with((&db.post.view_count).gt(1000))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(c, v), (x, y)| (c + x.is_some() as i64, v + y.is_some() as i64));
    let mp = rel(top_n(drain(&mp), |&(_, (c, v))| (Reverse(c), Reverse(v)), 5));
    let mut out = Vec::new();
    (&tc).cross(&pu).cross(&mp).drive(|((name, _), _), ((n, (u, (pn, ps))), (p, (c, v)))| {
        out.push(row(vec![V::S(name), V::I(n), user_col(db, u, "name"), V::I(pn), V::I(ps), title(db, p), V::I(c), V::I(v)]))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//            SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//            SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     WHERE u.CreationDate >= '2020-01-01' GROUP BY u.Id, u.DisplayName),
// PostTypeStats AS (
//     SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(p.Id) AS PostCount,
//            SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Id, pt.Name)
// SELECT u.UserId, u.DisplayName, u.PostCount, u.TotalViews, u.TotalScore, u.TotalComments,
//        p.PostTypeId, p.PostTypeName, p.PostCount AS TypePostCount, p.TotalViews AS TypeTotalViews,
//        p.TotalScore AS TypeTotalScore
// FROM UserPostStats u JOIN PostTypeStats p ON u.PostCount > 0
// ORDER BY u.TotalScore DESC, p.TotalViews DESC;
fn q12637(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let Post { view_count, score, .. } = &db.post;
    let ups = db
        .user
        .with((&db.user.creation_date).ge(date(2020, 1, 1)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(score).and((&cc).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((v, s), c)) => [a[0] + 1, a[1] + v.unwrap_or(0), a[2] + s, a[3] + c.unwrap_or(0)],
            None => a,
        });
    let pts = type_left_posts(db);
    let mut out = Vec::new();
    (&ups).filt(|a| a[0] > 0).cross(&pts).drive(|(u, t), (a, b)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(db.post_type.origid.get(t).unwrap()), tname(db, t), V::I(b[0]), V::I(b[3]), V::I(b[1])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts,
//            COUNT(CASE WHEN p.Score > 0 THEN 1 END) AS PositiveScorePosts,
//            AVG(p.ViewCount) AS AverageViews, AVG(p.AnswerCount) AS AverageAnswers,
//            AVG(p.CommentCount) AS AverageComments
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS TotalBadges, SUM(b.Class) AS TotalBadgeClass,
//            SUM(v.BountyAmount) AS TotalBounty
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.DisplayName),
// CommentStats AS (SELECT COUNT(c.Id) AS TotalComments, AVG(c.Score) AS AverageCommentScore FROM Comments c)
// SELECT ps.*, us.*, cs.* FROM PostStats ps CROSS JOIN UserStats us CROSS JOIN CommentStats cs
// ORDER BY ps.TotalPosts DESC, us.TotalBadges DESC;
fn q12758(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 5], |a, (c, v)| {
            let v = v.flatten();
            [a[0] + c.is_some() as i64, a[1] + c.is_some() as i64, a[2] + c.unwrap_or(0), a[3] + v.is_some() as i64, a[4] + v.unwrap_or(0)]
        });
    let (cn, cs) = (&db.comment.score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mut out = Vec::new();
    (&ps).cross(&us).drive(|(t, u), (a, b)| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(a[9]), avg(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0])];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b[0]), nullable(b[2], b[1]), nullable(b[4], b[3]), V::I(cn), avg(cs, cn)]);
        out.push(row(f))
    });
    rows(out)
}


/// Every tag whose name occurs in the post's Tags string, each pair once:
/// `JOIN Tags t ON p.Tags [I]LIKE '%' || t.TagName || '%'`. A tag name has
/// no `<` or `>`, so matching the whole string is matching one element.
fn like_tags(db: &'static So, ci: bool) -> MatSet<(Id<Post>, Id<Tag>)> {
    let elems: MatSet<Str> = (&db.post.tags_str).flat_map(tag_list).collect();
    let contains: HashIdx<Str, Id<Tag>> = (&elems)
        .select_where((&db.tag.tag_name).inv(), move |e: Str, n: Str| if ci { e.to_lowercase().contains(&n.to_lowercase()) } else { e.contains(n) })
        .collect();
    db.post.select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&contains))).collect()
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//            SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoredPosts, AVG(p.Score) AS AverageScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PopularTags AS (
//     SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
//     GROUP BY t.TagName HAVING COUNT(p.Id) > 10 ORDER BY TotalViews DESC LIMIT 5),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges FROM Badges b GROUP BY b.UserId)
// SELECT ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.PositiveScoredPosts,
//        ups.AverageScore, ut.TagName, ub.TotalBadges
// FROM UserPostStats ups LEFT JOIN UserBadges ub ON ups.UserId = ub.UserId CROSS JOIN PopularTags ut
// WHERE ups.TotalPosts > 5 ORDER BY ups.AverageScore DESC, ups.TotalPosts DESC;
fn q6303(db: &'static So) -> String {
    let ups = user_posts(db);
    let lt = like_tags(db, false);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let pt = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select((&db.post.view_count).opt()))
        .fold((0i64, 0i64, 0i64), |(n, wn, w), v| (n + 1, wn + v.is_some() as i64, w + v.unwrap_or(0)));
    let pt = rel(top_n(drain((&pt).filt(|(n, _, _)| n > 10)), |&(_, (_, wn, w))| (wn == 0, Reverse(w)), 5));
    let ub = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&ups).filt(|a| a[1] > 5).and((&ub).opt()).cross(&pt).drive(|(u, _), ((a, b), (t, _))| {
        out.push(row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), avg(a[4], a[1]), V::S(t), oint(b)]))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//            AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostTypeCounts AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Name),
// BadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS TotalBadges
//                 FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers,
//        ups.AverageScore AS UserAverageScore, ups.TotalViews, bc.TotalBadges, ptc.PostType,
//        ptc.PostCount, ptc.AverageScore AS PostTypeAverageScore
// FROM UserPostStats ups JOIN BadgeCounts bc ON ups.UserId = bc.UserId
// JOIN PostTypeCounts ptc ON ptc.PostCount > 0
// ORDER BY ups.TotalPosts DESC, ptc.PostCount DESC;
fn q11089(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ptc = type_left_posts_by_name(db);
    let mut out = Vec::new();
    (&ups).and(&bc).cross((&ptc).filt(|b| b[0] > 0)).drive(|(u, t), ((a, b), p)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1]), nullable(a[6], a[5]), V::I(b), V::S(t), V::I(p[0]), avg(p[1], p[0])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStats AS (SELECT Id AS UserId, Reputation, UpVotes, DownVotes, Views, CreationDate FROM Users),
// PostStats AS (
//     SELECT p.Id AS PostId, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
//            p.CommentCount, p.FavoriteCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id),
// AggregatePostStats AS (
//     SELECT PostTypeId, COUNT(*) AS TotalPosts, SUM(ViewCount) AS TotalViews, AVG(Score) AS AverageScore,
//            SUM(AnswerCount) AS TotalAnswers, SUM(CommentCount) AS TotalComments
//     FROM PostStats GROUP BY PostTypeId),
// UserPostStats AS (
//     SELECT u.Id AS UserId, COUNT(p.Id) AS TotalPostsByUser, SUM(p.ViewCount) AS TotalViewsByUser,
//            AVG(p.Score) AS AverageScoreByUser
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id)
// SELECT u.UserId, u.Reputation, ups.TotalPostsByUser, ups.TotalViewsByUser, ups.AverageScoreByUser,
//        aps.PostTypeId, aps.TotalPosts AS TotalPostsByType, aps.TotalViews AS TotalViewsByType,
//        aps.AverageScore AS AverageScoreByType, aps.TotalAnswers, aps.TotalComments
// FROM UserStats u JOIN UserPostStats ups ON u.UserId = ups.UserId
// JOIN AggregatePostStats aps ON aps.TotalPosts > 0
// ORDER BY u.Reputation DESC, aps.TotalViews DESC;
fn q12773(db: &'static So) -> String {
    let ups = user_posts(db);
    let Post { score, view_count, answer_count, comment_count, .. } = &db.post;
    let aps = db
        .post
        .group_by(&db.post.post_type_id)
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count))
        .fold([0i64; 7], |a, (((s, v), an), c)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s, a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c]);
    let mut out = Vec::new();
    (&ups).cross((&aps).filt(|b| b[0] > 0)).drive(|(u, t), (a, b)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[1]), nullable(a[6], a[5]), avg(a[4], a[1]), V::I(t), V::I(b[0]), nullable(b[2], b[1]), avg(b[3], b[0]), nullable(b[5], b[4]), V::I(b[6])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH post_stats AS (
//     SELECT PT.Name AS PostType, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore,
//            AVG(P.ViewCount) AS AvgViewCount, AVG(P.AnswerCount) AS AvgAnswerCount
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name),
// user_stats AS (
//     SELECT U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(U.UpVotes) AS TotalUpVotes, SUM(U.DownVotes) AS TotalDownVotes
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Reputation),
// comment_stats AS (SELECT COUNT(C.Id) AS TotalComments, AVG(LENGTH(C.Text)) AS AvgCommentLength FROM Comments C),
// vote_stats AS (SELECT VT.Name AS VoteType, COUNT(V.Id) AS VoteCount
//                FROM Votes V JOIN VoteTypes VT ON V.VoteTypeId = VT.Id GROUP BY VT.Name)
// SELECT PS.PostType, PS.PostCount, PS.TotalScore, PS.AvgViewCount, PS.AvgAnswerCount, US.Reputation,
//        US.BadgeCount, US.TotalUpVotes, US.TotalDownVotes, CS.TotalComments, CS.AvgCommentLength,
//        VS.VoteType, VS.VoteCount
// FROM post_stats PS
// CROSS JOIN (SELECT DISTINCT Reputation, BadgeCount, TotalUpVotes, TotalDownVotes FROM user_stats) US
// CROSS JOIN (SELECT TotalComments, AvgCommentLength FROM comment_stats) CS
// CROSS JOIN (SELECT VoteType, VoteCount FROM vote_stats) VS
// ORDER BY PS.PostType;
fn q11531(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = db
        .user
        .group_by(&db.user.reputation)
        .select(up_votes.and(down_votes).and(badges_of(db).opt()))
        .fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let us_rows: MatSet<(i64, [i64; 3])> = whole(&us).select(Same::new().and(&us)).collect();
    let (cn, cl) = (&db.comment.text).fold_flat((0i64, 0i64), |(n, l), t| (n + 1, l + t.chars().count() as i64));
    let vs = db.vote.group_by(vtype_name(db)).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&ps).cross(&us_rows).cross(&vs).drive(|((t, _), vt), ((a, (r, u)), vn)| {
        out.push(row(vec![V::S(t), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), avg(a[5], a[4]), V::I(r), V::I(u[0]), V::I(u[1]), V::I(u[2]), V::I(cn), avg(cl, cn), V::S(vt), V::I(vn)]))
    });
    rows(out)
}

// WITH RecentUsers AS (
//     SELECT Id AS UserId, DisplayName, Reputation, Views, UpVotes, DownVotes, CreationDate, LastAccessDate
//     FROM Users WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PopularTags AS (
//     SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews
//     FROM Tags t JOIN Posts p ON p.Tags ILIKE '%' || t.TagName || '%'
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months'
//     GROUP BY t.TagName HAVING COUNT(p.Id) > 10 ORDER BY TotalViews DESC LIMIT 5),
// UserInteractions AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//            AVG(b.Class) AS AverageBadgeClass
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT u.DisplayName AS User, u.Reputation, u.Views, u.UpVotes, u.DownVotes, u.CreationDate,
//        u.LastAccessDate, pt.TagName AS PopularTag, ut.CommentCount, ut.VoteCount, ut.AverageBadgeClass
// FROM RecentUsers u JOIN UserInteractions ut ON u.UserId = ut.UserId JOIN PopularTags pt ON pt.PostCount > 1
// ORDER BY u.LastAccessDate DESC, ut.VoteCount DESC;
fn q28894(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let lt = like_tags(db, true);
    let recent: MatSet<Id<Post>> = db.post.with((&db.post.creation_date).ge(add_months(now, -6))).collect();
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t)| t).inv().map(|(p, _)| p).with(&recent).collect();
    let pt = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).select((&db.post.view_count).opt()))
        .fold((0i64, 0i64, 0i64), |(n, wn, w), v| (n + 1, wn + v.is_some() as i64, w + v.unwrap_or(0)));
    let pt = rel(top_n(drain((&pt).filt(|(n, _, _)| n > 10)), |&(_, (_, wn, w))| (wn == 0, Reverse(w)), 5));
    let ui = db
        .user
        .with((&db.user.creation_date).ge(add_years(now, -1)))
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, ((c, v), b)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]);
    let mut out = Vec::new();
    (&ui).cross((&pt).filt(|(_, (n, _, _))| n > 1)).drive(|(u, _), (a, (t, _))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews", "uup", "udown", "ucreated", "last_access"]);
        f.extend([V::S(t), V::I(a[0]), V::I(a[1]), avg(a[3], a[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN P.ViewCount ELSE 0 END) AS TotalQuestionViews,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN P.ViewCount ELSE 0 END) AS TotalAnswerViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopQuestions AS (
//     SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, U.DisplayName AS OwnerName
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1
//     ORDER BY P.ViewCount DESC LIMIT 5),
// UserBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B GROUP BY B.UserId)
// SELECT UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.QuestionsCount, UPS.AnswersCount,
//        UPS.TotalQuestionViews, UPS.TotalAnswerViews, UB.BadgeCount, TQ.PostId, TQ.Title AS TopQuestionTitle,
//        TQ.Score, TQ.ViewCount AS TopQuestionViews, TQ.CreationDate AS TopQuestionDate, TQ.OwnerName AS TopQuestionOwner
// FROM UserPostStats UPS LEFT JOIN UserBadges UB ON UPS.UserId = UB.UserId
// LEFT JOIN TopQuestions TQ ON UPS.QuestionsCount > 0
// WHERE UPS.TotalPosts > 0 ORDER BY UPS.TotalPosts DESC, UPS.UserId;
//
// The CASE is NULL for a question with no ViewCount and 0 for any other
// post, so each SUM is NULL only when every row it saw was NULL.
fn q9861(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt())))
        .fold([0i64; 7], |a, (t, v)| {
            let q = if t == 1 { v } else { Some(0) };
            let an = if t == 2 { v } else { Some(0) };
            [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + q.is_some() as i64, a[4] + q.unwrap_or(0), a[5] + an.is_some() as i64, a[6] + an.unwrap_or(0)]
        });
    let tq = rel(top_n(drain(owned(db).with(post_type_id.eq(1)).select(view_count.opt())), |&(_, v)| (v.is_none(), Reverse(v)), 5));
    let tq: HashIdx<(), (Id<Post>, Option<i64>)> = (&tq).map(|_| ()).inv().select(&tq).collect();
    let ub = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&ups).filt(|a| a[0] > 0).and((&ub).opt()).and((&ups).filt(|a| a[1] > 0).map(|_| ()).select(&tq).opt()).drive(|u, ((a, b), q)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), nullable(a[6], a[5]), oint(b)]);
        match q {
            Some((p, _)) => f.extend(post_fields(db, p, &["id", "title", "score", "views", "created", "owner"])),
            None => f.extend((0..6).map(|_| V::Null)),
        }
        out.push(row(f))
    });
    rows(out)
}

// WITH PostMetrics AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount,
//            COALESCE(p.AnswerCount, 0) AS AnswerCount, COALESCE(p.CommentCount, 0) AS CommentCount,
//            COALESCE(p.FavoriteCount, 0) AS FavoriteCount,
//            ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><'), 1) AS TagCount,
//            u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TagDetails AS (
//     SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(pm.ViewCount) AS TotalViews, AVG(pm.Score) AS AverageScore
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%<' || t.TagName || '>%'
//     LEFT JOIN PostMetrics pm ON pm.PostId = p.Id GROUP BY t.TagName),
// TopPosts AS (
//     SELECT pm.PostId, pm.Title, pm.OwnerDisplayName, pm.OwnerReputation, pm.ViewCount, pm.AnswerCount,
//            pm.CommentCount, pm.FavoriteCount, pm.TagCount, pm.CreationDate
//     FROM PostMetrics pm ORDER BY pm.Score DESC, pm.ViewCount DESC LIMIT 10)
// SELECT td.TagName, td.PostCount, td.TotalViews, td.AverageScore, tp.Title AS TopPostTitle,
//        tp.OwnerDisplayName AS TopPostOwner, tp.ViewCount AS TopPostViewCount, tp.CreationDate AS TopPostCreationDate
// FROM TagDetails td LEFT JOIN TopPosts tp ON tp.TagCount > 0
// ORDER BY td.PostCount DESC, td.TotalViews DESC;
//
// `LIKE '%<' || name || '>%'` is exactly "the post has this tag", which is
// the exploded `Post.tags` edge.
fn q26066(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, tags_str, .. } = &db.post;
    let pm: MatSet<Id<Post>> = owned(db).with(post_type_id.eq(1)).collect();
    let tagged: HashIdx<Id<Tag>, Id<Post>> = (&db.post.tags).inv().collect();
    let td = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&tagged).select(Ident::<Post>::new().with(&pm).select(score.and(view_count.opt())).opt()).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(Some((s, v))) => [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + 1, a[4] + s],
            Some(None) => [a[0] + 1, a[1], a[2], a[3], a[4]],
            None => a,
        });
    let tp = top_n(drain((&pm).select(score.and(view_count.opt()))), |&(_, (s, v))| (Reverse(s), v.is_none(), Reverse(v)), 10);
    let tp = rel(tp);
    let tp = (&tp).map(|(p, _)| p).with(tags_str.map(split_n).gt(0));
    let tp: HashIdx<(), Id<Post>> = (&tp).map(|_| ()).inv().select(&tp).collect();
    let mut out = Vec::new();
    (&td).and(Same::<Str>::new().map(|_| ()).select(&tp).opt()).drive(|name, (a, p)| {
        let mut f = vec![V::S(name), V::I(a[0]), nullable(a[2], a[1]), avg(a[4], a[3])];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "owner", "views", "created"])),
            None => f.extend((0..4).map(|_| V::Null)),
        }
        out.push(row(f))
    });
    rows(out)
}

fn split_n(s: Str) -> i64 {
    let n = s.chars().count();
    let inner: String = s.chars().skip(1).take(n.saturating_sub(2)).collect();
    inner.split("><").count() as i64
}

// WITH TagWordCount AS (
//     SELECT TagName, SUM(LENGTH(Tags) - LENGTH(REPLACE(Tags, '<', '')) / LENGTH('<')) AS TagCount,
//            COUNT(*) as PostCount
//     FROM Tags INNER JOIN Posts ON Tags.Id = Posts.Id WHERE Posts.PostTypeId = 1 GROUP BY TagName),
// UsersWithBadges AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount,
//            SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, ... Silver, ... Bronze
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// HighReputationUsers AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName FROM Users U WHERE U.Reputation > 1000),
// QuestionStatistics AS (
//     SELECT P.OwnerUserId, COUNT(P.Id) AS QuestionCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount
//     FROM Posts P WHERE P.PostTypeId = 1 GROUP BY P.OwnerUserId)
// SELECT U.DisplayName AS User, U.Reputation, UA.BadgeCount, UA.GoldBadges, UA.SilverBadges, UA.BronzeBadges,
//        QS.QuestionCount, QS.TotalScore, QS.AvgViewCount, TW.TagName, TW.TagCount, TW.PostCount
// FROM HighReputationUsers U LEFT JOIN UsersWithBadges UA ON U.UserId = UA.UserId
// LEFT JOIN QuestionStatistics QS ON U.UserId = QS.OwnerUserId
// LEFT JOIN (SELECT TagName, COUNT(*) AS TagCount, SUM(PostCount) AS PostCount FROM TagWordCount
//            GROUP BY TagName HAVING COUNT(*) > 5) TW ON 1=1
// ORDER BY U.Reputation DESC, QS.TotalScore DESC;
//
// TagWordCount has one row per TagName, so TW's HAVING COUNT(*) > 5 keeps
// nothing and the LEFT JOIN supplies its NULL row.
fn q29930(db: &'static So) -> String {
    let pids: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let twc = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&db.tag.origid).select(&pids).with((&db.post.post_type_id).eq(1)))
        .fold(0i64, |n, _| n + 1);
    let tw = whole(&twc).group_by(Same::new()).select(&twc).fold((0i64, 0i64), |(n, s), c| (n + 1, s + c));
    let tw: HashIdx<(), (Str, (i64, i64))> = (&tw).filt(|(n, _)| n > 5).map(|_| ()).inv().select(Same::<Str>::new().and(&tw)).collect();
    let ua = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let qs = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(&db.post.owner_user)
        .select((&db.post.score).and((&db.post.view_count).opt()))
        .fold([0i64; 4], |a, (s, v)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]);
    let mut out = Vec::new();
    db.user
        .with((&db.user.reputation).gt(1000))
        .select(Ident::<User>::new().and(&ua).and((&qs).opt()).and(Ident::<User>::new().map(|_| ()).select(&tw).opt()))
        .drive(|_, (((u, a), q), t)| {
            let mut f = ucols(db, u, &["name", "rep"]);
            f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
            f.extend(match q {
                Some(q) => [V::I(q[0]), V::I(q[1]), avg(q[3], q[2])],
                None => [V::Null, V::Null, V::Null],
            });
            f.extend(match t {
                Some((name, (n, c))) => [V::S(name), V::I(n), V::I(c)],
                None => [V::Null, V::Null, V::Null],
            });
            out.push(row(f))
        });
    rows(out)
}


// SELECT P.Id AS PostId, P.Title, U.DisplayName AS OwnerDisplayName, P.CreationDate, P.ViewCount, P.Score,
//        C.CommentCount, T.TagName
// FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// LEFT JOIN (SELECT PostId, STRING_AGG(TagName, ', ') AS TagName FROM Tags GROUP BY PostId) T ON P.Id = T.PostId
// WHERE P.PostTypeId = 1
// GROUP BY P.Id, P.Title, U.DisplayName, P.CreationDate, P.ViewCount, P.Score, C.CommentCount, T.TagName
// ORDER BY P.CreationDate DESC LIMIT 10;
//
// Tags has no PostId, so DuckDB binds it to the C.PostId beside it and runs
// T as a LATERAL subquery: one row per C row, (C.PostId, every tag name).
// T then matches exactly when C did. The aggregate has no order; the
// rewrite orders it by Id.
fn q16303(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let all = whole(&db.tag.id).select(Same::new().and(&db.tag.origid).and(&db.tag.tag_name)).buf_fold(|mut v| {
        v.sort_by_key(|&((_, i), _)| i);
        leak_join(v.iter().map(|&(_, n)| n), ", ")
    });
    let all = (&all).fold_flat(None, |_, s| Some(s));
    let v = top_n(drain(owned(db).with((&db.post.post_type_id).eq(1)).select((&db.post.creation_date).and((&cc).opt()))), |&(_, (d, _))| Reverse(d), 10);
    rows(v.iter().map(|&(p, (_, c))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score"]);
        f.extend([oint(c), ostr(c.and(all))]);
        row(f)
    }))
}

fn leak_join(parts: impl IntoIterator<Item = Str>, sep: &str) -> Str {
    Box::leak(parts.into_iter().collect::<Vec<_>>().join(sep).into_boxed_str())
}

// SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts,
//        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//        SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS HighlyViewedPosts,
//        AVG(p.ViewCount) AS AverageViewCount,
//        AVG(EXTRACT(EPOCH FROM (COALESCE(p.LastActivityDate, CURRENT_TIMESTAMP) - p.CreationDate))) AS AveragePostAgeInSeconds,
//        COUNT(DISTINCT p.OwnerUserId) AS UniquePostOwners, SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY pt.Name ORDER BY TotalPosts DESC;
fn q12968(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let Post { score, view_count, last_activity_date, creation_date, owner_user_id, .. } = &db.post;
    let ps = db
        .post
        .group_by(ptype_name(db))
        .select(score.and(view_count.opt()).and(last_activity_date).and(creation_date).and((&cc).opt()))
        .fold(([0i64; 6], (0.0f64, 0.0f64)), |(a, t), ((((s, v), la), cd), c)| {
            (
                [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (v.unwrap_or(0) > 100) as i64, a[3] + v.is_some() as i64, a[4] + v.unwrap_or(0), a[5] + c.unwrap_or(0)],
                kahan(t, secs(tz_sub(la, cd))),
            )
        });
    let owners = db.post.group_by(ptype_name(db)).select(owner_user_id).count_distinct();
    let mut out = Vec::new();
    (&ps).and((&owners).opt()).drive(|t, ((a, d), o)| {
        out.push(row(vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), fmean(d, a[0]), V::I(o.unwrap_or(0)), V::I(a[5])]))
    });
    rows(out)
}

// WITH PostCounts AS (SELECT PostTypeId, COUNT(*) AS TotalPosts FROM Posts GROUP BY PostTypeId),
// UserActivity AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName)
// SELECT PCT.PostTypeId, PCT.TotalPosts, UA.UserId, UA.DisplayName, UA.PostCount, UA.Questions, UA.Answers
// FROM PostCounts PCT JOIN UserActivity UA ON UA.PostCount > 0
// ORDER BY PCT.PostTypeId, UA.PostCount DESC;
fn q14471(db: &'static So) -> String {
    let pc = db.post.group_by(&db.post.post_type_id).fold(0i64, |n, _| n + 1);
    let ua = user_posts(db);
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let mut out = Vec::new();
    (&pc).cross((&dp).filt(|n| n > 0).and(&ua)).drive(|(t, u), (n, (d, a))| {
        let mut f = vec![V::I(t), V::I(n)];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(d), V::I(a[2]), V::I(a[3])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostCounts AS (SELECT DATE_TRUNC('month', CreationDate) AS Month, COUNT(*) AS TotalPosts FROM Posts GROUP BY Month),
// UserCounts AS (... the same over Users ...), VoteCounts AS (... the same over Votes ...)
// SELECT COALESCE(pc.Month, uc.Month, vc.Month) AS Month, COALESCE(pc.TotalPosts, 0) AS TotalPosts,
//        COALESCE(uc.TotalUsers, 0) AS TotalUsers, COALESCE(vc.TotalVotes, 0) AS TotalVotes
// FROM PostCounts pc FULL OUTER JOIN UserCounts uc ON pc.Month = uc.Month
// FULL OUTER JOIN VoteCounts vc ON pc.Month = vc.Month
// ORDER BY Month;
//
// The second join is on pc.Month, not the COALESCE: a month with users but
// no posts cannot meet its votes, so the rows are the post months (with
// their users and votes), the user months without posts, and the vote
// months without posts, each on its own.
fn q13649(db: &'static So) -> String {
    let pc = db.post.group_by((&db.post.creation_date).map(trunc_month)).fold(0i64, |n, _| n + 1);
    let uc = db.user.group_by((&db.user.creation_date).map(trunc_month)).fold(0i64, |n, _| n + 1);
    let vc = db.vote.group_by((&db.vote.creation_date).map(trunc_month)).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&pc).and((&uc).opt()).and((&vc).opt()).drive(|m, ((p, u), v)| out.push(row(vec![V::T(m), V::I(p), V::I(u.unwrap_or(0)), V::I(v.unwrap_or(0))])));
    (&uc).minus(&pc).drive(|m, u| out.push(row(vec![V::T(m), V::I(0), V::I(u), V::I(0)])));
    (&vc).minus(&pc).drive(|m, v| out.push(row(vec![V::T(m), V::I(0), V::I(0), V::I(v)])));
    rows(out)
}

// WITH PostStatistics AS (
//     SELECT PT.Name AS PostType, COUNT(P.Id) AS PostCount,
//            SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY PT.Name),
// UserStatistics AS (
//     SELECT U.Reputation, COUNT(B.Id) AS BadgeCount, COUNT(DISTINCT P.Id) AS UserPostCount
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     GROUP BY U.Reputation)
// SELECT PS.PostType, PS.PostCount, PS.UpVotes, PS.DownVotes, US.Reputation, US.BadgeCount, US.UserPostCount
// FROM PostStatistics PS JOIN UserStatistics US ON US.UserPostCount > 0
// ORDER BY PS.PostType, US.Reputation;
fn q14739(db: &'static So) -> String {
    let ps = db
        .post
        .group_by(ptype_name(db))
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bc = db.user.group_by(&db.user.reputation).select(badges_of(db).opt().and(posts_of(db).opt())).fold(0i64, |n, (b, _)| n + b.is_some() as i64);
    let dp = db.user.group_by(&db.user.reputation).select(posts_of(db)).count_distinct();
    let mut out = Vec::new();
    (&ps).cross((&dp).filt(|n| n > 0).and(&bc)).drive(|(t, r), (a, (n, b))| {
        out.push(row(vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r), V::I(b), V::I(n)]))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount,
//            SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(c.Id, 0)) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId
//     GROUP BY u.Id, u.Reputation),
// PostTypesStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Name)
// SELECT us.UserId, us.Reputation, us.PostCount, us.TotalViews, us.TotalComments, pts.PostType,
//        pts.PostCount AS PostTypeCount, pts.TotalViews AS PostTypeTotalViews, pts.AverageScore
// FROM UserStats us CROSS JOIN PostTypesStats pts ORDER BY us.Reputation DESC, pts.PostType;
fn q13397(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(comments_of(db).select(&db.comment.origid).opt())).opt())
        .fold((0i64, 0i64), |(w, c), p| match p {
            Some((v, x)) => (w + v.unwrap_or(0), c + x.unwrap_or(0)),
            None => (w, c),
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let pts = type_left_posts_by_name(db);
    let mut out = Vec::new();
    (&us).and((&dp).opt()).cross(&pts).drive(|(u, t), (((w, c), n), b)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(w), V::I(c), V::S(t), V::I(b[0]), nullable(b[3], b[2]), avg(b[1], b[0])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount,
//            COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id GROUP BY u.Id),
// PostTypeStats AS (
//     SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(p.Id) AS TotalPosts,
//            AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Id, pt.Name)
// SELECT ups.UserId, ups.PostCount, ups.CommentCount, ups.TotalBounty, pts.PostTypeId, pts.PostTypeName,
//        pts.TotalPosts, pts.AverageScore, pts.TotalViews
// FROM UserPostStats ups CROSS JOIN PostTypeStats pts ORDER BY ups.UserId, pts.PostTypeId;
//
// `v.UserId = u.Id` with `p.OwnerUserId = u.Id` is the owner's own votes on
// the post.
fn q12011(db: &'static So) -> String {
    let Vote { user, post, .. } = &db.vote;
    let own: HashIdx<Id<Post>, Id<Vote>> = db.vote.with(user.and(post.select(&db.post.owner_user)).filt(|(a, b)| a == b)).select(post).inv().collect();
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&own).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold((0i64, 0i64, 0i64), |(n, bn, b), p| match p {
            Some((_, v)) => {
                let v = v.flatten();
                (n + 1, bn + v.is_some() as i64, b + v.unwrap_or(0))
            }
            None => (n, bn, b),
        });
    let dc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).count_distinct();
    let pts = type_left_posts(db);
    let mut out = Vec::new();
    (&ups).and((&dc).opt()).cross(&pts).drive(|(u, t), (((n, _, b), c), p)| {
        out.push(row(vec![user_col(db, u, "uid"), V::I(n), V::I(c.unwrap_or(0)), V::I(b), V::I(db.post_type.origid.get(t).unwrap()), tname(db, t), V::I(p[0]), avg(p[1], p[0]), nullable(p[3], p[2])]))
    });
    rows(out)
}

// WITH PostStatistics AS (
//     SELECT Pt.Name AS PostType, COUNT(P.Id) AS PostCount, AVG(P.Score) AS AverageScore, SUM(P.ViewCount) AS TotalViews
//     FROM Posts P INNER JOIN PostTypes Pt ON P.PostTypeId = Pt.Id
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY Pt.Name),
// UserStatistics AS (
//     SELECT U.DisplayName, COUNT(DISTINCT P.Id) AS PostsCreated, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON U.Id = V.UserId
//     GROUP BY U.DisplayName)
// SELECT PS.PostType, PS.PostCount, PS.AverageScore, PS.TotalViews, US.DisplayName, US.PostsCreated, US.TotalBounty
// FROM PostStatistics PS CROSS JOIN UserStatistics US ORDER BY PS.PostCount DESC, US.PostsCreated DESC;
fn q11055(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)))
        .group_by(ptype_name(db))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, v)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]);
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64), |(bn, b), (_, v)| {
            let v = v.flatten();
            (bn + v.is_some() as i64, b + v.unwrap_or(0))
        });
    let dp = db.user.group_by(&db.user.display_name).select(posts_of(db)).count_distinct();
    let mut out = Vec::new();
    (&ps).cross((&us).and((&dp).opt())).drive(|(t, name), (a, ((bn, b), n))| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::S(name), V::I(n.unwrap_or(0)), nullable(b, bn)]))
    });
    rows(out)
}


// WITH PostCounts AS (SELECT PostTypeId, COUNT(*) AS TotalPosts, COUNT(DISTINCT OwnerUserId) AS UniqueUsers
//                     FROM Posts GROUP BY PostTypeId),
// UserStatistics AS (
//     SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//            SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// BadgeCounts AS (SELECT UserId, COUNT(*) AS TotalBadges FROM Badges GROUP BY UserId)
// SELECT pt.Name AS PostType, pc.TotalPosts, pc.UniqueUsers, us.TotalPosts AS PostsByUsers, us.TotalScore,
//        us.TotalViews, bc.TotalBadges
// FROM PostCounts pc JOIN PostTypes pt ON pc.PostTypeId = pt.Id
// LEFT JOIN UserStatistics us ON us.TotalPosts > 0 LEFT JOIN BadgeCounts bc ON bc.UserId = us.UserId
// ORDER BY pc.TotalPosts DESC;
fn q12584(db: &'static So) -> String {
    let pc = db.post.group_by(&db.post.post_type).fold(0i64, |n, _| n + 1);
    let uu = db.post.group_by(&db.post.post_type).select(&db.post.owner_user_id).count_distinct();
    let ups = user_posts(db);
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let bc = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let us = (&dp).filt(|n| n > 0).and(&ups).and((&bc).opt());
    let us: HashIdx<(), ((i64, [i64; 10]), Option<i64>)> = (&us).map(|_| ()).inv().select(&us).collect();
    let mut out = Vec::new();
    (&pc).and((&uu).opt()).and(Ident::<PostType>::new().map(|_| ()).select(&us).opt()).drive(|t, ((n, o), x)| {
        let mut f = vec![tname(db, t), V::I(n), V::I(o.unwrap_or(0))];
        f.extend(match x {
            Some(((d, a), b)) => [V::I(d), V::I(a[4]), V::I(a[6]), oint(b)],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT p.ParentId) AS TotalAnswers,
//            MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostTypeCounts AS (SELECT p.PostTypeId, COUNT(p.Id) AS Count FROM Posts p GROUP BY p.PostTypeId),
// VotesSummary AS (SELECT v.PostId, COUNT(v.Id) AS TotalVotes FROM Votes v GROUP BY v.PostId)
// SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalAnswers, ups.LastPostDate, ptc.PostTypeId,
//        ptc.Count AS PostsOfType, COALESCE(vs.TotalVotes, 0) AS VoteCount
// FROM UserPostStats ups LEFT JOIN PostTypeCounts ptc ON ptc.PostTypeId IN (1, 2, 3, 4, 5, 6, 7, 8)
// LEFT JOIN VotesSummary vs ON ups.UserId = vs.PostId
// ORDER BY ups.TotalPosts DESC;
//
// The last join compares a user id with a post id, as written.
fn q14201(db: &'static So) -> String {
    let ups = user_posts(db);
    let pa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.parent_id)).count_distinct();
    let ptc = db.post.with((&db.post.post_type_id).is_in([1, 2, 3, 4, 5, 6, 7, 8])).group_by(&db.post.post_type_id).fold(0i64, |n, _| n + 1);
    let vs = db.vote.group_by(&db.vote.post_id).fold(0i64, |n, _| n + 1);
    let ptc: HashIdx<(), (i64, i64)> = (&ptc).map(|_| ()).inv().select(Same::<i64>::new().and(&ptc)).collect();
    let mut out = Vec::new();
    (&ups).and((&pa).opt()).and(Ident::<User>::new().map(|_| ()).select(&ptc).opt()).and((&db.user.origid).select(&vs).opt()).drive(|u, (((a, pa), x), v)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(pa.unwrap_or(0)), tmax(a[7])]);
        f.extend(match x {
            Some((t, n)) => [V::I(t), V::I(n)],
            None => [V::Null, V::Null],
        });
        f.push(V::I(v.unwrap_or(0)));
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStatistics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore,
//            SUM(p.ViewCount) AS TotalViews, COUNT(DISTINCT p.OwnerUserId) AS TotalOwners
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY pt.Name),
// UserStatistics AS (
//     SELECT u.DisplayName, COUNT(b.Id) AS TotalBadges, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT ps.PostType, ps.TotalPosts, ps.AverageScore, ps.TotalViews, us.DisplayName AS TopUser,
//        us.TotalBadges, us.TotalUpVotes, us.TotalDownVotes
// FROM PostStatistics ps JOIN UserStatistics us ON us.TotalUpVotes = (SELECT MAX(TotalUpVotes) FROM UserStatistics)
// ORDER BY ps.TotalPosts DESC;
fn q14396(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)))
        .group_by(ptype_name(db))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, v)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select(up_votes.and(down_votes).and(badges_of(db).opt()))
        .fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let most = (&us).fold_flat(i64::MIN, |m, a| m.max(a[1]));
    let mut out = Vec::new();
    (&ps).cross((&us).filt(|a| a[1] == most)).drive(|(t, name), (a, b)| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::S(name), V::I(b[0]), V::I(b[1]), V::I(b[2])]))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//            SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, AVG(p.ViewCount) AS AvgViews,
//            AVG(p.AnswerCount) AS AvgAnswers
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserActivity AS (
//     SELECT u.DisplayName, COUNT(CASE WHEN p.ViewCount > 0 THEN 1 END) AS PostsViewed,
//            SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.DisplayName, u.Id)
// SELECT pc.PostType, pc.TotalPosts, pc.PositivePosts, pc.NegativePosts, pc.AvgViews, pc.AvgAnswers,
//        ua.DisplayName, ua.PostsViewed, ua.TotalBounty
// FROM PostCounts pc JOIN UserActivity ua ON ua.PostsViewed > 0
// ORDER BY pc.TotalPosts DESC, ua.TotalBounty DESC;
fn q10830(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let neg = db.post.group_by(ptype_name(db)).select(&db.post.score).fold(0i64, |n, s| n + (s < 0) as i64);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold((0i64, 0i64), |(n, b), p| match p {
            Some((v, x)) => (n + (v.unwrap_or(0) > 0) as i64, b + x.flatten().unwrap_or(0)),
            None => (n, b),
        });
    let mut out = Vec::new();
    (&ps).and(&neg).cross((&ua).filt(|(n, _)| n > 0)).drive(|(t, u), ((a, ng), (n, b))| {
        out.push(row(vec![V::S(t), V::I(a[0]), V::I(a[9]), V::I(ng), avg(a[3], a[2]), avg(a[5], a[4]), user_col(db, u, "name"), V::I(n), V::I(b)]))
    });
    rows(out)
}

// WITH TagStats AS (
//     SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(COALESCE(p.Score, 0)) AS AverageScore
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// UserReputation AS (
//     SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadgeClass, AVG(u.Reputation) AS AverageReputation
//     FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id, u.DisplayName)
// SELECT ts.TagName, ts.PostCount, ts.QuestionCount, ts.AnswerCount, ts.AverageScore, ur.DisplayName AS TopUser,
//        ur.TotalBadgeClass, ur.AverageReputation
// FROM TagStats ts JOIN UserReputation ur ON ur.AverageReputation = (SELECT MAX(AverageReputation) FROM UserReputation)
// WHERE ts.PostCount > 0 ORDER BY ts.PostCount DESC, ts.AverageScore DESC;
fn q25602(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select((&db.post.post_type_id).and(&db.post.score)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + 1, a[4] + s],
            None => [a[0], a[1], a[2], a[3] + 1, a[4]],
        });
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (r, c)| [a[0] + 1, a[1] + r, a[2] + c.is_some() as i64, a[3] + c.unwrap_or(0)]);
    let most = (&ur).fold_flat(f64::MIN, |m, a| m.max(a[1] as f64 / a[0] as f64));
    let mut out = Vec::new();
    (&ts_).filt(|a| a[0] > 0).cross((&ur).filt(|a| a[1] as f64 / a[0] as f64 == most)).drive(|(name, u), (a, b)| {
        out.push(row(vec![V::S(name), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), user_col(db, u, "name"), nullable(b[3], b[2]), avg(b[1], b[0])]))
    });
    rows(out)
}

// WITH PostCounts AS (SELECT PostTypeId, COUNT(*) AS TotalPosts, COUNT(DISTINCT OwnerUserId) AS UniqueUsers
//                     FROM Posts GROUP BY PostTypeId),
// UserStats AS (
//     SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation),
// TopTags AS (SELECT T.TagName, T.Count FROM Tags T ORDER BY T.Count DESC LIMIT 10)
// SELECT PC.PostTypeId, PC.TotalPosts, PC.UniqueUsers, US.UserId, US.Reputation, US.TotalPosts AS UserTotalPosts,
//        US.TotalAnswers, US.TotalQuestions, TT.TagName, TT.Count AS TagCount
// FROM PostCounts PC JOIN UserStats US ON US.TotalPosts > 0 JOIN TopTags TT ON TT.Count > 0
// ORDER BY PC.TotalPosts DESC, US.Reputation DESC;
fn q14230(db: &'static So) -> String {
    let pc = db.post.group_by(&db.post.post_type_id).fold(0i64, |n, _| n + 1);
    let uu = db.post.group_by(&db.post.post_type_id).select(&db.post.owner_user_id).count_distinct();
    let ups = user_posts(db);
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let tt = rel(top_n(drain(&db.tag.count), |&(_, c)| Reverse(c), 10));
    let mut out = Vec::new();
    (&pc).and((&uu).opt()).cross((&dp).filt(|n| n > 0).and(&ups)).cross((&tt).filt(|(_, c)| c > 0)).drive(|((t, u), _), (((n, o), (d, a)), (g, c))| {
        let mut f = vec![V::I(t), V::I(n), V::I(o.unwrap_or(0))];
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(d), V::I(a[3]), V::I(a[2]), V::S(db.tag.tag_name.get(g).unwrap()), V::I(c)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS QuestionsWithScore,
//            AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount, AVG(COALESCE(p.Score, 0)) AS AvgScore,
//            MAX(p.CreationDate) AS LatestPost
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.Reputation, COUNT(b.Id) AS BadgeCount, AVG(COALESCE(u.Views, 0)) AS AvgViews,
//            AVG(COALESCE(u.UpVotes, 0)) AS AvgUpVotes, AVG(COALESCE(u.DownVotes, 0)) AS AvgDownVotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Reputation)
// SELECT ps.PostType, ps.TotalPosts, ps.QuestionsWithScore, ps.AvgViewCount, ps.AvgScore, us.Reputation,
//        us.BadgeCount, us.AvgViews, us.AvgUpVotes, us.AvgDownVotes
// FROM PostStats ps JOIN UserStats us ON us.Reputation > 1000 ORDER BY ps.TotalPosts DESC, us.Reputation DESC;
fn q14873(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let User { views, up_votes, down_votes, .. } = &db.user;
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(&db.user.reputation)
        .select(views.and(up_votes).and(down_votes).and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (((w, u), d), b)| [a[0] + 1, a[1] + b.is_some() as i64, a[2] + w, a[3] + u, a[4] + d]);
    let mut out = Vec::new();
    (&ps).cross(&us).drive(|(t, r), (a, b)| {
        out.push(row(vec![V::S(t), V::I(a[0]), V::I(a[9]), avg(a[3], a[0]), avg(a[1], a[0]), V::I(r), V::I(b[1]), avg(b[2], b[0]), avg(b[3], b[0]), avg(b[4], b[0])]))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT PT.Name AS PostType, COUNT(P.Id) AS TotalPosts, SUM(P.ViewCount) AS TotalViews,
//            SUM(COALESCE(P.Score, 0)) AS TotalScore, AVG(COALESCE(P.Score, 0)) AS AverageScore,
//            COUNT(DISTINCT P.OwnerUserId) AS TotalUsers
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name),
// UserStats AS (
//     SELECT U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(U.Reputation) AS TotalReputation,
//            AVG(U.Reputation) AS AverageReputation
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.DisplayName)
// SELECT PS.PostType, PS.TotalPosts, PS.TotalViews, PS.TotalScore, PS.AverageScore, PS.TotalUsers,
//        US.DisplayName, US.TotalBadges, US.TotalReputation, US.AverageReputation
// FROM PostStats PS LEFT JOIN UserStats US ON US.TotalReputation = (SELECT MAX(TotalReputation) FROM UserStats)
// ORDER BY PS.TotalPosts DESC;
fn q12528(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let uu = db.post.group_by(ptype_name(db)).select(&db.post.owner_user_id).count_distinct();
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select((&db.user.reputation).and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (r, b)| [a[0] + 1, a[1] + b.is_some() as i64, a[2] + r]);
    let most = (&us).fold_flat(i64::MIN, |m, a| m.max(a[2]));
    let top = (&us).filt(|a| a[2] == most);
    let top: HashIdx<(), (Str, [i64; 3])> = (&top).map(|_| ()).inv().select(Same::<Str>::new().and(&top)).collect();
    let mut out = Vec::new();
    (&ps).and((&uu).opt()).and(Same::<Str>::new().map(|_| ()).select(&top).opt()).drive(|t, ((a, o), x)| {
        let mut f = vec![V::S(t), V::I(a[0]), nullable(a[3], a[2]), V::I(a[1]), avg(a[1], a[0]), V::I(o.unwrap_or(0))];
        f.extend(match x {
            Some((name, b)) => [V::S(name), V::I(b[1]), V::I(b[2]), avg(b[2], b[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}


// WITH PostStats AS (
//     SELECT p.PostTypeId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
//            COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//            COUNT(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 END) AS TagWikiCount,
//            SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViewCount
//     FROM Posts p GROUP BY p.PostTypeId),
// UserStats AS (
//     SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount,
//            SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.Reputation)
// SELECT pts.PostTypeId, pts.QuestionCount, pts.AnswerCount, pts.TagWikiCount, pts.TotalScore, pts.TotalViewCount,
//        us.UserId, us.Reputation, us.BadgeCount, us.VoteCount
// FROM PostStats pts JOIN UserStats us ON us.VoteCount > 0 ORDER BY pts.TotalScore DESC, us.Reputation DESC;
fn q10945(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = db
        .post
        .group_by(post_type_id)
        .select(post_type_id.and(score).and(view_count.opt()))
        .fold([0i64; 6], |a, ((t, s), v)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 4 | 5) as i64, a[3] + s, a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0)]);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.creation_date).opt()))
        .fold((0i64, 0i64), |(b, v), (x, y)| (b + x.is_some() as i64, v + y.is_some() as i64));
    let mut out = Vec::new();
    (&ps).cross((&us).filt(|(_, v)| v > 0)).drive(|(t, u), (a, (b, v))| {
        let mut f = vec![V::I(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4])];
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(b), V::I(v)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT DATE_TRUNC('month', CreationDate) AS PostMonth, COUNT(*) AS TotalPosts, SUM(ViewCount) AS TotalViews,
//            SUM(AnswerCount) AS TotalAnswers, SUM(CommentCount) AS TotalComments
//     FROM Posts WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY PostMonth),
// UserStats AS (
//     SELECT DATE_TRUNC('month', CreationDate) AS UserMonth, COUNT(*) AS TotalUsers, SUM(Reputation) AS TotalReputation,
//            SUM(UpVotes) AS TotalUpVotes, SUM(DownVotes) AS TotalDownVotes
//     FROM Users WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY UserMonth)
// SELECT ps.PostMonth, ps.TotalPosts, ps.TotalViews, ps.TotalAnswers, ps.TotalComments, us.TotalUsers,
//        us.TotalReputation, us.TotalUpVotes, us.TotalDownVotes
// FROM PostStats ps FULL OUTER JOIN UserStats us ON ps.PostMonth = us.UserMonth
// ORDER BY COALESCE(ps.PostMonth, us.UserMonth);
fn q11761(db: &'static So) -> String {
    let since = ts(2023, 10, 1, 12, 34, 56);
    let Post { creation_date, view_count, answer_count, comment_count, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(since))
        .group_by(creation_date.map(trunc_month))
        .select(view_count.opt().and(answer_count.opt()).and(comment_count))
        .fold([0i64; 6], |a, ((v, an), c)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + an.is_some() as i64, a[4] + an.unwrap_or(0), a[5] + c]);
    let User { creation_date: ucd, reputation, up_votes, down_votes, .. } = &db.user;
    let us = db
        .user
        .with(ucd.ge(since))
        .group_by(ucd.map(trunc_month))
        .select(reputation.and(up_votes).and(down_votes))
        .fold([0i64; 4], |a, ((r, u), d)| [a[0] + 1, a[1] + r, a[2] + u, a[3] + d]);
    let pf = |a: [i64; 6]| vec![V::I(a[0]), nullable(a[2], a[1]), nullable(a[4], a[3]), V::I(a[5])];
    let uf = |b: [i64; 4]| vec![V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])];
    let mut out = Vec::new();
    (&ps).and((&us).opt()).drive(|m, (a, b)| {
        let mut f = vec![V::T(m)];
        f.extend(pf(a));
        f.extend(b.map_or_else(|| (0..4).map(|_| V::Null).collect(), uf));
        out.push(row(f))
    });
    (&us).minus(&ps).drive(|_, b| {
        let mut f: Vec<V> = (0..5).map(|_| V::Null).collect();
        f.extend(uf(b));
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//            SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostTypeStats AS (
//     SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(p.Id) AS CountPosts, AVG(p.Score) AS AvgScore,
//            SUM(p.ViewCount) AS TotalViews
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Id, pt.Name)
// SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalScore,
//        pts.PostTypeId, pts.PostTypeName, pts.CountPosts, pts.AvgScore, pts.TotalViews
// FROM UserPostStats ups JOIN PostTypeStats pts ON ups.TotalPosts > 0;
fn q14906(db: &'static So) -> String {
    let ups = user_posts(db);
    let pts = type_left_posts(db);
    let mut out = Vec::new();
    (&ups).filt(|a| a[1] > 0).cross(&pts).drive(|(u, t), (a, b)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(db.post_type.origid.get(t).unwrap()), tname(db, t), V::I(b[0]), avg(b[1], b[0]), nullable(b[3], b[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//            SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//            SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostTypeCounts AS (
//     SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(p.Id) AS PostCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Id, pt.Name)
// SELECT u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalViews, u.TotalScore,
//        ptc.PostTypeName, ptc.PostCount
// FROM UserPostStats u LEFT JOIN PostTypeCounts ptc ON u.TotalPosts > 0
// ORDER BY u.TotalScore DESC, u.TotalPosts DESC;
fn q14958(db: &'static So) -> String {
    let ups = user_posts(db);
    let ptc = db.post.group_by(&db.post.post_type).fold(0i64, |n, _| n + 1);
    let ptc: HashIdx<(), (Id<PostType>, i64)> = (&ptc).map(|_| ()).inv().select(Ident::<PostType>::new().and(&ptc)).collect();
    let mut out = Vec::new();
    (&ups).and((&ups).filt(|a| a[1] > 0).map(|_| ()).select(&ptc).opt()).drive(|u, (a, x)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4])];
        f.extend(match x {
            Some((t, n)) => [tname(db, t), V::I(n)],
            None => [V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//            SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS WikiPosts,
//            SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AverageViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopPosts AS (
//     SELECT P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id ORDER BY P.Score DESC, P.ViewCount DESC LIMIT 10)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.Questions, U.Answers, U.WikiPosts, U.TotalScore,
//        U.AverageViews, T.Title AS TopPostTitle, T.Score AS TopPostScore
// FROM UserStats U LEFT JOIN TopPosts T ON U.TotalPosts > 0 ORDER BY U.TotalScore DESC;
fn q11471(db: &'static So) -> String {
    let ups = user_posts(db);
    let wiki = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold(0i64, |n, t| n + matches!(t, Some(4 | 5)) as i64);
    let tp = rel(top_n(drain(owned(db).select((&db.post.score).and((&db.post.view_count).opt()))), |&(_, (s, v))| (Reverse(s), v.is_none(), Reverse(v)), 10));
    let tp: HashIdx<(), (Id<Post>, (i64, Option<i64>))> = (&tp).map(|_| ()).inv().select(&tp).collect();
    let mut out = Vec::new();
    (&ups).and(&wiki).and((&ups).filt(|a| a[1] > 0).map(|_| ()).select(&tp).opt()).drive(|u, ((a, w), x)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(w), nullable(a[4], a[1]), avg(a[6], a[5])]);
        f.extend(match x {
            Some((p, (s, _))) => [title(db, p), V::I(s)],
            None => [V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore,
//            AVG(p.ViewCount) AS AverageViewCount,
//            SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS UserOwnedCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.Reputation, COUNT(b.Id) AS TotalBadges, COUNT(c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Comments c ON u.Id = c.UserId
//     GROUP BY u.Reputation)
// SELECT ps.PostType, ps.TotalPosts, ps.AverageScore, ps.AverageViewCount, ps.UserOwnedCount,
//        COUNT(us.Reputation) AS UserCount, AVG(us.Reputation) AS AverageReputation,
//        SUM(us.TotalBadges) AS TotalBadges, SUM(us.TotalComments) AS TotalComments
// FROM PostStats ps JOIN UserStats us ON 1 = 1
// GROUP BY ps.PostType, ps.TotalPosts, ps.AverageScore, ps.AverageViewCount, ps.UserOwnedCount
// ORDER BY ps.TotalPosts DESC;
fn q13843(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let owned_n = db.post.group_by(ptype_name(db)).select((&db.post.owner_user_id).opt()).fold(0i64, |n, o| n + o.is_some() as i64);
    let us = db
        .user
        .group_by(&db.user.reputation)
        .select(badges_of(db).opt().and(comments_by(db).opt()))
        .fold((0i64, 0i64), |(b, c), (x, y)| (b + x.is_some() as i64, c + y.is_some() as i64));
    let t = whole(&us).select(Same::new().and(&us)).fold([0i64; 4], |a, (r, (b, c))| [a[0] + 1, a[1] + r, a[2] + b, a[3] + c]);
    let mut out = Vec::new();
    (&ps).and(&owned_n).cross(&t).drive(|(name, _), ((a, o), t)| {
        out.push(row(vec![V::S(name), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(o), V::I(t[0]), avg(t[1], t[0]), V::I(t[2]), V::I(t[3])]))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount,
//            SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//            SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, AVG(V.BountyAmount) AS AverageBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 GROUP BY U.Id, U.DisplayName),
// PostTypesStats AS (
//     SELECT PT.Name AS PostTypeName, COUNT(P.Id) AS PostCount, AVG(P.Score) AS AverageScore,
//            AVG(P.ViewCount) AS AverageViewCount
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name)
// SELECT U.UserId, U.DisplayName, U.PostCount, U.PositivePosts, U.NegativePosts, U.AverageBounty,
//        PTS.PostTypeName, PTS.PostCount AS TotalPostsByType, PTS.AverageScore, PTS.AverageViewCount
// FROM UserStats U JOIN PostTypesStats PTS ON PTS.PostCount > 0 ORDER BY U.PostCount DESC, PTS.PostTypeName;
fn q14905(db: &'static So) -> String {
    let bounties: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(8)).select(&db.vote.post).inv().collect();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and((&bounties).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, v)) => {
                let v = v.flatten();
                [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]
            }
            None => a,
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let ps = stats_by_type(db);
    let mut out = Vec::new();
    (&us).and((&dp).opt()).cross((&ps).filt(|a| a[0] > 0)).drive(|(u, t), ((a, n), b)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::S(t), V::I(b[0]), avg(b[1], b[0]), avg(b[3], b[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore,
//            SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//            SUM(CASE WHEN p.CommentCount IS NOT NULL THEN p.CommentCount ELSE 0 END) AS TotalComments,
//            SUM(CASE WHEN p.AnswerCount IS NOT NULL THEN p.AnswerCount ELSE 0 END) AS TotalAnswers
//     FROM Posts p INNER JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.DisplayName, COUNT(b.Id) AS TotalBadges, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT ps.PostType, ps.TotalPosts, ps.AverageScore, ps.TotalViews, ps.TotalComments, ps.TotalAnswers,
//        us.DisplayName, us.TotalBadges, us.TotalUpVotes, us.TotalDownVotes
// FROM PostStats ps LEFT JOIN UserStats us ON us.TotalBadges > 0 ORDER BY ps.TotalPosts DESC, us.TotalBadges DESC;
fn q13469(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = db
        .user
        .group_by(&db.user.display_name)
        .select(up_votes.and(down_votes).and(badges_of(db).opt()))
        .fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let us = (&us).filt(|b| b[0] > 0);
    let us: HashIdx<(), (Str, [i64; 3])> = (&us).map(|_| ()).inv().select(Same::<Str>::new().and(&us)).collect();
    let mut out = Vec::new();
    (&ps).and(Same::<Str>::new().map(|_| ()).select(&us).opt()).drive(|t, (a, x)| {
        let mut f = vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), V::I(a[3]), V::I(a[6]), V::I(a[5])];
        f.extend(match x {
            Some((name, b)) => [V::S(name), V::I(b[0]), V::I(b[1]), V::I(b[2])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}


// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT a.Id) AS TotalAnswers,
//            SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//            SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id
//     LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name)
// SELECT us.DisplayName, us.TotalPosts, us.TotalAnswers, us.TotalUpvotes, us.TotalDownvotes, ps.PostType,
//        ps.TotalPosts AS PostTypeTotal, ps.TotalScore, ps.AvgViewCount
// FROM UserStats us JOIN PostStats ps ON ps.TotalPosts > 0 ORDER BY us.TotalPosts DESC, ps.TotalPosts DESC;
fn q14422(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.accepted_answer).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold((0i64, 0i64), |(u, d), p| match p {
            Some((_, t)) => (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64),
            None => (u, d),
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let da = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.accepted_answer)).count_distinct();
    let ps = stats_by_type(db);
    let mut out = Vec::new();
    (&us).and((&dp).opt()).and((&da).opt()).cross((&ps).filt(|a| a[0] > 0)).drive(|(u, t), ((((up, dn), n), a), b)| {
        out.push(row(vec![user_col(db, u, "name"), V::I(n.unwrap_or(0)), V::I(a.unwrap_or(0)), V::I(up), V::I(dn), V::S(t), V::I(b[0]), V::I(b[1]), avg(b[3], b[2])]))
    });
    rows(out)
}

// WITH UserPostCounts AS (
//     SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// RecentActiveUsers AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.LastAccessDate, up.PostCount, up.QuestionCount, up.AnswerCount
//     FROM Users u JOIN UserPostCounts up ON u.Id = up.UserId
//     WHERE u.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     ORDER BY u.LastAccessDate DESC LIMIT 100),
// PopularTags AS (SELECT t.TagName, t.Count FROM Tags t ORDER BY t.Count DESC LIMIT 10)
// SELECT ru.UserId, ru.DisplayName, ru.Reputation, ru.PostCount, ru.QuestionCount, ru.AnswerCount,
//        pt.TagName, pt.Count AS TagUsage
// FROM RecentActiveUsers ru CROSS JOIN PopularTags pt;
fn q14802(db: &'static So) -> String {
    let ups = user_posts(db);
    let recent = db.user.with((&db.user.last_access_date).gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select((&db.user.last_access_date).and(&ups));
    let ru = rel(top_n(drain(recent), |&(_, (d, _))| Reverse(d), 100));
    let pt = rel(top_n(drain(&db.tag.count), |&(_, c)| Reverse(c), 10));
    let mut out = Vec::new();
    (&ru).cross(&pt).drive(|_, ((u, (_, a)), (t, c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::S(db.tag.tag_name.get(t).unwrap()), V::I(c)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT p.PostTypeId, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoredPosts,
//            SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScoredPosts, AVG(p.ViewCount) AS AverageViews,
//            AVG(p.AnswerCount) AS AverageAnswers, MAX(p.CreationDate) AS LatestPostDate
//     FROM Posts p GROUP BY p.PostTypeId),
// UserStats AS (
//     SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS TotalPostsByUser, SUM(p.Score) AS TotalScoreByUser,
//            AVG(u.Reputation) AS AverageReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id)
// SELECT pt.Name AS PostTypeName, ps.TotalPosts, ps.PositiveScoredPosts, ps.NegativeScoredPosts, ps.AverageViews,
//        ps.AverageAnswers, ps.LatestPostDate, us.UserId, us.TotalPostsByUser, us.TotalScoreByUser, us.AverageReputation
// FROM PostStats ps JOIN PostTypes pt ON ps.PostTypeId = pt.Id LEFT JOIN UserStats us ON us.TotalPostsByUser > 0
// ORDER BY ps.TotalPosts DESC;
fn q12401(db: &'static So) -> String {
    let ps = stats_by_type_id(db);
    let neg = db.post.group_by(&db.post.post_type).select(&db.post.score).fold(0i64, |n, s| n + (s < 0) as i64);
    let ups = user_posts(db);
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let us = (&dp).filt(|n| n > 0).and(&ups);
    let us: HashIdx<(), (Id<User>, (i64, [i64; 10]))> = (&us).map(|_| ()).inv().select(Ident::<User>::new().and(&us)).collect();
    let mut out = Vec::new();
    (&ps).and(&neg).and(Ident::<PostType>::new().map(|_| ()).select(&us).opt()).drive(|t, ((a, ng), x)| {
        let mut f = vec![tname(db, t), V::I(a[0]), V::I(a[9]), V::I(ng), avg(a[3], a[2]), avg(a[5], a[4]), tmax(a[10])];
        f.extend(match x {
            Some((u, (n, b))) => [user_col(db, u, "uid"), V::I(n), V::I(b[4]), avg(b[9], b[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.Reputation),
// TagStats AS (
//     SELECT T.Id AS TagId, T.TagName, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.Id, T.TagName)
// SELECT U.UserId, U.Reputation, U.PostCount, U.QuestionCount, U.AnswerCount, U.UpvoteCount, U.DownvoteCount,
//        T.TagId, T.TagName, T.PostCount AS TagPostCount, T.TotalViews, T.TotalScore
// FROM UserStats U JOIN TagStats T ON U.PostCount > 0
// ORDER BY U.Reputation DESC, T.TotalScore DESC LIMIT 100;
//
// The cross product is 80M rows. ORDER BY (reputation, tag score) is
// lexicographic, so only the users whose reputation rank can reach the first
// 100 rows are crossed with the tags (`cross_top`).
fn q13849(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (t, v)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]);
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select((&db.post.view_count).opt().and(&db.post.score)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((v, s)) => [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s],
            None => a,
        });
    let tags = drain(&ts_);
    let users = drain((&dp).filt(|n| n > 0).and(&us));
    let picked: Vec<_> = cross_top(users, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), tags, |&(_, a)| (a[0] == 0, Reverse(a[3])), 100)
        .into_iter()
        .map(|(u, (t, a))| (u, t, a))
        .collect();
    rows(picked.iter().map(|&((u, (n, a)), t, b)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(db.tag.origid.get(t).unwrap()), V::S(db.tag.tag_name.get(t).unwrap()), V::I(b[0]), nullable(b[2], b[1]), nullable(b[3], b[0])]);
        row(f)
    }))
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.ViewCount) AS TotalViewCount,
//            AVG(p.Score) AS AveragePostScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
//            SUM(p.ViewCount) AS TotalViewCount, AVG(p.Score) AS AverageScore
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name)
// SELECT u.UserId, u.DisplayName, u.PostCount, u.QuestionCount, u.AnswerCount, u.TotalViewCount, u.AveragePostScore,
//        p.PostType, p.TotalPosts, p.UniqueUsers, p.TotalViewCount AS PostTypeTotalViews, p.AverageScore AS PostTypeAverageScore
// FROM UserPostStats u CROSS JOIN PostStatistics p ORDER BY u.TotalViewCount DESC;
fn q12465(db: &'static So) -> String {
    let ups = user_posts(db);
    let ps = stats_by_type(db);
    let uu = db.post.group_by(ptype_name(db)).select(&db.post.owner_user_id).count_distinct();
    let mut out = Vec::new();
    (&ups).cross((&ps).and((&uu).opt())).drive(|(u, t), (a, (b, o))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), avg(a[4], a[1]), V::S(t), V::I(b[0]), V::I(o.unwrap_or(0)), nullable(b[3], b[2]), avg(b[1], b[0])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.Score) AS TotalScore,
//            AVG(P.ViewCount) AS AverageViewCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// PostTypeStats AS (
//     SELECT PT.Id AS PostTypeId, PT.Name AS PostTypeName, COUNT(P.Id) AS PostCount, AVG(P.Score) AS AverageScore,
//            SUM(P.ViewCount) AS TotalViewCount
//     FROM PostTypes PT LEFT JOIN Posts P ON PT.Id = P.PostTypeId GROUP BY PT.Id, PT.Name)
// SELECT UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.TotalQuestions, UPS.TotalAnswers, UPS.TotalScore,
//        UPS.AverageViewCount, PTS.PostTypeId, PTS.PostTypeName, PTS.PostCount, PTS.AverageScore, PTS.TotalViewCount
// FROM UserPostStats UPS JOIN PostTypeStats PTS ON UPS.UserId IS NOT NULL
// ORDER BY UPS.TotalPosts DESC, PTS.PostCount DESC;
fn q13477(db: &'static So) -> String {
    let ups = user_posts(db);
    let pts = type_left_posts(db);
    let mut out = Vec::new();
    (&ups).cross(&pts).drive(|(u, t), (a, b)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), avg(a[6], a[5]), V::I(db.post_type.origid.get(t).unwrap()), tname(db, t), V::I(b[0]), avg(b[1], b[0]), nullable(b[3], b[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//            SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserVotes AS (
//     SELECT u.DisplayName AS UserDisplayName, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u JOIN Votes v ON u.Id = v.UserId GROUP BY u.DisplayName),
// BadgeCounts AS (
//     SELECT u.DisplayName AS UserDisplayName, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT pc.PostType, pc.PostCount, pc.TotalScore, pc.TotalViews, pc.TotalAnswers, uv.UserDisplayName, uv.VoteCount,
//        uv.UpVotes, uv.DownVotes, COALESCE(bc.BadgeCount, 0) AS BadgeCount
// FROM PostCounts pc LEFT JOIN UserVotes uv ON 1=1 LEFT JOIN BadgeCounts bc ON uv.UserDisplayName = bc.UserDisplayName
// ORDER BY pc.PostType;
fn q12026(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let uv = db
        .user
        .group_by(&db.user.display_name)
        .select(votes_by(db).select(&db.vote.vote_type_id))
        .fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let bc = db.user.group_by(&db.user.display_name).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let uv = (&uv).and((&bc).opt());
    let uv: HashIdx<(), (Str, ([i64; 3], Option<i64>))> = (&uv).map(|_| ()).inv().select(Same::<Str>::new().and(&uv)).collect();
    let mut out = Vec::new();
    (&ps).and(Same::<Str>::new().map(|_| ()).select(&uv).opt()).drive(|t, (a, x)| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[3]), V::I(a[5])];
        f.extend(match x {
            Some((name, (b, c))) => [V::S(name), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(c.unwrap_or(0))],
            None => [V::Null, V::Null, V::Null, V::Null, V::I(0)],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//            SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
//            SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// PostTypeStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViewCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name)
// SELECT u.UserId, u.TotalPosts, u.Questions, u.Answers, u.TotalViews, u.UpVotes, u.DownVotes, p.PostType, p.PostCount,
//        p.AverageScore, p.AverageViewCount
// FROM UserPostStats u CROSS JOIN PostTypeStats p ORDER BY u.TotalPosts DESC, p.PostCount DESC;
fn q11664(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.view_count).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 6], |a, ((t, w), v)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64]);
    let ps = stats_by_type(db);
    let mut out = Vec::new();
    db.user.select(Ident::<User>::new().and((&us).opt())).cross(&ps).drive(|(_, t), ((u, a), b)| {
        let a = a.unwrap_or([0; 6]);
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(a.iter().map(|&x| V::I(x)));
        f.extend([V::S(t), V::I(b[0]), avg(b[1], b[0]), avg(b[3], b[2])]);
        out.push(row(f))
    });
    rows(out)
}


// WITH PostCounts AS (
//     SELECT PostTypeId, COUNT(*) AS TotalPosts, SUM(CASE WHEN AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//            SUM(ViewCount) AS TotalViews, AVG(Score) AS AvgScore
//     FROM Posts GROUP BY PostTypeId),
// UserMetrics AS (
//     SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS TotalBadges, SUM(v.BountyAmount) AS TotalBounty
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// PostHistoryTypesCount AS (
//     SELECT pht.Name AS PostHistoryType, COUNT(ph.Id) AS HistoryCount
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY pht.Name)
// SELECT pc.PostTypeId, pc.TotalPosts, pc.AcceptedAnswers, pc.TotalViews, pc.AvgScore, um.TotalBadges, um.TotalBounty,
//        phc.PostHistoryType, phc.HistoryCount
// FROM PostCounts pc JOIN UserMetrics um ON um.UserId = (SELECT MIN(Id) FROM Users) CROSS JOIN PostHistoryTypesCount phc
// ORDER BY pc.PostTypeId;
fn q12781(db: &'static So) -> String {
    let Post { accepted_answer_id, view_count, score, .. } = &db.post;
    let pc = db
        .post
        .group_by(&db.post.post_type_id)
        .select(accepted_answer_id.opt().and(view_count.opt()).and(score))
        .fold([0i64; 5], |a, ((x, v), s)| [a[0] + 1, a[1] + x.is_some() as i64, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4] + s]);
    let min_id = (&db.user.origid).fold_flat(i64::MAX, |m, x| m.min(x));
    let um = db
        .user
        .with((&db.user.origid).eq(min_id))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (b, v)| {
            let v = v.flatten();
            [a[0] + b.is_some() as i64, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0)]
        });
    let phc = db.post_history.group_by(htype_name(db)).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&pc).cross(&um).cross(&phc).drive(|((t, _), h), ((a, b), n)| {
        out.push(row(vec![V::I(t), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[4], a[0]), V::I(b[0]), nullable(b[2], b[1]), V::S(h), V::I(n)]))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT PostTypeId, COUNT(*) AS TotalPosts, SUM(CASE WHEN AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Posts GROUP BY PostTypeId),
// UserReputation AS (
//     SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// MostActiveUsers AS (SELECT UserId, COUNT(*) AS CommentCount FROM Comments GROUP BY UserId ORDER BY CommentCount DESC LIMIT 10)
// SELECT PC.PostTypeId, PC.TotalPosts, PC.AcceptedAnswers, MAX(UR.Reputation) AS HighestReputation,
//        SUM(UR.BadgeCount) AS TotalBadges, MA.CommentCount AS MostComments
// FROM PostCounts PC JOIN Users U ON (U.Id IN (SELECT UserId FROM MostActiveUsers))
// JOIN UserReputation UR ON (U.Id = UR.UserId)
// JOIN (SELECT UserId, SUM(CommentCount) AS CommentCount FROM MostActiveUsers GROUP BY UserId) MA ON (MA.UserId = U.Id)
// GROUP BY PC.PostTypeId, PC.TotalPosts, PC.AcceptedAnswers, MA.CommentCount ORDER BY PC.TotalPosts DESC;
//
// The comments with no UserId form their own group in MostActiveUsers, and
// then match no user.
fn q12635(db: &'static So) -> String {
    let pc = db
        .post
        .group_by(&db.post.post_type_id)
        .select((&db.post.accepted_answer_id).opt())
        .fold((0i64, 0i64), |(n, a), x| (n + 1, a + x.is_some() as i64));
    let per = db.comment.group_by((&db.comment.user_id).opt()).fold(0i64, |n, _| n + 1);
    let top = rel(top_n(drain(&per), |&(_, n)| Reverse(n), 10));
    let ma = (&top)
        .flat_map(|(u, n)| u.map(|u| (u, n)))
        .group_by(Same::<(i64, i64)>::new().map(|(u, _)| u))
        .select(Same::<(i64, i64)>::new().map(|(_, n)| n))
        .fold(0i64, |s, n| s + n);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ur = db
        .user
        .with((&db.user.origid).select(&ma))
        .group_by((&db.user.origid).select(&ma))
        .select((&db.user.reputation).and(&bc))
        .fold((i64::MIN, 0i64), |(m, s), (r, b)| (m.max(r), s + b));
    let mut out = Vec::new();
    (&pc).cross(&ur).drive(|(t, c), ((n, a), (r, b))| out.push(row(vec![V::I(t), V::I(n), V::I(a), V::I(r), V::I(b), V::I(c)])));
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(p.Score), 0) AS TotalScore,
//            COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COALESCE(SUM(p.CommentCount), 0) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostTypeSummary AS (
//     SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AvgScore, AVG(p.ViewCount) AS AvgViews
//     FROM PostTypes pt LEFT JOIN Posts p ON pt.Id = p.PostTypeId GROUP BY pt.Id, pt.Name)
// SELECT u.UserId, u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalScore, u.TotalViews, u.TotalComments,
//        pt.PostTypeId, pt.PostTypeName, pt.PostCount, pt.AvgScore, pt.AvgViews
// FROM UserPostStats u JOIN PostTypeSummary pt ON 1=1 ORDER BY u.TotalPosts DESC, pt.PostCount DESC;
fn q10989(db: &'static So) -> String {
    let ups = user_posts(db);
    let uc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.comment_count)).fold(0i64, |n, c| n + c);
    let pts = type_left_posts(db);
    let mut out = Vec::new();
    (&ups).and((&uc).opt()).cross(&pts).drive(|(u, t), ((a, c), b)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6]), V::I(c.unwrap_or(0))]);
        f.extend([V::I(db.post_type.origid.get(t).unwrap()), tname(db, t), V::I(b[0]), avg(b[1], b[0]), avg(b[3], b[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount,
//            COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//            COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//            COALESCE(SUM(p.ViewCount), 0) AS TotalViews, COALESCE(SUM(p.Score), 0) AS TotalScore, AVG(u.Reputation) AS AverageReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.ViewCount) AS AverageViews,
//            SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name)
// SELECT u.DisplayName, u.PostCount, u.QuestionCount, u.AnswerCount, u.TotalViews, u.TotalScore, u.AverageReputation,
//        ps.PostType, ps.TotalPosts, ps.AverageViews, ps.PositiveScoreCount
// FROM UserPostActivity u JOIN PostStatistics ps ON u.QuestionCount > 0 OR u.AnswerCount > 0 ORDER BY u.TotalViews DESC;
fn q14894(db: &'static So) -> String {
    let ups = user_posts(db);
    let ps = stats_by_type(db);
    let mut out = Vec::new();
    (&ups).filt(|a| a[2] > 0 || a[3] > 0).cross(&ps).drive(|(u, t), (a, b)| {
        out.push(row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), avg(a[9], a[0]), V::S(t), V::I(b[0]), avg(b[3], b[2]), V::I(b[9])]))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews,
//            SUM(p.AnswerCount) AS TotalAnswers, SUM(p.CommentCount) AS TotalComments
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserEngagement AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBounty,
//            SUM(CASE WHEN v.UserId IS NOT NULL AND v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN v.UserId IS NOT NULL AND v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT pc.PostType, pc.PostCount, pc.TotalScore, pc.TotalViews, pc.TotalAnswers, pc.TotalComments, ue.UserId,
//        ue.DisplayName, ue.BadgeCount, ue.TotalBounty, ue.UpVotes, ue.DownVotes
// FROM PostCounts pc JOIN UserEngagement ue ON ue.UserId IS NOT NULL ORDER BY pc.PostCount DESC, ue.BadgeCount DESC;
fn q11299(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt().and(&db.vote.vote_type_id)).opt()))
        .fold([0i64; 5], |a, (b, v)| match v {
            Some((x, t)) => [a[0] + b.is_some() as i64, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0), a[3] + (t == 2) as i64, a[4] + (t == 3) as i64],
            None => [a[0] + b.is_some() as i64, a[1], a[2], a[3], a[4]],
        });
    let mut out = Vec::new();
    (&ps).cross(&ue).drive(|(t, u), (a, b)| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), nullable(a[5], a[4]), V::I(a[6])];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b[0]), nullable(b[2], b[1]), V::I(b[3]), V::I(b[4])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT p.PostTypeId, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//            SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts, AVG(p.ViewCount) AS AvgViewCount,
//            AVG(p.AnswerCount) AS AvgAnswerCount, AVG(p.CommentCount) AS AvgCommentCount
//     FROM Posts p GROUP BY p.PostTypeId),
// UserStats AS (
//     SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//            SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT p.PostTypeId, p.TotalPosts, p.PositiveScorePosts, p.NegativeScorePosts, p.AvgViewCount, p.AvgAnswerCount,
//        p.AvgCommentCount, u.UserId, u.PostCount, u.GoldBadges, u.SilverBadges, u.BronzeBadges
// FROM PostStats p LEFT JOIN UserStats u ON u.PostCount > 0 ORDER BY p.PostTypeId, u.PostCount DESC;
fn q12146(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, .. } = &db.post;
    let ps = db
        .post
        .group_by(&db.post.post_type_id)
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count))
        .fold([0i64; 8], |a, (((s, v), an), c)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + v.is_some() as i64, a[4] + v.unwrap_or(0), a[5] + an.is_some() as i64, a[6] + an.unwrap_or(0), a[7] + c]);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (_, c)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let us = (&dp).filt(|n| n > 0).and(&us);
    let us: HashIdx<(), (Id<User>, (i64, [i64; 3]))> = (&us).map(|_| ()).inv().select(Ident::<User>::new().and(&us)).collect();
    let mut out = Vec::new();
    (&ps).and(Same::<i64>::new().map(|_| ()).select(&us).opt()).drive(|t, (a, x)| {
        let mut f = vec![V::I(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), avg(a[6], a[5]), avg(a[7], a[0])];
        f.extend(match x {
            Some((u, (n, b))) => [user_col(db, u, "uid"), V::I(n), V::I(b[0]), V::I(b[1]), V::I(b[2])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH TagStatistics AS (
//     SELECT t.TagName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Comments c ON c.PostId = p.Id
//     GROUP BY t.TagName),
// UserReputation AS (
//     SELECT u.Id, u.DisplayName, SUM(b.Class) AS TotalBadgePoints, SUM(v.BountyAmount) AS TotalBountyEarned
//     FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id AND v.VoteTypeId IN (8, 9)
//     WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName)
// SELECT ts.TagName, ts.TotalPosts, ts.TotalQuestions, ts.TotalAnswers, ts.TotalComments, ur.DisplayName AS TopContributor,
//        ur.TotalBadgePoints, ur.TotalBountyEarned
// FROM TagStatistics ts JOIN UserReputation ur ON ur.TotalBadgePoints = (SELECT MAX(TotalBadgePoints) FROM UserReputation)
// ORDER BY ts.TotalPosts DESC LIMIT 10;
fn q27823(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select((&db.post.post_type_id).and(comments_of(db).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64],
            None => a,
        });
    let bounty: HashIdx<Id<User>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).is_in([8, 9])).select(&db.vote.user).inv().collect();
    let ur = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and((&bounty).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 4], |a, (c, v)| {
            let v = v.flatten();
            [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0), a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]
        });
    let most = (&ur).filt(|a| a[0] > 0).fold_flat(i64::MIN, |m, a| m.max(a[1]));
    let v = drain((&ts_).cross((&ur).filt(|a| a[0] > 0 && a[1] == most)));
    let v = top_n(v, |&(_, (a, _))| Reverse(a[0]), 10);
    rows(v.iter().map(|&((name, u), (a, b))| row(vec![V::S(name), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), user_col(db, u, "name"), nullable(b[1], b[0]), nullable(b[3], b[2])])))
}

// WITH PostStatistics AS (
//     SELECT P.PostTypeId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore,
//            SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews, AVG(P.ViewCount) AS AvgViewsPerPost,
//            AVG(P.Score) AS AvgScorePerPost, MAX(P.CreationDate) AS MostRecentPost, MIN(P.CreationDate) AS OldestPost
//     FROM Posts P GROUP BY P.PostTypeId),
// UserStatistics AS (
//     SELECT U.Id, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(U.UpVotes) AS TotalUpVotes, SUM(U.DownVotes) AS TotalDownVotes,
//            SUM(U.Views) AS TotalViews
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName)
// SELECT PST.PostTypeId, PST.PostCount, PST.TotalScore, PST.TotalViews, PST.AvgViewsPerPost, PST.AvgScorePerPost,
//        PST.MostRecentPost, PST.OldestPost, UST.Id AS UserId, UST.DisplayName, UST.BadgeCount, UST.TotalUpVotes,
//        UST.TotalDownVotes, UST.TotalViews
// FROM PostStatistics PST CROSS JOIN UserStatistics UST ORDER BY PST.PostTypeId, UST.DisplayName;
fn q10998(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let ps = db
        .post
        .group_by(&db.post.post_type_id)
        .select(score.and(view_count.opt()).and(creation_date))
        .fold([0, 0, 0, 0, i64::MIN, i64::MAX], |a, ((s, v), d)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4].max(d), a[5].min(d)]);
    let User { up_votes, down_votes, views, .. } = &db.user;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(views).and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (((u, d), w), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d, a[3] + w]);
    let mut out = Vec::new();
    (&ps).cross(&us).drive(|(t, u), (a, b)| {
        let mut f = vec![V::I(t), V::I(a[0]), V::I(a[1]), V::I(a[3]), avg(a[3], a[2]), avg(a[1], a[0]), tmax(a[4]), tmin(a[5])];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        out.push(row(f))
    });
    rows(out)
}


// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers, SUM(p.Score) AS TotalScore,
//            SUM(p.ViewCount) AS TotalViews, AVG(p.CommentCount) AS AvgCommentsPerPost, AVG(p.FavoriteCount) AS AvgFavoritesPerPost,
//            MAX(p.CreationDate) AS LatestPostDate
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass,
//            COUNT(DISTINCT v.Id) AS VoteCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.DisplayName, u.Reputation)
// SELECT ps.PostType, ps.TotalPosts, ps.UniqueUsers, ps.TotalScore, ps.TotalViews, ps.AvgCommentsPerPost, ps.AvgFavoritesPerPost,
//        ps.LatestPostDate, us.DisplayName, us.Reputation, us.BadgeCount, us.TotalBadgeClass, us.VoteCount
// FROM PostStats ps JOIN UserStats us ON us.BadgeCount > 0 ORDER BY ps.TotalPosts DESC, us.Reputation DESC;
fn q14895(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let uu = db.post.group_by(ptype_name(db)).select(&db.post.owner_user_id).count_distinct();
    let User { display_name, reputation, .. } = &db.user;
    let us = db
        .user
        .group_by(display_name.and(reputation))
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).opt()))
        .fold((0i64, 0i64), |(n, s), (c, _)| (n + c.is_some() as i64, s + c.unwrap_or(0)));
    let dv = db.user.group_by(display_name.and(reputation)).select(votes_by(db)).count_distinct();
    let mut out = Vec::new();
    (&ps).and((&uu).opt()).cross((&us).filt(|(n, _)| n > 0).and((&dv).opt())).drive(|(t, (name, rep)), ((a, o), ((n, s), v))| {
        out.push(row(vec![V::S(t), V::I(a[0]), V::I(o.unwrap_or(0)), V::I(a[1]), nullable(a[3], a[2]), avg(a[6], a[0]), avg(a[8], a[7]), tmax(a[10]), V::S(name), V::I(rep), V::I(n), V::I(s), V::I(v.unwrap_or(0))]))
    });
    rows(out)
}

fn substr(s: Str, start: i64, len: i64) -> Str {
    let cs: Vec<char> = s.chars().collect();
    let from = (start - 1).max(0) as usize;
    let to = ((start - 1 + len).max(0) as usize).min(cs.len());
    Box::leak(cs.get(from..to.max(from)).map_or(String::new(), |c| c.iter().collect()).into_boxed_str())
}

// WITH TagCounts AS (SELECT Tags, COUNT(*) AS PostCount FROM Posts WHERE PostTypeId = 1 GROUP BY Tags),
// TopTags AS (SELECT SUBSTRING(Tags, 3, LENGTH(Tags) - 4) AS FormattedTags, PostCount FROM TagCounts WHERE PostCount > 10),
// UserActivity AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionCount,
//            SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.PostTypeId = 1 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionCount, UpVotes - DownVotes AS VoteBalance FROM UserActivity
//              WHERE QuestionCount > 5 ORDER BY VoteBalance DESC LIMIT 10)
// SELECT TU.DisplayName, T.FormattedTags, T.PostCount, TU.QuestionCount, TU.VoteBalance
// FROM TopUsers TU CROSS JOIN TopTags T ORDER BY TU.VoteBalance DESC, T.PostCount DESC;
fn q29284(db: &'static So) -> String {
    let q = db.post.with((&db.post.post_type_id).eq(1));
    let tc = q.group_by((&db.post.tags_str).opt()).fold(0i64, |n, _| n + 1);
    let q = db.post.with((&db.post.post_type_id).eq(1));
    let ua = q
        .with(&db.post.owner_user)
        .group_by(&db.post.owner_user)
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold(0i64, |b, t| b + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let q = db.post.with((&db.post.post_type_id).eq(1));
    let dq = q.with(&db.post.owner_user).group_by(&db.post.owner_user).count_distinct();
    let tu = rel(top_n(drain((&dq).filt(|n| n > 5).and(&ua)), |&(_, (_, b))| Reverse(b), 10));
    let mut out = Vec::new();
    (&tu).cross((&tc).filt(|n| n > 10)).drive(|(_, tags), ((u, (n, b)), c)| {
        let f = tags.map(|t| substr(t, 3, t.chars().count() as i64 - 4));
        out.push(row(vec![user_col(db, u, "name"), ostr(f), V::I(c), V::I(n), V::I(b)]))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT P.OwnerUserId, PT.Name AS PostType, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//            SUM(CASE WHEN P.ViewCount > 100 THEN 1 ELSE 0 END) AS HighViewPosts, AVG(P.ViewCount) AS AvgViews, AVG(P.Score) AS AvgScore
//     FROM Posts P INNER JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY P.OwnerUserId, PT.Name),
// UserStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(B.Id) AS BadgeCount, COALESCE(SUM(P.TotalPosts), 0) AS TotalPosts,
//            COALESCE(SUM(P.PositivePosts), 0) AS PositivePosts, COALESCE(SUM(P.HighViewPosts), 0) AS HighViewPosts,
//            COALESCE(AVG(P.AvgViews), 0) AS AvgViews, COALESCE(AVG(P.AvgScore), 0) AS AvgScore
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN PostStats P ON U.Id = P.OwnerUserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation)
// SELECT UserId, DisplayName, Reputation, BadgeCount, TotalPosts, PositivePosts, HighViewPosts, AvgViews, AvgScore
// FROM UserStats ORDER BY Reputation DESC, TotalPosts DESC;
fn q14171(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let ps = owned(db)
        .group_by(owner_user.and(ptype_name(db)))
        .select(score.and(view_count.opt()))
        .fold([0i64; 6], |a, (s, v)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (v.unwrap_or(0) > 100) as i64, a[3] + v.is_some() as i64, a[4] + v.unwrap_or(0), a[5] + s]);
    let keys: MatSet<(Id<User>, Str)> = whole(&ps).collect();
    let of_user: HashIdx<Id<User>, (Id<User>, Str)> = (&keys).map(|(u, _)| u).inv().collect();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and((&of_user).select(&ps).opt()))
        .fold(([0i64; 6], (0.0f64, 0.0f64), (0.0f64, 0.0f64)), |(a, w, s), (b, p)| match p {
            Some(p) => {
                let (w, wn) = if p[3] > 0 { ((w.0 + p[4] as f64 / p[3] as f64, 0.0), 1) } else { (w, 0) };
                ([a[0] + b.is_some() as i64, a[1] + p[0], a[2] + p[1], a[3] + p[2], a[4] + wn, a[5] + 1], w, (s.0 + p[5] as f64 / p[0] as f64, 0.0))
            }
            None => ([a[0] + b.is_some() as i64, a[1], a[2], a[3], a[4], a[5]], w, s),
        });
    let mut out = Vec::new();
    (&us).drive(|u, (a, w, s)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.push(if a[4] == 0 { V::F(0.0) } else { V::F(w.0 / a[4] as f64) });
        f.push(if a[5] == 0 { V::F(0.0) } else { V::F(s.0 / a[5] as f64) });
        out.push(row(f))
    });
    rows(out)
}

// WITH PostMetrics AS (
//     SELECT p.PostTypeId, COUNT(*) AS TotalPosts, COUNT(p.Id) FILTER (WHERE p.Score > 0) AS PositiveScorePosts,
//            SUM(p.ViewCount) AS TotalViews, SUM(p.AnswerCount) AS TotalAnswers, AVG(p.Score) AS AverageScore,
//            MAX(p.CreationDate) AS LatestPostDate
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.PostTypeId),
// UserEngagement AS (
//     SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(c.Score) AS TotalCommentScore,
//            SUM(v.BountyAmount) AS TotalBountyReceived
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id GROUP BY u.Id)
// SELECT pmt.PostTypeId, pmt.TotalPosts, pmt.PositiveScorePosts, pmt.TotalViews, pmt.TotalAnswers, pmt.AverageScore,
//        ueng.UserId, ueng.PostsCreated, ueng.TotalCommentScore, ueng.TotalBountyReceived
// FROM PostMetrics pmt JOIN UserEngagement ueng ON ueng.PostsCreated > 0 ORDER BY pmt.PostTypeId, ueng.PostsCreated DESC;
fn q10949(db: &'static So) -> String {
    let Post { score, view_count, answer_count, creation_date, .. } = &db.post;
    let pm = db
        .post
        .with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)))
        .group_by(&db.post.post_type_id)
        .select(score.and(view_count.opt()).and(answer_count.opt()))
        .fold([0i64; 7], |a, ((s, v), an)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + s]);
    let Vote { user, post, .. } = &db.vote;
    let own: HashIdx<Id<Post>, Id<Vote>> = db.vote.with(user.and(post.select(&db.post.owner_user)).filt(|(a, b)| a == b)).select(post).inv().collect();
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).select(&db.comment.score).opt().and((&own).select((&db.vote.bounty_amount).opt()).opt())))
        .fold([0i64; 4], |a, (c, v)| {
            let v = v.flatten();
            [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0), a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let mut out = Vec::new();
    (&pm).cross((&dp).filt(|n| n > 0).and(&ue)).drive(|(t, u), (a, (n, b))| {
        out.push(row(vec![V::I(t), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), nullable(a[5], a[4]), avg(a[6], a[0]), user_col(db, u, "uid"), V::I(n), nullable(b[1], b[0]), nullable(b[3], b[2])]))
    });
    rows(out)
}

// WITH UserStatistics AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
//            SUM(P.Score) AS TotalScore, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// PostTypesStats AS (
//     SELECT PT.Name AS PostTypeName, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AverageViewCount
//     FROM PostTypes PT LEFT JOIN Posts P ON PT.Id = P.PostTypeId GROUP BY PT.Name)
// SELECT UStats.UserId, UStats.DisplayName, UStats.TotalPosts, UStats.Questions, UStats.Answers, UStats.Wikis, UStats.TotalScore,
//        UStats.TotalViews, PTStats.PostTypeName, PTStats.PostCount, PTStats.TotalScore AS PostTypeTotalScore, PTStats.AverageViewCount
// FROM UserStatistics UStats JOIN PostTypesStats PTStats ON UStats.TotalPosts > 0 ORDER BY UStats.TotalScore DESC, PTStats.PostTypeName;
fn q13075(db: &'static So) -> String {
    let ups = user_posts(db);
    let wk = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).fold(0i64, |n, t| n + (t == 3) as i64);
    let pts = type_left_posts_by_name(db);
    let mut out = Vec::new();
    (&ups).filt(|a| a[1] > 0).and(&wk).cross(&pts).drive(|(u, t), ((a, w), b)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(w), V::I(a[4]), V::I(a[6]), V::S(t), V::I(b[0]), nullable(b[1], b[0]), avg(b[3], b[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//            SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(C.CommentCount, 0)) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
//     WHERE U.CreationDate >= '2022-01-01' GROUP BY U.Id, U.Reputation),
// PostTypesStats AS (
//     SELECT PT.Id AS PostTypeId, PT.Name AS PostTypeName, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews
//     FROM PostTypes PT LEFT JOIN Posts P ON PT.Id = P.PostTypeId GROUP BY PT.Id, PT.Name)
// SELECT U.UserId, U.Reputation, U.PostCount AS UserPostCount, U.TotalScore AS UserTotalScore, U.TotalViews AS UserTotalViews,
//        U.TotalComments AS UserTotalComments, PT.PostTypeId, PT.PostTypeName, PT.PostCount AS PostTypePostCount,
//        PT.TotalScore AS PostTypeTotalScore, PT.TotalViews AS PostTypeTotalViews
// FROM UserStats U JOIN PostTypesStats PT ON U.PostCount > 0 ORDER BY U.Reputation DESC, PT.TotalViews DESC;
fn q10533(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let us = db
        .user
        .with((&db.user.creation_date).ge(date(2022, 1, 1)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt()).and((&cc).opt())))
        .fold([0i64; 4], |a, ((s, v), c)| [a[0] + 1, a[1] + s, a[2] + v.unwrap_or(0), a[3] + c.unwrap_or(0)]);
    let pts = type_left_posts(db);
    let mut out = Vec::new();
    (&us).cross(&pts).drive(|(u, t), (a, b)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(db.post_type.origid.get(t).unwrap()), tname(db, t), V::I(b[0]), nullable(b[1], b[0]), nullable(b[3], b[2])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserBadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId),
// PopularTags AS (
//     SELECT t.TagName, SUM(p.ViewCount) AS TotalViews FROM Tags t JOIN Posts p ON t.Id = p.Id WHERE p.PostTypeId = 1
//     GROUP BY t.TagName ORDER BY TotalViews DESC LIMIT 10),
// PostStatistics AS (
//     SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Posts p WHERE p.LastActivityDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' GROUP BY p.OwnerUserId)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ub.BadgeCount, 0) AS BadgeCount,
//        COALESCE(ub.HighestBadgeClass, 0) AS HighestBadgeClass, ps.PostCount, ps.TotalViews, ps.TotalScore, pt.TagName AS PopularTag
// FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId
// CROSS JOIN PopularTags pt WHERE u.Reputation > 1000 ORDER BY u.Reputation DESC, ps.TotalViews DESC LIMIT 50;
fn q5339(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let pids: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let pt = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&db.tag.origid).select(&pids).with((&db.post.post_type_id).eq(1)).select((&db.post.view_count).opt()))
        .fold((0i64, 0i64), |(n, s), v| (n + v.is_some() as i64, s + v.unwrap_or(0)));
    let pt = rel(top_n(drain(&pt), |&(_, (n, s))| (n == 0, Reverse(s)), 10));
    let ps = db
        .post
        .with((&db.post.last_activity_date).ge(ts(2023, 10, 1, 12, 34, 56)))
        .with(&db.post.owner_user)
        .group_by(&db.post.owner_user)
        .select((&db.post.view_count).opt().and(&db.post.score))
        .fold([0i64; 4], |a, (v, s)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s]);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select(Ident::<User>::new().and((&ub).opt()).and((&ps).opt())).cross(&pt));
    let key = |x: &((Id<User>, usize), (((Id<User>, Option<(i64, i64)>), Option<[i64; 4]>), (Str, (i64, i64))))| {
        let p = (x.1).0 .1;
        let w = p.filter(|p| p[1] > 0).map(|p| p[2]);
        (Reverse(db.user.reputation.get((x.0).0).unwrap()), w.is_none(), Reverse(w))
    };
    let v = top_n(v, key, 50);
    rows(v.iter().map(|&(_, (((u, b), p), (t, _)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b.map_or(0, |b| b.0)), V::I(b.map_or(0, |b| b.1))]);
        f.extend(match p {
            Some(p) => [V::I(p[0]), nullable(p[2], p[1]), V::I(p[3])],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(t));
        row(f)
    }))
}

// WITH PostCounts AS (SELECT PostTypeId, COUNT(*) AS TotalPosts, AVG(Score) AS AvgScore, SUM(ViewCount) AS TotalViews FROM Posts GROUP BY PostTypeId),
// UserStats AS (
//     SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// PostTypeStats AS (
//     SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, pc.TotalPosts, pc.AvgScore, pc.TotalViews, us.UserId, us.Reputation,
//            us.BadgeCount, us.TotalBounties
//     FROM PostTypes pt LEFT JOIN PostCounts pc ON pt.Id = pc.PostTypeId LEFT JOIN UserStats us ON us.UserId IS NOT NULL)
// SELECT p.PostTypeId, p.PostTypeName, p.TotalPosts, p.AvgScore, p.TotalViews, COALESCE(SUM(p.Reputation), 0) AS TotalUserReputation,
//        COALESCE(SUM(p.BadgeCount), 0) AS TotalBadges, COALESCE(SUM(p.TotalBounties), 0) AS TotalBounties
// FROM PostTypeStats p GROUP BY p.PostTypeId, p.PostTypeName, p.TotalPosts, p.AvgScore, p.TotalViews ORDER BY p.TotalPosts DESC;
fn q14611(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let pc = db.post.group_by(&db.post.post_type).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, v)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (b, v)| {
            let v = v.flatten();
            [a[0] + b.is_some() as i64, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0)]
        });
    let tot = whole(db.user.iq()).select((&db.user.reputation).and(&us)).fold([0i64; 4], |a, (r, b)| [a[0] + r, a[1] + b[0], a[2] + b[1], a[3] + b[2]]);
    let mut out = Vec::new();
    db.post_type.select(Ident::<PostType>::new().and((&pc).opt()).and(Ident::<PostType>::new().map(|_| ()).select(&tot).opt())).drive(|_, ((t, b), x)| {
        let mut f = vec![V::I(db.post_type.origid.get(t).unwrap()), tname(db, t)];
        f.extend(match b {
            Some(b) => [V::I(b[0]), avg(b[1], b[0]), nullable(b[3], b[2])],
            None => [V::Null, V::Null, V::Null],
        });
        let x = x.unwrap_or([0; 4]);
        f.extend([V::I(x[0]), V::I(x[1]), V::I(x[3])]);
        out.push(row(f))
    });
    rows(out)
}


// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AvgScore, SUM(p.ViewCount) AS TotalViews,
//            SUM(p.AnswerCount) AS TotalAnswers
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// TopUsers AS (
//     SELECT u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes,
//            AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.DisplayName ORDER BY PostsCreated DESC LIMIT 10),
// VoteStats AS (
//     SELECT vt.Name AS VoteType, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY vt.Name)
// SELECT ps.PostType, ps.TotalPosts, ps.AvgScore, ps.TotalViews, ps.TotalAnswers, tu.DisplayName AS TopUser, tu.PostsCreated,
//        tu.TotalUpVotes, tu.TotalDownVotes, tu.AvgReputation, vs.VoteType, vs.TotalVotes, vs.UpVotes, vs.DownVotes
// FROM PostStats ps CROSS JOIN TopUsers tu CROSS JOIN VoteStats vs ORDER BY ps.TotalPosts DESC;
fn q10928(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let User { display_name, up_votes, down_votes, reputation, .. } = &db.user;
    let tu = db
        .user
        .group_by(display_name)
        .select(up_votes.and(down_votes).and(reputation).and(posts_of(db).opt()))
        .fold([0i64; 4], |a, (((u, d), r), _)| [a[0] + 1, a[1] + u, a[2] + d, a[3] + r]);
    let dp = db.user.group_by(display_name).select(posts_of(db)).count_distinct();
    let tu = rel(top_n(drain((&tu).and((&dp).opt()).map(|(a, n)| (n.unwrap_or(0), a))), |&(_, (n, _))| Reverse(n), 10));
    let vs = db.vote.group_by(vtype_name(db)).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut out = Vec::new();
    (&ps).cross(&tu).cross(&vs).drive(|((t, _), vt), ((a, (name, (n, b))), c)| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), nullable(a[5], a[4]), V::S(name), V::I(n), V::I(b[1]), V::I(b[2]), avg(b[3], b[0]), V::S(vt), V::I(c[0]), V::I(c[1]), V::I(c[2])]))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//            SUM(CASE WHEN P.PostTypeId = 4 THEN 1 ELSE 0 END) AS TagWikiCount, SUM(CASE WHEN P.PostTypeId = 10 THEN 1 ELSE 0 END) AS ClosedPostCount,
//            AVG(P.Score) AS AverageScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation, U.CreationDate),
// TagStats AS (
//     SELECT T.Id AS TagId, T.TagName, SUM(P.ViewCount) AS TotalViewCount, COUNT(DISTINCT P.Id) AS PostCount, AVG(P.Score) AS AverageScore
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.Id, T.TagName)
// SELECT U.UserId, U.Reputation, U.CreationDate, U.PostCount, U.QuestionCount, U.AnswerCount, U.TagWikiCount, U.ClosedPostCount,
//        U.AverageScore, T.TagId, T.TagName, T.TotalViewCount, T.PostCount AS TagPostCount, T.AverageScore AS TagAverageScore
// FROM UserStats U JOIN TagStats T ON T.PostCount > 0 ORDER BY U.PostCount DESC, T.TotalViewCount DESC LIMIT 100;
fn q14393(db: &'static So) -> String {
    let ups = user_posts(db);
    let kinds = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).fold((0i64, 0i64), |(w, c), t| (w + (t == 4) as i64, c + (t == 10) as i64));
    let ts_ = tag_stats(db);
    let users = drain((&ups).and((&kinds).opt()));
    let tags = drain((&ts_).filt(|a| a[0] > 0));
    let v = cross_top(users, |&(_, (a, _))| Reverse(a[1]), tags, |&(_, b)| (b[1] == 0, Reverse(b[2])), 100);
    rows(v.iter().map(|&((u, (a, k)), (t, b))| {
        let k = k.unwrap_or((0, 0));
        let mut f = ucols(db, u, &["uid", "rep", "ucreated"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(k.0), V::I(k.1), avg(a[4], a[1])]);
        f.extend([V::I(db.tag.origid.get(t).unwrap()), V::S(db.tag.tag_name.get(t).unwrap()), nullable(b[2], b[1]), V::I(b[0]), avg(b[3], b[0])]);
        row(f)
    }))
}

/// Per tag over `Tags LEFT JOIN Posts ON p.Tags LIKE '%' || t.TagName || '%'`:
/// [posts, views present, views sum, score sum, questions, answers].
fn tag_stats(db: &'static So) -> Fold<Id<Tag>, [i64; 6]> {
    tag_stats_by(db, Ident::<Tag>::new())
}

/// `tag_stats` grouped by the tag's name.
fn tag_stats_by_name(db: &'static So) -> Fold<Str, [i64; 6]> {
    tag_stats_by(db, &db.tag.tag_name)
}

/// COUNT(DISTINCT p.Id) per tag name over `Tags JOIN Posts ON p.Tags LIKE '%' || t.TagName || '%'`.
fn tag_posts_by_name(db: &'static So) -> Fold<Str, i64> {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t)| t).inv().map(|(p, _)| p).collect();
    db.tag.group_by(&db.tag.tag_name).select(&by_tag).count_distinct()
}

fn tag_stats_by<K: IntoQuery>(db: &'static So, key: K) -> Fold<ROf<K>, [i64; 6]>
where
    K::Q: Probe<D = Id<Tag>>,
    ROf<K>: Copy + Eq + std::hash::Hash,
{
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    db.tag
        .group_by(key)
        .select((&by_tag).map(|(p, _)| p).select((&db.post.view_count).opt().and(&db.post.score).and(&db.post.post_type_id)).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((v, s), t)) => [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s, a[4] + (t == 1) as i64, a[5] + (t == 2) as i64],
            None => a,
        })
}

// WITH UserPerformance AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TagPerformance AS (
//     SELECT T.TagName, COUNT(DISTINCT P.Id) AS TotalPostsTagged, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswersTagged,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestionsTagged
//     FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName)
// SELECT UP.UserId, UP.DisplayName, UP.Reputation, UP.TotalPosts, UP.TotalQuestions, UP.TotalAnswers, UP.TotalUpvotes, UP.TotalDownvotes,
//        TP.TagName, TP.TotalPostsTagged, TP.TotalAnswersTagged, TP.TotalQuestionsTagged
// FROM UserPerformance UP LEFT JOIN TagPerformance TP ON UP.TotalPosts > 0
// ORDER BY UP.Reputation DESC, UP.TotalPosts DESC, TP.TotalPostsTagged DESC LIMIT 10;
//
// Every user has at least one row, so only users ranked in the first ten by
// (reputation, posts) can appear: they are picked by a window first.
fn q5785(db: &'static So) -> String {
    let up = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (t, v)| [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]);
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let ts_ = tag_stats_by_name(db);
    let tpd = tag_posts_by_name(db);
    let tp = (&ts_).filt(|a| a[0] > 0).and(&tpd);
    let tp: HashIdx<(), (Str, ([i64; 6], i64))> = (&tp).map(|_| ()).inv().select(Same::<Str>::new().and(&tp)).collect();
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let ranked = whole(db.user.iq()).select(Ident::<User>::new().and((&dp).opt())).window(rank, |(u, n): (Id<User>, Option<i64>)| (Reverse(rep(u)), Reverse(n.unwrap_or(0))), asc);
    let picked = (&ranked)
        .filt(|(_, r)| r <= 10)
        .map(|((u, _), _)| u)
        .select(Ident::<User>::new().and((&dp).opt()).and((&up).opt()).and((&dp).filt(|n| n > 0).map(|_| ()).select(&tp).opt()));
    let v = top_n(drain(picked), |&(_, (((u, n), _), t))| (Reverse(rep(u)), Reverse(n.unwrap_or(0)), t.is_none(), Reverse(t.map(|(_, (_, d))| d))), 10);
    rows(v.iter().map(|&(_, (((u, n), a), t))| {
        let a = a.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[1]), V::I(a[0]), V::I(a[2]), V::I(a[3])]);
        f.extend(match t {
            Some((name, (b, d))) => [V::S(name), V::I(d), V::I(b[5]), V::I(b[4])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserPostMetrics AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
//            COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
//            SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers,
//            SUM(COALESCE(P.CommentCount, 0)) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (
//     SELECT PT.Name AS PostType, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore,
//            MIN(P.CreationDate) AS EarliestPost, MAX(P.CreationDate) AS LatestPost
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id GROUP BY PT.Name)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.Questions, U.Answers, U.TotalViews, U.TotalScore, U.TotalAnswers, U.TotalComments,
//        PS.PostType, PS.PostCount, PS.TotalViews, PS.AverageScore, PS.EarliestPost, PS.LatestPost
// FROM UserPostMetrics U LEFT JOIN PostStatistics PS ON PS.PostType IN ('Question', 'Answer') ORDER BY U.TotalScore DESC, U.TotalPosts DESC;
fn q11080(db: &'static So) -> String {
    let ups = user_posts(db);
    let Post { answer_count, comment_count, creation_date, .. } = &db.post;
    let ac = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(answer_count.opt().and(comment_count))).fold((0i64, 0i64), |(a, c), (x, y)| (a + x.unwrap_or(0), c + y));
    let ps = db
        .post
        .group_by(ptype_name(db).filt(|n| n == "Question" || n == "Answer"))
        .select((&db.post.view_count).opt().and(&db.post.score).and(creation_date))
        .fold([0, 0, 0, 0, i64::MAX, i64::MIN], |a, ((v, s), d)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s, a[4].min(d), a[5].max(d)]);
    let ps: HashIdx<(), (Str, [i64; 6])> = (&ps).map(|_| ()).inv().select(Same::<Str>::new().and(&ps)).collect();
    let mut out = Vec::new();
    (&ups).and((&ac).opt()).and(Ident::<User>::new().map(|_| ()).select(&ps).opt()).drive(|u, ((a, c), x)| {
        let c = c.unwrap_or((0, 0));
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), V::I(c.0), V::I(c.1)]);
        match x {
            Some((t, b)) => f.extend([V::S(t), V::I(b[0]), nullable(b[2], b[1]), avg(b[3], b[0]), tmin(b[4]), tmax(b[5])]),
            None => f.extend((0..6).map(|_| V::Null)),
        }
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT p.PostTypeId, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//            SUM(CASE WHEN p.ViewCount > 0 THEN 1 ELSE 0 END) AS ViewedPosts, AVG(p.ViewCount) AS AvgViews, AVG(p.Score) AS AvgScore
//     FROM Posts p GROUP BY p.PostTypeId),
// UserStats AS (
//     SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS TotalPostsByUser, SUM(p.Score) AS TotalScoreByUser, COUNT(b.Id) AS TotalBadgesByUser
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// VoteStats AS (SELECT v.VoteTypeId, COUNT(*) AS TotalVotes, COUNT(DISTINCT v.PostId) AS UniquePostsVoted FROM Votes v GROUP BY v.VoteTypeId)
// SELECT pts.PostTypeId, pts.TotalPosts, pts.PositiveScorePosts, pts.ViewedPosts, pts.AvgViews, pts.AvgScore, us.TotalPostsByUser,
//        us.TotalScoreByUser, us.TotalBadgesByUser, vs.VoteTypeId, vs.TotalVotes, vs.UniquePostsVoted
// FROM PostStats pts JOIN UserStats us ON us.TotalPostsByUser = (SELECT MAX(TotalPostsByUser) FROM UserStats)
// JOIN VoteStats vs ON vs.TotalVotes = (SELECT MAX(TotalVotes) FROM VoteStats) ORDER BY pts.PostTypeId;
fn q12697(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let ps = db
        .post
        .group_by(&db.post.post_type_id)
        .select(score.and(view_count.opt()))
        .fold([0i64; 6], |a, (s, v)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (v.unwrap_or(0) > 0) as i64, a[3] + v.is_some() as i64, a[4] + v.unwrap_or(0), a[5] + s]);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (s, b)| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0), a[2] + b.is_some() as i64]);
    let dpc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let dp = db.user.select((&dpc).opt()).map(|n| n.unwrap_or(0));
    let most = (&dp).fold_flat(i64::MIN, |m, n| m.max(n));
    let vs = db.vote.group_by(&db.vote.vote_type_id).fold(0i64, |n, _| n + 1);
    let vd = db.vote.group_by(&db.vote.vote_type_id).select(&db.vote.post_id).count_distinct();
    let vmost = (&vs).fold_flat(i64::MIN, |m, n| m.max(n));
    let mut out = Vec::new();
    (&ps).cross((&dp).filt(|n| n == most).and(&us)).cross((&vs).filt(|n| n == vmost).and(&vd)).drive(|((t, _), vt), ((a, (n, b)), (vn, vdn))| {
        out.push(row(vec![V::I(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), avg(a[5], a[0]), V::I(n), nullable(b[1], b[0]), V::I(b[2]), V::I(vt), V::I(vn), V::I(vdn)]))
    });
    rows(out)
}

// WITH UserVoteStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PopularPosts AS (
//     SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.Score, P.ViewCount, COUNT(C.Id) AS CommentCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title, P.OwnerUserId, P.Score, P.ViewCount ORDER BY P.Score DESC, P.ViewCount DESC LIMIT 10),
// UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT U.DisplayName, U.TotalVotes, U.UpVotes, U.DownVotes, PP.Title AS PopularPost, PP.Score, PP.ViewCount, UBC.BadgeCount
// FROM UserVoteStats U JOIN PopularPosts PP ON U.UserId = PP.OwnerUserId JOIN UserBadgeCounts UBC ON U.UserId = UBC.UserId
// WHERE U.TotalVotes > 50 ORDER BY U.TotalVotes DESC, U.UpVotes DESC;
//
// CURRENT_DATE is the day the query runs, so the answer depends on it; the
// data ends in 2024, so today it is empty.
fn q9648(db: &'static So) -> String {
    let uv = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| match t {
            Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let recent = db.post.with((&db.post.creation_date).ge(add_years(current_date(), -1))).select((&db.post.score).and((&db.post.view_count).opt()));
    let pp = rel(top_n(drain(recent), |&(_, (s, v))| (Reverse(s), v.is_none(), Reverse(v)), 10));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut out = Vec::new();
    (&pp).map(|(p, _)| p).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and((&uv).filt(|a| a[0] > 50)).and(&bc)))).drive(|_, (p, ((u, a), b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(V::I(b));
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// UserBadgeStats AS (
//     SELECT B.UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//            SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Badges B GROUP BY B.UserId)
// SELECT U.UserId, U.DisplayName, COALESCE(U.TotalPosts, 0) AS TotalPosts, ..., COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM UserPostStats U FULL OUTER JOIN UserBadgeStats B ON U.UserId = B.UserId ORDER BY TotalScore DESC, TotalPosts DESC;
//
// Badges.UserId never dangles (it is a dense edge in the cache), so the
// FULL JOIN's right-only side is empty and it is a LEFT JOIN.
fn q11446(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let mut out = Vec::new();
    (&ups).and((&ub).opt()).drive(|u, (a, b)| {
        let b = b.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments,
//            SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//            MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViewCount,
//            SUM(p.AnswerCount) AS TotalAnswers, SUM(p.CommentCount) AS TotalComments
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name)
// SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalComments, ua.TotalUpVotes, ua.TotalDownVotes, ua.LastPostDate, ps.PostType,
//        ps.TotalPosts AS posts_per_type, ps.AverageScore, ps.AverageViewCount, ps.TotalAnswers, ps.TotalComments AS comments_per_type
// FROM UserActivity ua CROSS JOIN PostStatistics ps ORDER BY ua.TotalPosts DESC, ua.TotalUpVotes DESC;
fn q11621(db: &'static So) -> String {
    let Vote { user, post, .. } = &db.vote;
    let own: HashIdx<Id<Post>, Id<Vote>> = db.vote.with(user.and(post.select(&db.post.owner_user)).filt(|(a, b)| a == b)).select(post).inv().collect();
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt())))
        .fold((0i64, 0i64), |(u, d), (_, t)| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let ups = user_posts(db);
    let dc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).count_distinct();
    let ps = stats_by_type(db);
    let mut out = Vec::new();
    (&ups).and((&ua).opt()).and((&dc).opt()).cross(&ps).drive(|(u, t), (((a, v), c), b)| {
        let v = v.unwrap_or((0, 0));
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(c.unwrap_or(0)), V::I(v.0), V::I(v.1), tmax(a[7])]);
        f.extend([V::S(t), V::I(b[0]), avg(b[1], b[0]), avg(b[3], b[2]), nullable(b[5], b[4]), V::I(b[6])]);
        out.push(row(f))
    });
    rows(out)
}


fn own_votes(db: &'static So) -> HashIdx<Id<Post>, Id<Vote>> {
    let Vote { user, post, .. } = &db.vote;
    db.vote.with(user.and(post.select(&db.post.owner_user)).filt(|(a, b)| a == b)).select(post).inv().collect()
}

fn fkey(x: f64) -> i64 {
    let b = x.to_bits() as i64;
    b ^ (((b >> 63) as u64) >> 1) as i64
}

// WITH PostStatistics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers, SUM(p.Score) AS TotalScore,
//            SUM(p.ViewCount) AS TotalViews, AVG(COALESCE(p.CommentCount, 0)) AS AvgCommentsPerPost,
//            AVG(COALESCE(p.AnswerCount, 0)) AS AvgAnswersPerPost, MAX(p.CreationDate) AS MostRecentPost
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserEngagement AS (
//     SELECT u.DisplayName AS User, COUNT(DISTINCT p.Id) AS TotalPostsByUser, SUM(v.BountyAmount) AS TotalBountyReceived,
//            SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id
//     GROUP BY u.DisplayName)
// SELECT p.*, u.* FROM PostStatistics p JOIN UserEngagement u ON u.TotalPostsByUser > 0
// ORDER BY p.TotalPosts DESC, u.TotalPostsByUser DESC;
fn q12276(db: &'static So) -> String {
    let ps = stats_by_type(db);
    let uu = db.post.group_by(ptype_name(db)).select(&db.post.owner_user_id).count_distinct();
    let own = own_votes(db);
    let User { display_name, up_votes, down_votes, reputation, .. } = &db.user;
    let ue = db
        .user
        .group_by(display_name)
        .select(up_votes.and(down_votes).and(reputation).and(posts_of(db).select((&own).select((&db.vote.bounty_amount).opt()).opt()).opt()))
        .fold([0i64; 6], |a, (((u, d), r), v)| {
            let v = v.flatten().flatten();
            [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + u, a[4] + d, a[5] + r]
        });
    let dp = db.user.group_by(display_name).select(posts_of(db)).count_distinct();
    let mut out = Vec::new();
    (&ps).and((&uu).opt()).cross((&dp).filt(|n| n > 0).and(&ue)).drive(|(t, name), ((a, o), (n, b))| {
        out.push(row(vec![
            V::S(t),
            V::I(a[0]),
            V::I(o.unwrap_or(0)),
            V::I(a[1]),
            nullable(a[3], a[2]),
            avg(a[6], a[0]),
            avg(a[5], a[0]),
            tmax(a[10]),
            V::S(name),
            V::I(n),
            nullable(b[2], b[1]),
            V::I(b[3]),
            V::I(b[4]),
            avg(b[5], b[0]),
        ]))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViewCount,
//            AVG(EXTRACT(EPOCH FROM p.CreationDate)) AS AvgPostAge
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TagPostStats AS (
//     SELECT t.TagName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViewCount
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName)
// SELECT u.UserId, u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalScore, u.TotalViewCount, u.AvgPostAge,
//        t.TagName, t.TotalPosts AS TagTotalPosts, t.TotalQuestions AS TagTotalQuestions, t.TotalAnswers AS TagTotalAnswers,
//        t.TotalScore AS TagTotalScore, t.TotalViewCount AS TagTotalViewCount
// FROM UserPostStats u JOIN TagPostStats t ON u.TotalPosts > 0 ORDER BY u.TotalScore DESC, t.TotalScore DESC FETCH FIRST 100 ROWS ONLY;
fn q12939(db: &'static So) -> String {
    let ups = user_posts(db);
    let age = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.creation_date))
        .fold(((0.0f64, 0.0f64), 0i64), |(s, n), d| (kahan(s, secs(d)), n + 1));
    let ts_ = tag_stats_by_name(db);
    let users = drain((&ups).filt(|a| a[1] > 0).and(&age));
    let tags = drain(&ts_);
    let v = cross_top(users, |&(_, (a, _))| Reverse(a[4]), tags, |&(_, b)| (b[0] == 0, Reverse(b[3])), 100);
    rows(v.iter().map(|&((u, (a, (s, n))), (t, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), nullable(a[6], a[5]), fmean(s, n)]);
        f.extend([V::S(t), V::I(b[0]), V::I(b[4]), V::I(b[5]), nullable(b[3], b[0]), nullable(b[2], b[1])]);
        row(f)
    }))
}

// WITH RecentPosts AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UpVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopUsers AS (
//     SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadges, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.DisplayName ORDER BY TotalScore DESC LIMIT 10),
// PostDetails AS (
//     SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, tu.DisplayName AS TopUser, tu.TotalBadges
//     FROM RecentPosts rp JOIN TopUsers tu ON rp.UpVoteCount > 0 ORDER BY rp.Score DESC, rp.ViewCount DESC)
// SELECT pd.* FROM PostDetails pd WHERE pd.UpVoteCount >= 5 ORDER BY pd.CreationDate DESC;
fn q9178(db: &'static So) -> String {
    let ups: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).inv().collect();
    let recent = db.post.with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let uv = recent.group_by(Ident::<Post>::new()).select((&ups).select(&db.vote.user_id)).count_distinct();
    let recent = db.post.with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let rp = recent.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and((&ups).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(&db.post.score).opt()))
        .fold([0i64; 4], |a, (c, s)| [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0), a[2] + s.is_some() as i64, a[3] + s.unwrap_or(0)]);
    let tu = rel(top_n(drain(&tu), |&(_, a)| (a[2] == 0, Reverse(a[3])), 10));
    let mut out = Vec::new();
    (&rp).and((&uv).filt(|n| n >= 5)).cross(&tu).drive(|(p, _), ((c, n), (u, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n), user_col(db, u, "name"), nullable(a[1], a[0])]);
        out.push(row(f))
    });
    rows(out)
}

// WITH UserVoteStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
//            SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY u.Id, u.DisplayName),
// PostStats AS (
//     SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.Score, p.ViewCount),
// TopPostsWithUserVotes AS (
//     SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.CommentCount, uvs.DisplayName AS TopVoter, uvs.TotalVotes, uvs.UpVotes, uvs.DownVotes
//     FROM PostStats ps JOIN UserVoteStats uvs ON uvs.TotalVotes = (SELECT MAX(TotalVotes) FROM UserVoteStats)
//     ORDER BY ps.Score DESC, ps.ViewCount DESC LIMIT 10)
// SELECT PostId, Title, Score, ViewCount, CommentCount, TopVoter, TotalVotes, UpVotes, DownVotes FROM TopPostsWithUserVotes;
fn q7217(db: &'static So) -> String {
    let uv = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vtype_name(db)).opt())
        .fold([0i64; 3], |a, n| match n {
            Some(n) => [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64],
            None => a,
        });
    let most = (&uv).fold_flat(i64::MIN, |m, a| m.max(a[0]));
    let ps = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let voters = rel(drain((&uv).filt(|a| a[0] == most)));
    let v = drain((&ps).cross(&voters));
    let v = top_n(v, |&((p, _), _)| {
        let w = db.post.view_count.get(p);
        (Reverse(db.post.score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    rows(v.iter().map(|&((p, _), (c, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, ...
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopPosts AS (
//     SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount,
//            COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END), 0) AS NetScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title ORDER BY NetScore DESC LIMIT 10)
// SELECT ubc.UserId, ubc.DisplayName, ubc.BadgeCount, tb.PostId, tb.Title, tb.CommentCount, tb.NetScore
// FROM UserBadgeCounts ubc JOIN TopPosts tb ON ubc.UserId IN (SELECT p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1)
// ORDER BY ubc.BadgeCount DESC, tb.NetScore DESC;
fn q28696(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tp = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(c, s), (x, t)| (c + x.is_some() as i64, s + (t == Some(2)) as i64 - (t == Some(3)) as i64));
    let tp = rel(top_n(drain(&tp), |&(_, (_, s))| Reverse(s), 10));
    let askers: MatSet<i64> = db.post.with((&db.post.post_type_id).eq(1)).select(&db.post.owner_user_id).collect();
    let mut out = Vec::new();
    db.user.with((&db.user.origid).with(&askers)).select(Ident::<User>::new().and(&bc)).cross(&tp).drive(|_, ((u, b), (p, (c, s)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(c), V::I(s)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH TagStats AS (
//     SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(P.ViewCount) AS AvgViewCount, AVG(P.Score) AS AvgScore
//     FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName),
// UserInteractions AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT V.Id) AS VoteCount,
//            SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
//            SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
//     FROM Users U LEFT JOIN Comments C ON C.UserId = U.Id LEFT JOIN Votes V ON V.UserId = U.Id LEFT JOIN Badges B ON B.UserId = U.Id
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName)
// SELECT TS.*, UI.* FROM TagStats TS JOIN UserInteractions UI ON TS.PostCount > 5
// ORDER BY TS.AvgScore DESC, UI.CommentCount DESC LIMIT 10;
fn q28469(db: &'static So) -> String {
    let ts_ = tag_stats_by_name(db);
    let ui = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (_, c)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let dc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(comments_by(db)).count_distinct();
    let dv = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(votes_by(db)).count_distinct();
    let tags = drain((&ts_).and(&tag_posts_by_name(db)).filt(|(_, d)| d > 5));
    let users = drain((&ui).and((&dc).opt()).and((&dv).opt()));
    let v = cross_top(tags, |&(_, (a, _))| Reverse(fkey(a[3] as f64 / a[0] as f64)), users, |&(_, ((_, c), _))| Reverse(c.unwrap_or(0)), 10);
    rows(v.iter().map(|&((t, (a, d)), (u, ((b, c), w)))| {
        let mut f = vec![V::S(t), V::I(d), V::I(a[4]), V::I(a[5]), avg(a[2], a[1]), avg(a[3], a[0])];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(w.unwrap_or(0)), V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        row(f)
    }))
}

// WITH UserBadges AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, ...
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (
//     SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount
//     FROM Posts P WHERE P.CreationDate > CURRENT_DATE - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// ActiveUsers AS (
//     SELECT U.Id AS UserId, U.DisplayName, UB.BadgeCount, PS.TotalPosts, PS.Questions, PS.Answers, PS.TotalScore, PS.AvgViewCount
//     FROM Users U JOIN UserBadges UB ON U.Id = UB.UserId JOIN PostStats PS ON U.Id = PS.OwnerUserId
//     WHERE U.LastAccessDate > CURRENT_DATE - INTERVAL '6 months')
// SELECT UserId, DisplayName, BadgeCount, TotalPosts, Questions, Answers, TotalScore, AvgViewCount
// FROM ActiveUsers ORDER BY TotalScore DESC, BadgeCount DESC LIMIT 100;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024.
fn q6006(db: &'static So) -> String {
    let today = current_date();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ps = db
        .post
        .with((&db.post.creation_date).gt(add_years(today, -1)))
        .with(&db.post.owner_user)
        .group_by(&db.post.owner_user)
        .select((&db.post.post_type_id).and(&db.post.score).and((&db.post.view_count).opt()))
        .fold([0i64; 6], |a, ((t, s), v)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0)]);
    let v = drain(db.user.with((&db.user.last_access_date).gt(add_months(today, -6))).select(Ident::<User>::new().and(&bc).and(&ps)));
    let v = top_n(v, |&(_, ((_, b), a))| (Reverse(a[3]), Reverse(b)), 100);
    rows(v.iter().map(|&(_, ((u, b), a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4])]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, ... GoldBadges, SilverBadges, BronzeBadges
//                     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (
//     SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount,
//            MAX(p.CreationDate) AS LastPostDate
//     FROM Posts p GROUP BY p.OwnerUserId),
// UserPerformance AS (
//     SELECT ub.UserId, COALESCE(ps.PostCount, 0) AS PostCount, ..., ps.LastPostDate
//     FROM UserBadges ub FULL OUTER JOIN PostStats ps ON ub.UserId = ps.OwnerUserId)
// SELECT u.DisplayName, up.PostCount, up.TotalScore, up.AvgViewCount, up.BadgeCount, up.GoldBadges, up.SilverBadges, up.BronzeBadges, up.LastPostDate
// FROM Users u JOIN UserPerformance up ON u.Id = up.UserId WHERE up.PostCount > 0
// ORDER BY up.TotalScore DESC, up.PostCount DESC LIMIT 10;
//
// UserPerformance.UserId is ub.UserId, NULL on the PostStats-only side, so
// the join to Users keeps only the UserBadges side: a LEFT JOIN.
fn q8342(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ps = owned(db)
        .group_by(&db.post.owner_user)
        .select((&db.post.score).and((&db.post.view_count).opt()).and(&db.post.creation_date))
        .fold([0, 0, 0, i64::MIN], |a, ((s, v), d)| [a[0] + 1, a[1] + s, a[2] + v.unwrap_or(0), a[3].max(d)]);
    let v = drain((&ub).and(&ps).filt(|(_, p)| p[0] > 0));
    let v = top_n(v, |&(_, (_, p))| (Reverse(p[1]), Reverse(p[0])), 10);
    rows(v.iter().map(|&(u, (b, p))| row(vec![user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), avg(p[2], p[0]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3]), tmax(p[3])])))
}

// WITH TopUsers AS (
//     SELECT U.Id AS UserId, U.DisplayName, SUM(U.UpVotes) AS TotalUpVotes, SUM(U.DownVotes) AS TotalDownVotes, COUNT(DISTINCT P.Id) AS PostCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     WHERE U.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName),
// PopularTags AS (
//     SELECT T.TagName, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews
//     FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName ORDER BY PostCount DESC LIMIT 10),
// RecentActivities AS (
//     SELECT U.Id AS UserId, U.DisplayName, PH.CreationDate, PH.Comment, P.Title AS PostTitle
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id JOIN Users U ON PH.UserId = U.Id
//     WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month')
// SELECT TU.DisplayName AS TopUser, TU.TotalUpVotes, TU.TotalDownVotes, TU.PostCount, PT.TagName AS PopularTag, PT.PostCount AS TagPostCount,
//        PT.TotalViews AS TagTotalViews, RA.UserId AS ActivityUserId, RA.PostTitle, RA.Comment, RA.CreationDate AS ActivityDate
// FROM TopUsers TU CROSS JOIN PopularTags PT LEFT JOIN RecentActivities RA ON TU.UserId = RA.UserId
// ORDER BY TU.TotalUpVotes DESC, PT.PostCount DESC, RA.CreationDate DESC;
fn q5844(db: &'static So) -> String {
    let now = ts(2024, 10, 1, 12, 34, 56);
    let User { up_votes, down_votes, .. } = &db.user;
    let tu = db
        .user
        .with((&db.user.creation_date).ge(add_years(now, -1)))
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).opt()))
        .fold((0i64, 0i64), |(u, d), ((x, y), _)| (u + x, d + y));
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let ts_ = tag_stats_by_name(db);
    let pt = rel(top_n(drain((&ts_).filt(|a| a[0] > 0)), |&(_, a)| Reverse(a[0]), 10));
    let ra: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with((&db.post_history.creation_date).ge(add_months(now, -1))).select(&db.post_history.user).inv().collect();
    let mut out = Vec::new();
    (&tu).and((&dp).opt()).and((&ra).opt()).cross(&pt).drive(|(u, _), ((((up, dn), n), h), (t, a))| {
        let mut f = vec![user_col(db, u, "name"), V::I(up), V::I(dn), V::I(n.unwrap_or(0)), V::S(t), V::I(a[0]), nullable(a[2], a[1])];
        f.extend(match h {
            Some(h) => [user_col(db, u, "uid"), title(db, db.post_history.post.get(h).unwrap()), ostr(db.post_history.comment.get(h)), V::T(db.post_history.creation_date.get(h).unwrap())],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}


// WITH RECURSIVE UserActivity AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount,
//            SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END) AS PositiveScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPosts AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 YEAR'
//     ORDER BY p.CreationDate DESC LIMIT 10),
// PostHistoryDetails AS (
//     SELECT ph.PostId, ph.CreationDate AS HistoryDate, pht.Name AS HistoryTypeName, ph.UserDisplayName AS EditorName
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
//     WHERE ph.CreationDate >= CURRENT_DATE - INTERVAL '6 MONTH')
// SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.PostCount, ua.PositiveScore, rp.PostId, rp.Title AS RecentPostTitle,
//        rp.CreationDate AS RecentPostDate, rp.Score AS RecentPostScore, rp.OwnerDisplayName, phd.HistoryDate, phd.HistoryTypeName, phd.EditorName
// FROM UserActivity ua LEFT JOIN RecentPosts rp ON ua.PostCount > 0 LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId
// WHERE ua.Reputation > 1000 ORDER BY ua.Reputation DESC, rp.CreationDate DESC;
//
// RECURSIVE is written but nothing recurses. CURRENT_DATE is the day the
// query runs; the data ends in 2024, so RecentPosts is empty.
fn q32144(db: &'static So) -> String {
    let today = current_date();
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt())
        .fold((0i64, 0i64), |(n, s), x| match x {
            Some(x) => (n + 1, s + x.max(0)),
            None => (n, s),
        });
    let rp = rel(top_n(drain(owned(db).with((&db.post.creation_date).ge(add_years(today, -1))).select(&db.post.creation_date)), |&(_, d)| Reverse(d), 10));
    let phd: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with((&db.post_history.creation_date).ge(add_months(today, -6))).select(&db.post_history.post).inv().collect();
    let rp: HashIdx<(), Id<Post>> = (&rp).map(|_| ()).inv().select((&rp).map(|(p, _)| p)).collect();
    let mut out = Vec::new();
    (&ua).and((&ua).filt(|(n, _)| n > 0).map(|_| ()).select(&rp).select(Ident::<Post>::new().and((&phd).opt())).opt()).drive(|u, ((n, s), x)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(s)]);
        match x {
            Some((p, h)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "score", "owner"]));
                f.extend(match h {
                    Some(h) => [V::T(db.post_history.creation_date.get(h).unwrap()), V::S(db.post_history_type.name.get(db.post_history.post_history_type.get(h).unwrap()).unwrap()), ostr(db.post_history.user_display_name.get(h))],
                    None => [V::Null, V::Null, V::Null],
                });
            }
            None => f.extend((0..8).map(|_| V::Null)),
        }
        out.push(row(f))
    });
    rows(out)
}

// WITH TagStats AS (
//     SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//            SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers, SUM(COALESCE(p.CommentCount, 0)) AS TotalComments
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(p.ViewCount) AS TotalPostViews,
//            SUM(COALESCE(b.Class, 0)) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// FavoritePosts AS (
//     SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, COUNT(v.Id) AS FavoriteCount
//     FROM Posts p LEFT JOIN Votes v ON v.PostId = p.Id AND v.VoteTypeId = 5 WHERE p.FavoriteCount > 0
//     GROUP BY p.Id, p.Title, p.ViewCount, p.AnswerCount)
// SELECT ts.TagName, ts.PostCount, ts.TotalViews, ts.TotalAnswers, ts.TotalComments, us.DisplayName AS UserWithMostPosts,
//        us.TotalPosts, us.TotalPostViews, us.TotalBadges, fp.Title AS FavoritePostTitle, fp.ViewCount AS FavoritePostViews,
//        fp.AnswerCount AS FavoritePostAnswers, fp.FavoriteCount AS FavoritePostFavorites
// FROM TagStats ts JOIN UserStats us ON us.TotalPosts = (SELECT MAX(TotalPosts) FROM UserStats)
// JOIN FavoritePosts fp ON fp.FavoriteCount = (SELECT MAX(FavoriteCount) FROM FavoritePosts)
// ORDER BY ts.TotalViews DESC, ts.PostCount DESC LIMIT 10;
fn q27925(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let Post { view_count, answer_count, comment_count, .. } = &db.post;
    let ts_ = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select(view_count.opt().and(answer_count.opt()).and(comment_count)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((v, an), c)) => [a[0] + 1, a[1] + v.unwrap_or(0), a[2] + an.unwrap_or(0), a[3] + c],
            None => a,
        });
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (v, c)| {
            let v = v.flatten();
            [a[0] + v.is_some() as i64, a[1] + v.unwrap_or(0), a[2] + c.unwrap_or(0)]
        });
    let tpd = tag_posts_by_name(db);
    let dpc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let dp = db.user.select((&dpc).opt()).map(|n| n.unwrap_or(0));
    let most = (&dp).fold_flat(i64::MIN, |m, n| m.max(n));
    let favs: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(5)).select(&db.vote.post).inv().collect();
    let fp = db.post.with((&db.post.favorite_count).gt(0)).group_by(Ident::<Post>::new()).select((&favs).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let fmost = (&fp).fold_flat(i64::MIN, |m, n| m.max(n));
    let users = rel(drain((&dp).filt(|n| n == most).and(&us)));
    let posts = rel(drain((&fp).filt(|n| n == fmost)));
    let v = drain((&ts_).and((&tpd).opt()).cross(&users).cross(&posts));
    let v = top_n(v, |&(_, (((a, d), _), _))| (Reverse(a[1]), Reverse(d.unwrap_or(0))), 10);
    rows(v.iter().map(|&(((t, _), _), (((a, d), (u, (n, b))), (p, c)))| {
        let mut f = vec![V::S(t), V::I(d.unwrap_or(0)), V::I(a[1]), V::I(a[2]), V::I(a[3]), user_col(db, u, "name"), V::I(n), nullable(b[1], b[0]), V::I(b[2])];
        f.extend(post_fields(db, p, &["title", "views", "answers"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserPosts AS (
//     SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.PostTypeId, COUNT(*) AS TotalPosts, AVG(p.ViewCount) AS AvgViewCount, AVG(p.Score) AS AvgScore
//                    FROM Posts p GROUP BY p.PostTypeId),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, ... GoldBadges, SilverBadges, BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT up.UserId, up.DisplayName, up.PostCount, up.QuestionCount, up.AnswerCount, up.TotalScore, up.AvgViewCount,
//        COALESCE(ub.BadgeCount, 0) AS BadgeCount, ..., ps.PostTypeId, ps.TotalPosts, ps.AvgViewCount AS PostAvgViewCount, ps.AvgScore AS PostAvgScore
// FROM UserPosts up LEFT JOIN UserBadges ub ON up.UserId = ub.UserId LEFT JOIN PostStatistics ps ON true
// ORDER BY up.TotalScore DESC, up.PostCount DESC;
fn q11001(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let ps = db
        .post
        .group_by(&db.post.post_type_id)
        .select((&db.post.view_count).opt().and(&db.post.score))
        .fold([0i64; 4], |a, (v, s)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s]);
    let ps: HashIdx<(), (i64, [i64; 4])> = (&ps).map(|_| ()).inv().select(Same::<i64>::new().and(&ps)).collect();
    let mut out = Vec::new();
    (&ups).and((&ub).opt()).and(Ident::<User>::new().map(|_| ()).select(&ps).opt()).drive(|u, ((a, b), x)| {
        let b = b.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), avg(a[6], a[5]), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        f.extend(match x {
            Some((t, p)) => [V::I(t), V::I(p[0]), avg(p[2], p[1]), avg(p[3], p[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH UserPostStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS TotalPositivePosts
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// PopularTags AS (
//     SELECT T.TagName, COUNT(P.Tags) AS TagCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%'
//     GROUP BY T.TagName ORDER BY TagCount DESC LIMIT 10),
// PostActivity AS (
//     SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, COUNT(C.Id) AS CommentCount, COUNT(V.Id) AS VoteCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' GROUP BY P.Id, P.Title, P.ViewCount, P.CreationDate)
// SELECT UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.TotalQuestions, UPS.TotalAnswers, UPS.TotalPositivePosts, PT.TagName AS PopularTag,
//        PA.Title AS PopularPostTitle, PA.ViewCount AS PopularPostViewCount, PA.CommentCount AS PopularPostCommentCount, PA.VoteCount AS PopularPostVoteCount
// FROM UserPostStats UPS CROSS JOIN PopularTags PT JOIN PostActivity PA ON PA.ViewCount = (SELECT MAX(ViewCount) FROM PostActivity)
// ORDER BY UPS.TotalPosts DESC, PA.ViewCount DESC LIMIT 10;
fn q26400(db: &'static So) -> String {
    let ups = user_posts(db);
    let ts_ = tag_stats_by_name(db);
    let pt = rel(top_n(drain((&ts_).filt(|a| a[0] > 0)), |&(_, a)| Reverse(a[0]), 10));
    let recent = db.post.with((&db.post.creation_date).ge(ts(2023, 10, 1, 12, 34, 56)));
    let pa = recent.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold((0i64, 0i64), |(c, v), (x, y)| (c + x.is_some() as i64, v + y.is_some() as i64));
    let vmax = db.post.with((&db.post.creation_date).ge(ts(2023, 10, 1, 12, 34, 56))).select(&db.post.view_count).fold_flat(i64::MIN, |m, v| m.max(v));
    let top = rel(drain(db.post.with((&db.post.view_count).eq(vmax)).select(Ident::<Post>::new().and(&pa))));
    let users = drain(&ups);
    let right = drain((&pt).cross(&top));
    let v = cross_top(users, |&(_, a)| Reverse(a[1]), right, |_| 0, 10);
    rows(v.iter().map(|&((u, a), (_, ((t, _), (_, (p, (c, n))))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), V::S(t)]);
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//            SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
//            MAX(u.CreationDate) AS AccountCreated
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopQuestions AS (
//     SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 ORDER BY p.Score DESC LIMIT 5),
// RecentEdits AS (
//     SELECT ph.PostId, ph.UserDisplayName, ph.CreationDate AS EditDate, ph.Comment, ph.Text FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) ORDER BY ph.CreationDate DESC LIMIT 10)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.QuestionCount, us.AnswerCount, us.WikiCount, us.AccountCreated,
//        tq.Title AS TopQuestionTitle, tq.Score AS TopQuestionScore, tq.OwnerDisplayName AS TopQuestionOwner,
//        re.UserDisplayName AS EditorDisplayName, re.EditDate, re.Comment AS EditComment, re.Text AS EditedText
// FROM UserStats us LEFT JOIN TopQuestions tq ON us.QuestionCount > 0
// LEFT JOIN RecentEdits re ON us.UserId = CAST(re.UserDisplayName AS int)
// WHERE us.Reputation > 1000 ORDER BY us.PostCount DESC, us.Reputation DESC;
//
// CAST(UserDisplayName AS int) would fail on a name that is not a number;
// the ten recent edits all have a NULL name, which casts to NULL and joins
// nothing.
fn q26210(db: &'static So) -> String {
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt())
        .fold([0i64; 4], |a, t| match t {
            Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
            None => a,
        });
    let tq = rel(top_n(drain(owned(db).with((&db.post.post_type_id).eq(1)).select(&db.post.score)), |&(_, s)| Reverse(s), 5));
    let PostHistory { post_history_type_id, creation_date, user_display_name, .. } = &db.post_history;
    let re = rel(top_n(drain(db.post_history.with(post_history_type_id.is_in([4, 5, 6])).select(creation_date)), |&(_, d)| Reverse(d), 10));
    let pairs: MatSet<(i64, Id<PostHistory>)> = (&re).map(|(h, _)| h).select(user_display_name.map(|n| n.trim().parse::<i64>().unwrap()).and(Ident::<PostHistory>::new())).collect();
    let re_by: HashIdx<i64, (i64, Id<PostHistory>)> = (&pairs).map(|(k, _)| k).inv().collect();
    let head = |u: Id<User>, a: [i64; 4]| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), user_col(db, u, "ucreated")]);
        f
    };
    let edit = |h: Option<Id<PostHistory>>| match h {
        Some(h) => [ostr(user_display_name.get(h)), V::T(creation_date.get(h).unwrap()), ostr(db.post_history.comment.get(h)), ostr(db.post_history.text.get(h))],
        None => [V::Null, V::Null, V::Null, V::Null],
    };
    let tq: HashIdx<(), Id<Post>> = (&tq).map(|_| ()).inv().select((&tq).map(|(p, _)| p)).collect();
    let mut out = Vec::new();
    (&us)
        .and((&us).filt(|a| a[1] > 0).map(|_| ()).select(&tq).opt())
        .and((&db.user.origid).select((&re_by).map(|(_, h)| h)).opt())
        .drive(|u, ((a, p), h)| {
            let mut f = head(u, a);
            match p {
                Some(p) => f.extend(post_fields(db, p, &["title", "score", "owner"])),
                None => f.extend([V::Null, V::Null, V::Null]),
            }
            f.extend(edit(h));
            out.push(row(f))
        });
    rows(out)
}


// WITH TagStatistics AS (
//     SELECT t.TagName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//            SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers, SUM(COALESCE(p.CommentCount, 0)) AS TotalComments
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%') WHERE p.PostTypeId = 1 GROUP BY t.TagName),
// UserReputation AS (
//     SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostActivity AS (
//     SELECT p.Id AS PostId, p.Title, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, MAX(p.CreationDate) AS LastActivityDate
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title)
// SELECT ts.TagName, ts.QuestionCount, ts.TotalViews, ts.TotalAnswers, ts.TotalComments, ur.DisplayName AS ActiveUser,
//        ur.Reputation AS UserReputation, ur.BadgeCount AS UserBadges, pa.Title AS PopularPostTitle, pa.CommentCount AS PopularPostComments,
//        pa.VoteCount AS PopularPostVotes, pa.LastActivityDate AS PopularPostLastActivity
// FROM TagStatistics ts JOIN UserReputation ur ON ur.Reputation = (SELECT MAX(Reputation) FROM UserReputation)
// JOIN PostActivity pa ON pa.CommentCount = (SELECT MAX(CommentCount) FROM PostActivity)
// ORDER BY ts.QuestionCount DESC, ts.TotalViews DESC LIMIT 10;
//
// The WHERE on p.PostTypeId drops the LEFT JOIN's NULL rows: an inner join.
fn q29437(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let Post { view_count, answer_count, comment_count, post_type_id, .. } = &db.post;
    let questions = (&by_tag).map(|(p, _)| p).with(post_type_id.eq(1));
    let ts_ = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&questions).select(view_count.opt().and(answer_count.opt()).and(comment_count)))
        .fold([0i64; 3], |a, ((v, an), c)| [a[0] + v.unwrap_or(0), a[1] + an.unwrap_or(0), a[2] + c]);
    let tq = db.tag.group_by(&db.tag.tag_name).select(&questions).count_distinct();
    let top = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation).fold_flat(i64::MIN, |m, r| m.max(r));
    let bd = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).count_distinct();
    let ur = db.user.with((&db.user.reputation).gt(1000)).with((&db.user.reputation).eq(top)).select((&bd).opt()).map(|n| n.unwrap_or(0));
    let cd = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let vd = db.post.group_by(Ident::<Post>::new()).select(votes_of(db)).count_distinct();
    let dc = db.post.with(post_type_id.is_in([1, 2])).select((&cd).opt()).map(|n| n.unwrap_or(0));
    let dv = db.post.with(post_type_id.is_in([1, 2])).select((&vd).opt()).map(|n| n.unwrap_or(0));
    let cmax = (&dc).fold_flat(i64::MIN, |m, n| m.max(n));
    let users = rel(drain(ur));
    let posts = rel(drain((&dc).filt(|n| n == cmax).and(&dv)));
    let v = drain((&tq).and(&ts_).cross(&users).cross(&posts));
    let v = top_n(v, |&(_, (((q, a), _), _))| (Reverse(q), Reverse(a[0])), 10);
    rows(v.iter().map(|&(((t, _), _), (((q, a), (u, b)), (p, (c, n))))| {
        let mut f = vec![V::S(t), V::I(q), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), title(db, p), V::I(c), V::I(n), V::T(db.post.creation_date.get(p).unwrap())]);
        row(f)
    }))
}

// WITH PostTagCount AS (SELECT p.Id AS PostId, COUNT(*) AS TagCount FROM Posts p WHERE p.PostTypeId = 1 GROUP BY p.Id),
// UserReputation AS (
//     SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAsked
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY u.Id, u.Reputation, u.DisplayName),
// ReputationBracket AS (
//     SELECT CASE WHEN Reputation >= 1000 THEN 'High Reputation' WHEN Reputation >= 100 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory,
//            COUNT(*) AS UserCount
//     FROM UserReputation GROUP BY CASE ... END),
// CloseReasonAnalysis AS (
//     SELECT chr.Id AS CloseReasonId, chr.Name AS CloseReason, COUNT(ph.Id) AS PostClosedCount
//     FROM CloseReasonTypes chr JOIN PostHistory ph ON chr.Id = CAST(ph.Comment AS int) WHERE ph.PostHistoryTypeId = 10 GROUP BY chr.Id, chr.Name),
// TagAnalysis AS (
//     SELECT t.TagName, COUNT(DISTINCT pl.PostId) AS LinkedPostCount FROM Tags t JOIN PostLinks pl ON pl.RelatedPostId = t.WikiPostId GROUP BY t.TagName)
// SELECT r.ReputationCategory, r.UserCount, c.CloseReason, c.PostClosedCount, t.TagName, t.LinkedPostCount, ptc.TagCount
// FROM ReputationBracket r LEFT JOIN CloseReasonAnalysis c ON 1=1 LEFT JOIN TagAnalysis t ON 1=1 LEFT JOIN PostTagCount ptc ON ptc.PostId = 1
// ORDER BY r.ReputationCategory, c.PostClosedCount DESC, t.LinkedPostCount DESC;
//
// None of the three LEFT JOIN conditions mentions the left side, so each is
// a cross join with its right side, or with one NULL row when that is empty.
fn q27620(db: &'static So) -> String {
    let ptc = db.post.with((&db.post.post_type_id).eq(1)).group_by(&db.post.origid).fold(0i64, |n, _| n + 1);
    let asker: MatSet<Id<User>> = db.post.with((&db.post.post_type_id).eq(1)).select(&db.post.owner_user).collect();
    let bracket = |r: i64| if r >= 1000 { "High Reputation" } else if r >= 100 { "Medium Reputation" } else { "Low Reputation" };
    let rb = db.user.with(&asker).group_by((&db.user.reputation).map(bracket)).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let crids: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    let cra = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(comment.map(|c| c.trim().parse::<i64>().unwrap()).select(&crids))
        .fold(0i64, |n, _| n + 1);
    let wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let ta = db.post_link.group_by((&db.post_link.related_post).select(&wiki).select(&db.tag.tag_name)).select(&db.post_link.post_id).count_distinct();
    let c: HashIdx<(), (Id<CloseReasonType>, i64)> = (&cra).map(|_| ()).inv().select(Ident::<CloseReasonType>::new().and(&cra)).collect();
    let t: HashIdx<(), (Str, i64)> = (&ta).map(|_| ()).inv().select(Same::<Str>::new().and(&ta)).collect();
    let p1: HashIdx<(), i64> = whole(&ptc).with(Same::new().eq(1)).select(&ptc).collect();
    let unit = || Same::<Str>::new().map(|_| ());
    let mut out = Vec::new();
    (&rb).and(unit().select(&c).opt()).and(unit().select(&t).opt()).and(unit().select(&p1).opt()).drive(|r, (((n, c), t), p)| {
        let mut f = vec![V::S(r), V::I(n)];
        f.extend(match c {
            Some((c, k)) => [V::S(db.close_reason_type.name.get(c).unwrap()), V::I(k)],
            None => [V::Null, V::Null],
        });
        f.extend(match t {
            Some((t, l)) => [V::S(t), V::I(l)],
            None => [V::Null, V::Null],
        });
        f.push(oint(p));
        out.push(row(f))
    });
    rows(out)
}


// WITH TagStats AS (
//     SELECT Tags.TagName, COUNT(Posts.Id) AS PostCount, SUM(Posts.ViewCount) AS TotalViews, AVG(Users.Reputation) AS AvgUserReputation
//     FROM Tags JOIN Posts ON Tags.ExcerptPostId = Posts.Id JOIN Users ON Posts.OwnerUserId = Users.Id
//     WHERE Posts.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY Tags.TagName),
// UserActivity AS (
//     SELECT Users.DisplayName, COUNT(Posts.Id) AS PostsMade, SUM(Comments.Score) AS TotalCommentScore,
//            SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived, SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvotesReceived
//     FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId LEFT JOIN Comments ON Posts.Id = Comments.PostId
//     LEFT JOIN Votes ON Posts.Id = Votes.PostId GROUP BY Users.DisplayName),
// RecentPostEdits AS (
//     SELECT Posts.Title, Posts.Body, PostHistory.CreationDate AS EditDate, PostHistory.UserDisplayName AS Editor, PostHistory.Comment AS EditComment
//     FROM PostHistory JOIN Posts ON PostHistory.PostId = Posts.Id
//     WHERE PostHistory.PostHistoryTypeId IN (4, 5) AND PostHistory.CreationDate >= CURRENT_DATE - INTERVAL '6 months')
// SELECT ts.TagName, ts.PostCount, ts.TotalViews, ts.AvgUserReputation, ua.DisplayName AS User, ua.PostsMade, ua.TotalCommentScore,
//        ua.UpvotesReceived, ua.DownvotesReceived, rpe.Title AS RecentEditedPostTitle, rpe.EditDate, rpe.Editor, rpe.EditComment
// FROM TagStats AS ts JOIN UserActivity AS ua ON ts.AvgUserReputation > 1000
// LEFT JOIN RecentPostEdits AS rpe ON rpe.EditDate = (SELECT MAX(EditDate) FROM RecentPostEdits)
// ORDER BY ts.PostCount DESC, ts.TotalViews DESC, ua.PostsMade DESC;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024, so TagStats
// is empty.
fn q29608(db: &'static So) -> String {
    let today = current_date();
    let ts_ = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&db.tag.excerpt_post).with((&db.post.creation_date).ge(add_years(today, -1))).select((&db.post.view_count).opt().and((&db.post.owner_user).select(&db.user.reputation))))
        .fold([0i64; 4], |a, (v, r)| [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + r]);
    let ua = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(comments_of(db).select(&db.comment.score).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((c, t)) => [a[0] + 1, a[1] + c.is_some() as i64, a[2] + c.unwrap_or(0), a[3] + (t == Some(2)) as i64, a[4] + (t == Some(3)) as i64],
            None => a,
        });
    let PostHistory { post_history_type_id, creation_date, .. } = &db.post_history;
    let rpe = db.post_history.with(post_history_type_id.is_in([4, 5])).with(creation_date.ge(add_months(today, -6)));
    let emax = rpe.select(creation_date).fold_flat(i64::MIN, |m, d| m.max(d));
    let rpe = db.post_history.with(post_history_type_id.is_in([4, 5])).with(creation_date.ge(add_months(today, -6))).with(creation_date.eq(emax));
    let edits: HashIdx<(), Id<PostHistory>> = rpe.map(|_| ()).inv().collect();
    let mut out = Vec::new();
    (&ts_).filt(|a| a[3] as f64 / a[0] as f64 > 1000.0).cross((&ua).and(Same::<Str>::new().map(|_| ()).select(&edits).opt())).drive(|(t, name), (a, (b, e))| {
        let mut f = vec![V::S(t), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), V::S(name), V::I(b[0]), nullable(b[2], b[1]), V::I(b[3]), V::I(b[4])];
        f.extend(match e {
            Some(h) => [title(db, db.post_history.post.get(h).unwrap()), V::T(creation_date.get(h).unwrap()), ostr(db.post_history.user_display_name.get(h)), ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COUNT(DISTINCT P.Id) AS TotalPosts,
//            SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//            SUM(CASE WHEN P.PostTypeId IN (5, 6) THEN 1 ELSE 0 END) AS TotalWikiPosts, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//            SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, ...
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// PopularTags AS (
//     SELECT T.TagName, COUNT(P.Id) AS UsageCount FROM Tags T JOIN Posts P ON POSITION('>' || T.TagName || '<' IN P.Tags) > 0
//     GROUP BY T.TagName HAVING COUNT(P.Id) > 50),
// ClosedPostReasons AS (SELECT PH.UserId, PH.Comment, COUNT(*) AS CloseReasonCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId, PH.Comment),
// FinalBenchmark AS (
//     SELECT US.UserId, US.DisplayName, US.Reputation, US.TotalPosts, ..., PT.TagName, PT.UsageCount, CPR.CloseReasonCount
//     FROM UserStats US JOIN PopularTags PT ON US.TotalPosts > (SELECT AVG(TotalPosts) FROM UserStats)
//     LEFT JOIN ClosedPostReasons CPR ON US.UserId = CPR.UserId WHERE US.Reputation > 1000 ORDER BY US.Reputation DESC, US.TotalPosts DESC)
// SELECT DisplayName, Reputation, TotalPosts, ..., TagName, UsageCount, CloseReasonCount FROM FinalBenchmark FETCH FIRST 100 ROWS ONLY;
//
// `'>' || name || '<'` would need a tag between a `>` and the next `<`, and
// in `<a><b>` those are adjacent: PopularTags is empty.
fn q26346(db: &'static So) -> String {
    let strs: MatSet<Str> = (&db.post.tags_str).collect();
    let hits: HashIdx<Str, Id<Tag>> = (&strs).select_where((&db.tag.tag_name).inv(), |s: Str, n: Str| s.contains(&format!(">{n}<"))).collect();
    let hit_by_tag: HashIdx<Id<Tag>, Str> = (&hits).inv().collect();
    let by_str: HashIdx<Str, Id<Post>> = (&db.post.tags_str).inv().collect();
    let pt = db.tag.group_by(&db.tag.tag_name).select((&hit_by_tag).select(&by_str)).fold(0i64, |n, _| n + 1);
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 8], |a, (p, c)| {
            let (t, v) = p.map_or((None, None), |(t, v)| (Some(t), v));
            [
                a[0] + (t == Some(1)) as i64,
                a[1] + (t == Some(2)) as i64,
                a[2] + matches!(t, Some(5 | 6)) as i64,
                a[3] + (v == Some(2)) as i64,
                a[4] + (v == Some(3)) as i64,
                a[5] + (c == Some(1)) as i64,
                a[6] + (c == Some(2)) as i64,
                a[7] + (c == Some(3)) as i64,
            ]
        });
    let dpc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let dp = db.user.select((&dpc).opt()).map(|n| n.unwrap_or(0));
    let (sum, n) = (&dp).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let cprs = db
        .post_history
        .with((&db.post_history.post_history_type_id).eq(10))
        .group_by((&db.post_history.user).and((&db.post_history.comment).opt()))
        .fold(0i64, |n, _| n + 1);
    let keys: MatSet<(Id<User>, Option<Str>)> = whole(&cprs).collect();
    let by_user: HashIdx<Id<User>, (Id<User>, Option<Str>)> = (&keys).map(|(u, _)| u).inv().collect();
    let v = drain(
        (&us)
            .and((&dp).filt(move |x| x as f64 > sum as f64 / n as f64))
            .and((&by_user).select(&cprs).opt())
            .cross((&pt).filt(|k| k > 50)),
    );
    let v = top_n(v, |&((u, _), ((_, d), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(d)), 100);
    rows(v.iter().map(|&((u, t), (((a, d), c), k))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(d));
        f.extend(a.iter().map(|&x| V::I(x)));
        f.extend([V::S(t), V::I(k), oint(c)]);
        row(f)
    }))
}


// WITH PostStatistics AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AvgScore,
//            AVG(CASE WHEN pt.Id = 1 THEN p.AnswerCount ELSE NULL END) AS AvgAnswersPerQuestion
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// BadgeStatistics AS (SELECT b.Class, COUNT(DISTINCT b.UserId) AS UserCount FROM Badges b WHERE b.Class IN (1, 2, 3) GROUP BY b.Class)
// SELECT ps.PostType, ps.PostCount, ps.AvgScore, ps.AvgAnswersPerQuestion, COALESCE(bs.UserCount, 0) AS UsersWithBadges
// FROM PostStatistics ps LEFT JOIN BadgeStatistics bs ON ps.PostType = 'Question' ORDER BY ps.PostType;
fn q12634(db: &'static So) -> String {
    let Post { score, answer_count, post_type_id, .. } = &db.post;
    let ps = db
        .post
        .group_by(ptype_name(db))
        .select(score.and(post_type_id).and(answer_count.opt()))
        .fold([0i64; 4], |a, ((s, t), an)| {
            let q = if t == 1 { an } else { None };
            [a[0] + 1, a[1] + s, a[2] + q.is_some() as i64, a[3] + q.unwrap_or(0)]
        });
    let bs = db.badge.with((&db.badge.class).is_in([1, 2, 3])).group_by(&db.badge.class).select(&db.badge.user_id).count_distinct();
    let bs: HashIdx<(), i64> = whole(&bs).select(&bs).collect();
    let mut out = Vec::new();
    (&ps).and(Same::<Str>::new().eq("Question").map(|_| ()).select(&bs).opt()).drive(|t, (a, b)| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(b.unwrap_or(0))]))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT PostTypeId, COUNT(*) AS TotalPosts, SUM(CASE WHEN AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers
//     FROM Posts GROUP BY PostTypeId),
// UserStats AS (
//     SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostsCount, SUM(V.BountyAmount) AS TotalBounties
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
//     GROUP BY U.Id, U.Reputation)
// SELECT PCT.PostTypeId, PCT.TotalPosts, PCT.TotalAcceptedAnswers, US.UserId, US.Reputation, US.PostsCount, US.TotalBounties
// FROM PostCounts PCT JOIN UserStats US ON US.PostsCount > 0 ORDER BY PCT.TotalPosts DESC, US.Reputation DESC;
fn q14848(db: &'static So) -> String {
    let pc = db.post.group_by(&db.post.post_type_id).select((&db.post.accepted_answer_id).opt()).fold((0i64, 0i64), |(n, a), x| (n + 1, a + x.is_some() as i64));
    let b8: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(8)).select(&db.vote.post).inv().collect();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&b8).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64), |(n, s), v| {
            let v = v.flatten();
            (n + v.is_some() as i64, s + v.unwrap_or(0))
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let mut out = Vec::new();
    (&pc).cross((&dp).filt(|n| n > 0).and(&us)).drive(|(t, u), ((n, a), (d, (bn, b)))| {
        let mut f = vec![V::I(t), V::I(n), V::I(a)];
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(d), nullable(b, bn)]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostStats AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.ViewCount) AS AvgViewCount, SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
//     FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY pt.Name),
// UserStats AS (
//     SELECT u.Id AS UserId, AVG(u.Reputation) AS AvgReputation, COUNT(b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT ps.PostType, ps.PostCount, ps.AvgViewCount, ps.TotalVotes, us.AvgReputation, us.TotalBadges
// FROM PostStats ps JOIN UserStats us ON us.UserId IN (SELECT DISTINCT OwnerUserId FROM Posts WHERE OwnerUserId IS NOT NULL)
// ORDER BY ps.PostType;
fn q12956(db: &'static So) -> String {
    let ps = db
        .post
        .group_by(ptype_name(db))
        .select((&db.post.view_count).opt().and(votes_of(db).opt()))
        .fold([0i64; 4], |a, (w, v)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + v.is_some() as i64]);
    let owners: MatSet<i64> = (&db.post.owner_user_id).collect();
    let us = db
        .user
        .with((&db.user.origid).with(&owners))
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (r, b)| [a[0] + 1, a[1] + r, a[2] + b.is_some() as i64]);
    let mut out = Vec::new();
    (&ps).cross(&us).drive(|(t, _), (a, b)| {
        out.push(row(vec![V::S(t), V::I(a[0]), avg(a[2], a[1]), V::I(a[3]), avg(b[1], b[0]), V::I(b[2])]))
    });
    rows(out)
}

// SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COALESCE(A.AcceptedAnswerId, 0) AS AcceptedAnswerId,
//        U.DisplayName AS OwnerDisplayName, U.Reputation AS OwnerReputation, C.CommentCount, B.BadgeCount, T.TagName
// FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON U.Id = B.UserId
// LEFT JOIN (SELECT PostId, STRING_AGG(TagName, ', ') AS TagName FROM PostLinks PL JOIN Tags T ON PL.RelatedPostId = T.Id
//            GROUP BY PL.PostId) T ON P.Id = T.PostId
// LEFT JOIN (SELECT AcceptedAnswerId FROM Posts WHERE PostTypeId = 1) A ON P.Id = A.AcceptedAnswerId
// WHERE P.PostTypeId = 1 ORDER BY P.CreationDate DESC LIMIT 100;
//
// The STRING_AGG has no ORDER BY; none of the hundred posts has a link to a
// post whose Id is also a tag's, so every value is NULL and the order never
// shows.
fn q14888(db: &'static So) -> String {
    let cc = db.comment.group_by(&db.comment.post).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let tids: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let tn = db.post_link.group_by(&db.post_link.post).select((&db.post_link.related_post_id).select(&tids).select(&db.tag.tag_name)).buf_fold(|v| {
        let mut v: Vec<Str> = v.into_iter().collect();
        v.sort_unstable();
        Box::leak(v.join(", ").into_boxed_str()) as Str
    });
    let acc: HashIdx<Id<Post>, Id<Post>> = db.post.with((&db.post.post_type_id).eq(1)).select(&db.post.accepted_answer).inv().collect();
    let q = owned(db)
        .with((&db.post.post_type_id).eq(1))
        .select(Ident::<Post>::new().and((&acc).opt()).and((&cc).opt()).and((&tn).opt()).and((&db.post.owner_user).select(Ident::<User>::new().and((&bc).opt()))));
    let v = top_n(drain(q), |&(p, _)| Reverse(db.post.creation_date.get(p).unwrap()), 100);
    rows(v.iter().map(|&(_, ((((p, a), c), t), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(a.map_or(0, |a| db.post.origid.get(a).unwrap())));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([oint(c), oint(b), ostr(t)]);
        row(f)
    }))
}

// WITH UserStats AS (
//     SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostsCount,
//            SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// PostStats AS (
//     SELECT p.PostTypeId, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AvgScore, AVG(p.ViewCount) AS AvgViews, COUNT(DISTINCT p.OwnerUserId) AS UniqueOwners
//     FROM Posts p GROUP BY p.PostTypeId)
// SELECT us.UserId, us.Reputation, us.BadgeCount, us.PostsCount, us.TotalViews, us.TotalScore, ps.PostTypeId, ps.TotalPosts, ps.AvgScore,
//        ps.AvgViews, ps.UniqueOwners
// FROM UserStats us JOIN PostStats ps ON us.PostsCount > 0 ORDER BY us.Reputation DESC, ps.TotalPosts DESC;
fn q10209(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt().and(&db.post.score)).opt()))
        .fold([0i64; 5], |a, (b, p)| match p {
            Some((v, s)) => [a[0] + b.is_some() as i64, a[1] + 1, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0), a[4] + s],
            None => [a[0] + b.is_some() as i64, a[1], a[2], a[3], a[4]],
        });
    let ps = db
        .post
        .group_by(&db.post.post_type_id)
        .select((&db.post.score).and((&db.post.view_count).opt()))
        .fold([0i64; 4], |a, (s, v)| [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]);
    let uo = db.post.group_by(&db.post.post_type_id).select(&db.post.owner_user_id).count_distinct();
    let mut out = Vec::new();
    (&us).filt(|a| a[1] > 0).cross((&ps).and((&uo).opt())).drive(|(u, t), (a, (b, o))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4]), V::I(t), V::I(b[0]), avg(b[1], b[0]), avg(b[3], b[2]), V::I(o.unwrap_or(0))]);
        out.push(row(f))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, COUNT(CASE WHEN p.Score > 0 THEN 1 END) AS TotalQuestions,
//            COUNT(CASE WHEN p.Score IS NOT NULL AND p.AcceptedAnswerId IS NOT NULL THEN 1 END) AS TotalAcceptedAnswers
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name),
// UserActivity AS (
//     SELECT u.DisplayName, COUNT(p.Id) AS PostsCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
//     GROUP BY u.DisplayName)
// SELECT pc.PostType, pc.TotalPosts, pc.TotalQuestions, pc.TotalAcceptedAnswers, ua.DisplayName, ua.PostsCount, ua.TotalBounty
// FROM PostCounts pc LEFT JOIN UserActivity ua ON ua.PostsCount > 0 ORDER BY pc.TotalPosts DESC, ua.PostsCount DESC;
fn q14034(db: &'static So) -> String {
    let pc = db
        .post
        .group_by(ptype_name(db))
        .select((&db.post.score).and((&db.post.accepted_answer_id).opt()))
        .fold([0i64; 3], |a, (s, x)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + x.is_some() as i64]);
    let b89: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).is_in([8, 9])).select(&db.vote.post).inv().collect();
    let ua = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select((&b89).select((&db.vote.bounty_amount).opt()).opt()).opt())
        .fold((0i64, 0i64), |(n, b), p| match p {
            Some(v) => (n + 1, b + v.flatten().unwrap_or(0)),
            None => (n, b),
        });
    let ua = (&ua).filt(|(n, _)| n > 0);
    let ua: HashIdx<(), (Str, (i64, i64))> = (&ua).map(|_| ()).inv().select(Same::<Str>::new().and(&ua)).collect();
    let mut out = Vec::new();
    (&pc).and(Same::<Str>::new().map(|_| ()).select(&ua).opt()).drive(|t, (a, x)| {
        let mut f = vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(match x {
            Some((name, (n, b))) => [V::S(name), V::I(n), V::I(b)],
            None => [V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH PostCounts AS (
//     SELECT p.PostTypeId, COUNT(*) AS TotalPosts,
//            SUM(CASE WHEN p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' THEN 1 ELSE 0 END) AS RecentPosts
//     FROM Posts p GROUP BY p.PostTypeId),
// UserActivity AS (
//     SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts,
//            SUM(CASE WHEN p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' THEN 1 ELSE 0 END) AS RecentPosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation)
// SELECT pt.Name AS PostType, pc.TotalPosts, pc.RecentPosts, ua.UserId, ua.Reputation, ua.TotalPosts AS UserTotalPosts, ua.RecentPosts AS UserRecentPosts
// FROM PostCounts pc JOIN PostTypes pt ON pc.PostTypeId = pt.Id LEFT JOIN UserActivity ua ON ua.TotalPosts > 0
// ORDER BY pc.TotalPosts DESC, ua.Reputation DESC;
fn q14175(db: &'static So) -> String {
    let since = add_months(ts(2024, 10, 1, 12, 34, 56), -1);
    let pc = db.post.group_by(&db.post.post_type).select(&db.post.creation_date).fold((0i64, 0i64), |(n, r), d| (n + 1, r + (d >= since) as i64));
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.creation_date)).fold((0i64, 0i64), |(n, r), d| (n + 1, r + (d >= since) as i64));
    let ua: HashIdx<(), (Id<User>, (i64, i64))> = (&ua).map(|_| ()).inv().select(Ident::<User>::new().and(&ua)).collect();
    let mut out = Vec::new();
    (&pc).and(Ident::<PostType>::new().map(|_| ()).select(&ua).opt()).drive(|t, ((n, r), x)| {
        let mut f = vec![tname(db, t), V::I(n), V::I(r)];
        f.extend(match x {
            Some((u, (m, q))) => [user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(m), V::I(q)],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        out.push(row(f))
    });
    rows(out)
}

// WITH UserStats AS (
//     SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(V.BountyAmount) AS TotalBounties
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TagStats AS (
//     SELECT T.Id AS TagId, T.TagName, COUNT(P.Id) AS PostsCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.Id, T.TagName)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalBounties, T.TagId, T.TagName, T.PostsCount, T.TotalViews
// FROM UserStats U JOIN TagStats T ON T.PostsCount > 0 ORDER BY U.TotalPosts DESC, T.TotalViews DESC LIMIT 100;
fn q13404(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())))
        .fold([0i64; 4], |a, (t, v)| {
            let v = v.flatten();
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)]
        });
    let dpc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).count_distinct();
    let dp = db.user.select((&dpc).opt()).map(|n| n.unwrap_or(0));
    let ts_ = tag_stats(db);
    let users = drain((&dp).and((&us).opt()));
    let tags = drain((&ts_).filt(|a| a[0] > 0));
    let v = cross_top(users, |&(_, (n, _))| Reverse(n), tags, |&(_, b)| Reverse(b[2]), 100);
    rows(v.iter().map(|&((u, (n, a)), (t, b))| {
        let a = a.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(db.tag.origid.get(t).unwrap()), V::S(db.tag.tag_name.get(t).unwrap()), V::I(b[0]), V::I(b[2])]);
        row(f)
    }))
}

// WITH PostStats AS (
//     SELECT DATE_TRUNC('month', CreationDate) AS Month, COUNT(*) AS TotalPosts, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//            SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
//     FROM Posts GROUP BY DATE_TRUNC('month', CreationDate)),
// VoteStats AS (SELECT DATE_TRUNC('month', CreationDate) AS Month, COUNT(*) AS TotalVotes FROM Votes GROUP BY DATE_TRUNC('month', CreationDate)),
// UserStats AS (SELECT DATE_TRUNC('month', CreationDate) AS Month, COUNT(*) AS TotalUsers FROM Users GROUP BY DATE_TRUNC('month', CreationDate))
// SELECT COALESCE(p.Month, v.Month, u.Month) AS Month, COALESCE(p.TotalPosts, 0) AS TotalPosts, COALESCE(p.TotalQuestions, 0) AS TotalQuestions,
//        COALESCE(p.TotalAnswers, 0) AS TotalAnswers, COALESCE(v.TotalVotes, 0) AS TotalVotes, COALESCE(u.TotalUsers, 0) AS TotalUsers
// FROM PostStats p FULL OUTER JOIN VoteStats v ON p.Month = v.Month FULL OUTER JOIN UserStats u ON p.Month = u.Month
// ORDER BY Month;
//
// Both joins are on p.Month, so the rows are the post months with whatever
// votes and users they have, and the vote-only and user-only months apart.
fn q12241(db: &'static So) -> String {
    let pm = db.post.group_by((&db.post.creation_date).map(trunc_month)).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let vm = db.vote.group_by((&db.vote.creation_date).map(trunc_month)).fold(0i64, |n, _| n + 1);
    let um = db.user.group_by((&db.user.creation_date).map(trunc_month)).fold(0i64, |n, _| n + 1);
    let mut out = Vec::new();
    (&pm).and((&vm).opt()).and((&um).opt()).drive(|m, ((a, v), u)| out.push(row(vec![V::T(m), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(v.unwrap_or(0)), V::I(u.unwrap_or(0))])));
    (&vm).minus(&pm).drive(|m, v| out.push(row(vec![V::T(m), V::I(0), V::I(0), V::I(0), V::I(v), V::I(0)])));
    (&um).minus(&pm).drive(|m, u| out.push(row(vec![V::T(m), V::I(0), V::I(0), V::I(0), V::I(0), V::I(u)])));
    rows(out)
}

// WITH PostStats AS (SELECT DATE_TRUNC('month', CreationDate) AS Month, COUNT(*) AS TotalPosts, ... TotalQuestions, ... TotalAnswers FROM Posts GROUP BY Month),
// CommentStats AS (SELECT DATE_TRUNC('month', CreationDate) AS Month, COUNT(*) AS TotalComments FROM Comments GROUP BY Month),
// UserStats AS (SELECT DATE_TRUNC('month', CreationDate) AS Month, COUNT(*) AS TotalUsers FROM Users GROUP BY Month)
// SELECT COALESCE(p.Month, c.Month, u.Month) AS Month, COALESCE(TotalPosts, 0) AS TotalPosts, COALESCE(TotalQuestions, 0) AS TotalQuestions,
//        COALESCE(TotalAnswers, 0) AS TotalAnswers, COALESCE(TotalComments, 0) AS TotalComments, COALESCE(TotalUsers, 0) AS TotalUsers
// FROM PostStats p FULL OUTER JOIN CommentStats c ON p.Month = c.Month FULL OUTER JOIN UserStats u ON COALESCE(p.Month, c.Month) = u.Month
// ORDER BY Month;
//
// Every join is on the one month, so the rows are the union of the three
// month sets.
fn q14540(db: &'static So) -> String {
    let pm = db.post.group_by((&db.post.creation_date).map(trunc_month)).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let cm = db.comment.group_by((&db.comment.creation_date).map(trunc_month)).fold(0i64, |n, _| n + 1);
    let um = db.user.group_by((&db.user.creation_date).map(trunc_month)).fold(0i64, |n, _| n + 1);
    let months: MatSet<i64> = (&pm).map(|_| ()).inv().union((&cm).map(|_| ()).inv()).union((&um).map(|_| ()).inv()).collect();
    let mut out = Vec::new();
    (&months).select(Same::new().and((&pm).opt()).and((&cm).opt()).and((&um).opt())).drive(|_, (((m, a), c), u)| {
        let a = a.unwrap_or([0; 3]);
        out.push(row(vec![V::T(m), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c.unwrap_or(0)), V::I(u.unwrap_or(0))]))
    });
    rows(out)
}

pub static ENTRIES: &[harness::Entry] = &[
    ("13405", q13405),
    ("11680", q11680),
    ("10579", q10579),
    ("25217", q25217),
    ("12369", q12369),
    ("25883", q25883),
    ("12637", q12637),
    ("12758", q12758),
    ("6303", q6303),
    ("11089", q11089),
    ("12773", q12773),
    ("11531", q11531),
    ("28894", q28894),
    ("9861", q9861),
    ("26066", q26066),
    ("29930", q29930),
    ("16303", q16303),
    ("12968", q12968),
    ("14471", q14471),
    ("13649", q13649),
    ("14739", q14739),
    ("13397", q13397),
    ("12011", q12011),
    ("11055", q11055),
    ("12584", q12584),
    ("14201", q14201),
    ("14396", q14396),
    ("10830", q10830),
    ("25602", q25602),
    ("14230", q14230),
    ("14873", q14873),
    ("12528", q12528),
    ("10945", q10945),
    ("11761", q11761),
    ("14906", q14906),
    ("14958", q14958),
    ("11471", q11471),
    ("13843", q13843),
    ("14905", q14905),
    ("13469", q13469),
    ("14422", q14422),
    ("14802", q14802),
    ("12401", q12401),
    ("13849", q13849),
    ("12465", q12465),
    ("13477", q13477),
    ("12026", q12026),
    ("11664", q11664),
    ("12781", q12781),
    ("12635", q12635),
    ("10989", q10989),
    ("14894", q14894),
    ("11299", q11299),
    ("12146", q12146),
    ("27823", q27823),
    ("10998", q10998),
    ("14895", q14895),
    ("29284", q29284),
    ("14171", q14171),
    ("10949", q10949),
    ("13075", q13075),
    ("10533", q10533),
    ("5339", q5339),
    ("14611", q14611),
    ("10928", q10928),
    ("14393", q14393),
    ("5785", q5785),
    ("11080", q11080),
    ("12697", q12697),
    ("9648", q9648),
    ("11446", q11446),
    ("11621", q11621),
    ("12276", q12276),
    ("12939", q12939),
    ("9178", q9178),
    ("7217", q7217),
    ("28696", q28696),
    ("28469", q28469),
    ("6006", q6006),
    ("8342", q8342),
    ("5844", q5844),
    ("32144", q32144),
    ("27925", q27925),
    ("11001", q11001),
    ("26400", q26400),
    ("26210", q26210),
    ("29437", q29437),
    ("27620", q27620),
    ("29608", q29608),
    ("26346", q26346),
    ("12634", q12634),
    ("14848", q14848),
    ("12956", q12956),
    ("14888", q14888),
    ("10209", q10209),
    ("14034", q14034),
    ("14175", q14175),
    ("13404", q13404),
    ("12241", q12241),
    ("14540", q14540),
];
