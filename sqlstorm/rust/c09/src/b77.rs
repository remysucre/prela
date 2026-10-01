use crate::q::{Question, by_created, by_score, by_views};
use harness::prelude::*;

// Seventy-three spellings of three queries. Every one is
//
//   SELECT <some four or five of p.Id, p.Title, p.CreationDate, p.Score,
//           p.ViewCount, p.Body, u.DisplayName>
//   FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//   WHERE p.PostTypeId = 1
//   ORDER BY <p.CreationDate | p.Score | p.ViewCount> DESC
//   LIMIT 10
//
// differing only in the select-list order, the column aliases, and whether the
// join is written `JOIN` or `INNER JOIN`. The comment over each one is its
// select list and its ORDER BY. All three cuts are clean: rows 10 and 11
// differ in the sort column, so no rewrite is needed.
fn top(db: &'static So, v: Vec<Question>, cols: &[&str]) -> String {
    rows(v.iter().map(|q| row(post_fields(db, q.pid, cols))))
}

fn newest(db: &'static So, cols: &[&str]) -> String {
    top(db, by_created(db), cols)
}

fn best(db: &'static So, cols: &[&str]) -> String {
    top(db, by_score(db), cols)
}

fn most_viewed(db: &'static So, cols: &[&str]) -> String {
    top(db, by_views(db), cols)
}

// title, created, owner, views | ORDER BY views DESC LIMIT 10
fn q19357(db: &'static So) -> String {
    most_viewed(db, &["title", "created", "owner", "views"])
}

// owner, title, created, views | ORDER BY created DESC LIMIT 10
fn q15074(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "views"])
}

// title, created, owner, views | ORDER BY created DESC LIMIT 10
fn q15102(db: &'static So) -> String {
    newest(db, &["title", "created", "owner", "views"])
}

// title, owner, created, views | ORDER BY created DESC LIMIT 10
fn q15928(db: &'static So) -> String {
    newest(db, &["title", "owner", "created", "views"])
}

// owner, title, created, views | ORDER BY created DESC LIMIT 10
fn q16488(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "views"])
}

// title, created, owner, views | ORDER BY created DESC LIMIT 10
fn q18531(db: &'static So) -> String {
    newest(db, &["title", "created", "owner", "views"])
}

// title, created, views, owner | ORDER BY created DESC LIMIT 10
fn q19591(db: &'static So) -> String {
    newest(db, &["title", "created", "views", "owner"])
}

// title, views, owner, created | ORDER BY created DESC LIMIT 10
fn q19615(db: &'static So) -> String {
    newest(db, &["title", "views", "owner", "created"])
}

// title, owner, views, created | ORDER BY created DESC LIMIT 10
fn q19715(db: &'static So) -> String {
    newest(db, &["title", "owner", "views", "created"])
}

// owner, title, created, views | ORDER BY created DESC LIMIT 10
fn q19942(db: &'static So) -> String {
    newest(db, &["owner", "title", "created", "views"])
}

// id, title, created, owner, score | ORDER BY created DESC LIMIT 10
fn q15035(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "score"])
}

// id, title, owner, created, score | ORDER BY created DESC LIMIT 10
fn q15046(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created", "score"])
}

// id, title, score, owner, created | ORDER BY created DESC LIMIT 10
fn q15141(db: &'static So) -> String {
    newest(db, &["id", "title", "score", "owner", "created"])
}

// id, title, created, score, owner | ORDER BY created DESC LIMIT 10
fn q15215(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "score", "owner"])
}

// id, title, created, score, owner | ORDER BY created DESC LIMIT 10
fn q15238(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "score", "owner"])
}

// id, title, owner, created, score | ORDER BY created DESC LIMIT 10
fn q15288(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created", "score"])
}

// id, title, owner, created, score | ORDER BY created DESC LIMIT 10
fn q15311(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created", "score"])
}

// id, title, owner, created, score | ORDER BY created DESC LIMIT 10
fn q15349(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created", "score"])
}

// id, title, owner, score, created | ORDER BY created DESC LIMIT 10
fn q15389(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "score", "created"])
}

// id, title, owner, created, score | ORDER BY created DESC LIMIT 10
fn q15418(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created", "score"])
}

// id, title, owner, created, score | ORDER BY created DESC LIMIT 10
fn q15556(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created", "score"])
}

// id, title, created, owner, score | ORDER BY created DESC LIMIT 10
fn q15617(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "score"])
}

// id, title, owner, score, created | ORDER BY created DESC LIMIT 10
fn q15675(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "score", "created"])
}

// id, title, score, owner, created | ORDER BY created DESC LIMIT 10
fn q15711(db: &'static So) -> String {
    newest(db, &["id", "title", "score", "owner", "created"])
}

// id, title, score, created, owner | ORDER BY created DESC LIMIT 10
fn q15965(db: &'static So) -> String {
    newest(db, &["id", "title", "score", "created", "owner"])
}

