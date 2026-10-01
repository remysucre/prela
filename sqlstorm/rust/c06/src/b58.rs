use harness::prelude::*;

fn user_cols(db: &'static So, u: Option<Id<User>>, cols: &[&str]) -> Vec<V> {
    let User { origid, display_name, reputation, creation_date, location, about_me, .. } = &db.user;
    cols.iter()
        .map(|c| match (*c, u) {
            (_, None) => V::Null,
            ("id", Some(u)) => V::I(origid.get(u).unwrap()),
            ("name", Some(u)) => V::S(display_name.get(u).unwrap()),
            ("rep", Some(u)) => V::I(reputation.get(u).unwrap()),
            ("created", Some(u)) => V::T(creation_date.get(u).unwrap()),
            ("location", Some(u)) => ostr(location.get(u)),
            ("about", Some(u)) => ostr(about_me.get(u)),
            _ => panic!("user_cols: {c}"),
        })
        .collect()
}

fn q12642(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let vc = votes_per_post(db);
    let eng = engagement(db, db.post.iq());
    let rk = whole(db.post.iq())
        .select(Ident::<Post>::new().and(score).and(&eng).and(&vc).and(owner_user.opt()))
        .window(dense_rank, |((((_, s), _), _), _)| s, desc);
    let mut out = Vec::new();
    (&rk).drive(|_, (((((p, _), (c, _, _, _)), v), u), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites"]);
        f.extend(user_cols(db, u, &["id", "name", "rep", "created", "location", "about"]));
        f.extend([V::I(c), V::I(v), V::I(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q8795(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, owner_user, creation_date, .. } = &db.post;
    let classes = (&db.badge.user).inv().select(&db.badge.class).fold((0i64, 0i64), |(n, s), c| (n + 1, s + c));
    let questions = db.post.with(post_type_id.eq(1));
    let rn = (&questions)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 3).map(|((p, _), _)| p).with(owner_user)).and(owner_user).drive(|p, ((c, _, up, d), u)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(up), V::I(d)]);
        f.extend(user_cols(db, Some(u), &["name", "rep"]));
        f.push(oint(classes.get(u).map(|x| x.1)));
        out.push(row(f))
    });
    rows(out)
}

fn q6706(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, owner_user, score, creation_date, .. } = &db.post;
    let cc = comments_per_post(db);
    let bc = badges_per_user(db);
    let questions = db.post.with(post_type_id.eq(1));
    let rn = (&questions)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 3)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.select(&bc).and(&cc)))
        .drive(|_, (p, (b, c))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend([V::I(c), V::I(b)]);
        out.push(row(f))
    });
    rows(out)
}

fn q25989(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, owner_user, score, view_count, last_activity_date, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;
    let questions = db.post.with(post_type_id.eq(1));
    let latest: MatSet<Id<Post>> = (&questions)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(last_activity_date))
        .window(row_number, |(_, la)| la, desc)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .collect();
    let e = engagement(db, &latest);
    let tu = (&latest)
        .group_by(owner_user.select(display_name).opt())
        .select(score.and(view_count.opt()).and(&e))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, cs), ((sc, v), (c, _, _, _))| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0), cs + c)
        });
    let positive = (&tu).filt(|a| a.1 > 0);
    let names: MatSet<Option<Str>> = whole(&positive)
        .select(Same::new().and(&tu))
        .window(rank, |(_, a)| a.1, desc)
        .filt(|(_, r)| r <= 10)
        .map(|((k, _), _)| k)
        .collect();
    let joined: MatSet<(Id<User>, (i64, i64, i64, i64, i64))> =
        db.user.select(Ident::<User>::new().and(display_name.map(Some).with(&names).select(&tu))).collect();
    let orank = whole(&joined).select(Same::new()).window(rank, |(_, a)| a.1, desc);
    let mut out = Vec::new();
    (&orank).drive(|_, ((u, (n, s, vn, vs, cs)), r)| {
        out.push(row(vec![
            V::S(display_name.get(u).unwrap()),
            V::I(reputation.get(u).unwrap()),
            V::I(s),
            nullable(vs, vn),
            avg(cs, n),
            V::I(r),
        ]))
    });
    rows(out)
}

