use harness::prelude::*;
use std::collections::BTreeMap;

struct QRow {
    pid: Id<Post>,
    uid: Id<User>,
    id: i64,
    score: i64,
    views: i64,
    created: i64,
    display_name: Str,
    reputation: i64,
}

fn questions(db: &'static So) -> Vec<QRow> {
    let Post { post_type_id, origid, score, view_count, creation_date, owner_user, .. } =
        &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(
            origid
                .and(score)
                .and(view_count)
                .and(creation_date)
                .and(owner_user)
                .and(owner_user.select(display_name.and(reputation))),
        )
        .drive(
            |pid,
             (((((id, score), views), created), uid), (display_name, reputation))| {
                v.push(QRow {
                    pid,
                    uid,
                    id,
                    score,
                    views,
                    created,
                    display_name,
                    reputation,
                })
            },
        );
    v
}

fn by_created(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

fn q16712(db: &'static So) -> String {
    let Post { title, .. } = &db.post;
    let User { origid: uid, .. } = &db.user;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(uid.get(r.uid).unwrap()),
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

fn q15021(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            V::I(r.views),
        ])
    }))
}

fn q18376(db: &'static So) -> String {
    let Post { title, .. } = &db.post;
    let User { origid: uid, .. } = &db.user;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(uid.get(r.uid).unwrap()),
            V::S(r.display_name),
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::T(r.created),
        ])
    }))
}

fn q19470(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::I(r.score),
            V::I(r.views),
            V::S(r.display_name),
        ])
    }))
}

fn q15516(db: &'static So) -> String {
    let Post { creation_date, owner_user, origid, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let g = db
        .post
        .with(creation_date.ge(date(2023, 1, 1)))
        .group_by(owner_user.select(display_name))
        .select(origid)
        .fold(0i64, |a, _| a + 1);

    let mut all: Vec<(Str, i64)> = Vec::new();
    g.drive(|k, n| all.push((k, n)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(dn, n)| row(vec![V::S(dn), V::I(*n)])))
}

fn q15517(db: &'static So) -> String {
    let title = &db.post.title;
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score).then(b.created.cmp(&a.created)));
    v.truncate(10);
    rows(v.iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
            V::I(r.views),
        ])
    }))
}

fn q15787(db: &'static So) -> String {
    let title = &db.post.title;
    let mut v = questions(db);
    v.sort_by(|a, b| b.views.cmp(&a.views));
    v.truncate(10);
    rows(v.iter().map(|r| {
        row(vec![ostr(title.get(r.pid)), V::T(r.created), V::S(r.display_name)])
    }))
}

fn q16999(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            V::I(r.reputation),
            ostr(title.get(r.pid)),
            V::T(r.created),
        ])
    }))
}

fn q18524(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type, title, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let PostType { name, .. } = &db.post_type;

    let mut v: Vec<(Id<Post>, i64, Str, Str)> = Vec::new();
    db.post
        .select(
            creation_date
                .and(owner_user.select(display_name))
                .and(post_type.select(name)),
        )
        .drive(|pid, ((created, dn), pt)| v.push((pid, created, dn, pt)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(pid, created, dn, pt)| {
        row(vec![ostr(title.get(*pid)), V::T(*created), V::S(dn), V::S(pt)])
    }))
}

fn q13108(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;

    let mut agg: BTreeMap<i64, (i64, i64, i64, i64)> = BTreeMap::new();

    db.post
        .group_by(post_type_id)
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (sc, vc)| {
            (n + 1, s + sc, vn + vc.is_some() as i64, vs + vc.unwrap_or(0))
        })
        .drive(|k, v| {
            agg.insert(k, v);
        });

    rows(agg.iter().map(|(k, (n, s, vn, vs))| {
        let avg = |c: i64, sum: i64| {
            if c == 0 { V::Null } else { V::F(sum as f64 / c as f64) }
        };
        row(vec![V::I(*k), V::I(*n), avg(*n, *s), avg(*vn, *vs)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("16712", q16712),
    ("15021", q15021),
    ("18376", q18376),
    ("19470", q19470),
    ("15516", q15516),
    ("15517", q15517),
    ("15787", q15787),
    ("16999", q16999),
    ("18524", q18524),
    ("13108", q13108),
];
