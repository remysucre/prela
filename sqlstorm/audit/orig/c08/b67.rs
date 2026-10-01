use harness::prelude::*;

fn q11129(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
            avg(a.rep_sum, a.rep_n),
        ])
    }))
}

fn q14591(db: &'static So) -> String {
    let Post { post_type, score, view_count, accepted_answer_id, .. } = &db.post;
    let mut out = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(accepted_answer_id.opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, ss, vn, vs, a), ((s, v), aa)| {
            (
                n + 1,
                ss + s,
                vn + v.is_some() as i64,
                vs + v.unwrap_or(0),
                a + aa.is_some() as i64,
            )
        })
        .drive(|k, (n, ss, vn, vs, a)| {
            out.push(row(vec![V::S(k), V::I(n), avg(ss, n), nullable(vs, vn), V::I(a)]))
        });
    rows(out)
}

// The Votes join multiplies, so COUNT/AVG/SUM over the posts count joined
// rows. The Users join is many-to-one and outer, so it changes nothing.
fn q12428(db: &'static So) -> String {
    let Post { post_type, score, view_count, .. } = &db.post;
    let mut out = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(votes_of(db).opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, ss, vn, vs), ((s, v), _)| {
            (n + 1, ss + s, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        })
        .drive(|k, (n, ss, vn, vs)| {
            out.push(row(vec![V::S(k), V::I(n), avg(ss, n), nullable(vs, vn)]))
        });
    rows(out)
}

fn q12576(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let User { origid, display_name, .. } = &db.user;
    let agg = owner_user.inv().select(score.and(comments_of(db).opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64),
        |(n, ss, cc), (s, c)| (n + 1, ss + s, cc + c.is_some() as i64),
    );
    let mut out = Vec::new();
    db.user.select(origid.and(display_name).and(&agg)).drive(
        |_, ((id, dn), (n, ss, cc))| {
            out.push(row(vec![V::I(id), V::S(dn), V::I(n), avg(ss, n), V::I(cc)]))
        },
    );
    rows(out)
}

fn q17150(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(score.and(&cc))
        .drive(|p, (s, c)| v.push((s, p, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner"]);
        f.push(V::I(c));
        row(f)
    }))
}

fn q15811(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post.with(owner_user).select(creation_date.and(&cc)).drive(|p, (cd, c)| v.push((cd, p, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(V::I(c));
        row(f)
    }))
}

fn q17517(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = title
        .opt()
        .and(creation_date)
        .and(owner_user.select(&db.user.display_name).opt());
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(children_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((t, cd), dn), n| v.push((cd, t, dn, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, t, dn, n)| {
        row(vec![ostr(t), V::T(cd), ostr(dn), V::I(n)])
    }))
}

fn q15332(db: &'static So) -> String {
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
        .drive(|(((dn, t), cd), s), n| v.push((s, dn, t, cd, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(s, dn, t, cd, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), V::I(s), V::I(n)])
    }))
}

fn q16259(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, view_count, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(view_count.opt());
    let mut out = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|(((dn, t), cd), vc), n| {
            out.push(row(vec![V::S(dn), ostr(t), V::T(cd), oint(vc), V::I(n)]))
        });
    rows(out)
}

fn q18573(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, score, owner_user, .. } = &db.post;
    let key = title
        .opt()
        .and(owner_user.select(&db.user.display_name))
        .and(creation_date)
        .and(score);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(votes_of(db).opt())
        .fold(0i64, |a, x| a + x.is_some() as i64)
        .drive(|(((t, dn), cd), s), n| v.push((s, t, dn, cd, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(s, t, dn, cd, n)| {
        row(vec![ostr(t), V::S(dn), V::T(cd), V::I(s), V::I(n)])
    }))
}

fn q16759(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, score, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(score)
        .and(creation_date);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|(((dn, t), s), cd), n| v.push((cd, dn, t, s, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, dn, t, s, n)| {
        row(vec![V::S(dn), ostr(t), V::I(s), V::T(cd), V::I(n)])
    }))
}

// The cut falls inside a four-way tie at CommentCount 25, so rewrites/17276
// refines the order with the group key itself. `Option` orders None below
// every Some, which is the `NULLS FIRST` the rewrite asks for on Title.
fn q17276(db: &'static So) -> String {
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

pub static ENTRIES: &[harness::Entry] = &[
    ("11129", q11129),
    ("12428", q12428),
    ("12576", q12576),
    ("14591", q14591),
    ("15332", q15332),
    ("15811", q15811),
    ("16259", q16259),
    ("16759", q16759),
    ("17150", q17150),
    ("17276", q17276),
    ("17517", q17517),
    ("18573", q18573),
];
