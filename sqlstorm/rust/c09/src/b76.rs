use harness::prelude::*;

// Twenty-seven spellings of one query:
//
//   SELECT <some four of p.Id, p.Title, p.CreationDate, p.Score, p.Body,
//           u.DisplayName>
//   FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//   WHERE p.PostTypeId = 1
//   ORDER BY p.CreationDate DESC
//   LIMIT 10
//
// They differ only in the order of the select list, in column aliases, and in
// whether the JOIN is spelled `JOIN` or `INNER JOIN`. Rows 10 and 11 differ in
// CreationDate, so the cut is clean.
fn newest(db: &'static So, cols: &[&str]) -> String {
    rows(by_created(db).iter().map(|q| row(post_fields(db, q.pid, cols))))
}

// p.Id, p.Title, u.DisplayName, p.CreationDate
fn q15069(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created"])
}

// p.Id, p.Title, p.CreationDate, u.DisplayName
fn q15175(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner"])
}

fn q15347(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner"])
}

fn q15649(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner"])
}

fn q15816(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner"])
}

fn q16642(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created"])
}

fn q17174(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created"])
}

fn q18730(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created"])
}

// u.DisplayName, p.Title, p.CreationDate, p.Body
fn q16395(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "body"])
}

// u.DisplayName, p.Title, p.CreationDate, p.Score
fn q15011(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "score"])
}

// u.DisplayName, p.Title, p.Score, p.CreationDate
fn q15504(db: &'static So) -> String {
    newest(db, &["owner", "title", "score", "created"])
}

// p.Title, p.CreationDate, u.DisplayName, p.Score
fn q15535(db: &'static So) -> String {
    newest(db, &["title", "created", "owner", "score"])
}

fn q15696(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "score"])
}

// p.Title, p.CreationDate, p.Score, u.DisplayName
fn q15704(db: &'static So) -> String {
    newest(db, &["title", "created", "score", "owner"])
}

fn q15724(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "score"])
}

// p.Title, u.DisplayName, p.Score, p.CreationDate
fn q16070(db: &'static So) -> String {
    newest(db, &["title", "owner", "score", "created"])
}

fn q16131(db: &'static So) -> String {
    newest(db, &["title", "created", "owner", "score"])
}

fn q16214(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "score"])
}

// p.Title, u.DisplayName, p.CreationDate, p.Score
fn q17203(db: &'static So) -> String {
    newest(db, &["title", "owner", "created", "score"])
}

fn q17223(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "score"])
}

// p.Title, p.Score, u.DisplayName, p.CreationDate
fn q17815(db: &'static So) -> String {
    newest(db, &["title", "score", "owner", "created"])
}

fn q18218(db: &'static So) -> String {
    newest(db, &["title", "owner", "created", "score"])
}

fn q18235(db: &'static So) -> String {
    newest(db, &["title", "created", "score", "owner"])
}

fn q18737(db: &'static So) -> String {
    newest(db, &["title", "score", "owner", "created"])
}

fn q19371(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "score"])
}

fn q19836(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "score"])
}

fn q15024(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "score"])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("15011", q15011),
    ("15024", q15024),
    ("15069", q15069),
    ("15175", q15175),
    ("15347", q15347),
    ("15504", q15504),
    ("15535", q15535),
    ("15649", q15649),
    ("15696", q15696),
    ("15704", q15704),
    ("15724", q15724),
    ("15816", q15816),
    ("16070", q16070),
    ("16131", q16131),
    ("16214", q16214),
    ("16395", q16395),
    ("16642", q16642),
    ("17174", q17174),
    ("17203", q17203),
    ("17223", q17223),
    ("17815", q17815),
    ("18218", q18218),
    ("18235", q18235),
    ("18730", q18730),
    ("18737", q18737),
    ("19371", q19371),
    ("19836", q19836),
];
