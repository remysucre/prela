use harness::prelude::*;

// Seventy-one more of the comment-count-and-vote-sums query. All but three
// name p.Id in the GROUP BY, so the group is one post:
//
//   SELECT <columns>, COUNT(c.Id),
//          [COALESCE(]SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END)[, 0)],
//          [COALESCE(]SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END)[, 0)]
//   FROM Posts p [LEFT] JOIN Users u ... LEFT JOIN Comments c ... LEFT JOIN Votes v ...
//   [WHERE p.PostTypeId = 1] GROUP BY <them> ORDER BY <one or two> DESC [LIMIT n]
//
// The COALESCE never fires: SUM over a CASE that yields 0 is 0, not NULL, as
// long as the group has a row — and it always does. COUNT(DISTINCT c.Id) and
// COUNT(DISTINCT v.Id) undo their own fan-out, so they are `#c` and `#v`
// rather than `#cv` and `#vc`.

// --- Comments and Votes, the group is one post -----------------------------

fn q15665(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15668(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16603(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16813(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19813(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16859(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16898(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17976(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q18450(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19188(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16205(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up"]) }
fn q16036(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q16332(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17380(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q14732(db: &'static So) -> String { post_rows_outer(db, false, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#v", "#up", "#down"]) }
fn q17951(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "rep", "#cx", "#up", "#down"]) }
fn q18065(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "views", "owner", "rep", "#cx", "#up", "#down"]) }
fn q14023(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "score", "owner", "#cx", "#up", "#down"]) }
fn q17353(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "score", "views", "#cx", "#up", "#down"]) }
fn q18037(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "score", "views", "#cx", "#up", "#down"]) }
fn q17810(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "views", "#cx", "#up", "#down"]) }
fn q16230(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "views", "#cx", "#up", "#down"]) }
fn q19024(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "views", "#cx", "#up", "#down"]) }
fn q18172(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "score", "views", "#cx", "#up", "#down"]) }
fn q10333(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down"]) }
fn q12175(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down"]) }
fn q13249(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down"]) }
fn q12809(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down"]) }
fn q12598(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down"]) }
fn q14769(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "score", "views", "#cx", "#up", "#down"]) }
fn q12734(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "views", "score", "#cx", "#vx", "#up", "#down"]) }
fn q14808(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#vx", "#up", "#down", "score", "views"]) }
fn q10327(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down", "score", "views", "answers"]) }
fn q12008(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down", "views", "score", "answers"]) }
fn q13636(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "owner", "created", "#cx", "#vx", "#up", "#down", "views", "score"]) }
fn q12498(db: &'static So) -> String { post_rows_outer(db, false, "cv", "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "#up", "#down"]) }
fn q10147(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx", "#up", "#down"]) }
fn q11397(db: &'static So) -> String { post_rows_outer(db, false, "cv", "created", 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx", "#up", "#down"]) }
fn q12671(db: &'static So) -> String { post_rows_outer(db, true, "cv", "score,views", 100, &["id", "title", "created", "score", "views", "owner", "#vx", "#cx", "#up", "#down"]) }
fn q12534(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "score", "views", "answers", "owner", "activity", "#cx", "#up", "#down"]) }
fn q12673(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "score", "views", "answers", "owner", "rep", "#cx", "#up", "#down"]) }
fn q13141(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down", "answers"]) }
fn q11696(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "score", "views", "#cx", "#up", "#down", "owner", "rep"]) }
fn q11108(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "views", "score", "owner", "#c", "#v", "#up", "#down"]) }
fn q14232(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "views", "score", "#cx", "#v"]) }
fn q14334(db: &'static So) -> String { post_rows(db, true, "cv", "score", 100, &["id", "title", "body", "created", "score", "views", "owner", "rep", "#v", "#c"]) }
fn q19673(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "type", "#cx", "#up", "#down"]) }
fn q17912(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "type", "#cx", "#up", "#down"]) }
fn q18868(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "type", "#cx", "#up", "#down"]) }
fn q13212(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "created", "owner", "type", "#cx", "#up", "#down"]) }
fn q11692(db: &'static So) -> String { post_rows(db, false, "cv", "rep,created", 100, &["owner_id", "owner", "rep", "id", "title", "created", "views", "#up", "#down", "#cx"]) }
fn q11988(db: &'static So) -> String { post_rows(db, true, "cv", "score,created", 100, &["id", "title", "created", "score", "views", "owner", "rep", "#cx", "#up", "#down", "activity"]) }
fn q12797(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "views", "owner", "#cx", "#up", "#down", "post_score_avg"]) }
fn q18729(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "body", "owner", "created", "activity", "#cx", "#up", "#down"]) }
fn q12695(db: &'static So) -> String { post_rows_outer(db, false, "cv", "#vx,created", 0, &["id", "title", "created", "owner", "#vx", "#up", "#down", "#cx", "post_rep_avg"]) }
fn q19495(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["title", "owner", "created", "score", "views", "#cx", "#up", "#down"]) }
fn q17971(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["title", "owner", "created", "views", "score", "#cx", "#up", "#down"]) }

