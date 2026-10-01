use harness::prelude::*;

fn year_ago() -> i64 {
    add_years(ts(2024, 10, 1, 12, 34, 56), -1)
}

fn q28124(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, user_display_name, .. } = &db.post_history;
    let edits = db.post_history.with(post_history_type_id.is_in([4, 5, 6]));
    let activity = (&edits)
        .group_by(post.and(user_display_name.opt()).and(hd).and(post_history_type_id))
        .select(post.and(hd))
        .fold((Id::new(0), 0i64, 0i64), |(_, _, n), (p, d)| (p, d, n + 1));
    let activity_by_post: HashIdx<Id<Post>, (Id<Post>, i64, i64)> = (&activity).map(|a| a.0).inv().select(&activity).collect();
    let mentions = tag_mentions(db);
    let tag_of = (&mentions).map(|(_, t)| t);
    let counts = (&mentions).group_by(&tag_of).fold(0i64, |a, _| a + 1);
    let popular_by_post: HashIdx<Id<Post>, Id<Tag>> =
        (&mentions).with((&tag_of).with((&counts).gt(10))).map(|(p, _)| p).inv().select(&tag_of).collect();
    let base = owned(db).with(post_type_id.eq(1));
    let rk = whole(&base).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 100)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(activity_by_post.opt().and(popular_by_post.opt())))
        .drive(|_, (p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner", "views", "score", "answers", "comments", "tags"]);
        f.push(ostr(t.map(|t| db.tag.tag_name.get(t).unwrap())));
        match a {
            Some((_, d, n)) => f.extend([V::T(d), V::I(n)]),
            None => f.extend([V::Null, V::Null]),
        }
        out.push(row(f))
    });
    rows(out)
}

fn q30013(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, user, comment, .. } = &db.post_history;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounties = db
        .vote
        .with(vote_type_id.is_in([8, 9]))
        .select(&db.vote.user)
        .inv()
        .select(bounty_amount)
        .fold(0i64, |a, b| a + b);
    let closed: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).with(user).select(post).inv().collect();
    let base = owned(db).with(post_type_id.eq(1));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.select((&bounties).gt(0)).opt().and(closed.opt())))
        .drive(|_, (p, (b, h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner", "created"]);
        match b {
            Some(b) => f.extend(post_fields(db, p, &["owner"]).into_iter().chain([V::I(b)])),
            None => f.extend([V::Null, V::Null]),
        }
        match h {
            Some(h) => f.extend([
                V::T(hd.get(h).unwrap()),
                V::S(db.user.display_name.get(user.get(h).unwrap()).unwrap()),
                ostr(comment.get(h)),
            ]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        out.push(row(f))
    });
    rows(out)
}

fn q8671(db: &'static So) -> String {
    let Post { post_type, post_type_id, score, creation_date, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let base = owned(db).with(post_type_id.is_in([1, 2]));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let stats = (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).with(vote_type_id.eq(8)).select(bounty_amount.opt()).opt()))
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), (ci, b)| {
            let b = b.flatten();
            (c + ci.is_some() as i64, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        });
    let mut out = Vec::new();
    (&stats).drive(|p, (c, bn, bs)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([V::I(c), nullable(bs, bn)]);
        out.push(row(f))
    });
    rows(out)
}

fn q8948(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(year_ago()));
    let (n, s, vn, vs) = (&base)
        .select(score.and(view_count.opt()))
        .fold_flat((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (sc, v)| (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0)));
    let avg_s = s as f64 / n as f64;
    let avg_v = (vn > 0).then(|| vs as f64 / vn as f64);
    let by_type = post_type.select(&db.post_type.name);
    let ranked = (&base)
        .group_by(&by_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), _)| s, desc)
        .window(rank, |(((_, _), v), _)| v, desc);
    let mut out = Vec::new();
    ranked.filt(|((_, a), b)| a <= 10 || b <= 10).drive(
        |_, ((((p, sc), v), _), _)| {
            let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
            f.extend([
                avg(s, n),
                avg(vs, vn),
                V::S(if sc as f64 > avg_s { "Above Average" } else { "Below Average" }),
                V::S(if v.zip(avg_v).is_some_and(|(v, a)| v as f64 > a) { "Above Average" } else { "Below Average" }),
            ]);
            out.push(row(f))
        },
    );
    rows(out)
}

