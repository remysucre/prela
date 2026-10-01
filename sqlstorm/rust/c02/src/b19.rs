use harness::prelude::*;
use crate::q::by_created;

fn q10397(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.views_sum, a.views_n),
            avg(a.score_sum, a.n),
            avg(a.comment_sum, a.n),
        ])
    }))
}

fn q14665(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.answers_sum, a.answers_n),
            avg(a.comment_sum, a.n),
        ])
    }))
}

fn q10534(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            V::I(a.score_max),
            V::I(a.score_min),
        ])
    }))
}

fn q12884(db: &'static So) -> String {
    let Post { post_type, score, view_count, last_activity_date, .. } = &db.post;
    let g = db
        .post
        .group_by(post_type)
        .select(score.and(view_count.opt()).and(last_activity_date))
        .fold([0, 0, 0, 0, i64::MIN], |a, ((s, w), la)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4].max(la)]);
    rows(drain(&g).into_iter().map(|(t, a)| row(vec![tname(db, t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::T(a[4])])))
}

fn q12908(db: &'static So) -> String {
    rows(owned_by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.views_sum, a.views_n),
            avg(a.rep_sum, a.n),
        ])
    }))
}

fn q11634(db: &'static So) -> String {
    let Post { creation_date, post_type, score, last_edit_date, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut sc: Vec<(Str, i64, i64, i64, i64)> = Vec::new();
    db.post
        .with(creation_date.ge(date(2023, 1, 1)))
        .group_by(post_type.select(name))
        .select(score.and(last_edit_date.opt()))
        .fold(
            (0i64, 0i64, 0i64, i64::MIN),
            |(n, s, en, m), (x, e)| {
                (n + 1, s + x, en + e.is_some() as i64, e.map_or(m, |y| m.max(y)))
            },
        )
        .drive(|k, (n, s, en, m)| sc.push((k, n, s, en, m)));

    sc.sort_by(|a, b| b.1.cmp(&a.1));
    rows(sc.iter().map(|(k, n, s, en, m)| {
        let last = if *en > 0 { V::T(*m) } else { V::Null };
        row(vec![V::S(k), V::I(*n), avg(*s, *n), last])
    }))
}

fn q15343(db: &'static So) -> String {
    let Post { post_type_id, origid, creation_date, score, view_count, owner_user, .. } =
        &db.post;
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
            V::T(*created),
            V::S(dn),
            V::I(*sc),
            oint(*views),
            V::S(tn),
        ])
    }))
}

fn q17195(db: &'static So) -> String {
    let Post { post_type_id, origid, creation_date, score, view_count, owner_user, .. } =
        &db.post;
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
    v.sort_by(|a, b| b.3.cmp(&a.3));
    rows(v.iter().take(10).map(|(p, id, created, sc, views, dn, tn)| {
        row(vec![
            V::I(*id),
            title(db, *p),
            V::S(dn),
            V::T(*created),
            V::I(*sc),
            oint(*views),
            V::S(tn),
        ])
    }))
}

fn q15089(db: &'static So) -> String {
    let Post { answer_count, comment_count, favorite_count, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            oint(r.views),
            oint(answer_count.get(r.pid)),
            V::I(comment_count.get(r.pid).unwrap()),
            oint(favorite_count.get(r.pid)),
        ])
    }))
}

fn q18479(db: &'static So) -> String {
    let Post { view_count, creation_date, score, owner_user, post_type, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let PostType { name, .. } = &db.post_type;

    let mut v: Vec<(Id<Post>, i64, i64, i64, Str, Str)> = Vec::new();
    db.post
        .with(view_count.gt(1000))
        .select(
            creation_date
                .and(view_count)
                .and(score)
                .and(owner_user.select(display_name))
                .and(post_type.select(name)),
        )
        .drive(|pid, ((((created, views), sc), dn), pt)| {
            v.push((pid, created, views, sc, dn, pt))
        });
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(pid, created, views, sc, dn, pt)| {
        row(vec![
            title(db, *pid),
            V::T(*created),
            V::S(dn),
            V::I(*views),
            V::I(*sc),
            V::S(pt),
        ])
    }))
}

const EDIT_TYPES: [i64; 6] = [4, 5, 6, 10, 11, 12];

fn q12446(db: &'static So) -> String {
    let PostHistory { post_history_type_id, user_id, creation_date, .. } = &db.post_history;
    let g = db
        .post_history
        .with(post_history_type_id.in_v(EDIT_TYPES.to_vec()))
        .group_by(user_id.opt())
        .select(creation_date)
        .fold((0i64, i64::MIN, i64::MAX), |(n, hi, lo), x| (n + 1, hi.max(x), lo.min(x)));
    rows(drain(&g).into_iter().map(|(u, (n, hi, lo))| row(vec![oint(u), V::I(n), V::T(hi), V::T(lo)])))
}

fn q19882(db: &'static So) -> String {
    let PostHistory { post_history_type_id, post, user, creation_date, comment, .. } =
        &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<PostHistory>, Id<Post>, i64, Str)> = Vec::new();
    db.post_history
        .with(post_history_type_id.in_v(vec![4, 5, 6]))
        .select(post.and(creation_date).and(user.select(display_name)))
        .drive(|h, ((p, created), dn)| v.push((h, p, created, dn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(h, p, created, dn)| {
        row(vec![V::S(dn), title(db, *p), V::T(*created), ostr(comment.get(*h))])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("10397", q10397),
    ("11634", q11634),
    ("14665", q14665),
    ("15343", q15343),
    ("12884", q12884),
    ("15089", q15089),
    ("12446", q12446),
    ("12908", q12908),
    ("17195", q17195),
    ("10534", q10534),
    ("18479", q18479),
    ("19882", q19882),
];
