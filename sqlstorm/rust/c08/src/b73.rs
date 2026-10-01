use harness::prelude::*;
use std::cmp::Reverse;

// Posts per user, counting every post (LEFT JOIN, so 0 for a user with none).
fn posts_per_user(db: &'static So) -> DenseFold<Id<User>, i64> {
    (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1)
}

// SQL Posts.Id -> the dense post, for the queries that join a user id to a
// post id. `origid` is the raw SQL id the cache keeps beside the dense one.
fn post_by_sql_id(db: &'static So) -> HashIdx<i64, Id<Post>> {
    (&db.post.origid).inv().collect()
}

fn user_by_sql_id(db: &'static So) -> HashIdx<i64, Id<User>> {
    (&db.user.origid).inv().collect()
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

// WITH UserActivity AS (
//   SELECT u.Id, u.DisplayName, COUNT(DISTINCT p.Id), COUNT(DISTINCT c.Id),
//          SUM(CASE WHEN v.VoteTypeId=2 THEN 1 ELSE 0 END),
//          SUM(CASE WHEN v.VoteTypeId=3 THEN 1 ELSE 0 END),
//          COUNT(DISTINCT b.Id), SUM(p.ViewCount), SUM(p.Score)
//   FROM Users u LEFT JOIN Posts p ON u.Id=p.OwnerUserId
//                LEFT JOIN Comments c ON p.Id=c.PostId
//                LEFT JOIN Votes v ON p.Id=v.PostId
//                LEFT JOIN Badges b ON u.Id=b.UserId
//   GROUP BY u.Id, u.DisplayName)
// SELECT * FROM UserActivity ORDER BY TotalScore DESC
//
// Four LEFT JOINs off two keys, so the FROM is a 314-million-row cross
// product: comments and votes multiply per post, badges multiply the lot.
// `user_rows` folds that product; each COUNT(DISTINCT) is the per-user sum of
// per-post (or per-user) child counts.
fn q10079(db: &'static So) -> String {
    user_rows(db, "cvb", false, "score_sum", 0,
        &["uid", "name", "#n", "#c", "#up", "#down", "#b", "views_sum", "score_sum"])
}

// SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score,
//        COALESCE(v.vote_count,0), COALESCE(c.comment_count,0),
//        u.Reputation, u.DisplayName
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN (SELECT PostId, COUNT(*) FROM Votes GROUP BY PostId) v ON p.Id=v.PostId
// LEFT JOIN (SELECT PostId, COUNT(*) FROM Comments GROUP BY PostId) c ON p.Id=c.PostId
// WHERE p.CreationDate >= '2023-01-01'
// ORDER BY p.Score DESC, p.ViewCount DESC [, p.Id] LIMIT 100
//
// The two derived tables are `votes_per_post` and `comments_per_post`: a
// pre-aggregated relation keyed by the join column is exactly a DenseFold,
// and COALESCE(...,0) is what `dense_fold_outer` already gives.
// rewrites/10082: rows 99-103 are all (Score 10, ViewCount NULL).
fn q10082(db: &'static So) -> String {
    let Post { score, view_count, creation_date, owner_user, .. } = &db.post;
    let vc = votes_per_post(db);
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post
        .with(creation_date.ge(date(2023, 1, 1)))
        .with(owner_user)
        .select(score.and(view_count.opt()).and(&vc).and(&cc))
        .drive(|p, (((s, w), nv), nc)| v.push((s, w, db.post.origid.get(p).unwrap(), p, nv, nc)));
    v.sort_by(|a, b| {
        b.0.cmp(&a.0).then_with(|| desc_nulls_last(&a.1, &b.1)).then_with(|| a.2.cmp(&b.2))
    });
    rows(v.iter().take(100).map(|&(_, _, _, p, nv, nc)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(nv), V::I(nc)]);
        f.extend(post_fields(db, p, &["rep", "owner"]));
        row(f)
    }))
}

