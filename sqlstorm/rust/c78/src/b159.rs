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

fn tag_name_stats(db: &'static So) -> Fold<Str, [i64; 7]> {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    db.tag
        .group_by(&db.tag.tag_name)
        .select((&by_tag).map(|(p, _)| p).select((&db.post.view_count).opt().and(&db.post.score).and(&db.post.post_type_id)).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((v, s), t)) => [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s, a[4] + (t == 1) as i64, a[5] + (t == 2) as i64, a[6] + 1],
            None => [a[0], a[1], a[2], a[3], a[4], a[5], a[6] + 1],
        })
}

fn ntile5<O, R>(g: &[(O, R)], out: &mut Vec<i64>) {
    out.extend((0..g.len()).map(|i| ntile(i, g.len(), 5)));
}

fn ntile(i: usize, n: usize, k: usize) -> i64 {
    let (q, r) = (n / k, n % k);
    (if i < r * (q + 1) { i / (q + 1) } else { r + (i - r * (q + 1)) / q }) as i64 + 1
}

fn post_comment_count<Q: Drive<R = Id<Post>>>(db: &'static So, posts: Q) -> Fold<Id<Post>, i64> {
    posts.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64)
}

fn post_updown<Q: Drive<R = Id<Post>>>(db: &'static So, posts: Q) -> Fold<Id<Post>, [i64; 2]> {
    posts.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64])
}

