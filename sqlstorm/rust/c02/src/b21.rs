use harness::prelude::*;

fn edit_type4(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let PostHistory { post_history_type_id, post, creation_date, .. } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str)> = Vec::new();
    db.post_history
        .with(post_history_type_id.eq(4))
        .select(post.and(creation_date).and(post.select(owner_user.select(display_name))))
        .drive(|_, ((p, created), dn)| v.push((p, created, dn)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(p, created, dn)| {
        row(vec![
            V::S(dn),
            title(db, *p),
            V::T(*created),
            V::I(score.get(*p).unwrap()),
        ])
    }))
}

fn q16085(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let Comment { post, text, creation_date: cdate, .. } = &db.comment;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str, Str, i64)> = Vec::new();
    db.comment
        .with(post.select(post_type_id).eq(1))
        .select(
            post.and(post.select(creation_date.and(owner_user.select(display_name))))
                .and(text)
                .and(cdate),
        )
        .drive(|_, (((p, (pcreated, dn)), txt), cc)| v.push((p, pcreated, dn, txt, cc)));
    v.sort_by(|a, b| b.4.cmp(&a.4));
    rows(v.iter().take(10).map(|(p, pcreated, dn, txt, cc)| {
        row(vec![V::S(dn), title(db, *p), V::T(*pcreated), V::S(txt), V::T(*cc)])
    }))
}

fn editor_stats(db: &'static So) -> String {
    let PostHistory { post_history_type_id, user, creation_date, .. } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut all: Vec<((i64, Str), i64, i64, i64)> = Vec::new();
    db.post_history
        .group_by(post_history_type_id.and(user.select(display_name)))
        .select(creation_date)
        .fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), x| {
            (n + 1, lo.min(x), hi.max(x))
        })
        .drive(|k, (n, lo, hi)| all.push((k, n, lo, hi)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().map(|((t, dn), n, lo, hi)| {
        row(vec![V::I(*t), V::I(*n), V::T(*lo), V::T(*hi), V::S(dn)])
    }))
}

fn q13701(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
            nullable(a.answers_sum, a.answers_n),
            V::I(a.comment_sum),
        ])
    }))
}

fn q14333(db: &'static So) -> String {
    rows(owned_by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.rep_sum, a.n),
            nullable(a.views_sum, a.views_n),
            V::I(a.score_sum),
        ])
    }))
}

fn q13012(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
            V::T(a.created_max),
            V::T(a.created_min),
        ])
    }))
}

fn q16748(db: &'static So) -> String {
    let Post { post_type_id, body, owner_user, .. } = &db.post;
    let PostHistory { post, creation_date, comment, .. } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<PostHistory>, Id<Post>, i64, Str, Str)> = Vec::new();
    db.post_history
        .with(post.select(post_type_id).eq(1))
        .select(
            post.and(creation_date)
                .and(post.select(body.and(owner_user.select(display_name)))),
        )
        .drive(|h, ((p, created), (bd, dn))| v.push((h, p, created, bd, dn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(h, p, created, bd, dn)| {
        row(vec![
            V::S(dn),
            title(db, *p),
            V::T(*created),
            V::S(bd),
            ostr(comment.get(*h)),
        ])
    }))
}

// SELECT u.DisplayName, COUNT(p.Id) AS TotalQuestions, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViewCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
// WHERE p.PostTypeId = 1 AND u.Reputation > 100 GROUP BY u.DisplayName ORDER BY AverageScore DESC, AverageViewCount DESC;
fn q12193(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let g = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user.select(&db.user.reputation).gt(100))
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    rows(drain(&g).into_iter().map(|(dn, a)| row(vec![V::S(dn), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2])])))
}

fn q18385(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let PostHistory {
        post_history_type_id, post, creation_date: hdate, comment, ..
    } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<PostHistory>, Id<Post>, i64, i64, Str)> = Vec::new();
    db.post_history
        .with(post_history_type_id.in_v(vec![4, 5]))
        .select(
            post.and(hdate)
                .and(post.select(creation_date.and(owner_user.select(display_name)))),
        )
        .drive(|h, ((p, hc), (pc, dn))| v.push((h, p, pc, hc, dn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(h, p, pc, hc, dn)| {
        row(vec![
            V::S(dn),
            title(db, *p),
            V::T(*pc),
            ostr(comment.get(*h)),
            V::T(*hc),
        ])
    }))
}

fn q19329(db: &'static So) -> String {
    let Post {
        post_type_id, origid, creation_date, score, view_count, answer_count, owner_user,
        ..
    } = &db.post;
    let Tag { excerpt_post, tag_name, .. } = &db.tag;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, i64, Option<i64>, Str, Str)> = Vec::new();
    db.tag
        .with(excerpt_post.select(post_type_id).eq(1))
        .select(
            excerpt_post
                .and(excerpt_post.select(
                    origid
                        .and(creation_date)
                        .and(score)
                        .and(view_count.opt())
                        .and(owner_user.select(display_name)),
                ))
                .and(tag_name),
        )
        .drive(|_, ((p, ((((id, created), sc), views), dn)), tn)| {
            v.push((p, id, created, sc, views, dn, tn))
        });
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(p, id, created, sc, views, dn, tn)| {
        row(vec![
            V::I(*id),
            title(db, *p),
            V::S(dn),
            V::T(*created),
            V::I(*sc),
            oint(*views),
            oint(answer_count.get(*p)),
            V::S(tn),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("15837", edit_type4),
    ("18346", edit_type4),
    ("16085", q16085),
    ("11685", editor_stats),
    ("13701", q13701),
    ("14333", q14333),
    ("16748", q16748),
    ("13012", q13012),
    ("12193", q12193),
    ("18385", q18385),
    ("19329", q19329),
    ("12753", editor_stats),
];
