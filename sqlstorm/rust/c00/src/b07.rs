use harness::prelude::*;

struct QRow {
    pid: Id<Post>,
    id: i64,
    score: i64,
    views: Option<i64>,
    created: i64,
    body: Str,
    display_name: Str,
    reputation: i64,
}

fn questions(db: &'static So) -> Vec<QRow> {
    let Post {
        post_type_id, origid, score, view_count, creation_date, body, owner_user, ..
    } = &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(
            origid
                .and(score)
                .and(view_count.opt())
                .and(creation_date)
                .and(body)
                .and(owner_user.select(display_name.and(reputation))),
        )
        .drive(
            |pid,
             (((((id, score), views), created), body), (display_name, reputation))| {
                v.push(QRow {
                    pid,
                    id,
                    score,
                    views,
                    created,
                    body,
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

fn by_score(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    v
}

fn q15106(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
            oint(r.views),
        ])
    }))
}

fn q15406(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::S(r.body),
            V::T(r.created),
            V::S(r.display_name),
        ])
    }))
}

fn q16776(db: &'static So) -> String {
    let Post { creation_date, origid, score, view_count, owner_user, title, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, Str)> = Vec::new();
    db.post
        .with(creation_date.ge(date(2023, 1, 1)))
        .select(origid.and(score).and(owner_user.select(display_name)))
        .drive(|pid, ((id, sc), dn)| v.push((pid, id, sc, dn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(pid, id, sc, dn)| {
        row(vec![
            V::I(*id),
            ostr(title.get(*pid)),
            V::I(*sc),
            oint(view_count.get(*pid)),
            V::S(dn),
        ])
    }))
}

fn q10117(db: &'static So) -> String {
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

fn q15101(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.reputation),
        ])
    }))
}

fn by_rep_count(db: &'static So, min_rep: i64) -> String {
    let Post { owner_user, origid, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let g = db
        .post
        .with(owner_user.select(reputation).gt(min_rep))
        .group_by(owner_user.select(display_name))
        .select(origid)
        .fold(0i64, |a, _| a + 1);

    let mut all: Vec<(Str, i64)> = Vec::new();
    g.drive(|k, n| all.push((k, n)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(dn, n)| row(vec![V::S(dn), V::I(*n)])))
}

fn q15791(db: &'static So) -> String {
    by_rep_count(db, 100)
}

fn q15920(db: &'static So) -> String {
    by_rep_count(db, 1000)
}

fn q15130(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_score(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.score),
            oint(r.views),
        ])
    }))
}

fn q15015(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
            oint(r.views),
        ])
    }))
}

fn q17071(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let g = db
        .post
        .with(owner_user)
        .group_by(owner_user.select(display_name))
        .select(score)
        .fold((0i64, 0i64), |(n, s), sc| (n + 1, s + sc));

    let mut all: Vec<(Str, i64, i64)> = Vec::new();
    g.drive(|k, (n, s)| all.push((k, n, s)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(dn, n, s)| {
        row(vec![V::S(dn), V::I(*n), V::I(*s)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("15106", q15106),
    ("15406", q15406),
    ("16776", q16776),
    ("10117", q10117),
    ("15101", q15101),
    ("15791", q15791),
    ("15130", q15130),
    ("15015", q15015),
    ("15920", q15920),
    ("17071", q17071),
];
