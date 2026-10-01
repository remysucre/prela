use harness::prelude::*;

// Every query here is the same shape: a CTE of per-user aggregates, then
// `ROW_NUMBER()`/`RANK() OVER (ORDER BY ...)` with `WHERE Rank <= 10`, a
// window over all users with `whole` as the single partition.

/// Per user over `Users LEFT JOIN Posts`: [posts, score sum, questions,
/// answers, view sum with NULLs as 0].
fn user_stats(db: &'static So) -> DenseFold<Id<User>, [i64; 5]> {
    let Post { owner_user, score, post_type_id, view_count, .. } = &db.post;
    owner_user.inv().select(score.and(post_type_id).and(view_count.opt())).dense_fold_outer(
        db.user.id.n,
        [0i64; 5],
        |a, ((sc, t), v)| [a[0] + 1, a[1] + sc, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + v.unwrap_or(0)],
    )
}

fn q13692(db: &'static So) -> String {
    let us = user_stats(db);
    let rn = whole(&db.user.id)
        .select(Ident::<User>::new().and(&us))
        .window(row_number, |(_, a): (Id<User>, [i64; 5])| a[1], desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, ((u, a), n)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[1]), V::I(n)]);
        out.push(row(f))
    });
    rows(out)
}

fn q12949(db: &'static So) -> String {
    let us = user_stats(db);
    let rn = whole(&db.user.id)
        .select(Ident::<User>::new().and(&us))
        .window(row_number, |(_, a): (Id<User>, [i64; 5])| a[1], desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, ((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(a[1]), V::I(a[4])]);
        out.push(row(f))
    });
    rows(out)
}

fn q13846(db: &'static So) -> String {
    let us = user_stats(db);
    let rn = whole(&db.user.id)
        .select(Ident::<User>::new().and(&us))
        .window(row_number, |(_, a): (Id<User>, [i64; 5])| a[1], desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, ((u, a), n)| {
        let mut f = vec![V::I(n)];
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[4]), V::I(a[2]), V::I(a[3])]);
        out.push(row(f))
    });
    rows(out)
}

fn q12563(db: &'static So) -> String {
    let us = user_stats(db);
    let rk = whole(&db.user.id)
        .select(Ident::<User>::new().and(&us))
        .window(rank, |(_, a): (Id<User>, [i64; 5])| a[1], desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 10).drive(|_, ((u, a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[1]), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q11199(db: &'static So) -> String {
    let Post { owner_user, post_type_id, accepted_answer_id, .. } = &db.post;
    let us = user_stats(db);
    let accepted = owner_user
        .inv()
        .with(post_type_id.eq(2))
        .with(accepted_answer_id)
        .dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let rn = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(row_number, |(_, r): (Id<User>, i64)| r, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .select(Ident::<User>::new().and(&us).and(&accepted))
        .drive(|_, ((u, a), acc)| {
            let mut f = ucols(db, u, &["uid", "rep"]);
            f.extend([V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(acc)]);
            out.push(row(f))
        });
    rows(out)
}

// ---- ... plus a second LEFT JOIN, which multiplies the rows -------------

fn badge_rows(db: &'static So) -> DenseFold<Id<User>, (i64, i64, i64, i64, i64, i64)> {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    db.user
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt().and(badges_of(db).opt()))
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(q, a, s, vn, vs, b), (p, bi)| {
            let t = p.map(|((t, _), _)| t);
            let w = p.and_then(|(_, w)| w);
            (
                q + (t == Some(1)) as i64,
                a + (t == Some(2)) as i64,
                s + p.map_or(0, |((_, sc), _)| sc),
                vn + w.is_some() as i64,
                vs + w.unwrap_or(0),
                b + bi.is_some() as i64,
            )
        })
}

fn q12517(db: &'static So) -> String {
    let us = user_stats(db);
    let br = badge_rows(db);
    let rn = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(row_number, |(_, r): (Id<User>, i64)| r, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .select(Ident::<User>::new().and(&us).and(&br))
        .drive(|_, ((u, a), (q, an, _, _, _, b))| {
            let mut f = ucols(db, u, &["uid", "rep"]);
            f.extend([V::I(a[0]), V::I(an), V::I(q), V::I(b)]);
            out.push(row(f))
        });
    rows(out)
}

/// `ORDER BY TotalPosts DESC, TotalScore DESC`: TotalScore is the SUM over
/// the joined rows, NULL (last) for a user with no posts.
fn q9585(db: &'static So) -> String {
    let us = user_stats(db);
    let br = badge_rows(db);
    let rn = whole(&db.user.id)
        .select(Ident::<User>::new().and(&us).and(&br))
        .window(
            row_number,
            |((_, a), b): ((Id<User>, [i64; 5]), (i64, i64, i64, i64, i64, i64))| (a[0], (a[0] > 0).then_some(b.2)),
            desc,
        );
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((u, a), (q, an, s, vn, vs, b)), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(q), V::I(an), nullable(s, a[0]), nullable(vs, vn), V::I(b)]);
        out.push(row(f))
    });
    rows(out)
}

fn q12087(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let nu = db.user.id.n;

    let posts = owner_user.inv().dense_fold_outer(nu, 0i64, |a, _| a + 1);
    let w = owner_user
        .inv()
        .select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .dense_fold_outer(nu, (0i64, 0i64, 0i64, 0i64, 0i64), |(q, a, vw, u, d), ((t, x), vt)| {
            (
                q + (t == 1) as i64,
                a + (t == 2) as i64,
                vw + x.unwrap_or(0),
                u + (vt == Some(2)) as i64,
                d + (vt == Some(3)) as i64,
            )
        });
    let rn = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation).and(&posts).and(&w))
        .window(row_number, |(((_, r), _), _): (((Id<User>, i64), i64), (i64, i64, i64, i64, i64))| r, desc);

    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, ((((u, _), p), (q, a, vi, up, d)), n)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(p), V::I(q), V::I(a), V::I(vi), V::I(up), V::I(d), V::I(n)]);
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("13692", q13692),
    ("11199", q11199),
    ("12517", q12517),
    ("9585", q9585),
    ("12949", q12949),
    ("12087", q12087),
    ("13846", q13846),
    ("12563", q12563),
];
