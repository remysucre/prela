use harness::prelude::*;

fn q10106(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let eng = engagement(db, db.post.iq());
    let rk = whole(db.post.iq())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()).and(&eng))
        .window(dense_rank, |(((_, s), v), _)| (s, v), desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 100).drive(|_, ((((p, _), _), (c, v, _, _)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(v), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q10184(db: &'static So) -> String {
    let cc = comments_per_post(db);
    let rn = whole(db.post.iq()).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(&cc))
        .drive(|_, (p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(if c == 0 { V::Null } else { V::I(c) });
        f.extend(post_fields(db, p, &["rep"]));
        out.push(row(f))
    });
    rows(out)
}

fn q11562(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let rn = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(row_number, |(_, rep)| rep, desc);
    let badges = (&db.badge.user).inv().fold(0i64, |a, _| a + 1);
    let latest: HashIdx<Id<User>, Id<Post>> = owned(db)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .collect();
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .select((&db.user.origid).and(&db.user.display_name).and(&db.user.reputation).and(badges.opt()).and(latest.opt()))
        .drive(|_, ((((id, dn), rep), b), p)| {
            let mut f = vec![V::I(id), V::S(dn), V::I(rep), oint(b)];
            match p {
                Some(p) => f.extend(post_fields(db, p, &["title", "created"])),
                None => f.extend([V::Null, V::Null]),
            }
            out.push(row(f))
        });
    rows(out)
}

fn q9023(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 10).map(|(((p, _), _), _)| p)).drive(|p, (c, _, u, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q28776(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let mentions = tag_mentions(db);
    let post_of = (&mentions).map(|(p, _)| p);
    let name_of = (&mentions).map(|(_, t)| t).select(&db.tag.tag_name);
    let joined = (&mentions).with((&post_of).select(owner_user));
    let sums = (&joined)
        .group_by(&name_of)
        .select((&post_of).select(post_type_id.and(view_count.opt()).and(owner_user.select(&db.user.reputation))))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, q, a, vn, vs, r), ((t, v), rep)| {
            (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, vn + v.is_some() as i64, vs + v.unwrap_or(0), r + rep)
        });
    let posts = (&joined).group_by(&name_of).select(&post_of).count_distinct();
    let stats = (&sums).and(&posts);
    let rk = whole(&stats).select(Same::new().and(&stats)).window(rank, |(_, (a, _))| (a.3 > 0).then_some(a.4), desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 10).drive(|_, ((t, ((n, q, a, vn, vs, r), d)), _)| {
        out.push(row(vec![
            V::S(t),
            V::I(d),
            V::I(q),
            V::I(a),
            nullable(vs, vn),
            avg(r, n),
        ]))
    });
    rows(out)
}
fn q14347(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let posts = owner_user.inv().select(post_type_id).dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64), |(n, q, a), t| {
        (n + 1, q + (t == 1) as i64, a + (t == 2) as i64)
    });
    let post_rows = posts_of(db).select(score.and(votes_of(db).with(vote_type_id.eq(8)).select(bounty_amount.opt()).opt()));
    let joined = db.user.select(badges_of(db).opt().and(post_rows.opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64),
        |(b, bn, bs, s), (bi, p)| {
            let bounty = p.and_then(|(_, v)| v.flatten());
            (b + bi.is_some() as i64, bn + bounty.is_some() as i64, bs + bounty.unwrap_or(0), s + p.map_or(0, |(sc, _)| sc))
        },
    );
    let rk = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.reputation).and(&joined).and(&posts))
        .window(rank, |(((_, rep), _), _)| rep, desc);
    let mut out = Vec::new();
    (&rk).drive(
        |_, ((((id, rep), (b, bn, bs, s)), (n, q, a)), r)| {
            out.push(row(vec![V::I(id), V::I(rep), V::I(b), nullable(bs, bn), V::I(n), V::I(q), V::I(a), V::I(s), V::I(r)]))
        },
    );
    rows(out)
}

fn q9016(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1));
    let rn = whole(&base).select(Ident::<Post>::new().and(view_count.opt())).window(row_number, |(_, v)| v, desc);
    let counts = (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).with((&db.vote.vote_type_id).eq(2)).opt()))
        .fold((0i64, 0i64), |(c, u), (ci, ui)| (c + ci.is_some() as i64, u + ui.is_some() as i64));
    let mut out = Vec::new();
    (&counts).drive(|p, (c, u)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views"]);
        f.extend([V::I(c), V::I(u)]);
        out.push(row(f))
    });
    rows(out)
}

fn q14268(db: &'static So) -> String {
    let Post { creation_date, view_count, score, .. } = &db.post;
    let cc = comments_per_post(db);
    let base = owned(db).with(creation_date.ge(date(2020, 1, 1)));
    let ranked = whole(&base)
        .select(Ident::<Post>::new().and(view_count.opt()).and(score).and(&cc))
        .window(rank, |(((_, v), _), _)| v, desc)
        .window(rank, |((((_, _), s), _), _)| s, desc);
    let mut out = Vec::new();
    ranked.filt(|((_, a), b)| a <= 10 || b <= 10).drive(|_, (((((p, _), _), c), a), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "owner"]);
        f.extend([V::I(c), V::I(a), V::I(b)]);
        out.push(row(f))
    });
    rows(out)
}

fn q10492(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, .. } = &db.post;
    let cc = comments_per_post(db);
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let bc = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let rn = whole(db.post.iq()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(_, cd)| cd, desc);
    let votes = (&rn)
        .filt(|(_, n)| n <= 100)
        .map(|((p, _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user_id.select(&bidx).opt()))
        .fold((0i64, 0i64), |(u, d), ((vt, _), _)| (u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64));
    let mut out = Vec::new();
    (&votes).and(&cc).and(owner_user_id.select(&bc).opt()).drive(|p, (((u, d), c), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(u), V::I(d), V::I(c), V::I(b.unwrap_or(0))]);
        out.push(row(f))
    });
    rows(out)
}

fn q10475(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let eng = engagement(db, db.post.iq());
    let rk = whole(db.post.iq())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()).and(&eng))
        .window(rank, |(((_, s), v), _)| (s, v), desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 100).drive(|_, ((((p, _), _), (c, v, _, _)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "rep"]);
        f.extend([V::I(c), V::I(v), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q9507(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 5).map(|((p, _), _)| p)).drive(|p, (c, _, u, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("10106", q10106),
    ("10184", q10184),
    ("11562", q11562),
    ("9023", q9023),
    ("28776", q28776),
    ("14347", q14347),
    ("9016", q9016),
    ("14268", q14268),
    ("10492", q10492),
    ("10475", q10475),
    ("9507", q9507),
];
