use harness::prelude::*;

fn q10596(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            nullable(a.answers_sum, a.answers_n),
            V::I(a.comment_sum),
        ])
    }))
}

fn q11289(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            V::T(a.created_min),
            V::T(a.created_max),
        ])
    }))
}

fn count_avgscore_avgviews_avgrep(db: &'static So) -> String {
    rows(owned_by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            avg(a.rep_sum, a.n),
        ])
    }))
}

fn q14197(db: &'static So) -> String {
    rows(owned_by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
            avg(a.rep_sum, a.n),
        ])
    }))
}

fn q14203(db: &'static So) -> String {
    let Post { creation_date, post_type, score, owner_user, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;
    let User { reputation, .. } = &db.user;

    let mut all: Vec<(Str, i64, i64, i64)> = Vec::new();
    db.post
        .with(creation_date.ge(date(2022, 1, 1)))
        .with(owner_user)
        .group_by(post_type.select(name))
        .select(score.and(owner_user.select(reputation)))
        .fold((0i64, 0i64, 0i64), |(n, s, r), (x, rep)| (n + 1, s + x, r + rep))
        .drive(|k, (n, s, r)| all.push((k, n, s, r)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().map(|(k, n, s, r)| {
        row(vec![V::S(k), V::I(*n), V::I(*s), avg(*r, *n)])
    }))
}

fn q11457(db: &'static So) -> String {
    let PostHistory { post_history_type_id, creation_date, .. } = &db.post_history;

    let mut all: Vec<(i64, i64, i64, i64)> = Vec::new();
    db.post_history
        .with(creation_date.ge(date(2023, 1, 1)))
        .with(creation_date.le(date(2023, 12, 31)))
        .group_by(post_history_type_id)
        .select(creation_date)
        .fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), x| {
            (n + 1, lo.min(x), hi.max(x))
        })
        .drive(|k, (n, lo, hi)| all.push((k, n, lo, hi)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().map(|(k, n, lo, hi)| {
        row(vec![V::I(*k), V::I(*n), V::T(*lo), V::T(*hi)])
    }))
}

// Posts JOIN PostTypes JOIN Users grouped by (pt.Name, u.Reputation): (key, posts, score sum, views present, views sum).
fn type_rep_groups(db: &'static So) -> Vec<((Str, i64), i64, i64, i64, i64)> {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let g = db
        .post
        .with(owner_user)
        .group_by(ptype_name(db).and(owner_user.select(&db.user.reputation)))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    drain(&g).into_iter().map(|(k, a)| (k, a[0], a[1], a[2], a[3])).collect()
}

fn q14514(db: &'static So) -> String {
    let mut a = type_rep_groups(db);
    a.sort_by(|x, y| x.0.0.cmp(y.0.0).then(x.0.1.cmp(&y.0.1)));
    rows(a.iter().map(|((nm, rep), n, s, vn, vs)| {
        row(vec![
            V::S(nm),
            V::I(*rep),
            V::I(*n),
            avg(*s, *n),
            nullable(*vs, *vn),
        ])
    }))
}

fn q11473(db: &'static So) -> String {
    let mut a = type_rep_groups(db);
    a.sort_by(|x, y| x.0.0.cmp(y.0.0).then(y.0.1.cmp(&x.0.1)));
    rows(a.iter().map(|((nm, rep), n, s, vn, vs)| {
        row(vec![
            V::S(nm),
            V::I(*rep),
            V::I(*n),
            avg(*s, *n),
            nullable(*vs, *vn),
        ])
    }))
}

fn q15222(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let Comment { post, text, creation_date: cdate, user, .. } = &db.comment;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str, Str, i64)> = Vec::new();
    db.comment
        .with(post.select(post_type_id).eq(1))
        .select(
            post.and(post.select(creation_date))
                .and(user.select(display_name))
                .and(text)
                .and(cdate),
        )
        .drive(|_, ((((p, pc), dn), txt), cc)| v.push((p, pc, dn, txt, cc)));
    v.sort_by(|a, b| b.4.cmp(&a.4));
    rows(v.iter().take(10).map(|(p, pc, dn, txt, cc)| {
        row(vec![V::S(dn), title(db, *p), V::T(*pc), V::S(txt), V::T(*cc)])
    }))
}

fn q18948(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let Comment { post, text, creation_date: cdate, .. } = &db.comment;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str, Str, i64)> = Vec::new();
    db.comment
        .with(post.select(post_type_id).eq(1))
        .select(
            post.and(post.select(creation_date.and(owner_user.select(display_name))))
                .and(text)
                .and(cdate),
        )
        .drive(|_, (((p, (pc, dn)), txt), cc)| v.push((p, pc, dn, txt, cc)));
    v.sort_by(|a, b| b.4.cmp(&a.4));
    rows(v.iter().take(10).map(|(p, pc, dn, txt, cc)| {
        row(vec![V::S(dn), title(db, *p), V::T(*pc), V::S(txt), V::T(*cc)])
    }))
}

fn q19883(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let PostHistory { post_history_type_id, post, creation_date: hdate, .. } =
        &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, Str)> = Vec::new();
    db.post_history
        .with(post_history_type_id.in_v(vec![1, 2, 4, 5]))
        .select(
            post.and(hdate)
                .and(post.select(creation_date.and(owner_user.select(display_name)))),
        )
        .drive(|_, ((p, hc), (pc, dn))| v.push((p, hc, pc, dn)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(100).map(|(p, hc, pc, dn)| {
        row(vec![V::S(dn), title(db, *p), V::T(*hc), V::T(*pc)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("10596", q10596),
    ("14203", q14203),
    ("11289", q11289),
    ("10588", count_avgscore_avgviews_avgrep),
    ("11250", count_avgscore_avgviews_avgrep),
    ("11457", q11457),
    ("14197", q14197),
    ("14514", q14514),
    ("15222", q15222),
    ("18948", q18948),
    ("19883", q19883),
    ("11473", q11473),
];