// id, title, score, created, owner | ORDER BY created DESC LIMIT 10
fn q16232(db: &'static So) -> String {
    newest(db, &["id", "title", "score", "created", "owner"])
}

// id, title, created, owner, score | ORDER BY created DESC LIMIT 10
fn q16461(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "score"])
}

// id, title, owner, created, score | ORDER BY created DESC LIMIT 10
fn q16808(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created", "score"])
}

// id, title, score, owner, created | ORDER BY created DESC LIMIT 10
fn q17577(db: &'static So) -> String {
    newest(db, &["id", "title", "score", "owner", "created"])
}

// id, title, created, score, owner | ORDER BY created DESC LIMIT 10
fn q17677(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "score", "owner"])
}

// id, title, score, owner, created | ORDER BY created DESC LIMIT 10
fn q17803(db: &'static So) -> String {
    newest(db, &["id", "title", "score", "owner", "created"])
}

// id, title, created, score, owner | ORDER BY created DESC LIMIT 10
fn q17887(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "score", "owner"])
}

// id, title, created, owner, score | ORDER BY created DESC LIMIT 10
fn q18048(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "score"])
}

// owner, title, created, views, score | ORDER BY score DESC LIMIT 10
fn q15285(db: &'static So) -> String {
    best(db, &["owner", "title", "created", "views", "score"])
}

// title, owner, created, score, views | ORDER BY score DESC LIMIT 10
fn q15527(db: &'static So) -> String {
    best(db, &["title", "owner", "created", "score", "views"])
}

// title, owner, created, score, views | ORDER BY score DESC LIMIT 10
fn q15646(db: &'static So) -> String {
    best(db, &["title", "owner", "created", "score", "views"])
}

// title, owner, created, score, views | ORDER BY score DESC LIMIT 10
fn q15872(db: &'static So) -> String {
    best(db, &["title", "owner", "created", "score", "views"])
}

// owner, title, created, score, views | ORDER BY score DESC LIMIT 10
fn q15878(db: &'static So) -> String {
    best(db, &["owner", "title", "created", "score", "views"])
}

// title, created, owner, views, score | ORDER BY score DESC LIMIT 10
fn q15879(db: &'static So) -> String {
    best(db, &["title", "created", "owner", "views", "score"])
}

// title, owner, score, views, created | ORDER BY score DESC LIMIT 10
fn q16158(db: &'static So) -> String {
    best(db, &["title", "owner", "score", "views", "created"])
}

// title, owner, created, score, views | ORDER BY score DESC LIMIT 10
fn q16275(db: &'static So) -> String {
    best(db, &["title", "owner", "created", "score", "views"])
}

// title, created, owner, score, views | ORDER BY score DESC LIMIT 10
fn q16817(db: &'static So) -> String {
    best(db, &["title", "created", "owner", "score", "views"])
}

// owner, title, created, views, score | ORDER BY score DESC LIMIT 10
fn q16895(db: &'static So) -> String {
    best(db, &["owner", "title", "created", "views", "score"])
}

// owner, title, created, views, score | ORDER BY score DESC LIMIT 10
fn q17816(db: &'static So) -> String {
    best(db, &["owner", "title", "created", "views", "score"])
}

// title, owner, created, score, views | ORDER BY score DESC LIMIT 10
fn q17835(db: &'static So) -> String {
    best(db, &["title", "owner", "created", "score", "views"])
}

// title, views, owner, score, created | ORDER BY score DESC LIMIT 10
fn q18754(db: &'static So) -> String {
    best(db, &["title", "views", "owner", "score", "created"])
}

// title, created, owner, views, score | ORDER BY score DESC LIMIT 10
fn q19712(db: &'static So) -> String {
    best(db, &["title", "created", "owner", "views", "score"])
}

// title, created, owner, score, views | ORDER BY score DESC LIMIT 10
fn q19845(db: &'static So) -> String {
    best(db, &["title", "created", "owner", "score", "views"])
}

// id, title, created, owner, views | ORDER BY views DESC LIMIT 10
fn q17249(db: &'static So) -> String {
    most_viewed(db, &["id", "title", "created", "owner", "views"])
}

// id, title, views, owner, created | ORDER BY views DESC LIMIT 10
fn q19109(db: &'static So) -> String {
    most_viewed(db, &["id", "title", "views", "owner", "created"])
}

// id, title, created, owner, views | ORDER BY created DESC LIMIT 10
fn q15340(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "views"])
}

// id, title, created, views, owner | ORDER BY created DESC LIMIT 10
fn q16213(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "views", "owner"])
}

// id, title, created, owner, views | ORDER BY created DESC LIMIT 10
fn q16347(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "views"])
}

// id, title, created, owner, views | ORDER BY created DESC LIMIT 10
fn q16557(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "views"])
}

// id, title, owner, created, views | ORDER BY created DESC LIMIT 10
fn q16648(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created", "views"])
}

