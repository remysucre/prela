use harness::prelude::*;

fn leak_join(parts: impl IntoIterator<Item = Str>, sep: &str) -> Str {
    Box::leak(parts.into_iter().collect::<Vec<_>>().join(sep).into_boxed_str())
}

fn close_reasons(db: &'static So, types: &[i64]) -> Fold<Id<Post>, Str> {
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let reason_by_id: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let types = types.to_vec();
    db.post_history
        .with(post_history_type_id.filt(move |t| types.contains(&t)))
        .select(post)
        .inv()
        .select(comment.flat_map(|c: Str| c.parse::<i64>().ok()).select(&reason_by_id))
        .buf_fold(|names| leak_join(names, ", "))
}

fn q14626(db: &'static So) -> String {
    let Post { accepted_answer, tags_str, .. } = &db.post;
    let accepted = accepted_answer.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut out = Vec::new();
    engagement(db, db.post.iq()).and(&accepted).drive(|p, ((c, v, _, _), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(v), V::L(vec![ostr(tags_str.get(p))]), V::I(a)]);
        out.push(row(f))
    });
    rows(out)
}

fn q2354(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cc = comments_per_post(db);
    let reason_by_id: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let reasons = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .select(post)
        .inv()
        .select(comment.flat_map(|c: Str| c.parse::<i64>().ok()).select(&reason_by_id))
        .buf_fold(|mut names| {
            names.sort_unstable();
            names.dedup();
            &*Box::leak(names.into_vec().into_boxed_slice())
        });
    let mut v = Vec::new();
    db.post.with(post_type_id.eq(1)).with(score.gt(0)).select(score.and(creation_date)).drive(|p, (s, cd)| v.push((s, cd, p)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(_, _, p)| {
        let mut f = post_fields(db, p, &["title", "created", "owner", "score"]);
        let names: &[Str] = reasons.get(p).unwrap_or(&[]);
        f.push(V::L(names.iter().map(|&n| V::S(n)).collect()));
        f.push(V::I(cc.get(p).unwrap()));
        row(f)
    }))
}

fn q2355(db: &'static So) -> String {
    let Post { post_type, creation_date, score, owner_user, .. } = &db.post;
    let reasons = close_reasons(db, &[10, 11]);
    let comments = (&db.comment.post).inv().fold(0i64, |a, _| a + 1);
    let base = db.post.with(creation_date.ge(date(2024, 9, 1)));
    let rk = (&base).group_by(post_type).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.and(comments.opt()).and(reasons.opt())))
        .drive(|_, (p, ((u, c), r))| {
        let mut f = vec![V::S(db.user.display_name.get(u).unwrap()), V::I(db.user.reputation.get(u).unwrap())];
        f.extend(post_fields(db, p, &["id", "score", "views"]));
        f.extend([V::I(c.unwrap_or(0)), ostr(r)]);
        out.push(row(f))
    });
    rows(out)
}

fn q8000(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let excerpt_tags = (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).buf_fold(|names| leak_join(names, ", "));
    let base = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let e = engagement(db, &base);
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(&e).and(excerpt_tags.opt()))
        .window(row_number, |((_, e), _)| e.2, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, (((p, (c, _, u, d)), t), _)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::I(u - d), ostr(t)]);
        out.push(row(f))
    });
    rows(out)
}

fn q1375(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let cc = comments_per_post(db);
    let reasons = close_reasons(db, &[10, 11]);
    let mut v = Vec::new();
    db.post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(score.gt(10))
        .select(score.and(&cc).and(reasons.opt()))
        .drive(|p, ((s, c), r)| v.push((s, p, c, r)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(100).map(|&(_, p, c, r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::S(r.unwrap_or("No close reasons"))]);
        row(f)
    }))
}

fn q33753(db: &'static So) -> String {
    let Post { owner_user, view_count, post_type_id, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let Badge { user, date: bdate, name, .. } = &db.badge;
    let activity = owner_user
        .inv()
        .select(view_count.opt().and(post_type_id).and(votes_of(db).with(vote_type_id.is_in([8, 9])).select(bounty_amount.opt()).opt()))
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64, 0i64), |(w, q, a, b), ((v, t), bounty)| {
            (w + v.unwrap_or(0), q + (t == 1) as i64, a + (t == 2) as i64, b + bounty.flatten().unwrap_or(0))
        });
    let rn = db.badge.group_by(user).select(bdate.and(name)).window(row_number, |(d, _)| d, desc);
    let recent = (&rn).filt(|(_, r)| r <= 3).map(|((_, nm), r)| (r, nm)).buf_fold(|mut xs| {
        xs.sort_unstable();
        leak_join(xs.into_iter().map(|x| x.1), ", ")
    });
    let mut v = Vec::new();
    db.user.select(&activity).filt(|a| a.0 > 1000).drive(|u, a| v.push((a, u)));
    v.sort_by(|x, y| (y.0.0, y.0.1).cmp(&(x.0.0, x.0.1)));
    rows(v.iter().take(10).map(|&((w, q, a, b), u)| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(w),
            V::I(q),
            V::I(a),
            V::I(b),
            V::S(recent.get(u).unwrap_or("No Badges")),
        ])
    }))
}

fn q3804(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, score, view_count, .. } = &db.post;
    let reasons = close_reasons(db, &[10]);
    let base = db.post.with(creation_date.ge(add_years(current_date(), -1)));
    let joined: MatSet<(Id<Post>, Option<Id<Comment>>, Option<Id<Vote>>)> = (&base)
        .select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).with((&db.vote.vote_type_id).eq(2)).opt()))
        .map(|((p, c), v)| (p, c, v))
        .collect();
    let post_of = (&joined).map(|(p, _, _)| p);
    let comments = (&joined).group_by(&post_of).select((&joined).map(|(_, c, _)| c.is_some())).fold(0i64, |n, c| n + c as i64);
    type Key = (i64, (Id<Post>, Option<Id<Comment>>, Option<Id<Vote>>));
    let rn = (&joined)
        .group_by((&post_of).select(owner_user_id.opt()))
        .select(Same::new().and((&post_of).select(score.and(view_count.opt()).and(&comments))))
        .window(row_number, |(t, ((s, _), _))| (s, t), |x: &Key, y: &Key| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
    let mut v = Vec::new();
    (&rn).filt(|(_, n)| n <= 5).drive(
        |_, (((p, _, _), ((s, w), c)), r)| v.push((s, w, p, c, r)),
    );
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().skip(10).take(10).map(|&(_, _, p, c, r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(r), V::S(reasons.get(p).unwrap_or("No close reasons"))]);
        row(f)
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("14626", q14626),
    ("2354", q2354),
    ("2355", q2355),
    ("8000", q8000),
    ("1375", q1375),
    ("33753", q33753),
    ("3804", q3804),
];
