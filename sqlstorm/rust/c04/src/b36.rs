use harness::prelude::*;

fn comments_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.comment.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

fn votes_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.vote.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

// ---- Per post type ------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TS {
    n: i64,
    score_sum: i64,
    view_n: i64,
    view_sum: i64,
    ans_n: i64,
    ans_sum: i64,
    com_col_sum: i64,
    owners: i64,
    comments: i64,
    votes: i64,
    votes_n: i64,
}

fn type_stats(db: &'static So) -> HashIdx<Str, TS> {
    let Post { post_type, score, view_count, answer_count, comment_count, owner_user_id, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let cpp = (&db.comment.post).inv().fold(0i64, |a, _| a + 1);
    let vpp = (&db.vote.post).inv().fold(0i64, |a, _| a + 1);
    let ow = db.post.group_by(post_type.select(name)).select(owner_user_id).count_distinct();

    let main = db
        .post
        .group_by(post_type.select(name))
        .select(score.and(comment_count).and((&cpp).opt()).and((&vpp).opt()).and(view_count.opt()).and(answer_count.opt()))
        .fold([0i64; 10], |a, (((((sc, ccol), c), v), w), an)| {
            [
                a[0] + 1,
                a[1] + sc,
                a[2] + ccol,
                a[3] + c.unwrap_or(0),
                a[4] + v.unwrap_or(0),
                a[5] + v.is_some() as i64,
                a[6] + w.is_some() as i64,
                a[7] + w.unwrap_or(0),
                a[8] + an.is_some() as i64,
                a[9] + an.unwrap_or(0),
            ]
        });
    (&main)
        .and((&ow).opt())
        .map(|(a, o): ([i64; 10], Option<i64>)| TS {
            n: a[0],
            score_sum: a[1],
            com_col_sum: a[2],
            comments: a[3],
            votes: a[4],
            votes_n: a[5],
            view_n: a[6],
            view_sum: a[7],
            ans_n: a[8],
            ans_sum: a[9],
            owners: o.unwrap_or(0),
        })
        .collect()
}

/// `(PostCount * 1.0) / NULLIF(SUM(PostCount) OVER (), 0) * 100`: the window
/// has no PARTITION BY, so its sum is one row, crossed with every type.
fn q13574(db: &'static So) -> String {
    let t = type_stats(db);
    let total = whole(&t).select(&t).fold(0i64, |a, x: TS| a + x.n);
    let mut out = Vec::new();
    (&t).cross(&total).drive(|(k, ()), (t, total)| {
        out.push(row(vec![
            V::S(k),
            V::I(t.n),
            avg(t.view_sum, t.view_n),
            avg(t.score_sum, t.n),
            if total == 0 { V::Null } else { V::F(t.n as f64 / total as f64 * 100.0) },
        ]))
    });
    rows(out)
}

/// `ROW_NUMBER() OVER (ORDER BY TotalPosts DESC, PostType)` — see
/// `rewrites/11478.sql`; the original ordering ties TagWiki with
/// TagWikiExcerpt at 857 and projects the rank.
fn q11478(db: &'static So) -> String {
    let t = type_stats(db);
    let rn = whole(&t)
        .select(Same::<Str>::new().and(&t))
        .window(row_number, |(k, t): (Str, TS)| (std::cmp::Reverse(t.n), k), asc);
    let mut out = Vec::new();
    (&rn).drive(|_, ((k, t), r)| {
        out.push(row(vec![
            V::S(k),
            V::I(t.n),
            V::I(t.owners),
            nullable(t.view_sum, t.view_n),
            avg(t.score_sum, t.n),
            avg(t.ans_sum, t.ans_n),
            avg(t.com_col_sum, t.n),
            V::I(r),
        ]))
    });
    rows(out)
}

/// `RANK() OVER (ORDER BY TotalViews DESC)` — DuckDB sorts NULLs last on DESC.
fn q14535(db: &'static So) -> String {
    let t = type_stats(db);
    let rk = whole(&t)
        .select(Same::<Str>::new().and(&t))
        .window(rank, |(_, t): (Str, TS)| (t.view_n == 0, std::cmp::Reverse(t.view_sum)), asc);
    let mut out = Vec::new();
    (&rk).drive(|_, ((k, t), r)| {
        out.push(row(vec![
            V::S(k),
            V::I(t.n),
            avg(t.score_sum, t.n),
            nullable(t.view_sum, t.view_n),
            avg(t.ans_sum, t.n),
            avg(t.com_col_sum, t.n),
            V::I(r),
        ]))
    });
    rows(out)
}

fn q14985(db: &'static So) -> String {
    let t = type_stats(db);
    let mut out = Vec::new();
    (&t).drive(|k, t| {
        out.push(row(vec![
            V::S(k),
            V::I(t.n),
            avg(t.score_sum, t.n),
            nullable(t.votes, t.votes_n),
            // TotalVotes / TotalPosts, and a NULL numerator stays NULL.
            if t.votes_n == 0 { V::Null } else { V::F(t.votes as f64 / t.n as f64) },
        ]))
    });
    rows(out)
}

/// `LEFT JOIN (SELECT PostId, COUNT(*) ... GROUP BY PostId)` — joining to an
/// already-grouped subquery gives one row per post, so nothing multiplies.
fn comment_rollup(db: &'static So) -> String {
    let t = type_stats(db);
    let mut out = Vec::new();
    (&t).drive(|k, t| out.push(row(vec![V::S(k), V::I(t.n), avg(t.score_sum, t.n), V::I(t.comments)])));
    rows(out)
}

/// The same roll-up, but `LEFT JOIN Comments` un-grouped, so every post is
/// multiplied by its comments and COUNT / SUM / AVG are over those rows.
fn q13029(db: &'static So) -> String {
    let Post { post_type, score, owner_user_id, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let vc = votes_per_post(db);

    let main = db
        .post
        .group_by(post_type.select(name))
        .select(score.and(&vc).and(comments_of(db).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(rows_, pos, ss, votes, coms), ((sc, vo), c)| {
            (rows_ + 1, pos + (sc > 0) as i64, ss + sc, votes + vo, coms + c.is_some() as i64)
        });
    let ow = db.post.group_by(post_type.select(name)).select(owner_user_id).count_distinct();

    let mut out = Vec::new();
    (&main).and((&ow).opt()).drive(|k, ((r, p, s, vo, c), o)| {
        out.push(row(vec![V::S(k), V::I(r), V::I(p), avg(s, r), V::I(o.unwrap_or(0)), V::I(vo), V::I(c)]))
    });
    rows(out)
}

fn q10286(db: &'static So) -> String {
    let Post { owner_user, score, post_type_id, view_count, .. } = &db.post;
    let User { origid, display_name, reputation, creation_date, .. } = &db.user;
    let nu = db.user.id.n;

    let sc = owner_user
        .inv()
        .select(score.and(post_type_id))
        .dense_fold_outer(nu, (0i64, 0i64, 0i64, 0i64), |(n, s, q, a), (x, t)| {
            (n + 1, s + x, q + (t == 1) as i64, a + (t == 2) as i64)
        });
    let vc = owner_user
        .inv()
        .select(view_count)
        .dense_fold_outer(nu, (0i64, 0i64), |(c, s), x| (c + 1, s + x));

    let avg_rep = whole(&db.user.id).select(reputation).fold((0i128, 0i64), |(s, n), r| (s + r as i128, n + 1));

    let mut out = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(creation_date).and(sc).and(vc))
        .cross(&avg_rep)
        .drive(|_, ((((((id, dn), rep), cd), (n, s, q, a)), (vn, vs)), (rs, rn))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(rep),
                V::I(n),
                V::I(q),
                V::I(a),
                nullable(s, n),
                nullable(vs, vn),
                V::F(rs as f64 / rn as f64),
                V::T(cd),
            ]))
        });
    rows(out)
}

