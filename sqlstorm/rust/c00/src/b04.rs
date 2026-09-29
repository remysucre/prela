use harness::prelude::*;

struct QRow {
    pid: Id<Post>,
    score: i64,
    created: i64,
    display_name: Str,
}

fn questions(db: &'static So) -> Vec<QRow> {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut out = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(score.and(creation_date).and(owner_user.select(display_name)))
        .drive(|pid, ((score, created), display_name)| {
            out.push(QRow { pid, score, created, display_name })
        });
    out
}

fn top_by_created(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

fn top_by_score(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    v
}

fn q17868(db: &'static So) -> String {
    let Post { owner_user, origid, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let g = db
        .post
        .with(owner_user)
        .group_by(owner_user.select(display_name))
        .select(origid)
        .fold(0i64, |a, _| a + 1);

    let mut all: Vec<(Str, i64)> = Vec::new();
    g.drive(|k, n| all.push((k, n)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(dn, n)| row(vec![V::S(dn), V::I(*n)])))
}

fn name_title_created(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_created(db).iter().map(|r| {
        row(vec![V::S(r.display_name), ostr(title.get(r.pid)), V::T(r.created)])
    }))
}

fn q15231(db: &'static So) -> String {
    name_title_created(db)
}

fn q15937(db: &'static So) -> String {
    name_title_created(db)
}

fn q16434(db: &'static So) -> String {
    name_title_created(db)
}

fn q16416(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_created(db).iter().map(|r| {
        row(vec![ostr(title.get(r.pid)), V::S(r.display_name), V::T(r.created)])
    }))
}

fn title_created_name(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_created(db).iter().map(|r| {
        row(vec![ostr(title.get(r.pid)), V::T(r.created), V::S(r.display_name)])
    }))
}

fn q17472(db: &'static So) -> String {
    title_created_name(db)
}

fn q17774(db: &'static So) -> String {
    title_created_name(db)
}

fn q18362(db: &'static So) -> String {
    title_created_name(db)
}

fn q15066(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::I(r.score),
            V::T(r.created),
        ])
    }))
}

fn q15239(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("17868", q17868),
    ("15231", q15231),
    ("15937", q15937),
    ("16416", q16416),
    ("17472", q17472),
    ("17774", q17774),
    ("18362", q18362),
    ("16434", q16434),
    ("15066", q15066),
    ("15239", q15239),
];
