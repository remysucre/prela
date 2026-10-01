use harness::prelude::*;

fn q12940(db: &'static So) -> String {
    let Post { view_count, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let post_rows = posts_of(db).select(view_count.opt().and(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt()));
    let totals = db.user.select(post_rows.opt().and(badges_of(db).opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, bn, bs, u, d, b, w), (p, bi)| {
            let vote = p.and_then(|(_, v)| v);
            let bounty = vote.and_then(|(_, x)| x);
            let t = vote.map(|(t, _)| t);
            (
                n + p.is_some() as i64,
                bn + bounty.is_some() as i64,
                bs + bounty.unwrap_or(0),
                u + (t == Some(2)) as i64,
                d + (t == Some(3)) as i64,
                b + bi.is_some() as i64,
                w + p.and_then(|(v, _)| v).unwrap_or(0),
            )
        },
    );
    let ranked = whole(&db.user.id)
        .select((&db.user.display_name).and(&totals))
        .window(rank, |(_, t)| t.6, desc)
        .window(rank, |((_, t), _)| t.0, desc);
    let mut out = Vec::new();
    ranked.drive(|_, (((dn, (n, bn, bs, u, d, b, w)), x), y)| {
        out.push(row(vec![V::S(dn), V::I(n), nullable(bs, bn), V::I(u), V::I(d), V::I(b), V::I(w), V::I(x), V::I(y)]))
    });
    rows(out)
}

fn q11501(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let rn = owned(db)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let stats = (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).with(vote_type_id.is_in([8, 9])).select(bounty_amount.opt()).opt()))
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), (ci, b)| {
            let b = b.flatten();
            (c + ci.is_some() as i64, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        });
    let mut out = Vec::new();
    (&stats).drive(|p, (c, bn, bs)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), nullable(bs, bn)]);
        out.push(row(f))
    });
    rows(out)
}

fn q10593(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let rk = whole(owned(db)).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let counts = (&rk)
        .filt(|(_, r)| r <= 100)
        .map(|((p, _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold((0i64, 0i64, 0i64), |(c, v, b), ((ci, vi), bi)| {
            (c + ci.is_some() as i64, v + vi.is_some() as i64, b + bi.is_some() as i64)
        });
    let mut out = Vec::new();
    (&counts).drive(|p, (c, v, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(v), V::I(b)]);
        f.extend(post_fields(db, p, &["type", "rep"]));
        out.push(row(f))
    });
    rows(out)
}

fn q28976(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, origid, tags_str, title, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2023, 10, 1)));
    let kept: MatSet<Id<Post>> = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), _)| s, desc)
        .window(row_number, |(((_, _), v), _)| v, desc)
        .filt(|((_, a), b)| a <= 5 || b <= 5)
        .map(|((((p, _), _), _), _)| p)
        .collect();
    let type_by_id: HashIdx<i64, Str> = (&db.post_type.origid).inv().select(&db.post_type.name).collect();
    let tag_count = tags_str.map(|t: Str| t.split('>').count() as i64);
    let titles = (&kept).group_by(origid.select(&type_by_id)).select(title.opt()).buf_fold(|ts| {
        let parts: Vec<Str> = ts.into_iter().flatten().collect();
        (!parts.is_empty()).then(|| &*Box::leak(parts.join("; ").into_boxed_str()))
    });
    let agg = (&kept)
        .group_by(origid.select(&type_by_id))
        .select(view_count.opt().and(score).and(tag_count.opt()).and(creation_date))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64, i64::MIN), |(n, vn, vs, s, tn, ts, last), (((v, sc), t), cd)| {
            (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0), s + sc, tn + t.is_some() as i64, ts + t.unwrap_or(0), last.max(cd))
        });
    let mut out = Vec::new();
    agg.drive(|name, (n, vn, vs, s, tn, ts, last)| {
        out.push(row(vec![
            V::S(name),
            V::I(n),
            avg(vs, vn),
            avg(s, n),
            nullable(ts, tn),
            ostr(titles.get(name).flatten()),
            V::T(last),
        ]))
    });
    rows(out)
}