/// `SUM(u.Reputation)` is over the JOINED rows, so a user's reputation is
/// counted once per post — and once when they have none.
fn q13710(db: &'static So) -> String {
    let User { origid, reputation, .. } = &db.user;

    let vpp = (&db.vote.post).inv().fold(0i64, |a, _| a + 1);
    let agg = db.user.select(reputation.and(posts_of(db).select(vpp.opt()).opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64),
        |(n, vn, vs, tr), (rep, p)| {
            let votes = p.flatten();
            (n + p.is_some() as i64, vn + votes.is_some() as i64, vs + votes.unwrap_or(0), tr + rep)
        },
    );

    let mut v: Vec<(i64, i64, i64, i64, i64)> = Vec::new();
    db.user
        .select(origid.and(agg))
        .drive(|_, (id, (n, vn, vs, tr))| v.push((id, n, vn, vs, tr)));

    v.sort_by(|a, b| b.4.cmp(&a.4));
    rows(v.iter().take(10).map(|(id, n, vn, vs, tr)| {
        row(vec![
            V::I(*id),
            V::I(*n),
            if *vn == 0 { V::F(0.0) } else { V::F(*vs as f64 / *vn as f64) },
            V::I(*tr),
        ])
    }))
}

fn q10622(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let User { reputation, .. } = &db.user;
    let nu = db.user.id.n;

    let pc = owner_user.inv().dense_fold_outer(nu, 0i64, |a, _| a + 1);
    let cm = (&db.comment.user).inv().dense_fold_outer(nu, 0i64, |a, _| a + 1);

    let g = db.user.group_by(reputation).select(pc.and(cm)).fold((0i64, 0i64), |(a, b), (x, y)| (a + x, b + y));
    let avg_rep = whole(&g).fold((0i128, 0i64), |(s, n), r| (s + r as i128, n + 1));

    let mut out = Vec::new();
    (&g).cross(&avg_rep).drive(|(rep, ()), ((p, c), (rs, rn))| {
        out.push(row(vec![V::I(rep), V::I(p), V::I(c), V::F(rs as f64 / rn as f64)]))
    });
    rows(out)
}