fn q7664(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, owner_user, score, creation_date, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let bc = badges_per_user(db);
    let phs = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).select(post).inv().select(hd).fold(
        (0i64, i64::MIN),
        |(n, m), d| (n + 1, m.max(d)),
    );
    let base = db.post.with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut v = Vec::new();
    (&rn)
        .filt(|(_, n)| n == 1)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(score.and(creation_date).and(owner_user)))
        .drive(|_, (p, ((s, cd), u))| v.push((s, cd, p, u)));
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    rows(v.iter().take(100).map(|&(_, _, p, u)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend(user_cols(db, Some(u), &["name", "rep"]));
        f.push(V::I(bc.get(u).unwrap()));
        match phs.get(p) {
            Some((n, last)) => f.extend([V::I(n), V::T(last)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

fn q28019(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, parent, last_activity_date, .. } = &db.post;
    let ac = parent.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let cc = comments_per_post(db);
    let questions = db.post.with(post_type_id.eq(1));
    let rk = (&questions)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(dense_rank, |(_, cd)| cd, desc);
    let votes = (&rk)
        .filt(|(_, r)| r <= 3)
        .map(|((p, _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(u, d), (_, vt)| (u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64));
    let cut = add_months(ts(2024, 10, 1, 12, 34, 56), -1);
    let mut out = Vec::new();
    (&votes).and(&ac).and(&cc).and(last_activity_date).drive(|p, ((((u, d), a), c), la)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "activity", "owner"]);
        f.extend([V::I(a), V::I(c), V::I(u), V::I(d)]);
        f.push(V::S(if a > 0 {
            "Answered"
        } else if la <= cut {
            "Inactive"
        } else {
            "Open"
        }));
        out.push(row(f))
    });
    rows(out)
}

fn q6588(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let base = owned(db).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(view_count.gt(100));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and((&cc).and(&up).and(&down)))
        .drive(|_, (p, ((c, u), d))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views", "answers"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::I(u - d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q14622(db: &'static So) -> String {
    let Post { score, post_type_id, view_count, owner_user, .. } = &db.post;
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let edits = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).select(post).inv().fold(0i64, |a, _| a + 1);
    let ats = owned(db)
        .with(post_type_id.eq(1))
        .group_by(post_type_id)
        .select(view_count.opt().and(owner_user.select(&db.user.reputation)))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, vn, vs, rs), (v, r)| (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0), rs + r));
    let rn = whole(owned(db)).select(Ident::<Post>::new().and(score)).window(row_number, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(edits.opt()))
        .cross(&ats)
        .drive(|_, ((p, e), (n, vn, vs, rs))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "rep", "owner"]);
        f.extend([oint(e), V::I(n), nullable(vs, vn), avg(rs, n)]);
        out.push(row(f))
    });
    rows(out)
}

fn q29798(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, answer_count, comment_count, creation_date, .. } = &db.post;
    let stats = db
        .post
        .with(post_type_id.eq(1))
        .select(owner_user)
        .inv()
        .select(view_count.opt().and(answer_count.opt()).and(comment_count).and(creation_date))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64, i64::MIN), |(n, vn, vs, an, asum, c, last), (((v, a), cm), cd)| {
            (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0), an + a.is_some() as i64, asum + a.unwrap_or(0), c + cm, last.max(cd))
        });
    let mut out = Vec::new();
    db.user.select((&db.user.origid).and(&db.user.display_name).and(&stats)).drive(|_, ((id, dn), (n, vn, vs, an, asum, c, last))| {
        out.push(row(vec![
            V::I(id),
            V::S(dn),
            V::I(n),
            nullable(vs, vn),
            avg(vs, vn),
            nullable(asum, an),
            V::I(c),
            V::T(last),
            V::S(match n {
                11.. => "Active Contributor",
                5..=10 => "Moderate Contributor",
                _ => "Occasional Contributor",
            }),
        ]))
    });
    rows(out)
}

