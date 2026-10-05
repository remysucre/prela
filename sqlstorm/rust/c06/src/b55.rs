use std::cmp::Reverse;
use harness::prelude::*;

fn year_ago() -> i64 {
    add_years(ts(2024, 10, 1, 12, 34, 56), -1)
}

fn q9787(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let questions = owned(db).with(post_type_id.eq(1));
    let per_user = (&questions)
        .select(owner_user)
        .inv()
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (sc, v)| (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0)));
    let by_name: HashIdx<Str, (i64, i64, i64, i64)> =
        db.user.with((&per_user).filt(|a| a.0 > 5)).select(&db.user.display_name).inv().select(&per_user).collect();
    let recent = (&questions).with(creation_date.ge(year_ago()));
    let rk = (&recent).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let mut v = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 5)
        .map(|((p, _), _)| p)
        .select(
            Ident::<Post>::new()
                .and(score.and(view_count.opt()).and(owner_user.select(&db.user.display_name).select(&by_name))),
        )
        .drive(|_, (p, ((s, vc), t))| v.push((s, vc, p, t)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(50).map(|&(_, _, p, (n, s, vn, vs))| {
        let mut f = post_fields(db, p, &["owner", "title", "score", "views", "created"]);
        f.extend([V::I(n), V::I(s), nullable(vs, vn)]);
        row(f)
    }))
}

fn q26351(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, view_count, score, .. } = &db.post;
    let history: HashIdx<Id<Post>, Id<PostHistory>> = (&db.post_history.post).inv().collect();
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.gt(year_ago())).with(tags_str);
    let joined: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = (&base).select(Ident::<Post>::new().and(history.opt())).collect();
    let post_of = (&joined).map(|(p, _)| p);
    let by_tags = (&post_of).select(tags_str);
    let ranked = (&joined)
        .group_by(&by_tags)
        .select(Same::new().and((&post_of).select(view_count.opt())).and((&post_of).select(score)))
        .window(rank, |((_, v), _)| v, desc)
        .window(rank, |(((_, _), s), _)| s, desc);
    let mut out = Vec::new();
    ranked.filt(|((_, a), b)| a <= 5 || b <= 5).drive(|_, (((((p, h), _), _), a), b)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "views", "score", "owner"]);
        f.push(ots(h.map(|h| db.post_history.creation_date.get(h).unwrap())));
        f.push(V::S(if a <= 5 {
            "Top Viewed"
        } else if b <= 5 {
            "Top Scored"
        } else {
            "Other"
        }));
        out.push(row(f))
    });
    rows(out)
}

fn q5906(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let totals = db
        .user
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()))
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64), |(bn, bs, b), (v, bi)| {
            let v = v.flatten();
            (bn + v.is_some() as i64, bs + v.unwrap_or(0), b + bi.is_some() as i64)
        })
        .map(|(bn, bs, b)| ((bn > 0).then_some(bs), b));
    let top_users = rel(drain(db.user.select((&db.user.display_name).and(&totals)).filt(|(_, (t, _))| t.is_some_and(|t| t > 10))));
    let base = owned(db).with(post_type_id.eq(1)).with(score.gt(10));
    let rk = whole(&base).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, cd)| cd, desc);
    // a post ranked r has r - 1 posts ahead of it, each crossed with every top user, so only r <= 100 can reach the first 100 rows
    let v = top_n(drain((&rk).filt(|(_, r)| r <= 100).cross(&top_users)), |(_, ((_, r), (_, (_, (t, _)))))| (*r, Reverse(t.unwrap())), 100);
    rows(v.into_iter().map(|(_, (((p, _), _), (_, (dn, (t, b)))))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::S(dn), V::I(t.unwrap()), V::I(b)]);
        row(f)
    }))
}

