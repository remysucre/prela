use harness::prelude::*;

fn q12476(db: &'static So) -> String {
    let Post { post_type, post_type_id, score, view_count, .. } = &db.post;
    let mut out = Vec::new();
    db.post
        .with(post_type_id.is_in([1, 2]))
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(comments_of(db).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, ss, vn, vs, cn), ((s, v), c)| {
            (
                n + 1,
                ss + s,
                vn + v.is_some() as i64,
                vs + v.unwrap_or(0),
                cn + c.is_some() as i64,
            )
        })
        .drive(|k, (n, ss, vn, vs, cn)| {
            out.push(row(vec![V::S(k), avg(ss, n), avg(vs, vn), V::I(cn)]))
        });
    rows(out)
}

fn q15898(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = title.opt().and(creation_date).and(owner_user.select(&db.user.display_name));
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(children_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((t, cd), dn), n| v.push((cd, t, dn, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, t, dn, n)| {
        row(vec![ostr(t), V::T(cd), V::S(dn), V::I(n)])
    }))
}

// LEFT JOIN Users, not JOIN: the 2,874 ownerless questions stay, in a group
// whose DisplayName is NULL. That is what `.opt()` on the key gives.
fn q18978(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = title
        .opt()
        .and(creation_date)
        .and(owner_user.select(&db.user.display_name).opt());
    let mut out = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(children_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((t, cd), dn), n| {
            out.push(row(vec![ostr(t), V::T(cd), ostr(dn), V::I(n)]))
        });
    rows(out)
}

fn q15577(db: &'static So) -> String {
    let Post { post_type_id, title, score, owner_user, .. } = &db.post;
    let key = title.opt().and(score).and(owner_user.select(&db.user.display_name));
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((t, s), dn), n| v.push((s, t, dn, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(s, t, dn, n)| {
        row(vec![ostr(t), V::I(s), V::S(dn), V::I(n)])
    }))
}

fn q11321(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let User { origid, display_name, .. } = &db.user;
    let agg = owner_user.inv().select(score.and(comments_of(db).opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64),
        |(n, ss, cc), (s, c)| (n + 1, ss + s, cc + c.is_some() as i64),
    );
    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(&agg))
        .drive(|_, ((id, dn), (n, ss, cc))| v.push((n, id, dn, ss, cc)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(100).map(|&(n, id, dn, ss, cc)| {
        row(vec![V::I(id), V::S(dn), V::I(n), avg(ss, n), V::I(cc)])
    }))
}

// AVG(u.Reputation) is grouped by the user, and a user with no posts still
// has one LEFT JOIN row, so the average is always just the reputation.
fn q14012(db: &'static So) -> String {
    let Post { score, last_activity_date, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = owner_user.inv().select(score.and(last_activity_date)).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, i64::MIN),
        |(n, ss, mx), (s, la)| (n + 1, ss + s, mx.max(la)),
    );
    let mut out = Vec::new();
    db.user.select(origid.and(display_name).and(reputation).and(&agg)).drive(
        |_, (((id, dn), rep), (n, ss, mx))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                nullable(ss, n),
                V::F(rep as f64),
                if n == 0 { V::Null } else { V::T(mx) },
            ]))
        },
    );
    rows(out)
}

fn q15531(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(creation_date.and(comments_of(db).opt()))
        .drive(|p, (cd, c)| v.push((cd, p, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["title", "created", "owner", "views", "score", "answers"]);
        f.push(oint(c.map(|c| db.comment.score.get(c).unwrap())));
        row(f)
    }))
}

fn q15529(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = title.opt().and(owner_user.select(&db.user.display_name)).and(creation_date);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((t, dn), cd), n| v.push((cd, t, dn, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, t, dn, n)| {
        row(vec![ostr(t), V::S(dn), V::T(cd), V::I(n)])
    }))
}

fn q11261(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = owner_user
        .inv()
        .select(score)
        .dense_fold_outer(db.user.id.n, (0i64, 0i64), |(n, ss), s| (n + 1, ss + s));
    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(&agg))
        .drive(|_, (((id, dn), rep), (n, ss))| v.push((rep, n, id, dn, ss)));
    v.sort_by(|a, b| {
        let m = |x: &(i64, i64, i64, Str, i64)| {
            if x.1 == 0 { f64::NEG_INFINITY } else { x.4 as f64 / x.1 as f64 }
        };
        (b.0, b.1).cmp(&(a.0, a.1)).then_with(|| m(b).total_cmp(&m(a)))
    });
    rows(v.iter().take(10).map(|&(rep, n, id, dn, ss)| {
        row(vec![V::I(id), V::S(dn), V::I(rep), V::I(n), avg(ss, n)])
    }))
}

fn type_accepted(db: &'static So) -> String {
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

fn q13002(db: &'static So) -> String {
    type_accepted(db)
}

fn q10656(db: &'static So) -> String {
    type_accepted(db)
}

// MAX over a VARCHAR. `Option<&str>` orders None below every Some, so the
// plain `.max` is both "ignore the NULLs" and "compare by bytes", which is
// what DuckDB does.
fn q11317(db: &'static So) -> String {
    let Post { score, title, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = owner_user.inv().select(score.and(title.opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, i64::MIN, None::<Str>),
        |(n, ms, mt), (s, t)| (n + 1, ms.max(s), mt.max(t)),
    );
    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(&agg))
        .drive(|_, (((id, dn), rep), (n, ms, mt))| v.push((rep, n, id, dn, ms, mt)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(rep, n, id, dn, ms, mt)| {
        row(vec![
            V::I(id),
            V::S(dn),
            V::I(rep),
            V::I(n),
            if n == 0 { V::Null } else { V::I(ms) },
            ostr(mt),
        ])
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10656", q10656),
    ("11261", q11261),
    ("11317", q11317),
    ("11321", q11321),
    ("12476", q12476),
    ("13002", q13002),
    ("14012", q14012),
    ("15529", q15529),
    ("15531", q15531),
    ("15577", q15577),
    ("15898", q15898),
    ("18978", q18978),
];
