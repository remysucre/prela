use harness::prelude::*;

// ---- Users LEFT JOIN Posts ----------------------------------------------

struct UAgg {
    id: i64,
    display_name: Str,
    reputation: i64,
    n: i64,
    score_sum: i64,
}

fn user_aggs(db: &'static So) -> Vec<UAgg> {
    let Post { owner_user, score, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;

    let sc = owner_user
        .inv()
        .select(score)
        .dense_fold_outer(db.user.id.n, (0i64, 0i64), |(c, s), x| (c + 1, s + x));

    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(sc))
        .drive(|_, (((id, dn), rep), (n, score_sum))| {
            v.push(UAgg { id, display_name: dn, reputation: rep, n, score_sum })
        });
    v
}

fn q13735(db: &'static So) -> String {
    let v = user_aggs(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.score_sum, r.n),
            V::I(r.reputation),
        ])
    }))
}

fn q11999(db: &'static So) -> String {
    let mut v = user_aggs(db);
    v.sort_by(|a, b| b.reputation.cmp(&a.reputation).then(b.n.cmp(&a.n)));
    rows(v.iter().take(100).map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.reputation),
            V::I(r.n),
            avg(r.score_sum, r.n),
        ])
    }))
}

/// `LEFT JOIN Posts ... WHERE p.PostTypeId = 1` — the WHERE makes it an
/// inner join, so only users with a question survive. `dense_fold`, not
/// `dense_fold_outer`.
fn q13706(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let User { origid, display_name, .. } = &db.user;
    let nu = db.user.id.n;

    let sc = owner_user
        .inv()
        .with(post_type_id.eq(1))
        .select(score)
        .dense_fold(nu, (0i64, 0i64), |(c, s), x| (c + 1, s + x));
    let vc = owner_user
        .inv()
        .with(post_type_id.eq(1))
        .select(view_count)
        .dense_fold_outer(nu, (0i64, 0i64), |(c, s), x| (c + 1, s + x));

    let mut v: Vec<(i64, Str, i64, i64, i64, i64)> = Vec::new();
    db.user
        .select(origid.and(display_name).and(sc).and(vc))
        .drive(|_, (((id, dn), (n, ss)), (vn, vs))| v.push((id, dn, n, ss, vn, vs)));

    rows(v.iter().map(|(id, dn, n, ss, vn, vs)| {
        row(vec![
            V::I(*id),
            V::S(dn),
            V::I(*n),
            avg(*ss, *n),
            nullable(*vs, *vn),
        ])
    }))
}

// ---- Posts JOIN PostTypes, GROUP BY pt.Name -----------------------------

fn q12825(db: &'static So) -> String {
    let Post { post_type, score, owner_user, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut sc: Vec<(Str, i64, i64)> = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x))
        .drive(|k, (n, s)| sc.push((k, n, s)));

    let uu = db
        .post
        .group_by(post_type.select(name))
        .select(owner_user)
        .count_distinct();

    rows(sc.iter().map(|(k, n, s)| {
        row(vec![
            V::S(k),
            V::I(*n),
            avg(*s, *n),
            V::I(uu.get(k).unwrap_or(0)),
        ])
    }))
}

