// Batch 03 — worklist positions 31..40.
//
// Ten more projections of the same template as batches 01/02. This is where
// the easy tier is: a handful of query SHAPES, each spelled many ways. See
// notes/blocked.md on clustering the corpus by shape rather than by text.

use harness::prelude::*;

struct QRow {
    pid: Id<Post>,
    id: i64,
    score: i64,
    created: i64,
    display_name: Str,
}

fn questions(db: &'static So) -> Vec<QRow> {
    let Post { post_type_id, origid, score, creation_date, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut out = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(
            origid
                .and(score)
                .and(creation_date)
                .and(owner_user.select(display_name)),
        )
        .drive(|pid, (((id, score), created), display_name)| {
            out.push(QRow { pid, id, score, created, display_name })
        });
    out
}

fn top_by_score(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    v
}

fn top_by_created(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

// 15007 — p.Id, p.Title, p.CreationDate, u.DisplayName / by CreationDate
fn q15007(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
        ])
    }))
}

// 15096 — p.Id, p.Title, u.DisplayName, p.CreationDate / by CreationDate
fn q15096(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
        ])
    }))
}

// 19760, 18468 — u.DisplayName, p.Title, p.CreationDate / by CreationDate
fn q19760(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_created(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
        ])
    }))
}

// 15042 — p.Title, p.Body, u.DisplayName, p.CreationDate / by CreationDate
fn q15042(db: &'static So) -> String {
    let Post { title, body, .. } = &db.post;
    rows(top_by_created(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            ostr(body.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
        ])
    }))
}

// 15003 — p.Title, u.DisplayName, p.CreationDate, p.Score / by CreationDate
fn q15003(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_created(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

// 15316 — u.DisplayName, p.Title, p.CreationDate, p.Score / by CreationDate
fn q15316(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_created(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

// 15341 — p.Id, p.Title, p.Score, u.DisplayName, p.CreationDate / by Score
fn q15341(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::I(r.score),
            V::S(r.display_name),
            V::T(r.created),
        ])
    }))
}

// 15437 — p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score / by Score
fn q15437(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.score),
        ])
    }))
}

// 16701 — p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score / by Score
fn q16701(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
        ("15007", q15007),
        ("15096", q15096),
        ("19760", q19760),
        ("18468", q19760),
        ("15042", q15042),
        ("15341", q15341),
        ("15437", q15437),
        ("16701", q16701),
        ("15003", q15003),
        ("15316", q15316),
];
