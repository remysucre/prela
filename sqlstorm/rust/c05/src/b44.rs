use harness::prelude::*;

fn q29270(db: &'static So) -> String {
    let Post { post_type_id, score, tags_str, creation_date, accepted_answer, .. } = &db.post;
    let accepted_by = accepted_answer.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let base = db.post.with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(creation_date).and(&accepted_by))
        .window(row_number, |((_, cd), _)| cd, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 5).drive(|_, (((p, _), a), r)| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "score", "created", "views", "owner"]);
        f.extend([V::S(if a > 0 { "Yes" } else { "No" }), V::Owned(format!("Tag Rank: {r}"))]);
        out.push(row(f))
    });
    rows(out)
}

fn q12277(db: &'static So) -> String {
    let Post { post_type_id, creation_date, parent, .. } = &db.post;
    let ac = parent.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let base = db.post.with(post_type_id.eq(1));
    let rn = whole(&base).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(_, cd)| cd, desc);
    let votes = (&rn)
        .filt(|(_, n)| n <= 100)
        .map(|((p, _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(u, d), (_, vt)| (u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64));
    let mut out = Vec::new();
    (&votes).and(&ac).drive(|p, ((u, d), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.extend([V::I(a), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q12224(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1));
    let name = (&db.post.owner_user).select(&db.user.display_name);
    let eng = engagement(db, &base);
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(score).and(creation_date).and(&eng).and(name.opt()))
        .window(row_number, |((((_, s), cd), _), _)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((((p, _), _), (c, _, u, d)), dn), r)| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::S(dn.unwrap_or("Community User")), V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q13726(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1));
    let eng = engagement(db, &base);
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(view_count.opt()).and(&eng))
        .window(row_number, |((_, v), _)| v, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((p, _), (c, _, u, d)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "rep"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q8376(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let e = engagement(db, &base);
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(&e).and(creation_date))
        .window(row_number, |((_, e), cd)| (e.0, cd), desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((p, (c, _, u, d)), _), _)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q5758(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(post_type_id.eq(1));
    let e = engagement(db, &base);
    let rk = whole(&base).select(Ident::<Post>::new().and(&e)).window(dense_rank, |(_, e)| e.0, desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 10).drive(|_, ((p, (c, _, u, d)), _)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::I(u - d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q9571(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, view_count, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rk = (&base).group_by(owner_user).select(view_count.opt()).window(rank, |v| v, desc);
    let top = (&rk)
        .filt(|(_, r)| r <= 5)
        .fold((0i64, 0i64, 0i64), |(n, vn, vs), (v, _)| (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0)));
    let mut v = Vec::new();
    db.user
        .with((&db.user.reputation).gt(1000))
        .select((&db.user.origid).and(&db.user.display_name).and(&db.user.reputation).and(&top))
        .drive(|_, (((id, dn), rep), (n, vn, vs))| v.push((id, dn, rep, n, (vn > 0).then_some(vs))));
    v.sort_by(|a, b| (b.4, b.3).cmp(&(a.4, a.3)));
    rows(v.iter().take(10).map(|&(id, dn, rep, n, vs)| row(vec![V::I(id), V::S(dn), V::I(rep), V::I(n), oint(vs)])))
}

fn q9291(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.gt(ts(2024, 10, 1, 12, 34, 56) - 30 * DAY_US));
    let rn = (&base)
        .group_by(post_type)
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

fn q8853(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_years(current_date(), -1)));
    let rn = (&base)
        .group_by(post_type)
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

fn q5781(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, origid, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let base = db.post.with(creation_date.ge(add_years(current_date(), -1)));
    let rn = (&base)
        .group_by(owner_user.opt())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let users: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.origid).inv().collect();
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(origid.select(&users)).and(&cc).and(&up))
        .drive(|_, (((p, u), c), up)| {
        let mut f = vec![V::S(db.user.display_name.get(u).unwrap())];
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(up)]);
        out.push(row(f))
    });
    rows(out)
}

fn q4027(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_years(current_date(), -1)));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let latest_of: HashIdx<Id<User>, Id<Post>> = (&rn).filt(|(_, n)| n == 1).map(|((p, _), _)| p).collect();
    let counts = (&latest_of)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).filt(|t| t == 2 || t == 3).opt()))
        .fold((0i64, 0i64), |(c, u), (ci, vt)| (c + ci.is_some() as i64, u + (vt == Some(2)) as i64));
    let agg = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(&db.user.display_name)
        .select((&latest_of).select(score.and(&counts)))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, cs, us), (sc, (c, u))| (n + 1, s + sc, cs + c, us + u));
    let mut v = Vec::new();
    (&agg).filt(|(n, _, _, _)| n > 5).drive(|dn, (n, s, cs, us)| v.push((dn, n, s, cs, us)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|&(dn, n, s, cs, us)| row(vec![V::S(dn), V::I(n), avg(s, n), V::I(cs), V::I(us)])))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("29270", q29270),
    ("12277", q12277),
    ("12224", q12224),
    ("13726", q13726),
    ("8376", q8376),
    ("5758", q5758),
    ("9571", q9571),
    ("9291", q9291),
    ("8853", q8853),
    ("5781", q5781),
    ("4027", q4027),
];
