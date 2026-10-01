use harness::prelude::*;

fn q17061(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, .. } = &db.post;
    let Vote { vote_type_id, post, .. } = &db.vote;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str)> = Vec::new();
    db.vote
        .with(vote_type_id.eq(2))
        .select(post.and(post.select(creation_date.and(owner_user.select(display_name)))))
        .drive(|_, (p, (created, dn))| v.push((p, created, dn)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(p, created, dn)| {
        row(vec![V::S(dn), ostr(title.get(*p)), V::T(*created), V::I(2)])
    }))
}

fn q16044(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, title, .. } = &db.post;
    let Tag { excerpt_post, tag_name, .. } = &db.tag;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str, Str)> = Vec::new();
    db.tag
        .with(excerpt_post.select(post_type_id).eq(1))
        .select(
            excerpt_post
                .and(excerpt_post.select(creation_date.and(owner_user.select(display_name))))
                .and(tag_name),
        )
        .drive(|_, ((p, (created, dn)), tn)| v.push((p, created, dn, tn)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(p, created, dn, tn)| {
        row(vec![V::S(dn), ostr(title.get(*p)), V::T(*created), V::S(tn)])
    }))
}

fn q15315(db: &'static So) -> String {
    let Post { owner_user, title, .. } = &db.post;
    let PostHistory { post_history_type_id, post, creation_date, .. } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str)> = Vec::new();
    db.post_history
        .with(post_history_type_id.eq(4))
        .select(post.and(creation_date).and(post.select(owner_user.select(display_name))))
        .drive(|_, ((p, created), dn)| v.push((p, created, dn)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(p, created, dn)| {
        row(vec![V::S(dn), ostr(title.get(*p)), V::T(*created)])
    }))
}

fn q11414(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;

    let g = db
        .post
        .group_by(creation_date.map(trunc_day))
        .select(score)
        .fold((0i64, 0i64), |(n, s), sc| (n + 1, s + sc));

    let mut all: Vec<(i64, i64, i64)> = Vec::new();
    g.drive(|d, (n, s)| all.push((d, n, s)));
    all.sort_by_key(|r| r.0);
    rows(all.iter().map(|(d, n, s)| {
        row(vec![V::D(*d), V::I(*n), V::F(*s as f64 / *n as f64)])
    }))
}

fn q15022(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, title, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str, i64)> = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(creation_date.and(owner_user.select(display_name.and(reputation))))
        .drive(|pid, (created, (dn, rep))| v.push((pid, created, dn, rep)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(pid, created, dn, rep)| {
        row(vec![V::S(dn), V::I(*rep), ostr(title.get(*pid)), V::T(*created)])
    }))
}

fn post_type_count_avg(db: &'static So) -> String {
    let Post { post_type, score, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let g = db
        .post
        .group_by(post_type.select(name))
        .select(score)
        .fold((0i64, 0i64), |(n, s), sc| (n + 1, s + sc));

    let mut all: Vec<(Str, i64, i64)> = Vec::new();
    g.drive(|k, (n, s)| all.push((k, n, s)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().map(|(nm, n, s)| {
        row(vec![V::S(nm), V::I(*n), V::F(*s as f64 / *n as f64)])
    }))
}

fn q10284(db: &'static So) -> String {
    let Post { owner_user, origid, .. } = &db.post;
    let User { origid: uid, reputation, .. } = &db.user;

    let g = db
        .post
        .with(owner_user)
        .group_by(owner_user)
        .select(origid)
        .fold(0i64, |a, _| a + 1);

    let mut all: Vec<(i64, i64, i64)> = Vec::new();
    g.drive(|u, n| {
        all.push((uid.get(u).unwrap(), n, reputation.get(u).unwrap()))
    });
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(100).map(|(id, n, rep)| {
        row(vec![V::I(*id), V::I(*n), V::I(*rep)])
    }))
}

fn q19125(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, score, owner_user, title, .. } =
        &db.post;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, i64, Str)> = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(
            creation_date
                .and(view_count)
                .and(score)
                .and(owner_user.select(display_name)),
        )
        .drive(|pid, (((created, views), sc), dn)| v.push((pid, created, views, sc, dn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(pid, created, views, sc, dn)| {
        row(vec![
            V::S(dn),
            ostr(title.get(*pid)),
            V::T(*created),
            V::I(*views),
            V::I(*sc),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("17061", q17061),
    ("16044", q16044),
    ("15315", q15315),
    ("11414", q11414),
    ("15022", q15022),
    ("10507", post_type_count_avg),
    ("10647", post_type_count_avg),
    ("10065", post_type_count_avg),
    ("10284", q10284),
    ("19125", q19125),
];
