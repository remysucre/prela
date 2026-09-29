// Batch 02 — worklist positions 21..30.
//
// Mostly the batch-01 template again with permuted projections; the new
// shapes are `p.ViewCount` (nullable, so a probe), GROUP BY the user rather
// than the display name, and AVG over a group.

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

// 19546 — u.DisplayName, p.Title, p.Score, p.ViewCount ORDER BY Score DESC
fn q19546(db: &'static So) -> String {
    let Post { title, view_count, .. } = &db.post;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::I(r.score),
            oint(view_count.get(r.pid)),
        ])
    }))
}

// 16090 — u.DisplayName, p.Title, p.CreationDate ORDER BY CreationDate DESC
fn q16090(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_created(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
        ])
    }))
}

// 15061 — u.DisplayName, p.Title, p.CreationDate, p.Score
fn q15061(db: &'static So) -> String {
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

// 15210 — p.Title, p.CreationDate, u.DisplayName, p.Score
fn q15210(db: &'static So) -> String {
    let title = &db.post.title;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.score),
        ])
    }))
}

// 15290 — p.Title, u.DisplayName, p.CreationDate, p.Score
fn q15290(db: &'static So) -> String {
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

// 18331 — p.Title, p.Score, u.DisplayName, p.CreationDate
fn q18331(db: &'static So) -> String {
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

// 18664 — u.DisplayName, p.Title, p.Score, p.CreationDate
fn q18664(db: &'static So) -> String {
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

// 15932 — p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName
fn q15932(db: &'static So) -> String {
    let Post { title, view_count, .. } = &db.post;
    rows(top_by_score(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::I(r.score),
            oint(view_count.get(r.pid)),
            V::S(r.display_name),
        ])
    }))
}

// 19778 — u.DisplayName, COUNT(p.Id) GROUP BY u.Id, u.DisplayName
//         ORDER BY PostCount DESC LIMIT 10. Grouping by the user id rather
//         than the name, so two users with the same name stay apart.
fn q19778(db: &'static So) -> String {
    let Post { owner_user, origid, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let g = db
        .post
        .with(owner_user)
        .group_by(owner_user)
        .select(origid)
        .fold(0i64, |a, _| a + 1);

    let mut all: Vec<(Id<User>, i64)> = Vec::new();
    g.drive(|k, n| all.push((k, n)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(u, n)| {
        row(vec![ostr(display_name.get(*u)), V::I(*n)])
    }))
}

// 12863 — PostTypeId, COUNT(P.Id), AVG(P.Score) GROUP BY PostTypeId
fn q12863(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;

    let g = db
        .post
        .group_by(post_type_id)
        .select(score)
        .fold((0i64, 0i64), |(s, c), sc| (s + sc, c + 1));

    let mut out = Vec::new();
    g.drive(|k, (s, c)| {
        out.push(row(vec![V::I(k), V::I(c), V::F(s as f64 / c as f64)]))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
        ("19546", q19546),
        ("16090", q16090),
        ("15061", q15061),
        ("15210", q15210),
        ("15290", q15290),
        ("18331", q18331),
        ("18664", q18664),
        ("19778", q19778),
        ("12863", q12863),
        ("15932", q15932),
];
