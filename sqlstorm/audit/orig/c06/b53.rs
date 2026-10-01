use harness::prelude::*;

fn q14117(db: &'static So) -> String {
    let rk = whole(db.post.iq())
        .select(Ident::<Post>::new().and((&db.post.view_count).opt()))
        .window(rank, |(_, v)| v, desc);
    let mut out = Vec::new();
    engagement(db, (&rk).filt(|(_, r)| r <= 10).map(|((p, _), _)| p)).drive(|p, (c, v, _, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(v)]);
        f.extend(post_fields(db, p, &["answers"]));
        out.push(row(f))
    });
    rows(out)
}

fn q9422(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let base = owned(db).with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 10).map(|(((p, _), _), _)| p)).drive(|p, (c, _, u, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q5949(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let User { origid, reputation, display_name, .. } = &db.user;
    let bc = badges_per_user(db);
    let questions = db.post.with(post_type_id.eq(1));
    let rn = (&questions)
        .group_by((&db.post.owner_user_id).opt())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let rich = db.user.with(reputation.gt(1000));
    let top: MatSet<Id<User>> = whole(&rich)
        .select(Ident::<User>::new().and(reputation))
        .window(row_number, |(_, r)| r, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 3)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.with(&top)))
        .drive(|_, (p, u)| {
        let mut f = vec![
            V::I(origid.get(u).unwrap()),
            V::S(display_name.get(u).unwrap()),
            V::I(reputation.get(u).unwrap()),
            V::I(bc.get(u).unwrap()),
        ];
        f.extend(post_fields(db, p, &["id", "title", "score", "created", "views"]));
        out.push(row(f))
    });
    rows(out)
}

fn q6809(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(score).and(creation_date).and(&cc).and(&up))
        .window(row_number, |((((_, s), cd), _), _)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn).filt(|(((_, c), _), _)| c > 5).drive(|_, (((((p, _), _), c), u), r)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner"]);
        f.extend([V::I(c), V::I(u)]);
        f.push(V::S(match r {
            ..=10 => "Top Post",
            11..=50 => "Popular Post",
            _ => "Regular Post",
        }));
        out.push(row(f))
    });
    rows(out)
}

fn q13482(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let rn = whole(&base).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 10).map(|((p, _), _)| p)).drive(|p, (_, v, _, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        f.push(V::I(v));
        out.push(row(f))
    });
    rows(out)
}

fn q13839(db: &'static So) -> String {
    let vc = votes_per_post(db);
    let rn = whole(db.post.iq())
        .select(Ident::<Post>::new().and(&db.post.creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(&vc))
        .drive(|_, (p, v)| {
        let mut f = post_fields(
            db,
            p,
            &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "rep"],
        );
        f.push(V::I(v));
        out.push(row(f))
    });
    rows(out)
}

fn q14822(db: &'static So) -> String {
    let Post { creation_date, view_count, score, .. } = &db.post;
    let cc = comments_per_post(db);
    let base = owned(db).with(creation_date.ge(date(2023, 1, 1)));
    let engagement = view_count.opt().and(&cc).and(score).map(|((v, c), s)| v.map(|v| v + c + s));
    let rk = whole(&base).select(Ident::<Post>::new().and(&cc).and(&engagement)).window(rank, |(_, e)| e, desc);
    let mut out = Vec::new();
    (&rk).drive(|_, (((p, c), e), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "rep", "owner"]);
        f.extend([V::I(c), oint(e), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q9109(db: &'static So) -> String {
    let Post { owner_user, score, creation_date, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let base = owned(db).with(score.gt(10));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let top = (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .group_by(owner_user.select(display_name))
        .select(score)
        .fold((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let badges = db.user.group_by(display_name).select(badges_per_user(db)).fold(0i64, |a, b| a + b);
    let mut v = Vec::new();
    (&top).and(&badges).drive(|dn, ((s, n), b)| v.push((s, n, dn, b)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(s, n, dn, b)| row(vec![V::S(dn), V::I(s), V::I(n), V::I(b)])))
}

fn q13999(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(date(2022, 1, 1)));
    let rn = whole(&base).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, ((p, _), _)| {
        out.push(row(post_fields(
            db,
            p,
            &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "rep"],
        )))
    });
    rows(out)
}

fn q7151(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let e = engagement(db, &base);
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(&e))
        .window(row_number, |(_, (_, _, u, d))| u - d, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, ((p, (c, _, u, d)), _)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "rep"]);
        f.extend([V::I(c), V::I(u - d)]);
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("14117", q14117),
    ("9422", q9422),
    ("5949", q5949),
    ("6809", q6809),
    ("13482", q13482),
    ("13839", q13839),
    ("14822", q14822),
    ("9109", q9109),
    ("13999", q13999),
    ("7151", q7151),
];
