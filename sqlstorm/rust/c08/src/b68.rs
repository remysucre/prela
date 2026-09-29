use harness::prelude::*;

fn q10029(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.views_sum, a.views_n),
            avg(a.score_sum, a.n),
            avg(a.rep_sum, a.rep_n),
        ])
    }))
}

fn q12050(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, creation_date, .. } = &db.user;
    let agg = owner_user.inv().select(score.and(view_count.opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64),
        |(n, ss, vn, vs), (s, v)| {
            (n + 1, ss + s, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        },
    );
    let mut out = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(creation_date).and(&agg))
        .drive(|_, ((((id, dn), rep), cd), (n, ss, vn, vs))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                avg(ss, n),
                avg(vs, vn),
                V::I(rep),
                V::T(cd),
            ]))
        });
    rows(out)
}

fn q13598(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = owner_user.inv().select(score.and(comments_of(db).opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64),
        |(n, ss, cc), (s, c)| (n + 1, ss + s, cc + c.is_some() as i64),
    );
    let mut out = Vec::new();
    db.user.select(origid.and(display_name).and(reputation).and(&agg)).drive(
        |_, (((id, dn), rep), (n, ss, cc))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(rep),
                V::I(n),
                avg(ss, n),
                V::I(cc),
            ]))
        },
    );
    rows(out)
}

// GROUP BY (DisplayName, Title, CreationDate) over every post type, with the
// comment count as the sort key: the same query as 17276, and the same
// four-way tie at 25 across the cut, so rewrites/17919 refines it the same way.
fn q17919(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date);
    let mut v = Vec::new();
    db.post
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((dn, t), cd), n| v.push((n, dn, cd, t)));
    v.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| (a.1, a.2, a.3).cmp(&(b.1, b.2, b.3))));
    rows(v.iter().take(10).map(|&(n, dn, cd, t)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), V::I(n)])
    }))
}

// (DisplayName, Title, CreationDate, Score) grouped, comment count folded —
// 15443, 15001 and 15095 differ only in which column they sort on.
fn score_date_groups(db: &'static So) -> Vec<(Str, Option<Str>, i64, i64, i64)> {
    let Post { post_type_id, title, creation_date, score, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(score);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|(((dn, t), cd), s), n| v.push((dn, t, cd, s, n)));
    v
}

fn q15443(db: &'static So) -> String {
    let mut v = score_date_groups(db);
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|&(dn, t, cd, s, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), V::I(s), V::I(n)])
    }))
}

fn q15001(db: &'static So) -> String {
    q15443(db)
}

fn q15095(db: &'static So) -> String {
    let mut v = score_date_groups(db);
    v.sort_by(|a, b| b.3.cmp(&a.3));
    rows(v.iter().take(10).map(|&(dn, t, cd, s, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), V::I(s), V::I(n)])
    }))
}

// (DisplayName, Title, CreationDate) grouped, no Score — 16407 takes the whole
// result, 15399 the ten newest.
fn date_groups(db: &'static So) -> Vec<(Str, Option<Str>, i64, i64)> {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((dn, t), cd), n| v.push((dn, t, cd, n)));
    v
}

fn q16407(db: &'static So) -> String {
    rows(date_groups(db).iter().map(|&(dn, t, cd, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), V::I(n)])
    }))
}

fn q15399(db: &'static So) -> String {
    let mut v = date_groups(db);
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|&(dn, t, cd, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), V::I(n)])
    }))
}

fn q15182(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, view_count, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(view_count.opt());
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|(((dn, t), cd), vc), n| v.push((vc, dn, t, cd, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(vc, dn, t, cd, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), oint(vc), V::I(n)])
    }))
}

fn q18184(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let vc = votes_per_post(db);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(&vc)
        .drive(|p, n| v.push((n, p)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(n, p)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(V::I(n));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10029", q10029),
    ("12050", q12050),
    ("13598", q13598),
    ("15001", q15001),
    ("15095", q15095),
    ("15182", q15182),
    ("15399", q15399),
    ("15443", q15443),
    ("16407", q16407),
    ("17919", q17919),
    ("18184", q18184),
];
