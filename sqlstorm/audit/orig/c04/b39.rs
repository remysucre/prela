use harness::prelude::*;

fn q11057(db: &'static So) -> String {
    let Post { post_type_id, owner_user, origid, title, view_count, score, creation_date, .. } =
        &db.post;
    let rep = owner_user.select(&db.user.reputation);
    let base = owned(db).with(post_type_id.eq(1));

    let (n, ss, vn, vs, rs) = (&base).select(score.and(view_count.opt()).and(&rep)).fold_flat(
        (0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, ss, vn, vs, rs), ((s, v), r)| {
            (n + 1, ss + s, vn + v.is_some() as i64, vs + v.unwrap_or(0), rs + r)
        },
    );

    let mut v: Vec<(Id<Post>, i64, i64, i64, i64)> = Vec::new();
    (&base).select(origid.and(score).and(&rep).and(creation_date))
        .drive(|p, (((id, s), r), cd)| v.push((p, id, s, r, cd)));
    v.sort_by(|a, b| b.4.cmp(&a.4));
    rows(v.iter().take(100).map(|&(p, id, s, r, _)| {
        row(vec![
            V::I(id),
            ostr(title.get(p)),
            oint(view_count.get(p)),
            V::I(s),
            V::I(r),
            avg(vs, vn),
            avg(ss, n),
            avg(rs, n),
        ])
    }))
}

fn q13330(db: &'static So) -> String {
    let Post { owner_user, origid, title, score, creation_date, .. } = &db.post;
    let User { origid: uid, display_name, reputation, .. } = &db.user;
    let base = owned(db).with(creation_date.between(date(2023, 1, 1), date(2023, 12, 31)));

    let (n, ss) = (&base).select(score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mut out = Vec::new();
    (&base).select(origid.and(creation_date).and(score).and(owner_user.select(uid.and(display_name).and(reputation))))
        .drive(|p, (((id, cd), s), ((u, dn), rep))| {
            out.push(row(vec![
                V::I(id),
                ostr(title.get(p)),
                V::T(cd),
                V::I(s),
                V::I(u),
                V::S(dn),
                V::I(rep),
                avg(ss, n),
                V::I(n),
            ]))
        });
    rows(out)
}

fn q14071(db: &'static So) -> String {
    let Post { post_type_id, owner_user, title, score, view_count, creation_date, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1));
    let rn = whole(&base)
        .select(
            Ident::<Post>::new()
                .and(score)
                .and(view_count.opt())
                .and(creation_date)
                .and(owner_user.select(&db.user.reputation)),
        )
        .window(row_number, |((((_, s), v), _), _)| (s, v), desc);

    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 100).drive(|_, (((((p, s), v), cd), rep), r)| {
        out.push(row(vec![V::I(r), ostr(title.get(p)), V::T(cd), V::I(s), oint(v), V::I(rep)]))
    });
    rows(out)
}

fn q14362(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1));
    let rn = whole(&base).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 100).drive(|_, ((p, _), _)| {
        out.push(row(post_fields(db, p, &["id", "title", "owner", "created", "score", "views", "answers", "comments"])))
    });
    rows(out)
}

fn q12676(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let rn = owned(db)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, ((p, _), _)| {
        out.push(row(post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner"])))
    });
    rows(out)
}

fn q14505(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let rn = owned(db)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((p, _), _), _)| {
        out.push(row(post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"])))
    });
    rows(out)
}

fn q11825(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1));
    let rn = whole(&base).select(Ident::<Post>::new().and(score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 100).drive(|_, ((p, _), _)| {
        out.push(row(post_fields(
            db,
            p,
            &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner"],
        )))
    });
    rows(out)
}

fn q14639(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rk = whole(&base).select(Ident::<Post>::new().and(score)).window(dense_rank, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 100).drive(|_, ((p, _), r)| {
        let mut v = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        v.push(V::I(r));
        out.push(row(v))
    });
    rows(out)
}

fn q9163(db: &'static So) -> String {
    let Post { post_type, creation_date, origid, .. } = &db.post;
    let PostType { origid: tid, name, .. } = &db.post_type;
    let base = owned(db).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let types: HashIdx<i64, Str> = tid.inv().select(name).collect();
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(origid.select(&types)))
        .drive(|_, (p, tn)| {
        let mut v = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        v.push(V::S(tn));
        out.push(row(v))
    });
    rows(out)
}

fn q8912(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(date(2024, 9, 1)));
    let rn = (&base)
        .group_by(post_type.select(&db.post_type.name))
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 5).drive(|_, ((p, _), _)| {
        out.push(row(post_fields(
            db,
            p,
            &["id", "title", "created", "views", "score", "answers", "comments", "type", "owner", "rep"],
        )))
    });
    rows(out)
}

fn q7095(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, .. } = &db.post;
    let base = owned(db)
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(view_count.gt(100));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((p, _), _), _)| {
        out.push(row(post_fields(
            db,
            p,
            &["id", "title", "created", "views", "score", "answers", "comments", "owner", "type_id"],
        )))
    });
    rows(out)
}

fn q11570(db: &'static So) -> String {
    let Post { post_type, score, view_count, accepted_answer_id, .. } = &db.post;
    let stats = db
        .post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(accepted_answer_id.opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, acc), ((sc, v), a)| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0), acc + a.is_some() as i64)
        });
    let rk = whole(&stats).select(Same::new().and(&stats)).window(
        rank,
        |(_, (n, s, ..))| s as f64 / n as f64,
        |a: &f64, b: &f64| b.partial_cmp(a).unwrap(),
    );
    let mut out = Vec::new();
    (&rk).drive(|_, ((name, (n, s, vn, vs, acc)), r)| {
        out.push(row(vec![V::S(name), V::I(n), avg(s, n), nullable(vs, vn), V::I(acc), V::I(r)]))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("11057", q11057),
    ("13330", q13330),
    ("14071", q14071),
    ("14362", q14362),
    ("12676", q12676),
    ("14505", q14505),
    ("11825", q11825),
    ("14639", q14639),
    ("9163", q9163),
    ("8912", q8912),
    ("7095", q7095),
    ("11570", q11570),
];