fn q29454(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, answer_count, view_count, comment_count, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, user_display_name, .. } = &db.post_history;
    let questions = db.post.with(post_type_id.eq(1));
    let ts = (&questions)
        .group_by(tags_str)
        .select(answer_count.opt().and(view_count.opt()).and(comment_count))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, an, asum, vn, vs, c), ((a, v), cm)| {
            (n + 1, an + a.is_some() as i64, asum + a.unwrap_or(0), vn + v.is_some() as i64, vs + v.unwrap_or(0), c + cm)
        });
    let edits = db.post_history.with(post_history_type_id.is_in([4, 5, 6]));
    let last = (&edits).select(post).inv().select(hd).fold(i64::MIN, |m, d| m.max(d));
    let updates: HashIdx<Id<Post>, Id<PostHistory>> = (&edits).select(post).inv().collect();
    let base = owned(db).with(post_type_id.eq(1));
    let rn = (&base)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(tags_str.select(&ts).opt().and(updates.opt())))
        .drive(|_, (p, (t, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        match t {
            Some((n, an, asum, vn, vs, c)) => f.extend([nullable(vs, vn), V::I(n), nullable(asum, an), avg(c, n)]),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        match h {
            Some(h) => f.extend([ostr(user_display_name.get(h)), V::T(hd.get(h).unwrap()), V::T(last.get(p).unwrap())]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        out.push(row(f))
    });
    rows(out)
}

fn q26640(db: &'static So) -> String {
    let Post { post_type_id, parent, owner_user, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, comment, .. } = &db.post_history;
    let reason_by_id: HashIdx<Str, Str> = (&db.close_reason_type.origid)
        .map(|i| &*Box::leak(i.to_string().into_boxed_str()))
        .inv()
        .select(&db.close_reason_type.name)
        .collect();
    let closures: HashIdx<Id<Post>, (i64, Str)> = db
        .post_history
        .with(post_history_type_id.eq(10))
        .select(post)
        .inv()
        .select(hd.and(comment.select(&reason_by_id)))
        .collect();
    let answers: HashIdx<Id<Post>, Id<Post>> = db.post.with(post_type_id.eq(2)).with(owner_user).select(parent).inv().collect();
    let mut out = Vec::new();
    db.post.with(post_type_id.eq(1)).select(answers.opt().and(closures.opt())).drive(|p, (a, c)| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "created"]);
        match a {
            Some(a) => {
                f.extend(post_fields(db, a, &["id", "owner", "score"]));
            }
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        match c {
            Some((d, r)) => f.extend([V::T(d), V::S(r)]),
            None => f.extend([V::Null, V::Null]),
        }
        out.push(row(f))
    });
    rows(out)
}

fn q7117(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let mut top = Vec::new();
    whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(row_number, |(_, rep)| rep, desc)
        .filt(|((_, rep), n)| n == 1 && rep > 5000)
        .drive(|_, ((u, _), _)| top.push(u));
    let top = rel(top);
    let mut out = Vec::new();
    db.post.with(post_type_id.eq(1)).with(score.gt(0)).cross(&top).drive(|(p, _), (_, u)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "comments", "activity"]);
        f.extend(user_cols(db, Some(u), &["name", "rep"]));
        out.push(row(f))
    });
    rows(out)
}

fn q14485(db: &'static So) -> String {
    let Post { score, view_count, answer_count, owner_user, .. } = &db.post;
    let last_vote = (&db.vote.post).inv().select(&db.vote.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let eng = engagement(db, db.post.iq());
    let ranked = whole(db.post.iq())
        .select(
            Ident::<Post>::new()
                .and(score)
                .and(view_count.opt())
                .and(answer_count.opt())
                .and(&eng)
                .and(last_vote.opt())
                .and(owner_user.opt()),
        )
        .window(rank, |((((((_, s), _), _), _), _), _)| s, desc)
        .window(rank, |(((((((_, _), v), _), _), _), _), _)| v, desc)
        .window(rank, |((((((((_, _), _), a), _), _), _), _), _)| a, desc);
    let mut out = Vec::new();
    ranked.filt(|(((_, a), b), c)| a <= 10 || b <= 10 || c <= 10).drive(
        |_, (((((((((p, _), _), _), (c, _, _, _)), last), u), a), b), r)| {
            let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
            f.extend(user_cols(db, u, &["id", "name", "rep"]));
            f.extend([V::I(c), ots(last), V::I(a), V::I(b), V::I(r)]);
            out.push(row(f))
        },
    );
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("12642", q12642),
    ("8795", q8795),
    ("6706", q6706),
    ("25989", q25989),
    ("7664", q7664),
    ("28019", q28019),
    ("6588", q6588),
    ("14622", q14622),
    ("29798", q29798),
    ("29454", q29454),
    ("26640", q26640),
    ("7117", q7117),
    ("14485", q14485),
];