// --- Votes alone -----------------------------------------------------------

fn q10309(db: &'static So) -> String { post_rows(db, true, "v", "created", 0, &["id", "title", "created", "score", "views", "owner_id", "owner", "rep", "#v", "#up", "#down"]) }
fn q14805(db: &'static So) -> String { post_rows(db, false, "v", "created", 100, &["id", "title", "type_id", "created", "score", "views", "answers", "owner_id", "owner", "rep", "#v", "#up", "#down"]) }
fn q14929(db: &'static So) -> String { post_rows_outer(db, false, "v", "created", 100, &["id", "title", "created", "score", "views", "owner", "rep", "#v", "#up", "#down"]) }
fn q19676(db: &'static So) -> String { post_rows_outer(db, true, "v", "created", 10, &["id", "title", "owner", "created", "score", "views", "#up", "#down"]) }

// --- the group is a value tuple --------------------------------------------

fn q17584(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 0, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15080(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15654(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16308(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17184(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19247(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16675(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "score", "views", "#cx", "#up", "#down"]) }
fn q10855(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "cv", PostWhere::All), "created", 100, &["title", "created", "views", "score", "owner", "#cx", "#up", "#down"]) }
fn q14387(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "cv", PostWhere::All), "created", 0, &["owner", "title", "created", "views", "score", "#cx", "#v", "#up", "#down"]) }
fn q13752(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "cv", PostWhere::All), "score,views", 100, &["owner", "title", "created", "views", "score", "#cx", "#up", "#down"]) }

pub static ENTRIES: &[harness::Entry] = &[
    ("10147", q10147), ("10309", q10309), ("10327", q10327), ("10333", q10333),
    ("10855", q10855), ("11108", q11108), ("11397", q11397), ("11692", q11692),
    ("11696", q11696), ("11988", q11988), ("12008", q12008), ("12175", q12175),
    ("12498", q12498), ("12534", q12534), ("12598", q12598), ("12671", q12671),
    ("12673", q12673), ("12695", q12695), ("12734", q12734), ("12797", q12797),
    ("12809", q12809), ("13141", q13141), ("13212", q13212), ("13249", q13249),
    ("13636", q13636), ("13752", q13752), ("14023", q14023), ("14232", q14232),
    ("14334", q14334), ("14387", q14387), ("14732", q14732), ("14769", q14769),
    ("14805", q14805), ("14808", q14808), ("14929", q14929), ("15080", q15080),
    ("15654", q15654), ("15665", q15665), ("15668", q15668), ("16036", q16036),
    ("16205", q16205), ("16230", q16230), ("16308", q16308), ("16332", q16332),
    ("16603", q16603), ("16675", q16675), ("16813", q16813), ("16859", q16859),
    ("16898", q16898), ("17184", q17184), ("17353", q17353), ("17380", q17380),
    ("17584", q17584), ("17810", q17810), ("17912", q17912), ("17951", q17951),
    ("17971", q17971), ("17976", q17976), ("18037", q18037), ("18065", q18065),
    ("18172", q18172), ("18450", q18450), ("18729", q18729), ("18868", q18868),
    ("19024", q19024), ("19188", q19188), ("19247", q19247), ("19495", q19495),
    ("19673", q19673), ("19676", q19676), ("19813", q19813),
];
