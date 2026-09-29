use harness::prelude::*;

fn leak_join(parts: impl IntoIterator<Item = Str>, sep: &str) -> Str {
    Box::leak(parts.into_iter().collect::<Vec<_>>().join(sep).into_boxed_str())
}

fn now() -> i64 {
    ts(2024, 10, 1, 12, 34, 56)
}

fn year_ago() -> i64 {
    add_years(now(), -1)
}

// ORDER BY x DESC in DuckDB is NULLS LAST.
fn desc_nulls_last(a: &Option<i64>, b: &Option<i64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => y.cmp(x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

// The latest PostHistory row of the given types for each post, or nothing.
fn last_history(db: &'static So, types: [i64; 2]) -> DenseFold<Id<Post>, i64> {
    db.post_history
        .with((&db.post_history.post_history_type_id).is_in(types))
        .select(&db.post_history.post)
        .inv()
        .select(&db.post_history.creation_date)
        .dense_fold_outer(db.post.id.n, i64::MIN, |a, d| a.max(d))
}

// WITH RankedPosts AS (
//   SELECT p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, p.OwnerUserId,
//          ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) rk,
//          ARRAY_AGG(DISTINCT t.TagName) tags
//   FROM Posts p JOIN UNNEST(string_to_array(trim(both '{}' FROM p.Tags),'><')) t(TagName) ON true
//   WHERE p.CreationDate >= '2024-10-01 12:34:56' - INTERVAL '1 year'
//   GROUP BY p.Id, ...),
// TopPosts AS (
//   SELECT rp.Id, ..., COUNT(c.Id), MAX(b.Date), COALESCE(SUM(CASE WHEN v.VoteTypeId=2 THEN 1 ELSE 0 END),0)
//   FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId=rp.Id
//                       LEFT JOIN Badges b ON b.UserId=rp.OwnerUserId
//                       LEFT JOIN Votes v ON v.PostId=rp.Id
//   WHERE rp.rk <= 5 GROUP BY rp.Id, ...)
// SELECT Id, Title, Score, ViewCount, CommentCount, BadgeStatus, ValueCategory,
//        (SELECT MAX(CreationDate) FROM PostHistory ph
//         WHERE ph.PostId = ps.Id AND ph.PostHistoryTypeId IN (10,11))
// FROM PostStats ps WHERE ps.OwnerUserId IS NOT NULL
// ORDER BY ps.Score DESC, ps.ViewCount DESC LIMIT 50
//
// The tag unnest is an inner join, so it drops every post whose Tags is NULL —
// that is the whole filter it contributes, since `tags` is never projected.
// Comments, Badges and Votes then cross: CommentCount counts a comment once
// per badge and per vote.
fn q21293(db: &'static So) -> String {
    let Post { score, view_count, creation_date, tags_str, owner_user, .. } = &db.post;
    let closed = last_history(db, [10, 11]);
    let rk = db
        .post
        .with(creation_date.ge(year_ago()))
        .with(tags_str)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(_, s)| s, desc);
    // each of the top five is joined to its comments, its owner's badges and
    // its votes, and the three cross: COUNT(c.Id) and the upvote sum are over
    // those joined rows, MAX(b.Date) over the badges among them
    let stats = (&rk)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(
            comments_of(db)
                .opt()
                .and(owner_user.select(badges_of(db).select(&db.badge.date)).opt())
                .and(votes_of(db).select(&db.vote.vote_type_id).opt()),
        )
        .fold((0i64, i64::MIN, 0i64), |(c, bd, u), ((ci, b), vt)| {
            (c + ci.is_some() as i64, b.map_or(bd, |d| bd.max(d)), u + (vt == Some(2)) as i64)
        });
    let mut v = Vec::new();
    (&stats).and(score.and(view_count.opt())).and(&closed).drive(|p, (((c, bd, upvotes), (s, w)), ch)| {
        v.push((s, w, p, c, upvotes, bd, ch))
    });
    v.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| desc_nulls_last(&a.1, &b.1)));
    rows(v.iter().take(50).map(|&(s, w, p, c, upvotes, bd, ch)| {
        row(vec![
            V::I(db.post.origid.get(p).unwrap()),
            title(db, p),
            V::I(s),
            oint(w),
            V::I(c),
            V::S(if bd == i64::MIN { "No Active Badges" } else { "Active User" }),
            V::S(match upvotes {
                n if n > 5 => "Highly Valued",
                1..=5 => "Moderately Valued",
                _ => "Needs Attention",
            }),
            if ch == i64::MIN { V::Null } else { V::T(ch) },
        ])
    }))
}