// WITH RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, U.DisplayName AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS TotalComments,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS TotalUpVotes,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS TotalDownVotes,
//        ROW_NUMBER() OVER (ORDER BY P.CreationDate DESC) AS RowNum
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')),
// PostStatistics AS (SELECT RP.PostId, RP.Title, RP.Body, RP.CreationDate, RP.OwnerDisplayName, RP.TotalComments, RP.TotalUpVotes, RP.TotalDownVotes,
//        (RP.TotalUpVotes - RP.TotalDownVotes) AS NetVotes, CASE WHEN RP.TotalComments > 5 THEN 'Hot' WHEN RP.TotalUpVotes > 10 THEN 'Trending' ELSE 'New' END AS PostCategory,
//        ROW_NUMBER() OVER (ORDER BY RP.CreationDate DESC) AS RowNum FROM RecentPosts RP)
// SELECT PS.PostId, PS.Title, PS.Body, PS.CreationDate, PS.OwnerDisplayName, PS.TotalComments, PS.TotalUpVotes, PS.TotalDownVotes, PS.NetVotes, PS.PostCategory,
//        PH.UserDisplayName AS LastEditor, PH.CreationDate AS LastEditDate, PHT.Name AS PostHistoryTypeName
// FROM PostStatistics PS LEFT JOIN PostHistory PH ON PS.PostId = PH.PostId LEFT JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// WHERE PS.RowNum <= 50 ORDER BY PS.CreationDate DESC;
fn q5065(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(creation_date));
    let v = top_n(v, |&(p, d)| (Reverse(d), p), 50);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&post_comment_count(db, &tp)).and(&post_updown(db, &tp)).and(history_of(db).opt()));
    rows(v.into_iter().map(|(p, ((c, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::S(if c > 5 { "Hot" } else if a[0] > 10 { "Trending" } else { "New" })]);
        f.extend(match h {
            Some(h) => [ostr(db.post_history.user_display_name.get(h)), V::T(db.post_history.creation_date.get(h).unwrap()), V::S(htype_name(db).get(h).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, NTILE(5) OVER (ORDER BY U.Reputation DESC) AS ReputationBucket FROM Users U),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId IN (1, 2) THEN P.Score ELSE 0 END) AS TotalScore
//     FROM Posts P GROUP BY P.OwnerUserId),
// VoteSummary AS (SELECT V.UserId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes V GROUP BY V.UserId),
// TopUsers AS (SELECT UR.UserId, UR.DisplayName, PS.PostCount, PS.QuestionCount, PS.AnswerCount, COALESCE(VS.UpVotes, 0) AS UpVotes, COALESCE(VS.DownVotes, 0) AS DownVotes,
//        UR.ReputationBucket FROM UserReputation UR LEFT JOIN PostStats PS ON UR.UserId = PS.OwnerUserId LEFT JOIN VoteSummary VS ON UR.UserId = VS.UserId WHERE UR.Reputation > 50),
// FilteredUsers AS (SELECT *, RANK() OVER (PARTITION BY ReputationBucket ORDER BY UpVotes DESC) AS RankByVotes FROM TopUsers WHERE UpVotes - DownVotes > 0)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, ReputationBucket, RankByVotes
// FROM FilteredUsers WHERE RankByVotes <= 5 ORDER BY ReputationBucket, RankByVotes;
fn q20337(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let ur = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(ntile5, |(u, r)| (Reverse(r), u), asc);
    type B = ((Id<User>, i64), i64);
    let urs: MatSet<B> = (&ur).collect();
    let bucket: HashIdx<Id<User>, i64> = (&urs).map(|((u, _), _): B| u).inv().select((&urs).map(|(_, b): B| b)).collect();
    let ps = db.post.group_by(owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type T = ((i64, Option<[i64; 3]>), [i64; 2]);
    let tu: HashIdx<Id<User>, T> = db
        .user
        .with((&db.user.reputation).gt(50))
        .select((&bucket).and((&ps).opt()).and((&vs).opt().map(|o: Option<[i64; 2]>| o.unwrap_or([0, 0]))))
        .filt(|(_, a): T| a[0] - a[1] > 0)
        .collect();
    let w = db.user.with(&tu).group_by((&tu).map(|((b, _), _): T| b)).select(Ident::<User>::new().and(&tu)).window(rank, |(_, (_, a)): (Id<User>, T)| Reverse(a[0]), asc);
    let v = top_n(drain((&w).filt(|(_, r)| r <= 5)), |&(b, (_, r))| (b, r), 0);
    rows(v.into_iter().map(|(b, ((u, ((_, p), a)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(match p {
            Some(p) => [V::I(p[0]), V::I(p[1]), V::I(p[2])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH PostDetail AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.LastEditDate, ph.CreationDate AS HistoryEditDate,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (4, 5, 6) WHERE p.PostTypeId = 1),
// AggregatedPosts AS (SELECT pd.PostId, pd.Title, pd.OwnerDisplayName, pd.CreationDate, COALESCE(MAX(pd.HistoryEditDate), pd.CreationDate) AS RecentEdit,
//        SUM(pd.CommentCount) AS TotalComments, SUM(pd.UpVoteCount) AS TotalUpVotes, SUM(pd.DownVoteCount) AS TotalDownVotes
//     FROM PostDetail pd GROUP BY pd.PostId, pd.Title, pd.OwnerDisplayName, pd.CreationDate),
// RankedPosts AS (SELECT *, RANK() OVER (ORDER BY TotalUpVotes DESC, TotalComments DESC) AS Rank FROM AggregatedPosts)
// SELECT rp.Rank, rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.RecentEdit, rp.TotalComments, rp.TotalUpVotes, rp.TotalDownVotes,
//        CASE WHEN rp.TotalUpVotes >= 20 THEN 'Popular' WHEN rp.TotalUpVotes BETWEEN 10 AND 19 THEN 'Trending' ELSE 'New' END AS PopularityStatus
// FROM RankedPosts rp WHERE rp.Rank <= 100 ORDER BY rp.Rank;
fn q26087(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))).select(&db.post_history.creation_date);
    let (cc, ud) = (post_comment_count(db, qs()), post_updown(db, qs()));
    let per = (&cc).and(&ud);
    let agg = qs().group_by(Ident::<Post>::new()).select(edits.opt().and(&per)).fold([0, 0, 0, i64::MIN], |a, (d, (c, u))| [a[0] + c, a[1] + u[0], a[2] + u[1], d.map_or(a[3], |d| a[3].max(d))]);
    let w = whole(&agg).select(Ident::<Post>::new().and(&agg)).window(rank, |(_, a)| (Reverse(a[1]), Reverse(a[0])), asc);
    let v = drain((&w).filt(|(_, r)| r <= 100));
    rows(v.into_iter().map(|(_, ((p, a), r))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["id", "title", "owner", "created"]));
        f.extend([V::T(if a[3] == i64::MIN { creation_date.get(p).unwrap() } else { a[3] }), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if a[1] >= 20 { "Popular" } else if a[1] >= 10 && a[1] <= 19 { "Trending" } else { "New" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//     GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, us.TotalVotes, us.UpVotes, us.DownVotes, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u JOIN UserVoteStats us ON u.Id = us.UserId WHERE u.Reputation > 1000 ORDER BY ReputationRank LIMIT 10)
// SELECT ps.PostId, ps.Title, u.DisplayName AS PostOwner, ps.CommentCount, ps.UpVotes AS PostUpVotes, ps.DownVotes AS PostDownVotes, tu.UserId AS TopUserId,
//        tu.DisplayName AS TopUserDisplayName, tu.Reputation AS TopUserReputation, tu.TotalVotes AS TopUserTotalVotes
// FROM PostStats ps JOIN Users u ON ps.OwnerUserId = u.Id JOIN TopUsers tu ON ps.UpVotes > 5 OR ps.DownVotes > 5
// ORDER BY ps.CommentCount DESC, ps.UpVotes - ps.DownVotes DESC LIMIT 20;
//
// The ON clause names only ps, so the qualifying posts are crossed with the ten top users.
fn q5172(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let uv = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let ps = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = Vec::new();
    (&ps).filt(|a| a[1] > 5 || a[2] > 5).cross(&uv).drive(|(p, u), (a, n)| v.push((p, a, u, n)));
    let v = top_n(v, |&(p, a, u, _)| (Reverse(a[0]), Reverse(a[1] - a[2]), p, db.user.origid.get(u).unwrap()), 20);
    rows(v.into_iter().map(|(p, a, u, n)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.push(V::I(n));
        row(f)
    }))
}

fn user_badge_classes(db: &'static So) -> Fold<Id<User>, [i64; 4]> {
    db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    })
}

// WITH RecursiveCTE AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1),
// RecentQuestions AS (SELECT PostId, Title, CreationDate, ViewCount FROM RecursiveCTE WHERE rn <= 10),
// AnsweredQuestions AS (SELECT rq.PostId, rq.Title, rq.CreationDate, rq.ViewCount, COUNT(a.Id) AS AnswerCount FROM RecentQuestions rq LEFT JOIN Posts a ON rq.PostId = a.ParentId
//     GROUP BY rq.PostId, rq.Title, rq.CreationDate, rq.ViewCount),
// UserWithBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// FinalResults AS (SELECT aq.PostId, aq.Title, aq.CreationDate, aq.ViewCount, aq.AnswerCount, ub.DisplayName, ub.BadgeCount
//     FROM AnsweredQuestions aq JOIN RecentQuestions rq ON aq.PostId = rq.PostId JOIN Users u ON rq.PostId = u.Id LEFT JOIN UserWithBadges ub ON u.Id = ub.UserId)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.ViewCount, fr.AnswerCount, fr.DisplayName, COALESCE(fr.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN fr.AnswerCount > 0 THEN 'Answered' ELSE 'Not Answered' END AS AnswerStatus
// FROM FinalResults fr WHERE fr.ViewCount > 100 ORDER BY fr.CreationDate DESC;
//
// `JOIN Users u ON rq.PostId = u.Id` compares a post id with a user id, so it goes through the raw ids.
fn q31570(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, view_count, origid, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rq: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ac = (&rq).with(view_count.gt(100)).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let v = drain((&ac).and(origid.select(&uid).select(Ident::<User>::new().and(&bc))));
    rows(v.into_iter().map(|(p, (n, (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(n), user_col(db, u, "name"), V::I(b), V::S(if n > 0 { "Answered" } else { "Not Answered" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// MostActiveUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, WikiCount, Upvotes, Downvotes, RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserStats),
// TopBadgeHolders AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldCount, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// FinalStats AS (SELECT MA.UserId, MA.DisplayName, MA.Reputation, MA.PostCount, MA.QuestionCount, MA.AnswerCount, MA.WikiCount, MA.Upvotes, MA.Downvotes,
//        TB.BadgeCount, TB.GoldCount, TB.SilverCount, TB.BronzeCount FROM MostActiveUsers MA LEFT JOIN TopBadgeHolders TB ON MA.UserId = TB.UserId WHERE MA.PostRank <= 10)
// SELECT * FROM FinalStats ORDER BY Reputation DESC, PostCount DESC;
fn q7144(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let us = || db.user.with((&db.user.reputation).gt(1000));
    let pv = Ident::<Post>::new().and(post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt());
    let s = us().group_by(Ident::<User>::new()).select(posts_of(db).select(pv).opt()).fold([0i64; 5], |a, x| match x {
        Some(((_, t), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
        None => a,
    });
    let dp = us().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ub = user_badge_classes(db);
    let w = whole(&dp).select(Ident::<User>::new().and((&dp).and(&s).and(&ub))).window(rank, |(_, ((n, _), _))| Reverse(n), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, ((u, ((n, a), b)), _))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverCount, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AvgScore FROM Posts P GROUP BY P.OwnerUserId),
// RankedPosts AS (SELECT PS.OwnerUserId, PS.PostCount, PS.TotalViews, PS.AvgScore, ROW_NUMBER() OVER (ORDER BY PS.PostCount DESC) AS PostRank FROM PostStats PS),
// UserInfo AS (SELECT UB.UserId, UB.DisplayName, COALESCE(RP.PostCount, 0) AS PostCount, COALESCE(RP.TotalViews, 0) AS TotalViews, COALESCE(RP.AvgScore, 0) AS AvgScore,
//        UB.BadgeCount, UB.GoldCount, UB.SilverCount, UB.BronzeCount FROM UserBadges UB LEFT JOIN RankedPosts RP ON UB.UserId = RP.OwnerUserId)
// SELECT UI.DisplayName, UI.PostCount, UI.TotalViews, UI.AvgScore, (CASE WHEN UI.BadgeCount > 10 THEN 'Expert' WHEN UI.BadgeCount > 5 THEN 'Intermediate' ELSE 'Novice' END) AS ExperienceLevel,
//        (CASE WHEN UI.TotalViews > 10000 THEN 'Highly Visible' ELSE 'Moderate Visibility' END) AS VisibilityStatus
// FROM UserInfo UI WHERE UI.PostCount > 0 ORDER BY UI.PostCount DESC, UI.TotalViews DESC;
fn q2246(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let ub = user_badge_classes(db);
    let v = drain((&ub).and(&ps));
    rows(v.into_iter().map(|(u, (b, a))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[2]),
            avg(a[1], a[0]),
            V::S(if b[0] > 10 { "Expert" } else if b[0] > 5 { "Intermediate" } else { "Novice" }),
            V::S(if a[2] > 10000 { "Highly Visible" } else { "Moderate Visibility" }),
        ])
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(P.Score) AS AvgScore FROM Posts P GROUP BY P.OwnerUserId),
// UserPostBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.TotalBadges, 0) AS TotalBadges, COALESCE(PS.TotalPosts, 0) AS TotalPosts,
//        COALESCE(PS.TotalQuestions, 0) AS TotalQuestions, COALESCE(PS.TotalAnswers, 0) AS TotalAnswers, COALESCE(PS.AvgScore, 0) AS AvgScore
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT U.UserId, U.DisplayName, U.TotalBadges, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.AvgScore,
//        CASE WHEN U.TotalPosts > 50 AND U.TotalBadges > 5 THEN 'Top Contributor' WHEN U.TotalPosts > 20 AND U.TotalBadges > 0 THEN 'Moderate Contributor' ELSE 'Novice Contributor' END AS ContributorLevel,
//        RANK() OVER (ORDER BY U.AvgScore DESC) AS ScoreRank
// FROM UserPostBadgeStats U WHERE U.TotalPosts > 0 ORDER BY ContributorLevel DESC, AvgScore DESC;
fn q23200(db: &'static So) -> String {
    let Post { owner_user, score, post_type_id, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(post_type_id)).fold([0i64; 4], |a, (s, t)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = whole(&ps).select(Ident::<User>::new().and((&bc).and(&ps))).window(rank, |(_, (_, a))| Reverse(fkey(a[3] as f64 / a[0] as f64)), asc);
    rows(drain(&w).into_iter().map(|(_, ((u, (b, a)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])]);
        f.push(V::S(if a[0] > 50 && b > 5 { "Top Contributor" } else if a[0] > 20 && b > 0 { "Moderate Contributor" } else { "Novice Contributor" }));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount, p.ViewCount,
//        u.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName, p.OwnerUserId, p.Score),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.AnswerCount, rp.ViewCount, rp.OwnerDisplayName, ua.DisplayName AS VoterDisplayName,
//        ua.TotalUpvotes, ua.TotalDownvotes, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges
// FROM RankedPosts rp JOIN UserActivity ua ON rp.OwnerDisplayName = ua.DisplayName WHERE rp.UserPostRank <= 5 ORDER BY rp.ViewCount DESC, rp.CreationDate DESC;
//
// The rank is within each owner, so the ownerless questions (whose name is NULL and cannot join) are left out before ranking.
fn q27273(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = post_comment_count(db, &tp);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (v, b)| [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (b == Some(1)) as i64, a[3] + (b == Some(2)) as i64, a[4] + (b == Some(3)) as i64]);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain((&cc).and(&ac).and(owner_user.select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&ua))));
    rows(v.into_iter().map(|(p, ((c, n), (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(post_fields(db, p, &["views", "owner"]));
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(p.Score) AS AvgScore FROM Posts p GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.Questions, 0) AS Questions, COALESCE(ps.Answers, 0) AS Answers, COALESCE(ps.AvgScore, 0) AS AvgScore
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges, ua.TotalPosts, ua.Questions, ua.Answers, ua.AvgScore,
//        RANK() OVER (ORDER BY ua.Reputation DESC) AS ReputationRank
// FROM UserActivity ua WHERE ua.Reputation > 1000 AND (ua.GoldBadges + ua.SilverBadges + ua.BronzeBadges > 0 OR ua.TotalPosts > 10) ORDER BY ua.Reputation DESC LIMIT 50;
fn q1051(db: &'static So) -> String {
    let Post { owner_user, score, post_type_id, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(post_type_id)).fold([0i64; 4], |a, (s, t)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let ub = user_badge_classes(db);
    let ua: HashIdx<Id<User>, ([i64; 4], [i64; 4])> = db
        .user
        .with((&db.user.reputation).gt(1000))
        .select((&ub).and((&ps).opt().map(|o: Option<[i64; 4]>| o.unwrap_or([0; 4]))))
        .filt(|(b, a): ([i64; 4], [i64; 4])| b[1] + b[2] + b[3] > 0 || a[0] > 10)
        .collect();
    let w = whole(&ua).select(Ident::<User>::new().and(&db.user.reputation).and(&ua)).window(rank, |((_, r), _)| Reverse(r), asc);
    let v = top_n(drain(&w), |&(_, (((u, r), _), _))| (Reverse(r), u), 50);
    rows(v.into_iter().map(|(_, (((u, _), (b, a)), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(if a[0] == 0 { V::F(0.0) } else { avg(a[3], a[0]) });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// PostMetrics AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.AnswerCount, P.CommentCount, P.Tags, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id, P.Title, P.ViewCount, P.AnswerCount, P.CommentCount, P.Tags),
// RankedPosts AS (SELECT PM.*, RANK() OVER (ORDER BY PM.TotalUpVotes DESC, PM.ViewCount DESC) AS PostRank FROM PostMetrics PM)
// SELECT U.UserId, U.DisplayName, RP.PostId, RP.Title, RP.ViewCount, RP.AnswerCount, RP.CommentCount, RP.TotalUpVotes, RP.TotalDownVotes, RP.CloseCount,
//        CASE WHEN RP.TotalUpVotes > 0 THEN CAST(RP.TotalUpVotes AS FLOAT) / NULLIF(RP.TotalUpVotes + RP.TotalDownVotes, 0) ELSE 0 END AS UpVoteRatio
// FROM UserVoteStats U JOIN RankedPosts RP ON RP.TotalUpVotes > 0 WHERE RP.PostRank <= 10 OR (U.VoteCount > 5 AND RP.CloseCount = 0) ORDER BY U.VoteCount DESC, UpVoteRatio DESC;
//
// The ON clause names only RP, so users and the upvoted posts are crossed and the WHERE filters the pairs.
fn q2561(db: &'static So) -> String {
    let uv = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let pm = db
        .post
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, (v, h)| [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (h == Some(10)) as i64]);
    let w = whole(&pm).select(Ident::<Post>::new().and(&pm).and((&db.post.view_count).opt())).window(rank, |((_, a), w)| (Reverse(a[0]), w.is_none(), Reverse(w)), asc);
    type R = (((Id<Post>, [i64; 3]), Option<i64>), i64);
    let rp: MatSet<R> = (&w).filt(|(((_, a), _), _): R| a[0] > 0).collect();
    let mut v = Vec::new();
    (&uv).cross(&rp).filt(|(n, (((_, a), _), r)): (i64, R)| r <= 10 || (n > 5 && a[2] == 0)).drive(|(u, _), (_, (((p, a), _), _))| v.push((u, p, a)));
    rows(v.into_iter().map(|(u, p, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "views", "answers", "comments"]));
        f.extend(a.map(V::I));
        f.push(V::F((a[0] as f32 / (a[0] + a[1]) as f32) as f64));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(p.Score) AS TotalScore
//     FROM Posts p GROUP BY p.OwnerUserId),
// UserSummaries AS (SELECT u.Id, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ps.QuestionCount, 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount,
//        COALESCE(ps.TotalScore, 0) AS TotalScore, RANK() OVER (ORDER BY COALESCE(ps.TotalScore, 0) DESC) AS ScoreRank
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT us.DisplayName, us.BadgeCount, us.QuestionCount, us.AnswerCount, us.TotalScore, us.ScoreRank, COALESCE(pht.Name, 'No Activity') AS RecentActivity,
//        COALESCE(MAX(ph.CreationDate), '1900-01-01') AS LastActivityDate
// FROM UserSummaries us LEFT JOIN Posts p ON us.Id = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// GROUP BY us.DisplayName, us.BadgeCount, us.QuestionCount, us.AnswerCount, us.TotalScore, us.ScoreRank, pht.Name
// HAVING us.BadgeCount > 0 OR us.QuestionCount > 0 ORDER BY us.ScoreRank ASC, us.DisplayName ASC;
//
// The GROUP BY names the summary columns, not the user id, so users that agree on all of them share a group.
fn q2857(db: &'static So) -> String {
    let Post { owner_user, score, post_type_id, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(post_type_id)).fold([0i64; 3], |a, (s, t)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ua: HashIdx<Id<User>, (i64, [i64; 3])> = (&bc).and((&ps).opt().map(|o: Option<[i64; 3]>| o.unwrap_or([0; 3]))).collect();
    let w = whole(&ua).select(Ident::<User>::new().and(&ua).and(&db.user.display_name)).window(rank, |((_, (_, a)), _)| Reverse(a[2]), asc);
    type W = (((Id<User>, (i64, [i64; 3])), Str), i64);
    type T = (Str, i64, i64, i64, i64, i64);
    let ws: MatSet<W> = (&w).filt(|(((_, (b, a)), _), _): W| b > 0 || a[0] > 0).collect();
    let sums: HashIdx<Id<User>, T> = (&ws).map(|(((u, _), _), _): W| u).inv().select((&ws).map(|(((_, (b, a)), n), r): W| (n, b, a[0], a[1], a[2], r))).collect();
    let j: MatSet<(Id<User>, Option<Id<PostHistory>>)> =
        db.user.with(&sums).select(Ident::<User>::new().and(posts_of(db).select(history_of(db).opt()).opt().map(|o: Option<Option<Id<PostHistory>>>| o.flatten()))).collect();
    let hist = (&j).flat_map(|(_, h)| h);
    let g = (&j)
        .group_by((&j).map(|(u, _)| u).select(&sums).and((&hist).select(htype_name(db)).opt()))
        .select((&hist).select(&db.post_history.creation_date).opt())
        .fold(i64::MIN, |m, d| d.map_or(m, |d| m.max(d)));
    rows(drain(&g).into_iter().map(|((t, n), d)| {
        row(vec![V::S(t.0), V::I(t.1), V::I(t.2), V::I(t.3), V::I(t.4), V::I(t.5), V::S(n.unwrap_or("No Activity")), V::T(if d == i64::MIN { date(1900, 1, 1) } else { d })])
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(v.BountyAmount) AS TotalBounties FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, MIN(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Comment END) AS CloseReason, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenedDate
//     FROM PostHistory ph WHERE ph.PostId IN (SELECT PostId FROM RecursivePostHistory WHERE rn = 1) GROUP BY ph.PostId),
// UserContributions AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePostCount
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT a.UserId, a.DisplayName, a.PostCount, a.TotalBounties, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(c.PositivePostCount, 0) AS PositivePostCount, cp.CloseReason, cp.ReopenedDate
// FROM MostActiveUsers a LEFT JOIN UserContributions c ON a.UserId = c.UserId LEFT JOIN ClosedPosts cp ON a.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = cp.PostId)
// WHERE a.PostCount > 10 ORDER BY a.TotalBounties DESC, a.PostCount DESC;
//
// A post has a rn = 1 row exactly when it has a history row of type 10, 11 or 12. UserContributions is only read for the active users, so its product is driven for them alone.
fn q34272(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ma = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(bounty.opt()))
        .fold([0i64; 3], |a, b| {
            let b = b.flatten();
            [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let act: MatSet<Id<User>> = (&ma).filt(|a| a[0] > 10).map(|_| ()).inv().map(|u| u).collect::<MatSet<Id<User>>>();
    let uc = (&act)
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(posts_of(db).select(score).opt()))
        .fold([0i64; 2], |a, (c, s)| [a[0] + c.is_some() as i64, a[1] + s.map_or(false, |s| s > 0) as i64]);
    let PostHistory { post, post_history_type_id, comment, creation_date, .. } = &db.post_history;
    let closed: MatSet<Id<Post>> = db.post_history.with(post_history_type_id.is_in([10, 11, 12])).select(post).collect();
    let cp = (&closed)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(post_history_type_id.and(comment.opt()).and(creation_date)))
        .fold((None::<Str>, i64::MIN), |(m, d), ((t, c), cd)| {
            let m = if t == 10 { match (m, c) { (Some(a), Some(b)) => Some(if b < a { b } else { a }), (a, b) => a.or(b) } } else { m };
            (m, if t == 11 { d.max(cd) } else { d })
        });
    let by_owner: HashIdx<Id<User>, (Id<Post>, (Option<Str>, i64))> = (&closed).select(owner_user).inv().select(Ident::<Post>::new().and(&cp)).collect();
    let v = drain((&act).select(Ident::<User>::new().and((&ma).and(&uc).and((&by_owner).opt()))));
    rows(v.into_iter().map(|(_, (u, ((a, c), x)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::I(c[0]), V::I(c[1])]);
        f.extend(match x {
            Some((_, (m, d))) => [ostr(m), tmax(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY COALESCE(MAX(ph.CreationDate), p.CreationDate) DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (10, 11, 12, 13) GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// PopularPosts AS (SELECT rp.Id, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, (rp.UpVotes - rp.DownVotes) AS Score,
//        CASE WHEN rp.CommentCount > 10 THEN 'High Activity' WHEN rp.CommentCount BETWEEN 5 AND 10 THEN 'Moderate Activity' ELSE 'Low Activity' END AS ActivityLevel
//     FROM RankedPosts rp WHERE rp.rn = 1 AND rp.CommentCount > 0),
// TopPosts AS (SELECT pp.*, DENSE_RANK() OVER (ORDER BY pp.Score DESC) AS PostRank FROM PopularPosts pp)
// SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.CommentCount, tp.UpVotes, tp.DownVotes, tp.Score, tp.ActivityLevel
// FROM TopPosts tp WHERE tp.PostRank <= 10 OR (tp.ActivityLevel = 'High Activity' AND tp.CommentCount > 0) ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// The window is partitioned by the post itself, so rn is 1 on every row.
fn q516(db: &'static So) -> String {
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11, 12, 13])));
    let s = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(closes.opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let sf: HashIdx<Id<Post>, [i64; 3]> = (&s).filt(|a| a[0] > 0).collect();
    let w = whole(&sf).select(Ident::<Post>::new().and(&sf)).window(dense_rank, |(_, a)| Reverse(a[1] - a[2]), asc);
    let v = drain((&w).filt(|((_, a), r): ((Id<Post>, [i64; 3]), i64)| r <= 10 || a[0] > 10));
    rows(v.into_iter().map(|(_, ((p, a), _))| {
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])]);
        f.push(V::S(if a[0] > 10 { "High Activity" } else if a[0] >= 5 { "Moderate Activity" } else { "Low Activity" }));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions, COALESCE(SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END), 0) AS PositiveScorePosts,
//        COALESCE(AVG(P.Score), 0) AS AverageScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers, TotalQuestions, PositiveScorePosts, AverageScore, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank
//     FROM UserPostStats WHERE TotalPosts > 0),
// HighScoringUsers AS (SELECT T.DisplayName, T.TotalQuestions, T.TotalAnswers, T.PositiveScorePosts, T.AverageScore, COUNT(CASE WHEN B.Name = 'Gold' THEN 1 END) AS GoldBadges,
//        COUNT(CASE WHEN B.Name = 'Silver' THEN 1 END) AS SilverBadges, COUNT(CASE WHEN B.Name = 'Bronze' THEN 1 END) AS BronzeBadges
//     FROM TopUsers T LEFT JOIN Badges B ON T.UserId = B.UserId WHERE T.AverageScore > 10 GROUP BY T.DisplayName, T.TotalQuestions, T.TotalAnswers, T.PositiveScorePosts, T.AverageScore)
// SELECT H.DisplayName, H.TotalQuestions, H.TotalAnswers, H.PositiveScorePosts, H.AverageScore, COALESCE(H.GoldBadges, 0) AS GoldBadges, COALESCE(H.SilverBadges, 0) AS SilverBadges,
//        COALESCE(H.BronzeBadges, 0) AS BronzeBadges
// FROM HighScoringUsers H WHERE H.TotalQuestions > 5 ORDER BY H.AverageScore DESC, H.PositiveScorePosts DESC FETCH FIRST 10 ROWS ONLY;
//
// The GROUP BY names the user's columns rather than the id, so users agreeing on all of them (the mean as a DOUBLE) share a group.
fn q1925(db: &'static So) -> String {
    let ups = user_posts(db);
    let hs = (&ups).filt(|a| a[1] > 0 && a[4] > 10 * a[1] && a[2] > 5);
    let key = (&db.user.display_name).and((&ups).map(|a| (a[2], a[3], a[8], fkey(a[4] as f64 / a[1] as f64))));
    let g = db
        .user
        .with(hs)
        .group_by(key)
        .select(badges_of(db).select(&db.badge.name).opt())
        .fold([0i64; 3], |a, n| [a[0] + (n == Some("Gold")) as i64, a[1] + (n == Some("Silver")) as i64, a[2] + (n == Some("Bronze")) as i64]);
    let fv = |k: i64| f64::from_bits((k ^ (((k >> 63) as u64) >> 1) as i64) as u64);
    let v = top_n(drain(&g), |&((_, (_, _, p, m)), _)| (Reverse(m), Reverse(p)), 10);
    rows(v.into_iter().map(|((n, (q, a, p, m)), b)| {
        let mut f = vec![V::S(n), V::I(q), V::I(a), V::I(p), V::F(fv(m))];
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        AVG(COALESCE(p.Score, 0)) AS AverageScore FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(p.Id) AS PostsContributed, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven, AVG(COALESCE(p.Score, 0)) AS AveragePostScore FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.Reputation),
// TopTags AS (SELECT ts.TagName, ts.PostCount, ts.QuestionCount, ts.AnswerCount, ts.AverageScore, RANK() OVER (ORDER BY ts.PostCount DESC) AS TagRank FROM TagStatistics ts),
// ActiveUsers AS (SELECT ur.UserId, ur.Reputation, ur.PostsContributed, ur.QuestionsAsked, ur.AnswersGiven, ur.AveragePostScore, RANK() OVER (ORDER BY ur.Reputation DESC) AS UserRank FROM UserReputation ur)
// SELECT t.TagName, t.PostCount, t.QuestionCount, t.AnswerCount, t.AverageScore, u.UserId, u.Reputation, u.PostsContributed, u.QuestionsAsked, u.AnswersGiven, u.AveragePostScore
// FROM TopTags t JOIN ActiveUsers u ON u.PostsContributed > 0 WHERE t.TagRank <= 10 AND u.UserRank <= 10 ORDER BY t.PostCount DESC, u.Reputation DESC;
//
// The ON clause names only u, so the top tags and top users are crossed.
fn q27570(db: &'static So) -> String {
    let tst = tag_name_stats(db);
    let tw = whole(&tst).select(Same::<Str>::new().and(&tst)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let tt: MatSet<(Str, [i64; 7])> = (&tw).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let ups = user_posts(db);
    let uw = whole(&ups).select(Ident::<User>::new().and(&ups).and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let au: MatSet<(Id<User>, [i64; 10])> = (&uw).filt(|(((_, a), _), r): (((Id<User>, [i64; 10]), i64), i64)| r <= 10 && a[1] > 0).map(|((x, _), _)| x).collect();
    let mut v = Vec::new();
    (&tt).cross(&au).drive(|_, ((t, b), (u, a))| v.push((t, b, u, a)));
    rows(v.into_iter().map(|(t, b, u, a)| {
        let mut f = vec![V::S(t), V::I(b[0]), V::I(b[4]), V::I(b[5]), avg(b[3], b[6])];
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0])]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostAnalytics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AvgScore, SUM(p.ViewCount) AS TotalViews, MAX(p.CreationDate) AS LastPostDate FROM Posts p GROUP BY p.OwnerUserId),
// UserEngagement AS (SELECT u.Id AS UserId, COALESCE(pb.PostCount, 0) AS PostCount, COALESCE(pb.AvgScore, 0) AS AvgScore, COALESCE(pb.TotalViews, 0) AS TotalViews,
//        COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(pb.PostCount, 0) DESC, COALESCE(pb.AvgScore, 0) DESC) AS Rank
//     FROM Users u LEFT JOIN PostAnalytics pb ON u.Id = pb.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId)
// SELECT u.Id, u.DisplayName, ue.PostCount, ue.AvgScore, ue.TotalViews, ue.BadgeCount, ue.GoldBadges, ue.SilverBadges, ue.BronzeBadges,
//        CASE WHEN ue.PostCount > 0 THEN 'Active' ELSE 'Inactive' END AS UserStatus
// FROM UserEngagement ue JOIN Users u ON ue.UserId = u.Id WHERE (ue.TotalViews >= 1000 OR ue.BadgeCount > 2) AND ue.Rank <= 10 ORDER BY ue.Rank;
fn q3366(db: &'static So) -> String {
    let ups = user_posts(db);
    let ub = user_badge_classes(db);
    let mean = |a: &[i64; 10]| if a[1] == 0 { 0.0 } else { a[4] as f64 / a[1] as f64 };
    let v = top_n(drain(&ups), |&(u, a)| (Reverse(a[1]), Reverse(fkey(mean(&a))), u), 10);
    let top = rel(v);
    let hit = (&top).select(Same::<(Id<User>, [i64; 10])>::new().and(Same::<(Id<User>, [i64; 10])>::new().map(|(u, _)| u).select(&ub)))
        .filt(|((_, a), b): ((Id<User>, [i64; 10]), [i64; 4])| a[6] >= 1000 || b[0] > 2);
    let v = drain(hit);
    rows(v.into_iter().map(|(_, ((u, a), b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::F(mean(&a)), V::I(a[6])]);
        f.extend(b.map(V::I));
        f.push(V::S(if a[1] > 0 { "Active" } else { "Inactive" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopPostsWithBadges AS (SELECT r.PostId, r.Title, r.OwnerDisplayName, r.CreationDate, r.Score, r.ViewCount, COUNT(b.Id) AS BadgeCount FROM RankedPosts r
//     LEFT JOIN Badges b ON r.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId) WHERE r.PostRank <= 3
//     GROUP BY r.PostId, r.Title, r.OwnerDisplayName, r.CreationDate, r.Score, r.ViewCount),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// FinalResults AS (SELECT t.PostId, t.Title, t.OwnerDisplayName, t.Score, t.ViewCount, COALESCE(c.CloseCount, 0) AS ClosedCount, t.BadgeCount,
//        CASE WHEN t.BadgeCount > 1 THEN 'Multiple Badges' WHEN t.BadgeCount = 1 THEN 'Single Badge' ELSE 'No Badges' END AS BadgeStatus
//     FROM TopPostsWithBadges t LEFT JOIN ClosedPosts c ON t.PostId = c.PostId)
// SELECT *, (CASE WHEN ClosedCount > 5 THEN 'Very Inactive' WHEN ClosedCount BETWEEN 1 AND 5 THEN 'Somewhat Inactive' ELSE 'Active' END) AS ActivityLevel
// FROM FinalResults ORDER BY Score DESC, ViewCount DESC;
fn q34619(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 3).map(|((p, _), _)| p).collect();
    let by_name = db.badge.group_by((&db.badge.user).select(&db.user.display_name)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(&db.user.display_name).select((&by_name).opt())));
    rows(v.into_iter().map(|(p, (c, b))| {
        let b = b.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.extend([V::I(c), V::I(b), V::S(if b > 1 { "Multiple Badges" } else if b == 1 { "Single Badge" } else { "No Badges" })]);
        f.push(V::S(if c > 5 { "Very Inactive" } else if c >= 1 { "Somewhat Inactive" } else { "Active" }));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(p.Score) AS AvgScore, MAX(p.CreationDate) AS LastPostDate FROM Posts p GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.TotalQuestions, 0) AS TotalQuestions,
//        COALESCE(ps.TotalAnswers, 0) AS TotalAnswers, COALESCE(bs.TotalBadges, 0) AS TotalBadges, COALESCE(bs.GoldBadges, 0) AS GoldBadges, COALESCE(bs.SilverBadges, 0) AS SilverBadges,
//        COALESCE(bs.BronzeBadges, 0) AS BronzeBadges, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank
//     FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN UserBadgeStats bs ON u.Id = bs.UserId)
// SELECT ua.DisplayName, ua.Reputation, ua.TotalPosts, ua.TotalQuestions, ua.TotalAnswers, ua.TotalBadges, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges, ua.UserRank
// FROM UserActivity ua WHERE ua.TotalPosts > 5 AND ua.Reputation > 100 ORDER BY ua.UserRank, ua.Reputation DESC FETCH FIRST 10 ROWS ONLY;
fn q3693(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(rep)).window(dense_rank, |(_, r)| Reverse(r), asc);
    type R = ((Id<User>, i64), i64);
    let dr: MatSet<R> = (&w).collect();
    let ups = user_posts(db);
    let ub = user_badge_classes(db);
    let hit = (&dr).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| u).select(Ident::<User>::new().with(rep.gt(100)).select((&ups).filt(|a| a[1] > 5).and(&ub)))));
    let v = top_n(drain(hit), |&(_, (((u, rp), r), _))| (r, Reverse(rp), u), 10);
    rows(v.into_iter().map(|(_, (((u, _), r), (a, b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.ViewCount > 10),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerReputation FROM RankedPosts WHERE PostRank <= 10),
// PostVoteSummary AS (SELECT p.Id AS PostId, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// PostHistoryDetails AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEditDate, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId IN (4, 5, 6)) AS EditCount,
//        SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerReputation, pvs.UpVotes, pvs.DownVotes, phd.LastEditDate, phd.EditCount, phd.CloseCount,
//        CASE WHEN phd.CloseCount > 0 THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM TopPosts tp LEFT JOIN PostVoteSummary pvs ON tp.PostId = pvs.PostId LEFT JOIN PostHistoryDetails phd ON tp.PostId = phd.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q30063(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(view_count.gt(10)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold([0, 0, i64::MIN], |a, (t, d)| [a[0] + matches!(t, 4 | 5 | 6) as i64, a[1] + (t == 10) as i64, a[2].max(d)]);
    let v = drain((&post_updown(db, &tp)).and((&phd).opt()));
    rows(v.into_iter().map(|(p, (u, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.extend(u.map(V::I));
        f.extend(match h {
            Some(a) => [V::T(a[2]), V::I(a[0]), V::I(a[1]), V::S(if a[1] > 0 { "Closed" } else { "Open" })],
            None => [V::Null, V::Null, V::Null, V::S("Open")],
        });
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// TopUsers AS (SELECT Id, DisplayName, Reputation, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, ROW_NUMBER() OVER (ORDER BY Reputation DESC, BadgeCount DESC) AS Rank
//     FROM Users U JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId WHERE U.Reputation > 1000),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.CreationDate, ROW_NUMBER() OVER (ORDER BY P.ViewCount DESC, P.Score DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND P.PostTypeId = 1),
// CombinedData AS (SELECT TU.DisplayName, TU.Reputation, TU.GoldBadges, TU.SilverBadges, TU.BronzeBadges, PP.Title, PP.Score, PP.ViewCount, PP.AnswerCount, PP.CommentCount, PP.CreationDate
//     FROM TopUsers TU JOIN PopularPosts PP ON TU.Rank <= 10)
// SELECT DisplayName, Reputation, GoldBadges, SilverBadges, BronzeBadges, Title, Score, ViewCount, AnswerCount, CommentCount, CreationDate FROM CombinedData ORDER BY Reputation DESC, Score DESC;
//
// The ON clause names only TU, so the top ten users are crossed with every recent question.
fn q9803(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let ub = user_badge_classes(db);
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&ub)), |&(u, b)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b[0]), u), 10);
    let tu = rel(tu);
    let pp: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(post_type_id.eq(1))).collect();
    let mut v = Vec::new();
    (&tu).cross(&pp).drive(|_, ((u, b), p)| v.push((u, b, p)));
    rows(v.into_iter().map(|(u, b, p)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        f.extend(post_fields(db, p, &["title", "score", "views", "answers", "comments", "created"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// RecentEdits AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.PostId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        RANK() OVER (ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS UserRank FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerName, rp.CommentCount, re.LastEditDate, tu.DisplayName AS TopVoterName, tu.TotalUpVotes, tu.TotalDownVotes
// FROM RankedPosts rp JOIN RecentEdits re ON rp.PostId = re.PostId
// LEFT JOIN TopUsers tu ON tu.UserId = (SELECT UserId FROM Votes v WHERE v.PostId = rp.PostId ORDER BY v.CreationDate DESC LIMIT 1)
// WHERE rp.CommentCount > 5 AND rp.CreationDate >= CURRENT_DATE - INTERVAL '30 days' ORDER BY rp.Score DESC, rp.CommentCount DESC;
//
// The correlated LIMIT 1 is ROW_NUMBER() = 1 over each post's votes; ties on CreationDate go to the larger vote id (the SQL leaves them open).
fn q1479(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let rp = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(current_date(), -30))));
    let cc = post_comment_count(db, rp);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let re = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let Vote { creation_date: vd, origid, user, .. } = &db.vote;
    let lw = db.vote.with(&db.vote.post).group_by(&db.vote.post).select(Ident::<Vote>::new().and(vd).and(origid)).window(row_number, |((_, d), i)| (d, i), desc);
    let last: HashIdx<Id<Post>, Id<Vote>> = (&lw).filt(|(_, n)| n == 1).map(|(((v, _), _), _)| v).collect();
    let tu = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let voter = (&last).select(user).select(Ident::<User>::new().and(&tu));
    let v = drain((&cc).filt(|n| n > 5).and(&re).and(voter.opt()));
    rows(v.into_iter().map(|(p, ((n, d), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(n), V::T(d)]);
        f.extend(match t {
            Some((u, a)) => [user_col(db, u, "name"), V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.DisplayName ORDER BY TotalViews DESC LIMIT 5),
// PostHistoryWithCloseReasons AS (SELECT ph.PostId, MIN(ph.CreationDate) AS FirstClosedDate, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Comment END) AS CloseReason
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, tu.DisplayName AS TopUser, tu.TotalViews, tu.TotalScore, ph.CloseReason,
//        CASE WHEN rp.PostRank = 1 THEN 'Most Recent Post' ELSE 'Other Posts' END AS PostCategory
// FROM RankedPosts rp JOIN PostHistoryWithCloseReasons ph ON rp.PostId = ph.PostId JOIN TopUsers tu ON rp.ViewCount > tu.TotalViews / 10
// WHERE rp.CommentCount > 5 AND COALESCE(ph.FirstClosedDate, '1970-01-01') > '2023-01-01' ORDER BY rp.ViewCount DESC, rp.Title;
fn q34544(db: &'static So) -> String {
    let Post { owner_user_id, view_count, score, creation_date, .. } = &db.post;
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.group_by(post).select(hd.and(post_history_type_id).and(comment.opt())).fold((i64::MAX, None::<Str>), |(m, c), ((d, t), x)| {
        (m.min(d), if t == 10 { match (c, x) { (Some(a), Some(b)) => Some(if b > a { b } else { a }), (a, b) => a.or(b) } } else { c })
    });
    let recent = (&ph).filt(|(d, _): (i64, Option<Str>)| d > ts(2023, 1, 1, 0, 0, 0));
    let rp_ids: MatSet<Id<Post>> = (&recent).map(|_| ()).inv().collect();
    let rp = (&rp_ids)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt().and(score))).fold([0i64; 3], |a, (w, s)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + s]);
    let tu = rel(top_n(drain(&tu), |&(u, a)| (a[0] == 0, Reverse(a[1]), u), 5));
    let fw = db.post.group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&fw).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let left = (&rp).filt(|a| a[0] > 5).and(view_count).and(&recent).and(Ident::<Post>::new().with(&first).opt());
    let mut v = Vec::new();
    left.cross((&tu).filt(|(_, a): (Id<User>, [i64; 3])| a[0] > 0))
        .filt(|((((_, w), _), _), (_, a)): (((([i64; 3], i64), (i64, Option<Str>)), Option<Id<Post>>), (Id<User>, [i64; 3]))| w as f64 > a[1] as f64 / 10.0)
        .drive(|(p, _), ((((a, _), (_, r)), f), (u, t))| v.push((p, a, r, f, u, t)));
    rows(v.into_iter().map(|(p, a, r, f, u, t)| {
        let mut x = post_fields(db, p, &["id", "title", "views"]);
        x.extend(a.map(V::I));
        x.extend([user_col(db, u, "name"), V::I(t[1]), V::I(t[2]), ostr(r), V::S(if f.is_some() { "Most Recent Post" } else { "Other Posts" })]);
        row(x)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT u.UserId, u.DisplayName, u.BadgeCount, u.GoldBadges, u.SilverBadges, u.BronzeBadges, RANK() OVER (ORDER BY u2.Reputation DESC) AS UserRank
//     FROM UserBadgeCounts u JOIN Users u2 ON u.UserId = u2.Id),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.PostTypeId, p.Score, p.ViewCount, p.Tags
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') ORDER BY p.CreationDate DESC),
// UserPostStats AS (SELECT rp.OwnerUserId, COUNT(rp.PostId) AS PostCount, SUM(rp.ViewCount) AS TotalViews, SUM(rp.Score) AS TotalScore FROM RecentPosts rp GROUP BY rp.OwnerUserId)
// SELECT tu.UserId, tu.DisplayName, tu.BadgeCount, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, ups.PostCount, ups.TotalViews, ups.TotalScore
// FROM TopUsers tu LEFT JOIN UserPostStats ups ON tu.UserId = ups.OwnerUserId WHERE tu.UserRank <= 10 ORDER BY tu.UserRank;
fn q28379(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, score, .. } = &db.post;
    let ups = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user)
        .select(view_count.opt().and(score))
        .fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    type T = (Id<User>, i64);
    let top: MatSet<T> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let v = drain((&top).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(Ident::<User>::new().and(user_badge_classes(db)).and((&ups).opt())))));
    rows(v.into_iter().map(|(_, (_, ((u, b), a)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(match a {
            Some(a) => [V::I(a[0]), nullable(a[2], a[1]), V::I(a[3])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(SUM(b.Class), 0) AS TotalBadges, COALESCE(SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END), 0) AS VoteScore
//     FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.Reputation, u.DisplayName),
// ClosedPostDetails AS (SELECT ph.PostId, ph.CreationDate, hp.Name AS CloseReason FROM PostHistory ph INNER JOIN PostHistoryTypes hp ON ph.PostHistoryTypeId = hp.Id WHERE hp.Name LIKE 'Post Closed%')
// SELECT rp.PostId, rp.Title, us.DisplayName, us.Reputation, us.TotalBadges, us.VoteScore, COALESCE(cp.CloseReason, 'Not Closed') AS CloseReason, COALESCE(cp.CreationDate::TEXT, 'N/A') AS CloseDate,
//        rp.CommentCount, CASE WHEN rp.UserPostRank = 1 THEN 'Latest' WHEN rp.UserPostRank <= 5 THEN 'Top 5' ELSE 'Older Posts' END AS PostRankCategory
// FROM RankedPosts rp JOIN UserStats us ON us.UserId = rp.OwnerUserId LEFT JOIN ClosedPostDetails cp ON cp.PostId = rp.PostId
// WHERE us.Reputation > 100 ORDER BY us.Reputation DESC, rp.CommentCount DESC FETCH FIRST 50 ROWS ONLY;
fn q23752(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date).and(owner_user)).window(row_number, |((p, d), _)| (Reverse(d), p), asc);
    let rn: MatSet<(Id<Post>, Id<User>, i64)> = (&w).map(|(((p, _), u), r)| (p, u, r)).collect();
    let us = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (b, t)| [a[0] + b.unwrap_or(0), a[1] + matches!(t, Some(2 | 3)) as i64]);
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(htype_name(db).filt(|n: Str| n.starts_with("Post Closed"))));
    type R = (Id<Post>, Id<User>, i64);
    let j = (&rn).select(
        Same::<R>::new()
            .and(Same::<R>::new().map(|(_, u, _): R| u).select(&us))
            .and(Same::<R>::new().map(|(p, _, _): R| p).select(post_comment_count(db, db.post.with(post_type_id.eq(1)))))
            .and(Same::<R>::new().map(|(p, _, _): R| p).select(closed.opt())),
    );
    let v = top_n(drain(j), |&(_, ((((p, u, _), _), c), h))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(c), p, h), 50);
    rows(v.into_iter().map(|(_, ((((p, u, r), a), c), h))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [V::S(htype_name(db).get(h).unwrap()), V::Owned(ts_text(db.post_history.creation_date.get(h).unwrap()))],
            None => [V::S("Not Closed"), V::S("N/A")],
        });
        f.extend([V::I(c), V::S(if r == 1 { "Latest" } else if r <= 5 { "Top 5" } else { "Older Posts" })]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(P.Id) AS PostCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.Reputation),
// VoteStats AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostMetrics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(V.UpVotes, 0) AS UpVotes, COALESCE(V.DownVotes, 0) AS DownVotes,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount FROM Posts P LEFT JOIN VoteStats V ON P.Id = V.PostId
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopUsers AS (SELECT U.Id, U.DisplayName, R.Reputation, R.PostCount FROM Users U JOIN UserReputation R ON U.Id = R.UserId WHERE R.Reputation > 1000 ORDER BY R.Reputation DESC LIMIT 10)
// SELECT U.DisplayName AS TopUser, P.Title AS PostTitle, P.CreationDate AS PostDate, P.UpVotes, P.DownVotes, P.CommentCount,
//        CASE WHEN P.UpVotes - P.DownVotes > 0 THEN 'Positive' WHEN P.UpVotes - P.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment,
//        (SELECT STRING_AGG(Name, ', ') FROM Badges B WHERE B.UserId = U.Id GROUP BY B.UserId) AS Badges
// FROM TopUsers U JOIN PostMetrics P ON U.Id = P.PostId WHERE P.CommentCount > 5 ORDER BY P.UpVotes DESC, P.CommentCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. `U.Id = P.PostId` compares a user id with a post id, so it goes through the raw ids.
// STRING_AGG has no ORDER BY; the port joins the badge names in badge id order.
fn q34974(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pid: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pm: MatSet<(Id<User>, Id<Post>)> = (&tu).select(Ident::<User>::new().and((&db.user.origid).select(&pid).select(recent))).collect();
    let pp = (&pm).map(|(_, p)| p);
    let cc = (&pm).group_by(Same::<(Id<User>, Id<Post>)>::new()).select((&pp).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ud = (&pm).group_by(Same::<(Id<User>, Id<Post>)>::new()).select((&pp).select(votes_of(db).select(&db.vote.vote_type_id)).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let names = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(|it| {
        let mut b: Vec<Id<Badge>> = it.into_iter().flatten().collect();
        b.sort_unstable();
        let n: Vec<Str> = b.into_iter().map(|b| db.badge.name.get(b).unwrap()).collect();
        if n.is_empty() { None } else { Some(&*Box::leak(n.join(", ").into_boxed_str())) }
    });
    let v = drain((&cc).filt(|n| n > 5).and(&ud).and(Same::<(Id<User>, Id<Post>)>::new().map(|(u, _)| u).select(&names)));
    rows(v.into_iter().map(|((u, p), ((c, a), n))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" }), ostr(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// RecentBadgedUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u JOIN Badges b ON u.Id = b.UserId
//     WHERE b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(ph.Id) AS ChangeCount FROM PostHistory ph
//     WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 MONTHS' GROUP BY ph.PostId, ph.PostHistoryTypeId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.UpVoteCount, rp.DownVoteCount, u.DisplayName AS TopUser, ub.BadgeCount, phs.ChangeCount
// FROM RankedPosts rp JOIN RecentBadgedUsers ub ON rp.ViewCount = (SELECT MAX(ViewCount) FROM RankedPosts)
// LEFT JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) LEFT JOIN PostHistoryStats phs ON phs.PostId = rp.PostId
// WHERE rp.rn = 1 ORDER BY rp.ViewCount DESC;
//
// The ON clause names rp and a scalar, so the most viewed questions are crossed with every recently badged user; rn is 1 on every row.
fn q6947(db: &'static So) -> String {
    let Post { post_type_id, view_count, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let most = qs().select(view_count).fold_flat(i64::MIN, |m, w| m.max(w));
    let rp = qs()
        .with(view_count.eq(most))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ub = db.badge.with((&db.badge.date).ge(add_years(t0, -1))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(hd.ge(add_months(t0, -6))).group_by(post.and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let phs = rel(drain(&phs));
    let by_post: HashIdx<Id<Post>, ((Id<Post>, i64), i64)> = (&phs).map(|((p, _), _)| p).inv().select(&phs).collect();
    let left = (&rp).and((&by_post).map(|(_, n)| n).opt());
    let mut v = Vec::new();
    left.cross(&ub).drive(|(p, u), ((a, n), b)| v.push((p, a, n, u, b)));
    rows(v.into_iter().map(|(p, a, n, _, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(a.map(V::I));
        f.push(ostr(owner_user.get(p).map(|u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(b), oint(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount
//     FROM PostHistory ph GROUP BY ph.PostId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, ub.BadgeCount, ub.HighestBadgeClass, phs.CloseCount, phs.DeleteCount FROM RankedPosts rp JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId
//     LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId WHERE rp.UserRank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.BadgeCount, tp.HighestBadgeClass, COALESCE(tp.CloseCount, 0) AS CloseCount, COALESCE(tp.DeleteCount, 0) AS DeleteCount,
//        CASE WHEN tp.HighestBadgeClass = 1 THEN 'Gold' WHEN tp.HighestBadgeClass = 2 THEN 'Silver' WHEN tp.HighestBadgeClass = 3 THEN 'Bronze' ELSE 'No Badge' END AS BadgeType,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = tp.PostId) AS CommentCount
// FROM TopPosts tp ORDER BY tp.CreationDate DESC FETCH FIRST 50 ROWS ONLY;
fn q33132(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top = top_n(drain((&w).filt(|(_, n)| n <= 5)), |&(_, ((p, d), _))| (Reverse(d), p), 50);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|(_, ((p, _), _))| p).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, None::<i64>), |(n, m), c| match c {
        Some(c) => (n + 1, Some(m.map_or(c, |m| m.max(c)))),
        None => (n, m),
    });
    let ph = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + (t == 12) as i64]);
    let v = drain((&post_comment_count(db, &tp)).and(owner_user.select(&ub)).and((&ph).opt()));
    rows(v.into_iter().map(|(p, ((c, (n, m)), h))| {
        let h = h.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(n), oint(m), V::I(h[0]), V::I(h[1])]);
        f.push(V::S(match m { Some(1) => "Gold", Some(2) => "Silver", Some(3) => "Bronze", _ => "No Badge" }));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges, COUNT(B.Id) AS TotalBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        MAX(P.CreationDate) AS LastPostDate FROM Posts P GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT UB.UserId, UB.DisplayName, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.TotalQuestions, 0) AS TotalQuestions, COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
//        COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, COALESCE(UB.TotalBadges, 0) AS TotalBadges
//     FROM UserBadgeCounts UB LEFT JOIN PostStatistics PS ON UB.UserId = PS.OwnerUserId)
// SELECT C.DisplayName, C.TotalPosts, C.TotalQuestions, C.TotalAnswers, C.GoldBadges, C.SilverBadges, C.BronzeBadges, C.TotalBadges, RANK() OVER (ORDER BY C.TotalPosts DESC) AS PostRank,
//        RANK() OVER (ORDER BY C.TotalBadges DESC) AS BadgeRank
// FROM CombinedStats C WHERE C.TotalPosts > 0 AND (C.GoldBadges + C.SilverBadges + C.BronzeBadges) > 0 ORDER BY C.TotalPosts DESC, C.TotalBadges DESC LIMIT 10;
fn q643(db: &'static So) -> String {
    let ups = user_posts(db);
    let x: HashIdx<Id<User>, ([i64; 4], [i64; 10])> = (&user_badge_classes(db)).filt(|b| b[1] + b[2] + b[3] > 0).and((&ups).filt(|a| a[1] > 0)).collect();
    let w1 = whole(&x).select(Ident::<User>::new().and(&x)).window(rank, |(_, (_, a))| Reverse(a[1]), asc);
    let w2 = (&w1).window(rank, |((_, (b, _)), _): ((Id<User>, ([i64; 4], [i64; 10])), i64)| Reverse(b[0]), asc);
    let v = top_n(drain(&w2), |&(_, (((u, (b, a)), _), _))| (Reverse(a[1]), Reverse(b[0]), u), 10);
    rows(v.into_iter().map(|(_, (((u, (b, a)), pr), br))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(b[0])];
        f.extend([V::I(pr), V::I(br)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><'), 1) AS TagCount,
//        u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, TagCount, OwnerDisplayName, OwnerReputation FROM RankedPosts WHERE RankScore <= 100),
// PostVoteCount AS (SELECT PostId, COUNT(*) FILTER (WHERE VoteTypeId = 2) AS UpVotes, COUNT(*) FILTER (WHERE VoteTypeId = 3) AS DownVotes FROM Votes GROUP BY PostId),
// PostWithVotes AS (SELECT trp.*, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes FROM TopRankedPosts trp LEFT JOIN PostVoteCount pvc ON trp.PostId = pvc.PostId)
// SELECT pwv.PostId, pwv.Title, pwv.CreationDate, pwv.Score, pwv.ViewCount, pwv.TagCount, pwv.OwnerDisplayName, pwv.OwnerReputation, pwv.UpVotes, pwv.DownVotes, (pwv.UpVotes - pwv.DownVotes) AS NetVotes,
//        (EXTRACT(EPOCH FROM cast('2024-10-01 12:34:56' as timestamp) - pwv.CreationDate) / 3600) AS HoursSinceCreation
// FROM PostWithVotes pwv WHERE pwv.UpVotes > pwv.DownVotes ORDER BY NetVotes DESC, pwv.Score DESC;
fn q25800(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, tags_str, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = whole(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).with(owner_user))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 100).map(|(((p, _), _), _)| p).collect();
    let v = drain((&post_updown(db, &tp)).filt(|a| a[0] > a[1]));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(oint(tags_str.get(p).map(|t| tag_list(t).count() as i64)));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::F(secs(t0 - creation_date.get(p).unwrap()) / 3600.0)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// UserVotingStats AS (SELECT v.UserId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVotesCount, COUNT(v.Id) AS TotalVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(CASE WHEN pht.Name IN ('Post Closed', 'Post Deleted') THEN 1 END) AS CloseDeleteCount, COUNT(CASE WHEN pht.Name = 'Post Reopened' THEN 1 END) AS ReopenedCount
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.OwnerDisplayName, COALESCE(uvs.UpVotesCount, 0) AS UpVotesCount, COALESCE(uvs.DownVotesCount, 0) AS DownVotesCount,
//        COALESCE(phs.CloseDeleteCount, 0) AS CloseDeleteCount, COALESCE(phs.ReopenedCount, 0) AS ReopenedCount, DENSE_RANK() OVER (ORDER BY rp.Score DESC) AS ScoreRank
// FROM RankedPosts rp LEFT JOIN UserVotingStats uvs ON rp.OwnerUserId = uvs.UserId LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId WHERE rp.Rank = 1
// ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 100;
fn q34851(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let fw = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&fw).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let uvs = db.vote.group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let phs = db.post_history.group_by(&db.post_history.post).select(htype_name(db)).fold([0i64; 2], |a, n| [a[0] + matches!(n, "Post Closed" | "Post Deleted") as i64, a[1] + (n == "Post Reopened") as i64]);
    let w = whole(&tp).select(Ident::<Post>::new().and(score).and(owner_user.select(&uvs).opt()).and((&phs).opt())).window(dense_rank, |(((_, s), _), _)| Reverse(s), asc);
    let v = top_n(drain(&w), |&(_, ((((p, s), _), _), _))| (Reverse(s), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(_, ((((p, _), u), h), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "owner"]);
        f.extend(u.unwrap_or([0, 0]).map(V::I));
        f.extend(h.unwrap_or([0, 0]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis, SUM(CASE WHEN P.PostTypeId = 4 THEN 1 ELSE 0 END) AS TagWikis,
//        SUM(CASE WHEN P.PostTypeId = 5 THEN 1 ELSE 0 END) AS TagWikiExcerpts FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// BadgeCounts AS (SELECT B.UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId),
// TopUsers AS (SELECT US.UserId, US.DisplayName, US.Reputation, US.TotalPosts, US.Questions, US.Answers, US.Wikis, US.TagWikis, US.TagWikiExcerpts, BC.TotalBadges, BC.GoldBadges, BC.SilverBadges,
//        BC.BronzeBadges, ROW_NUMBER() OVER (ORDER BY US.Reputation DESC) AS Rank FROM UserStatistics US LEFT JOIN BadgeCounts BC ON US.UserId = BC.UserId)
// SELECT Rank, UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, Wikis, TagWikis, TagWikiExcerpts, TotalBadges, GoldBadges, SilverBadges, BronzeBadges
// FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q9172(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    type R = ((Id<User>, i64), i64);
    let tu: MatSet<R> = (&w).filt(|(_, n)| n <= 10).collect();
    let tus: MatSet<Id<User>> = (&tu).map(|((u, _), _): R| u).collect();
    let pt = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 6], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + (t == 4) as i64, a[5] + (t == 5) as i64],
        None => a,
    });
    let bc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&tu).select(Same::<R>::new().map(|((u, _), _): R| u).select(Ident::<User>::new().and(&pt).and((&bc).opt()))));
    rows(v.into_iter().map(|(((_, _), n), ((u, a), b))| {
        let mut f = vec![V::I(n)];
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.extend(a.map(V::I));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, COUNT(v.Id) OVER (PARTITION BY p.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, SUM(COALESCE(u.UpVotes, 0)) AS TotalUpVotes,
//        DENSE_RANK() OVER (ORDER BY SUM(COALESCE(v.BountyAmount, 0)) DESC) AS UserRank FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT post.Id AS PostId, COUNT(ph.Id) AS EditHistoryCount, MAX(ph.CreationDate) AS LastEditDate FROM Posts post LEFT JOIN PostHistory ph ON post.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY post.Id)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, tuv.DisplayName AS TopUserName, tuv.TotalBounty, ih.EditHistoryCount, ih.LastEditDate
// FROM RankedPosts rp LEFT JOIN TopUsers tuv ON rp.OwnerUserId = tuv.UserId LEFT JOIN PostHistoryStats ih ON rp.PostId = ih.PostId
// WHERE rp.Rank <= 5 AND (rp.Score > 10 OR rp.CommentCount > 5) ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// The window numbers the joined post x comment x upvote rows, not the posts, so those rows are materialised and ranked.
fn q32053(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let up = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    type J = (Id<Post>, (Option<Id<Comment>>, Option<Id<Vote>>));
    let j: MatSet<J> = recent().select(Ident::<Post>::new().and(comments_of(db).opt().and(up().opt()))).collect();
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let pp = || Same::<J>::new().map(|(p, _): J| p);
    let w = (&j).group_by(pp().select(post_type_id)).select(Same::<J>::new().and(pp().select(score))).window(row_number, |(x, s)| (Reverse(s), x), asc);
    let rn: MatSet<J> = (&w).filt(|(_, n)| n <= 5).map(|((x, _), _)| x).collect();
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt()).opt())
        .fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ih = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain(
        (&rn).select(
            pp().select(Ident::<Post>::new().and(&cc).and(score))
                .filt(|((_, c), s): ((Id<Post>, i64), i64)| s > 10 || c > 5)
                .and(pp().select(owner_user.select(Ident::<User>::new().and(&tu)).opt()))
                .and(pp().select((&ih).opt())),
        ),
    );
    rows(v.into_iter().map(|(_, ((((p, c), _), u), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.push(V::I(c));
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "name"), V::I(b)],
            None => [V::Null, V::Null],
        });
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopUsers AS (SELECT OwnerDisplayName, SUM(Score) AS TotalScore, COUNT(*) AS TotalPosts FROM RankedPosts WHERE PostRank <= 5 GROUP BY OwnerDisplayName ORDER BY TotalScore DESC LIMIT 10),
// PostWithTopUsers AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, t.OwnerDisplayName, t.TotalScore, t.TotalPosts, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(v.UpVotes, 0) AS UpVotes,
//        COALESCE(v.DownVotes, 0) AS DownVotes FROM Posts p JOIN TopUsers t ON p.OwnerDisplayName = t.OwnerDisplayName
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId)
// SELECT PostId, Title, CreationDate, OwnerDisplayName, TotalScore, TotalPosts, CommentCount, UpVotes, DownVotes FROM PostWithTopUsers ORDER BY TotalScore DESC, CreationDate DESC;
//
// The final join is on the Posts column OwnerDisplayName, not on the owner's Users.DisplayName.
fn q5567(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, owner_display_name, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 5).map(|(((p, _), _), _)| p).collect();
    let tu = (&tp).group_by(owner_user.select(&db.user.display_name)).select(score).fold([0i64; 2], |a, s| [a[0] + s, a[1] + 1]);
    let tu = rel(top_n(drain(&tu), |&(n, a)| (Reverse(a[0]), n), 10));
    let by_name: HashIdx<Str, (Str, [i64; 2])> = (&tu).map(|(n, _)| n).inv().select(&tu).collect();
    let ps: MatSet<Id<Post>> = db.post.with(owner_display_name.select(&by_name)).collect();
    let v = drain((&post_comment_count(db, &ps)).and(&post_updown(db, &ps)).and(owner_display_name.select(&by_name)));
    rows(v.into_iter().map(|(p, ((c, u), (n, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::S(n), V::I(a[0]), V::I(a[1]), V::I(c), V::I(u[0]), V::I(u[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT B.Id) AS BadgeCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Badges B ON U.Id = B.UserId
//     LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id WHERE U.CreationDate >= '2023-01-01' GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COUNT(C.Id) AS TotalComments, COUNT(DISTINCT V.UserId) AS TotalVotes
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, TotalComments, TotalVotes, ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostStats)
// SELECT UA.DisplayName, UA.PostCount, UA.CommentCount, UA.BadgeCount, UA.UpVotes, UA.DownVotes, TP.Title AS TopPostTitle, TP.Score AS TopPostScore, TP.ViewCount AS TopPostViews,
//        TP.TotalComments AS TopPostComments
// FROM UserActivity UA JOIN TopPosts TP ON UA.UpVotes > 10 WHERE UA.BadgeCount > 0 ORDER BY UA.PostCount DESC, UA.UpVotes DESC FETCH FIRST 10 ROWS ONLY;
//
// `V.PostId = P.Id AND V.UserId = U.Id` with `P.OwnerUserId = U.Id` is the post's votes cast by its owner (`own_votes`). The ON clause names only UA, so the users are crossed with every post.
fn q7123(db: &'static So) -> String {
    let us = || db.user.with((&db.user.creation_date).ge(ts(2023, 1, 1, 0, 0, 0)));
    let ov = own_votes(db);
    let ua = us()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(ov.select(&db.vote.vote_type_id).opt()).opt().and(comments_by(db).opt()).and(badges_of(db).opt()))
        .fold([0i64; 2], |a, ((v, _), _)| {
            let t = v.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let dp = us().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let dc = us().group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let db_ = us().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let left = (&ua).filt(|a| a[0] > 10).and(&dp).and(&dc).and((&db_).filt(|n| n > 0));
    let tc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let mut v = Vec::new();
    left.cross(&tc).drive(|(u, p), ((((a, np), nc), nb), c)| v.push((u, a, np, nc, nb, p, c)));
    let v = top_n(v, |&(u, a, np, _, _, p, _)| (Reverse(np), Reverse(a[0]), u, p), 10);
    rows(v.into_iter().map(|(u, a, np, nc, nb, p, c)| {
        let mut f = vec![user_col(db, u, "name"), V::I(np), V::I(nc), V::I(nb), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(P.Score) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// ClosedPosts AS (SELECT PH.UserId, COUNT(*) AS ClosedCount, MAX(PH.CreationDate) AS LastClosedDate FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId),
// RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(CP.ClosedCount, 0) AS ClosedCount,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(PS.TotalPosts, 0) DESC, COALESCE(UB.BadgeCount, 0) DESC) AS UserRank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId LEFT JOIN ClosedPosts CP ON U.Id = CP.UserId)
// SELECT RU.UserId, RU.DisplayName, RU.BadgeCount, RU.TotalPosts, RU.ClosedCount, COALESCE((SELECT MAX(Score) FROM Posts WHERE OwnerUserId = RU.UserId), 0) AS MaxPostScore,
//        CASE WHEN RU.ClosedCount > 0 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorStatus
// FROM RankedUsers RU WHERE RU.UserRank <= 10 ORDER BY RU.UserRank;
fn q1714(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&bc).and(&ups)), |&(u, (b, a))| (Reverse(a[1]), Reverse(b), u), 10);
    let tu = rel(v.into_iter().map(|(u, (b, a))| (u, b, a[1])).collect());
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let ms = db.post.group_by(&db.post.owner_user).select(&db.post.score).fold(i64::MIN, |m, s| m.max(s));
    type T = (Id<User>, i64, i64);
    let u = || Same::<T>::new().map(|(u, _, _): T| u);
    let v = drain((&tu).select(Same::<T>::new().and(u().select(&cp).opt()).and(u().select(&ms).opt())));
    rows(v.into_iter().map(|(_, (((u, b, n), c), m))| {
        let c = c.unwrap_or(0);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(n), V::I(c), V::I(m.unwrap_or(0)), V::S(if c > 0 { "Active Contributor" } else { "New Contributor" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// PostStats AS (SELECT P.OwnerUserId, COUNT(*) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
//        SUM(P.ViewCount) AS TotalViews, MAX(P.CreationDate) AS LatestPostDate FROM Posts P GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT U.UserId, U.Reputation, U.BadgeCount, U.GoldBadges, U.SilverBadges, U.BronzeBadges, P.TotalPosts, P.Questions, P.Answers, P.TotalViews, P.LatestPostDate,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM UserReputation U LEFT JOIN PostStats P ON U.UserId = P.OwnerUserId)
// SELECT C.UserId, C.Reputation, C.BadgeCount, C.GoldBadges, C.SilverBadges, C.BronzeBadges, COALESCE(C.TotalPosts, 0) AS TotalPosts, COALESCE(C.Questions, 0) AS Questions,
//        COALESCE(C.Answers, 0) AS Answers, COALESCE(C.TotalViews, 0) AS TotalViews, C.LatestPostDate,
//        CASE WHEN C.ReputationRank <= 10 THEN 'Top Contributor' WHEN C.ReputationRank <= 50 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorLevel
// FROM CombinedStats C LEFT JOIN Votes V ON C.UserId = V.UserId WHERE C.Reputation > 1000 ORDER BY C.Reputation DESC;
fn q3619(db: &'static So) -> String {
    let rep = &db.user.reputation;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(rep)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let ups = user_posts(db);
    let ub = user_badge_classes(db);
    type R = ((Id<User>, i64), i64);
    type T = (Id<User>, i64);
    let rr = (&w).filt(|((_, r), _): R| r > 1000).map(|((u, _), n): R| (u, n));
    let v = drain(rr.select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select((&ub).and(&ups).and(votes_by(db).opt())))));
    rows(v.into_iter().map(|(_, ((u, r), ((b, a), _)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), tmax(a[7])]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Active Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.Questions, 0) AS Questions,
//        COALESCE(PS.Answers, 0) AS Answers, COALESCE(PS.AverageScore, 0) AS AverageScore, ROW_NUMBER() OVER (ORDER BY COALESCE(PS.TotalPosts, 0) DESC, COALESCE(UB.BadgeCount, 0) DESC) AS EngagementRank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT UEng.UserId, UEng.DisplayName, UEng.BadgeCount, UEng.TotalPosts, UEng.Questions, UEng.Answers, UEng.AverageScore, UEng.EngagementRank,
//        CASE WHEN UEng.BadgeCount > 10 THEN 'High Achiever' WHEN UEng.TotalPosts > 50 THEN 'Active Contributor' ELSE 'New User' END AS EngagementLevel
// FROM UserEngagement UEng WHERE UEng.EngagementRank <= 10 ORDER BY UEng.EngagementRank;
fn q1771(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let x: HashIdx<Id<User>, (i64, [i64; 10])> = (&bc).and(&ups).collect();
    let w = whole(&x).select(Ident::<User>::new().and(&x)).window(row_number, |(u, (b, a))| (Reverse(a[1]), Reverse(b), u), asc);
    let v = drain((&w).filt(|(_, n)| n <= 10));
    rows(v.into_iter().map(|(_, ((u, (b, a)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[1] == 0 { V::F(0.0) } else { avg(a[4], a[1]) }, V::I(r)]);
        f.push(V::S(if b > 10 { "High Achiever" } else if a[1] > 50 { "Active Contributor" } else { "New User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoters, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByUser
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CommentCount, rp.UniqueVoters, rp.UpVotes, rp.DownVotes, rp.CreationDate FROM RankedPosts rp WHERE rp.RankByUser = 1
//     ORDER BY rp.ViewCount DESC LIMIT 10),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT tp.Title, tp.ViewCount, tp.CommentCount, tp.UniqueVoters, tp.UpVotes, tp.DownVotes, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, tp.CreationDate
// FROM TopPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) JOIN UserBadges ub ON u.Id = ub.UserId ORDER BY tp.ViewCount DESC;
//
// RankByUser and the LIMIT read only base columns, so the ten posts are picked first and the comment x vote product is driven for those alone.
fn q26811(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, view_count, .. } = &db.post;
    let fw = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first = drain((&fw).filt(|(_, n)| n == 1).map(|((p, _), _)| p).select(Ident::<Post>::new().and(view_count.opt())));
    let top = top_n(first, |&(_, (p, w))| (w.is_none(), Reverse(w), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|(_, (p, _))| p).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let v = drain((&s).and((&uv).opt()).and(owner_user.select(user_badge_classes(db))));
    rows(v.into_iter().map(|(p, ((a, n), b))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::I(a[0]), V::I(n.unwrap_or(0)), V::I(a[1]), V::I(a[2])]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH UserPostStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews, COUNT(c.Id) AS TotalComments,
//        COALESCE(SUM(b.Class), 0) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// ExpandedUserStatistics AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, TotalComments, TotalBadges,
//        RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStatistics),
// FilteredUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, TotalComments, TotalBadges, ScoreRank, PostRank
//     FROM ExpandedUserStatistics WHERE TotalPosts >= 10 AND TotalViews >= 100)
// SELECT fu.DisplayName, fu.TotalPosts, fu.TotalQuestions, fu.TotalAnswers, fu.TotalScore, fu.TotalViews, fu.TotalComments, fu.TotalBadges,
//        CONCAT('Rank by Score: ', fu.ScoreRank, ', Rank by Posts: ', fu.PostRank) AS RankDetails
// FROM FilteredUsers fu ORDER BY TotalScore DESC, TotalViews DESC;
fn q27203(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let pc = Ident::<Post>::new().and(post_type_id).and(score).and(view_count.opt()).and(comments_of(db).opt());
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(pc).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 9], |a, (p, b)| {
            let mut a = a;
            if let Some(((((_, t), s), w), c)) = p {
                a[0] += 1;
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += s;
                a[4] += w.is_some() as i64;
                a[5] += w.unwrap_or(0);
                a[6] += c.is_some() as i64;
            }
            a[7] += b.unwrap_or(0);
            a
        });
    let w1 = whole(&s).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| (a[0] == 0, Reverse(a[3])), asc);
    let w2 = (&w1).window(rank, |((_, a), _): ((Id<User>, [i64; 9]), i64)| Reverse(a[0]), asc);
    let v = drain((&w2).filt(|(((_, a), _), _): (((Id<User>, [i64; 9]), i64), i64)| a[0] >= 10 && a[4] > 0 && a[5] >= 100));
    rows(v.into_iter().map(|(_, (((u, a), sr), pr))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[5]), V::I(a[6]), V::I(a[7])];
        f.push(V::Owned(format!("Rank by Score: {sr}, Rank by Posts: {pr}")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.AnswerCount, P.CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2), 0) AS Upvotes,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3), 0) AS Downvotes,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank, P.OwnerUserId FROM Posts P WHERE P.PostTypeId = 1),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalQuestions, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
//        SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers, SUM(COALESCE(P.CommentCount, 0)) AS TotalComments, SUM(COALESCE(RP.Upvotes, 0)) AS TotalUpvotes,
//        SUM(COALESCE(RP.Downvotes, 0)) AS TotalDownvotes FROM Users U LEFT JOIN RankedPosts RP ON U.Id = RP.OwnerUserId LEFT JOIN Posts P ON P.OwnerUserId = U.Id
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT U.UserId, U.DisplayName, U.Reputation, U.TotalQuestions, U.TotalViews, U.TotalAnswers, U.TotalComments, U.TotalUpvotes, U.TotalDownvotes,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC, U.TotalUpvotes DESC) AS UserRank FROM UserStats U)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.TotalQuestions, TU.TotalViews, TU.TotalAnswers, TU.TotalComments, TU.TotalUpvotes, TU.TotalDownvotes
// FROM TopUsers TU WHERE TU.UserRank <= 10 ORDER BY TU.UserRank;
fn q27951(db: &'static So) -> String {
    let Post { post_type_id, view_count, answer_count, comment_count, .. } = &db.post;
    let ud = post_updown(db, db.post.with(post_type_id.eq(1)));
    let rp = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(&ud);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(rp.opt().and(posts_of(db).select(view_count.opt().and(answer_count.opt()).and(comment_count)).opt()))
        .fold([0i64; 5], |a, (r, p)| {
            let r = r.unwrap_or([0, 0]);
            let ((w, n), c) = p.unwrap_or(((None, None), 0));
            [a[0] + w.unwrap_or(0), a[1] + n.unwrap_or(0), a[2] + c, a[3] + r[0], a[4] + r[1]]
        });
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = top_n(drain((&s).and(&dp)), |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[3]), u), 10);
    rows(v.into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(p.Id) >= 5),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Date >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' GROUP BY b.UserId),
// FinalRanking AS (SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.PostCount, tu.TotalScore, COALESCE(ub.BadgeCount, 0) AS BadgeCount,
//        DENSE_RANK() OVER (ORDER BY tu.TotalScore DESC, tu.Reputation DESC) AS UserRank FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId)
// SELECT fr.UserId, fr.DisplayName, fr.Reputation, fr.PostCount, fr.TotalScore, fr.BadgeCount, fr.UserRank, rp.Title AS BestPostTitle, rp.CreationDate AS BestPostDate, rp.Score AS BestPostScore
// FROM FinalRanking fr LEFT JOIN RankedPosts rp ON fr.UserId = rp.OwnerUserId AND rp.Rank = 1 ORDER BY fr.UserRank;
fn q5943(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let bw = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(since)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let by_user: HashIdx<Id<User>, Id<Post>> = (&bw).filt(|(_, n)| n == 1).map(|(((p, _), _), _)| p).collect();
    let tu = db.post.with(creation_date.ge(since)).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let ub = db.badge.with((&db.badge.date).ge(since)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let x: HashIdx<Id<User>, (([i64; 2], Option<i64>), Option<Id<Post>>)> = (&tu).filt(|a| a[0] >= 5).and((&ub).opt()).and((&by_user).opt()).collect();
    let w = whole(&x).select(Ident::<User>::new().and(&x).and(&db.user.reputation)).window(dense_rank, |((_, ((a, _), _)), r)| (Reverse(a[1]), Reverse(r)), asc);
    rows(drain(&w).into_iter().map(|(_, (((u, ((a, b), p)), _), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b.unwrap_or(0)), V::I(r)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// FilteredPostHistory AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate, MIN(ph.CreationDate) AS FirstEditDate FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (4, 5, 24) GROUP BY ph.PostId),
// UserPostStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(pp.EditCount), 0) AS TotalEdits,
//        COUNT(DISTINCT ph.PostId) AS PostHistoryCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId
//     LEFT JOIN FilteredPostHistory pp ON p.Id = pp.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT u.DisplayName, u.Reputation, up.QuestionCount, up.TotalBounty, up.TotalEdits, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u
//     JOIN UserPostStatistics up ON u.Id = up.UserId WHERE up.QuestionCount > 0)
// SELECT UserRank, DisplayName, Reputation, QuestionCount, TotalBounty, TotalEdits FROM TopUsers WHERE UserRank <= 10 ORDER BY UserRank;
//
// UserRank reads Reputation among users with a question, so those users are ranked first and the vote x history product is driven for the top ten alone.
fn q28511(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let asks: MatSet<Id<User>> = db.post.with(post_type_id.eq(1)).select(owner_user).collect();
    let w = whole(&asks).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    type T = (Id<User>, i64);
    let tu: MatSet<T> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let fph = db.post_history.with(post_history_type_id.is_in([4, 5, 24])).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let q = || Ident::<Post>::new().with(post_type_id.eq(1));
    let prod = q().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).and((&fph).opt()).and(history_of(db).opt());
    let s = (&tu)
        .group_by(Same::<T>::new())
        .select(Same::<T>::new().map(|(u, _): T| u).select(posts_of(db).select(prod)))
        .fold([0i64; 2], |a, (((_, b), e), _)| [a[0] + b.flatten().unwrap_or(0), a[1] + e.unwrap_or(0)]);
    let qc = (&tu).group_by(Same::<T>::new()).select(Same::<T>::new().map(|(u, _): T| u).select(posts_of(db).select(q()))).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and(&qc));
    rows(v.into_iter().map(|((u, r), (a, n))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, p.PostTypeId),
// ClosedPosts AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS CloseDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// PostWithCloseInfo AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.Rank, cp.CloseDate,
//        (SELECT COUNT(*) FROM PostHistory WHERE PostId = rp.PostId AND PostHistoryTypeId = 16) AS CommunityOwnedCount,
//        CASE WHEN cp.CloseDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT pwci.PostId, pwci.Title, pwci.Score, pwci.ViewCount, pwci.Rank, pwci.CloseDate, pwci.PostStatus, pwci.CommunityOwnedCount,
//        CASE WHEN pwci.PostStatus = 'Closed' AND pwci.CommunityOwnedCount > 0 THEN 'Community Owned and Closed' WHEN pwci.PostStatus = 'Closed' THEN 'Closed' ELSE 'Active' END AS DetailedStatus
// FROM PostWithCloseInfo pwci WHERE pwci.Rank <= 5 ORDER BY pwci.Rank, pwci.Score DESC;
//
// CommentCount and Upvotes are never read, and the GROUP BY gives one row per post whatever joins in, so the comment x vote product is not driven.
fn q22976(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    type T = (Id<Post>, i64);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), r)| (p, r)).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(post_history_type_id.and(hd)).fold(i64::MIN, |m, (t, d)| if t == 10 { m.max(d) } else { m });
    let co = db.post_history.with(post_history_type_id.eq(16)).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let p = || Same::<T>::new().map(|(p, _): T| p);
    let v = drain((&tp).select(Same::<T>::new().and(p().select(&cp).opt()).and(p().select(&co).opt())));
    rows(v.into_iter().map(|(_, (((p, r), c), n))| {
        let closed = c.map_or(false, |d| d != i64::MIN);
        let n = n.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(r), c.map_or(V::Null, tmax), V::S(if closed { "Closed" } else { "Open" }), V::I(n)]);
        f.push(V::S(if closed && n > 0 { "Community Owned and Closed" } else if closed { "Closed" } else { "Active" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS Author, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, p.PostTypeId),
// FilteredPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, Author, Rank, CommentCount FROM RankedPosts WHERE Rank <= 10),
// RecentHistory AS (SELECT ph.PostId, ph.CreationDate AS HistoryDate, M.HistoryType, M.UserDisplayName, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph INNER JOIN (SELECT PostId, UserDisplayName, CASE WHEN PostHistoryTypeId IN (10, 11) THEN 'Closed/Reopened' WHEN PostHistoryTypeId IN (12, 13) THEN 'Deleted/Undeleted'
//        ELSE 'Edited' END AS HistoryType FROM PostHistory WHERE CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month')) M ON ph.PostId = M.PostId)
// SELECT fp.PostId, fp.Title, fp.CreationDate AS PostCreationDate, fp.ViewCount, fp.Score, fp.Author AS PostAuthor, fp.CommentCount, rh.HistoryDate, rh.HistoryType, rh.UserDisplayName AS Editor
// FROM FilteredPosts fp LEFT JOIN RecentHistory rh ON fp.PostId = rh.PostId AND rh.HistoryRank = 1 WHERE fp.ViewCount > 100 ORDER BY fp.Score DESC, fp.CreationDate DESC;
//
// RecentHistory pairs each history row of a post with each of its recent rows (M); the latest pair is taken, ties going to the smallest M id (the SQL leaves them open).
fn q55(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let fp = || (&tp).with(view_count.gt(100));
    let cc = post_comment_count(db, fp());
    let PostHistory { creation_date: hd, origid, .. } = &db.post_history;
    let recent = history_of(db).select(Ident::<PostHistory>::new().with(hd.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1))));
    let rh = fp()
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(hd).and(recent.select(Ident::<PostHistory>::new().and(origid))))
        .fold(None::<(i64, Reverse<i64>, Id<PostHistory>)>, |b, (d, (m, i))| {
            let x = (d, Reverse(i), m);
            Some(match b { Some(y) if (y.0, y.1) >= (x.0, x.1) => y, _ => x })
        });
    let v = drain((&cc).and((&rh).opt()));
    rows(v.into_iter().map(|(p, (c, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.push(V::I(c));
        f.extend(match h.flatten() {
            Some((d, _, m)) => {
                let t = db.post_history.post_history_type_id.get(m).unwrap();
                [V::T(d), V::S(if matches!(t, 10 | 11) { "Closed/Reopened" } else if matches!(t, 12 | 13) { "Deleted/Undeleted" } else { "Edited" }), ostr(db.post_history.user_display_name.get(m))]
            }
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers, COALESCE(SUM(CASE WHEN p.PostTypeId = 4 THEN 1 ELSE 0 END), 0) AS TotalTagWikis,
//        AVG(COALESCE(p.ViewCount, 0)) AS AvgViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// BountyStats AS (SELECT v.UserId, COUNT(v.Id) AS TotalBountiesGiven, SUM(v.BountyAmount) AS TotalBountyAmount FROM Votes v WHERE v.VoteTypeId = 8 GROUP BY v.UserId),
// CombinedStats AS (SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalTagWikis, ups.AvgViews, COALESCE(bs.TotalBountiesGiven, 0) AS TotalBountiesGiven,
//        COALESCE(bs.TotalBountyAmount, 0) AS TotalBountyAmount FROM UserPostStats ups LEFT JOIN BountyStats bs ON ups.UserId = bs.UserId),
// RankedUsers AS (SELECT *, RANK() OVER (ORDER BY TotalPosts DESC, AvgViews DESC) AS UserRank FROM CombinedStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalTagWikis, AvgViews, TotalBountiesGiven, TotalBountyAmount,
//        CASE WHEN TotalBountyAmount > 100 THEN 'High Bounty Giver' WHEN TotalBountyAmount BETWEEN 50 AND 100 THEN 'Moderate Bounty Giver' ELSE 'Low Bounty Giver' END AS BountyCategory, UserRank
// FROM RankedUsers WHERE UserRank <= 10 ORDER BY UserRank;
fn q20548(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some((t, w)) => [a[0] + 1, a[1] + 1, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + (t == 4) as i64, a[5] + w.unwrap_or(0)],
        None => [a[0] + 1, a[1], a[2], a[3], a[4], a[5]],
    });
    let Vote { user, vote_type_id, bounty_amount, .. } = &db.vote;
    let bs = db.vote.with(vote_type_id.eq(8)).group_by(user).select(bounty_amount.opt()).fold([0i64; 3], |a, b| [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]);
    let mean = |a: &[i64; 6]| a[5] as f64 / a[0] as f64;
    let x: HashIdx<Id<User>, ([i64; 6], Option<[i64; 3]>)> = (&s).and((&bs).opt()).collect();
    let w = whole(&x).select(Ident::<User>::new().and(&x)).window(rank, |(_, (a, _))| (Reverse(a[1]), Reverse(fkey(mean(&a)))), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, (a, b)), r))| {
        let (n, amt) = match b {
            Some(b) => (b[0], if b[1] == 0 { 0 } else { b[2] }),
            None => (0, 0),
        };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::F(mean(&a)), V::I(n), V::I(amt)]);
        f.push(V::S(if amt > 100 { "High Bounty Giver" } else if amt >= 50 { "Moderate Bounty Giver" } else { "Low Bounty Giver" }));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
//        SUM(p.ViewCount) AS TotalViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName
//     ORDER BY QuestionCount DESC, TotalViews DESC LIMIT 10)
// SELECT ru.DisplayName AS TopUser, rp.PostId, rp.Title AS PostTitle, rp.Body AS PostBody, rp.CreationDate AS PostCreationDate, rp.ViewCount AS PostViewCount, rp.CommentCount AS PostCommentCount,
//        rp.Score AS PostScore
// FROM RankedPosts rp JOIN TopUsers ru ON rp.Rank = 1 AND rp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = ru.UserId) ORDER BY ru.DisplayName, rp.CreationDate DESC;
//
// `rp.PostId IN (posts of ru)` makes rp the user's newest question, so that is picked first and its comment x vote product driven alone.
fn q25941(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, _)| match p {
            Some((t, w)) => [a[0] + (t == 1) as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)],
            None => a,
        });
    let tu = rel(top_n(drain(&tu), |&(u, a)| (Reverse(a[0]), a[1] == 0, Reverse(a[2]), u), 10));
    let fw = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date).and(owner_user)).window(row_number, |((p, d), _)| (Reverse(d), p), asc);
    let latest: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&fw).filt(|(_, n)| n == 1).map(|(((p, _), u), _)| (u, p)).collect();
    let rp: MatSet<(Id<User>, Id<Post>)> = (&tu).map(|(u, _)| u).select(&latest).collect();
    let pp = (&rp).map(|(_, p)| p);
    let s = (&rp)
        .group_by(Same::<(Id<User>, Id<Post>)>::new())
        .select((&pp).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|((u, p), a)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "body", "created", "views"]));
        f.extend([V::I(a[0]), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerDisplayName, p.Score, p.ViewCount, COUNT(a.Id) AS AnswerCount, RANK() OVER (PARTITION BY p.Id ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerDisplayName, p.Score, p.ViewCount),
// PostHistoryWithTypes AS (SELECT ph.PostId, MAX(CASE WHEN pht.Name = 'Post Closed' THEN ph.CreationDate END) AS LastCloseDate, MAX(CASE WHEN pht.Name = 'Post Reopened' THEN ph.CreationDate END) AS LastReopenDate
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId),
// UserVoteStats AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVoteCount
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId)
// SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.Score, rp.ViewCount, rp.AnswerCount, phwt.LastCloseDate, phwt.LastReopenDate, uvs.UpVoteCount, uvs.DownVoteCount,
//        CASE WHEN phwt.LastCloseDate IS NOT NULL AND phwt.LastReopenDate IS NULL THEN 'Closed' WHEN phwt.LastReopenDate IS NOT NULL THEN 'Reopened' ELSE 'Active' END AS PostStatus
// FROM RankedPosts rp LEFT JOIN PostHistoryWithTypes phwt ON rp.Id = phwt.PostId LEFT JOIN UserVoteStats uvs ON rp.Id = uvs.PostId WHERE rp.Score > 10 ORDER BY rp.Score DESC, rp.CreationDate ASC;
fn q31367(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let qs = db.post.with(post_type_id.eq(1).and(score.gt(10)));
    let ac = qs.group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ph = db.post_history.group_by(&db.post_history.post).select(htype_name(db).and(&db.post_history.creation_date)).fold([i64::MIN; 2], |a, (n, d)| {
        [if n == "Post Closed" { a[0].max(d) } else { a[0] }, if n == "Post Reopened" { a[1].max(d) } else { a[1] }]
    });
    let uv = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let v = drain((&ac).and((&ph).opt()).and((&uv).opt()));
    rows(v.into_iter().map(|(p, ((n, h), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner_name", "score", "views"]);
        f.push(V::I(n));
        let (c, r) = h.map_or((i64::MIN, i64::MIN), |h| (h[0], h[1]));
        f.extend([tmax(c), tmax(r)]);
        f.extend(match u {
            Some(u) => [V::I(u[0]), V::I(u[1])],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if c != i64::MIN && r == i64::MIN { "Closed" } else if r != i64::MIN { "Reopened" } else { "Active" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVotes, p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p WHERE p.PostTypeId = 1),
// AggregateVotes AS (SELECT PostId, SUM(UpVotes) AS TotalUpVotes, SUM(DownVotes) AS TotalDownVotes FROM RankedPosts GROUP BY PostId),
// PostHistoryInfo AS (SELECT ph.PostId, ph.UserDisplayName, ph.CreationDate, ph.Comment, p.Title FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (10, 11)),
// FinalReport AS (SELECT r.PostId, r.Title, a.TotalUpVotes, a.TotalDownVotes, p.UserDisplayName AS LastUser, p.CreationDate AS LastEditDate FROM RankedPosts r
//     JOIN AggregateVotes a ON r.PostId = a.PostId LEFT JOIN PostHistoryInfo p ON r.PostId = p.PostId WHERE r.RowNum <= 100)
// SELECT f.PostId, f.Title, f.TotalUpVotes, f.TotalDownVotes, COALESCE(f.LastUser, 'No edits') AS LastEditedBy, f.LastEditDate,
//        CASE WHEN f.TotalDownVotes > f.TotalUpVotes THEN 'Needs Attention' WHEN f.TotalUpVotes > f.TotalDownVotes THEN 'Popular' ELSE 'Neutral' END AS PostStatus
// FROM FinalReport f ORDER BY f.TotalUpVotes DESC, f.LastEditDate DESC;
fn q31244(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(p, d)| (Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let v = drain((&post_updown(db, &tp)).and(closes.opt()));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [V::S(db.post_history.user_display_name.get(h).unwrap_or("No edits")), V::T(db.post_history.creation_date.get(h).unwrap())],
            None => [V::S("No edits"), V::Null],
        });
        f.push(V::S(if a[1] > a[0] { "Needs Attention" } else if a[0] > a[1] { "Popular" } else { "Neutral" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViews, COUNT(DISTINCT P.Tags) AS UniqueTags FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UBC.GoldBadges, 0) AS GoldBadges, COALESCE(UBC.SilverBadges, 0) AS SilverBadges, COALESCE(UBC.BronzeBadges, 0) AS BronzeBadges,
//        PS.TotalPosts, PS.TotalScore, PS.AvgViews, PS.UniqueTags FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId WHERE U.Reputation > 1000),
// RankedUsers AS (SELECT *, RANK() OVER (ORDER BY TotalScore DESC, TotalPosts DESC) AS ScoreRank FROM TopUsers),
// FinalOutput AS (SELECT UserId, DisplayName, GoldBadges, SilverBadges, BronzeBadges, TotalPosts, TotalScore, AvgViews, UniqueTags, ScoreRank FROM RankedUsers WHERE ScoreRank <= 10)
// SELECT CONCAT(DisplayName, ' (Gold: ', GoldBadges, ', Silver: ', SilverBadges, ', Bronze: ', BronzeBadges, ')') AS UserInfo, TotalPosts, TotalScore, AvgViews, UniqueTags
// FROM FinalOutput ORDER BY ScoreRank;
fn q415(db: &'static So) -> String {
    let Post { owner_user, score, view_count, tags_str, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let ut = db.post.group_by(owner_user).select(tags_str).count_distinct();
    type X = ([i64; 4], Option<([i64; 4], Option<i64>)>);
    let x: HashIdx<Id<User>, X> = db.user.with((&db.user.reputation).gt(1000)).select(user_badge_classes(db).and((&ps).and((&ut).opt()).opt())).collect();
    let w = whole(&x).select(Ident::<User>::new().and(&x)).window(rank, |(_, (_, p)): (Id<User>, X)| p.map_or((true, Reverse(0), Reverse(0)), |(a, _)| (false, Reverse(a[1]), Reverse(a[0]))), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, (b, p)), _))| {
        let mut f = vec![V::Owned(format!("{} (Gold: {}, Silver: {}, Bronze: {})", db.user.display_name.get(u).unwrap(), b[1], b[2], b[3]))];
        f.extend(match p {
            Some((a, t)) => [V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(t.unwrap_or(0))],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        COALESCE(z.Reputation, 0) AS UserReputation, COALESCE(COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2), 0) AS UpVotes, COALESCE(COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Users z ON p.OwnerUserId = z.Id LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, z.Reputation),
// FilteredPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.OwnerUserId, rp.UserReputation, rp.UpVotes, rp.DownVotes FROM RankedPosts rp
//     WHERE rp.rn = 1 AND rp.PostTypeId IN (1, 2) AND rp.UserReputation > 100),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount, COUNT(CASE WHEN ph.PostHistoryTypeId IN (12) THEN 1 END) AS DeletionCount
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT fp.Title, fp.CreationDate, CONCAT('Owner ID: ', fp.OwnerUserId, ' | Reputation: ', fp.UserReputation) AS UserDetails, fp.UpVotes - fp.DownVotes AS VoteBalance,
//        COALESCE(pHS.CloseReopenCount, 0) AS CloseReopenCount, COALESCE(pHS.DeletionCount, 0) AS DeletionCount,
//        CASE WHEN COALESCE(pHS.DeletionCount, 0) > 0 THEN 'Deleted' ELSE 'Active' END AS PostStatus
// FROM FilteredPosts fp LEFT JOIN PostHistoryStats pHS ON fp.Id = pHS.PostId ORDER BY fp.CreationDate DESC FETCH FIRST 100 ROWS ONLY;
fn q2181(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, owner_user_id, .. } = &db.post;
    let fw = db.post.group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&fw).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let fp = (&first).with(post_type_id.is_in([1, 2])).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100))));
    let v = top_n(drain(fp.select(creation_date)), |&(p, d)| (Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ph = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + (t == 12) as i64]);
    let v = drain((&post_updown(db, &tp)).and((&ph).opt()));
    rows(v.into_iter().map(|(p, (a, h))| {
        let h = h.unwrap_or([0, 0]);
        let u = owner_user.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created"]);
        f.push(V::Owned(format!("Owner ID: {} | Reputation: {}", owner_user_id.get(p).unwrap(), db.user.reputation.get(u).unwrap())));
        f.extend([V::I(a[0] - a[1]), V::I(h[0]), V::I(h[1]), V::S(if h[1] > 0 { "Deleted" } else { "Active" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// BadgeSummary AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalQuestions, ua.TotalAnswers, ua.TotalBounty, ua.TotalUpvotes, ua.TotalDownvotes, COALESCE(bs.TotalBadges, 0) AS TotalBadges,
//        COALESCE(bs.GoldBadges, 0) AS GoldBadges, COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges FROM UserActivity ua LEFT JOIN BadgeSummary bs ON ua.UserId = bs.UserId)
// SELECT *, (TotalBounty + TotalUpvotes * 5 - TotalDownvotes * 3) AS EngagementScore, RANK() OVER (ORDER BY (TotalBounty + TotalUpvotes * 5 - TotalDownvotes * 3) DESC) AS Rank
// FROM TopUsers WHERE TotalPosts > 0 ORDER BY EngagementScore DESC LIMIT 10;
fn q2061(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let pv = post_type_id.and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt());
    let s = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(pv)).fold([0i64; 6], |a, (t, v)| {
        let (vt, b) = v.map_or((None, None), |(t, b)| (Some(t), b));
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.unwrap_or(0), a[4] + (vt == Some(2)) as i64, a[5] + (vt == Some(3)) as i64]
    });
    let e = |a: &[i64; 6]| a[3] + a[4] * 5 - a[5] * 3;
    let x: HashIdx<Id<User>, ([i64; 6], [i64; 4])> = (&s).and(user_badge_classes(db)).collect();
    let w = whole(&x).select(Ident::<User>::new().and(&x)).window(rank, |(_, (a, _))| Reverse(e(&a)), asc);
    let v = top_n(drain(&w), |&(_, ((u, (a, _)), _))| (Reverse(e(&a)), u), 10);
    rows(v.into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.extend([V::I(e(&a)), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, P.AcceptedAnswerId, COUNT(CM.Id) AS CommentCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments CM ON P.Id = CM.PostId WHERE P.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName, P.AcceptedAnswerId),
// AcceptedAnswers AS (SELECT P.Id AS AnswerId, P.ParentId AS QuestionId, P.OwnerUserId, U.DisplayName AS AnswerOwner FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.AcceptedAnswerId IS NOT NULL),
// PostHistoryDetails AS (SELECT PH.PostId, PH.UserId, PHT.Name AS PostHistoryType, PH.CreationDate FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
//     WHERE PH.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.ViewCount, RP.Score, RP.CommentCount, UR.DisplayName AS TopUser, UR.Reputation AS TopUserReputation, PH.UserId AS HistoryUser,
//        PH.PostHistoryType, PH.CreationDate AS HistoryDate, A.AnswerOwner, A.AnswerId
// FROM RecentPosts RP LEFT JOIN UserReputation UR ON UR.Rank = 1 LEFT JOIN PostHistoryDetails PH ON RP.PostId = PH.PostId LEFT JOIN AcceptedAnswers A ON A.QuestionId = RP.PostId
// WHERE RP.Score > 10 ORDER BY RP.ViewCount DESC, RP.CreationDate DESC LIMIT 100;
//
// `ON UR.Rank = 1` names only UR, so each post is crossed with the top-ranked users.
fn q1057(db: &'static So) -> String {
    let Post { creation_date, score, view_count, accepted_answer_id, owner_user, parent, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = db.post.with(creation_date.gt(add_days(t0, -30)).and(score.gt(10)));
    let cc = post_comment_count(db, rp);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let ur: MatSet<Id<User>> = (&w).filt(|(_, r)| r == 1).map(|((u, _), _)| u).collect();
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let phd = history_of(db).select(Ident::<PostHistory>::new().with(hd.gt(add_years(t0, -1))));
    let aa: HashIdx<Id<Post>, Id<Post>> = db.post.with(accepted_answer_id).with(owner_user).select(parent).inv().collect();
    let left = (&cc).and(phd.opt()).and((&aa).opt());
    let mut v = Vec::new();
    left.cross(&ur).drive(|(p, _), (((c, h), a), u)| v.push((p, c, h, a, u)));
    let v = top_n(v, |&(p, _, h, a, u)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()), p, h, a, u)
    }, 100);
    rows(v.into_iter().map(|(p, c, h, a, u)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), user_col(db, u, "name"), user_col(db, u, "rep")]);
        f.extend(match h {
            Some(h) => [oint(db.post_history.user_id.get(h)), V::S(htype_name(db).get(h).unwrap()), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match a {
            Some(a) => [user_col(db, owner_user.get(a).unwrap(), "name"), V::I(db.post.origid.get(a).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// RecentPostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// ConsolidatedPostData AS (SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.OwnerDisplayName, rp.LinkCount, COALESCE(phh.PostHistoryTypeId, 0) AS LastHistoryType
//     FROM RankedPosts r LEFT JOIN (SELECT p.Id AS PostId, COUNT(pl.Id) AS LinkCount FROM Posts p JOIN PostLinks pl ON p.Id = pl.PostId GROUP BY p.Id) rp ON r.PostId = rp.PostId
//     LEFT JOIN RecentPostHistory phh ON r.PostId = phh.PostId AND phh.HistoryRank = 1)
// SELECT cp.Title, cp.OwnerDisplayName, cp.CreationDate, cp.Score, cp.ViewCount, cp.LinkCount,
//        CASE WHEN cp.LastHistoryType = 10 THEN 'Closed' WHEN cp.LastHistoryType = 11 THEN 'Reopened' WHEN cp.LastHistoryType = 12 THEN 'Deleted' ELSE 'Active' END AS Status
// FROM ConsolidatedPostData cp WHERE cp.ViewCount > (SELECT AVG(ViewCount) FROM Posts) ORDER BY cp.Score DESC, cp.CreationDate DESC FETCH FIRST 10 ROWS ONLY;
//
// The latest history row per post breaks ties on CreationDate by the larger id (the SQL leaves them open).
fn q33460(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, creation_date, owner_user, .. } = &db.post;
    let (n, s) = db.post.select(view_count).fold_flat((0i64, 0i64), |(n, s), w| (n + 1, s + w));
    let mean = s as f64 / n as f64;
    let cp = db.post.with(post_type_id.eq(1)).with(view_count.filt(move |w: i64| w as f64 > mean));
    let top = top_n(drain(cp.select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let lc = (&tp).group_by(Ident::<Post>::new()).select(links_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let lw = db.post_history.with(post_history_type_id.is_in([10, 11, 12])).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (d, h), desc);
    let last: HashIdx<Id<Post>, Id<PostHistory>> = (&lw).filt(|(_, n)| n == 1).map(|((h, _), _)| h).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and((&lc).opt()).and((&last).select(post_history_type_id).opt())));
    rows(v.into_iter().map(|(_, ((p, l), h))| {
        let t = h.unwrap_or(0);
        let mut f = vec![post_fields(db, p, &["title"]).pop().unwrap(), V::S(owner_user.get(p).map_or("Community", |u| db.user.display_name.get(u).unwrap()))];
        f.extend(post_fields(db, p, &["created", "score", "views"]));
        f.extend([oint(l), V::S(match t { 10 => "Closed", 11 => "Reopened", 12 => "Deleted", _ => "Active" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// PostVoteStats AS (SELECT pv.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVotes
//     FROM Votes pv JOIN VoteTypes vt ON pv.VoteTypeId = vt.Id GROUP BY pv.PostId),
// PostBadgeStats AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// FinalStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerUserId, rp.OwnerDisplayName, rp.Score, rp.ViewCount, pvs.UpVotes, pvs.DownVotes, COALESCE(bs.GoldBadges, 0) AS GoldBadges,
//        COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges, rp.Rank FROM RankedPosts rp LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId
//     LEFT JOIN PostBadgeStats bs ON rp.OwnerUserId = bs.UserId)
// SELECT PostId, Title, CreationDate, OwnerUserId, OwnerDisplayName, Score, ViewCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, Rank FROM FinalStats WHERE Rank <= 100 ORDER BY Rank;
fn q9960(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let w = whole(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    type T = (Id<Post>, i64);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 100).map(|(((p, _), _), r)| (p, r)).collect();
    let uv = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select((&uv).opt().and(owner_user.select(user_badge_classes(db)))))));
    rows(v.into_iter().map(|(_, ((p, r), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner_id", "owner", "score", "views"]);
        f.extend(match u {
            Some(u) => [V::I(u[0]), V::I(u[1])],
            None => [V::Null, V::Null],
        });
        f.extend([V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
//        SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, TotalBounty, ROW_NUMBER() OVER (ORDER BY Reputation DESC, QuestionCount DESC) AS Rank FROM UserStats),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(ph.EditCount, 0) AS EditCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) pc ON p.Id = pc.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId) ph ON p.Id = ph.PostId)
// SELECT u.UserId, u.DisplayName, u.Reputation, p.Title, p.CreationDate, p.Score, p.ViewCount, p.CommentCount, p.EditCount
// FROM TopUsers u JOIN PostActivity p ON u.UserId = p.OwnerUserId WHERE u.Rank <= 10 ORDER BY u.Reputation DESC, p.Score DESC;
fn q348(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9])));
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt()).fold(0i64, |n, x| n + (x.map(|x| x.0) == Some(1)) as i64);
    let tu = top_n(drain(&qc), |&(u, q)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(q), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ps: MatSet<Id<Post>> = (&tu).select(posts_of(db)).collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let ec = (&ps).group_by(Ident::<Post>::new()).select(edits.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&post_comment_count(db, &ps)).and(&ec).and(&db.post.owner_user));
    rows(v.into_iter().map(|(p, ((c, e), u))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(e)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(u.Views) AS AvgViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, QuestionCount, AnswerCount, AvgViews, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStatistics),
// ActiveUsers AS (SELECT u.UserId, u.DisplayName, u.Reputation, u.TotalPosts, u.AvgViews, COUNT(c.Id) AS CommentsMade FROM TopUsers u LEFT JOIN Comments c ON u.UserId = c.UserId
//     GROUP BY u.UserId, u.DisplayName, u.Reputation, u.TotalPosts, u.AvgViews),
// PostsWithVotes AS (SELECT p.Id AS PostId, p.Title, p.Score, v.VoteTypeId, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.Score, v.VoteTypeId),
// PostAnalysis AS (SELECT p.PostId, p.Title, SUM(CASE WHEN p.VoteTypeId = 2 THEN p.VoteCount ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN p.VoteTypeId = 3 THEN p.VoteCount ELSE 0 END) AS TotalDownVotes
//     FROM PostsWithVotes p GROUP BY p.PostId, p.Title)
// SELECT au.DisplayName, au.Reputation, au.TotalPosts, au.AvgViews, pa.Title, pa.TotalUpVotes, pa.TotalDownVotes
// FROM ActiveUsers au JOIN PostAnalysis pa ON au.UserId = pa.PostId WHERE au.CommentsMade > 5 ORDER BY au.Reputation DESC, pa.TotalUpVotes DESC;
//
// `au.UserId = pa.PostId` compares a user id with a post id, so it goes through the raw ids. Summing the per-type counts back per post is the per-post vote count by type.
fn q8205(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select((&db.user.views).and(posts_of(db).opt())).fold([0i64; 2], |a, (w, _)| [a[0] + 1, a[1] + w]);
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cm = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let Vote { post, vote_type_id, .. } = &db.vote;
    let pwv = db.vote.group_by(post.and(vote_type_id)).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let pwv = rel(drain(&pwv));
    let pa = (&pwv).group_by(Same::<((Id<Post>, i64), i64)>::new().map(|((p, _), _)| p)).select(Same::<((Id<Post>, i64), i64)>::new()).fold([0i64; 2], |a, ((_, t), n)| {
        [a[0] + if t == 2 { n } else { 0 }, a[1] + if t == 3 { n } else { 0 }]
    });
    let pid: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain((&cm).filt(|n| n > 5).and(&us).and(&dp).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and((&pa).opt()))));
    rows(v.into_iter().map(|(u, (((_, a), n), (p, x)))| {
        let x = x.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), avg(a[1], a[0]), title(db, p), V::I(x[0]), V::I(x[1])]);
        row(f)
    }))
}

// Rewritten (rewrites/3822.sql): the final ORDER BY tie-broken on PS.PostId, U.UserId.
// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= DATE '2023-01-01' AND P.Score IS NOT NULL),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, U.Views, (U.UpVotes - U.DownVotes) AS VoteBalance FROM Users U WHERE U.Reputation >= 1000),
// PostSummary AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, COUNT(CASE WHEN C.UserId IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVoteCount, COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVoteCount FROM RankedPosts RP LEFT JOIN Comments C ON RP.PostId = C.PostId
//     LEFT JOIN Votes V ON RP.PostId = V.PostId GROUP BY RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName)
// SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.ViewCount, PS.OwnerDisplayName, PS.CommentCount, U.VoteBalance, RP.Rank,
//        CASE WHEN PS.CommentCount > 10 THEN 'Hot' WHEN PS.Score > 50 THEN 'Popular' ELSE 'Normal' END AS PopularityRank
// FROM PostSummary PS JOIN UserStats U ON PS.OwnerDisplayName = U.DisplayName JOIN RankedPosts RP ON PS.PostId = RP.PostId WHERE PS.Score > 0
// ORDER BY PS.Score DESC, PS.ViewCount DESC, PS.PostId, U.UserId LIMIT 50;
fn q3822(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, post_type_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(date(2023, 1, 1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    type T = ((Id<Post>, i64), i64);
    let rp: MatSet<T> = (&w).filt(|((_, s), _): T| s > 0).collect();
    let by_name: HashIdx<Str, Id<User>> = db.user.with((&db.user.reputation).ge(1000)).select(&db.user.display_name).inv().collect();
    let pp = || Same::<T>::new().map(|((p, _), _): T| p);
    let j = (&rp).select(Same::<T>::new().and(pp().select(owner_user.select(&db.user.display_name).select(&by_name))));
    let v = top_n(drain(j), |&(_, (((p, s), _), u))| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p, u)
    }, 50);
    let tp = rel(v.into_iter().map(|x| x.1).collect());
    type R = (T, Id<User>);
    let cc = (&tp)
        .group_by(Same::<R>::new())
        .select(Same::<R>::new().map(|(((p, _), _), _): R| p).select(comments_of(db).select((&db.comment.user_id).opt()).opt().and(votes_of(db).opt())))
        .fold(0i64, |n, (c, _)| n + c.flatten().is_some() as i64);
    rows(drain(&cc).into_iter().map(|((((p, s), r), u), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(db.user.up_votes.get(u).unwrap() - db.user.down_votes.get(u).unwrap()), V::I(r)]);
        f.push(V::S(if c > 10 { "Hot" } else if s > 50 { "Popular" } else { "Normal" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT ub.UserId, ub.DisplayName, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.Questions, 0) AS Questions, COALESCE(ps.Answers, 0) AS Answers,
//        COALESCE(ps.TotalViews, 0) AS TotalViews, COALESCE(ps.TotalScore, 0) AS TotalScore, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
//     FROM UserBadges ub LEFT JOIN PostStats ps ON ub.UserId = ps.OwnerUserId),
// Ranks AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalViews, TotalScore, BadgeCount, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY TotalScore DESC, TotalViews DESC) AS ScoreRank FROM UserPerformance)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalViews, TotalScore, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, ScoreRank FROM Ranks WHERE ScoreRank <= 10 ORDER BY ScoreRank;
fn q7328(db: &'static So) -> String {
    let ups = user_posts(db);
    let x: HashIdx<Id<User>, ([i64; 10], [i64; 4])> = (&ups).and(user_badge_classes(db)).collect();
    let w = whole(&x).select(Ident::<User>::new().and(&x)).window(rank, |(_, (a, _))| (Reverse(a[4]), Reverse(a[6])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(a[4])]);
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UsersWithBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostScores AS (SELECT p.Id AS PostId, p.Score, p.PostTypeId, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes,
//        CASE WHEN p.PostTypeId = 1 THEN 'Question' WHEN p.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType FROM Posts p
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId),
// TopPosts AS (SELECT ps.PostId, ps.Score + (ps.UpVotes - ps.DownVotes) AS NetScore, ROW_NUMBER() OVER (PARTITION BY ps.PostType ORDER BY (ps.Score + (ps.UpVotes - ps.DownVotes)) DESC) AS Rank
//     FROM PostScores ps WHERE ps.PostType IN ('Question', 'Answer'))
// SELECT u.DisplayName, u.BadgeCount, u.GoldBadges, u.SilverBadges, u.BronzeBadges, tp.PostId, tp.NetScore, tp.Rank, COALESCE(ph.Comment, 'No comments') AS HistoryComment
// FROM UsersWithBadges u JOIN TopPosts tp ON u.UserId = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = tp.PostId)
// LEFT JOIN PostHistory ph ON ph.PostId = tp.PostId AND ph.PostHistoryTypeId IN (10, 11) WHERE u.BadgeCount > 0 AND tp.Rank <= 5 ORDER BY u.BadgeCount DESC, tp.NetScore DESC;
fn q24163(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let qa = || db.post.with(post_type_id.is_in([1, 2]));
    let ud = post_updown(db, qa());
    let w = qa().group_by(post_type_id).select(Ident::<Post>::new().and(score).and(&ud)).window(row_number, |((p, s), a)| (Reverse(s + a[0] - a[1]), p), asc);
    type T = (Id<Post>, i64, i64);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 5).map(|(((p, s), a), r)| (p, s + a[0] - a[1], r)).collect();
    let pp = || Same::<T>::new().map(|(p, _, _): T| p);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let v = drain((&tp).select(Same::<T>::new().and(pp().select(owner_user.select(Ident::<User>::new().and(user_badge_classes(db).filt(|b| b[0] > 0))))).and(pp().select(closes.opt()))));
    rows(v.into_iter().map(|(_, (((p, n, r), (u, b)), h))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(n), V::I(r), V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No comments"))]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, (SELECT COUNT(*) FROM Posts P WHERE P.OwnerUserId = U.Id) AS PostCount,
//        (SELECT COUNT(*) FROM Comments C WHERE C.UserId = U.Id) AS CommentCount, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U),
// PopularTags AS (SELECT T.TagName, T.Count, ROW_NUMBER() OVER (ORDER BY T.Count DESC) AS TagRank FROM Tags T),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, PT.Name AS PostType, COUNT(C.Id) AS TotalComments, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Posts P LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
//     GROUP BY P.Id, P.Title, P.ViewCount, P.Score, PT.Name),
// ClosedPostStats AS (SELECT PH.PostId, COUNT(*) AS CloseCount, COUNT(DISTINCT PH.UserId) AS UserCloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId)
// SELECT U.UserRank, U.DisplayName, U.Reputation, U.PostCount, U.CommentCount, T.TagName, PS.PostId, PS.Title, PS.ViewCount, PS.Score, PS.TotalComments, PS.TotalBounty,
//        COALESCE(CPS.CloseCount, 0) AS CloseCount, COALESCE(CPS.UserCloseCount, 0) AS UserCloseCount
// FROM RankedUsers U JOIN PopularTags T ON U.UserRank <= 5 JOIN PostStats PS ON U.UserId = PS.PostId LEFT JOIN ClosedPostStats CPS ON PS.PostId = CPS.PostId
// WHERE T.Count > 100 ORDER BY U.Reputation DESC, PS.ViewCount DESC;
//
// `ON U.UserRank <= 5` names only U, so the top users are crossed with the popular tags; `U.UserId = PS.PostId` goes through the raw ids.
fn q8072(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    type T = (Id<User>, i64);
    let tu: MatSet<T> = (&w).filt(|(_, n)| n <= 5).map(|((u, _), n)| (u, n)).collect();
    let pid: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let PostHistory { post, post_history_type_id, user_id, .. } = &db.post_history;
    let cps = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cpu = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(user_id).count_distinct();
    let uu = || Same::<T>::new().map(|(u, _): T| u);
    let pcount = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ccount = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ps = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let left = (&tu).select(Same::<T>::new().and(uu().select((&pcount).and(&ccount))).and(uu().select(&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&ps).and((&cps).and((&cpu).opt()).opt()))));
    let tags = db.tag.with((&db.tag.count).gt(100));
    let mut v = Vec::new();
    left.cross(tags).drive(|_, ((((u, r), (n, c)), ((p, a), x)), t)| v.push((u, r, n, c, t, p, a, x)));
    rows(v.into_iter().map(|(u, r, n, c, t, p, a, x)| {
        let x = x.map_or((0, 0), |(k, d)| (k, d.unwrap_or(0)));
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(n), V::I(c), V::S(db.tag.tag_name.get(t).unwrap())]);
        f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(x.0), V::I(x.1)]);
        row(f)
    }))
}

// WITH RankedPostData AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank, CASE WHEN p.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id),
// PostVoteDetails AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId)
// SELECT rpd.PostId, rpd.Title, rpd.CreationDate, rpd.Score, rpd.AnswerCount, rpd.OwnerDisplayName, rpd.PostRank, pvd.UpVotes, pvd.DownVotes, pvd.TotalVotes, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        CASE WHEN rpd.PostStatus = 'Closed' AND rpd.PostRank = 1 THEN 'Most Recent Closed Post' WHEN rpd.PostStatus = 'Open' AND rpd.PostRank = 1 THEN 'Most Recent Open Post' ELSE 'Other' END AS PostCategory
// FROM RankedPostData rpd JOIN PostVoteDetails pvd ON rpd.PostId = pvd.PostId LEFT JOIN UserBadges ub ON rpd.OwnerDisplayName = (SELECT DisplayName FROM Users u WHERE u.Id = ub.UserId)
// WHERE rpd.PostRank <= 5 AND (rpd.Score > 10 OR pvd.TotalVotes > 20) ORDER BY rpd.CreationDate DESC, pvd.UpVotes DESC;
fn q23505(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, closed_date, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    type T = (Id<Post>, i64);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), r)| (p, r)).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let named: HashIdx<Str, Id<User>> = db.user.with(&ub).select(&db.user.display_name).inv().collect();
    let vt = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]);
    let pp = || Same::<T>::new().map(|(p, _): T| p);
    let j = (&tp).select(
        Same::<T>::new()
            .and(pp().select(Ident::<Post>::new().and(&vt).and(score)))
            .filt(|(_, ((_, a), s)): (T, ((Id<Post>, [i64; 3]), i64))| s > 10 || a[2] > 20)
            .and(pp().select(owner_user.select(&db.user.display_name).select(&named).select(&ub)).opt()),
    );
    rows(drain(j).into_iter().map(|(_, (((_, r), ((p, a), _)), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "answers", "owner"]);
        f.push(V::I(r));
        f.extend(a.map(V::I));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        let closed = closed_date.get(p).is_some();
        f.push(V::S(if closed && r == 1 { "Most Recent Closed Post" } else if !closed && r == 1 { "Most Recent Open Post" } else { "Other" }));
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AvgViewsPerPost, AVG(p.Score) AS AvgScorePerPost
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%<' || t.TagName || '>%' GROUP BY t.TagName),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAsked, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswersProvided,
//        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, STRING_AGG(t.TagName, ', ' ORDER BY t.Id) AS TagsList FROM Posts p JOIN Tags t ON p.Tags LIKE '%<' || t.TagName || '>%'
//     WHERE p.ViewCount > 1000 GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate)
// SELECT ts.TagName, ts.PostCount, ts.TotalViews, ts.TotalScore, ts.AvgViewsPerPost, ts.AvgScorePerPost, us.DisplayName AS TopUser, us.QuestionsAsked, us.AnswersProvided, us.TotalViews AS UserTotalViews,
//        us.TotalScore AS UserTotalScore, pp.Title AS PopularPostTitle, pp.ViewCount AS PopularPostViews, pp.Score AS PopularPostScore, pp.TagsList
// FROM TagStats ts LEFT JOIN UserStats us ON us.TotalViews = (SELECT MAX(TotalViews) FROM UserStats) LEFT JOIN PopularPosts pp ON pp.ViewCount = (SELECT MAX(ViewCount) FROM PopularPosts)
// ORDER BY ts.PostCount DESC, ts.TotalScore DESC;
//
// The two ON clauses name only the right side, so each tag is crossed with the top user(s) and the most viewed post(s). STRING_AGG is
// ordered by Tags.Id (uses rewrites/29615.sql).
fn q29615(db: &'static So) -> String {
    let Post { view_count, score, post_type_id, .. } = &db.post;
    type R = (Id<Post>, Id<Tag>);
    let bt = bracketed(db, true);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&bt).map(|(_, t): R| t).inv().map(|(p, _): R| p).collect();
    let tags: HashIdx<Id<Post>, Id<Tag>> = (&bt).map(|(p, _): R| p).inv().map(|(_, t): R| t).collect();
    let tags = &tags;
    let ts = db.tag.group_by(&db.tag.tag_name).select((&by_tag).select(view_count.opt().and(score))).fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let tn = db.tag.group_by(&db.tag.tag_name).select(&by_tag).count_distinct();
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt().and(score).and(post_type_id)).opt()).fold([0i64; 4], |a, p| match p {
        Some(((w, s), t)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0), a[3] + s],
        None => a,
    });
    let mv = (&us).map(|a| a[2]).fold_flat(i64::MIN, |m, x| m.max(x));
    let top: HashIdx<(), (Id<User>, [i64; 4])> = whole(&us).select(Ident::<User>::new().and((&us).filt(move |a| a[2] == mv))).collect();
    let popular = || db.post.with(view_count.gt(1000)).with(tags);
    let mp = popular().select(view_count).fold_flat(i64::MIN, |m, w| m.max(w));
    let names = popular().with(view_count.eq(mp)).group_by(Ident::<Post>::new()).select(tags).buf_fold(|it| {
        let mut t: Vec<Id<Tag>> = it.into_iter().collect();
        t.sort_unstable();
        let n: Vec<Str> = t.into_iter().map(|t| db.tag.tag_name.get(t).unwrap()).collect();
        &*Box::leak(n.join(", ").into_boxed_str())
    });
    let pp: HashIdx<(), (Id<Post>, Str)> = whole(&names).select(Ident::<Post>::new().and(&names)).collect();
    let unit = || Same::<Str>::new().map(|_: Str| ());
    let v = drain((&ts).and(&tn).and(unit().select(&top).opt()).and(unit().select(&pp).opt()));
    rows(v.into_iter().map(|(t, (((a, n), u), p))| {
        let mut f = vec![V::S(t), V::I(n), nullable(a[2], a[1]), V::I(a[3]), avg(a[2], a[1]), avg(a[3], a[0])];
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "name"), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match p {
            Some((p, n)) => {
                let mut x = post_fields(db, p, &["title", "views", "score"]);
                x.push(V::S(n));
                x
            }
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, u.Views, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation IS NOT NULL),
// PostStats AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// AnsweredQuestions AS (SELECT p.OwnerUserId, MIN(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN p.CreationDate END) AS FirstAcceptedAnswerDate FROM Posts p WHERE p.PostTypeId = 1 GROUP BY p.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId)
// SELECT ur.DisplayName, ur.Reputation, ur.Views, ps.TotalPosts, ps.TotalQuestions, ps.TotalAnswers, ps.TotalScore, ps.TotalViews, aq.FirstAcceptedAnswerDate, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        CASE WHEN aq.FirstAcceptedAnswerDate IS NULL THEN 'No Accepted Answers' ELSE 'Has Accepted Answers' END AS AnswerStatus, CASE WHEN ur.UserRank <= 10 THEN 'Top 10 User' ELSE 'Regular User' END AS UserCategory
// FROM RankedUsers ur JOIN PostStats ps ON ur.Id = ps.OwnerUserId LEFT JOIN AnsweredQuestions aq ON ur.Id = aq.OwnerUserId LEFT JOIN UserBadges ub ON ur.Id = ub.UserId
// WHERE ur.Reputation > 1000 ORDER BY ur.UserRank, ps.TotalViews DESC LIMIT 50 OFFSET 0;
fn q24737(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, accepted_answer_id, creation_date, .. } = &db.post;
    let rep = &db.user.reputation;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(rep)).window(rank, |(_, r)| Reverse(r), asc);
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]);
    let aq = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(accepted_answer_id.opt().and(creation_date)).fold(i64::MAX, |m, (a, d)| if a.is_some() { m.min(d) } else { m });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    type R = ((Id<User>, i64), i64);
    type T = (Id<User>, i64);
    let uu = || Same::<T>::new().map(|(u, _): T| u);
    let j = (&w).filt(|((_, r), _): R| r > 1000).map(|((u, _), r): R| (u, r)).select(Same::<T>::new().and(uu().select((&ps).and((&aq).opt()).and((&ub).opt()))));
    let v = top_n(drain(j), |&(_, ((u, r), ((a, _), _)))| (r, a[4] == 0, Reverse(a[5]), u), 50);
    rows(v.into_iter().map(|(_, ((u, r), ((a, q), b)))| {
        let q = q.filter(|&d| d != i64::MAX);
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), ots(q)]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::S(if q.is_none() { "No Accepted Answers" } else { "Has Accepted Answers" }), V::S(if r <= 10 { "Top 10 User" } else { "Regular User" })]);
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserId, ph.CreationDate, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// UserReputationRank AS (SELECT u.Id AS UserId, u.Reputation, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostVoteSummary AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(ph.cumulativeComments, 0) AS TotalComments, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes, u.DisplayName, u.Reputation,
//        ur.ReputationRank  -- Changed from u.DensityRank to ur.ReputationRank
// FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS cumulativeComments FROM Comments GROUP BY PostId) ph ON p.Id = ph.PostId JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN PostVoteSummary vs ON p.Id = vs.PostId JOIN UserReputationRank ur ON u.Id = ur.UserId  -- Join to get ReputationRank
// WHERE EXISTS (SELECT 1 FROM RecursivePostHistory rph WHERE rph.PostId = p.Id AND rph.PostHistoryTypeId = 11) AND u.Reputation > 1000  -- Changed to a direct condition instead of a subquery
// ORDER BY p.ViewCount DESC, UpVotes DESC LIMIT 50;
fn q31757(db: &'static So) -> String {
    let Post { owner_user, view_count, .. } = &db.post;
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(dense_rank, |(_, r)| Reverse(r), asc);
    let drs: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let dr: HashIdx<Id<User>, (Id<User>, i64)> = (&drs).map(|(u, _): (Id<User>, i64)| u).inv().collect();
    let reopened: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(11)).select(&db.post_history.post).collect();
    let ps = || (&reopened).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))));
    let v = drain((&post_comment_count(db, ps())).and(&post_updown(db, ps())).and(owner_user.select(&dr)));
    let v = top_n(v, |&(p, ((_, a), _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(a[0]), p)
    }, 50);
    rows(v.into_iter().map(|(p, ((c, a), (u, r)))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges GROUP BY UserId),
// PostStats AS (SELECT OwnerUserId, COUNT(Id) AS PostCount, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(ViewCount) AS TotalViews FROM Posts GROUP BY OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.QuestionCount, 0) AS QuestionCount, COALESCE(ps.AnswerCount, 0) AS AnswerCount,
//        COALESCE(ps.TotalViews, 0) AS TotalViews, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
//     FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId WHERE u.Reputation > 1000 ORDER BY TotalViews DESC),
// RankedUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM TopUsers)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, GoldBadges, SilverBadges, BronzeBadges, ViewRank FROM RankedUsers
// WHERE ViewRank <= 10 OR (GoldBadges > 0 AND ViewRank <= 20) ORDER BY ViewRank;
fn q210(db: &'static So) -> String {
    let ups = user_posts(db);
    type X = ([i64; 10], [i64; 4]);
    let x: HashIdx<Id<User>, X> = db.user.with((&db.user.reputation).gt(1000)).select((&ups).and(user_badge_classes(db))).collect();
    let w = whole(&x).select(Ident::<User>::new().and(&x)).window(rank, |(_, (a, _)): (Id<User>, X)| Reverse(a[6]), asc);
    let v = drain((&w).filt(|((_, (_, b)), r): ((Id<User>, X), i64)| r <= 10 || (b[1] > 0 && r <= 20)));
    rows(v.into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, COUNT(a.Id) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.Score DESC) AS Rank
//     FROM Posts AS p LEFT JOIN Posts AS a ON p.Id = a.ParentId AND a.PostTypeId = 2 WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, COALESCE(SUM(b.Class), 0) AS TotalBadges, SUM(u.Reputation) AS TotalReputation FROM Users AS u LEFT JOIN Badges AS b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS TotalClose, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS TotalReopen FROM PostHistory AS ph GROUP BY ph.PostId),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, ur.TotalBadges, ur.TotalReputation, COALESCE(phs.TotalClose, 0) AS TotalClose,
//        COALESCE(phs.TotalReopen, 0) AS TotalReopen FROM RankedPosts AS rp JOIN UserReputation AS ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostHistoryStats AS phs ON rp.PostId = phs.PostId)
// SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, TotalBadges, TotalReputation, TotalClose, TotalReopen FROM FinalResults
// WHERE (TotalBadges > 0 OR TotalReputation > 100) AND (Score > 5 OR AnswerCount > 3) ORDER BY Score DESC, ViewCount DESC;
fn q33090(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let ac = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let ur = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 2], |a, (r, c)| [a[0] + c.unwrap_or(0), a[1] + r]);
    let ph = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let v = drain((&ac).and(score).and(owner_user.select((&ur).filt(|a| a[0] > 0 || a[1] > 100))).filt(|((n, s), _): ((i64, i64), [i64; 2])| s > 5 || n > 3).and((&ph).opt()));
    rows(v.into_iter().map(|(p, (((n, _), u), h))| {
        let h = h.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(n), V::I(u[0]), V::I(u[1]), V::I(h[0]), V::I(h[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId
//     JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, u.DisplayName, p.OwnerUserId, p.CreationDate),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        COUNT(DISTINCT p.Id) AS PostsCount FROM Users u JOIN Votes v ON u.Id = v.UserId JOIN Posts p ON p.Id = v.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) >= 5),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.Body, rp.OwnerDisplayName, rp.CommentCount, rp.AnswerCount, mau.DisplayName AS MostActiveUser, mau.TotalUpVotes, mau.TotalDownVotes, mau.PostsCount
//     FROM RankedPosts rp JOIN MostActiveUsers mau ON rp.OwnerPostRank <= 3 ORDER BY rp.CommentCount DESC, rp.AnswerCount DESC)
// SELECT PostId, Title, OwnerDisplayName, CommentCount, AnswerCount, MostActiveUser, TotalUpVotes, TotalDownVotes, PostsCount FROM PostStats WHERE AnswerCount > 0 ORDER BY TotalUpVotes DESC, CommentCount DESC;
//
// `ON rp.OwnerPostRank <= 3` names only rp, so each owner's three newest questions are crossed with the active voters; the rank reads base columns, so those questions are picked first.
fn q25272(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 3).map(|((p, _), _)| p).collect();
    let rp = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(children_of(db).opt())).fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let qa = || Ident::<Post>::new().with(post_type_id.is_in([1, 2]));
    let Vote { post, vote_type_id, user, .. } = &db.vote;
    let ud = db.vote.with(post.select(qa())).group_by(user).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pc = db.vote.with(post.select(qa())).group_by(user).select(post).count_distinct();
    let mau: HashIdx<Id<User>, ([i64; 2], i64)> = (&ud).and((&pc).filt(|n| n >= 5)).collect();
    let mut v = Vec::new();
    (&rp).filt(|a| a[1] > 0).cross(&mau).drive(|(p, u), (a, (m, n))| v.push((p, a, u, m, n)));
    rows(v.into_iter().map(|(p, a, u, m, n)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), user_col(db, u, "name"), V::I(m[0]), V::I(m[1]), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.TagRank = 1),
// PostVoteCounts AS (SELECT v.PostId, COUNT(*) FILTER (WHERE v.VoteTypeId = 2) AS Upvotes, COUNT(*) FILTER (WHERE v.VoteTypeId = 3) AS Downvotes FROM Votes v GROUP BY v.PostId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostWithVoteAndCommentStats AS (SELECT tp.PostId, tp.Title, tp.ViewCount, tp.Score, tp.CreationDate, tp.AnswerCount, COALESCE(pvc.Upvotes, 0) AS TotalUpvotes, COALESCE(pvc.Downvotes, 0) AS TotalDownvotes,
//        COALESCE(pc.CommentCount, 0) AS TotalComments FROM TopPosts tp LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId LEFT JOIN PostComments pc ON tp.PostId = pc.PostId)
// SELECT pwvcs.PostId, pwvcs.Title, pwvcs.ViewCount, pwvcs.Score, pwvcs.TotalUpvotes, pwvcs.TotalDownvotes, pwvcs.TotalComments, (pwvcs.TotalUpvotes - pwvcs.TotalDownvotes) AS NetScore,
//        (EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - pwvcs.CreationDate)) / 86400) AS AgeInDays
// FROM PostWithVoteAndCommentStats pwvcs WHERE pwvcs.TotalComments > 0 ORDER BY NetScore DESC, AgeInDays ASC LIMIT 10;
fn q28886(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, tags_str, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).group_by(tags_str.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let v = drain((&post_comment_count(db, &tp)).filt(|c| c > 0).and(&post_updown(db, &tp)));
    let v = top_n(v, |&(p, (_, a))| (Reverse(a[0] - a[1]), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(a[0] - a[1]), V::F(secs(t0 - creation_date.get(p).unwrap()) / 86400.0)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// AnswersCount AS (SELECT ParentId AS QuestionId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId),
// ClosedQuestionInfo AS (SELECT ph.PostId, ph.CreationDate AS ClosedDate, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10),
// UsersWithBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostInsights AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, COALESCE(ac.AnswerCount, 0) AS AnswerCount, COALESCE(cqi.ClosedDate, NULL) AS ClosedDate,
//        COALESCE(cqi.CloseReason, 'Open') AS CloseReason, ub.BadgeCount FROM RankedPosts rp LEFT JOIN AnswersCount ac ON rp.PostId = ac.QuestionId
//     LEFT JOIN ClosedQuestionInfo cqi ON rp.PostId = cqi.PostId LEFT JOIN UsersWithBadges ub ON rp.OwnerUserId = ub.UserId WHERE rp.UserPostRank = 1)
// SELECT pi.PostId, pi.Title, pi.CreationDate, pi.Score, pi.AnswerCount, pi.ClosedDate, pi.CloseReason, pi.BadgeCount,
//        CASE WHEN pi.BadgeCount > 5 THEN 'Highly Recognized' ELSE 'Regular User' END AS UserRecognition
// FROM PostInsights pi WHERE pi.Score > 10 ORDER BY pi.CreationDate DESC;
fn q34788(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, score, .. } = &db.post;
    let fw = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&fw).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let pi = || (&first).with(score.gt(10));
    let ac = pi().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&ac).and(closes.opt()).and(owner_user.select(&bc).opt()));
    rows(v.into_iter().map(|(p, ((n, h), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(n));
        f.extend(match h {
            Some(h) => [V::T(db.post_history.creation_date.get(h).unwrap()), V::S(db.post_history.comment.get(h).unwrap_or("Open"))],
            None => [V::Null, V::S("Open")],
        });
        f.extend([oint(b), V::S(if b.map_or(false, |b| b > 5) { "Highly Recognized" } else { "Regular User" })]);
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsPosted,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersPosted, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvotesReceived, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvotesReceived,
//        RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS UserRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, RANK() OVER (ORDER BY p.CreationDate DESC) AS RecentRank FROM Posts p
//     LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 5)
// SELECT ua.DisplayName, ua.Reputation, ua.PostCount, ua.QuestionsPosted, ua.AnswersPosted, ua.UpvotesReceived, ua.DownvotesReceived, rp.Title AS RecentPostTitle, rp.CommentCount, pt.TagName AS PopularTag
// FROM UserActivity ua JOIN RecentPosts rp ON ua.UserId = rp.OwnerUserId CROSS JOIN PopularTags pt WHERE ua.PostCount > 10 ORDER BY ua.UserRank;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q33435(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let rp = post_comment_count(db, db.post.with(creation_date.gt(add_days(date(2024, 10, 1), -30))).with(owner_user));
    let pt = rel(top_n(drain((&tag_name_stats(db)).filt(|a| a[0] > 0)), |&(t, a)| (Reverse(a[0]), t), 5));
    let j = (&rp).and(owner_user.select(Ident::<User>::new().and((&ua).filt(|a| a[0] > 10))));
    let mut v = Vec::new();
    j.cross(&pt).drive(|(p, _), ((c, (u, a)), (t, _))| v.push((p, c, u, a, t)));
    rows(v.into_iter().map(|(p, c, u, a, t)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([title(db, p), V::I(c), V::S(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > DATE '2021-01-01' GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, CASE WHEN u.Reputation >= 1000 THEN 'Veteran' WHEN u.Reputation >= 500 THEN 'Experienced' ELSE 'Novice' END AS ExperienceLevel FROM Users u),
// PostHistoryDetails AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS Closed, MAX(CASE WHEN ph.PostHistoryTypeId IN (11, 12) THEN 1 ELSE 0 END) AS IsDeleted,
//        COUNT(ph.Id) AS EditCount FROM PostHistory ph GROUP BY ph.PostId),
// PopularPostLinks AS (SELECT pl.PostId, pl.RelatedPostId, lt.Name AS LinkType, COUNT(pl.Id) AS LinkCount FROM PostLinks pl JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id GROUP BY pl.PostId, pl.RelatedPostId, lt.Name)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, ur.DisplayName, ur.ExperienceLevel, COALESCE(phd.Closed, 0) AS PostClosed, COALESCE(phd.IsDeleted, 0) AS PostDeleted, phd.EditCount,
//        COALESCE(pl.LinkCount, 0) AS RelatedPostLinks
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId LEFT JOIN PopularPostLinks pl ON rp.PostId = pl.PostId
// WHERE rp.ScoreRank <= 5 ORDER BY rp.Score DESC, ur.Reputation DESC OFFSET 0 ROWS FETCH NEXT 20 ROWS ONLY;
//
// CommentCount is never read, and the GROUP BY gives one row per post, so the comment join is not driven.
fn q34918(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.gt(date(2021, 1, 1))).group_by(owner_user).select(Ident::<Post>::new().and(score).and(owner_user)).window(row_number, |((p, s), _)| (Reverse(s), p), asc);
    type T = (Id<Post>, Id<User>);
    let tp: MatSet<T> = (&w).filt(|(_, n)| n <= 5).map(|(((p, _), u), _)| (p, u)).collect();
    let phd = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| [a[0].max((t == 10) as i64), a[1].max(matches!(t, 11 | 12) as i64), a[2] + 1]);
    let PostLink { post, related_post_id, link_type, .. } = &db.post_link;
    let ppl = db.post_link.group_by(post.and(related_post_id).and(link_type.select(&db.link_type.name))).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let ppl = rel(drain(&ppl));
    let pl: HashIdx<Id<Post>, (((Id<Post>, i64), Str), i64)> = (&ppl).map(|(((p, _), _), _)| p).inv().select(&ppl).collect();
    let pp = || Same::<T>::new().map(|(p, _): T| p);
    let v = drain((&tp).select(Same::<T>::new().and(pp().select((&phd).opt())).and(pp().select((&pl).map(|(_, n)| n).opt()))));
    let v = top_n(v, |&(_, (((p, u), _), l))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p, l), 20);
    rows(v.into_iter().map(|(_, (((p, u), h), l))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([user_col(db, u, "name"), V::S(if r >= 1000 { "Veteran" } else if r >= 500 { "Experienced" } else { "Novice" })]);
        f.extend(match h {
            Some(h) => [V::I(h[0]), V::I(h[1]), V::I(h[2])],
            None => [V::I(0), V::I(0), V::Null],
        });
        f.push(V::I(l.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerUserId, rp.OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, COUNT(b.Id) AS BadgeCount
//     FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId LEFT JOIN Badges b ON rp.OwnerUserId = b.UserId WHERE rp.Rank = 1
//     GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerUserId, rp.OwnerDisplayName)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.OwnerDisplayName, ps.CommentCount, ps.VoteCount, ps.BadgeCount,
//        COALESCE(ROUND(AVG(EXTRACT(EPOCH FROM ph.CreationDate) - EXTRACT(EPOCH FROM ps.CreationDate)), 2), 0) AS AverageEditTime
// FROM PostStats ps LEFT JOIN PostHistory ph ON ps.PostId = ph.PostId WHERE ph.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.OwnerDisplayName, ps.CommentCount, ps.VoteCount, ps.BadgeCount ORDER BY ps.Score DESC, ps.ViewCount DESC LIMIT 10;
fn q8792(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let fw = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&fw).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))).select(&db.post_history.creation_date);
    let et = (&first).group_by(Ident::<Post>::new()).select(creation_date.and(edits)).fold((0i64, 0.0f64), |(n, s), (c, d)| (n + 1, s + (d as f64 / 1e6 - c as f64 / 1e6)));
    let ps = (&first)
        .with(&et)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 3], |a, ((c, v), b)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + b.is_some() as i64]);
    let v = top_n(drain((&ps).and(&et)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (a, (n, s)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::F((s / n as f64 * 100.0).round() / 100.0));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionCount, COUNT(DISTINCT A.Id) AS AcceptedAnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount, SUM(CASE WHEN V.VoteTypeId = 10 THEN 1 ELSE 0 END) AS DeletionCount, SUM(CASE WHEN V.VoteTypeId = 16 THEN 1 ELSE 0 END) AS ApproveEditCount
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id AND P.PostTypeId = 1 LEFT JOIN Posts A ON A.AcceptedAnswerId = P.Id LEFT JOIN Votes V ON V.UserId = U.Id WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionCount, AcceptedAnswerCount, UpVotesCount, DownVotesCount, DENSE_RANK() OVER (ORDER BY QuestionCount DESC) AS RankByQuestions,
//        DENSE_RANK() OVER (ORDER BY AcceptedAnswerCount DESC) AS RankByAcceptedAnswers, DENSE_RANK() OVER (ORDER BY UpVotesCount - DownVotesCount DESC) AS RankByVoteDifference FROM UserActivity)
// SELECT UserId, DisplayName, QuestionCount, AcceptedAnswerCount, UpVotesCount, DownVotesCount, RankByQuestions, RankByAcceptedAnswers, RankByVoteDifference,
//        CASE WHEN RankByQuestions = 1 THEN 'Top Question Asker' WHEN RankByAcceptedAnswers = 1 THEN 'Top Answer Provider' WHEN RankByVoteDifference = 1 THEN 'Most Positive Impact' ELSE 'Contributor' END AS UserCategory
// FROM TopUsers WHERE RankByQuestions <= 10 OR RankByAcceptedAnswers <= 10 OR RankByVoteDifference <= 10 ORDER BY RankByQuestions, RankByAcceptedAnswers, RankByVoteDifference;
fn q26547(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, .. } = &db.post;
    let acceptors: HashIdx<Id<Post>, Id<Post>> = accepted_answer.inv().collect();
    let q = || Ident::<Post>::new().with(post_type_id.eq(1));
    let us = || db.user.with((&db.user.reputation).gt(0));
    let s = us()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(q().and((&acceptors).opt())).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let dq = us().group_by(Ident::<User>::new()).select(posts_of(db).select(q())).count_distinct();
    let da = us().group_by(Ident::<User>::new()).select(posts_of(db).select(q()).select(&acceptors)).count_distinct();
    let x: HashIdx<Id<User>, [i64; 4]> = (&s).and((&dq).opt()).and((&da).opt()).map(|((a, q), r): (([i64; 2], Option<i64>), Option<i64>)| [q.unwrap_or(0), r.unwrap_or(0), a[0], a[1]]).collect();
    let w1 = whole(&x).select(Ident::<User>::new().and(&x)).window(dense_rank, |(_, a)| Reverse(a[0]), asc);
    let w2 = (&w1).window(dense_rank, |((_, a), _): ((Id<User>, [i64; 4]), i64)| Reverse(a[1]), asc);
    let w3 = (&w2).window(dense_rank, |(((_, a), _), _): (((Id<User>, [i64; 4]), i64), i64)| Reverse(a[2] - a[3]), asc);
    let v = drain((&w3).filt(|(((_, q), r), d): ((((Id<User>, [i64; 4]), i64), i64), i64)| q <= 10 || r <= 10 || d <= 10));
    rows(v.into_iter().map(|(_, ((((u, a), q), r), d))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(q), V::I(r), V::I(d)]);
        f.push(V::S(if q == 1 { "Top Question Asker" } else if r == 1 { "Top Answer Provider" } else if d == 1 { "Most Positive Impact" } else { "Contributor" }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotesCount,
//        SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        AVG(p.Score) AS AverageScore FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// TopUsers AS (SELECT UserId, DisplayName, RANK() OVER (ORDER BY UpVotesCount DESC) AS UpvotesRank FROM UserVoteStats WHERE TotalVotes > 0),
// ActivePosts AS (SELECT ps.PostId, ps.CommentCount, ps.QuestionCount, ps.AnswerCount, ps.AverageScore, tu.DisplayName AS TopUser FROM PostStatistics ps JOIN Votes v ON ps.PostId = v.PostId
//     JOIN TopUsers tu ON v.UserId = tu.UserId WHERE ps.CommentCount > 10 AND ps.AverageScore > 5)
// SELECT ap.PostId, ap.CommentCount, ap.QuestionCount, ap.AnswerCount, ap.AverageScore, COUNT(DISTINCT ap.TopUser) AS UniqueTopUsers FROM ActivePosts ap
// GROUP BY ap.PostId, ap.CommentCount, ap.QuestionCount, ap.AnswerCount, ap.AverageScore ORDER BY UniqueTopUsers DESC, ap.AverageScore DESC LIMIT 100;
fn q6632(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(Ident::<Post>::new()).select(post_type_id.and(score).and(comments_of(db).opt())).fold([0i64; 5], |a, ((t, s), c)| {
        [a[0] + c.is_some() as i64, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + 1]
    });
    let tu = db.vote.with((&db.vote.vote_type_id).is_in([2, 3])).group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let voters = votes_of(db).select((&db.vote.user).select(Ident::<User>::new().with(&tu)).select(&db.user.display_name));
    let ap = || (&ps).filt(|a| a[0] > 10 && a[3] > 5 * a[4]);
    let v = drain(ap().and(db.post.with(ap()).group_by(Ident::<Post>::new()).select(voters).count_distinct()));
    let v = top_n(v, |&(p, (a, n))| (Reverse(n), Reverse(fkey(a[3] as f64 / a[4] as f64)), p), 100);
    rows(v.into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4]), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(a.Id) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN,
//        p.OwnerUserId FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(SUM(b.Class), 0) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// QuestionRatings AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ur.Reputation, ur.TotalBadges,
//        CASE WHEN ur.Reputation >= 1000 THEN 'High Reputation' WHEN ur.Reputation BETWEEN 500 AND 999 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.RN = 1)
// SELECT q.PostId, q.Title, q.CreationDate, q.Score, q.ViewCount, q.Reputation, q.TotalBadges, q.ReputationCategory, COALESCE(c.CommentCount, 0) AS CommentCount,
//        CASE WHEN q.Score > 10 THEN 'Popular' ELSE 'Less Popular' END AS Popularity
// FROM QuestionRatings q LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON q.PostId = c.PostId ORDER BY q.Reputation DESC, q.Score DESC FETCH FIRST 100 ROWS ONLY;
//
// AnswerCount is never read, and the GROUP BY gives one row per post, so the answer join is not driven.
fn q4972(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let fw = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first = drain((&fw).filt(|(_, n)| n == 1).map(|((p, _), _)| p));
    let top = top_n(first, |&(u, p)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.1).collect()).map(|p| p).collect();
    let tb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |s, c| s + c.unwrap_or(0));
    let v = drain((&post_comment_count(db, &tp)).and(owner_user.select(Ident::<User>::new().and(&tb))));
    rows(v.into_iter().map(|(p, (c, (u, b)))| {
        let r = db.user.reputation.get(u).unwrap();
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(r), V::I(b), V::S(if r >= 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }), V::I(c), V::S(if s > 10 { "Popular" } else { "Less Popular" })]);
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(p.AnswerCount) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.TotalAnswers, 0) AS TotalAnswers, COALESCE(ps.TotalQuestions, 0) AS TotalQuestions,
//        COALESCE(ps.TotalViews, 0) AS TotalViews, COALESCE(ps.AverageScore, 0) AS AverageScore, COALESCE(bc.BadgeCount, 0) AS TotalBadges, COALESCE(bc.GoldBadges, 0) AS GoldBadges,
//        COALESCE(bc.SilverBadges, 0) AS SilverBadges, COALESCE(bc.BronzeBadges, 0) AS BronzeBadges FROM Users u LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId LEFT JOIN UserBadgeCounts bc ON u.Id = bc.UserId),
// TopUsers AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC, TotalPosts DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalQuestions, TotalViews, AverageScore, TotalBadges, GoldBadges, SilverBadges, BronzeBadges FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q33633(db: &'static So) -> String {
    let Post { owner_user, score, post_type_id, view_count, answer_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(post_type_id).and(view_count.opt()).and(answer_count.opt())).fold([0i64; 5], |a, (((s, t), w), n)| {
        [a[0] + 1, a[1] + n.unwrap_or(0), a[2] + (t == 1) as i64, a[3] + w.unwrap_or(0), a[4] + s]
    });
    let x: HashIdx<Id<User>, ([i64; 5], [i64; 4])> = db.user.select((&ps).opt().map(|o: Option<[i64; 5]>| o.unwrap_or([0; 5])).and(user_badge_classes(db))).collect();
    let w = whole(&x).select(Ident::<User>::new().and(&x).and(&db.user.reputation)).window(rank, |((_, (a, _)), r)| (Reverse(r), Reverse(a[0])), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, (((u, (a, b)), _), _))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[0] == 0 { V::F(0.0) } else { avg(a[4], a[0]) }]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges GROUP BY UserId),
// UserVotes AS (SELECT UserId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY UserId),
// PostStats AS (SELECT OwnerUserId, COUNT(CASE WHEN PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(ViewCount) AS TotalViews FROM Posts GROUP BY OwnerUserId),
// UserActivity AS (SELECT U.Id AS UserId, U.Reputation, COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
//        COALESCE(UV.UpVotes, 0) AS UpVotes, COALESCE(UV.DownVotes, 0) AS DownVotes, COALESCE(PS.QuestionCount, 0) AS QuestionCount, COALESCE(PS.AnswerCount, 0) AS AnswerCount,
//        COALESCE(PS.TotalViews, 0) AS TotalViews FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN UserVotes UV ON U.Id = UV.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT UserId, Reputation, GoldBadges, SilverBadges, BronzeBadges, UpVotes, DownVotes, QuestionCount, AnswerCount, TotalViews, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
// FROM UserActivity WHERE (UpVotes - DownVotes) > 10 OR (GoldBadges + SilverBadges) >= 2 ORDER BY Reputation DESC, TotalViews DESC LIMIT 100;
fn q2510(db: &'static So) -> String {
    let ups = user_posts(db);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type X = (([i64; 4], [i64; 2]), [i64; 10]);
    let ua: HashIdx<Id<User>, X> = db
        .user
        .select(user_badge_classes(db).and((&uv).opt().map(|o: Option<[i64; 2]>| o.unwrap_or([0, 0]))).and(&ups))
        .filt(|((b, v), _): X| v[0] - v[1] > 10 || b[1] + b[2] >= 2)
        .collect();
    let rep = &db.user.reputation;
    let w = whole(&ua).select(Ident::<User>::new().and(&ua).and(rep)).window(rank, |(_, r)| Reverse(r), asc);
    let v = top_n(drain(&w), |&(_, (((u, (_, a)), r), _))| (Reverse(r), Reverse(a[6]), u), 100);
    rows(v.into_iter().map(|(_, (((u, ((b, x), a)), _), r))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(x[0]), V::I(x[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), V::I(r)]);
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT Id, Reputation, CreationDate, 0 AS Level FROM Users WHERE Reputation > 100 UNION ALL SELECT U.Id, U.Reputation, U.CreationDate, UR.Level + 1
//     FROM Users U INNER JOIN UserReputation UR ON U.Id = UR.Id WHERE U.Reputation > 200),
// PostCounts AS (SELECT OwnerUserId, COUNT(*) AS TotalPosts FROM Posts GROUP BY OwnerUserId),
// TopPosters AS (SELECT U.Id, U.DisplayName, COALESCE(PC.TotalPosts, 0) AS PostCount FROM Users U LEFT JOIN PostCounts PC ON U.Id = PC.OwnerUserId WHERE U.Reputation > 100 ORDER BY PostCount DESC LIMIT 10),
// RecentActivity AS (SELECT U.DisplayName, COUNT(CASE WHEN P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 1 END) AS RecentPosts,
//        COUNT(CASE WHEN C.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 1 END) AS RecentComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.DisplayName),
// VotingSummary AS (SELECT UserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY UserId)
// SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RA.RecentPosts, RA.RecentComments, COALESCE(VS.UpVotes, 0) AS TotalUpVotes, COALESCE(VS.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(TP.PostCount, 0) AS TotalPosts, U.CreationDate
// FROM Users U LEFT JOIN RecentActivity RA ON U.DisplayName = RA.DisplayName LEFT JOIN VotingSummary VS ON U.Id = VS.UserId LEFT JOIN TopPosters TP ON U.Id = TP.Id
// WHERE U.Reputation IS NOT NULL AND (RA.RecentPosts > 0 OR RA.RecentComments > 0) ORDER BY U.Reputation DESC LIMIT 50;
//
// The recursive UserReputation is never referenced, so it is not computed.
fn q34298(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let since = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let ups = user_posts(db);
    let tp = rel(top_n(drain(db.user.with((&db.user.reputation).gt(100)).select(&ups)), |&(u, a)| (Reverse(a[1]), u), 10));
    let tpi: HashIdx<Id<User>, (Id<User>, [i64; 10])> = (&tp).map(|(u, _)| u).inv().select(&tp).collect();
    let ra = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(creation_date.and(comments_of(db).select(&db.comment.creation_date).opt())).opt())
        .fold([0i64; 2], |a, x| match x {
            Some((p, c)) => [a[0] + (p > since) as i64, a[1] + c.map_or(false, |c| c > since) as i64],
            None => a,
        });
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let j = db.user.select((&db.user.display_name).select((&ra).filt(|a| a[0] > 0 || a[1] > 0)).and((&vs).opt()).and((&tpi).opt()));
    let v = top_n(drain(j), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 50);
    rows(v.into_iter().map(|(u, ((a, x), t))| {
        let x = x.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(x[0]), V::I(x[1]), V::I(t.map_or(0, |(_, t)| t[1])), user_col(db, u, "ucreated")]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(v.BountyAmount) AS TotalBounties, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalBounties, AvgReputation, RANK() OVER (ORDER BY TotalPosts DESC) AS RankPosts,
//        RANK() OVER (ORDER BY TotalQuestions DESC) AS RankQuestions, RANK() OVER (ORDER BY TotalAnswers DESC) AS RankAnswers FROM UserStats),
// ActiveTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS RankPostCount, RANK() OVER (ORDER BY QuestionCount DESC) AS RankQuestionCount FROM ActiveTags)
// SELECT u.DisplayName AS TopUser, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalBounties, u.AvgReputation, t.TagName AS MostActiveTag, t.PostCount AS TagPostCount,
//        t.QuestionCount AS TagQuestionCount, t.AnswerCount AS TagAnswerCount
// FROM TopUsers u JOIN TopTags t ON u.RankPosts = 1 WHERE u.RankPosts <= 10 ORDER BY u.TotalPosts DESC, t.PostCount DESC;
//
// RankPosts reads COUNT(DISTINCT p.Id), which the vote fan-out does not change, so the top posters are ranked first and the post x vote product is driven for them alone.
// `ON u.RankPosts = 1` names only u, so they are crossed with every tag.
fn q26717(db: &'static So) -> String {
    let dp = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&dp).select(Ident::<User>::new().and(&dp)).window(rank, |(_, n)| Reverse(n), asc);
    let top: MatSet<Id<User>> = (&w).filt(|(_, r)| r == 1).map(|((u, _), _)| u).collect();
    let us = (&top)
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(&db.post.post_type_id).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 6], |a, ((r, t), b)| {
            let b = b.flatten();
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0), a[4] + r, a[5] + 1]
        });
    let tst = tag_name_stats(db);
    let at = (&tst).filt(|a| a[0] > 0);
    let mut v = Vec::new();
    (&us).and(&dp).cross(at).drive(|(u, t), ((a, n), b)| v.push((u, a, n, t, b)));
    rows(v.into_iter().map(|(u, a, n, t, b)| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[4], a[5])];
        f.extend([V::S(t), V::I(b[0]), V::I(b[4]), V::I(b[5])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerUserId, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// ExtendedPosts AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerUserId, tp.OwnerDisplayName, COUNT(c.Id) AS TotalComments,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId
//     LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerUserId, tp.OwnerDisplayName)
// SELECT ep.PostId, ep.Title, ep.CreationDate, ep.Score, ep.ViewCount, ep.AnswerCount, ep.CommentCount, ep.OwnerUserId, ep.OwnerDisplayName, ep.TotalComments, ep.UpVotes, ep.DownVotes,
//        CASE WHEN ep.Score > 50 THEN 'High Score' WHEN ep.Score BETWEEN 20 AND 50 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory
// FROM ExtendedPosts ep ORDER BY ep.Score DESC;
fn q5434(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner_id", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(if sc > 50 { "High Score" } else if sc >= 20 { "Moderate Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.ViewCount, p.CreationDate, p.OwnerUserId, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, CASE WHEN u.Reputation > 1000 THEN 'High' WHEN u.Reputation BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'Low' END AS ReputationLevel FROM Users u),
// TopQuestions AS (SELECT rp.PostId, rp.Title, rp.ViewCount, ur.DisplayName, ur.ReputationLevel, COUNT(a.Id) AS AnswerCount FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId
//     LEFT JOIN Posts a ON rp.PostId = a.ParentId GROUP BY rp.PostId, rp.Title, rp.ViewCount, ur.DisplayName, ur.ReputationLevel ORDER BY rp.ViewCount DESC LIMIT 10),
// VoteDetails AS (SELECT p.Id AS PostId, v.VoteTypeId, COUNT(v.Id) AS VoteCount FROM Posts p JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, v.VoteTypeId)
// SELECT tq.PostId, tq.Title, tq.ViewCount, tq.DisplayName, tq.ReputationLevel, COALESCE(SUM(CASE WHEN vd.VoteTypeId = 2 THEN vd.VoteCount ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN vd.VoteTypeId = 3 THEN vd.VoteCount ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN vd.VoteTypeId = 6 THEN vd.VoteCount ELSE 0 END), 0) AS CloseVotes, tq.AnswerCount
// FROM TopQuestions tq LEFT JOIN VoteDetails vd ON tq.PostId = vd.PostId GROUP BY tq.PostId, tq.Title, tq.ViewCount, tq.DisplayName, tq.ReputationLevel, tq.AnswerCount ORDER BY tq.ViewCount DESC;
//
// Summing the per-type vote counts back per post is the post's vote count by type.
fn q28751(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(view_count.opt())), |&(p, w)| (w.is_none(), Reverse(w), p), 10);
    let tq: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ac = (&tq).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let Vote { post, vote_type_id, .. } = &db.vote;
    let vd = rel(drain(db.vote.with(post.select(&tq)).group_by(post.and(vote_type_id)).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1)));
    let vs = (&vd).group_by(Same::<((Id<Post>, i64), i64)>::new().map(|((p, _), _)| p)).select(Same::<((Id<Post>, i64), i64)>::new()).fold([0i64; 3], |a, ((_, t), n)| {
        [a[0] + if t == 2 { n } else { 0 }, a[1] + if t == 3 { n } else { 0 }, a[2] + if t == 6 { n } else { 0 }]
    });
    let v = drain((&ac).and((&vs).opt()));
    rows(v.into_iter().map(|(p, (n, a))| {
        let a = a.unwrap_or([0; 3]);
        let r = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "views", "owner"]);
        f.push(V::S(if r > 1000 { "High" } else if r >= 500 { "Medium" } else { "Low" }));
        f.extend(a.map(V::I));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpVotesCount, DownVotesCount, RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserActivity WHERE PostCount > 0)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, COALESCE(ROUND(100.0 * tu.UpVotesCount / NULLIF(tu.PostCount, 0), 2), 0) AS UpVotePercentage,
//        COALESCE(ROUND(100.0 * tu.DownVotesCount / NULLIF(tu.PostCount, 0), 2), 0) AS DownVotePercentage,
//        CASE WHEN tu.PostCount > 100 THEN 'Expert' WHEN tu.PostCount > 50 THEN 'Pro' ELSE 'Novice' END AS UserCategory,
//        CASE WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = tu.UserId AND b.Class = 1) THEN 'Gold' WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = tu.UserId AND b.Class = 2) THEN 'Silver'
//        WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = tu.UserId AND b.Class = 3) THEN 'Bronze' ELSE 'No Badge' END AS BadgeStatus
// FROM TopUsers tu WHERE tu.PostRank <= 10 ORDER BY tu.PostRank;
fn q1926(db: &'static So) -> String {
    let pv = (&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt());
    let s = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(pv)).fold([0i64; 5], |a, (t, v)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]
    });
    let x: HashIdx<Id<User>, ([i64; 5], [i64; 4])> = (&s).and(user_badge_classes(db)).collect();
    let w = whole(&x).select(Ident::<User>::new().and(&x)).window(rank, |(_, (a, _))| Reverse(a[0]), asc);
    let pct = |x: i64, n: i64| V::F((100.0 * x as f64 / n as f64 * 100.0).round() / 100.0);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|(_, ((u, (a, b)), _))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), pct(a[3], a[0]), pct(a[4], a[0])];
        f.push(V::S(if a[0] > 100 { "Expert" } else if a[0] > 50 { "Pro" } else { "Novice" }));
        f.push(V::S(if b[1] > 0 { "Gold" } else if b[2] > 0 { "Silver" } else if b[3] > 0 { "Bronze" } else { "No Badge" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS RankByScore, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
//     LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 50
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, pt.Name),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.ViewCount, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.RankByScore <= 5),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b WHERE b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years' GROUP BY b.UserId),
// UserDetails AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ub.BadgeCount FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId)
// SELECT td.UserId, td.DisplayName, td.Reputation, td.BadgeCount, tp.Title AS TopPostTitle, tp.ViewCount AS TopPostViewCount, tp.Score AS TopPostScore, tp.CommentCount AS TopPostCommentCount,
//        tp.UpVotes AS TopPostUpVotes, tp.DownVotes AS TopPostDownVotes
// FROM TopPosts tp JOIN UserDetails td ON tp.Id = td.UserId ORDER BY td.Reputation DESC, tp.Score DESC LIMIT 10;
//
// `tp.Id = td.UserId` compares a post id with a user id, so it goes through the raw ids. RankByScore reads base columns, so the top posts are picked first.
fn q34320(db: &'static So) -> String {
    let Post { creation_date, view_count, score, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1)).and(view_count.gt(50))).group_by(ptype_name(db)).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, n)| n <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.badge.with((&db.badge.date).ge(add_years(t0, -2))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and(origid.select(&uid).select(Ident::<User>::new().and((&ub).opt()))));
    let v = top_n(v, |&(p, (_, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(oint(b));
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore, AVG(P.Score) AS AverageScore
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' WHERE P.PostTypeId = 1 GROUP BY T.TagName),
// UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionsAsked, COUNT(DISTINCT C.Id) AS CommentsMade, COUNT(DISTINCT V.Id) AS VotesReceived,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1 LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Votes V ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// TopTags AS (SELECT TagName, PostCount, TotalViews, TotalScore, AverageScore, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM TagStatistics),
// TopUsers AS (SELECT UserId, DisplayName, QuestionsAsked, CommentsMade, VotesReceived, UpVotesReceived, DownVotesReceived, ROW_NUMBER() OVER (ORDER BY VotesReceived DESC) AS Rank FROM UserEngagement)
// SELECT TT.TagName, TT.PostCount, TT.TotalViews, TT.TotalScore, TT.AverageScore, TU.DisplayName AS TopUser, TU.QuestionsAsked, TU.CommentsMade, TU.VotesReceived, TU.UpVotesReceived, TU.DownVotesReceived
// FROM TopTags TT JOIN TopUsers TU ON TT.Rank = TU.Rank WHERE TT.Rank <= 10 AND TU.QuestionsAsked >= 5 ORDER BY TT.TotalScore DESC;
//
// VotesReceived is COUNT(DISTINCT V.Id), which the comment fan-out does not change, so the users are ranked on it first and the question x comment x vote product is driven for the top ten alone.
fn q25073(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let q = || Ident::<Post>::new().with(post_type_id.eq(1));
    let tq = || (&by_tag).map(|(p, _)| p).select(q());
    let ts_ = db.tag.group_by(&db.tag.tag_name).select(tq().select(view_count.opt().and(score))).fold([0i64; 4], |a, (w, s)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]
    });
    let tn = db.tag.group_by(&db.tag.tag_name).select(tq()).count_distinct();
    let tsx: HashIdx<Str, ([i64; 4], i64)> = (&ts_).and(&tn).collect();
    let tw = whole(&tsx).select(Same::<Str>::new().and(&tsx)).window(row_number, |(t, (a, _))| (Reverse(a[3]), t), asc);
    type TT = ((Str, ([i64; 4], i64)), i64);
    let tts: MatSet<TT> = (&tw).filt(|(_, r)| r <= 10).collect();
    let tt: HashIdx<i64, TT> = (&tts).map(|(_, r): TT| r).inv().collect();
    let vr = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(q()).select(votes_of(db))).fold(0i64, |n, _| n + 1);
    let uw = whole(db.user.iq()).select(Ident::<User>::new().and((&vr).opt())).window(row_number, |(u, n)| (Reverse(n.unwrap_or(0)), u), asc);
    type UT = ((Id<User>, Option<i64>), i64);
    let uts: MatSet<UT> = (&uw).filt(|(_, r)| r <= 10).collect();
    let tus: MatSet<Id<User>> = (&uts).map(|((u, _), _): UT| u).collect();
    let dq = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).select(q()).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let dc = (&tus).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ue = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(q().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(comments_by(db).opt()))
        .fold([0i64; 2], |a, (p, _)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let tu: HashIdx<i64, (UT, ((i64, i64), [i64; 2]))> =
        (&uts).map(|(_, r): UT| r).inv().select(Same::<UT>::new().and(Same::<UT>::new().map(|((u, _), _): UT| u).select((&dq).filt(|n| n >= 5).and(&dc).and(&ue)))).collect();
    let v = drain((&tt).and(&tu));
    rows(v.into_iter().map(|(_, (((t, (a, n)), _), (((u, x), _), ((q, c), y))))| {
        let mut f = vec![V::S(t), V::I(n), nullable(a[2], a[1]), V::I(a[3]), avg(a[3], a[0])];
        f.extend([user_col(db, u, "name"), V::I(q), V::I(c), V::I(x.unwrap_or(0)), V::I(y[0]), V::I(y[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty FROM Users u
//     LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.Reputation),
// PostHistoryWithComments AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, ph.Comment, ph.Text, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS CommentRank
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)),
// FlaggedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, u.UserId, u.Reputation, u.QuestionCount, u.TotalBounty, COALESCE(phwc.Comment, 'No Comments') AS LastHistoryComment,
//        COALESCE(phwc.Text, 'No Text') AS LastHistoryText FROM RankedPosts rp JOIN UserReputation u ON rp.PostId = u.UserId LEFT JOIN PostHistoryWithComments phwc ON rp.PostId = phwc.PostId AND phwc.CommentRank = 1)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.Reputation, fp.QuestionCount, fp.TotalBounty, fp.LastHistoryComment, fp.LastHistoryText
// FROM FlaggedPosts fp WHERE fp.Score < 0 AND fp.Reputation < 100 ORDER BY fp.Score ASC, fp.ViewCount DESC;
//
// `rp.PostId = u.UserId` compares a post id with a user id, so it goes through the raw ids. The latest history row per post breaks ties on CreationDate by the larger id (the SQL leaves them open).
fn q31084(db: &'static So) -> String {
    let Post { post_type_id, score, origid, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let q = || Ident::<Post>::new().with(post_type_id.eq(1));
    let us = || db.user.with((&db.user.reputation).lt(100));
    let tb = us().group_by(Ident::<User>::new()).select(posts_of(db).select(q().select(bounty.opt())).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let qc = us().group_by(Ident::<User>::new()).select(posts_of(db).select(q()).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let uid: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let lw = db.post_history.with(post_history_type_id.is_in([10, 11, 12])).group_by(post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (d, h), desc);
    let last: HashIdx<Id<Post>, Id<PostHistory>> = (&lw).filt(|(_, n)| n == 1).map(|((h, _), _)| h).collect();
    let v = drain(db.post.with(post_type_id.eq(1).and(score.lt(0))).select(origid.select(&uid).select(Ident::<User>::new().and(&tb).and(&qc)).and((&last).opt())));
    rows(v.into_iter().map(|(p, (((u, b), n), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([user_col(db, u, "rep"), V::I(n), V::I(b)]);
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No Comments")));
        f.push(V::S(h.and_then(|h| db.post_history.text.get(h)).unwrap_or("No Text")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(NULLIF(P.AcceptedAnswerId, -1), 0) AS AcceptedAnswerId, COALESCE(V.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(V.DownVotes, 0) AS TotalDownVotes, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS UserRank, P.OwnerUserId
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes GROUP BY PostId) V ON P.Id = V.PostId WHERE P.PostTypeId = 1),
// UserBadges AS (SELECT UserId, COUNT(*) FILTER (WHERE Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE Class = 3) AS BronzeBadges FROM Badges GROUP BY UserId),
// ClosedPosts AS (SELECT PH.PostId, MIN(PH.CreationDate) AS FirstClosedDate, COUNT(*) AS CloseChangeCount FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, CP.FirstClosedDate, CP.CloseChangeCount, RP.TotalUpVotes, RP.TotalDownVotes,
//        CASE WHEN CP.CloseChangeCount > 0 THEN 'Yes' ELSE 'No' END AS IsClosed, CASE WHEN RP.UserRank = 1 THEN 'Most Recent' ELSE 'Other' END AS PostRank
// FROM RankedPosts RP LEFT JOIN UserBadges UB ON RP.OwnerUserId = UB.UserId LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId WHERE RP.TotalUpVotes > 10 OR RP.UserRank = 1
// ORDER BY RP.CreationDate DESC, RP.TotalUpVotes DESC;
fn q1466(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    type T = (Id<Post>, i64);
    let rp: MatSet<T> = (&w).map(|((p, _), r)| (p, r)).collect();
    let ud = post_updown(db, db.post.with(post_type_id.eq(1)));
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(hd).fold((i64::MAX, 0i64), |(m, n), d| (m.min(d), n + 1));
    let pp = || Same::<T>::new().map(|(p, _): T| p);
    let j = (&rp).select(Same::<T>::new().and(pp().select(&ud)).filt(|((_, r), a): (T, [i64; 2])| a[0] > 10 || r == 1).and(pp().select(owner_user.select(&ub)).opt()).and(pp().select((&cp).opt())));
    rows(drain(j).into_iter().map(|(_, ((((p, r), a), b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some((d, n)) => [V::T(d), V::I(n)],
            None => [V::Null, V::Null],
        });
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if c.is_some() { "Yes" } else { "No" }), V::S(if r == 1 { "Most Recent" } else { "Other" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(c.Score) AS TotalCommentScore, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        AVG(EXTRACT(EPOCH FROM (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - p.CreationDate)) / 60) AS AvgPostAgeInMinutes, MAX(p.CreationDate) AS LastActivityDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PostScore AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS ScoreRank FROM Posts p WHERE p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ua.DisplayName AS UserDisplayName, ua.UserId, ua.TotalPosts, ua.TotalQuestions, ua.TotalAnswers, ua.TotalCommentScore, ua.TotalUpVotes,
//        ua.TotalDownVotes FROM PostScore ps JOIN UserActivity ua ON ps.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = ua.UserId) WHERE ps.ScoreRank <= 100)
// SELECT pp.UserDisplayName, pp.Title, pp.Score, pp.ViewCount, pp.TotalPosts, pp.TotalQuestions, pp.TotalAnswers, pp.TotalCommentScore, pp.TotalUpVotes, pp.TotalDownVotes
// FROM TopPosts pp ORDER BY pp.Score DESC, pp.ViewCount DESC;
//
// `ps.PostId IN (posts of ua)` joins each top post to its owner, so the post x comment x vote product is driven for those owners alone.
fn q8656(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.is_in([1, 2])).select(score)), |&(p, s)| (Reverse(s), p), 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let prod = post_type_id.and(comments_of(db).select(&db.comment.score).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt());
    let ua = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(prod)).fold([0i64; 7], |a, ((t, c), v)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + c.unwrap_or(0), a[5] + (v == Some(2)) as i64, a[6] + (v == Some(3)) as i64]
    });
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ua)))));
    rows(v.into_iter().map(|(_, (p, (u, a)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), V::I(a[5]), V::I(a[6])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.LastActivityDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserDetails AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostMetrics AS (SELECT rp.PostId, rp.Title, ud.DisplayName, ud.Reputation, ud.BadgeCount, ud.GoldBadges, ud.SilverBadges, ud.BronzeBadges, COALESCE(cp.CloseCount, 0) AS CloseCount,
//        COALESCE(cp.LastClosedDate, DATE '1900-01-01') AS LastClosedDate, rp.ViewCount FROM RankedPosts rp JOIN UserDetails ud ON rp.OwnerUserId = ud.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT pm.Title, pm.DisplayName, pm.Reputation, pm.BadgeCount, pm.GoldBadges, pm.SilverBadges, pm.BronzeBadges, pm.CloseCount, pm.LastClosedDate, pm.ViewCount
// FROM PostMetrics pm WHERE pm.CloseCount > 0 AND pm.Reputation >= 1000 ORDER BY pm.ViewCount DESC, pm.Reputation DESC LIMIT 10;
fn q33231(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain(db.post.with(post_type_id.eq(1)).select((&cp).and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).ge(1000)).and(user_badge_classes(db))))));
    let v = top_n(v, |&(p, (_, (u, _)))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(db.user.reputation.get(u).unwrap()), p)
    }, 10);
    rows(v.into_iter().map(|(p, ((n, d), (u, b)))| {
        let mut f = vec![title(db, p)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(b.map(V::I));
        f.extend([V::I(n), V::T(d)]);
        f.extend(post_fields(db, p, &["views"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotesCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopVotes AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerUserId, rp.UpVotesCount, rp.DownVotesCount, CASE WHEN (rp.UpVotesCount - rp.DownVotesCount) < 0 THEN 'Negative Votes'
//        WHEN (rp.UpVotesCount - rp.DownVotesCount) = 0 THEN 'No Votes' ELSE 'Positive Votes' END AS VoteStatus FROM RankedPosts rp WHERE rp.rn = 1),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.VoteStatus, u.DisplayName AS OwnerDisplayName, u.Reputation, COALESCE(b.BadgeCount, 0) AS BadgeCount FROM TopVotes tp
//     JOIN Users u ON tp.OwnerUserId = u.Id LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON u.Id = b.UserId)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.OwnerDisplayName, pd.VoteStatus, pd.Reputation, pd.BadgeCount,
//        CASE WHEN pd.BadgeCount > 5 THEN 'Expert' WHEN pd.BadgeCount BETWEEN 3 AND 5 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel
// FROM PostDetails pd WHERE pd.VoteStatus = 'Positive Votes' ORDER BY pd.CreationDate DESC LIMIT 10 OFFSET 5;
fn q24393(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let fw = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&fw).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&post_updown(db, &first)).filt(|a| a[0] > a[1]).and(owner_user.select(Ident::<User>::new().and((&bc).opt()))));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 15);
    rows(v.into_iter().skip(5).map(|(p, (_, (u, b)))| {
        let b = b.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::S("Positive Votes"), user_col(db, u, "rep"), V::I(b), V::S(if b > 5 { "Expert" } else if b >= 3 { "Intermediate" } else { "Novice" })]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostCount AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, RANK() OVER (ORDER BY PostCount DESC) AS UserRank FROM UserPostCount WHERE PostCount > 0),
// PostVoteDetails AS (SELECT P.Id AS PostId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, MAX(B.Class) AS HighestBadgeLevel FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// FinalResults AS (SELECT U.UserId, U.DisplayName, U.PostCount, PV.VoteCount, PV.UpVotes, PV.DownVotes, UB.BadgeCount, UB.HighestBadgeLevel FROM TopUsers U
//     LEFT JOIN PostVoteDetails PV ON U.UserId = PV.PostId LEFT JOIN UserBadges UB ON U.UserId = UB.UserId)
// SELECT FR.UserId, FR.DisplayName, COALESCE(FR.PostCount, 0) AS TotalPosts, COALESCE(FR.VoteCount, 0) AS TotalVotes, COALESCE(FR.UpVotes, 0) AS TotalUpVotes, COALESCE(FR.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(FR.BadgeCount, 0) AS TotalBadges, COALESCE(FR.HighestBadgeLevel, 0) AS HighestBadgeLevel,
//        CASE WHEN FR.HighestBadgeLevel = 1 THEN 'Gold' WHEN FR.HighestBadgeLevel = 2 THEN 'Silver' WHEN FR.HighestBadgeLevel = 3 THEN 'Bronze' ELSE 'No Badge' END AS BadgeLevelDescription
// FROM FinalResults FR ORDER BY TotalPosts DESC, TotalVotes DESC LIMIT 10;
//
// WITH RECURSIVE, but no CTE refers to itself. `U.UserId = PV.PostId` compares a user id with a post id, so it goes through the raw ids.
fn q31868(db: &'static So) -> String {
    let ups = user_posts(db);
    let pid: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let v = drain((&ups).filt(|a| a[1] > 0).and((&db.user.origid).select(&pid).select(&pv).opt()).and(&ub));
    let v = top_n(v, |&(u, ((a, x), _))| (Reverse(a[1]), Reverse(x.map_or(0, |x| x[0])), u), 10);
    rows(v.into_iter().map(|(u, ((a, x), (n, m)))| {
        let x = x.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(x[0]), V::I(x[1]), V::I(x[2]), V::I(n), V::I(m)]);
        f.push(V::S(match m { 1 => "Gold", 2 => "Silver", 3 => "Bronze", _ => "No Badge" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS Author, COUNT(a.Id) AS AnswerCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, u.DisplayName, p.Title, p.Body, p.CreationDate),
// TagStatistics AS (SELECT t.TagName, COUNT(pt.PostId) AS PostCount, SUM(pt.AnswerCount) AS TotalAnswers, AVG(pt.UpVotes - pt.DownVotes) AS NetVotes FROM Tags t
//     JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN RankedPosts pt ON pt.PostId = p.Id GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, TotalAnswers, NetVotes, RANK() OVER (ORDER BY TotalAnswers DESC) AS RankByAnswers, RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts,
//        RANK() OVER (ORDER BY NetVotes DESC) AS RankByNetVotes FROM TagStatistics)
// SELECT TagName, PostCount, TotalAnswers, NetVotes, CASE WHEN RankByAnswers = 1 THEN 'Top' WHEN RankByAnswers <= 5 THEN 'Top 5' ELSE 'Other' END AS AnswerRank,
//        CASE WHEN RankByPosts = 1 THEN 'Top' WHEN RankByPosts <= 5 THEN 'Top 5' ELSE 'Other' END AS PostRank, CASE WHEN RankByNetVotes = 1 THEN 'Top' WHEN RankByNetVotes <= 5 THEN 'Top 5' ELSE 'Other' END AS VoteRank
// FROM TopTags WHERE PostCount > 10 ORDER BY NetVotes DESC, TotalAnswers DESC;
fn q27475(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ts_ = db.tag.group_by(&db.tag.tag_name).select((&by_tag).map(|(p, _)| p).select(&rp)).fold([0i64; 3], |a, x| [a[0] + 1, a[1] + x[0], a[2] + x[1] - x[2]]);
    type A = (Str, [i64; 3]);
    let w1 = whole(&ts_).select(Same::<Str>::new().and(&ts_)).window(rank, |(_, a)| Reverse(a[1]), asc);
    let w2 = (&w1).window(rank, |((_, a), _): (A, i64)| Reverse(a[0]), asc);
    let w3 = (&w2).window(rank, |(((_, a), _), _): ((A, i64), i64)| Reverse(fkey(a[2] as f64 / a[0] as f64)), asc);
    let lab = |r: i64| V::S(if r == 1 { "Top" } else if r <= 5 { "Top 5" } else { "Other" });
    let v = drain((&w3).filt(|((((_, a), _), _), _): (((A, i64), i64), i64)| a[0] > 10));
    rows(v.into_iter().map(|(_, ((((t, a), ra), rp), rn))| row(vec![V::S(t), V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), lab(ra), lab(rp), lab(rn)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.Reputation),
// PostHistoryDetails AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(phd.EditCount, 0) AS EditCount, phd.LastEditDate, ur.Reputation, ur.BadgeCount, ur.GoldBadges,
//        ur.SilverBadges, ur.BronzeBadges FROM RankedPosts rp LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.PostRank = 1)
// SELECT ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.EditCount, ps.LastEditDate, ps.Reputation, ps.BadgeCount, ps.GoldBadges, ps.SilverBadges, ps.BronzeBadges
// FROM PostStatistics ps WHERE ps.Reputation > 100 ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q31452(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let fw = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&fw).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&first).select(Ident::<Post>::new().and((&phd).opt()).and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100)).and(user_badge_classes(db))))));
    rows(v.into_iter().map(|(_, ((p, e), (u, b)))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(match e {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        f.push(user_col(db, u, "rep"));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT u.Id AS UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY u.Id),
// TopUsers AS (SELECT UserId, TotalVotes, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY TotalVotes DESC) AS Rank FROM UserVoteCounts WHERE TotalVotes > 0),
// PostStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END), 0) AS DownVotes,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY p.Id),
// TopPosts AS (SELECT ps.PostId, ps.CommentCount, ps.UpVotes, ps.DownVotes, ROW_NUMBER() OVER (ORDER BY ps.UpVotes DESC, ps.CommentCount DESC) AS Rank FROM PostStats ps WHERE ps.UpVotes > 0 OR ps.CommentCount > 0)
// SELECT tu.UserId, tu.TotalVotes, tu.UpVotes AS UserUpVotes, tu.DownVotes AS UserDownVotes, tp.PostId, tp.CommentCount, tp.UpVotes AS PostUpVotes, tp.DownVotes AS PostDownVotes
// FROM TopUsers tu JOIN TopPosts tp ON tu.UpVotes > tp.UpVotes WHERE tu.Rank <= 10 AND tp.Rank <= 10 ORDER BY tu.TotalVotes DESC, tp.UpVotes DESC;
//
// The ON clause compares a column of each side but no key, so the top users and top posts are crossed and the pairs filtered.
fn q5984(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let tu = rel(top_n(drain(&uv), |&(u, a)| (Reverse(a[0]), u), 10));
    let ps = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(vtype_name(db)).opt())).fold([0i64; 3], |a, (c, n)| {
        [a[0] + c.is_some() as i64, a[1] + (n == Some("UpMod")) as i64, a[2] + (n == Some("DownMod")) as i64]
    });
    let tp = rel(top_n(drain((&ps).filt(|a| a[1] > 0 || a[0] > 0)), |&(p, a)| (Reverse(a[1]), Reverse(a[0]), p), 10));
    let mut v = Vec::new();
    (&tu).cross(&tp).filt(|((_, a), (_, b)): ((Id<User>, [i64; 3]), (Id<Post>, [i64; 3]))| a[1] > b[1]).drive(|_, ((u, a), (p, b))| v.push((u, a, p, b)));
    rows(v.into_iter().map(|(u, a, p, b)| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id"]));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// RecentVotes AS (SELECT v.PostId, v.VoteTypeId, COUNT(v.VoteTypeId) AS VoteCount FROM Votes v WHERE v.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId, v.VoteTypeId),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(*) AS HistoryCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph GROUP BY ph.PostId),
// FrequentlyEditedPosts AS (SELECT p.Id, p.Title, ps.HistoryCount, ps.LastEditDate FROM Posts p JOIN PostHistoryStats ps ON p.Id = ps.PostId WHERE ps.HistoryCount > 5),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, COALESCE(rv.VoteCount, 0) AS RecentVoteCount, COALESCE(fep.HistoryCount, 0) AS EditCount
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId AND rv.VoteTypeId = 2 LEFT JOIN FrequentlyEditedPosts fep ON rp.PostId = fep.Id WHERE rp.PostRank = 1)
// SELECT fr.PostId, fr.Title, fr.CreationDate, fr.Score, fr.OwnerDisplayName, fr.RecentVoteCount, fr.EditCount FROM FinalResults fr ORDER BY fr.Score DESC, fr.RecentVoteCount DESC LIMIT 10;
fn q34799(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let fw = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&fw).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let Vote { post, vote_type_id, creation_date: vd, .. } = &db.vote;
    let rv = db.vote.with(vd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(vote_type_id.eq(2))).group_by(post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let hc = db.post_history.group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&first).select(Ident::<Post>::new().and((&rv).opt()).and((&hc).filt(|n| n > 5).opt())));
    let v = top_n(v, |&(_, ((p, r), _))| (Reverse(score.get(p).unwrap()), Reverse(r.unwrap_or(0)), p), 10);
    rows(v.into_iter().map(|(_, ((p, r), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(r.unwrap_or(0)), V::I(h.unwrap_or(0))]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("5065", q5065),
    ("20337", q20337),
    ("26087", q26087),
    ("5172", q5172),
    ("31570", q31570),
    ("7144", q7144),
    ("2246", q2246),
    ("23200", q23200),
    ("27273", q27273),
    ("1051", q1051),
    ("2561", q2561),
    ("2857", q2857),
    ("34272", q34272),
    ("516", q516),
    ("1925", q1925),
    ("27570", q27570),
    ("3366", q3366),
    ("34619", q34619),
    ("3693", q3693),
    ("30063", q30063),
    ("9803", q9803),
    ("1479", q1479),
    ("34544", q34544),
    ("28379", q28379),
    ("23752", q23752),
    ("34974", q34974),
    ("6947", q6947),
    ("33132", q33132),
    ("643", q643),
    ("25800", q25800),
    ("34851", q34851),
    ("9172", q9172),
    ("32053", q32053),
    ("5567", q5567),
    ("7123", q7123),
    ("1714", q1714),
    ("3619", q3619),
    ("1771", q1771),
    ("26811", q26811),
    ("27203", q27203),
    ("27951", q27951),
    ("5943", q5943),
    ("28511", q28511),
    ("22976", q22976),
    ("55", q55),
    ("20548", q20548),
    ("25941", q25941),
    ("31367", q31367),
    ("31244", q31244),
    ("415", q415),
    ("2181", q2181),
    ("2061", q2061),
    ("1057", q1057),
    ("33460", q33460),
    ("9960", q9960),
    ("348", q348),
    ("8205", q8205),
    ("3822", q3822),
    ("7328", q7328),
    ("24163", q24163),
    ("8072", q8072),
    ("23505", q23505),
    ("29615", q29615),
    ("24737", q24737),
    ("31757", q31757),
    ("210", q210),
    ("33090", q33090),
    ("25272", q25272),
    ("28886", q28886),
    ("34788", q34788),
    ("33435", q33435),
    ("34918", q34918),
    ("8792", q8792),
    ("26547", q26547),
    ("6632", q6632),
    ("4972", q4972),
    ("33633", q33633),
    ("2510", q2510),
    ("34298", q34298),
    ("26717", q26717),
    ("5434", q5434),
    ("28751", q28751),
    ("1926", q1926),
    ("34320", q34320),
    ("25073", q25073),
    ("31084", q31084),
    ("1466", q1466),
    ("8656", q8656),
    ("33231", q33231),
    ("24393", q24393),
    ("31868", q31868),
    ("27475", q27475),
    ("31452", q31452),
    ("5984", q5984),
    ("34799", q34799),
];
