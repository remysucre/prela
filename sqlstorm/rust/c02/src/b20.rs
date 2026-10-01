use harness::prelude::*;
use crate::q::by_created;

fn q15108(db: &'static So) -> String {
    let Post { answer_count, comment_count, tags_str, .. } = &db.post;
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
            ostr(tags_str.get(r.pid)),
        ])
    }))
}

fn q16421(db: &'static So) -> String {
    let Post { answer_count, .. } = &db.post;
    let User { origid: uid, .. } = &db.user;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(uid.get(r.uid).unwrap()),
            V::S(r.display_name),
            V::I(r.reputation),
            title(db, r.pid),
            V::T(r.created),
            oint(r.views),
            oint(answer_count.get(r.pid)),
            V::I(r.score),
        ])
    }))
}

struct TRow {
    pid: Id<Post>,
    id: i64,
    created: i64,
    score: i64,
    display_name: Str,
    type_name: Str,
}

fn typed(db: &'static So, pred: Pred) -> Vec<TRow> {
    let Post { origid, creation_date, score, view_count, owner_user, post_type, .. } =
        &db.post;
    let User { display_name, .. } = &db.user;
    let PostType { name, .. } = &db.post_type;

    let proj = origid
        .and(creation_date)
        .and(score)
        .and(owner_user.select(display_name))
        .and(post_type.select(name));

    let mut v: Vec<TRow> = Vec::new();
    let mut push = |pid, ((((id, created), sc), display_name), type_name)| {
        v.push(TRow {
            pid,
            id,
            created,
            score: sc,
            display_name,
            type_name,
        })
    };
    match pred {
        Pred::Score(n) => db.post.with(score.gt(n)).select(proj).drive(&mut push),
        Pred::Views(n) => db.post.with(view_count.gt(n)).select(proj).drive(&mut push),
    }
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

enum Pred {
    Score(i64),
    Views(i64),
}

fn q19599(db: &'static So) -> String {
    let vc = &db.post.view_count;
    rows(typed(db, Pred::Score(10)).iter().map(|r| {
        row(vec![
            title(db, r.pid),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            oint(vc.get(r.pid)),
            V::S(r.type_name),
        ])
    }))
}

fn q17192(db: &'static So) -> String {
    rows(typed(db, Pred::Views(100)).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            V::S(r.type_name),
        ])
    }))
}

fn q17800(db: &'static So) -> String {
    let vc = &db.post.view_count;
    rows(typed(db, Pred::Score(0)).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.type_name),
            V::S(r.display_name),
            V::T(r.created),
            oint(vc.get(r.pid)),
            V::I(r.score),
        ])
    }))
}

fn q11607(db: &'static So) -> String {
    let PostHistory { post_history_type_id, creation_date, user_id, .. } =
        &db.post_history;

    let mut cd: Vec<(i64, i64, i64, i64, i64, i64, i64)> = Vec::new();
    db.post_history
        .group_by(post_history_type_id)
        .select(creation_date.and(user_id.opt()))
        .fold(
            (0i64, i64::MAX, i64::MIN, 0i64, i64::MAX, i64::MIN),
            |(n, lo, hi, un, ulo, uhi), (x, u)| {
                (
                    n + 1,
                    lo.min(x),
                    hi.max(x),
                    un + u.is_some() as i64,
                    u.map_or(ulo, |y| ulo.min(y)),
                    u.map_or(uhi, |y| uhi.max(y)),
                )
            },
        )
        .drive(|k, (n, lo, hi, un, ulo, uhi)| cd.push((k, n, lo, hi, un, ulo, uhi)));

    cd.sort_by(|a, b| b.1.cmp(&a.1));
    rows(cd.iter().map(|(k, n, lo, hi, un, ulo, uhi)| {
        row(vec![
            V::I(*k),
            V::I(*n),
            V::T(*lo),
            V::T(*hi),
            nullable(*ulo, *un),
            nullable(*uhi, *un),
        ])
    }))
}