// WITH RankedPosts AS (
//   SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount,
//          RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) RankScore,
//          (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id)
//   FROM Posts p WHERE p.CreationDate >= '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PopularUsers AS (
//   SELECT u.Id, u.DisplayName, SUM(COALESCE(v.BountyAmount,0)), COUNT(DISTINCT p.Id)
//   FROM Users u LEFT JOIN Posts p ON u.Id=p.OwnerUserId AND p.PostTypeId=1
//                LEFT JOIN Votes v ON p.Id=v.PostId AND v.VoteTypeId=9
//   GROUP BY u.Id, u.DisplayName
//   HAVING SUM(COALESCE(v.BountyAmount,0)) > 0 OR COUNT(DISTINCT p.Id) > 5),
// MergedInfo AS (SELECT ... FROM RankedPosts rp LEFT JOIN PopularUsers pu ON rp.OwnerUserId = pu.UserId)
// SELECT mi.PostId, mi.Title,
//        COALESCE(CAST(EXTRACT(EPOCH FROM ('2024-10-01 12:34:56' - mi.CreationDate))/3600 AS INT),0),
//        mi.ViewCount, mi.Score, mi.RankScore, mi.DisplayName, mi.TotalBounties,
//        mi.PostedQuestions, mi.ScoreCategory, STRING_AGG(t.TagName, ', ')
// FROM MergedInfo mi LEFT JOIN LATERAL (SELECT unnest(string_to_array(Tags,'<>')) FROM Posts WHERE Id=mi.PostId) t ON true
// WHERE (mi.Score IS NOT NULL AND mi.Score > 0) OR (mi.TotalBounties IS NOT NULL AND mi.TotalBounties > 0)
// GROUP BY ... ORDER BY mi.RankScore ASC, mi.ViewCount DESC LIMIT 100
//
// `string_to_array(Tags, '<>')` splits on a pair that never occurs — Tags is
// `<a><b>`, whose only pair is `><` — so the lateral gives back the whole tag
// string, one row, and STRING_AGG of it is `tags_str` itself.
// The Votes join carries `v.VoteTypeId = 9` in the ON clause, so it is a
// per-question bounty sum, not a cross product.
fn q22123(db: &'static So) -> String {
    let Post { post_type_id, title, score, view_count, creation_date, tags_str, owner_user, origid, .. } =
        &db.post;
    let User { display_name, .. } = &db.user;
    let bounty9 = db
        .vote
        .with((&db.vote.vote_type_id).eq(9))
        .select(&db.vote.post)
        .inv()
        .select((&db.vote.bounty_amount).opt())
        .dense_fold_outer(db.post.id.n, 0i64, |a, b| a + b.unwrap_or(0));
    let pop = db
        .post
        .with(post_type_id.eq(1))
        .select(owner_user)
        .inv()
        .select(&bounty9)
        .dense_fold_outer(db.user.id.n, (0i64, 0i64), |(n, b), x| (n + 1, b + x));
    let cmp = |a: &(i64, i64), b: &(i64, i64)| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1));
    let rk = db
        .post
        .with(creation_date.ge(year_ago()))
        .group_by(post_type_id)
        .select(
            origid
                .and(title.opt())
                .and(creation_date)
                .and(view_count.opt())
                .and(score)
                .and(owner_user.select(Ident::<User>::new().with((&pop).filt(|(n, b)| b > 0 || n > 5))).select(display_name.and(&pop)).opt())
                .and(tags_str.opt()),
        )
        .window(rank, |((((((_, _), cd), _), s), _), _)| (s, cd), cmp);
    let mut v = Vec::new();
    (&rk)
        .filt(|(((((((_, _), _), _), s), pu), _), _)| s > 0 || pu.map_or(false, |(_, (_, b))| b > 0))
        .drive(|_, (((((((id, t), cd), w), s), pu), tags), r)| v.push((r, w, id, t, cd, s, pu, tags)));
    v.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| desc_nulls_last(&a.1, &b.1)));
    rows(v.iter().take(100).map(|&(r, w, id, t, cd, s, pu, tags)| {
        row(vec![
            V::I(id),
            ostr(t),
            V::I(((now() - cd) as f64 / 1e6 / 3600.0).round() as i64),
            oint(w),
            V::I(s),
            V::I(r),
            ostr(pu.map(|(dn, _)| dn)),
            oint(pu.map(|(_, (_, b))| b)),
            oint(pu.map(|(_, (n, _))| n)),
            V::S(match s {
                x if x <= 5 => "Low",
                6..=15 => "Medium",
                _ => "High",
            }),
            ostr(tags),
        ])
    }))
}

