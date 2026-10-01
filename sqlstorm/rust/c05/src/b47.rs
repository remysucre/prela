use harness::prelude::*;

fn year_ago() -> i64 {
    add_years(ts(2024, 10, 1, 12, 34, 56), -1)
}

fn q8676(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let User { origid, display_name, reputation, up_votes, down_votes, .. } = &db.user;
    let pd = owner_user.inv().select(post_type_id.and(score).and(view_count.opt())).fold(
        (0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, q, a, s, vn, vs), ((t, sc), v)| {
            (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        },
    );
    let users = db.user.with(reputation.gt(1000)).with(&pd);
    let rk = whole(&users)
        .select(origid.and(display_name).and(reputation).and(&pd).and(up_votes.and(down_votes)))
        .window(dense_rank, |((_, a), _)| (a.4 > 0).then_some(a.5), desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .drive(|_, (((((id, dn), rep), (n, q, a, s, vn, vs)), (u, d)), r)| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(rep),
                V::I(n),
                V::I(q),
                V::I(a),
                avg(s, n),
                nullable(vs, vn),
                V::I(u - d),
                V::I(r),
            ]))
        });
    rows(out)
}

fn q7934(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(year_ago()));
    let eng = engagement(db, &base);
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date).and(&eng))
        .window(row_number, |((_, cd), _)| cd, desc);
    let mut out = Vec::new();
    (&rn).filt(|(((_, _), (_, _, u, d)), r)| r <= 10 || u - d > 0).drive(|_, (((p, _), (c, _, u, d)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::I(u - d), V::S(if r <= 10 { "Top" } else { "Others" })]);
        out.push(row(f))
    });
    rows(out)
}

fn q5356(db: &'static So) -> String {
    let Post { post_type, post_type_id, creation_date, score, view_count, .. } = &db.post;
    let Vote { post, vote_type_id, .. } = &db.vote;
    let pvs = post.inv().select(vote_type_id).fold((0i64, 0i64, 0i64), |(u, d, n), t| {
        (u + (t == 2) as i64, d + (t == 3) as i64, n + 1)
    });
    let pc = (&db.comment.post).inv().fold(0i64, |a, _| a + 1);
    let base = owned(db).with(creation_date.ge(year_ago())).with(post_type_id.is_in([1, 2]));
    let rk = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()).and((&pvs).opt()).and((&pc).opt()))
        .window(dense_rank, |((((_, s), v), _), _)| (s, v), desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(((_, pv), c), r)| r <= 10 && pv.is_some() && c.is_some())
        .drive(|_, (((((p, _), _), pv), c), r)| {
        let ((u, d, n), c) = (pv.unwrap(), c.unwrap());
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.extend([V::I(u), V::I(d), V::I(n), V::I(c), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q6127(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.gt(date(2024, 9, 1)));
    let rn = (&base).group_by(owner_user).select(score).window(row_number, |s| s, desc);
    let tu = (&rn).filt(|(_, n)| n <= 5).fold((0i64, 0i64), |(n, s), (x, _)| (n + 1, s + x));
    let phd = (&db.post_history.post)
        .inv()
        .select(&db.post_history.creation_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    (&base).select(owner_user.select(&tu).and(&phd).and(score)).drive(|p, x| v.push((p, x)));
    v.sort_by(|a, b| (b.1.0.0.1, b.1.0.0.0, b.1.1).cmp(&(a.1.0.0.1, a.1.0.0.0, a.1.1)));
    rows(v.iter().take(10).map(|&(p, (((n, s), (ec, last)), _))| {
        let u = owner_user.get(p).unwrap();
        let mut f = vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(n), V::I(s)];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(ec), V::T(last)]);
        row(f)
    }))
}

fn q7740(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let questions = owned(db).with(post_type_id.eq(1));
    let us = (&questions).select(owner_user).inv().select(score).fold((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let base = (&questions).with(creation_date.ge(year_ago()));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.select(&us)))
        .drive(|_, (p, (s, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(s), V::I(n)]);
        out.push(row(f))
    });
    rows(out)
}

