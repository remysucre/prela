use harness::prelude::*;

fn q14954(db: &'static So) -> String {
    let Post { post_type, post_type_id, score, view_count, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut base: Vec<(Str, i64, i64, [i64; 5], i64, i64)> = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(score.and(post_type_id).and(view_count.opt()))
        .fold(
            (0i64, 0i64, [0i64; 5], 0i64, 0i64),
            |(n, s, mut k, vn, vs), ((x, ty), v)| {
                for (i, want) in [1, 2, 3, 10, 11].iter().enumerate() {
                    k[i] += (ty == *want) as i64;
                }
                (n + 1, s + x, k, vn + v.is_some() as i64, vs + v.unwrap_or(0))
            },
        )
        .drive(|key, (n, s, k, vn, vs)| base.push((key, n, s, k, vn, vs)));

    base.sort_by(|a, b| b.1.cmp(&a.1));
    rows(base.iter().map(|(key, n, s, k, vn, vs)| {
        row(vec![
            V::S(key),
            V::I(*n),
            avg(*s, *n),
            avg(*vs, *vn),
            V::I(k[0]),
            V::I(k[1]),
            V::I(k[2]),
            V::I(k[3]),
            V::I(k[4]),
        ])
    }))
}

fn q14220(db: &'static So) -> String {
    let PostHistory { post_id, post, post_history_type_id, creation_date, .. } =
        &db.post_history;
    let Post { creation_date: pcd, .. } = &db.post;

    let mut all: Vec<(i64, i64, i64, i64, [i64; 3])> = Vec::new();
    db.post_history
        .with(post.select(pcd).ge(date(2022, 1, 1)))
        .group_by(post_id)
        .select(creation_date.and(post_history_type_id))
        .fold(
            (0i64, i64::MAX, i64::MIN, [0i64; 3]),
            |(n, lo, hi, mut k), (cd, ty)| {
                k[0] += (ty == 10 || ty == 11) as i64;
                k[1] += (ty == 12 || ty == 13) as i64;
                k[2] += (ty == 24) as i64;
                (n + 1, lo.min(cd), hi.max(cd), k)
            },
        )
        .drive(|p, (n, lo, hi, k)| all.push((p, n, lo, hi, k)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().map(|(p, n, lo, hi, k)| {
        row(vec![
            V::I(*p),
            V::I(*n),
            V::T(*lo),
            V::T(*hi),
            V::Iv(hi - lo),
            V::I(k[0]),
            V::I(k[1]),
            V::I(k[2]),
        ])
    }))
}

fn q16078(db: &'static So) -> String {
    let Badge { user, origid, .. } = &db.badge;
    let User { display_name, .. } = &db.user;

    let mut all: Vec<(Str, i64)> = Vec::new();
    db.badge
        .group_by(user.select(display_name))
        .select(origid)
        .fold(0i64, |a, _| a + 1)
        .drive(|k, n| all.push((k, n)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(dn, n)| row(vec![V::S(dn), V::I(*n)])))
}

fn posts_per_name(db: &'static So) -> Vec<(Str, i64)> {
    let Post { owner_user, origid, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut all: Vec<(Str, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user.select(display_name))
        .select(origid)
        .fold(0i64, |a, _| a + 1)
        .drive(|k, n| all.push((k, n)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    all
}

fn q17876(db: &'static So) -> String {
    rows(posts_per_name(db).iter().take(10).map(|(dn, n)| {
        row(vec![V::S(dn), V::I(*n)])
    }))
}

fn q16034(db: &'static So) -> String {
    let Post { owner_user, origid, .. } = &db.post;
    let User { origid: uid, display_name, .. } = &db.user;

    let mut all: Vec<(Id<User>, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user)
        .select(origid)
        .fold(0i64, |a, _| a + 1)
        .drive(|u, n| all.push((u, n)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(u, n)| {
        row(vec![
            V::I(uid.get(*u).unwrap()),
            V::S(display_name.get(*u).unwrap()),
            V::I(*n),
        ])
    }))
}

// SELECT U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(P.ViewCount) AS TotalViews FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.DisplayName ORDER BY TotalPosts DESC LIMIT 10;
fn q15310(db: &'static So) -> String {
    let g = db
        .user
        .group_by(&db.user.display_name)
        .select(posts_of(db).select((&db.post.view_count).opt()).opt())
        .fold([0i64; 3], |a, p| match p {
            Some(w) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)],
            None => a,
        });
    let v = top_n(drain(&g), |&(_, a)| std::cmp::Reverse(a[0]), 10);
    rows(v.into_iter().map(|(dn, a)| row(vec![V::S(dn), V::I(a[0]), nullable(a[2], a[1])])))
}

/// `COUNT(c.Id)` per post under a LEFT JOIN: the forward edge inverted, then
/// an outer fold so posts with no comments come back 0 rather than absent.
fn comments_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.comment.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

fn q19184(db: &'static So) -> String {
    let Post { origid, creation_date, title: pt, .. } = &db.post;

    let cc = comments_per_post(db);
    let mut v: Vec<(Id<Post>, i64, i64, i64)> = Vec::new();
    db.post
        .select(origid.and(creation_date).and(&cc))
        .drive(|p, ((id, cd), n)| v.push((p, id, cd, n)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(p, id, cd, n)| {
        row(vec![V::I(*id), ostr(pt.get(*p)), V::T(*cd), V::I(*n)])
    }))
}

/// The Votes side of `Posts LEFT JOIN Votes`, as a relation keyed by post:
/// the forward edge inverted and bucketed. `HashIdx` probes multi-valued, so
/// a post with three votes yields three rows and one with none yields zero.
fn votes_per_post(db: &'static So) -> HashIdx<Id<Post>, i64> {
    let Vote { post, vote_type_id, .. } = &db.vote;
    post.inv().select(vote_type_id).collect()
}

// Questions JOIN Users LEFT JOIN Votes, one row per (question, vote) and one with no vote, newest ten rows.
fn newest_questions_with_votes(db: &'static So) -> Vec<(Id<Post>, i64, i64, Str, Option<i64>)> {
    let Post { origid, creation_date, post_type_id, owner_user, .. } = &db.post;
    let vp = votes_per_post(db);
    let v = drain(db.post.with(post_type_id.eq(1)).select(origid.and(creation_date).and(owner_user.select(&db.user.display_name)).and((&vp).opt())));
    let v = top_n(v, |&(_, (((_, cd), _), _))| std::cmp::Reverse(cd), 10);
    v.into_iter().map(|(p, (((id, cd), dn), t))| (p, id, cd, dn, t)).collect()
}

fn q18004(db: &'static So) -> String {
    let pt = &db.post.title;
    rows(newest_questions_with_votes(db).iter().map(|(p, _, cd, dn, vt)| {
        row(vec![V::S(dn), ostr(pt.get(*p)), V::T(*cd), oint(*vt)])
    }))
}

fn q16957(db: &'static So) -> String {
    let pt = &db.post.title;
    rows(newest_questions_with_votes(db).iter().map(|(p, id, cd, dn, vt)| {
        row(vec![V::I(*id), ostr(pt.get(*p)), V::T(*cd), V::S(dn), oint(*vt)])
    }))
}

fn q16665(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let Vote { bounty_amount, .. } = &db.vote;
    let User { display_name, .. } = &db.user;

    let mut per_user: Vec<(Str, i64, i64, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user.select(display_name))
        .select(votes_of(db).select(bounty_amount.opt()).opt())
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), b| {
            let b = b.flatten();
            (c + 1, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        })
        .drive(|dn, (c, bn, bs)| per_user.push((dn, c, bn, bs)));

    per_user.sort_by(|a, b| b.1.cmp(&a.1));
    rows(per_user.iter().take(10).map(|(dn, c, bn, bs)| {
        row(vec![V::S(dn), V::I(*c), nullable(*bs, *bn)])
    }))
}

// SELECT DATE_TRUNC('month', CreationDate) AS Month, COUNT(*) AS TotalPosts, AVG(Score) AS AverageScore, COUNT(DISTINCT OwnerUserId) AS UniqueUsers
// FROM Posts GROUP BY Month ORDER BY Month;
fn q12553(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, .. } = &db.post;
    let base = db.post.group_by(creation_date.map(trunc_month)).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let du = db.post.group_by(creation_date.map(trunc_month)).select(owner_user_id).count_distinct();
    let v = drain((&base).and((&du).opt()));
    rows(v.into_iter().map(|(m, (a, d))| row(vec![V::T(m), V::I(a[0]), avg(a[1], a[0]), V::I(d.unwrap_or(0))])))
}

fn q13453(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, .. } = &db.post;
    let cut = ts(2023, 10, 1, 12, 34, 56);

    let (n, s) = db
        .post
        .with(creation_date.ge(cut))
        .select(score)
        .fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    // COUNT(DISTINCT ...) with no GROUP BY: one group, keyed by a constant.
    let mut d = 0i64;
    db.post
        .with(creation_date.ge(cut))
        .group_by(creation_date.map(|_| 0i64))
        .select(owner_user_id)
        .count_distinct()
        .drive(|_, c| d = c);

    row(vec![V::I(n), avg(s, n), V::I(d)])
}

pub const ENTRIES: &[harness::Entry] = &[
    ("14954", q14954),
    ("14220", q14220),
    ("16078", q16078),
    ("17876", q17876),
    ("16034", q16034),
    ("19184", q19184),
    ("18004", q18004),
    ("16957", q16957),
    ("12553", q12553),
    ("15310", q15310),
    ("13453", q13453),
    ("16665", q16665),
];
