use harness::prelude::*;

fn year_ago() -> i64 {
    add_years(ts(2024, 10, 1, 12, 34, 56), -1)
}

fn comments_and_answers<Q: Drive<R = Id<Post>>>(db: &'static So, posts: Q) -> Fold<Id<Post>, (i64, i64)> {
    posts
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).with((&db.post.post_type_id).eq(2)).opt()))
        .fold((0i64, 0i64), |(c, a), (ci, ai)| (c + ci.is_some() as i64, a + ai.is_some() as i64))
}

fn q8179(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let bc = badges_per_user(db);
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(date(2023, 10, 1)));
    let rn = whole(&base).select(Ident::<Post>::new().and(score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and((&cc).and(&up).and(owner_user.select(&bc))))
        .drive(|_, (p, ((c, u), b))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score", "answers", "comments", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(b)]);
        out.push(row(f))
    });
    rows(out)
}

fn q28962(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let questions = db.post.with(post_type_id.eq(1));
    let counts = comments_and_answers(db, &questions);
    let ranked: MatSet<(Id<Post>, i64, i64)> = whole(&questions)
        .select(Ident::<Post>::new().and(&counts))
        .window(rank, |(_, x)| x.0, desc)
        .window(rank, |((_, x), _)| x.1, desc)
        .map(|(((p, _), cr), ar)| (p, cr, ar))
        .collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64, i64)> = (&ranked).map(|(p, _, _)| p).inv().collect();
    let rn = (&questions)
        .group_by((&by_post).map(|(_, x, y)| (x, y)))
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(&counts).and(&by_post))
        .drive(|_, ((p, (c, a)), (_, x, y))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags"]);
        f.extend([V::I(c), V::I(a)]);
        f.extend(post_fields(db, p, &["created", "score"]));
        f.extend([V::I(x), V::I(y)]);
        out.push(row(f))
    });
    rows(out)
}

fn q5680(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, parent, .. } = &db.post;
    let ac = parent.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let cc = comments_per_post(db);
    let base = db.post.with(post_type_id.eq(1)).with(creation_date.ge(ts(2024, 10, 1, 12, 34, 56) - 30 * DAY_US));
    let votes = (&base)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(u, d), (_, vt)| (u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64));
    let rk = whole(&base)
        .select(Ident::<Post>::new().and(&votes).and(score))
        .window(rank, |((_, (u, d)), _)| u - d, desc);
    let mut out = Vec::new();
    (&rk).filt(|((_, s), _)| s > 10).drive(|_, (((p, (u, d)), _), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(ac.get(p).unwrap()), V::I(cc.get(p).unwrap()), V::I(u), V::I(d)]);
        f.push(V::S(match r {
            ..=10 => "Top 10",
            11..=50 => "Top 50",
            _ => "Below 50",
        }));
        out.push(row(f))
    });
    rows(out)
}

fn q6339(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, view_count, post_type, .. } = &db.post;
    let recent = db.post.with(creation_date.ge(year_ago())).with(score.gt(0));
    let stats = (&recent).select(owner_user).inv().select(score.and(view_count.opt())).fold(
        (0i64, 0i64, 0i64, 0i64),
        |(n, s, vn, vs), (sc, v)| (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0)),
    );
    let top: MatSet<Id<User>> = whole(&stats)
        .select(Ident::<User>::new().and(&stats))
        .window(rank, |(_, a)| a.1, desc)
        .filt(|(_, r)| r <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let mut out = Vec::new();
    db.post.select(owner_user.and(owner_user.with(&top).select(&stats)).and(post_type.select(&db.post_type.name))).drive(
        |_, ((u, (n, s, vn, vs)), name)| {
            out.push(row(vec![V::S(db.user.display_name.get(u).unwrap()), V::I(n), V::I(s), nullable(vs, vn), V::S(name)]))
        },
    );
    rows(out)
}

fn q28150(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, view_count, body, .. } = &db.post;
    let questions = db.post.with(post_type_id.eq(1));
    let rn = (&questions)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let latest = (&rn).filt(|(_, n)| n == 1).map(|((p, _), _)| p).with(view_count.gt(100));
    let mut v = Vec::new();
    comments_and_answers(db, &latest).filt(|(_, a)| a < 5).and(view_count).drive(|p, ((c, a), w)| v.push((w, p, a, c)));
    v.sort_by(|x, y| y.0.cmp(&x.0));
    rows(v.iter().take(10).map(|&(_, p, a, c)| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "views"]);
        f.extend([V::I(a), V::I(c)]);
        let b = body.get(p).unwrap();
        f.push(V::S(match b.char_indices().nth(200) {
            Some((i, _)) => &b[..i],
            None => b,
        }));
        row(f)
    }))
}

fn q26246(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let bc = badges_per_user(db);
    let base = owned(db).with(post_type_id.eq(1));
    let rk = whole(&base).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.select((&bc).gt(0))))
        .drive(|_, (p, b)| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "created", "score", "owner"]);
        f.push(V::I(b));
        out.push(row(f))
    });
    rows(out)
}

