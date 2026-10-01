use harness::prelude::*;

/// One row per user, whether or not they posted — `Users LEFT JOIN Posts`
/// with the aggregate computed on the Posts side. The forward edge inverted
/// gives Id<User> -> Id<Post>; `dense_fold_outer` seeds every user, so the
/// users with no posts come back as zeroes instead of being absent.
struct Row {
    id: i64,
    display_name: Str,
    reputation: i64,
    n: i64,
    score_sum: i64,
    views_n: i64,
    views_sum: i64,
}

fn user_rows(db: &'static So) -> Vec<Row> {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let n = db.user.id.n;

    let sc = owner_user
        .inv()
        .select(score)
        .dense_fold_outer(n, (0i64, 0i64), |(c, s), x| (c + 1, s + x));
    let vc = owner_user
        .inv()
        .select(view_count)
        .dense_fold_outer(n, (0i64, 0i64), |(c, s), x| (c + 1, s + x));

    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(sc).and(vc))
        .drive(|_, ((((id, dn), rep), (n, score_sum)), (views_n, views_sum))| {
            v.push(Row { id, display_name: dn, reputation: rep, n, score_sum, views_n, views_sum })
        });
    v
}

fn q11884(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.n.cmp(&x.n));
    rows(v.iter().map(|r| {
        row(vec![V::I(r.id), V::S(r.display_name), V::I(r.reputation), V::I(r.n)])
    }))
}

fn id_name_count_avgscore(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.n.cmp(&x.n));
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.score_sum, r.n),
        ])
    }))
}

fn q10571(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.reputation.cmp(&x.reputation));
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::I(r.reputation),
            V::I(r.n),
            avg(r.views_sum, r.views_n),
        ])
    }))
}

fn by_rep(db: &'static So, take: usize) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.reputation.cmp(&x.reputation));
    rows(v.iter().take(take).map(|r| {
        row(vec![V::I(r.id), V::S(r.display_name), V::I(r.reputation), V::I(r.n)])
    }))
}

fn q10801(db: &'static So) -> String {
    by_rep(db, 10)
}

fn q10133(db: &'static So) -> String {
    by_rep(db, 100)
}

fn q10131(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.n.cmp(&x.n));
    rows(v.iter().take(100).map(|r| {
        row(vec![V::I(r.id), V::S(r.display_name), V::I(r.reputation), V::I(r.n)])
    }))
}

fn name_groups(db: &'static So) -> Vec<(Str, i64, i64)> {
    let Post { owner_user, score, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let mut v: Vec<(Str, i64, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user.select(display_name))
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x))
        .drive(|k, (n, s)| v.push((k, n, s)));
    v
}

fn q18645(db: &'static So) -> String {
    let mut v = name_groups(db);
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(dn, n, s)| {
        row(vec![V::S(dn), V::I(*n), V::I(*s)])
    }))
}

fn q11283(db: &'static So) -> String {
    let mut v = name_groups(db);
    v.sort_by(|a, b| {
        let f = |t: &(Str, i64, i64)| t.2 as f64 / t.1 as f64;
        b.1.cmp(&a.1).then(f(b).partial_cmp(&f(a)).unwrap())
    });
    rows(v.iter().take(100).map(|(dn, n, s)| {
        row(vec![V::S(dn), V::I(*n), avg(*s, *n)])
    }))
}

fn comments_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.comment.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

fn q16616(db: &'static So) -> String {
    let Post { post_type_id, title: pt, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let cc = comments_per_post(db);
    let mut v: Vec<((Str, Str), i64)> = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(pt.and(owner_user.select(display_name)))
        .select(cc)
        .fold(0i64, |a, x| a + x)
        .drive(|k, n| v.push((k, n)));
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.0.cmp(b.0.0)).then(a.0.1.cmp(b.0.1)));
    rows(v.iter().take(10).map(|((ti, dn), n)| {
        row(vec![V::S(ti), V::S(dn), V::I(*n)])
    }))
}

type PostKey = (Str, Option<Str>, i64);

/// Grouped by (DisplayName, Title, CreationDate) where Title is nullable, so
/// the NULL-titled posts need their own fold over the complement.
fn post_comment_groups(db: &'static So) -> Vec<(PostKey, i64)> {
    let Post { title: pt, creation_date, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let cc = comments_per_post(db);
    let mut v: Vec<(PostKey, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user.select(display_name).and(pt).and(creation_date))
        .select(&cc)
        .fold(0i64, |a, x| a + x)
        .drive(|((dn, ti), cd), n| v.push(((dn, Some(ti), cd), n)));

    db.post
        .with(owner_user)
        .minus(pt)
        .group_by(owner_user.select(display_name).and(creation_date))
        .select(&cc)
        .fold(0i64, |a, x| a + x)
        .drive(|(dn, cd), n| v.push(((dn, None, cd), n)));
    v
}

fn q18612(db: &'static So) -> String {
    let mut v = post_comment_groups(db);
    v.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(a.0.0.cmp(b.0.0))
            .then(a.0.1.cmp(&b.0.1))
            .then(a.0.2.cmp(&b.0.2))
    });
    rows(v.iter().take(10).map(|((dn, ti, cd), n)| {
        row(vec![V::S(dn), ti.map(V::S).unwrap_or(V::Null), V::T(*cd), V::I(*n)])
    }))
}

fn q15616(db: &'static So) -> String {
    let mut v = post_comment_groups(db);
    v.sort_by(|a, b| b.0.2.cmp(&a.0.2));
    rows(v.iter().take(10).map(|((dn, ti, cd), n)| {
        row(vec![ti.map(V::S).unwrap_or(V::Null), V::T(*cd), V::S(dn), V::I(*n)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("11884", q11884),
    ("14852", id_name_count_avgscore),
    ("10459", id_name_count_avgscore),
    ("18645", q18645),
    ("16616", q16616),
    ("10801", q10801),
    ("18612", q18612),
    ("10571", q10571),
    ("10133", q10133),
    ("15616", q15616),
    ("10131", q10131),
    ("11283", q11283),
];
