use harness::prelude::*;

// More of the comment-count query. Twenty-nine of these name p.Id in the
// GROUP BY, so the group is one post and `post_rows` walks the posts; the
// other four group by a value tuple. Three join Votes as well, and there the
// two LEFT JOINs cross: COUNT(c.Id) is `#cv` and COUNT(v.Id) is `#vc`.
//
//   SELECT <columns>, COUNT(c.Id) [, COUNT(v.Id)]
//   FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//        [JOIN PostTypes pt ...] LEFT JOIN Comments c ON p.Id = c.PostId
//        [LEFT JOIN Votes v ON p.Id = v.PostId]
//   [WHERE p.PostTypeId = 1] GROUP BY <them> ORDER BY <one> DESC [LIMIT n]

// --- the group is one post -------------------------------------------------

fn q17624(db: &'static So) -> String { post_rows(db, false, "c", "created", 100, &["id", "title", "created", "owner", "type", "#c"]) }
fn q15900(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "owner", "created", "type", "#c"]) }
fn q19721(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c", "score", "views"]) }
fn q16541(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q17131(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q18509(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q19440(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q15453(db: &'static So) -> String { post_rows(db, true, "c", "created", 50, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q18088(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q16795(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c", "views", "score"]) }
fn q15036(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "views", "score", "#c"]) }
fn q17119(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "views", "score", "#c"]) }
fn q16466(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "created", "owner", "views", "score", "#c"]) }
fn q19202(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "score", "owner", "views", "#c"]) }
fn q16294(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "score", "views", "owner", "#c"]) }
fn q16381(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "views", "score", "owner", "#c"]) }
fn q15174(db: &'static So) -> String { post_rows(db, true, "c", "created", 0, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q15082(db: &'static So) -> String { post_rows(db, true, "c", "created", 100, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q17132(db: &'static So) -> String { post_rows(db, true, "c", "created", 100, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q17731(db: &'static So) -> String { post_rows(db, false, "c", "created", 100, &["id", "title", "owner", "created", "score", "#c"]) }
fn q19049(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "score", "created", "views", "#c"]) }
fn q19487(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "score", "owner", "created", "activity", "#c"]) }
fn q15491(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "score", "owner", "created", "views", "#c"]) }
fn q17722(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "score", "views", "owner", "created", "#c"]) }

// ... with Votes joined too
fn q16980(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q15078(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q15131(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q18670(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#vx"]) }
fn q19124(db: &'static So) -> String { post_rows(db, true, "cv", "#vx,#cx", 0, &["id", "title", "owner", "#cx", "#vx"]) }

// --- the group is a value tuple --------------------------------------------

fn q16108(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#cx", "#vx"]) }
fn q18194(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "views", 10, &["owner", "title", "created", "views", "#c"]) }
fn q15605(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#vx"]) }
fn q16292(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "views", "#c"]) }

pub static ENTRIES: &[harness::Entry] = &[
    ("15036", q15036), ("15078", q15078), ("15082", q15082), ("15131", q15131),
    ("15174", q15174), ("15453", q15453), ("15491", q15491), ("15605", q15605),
    ("15900", q15900), ("16108", q16108), ("16292", q16292), ("16294", q16294),
    ("16381", q16381), ("16466", q16466), ("16541", q16541), ("16795", q16795),
    ("16980", q16980), ("17119", q17119), ("17131", q17131), ("17132", q17132),
    ("17624", q17624), ("17722", q17722), ("17731", q17731), ("18088", q18088),
    ("18194", q18194), ("18509", q18509), ("18670", q18670), ("19049", q19049),
    ("19124", q19124), ("19202", q19202), ("19440", q19440), ("19487", q19487),
    ("19721", q19721),
];
