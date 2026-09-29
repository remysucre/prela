use harness::prelude::*;

fn q13875(db: &'static So) -> String {
    let PostHistory { post_id, creation_date, .. } = &db.post_history;

    let mut all: Vec<(i64, i64, i64, i64)> = Vec::new();
    db.post_history
        .group_by(post_id)
        .select(creation_date)
        .fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), x| {
            (n + 1, lo.min(x), hi.max(x))
        })
        .drive(|k, (n, lo, hi)| all.push((k, n, lo, hi)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().map(|(pid, n, lo, hi)| {
        row(vec![V::I(*pid), V::I(*n), V::T(*lo), V::T(*hi), V::Iv(hi - lo)])
    }))
}

fn q16102(db: &'static So) -> String {
    let Post { owner_user_id, answer_count, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            oint(owner_user_id.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            V::I(r.views),
            oint(answer_count.get(r.pid)),
        ])
    }))
}

fn q10825(db: &'static So) -> String {
    rows(owned_by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.rep_sum, a.n),
        ])
    }))
}

fn q18202(db: &'static So) -> String {
    rows(owned_by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            nullable(a.views_sum, a.views_n),
            avg(a.rep_sum, a.n),
        ])
    }))
}

fn q13711(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.views_sum, a.views_n),
            avg(a.score_sum, a.n),
            avg(a.answers_sum, a.answers_n),
        ])
    }))
}

fn q10488(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            avg(a.comment_sum, a.n),
        ])
    }))
}

fn q14878(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
            V::T(a.created_max),
        ])
    }))
}

fn q12394(db: &'static So) -> String {
    let mut a = type_aggs(db);
    a.sort_by(|x, y| {
        y.views_sum.cmp(&x.views_sum).then(y.score_sum.cmp(&x.score_sum))
    });
    rows(a.iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            V::I(a.score_sum),
            nullable(a.views_sum, a.views_n),
            nullable(a.answers_sum, a.answers_n),
        ])
    }))
}

// SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS Author, p.Tags FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// WHERE p.PostTypeId = 1 AND p.Score > 0 ORDER BY p.Score DESC, p.ViewCount DESC LIMIT 100;
fn q14411(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(score.and(view_count.opt())));
    let v = top_n(v, |&(_, (s, w))| (std::cmp::Reverse(s), w.is_none(), std::cmp::Reverse(w)), 100);
    rows(v.into_iter().map(|(p, _)| row(post_fields(db, p, &["id", "title", "score", "views", "created", "owner", "tags"]))))
}

fn q16359(db: &'static So) -> String {
    let Post { post_type_id, origid, creation_date, score, owner_user, .. } = &db.post;
    let Tag { excerpt_post, tag_name, .. } = &db.tag;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, i64, Str, Str)> = Vec::new();
    db.tag
        .with(excerpt_post.select(post_type_id).eq(1))
        .select(
            excerpt_post
                .and(excerpt_post.select(
                    origid
                        .and(creation_date)
                        .and(score)
                        .and(owner_user.select(display_name)),
                ))
                .and(tag_name),
        )
        .drive(|_, ((p, (((id, created), sc), dn)), tn)| {
            v.push((p, id, created, sc, dn, tn))
        });
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(p, id, created, sc, dn, tn)| {
        row(vec![
            V::I(*id),
            title(db, *p),
            V::I(*sc),
            V::S(dn),
            V::T(*created),
            V::S(tn),
        ])
    }))
}

fn q15586(db: &'static So) -> String {
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.display_name),
            V::I(r.reputation),
            V::T(r.created),
            V::I(r.score),
            V::I(r.views),
        ])
    }))
}

fn q11329(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let User { origid: uid, display_name, reputation, .. } = &db.user;

    let mut all: Vec<(Id<User>, i64, i64)> = Vec::new();
    db.post
        .with(score.gt(0))
        .group_by(owner_user)
        .select(score)
        .fold((0i64, i64::MIN), |(n, hi), x| (n + 1, hi.max(x)))
        .drive(|u, (n, hi)| all.push((u, n, hi)));
    all.sort_by(|a, b| {
        reputation.get(b.0).unwrap().cmp(&reputation.get(a.0).unwrap())
    });
    rows(all.iter().take(10).map(|(u, n, hi)| {
        row(vec![
            V::I(uid.get(*u).unwrap()),
            V::S(display_name.get(*u).unwrap()),
            V::F(reputation.get(*u).unwrap() as f64),
            V::I(*n),
            V::I(*hi),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("13875", q13875),
    ("16102", q16102),
    ("10825", q10825),
    ("13711", q13711),
    ("14411", q14411),
    ("16359", q16359),
    ("14878", q14878),
    ("10488", q10488),
    ("15586", q15586),
    ("11329", q11329),
    ("12394", q12394),
    ("18202", q18202),
];