fn q28106(db: &'static So) -> String {
    let Post { post_type_id, view_count, tags_str, creation_date, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let base = owned(db).with(post_type_id.eq(1)).with(view_count.gt(1000));
    let rn = (&base)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(view_count).and(creation_date))
        .window(row_number, |((_, v), cd)| (v, cd), desc);
    let edits = db.post_history.with(post_history_type_id.is_in([4, 5]));
    let latest: HashIdx<Id<Post>, Id<PostHistory>> = (&edits)
        .group_by(post)
        .select(Ident::<PostHistory>::new().and(hd))
        .window(rank, |(_, d)| d, desc)
        .filt(|(_, r)| r == 1)
        .map(|((h, _), _)| h)
        .collect();
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 3)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(&latest))
        .drive(|_, (p, h)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "tags", "owner"]);
        f.extend([ostr(comment.get(h)), V::T(hd.get(h).unwrap())]);
        out.push(row(f))
    });
    rows(out)
}

fn title_prefix(t: Str) -> Str {
    match t.char_indices().nth(10) {
        Some((i, _)) => &t[..i],
        None => t,
    }
}

fn q28390(db: &'static So) -> String {
    let Post { post_type_id, view_count, title, creation_date, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let base = owned(db).with(post_type_id.eq(1)).with(view_count.gt(100));
    let rn = (&base)
        .group_by(title.map(title_prefix).opt())
        .select(Ident::<Post>::new().and(&up))
        .window(row_number, |(_, u)| u, desc);
    let mut v = Vec::new();
    (&rn)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and((&up).and(creation_date)))
        .drive(|_, (p, (u, cd))| v.push((u, cd, p)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(u, _, p)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.push(V::I(u));
        f.extend(post_fields(db, p, &["created", "answers", "views"]));
        row(f)
    }))
}

fn q9211(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1));
    let e = engagement(db, &base);
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(score).and(&e))
        .window(row_number, |((_, s), e)| (s, e.0), desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((p, _), (c, v, _, _)), _)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([V::I(c), V::I(v)]);
        out.push(row(f))
    });
    rows(out)
}

fn q5936(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let Vote { post, .. } = &db.vote;
    let base = db.post.with(creation_date.ge(add_years(utc_to_ny(now_utc()), -1)));
    let votes_of: HashIdx<Id<Post>, Id<Vote>> = post.inv().collect();
    let joined: MatSet<(Id<Post>, Option<Id<Vote>>)> = (&base).select(Ident::<Post>::new().and(votes_of.opt())).collect();
    let post_of = (&joined).map(|(p, _)| p);
    type Key = ((i64, i64), (Id<Post>, Option<Id<Vote>>));
    let rn = (&joined)
        .group_by((&post_of).select(post_type))
        .select(Same::new().and((&post_of).select(score.and(creation_date))))
        .window(row_number, |(pv, sc)| (sc, pv), |x: &Key, y: &Key| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 5).drive(|_, (((p, _), _), r)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "owner"]);
        f.extend([V::I(up.get(p).unwrap()), V::I(down.get(p).unwrap()), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q8689(db: &'static So) -> String {
    let Post { post_type, post_type_id, creation_date, .. } = &db.post;
    let cc = comments_per_post(db);
    let base = owned(db).with(creation_date.ge(ts(2024, 10, 1, 12, 34, 56) - 30 * DAY_US)).with(post_type_id.is_in([1, 2]));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 10).map(|((p, _), _)| p)).and(&cc).drive(|p, ((_, _, u, d), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q9511(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let badges = (&db.badge.user).inv().fold(0i64, |a, _| a + 1);
    let by_name: HashIdx<Str, i64> = db.user.select(&db.user.display_name).inv().select(&badges).collect();
    let base = owned(db).with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(by_name.opt())))
        .drive(|_, (p, b)| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.push(oint(b));
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("12940", q12940),
    ("11501", q11501),
    ("10593", q10593),
    ("28976", q28976),
    ("28106", q28106),
    ("28390", q28390),
    ("9211", q9211),
    ("5936", q5936),
    ("8689", q8689),
    ("9511", q9511),
];
