use harness::prelude::*;

// Forty-one more of the same query, now over each combination of the three
// children:
//
//   SELECT <columns>, COUNT(c.Id) | COUNT(v.Id) | COUNT(a.Id)
//   FROM Posts p [LEFT] JOIN Users u ON p.OwnerUserId = u.Id
//        [JOIN PostTypes pt ...] [LEFT JOIN Comments c ON p.Id = c.PostId]
//        [LEFT JOIN Votes v ON p.Id = v.PostId]
//        [LEFT JOIN Posts a ON p.Id = a.ParentId]
//   [WHERE p.PostTypeId = 1] GROUP BY <them> ORDER BY <one or two> DESC [LIMIT n]
//
// With one child joined the count is the plain one (`#c`, `#v`, `#a`); with
// two the fan-outs cross, so a comment is counted once per vote (`#cv`) or per
// answer (`#ca`). `post_rows_outer` is where SQL writes `LEFT JOIN Users` and
// the ownerless posts keep a row with a NULL DisplayName.

// --- Comments alone --------------------------------------------------------

fn q18458(db: &'static So) -> String { post_rows(db, true, "c", "score,created", 10, &["id", "title", "owner", "#c", "created", "score"]) }
fn q17085(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "body", "owner", "created", "views", "score", "#c"]) }
fn q15454(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "views", "owner", "rep", "#c"]) }
fn q16597(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "rep", "created", "score", "views", "#c"]) }
fn q17062(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["title", "created", "owner", "score", "views", "#c"]) }
fn q17379(db: &'static So) -> String { post_rows(db, true, "c", "score,created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q18219(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views_ptype(db, false, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "views", "score", "ptype", "#c"]) }
fn q19452(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"]) }

// --- Votes alone -----------------------------------------------------------

fn q10877(db: &'static So) -> String { post_rows(db, false, "v", "#v,created", 0, &["id", "title", "created", "views", "score", "#v", "rep", "owner"]) }
fn q10664(db: &'static So) -> String { post_rows(db, false, "v", "#v,score", 100, &["id", "title", "created", "score", "views", "#v", "rep", "owner"]) }

// --- the answers self-join -------------------------------------------------

fn q16600(db: &'static So) -> String { post_rows(db, true, "a", "created", 10, &["id", "title", "created", "owner", "type", "#a"]) }

// --- Comments and the answers self-join, which cross -----------------------

fn q18973(db: &'static So) -> String { post_rows(db, true, "ca", "created", 10, &["id", "title", "created", "owner", "#cx", "#ax"]) }
fn q16109(db: &'static So) -> String { post_rows(db, true, "ca", "created", 10, &["id", "title", "owner", "created", "#cx", "#ax"]) }
fn q19992(db: &'static So) -> String { post_rows_outer(db, true, "ca", "created", 10, &["id", "title", "created", "owner", "#cx", "#ax"]) }

// --- Comments and Votes, which cross ---------------------------------------

fn q15971(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 50, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q15062(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "score", "#cx", "#vx"]) }
fn q16902(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "rep", "#cx", "#vx"]) }
fn q16010(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "views", "score", "#cx", "#vx"]) }
fn q16834(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "type", "#cx", "#vx"]) }
fn q18372(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "created", "score", "owner", "#cx", "#vx"]) }
fn q19633(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "score", "owner", "#cx", "#vx"]) }
fn q15047(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "created", "score", "owner", "#cx", "#vx"]) }
fn q10845(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx"]) }
fn q13630(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx"]) }
fn q14464(db: &'static So) -> String { post_rows_outer(db, false, "cv", "created", 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx"]) }
fn q16747(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "created", "views", "owner", "#cx", "#vx"]) }
fn q15354(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "score", "#cx", "#vx"]) }
fn q16134(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "score", "#cx", "#vx"]) }
fn q18108(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "score", "#cx", "#vx"]) }
fn q15563(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "owner", "created", "score", "views", "#cx", "#vx"]) }
fn q15614(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "score", "views", "#cx", "#vx"]) }
fn q15633(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "score", "views", "#cx", "#vx"]) }
fn q16846(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "score", "views", "#cx", "#vx"]) }
fn q15192(db: &'static So) -> String { post_rows(db, true, "cv", "score", 10, &["id", "title", "owner", "created", "score", "views", "#cx", "#vx"]) }
fn q16500(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 0, &["id", "title", "owner", "created", "score", "views", "#cx", "#vx"]) }
fn q19609(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "owner", "created", "views", "score", "#cx", "#vx"]) }
fn q19635(db: &'static So) -> String { tuple_rows(by_owner_name_title_date_score_views(db, true, "cv", PostWhere::All), "created", 10, &["owner_id", "owner", "title", "created", "score", "views", "#cx", "#vx"]) }
fn q19812(db: &'static So) -> String { post_rows(db, true, "cv", "score", 10, &["id", "title", "score", "views", "owner", "#cx", "#vx"]) }
fn q18640(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#vx"]) }
fn q19779(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "#vx,#cx", 0, &["owner", "title", "created", "#cx", "#vx"]) }
fn q19248(db: &'static So) -> String { tuple_rows(by_name_title_date_views_body(db, true, "cv", PostWhere::All), "created", 10, &["title", "body", "owner", "created", "views", "#cx", "#vx"]) }

pub static ENTRIES: &[harness::Entry] = &[
    ("10664", q10664), ("10845", q10845), ("10877", q10877), ("13630", q13630),
    ("14464", q14464), ("15047", q15047), ("15062", q15062), ("15192", q15192),
    ("15354", q15354), ("15454", q15454), ("15563", q15563), ("15614", q15614),
    ("15633", q15633), ("15971", q15971), ("16010", q16010), ("16109", q16109),
    ("16134", q16134), ("16500", q16500), ("16597", q16597), ("16600", q16600),
    ("16747", q16747), ("16834", q16834), ("16846", q16846), ("16902", q16902),
    ("17062", q17062), ("17085", q17085), ("17379", q17379), ("18108", q18108),
    ("18219", q18219), ("18372", q18372), ("18458", q18458), ("18640", q18640),
    ("18973", q18973), ("19248", q19248), ("19452", q19452), ("19609", q19609),
    ("19633", q19633), ("19635", q19635), ("19779", q19779), ("19812", q19812),
    ("19992", q19992),
];