fn q5792(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, answer_count, comment_count, origid, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(year_ago()));
    let rn = (&base)
        .group_by(post_type.select(&db.post_type.name))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let type_by_id: HashIdx<i64, Str> = (&db.post_type.origid).inv().select(&db.post_type.name).collect();
    let stats = (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .group_by(origid.select(&type_by_id))
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, an, asum, c), (((sc, v), a), cm)| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0), an + a.is_some() as i64, asum + a.unwrap_or(0), c + cm)
        });
    let mut out = Vec::new();
    stats.drive(|name, (n, s, vn, vs, an, asum, c)| {
        let per = |sum: i64, k: i64| if k == 0 { V::Null } else { V::F(sum as f64 / n as f64) };
        out.push(row(vec![
            V::S(name),
            V::I(n),
            avg(s, n),
            nullable(vs, vn),
            nullable(asum, an),
            V::I(c),
            per(vs, vn),
            per(asum, an),
            V::F(c as f64 / n as f64),
        ]))
    });
    rows(out)
}

type Tu = (Id<User>, Str, (i64, i64, i64, i64, i64), i64);

fn q28619(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, score, accepted_answer_id, creation_date, .. } = &db.post;
    let us = owner_user.inv().select(accepted_answer_id.opt().and(view_count.opt()).and(score)).fold(
        (0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, acc, vn, vs, s), ((a, v), sc)| (n + 1, acc + a.is_some() as i64, vn + v.is_some() as i64, vs + v.unwrap_or(0), s + sc),
    );
    let users = db.user.with((&db.user.reputation).gt(0)).with(&us);
    let tu: MatSet<Tu> = whole(&users)
        .select(Ident::<User>::new().and(&db.user.display_name).and(&us))
        .window(dense_rank, |(_, a)| a.4, desc)
        .map(|(((u, dn), a), r)| (u, dn, a, r))
        .collect();
    let questions: HashIdx<Id<User>, Id<Post>> = db.post.with(post_type_id.eq(1)).select(owner_user).inv().collect();
    let g = (&tu)
        .group_by(Same::<Tu>::new().map(|(_, dn, a, r): Tu| (dn, a, r)))
        .select(Same::<Tu>::new().map(|x: Tu| x.0).select(&questions).select(view_count.opt().and(creation_date)))
        .fold((0i64, 0i64, 0i64, i64::MIN), |(n, vn, vs, last), (v, cd)| {
            (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0), last.max(cd))
        });
    let mut v = Vec::new();
    (&g).drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| a.0.2.cmp(&b.0.2).then(b.1.0.cmp(&a.1.0)));
    rows(v.iter().take(10).map(|&((dn, (n, acc, vn, vs, s), r), (rn, rvn, rvs, last))| {
        row(vec![
            V::S(dn),
            V::I(rn),
            nullable(rvs, rvn),
            V::T(last),
            V::I(n),
            V::I(acc),
            nullable(vs, vn),
            V::I(s),
            V::I(r),
        ])
    }))
}

fn q5561(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let types_of: HashIdx<Id<Post>, i64> = (&db.post_history.post).inv().select(&db.post_history.post_history_type_id).collect();
    let bc = (&db.badge.user).inv().fold(0i64, |a, _| a + 1);
    let badged: HashIdx<Str, (Str, i64)> = db
        .user
        .with((&bc).gt(5))
        .select(&db.user.display_name)
        .inv()
        .select((&db.user.display_name).and(&bc))
        .collect();
    let base = owned(db).with(post_type_id.eq(1));
    let groups: MatSet<(Id<Post>, Option<i64>)> = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(types_of.opt()))
        .collect();
    let mut v = Vec::new();
    (&groups)
        .map(|(p, _)| p)
        .select(score.and(view_count.opt()).and(owner_user.select(&db.user.display_name).select(&badged)))
        .drive(|(p, _), ((s, w), b)| v.push((s, w, p, b)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(_, _, p, (dn, n))| {
        let mut f = post_fields(db, p, &["title", "created", "owner", "score", "views", "answers"]);
        f.extend([V::S(dn), V::I(n)]);
        row(f)
    }))
}