fn q11799(db: &'static So) -> String {
    let Post { post_type, score, owner_user, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;
    let User { reputation, .. } = &db.user;

    let mut all: Vec<((Str, i64), i64, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(post_type.select(name).and(owner_user.select(reputation)))
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x))
        .drive(|k, (n, s)| all.push((k, n, s)));
    all.sort_by(|a, b| a.0.0.cmp(b.0.0).then(a.0.1.cmp(&b.0.1)));
    rows(all.iter().map(|((nm, rep), n, s)| {
        row(vec![V::S(nm), V::I(*rep), V::I(*n), avg(*s, *n)])
    }))
}

fn q15400(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let PostHistory { post_history_type_id, post, creation_date, .. } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str)> = Vec::new();
    db.post_history
        .with(post_history_type_id.in_v(vec![4, 5]))
        .select(post.and(creation_date).and(post.select(owner_user.select(display_name))))
        .drive(|_, ((p, created), dn)| v.push((p, created, dn)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(p, created, dn)| {
        row(vec![
            V::S(dn),
            title(db, *p),
            V::T(*created),
            V::I(score.get(*p).unwrap()),
            oint(view_count.get(*p)),
        ])
    }))
}

fn q16503(db: &'static So) -> String {
    let PostHistory { post_history_type_id, post, user, creation_date, comment, .. } =
        &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<PostHistory>, Id<Post>, i64, Str)> = Vec::new();
    db.post_history
        .with(post_history_type_id.in_v(vec![4, 5]))
        .select(post.and(creation_date).and(user.select(display_name)))
        .drive(|h, ((p, created), dn)| v.push((h, p, created, dn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(h, p, created, dn)| {
        row(vec![V::S(dn), title(db, *p), ostr(comment.get(*h)), V::T(*created)])
    }))
}

fn q19443(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let Comment { post, text, creation_date: cdate, .. } = &db.comment;
    let User { display_name, reputation, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str, Str, i64)> = Vec::new();
    db.comment
        .with(post.select(owner_user.select(reputation)).gt(1000))
        .select(
            post.and(post.select(creation_date.and(owner_user.select(display_name))))
                .and(text)
                .and(cdate),
        )
        .drive(|_, (((p, (pcreated, dn)), txt), cc)| v.push((p, pcreated, dn, txt, cc)));
    v.sort_by(|a, b| b.4.cmp(&a.4));
    rows(v.iter().map(|(p, pcreated, dn, txt, cc)| {
        row(vec![
            V::S(dn),
            title(db, *p),
            V::T(*pcreated),
            V::S(txt),
            V::T(*cc),
        ])
    }))
}

fn q15103(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let Comment { post, text, creation_date: cdate, user, .. } = &db.comment;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str, Str, i64)> = Vec::new();
    db.comment
        .with(post.select(post_type_id).eq(1))
        .select(
            post.and(post.select(creation_date))
                .and(user.select(display_name))
                .and(text)
                .and(cdate),
        )
        .drive(|_, ((((p, pcreated), dn), txt), cc)| v.push((p, pcreated, dn, txt, cc)));
    v.sort_by(|a, b| b.4.cmp(&a.4));
    rows(v.iter().take(10).map(|(p, pcreated, dn, txt, cc)| {
        row(vec![
            V::S(dn),
            title(db, *p),
            V::T(*pcreated),
            V::S(txt),
            V::T(*cc),
        ])
    }))
}

fn q10322(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;
    let User { reputation, .. } = &db.user;

    let mut sc: Vec<(Str, i64, i64, i64, i64)> = Vec::new();
    db.post
        .with(owner_user.select(reputation).ge(1000))
        .group_by(post_type.select(name))
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (x, v)| {
            (n + 1, s + x, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        })
        .drive(|k, (n, s, vn, vs)| sc.push((k, n, s, vn, vs)));

    sc.sort_by(|a, b| b.1.cmp(&a.1));
    rows(sc.iter().map(|(k, n, s, vn, vs)| {
        row(vec![V::S(k), V::I(*n), avg(*s, *n), nullable(*vs, *vn)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("15108", q15108),
    ("19599", q19599),
    ("11607", q11607),
    ("11799", q11799),
    ("15400", q15400),
    ("16421", q16421),
    ("16503", q16503),
    ("17192", q17192),
    ("17800", q17800),
    ("19443", q19443),
    ("15103", q15103),
    ("10322", q10322),
];
