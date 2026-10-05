use harness::prelude::*;

fn q13677(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let comments = owner_user.inv().select(comments_of(db)).dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let t = owner_user
        .inv()
        .select(post_type_id.and(view_count.opt()).and(score).and(comments_of(db).opt()))
        .dense_fold_outer(
            db.user.id.n,
            (0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
            |(n, q, a, vn, vs, s), (((ty, w), sc), _)| {
                (n + 1, q + (ty == 1) as i64, a + (ty == 2) as i64, vn + w.is_some() as i64, vs + w.unwrap_or(0), s + sc)
            },
        );
    let users = db.user.with((&db.user.reputation).gt(0));
    let ranked = whole(&users)
        .select((&db.user.origid).and(&db.user.display_name).and(&t).and(&comments))
        .window(rank, |((_, t), _)| (t.0 > 0).then_some(t.5), desc)
        .window(rank, |(((_, t), _), _)| (t.3 > 0).then_some(t.4), desc);
    let mut out = Vec::new();
    ranked.drive(
        |_, (((((id, dn), (n, q, a, vn, vs, s)), c), x), y)| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                V::I(q),
                V::I(a),
                nullable(vs, vn),
                nullable(s, n),
                V::I(c),
                V::I(x),
                V::I(y),
            ]))
        },
    );
    rows(out)
}

fn q5469(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user_id, .. } = &db.post;
    let Vote { vote_type_id, post, user_id, .. } = &db.vote;
    let voters = db.vote.with(vote_type_id.is_in([2, 3])).group_by(post).select(user_id).count_distinct();
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let bc = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let base = db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let comments = (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(
            comments_of(db)
                .opt()
                .and(votes_of(db).with(vote_type_id.is_in([2, 3])).opt())
                .and(owner_user_id.select(&bidx).opt()),
        )
        .fold(0i64, |c, ((ci, _), _)| c + ci.is_some() as i64);
    let mut out = Vec::new();
    (&comments).and((&voters).opt()).and(owner_user_id.select(&bc).opt()).drive(|p, ((c, nv), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(nv.unwrap_or(0)), V::I(b.unwrap_or(0))]);
        out.push(row(f))
    });
    rows(out)
}

fn q7311(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let User { display_name, reputation, creation_date: ucd, .. } = &db.user;
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rk = (&base).group_by(owner_user).select(creation_date).window(rank, |cd| cd, desc);
    let top = display_name.inv().select((&rk).filt(|(_, r)| r <= 5)).fold(0i64, |a, _| a + 1);
    let bc = badges_per_user(db);
    let mut out = Vec::new();
    db.user.with(reputation.gt(1000)).select(display_name.and(reputation).and(ucd).and(display_name.select(&top)).and(&bc)).drive(
        |_, ((((dn, rep), cd), n), b)| out.push(row(vec![V::S(dn), V::I(rep), V::T(cd), V::I(n), V::I(b)])),
    );
    rows(out)
}

fn q11500(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let t = owner_user
        .inv()
        .select(
            post_type_id
                .and(votes_of(db).select(&db.vote.vote_type_id).opt())
                .and(comments_of(db).select(&db.comment.score).opt()),
        )
        .dense_fold_outer(
            db.user.id.n,
            (0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
            |(n, q, a, u, d, s), ((ty, vt), cs)| {
                (
                    n + 1,
                    q + (ty == 1) as i64,
                    a + (ty == 2) as i64,
                    u + (vt == Some(2)) as i64,
                    d + (vt == Some(3)) as i64,
                    s + cs.unwrap_or(0),
                )
            },
        );
    let users = db.user.with((&db.user.reputation).gt(0));
    let rk = whole(&users)
        .select((&db.user.origid).and(&db.user.display_name).and(&t).and(&db.user.reputation))
        .window(rank, |(_, rep)| rep, desc);
    let mut v = Vec::new();
    (&rk).drive(|_, ((x, _), r)| v.push((x, r)));
    v.sort_by(|a, b| (b.0.1.0, b.0.1.3).cmp(&(a.0.1.0, a.0.1.3)));
    rows(v.iter().take(100).map(|&(((id, dn), (n, q, a, u, d, s)), r)| {
        row(vec![V::I(id), V::S(dn), V::I(n), V::I(q), V::I(a), V::I(u), V::I(d), V::I(s), V::I(r)])
    }))
}

fn q14451(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let cc = comments_per_post(db);
    let vc = votes_per_post(db);
    let base = owned(db).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ranked = whole(&base)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()).and(&cc).and(&vc))
        .window(rank, |((((_, s), v), _), _)| (s, v), desc)
        .window(row_number, |(((((p, s), v), _), _), _)| (s, v, std::cmp::Reverse(p)), desc);
    let mut out = Vec::new();
    ranked.filt(|(_, n)| n <= 100).drive(|_, ((((((p, _), _), c), v), r), _)| {
        let mut f = post_fields(
            db,
            p,
            &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "rep", "owner"],
        );
        f.extend([V::I(c), V::I(v), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q5822(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = (&base)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 5).map(|((p, _), _)| p)).drive(|p, (c, _, u, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q12779(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let t = owner_user
        .inv()
        .select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .dense_fold_outer(
            db.user.id.n,
            (0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
            |(n, q, a, w, u, d), ((ty, pw), vt)| {
                (
                    n + 1,
                    q + (ty == 1) as i64,
                    a + (ty == 2) as i64,
                    w + pw.unwrap_or(0),
                    u + (vt == Some(2)) as i64,
                    d + (vt == Some(3)) as i64,
                )
            },
        );
    let ranked = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t))
        .window(rank, |(_, t)| t.3, desc)
        .window(rank, |((_, t), _)| t.0, desc);
    let mut out = Vec::new();
    ranked.drive(
        |_, ((((id, dn), (n, q, a, w, u, d)), x), y)| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                V::I(q),
                V::I(a),
                V::I(w),
                V::I(u),
                V::I(d),
                V::I(u - d),
                V::I(x),
                V::I(y),
            ]))
        },
    );
    rows(out)
}

