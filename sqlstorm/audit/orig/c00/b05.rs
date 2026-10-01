use harness::prelude::*;

struct QRow {
    pid: Id<Post>,
    score: i64,
    created: i64,
    display_name: Str,
}

fn top_by_score(db: &'static So) -> Vec<QRow> {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(score.and(creation_date).and(owner_user.select(display_name)))
        .drive(|pid, ((score, created), display_name)| {
            v.push(QRow { pid, score, created, display_name })
        });
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    v
}

fn title_name_created_score(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

fn name_title_created_score(db: &'static So) -> String {
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

fn title_name_score_created(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            V::I(r.score),
            V::T(r.created),
        ])
    }))
}

fn name_title_score_created(db: &'static So) -> String {
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

fn q19563(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::I(r.score),
            V::S(r.display_name),
            V::T(r.created),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("15538", title_name_created_score),
    ("18885", title_name_created_score),
    ("15638", name_title_created_score),
    ("16607", name_title_created_score),
    ("18918", name_title_created_score),
    ("15246", name_title_created_score),
    ("16572", title_name_score_created),
    ("18477", name_title_score_created),
    ("19196", name_title_score_created),
    ("19563", q19563),
];