// WITH UserVoteStats AS (
//   SELECT U.Id, U.DisplayName, SUM(CASE WHEN V.VoteTypeId=2 THEN 1 ELSE 0 END),
//          SUM(CASE WHEN V.VoteTypeId=3 THEN 1 ELSE 0 END), ...
//   FROM Users U LEFT JOIN Votes V ON U.Id=V.UserId LEFT JOIN Badges B ON U.Id=B.UserId
//   GROUP BY U.Id, U.DisplayName),
// PostsWithHistory AS (
//   SELECT P.Id, P.Title, PH.Comment, PH.CreationDate,
//          ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY PH.CreationDate DESC)
//   FROM Posts P LEFT JOIN PostHistory PH ON P.Id=PH.PostId AND PH.PostHistoryTypeId IN (10,11)
//   WHERE P.CreationDate >= DATE '2024-10-01' - INTERVAL '1 YEAR'),
// PostStatistics AS (
//   SELECT P.Id, P.Title, P.ViewCount, COALESCE(ROUND(AVG(C.Score),2),0), STRING_AGG(DISTINCT Tags.TagName,', ')
//   FROM Posts P LEFT JOIN Comments C ON P.Id=C.PostId
//                LEFT JOIN LATERAL (SELECT unnest(string_to_array(P.Tags,','))) Tags ON TRUE
//   WHERE P.PostTypeId=1 GROUP BY P.Id, P.Title, P.ViewCount)
// SELECT U.DisplayName, U.Reputation, COALESCE(PW.PostId,-1), COALESCE(PW.Title,'No Recent Post'),
//        COALESCE(PW.CloseReason,'N/A'), UPV.UpVotes, UPV.DownVotes, PS.ViewCount,
//        PS.AvgCommentScore, PS.TagList
// FROM UserVoteStats UPV JOIN Users U ON U.Id=UPV.UserId
// LEFT JOIN PostsWithHistory PW ON PW.LatestHistory=1 AND PW.PostId = U.Id
// LEFT JOIN PostStatistics PS ON PS.PostId = PW.PostId
// WHERE U.Reputation IS NOT NULL AND (U.Location IS NOT NULL OR U.AboutMe IS NOT NULL)
// ORDER BY UPV.UpVotes DESC, U.Reputation DESC LIMIT 50
//
// `PW.PostId = U.Id` is another user-id-to-post-id join, so it goes through
// `origid`. Votes.UserId is NULL for all but a handful of rows in this dump,
// so every one of the fifty rows has UpVotes 0 and no matching post: the
// ordering is decided entirely by Reputation.
fn q22247(db: &'static So) -> String {
    let User { origid, display_name, reputation, location, about_me, .. } = &db.user;
    // Votes and Badges both hang off u.Id, so they cross: the vote sums are
    // over the joined rows
    let votes = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold((0i64, 0i64), |(u, d), (t, _)| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let latest: HashIdx<Id<Post>, Id<PostHistory>> = db
        .post_history
        .with((&db.post_history.post_history_type_id).is_in([10, 11]))
        .group_by(&db.post_history.post)
        .select(Ident::<PostHistory>::new().and(&db.post_history.creation_date))
        .window(row_number, |(_, d)| d, desc)
        .filt(|(_, n)| n <= 1)
        .map(|((h, _), _)| h)
        .collect();
    let avgc = (&db.comment.post).inv().select(&db.comment.score).dense_fold_outer(
        db.post.id.n,
        (0i64, 0i64),
        |(n, s), x| (n + 1, s + x),
    );
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let recent = origid.select(&pidx).with((&db.post.creation_date).ge(date(2023, 10, 1)));
    let mut v = Vec::new();
    db.user
        .select(
            display_name
                .and(reputation)
                .and(&votes)
                .and(location.opt())
                .and(about_me.opt())
                .and(recent.opt()),
        )
        .filt(|(((((_, _), _), loc), about), _)| loc.is_some() || about.is_some())
        .drive(|_, (((((dn, rep), (u2, d3)), _), _), post)| v.push((u2, rep, dn, d3, post)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(50).map(|&(up, rep, dn, down, post)| {
        let mut f = vec![V::S(dn), V::I(rep)];
        match post {
            None => f.extend([V::I(-1), V::S("No Recent Post"), V::S("N/A"), V::I(up), V::I(down),
                              V::Null, V::Null, V::Null]),
            Some(p) => {
                let ph = latest.get(p);
                f.extend([
                    V::I(db.post.origid.get(p).unwrap()),
                    V::S(db.post.title.get(p).unwrap_or("No Recent Post")),
                    V::S(ph.and_then(|h| db.post_history.comment.get(h)).unwrap_or("N/A")),
                    V::I(up),
                    V::I(down),
                ]);
                if db.post.post_type_id.get(p) == Some(1) {
                    let (n, s) = avgc.get(p).unwrap();
                    f.extend([
                        oint(db.post.view_count.get(p)),
                        V::F(if n == 0 { 0.0 } else { (s as f64 / n as f64 * 100.0).round() / 100.0 }),
                        ostr(db.post.tags_str.get(p)),
                    ])
                } else {
                    f.extend([V::Null, V::Null, V::Null])
                }
            }
        }
        row(f)
    }))
}

// WITH RankedPosts AS (
//   SELECT P.Id, P.Title, P.CreationDate, P.Body, U.DisplayName,
//          COUNT(C.id), COUNT(A.id), SUM(CASE WHEN V.VoteTypeId=2 THEN 1 ELSE 0 END),
//          SUM(CASE WHEN V.VoteTypeId=3 THEN 1 ELSE 0 END), ROW_NUMBER() OVER (PARTITION BY P.Id ...)
//   FROM Posts P LEFT JOIN Comments C ON P.Id=C.PostId
//                LEFT JOIN Posts A ON P.Id=A.ParentId AND A.PostTypeId=2
//                LEFT JOIN Votes V ON P.Id=V.PostId
//                LEFT JOIN Users U ON P.OwnerUserId=U.Id
//   WHERE P.PostTypeId=1 GROUP BY P.Id, ...),
// HighScorePosts AS (SELECT ..., (UpVotes-DownVotes) Score, RANK() OVER (ORDER BY (UpVotes-DownVotes) DESC))
// SELECT ..., TagArray FROM HighScorePosts HSP
// JOIN (SELECT P.Id, STRING_AGG(T.TagName, ', ') FROM Posts P
//       CROSS JOIN LATERAL UNNEST(STRING_TO_ARRAY(SUBSTRING(P.Tags,2,LENGTH(P.Tags)-2),'><')) T(TagName)
//       GROUP BY P.Id) Tags ON HSP.PostId = Tags.PostId
// WHERE HSP.PostRank <= 10 ORDER BY HSP.Score DESC
//
// `substring(Tags, 2, len-2)` split on `><` is `harness::tag_list`. RANK, not
// ROW_NUMBER, so the ties at the cut all share a rank and the answer is total.
// rewrites/25191: STRING_AGG with no ORDER BY. DuckDB is stable here across
// thread counts but emits the tags in reverse array order, which is not an
// answer the SQL asks for, so the rewrite sorts them.
fn q25191(db: &'static So) -> String {
    let Post { post_type_id, title, body, creation_date, tags_str, owner_user, origid, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1));
    // each question joined to its comments, answers and votes, which cross:
    // COUNT(C.id), COUNT(A.id) and the vote sums are over the joined rows
    let per = (&base)
        .group_by(Ident::<Post>::new())
        .select(
            comments_of(db)
                .opt()
                .and(children_of(db).with(post_type_id.eq(2)).opt())
                .and(votes_of(db).select(&db.vote.vote_type_id).opt()),
        )
        .fold((0i64, 0i64, 0i64, 0i64), |(c, a, u, d), ((ci, ai), vt)| {
            (c + ci.is_some() as i64, a + ai.is_some() as i64, u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64)
        });
    let rk = whole(&base).select(Ident::<Post>::new().and(&per)).window(rank, |(_, (_, _, u, d))| u - d, desc);
    let mut v = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .with(tags_str)
        .select(
            origid
                .and(title.opt())
                .and(creation_date)
                .and(body)
                .and(owner_user.select(&db.user.display_name).opt())
                .and(&per)
                .and(tags_str),
        )
        .drive(|_, ((((((id, t), cd), bd), au), (c, a, u, d)), tags)| {
            v.push((u - d, id, t, cd, bd, au, c, a, tags))
        });
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().map(|&(sc, id, t, cd, bd, au, c, a, tags)| {
        row(vec![
            V::I(id),
            ostr(t),
            V::T(cd),
            V::S(bd),
            ostr(au),
            V::I(c),
            V::I(a),
            V::I(sc),
            V::S(leak_join({
                let mut ts: Vec<Str> = tag_list(tags).collect();
                ts.sort_unstable();
                ts
            }, ", ")),
        ])
    }))
}