fn q22599(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, .. } = &db.post;
    let base = db.post.with(creation_date.ge(year_ago()));
    let joined: MatSet<(Id<Post>, Option<Id<Comment>>, Option<Id<Vote>>)> = (&base)
        .select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt()))
        .map(|((p, c), v)| (p, c, v))
        .collect();
    let post_of = (&joined).map(|(p, _, _)| p);
    let vote_type_of = (&joined).map(|(_, _, v)| v).flat_map(|v| v).select(&db.vote.vote_type_id);
    let per_post = (&joined)
        .group_by(&post_of)
        .select((&joined).map(|(_, c, _)| c.is_some()).and(vote_type_of.opt()))
        .fold((0i64, 0i64, 0i64), |(c, u, d), (ci, vt)| (c + ci as i64, u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64));
    type Key = (i64, (Id<Post>, Option<Id<Comment>>, Option<Id<Vote>>));
    let rn = (&joined)
        .group_by((&post_of).select(post_type))
        .select(Same::new().and((&post_of).select(creation_date)))
        .window(row_number, |(t, cd)| (cd, t), |x: &Key, y: &Key| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
    let mut v = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((t, _), _)| t)
        .select(Same::new().and((&post_of).select(view_count.opt().and(score).and(&per_post))))
        .drive(|_, ((p, _, _), x)| v.push((p, x)));
    v.sort_by(|a, b| (b.1.0.0, b.1.0.1).cmp(&(a.1.0.0, a.1.0.1)));
    rows(v.iter().take(50).map(|&(p, ((w, s), (c, u, d)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        f.push(V::S(if s < 0 { "Negative Score" } else { "Positive Score" }));
        f.push(V::S(match w {
            Some(x) if x < 10 => "Low View Count",
            Some(10..=100) => "Moderate View Count",
            _ => "High View Count",
        }));
        row(f)
    }))
}

fn q9772(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let Vote { post, vote_type_id, .. } = &db.vote;
    let pv = post.inv().select(vote_type_id).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64));
    let base = owned(db).with(creation_date.ge(date(2024, 9, 1))).with(score.gt(0));
    let rk = (&base).group_by(post_type).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(score.and(&pv)))
        .drive(|_, (p, (s, (u, d)))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(u), V::I(d)]);
        f.extend(post_fields(db, p, &["owner"]));
        f.push(V::S(match s {
            100.. => "Hot",
            50..=99 => "Trending",
            _ => "New",
        }));
        out.push(row(f))
    });
    rows(out)
}

fn q9370(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, owner_user_id, creation_date, view_count, .. } = &db.post;
    let stats = db
        .user
        .select(badges_of(db).opt().and(posts_of(db).select(view_count.opt().and(score)).opt()))
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64, 0i64), |(n, vn, vs, s), (_, p)| {
            let w = p.and_then(|(v, _)| v);
            (n + p.is_some() as i64, vn + w.is_some() as i64, vs + w.unwrap_or(0), s + p.map_or(0, |(_, sc)| sc))
        })
        .map(|(n, vn, vs, s)| ((vn > 0).then_some(vs), (n > 0).then_some(s)));
    let base = db.post.with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut v = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(view_count.opt().and(owner_user.select((&stats).and(&db.user.reputation)))))
        .drive(|_, (p, (w, ((tv, tsc), rep)))| v.push((tsc, w, p, tv, rep)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(tsc, _, p, tv, rep)| {
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.extend([oint(tv), oint(tsc), V::I(rep)]);
        row(f)
    }))
}

fn q6545(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let Badge { name, class, .. } = &db.badge;
    let cc = comments_per_post(db);
    let votes = (&db.vote.post).inv().select(&db.vote.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let badges_of: HashIdx<Id<User>, Id<Badge>> = (&db.badge.user).inv().collect();
    let base = db.post.with(post_type_id.eq(1)).with((&cc).gt(0));
    let rk = whole(&base)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.and(&cc).and(owner_user.select(badges_of.opt())).and(votes.opt())))
        .drive(|_, (p, (((u, c), b), v))| {
            let mut f = post_fields(db, p, &["id", "title"]);
            f.push(V::S(db.user.display_name.get(u).unwrap()));
            f.extend(post_fields(db, p, &["score", "views"]));
            f.push(V::I(c));
            match b {
                Some(b) => f.extend([V::I(db.badge.origid.get(b).unwrap()), V::S(name.get(b).unwrap()), V::I(class.get(b).unwrap())]),
                None => f.extend([V::Null, V::Null, V::Null]),
            }
            match v {
                Some((n, last)) => f.extend([V::I(n), V::T(last)]),
                None => f.extend([V::I(0), V::Null]),
            }
            out.push(row(f))
        });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("8179", q8179),
    ("28962", q28962),
    ("5680", q5680),
    ("6339", q6339),
    ("28150", q28150),
    ("26246", q26246),
    ("22599", q22599),
    ("9772", q9772),
    ("9370", q9370),
    ("6545", q6545),
];