// SELECT pt.Name, COUNT(DISTINCT p.Id),
//        COALESCE(AVG(vote_counts.VoteCount),0), COALESCE(AVG(comment_counts.CommentCount),0)
// FROM PostTypes pt LEFT JOIN Posts p ON p.PostTypeId = pt.Id
// LEFT JOIN (SELECT PostId, COUNT(*) FROM Votes GROUP BY PostId) vote_counts ON ...
// LEFT JOIN (SELECT PostId, COUNT(*) FROM Comments GROUP BY PostId) comment_counts ON ...
// GROUP BY pt.Name ORDER BY TotalPosts DESC
//
// Driven from PostTypes, so the two types with no posts keep a row: that is
// `dense_fold_outer` over the post-type ids. The derived tables hold only
// posts that have a vote (comment), so AVG divides by that count, not by the
// number of posts.
fn q10066(db: &'static So) -> String {
    let vc = (&db.vote.post).inv().fold(0i64, |a, _| a + 1);
    let cc = (&db.comment.post).inv().fold(0i64, |a, _| a + 1);
    let agg = db.post.select(&db.post.post_type).inv().select((&vc).opt().and((&cc).opt())).dense_fold_outer(
        db.post_type.id.n,
        (0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, vs, vn, cs, cn), (v, c)| {
            (n + 1, vs + v.unwrap_or(0), vn + v.is_some() as i64, cs + c.unwrap_or(0), cn + c.is_some() as i64)
        },
    );
    let mean = |s: i64, n: i64| V::F(if n == 0 { 0.0 } else { s as f64 / n as f64 });
    let mut out = Vec::new();
    db.post_type.select((&db.post_type.name).and(&agg)).drive(|_, (name, (n, vs, vn, cs, cn))| {
        out.push(row(vec![V::S(name), V::I(n), mean(vs, vn), mean(cs, cn)]))
    });
    rows(out)
}

// WITH UserPostCount AS (SELECT u.Id UserId, COUNT(p.Id) FROM Users u
//        LEFT JOIN Posts p ON u.Id=p.OwnerUserId GROUP BY u.Id),
//      PostVoteCount AS (SELECT p.Id PostId, COUNT(v.Id) ... GROUP BY p.Id),
//      PostCommentCount AS (SELECT p.Id PostId, COUNT(c.Id) ... GROUP BY p.Id)
// SELECT u.DisplayName, u.Reputation, up.PostCount, pv.VoteCount, pc.CommentCount
// FROM Users u JOIN UserPostCount up ON u.Id=up.UserId
// LEFT JOIN PostVoteCount pv ON up.UserId = pv.PostId
// LEFT JOIN PostCommentCount pc ON up.UserId = pc.PostId
// ORDER BY u.Reputation DESC
//
// The last two joins compare a user id to a post id, which is meaningless but
// legal. prela renumbers ids densely per entity, so the join has to go through
// `origid`, the raw SQL id: `post_by_sql_id` is that index.
fn q10036(db: &'static So) -> String {
    let User { origid, display_name, reputation, .. } = &db.user;
    let pidx = post_by_sql_id(db);
    let pu = posts_per_user(db);
    let vc = votes_per_post(db);
    let cc = comments_per_post(db);
    let same = origid.select(&pidx).select((&vc).and(&cc));
    let mut out = Vec::new();
    db.user.select(display_name.and(reputation).and(&pu).and(same.opt())).drive(
        |_, (((dn, rep), n), hit)| {
            let (v, c) = match hit {
                Some((v, c)) => (V::I(v), V::I(c)),
                None => (V::Null, V::Null),
            };
            out.push(row(vec![V::S(dn), V::I(rep), V::I(n), v, c]))
        },
    );
    rows(out)
}

// WITH UserStats AS (SELECT u.Id, u.DisplayName, COUNT(DISTINCT p.Id),
//        COALESCE(SUM(v.BountyAmount),0), SUM(u.UpVotes), SUM(u.DownVotes)
//      FROM Users u LEFT JOIN Posts p ON u.Id=p.OwnerUserId
//                   LEFT JOIN Votes v ON u.Id=v.UserId GROUP BY u.Id, u.DisplayName),
//      PostStats AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score,
//        COUNT(c.Id) FROM Posts p LEFT JOIN Comments c ON p.Id=c.PostId GROUP BY p.Id, ...)
// SELECT us.*, ps.* FROM UserStats us JOIN PostStats ps ON us.UserId = ps.PostId
// ORDER BY us.TotalPosts DESC, ps.ViewCount DESC
//
// Posts and Votes both hang off u.Id and are independent, so the group is
// their product, and the bounty sum and SUM(u.UpVotes) are over its rows.
fn q10011(db: &'static So) -> String {
    let User { origid, display_name, up_votes, down_votes, .. } = &db.user;
    let pu = posts_per_user(db);
    // Posts and Votes both hang off u.Id and cross; the sums are over those
    // joined rows, the user's own vote counts read once per row
    let joined = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64, 0i64), |(bs, us, ds), (((uv, dv), _), b)| {
            (bs + b.flatten().unwrap_or(0), us + uv, ds + dv)
        });
    let pidx = post_by_sql_id(db);
    let cc = comments_per_post(db);
    let mut out = Vec::new();
    db.user
        .select(
            origid
                .and(display_name)
                .and(&pu)
                .and(&joined)
                .and(origid.select(&pidx).select(Ident::<Post>::new().and(&cc))),
        )
        .drive(|_, ((((id, dn), p), (bs, us, ds)), (post, nc))| {
            let mut f = vec![V::I(id), V::S(dn), V::I(p), V::I(bs), V::I(us), V::I(ds)];
            f.extend(post_fields(db, post, &["id", "title", "created", "views", "score"]));
            f.push(V::I(nc));
            out.push(row(f))
        });
    rows(out)
}

