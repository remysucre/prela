use harness::prelude::*;

fn q27229(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, score, body, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(add_years(current_date(), -1)));
    let _rank = (&base).group_by(tags_str.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(_, s)| s, desc);
    let tag_users = owned(db)
        .with(post_type_id.eq(1))
        .with(tags_str)
        .select(owner_user)
        .inv()
        .fold(0i64, |a, _| a + 1);
    let frequent = (&tag_users).filt(|n| n > 10);
    let top: MatSet<Id<User>> = whole(&frequent)
        .select(Ident::<User>::new().and(&tag_users))
        .window(
            row_number,
            |(u, n)| (n, u),
            |x: &(i64, Id<User>), y: &(i64, Id<User>)| y.0.cmp(&x.0).then(x.1.cmp(&y.1)),
        )
        .filt(|(_, n)| n <= 5)
        .map(|((u, _), _)| u)
        .collect();
    let top_rows: MatSet<(Str, i64)> = db.user.with(&top).select((&db.user.display_name).and(&tag_users)).collect();
    let contributors: HashIdx<Str, (Str, i64)> = (&top_rows).map(|(dn, _)| dn).inv().collect();
    let mut out = Vec::new();
    (&base)
        .with(body.filt(|b: Str| b.chars().count() > 1000))
        .select(owner_user.select(&db.user.display_name).select(&contributors))
        .drive(|p, (dn, n)| {
            let mut f = post_fields(db, p, &["id", "title", "body", "created", "score", "views", "answers", "owner", "rep"]);
            f.extend([V::S(dn), V::I(n)]);
            out.push(row(f))
        });
    rows(out)
}

fn q8309(db: &'static So) -> String {
    let Post { post_type, post_type_id, creation_date, score, view_count, owner_user, parent, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let answerers = (&base)
        .with(post_type_id.eq(2))
        .select(owner_user)
        .inv()
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let pha = (&db.post_history.post)
        .inv()
        .select(&db.post_history.creation_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let children = parent.inv().fold(0i64, |a, _| a + 1);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .with(&children)
        .select(Ident::<Post>::new().and(&pha))
        .drive(|_, (p, (ec, last))| {
        db.user.select((&db.user.display_name).and((&answerers).filt(|a| a.0 > 5))).drive(|_, (dn, (n, s))| {
            let mut f = post_fields(db, p, &["title", "views", "score", "answers"]);
            f.extend([V::I(ec), V::T(last), V::S(dn), V::I(n), V::I(s)]);
            out.push(row(f))
        })
    });
    rows(out)
}

fn q27348(db: &'static So) -> String {
    let Post { post_type_id, origid, view_count, score, owner_user, answer_count, .. } = &db.post;
    let tag_by_id: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let tag_counts = db
        .post
        .with(post_type_id.eq(1))
        .group_by(origid.select(&tag_by_id).select(&db.tag.tag_name))
        .select(view_count.opt().and(score))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, vn, vs, s), (v, sc)| (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0), s + sc));
    let tag_rank = whole(&tag_counts).select(Same::new().and(&tag_counts)).window(rank, |(_, a)| a.0, desc);
    let activity = owned(db)
        .with(post_type_id.eq(1))
        .group_by(owner_user.select(&db.user.display_name))
        .select(view_count.opt().and(answer_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, vn, vs, an, asum), (v, a)| {
            (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0), an + a.is_some() as i64, asum + a.unwrap_or(0))
        });
    let views = (&activity).map(|a| (a.1 > 0).then_some(a.2));
    let max_views = (&views).fold_flat(None, |m: Option<i64>, v| m.max(v));
    let user_rank = whole(&activity).select(Same::new().and(&activity)).window(rank, |(_, a)| a.0, desc);
    let users = rel(drain((&user_rank).filt(|((_, a), r)| {
        let v = (a.1 > 0).then_some(a.2);
        r <= 5 && v.is_some() && v == max_views
    })));
    let mut out = Vec::new();
    (&tag_rank).filt(|(_, r)| r <= 5).cross(&users).drive(|_, (((t, (n, vn, vs, s)), _), (_, ((dn, (un, _, uvs, uan, uasum)), _)))| {
        out.push(row(vec![V::S(t), V::I(n), nullable(vs, vn), avg(s, n), V::S(dn), V::I(un), V::I(uvs), nullable(uasum, uan)]))
    });
    rows(out)
}

fn user_post_totals(db: &'static So) -> DenseFold<Id<User>, (i64, i64, i64, i64)> {
    let Post { owner_user, view_count, score, .. } = &db.post;
    owner_user.inv().select(view_count.opt().and(score)).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64),
        |(n, vn, vs, s), (v, sc)| (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0), s + sc),
    )
}

