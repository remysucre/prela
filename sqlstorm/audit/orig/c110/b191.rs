use harness::prelude::*;
use std::cmp::Reverse;

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn agg_distinct(mut v: Vec<&'static str>, sep: &str) -> Option<Str> {
    v.sort();
    v.dedup();
    if v.is_empty() { None } else { Some(leak(v.join(sep))) }
}

fn close_reasons(db: &'static So) -> HashIdx<i64, Id<CloseReasonType>> {
    (&db.close_reason_type.origid).inv().collect()
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title AS PostTitle, P.Body, U.DisplayName AS OwnerDisplayName, P.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND P.Score IS NOT NULL AND P.PostTypeId IN (1, 2)),
// PostVoteDetails AS (SELECT V.PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN V.VoteTypeId IN (10, 11) THEN 1 END) AS Deletions, COUNT(CASE WHEN V.VoteTypeId IN (4, 12) THEN 1 END) AS OffensiveReports FROM Votes V GROUP BY V.PostId),
// PostHistoryDetails AS (SELECT PH.PostId, COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 12) THEN 1 END) AS Changes, STRING_AGG(PH.Comment, '; ') AS Comments
//     FROM PostHistory PH WHERE PH.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY PH.PostId)
// SELECT RP.PostId, RP.PostTitle, RP.OwnerDisplayName, RP.CreationDate, COALESCE(PVD.UpVotes, 0) AS TotalUpVotes, COALESCE(PVD.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(PVD.Deletions, 0) AS TotalDeletions, COALESCE(PVD.OffensiveReports, 0) AS TotalOffensiveReports, COALESCE(PHD.Changes, 0) AS TotalChanges,
//        COALESCE(PHD.Comments, 'No comments') AS RecentComments
// FROM RankedPosts RP LEFT JOIN PostVoteDetails PVD ON RP.PostId = PVD.PostId LEFT JOIN PostHistoryDetails PHD ON RP.PostId = PHD.PostId
// WHERE RP.Rank <= 5 AND (COALESCE(PVD.UpVotes, 0) - COALESCE(PVD.DownVotes, 0) > 0 OR RP.PostTitle LIKE '%interesting%') ORDER BY RP.Rank, RP.CreationDate DESC;
//
// Rank reads only base columns, so the posts are ranked first. The STRING_AGG order is left open by the SQL; the port joins in history id order.
fn q21364(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, title, .. } = &db.post;
    let cd = current_date();
    let v = drain(db.post.with(creation_date.ge(add_years(cd, -1))).with(post_type_id.is_in([1, 2])).select(post_type_id));
    let r = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), false), |x| x.1);
    let top: Vec<(Id<Post>, i64)> = r.into_iter().filter(|x| x.1 <= 5).map(|((p, _), k)| (p, k)).collect();
    let tp: MatSet<Id<Post>> = rel(top.clone()).map(|x: (Id<Post>, i64)| x.0).collect();
    let pvd = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 4], |a, t| {
        [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 10 || t == 11) as i64, a[3] + (t == 4 || t == 12) as i64]
    });
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.with(hd.ge(add_months(cd, -6))).group_by(post).select(post_history_type_id.and(comment.opt())).buf_fold(|v| {
        let n = v.iter().filter(|x| x.0 == 10 || x.0 == 12).count() as i64;
        let c: Vec<&str> = v.iter().filter_map(|x| x.1).collect();
        (n, if c.is_empty() { None } else { Some(leak(c.join("; "))) })
    });
    type R = (Id<Post>, i64);
    type J = (((R, Option<[i64; 4]>), Option<Str>), Option<(i64, Option<Str>)>);
    let k = || Same::<R>::new().map(|x: R| x.0);
    let tv = rel(top);
    let v = drain(
        (&tv)
            .select(Same::<R>::new().and(k().select(&pvd).opt()).and(k().select(title.opt())).and(k().select(&phd).opt()))
            .filt(|(((_, a), t), _): J| {
                let a = a.unwrap_or([0; 4]);
                a[0] - a[1] > 0 || t.map_or(false, |t| t.contains("interesting"))
            }),
    );
    rows(v.into_iter().map(|(_, ((((p, _), a), _), h))| {
        let a = a.unwrap_or([0; 4]);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend(a.map(V::I));
        f.push(V::I(h.map_or(0, |h| h.0)));
        f.push(V::S(h.and_then(|h| h.1).unwrap_or("No comments")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.Reputation,
//        DENSE_RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.ViewCount DESC) AS Rank, P.OwnerUserId
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, MAX(B.Class) AS HighestBadgeClass FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostVoteSummary AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(V.Id) AS TotalVotes FROM Votes V GROUP BY V.PostId),
// ClosedPosts AS (SELECT PH.PostId, MAX(PH.CreationDate) AS LastClosedDate, STRING_AGG(DISTINCT CT.Name, ', ') AS CloseReasons
//     FROM PostHistory PH JOIN CloseReasonTypes CT ON PH.Comment::int = CT.Id WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId)
// SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, U.UserId, U.BadgeCount, U.HighestBadgeClass, PVS.UpVotes, PVS.DownVotes, PVS.TotalVotes, CPL.LastClosedDate, CPL.CloseReasons,
//        CASE WHEN CPL.LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RankedPosts RP LEFT JOIN UserBadges U ON RP.OwnerUserId = U.UserId LEFT JOIN PostVoteSummary PVS ON RP.PostId = PVS.PostId LEFT JOIN ClosedPosts CPL ON RP.PostId = CPL.PostId
// WHERE RP.Rank <= 3 AND (U.BadgeCount IS NULL OR U.BadgeCount BETWEEN 1 AND 5) ORDER BY RP.PostId;
//
// Rank reads only base columns, so the posts are ranked first. The distinct close reasons are joined in name order (the SQL leaves the order open).
fn q23903(db: &'static So) -> String {
    let Post { creation_date, score, view_count, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let r = per_group(
        ranked(v, |&(p, t)| {
            let w = view_count.get(p);
            (t, Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
        }, true),
        |x| x.1,
    );
    let tp: MatSet<Id<Post>> = rel(r.into_iter().filter(|x| x.1 <= 3).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ub = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, i64::MIN), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cpl = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name)))
        .buf_fold(|v| (v.iter().map(|x| x.0).max().unwrap(), agg_distinct(v.iter().map(|x| x.1).collect(), ", ").unwrap()));
    type J = (((Id<Post>, Option<(Id<User>, (i64, i64))>), Option<[i64; 3]>), Option<(i64, Str)>);
    let v = drain(
        (&tp)
            .select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ub)).opt()).and((&pvs).opt()).and((&cpl).opt()))
            .filt(|(((_, u), _), _): J| u.map_or(true, |(_, (n, _))| (1..=5).contains(&n))),
    );
    rows(v.into_iter().map(|(_, (((p, u), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(match u {
            Some((u, (n, m))) => [user_col(db, u, "uid"), V::I(n), omax(m, n)],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some((d, s)) => [V::T(d), V::S(s)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if c.is_some() { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.Score IS NOT NULL AND p.ViewCount > 0),
// ClosedPostHistories AS (SELECT ph.PostId, ph.CreationDate AS ClosedDate, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons
//     FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate),
// FinalResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.UpVotes, rp.DownVotes, CASE WHEN ch.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus,
//        COALESCE(ch.CloseReasons, 'No Close Reasons') AS CloseReasons FROM RankedPosts rp LEFT JOIN ClosedPostHistories ch ON rp.PostId = ch.PostId WHERE rp.rn = 1)
// SELECT FR.PostId, FR.Title, FR.CreationDate, FR.ViewCount, FR.Score, FR.UpVotes, FR.DownVotes, FR.PostStatus, FR.CloseReasons,
//        CONCAT('Post ID: ', FR.PostId, ', Title: ', FR.Title, ', Status: ', FR.PostStatus) AS PostDescription
// FROM FinalResults FR WHERE FR.ViewCount > (SELECT AVG(ViewCount) FROM Posts) AND FR.Score > (SELECT AVG(Score) FROM Posts) ORDER BY FR.CreationDate DESC LIMIT 50;
//
// rn = 1 keeps one joined row per post type, from its newest post, so the newest posts are picked first; the window sums are that post's totals.
// A tie on CreationDate goes to the smaller post id (the SQL leaves it open). The distinct close reasons are joined in name order.
fn q24602(db: &'static So) -> String {
    let Post { view_count, creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(view_count.gt(0)).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let (vs, vn) = db.post.select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let (ss, sn) = db.post.select(score).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let (av, asc) = (vs as f64 / vn as f64, ss as f64 / sn as f64);
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let ch = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post.and(hd))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| agg_distinct(v.iter().copied().collect(), ", ").unwrap());
    let chv = rel(drain(&ch));
    type C = ((Id<Post>, i64), Str);
    let chi: HashIdx<Id<Post>, C> = (&chv).map(|x: C| x.0 .0).inv().select(&chv).collect();
    let v = drain(
        (&tp)
            .with(view_count.filt(|w| w as f64 > av))
            .with(score.filt(|s| s as f64 > asc))
            .select(Ident::<Post>::new().and(&ud).and((&chi).opt())),
    );
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(_, ((p, a), c))| {
        let st = if c.is_some() { "Closed" } else { "Active" };
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(st), V::S(c.map_or("No Close Reasons", |c| c.1))]);
        f.push(V::Owned(format!("Post ID: {}, Title: {}, Status: {st}", db.post.origid.get(p).unwrap(), title_or_empty(db, p))));
        row(f)
    }))
}

fn title_or_empty(db: &'static So, p: Id<Post>) -> &'static str {
    db.post.title.get(p).unwrap_or("")
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, CreationDate, LastAccessDate, DisplayName, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswer,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes v GROUP BY v.PostId),
// ClosedPosts AS (SELECT p.Id AS ClosedPostId, ph.UserId AS CloserUserId, ph.CreationDate AS ClosedDate, ph.Comment,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS ClosureRank FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId = 10)
// SELECT u.DisplayName, u.Reputation, COUNT(DISTINCT rp.PostId) AS RecentPostCount, SUM(ps.Upvotes) - SUM(ps.Downvotes) AS NetVotes, STRING_AGG(DISTINCT cp.Comment, '; ') AS CloseComments,
//        STRING_AGG(DISTINCT COALESCE(CAST(cp.ClosedPostId AS TEXT), 'N/A'), ', ') AS ClosedPostIds, SUM(CASE WHEN rp.RecentPostRank = 1 THEN 1 ELSE 0 END) AS MostRecentPostExists
// FROM UserReputation u LEFT JOIN RecentPosts rp ON u.UserId = rp.OwnerUserId LEFT JOIN PostVoteSummary ps ON rp.PostId = ps.PostId
// LEFT JOIN ClosedPosts cp ON rp.PostId = cp.ClosedPostId AND cp.ClosureRank = 1
// WHERE u.Reputation > 1000 GROUP BY u.DisplayName, u.Reputation ORDER BY u.Reputation DESC;
//
// The distinct strings are joined in sorted order (the SQL leaves it open). A CreationDate tie inside a rank goes to the smaller id.
fn q2762(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, origid, .. } = &db.post;
    let recent = || Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let first = top_per(drain(db.post.select(recent()).select(owner_user_id.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = db.post.select(recent()).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let lc = top_per(drain(db.post_history.with(post_history_type_id.eq(10)).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false);
    let lc: MatSet<Id<PostHistory>> = rel(lc.into_iter().map(|x| x.0).collect()).map(|h| h).collect();
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(&lc)).select(comment.opt());
    let User { display_name, reputation, .. } = &db.user;
    let g = db
        .user
        .with(reputation.gt(1000))
        .group_by(display_name.and(reputation))
        .select(posts_of(db).select(recent()).select(Ident::<Post>::new().and(origid).and((&ps).opt()).and(cp.opt()).and(Ident::<Post>::new().with(&first).opt())).opt())
        .buf_fold(|v| {
            let mut ids: Vec<Id<Post>> = v.iter().filter_map(|x| x.map(|x| x.0 .0 .0 .0)).collect();
            ids.sort();
            ids.dedup();
            let (mut up, mut dn, mut any, mut mr) = (0i64, 0i64, false, 0i64);
            let mut cs: Vec<&'static str> = Vec::new();
            let mut pids: Vec<&'static str> = Vec::new();
            for x in v.iter() {
                if let Some(((((_, oid), a), c), f)) = *x {
                    if let Some(a) = a {
                        up += a[0];
                        dn += a[1];
                        any = true;
                    }
                    if let Some(Some(c)) = c {
                        cs.push(c);
                    }
                    pids.push(if c.is_some() { leak(oid.to_string()) } else { "N/A" });
                    mr += f.is_some() as i64;
                } else {
                    pids.push("N/A");
                }
            }
            (ids.len() as i64, if any { Some(up - dn) } else { None }, agg_distinct(cs, "; "), agg_distinct(pids, ", ").unwrap(), mr)
        });
    rows(drain(&g).into_iter().map(|((n, r), (c, nv, cc, ci, mr))| row(vec![V::S(n), V::I(r), V::I(c), oint(nv), ostr(cc), V::S(ci), V::I(mr)])))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year'),
// RecentPosts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.AnswerCount, ua.DisplayName AS OwnerName, ua.TotalUpvotes, ua.TotalDownvotes
//     FROM PostStats ps JOIN UserActivity ua ON ps.PostId = ANY (SELECT Id FROM Posts WHERE OwnerUserId = ua.UserId) WHERE ps.rn = 1 AND ps.Score > 0)
// SELECT rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, COALESCE(rp.TotalUpvotes - rp.TotalDownvotes, 0) AS NetVotes,
//        CASE WHEN rp.TotalUpvotes > 3 THEN 'Active User' WHEN rp.TotalDownvotes > 3 THEN 'Content Issues' ELSE 'Moderate User' END AS UserActivityStatus,
//        STRING_AGG(DISTINCT pt.Name, ', ') AS RelatedPostTypes
// FROM RecentPosts rp LEFT JOIN PostTypes pt ON pt.Id = (SELECT PostTypeId FROM Posts WHERE Id = rp.PostId)
// GROUP BY rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.TotalUpvotes, rp.TotalDownvotes ORDER BY NetVotes DESC, rp.ViewCount DESC LIMIT 10;
//
// `ps.PostId = ANY (posts of ua.UserId)` is the post's owner. The scalar subquery reads the post's own type. rn ties go to the smaller post id,
// and the distinct type names are joined in name order (the SQL leaves both open). The votes x posts product is driven for the owners of the picked posts alone.
fn q24520(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, score, title, view_count, answer_count, .. } = &db.post;
    let cut = add_years(utc_to_ny(now_utc()), -1);
    let v = drain(db.post.with(creation_date.filt(move |d| ny_to_utc(d) >= ny_to_utc(cut))).select(owner_user_id.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ua = (&owners)
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type K = (Option<Str>, i64, Option<i64>, Option<i64>, [i64; 2]);
    type X = (K, Str);
    let rp = drain(
        (&tp)
            .with(score.gt(0))
            .select(title.opt().and(score).and(view_count.opt()).and(answer_count.opt()).and(owner_user.select(&ua)).and(ptype_name(db)))
            .map(|(((((t, s), w), a), u), n)| ((t, s, w, a, u), n)),
    );
    let g = rel(rp.into_iter().map(|x| x.1).collect::<Vec<X>>())
        .group_by(Same::<X>::new().map(|x: X| x.0))
        .select(Same::<X>::new().map(|x: X| x.1))
        .buf_fold(|v| agg_distinct(v.iter().copied().collect(), ", ").unwrap());
    let v = top_n(drain(&g), |&((_, _, w, _, u), _)| (Reverse(u[0] - u[1]), w.is_none(), Reverse(w)), 10);
    rows(v.into_iter().map(|((t, s, w, a, u), n)| {
        let st = if u[0] > 3 { "Active User" } else if u[1] > 3 { "Content Issues" } else { "Moderate User" };
        row(vec![ostr(t), V::I(s), oint(w), oint(a), V::I(u[0] - u[1]), V::S(st), V::S(n)])
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, CreationDate, Rank() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStatistics AS (SELECT P.Id AS PostId, P.OwnerUserId, P.PostTypeId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN V.VoteTypeId = 6 THEN 1 END) AS CloseVotes, MAX(P.CreationDate) AS LastActivity,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY MAX(P.CreationDate) DESC) AS PostRank
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.OwnerUserId, P.PostTypeId),
// UserPosts AS (SELECT U.Id AS UserId, U.DisplayName, PS.PostId, PS.PostTypeId, PS.UpVotes, PS.DownVotes, PS.CloseVotes, PS.LastActivity
//     FROM Users U JOIN PostStatistics PS ON U.Id = PS.OwnerUserId WHERE PS.PostRank <= 3),
// ClosedPosts AS (SELECT PH.PostId, COUNT(*) AS HistoryCount, STRING_AGG(DISTINCT PT.Name, ', ') AS HistoryTypes
//     FROM PostHistory PH JOIN PostHistoryTypes PT ON PH.PostHistoryTypeId = PT.Id WHERE PH.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY PH.PostId)
// SELECT UP.UserId, UP.DisplayName, COUNT(DISTINCT UP.PostId) AS TotalPosts, SUM(UP.UpVotes) AS TotalUpVotes, SUM(UP.DownVotes) AS TotalDownVotes,
//        COALESCE(CP.HistoryCount, 0) AS ClosedPostHistoryCount, COALESCE(CP.HistoryTypes, 'No History') AS ClosedPostHistoryTypes, R.ReputationRank
// FROM UserPosts UP LEFT JOIN ClosedPosts CP ON UP.PostId = CP.PostId JOIN UserReputation R ON UP.UserId = R.UserId
// WHERE R.Reputation > 1000 GROUP BY UP.UserId, UP.DisplayName, CP.HistoryCount, CP.HistoryTypes, R.ReputationRank ORDER BY TotalPosts DESC, R.ReputationRank ASC LIMIT 10;
//
// PostRank reads only CreationDate, so the posts are ranked first (a tie goes to the smaller post id; the SQL leaves it open). The distinct type names are joined in name order.
fn q24245(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let top = top_per(drain(db.post.select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let tv = rel(top);
    type P = (Id<Post>, Id<User>);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(hd.ge(add_days(date(2024, 10, 1), -30))).group_by(post).select(htype_name(db)).buf_fold(|v| (v.len() as i64, agg_distinct(v.iter().copied().collect(), ", ").unwrap()));
    let pv = votes_of(db).select(&db.vote.vote_type_id).opt();
    type X = (P, (Option<(i64, Str)>, Option<i64>));
    let up = drain(
        (&tv)
            .select(Same::<P>::new().with(Same::<P>::new().map(|x: P| x.1).select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))))
            .select(Same::<P>::new().and(Same::<P>::new().map(|x: P| x.0).select((&cp).opt().and(pv)))),
    );
    let g = rel(up.into_iter().map(|x| x.1).collect::<Vec<X>>())
        .group_by(Same::<X>::new().map(|x: X| (x.0 .1, x.1 .0)))
        .select(Same::<X>::new())
        .buf_fold(|v| {
            let mut ids: Vec<Id<Post>> = v.iter().map(|x| x.0 .0).collect();
            ids.sort();
            ids.dedup();
            let up = v.iter().filter(|x| x.1 .1 == Some(2)).count() as i64;
            let dn = v.iter().filter(|x| x.1 .1 == Some(3)).count() as i64;
            [ids.len() as i64, up, dn]
        });
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false));
    type U = ((Id<User>, i64), i64);
    let ridx: HashIdx<Id<User>, U> = (&rr).map(|x: U| x.0 .0).inv().select(&rr).collect();
    type G = (Id<User>, Option<(i64, Str)>);
    let v = drain((&g).and(Same::<G>::new().map(|k: G| k.0).select(&ridx).map(|x: U| x.1)));
    let v = top_n(v, |&(k, (a, r))| (Reverse(a[0]), r, k.0), 10);
    rows(v.into_iter().map(|((u, c), (a, r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c.map_or(0, |c| c.0)), V::S(c.map_or("No History", |c| c.1)), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.OwnerUserId,
//        RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS RankScore, ROW_NUMBER() OVER (ORDER BY P.CreationDate DESC) AS RowNum,
//        COALESCE(NULLIF(P.Body, ''), 'No Content') AS BodyContent
//     FROM Posts P WHERE P.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND P.PostTypeId = 1),
// UserVotes AS (SELECT V.PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN V.VoteTypeId = 1 THEN 1 END) AS AcceptedVotes FROM Votes V JOIN RankedPosts RP ON V.PostId = RP.PostId GROUP BY V.PostId),
// CloseReasons AS (SELECT PH.PostId, STRING_AGG(CRT.Name, ', ') AS CloseReasons FROM PostHistory PH JOIN CloseReasonTypes CRT ON PH.Comment = CAST(CRT.Id AS TEXT)
//     WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId)
// SELECT UP.Id AS UserId, UP.DisplayName, RP.PostId, RP.Title, RP.BodyContent, RP.CreationDate, RP.Score, COALESCE(UV.UpVotes, 0) AS TotalUpVotes, COALESCE(UV.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(CR.CloseReasons, 'No close reasons') AS CloseReasons,
//        CASE WHEN RP.RankScore = 1 THEN 'Top Question' WHEN RP.RankScore IS NULL THEN 'No Questions Available' ELSE 'Regular Question' END AS QuestionCategory
// FROM RankedPosts RP LEFT JOIN Users UP ON RP.OwnerUserId = UP.Id LEFT JOIN UserVotes UV ON RP.PostId = UV.PostId LEFT JOIN CloseReasons CR ON RP.PostId = CR.PostId
// WHERE RP.RowNum <= 10 ORDER BY RP.Score DESC, UP.Reputation DESC;
//
// The STRING_AGG order is left open by the SQL; the port joins in history id order.
fn q22350(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user_id, body, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(post_type_id.eq(1)).select(owner_user_id.opt()));
    let r = per_group(ranked(v.clone(), |&(p, u)| (u, Reverse(score.get(p).unwrap())), false), |x| x.1);
    let rv = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    type R = (Id<Post>, i64);
    let rs: HashIdx<Id<Post>, R> = (&rv).map(|x: R| x.0).inv().select(&rv).collect();
    let top = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let crt: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i: i64| -> Str { leak(i.to_string()) }).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(comment.select(&crt)).buf_fold(|v| leak(v.join(", ")));
    let v = drain((&tp).select(Ident::<Post>::new().and((&db.post.owner_user).opt()).and((&uv).opt()).and((&cr).opt()).and((&rs).map(|x: R| x.1))));
    rows(v.into_iter().map(|(_, ((((p, u), a), c), k))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = match u {
            Some(u) => ucols(db, u, &["uid", "name"]),
            None => vec![V::Null, V::Null],
        };
        f.extend(post_fields(db, p, &["id", "title"]));
        let b = body.get(p).unwrap();
        f.push(V::S(if b.is_empty() { "No Content" } else { b }));
        f.extend(post_fields(db, p, &["created", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("No close reasons"))]);
        f.push(V::S(if k == 1 { "Top Question" } else { "Regular Question" }));
        row(f)
    }))
}

fn crt_text(db: &'static So) -> HashIdx<Str, Str> {
    (&db.close_reason_type.origid).map(|i: i64| -> Str { leak(i.to_string()) }).inv().select(&db.close_reason_type.name).collect()
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// AggregatedData AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// TopLinks AS (SELECT pl.PostId, pl.RelatedPostId, lt.Name AS LinkType, COUNT(*) AS LinkCount FROM PostLinks pl JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id
//     GROUP BY pl.PostId, pl.RelatedPostId, lt.Name HAVING COUNT(*) > 1),
// CloseReasons AS (SELECT ph.PostId, COUNT(*) AS CloseReasonCount, STRING_AGG(DISTINCT crt.Name, ', ') AS Reasons FROM PostHistory ph JOIN CloseReasonTypes crt ON ph.Comment = crt.Id::text
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT p.PostId, p.Title, p.CreationDate, ad.TotalPosts, ad.PositivePosts, ad.NegativePosts, ad.TotalBountyAmount, tl.LinkCount, cr.Reasons AS CloseReasons
// FROM RankedPosts p JOIN AggregatedData ad ON p.PostId = ad.UserId LEFT JOIN TopLinks tl ON p.PostId = tl.PostId LEFT JOIN CloseReasons cr ON p.PostId = cr.PostId
// WHERE (p.Rank <= 5 OR cr.CloseReasonCount > 0) ORDER BY p.Score DESC, ad.TotalPosts DESC;
//
// `p.PostId = ad.UserId` joins a post id to a user id, so it goes through the raw ids, and the users' posts x votes product is driven for the matched users alone.
// Rank ties go to the smaller post id and the distinct reasons are joined in name order (the SQL leaves both open).
fn q21357(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let r = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false), |x| x.1);
    type R = (Id<Post>, i64);
    let rv = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect::<Vec<R>>());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let hi = Ident::<User>::new().with((&db.user.reputation).gt(100));
    let cand: MatSet<Id<User>> = (&rv).map(|x: R| x.0).select(origid).select(&uidx).select(hi).collect();
    let ad = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((s, b)) => [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let tp = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostLink { post, related_post_id, link_type, .. } = &db.post_link;
    let tl = db.post_link.group_by(post.and(related_post_id).and(link_type.select(&db.link_type.name))).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    type L = (((Id<Post>, i64), Str), i64);
    let tlv = rel(drain((&tl).filt(|n| n > 1)));
    let tli: HashIdx<Id<Post>, L> = (&tlv).map(|x: L| x.0 .0 .0).inv().select(&tlv).collect();
    let crt = crt_text(db);
    let PostHistory { post: hp, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.eq(10)).group_by(hp).select(comment.select(&crt)).buf_fold(|v| (v.len() as i64, agg_distinct(v.iter().copied().collect(), ", ").unwrap()));
    let k = || Same::<R>::new().map(|x: R| x.0);
    type J = (((R, (Id<User>, ([i64; 3], i64))), Option<L>), Option<(i64, Str)>);
    let v = drain(
        (&rv)
            .select(Same::<R>::new().and(k().select(origid).select(&uidx).select(Ident::<User>::new().and((&ad).and(&tp)))).and(k().select(&tli).opt()).and(k().select(&cr).opt()))
            .filt(|(((r, _), _), c): J| r.1 <= 5 || c.map_or(false, |c| c.0 > 0)),
    );
    rows(v.into_iter().map(|(_, ((((p, _), (_, (a, n))), l), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), oint(l.map(|l| l.1)), ostr(c.map(|c| c.1))]);
        row(f)
    }))
}

// WITH RecursiveCTE AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted Answer' ELSE 'Unanswered' END AS AnswerStatus FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// RecentActivity AS (SELECT u.Id AS UserId, MAX(v.CreationDate) AS LastVoteDate, COUNT(DISTINCT v.Id) AS TotalVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostHistoryWithVotes AS (SELECT ph.PostId, ph.UserId, ph.PostHistoryTypeId, COUNT(v.Id) AS VoteCount FROM PostHistory ph LEFT JOIN Votes v ON ph.PostId = v.PostId
//     GROUP BY ph.PostId, ph.UserId, ph.PostHistoryTypeId)
// SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.AnswerCount, r.AnswerStatus, COALESCE(a.Reputation, 0) AS OwnerReputation, ra.LastVoteDate, ra.TotalVotes,
//        COUNT(pv.VoteCount) FILTER (WHERE pv.PostHistoryTypeId IN (10, 11)) AS CloseVotes, SUM(CASE WHEN pv.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalClosureActions,
//        STRING_AGG(DISTINCT CASE WHEN pv.PostHistoryTypeId = 10 THEN 'Closed' ELSE NULL END, ', ') AS ClosureRemarks
// FROM RecursiveCTE r LEFT JOIN Users a ON r.OwnerUserId = a.Id LEFT JOIN RecentActivity ra ON r.OwnerUserId = ra.UserId LEFT JOIN PostHistoryWithVotes pv ON r.PostId = pv.PostId
// GROUP BY r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.AnswerCount, r.AnswerStatus, a.Reputation, ra.LastVoteDate, ra.TotalVotes
// ORDER BY r.Score DESC, r.CreationDate DESC LIMIT 100;
//
// Not recursive. Each question is one output row ordered by its own columns, so the 100 are picked first. VoteCount is a COUNT and never NULL,
// so COUNT(pv.VoteCount) counts the (user, type) history groups and the vote join does not change it.
fn q33457(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, accepted_answer_id, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score.and(creation_date))), |&(p, (s, d))| (Reverse(s), Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ra = (&owners).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.creation_date).opt()).fold((i64::MIN, 0i64), |(m, n), d| match d {
        Some(d) => (m.max(d), n + 1),
        None => (m, n),
    });
    let PostHistory { user_id, post_history_type_id, .. } = &db.post_history;
    let pv = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(user_id.opt().and(post_history_type_id))).buf_fold(|v| {
        let mut g: Vec<(Option<i64>, i64)> = v.iter().copied().collect();
        g.sort();
        g.dedup();
        (g.iter().filter(|x| x.1 == 10 || x.1 == 11).count() as i64, g.iter().filter(|x| x.1 == 10).count() as i64)
    });
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ra)).opt()).and((&pv).opt())));
    rows(v.into_iter().map(|(_, ((p, u), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Accepted Answer" } else { "Unanswered" }));
        match u {
            Some((u, (m, n))) => f.extend([user_col(db, u, "rep"), tmax(m), V::I(n)]),
            None => f.extend([V::I(0), V::Null, V::Null]),
        }
        let (cv, ca) = c.unwrap_or((0, 0));
        f.extend([V::I(cv), V::I(ca), if ca > 0 { V::S("Closed") } else { V::Null }]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Tags, p.ViewCount, p.Score, p.OwnerUserId, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RecentRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL GROUP BY p.Id, p.Title, p.CreationDate, p.Tags, p.ViewCount, p.Score, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, b.Class, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, b.Class),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, COUNT(DISTINCT ph.UserId) AS EditorCount, STRING_AGG(DISTINCT ph.Comment, '; ') AS HistoryComments
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 19, 20) GROUP BY ph.PostId, ph.PostHistoryTypeId, ph.CreationDate)
// SELECT up.DisplayName AS UserDisplayName, rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, CASE WHEN ub.BadgeCount > 0 THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus,
//        CASE WHEN phd.EditorCount IS NOT NULL THEN phd.EditorCount ELSE 0 END AS TotalEditors, COALESCE(phd.HistoryComments, 'No Edit History') AS EditHistory
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id LEFT JOIN UserBadges ub ON up.Id = ub.UserId LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId
// WHERE rp.PostRank = 1 AND rp.RecentRank <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC OFFSET 5 ROWS FETCH NEXT 5 ROWS ONLY;
//
// The ranks read only base columns, so the ten newest questions are picked first. A user with no badges is UserBadges' one (user, NULL, 0) row.
// The distinct comments are joined in sorted order (the SQL leaves it open).
fn q22205(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user_id, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user_id.opt()));
    let r = per_group(ranked(v.clone(), |&(p, u)| (u, Reverse(score.get(p).unwrap())), false), |x| x.1);
    type R = (Id<Post>, i64);
    let rv = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect::<Vec<R>>());
    let rs: HashIdx<Id<Post>, R> = (&rv).map(|x: R| x.0).inv().select(&rv).collect();
    let top = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let Badge { user, class, .. } = &db.badge;
    let ub = db.badge.group_by(user.and(class)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    type B = ((Id<User>, i64), i64);
    let ubv = rel(drain(&ub));
    let ubi: HashIdx<Id<User>, B> = (&ubv).map(|x: B| x.0 .0).inv().select(&ubv).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, user_id, comment, .. } = &db.post_history;
    let phd = db
        .post_history
        .with(post_history_type_id.is_in([10, 11, 12, 19, 20]))
        .group_by(post.and(post_history_type_id).and(hd))
        .select(user_id.opt().and(comment.opt()))
        .buf_fold(|v| (distinct_some(v.iter().map(|x| x.0)), agg_distinct(v.iter().filter_map(|x| x.1).collect(), "; ")));
    type H = (((Id<Post>, i64), i64), (i64, Option<Str>));
    let phv = rel(drain(&phd));
    let phi: HashIdx<Id<Post>, H> = (&phv).map(|x: H| x.0 .0 .0).inv().select(&phv).collect();
    let v = drain(
        (&tp)
            .with((&rs).map(|x: R| x.1).eq(1))
            .select(Ident::<Post>::new().and(&cc).and(owner_user.select(Ident::<User>::new().and((&ubi).map(|x: B| x.1).opt()))).and((&phi).map(|x: H| x.1).opt())),
    );
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().skip(5).map(|(_, (((p, c), (u, b)), h))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.push(V::I(c));
        f.push(V::S(if b.map_or(false, |b| b > 0) { "Has Badges" } else { "No Badges" }));
        f.push(V::I(h.map_or(0, |h| h.0)));
        f.push(V::S(h.and_then(|h| h.1).unwrap_or("No Edit History")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        COALESCE(p.Body, 'No content provided') AS SafeBody
//     FROM Posts p WHERE p.CreationDate >= COALESCE((SELECT MIN(CreationDate) FROM Posts WHERE PostTypeId = 1), '2000-01-01')),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(b.Class, 0)) AS TotalBadgePoints, COUNT(DISTINCT p.Id) AS QuestionsCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(DISTINCT crt.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes crt ON ph.Comment = CAST(crt.Id AS VARCHAR)
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT pu.DisplayName, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(cp.CloseReasons, 'No close reasons') AS FinalCloseReasons, t.TotalBadgePoints, t.QuestionsCount,
//        CASE WHEN rp.ViewCount IS NULL THEN 'No views registered' ELSE CASE WHEN rp.ViewCount > 1000 THEN 'Highly popular' WHEN rp.ViewCount BETWEEN 500 AND 1000 THEN 'Moderately popular'
//        ELSE 'Less popular' END END AS PopularityStatus
// FROM RankedPosts rp JOIN Users pu ON rp.OwnerUserId = pu.Id LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId INNER JOIN TopUsers t ON pu.Id = t.UserId
// WHERE rp.rn = 1 AND (rp.Score IS NULL OR rp.Score > 10) ORDER BY t.TotalBadgePoints DESC, rp.CreationDate DESC LIMIT 100 OFFSET 0;
//
// rn reads only base columns, so each owner's newest post is picked first (a tie goes to the smaller id; the SQL leaves it open), and the badges x questions
// product is driven for the owners that survive. The distinct reasons are joined in name order.
fn q22279(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user_id, owner_user, view_count, .. } = &db.post;
    let lo = db.post.with(post_type_id.eq(1)).select(creation_date).fold_flat(i64::MAX, |m, d| m.min(d));
    let lo = if lo == i64::MAX { ts(2000, 1, 1, 0, 0, 0) } else { lo };
    let first = top_per(drain(db.post.with(creation_date.ge(lo)).select(owner_user_id.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let hi = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let owners: MatSet<Id<User>> = (&tp).with(score.gt(10)).select(owner_user).select(hi).collect();
    let q = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let tu = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(q().opt())).fold(0i64, |n, (c, _)| n + c.unwrap_or(0));
    let qc = (&owners).group_by(Ident::<User>::new()).select(q().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let crt = crt_text(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(comment.select(&crt)).buf_fold(|v| agg_distinct(v.iter().copied().collect(), ", ").unwrap());
    let v = drain((&tp).with(score.gt(10)).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&tu).and(&qc))).and((&cp).opt())));
    let v = top_n(v, |&(p, ((_, ((_, t), _)), _))| (Reverse(t), Reverse(creation_date.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(_, ((p, ((u, t), n)), c))| {
        let w = view_count.get(p);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend([V::S(c.unwrap_or("No close reasons")), V::I(t), V::I(n)]);
        f.push(V::S(match w {
            None => "No views registered",
            Some(w) if w > 1000 => "Highly popular",
            Some(w) if w >= 500 => "Moderately popular",
            _ => "Less popular",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.UpVotes, rp.DownVotes, rp.ScoreRank FROM RankedPosts rp WHERE rp.ScoreRank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, '; ') AS CommentTexts FROM Comments c GROUP BY c.PostId),
// FinalResult AS (SELECT fp.*, pc.CommentCount, pc.CommentTexts, CASE WHEN fp.UpVotes > fp.DownVotes THEN 'Positive' WHEN fp.UpVotes = fp.DownVotes THEN 'Neutral' ELSE 'Negative' END AS PostSentiment,
//        (SELECT COUNT(DISTINCT bh.UserId) FROM Badges bh WHERE bh.UserId IN (SELECT DISTINCT OwnerUserId FROM Posts WHERE Id = fp.PostId) AND bh.Class = 1) AS GoldBadgeCount
//     FROM FilteredPosts fp LEFT JOIN PostComments pc ON fp.PostId = pc.PostId)
// SELECT p.PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.UpVotes, p.DownVotes, p.CommentCount, p.CommentTexts, p.PostSentiment, p.GoldBadgeCount
// FROM FinalResult p ORDER BY p.Score DESC, p.CreationDate DESC LIMIT 10;
//
// ScoreRank and the ORDER BY read only base columns, so the ten posts are picked first. The comment texts are joined in comment id order (the SQL leaves it open).
fn q22177(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let r = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |x| x.1);
    let fp: Vec<Id<Post>> = r.into_iter().filter(|x| x.1 <= 5).map(|x| x.0 .0).collect();
    let top = top_n(fp, |&p| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    let tp: MatSet<Id<Post>> = rel(top).map(|p| p).collect();
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.text)).buf_fold(|v| (v.len() as i64, leak(v.join("; "))));
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.user);
    let gb = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(gold).opt()).buf_fold(|v| distinct_some(v.iter().copied()));
    let v = drain((&tp).select(Ident::<Post>::new().and(&ud).and((&pc).opt()).and(&gb)));
    rows(v.into_iter().map(|(_, (((p, a), c), g))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), oint(c.map(|c| c.0)), ostr(c.map(|c| c.1))]);
        f.push(V::S(if a[0] > a[1] { "Positive" } else if a[0] == a[1] { "Neutral" } else { "Negative" }));
        f.push(V::I(g));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS QuestionCount, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1
//     GROUP BY u.Id HAVING SUM(p.Score) > 10),
// UserWithBadges AS (SELECT u.Id AS UserId, u.DisplayName, b.Name AS BadgeName, b.Class AS BadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE b.Class = 1),
// ClosedPosts AS (SELECT ph.PostId, ph.UserId, STRING_AGG(ctr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes ctr ON CAST(ph.Comment AS int) = ctr.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.UserId)
// SELECT p.Title, p.CreationDate, u.DisplayName AS Owner, COUNT(c.Id) AS CommentCount, COALESCE(b.BadgeName, 'No Badge') AS Badge,
//        CASE WHEN r.PostRank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory, ARRAY_AGG(DISTINCT cp.CloseReasons) AS ReasonsForClosure
// FROM RankedPosts r JOIN Posts p ON r.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN UserWithBadges b ON u.Id = b.UserId
// LEFT JOIN ClosedPosts cp ON p.Id = cp.PostId
// WHERE r.PostRank <= 10 GROUP BY p.Title, p.CreationDate, u.DisplayName, b.BadgeName, r.PostRank ORDER BY p.CreationDate DESC;
//
// TopUsers is never joined. The ARRAY_AGG and STRING_AGG orders are left open by the SQL; the port sorts the distinct list and joins in history id order.
fn q391(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user_id, owner_user, title, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(post_type_id.eq(1)).select(owner_user_id.opt()));
    let r = per_group(ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap())), false), |x| x.1);
    type R = (Id<Post>, i64);
    let rv = rel(r.into_iter().filter(|x| x.1 <= 10).map(|((p, _), k)| (p, k)).collect::<Vec<R>>());
    let cr = close_reasons(db);
    let PostHistory { post, user_id, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(user_id.opt()))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| leak(v.join(", ")));
    type C = ((Id<Post>, Option<i64>), Str);
    let cpv = rel(drain(&cp));
    let cpi: HashIdx<Id<Post>, C> = (&cpv).map(|x: C| x.0 .0).inv().select(&cpv).collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.name);
    type K = (Option<Str>, i64, Str, Option<Str>, i64);
    type X = (K, (bool, Option<Str>));
    let j = drain(
        (&rv)
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(
                title.opt().and(creation_date).and(owner_user.select(&db.user.display_name)).and(comments_of(db).opt()).and(owner_user.select(gold).opt()).and((&cpi).map(|x: C| x.1).opt()),
            )))
            .map(|((_, k), (((((t, d), n), c), b), cp)): (R, (((((Option<Str>, i64), Str), Option<Id<Comment>>), Option<Str>), Option<Str>))| ((t, d, n, b, k), (c.is_some(), cp))),
    );
    let g = rel(j.into_iter().map(|x| x.1).collect::<Vec<X>>()).group_by(Same::<X>::new().map(|x: X| x.0)).select(Same::<X>::new().map(|x: X| x.1)).buf_fold(|v| {
        let mut l: Vec<Option<Str>> = v.iter().map(|x| x.1).collect();
        l.sort();
        l.dedup();
        (v.iter().filter(|x| x.0).count() as i64, leak_list(l))
    });
    rows(drain(&g).into_iter().map(|((t, d, n, b, k), (c, l))| {
        row(vec![ostr(t), V::T(d), V::S(n), V::I(c), V::S(b.unwrap_or("No Badge")), V::S(if k <= 5 { "Top Post" } else { "Regular Post" }), V::L(l.iter().map(|&x| ostr(x)).collect())])
    }))
}