// WITH PostStats AS (SELECT Posts.Id, Title, CreationDate, Score, ViewCount,
//        COUNT(Votes.Id), COUNT(Comments.Id),
//        SUM(CASE WHEN Votes.VoteTypeId=2 THEN 1 ELSE 0 END),
//        SUM(CASE WHEN Votes.VoteTypeId=3 THEN 1 ELSE 0 END),
//        (SELECT COUNT(1) FROM Posts Answers WHERE Answers.ParentId=Posts.Id)
//      FROM Posts LEFT JOIN Votes ON ... LEFT JOIN Comments ON ...
//      WHERE Posts.PostTypeId=1 GROUP BY Posts.Id, ...),
//      UserStats AS (SELECT Users.Id, DisplayName, COUNT(Badges.Id),
//        SUM(Posts.ViewCount), SUM(Posts.Score)
//      FROM Users LEFT JOIN Badges ON ... LEFT JOIN Posts ON ... GROUP BY Users.Id, ...)
// SELECT ... FROM PostStats PS JOIN UserStats US ON PS.PostId = US.UserId
// ORDER BY PS.Score DESC, PS.ViewCount DESC LIMIT 100
//
// Both CTEs cross two independent fan-outs, and the join again matches a post
// id against a user id.
fn q10033(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, origid, parent, .. } = &db.post;
    let questions = db.post.with(post_type_id.eq(1));
    let eng = engagement(db, &questions);
    let ans = parent.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    // Badges and Posts both hang off Users.Id and cross; every count and sum
    // is over those joined rows
    let ustat = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(score.and(view_count.opt())).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(b, pn, ss, vn, vs), (bi, p)| {
            let w = p.and_then(|(_, v)| v);
            (b + bi.is_some() as i64, pn + p.is_some() as i64, ss + p.map_or(0, |(s, _)| s), vn + w.is_some() as i64, vs + w.unwrap_or(0))
        });
    let uidx = user_by_sql_id(db);
    let mut v = Vec::new();
    (&questions)
        .with(origid.select(&uidx))
        .select(
            score
                .and(view_count.opt())
                .and(&eng)
                .and(&ans)
                .and(origid.select(&uidx).select((&db.user.origid).and(&db.user.display_name).and(&ustat))),
        )
        .drive(|p, ((((s, w), (c, vt, u2, d3)), a), ((uid, udn), (b, pn, ss, vn, vs)))| {
            v.push((s, w, p, c, vt, u2, d3, a, uid, udn, b, pn, ss, vn, vs))
        });
    v.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| desc_nulls_last(&a.1, &b.1)));
    rows(v.iter().take(100).map(
        |&(s, w, p, c, vt, u2, d3, a, uid, udn, b, pn, ss, vn, vs)| {
            row(vec![
                V::I(db.post.origid.get(p).unwrap()),
                title(db, p),
                V::T(db.post.creation_date.get(p).unwrap()),
                V::I(s),
                oint(w),
                V::I(vt),
                V::I(c),
                V::I(u2),
                V::I(d3),
                V::I(a),
                V::I(uid),
                V::S(udn),
                V::I(b),
                nullable(vs, vn),
                nullable(ss, pn),
            ])
        },
    ))
}