fn q14558(db: &'static So) -> String {
    let t = user_post_totals(db);
    let rk = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t).and(&db.user.reputation))
        .window(rank, |(_, rep)| rep, desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 10).drive(|_, ((((id, dn), (n, _, _, s)), _), _)| {
        out.push(row(vec![V::I(id), V::S(dn), V::I(n), avg(s, n)]))
    });
    rows(out)
}

fn q14389(db: &'static So) -> String {
    let t = user_post_totals(db);
    let rk = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t))
        .window(rank, |(_, a)| (a.0 > 0).then_some(a.3), desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 10).drive(
        |_, (((id, dn), (n, vn, vs, s)), _)| out.push(row(vec![V::I(id), V::S(dn), V::I(n), nullable(vs, vn), nullable(s, n)])),
    );
    rows(out)
}

fn q14345(db: &'static So) -> String {
    let t = user_post_totals(db);
    let ranked = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t))
        .window(rank, |(_, t)| t.0, desc)
        .window(rank, |((_, t), _)| (t.1 > 0).then_some(t.2), desc)
        .window(rank, |(((_, t), _), _)| (t.0 > 0).then_some(t.3), desc);
    let mut out = Vec::new();
    ranked
        .filt(|((((_, a), _), _), _)| a.0 > 0)
        .drive(|_, (((((id, dn), (n, vn, vs, s)), a), b), c)| {
            out.push(row(vec![V::I(id), V::S(dn), V::I(n), nullable(vs, vn), nullable(s, n), V::I(a), V::I(b), V::I(c)]))
        });
    rows(out)
}

fn q12800(db: &'static So) -> String {
    let PostHistory { post, creation_date, post_history_type_id, .. } = &db.post_history;
    let edits = db.post_history.with(post_history_type_id.is_in([4, 5, 6]));
    let next = (&edits).group_by(post).select(creation_date).window(lead, |d| d, asc);
    let mut out = Vec::new();
    (&next)
        .filt(|(_, nx)| nx.is_some())
        .fold((0i64, 0f64), |(n, s), (d, nx)| (n + 1, s + (nx.unwrap() - d) as f64 / 1e6))
        .drive(|_, (n, s)| out.push(row(vec![V::F(s / n as f64), V::I(n)])));
    rows(out)
}

fn q11847(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let vc = votes_per_post(db);
    let rn = whole(owned(db))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(&vc))
        .drive(|_, (p, v)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        f.push(V::I(v));
        out.push(row(f))
    });
    rows(out)
}

fn q13279(db: &'static So) -> String {
    let Post { score, view_count, post_type_id, parent, .. } = &db.post;
    let ac = parent.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let base = db.post.with(post_type_id.eq(1));
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(&ac))
        .drive(|_, (p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner_id", "rep"]);
        f.push(V::I(a));
        out.push(row(f))
    });
    rows(out)
}

fn q10804(db: &'static So) -> String {
    let e = engagement(db, db.post.iq());
    let ranked = whole(db.post.iq())
        .select(Ident::<Post>::new().and(&e))
        .window(rank, |(_, e)| e.1, desc)
        .window(rank, |((_, e), _)| e.0, desc);
    let mut out = Vec::new();
    ranked.drive(|_, (((p, (c, v, _, _)), a), b)| {
        if a <= 10 || b <= 10 {
            let mut f = post_fields(db, p, &["id", "title"]);
            f.extend([V::I(v), V::I(c), V::I(a), V::I(b)]);
            out.push(row(f))
        }
    });
    rows(out)
}

fn q10127(db: &'static So) -> String {
    let vc = votes_per_post(db);
    let rk = whole(db.post.iq()).select(Ident::<Post>::new().and(&db.post.score)).window(rank, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(&vc))
        .drive(|_, (p, v)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.push(V::I(v));
        out.push(row(f))
    });
    rows(out)
}

fn q13510(db: &'static So) -> String {
    let vc = votes_per_post(db);
    let rn = whole(owned(db))
        .select(Ident::<Post>::new().and(&db.post.score).and(&vc))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((p, _), v), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "rep"]);
        f.extend([V::I(v), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("27229", q27229),
    ("8309", q8309),
    ("27348", q27348),
    ("14558", q14558),
    ("14389", q14389),
    ("14345", q14345),
    ("12800", q12800),
    ("11847", q11847),
    ("13279", q13279),
    ("10804", q10804),
    ("10127", q10127),
    ("13510", q13510),
];