fn q7454(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1)).with(creation_date.ge(year_ago()));
    let counts = (&base)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(u, d, c, h), ((vt, ci), hi)| {
            (u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64, c + ci.is_some() as i64, h + hi.is_some() as i64)
        });
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(&counts))
        .window(row_number, |(_, (u, d, _, _))| u - d, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(|_, ((p, (u, d, c, h)), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(u), V::I(d), V::I(c), V::I(h)]);
        out.push(row(f))
    });
    rows(out)
}

fn q29603(db: &'static So) -> String {
    let Post { post_type_id, creation_date, parent, owner_user_id, .. } = &db.post;
    let cc = comments_per_post(db);
    let ac = db.post.with(post_type_id.eq(2)).select(parent).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let base = db.post.with(post_type_id.eq(1));
    let rn = (&base)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let votes = (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(
            comments_of(db)
                .opt()
                .and(children_of(db).with(post_type_id.eq(2)).opt())
                .and(votes_of(db).select(&db.vote.vote_type_id).opt()),
        )
        .fold((0i64, 0i64), |(u, d), (_, vt)| (u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64));
    let mut out = Vec::new();
    (&votes).and(&cc).and(&ac).drive(|p, (((u, d), c), a)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner"]);
        f.extend([V::I(c), V::I(a), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q8028(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let cc = comments_per_post(db);
    let mentions = tag_mentions(db);
    let name_of = (&mentions).map(|(_, t)| t).select(&db.tag.tag_name);
    let counts = (&mentions).group_by(&name_of).fold(0i64, |a, _| a + 1);
    let top: MatSet<Str> = whole(&counts)
        .select(Same::<Str>::new().and(&counts))
        .window(
            row_number,
            |(t, n)| (n, t),
            |x: &(i64, Str), y: &(i64, Str)| y.0.cmp(&x.0).then(x.1.cmp(&y.1)),
        )
        .filt(|(_, n)| n <= 5)
        .map(|((t, _), _)| t)
        .collect();
    let base = db.post.with(creation_date.ge(date(2022, 1, 1)));
    let rk = (&base).group_by(post_type).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| s, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(&cc))
        .cross(&top)
        .drive(|_, ((p, c), t)| {
            let mut f = post_fields(db, p, &["title", "score"]);
            f.extend([V::I(c), V::S(t)]);
            out.push(row(f))
        });
    rows(out)
}

fn q12267(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let vc = votes_per_post(db);
    let rn = whole(db.post.iq())
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and((&vc).and(owner_user.select(&db.user.origid).opt())))
        .drive(|_, (p, (v, uid))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "answers", "comments", "created", "activity"]);
        f.push(oint(uid));
        f.extend(post_fields(db, p, &["owner"]));
        f.push(V::I(v));
        out.push(row(f))
    });
    rows(out)
}

fn q10565(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let rn = whole(&base)
        .select(Ident::<Post>::new().and(&db.post.creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 10).map(|((p, _), _)| p)).drive(|p, (c, v, _, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::I(c), V::I(v)]);
        out.push(row(f))
    });
    rows(out)
}

fn q14428(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let nu = db.user.id.n;
    let posts = owner_user.inv().dense_fold_outer(nu, 0i64, |a, _| a + 1);
    let comments = owner_user.inv().select(comments_of(db)).dense_fold_outer(nu, 0i64, |a, _| a + 1);
    let t = owner_user
        .inv()
        .select(post_type_id.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt()))
        .dense_fold_outer(nu, (0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(v, q, a, w, vn, vs), (((ty, pw), _), vi)| {
            (
                v + vi.is_some() as i64,
                q + (ty == 1) as i64,
                a + (ty == 2) as i64,
                w + (ty == 4 || ty == 5) as i64,
                vn + pw.is_some() as i64,
                vs + pw.unwrap_or(0),
            )
        });
    let ranked = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&db.user.reputation).and((&posts).and(&comments).and(&t)))
        .window(rank, |(_, (_, t))| t.0, desc)
        .window(rank, |((_, ((n, _), _)), _)| n, desc);
    let mut out = Vec::new();
    ranked
        .drive(|_, (((((id, dn), rep), ((n, c), (v, q, a, w, vn, vs))), x), y)| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(rep),
                V::I(n),
                V::I(c),
                V::I(v),
                V::I(q),
                V::I(a),
                V::I(w),
                nullable(vs, vn),
                V::I(x),
                V::I(y),
            ]))
        });
    rows(out)
}