// WITH PostStats AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount,
//        COUNT(c.Id), COUNT(v.Id), pt.Name, u.DisplayName
//      FROM Posts p LEFT JOIN Comments c ON p.Id=c.PostId
//                   LEFT JOIN Votes v ON p.Id=v.PostId
//                   LEFT JOIN PostTypes pt ON ... LEFT JOIN Users u ON ...
//      GROUP BY p.Id, ...),
//      TopPosts AS (SELECT *, ROW_NUMBER() OVER (ORDER BY Score DESC [, PostId]) ScoreRank,
//                             ROW_NUMBER() OVER (ORDER BY ViewCount DESC [, PostId]) ViewRank
//                   FROM PostStats)
// SELECT * FROM TopPosts WHERE ScoreRank <= 10 OR ViewRank <= 10
// ORDER BY Score DESC, ViewCount DESC
//
// rewrites/10206: both ROW_NUMBERs are projected and neither window ORDER BY
// is total, so the ranks printed for the NULL-ViewCount answers were whatever
// DuckDB happened to pick.
fn q10206(db: &'static So) -> String {
    let Post { score, view_count, origid, post_type, owner_user, .. } = &db.post;
    let base = db.post.iq();
    let eng = engagement(db, db.post.iq());
    let by_score = |a: &(i64, i64), b: &(i64, i64)| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1));
    let by_views = |a: &(Option<i64>, i64), b: &(Option<i64>, i64)| {
        desc_nulls_last(&a.0, &b.0).then_with(|| a.1.cmp(&b.1))
    };
    let ranked = whole(&base)
        .select(
            Ident::<Post>::new()
                .and(score)
                .and(view_count.opt())
                .and(origid)
                .and(&eng)
                .and(post_type.select(&db.post_type.name))
                .and(owner_user.select(&db.user.display_name).opt()),
        )
        .window(row_number, |((((((_, s), _), id), _), _), _)| (s, id), by_score)
        .window(row_number, |(((((((_, _), w), id), _), _), _), _)| (w, id), by_views);
    let mut out = Vec::new();
    ranked
        .filt(|((_, sr), vr)| sr <= 10 || vr <= 10)
        .drive(|_, ((((((((p, s), w), _), (c, v, _, _)), pt), dn), sr), vr)| {
            out.push(row(vec![
                V::I(db.post.origid.get(p).unwrap()),
                title(db, p),
                V::T(db.post.creation_date.get(p).unwrap()),
                V::I(s),
                oint(w),
                V::I(c),
                V::I(v),
                V::S(pt),
                ostr(dn),
                V::I(sr),
                V::I(vr),
            ]))
        });
    rows(out)
}

// WITH PostMetrics AS (SELECT P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score,
//        P.AnswerCount, COALESCE(U.DisplayName,'Deleted User'),
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END),
//        SUM(CASE WHEN V.VoteTypeId=2 THEN 1 ELSE 0 END),
//        SUM(CASE WHEN V.VoteTypeId=3 THEN 1 ELSE 0 END)
//      FROM Posts P LEFT JOIN Users U ... LEFT JOIN Comments C ... LEFT JOIN Votes V ...
//      WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year'
//      GROUP BY P.Id, ..., U.DisplayName)
// SELECT *, RANK() OVER (ORDER BY ViewCount DESC), RANK() OVER (ORDER BY Score DESC)
// FROM PostMetrics ORDER BY ViewCount DESC, Score DESC
//
// CURRENT_DATE, so the answer moves with the clock: the dba dump stops in
// October 2024 and this is empty for any run after October 2025.
fn q10089(db: &'static So) -> String {
    let Post { score, view_count, answer_count, creation_date, origid, owner_user, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_years(current_date(), -1)));
    let eng = engagement(db, &base);
    let by_views = |a: &Option<i64>, b: &Option<i64>| desc_nulls_last(a, b);
    let ranked = whole(&base)
        .select(
            Ident::<Post>::new()
                .and(origid)
                .and(view_count.opt())
                .and(score)
                .and(answer_count.opt())
                .and(owner_user.select(&db.user.display_name).opt())
                .and(&eng),
        )
        .window(rank, |(((((( _, _), w), _), _), _), _)| w, by_views)
        .window(rank, |((((((( _, _), _), s), _), _), _), _)| s, desc);
    let mut out = Vec::new();
    ranked
        .drive(|_, ((((((((p, id), w), s), ac), dname), (c, _, u2, d3)), vr), sr)| {
            out.push(row(vec![
                V::I(id),
                title(db, p),
                V::T(db.post.creation_date.get(p).unwrap()),
                oint(w),
                V::I(s),
                oint(ac),
                V::S(dname.unwrap_or("Deleted User")),
                V::I(c),
                V::I(u2),
                V::I(d3),
                V::I(vr),
                V::I(sr),
            ]))
        });
    rows(out)
}