fn q25990(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let PostHistory { post, post_history_type_id, user_display_name, creation_date: hd, text, .. } = &db.post_history;
    let cc = comments_per_post(db);
    let edits: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).select(post).inv().collect();
    let questions = db.post.with(post_type_id.eq(1));
    let joined: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = (&questions).select(Ident::<Post>::new().and(edits.opt())).collect();
    let post_of = (&joined).map(|(p, _)| p);
    let rk = whole(&joined)
        .select(Same::new().and((&post_of).select(score.and(view_count.opt()))).and((&post_of).select(creation_date)))
        .window(rank, |((_, sv), _)| sv, desc);
    let mut v = Vec::new();
    (&rk).drive(|_, (((x, _), cd), r)| v.push((r, cd, x)));
    v.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    rows(v.iter().take(100).map(|&(r, _, (p, h))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "owner", "created", "views", "score"]);
        f.push(V::I(cc.get(p).unwrap()));
        match h {
            Some(h) => f.extend([
                ostr(user_display_name.get(h)),
                V::T(hd.get(h).unwrap()),
                V::S(db.post_history_type.name.get(db.post_history.post_history_type.get(h).unwrap()).unwrap()),
                ostr(text.get(h)),
            ]),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        f.push(V::I(r));
        row(f)
    }))
}

fn q28916(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let active = db
        .user
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64), |(b, bn, bs), (bi, v)| {
            let v = v.flatten();
            (b + bi.is_some() as i64, bn + v.is_some() as i64, bs + v.unwrap_or(0))
        })
        .map(|(b, bn, bs)| (b, (bn > 0).then_some(bs)));
    let top: MatSet<Id<User>> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&active))
        .window(row_number, |(_, a)| a, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let questions = db.post.with(post_type_id.eq(1));
    let rn = (&questions)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.with(&top).and(&cc)))
        .drive(|_, (p, (u, c))| {
        let (b, t) = active.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "tags", "owner", "rep"]);
        f.extend([V::I(c), V::S(db.user.display_name.get(u).unwrap()), V::I(b), oint(t)]);
        out.push(row(f))
    });
    rows(out)
}

fn q27857(db: &'static So) -> String {
    let Post { post_type, creation_date, title, body, parent, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let cc = comments_per_post(db);
    let ac = parent.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let base = owned(db).with(creation_date.ge(year_ago())).with(title).with(body.filt(|b: Str| !b.trim_matches(' ').is_empty()));
    let rn = (&base)
        .group_by(post_type.select(&db.post_type.name))
        .select(Ident::<Post>::new().and(&up).and(&ac).and(&down).and(&cc))
        .window(row_number, |((((_, u), a), _), _)| (u, a), desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((((p, u), a), d), c), r)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "owner"]);
        let lpad = |x: i64| V::Owned(format!("{x:05}").chars().take(5).collect());
        f.extend([lpad(u), lpad(d), V::I(c), V::I(a)]);
        f.extend(post_fields(db, p, &["type"]));
        f.push(V::I(r));
        out.push(row(f))
    });
    rows(out)
}

fn q11076(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let t = db
        .user
        .select(posts_of(db).select(view_count.opt().and(score).and(comments_of(db).opt())).opt())
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64, 0i64), |(n, c, w, s), p| match p {
            Some(((v, sc), ci)) => (n + 1, c + ci.is_some() as i64, w + v.unwrap_or(0), s + sc),
            None => (n, c, w, s),
        });
    let rn = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t))
        .window(row_number, |(_, a)| a.3, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(
        |_, (((id, dn), (n, c, w, s)), r)| out.push(row(vec![V::I(id), V::S(dn), V::I(n), V::I(c), V::I(w), V::I(s), V::I(r)])),
    );
    rows(out)
}

fn q12772(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let t = owner_user.inv().select(post_type_id).dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64), |(n, q, a), ty| {
        (n + 1, q + (ty == 1) as i64, a + (ty == 2) as i64)
    });
    let rk = whole(&db.user.id)
        .select((&db.user.display_name).and(&db.user.reputation).and(&db.user.creation_date).and(&t))
        .window(rank, |(_, t)| t.0, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .drive(|_, ((((dn, rep), cd), (n, q, a)), _)| out.push(row(vec![V::S(dn), V::I(rep), V::T(cd), V::I(n), V::I(q), V::I(a)])));
    rows(out)
}

fn q12147(db: &'static So) -> String {
    let posts = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let comments = (&db.comment.user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let rk = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.reputation).and(&posts).and(&comments))
        .window(rank, |(((_, rep), _), _)| rep, desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 10).drive(
        |_, ((((id, rep), p), c), r)| out.push(row(vec![V::I(id), V::I(rep), V::I(p), V::I(c), V::I(r)])),
    );
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("28124", q28124),
    ("30013", q30013),
    ("8671", q8671),
    ("8948", q8948),
    ("5792", q5792),
    ("28619", q28619),
    ("5561", q5561),
    ("25990", q25990),
    ("28916", q28916),
    ("27857", q27857),
    ("11076", q11076),
    ("12772", q12772),
    ("12147", q12147),
];
