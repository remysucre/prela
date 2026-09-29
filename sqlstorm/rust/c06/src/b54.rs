use harness::prelude::*;

fn year_ago() -> i64 {
    add_years(ts(2024, 10, 1, 12, 34, 56), -1)
}

fn q12895(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(add_years(current_date(), -1)));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(score))
        .drive(|_, (p, s)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner"]);
        f.push(V::S(match s {
            10.. => "Highly Engaging",
            5..=9 => "Moderately Engaging",
            _ => "Less Engaging",
        }));
        out.push(row(f))
    });
    rows(out)
}

fn q10004(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).gt(date(2022, 1, 1)));
    let rn = whole(&base).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 10).map(|((p, _), _)| p)).drive(|p, (c, v, _, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(v)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        out.push(row(f))
    });
    rows(out)
}

fn q8100(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let base = owned(db).with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(_, s)| s, desc);
    let top = (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .group_by(owner_user.select(display_name))
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let badges_of: HashIdx<Id<User>, Id<Badge>> = (&db.badge.user).inv().collect();
    let badges = db.user.group_by(display_name).select((&badges_of).select(&db.badge.class).opt()).fold(
        (0i64, 0i64),
        |(n, s), c: Option<i64>| (n + c.is_some() as i64, s + c.unwrap_or(0)),
    );
    let mut out = Vec::new();
    (&top).and(&badges).drive(|dn, ((n, s), (b, cs))| {
        out.push(row(vec![V::S(dn), V::I(n), V::I(s), V::I(b), nullable(cs, b)]))
    });
    rows(out)
}

fn q6620(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let base = owned(db).with(creation_date.ge(ts(2024, 10, 1, 12, 34, 56) - 30 * DAY_US));
    let rk = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date).and(&up).and(&down))
        .window(rank, |((((_, s), cd), _), _)| (s, cd), desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 10).drive(|_, (((((p, _), _), u), d), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        let pct = if u + d == 0 { 0.0 } else { (u as f64 / (u + d) as f64 * 100.0 * 100.0).round() / 100.0 };
        f.extend([V::I(r), V::I(u), V::I(d), V::F(pct)]);
        out.push(row(f))
    });
    rows(out)
}

fn q8214(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let base = owned(db).with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(_, s)| s, desc);
    let mut v = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and((&up).and(score)))
        .drive(|_, (p, (u, s))| v.push((u, s, p)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(u, _, p)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.push(V::I(u));
        row(f)
    }))
}

fn q6423(db: &'static So) -> String {
    let Post { owner_user_id, score, creation_date, origid, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2023, 1, 1)));
    let rn = (&base)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let user_by_id: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let bc = badges_per_user(db);
    let mut v = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(origid.select(&user_by_id)))
        .drive(|_, (p, u)| v.push((p, u)));
    v.sort_by(|a, b| {
        let key = |x: &(Id<Post>, Id<User>)| (score.get(x.0), db.post.view_count.get(x.0));
        key(b).cmp(&key(a))
    });
    rows(v.iter().map(|&(p, u)| {
        let mut f = post_fields(db, p, &["title", "score", "views", "answers", "comments"]);
        f.extend([V::I(db.user.reputation.get(u).unwrap()), V::I(bc.get(u).unwrap())]);
        row(f)
    }))
}

fn q7485(db: &'static So) -> String {
    let Post { post_type_id, creation_date, parent, .. } = &db.post;
    let ac = parent.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mentions = tag_mentions(db);
    let tag_of = (&mentions).map(|(_, t)| t);
    let counts = (&mentions).group_by(&tag_of).fold(0i64, |a, _| a + 1);
    let top: MatSet<Id<Tag>> = whole(&counts)
        .select(Ident::<Tag>::new().and(&counts))
        .window(
            row_number,
            |(t, n)| (n, t),
            |x: &(i64, Id<Tag>), y: &(i64, Id<Tag>)| y.0.cmp(&x.0).then(x.1.cmp(&y.1)),
        )
        .filt(|(_, n)| n <= 5)
        .map(|((t, _), _)| t)
        .collect();
    let tags_by_post: HashIdx<Id<Post>, Id<Tag>> =
        (&mentions).with((&tag_of).with(&top)).map(|(p, _)| p).inv().select(&tag_of).collect();
    let mut v = Vec::new();
    owned(db).with(post_type_id.eq(1)).select(creation_date.and(&tags_by_post)).drive(|p, (cd, t)| v.push((cd, p, t)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, t)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(ac.get(p).unwrap())]);
        row(f)
    }))
}

fn q9716(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let cc = comments_per_post(db);
    let base = owned(db).with(creation_date.gt(ts(2024, 10, 1, 12, 34, 56) - 30 * DAY_US));
    let eng = engagement(db, &base);
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date).and(&eng).and(&cc))
        .window(row_number, |(((_, cd), _), _)| cd, desc);
    let mut out = Vec::new();
    (&rn).drive(|_, ((((p, _), (_, _, u, d)), c), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        f.push(V::S(if r <= 5 { "Top Recent Post" } else { "Other Recent Post" }));
        out.push(row(f))
    });
    rows(out)
}

fn q9437(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let base = owned(db).with(creation_date.ge(year_ago()));
    let rk = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()).and(&cc).and(&up).and(&down))
        .window(rank, |(((((_, s), v), _), _), _)| (s, v), desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 15).drive(|_, ((((((p, _), _), c), u), d), r)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        f.push(V::S(match r {
            ..=5 => "Top",
            6..=15 => "Middle",
            _ => "Low",
        }));
        out.push(row(f))
    });
    rows(out)
}

fn q6187(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, origid, .. } = &db.post;
    let questions = owned(db).with(post_type_id.eq(1));
    let per_user = (&questions).select(owner_user).inv().select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let enough = (&per_user).filt(|a| a.0 >= 5);
    let top: MatSet<Id<User>> = whole(&enough)
        .select(Ident::<User>::new().and(&per_user))
        .window(
            row_number,
            |(u, a)| (a.1, u),
            |x: &(i64, Id<User>), y: &(i64, Id<User>)| y.0.cmp(&x.0).then(x.1.cmp(&y.1)),
        )
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let user_by_id: HashIdx<i64, Id<User>> = db.user.with(&top).select(&db.user.origid).inv().collect();
    let recent = (&questions).with(creation_date.ge(year_ago()));
    let rn = (&recent).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(origid.select(&user_by_id)))
        .drive(|_, (p, u)| {
        let mut f = vec![V::S(db.user.display_name.get(u).unwrap())];
        f.extend(post_fields(db, p, &["title", "score", "created"]));
        f.push(V::I(1));
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("12895", q12895),
    ("10004", q10004),
    ("8100", q8100),
    ("6620", q6620),
    ("8214", q8214),
    ("6423", q6423),
    ("7485", q7485),
    ("9716", q9716),
    ("9437", q9437),
    ("6187", q6187),
];