// WITH UserActivity AS (SELECT u.Id, u.DisplayName, COUNT(DISTINCT p.Id),
//        COUNT(DISTINCT c.Id), SUM(v.BountyAmount), SUM(u.UpVotes), SUM(u.DownVotes)
//      FROM Users u LEFT JOIN Posts p ON u.Id=p.OwnerUserId
//                   LEFT JOIN Comments c ON u.Id=c.UserId
//                   LEFT JOIN Votes v ON u.Id=v.UserId GROUP BY u.Id, u.DisplayName),
//      TopUsers AS (SELECT *, ROW_NUMBER() OVER (ORDER BY PostCount DESC) Rank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, CommentCount, TotalBounties,
//        TotalUpVotes, TotalDownVotes FROM TopUsers WHERE Rank <= 10
//
// Three independent fan-outs off u.Id: a 1.08-billion-row FROM. The rank
// reads only COUNT(DISTINCT p.Id), the user's own post count, so the ten users
// are picked first and the product is driven only for them.
fn q10002(db: &'static So) -> String {
    let User { up_votes, down_votes, .. } = &db.user;
    let pc = posts_per_user(db);
    let cu = comments_per_user(db);
    let rn = whole(&db.user.id)
        .select(Ident::<User>::new().and(&pc))
        .window(row_number, |(u, n): (Id<User>, i64)| (n, Reverse(u)), desc);
    let agg = (&rn)
        .filt(|(_, r)| r <= 10)
        .map(|((u, _), _)| u)
        .group_by(Ident::<User>::new())
        .select(
            up_votes
                .and(down_votes)
                .and(posts_of(db).opt())
                .and(comments_by(db).opt())
                .and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()),
        )
        .fold((0i64, 0i64, 0i64, 0i64), |(bs, bn, us, ds), ((((uv, dv), _), _), b)| {
            let b = b.flatten();
            (bs + b.unwrap_or(0), bn + b.is_some() as i64, us + uv, ds + dv)
        });
    let mut out = Vec::new();
    (&agg).and(&pc).and(&cu).drive(|u, (((bs, bn, us, ds), n), c)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c), nullable(bs, bn), V::I(us), V::I(ds)]);
        out.push(row(f))
    });
    rows(out)
}

// The rank reads only TotalPosts, the user's own post count, so the ten users
// are picked first. `JOIN PostTypesCount ON M.TotalPosts > 0` is a cross join
// with a condition on M alone: a filter on the ranked users, then `.cross`.
fn q10046(db: &'static So) -> String {
    let User { reputation, .. } = &db.user;
    let pc = posts_per_user(db);
    let cu = comments_per_user(db);
    let base = db.user.with(reputation.gt(100));
    let rn = whole(&base)
        .select(Ident::<User>::new().and(&pc))
        .window(row_number, |(u, n): (Id<User>, i64)| (n, Reverse(u)), desc);
    let agg = (&rn)
        .filt(|((_, n), r)| r <= 10 && n > 0)
        .map(|((u, _), _)| u)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(comments_by(db).opt()))
        .fold((0i64, 0i64, 0i64), |(up, down, fav), (v, _)| {
            let t = v.flatten();
            (up + (t == Some(2)) as i64, down + (t == Some(3)) as i64, fav + (t == Some(5)) as i64)
        });
    let types = db.post.group_by((&db.post.post_type).select(&db.post_type.name)).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&agg).and(&pc).and(&cu).cross(&types).drive(|(u, t), ((((up, down, fav), n), c), tn)| {
        v.push((n, t, u, c, up, down, fav, tn))
    });
    v.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    rows(v.into_iter().map(|(n, t, u, c, up, down, fav, tn)| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(n), V::I(c), V::I(up), V::I(down), V::I(fav), V::S(t), V::I(tn)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10002", q10002),
    ("10011", q10011),
    ("10033", q10033),
    ("10036", q10036),
    ("10046", q10046),
    ("10066", q10066),
    ("10079", q10079),
    ("10082", q10082),
    ("10089", q10089),
    ("10206", q10206),
];
