use harness::prelude::*;

// The comment-count query again, this time also over the Votes join and the
// answers self-join:
//
//   SELECT <columns>, COUNT(c.Id) | COUNT(v.Id) | COUNT(p.Id) | COUNT(a.Id)
//   FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//        [LEFT JOIN Comments c ON p.Id = c.PostId]
//        [LEFT JOIN Votes v ON p.Id = v.PostId]
//        [LEFT JOIN Posts a ON p.Id = a.ParentId]
//   [WHERE p.PostTypeId = 1] GROUP BY <them> ORDER BY <one or two> DESC [LIMIT n]
//
// Which children are joined decides the arithmetic. With one child the count
// is the plain one (`#c`, `#v`); with two the fan-outs cross, so a comment is
// counted once per answer (`#ca`) and COUNT(p.Id) counts the post once per
// pair (`#rows_ca`). See notes/translation-failures.md 3.

// --- the group is one post -------------------------------------------------

fn q15272(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "body", "owner", "created", "score", "views", "#c"]) }
fn q19157(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q19481(db: &'static So) -> String { post_rows(db, false, "v", "created", 10, &["id", "title", "created", "owner", "score", "views", "#v"]) }
fn q19953(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "#c"]) }
fn q19656(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q15957(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "views", "owner", "#c"]) }
fn q16342(db: &'static So) -> String { post_rows(db, true, "v", "#v,created", 10, &["id", "title", "created", "views", "owner", "#v"]) }
fn q19261(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q16546(db: &'static So) -> String { post_rows(db, true, "c", "score,created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q19382(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "owner", "created", "views", "score", "#c"]) }
fn q16231(db: &'static So) -> String { post_rows(db, true, "c", "score,created", 10, &["id", "title", "owner", "created", "views", "score", "#c"]) }
fn q10919(db: &'static So) -> String { post_rows(db, false, "v", "created", 1000, &["id", "title", "score", "views", "created", "owner", "#v"]) }
fn q13406(db: &'static So) -> String { post_rows(db, true, "c", "views,score", 100, &["id", "title", "views", "score", "answers", "owner", "#c"]) }

// ... with Votes joined as well
fn q15552(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q15194(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q15235(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q15433(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q16353(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q16646(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q17753(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q19085(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q15040(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q16411(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q19499(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "score", "owner", "#cx", "#vx"]) }
fn q16183(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "owner", "created", "score", "#cx", "#vx"]) }
fn q17842(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "#cx", "#vx", "owner", "created"]) }
fn q15274(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#vx"]) }
fn q15998(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#vx"]) }
fn q16377(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#vx"]) }
fn q18214(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#vx"]) }

// --- the group is a value tuple --------------------------------------------

fn q18141(db: &'static So) -> String { tuple_rows(by_name_rep_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "rep", "title", "created", "score", "#c"]) }
fn q16832(db: &'static So) -> String { tuple_rows(by_name_rep_title_date(db, true, "c", PostWhere::All), "rep,created", 10, &["owner", "rep", "title", "created", "#c"]) }
fn q16545(db: &'static So) -> String { tuple_rows(by_name_rep_title_date(db, true, "v", PostWhere::All), "created", 10, &["title", "created", "owner", "rep", "#v"]) }

fn q16095(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"]) }
fn q16168(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"]) }
fn q16208(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"]) }
fn q16107(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q18592(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q17006(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q16804(db: &'static So) -> String { tuple_rows(by_name_title_date_score_outer(db, true, "c", PostWhere::All), "created", 100, &["owner", "title", "created", "score", "#c"]) }
fn q18118(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, false, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"]) }
fn q15291(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "a", PostWhere::All), "created", 10, &["title", "created", "score", "owner", "#a"]) }
fn q19156(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "a", PostWhere::All), "created", 10, &["title", "owner", "created", "score", "views", "#a"]) }
fn q16059(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, false, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "#c"]) }

fn q15478(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q16624(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q17539(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q19472(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q15674(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q15766(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q17937(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q17390(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q15625(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 0, &["owner", "title", "created", "#c"]) }
fn q19675(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }

// Comments and the answers self-join both present, so they cross
fn q15006(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "ca", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#ax"]) }
fn q16123(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "ca", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#ax"]) }
fn q19824(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "ca", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#ax"]) }
fn q18161(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "ca", PostWhere::All), "created", 10, &["title", "created", "owner", "#ax", "#cx"]) }

// the answers self-join alone: COUNT(p.Id) counts a post once per answer
fn q17026(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "a", PostWhere::All), "created", 0, &["title", "created", "owner", "#a"]) }
fn q17509(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "a", PostWhere::All), "created", 10, &["title", "created", "owner", "#a"]) }
fn q18341(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "a", PostWhere::All), "created", 10, &["title", "created", "owner", "#a"]) }
fn q19219(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "a", PostWhere::All), "created", 10, &["title", "created", "owner", "#a"]) }

fn q15594(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"]) }
fn q15956(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"]) }
fn q16445(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"]) }
fn q16089(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "v", PostWhere::All), "#v", 10, &["title", "created", "owner", "#v"]) }
fn q16658(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "v", PostWhere::All), "#v", 10, &["title", "created", "owner", "#v"]) }

pub static ENTRIES: &[harness::Entry] = &[
    ("10919", q10919), ("13406", q13406), ("15006", q15006), ("15040", q15040),
    ("15194", q15194), ("15235", q15235), ("15272", q15272), ("15274", q15274),
    ("15291", q15291), ("15433", q15433), ("15478", q15478), ("15552", q15552),
    ("15594", q15594), ("15625", q15625), ("15674", q15674), ("15766", q15766),
    ("15956", q15956), ("15957", q15957), ("15998", q15998), ("16059", q16059),
    ("16089", q16089), ("16095", q16095), ("16107", q16107), ("16123", q16123),
    ("16168", q16168), ("16183", q16183), ("16208", q16208), ("16231", q16231),
    ("16342", q16342), ("16353", q16353), ("16377", q16377), ("16411", q16411),
    ("16445", q16445), ("16545", q16545), ("16546", q16546), ("16624", q16624),
    ("16646", q16646), ("16658", q16658), ("16804", q16804), ("16832", q16832),
    ("17006", q17006), ("17026", q17026), ("17390", q17390), ("17509", q17509),
    ("17539", q17539), ("17753", q17753), ("17842", q17842), ("17937", q17937),
    ("18118", q18118), ("18141", q18141), ("18161", q18161), ("18214", q18214),
    ("18341", q18341), ("18592", q18592), ("19085", q19085), ("19156", q19156),
    ("19157", q19157), ("19219", q19219), ("19261", q19261), ("19382", q19382),
    ("19472", q19472), ("19481", q19481), ("19499", q19499), ("19656", q19656),
    ("19675", q19675), ("19824", q19824), ("19953", q19953),
];
