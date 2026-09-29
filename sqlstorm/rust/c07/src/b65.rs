use harness::prelude::*;

// Posts per user: count, score sum, and the viewcount count/sum, which a
// nullable ViewCount needs kept apart. `dense_fold_outer` so a user with no
// posts still gets a row — every query here LEFT JOINs Posts.
fn user_posts(db: &'static So) -> DenseFold<Id<User>, (i64, i64, i64, i64)> {
    let Post { score, view_count, owner_user, .. } = &db.post;
    owner_user.inv().select(score.and(view_count.opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64),
        |(n, ss, vn, vs), (s, v)| {
            (n + 1, ss + s, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        },
    )
}

fn q11659(db: &'static So) -> String {
    let User { origid, reputation, creation_date, .. } = &db.user;
    let agg = user_posts(db);
    let mut out = Vec::new();
    db.user.select(origid.and(reputation).and(creation_date).and(&agg)).drive(
        |_, (((id, rep), cd), (n, ss, vn, vs))| {
            out.push(row(vec![
                V::I(id),
                V::I(rep),
                V::T(cd),
                V::I(n),
                avg(ss, n),
                nullable(vs, vn),
            ]))
        },
    );
    rows(out)
}

fn q14804(db: &'static So) -> String {
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = user_posts(db);
    let mut out = Vec::new();
    db.user.select(origid.and(display_name).and(reputation).and(&agg)).drive(
        |_, (((id, dn), rep), (n, ss, vn, vs))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                avg(ss, n),
                nullable(vs, vn),
                V::I(rep),
            ]))
        },
    );
    rows(out)
}

fn q12051(db: &'static So) -> String {
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = user_posts(db);
    let mut out = Vec::new();
    db.user.select(origid.and(display_name).and(reputation).and(&agg)).drive(
        |_, (((id, dn), rep), (n, ss, vn, vs))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(rep),
                V::I(n),
                avg(vs, vn),
                nullable(ss, n),
            ]))
        },
    );
    rows(out)
}

fn q13367(db: &'static So) -> String {
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = user_posts(db);
    let mut out = Vec::new();
    db.user.select(origid.and(display_name).and(reputation).and(&agg)).drive(
        |_, (((id, dn), rep), (n, ss, vn, vs))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                avg(ss, n),
                avg(vs, vn),
                V::I(rep),
            ]))
        },
    );
    rows(out)
}

// The comments join multiplies: a post with three comments is three rows, so
// COUNT(P.Id) and AVG(P.Score) count it three times. See
// notes/translation-failures.md 3.
fn q14632(db: &'static So) -> String {
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

fn q13971(db: &'static So) -> String {
    let Post { post_type, view_count, accepted_answer, .. } = &db.post;
    let mut out = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(view_count.opt().and(accepted_answer.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, vn, vs, a), (v, aa)| {
            (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0), a + aa.is_some() as i64)
        })
        .drive(|k, (n, vn, vs, a)| {
            out.push(row(vec![V::S(k), V::I(n), avg(vs, vn), V::I(a)]))
        });
    rows(out)
}

fn q11881(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let mut out = Vec::new();
    db.post
        .group_by(post_type_id)
        .select(score.and(creation_date).and(comments_of(db).select(&db.comment.score).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, i64::MIN), |(n, ss, cs, cn, mx), ((s, cd), c)| {
            (n + 1, ss + s, cs + c.unwrap_or(0), cn + c.is_some() as i64, mx.max(cd))
        })
        .drive(|k, (n, ss, cs, cn, mx)| {
            out.push(row(vec![
                V::I(k),
                V::I(n),
                avg(ss, n),
                nullable(cs, cn),
                V::I(cn),
                V::T(mx),
            ]))
        });
    rows(out)
}

fn q10253(db: &'static So) -> String {
    let Post { post_type, score, view_count, .. } = &db.post;
    let mut out = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(votes_of(db).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, ss, vn, vs, vc), ((s, v), vote)| {
            (
                n + 1,
                ss + s,
                vn + v.is_some() as i64,
                vs + v.unwrap_or(0),
                vc + vote.is_some() as i64,
            )
        })
        .drive(|k, (n, ss, vn, vs, vc)| {
            out.push(row(vec![V::S(k), V::I(n), avg(ss, n), avg(vs, vn), V::I(vc)]))
        });
    rows(out)
}

// GROUP BY u.DisplayName alone, so the eight rep>1000 users who share a
// display name merge: aggregate per user first, then group those by name.
fn q15793(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let Vote { post, vote_type_id, bounty_amount, .. } = &db.vote;
    let User { display_name, reputation, .. } = &db.user;
    let bounty: HashIdx<Id<Post>, Id<Vote>> =
        db.vote.with(vote_type_id.eq(8)).select(post).inv().collect();
    let per_user = owner_user
        .inv()
        .select(bounty.select(bounty_amount).opt())
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64), |(n, bs, bn), b| {
            (n + 1, bs + b.unwrap_or(0), bn + b.is_some() as i64)
        });
    let mut v = Vec::new();
    db.user
        .with(reputation.gt(1000))
        .group_by(display_name)
        .select(&per_user)
        .fold((0i64, 0i64, 0i64), |(a, b, c), (x, y, z)| (a + x, b + y, c + z))
        .drive(|dn, (n, bs, bn)| v.push((n, dn, bs, bn)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(n, dn, bs, bn)| {
        row(vec![V::S(dn), V::I(n), nullable(bs, bn)])
    }))
}

fn q16549(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = title.opt().and(creation_date).and(owner_user.select(&db.user.display_name));
    let mut out = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(children_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((t, cd), dn), n| {
            out.push(row(vec![ostr(t), V::T(cd), V::S(dn), V::I(n)]))
        });
    rows(out)
}

fn q18677(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.origid)
        .and(owner_user.select(&db.user.display_name))
        .and(title.opt())
        .and(creation_date);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|(((id, dn), t), cd), n| v.push((n, id, t, cd, dn)));
    v.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| (a.1, a.2, a.3).cmp(&(b.1, b.2, b.3))));
    rows(v.iter().take(10).map(|&(n, id, t, cd, dn)| {
        row(vec![V::I(id), V::S(dn), ostr(t), V::T(cd), V::I(n)])
    }))
}

// No PostTypeId filter, so answers come in with a NULL Title and 620 posts
// share a (DisplayName, Title, CreationDate) key: the group is the tuple,
// not the post, and `title.opt()` is what puts the answers in it.
fn q16584(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date);
    let mut v = Vec::new();
    db.post
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((dn, t), cd), n| v.push((cd, dn, t, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, dn, t, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), V::I(n)])
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10253", q10253),
    ("11659", q11659),
    ("11881", q11881),
    ("12051", q12051),
    ("13367", q13367),
    ("13971", q13971),
    ("14632", q14632),
    ("14804", q14804),
    ("15793", q15793),
    ("16549", q16549),
    ("16584", q16584),
    ("18677", q18677),
];
