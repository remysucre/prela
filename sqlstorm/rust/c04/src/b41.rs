use harness::prelude::*;

fn q14723(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1));
    let counts = (&base)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()))
        .fold((0i64, 0i64), |(c, a), (ci, ai)| (c + ci.is_some() as i64, a + ai.is_some() as i64));
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(score).and(creation_date).and(&counts))
        .window(row_number, |(((_, s), cd), _)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 100).drive(|_, ((((p, _), _), (c, a)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "activity", "views"]);
        f.extend([V::I(c), V::I(a), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q14780(db: &'static So) -> String {
    let Post { creation_date, view_count, score, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2023, 1, 1)));
    let counts = (&base)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, 0i64, None), |(c, v, last): (i64, i64, Option<i64>), ((ci, vi), h)| {
            (c + ci.is_some() as i64, v + vi.is_some() as i64, last.max(h))
        });
    let ranked = whole(&base)
        .select(Ident::<Post>::new().and(view_count.opt()).and(score).and(&counts))
        .window(dense_rank, |(((_, v), _), _)| v, desc)
        .window(dense_rank, |((((_, _), s), _), _)| s, desc);
    let mut out = Vec::new();
    ranked.drive(|_, (((((p, _), _), (c, v, last)), a), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(v), ots(last), V::I(a), V::I(b)]);
        out.push(row(f))
    });
    rows(out)
}

fn q9093(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ranked = (&base)
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .window(row_number, |(s, _)| s, desc)
        .window(row_number, |((_, v), _)| v, desc);
    let agg = (&db.user.display_name)
        .inv()
        .select(&ranked)
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(ts, tv, s, vn, vs), (((sc, v), a), b)| {
            (ts + (a <= 5) as i64, tv + (b <= 5) as i64, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        });
    let mut out = Vec::new();
    (&agg).filt(|(_, _, s, _, _)| s > 10).drive(|k, (ts, tv, s, vn, vs)| out.push(row(vec![V::S(k), V::I(ts), V::I(tv), V::I(s), nullable(vs, vn)])));
    rows(out)
}

fn q14349(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let cc = comments_per_post(db);
    let base = db.post.with(post_type_id.eq(1));
    let rn = whole(&base).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 100)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(&cc))
        .drive(|_, (p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(c));
        out.push(row(f))
    });
    rows(out)
}

fn user_totals(db: &'static So) -> impl Probe<D = Id<User>, R = (i64, i64, i64, i64, i64, i64)> {
    let Post { owner_user, view_count, score, .. } = &db.post;
    let nu = db.user.id.n;
    let posts = owner_user.inv().dense_fold_outer(nu, 0i64, |a, _| a + 1);
    let comments = owner_user.inv().select(comments_of(db)).dense_fold_outer(nu, 0i64, |a, _| a + 1);
    let votes = owner_user.inv().select(votes_of(db)).dense_fold_outer(nu, 0i64, |a, _| a + 1);
    let joined = owner_user
        .inv()
        .select(view_count.opt().and(score).and(comments_of(db).opt()).and(votes_of(db).opt()))
        .dense_fold_outer(nu, (0i64, 0i64, 0i64), |(vn, vs, s), (((w, sc), _), _)| {
            (vn + w.is_some() as i64, vs + w.unwrap_or(0), s + sc)
        });
    posts.and(comments).and(votes).and(joined).map(|(((n, c), v), (vn, vs, s))| (n, c, v, vn, vs, s))
}

fn user_score_rank(db: &'static So) -> String {
    let t = user_totals(db);
    let rk = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t))
        .window(rank, |(_, a)| (a.0 > 0).then_some(a.5), desc);
    let mut out = Vec::new();
    (&rk).drive(
        |_, (((id, dn), (n, c, v, vn, vs, s)), r)| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                V::I(c),
                V::I(v),
                nullable(vs, vn),
                nullable(s, n),
                V::I(r),
            ]))
        },
    );
    rows(out)
}

fn q11499(db: &'static So) -> String {
    user_score_rank(db)
}

fn q14554(db: &'static So) -> String {
    user_score_rank(db)
}

fn q12048(db: &'static So) -> String {
    let t = user_totals(db);
    let ranked = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t))
        .window(rank, |(_, t)| t.0, desc)
        .window(rank, |((_, t), _)| t.1, desc)
        .window(rank, |(((_, t), _), _)| t.2, desc)
        .window(rank, |((((_, t), _), _), _)| (t.3 > 0).then_some(t.4), desc);
    let mut out = Vec::new();
    ranked
        .drive(|_, ((((((id, dn), (n, c, v, vn, vs, _)), a), b), d), e)| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                V::I(c),
                V::I(v),
                nullable(vs, vn),
                V::I(a),
                V::I(b),
                V::I(d),
                V::I(e),
            ]))
        });
    rows(out)
}

