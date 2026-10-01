use harness::prelude::*;

fn year_ago() -> i64 {
    add_years(ts(2024, 10, 1, 12, 34, 56), -1)
}

fn q9737(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;
    let bc = badges_per_user(db);
    let questions = db.post.with(post_type_id.eq(1));
    let popular = whole(&questions)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let best = (&popular).filt(|(_, n)| n == 1).map(|(((p, _), _), _)| p);
    let posts = owner_user.inv().fold(0i64, |a, _| a + 1);
    let rk = whole(&db.user.id)
        .select(display_name.and(reputation).and(&bc).and(posts.opt()))
        .window(rank, |(((_, rep), _), _)| rep, desc);
    let mut out = Vec::new();
    (&rk).filt(|((((_, rep), _), _), _)| rep > 1000).and(best.opt()).drive(|_, (((((dn, rep), b), n), r), p)| {
        let mut f = vec![V::S(dn), V::I(rep), V::I(b), V::I(r)];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score"]),
            None => vec![V::Null, V::Null],
        });
        f.push(oint(n));
        out.push(row(f))
    });
    rows(out)
}

fn q6907(db: &'static So) -> String {
    let Post { post_type, score, creation_date, .. } = &db.post;
    let PostHistory { post, creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let activity = post.inv().select(hd.and(post_history_type_id)).fold((i64::MIN, 0i64, 0i64), |(last, cl, hot), (d, t)| {
        (last.max(d), cl + (t == 10 || t == 11) as i64, hot + (t == 52 || t == 53) as i64)
    });
    let high_then_old = |a: &(i64, i64), b: &(i64, i64)| b.0.cmp(&a.0).then(a.1.cmp(&b.1));
    let rk = db
        .post
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), cd)| (s, cd), high_then_old);
    let mut out = Vec::new();
    engagement(db, (&rk).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p)).and(activity.opt()).drive(|p, ((c, v, _, _), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(v)]);
        match a {
            Some((last, cl, hot)) => f.extend([V::T(last), V::I(cl), V::I(hot)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        out.push(row(f))
    });
    rows(out)
}

fn q12294(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let rn = whole(db.post.iq())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 100).map(|(((p, _), _), _)| p).with(&db.post.owner_user)).drive(|p, (c, v, _, _)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id", "owner"]);
        f.extend([V::I(c), V::I(v)]);
        f.extend(post_fields(db, p, &["created", "activity", "views", "score"]));
        out.push(row(f))
    });
    rows(out)
}

fn q12254(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let User { origid, reputation, up_votes, down_votes, .. } = &db.user;
    let nu = db.user.id.n;
    let posts = owner_user.inv().dense_fold_outer(nu, 0i64, |a, _| a + 1);
    let comments = owner_user.inv().select(comments_of(db)).dense_fold_outer(nu, 0i64, |a, _| a + 1);
    let joined = db
        .user
        .select(posts_of(db).select(post_type_id.and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .dense_fold_outer(nu, (0i64, 0i64, 0i64), |(q, a, b), (p, bi)| {
            let t = p.map(|(t, _)| t);
            (q + (t == Some(1)) as i64, a + (t == Some(2)) as i64, b + bi.is_some() as i64)
        });
    let stats = (&posts).and(&comments).and(&joined).and(up_votes.and(down_votes)).map(|(((n, c), (q, a, b)), (u, d))| {
        (u - d, n, c, q, a, b)
    });
    let ranked = whole(&db.user.id)
        .select(origid.and(reputation).and(&stats))
        .window(rank, |((_, rep), _)| rep, desc)
        .window(rank, |(((_, _), s), _)| s.0, desc)
        .window(rank, |((((_, _), s), _), _)| s.1, desc);
    let mut v = Vec::new();
    ranked.drive(|_, x| v.push(x));
    v.sort_by(|a, b| (b.0.0.0.0.1, b.0.0.0.1.0).cmp(&(a.0.0.0.0.1, a.0.0.0.1.0)));
    rows(v.iter().take(100).map(|&(((((id, rep), (vs, n, c, q, a, b)), x), y), z)| {
        row(vec![V::I(id), V::I(rep), V::I(vs), V::I(n), V::I(c), V::I(q), V::I(a), V::I(b), V::I(x), V::I(y), V::I(z)])
    }))
}

fn q8358(db: &'static So) -> String {
    let Post { post_type, post_type_id, score, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let cc = comments_per_post(db);
    let base = owned(db).with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base).group_by(post_type).select(Ident::<Post>::new().and(score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and((&up).and(&down).and(&cc)))
        .drive(|_, (p, ((u, d), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(u), V::I(d), V::I(c)]);
        out.push(row(f))
    });
    rows(out)
}

fn q28369(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let vc = votes_per_post(db);
    let questions = owned(db).with(post_type_id.eq(1));
    let per_user = (&questions).select(owner_user).inv().select(score).fold((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let top: MatSet<Id<User>> = whole(&per_user)
        .select(Ident::<User>::new().and(&per_user))
        .window(rank, |(_, a)| a.0, desc)
        .filt(|(_, r)| r <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let mut out = Vec::new();
    (&questions).select(owner_user.with(&top).select(&per_user).and(owner_user).and(&vc)).drive(|p, (((s, n), u), v)| {
        let mut f = vec![V::S(db.user.display_name.get(u).unwrap()), V::I(n), V::I(s)];
        f.extend(post_fields(db, p, &["title"]));
        f.push(V::I(v));
        out.push(row(f))
    });
    rows(out)
}

fn q6033(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, owner_user_id, .. } = &db.post;
    let bc = badges_per_user(db);
    let questions = db.post.with(post_type_id.eq(1));
    let rn = (&questions)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(_, s)| s, desc);
    let top = (&rn).filt(|(_, n)| n <= 5).map(|((p, _), _)| p).select(owner_user).inv().fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user.select((&db.user.display_name).and(&top).and(&bc)).drive(|_, ((dn, n), b)| v.push((n, b, dn)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(n, b, dn)| row(vec![V::S(dn), V::I(n), V::I(b)])))
}

fn q6432(db: &'static So) -> String {
    let Post { post_type, creation_date, score, origid, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(year_ago()));
    let top: MatSet<Id<Post>> = (&base)
        .group_by(post_type.select(&db.post_type.name))
        .select(Ident::<Post>::new().and(score).and(origid))
        .window(row_number, |((_, s), id)| (s, id), |a: &(i64, i64), b: &(i64, i64)| b.0.cmp(&a.0).then(a.1.cmp(&b.1)))
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let rk = whole(&top).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rk).drive(|_, ((p, _), r)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views", "score", "answers", "comments"]);
        f.push(V::S(match r {
            ..=3 => "Top Performer",
            4..=10 => "Notable",
            _ => "Average",
        }));
        out.push(row(f))
    });
    rows(out)
}

fn q1971(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user, .. } = &db.post;
    let eng = engagement(db, db.post.with(view_count.gt(100)).with(owner_user));
    let rn = db
        .post
        .group_by(post_type.select(&db.post_type.name))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()).and((&eng).opt()))
        .window(row_number, |(((_, s), _), _)| s, desc);
    let mut v = Vec::new();
    (&rn)
        .filt(|((_, e), _)| e.is_some_and(|(_, _, u, d)| u - d > 0))
        .drive(|_, ((((p, s), w), e), r)| {
            let (c, _, u, d) = e.unwrap();
            v.push((w.unwrap(), s, p, r, u - d, c))
        });
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().skip(10).take(10).map(|&(_, _, p, r, net, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.extend([V::I(net), V::I(c)]);
        f.push(V::S(match r {
            1 => "Top Post",
            2..=5 => "Top 5 Post",
            _ => "Other Post",
        }));
        row(f)
    }))
}