// WITH PostDetails AS (
//   SELECT p.Id, p.Title, p.Score, p.CreationDate, COALESCE(p.AcceptedAnswerId,-1), p.ViewCount,
//          COUNT(c.Id), SUM(CASE WHEN v.VoteTypeId=2 THEN 1 ELSE 0 END),
//          SUM(CASE WHEN v.VoteTypeId=3 THEN 1 ELSE 0 END), STRING_AGG(DISTINCT t.TagName,', ')
//   FROM Posts p LEFT JOIN Comments c ON c.PostId=p.Id
//                LEFT JOIN Votes v ON v.PostId=p.Id
//                LEFT JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags,2,length(p.Tags)-2),'><'))) tag ON true
//                LEFT JOIN Tags t ON t.TagName = tag.TagName
//   WHERE p.CreationDate >= '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.ViewCount > 0
//   GROUP BY p.Id, ...),
// RankedPosts AS (SELECT pd.*, ROW_NUMBER() OVER (PARTITION BY pd.AcceptedAnswerId ORDER BY pd.Score DESC [, pd.PostId]) Rank),
// PostsWithBadges AS (SELECT rp.*, b.Name, COUNT(b.Id) OVER (PARTITION BY rp.PostId) BadgeCount
//   FROM RankedPosts rp LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id=rp.PostId)),
// FinalOutput AS (SELECT ..., CASE WHEN BadgeCount IS NULL THEN 'No Badge' ELSE 'Has Badge' END WHERE Rank=1)
// SELECT *, CASE WHEN BadgeStatus='Has Badge' THEN 'Congratulations...' ELSE '...' END
// FROM FinalOutput ORDER BY CommentCount DESC NULLS LAST, Score DESC
//
// `COUNT(...) OVER (...)` is 0, never NULL, so BadgeStatus is 'Has Badge' on
// every row of the answer — including the posts whose owner has no badge at
// all, which still contribute one row with a NULL BadgeName.
// The tag lateral is an outer join, so a post with no Tags still counts once;
// it is the third child in the product the counts are taken over.
// rewrites/20479: the ROW_NUMBER over `AcceptedAnswerId = -1` covers thousands
// of posts and its ORDER BY is not total.
fn q20479(db: &'static So) -> String {
    let Post { score, view_count, creation_date, tags_str, accepted_answer_id, owner_user, origid, .. } =
        &db.post;
    let base = db.post.with(creation_date.ge(year_ago())).with(view_count.gt(0));
    // each post joined to its comments, its votes and the pieces of its tag
    // string (the lateral; a post with no tags keeps one row): the counts are
    // over those joined rows
    let per = (&base)
        .group_by(Ident::<Post>::new())
        .select(
            comments_of(db)
                .opt()
                .and(votes_of(db).select(&db.vote.vote_type_id).opt())
                .and(tags_str.flat_map(tag_list).opt()),
        )
        .fold((0i64, 0i64, 0i64), |(c, u, d), ((ci, vt), _)| {
            (c + ci.is_some() as i64, u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64)
        });
    let key = accepted_answer_id.opt().map(|a: Option<i64>| a.unwrap_or(-1));
    let cmp = |a: &(i64, i64), b: &(i64, i64)| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1));
    let rk = (&base)
        .group_by(key)
        .select(Ident::<Post>::new().and(score).and(origid))
        .window(row_number, |((_, s), id)| (s, id), cmp);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, n)| n == 1)
        .map(|(((p, _), _), _)| p)
        .select(
            Ident::<Post>::new()
                .and(origid)
                .and(score)
                .and(&per)
                .and(owner_user.select(badges_of(db).select(&db.badge.name)).opt()),
        )
        .drive(|_, ((((p, id), s), (c, u2, d3)), badge)| out.push((id, p, s, c, u2, d3, badge)));
    rows(out.iter().map(|&(id, p, s, c, u2, d3, badge)| {
        row(vec![
            V::I(id),
            title(db, p),
            V::I(s),
            V::I(c),
            V::I(u2),
            V::I(d3),
            ostr(badge),
            V::S("Has Badge"),
            V::S("Congratulations on your achievement!"),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, ..., ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC)
//                      FROM Posts p WHERE p.PostTypeId=1 AND p.CreationDate >= '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserAggregates AS (SELECT u.Id, u.DisplayName, COUNT(DISTINCT r.PostId), SUM(r.Score)
//                    FROM Users u LEFT JOIN RankedPosts r ON u.Id=r.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) ScoreRank FROM UserAggregates)
// SELECT tu.DisplayName, tu.QuestionCount, tu.TotalScore,
//        CASE WHEN tu.ScoreRank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END,
//        COALESCE((SELECT STRING_AGG(DISTINCT t.TagName, ', ' [ORDER BY t.TagName])
//                  FROM Posts p JOIN UNNEST(string_to_array(p.Tags, ',')) t(TagName) ON t.TagName IS NOT NULL
//                  WHERE p.OwnerUserId = tu.UserId AND p.PostTypeId = 1), 'No Tags')
// FROM TopUsers tu WHERE tu.QuestionCount > 5 ORDER BY tu.TotalScore DESC, tu.QuestionCount DESC
//
// The correlated aggregate splits Tags on ',', which never occurs, so it is
// the distinct whole tag strings of the user's questions — over all of them,
// not just the recent ones the rank is built from.
// rewrites/2020: STRING_AGG(DISTINCT ...) has no ORDER BY, so DuckDB's answer
// moved with the thread count.
fn q2020(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, tags_str, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let ua = db
        .post
        .with(post_type_id.eq(1))
        .with(creation_date.ge(year_ago()))
        .select(owner_user)
        .inv()
        .select(score)
        .dense_fold_outer(db.user.id.n, (0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let tags = db
        .post
        .with(post_type_id.eq(1))
        .with(tags_str)
        .group_by(owner_user)
        .select(tags_str)
        .buf_fold(|mut xs| {
            xs.sort_unstable();
            xs.dedup();
            leak_join(xs.into_iter(), ", ")
        });
    let base = db.user.with(&ua);
    let rk = whole(&base)
        .select(display_name.and(&ua).and((&tags).opt()))
        .window(rank, |((_, (n, s)), _)| if n == 0 { None } else { Some(s) }, desc_nulls_last);
    let mut v = Vec::new();
    (&rk).filt(|(((_, (n, _)), _), _)| n > 5).drive(|_, (((dn, (n, s)), tg), r)| v.push((s, n, dn, r, tg)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().map(|&(s, n, dn, r, tg)| {
        row(vec![
            V::S(dn),
            V::I(n),
            V::I(s),
            V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" }),
            V::S(tg.unwrap_or("No Tags")),
        ])
    }))
}

// WITH TaggedPosts AS (
//   SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score,
//          (SELECT COUNT(*) FROM Comments c WHERE c.PostId=p.Id), t.TagName,
//          DENSE_RANK() OVER (PARTITION BY t.TagName ORDER BY p.ViewCount DESC [, p.Id]) TagRank
//   FROM Posts p JOIN UNNEST(string_to_array(substring(p.Tags,2,length(p.Tags)-2),'><')) t(TagName) ON t.TagName IS NOT NULL
//   WHERE p.PostTypeId=1 AND p.Score>0),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId=2 THEN 1 END), COUNT(CASE WHEN v.VoteTypeId=3 THEN 1 END)
//                 FROM Votes v WHERE v.CreationDate > '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY v.PostId),
// PostHistories AS (SELECT ph.PostId, ARRAY_AGG(DISTINCT pht.Name [ORDER BY pht.Name])
//                   FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId=pht.Id GROUP BY ph.PostId),
// PostStats AS (SELECT ..., COALESCE(tp.Score + COALESCE(up,0) - COALESCE(down,0), tp.Score) AdjustedScore, tp.TagName, tp.TagRank)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.ChangeTypes, ps.AdjustedScore, EngagementLevel
// FROM PostStats ps
// WHERE EXISTS (SELECT 1 FROM PostStats ps2 WHERE ps2.TagRank <= 5 AND ps.TagName = ps2.TagName)
// ORDER BY ps.AdjustedScore DESC NULLS LAST, ps.ViewCount DESC [, ps.PostId, ps.TagName]
// OFFSET 10 ROWS FETCH NEXT 20 ROWS ONLY
//
// The EXISTS is a no-op: DENSE_RANK starts at 1 in every partition, so every
// tag in PostStats has a row with TagRank 1 and passes.
// TagName is not projected, so a post with several tags appears several times
// as identical rows — the first two rows of the answer are one such pair.
// rewrites/22681: ARRAY_AGG(DISTINCT ...) is unordered, the DENSE_RANK order
// is not total, and neither is the final ORDER BY under OFFSET/FETCH.
fn q22681(db: &'static So) -> String {
    let Post { post_type_id, title, score, view_count, creation_date, tags_str, origid, .. } = &db.post;
    let recent = db.vote.with((&db.vote.creation_date).gt(add_days(now(), -30)));
    let rv = recent.select(&db.vote.post).inv().select(&db.vote.vote_type_id).dense_fold_outer(
        db.post.id.n,
        (0i64, 0i64),
        |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64),
    );
    let ct = db
        .post_history
        .group_by(&db.post_history.post)
        .select((&db.post_history.post_history_type).select(&db.post_history_type.name))
        .buf_fold(|mut xs| {
            xs.sort_unstable();
            xs.dedup();
            &*Box::leak(xs.into_iter().collect::<Vec<Str>>().into_boxed_slice())
        });
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(score.gt(0))
        .with(tags_str)
        .select(
            origid
                .and(title.opt())
                .and(creation_date)
                .and(view_count.opt())
                .and(score)
                .and(&rv)
                .and((&ct).opt())
                .and(tags_str.flat_map(tag_list)),
        )
        .drive(|_, (((((((id, t), cd), w), s), (u, d)), types), tag)| v.push((s + u - d, w, id, tag, t, cd, types)));
    v.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| desc_nulls_last(&a.1, &b.1))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.3.cmp(b.3))
    });
    rows(v.iter().skip(10).take(20).map(|&(adj, w, id, _, t, cd, types)| {
        row(vec![
            V::I(id),
            ostr(t),
            V::T(cd),
            oint(w),
            match types {
                None => V::Null,
                Some(ts) => V::L(ts.iter().map(|&s| V::S(s)).collect()),
            },
            V::I(adj),
            V::S(match adj {
                x if x < 0 => "Needs improvement",
                0..=50 => "Moderate engagement",
                _ => "High engagement",
            }),
        ])
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("2020", q2020),
    ("20479", q20479),
    ("21293", q21293),
    ("22123", q22123),
    ("22247", q22247),
    ("22681", q22681),
    ("25191", q25191),
];
