use harness::prelude::*;

fn q12297(db: &'static So) -> String {
    let rn = whole(db.post.iq())
        .select(Ident::<Post>::new().and(&db.post.creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 100).map(|((p, _), _)| p)).drive(|p, (c, v, _, _)| {
        let mut r = post_fields(db, p, &["id", "title", "views", "created", "score"]);
        r.extend([V::I(c), V::I(v)]);
        out.push(row(r))
    });
    rows(out)
}

fn q11287(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1));
    let eng = engagement(db, &base);
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(creation_date).and(score).and(&eng))
        .window(row_number, |(((_, cd), s), _)| (cd, s), desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 100).drive(|_, ((((p, _), _), (c, v, _, _)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(v), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q13085(db: &'static So) -> String {
    let Post { title, view_count, score, owner_user, creation_date, .. } = &db.post;
    let rep = owner_user.select(&db.user.reputation);
    let agg = db
        .post
        .with(creation_date.ge(date(2023, 1, 1)))
        .group_by(title.opt().and(view_count.opt()).and(score).and(rep.opt()))
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold((0i64, 0i64, 0i64), |(c, v, b), ((ci, vi), bi)| {
            (c + ci.is_some() as i64, v + vi.is_some() as i64, b + bi.is_some() as i64)
        });

    let rn = whole(&agg)
        .select(Same::new().and(&agg))
        .window(
            row_number,
            |((((_, vw), s), _), _): ((((Option<Str>, Option<i64>), i64), Option<i64>), (i64, i64, i64))| (s, vw),
            desc,
        );
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((((t, vw), s), r), (c, vo, b)), n)| {
        out.push(row(vec![ostr(t), oint(vw), V::I(s), oint(r), V::I(c), V::I(vo), V::I(b), V::I(n)]))
    });
    rows(out)
}

fn q14980(db: &'static So) -> String {
    let Post { owner_user_id, view_count, score, .. } = &db.post;
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let counts = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(owner_user_id.select(&bidx).opt()))
        .fold((0i64, 0i64), |(c, b), (ci, bi)| (c + ci.is_some() as i64, b + bi.is_some() as i64));
    let ranked = whole(db.post.iq())
        .select(Ident::<Post>::new().and(view_count.opt()).and(score).and(&counts))
        .window(rank, |(((_, v), _), _)| v, desc)
        .window(rank, |((((_, _), s), _), _)| s, desc);
    let mut out = Vec::new();
    ranked.drive(|_, (((((p, _), _), (c, b)), vr), sr)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(b), V::I(vr), V::I(sr)]);
        out.push(row(f))
    });
    rows(out)
}

fn q10980(db: &'static So) -> String {
    let rn = db
        .post
        .group_by(&db.post.post_type)
        .select(Ident::<Post>::new().and(&db.post.creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 10).map(|((p, _), _)| p)).drive(|p, (c, v, _, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(v)]);
        out.push(row(f))
    });
    rows(out)
}

fn q14668(db: &'static So) -> String {
    let rn = whole(db.post.iq())
        .select(Ident::<Post>::new().and(&db.post.creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 100).map(|((p, _), _)| p)).drive(|p, (c, v, _, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(v)]);
        f.extend(post_fields(db, p, &["answers"]));
        out.push(row(f))
    });
    rows(out)
}

fn q13827(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let rn = whole(db.post.iq())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 100).map(|(((p, _), _), _)| p)).drive(|p, (c, v, _, _)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        f.extend([V::I(c), V::I(v)]);
        out.push(row(f))
    });
    rows(out)
}

fn q7171(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = (&base)
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .window(row_number, |(s, _)| s, desc);

    let agg = (&db.user.display_name)
        .inv()
        .select(&rn)
        .fold((0i64, 0i64, 0i64, 0i64, i64::MIN, 0i64), |(n, s, vn, vs, vm, rs), ((sc, v), r)| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0), vm.max(v.unwrap_or(i64::MIN)), rs + r)
        });
    let mut v = Vec::new();
    (&agg).filt(|a| a.0 > 5).drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.1.1.cmp(&a.1.1));
    rows(v.iter().take(10).map(|&(k, (n, s, vn, vs, vm, rs))| {
        row(vec![
            V::S(k),
            V::I(n),
            V::I(s),
            avg(vs, vn),
            if vn == 0 { V::Null } else { V::I(vm) },
            avg(rs, n),
        ])
    }))
}

fn q12878(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date, .. } = &db.post_history;
    let ranked: MatSet<(Id<Post>, i64, i64)> = whole(owned(db))
        .select(Ident::<Post>::new().and(view_count.opt()).and(score))
        .window(rank, |((_, v), _)| v, desc)
        .window(rank, |(((_, _), s), _)| s, desc)
        .filt(|((_, v), s)| v <= 10 || s <= 10)
        .map(|((((p, _), _), vr), sr)| (p, vr, sr))
        .collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64, i64)> = (&ranked).map(|(p, _, _)| p).inv().collect();
    let history: HashIdx<Id<Post>, Id<PostHistory>> = (&db.post_history.post).inv().collect();

    let mut out = Vec::new();
    (&by_post)
        .and(history.opt())
        .drive(|p, ((_, vr, sr), h)| {
            let mut f =
                post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "owner"]);
            f.extend([V::I(vr), V::I(sr)]);
            match h {
                Some(h) => f.extend([
                    V::I(post_history_type_id.get(h).unwrap()),
                    V::T(creation_date.get(h).unwrap()),
                ]),
                None => f.extend([V::Null, V::Null]),
            }
            out.push(row(f))
        });
    rows(out)
}

fn q14426(db: &'static So) -> String {
    let rn = whole(db.post.iq())
        .select(Ident::<Post>::new().and(&db.post.creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 100).map(|((p, _), _)| p)).drive(|p, (c, v, _, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(v)]);
        out.push(row(f))
    });
    rows(out)
}

fn q13144(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1));
    let counts = (&base)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).with(post_type_id.eq(2)).opt()))
        .fold((0i64, 0i64), |(c, a), (ci, ai)| (c + ci.is_some() as i64, a + ai.is_some() as i64));
    let sr = whole(&base)
        .select(Ident::<Post>::new().and(creation_date).and(score).and(&counts))
        .window(row_number, |(((_, cd), _), _)| cd, desc)
        .filt(|(_, n)| n <= 100)
        .window(dense_rank, |((((_, _), s), _), _)| s, desc);
    let mut out = Vec::new();
    sr.drive(|_, (((((p, _), _), (c, a)), _), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q12645(db: &'static So) -> String {
    let eng = engagement(db, db.post.iq());
    let sr = whole(db.post.iq())
        .select(Ident::<Post>::new().and(&db.post.score).and(&eng))
        .window(dense_rank, |((_, s), _)| s, desc);
    let mut v = Vec::new();
    sr.drive(|_, (((p, _), e), r)| v.push((p, (e, r))));
    let v = top_n(v, |&(_, (_, r))| r, 100);
    rows(v.iter().map(|&(p, ((c, vo, _, _), r))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c), V::I(vo)]);
        f.extend(post_fields(db, p, &["created", "score", "views", "answers", "type"]));
        f.push(V::I(r));
        row(f)
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("12297", q12297),
    ("11287", q11287),
    ("13085", q13085),
    ("14980", q14980),
    ("10980", q10980),
    ("14668", q14668),
    ("13827", q13827),
    ("7171", q7171),
    ("12878", q12878),
    ("14426", q14426),
    ("13144", q13144),
    ("12645", q12645),
];