/// `SUM(CASE WHEN p.Score IS NOT NULL THEN 1 ELSE 0)` — Score has no nulls
/// in this data, so it is COUNT(p.Id) spelled twice. AVG(p.ViewCount) does
/// skip nulls, so it gets its own pass.
fn q12397(db: &'static So) -> String {
    let Post { post_type, view_count, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut n: Vec<(Str, i64, i64, i64)> = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select((&db.post.score).and(view_count.opt()))
        .fold((0i64, 0i64, 0i64), |(c, vn, vs), (_, v)| {
            (c + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        })
        .drive(|k, (c, vn, vs)| n.push((k, c, vn, vs)));

    rows(n.iter().map(|(k, c, vn, vs)| {
        row(vec![V::S(k), V::I(*c), avg(*vs, *vn), V::I(*c)])
    }))
}

/// `SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0)` — a fold
/// over the nullable column turned into a 0/1 indicator over every post.
fn q13374(db: &'static So) -> String {
    let Post { post_type, score, accepted_answer_id, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let has_accepted = accepted_answer_id
        .map(|_| 1i64)
        .dense_fold_outer(db.post.id.n, 0i64, |_, x| x);

    let mut v: Vec<(Str, i64, i64, i64)> = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(score.and(has_accepted))
        .fold((0i64, 0i64, 0i64), |(n, s, a), (x, acc)| (n + 1, s + x, a + acc))
        .drive(|k, (n, s, a)| v.push((k, n, s, a)));

    rows(v.iter().map(|(k, n, s, a)| {
        row(vec![V::S(k), V::I(*n), avg(*s, *n), V::I(*a)])
    }))
}

/// AVG(u.Reputation) over the posts that have an owner — its own divisor.
fn q13860(db: &'static So) -> String {
    let Post { post_type, score, owner_user, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;
    let User { reputation, .. } = &db.user;

    let mut sc: Vec<(Str, i64, i64, i64, i64)> = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(score.and(owner_user.select(reputation).opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, rn, rs), (x, r)| {
            (n + 1, s + x, rn + r.is_some() as i64, rs + r.unwrap_or(0))
        })
        .drive(|k, (n, s, rn, rs)| sc.push((k, n, s, rn, rs)));

    rows(sc.iter().map(|(k, n, s, rn, rs)| {
        row(vec![V::S(k), V::I(*n), avg(*s, *n), avg(*rs, *rn)])
    }))
}

/// No GROUP BY at all. `count_distinct` only exists per group, so the whole
/// table has to be given one — the constant key. See `notes/limitations.md`.
fn q12036(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let since = add_years(date(2024, 10, 1), -1);

    let (n, score_sum) = db
        .post
        .with(creation_date.ge(since))
        .with(owner_user)
        .select(score)
        .fold_flat((0i64, 0i64), |(c, s), x| (c + 1, s + x));
    let (vn, vs) = db
        .post
        .with(creation_date.ge(since))
        .with(owner_user)
        .select(view_count)
        .fold_flat((0i64, 0i64), |(c, s), x| (c + 1, s + x));

    let users = db
        .post
        .with(creation_date.ge(since))
        .group_by(creation_date.map(|_| 0i64))
        .select(owner_user)
        .count_distinct();

    row(vec![
        V::I(n),
        avg(score_sum, n),
        avg(vs, vn),
        V::I(users.get(0i64).unwrap_or(0)),
    ])
}

// ---- Questions LEFT JOIN Comments ---------------------------------------

fn comments_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.comment.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

fn q15107(db: &'static So) -> String {
    let Post { post_type_id, origid, creation_date, score, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let cc = comments_per_post(db);
    let mut v: Vec<(Id<Post>, i64, i64, Str, i64, i64)> = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(
            origid
                .and(creation_date)
                .and(owner_user.select(display_name))
                .and(score)
                .and(cc),
        )
        .drive(|pid, ((((id, cd), dn), sc), n)| v.push((pid, id, cd, dn, sc, n)));

    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(pid, id, cd, dn, sc, n)| {
        row(vec![
            V::I(*id),
            title(db, *pid),
            V::T(*cd),
            V::S(dn),
            V::I(*sc),
            V::I(*n),
        ])
    }))
}

/// `GROUP BY p.Title, u.DisplayName, p.CreationDate, p.Score`
fn q16864(db: &'static So) -> String {
    let Post { post_type_id, title: pt, creation_date, score, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let cc = comments_per_post(db);
    let mut v: Vec<((Str, Str, i64, i64), i64)> = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(
            pt.and(owner_user.select(display_name)).and(creation_date).and(score),
        )
        .select(cc)
        .fold(0i64, |a, x| a + x)
        .drive(|(((ti, dn), cd), sc), n| v.push(((ti, dn, cd, sc), n)));

    v.sort_by(|a, b| b.0.3.cmp(&a.0.3).then(b.0.2.cmp(&a.0.2)));
    rows(v.iter().take(10).map(|((ti, dn, cd, sc), n)| {
        row(vec![V::S(ti), V::S(dn), V::T(*cd), V::I(*sc), V::I(*n)])
    }))
}

fn q19482(db: &'static So) -> String {
    let Post { post_type_id, title: pt, creation_date, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let cc = comments_per_post(db);
    let mut v: Vec<((Str, Str, i64), i64)> = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(owner_user.select(display_name).and(pt).and(creation_date))
        .select(cc)
        .fold(0i64, |a, x| a + x)
        .drive(|((dn, ti), cd), n| v.push(((dn, ti, cd), n)));

    rows(v.iter().map(|((dn, ti, cd), n)| {
        row(vec![V::S(dn), V::S(ti), V::T(*cd), V::I(*n)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("13735", q13735),
    ("12825", q12825),
    ("12397", q12397),
    ("12036", q12036),
    ("11999", q11999),
    ("13374", q13374),
    ("13860", q13860),
    ("13706", q13706),
    ("15107", q15107),
    ("16864", q16864),
    ("19482", q19482),
];
