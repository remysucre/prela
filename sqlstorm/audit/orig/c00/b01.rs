// Batch 01 — worklist positions 11..20.
//
// One template with ten spellings: Posts JOIN Users on the owner, filtered,
// ordered, LIMIT 10. The join is `owner_user.select(..)`, which drops the
// posts with no owner exactly as INNER JOIN does. `Title` is nullable, so
// projecting it is a probe (`get`) rather than a `.select` — a `.select`
// would drop the row instead of emitting NULL.

use harness::prelude::*;

/// The shared shape: every question with an owner, carrying what the ten
/// projections choose from.
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

fn by_score_desc(mut v: Vec<QRow>) -> Vec<QRow> {
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    v
}

fn by_created_desc(mut v: Vec<QRow>) -> Vec<QRow> {
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

// 18517 — p.Id, p.Title, p.Score, u.DisplayName ORDER BY p.Score DESC
fn q18517(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_score_desc(questions(db)).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::I(r.score),
            V::S(r.display_name),
        ])
    }))
}

// 15070 — u.DisplayName, p.Title, p.CreationDate ORDER BY p.Score DESC
fn q15070(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_score_desc(questions(db)).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
        ])
    }))
}

// 18086 — p.Id, p.Title, u.DisplayName, p.CreationDate ORDER BY Score DESC
fn q18086(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_score_desc(questions(db)).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
        ])
    }))
}

// 18772 — p.Id, p.Title, p.CreationDate, u.DisplayName ORDER BY Score DESC
fn q18772(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_score_desc(questions(db)).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
        ])
    }))
}

// 15065, 17367 — u.DisplayName, p.Title, p.CreationDate ORDER BY
//                p.CreationDate DESC (17367 permutes the columns)
fn q15065(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created_desc(questions(db)).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
        ])
    }))
}

fn q17367(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created_desc(questions(db)).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
        ])
    }))
}

// 19346 — p.Title, u.DisplayName, p.CreationDate ORDER BY CreationDate DESC
fn q19346(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created_desc(questions(db)).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
        ])
    }))
}

// 15657, 15374 — u.DisplayName, COUNT(p.Id) GROUP BY u.DisplayName
//                ORDER BY the count DESC LIMIT 10
fn q15657(db: &'static So) -> String {
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

// 18504 — U.DisplayName, P.Title, P.Score WHERE P.CreationDate >=
//         '2023-01-01' ORDER BY P.Score DESC LIMIT 10 (no post-type filter)
fn q18504(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, title, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut all: Vec<(Id<Post>, i64, Str)> = Vec::new();
    db.post
        .with(creation_date.ge(date(2023, 1, 1)))
        .select(score.and(owner_user.select(display_name)))
        .drive(|pid, (sc, dn)| all.push((pid, sc, dn)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(pid, sc, dn)| {
        row(vec![V::S(dn), ostr(title.get(*pid)), V::I(*sc)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
        ("18517", q18517),
        ("15070", q15070),
        ("15657", q15657),
        ("15374", q15657),
        ("18086", q18086),
        ("15065", q15065),
        ("17367", q17367),
        ("18504", q18504),
        ("18772", q18772),
        ("19346", q19346),
];