fn q5333(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(ts(2024, 10, 1, 12, 34, 56) - 30 * DAY_US));
    let key = engagement(db, &base).map(|(c, _, u, d)| (c, u - d));
    let rn = (&base).group_by(post_type).select(Ident::<Post>::new().and(&key)).window(row_number, |(_, k)| k, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, ((p, (c, s)), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(s)]);
        out.push(row(f))
    });
    rows(out)
}

fn q10871(db: &'static So) -> String {
    let Post { owner_user, post_type_id, creation_date, .. } = &db.post;
    let posts = owner_user.inv().select(creation_date).dense_fold_outer(db.user.id.n, (0i64, i64::MIN), |(n, last), cd| {
        (n + 1, last.max(cd))
    });
    let t = owner_user.inv().select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64),
        |(q, a, w, tv), (ty, vt)| {
            (q + (ty == 1) as i64, a + (ty == 2) as i64, w + (ty == 3) as i64, tv + matches!(vt, Some(2 | 3)) as i64)
        },
    );
    let rk = whole(&db.user.id)
        .select(
            (&db.user.origid)
                .and(&db.user.display_name)
                .and(&db.user.reputation)
                .and(&db.user.creation_date)
                .and((&posts).and(&t)),
        )
        .window(rank, |((((_, _), rep), _), _)| rep, desc);
    let mut v = Vec::new();
    (&rk).drive(|_, x| v.push(x));
    v.sort_by(|a, b| b.0.0.0.1.cmp(&a.0.0.0.1));
    rows(v.iter().take(100).map(|&(((((id, dn), rep), cd), ((n, last), (q, a, w, tv))), r)| {
        row(vec![
            V::I(id),
            V::S(dn),
            V::I(rep),
            V::T(cd),
            V::I(n),
            V::I(q),
            V::I(a),
            V::I(w),
            V::I(tv),
            if n == 0 { V::Null } else { V::T(last) },
            V::I(r),
        ])
    }))
}

fn recent_by_type(db: &'static So, cut: i64, n: i64, cols: &[&str], type_name: bool) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(cut));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, r)| r <= n).map(|((p, _), _)| p)).drive(|p, (c, _, u, d)| {
        let mut f = post_fields(db, p, cols);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        if type_name {
            f.extend(post_fields(db, p, &["type"]));
        }
        out.push(row(f))
    });
    rows(out)
}

fn q7706(db: &'static So) -> String {
    recent_by_type(db, ts(2024, 10, 1, 12, 34, 56) - 30 * DAY_US, 10, &["id", "title", "created", "owner"], true)
}

fn q8669(db: &'static So) -> String {
    recent_by_type(db, add_months(ts(2024, 10, 1, 12, 34, 56), -1), 5, &["id", "title", "created", "views", "score", "owner"], false)
}

fn q8203(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let vc = votes_per_post(db);
    let base = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(score.gt(0));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 5).map(|(((p, _), _), _)| p)).and(&vc).drive(|p, ((c, _, _, _), v)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.extend([V::I(c), V::I(v)]);
        f.extend(post_fields(db, p, &["type"]));
        out.push(row(f))
    });
    rows(out)
}

fn q8789(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, owner_user, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1)).with(creation_date.ge(date(2024, 9, 1)));
    let rn = (&base)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 5).map(|((p, _), _)| p).with(owner_user)).drive(|p, (c, _, u, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("13677", q13677),
    ("5469", q5469),
    ("7311", q7311),
    ("11500", q11500),
    ("14451", q14451),
    ("5822", q5822),
    ("12779", q12779),
    ("5333", q5333),
    ("10871", q10871),
    ("7706", q7706),
    ("8203", q8203),
    ("8789", q8789),
    ("8669", q8669),
];