// ---- Per post -----------------------------------------------------------

/// `PostHistory JOIN Posts LEFT JOIN Comments LEFT JOIN Votes` grouped by
/// PostId: the three children form a cross product, so COUNT(*) is
/// revisions x comments x votes. `STRING_AGG(DISTINCT p.Title, ', ')` is
/// degenerate here — the group is one post, so there is one title.
fn q10975(db: &'static So) -> String {
    let Post { origid, title: pt, .. } = &db.post;
    let cc = comments_per_post(db);
    let vc = votes_per_post(db);
    let stats = db
        .post
        .with(history_of(db))
        .group_by(Ident::<Post>::new())
        .select(
            history_of(db)
                .select(&db.post_history.creation_date)
                .and(comments_of(db).opt())
                .and(votes_of(db).select(&db.vote.vote_type_id).opt()),
        )
        .fold((0i64, i64::MAX, i64::MIN, 0i64, 0i64), |(n, lo, hi, u, d), ((h, _), vt)| {
            (n + 1, lo.min(h), hi.max(h), u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64)
        });

    let mut out = Vec::new();
    (&stats).and(origid).and(&cc).and(&vc).drive(|pid, ((((n, lo, hi, u, d), id), c), vo)| {
        out.push(row(vec![
            V::I(id),
            V::I(n),
            V::T(lo),
            V::T(hi),
            ostr(pt.get(pid)),
            V::I(c),
            V::I(vo),
            V::I(u),
            V::I(d),
        ]))
    });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, U.Reputation,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT PostId, Title, CreationDate, Score, ViewCount, Reputation, Rank FROM RankedPosts WHERE Rank <= 10 ORDER BY Rank;
fn q10460(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let rn = db
        .post
        .with(creation_date.ge(since))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (std::cmp::Reverse(d), p), asc);
    let mut out = Vec::new();
    (&rn).filt(|(_, r)| r <= 10).drive(|_, ((p, _), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.push(V::I(r));
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("10622", q10622),
    ("13574", q13574),
    ("11478", q11478),
    ("10286", q10286),
    ("10460", q10460),
    ("13029", q13029),
    ("13710", q13710),
    ("13719", comment_rollup),
    ("13287", comment_rollup),
    ("10975", q10975),
    ("14535", q14535),
    ("14985", q14985),
];