fn leak_list(v: Vec<Option<Str>>) -> &'static [Option<Str>] {
    Box::leak(v.into_boxed_slice())
}

fn avg_score(db: &'static So) -> f64 {
    let (s, n) = db.post.select(&db.post.score).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    s as f64 / n as f64
}

fn avg_views(db: &'static So) -> f64 {
    let (s, n) = db.post.select(&db.post.view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    s as f64 / n as f64
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) OVER (PARTITION BY p.Id) AS Upvotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) OVER (PARTITION BY p.Id) AS Downvotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2023-01-01' AND p.Score IS NOT NULL),
// PostEngagement AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Upvotes, rp.Downvotes, COALESCE(NULLIF(rp.Upvotes - rp.Downvotes, 0), NULL) AS EngagementScore
//     FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostHistoryDetails AS (SELECT ph.PostId, COUNT(DISTINCT ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate, STRING_AGG(DISTINCT pht.Name, ', ') AS HistoryTypes
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT pe.PostId, pe.Title, pe.CreationDate, pe.Score, pe.ViewCount, pe.Upvotes, pe.Downvotes, pe.EngagementScore, COALESCE(phd.EditCount, 0) AS TotalEdits,
//        COALESCE(phd.LastEditDate, NULL) AS LastEdit, COALESCE(phd.HistoryTypes, 'No History') AS ChangeHistory,
//        CASE WHEN pe.EngagementScore IS NULL THEN 'No Engagement' WHEN pe.EngagementScore > 0 THEN 'Positive Engagement' ELSE 'Negative Engagement' END AS EngagementStatus
// FROM PostEngagement pe LEFT JOIN PostHistoryDetails phd ON pe.PostId = phd.PostId
// WHERE pe.ViewCount > (SELECT AVG(ViewCount) FROM Posts) OR pe.Score > (SELECT AVG(Score) FROM Posts) ORDER BY pe.EngagementScore DESC, pe.CreationDate DESC FETCH FIRST 20 ROWS ONLY;
//
// Rank numbers the post x vote rows, so a post fills one rank per vote; ties inside a post go by vote id, across posts by post id (the SQL leaves them open).
// The distinct type names are joined in name order.
fn q22662(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let j = drain(db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).select(post_type_id.and(votes_of(db).opt())));
    let top = top_per(j, |&(_, (t, _))| t, |&(p, (_, v))| (Reverse(creation_date.get(p).unwrap()), p, v), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let phd = db
        .post_history
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(hd).and(htype_name(db)))
        .buf_fold(|v| (v.len() as i64, v.iter().map(|x| x.0 .1).max().unwrap(), agg_distinct(v.iter().map(|x| x.1).collect(), ", ").unwrap()));
    let (av, asc) = (avg_views(db), avg_score(db));
    type R = (Id<Post>, (i64, Option<Id<Vote>>));
    let k = || Same::<R>::new().map(|x: R| x.0);
    let tv = rel(top);
    let v = drain(
        (&tv)
            .select(k().and(k().select(view_count.opt())).and(k().select(score)))
            .filt(move |((_, w), s): ((Id<Post>, Option<i64>), i64)| w.map_or(false, |w| w as f64 > av) || s as f64 > asc)
            .map(|((p, _), _): ((Id<Post>, Option<i64>), i64)| p)
            .select(Ident::<Post>::new().and(&ud).and((&phd).opt())),
    );
    let v = top_n(v, |&(_, ((p, a), _))| {
        let e = if a[0] == a[1] { None } else { Some(a[0] - a[1]) };
        (e.is_none(), Reverse(e), Reverse(creation_date.get(p).unwrap()))
    }, 20);
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let e = a[0] - a[1];
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), if e == 0 { V::Null } else { V::I(e) }]);
        match h {
            Some((n, d, s)) => f.extend([V::I(n), V::T(d), V::S(s)]),
            None => f.extend([V::I(0), V::Null, V::S("No History")]),
        }
        f.push(V::S(if e == 0 { "No Engagement" } else if e > 0 { "Positive Engagement" } else { "Negative Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// FilteredPosts AS (SELECT r.*, CASE WHEN r.RecentPostRank <= 5 THEN 'Top Recent' ELSE 'Older Posts' END AS PostCategory, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName
//     FROM RankedPosts r LEFT JOIN Users u ON r.OwnerUserId = u.Id WHERE r.ScoreRank <= 10),
// SubqueryComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, '; ') AS Comments FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT f.Title, f.CreationDate, f.Score, f.ViewCount, f.PostCategory, COALESCE(sc.CommentCount, 0) AS TotalComments, COALESCE(sc.Comments, 'No comments') AS CommentTexts
//     FROM FilteredPosts f LEFT JOIN SubqueryComments sc ON f.PostId = sc.PostId WHERE f.PostCategory = 'Top Recent' AND f.Score > (SELECT AVG(Score) FROM Posts) ORDER BY f.Score DESC)
// SELECT f.Title, f.CreationDate, f.Score, f.ViewCount, f.PostCategory, f.TotalComments, f.CommentTexts,
//        CASE WHEN f.Score IS NULL THEN 'Unknown Score' WHEN f.Score > 100 THEN 'Highly Rated' ELSE 'Moderately Rated' END AS RatingCategory
// FROM FinalResults f WHERE f.TotalComments > 1 OR f.Score < 50 ORDER BY f.Score DESC, f.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 100 ROWS ONLY;
//
// RecentPostRank ties go to the smaller post id; the comment texts are joined in comment id order (the SQL leaves both open).
fn q24081(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let sr = per_group(ranked(v.clone(), |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |x| x.1);
    let rr = per_group(ranked(v, |&(p, t)| (t, Reverse(creation_date.get(p).unwrap()), p), false), |x| x.1);
    type R = ((Id<Post>, i64), i64);
    let srv = rel(sr);
    let rrv = rel(rr);
    let rri: HashIdx<Id<Post>, R> = (&rrv).map(|x: R| x.0 .0).inv().select(&rrv).collect();
    let asc = avg_score(db);
    let sc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.text)).buf_fold(|v| (v.len() as i64, leak(v.join("; "))));
    type J = (Id<Post>, Option<(i64, Str)>);
    let v = drain(
        (&srv)
            .filt(|x: R| x.1 <= 10)
            .map(|x: R| x.0 .0)
            .with((&rri).map(|x: R| x.1).le(5))
            .with(score.filt(move |s| s as f64 > asc))
            .select(Ident::<Post>::new().and((&sc).opt()))
            .filt(|(p, c): J| c.map_or(0, |c| c.0) > 1 || score.get(p).unwrap() < 50),
    );
    let v = top_n(v, |&(_, (p, _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(_, (p, c))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::S("Top Recent"), V::I(c.map_or(0, |c| c.0)), V::S(c.map_or("No comments", |c| c.1))]);
        f.push(V::S(if s > 100 { "Highly Rated" } else { "Moderately Rated" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVotes, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVotes
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.Score > 0 THEN 'Positive' WHEN rp.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreType FROM RankedPosts rp WHERE rp.rn = 1),
// CommentCounts AS (SELECT pc.PostId, COUNT(*) AS TotalComments FROM Comments pc GROUP BY pc.PostId),
// CombinedData AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.Score, pd.UpVotes, pd.DownVotes, pd.ScoreType, COALESCE(cc.TotalComments, 0) AS CommentCount
//     FROM PostDetails pd LEFT JOIN CommentCounts cc ON pd.PostId = cc.PostId)
// SELECT cd.*, CASE WHEN cd.ViewCount IS NULL THEN 'Views Not Available' ELSE CONCAT(cd.ViewCount, ' views') END AS ViewCountDetails,
//        CASE WHEN cd.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus,
//        (SELECT STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) FROM Tags t JOIN Posts p ON p.Id = cd.PostId WHERE p.Tags IS NOT NULL AND POSITION(t.TagName IN p.Tags) > 0) AS AssociatedTags
// FROM CombinedData cd ORDER BY cd.Score DESC NULLS LAST, cd.ViewCount DESC NULLS FIRST LIMIT 100 OFFSET 0;
//
// Ported from rewrites/23106.sql, which orders the STRING_AGG by TagName. rn ties go to the smaller post id (the SQL leaves it open).
fn q23106(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tm = tag_mentions(db);
    let tmv: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&tm).map(|(p, _)| p).inv().collect();
    let at = (&tp).group_by(Ident::<Post>::new()).select((&tmv).map(|(_, t)| t).select(&db.tag.tag_name)).buf_fold(|mut v| {
        v.sort();
        leak(v.join(", "))
    });
    let v = drain((&tp).select(Ident::<Post>::new().and(&ud).and(&cc).and((&at).opt())));
    rows(v.into_iter().map(|(_, (((p, a), c), t))| {
        let s = db.post.score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }), V::I(c)]);
        f.push(match view_count.get(p) {
            Some(w) => V::Owned(format!("{w} views")),
            None => V::S("Views Not Available"),
        });
        f.extend([V::S(if c > 0 { "Has Comments" } else { "No Comments" }), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, AVG(v.BountyAmount) AS AvgBountyAmount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// CloseReasons AS (SELECT ph.PostId, ARRAY_AGG(cr.Name) AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment::int = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT p.Id) AS TotalPosts
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, COALESCE(cr.CloseReasons, ARRAY[NULL]::varchar[]) AS CloseReasons, us.UserId,
//        us.DisplayName AS UserDisplayName, us.GoldBadges, us.SilverBadges, us.BronzeBadges, us.TotalPosts, rp.AvgBountyAmount
// FROM RankedPosts rp LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId JOIN UserStats us ON rp.OwnerUserId = us.UserId
// WHERE rp.PostRank = 1 AND (rp.Score > 10 OR rp.CommentCount > 5) ORDER BY rp.CreationDate DESC LIMIT 100;
//
// PostRank reads only base columns, so each owner's newest post is picked first (a tie goes to the smaller id; the SQL leaves it open). The ORDER BY is by the post's
// own date, so the badges x posts product is driven only for the owners of the 100 rows kept. The ARRAY_AGG is in history id order (the SQL leaves it open).
fn q30175(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(owner_user_id.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let fs: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tp: MatSet<Id<Post>> = (&fs).with(owner_user).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let rp = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 3], |a, (c, b)| {
        let b = b.flatten();
        [a[0] + c.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + b.is_some() as i64]
    });
    type J = (Id<Post>, [i64; 3]);
    let cand = drain((&tp).select(Ident::<Post>::new().and(&rp)).filt(|(p, a): J| score.get(p).unwrap() > 10 || a[0] > 5));
    let top = top_n(cand, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 100);
    let tv = rel(top);
    type T = (Id<Post>, (Id<Post>, [i64; 3]));
    let owners: MatSet<Id<User>> = (&tv).map(|x: T| x.0).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold([0i64; 3], |a, (c, _)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let tpc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cl = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| leak_list(v.iter().map(|&x| Some(x)).collect()));
    let v = drain((&tv).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select((&cl).opt().and(owner_user.select(Ident::<User>::new().and(&us).and(&tpc)))))));
    rows(v.into_iter().map(|(_, ((p, (_, a)), (l, ((u, b), n))))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(a[0]));
        f.push(V::L(l.map_or(vec![V::Null], |l| l.iter().map(|&x| ostr(x)).collect())));
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(n), avg(a[1], a[2])]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId),
// PostVoteDetails AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, u.DisplayName AS OwnerDisplayName, rp.UpVotes, rp.DownVotes, ROW_NUMBER() OVER (ORDER BY rp.Score DESC) AS Rank
//     FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE u.Reputation > 1000),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(pt.Name, ', ') AS CloseReasons, COUNT(*) AS CloseCount FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id
//     WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND ph.Comment IS NOT NULL GROUP BY ph.PostId),
// FinalDetails AS (SELECT pvd.PostId, pvd.Title, pvd.Score, pvd.ViewCount, pvd.CreationDate, pvd.OwnerDisplayName, pvd.UpVotes, pvd.DownVotes, COALESCE(cp.CloseCount, 0) AS TotalCloseCount,
//        COALESCE(cp.CloseReasons, 'No Close Reasons') AS CloseReasons FROM PostVoteDetails pvd LEFT JOIN ClosedPosts cp ON pvd.PostId = cp.PostId)
// SELECT *, (UpVotes - DownVotes) AS NetVotes, (Score + (ViewCount / 100.0)) AS WeightedScore FROM FinalDetails WHERE (UpVotes - DownVotes) >= 10
// ORDER BY WeightedScore DESC, CreationDate DESC LIMIT 50;
//
// Rank is never read. The STRING_AGG is joined in history id order (the SQL leaves it open).
fn q4431(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, .. } = &db.post;
    let t0 = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let hi = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let rp: MatSet<Id<Post>> = db.post.with(creation_date.ge(t0)).with(owner_user.select(hi)).collect();
    let ud = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post, creation_date: hd, comment, .. } = &db.post_history;
    let cp = db.post_history.with(hd.ge(t0)).with(comment).group_by(post).select(htype_name(db)).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let v = drain((&ud).filt(|a| a[0] - a[1] >= 10).and((&cp).opt()));
    let ws = |p: Id<Post>| view_count.get(p).map(|w| fkey(score.get(p).unwrap() as f64 + w as f64 / 100.0));
    let v = top_n(v, |&(p, _)| (ws(p).is_none(), Reverse(ws(p)), Reverse(creation_date.get(p).unwrap())), 50);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.map_or(0, |c| c.0)), V::S(c.map_or("No Close Reasons", |c| c.1)), V::I(a[0] - a[1])]);
        f.push(match view_count.get(p) {
            Some(w) => V::F(score.get(p).unwrap() as f64 + w as f64 / 100.0),
            None => V::Null,
        });
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostComments AS (SELECT c.PostId, COUNT(*) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// PostHistoryDetails AS (SELECT ph.PostId, STRING_AGG(DISTINCT pht.Name, ', ') AS HistoryTypes, COUNT(*) AS TotalChanges FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
//     GROUP BY ph.PostId)
// SELECT rp.Title, rp.CreationDate, ur.DisplayName, ur.Reputation, ur.GoldBadges, ur.SilverBadges, ur.BronzeBadges, COALESCE(pc.CommentCount, 0) AS CommentCount, pc.LastCommentDate,
//        COALESCE(phd.TotalChanges, 0) AS TotalChanges, CASE WHEN rp.Score > 10 THEN 'Hot' WHEN rp.Score BETWEEN 1 AND 10 THEN 'Warm' ELSE 'Cool' END AS PostTemperature
// FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId
// LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId
// WHERE ur.Reputation > 1000 OR (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 2) > 5 ORDER BY rp.CreationDate DESC, ur.Reputation DESC LIMIT 100;
//
// rn and HistoryTypes are never read.
fn q21391(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let rp: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).with(owner_user).collect();
    let up = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64);
    let rep = &db.user.reputation;
    let v = drain((&rp).select(Ident::<Post>::new().and(owner_user.select(rep)).and(&up)).filt(|((_, r), n): ((Id<Post>, i64), i64)| r > 1000 || n > 5));
    let v = top_n(v, |&(p, ((_, r), _))| (Reverse(creation_date.get(p).unwrap()), Reverse(r), p), 100);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ur = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let phd = (&tp).group_by(Ident::<Post>::new()).select(history_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ur))).and((&pc).opt()).and((&phd).opt())));
    rows(v.into_iter().map(|(_, (((p, (u, b)), c), h))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(c.map_or(0, |c| c.0)), ots(c.map(|c| c.1)), V::I(h.unwrap_or(0))]);
        f.push(V::S(if s > 10 { "Hot" } else if s >= 1 { "Warm" } else { "Cool" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.PostTypeId, p.CreationDate, p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score IS NOT NULL),
// CloseReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasonNames FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// PostsWithVotes AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, COALESCE(cr.CloseReasonNames, 'Not Closed') AS CloseReasonNames, COALESCE(pwv.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(pwv.DownVotes, 0) AS TotalDownVotes, ub.BadgeCount, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount,
//        CASE WHEN ub.BadgeCount < 5 THEN 'Newbie' WHEN ub.BadgeCount < 10 THEN 'Intermediate' ELSE 'Veteran' END AS UserLevel
// FROM RankedPosts rp LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId LEFT JOIN PostsWithVotes pwv ON rp.PostId = pwv.PostId LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId
// WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC FETCH FIRST 25 ROWS ONLY;
//
// rn reads only base columns, so the newest owned post per type is picked first (a tie goes to the smaller id; the SQL leaves it open).
// The STRING_AGG is joined in history id order (the SQL leaves it open).
fn q23256(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let crn = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| leak(v.join(", ")));
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ub = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tp).select(Ident::<Post>::new().and((&crn).opt()).and(&ud).and(owner_user.select(&ub)).and(&cc)));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 25);
    rows(v.into_iter().map(|(_, ((((p, r), a), b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::S(r.unwrap_or("Not Closed")), V::I(a[0]), V::I(a[1]), V::I(b), V::I(c)]);
        f.push(V::S(if b < 5 { "Newbie" } else if b < 10 { "Intermediate" } else { "Veteran" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankByScore,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) OVER (PARTITION BY p.Id) AS UpVoteCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) OVER (PARTITION BY p.Id) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp
//     WHERE rp.RankByScore <= 10 AND rp.UpVoteCount - rp.DownVoteCount > 0),
// PostComments AS (SELECT fp.PostId, STRING_AGG(c.Text, ' | ' ORDER BY c.CreationDate) AS AllComments FROM FilteredPosts fp LEFT JOIN Comments c ON fp.PostId = c.PostId GROUP BY fp.PostId)
// SELECT fp.Title, fp.CreationDate, LEAST(fp.ViewCount, 100) AS LimitedViewCount, COALESCE(pc.AllComments, 'No comments available') AS Comments,
//        CASE WHEN fp.Score IS NULL THEN 'Unscored' WHEN fp.Score > 50 THEN 'High Score' ELSE 'Low Score' END AS ScoreCategory,
//        CASE WHEN fp.CommentCount = 0 THEN 'No Comments' WHEN fp.CommentCount >= 5 THEN 'Many Comments' ELSE 'Few Comments' END AS CommentCategory,
//        (SELECT COUNT(*) FROM Posts ph WHERE ph.ParentId = fp.PostId) AS AnswerCount
// FROM FilteredPosts fp LEFT JOIN PostComments pc ON fp.PostId = pc.PostId ORDER BY fp.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 25 ROWS ONLY;
//
// RankByScore numbers the post x comment x vote rows, so a post fills one rank per joined row; the window counts are over those rows too.
// Ties between posts go to the smaller post id (the SQL leaves them open). PostComments groups the FilteredPosts rows, so each post's comments repeat once per kept row.
fn q24637(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let j = drain(recent().select(post_type_id.and(comments_of(db).opt().and(votes_of(db).opt()))));
    let top = top_per(j, |&(_, (t, _))| t, |&(p, (_, cv))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, cv)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let wc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    type R = (Id<Post>, (i64, (Option<Id<Comment>>, Option<Id<Vote>>)));
    let tv = rel(top);
    let fp = drain((&tv).map(|x: R| x.0).select(Ident::<Post>::new().and(&wc)).filt(|(_, a): (Id<Post>, [i64; 3])| a[1] - a[2] > 0));
    type F = (Id<Post>, [i64; 3]);
    let fpv = rel(fp.into_iter().map(|x| x.1).collect::<Vec<F>>());
    let Comment { text, creation_date: cd, .. } = &db.comment;
    let pc = (&fpv)
        .group_by(Same::<F>::new().map(|x: F| x.0))
        .select(Same::<F>::new().map(|x: F| x.0).select(comments_of(db).select(cd.and(text).and(Ident::<Comment>::new()))))
        .buf_fold(|mut v| {
            v.sort_by_key(|x| (x.0, x.1));
            leak(v.iter().map(|x| x.0 .1).collect::<Vec<_>>().join(" | "))
        });
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&fpv).select(Same::<F>::new().and(Same::<F>::new().map(|x: F| x.0).select((&pc).opt().and(&ac)))));
    let v = top_n(v, |&(_, ((p, _), _))| Reverse(creation_date.get(p).unwrap()), 25);
    rows(v.into_iter().map(|(_, ((p, a), (c, n)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created"]);
        f.push(V::I(view_count.get(p).map_or(100, |w| w.min(100))));
        f.push(V::S(c.unwrap_or("No comments available")));
        f.push(V::S(if s > 50 { "High Score" } else { "Low Score" }));
        f.push(V::S(if a[0] == 0 { "No Comments" } else if a[0] >= 5 { "Many Comments" } else { "Few Comments" }));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND p.ViewCount IS NOT NULL),
// PostScore AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.PostTypeId, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes FROM RankedPosts rp LEFT JOIN Votes v ON v.PostId = rp.PostId GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.PostTypeId),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS VersionCount, STRING_AGG(CONCAT(CAST(ph.CreationDate AS DATE), ' ', ph.UserDisplayName, ': ', ph.Comment), '; ') AS EditComments
//     FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId HAVING COUNT(*) > 1),
// FinalResults AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.TotalBounty, ps.TotalUpvotes, ps.TotalDownvotes, COALESCE(pht.VersionCount, 0) AS EditVersionCount,
//        COALESCE(pht.EditComments, 'No edits') AS EditComments FROM PostScore ps LEFT JOIN PostHistoryDetails pht ON ps.PostId = pht.PostId WHERE ps.PostTypeId IN (1, 2))
// SELECT fr.*, CASE WHEN fr.EditVersionCount > 0 THEN 'Edited' ELSE 'Not Edited' END AS EditStatus, (fr.TotalUpvotes - fr.TotalDownvotes) AS NetVotes,
//        CASE WHEN fr.TotalBounty > 0 THEN 'Has Bounty' ELSE 'No Bounty' END AS BountyStatus
// FROM FinalResults fr ORDER BY fr.TotalUpvotes DESC, fr.CreationDate DESC LIMIT 10;
//
// rn is never read. The STRING_AGG is joined in history id order (the SQL leaves it open).
fn q24394(db: &'static So) -> String {
    let Post { creation_date, view_count, post_type_id, .. } = &db.post;
    let rp: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(view_count).with(post_type_id.is_in([1, 2])).collect();
    let ps = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let PostHistory { post, post_history_type_id, creation_date: hd, user_display_name, comment, .. } = &db.post_history;
    let phd = (&rp)
        .select(history_of(db))
        .group_by(post.and(post_history_type_id))
        .select(hd.and(user_display_name.opt()).and(comment.opt()))
        .buf_fold(|v| {
            let s: Vec<String> = v.iter().map(|&((d, n), c)| format!("{} {}: {}", fmt_date(d), n.unwrap_or(""), c.unwrap_or(""))).collect();
            (v.len() as i64, leak(s.join("; ")))
        });
    type H = ((Id<Post>, i64), (i64, Str));
    let phv = rel(drain((&phd).filt(|x: (i64, Str)| x.0 > 1)));
    let phi: HashIdx<Id<Post>, H> = (&phv).map(|x: H| x.0 .0).inv().select(&phv).collect();
    let v = drain((&ps).and((&phi).map(|x: H| x.1).opt()));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[1]), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(h.map_or(0, |h| h.0)), V::S(h.map_or("No edits", |h| h.1))]);
        f.extend([V::S(if h.is_some() { "Edited" } else { "Not Edited" }), V::I(a[1] - a[2]), V::S(if a[0] > 0 { "Has Bounty" } else { "No Bounty" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(V.BountyAmount) AS TotalBounties, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopContributors AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, TotalBounties, ReputationRank FROM UserStatistics WHERE PostCount > 20 AND ReputationRank <= 10),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COALESCE(MAX(CASE WHEN PH.PostHistoryTypeId = 10 THEN PH.CreationDate END), '1970-01-01') AS ClosestCloseDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount)
// SELECT T.DisplayName, T.Reputation, T.PostCount, T.AnswerCount, T.QuestionCount, T.TotalBounties, COUNT(DISTINCT PD.PostId) AS TotalPosts, SUM(PD.Score) AS TotalScore,
//        AVG(PD.ViewCount) AS AverageViewCount, STRING_AGG(DISTINCT PD.Title, ', ') AS PostTitles, COUNT(CASE WHEN PD.ClosestCloseDate IS NOT NULL THEN 1 END) AS ClosedPosts
// FROM TopContributors T JOIN PostDetails PD ON T.UserId = PD.PostId GROUP BY T.UserId, T.DisplayName, T.Reputation, T.PostCount, T.AnswerCount, T.QuestionCount, T.TotalBounties
// HAVING AVG(PD.ViewCount) > 100 ORDER BY T.Reputation DESC;
//
// ReputationRank reads only Reputation, so the ten users are picked first and the posts x votes product is driven for them alone. `T.UserId = PD.PostId`
// joins a user id to a post id, so it goes through the raw ids. ClosestCloseDate is COALESCEd, so never NULL, and PostDetails' comment and history
// aggregates are not otherwise read. The distinct titles are joined in sorted order (the SQL leaves it open).
fn q1930(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, score, view_count, title, .. } = &db.post;
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt()).fold([0i64; 4], |a, x| match x {
        Some((t, b)) => {
            let b = b.flatten();
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + b.unwrap_or(0), a[3] + b.is_some() as i64]
        }
        None => a,
    });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let pd = (&tu)
        .group_by(Ident::<User>::new())
        .select((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(score).and(view_count.opt()).and(title.opt())))
        .buf_fold(|v| {
            let mut ids: Vec<Id<Post>> = v.iter().map(|x| x.0 .0 .0).collect();
            ids.sort();
            ids.dedup();
            let s: i64 = v.iter().map(|x| x.0 .0 .1).sum();
            let w: Vec<i64> = v.iter().filter_map(|x| x.0 .1).collect();
            let wa = if w.is_empty() { None } else { Some(w.iter().sum::<i64>() as f64 / w.len() as f64) };
            (ids.len() as i64, s, wa, agg_distinct(v.iter().filter_map(|x| x.1).collect::<Vec<_>>(), ", "), v.len() as i64)
        });
    let v = drain((&us).and(&pc).filt(|(_, n): ([i64; 4], i64)| n > 20).and((&pd).filt(|x: (i64, i64, Option<f64>, Option<Str>, i64)| x.2.map_or(false, |w| w > 100.0))));
    rows(v.into_iter().map(|(u, ((a, n), (tp, s, w, t, c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[2], a[3]), V::I(tp), V::I(s), V::F(w.unwrap()), ostr(t), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByUser, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 DAY' AND p.Score > 0 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// OpenPosts AS (SELECT rp.PostId, rp.Title, rp.RankByUser, rp.CommentCount, rp.Score,
//        CASE WHEN rp.Score IS NULL THEN 'Score not available' WHEN rp.Score < 0 THEN 'Negative score' ELSE 'Score available' END AS Score_Status FROM RankedPosts rp WHERE rp.RankByUser = 1),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserId AS EditorId, u.DisplayName AS EditorName, ph.CreationDate AS EditDate,
//        CASE WHEN ph.Comment IS NOT NULL THEN ph.Comment ELSE 'No comment provided' END AS EditComment FROM PostHistory ph JOIN Users u ON ph.UserId = u.Id WHERE ph.PostHistoryTypeId IN (4, 5, 6))
// SELECT op.PostId, op.Title, op.CommentCount, op.Score_Status, COUNT(DISTINCT phd.EditorId) AS TotalEditors, MAX(phd.EditDate) AS LastEditDate, STRING_AGG(phd.EditComment, '; ') AS EditorComments
// FROM OpenPosts op LEFT JOIN PostHistoryDetails phd ON op.PostId = phd.PostId GROUP BY op.PostId, op.Title, op.CommentCount, op.Score_Status
// HAVING COUNT(DISTINCT phd.EditorId) > 0 ORDER BY op.CommentCount DESC, op.Score_Status DESC;
//
// RankByUser reads only base columns, so each owner's newest post is picked first (a tie goes to the smaller id; the SQL leaves it open).
// The STRING_AGG is joined in history id order (the SQL leaves it open).
fn q21281(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(current_date(), -30))).with(score.gt(0)).select(owner_user_id.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v23 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(v23.opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let PostHistory { post_history_type_id, user, creation_date: hd, comment, .. } = &db.post_history;
    let ed = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6])).with(user));
    let ph = (&tp).group_by(Ident::<Post>::new()).select(ed.select(user.and(hd).and(comment.opt()))).buf_fold(|v| {
        let n = distinct_some(v.iter().map(|x| Some(x.0 .0)));
        let m = v.iter().map(|x| x.0 .1).max().unwrap();
        let s: Vec<&str> = v.iter().map(|x| x.1.unwrap_or("No comment provided")).collect();
        (n, m, leak(s.join("; ")))
    });
    let v = drain((&tp).select(Ident::<Post>::new().and(&cc).and((&ph).filt(|x: (i64, i64, Str)| x.0 > 0))));
    rows(v.into_iter().map(|(_, ((p, c), (n, m, s)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c), V::S("Score available"), V::I(n), V::T(m), V::S(s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COALESCE(AVG(u.Reputation), 0) AS AverageReputation
//     FROM Votes v LEFT JOIN Users u ON v.UserId = u.Id GROUP BY v.PostId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, ' | ') AS CommentTexts FROM Comments c GROUP BY c.PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.Score, COALESCE(u.UpVotes, 0) AS UpVotes, COALESCE(u.DownVotes, 0) AS DownVotes, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        COALESCE(pc.CommentTexts, '[No Comments]') AS CommentTexts FROM RankedPosts rp LEFT JOIN UserVotes u ON rp.PostId = u.PostId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE rp.Rank <= 5),
// FinalSelection AS (SELECT pd.*, CASE WHEN pd.Score IS NULL THEN 'No Score Available' WHEN pd.UpVotes = 0 THEN 'No Upvotes' ELSE 'Interactive' END AS InteractionStatus FROM PostDetails pd)
// SELECT fs.PostId, fs.Title, fs.ViewCount, fs.CreationDate, fs.Score, fs.UpVotes, fs.DownVotes, fs.CommentCount, fs.CommentTexts, fs.InteractionStatus
// FROM FinalSelection fs WHERE fs.CommentCount > 0 OR fs.UpVotes > 5 ORDER BY fs.Score DESC NULLS LAST, fs.ViewCount DESC;
//
// Rank reads only base columns, so the posts are ranked first (ties go to the smaller id). The comment texts are joined in comment id order (the SQL leaves both open).
fn q23306(db: &'static So) -> String {
    let Post { creation_date, score, view_count, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.text)).buf_fold(|v| (v.len() as i64, leak(v.join(" | "))));
    type J = (Id<Post>, (Option<[i64; 2]>, Option<(i64, Str)>));
    let v = drain((&tp).select(Ident::<Post>::new().and((&uv).opt().and((&pc).opt()))).filt(|(_, (a, c)): J| c.map_or(0, |c| c.0) > 0 || a.map_or(0, |a| a[0]) > 5));
    rows(v.into_iter().map(|(_, (p, (a, c)))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.map_or(0, |c| c.0)), V::S(c.map_or("[No Comments]", |c| c.1))]);
        f.push(V::S(if a[0] == 0 { "No Upvotes" } else { "Interactive" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0),
// MostActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName
//     HAVING COUNT(DISTINCT p.Id) > 10 AND SUM(COALESCE(v.BountyAmount, 0)) > 0),
// Mortuary AS (SELECT ph.PostId, ph.CreationDate, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) AND ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 year'),
// FinalReport AS (SELECT mu.UserId, mu.DisplayName, rp.PostId, rp.Title, rp.CreationDate AS PostCreationDate, rp.ViewCount, rp.Score, COALESCE(m.CloseReason, 'N/A') AS CloseReason, mu.TotalBounty,
//        CASE WHEN mu.PostCount >= 20 THEN 'Frequent Contributor' ELSE 'Occasional Contributor' END AS ContributorType
//     FROM MostActiveUsers mu JOIN RankedPosts rp ON mu.UserId = rp.OwnerUserId LEFT JOIN Mortuary m ON rp.PostId = m.PostId)
// SELECT UserId, DisplayName, COUNT(PostId) AS ActivePostCount, SUM(ViewCount) AS TotalViews, AVG(Score) AS AverageScore, STRING_AGG(DISTINCT CloseReason, ', ' ORDER BY CloseReason) AS CloseReasons, ContributorType
// FROM FinalReport GROUP BY UserId, DisplayName, ContributorType ORDER BY TotalViews DESC, AverageScore DESC LIMIT 50;
//
// Ported from rewrites/24432.sql, which orders the STRING_AGG. PostRank is never read.
fn q24432(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let b8 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let tb = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(b8.opt()).opt()).fold(0i64, |n, x| n + x.flatten().flatten().unwrap_or(0));
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let mu: MatSet<Id<User>> = db.user.with((&tb).filt(|b| b > 0)).with((&pc).filt(|n| n > 10)).collect();
    let PostHistory { post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let m = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11])).with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -2)))).select(comment.opt());
    let rp = || Ident::<Post>::new().with(creation_date.lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(score.gt(0));
    let g = (&mu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(rp()).select(view_count.opt().and(score).and(m.opt())))
        .buf_fold(|v| {
            let w: Vec<i64> = v.iter().filter_map(|x| x.0 .0).collect();
            let s: i64 = v.iter().map(|x| x.0 .1).sum();
            let cs: Vec<&'static str> = v.iter().map(|x| x.1.flatten().unwrap_or("N/A")).collect();
            (v.len() as i64, if w.is_empty() { None } else { Some(w.iter().sum::<i64>()) }, s, agg_distinct(cs, ", ").unwrap())
        });
    let v = drain((&g).and(&pc));
    let v = top_n(v, |&(u, ((n, w, s, _), _))| (w.is_none(), Reverse(w), Reverse(fkey(s as f64 / n as f64)), u), 50);
    rows(v.into_iter().map(|(u, ((n, w, s, c), pn))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), oint(w), avg(s, n), V::S(c), V::S(if pn >= 20 { "Frequent Contributor" } else { "Occasional Contributor" })]);
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 END) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// ClosedPosts AS (SELECT P.Id AS PostId, P.Title, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseVoteCount, MAX(PH.CreationDate) AS LastCloseDate,
//        STRING_AGG(CASE WHEN PH.UserId IS NOT NULL THEN PH.UserDisplayName END, ', ') AS Closers FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId WHERE PH.PostHistoryTypeId = 10 GROUP BY P.Id, P.Title),
// TopClosedPosts AS (SELECT CP.PostId, CP.Title, CP.CloseVoteCount, CP.LastCloseDate, CP.Closers, ROW_NUMBER() OVER (ORDER BY CP.CloseVoteCount DESC, CP.LastCloseDate ASC) AS Rank
//     FROM ClosedPosts CP WHERE CP.CloseVoteCount > 0),
// PostStats AS (SELECT P.Id AS PostId, P.Title, (P.ViewCount + COALESCE((SELECT SUM(V.BountyAmount) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId IN (8, 9)), 0)) AS EngagementScore,
//        U.DisplayName AS OwnerDisplayName, P.CreationDate FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND P.Score IS NOT NULL)
// SELECT PS.PostId, PS.Title, PS.EngagementScore, PS.OwnerDisplayName, TCP.CloseVoteCount, TCP.LastCloseDate, TCP.Closers, UVC.VoteCount AS UserVoteCount, UVC.UpVoteCount, UVC.DownVoteCount
// FROM PostStats PS LEFT JOIN TopClosedPosts TCP ON PS.PostId = TCP.PostId LEFT JOIN UserVoteCounts UVC ON PS.OwnerDisplayName = UVC.DisplayName
// WHERE PS.EngagementScore >= (SELECT AVG(EngagementScore) FROM PostStats) AND TCP.Rank <= 10 ORDER BY PS.EngagementScore DESC, PS.CreationDate ASC;
//
// TCP.Rank <= 10 makes the LEFT JOIN inner, and Rank reads only ClosedPosts' own aggregates, so the ten closed posts are picked first (ties go to the smaller id;
// the SQL leaves them open). The Closers STRING_AGG is joined in history id order. The name join can match several users, one row each.
fn q20057(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, user_id, user_display_name, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd.and(user_id.opt()).and(user_display_name.opt())).buf_fold(|v| {
        let c: Vec<&str> = v.iter().filter_map(|x| if x.0 .1.is_some() { x.1 } else { None }).collect();
        (v.len() as i64, v.iter().map(|x| x.0 .0).max().unwrap(), if c.is_empty() { None } else { Some(leak(c.join(", "))) })
    });
    let tcp = top_n(drain(&cp), |&(p, (n, d, _))| (Reverse(n), d, p), 10);
    type C = (Id<Post>, (i64, i64, Option<Str>));
    let tv = rel(tcp);
    let tci: HashIdx<Id<Post>, C> = (&tv).map(|x: C| x.0).inv().select(&tv).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select(&db.vote.bounty_amount);
    let ps_base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let bs = ps_base().group_by(Ident::<Post>::new()).select(bounty).fold(0i64, |n, b| n + b);
    let es = ps_base().select(view_count.and((&bs).opt())).map(|(w, b): (i64, Option<i64>)| w + b.unwrap_or(0));
    let (s, n) = es.fold_flat((0i64, 0i64), |(s, n), e| (s + e, n + 1));
    let ae = s as f64 / n as f64;
    let names: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let uvc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2) || t == Some(3)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain(
        ps_base()
            .with(view_count.and((&bs).opt()).map(|(w, b): (i64, Option<i64>)| w + b.unwrap_or(0)).filt(move |e| e as f64 >= ae))
            .select(Ident::<Post>::new().and(view_count.and((&bs).opt())).and(&tci).and(owner_user.select(&db.user.display_name).select((&names).select(&uvc).opt()))),
    );
    rows(v.into_iter().map(|(_, (((p, (w, b)), (_, (n, d, c))), u))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(w + b.unwrap_or(0))]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(n), V::T(d), ostr(c)]);
        match u {
            Some(a) => f.extend(a.map(V::I)),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("21364", q21364),
    ("23903", q23903),
    ("24602", q24602),
    ("2762", q2762),
    ("24520", q24520),
    ("24245", q24245),
    ("22350", q22350),
    ("21357", q21357),
    ("33457", q33457),
    ("22205", q22205),
    ("22279", q22279),
    ("22177", q22177),
    ("391", q391),
    ("22662", q22662),
    ("24081", q24081),
    ("23106", q23106),
    ("30175", q30175),
    ("4431", q4431),
    ("21391", q21391),
    ("23256", q23256),
    ("24637", q24637),
    ("24394", q24394),
    ("1930", q1930),
    ("21281", q21281),
    ("23306", q23306),
    ("24432", q24432),
    ("20057", q20057),
];
