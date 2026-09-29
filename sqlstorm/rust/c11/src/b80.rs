use harness::prelude::*;

// Sixteen more of the comment-count query, all but two of them grouped by
// (u.DisplayName, p.Title, p.CreationDate):
//
//   SELECT p.Title, p.CreationDate, u.DisplayName, COUNT(c.Id)
//   FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//   LEFT JOIN Comments c ON p.Id = c.PostId
//   [WHERE p.PostTypeId = 1]
//   GROUP BY p.Title, p.CreationDate, u.DisplayName
//   ORDER BY p.CreationDate DESC [LIMIT 10]

fn q15111(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q15137(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q15281(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q15701(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q15972(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q16256(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q17377(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q15854(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q17701(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 0, &["title", "created", "owner", "#c"]) }

// the same over every post type
fn q16842(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q16879(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q16945(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q19847(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }

// p.Id in the GROUP BY, so the group is one post
fn q16361(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q18353(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }

// (DisplayName, Title, ViewCount), the one key with no CreationDate in it
fn q18668(db: &'static So) -> String { tuple_rows(by_name_title_views(db, true, "c", PostWhere::All), "views", 10, &["title", "views", "owner", "#c"]) }

pub static ENTRIES: &[harness::Entry] = &[
    ("15111", q15111), ("15137", q15137), ("15281", q15281), ("15701", q15701),
    ("15854", q15854), ("15972", q15972), ("16256", q16256), ("16361", q16361),
    ("16842", q16842), ("16879", q16879), ("16945", q16945), ("17377", q17377),
    ("17701", q17701), ("18353", q18353), ("18668", q18668), ("19847", q19847),
];
