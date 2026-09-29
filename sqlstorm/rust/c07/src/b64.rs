use harness::prelude::*;

fn q10210(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![V::S(a.name), V::I(a.n), avg(a.score_sum, a.n), avg(a.rep_sum, a.rep_n)])
    }))
}

// DuckDB sums DOUBLEs with Kahan compensation, so a naive fold drifts on a
// group of six figures. See notes/translation-failures.md.
fn kahan((s, err): (f64, f64), x: f64) -> (f64, f64) {
    let y = x - err;
    let t = s + y;
    (t, (t - s) - y)
}

fn q11087(db: &'static So) -> String {
    let Post { post_type, score, creation_date, last_activity_date, .. } = &db.post;
    let mut out = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(last_activity_date.and(creation_date).and(score))
        .fold((0i64, (0.0f64, 0.0f64), 0i64), |(n, secs, sc), ((la, cd), s)| {
            (n + 1, kahan(secs, (la - cd) as f64 / 1e6), sc + s)
        })
        .drive(|k, (n, (secs, _), sc)| {
            out.push(row(vec![V::S(k), V::I(n), V::F(secs / n as f64), V::I(sc)]))
        });
    rows(out)
}

fn q10790(db: &'static So) -> String {
    let Post { post_type_id, view_count, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(view_count.and(&cc))
        .drive(|p, (vc, c)| v.push((vc, p, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(100).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.push(V::I(c));
        row(f)
    }))
}

fn q16079(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(score.and(comments_of(db).opt()))
        .drive(|p, (s, c)| v.push((s, p, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "score"]);
        f.extend([
            ostr(c.map(|c| db.comment.text.get(c).unwrap())),
            ots(c.map(|c| db.comment.creation_date.get(c).unwrap())),
        ]);
        row(f)
    }))
}

fn q19189(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(score.and(creation_date).and(comments_of(db).opt()))
        .drive(|p, ((s, cd), c)| v.push((s, cd, p, c)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(_, _, p, c)| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "score"]);
        f.push(ostr(c.map(|c| db.comment.text.get(c).unwrap())));
        f.extend(post_fields(db, p, &["activity"]));
        row(f)
    }))
}

fn q19422(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(creation_date.and(comments_of(db).opt()))
        .drive(|p, (cd, c)| v.push((cd, p, c)));
    v.sort_by(|a, b| {
        let key = |x: &(i64, Id<Post>, Option<Id<Comment>>)| {
            x.2.map(|c| db.comment.origid.get(c).unwrap())
        };
        b.0.cmp(&a.0).then_with(|| match (key(a), key(b)) {
            (Some(x), Some(y)) => x.cmp(&y),
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        })
    });
    rows(v.iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.push(ostr(c.map(|c| db.comment.text.get(c).unwrap())));
        row(f)
    }))
}

fn q11753(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = owner_user
        .inv()
        .select(score.and(view_count.opt()))
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64, 0i64), |(n, ss, vn, vs), (s, v)| {
            (n + 1, ss + s, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        });
    let mut out = Vec::new();
    db.user.select(origid.and(display_name).and(reputation).and(&agg)).drive(
        |_, (((id, dn), rep), (n, ss, vn, vs))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(rep),
                V::I(n),
                avg(ss, n),
                avg(vs, vn),
            ]))
        },
    );
    rows(out)
}

fn q15669(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post.with(owner_user).select(creation_date.and(&cc)).drive(|p, (cd, c)| v.push((cd, p, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.push(V::I(c));
        row(f)
    }))
}

fn q15370(db: &'static So) -> String {
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

fn comment_count_by_name_title_date(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((dn, t), cd), n| v.push((cd, dn, t, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, dn, t, n)| row(vec![V::S(dn), ostr(t), V::T(cd), V::I(n)])))
}

fn q15151(db: &'static So) -> String {
    comment_count_by_name_title_date(db)
}

fn q15304(db: &'static So) -> String {
    comment_count_by_name_title_date(db)
}

fn q15678(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = title.opt().and(creation_date).and(owner_user.select(&db.user.display_name));
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(votes_of(db).opt())
        .fold(0i64, |a, x| a + x.is_some() as i64)
        .drive(|((t, cd), dn), n| v.push((n, t, cd, dn)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(n, t, cd, dn)| row(vec![ostr(t), V::T(cd), V::S(dn), V::I(n)])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10210", q10210),
    ("10790", q10790),
    ("11087", q11087),
    ("11753", q11753),
    ("15151", q15151),
    ("15304", q15304),
    ("15370", q15370),
    ("15669", q15669),
    ("15678", q15678),
    ("16079", q16079),
    ("19189", q19189),
    ("19422", q19422),
];