fn type_stats(db: &'static So) -> Fold<Str, (i64, i64, i64, i64, i64)> {
    let Post { post_type, score, view_count, .. } = &db.post;
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, pos, vn, vs), (sc, v)| {
            (n + 1, s + sc, pos + (sc > 0) as i64, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        })
}

/// `SUM(TotalPosts) OVER ()` is one row over the grouped result, crossed with
/// each group.
fn q11469(db: &'static So) -> String {
    let stats = type_stats(db);
    let authors = db
        .post
        .group_by((&db.post.post_type).select(&db.post_type.name))
        .select(&db.post.owner_user_id)
        .count_distinct();
    let total = whole(&stats).select(&stats).fold(0i64, |a, s: (i64, i64, i64, i64, i64)| a + s.0);
    let mut out = Vec::new();
    (&stats).and((&authors).opt()).cross(&total).drive(|(k, ()), (((n, s, pos, vn, vs), au), total)| {
        out.push(row(vec![
            V::S(k),
            V::I(n),
            V::I(pos),
            avg(s, n),
            nullable(vs, vn),
            avg(vs, n),
            V::I(au.unwrap_or(0)),
            if total == 0 { V::Null } else { V::F(n as f64 / total as f64 * 100.0) },
            V::F(pos as f64 / n as f64 * 100.0),
        ]))
    });
    rows(out)
}

fn q14162(db: &'static So) -> String {
    let stats = type_stats(db);
    let rk = whole(&db.user.id)
        .select((&db.user.display_name).and(&db.user.reputation))
        .window(rank, |(_, rep)| rep, desc);
    let tu: HashIdx<(), (Str, i64)> = (&rk).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let mut out = Vec::new();
    (&stats).cross(&tu).drive(|(k, ()), ((n, s, _, vn, vs), (dn, rep))| {
        out.push(row(vec![V::S(k), V::I(n), nullable(vs, vn), avg(s, n), V::S(dn), V::I(rep)]))
    });
    rows(out)
}

/// `COUNT(*) OVER ()` over TopPosts: a fold of the ranked rows, keyed by the
/// window's single partition, so it pairs with each row by `.and`.
fn q14525(db: &'static So) -> String {
    let Post { score, creation_date, .. } = &db.post;
    let rk = whole(owned(db))
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), cd)| (s, cd), desc);
    let top = (&rk).filt(|(_, r)| r <= 100);
    let n = (&top).fold(0i64, |a, _| a + 1);
    let mut out = Vec::new();
    (&top).and(&n).drive(|_, ((((p, _), _), _), n)| {
        let mut f = post_fields(
            db,
            p,
            &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner"],
        );
        f.push(V::I(n));
        out.push(row(f))
    });
    rows(out)
}

fn q12158(db: &'static So) -> String {
    let rn = whole(db.post.iq())
        .select(Ident::<Post>::new().and(&db.post.creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let name = (&db.post.owner_user).select(&db.user.display_name);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 100).map(|((p, _), _)| p))
        .and(name.opt())
        .drive(|p, ((c, v, _, _), dn)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::S(dn.unwrap_or("Deleted User")), V::I(c), V::I(v)]);
        out.push(row(f))
    });
    rows(out)
}

fn q6625(db: &'static So) -> String {
    let Post { owner_user, post_type_id, creation_date, score, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(date(2022, 1, 1)));
    let rk = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(dense_rank, |(_, s)| s, desc);
    let mx = (&db.user.display_name).inv().select(&rk).fold(0i64, |a, (_, r)| a.max(r));
    let mut out = Vec::new();
    (&rk)
        .and((&db.user.display_name).select(&mx))
        .filt(|((_, r), m)| r == m)
        .drive(|_, (((p, _), _), _)| {
        out.push(row(post_fields(db, p, &["title", "created", "score", "views", "owner"])))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("14723", q14723),
    ("14780", q14780),
    ("9093", q9093),
    ("14349", q14349),
    ("11499", q11499),
    ("14554", q14554),
    ("11469", q11469),
    ("14162", q14162),
    ("14525", q14525),
    ("12158", q12158),
    ("6625", q6625),
    ("12048", q12048),
];
