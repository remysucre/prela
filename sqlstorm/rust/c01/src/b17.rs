use harness::prelude::*;
use crate::q::{by_created, by_score};

fn q13246(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
            nullable(a.answers_sum, a.answers_n),
        ])
    }))
}

fn q11234(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.views_sum, a.views_n),
            nullable(a.views_max, a.views_n),
            nullable(a.views_min, a.views_n),
        ])
    }))
}

fn q11731(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.views_sum, a.views_n),
            nullable(a.views_min, a.views_n),
            nullable(a.views_max, a.views_n),
        ])
    }))
}

fn count_avgscore_sumviews_maxcreated(db: &'static So) -> String {
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

fn q13416(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
            V::T(a.last_activity_max),
        ])
    }))
}

fn q15536(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, owner_user, .. } = &db.post;
    let Tag { excerpt_post, tag_name, .. } = &db.tag;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, Str, Str)> = Vec::new();
    db.tag
        .with(excerpt_post.select(post_type_id).eq(1))
        .select(
            excerpt_post
                .and(excerpt_post.select(
                    origid.and(creation_date).and(owner_user.select(display_name)),
                ))
                .and(tag_name),
        )
        .drive(|_, ((p, ((id, created), dn)), tn)| v.push((p, id, created, dn, tn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(p, id, created, dn, tn)| {
        row(vec![V::I(*id), title(db, *p), V::T(*created), V::S(dn), V::S(tn)])
    }))
}

fn q18311(db: &'static So) -> String {
    let Post { creation_date, origid, score, owner_user, .. } = &db.post;
    let Tag { excerpt_post, tag_name, .. } = &db.tag;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, i64, Str, Str)> = Vec::new();
    db.tag
        .with(excerpt_post.select(creation_date).ge(date(2023, 1, 1)))
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
    rows(v.iter().take(10).map(|(p, id, _, sc, dn, tn)| {
        row(vec![V::I(*id), title(db, *p), V::I(*sc), V::S(dn), V::S(tn)])
    }))
}

fn q17640(db: &'static So) -> String {
    let Post { answer_count, tags_str, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.score),
            oint(r.views),
            oint(answer_count.get(r.pid)),
            ostr(tags_str.get(r.pid)),
        ])
    }))
}

fn q18300(db: &'static So) -> String {
    let Post { answer_count, comment_count, .. } = &db.post;
    rows(by_score(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            oint(r.views),
            oint(answer_count.get(r.pid)),
            V::I(comment_count.get(r.pid).unwrap()),
        ])
    }))
}

fn q18330(db: &'static So) -> String {
    let Post { answer_count, comment_count, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::S(r.display_name),
            oint(r.views),
            oint(answer_count.get(r.pid)),
            V::I(comment_count.get(r.pid).unwrap()),
        ])
    }))
}

fn q10461(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let User { origid: uid, display_name, .. } = &db.user;

    let mut all: Vec<(Id<User>, i64, i64)> = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x))
        .drive(|u, (n, s)| all.push((u, n, s)));
    all.sort_by(|a, b| {
        let f = |t: &(Id<User>, i64, i64)| t.2 as f64 / t.1 as f64;
        f(b).partial_cmp(&f(a)).unwrap()
    });
    rows(all.iter().take(10).map(|(u, n, s)| {
        row(vec![
            V::I(uid.get(*u).unwrap()),
            V::S(display_name.get(*u).unwrap()),
            avg(*s, *n),
            V::I(*n),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("13246", q13246),
    ("15536", q15536),
    ("11234", q11234),
    ("17640", q17640),
    ("11755", count_avgscore_sumviews_maxcreated),
    ("12539", count_avgscore_sumviews_maxcreated),
    ("18300", q18300),
    ("10461", q10461),
    ("18311", q18311),
    ("18330", q18330),
    ("11731", q11731),
    ("13416", q13416),
];
