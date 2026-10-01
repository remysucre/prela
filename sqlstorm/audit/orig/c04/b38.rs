use harness::prelude::*;

fn q7587(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, score, view_count, answer_count, comment_count, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let agg = (&rn)
        .filt(|(_, n)| n <= 3)
        .map(|(((p, _), _), _)| p)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, an, asum, c), (((sc, v), a), cm)| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0), an + a.is_some() as i64, asum + a.unwrap_or(0), c + cm)
        });
    let mut v = Vec::new();
    agg.drive(|dn, a| v.push((dn, a)));
    v.sort_by(|x, y| y.1.0.cmp(&x.1.0).then((y.1.1 as f64 / y.1.0 as f64).partial_cmp(&(x.1.1 as f64 / x.1.0 as f64)).unwrap()));
    rows(v.iter().take(10).map(|&(dn, (n, s, vn, vs, an, asum, c))| {
        row(vec![V::S(dn), V::I(n), avg(s, n), nullable(vs, vn), nullable(asum, an), V::I(c)])
    }))
}

fn q4719(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;
    let recent = db.post.with(creation_date.ge(ts(2024, 10, 1, 12, 34, 56) - 30 * DAY_US));
    let rk = (&recent)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(rank, |(_, cd)| cd, desc);
    let latest: HashIdx<Id<User>, Id<Post>> = (&rk).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let bc = badges_per_user(db);
    let mut v = Vec::new();
    db.user
        .with(reputation.gt(1000))
        .select(display_name.and(reputation).and(&bc).and(latest.opt()))
        .drive(|_, x| v.push(x));
    v.sort_by(|a, b| (b.0.0.1, b.0.1).cmp(&(a.0.0.1, a.0.1)));
    rows(v.iter().take(10).map(|&(((dn, rep), b), p)| {
        let mut f = vec![V::S(dn), V::I(rep), V::I(b)];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "created"])),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

pub const ENTRIES: &[harness::Entry] = &[("7587", q7587), ("4719", q4719)];