// id, title, owner, created, views | ORDER BY created DESC LIMIT 10
fn q17356(db: &'static So) -> String {
    newest(db, &["id", "title", "owner", "created", "views"])
}

// id, title, created, views, owner | ORDER BY created DESC LIMIT 10
fn q17796(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "views", "owner"])
}

// id, title, views, owner, created | ORDER BY created DESC LIMIT 10
fn q18001(db: &'static So) -> String {
    newest(db, &["id", "title", "views", "owner", "created"])
}

// id, title, views, owner, created | ORDER BY created DESC LIMIT 10
fn q18319(db: &'static So) -> String {
    newest(db, &["id", "title", "views", "owner", "created"])
}

// id, title, views, created, owner | ORDER BY created DESC LIMIT 10
fn q18363(db: &'static So) -> String {
    newest(db, &["id", "title", "views", "created", "owner"])
}

// id, title, created, views, owner | ORDER BY created DESC LIMIT 10
fn q18494(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "views", "owner"])
}

// id, title, body, owner, created | ORDER BY created DESC LIMIT 10
fn q16745(db: &'static So) -> String {
    newest(db, &["id", "title", "body", "owner", "created"])
}

// id, title, created, owner, rep | ORDER BY created DESC LIMIT 10
fn q15193(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "rep"])
}

// id, title, created, owner, rep | ORDER BY created DESC LIMIT 10
fn q15627(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "rep"])
}

// id, title, created, owner, rep | ORDER BY created DESC LIMIT 10
fn q15775(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "rep"])
}

// id, title, created, owner, rep | ORDER BY created DESC LIMIT 10
fn q16316(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "rep"])
}

// id, title, created, owner, rep | ORDER BY created DESC LIMIT 10
fn q18825(db: &'static So) -> String {
    newest(db, &["id", "title", "created", "owner", "rep"])
}

// id, title, created, owner, views, score | ORDER BY score DESC LIMIT 10
fn q15881(db: &'static So) -> String {
    best(db, &["id", "title", "created", "owner", "views", "score"])
}

// id, title, owner, created, score, views | ORDER BY score DESC LIMIT 10
fn q15964(db: &'static So) -> String {
    best(db, &["id", "title", "owner", "created", "score", "views"])
}

// id, title, created, owner, score, views | ORDER BY score DESC LIMIT 10
fn q16321(db: &'static So) -> String {
    best(db, &["id", "title", "created", "owner", "score", "views"])
}

// id, title, owner, created, score, views | ORDER BY score DESC LIMIT 10
fn q16661(db: &'static So) -> String {
    best(db, &["id", "title", "owner", "created", "score", "views"])
}

// id, title, owner, created, views, score | ORDER BY score DESC LIMIT 10
fn q16866(db: &'static So) -> String {
    best(db, &["id", "title", "owner", "created", "views", "score"])
}

// id, title, score, views, owner, created | ORDER BY score DESC LIMIT 10
fn q16962(db: &'static So) -> String {
    best(db, &["id", "title", "score", "views", "owner", "created"])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("15035", q15035),
    ("15046", q15046),
    ("15074", q15074),
    ("15102", q15102),
    ("15141", q15141),
    ("15193", q15193),
    ("15215", q15215),
    ("15238", q15238),
    ("15285", q15285),
    ("15288", q15288),
    ("15311", q15311),
    ("15340", q15340),
    ("15349", q15349),
    ("15389", q15389),
    ("15418", q15418),
    ("15527", q15527),
    ("15556", q15556),
    ("15617", q15617),
    ("15627", q15627),
    ("15646", q15646),
    ("15675", q15675),
    ("15711", q15711),
    ("15775", q15775),
    ("15872", q15872),
    ("15878", q15878),
    ("15879", q15879),
    ("15881", q15881),
    ("15928", q15928),
    ("15964", q15964),
    ("15965", q15965),
    ("16158", q16158),
    ("16213", q16213),
    ("16232", q16232),
    ("16275", q16275),
    ("16316", q16316),
    ("16321", q16321),
    ("16347", q16347),
    ("16461", q16461),
    ("16488", q16488),
    ("16557", q16557),
    ("16648", q16648),
    ("16661", q16661),
    ("16745", q16745),
    ("16808", q16808),
    ("16817", q16817),
    ("16866", q16866),
    ("16895", q16895),
    ("16962", q16962),
    ("17249", q17249),
    ("17356", q17356),
    ("17577", q17577),
    ("17677", q17677),
    ("17796", q17796),
    ("17803", q17803),
    ("17816", q17816),
    ("17835", q17835),
    ("17887", q17887),
    ("18001", q18001),
    ("18048", q18048),
    ("18319", q18319),
    ("18363", q18363),
    ("18494", q18494),
    ("18531", q18531),
    ("18754", q18754),
    ("18825", q18825),
    ("19109", q19109),
    ("19357", q19357),
    ("19591", q19591),
    ("19615", q19615),
    ("19712", q19712),
    ("19715", q19715),
    ("19845", q19845),
    ("19942", q19942),
];