fn q5983(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, score, view_count, answer_count, .. } = &db.post;
    let cc = comments_per_post(db);
    let questions = db.post.with(post_type_id.eq(1));
    let rn = (&questions)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let tu = (&questions).select(owner_user).inv().select(score.and(view_count.opt()).and(answer_count.opt())).fold(
        (0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, s, vn, vs, an, asum), ((sc, v), a)| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0), an + a.is_some() as i64, asum + a.unwrap_or(0))
        },
    );
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.and(owner_user.select((&tu).filt(|a| a.0 > 10)))).and(&cc))
        .drive(
        |_, ((p, (u, (_, s, vn, vs, an, asum))), c)| {
            let mut f = vec![
                V::I(db.user.origid.get(u).unwrap()),
                V::S(db.user.display_name.get(u).unwrap()),
                V::I(s),
                nullable(vs, vn),
                nullable(asum, an),
            ];
            f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
            f.push(V::I(c));
            out.push(row(f))
        },
    );
    rows(out)
}

fn q7937(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, answer_count, origid, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(year_ago())).with(score.ge(5));
    let type_by_id: HashIdx<i64, Str> = (&db.post_type.origid).inv().select(&db.post_type.name).collect();
    let _ranks = (&base).group_by(post_type).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let agg = (&base).group_by(origid.select(&type_by_id)).select(score.and(view_count.opt()).and(answer_count.opt())).fold(
        (0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, s, vn, vs, an, asum), ((sc, v), a)| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0), an + a.is_some() as i64, asum + a.unwrap_or(0))
        },
    );
    let mut out = Vec::new();
    agg.drive(|name, (n, s, vn, vs, an, asum)| {
        let level = match n {
            51.. => "High Activity",
            20..=50 => "Moderate Activity",
            _ => "Low Activity",
        };
        out.push(row(vec![V::S(name), V::I(n), avg(s, n), avg(vs, vn), nullable(asum, an), V::S(level)]))
    });
    rows(out)
}

fn q25507(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, tags_str, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, user_display_name, comment, text, .. } = &db.post_history;
    let edits: HashIdx<Id<Post>, Id<PostHistory>> = db
        .post_history
        .with(post_history_type_id.is_in([4, 5, 6]))
        .with(hd.ge(date(2023, 1, 1)))
        .select(post)
        .inv()
        .collect();
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(date(2023, 1, 1))).with(view_count.gt(100));
    let rk = (&base)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(rank, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r == 1)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(edits.opt()))
        .drive(|_, (p, h)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "answers", "tags"]);
        match h {
            Some(h) => f.extend([V::T(hd.get(h).unwrap()), ostr(user_display_name.get(h)), ostr(comment.get(h)), ostr(text.get(h))]),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("9737", q9737),
    ("6907", q6907),
    ("12294", q12294),
    ("12254", q12254),
    ("8358", q8358),
    ("28369", q28369),
    ("6033", q6033),
    ("6432", q6432),
    ("1971", q1971),
    ("5983", q5983),
    ("7937", q7937),
    ("25507", q25507),
];
