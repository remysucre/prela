use harness::prelude::*;

struct QRow {
    pid: Id<Post>,
    id: i64,
    score: i64,
    views: Option<i64>,
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
                .and(view_count.opt())
                .and(creation_date)
                .and(owner_user.select(display_name.and(reputation))),
        )
        .drive(|pid, ((((id, score), views), created), (display_name, reputation))| {
            v.push(QRow { pid, id, score, views, created, display_name, reputation })
        });
    v
}

fn by_views(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by_key(|r| (r.views.is_none(), std::cmp::Reverse(r.views)));
    v.truncate(10);
    v
}

fn by_created(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

fn by_score(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    v
}

fn name_title_created_views(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_views(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            oint(r.views),
        ])
    }))
}

fn q17105(db: &'static So) -> String {
    let Post { view_count, score, creation_date, owner_user, title, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, Str)> = Vec::new();
    db.post
        .with(view_count.gt(1000))
        .select(score.and(creation_date).and(owner_user.select(display_name)))
        .drive(|pid, ((sc, created), dn)| v.push((pid, sc, created, dn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(pid, sc, created, dn)| {
        row(vec![ostr(title.get(*pid)), V::S(dn), V::T(*created), V::I(*sc)])
    }))
}

fn q15050(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            oint(r.views),
            V::T(r.created),
        ])
    }))
}

fn q19475(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_views(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            oint(r.views),
            V::S(r.display_name),
            V::I(r.reputation),
        ])
    }))
}

fn q15026(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::I(r.score),
            V::S(r.display_name),
            V::T(r.created),
        ])
    }))
}

fn q15018(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_score(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
            oint(r.views),
        ])
    }))
}

fn q15439(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_views(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            oint(r.views),
            V::T(r.created),
        ])
    }))
}

fn q10767(db: &'static So) -> String {
    let Post { post_type, origid, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let g = db
        .post
        .group_by(post_type.select(name))
        .select(origid)
        .fold(0i64, |a, _| a + 1);

    let mut all: Vec<(Str, i64)> = Vec::new();
    g.drive(|k, n| all.push((k, n)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().map(|(nm, n)| row(vec![V::S(nm), V::I(*n)])))
}

fn q16591(db: &'static So) -> String {
    let Post { owner_user, origid, .. } = &db.post;
    let User { display_name, origid: uid, .. } = &db.user;

    let g = db
        .post
        .with(owner_user)
        .group_by(owner_user)
        .select(origid)
        .fold(0i64, |a, _| a + 1);

    let mut all: Vec<(i64, Str, i64)> = Vec::new();
    g.drive(|u, n| {
        all.push((uid.get(u).unwrap(), display_name.get(u).unwrap(), n))
    });
    all.sort_by(|a, b| b.2.cmp(&a.2));
    rows(all.iter().take(10).map(|(id, dn, n)| {
        row(vec![V::I(*id), V::S(dn), V::I(*n)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("15144", name_title_created_views),
    ("15254", name_title_created_views),
    ("17105", q17105),
    ("15050", q15050),
    ("19475", q19475),
    ("15026", q15026),
    ("15018", q15018),
    ("15439", q15439),
    ("10767", q10767),
    ("16591", q16591),
];
