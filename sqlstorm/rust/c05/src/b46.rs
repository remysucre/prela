use harness::prelude::*;

fn year_ago() -> i64 {
    add_years(ts(2024, 10, 1, 12, 34, 56), -1)
}

fn q26862(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, tags_str, accepted_answer_id, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;
    let base = db.post.with(post_type_id.eq(1));
    let rk = (&base).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rk)
        .and(display_name.and(reputation))
        .filt(|((_, r), (_, rep))| r <= 5 && rep > 1000)
        .drive(|_, (((p, _), r), (dn, rep))| {
            let mut f = vec![V::S(dn), V::I(rep)];
            f.extend(post_fields(db, p, &["title", "created", "views", "score"]));
            f.push(oint(tags_str.get(p).map(|t| t.matches("><").count() as i64 + 1)));
            f.push(V::I(accepted_answer_id.get(p).unwrap_or(-1)));
            f.push(V::I(r));
            out.push(row(f))
        });
    rows(out)
}

fn q9705(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let base = owned(db).with(creation_date.ge(year_ago()));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(&cc).and(&up).and(&down))
        .drive(|_, (((p, c), u), d)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q5006(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, answer_count, .. } = &db.post;
    let agg = db
        .post
        .group_by(post_type)
        .select(score.and(view_count.opt()).and(answer_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, an, asum), ((sc, v), a)| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0), an + a.is_some() as i64, asum + a.unwrap_or(0))
        });
    let base = owned(db).with(creation_date.ge(date(2024, 9, 1)));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(post_type.select(&agg)))
        .drive(|_, (p, (n, s, vn, vs, an, asum))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::I(n), V::I(s), avg(vs, vn), avg(asum, an)]);
        out.push(row(f))
    });
    rows(out)
}

fn votes_named(db: &'static So, name: Str) -> DenseFold<Id<Post>, i64> {
    db.vote.with(vtype_name(db).eq(name)).select(&db.vote.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

fn q5891(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_named(db, "UpMod");
    let down = votes_named(db, "DownMod");
    let base = owned(db).with(creation_date.ge(year_ago()));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(&cc).and(&up).and(&down))
        .drive(|_, (((p, c), u), d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q7358(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(year_ago())).with(view_count.gt(100));
    let rn = (&base)
        .group_by(post_type.select(&db.post_type.name))
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(_, s)| s, desc);
    let cats = db
        .post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (sc, v)| (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0)));
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n == 1).cross(&cats).drive(|(_, name), (((p, _), _), (n, s, vn, vs))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score", "answers", "comments"]);
        f.extend([V::S(name), avg(s, n), nullable(vs, vn)]);
        out.push(row(f))
    });
    rows(out)
}

fn q7647(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, creation_date, .. } = &db.post;
    let pv = (&db.vote.post).inv().select(vtype_name(db)).fold((0i64, 0i64, 0i64), |(u, d, a), t| {
        (u + (t == "UpMod") as i64, d + (t == "DownMod") as i64, a + (t == "AcceptedByOriginator") as i64)
    });
    let base = owned(db).with(post_type_id.eq(1));
    let score_then_oldest = |a: &(i64, i64), b: &(i64, i64)| b.0.cmp(&a.0).then(a.1.cmp(&b.1));
    let rk = (&base)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), cd)| (s, cd), score_then_oldest);
    let mut v = Vec::new();
    (&rk)
        .filt(|(_, r)| r == 1)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(score.and(creation_date).and(&pv)))
        .drive(|_, (p, x)| v.push((p, x)));
    v.sort_by(|a, b| score_then_oldest(&a.1.0, &b.1.0));
    rows(v.iter().take(50).map(|&(p, (_, (u, d, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        f.extend([V::I(u), V::I(d), V::I(a)]);
        row(f)
    }))
}

fn q5261(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(year_ago()));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let stats = (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (sc, v)| (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0)));
    let rk = whole(&stats).select(Same::new().and(&stats)).window(rank, |(_, a)| a.0, desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 10).drive(|_, ((dn, (n, s, vn, vs)), r)| {
        out.push(row(vec![V::I(r), V::S(dn), V::I(n), avg(s, n), nullable(vs, vn)]))
    });
    rows(out)
}

fn q11085(db: &'static So) -> String {
    let Post { post_type, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(year_ago()));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 10).map(|((p, _), _)| p)).drive(|p, (c, _, u, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        let owner = db.post.owner_user.get(p);
        f.push(oint(owner.map(|u| db.user.origid.get(u).unwrap())));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend([V::I(c), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q6765(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(year_ago()));
    let rk = whole(&base).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(&up).and(&down))
        .drive(|_, ((p, u), d)| {
        let mut f = post_fields(db, p, &["title", "owner", "views", "score", "answers", "comments"]);
        f.extend([V::I(u), V::I(d)]);
        f.extend(post_fields(db, p, &["created"]));
        out.push(row(f))
    });
    rows(out)
}

fn q6109(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1)).with(creation_date.ge(year_ago()));
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let counts = (&rn)
        .filt(|(_, n)| n <= 50)
        .map(|(((p, _), _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).with(post_type_id.eq(2)).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64, 0i64), |(a, u, d), (ai, vt)| {
            (a + ai.is_some() as i64, u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64)
        });
    let mut out = Vec::new();
    (&counts).drive(|p, (a, u, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a), V::I(u), V::I(d)]);
        f.push(V::S(match u - d {
            x if x >= 10 => "Highly Upvoted",
            1..=9 => "Moderately Upvoted",
            _ => "Needs Improvement",
        }));
        out.push(row(f))
    });
    rows(out)
}

fn q9466(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, .. } = &db.post;
    let mentions = tag_mentions_ci(db);
    let stats = (&mentions)
        .group_by((&mentions).map(|(_, t)| t).select(&db.tag.tag_name))
        .select((&mentions).map(|(p, _)| p).select(view_count.opt()))
        .fold((0i64, 0i64, 0i64), |(n, vn, vs), v| (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0)));
    let base = owned(db).with(creation_date.ge(year_ago()));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 5).cross((&stats).filt(|(n, _, _)| n > 10)).drive(|(_, t), ((((p, _), _), _), (_, vn, vs))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.extend([V::S(t), nullable(vs, vn)]);
        out.push(row(f))
    });
    rows(out)
}

fn tag_mentions_ci(db: &'static So) -> MatSet<(Id<Post>, Id<Tag>)> {
    let elems: MatSet<Str> = (&db.post.tags_str).flat_map(tag_list).collect();
    let contains: HashIdx<Str, Id<Tag>> =
        (&elems).select_where((&db.tag.tag_name).inv(), |e: Str, n: Str| e.to_lowercase().contains(&n.to_lowercase())).collect();
    db.post.select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&contains))).collect()
}

pub const ENTRIES: &[harness::Entry] = &[
    ("26862", q26862),
    ("9705", q9705),
    ("5006", q5006),
    ("5891", q5891),
    ("7358", q7358),
    ("7647", q7647),
    ("5261", q5261),
    ("11085", q11085),
    ("6765", q6765),
    ("6109", q6109),
    ("9466", q9466),
];