fn q9441(db: &'static So) -> String {
    let Comment { post, creation_date, user_display_name, text, .. } = &db.comment;
    let Post { creation_date: pcd, view_count, .. } = &db.post;
    let recent = db.comment.with(creation_date.ge(date(2024, 9, 24)));
    let rn = (&recent)
        .group_by(post)
        .select(Ident::<Comment>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let mut v = Vec::new();
    (&rn)
        .filt(|(_, n)| n == 1)
        .map(|((c, _), _)| c)
        .with(post.with(pcd.ge(date(2024, 9, 1))))
        .select(Ident::<Comment>::new().and(user_display_name.select(&by_name)).and(post))
        .drive(|_, ((c, u), p)| v.push((view_count.get(p), db.user.reputation.get(u).unwrap(), c, u, p)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(vc, rep, c, u, p)| {
        let mut f = vec![V::S(db.user.display_name.get(u).unwrap()), V::I(rep)];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([oint(vc)]);
        f.extend(post_fields(db, p, &["score"]));
        f.extend([V::S(text.get(c).unwrap()), V::T(creation_date.get(c).unwrap())]);
        row(f)
    }))
}

fn q6621(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let base = owned(db).with(creation_date.ge(date(2024, 9, 1)));
    let rk = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 5)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(&up).and(&down))
        .drive(|_, ((p, u), d)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::B(u > 0), V::B(d > 0)]);
        out.push(row(f))
    });
    rows(out)
}

fn q8584(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let ps = owner_user.inv().select(post_type_id.and(score).and(view_count.opt())).fold(
        (0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, q, a, s, vn, vs), ((t, sc), v)| {
            (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        },
    );
    let bc = badges_per_user(db);
    let cm = (&db.comment.user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user
        .with(reputation.gt(1000))
        .select(origid.and(display_name).and(reputation).and(&ps).and(&bc).and(&cm))
        .drive(|_, x| v.push(x));
    v.sort_by(|a, b| (b.0.0.0.1, b.0.0.1.0).cmp(&(a.0.0.0.1, a.0.0.1.0)));
    rows(v.iter().take(10).map(|&(((((id, dn), rep), (n, q, a, s, vn, vs)), b), c)| {
        row(vec![
            V::I(id),
            V::S(dn),
            V::I(rep),
            V::I(n),
            V::I(q),
            V::I(a),
            nullable(vs, vn),
            avg(s, n),
            V::I(b),
            V::I(c),
        ])
    }))
}

fn q10452(db: &'static So) -> String {
    let Post { post_type, creation_date, score, comment_count, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(year_ago()));
    let cr: MatSet<Id<Post>> = whole(&base)
        .select(Ident::<Post>::new().and(comment_count))
        .window(row_number, |(_, c)| c, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let sr = (&base).group_by(post_type).select(Ident::<Post>::new().and(score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&sr).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).with(&cr).drive(|_, p| {
        let f = post_fields(db, p, &["id", "title", "score", "views", "answers", "owner", "created", "comments", "owner", "created"]);
        out.push(row(f))
    });
    rows(out)
}

fn q6969(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, score, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(year_ago()));
    let votes = (&db.vote.user).inv().fold(0i64, |a, _| a + 1);
    let top: MatSet<Id<User>> = whole(&votes)
        .select(Ident::<User>::new().and(&votes))
        .window(
            row_number,
            |(u, n)| (n, u),
            |x: &(i64, Id<User>), y: &(i64, Id<User>)| y.0.cmp(&x.0).then(x.1.cmp(&y.1)),
        )
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let posts = owner_user.inv().fold(0i64, |a, _| a + 1);
    let active: HashIdx<Str, i64> = db.user.with(&top).select(&db.user.display_name).inv().select(&posts).collect();
    let rn = (&base)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, r)| r <= 5)
        .map(|(((p, _), _), r)| (p, r))
        .select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|(p, _)| p).select(owner_user).select(&db.user.display_name).select(&active)))
        .drive(|_, ((p, r), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(n), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q27132(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let base = owned(db).with(post_type_id.eq(1));
    let rk = whole(&base)
        .select(Ident::<Post>::new().and(creation_date).and(view_count.opt()).and(&cc).and(&up).and(&down))
        .window(dense_rank, |(((((_, cd), _), _), _), _)| cd, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|((((((_, _), v), _), u), d), r)| r <= 100 && v.is_some_and(|v| v >= 1000) && u - d > 0)
        .drive(|_, ((((((p, _), _), c), u), d), r)| {
            let mut f = post_fields(db, p, &["id", "title", "body"]);
            f.push(db.post.tags_str.get(p).map_or(V::Null, |t| V::Owned(t.split("<>").collect::<Vec<_>>().join(", "))));
            f.extend(post_fields(db, p, &["views", "owner"]));
            f.extend([V::I(c), V::I(u), V::I(d), V::I(r)]);
            out.push(row(f))
        });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("8676", q8676),
    ("7934", q7934),
    ("5356", q5356),
    ("6127", q6127),
    ("7740", q7740),
    ("9441", q9441),
    ("6621", q6621),
    ("8584", q8584),
    ("10452", q10452),
    ("6969", q6969),
    ("27132", q27132),
];
