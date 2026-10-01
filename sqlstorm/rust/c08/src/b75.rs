use harness::prelude::*;

// Thirteen spellings of two queries. Every one of them is
//
//   SELECT <some four or five of p.Id, p.Title, p.CreationDate, p.Score,
//           p.ViewCount, u.DisplayName>
//   FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//   WHERE p.PostTypeId = 1
//   ORDER BY <p.Score | p.ViewCount> DESC
//   LIMIT 10
//
// and differs from its neighbours only in the order of the select list and in
// column aliases, which the oracle does not print. Both cuts are clean (rows
// 10 and 11 differ in the sort column).
fn top_questions<T: Ord>(db: &'static So, key: impl Fn(i64, Option<i64>) -> T, cols: &[&str]) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(score.and(view_count.opt())));
    rows(top_n(v, |&(_, (s, w))| key(s, w), 10).iter().map(|&(p, _)| row(post_fields(db, p, cols))))
}

fn best(db: &'static So, cols: &[&str]) -> String {
    top_questions(db, |s, _| std::cmp::Reverse(s), cols)
}

fn most_viewed(db: &'static So, cols: &[&str]) -> String {
    top_questions(db, |_, w| (w.is_none(), std::cmp::Reverse(w)), cols)
}

// ORDER BY p.Score DESC: u.DisplayName, p.Title, p.CreationDate, p.Score
fn q19115(db: &'static So) -> String {
    best(db, &["owner", "title", "created", "score"])
}

// p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName
fn q15145(db: &'static So) -> String {
    best(db, &["id", "title", "created", "score", "owner"])
}

// p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score
fn q15533(db: &'static So) -> String {
    best(db, &["id", "title", "created", "owner", "score"])
}

// p.Id, p.Title, p.Score, u.DisplayName, p.CreationDate
fn q15581(db: &'static So) -> String {
    best(db, &["id", "title", "score", "owner", "created"])
}

// p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score
fn q16663(db: &'static So) -> String {
    best(db, &["id", "title", "created", "owner", "score"])
}

// p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score
fn q16881(db: &'static So) -> String {
    best(db, &["id", "title", "owner", "created", "score"])
}

// p.Id, p.Title, p.Score, u.DisplayName, p.CreationDate
fn q17058(db: &'static So) -> String {
    best(db, &["id", "title", "score", "owner", "created"])
}

// ORDER BY p.ViewCount DESC: p.Title, p.ViewCount, u.DisplayName, p.CreationDate
fn q15241(db: &'static So) -> String {
    most_viewed(db, &["title", "views", "owner", "created"])
}

// u.DisplayName, p.Title, p.CreationDate, p.ViewCount
fn q15367(db: &'static So) -> String {
    most_viewed(db, &["owner", "title", "created", "views"])
}

// p.Title, u.DisplayName, p.CreationDate, p.ViewCount
fn q15540(db: &'static So) -> String {
    most_viewed(db, &["title", "owner", "created", "views"])
}

// u.DisplayName, p.Title, p.ViewCount, p.CreationDate
fn q18106(db: &'static So) -> String {
    most_viewed(db, &["owner", "title", "views", "created"])
}

// p.Title, u.DisplayName, p.ViewCount, p.CreationDate
fn q18884(db: &'static So) -> String {
    most_viewed(db, &["title", "owner", "views", "created"])
}

// u.DisplayName, p.Title, p.CreationDate, p.ViewCount
fn q19336(db: &'static So) -> String {
    most_viewed(db, &["owner", "title", "created", "views"])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("15145", q15145),
    ("15241", q15241),
    ("15367", q15367),
    ("15533", q15533),
    ("15540", q15540),
    ("15581", q15581),
    ("16663", q16663),
    ("16881", q16881),
    ("17058", q17058),
    ("18106", q18106),
    ("18884", q18884),
    ("19115", q19115),
    ("19336", q19336),
];
