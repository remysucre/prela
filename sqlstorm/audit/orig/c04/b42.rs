use harness::prelude::*;

fn q10746(db: &'static So) -> String {
    let Post { owner_user, post_type, view_count, score, .. } = &db.post;
    let stats = db
        .post
        .with(owner_user)
        .group_by(post_type.and(owner_user))
        .select(post_type.and(owner_user.select(&db.user.reputation)).and(score.and(view_count.opt())))
        .fold((Id::new(0), 0i64, 0i64, 0i64, 0i64, 0i64), |(_, _, n, s, vn, vs), ((t, r), (sc, v))| {
            (t, r, n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        });
    let rn = (&stats).map(|a| a.0).inv().select(Same::new().and(&stats)).window(
        row_number,
        |(tu, a)| (a.1, tu),
        |x: &(i64, (Id<PostType>, Id<User>)), y: &(i64, (Id<PostType>, Id<User>))| y.0.cmp(&x.0).then(x.1.cmp(&y.1)),
    );
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n == 1).drive(|_, (((t, u), (_, rep, n, s, vn, vs)), _)| {
        out.push(row(vec![
            V::S(db.post_type.name.get(t).unwrap()),
            V::I(n),
            avg(vs, vn),
            avg(s, n),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(rep),
        ]))
    });
    rows(out)
}

fn q6392(db: &'static So) -> String {
    let rn = db
        .post
        .group_by(&db.post.post_type)
        .select(Ident::<Post>::new().and(&db.post.creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let vc = votes_per_post(db);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 5).map(|((p, _), _)| p)).and(&vc).drive(|p, ((c, _, _, _), v)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(v)]);
        out.push(row(f))
    });
    rows(out)
}

fn q26873(db: &'static So) -> String {
    let Post { owner_user, post_type_id, creation_date, tags_str, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let base = owned(db).with(post_type_id.eq(1));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(&cc).and(&up))
        .drive(|_, ((p, c), u)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.push(oint(tags_str.get(p).map(|t| t.matches("><").count() as i64 + 1)));
        f.extend([V::I(c), V::I(u)]);
        out.push(row(f))
    });
    rows(out)
}

fn q7237(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(ts(2024, 10, 1, 12, 34, 56) - 30 * DAY_US));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let stats = (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, None, None), |(c, lo, hi): (i64, Option<i64>, Option<i64>), (ci, h)| {
            (c + ci.is_some() as i64, min_some(lo, h), hi.max(h))
        });
    let mut out = Vec::new();
    (&stats).drive(|p, (c, lo, hi)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(c), ots(lo), ots(hi)]);
        out.push(row(f))
    });
    rows(out)
}

fn q7759(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rk = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    engagement(db, (&rk).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p)).drive(|p, (c, v, _, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(v)]);
        out.push(row(f))
    });
    rows(out)
}

fn q8808(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let mut v = Vec::new();
    engagement(db, owned(db).with(creation_date.ge(date(2023, 1, 1))))
        .and(creation_date)
        .drive(|p, ((c, _, u, d), cd)| v.push((u - d, cd, p, c, u, d)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(s, _, p, c, u, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::I(s)]);
        row(f)
    }))
}

fn q10545(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let cc = comments_per_post(db);
    let base = owned(db).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rk = whole(&base).select(Ident::<Post>::new().and(score).and(&cc)).window(rank, |((_, s), _)| s, desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 10).drive(|_, (((p, _), c), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::I(c), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q11121(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let t = owner_user.inv().select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, q, a, u, d, o), (ty, vt)| {
            (
                n + 1,
                q + (ty == 1) as i64,
                a + (ty == 2) as i64,
                u + (vt == Some(2)) as i64,
                d + (vt == Some(3)) as i64,
                o + (vt == Some(4)) as i64,
            )
        },
    );
    let rk = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t))
        .window(rank, |(_, t)| t.0, desc);
    let mut out = Vec::new();
    (&rk).drive(
        |_, (((id, dn), (n, q, a, u, d, o)), r)| {
            out.push(row(vec![V::I(id), V::S(dn), V::I(n), V::I(q), V::I(a), V::I(u), V::I(d), V::I(o), V::I(r)]))
        },
    );
    rows(out)
}

fn q5871(db: &'static So) -> String {
    let Post { post_type, score, creation_date, origid, .. } = &db.post;
    let rn = db
        .post
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let types: HashIdx<i64, Str> = (&db.post_type.origid).inv().select(&db.post_type.name).collect();
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 5).map(|(((p, _), _), _)| p))
        .and(origid.select(&types))
        .drive(|p, ((c, _, u, d), name)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::S(name)]);
        out.push(row(f))
    });
    rows(out)
}

fn q11639(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(date(2020, 1, 1)));
    let rn = whole(&base).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, ((p, _), _)| {
        out.push(row(post_fields(
            db,
            p,
            &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "rep", "owner"],
        )))
    });
    rows(out)
}

fn q14758(db: &'static So) -> String {
    let rk = owned(db)
        .group_by(&db.post.post_type)
        .select(Ident::<Post>::new().and(&db.post.score))
        .window(rank, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and((&db.post.owner_user).select(&db.user.creation_date)))
        .drive(|_, (p, ucd)| {
        let mut f = post_fields(
            db,
            p,
            &["id", "type_id", "created", "score", "views", "answers", "comments", "title", "tags", "rep"],
        );
        f.push(V::T(ucd));
        out.push(row(f))
    });
    rows(out)
}

fn q12293(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(&db.post.creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 100).map(|((p, _), _)| p)).drive(|p, (c, _, u, d)| {
        let mut f =
            post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q13547(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let eng = engagement(db, &base);
    let rk = whole(&base)
        .select(Ident::<Post>::new().and(&db.post.creation_date).and(&eng))
        .window(dense_rank, |((_, cd), _)| cd, desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 100).drive(|_, (((p, _), (c, _, u, d)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(u), V::I(d), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("10746", q10746),
    ("6392", q6392),
    ("26873", q26873),
    ("7237", q7237),
    ("7759", q7759),
    ("8808", q8808),
    ("10545", q10545),
    ("11121", q11121),
    ("5871", q5871),
    ("11639", q11639),
    ("14758", q14758),
    ("12293", q12293),
    ("13547", q13547),
];