fn q7306(db: &'static So) -> String {
    let Post { post_type, post_type_id, creation_date, score, view_count, owner_user_id, .. } = &db.post;
    let pv = (&db.vote.post)
        .inv()
        .select(vtype_name(db))
        .fold((0i64, 0i64), |(u, d), n| (u + (n == "UpMod") as i64, d + (n == "DownMod") as i64));
    let badges = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let base = db.post.with(creation_date.ge(add_years(current_date(), -1))).with(post_type_id.is_in([1, 2]));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and((&pv).and(owner_user_id.select(&badges))))
        .drive(|_, (p, ((u, d), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(u), V::I(d), V::I(b)]);
        out.push(row(f))
    });
    rows(out)
}

fn q2703(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, owner_user, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2022, 1, 1))).with(score.ge(0));
    let joined: MatSet<(Id<Post>, Option<Id<Comment>>, Option<Id<Vote>>)> = (&base)
        .select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt()))
        .map(|((p, c), v)| (p, c, v))
        .collect();
    let post_of = (&joined).map(|(p, _, _)| p);
    let comment_of = (&joined).map(|(_, c, _)| c);
    let bounty_of = (&joined).map(|(_, _, v)| v).flat_map(|v| v).select(&db.vote.bounty_amount);
    let per_post = (&joined)
        .group_by(&post_of)
        .select((&comment_of).and(bounty_of.opt()))
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), (ci, b)| (c + ci.is_some() as i64, bn + b.is_some() as i64, bs + b.unwrap_or(0)));
    type Key = (i64, (Id<Post>, Option<Id<Comment>>, Option<Id<Vote>>));
    let rn = (&joined)
        .group_by((&post_of).select(owner_user_id.opt()))
        .select(Same::new().and((&post_of).select(creation_date)))
        .window(row_number, |(t, cd)| (cd, t), |x: &Key, y: &Key| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
    let summary = (&rn)
        .filt(|(_, n)| n <= 3)
        .map(|((t, _), _)| t)
        .group_by((&post_of).select(owner_user))
        .select((&post_of).select(score.and(&per_post)))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, c, bn, bs), (sc, (pc, pbn, pbs))| {
            (n + 1, s + sc, c + pc, bn + (pbn > 0) as i64, bs + pbs)
        });
    let mut v = Vec::new();
    db.user
        .with((&db.user.reputation).gt(1000))
        .select((&db.user.display_name).and(&db.user.reputation).and(&summary))
        .drive(|_, x| v.push(x));
    v.sort_by(|a, b| (b.1.1, b.1.0).cmp(&(a.1.1, a.1.0)));
    rows(v.iter().take(10).map(|&((dn, rep), (n, s, c, bn, bs))| {
        row(vec![V::S(dn), V::I(rep), V::I(n), V::I(s), V::I(c), avg(bs, bn)])
    }))
}

fn q8770(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, owner_user, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(year_ago()));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let stats = (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (sc, v)| (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0)));
    let mut out = Vec::new();
    (&stats).filt(|a| a.2 > 0 && a.3 > 50).drive(|dn, (n, s, _, vs)| {
        let kind = if n > 5 {
            "Prolific Contributor"
        } else if s > 100 {
            "High Scorer"
        } else {
            "Regular Contributor"
        };
        out.push(row(vec![V::S(dn), V::I(n), V::I(s), V::I(vs), V::S(kind)]))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("9787", q9787),
    ("26351", q26351),
    ("5906", q5906),
    ("7454", q7454),
    ("29603", q29603),
    ("8028", q8028),
    ("12267", q12267),
    ("10565", q10565),
    ("14428", q14428),
    ("7306", q7306),
    ("2703", q2703),
    ("8770", q8770),
];
