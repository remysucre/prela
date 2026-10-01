use harness::prelude::*;
use crate::q::{by_created, by_score};

fn q16767(db: &'static So) -> String {
    let Post { answer_count, comment_count, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
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

fn q17969(db: &'static So) -> String {
    let Post { answer_count, comment_count, .. } = &db.post;
    rows(by_score(db).iter().map(|r| {
        row(vec![
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

// SELECT u.DisplayName AS UserDisplayName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
// WHERE u.Reputation > 1000 GROUP BY u.DisplayName ORDER BY PostCount DESC LIMIT 10;
fn q18644(db: &'static So) -> String {
    let Post { owner_user, view_count, .. } = &db.post;
    let g = db.post.with(owner_user.select(&db.user.reputation).gt(1000)).group_by(owner_user.select(&db.user.display_name)).select(view_count.opt()).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let v = top_n(drain(&g), |&(_, a)| std::cmp::Reverse(a[0]), 10);
    rows(v.into_iter().map(|(dn, a)| row(vec![V::S(dn), V::I(a[0]), nullable(a[2], a[1])])))
}

fn q12741(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            V::I(a.score_max),
            V::I(a.score_min),
        ])
    }))
}

fn count_avgrep(db: &'static So) -> String {
    let owner_user = &db.post.owner_user;
    let g = db.post.with(owner_user).group_by(ptype_name(db)).select(owner_user.select(&db.user.reputation)).fold([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let v = top_n(drain(&g), |&(_, a)| std::cmp::Reverse(a[0]), 0);
    rows(v.into_iter().map(|(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0])])))
}

fn q15620(db: &'static So) -> String {
    let tags = &db.post.tags_str;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            oint(r.views),
            ostr(tags.get(r.pid)),
        ])
    }))
}

fn q19521(db: &'static So) -> String {
    rows(by_score(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::I(r.score),
            V::S(r.display_name),
            V::I(r.reputation),
        ])
    }))
}

fn q19785(db: &'static So) -> String {
    let Post { origid, creation_date, owner_user, post_type, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let PostType { name, .. } = &db.post_type;

    let mut v: Vec<(Id<Post>, i64, i64, Str, Str)> = Vec::new();
    db.post
        .select(
            origid
                .and(creation_date)
                .and(owner_user.select(display_name))
                .and(post_type.select(name)),
        )
        .drive(|pid, (((id, created), dn), pt)| v.push((pid, id, created, dn, pt)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(pid, id, created, dn, pt)| {
        row(vec![V::I(*id), title(db, *pid), V::T(*created), V::S(dn), V::S(pt)])
    }))
}

// SELECT u.Reputation, COUNT(p.Id) AS TotalPosts, SUM(p.ViewCount) AS TotalViewCount, AVG(p.Score) AS AverageScore
// FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY u.Reputation ORDER BY u.Reputation DESC;
fn q12905(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let g = db.post.with(post_type_id.eq(1)).group_by(owner_user.select(&db.user.reputation)).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    rows(drain(&g).into_iter().map(|(r, a)| row(vec![V::I(r), V::I(a[0]), nullable(a[3], a[2]), avg(a[1], a[0])])))
}

fn q18933(db: &'static So) -> String {
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::I(r.score),
            oint(r.views),
            V::S(r.display_name),
            V::I(r.reputation),
        ])
    }))
}

fn q18862(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let Tag { excerpt_post, tag_name, .. } = &db.tag;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, Option<i64>, Str, Str)> = Vec::new();
    db.tag
        .with(excerpt_post.select(post_type_id).eq(1))
        .select(
            excerpt_post
                .and(excerpt_post.select(
                    creation_date
                        .and(score)
                        .and(view_count.opt())
                        .and(owner_user.select(display_name)),
                ))
                .and(tag_name),
        )
        .drive(|_, ((p, (((created, sc), views), dn)), tn)| {
            v.push((p, created, sc, views, dn, tn))
        });
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(p, created, sc, views, dn, tn)| {
        row(vec![
            title(db, *p),
            V::S(dn),
            V::T(*created),
            V::I(*sc),
            oint(*views),
            V::S(tn),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("16767", q16767),
    ("18644", q18644),
    ("12741", q12741),
    ("12931", count_avgrep),
    ("15620", q15620),
    ("17969", q17969),
    ("19521", q19521),
    ("19785", q19785),
    ("16324", count_avgrep),
    ("12905", q12905),
    ("18933", q18933),
    ("18862", q18862),
];
