use harness::prelude::*;

// A hundred spellings of one question: "how many comments does each post
// have?" Every query here is
//
//   SELECT <some columns>, COUNT(c.Id) [, COUNT(v.Id)]
//   FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//        LEFT JOIN Comments c ON p.Id = c.PostId [LEFT JOIN Votes v ON p.Id = v.PostId]
//   [WHERE p.PostTypeId = 1]
//   GROUP BY <those columns>
//   ORDER BY <one of them> DESC [LIMIT n]
//
// and they differ in which columns the tuple holds, which of them get printed,
// and what the sort is. Two shapes hide in that:
//
//   * the GROUP BY names p.Id, so every group is exactly one post and no
//     grouping is needed — `post_counts` walks the posts and reads the comment
//     count off `comments_per_post`;
//   * the GROUP BY is a value tuple, and then two posts agreeing on all of
//     title, author and instant really do merge, so it really is a `group_by`.
//
// Four distinct tuples appear, all of them (DisplayName, Title, CreationDate)
// plus nothing, Score, ViewCount, or both. GROUP BY is order-insensitive, so
// the many orderings of those columns are the same four keys.
//
// Where Votes is joined as well the two LEFT JOINs cross and a comment is
// counted once per vote: that is `Agg::cv` rather than `Agg::c`. See
// notes/translation-failures.md 3.

// --- GROUP BY names p.Id, so the group is one post -------------------------

// p.Id, p.Title, p.CreationDate, u.DisplayName, COUNT(c.Id) | created DESC, 10
fn q19213(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q15053(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q15134(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q15421(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q15884(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q16378(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q16576(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q16739(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q16843(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }
fn q19073(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c"]) }

// the same, without a LIMIT and with LIMIT 100
fn q17288(db: &'static So) -> String { post_rows(db, true, "c", "created", 0, &["id", "title", "created", "owner", "#c"]) }
fn q18873(db: &'static So) -> String { post_rows(db, true, "c", "created", 100, &["id", "title", "created", "owner", "#c"]) }

// ... with p.Score printed too, in each of the places it turns up
fn q15739(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "#c", "score"]) }
fn q16105(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "#c"]) }
fn q16156(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "#c"]) }
fn q16272(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "#c"]) }
fn q18204(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "#c"]) }
fn q15056(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "score", "owner", "#c"]) }
fn q15297(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "score", "owner", "#c"]) }
fn q15755(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "score", "owner", "#c"]) }
fn q18834(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "score", "owner", "#c"]) }
fn q15305(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "#c"]) }
fn q15507(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "#c"]) }
fn q15660(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "#c"]) }
fn q16554(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "#c"]) }
fn q16693(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "#c"]) }
fn q16746(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "#c"]) }
fn q17613(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "#c"]) }
fn q15355(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "score", "owner", "created", "#c"]) }
fn q16004(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "score", "owner", "created", "#c"]) }
fn q15679(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "#c"]) }
fn q16288(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "#c"]) }
fn q16348(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "#c"]) }
fn q17602(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "#c"]) }

// p.Id is in the GROUP BY but not printed
fn q15221(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "created", "owner", "score", "#c"]) }
fn q17658(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "owner", "created", "score", "#c"]) }
fn q17679(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "owner", "created", "score", "#c"]) }
fn q19044(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "owner", "created", "score", "#c"]) }

// sorted by p.Score and by p.ViewCount instead
fn q15530(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "owner", "created", "score", "#c"]) }
fn q18191(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "score", "owner", "#c"]) }
fn q19438(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "score", "owner", "#c"]) }
fn q18620(db: &'static So) -> String { post_rows(db, true, "c", "views", 10, &["id", "title", "views", "owner", "#c"]) }

// no WHERE, so answers as well as questions
fn q18231(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "created", "score", "owner", "#c"]) }
fn q18914(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "owner", "created", "#c"]) }
fn q15094(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q15456(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }

// --- GROUP BY (DisplayName, Title, CreationDate) ---------------------------

fn q15457(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q17502(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q15413(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q18287(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q19383(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q16258(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"]) }
fn q19631(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q18845(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c"]) }
fn q16112(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "#c"]) }
fn q17635(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "#c", "created"]) }

// the same key over every post type, with Votes joined as well: the two
// LEFT JOINs cross, so COUNT(c.Id) counts a comment once per vote
fn q17305(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"]) }
fn q19594(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"]) }

// --- ... plus p.Score ------------------------------------------------------

fn q15227(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"]) }
fn q15348(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"]) }
fn q15772(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"]) }
fn q17593(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"]) }
fn q15037(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"]) }
fn q15682(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q16097(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q18510(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q18697(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q15431(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q19797(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score,created", 10, &["owner", "title", "created", "score", "#c"]) }
fn q15209(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 0, &["owner", "title", "score", "created", "#c"]) }
fn q15908(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c", "score"]) }
fn q16679(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["title", "created", "owner", "#c", "score"]) }
fn q15697(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "score", "#c"]) }
fn q15815(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "score", "#c"]) }
fn q18561(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "score", "#c"]) }
fn q16222(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "score", "owner", "#c"]) }
fn q16365(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "score", "#c"]) }
fn q17047(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "score", "#c"]) }
fn q18304(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["title", "owner", "created", "score", "#c"]) }
fn q19097(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["title", "owner", "created", "score", "#c"]) }
fn q19907(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["title", "score", "owner", "created", "#c"]) }

// --- ... plus p.ViewCount --------------------------------------------------

fn q16847(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "#c"]) }
fn q18715(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "#c"]) }
fn q19456(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "#c"]) }
fn q18756(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "views", 10, &["owner", "title", "created", "views", "#c"]) }
fn q18886(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "views", 10, &["owner", "title", "created", "views", "#c"]) }
fn q15819(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "views", "owner", "#c"]) }
fn q18333(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "views", "owner", "#c"]) }
fn q17480(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "views", 10, &["title", "views", "created", "owner", "#c"]) }

// --- ... plus both ---------------------------------------------------------

fn q18541(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "views", "#c"]) }
fn q15537(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "views", "#c"]) }
fn q15488(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "score", "#c"]) }
fn q15557(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "score", "#c"]) }
fn q16337(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "score", "#c"]) }
fn q18718(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "score", "#c"]) }
fn q19445(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "score", "#c"]) }
fn q19533(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "score", "#c"]) }
fn q17528(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "views", "score", "#c"]) }
fn q15662(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "score", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q16685(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "score", 10, &["title", "owner", "created", "score", "views", "#c"]) }

pub static ENTRIES: &[harness::Entry] = &[
    ("15037", q15037), ("15053", q15053), ("15056", q15056), ("15094", q15094),
    ("15134", q15134), ("15209", q15209), ("15221", q15221), ("15227", q15227),
    ("15297", q15297), ("15305", q15305), ("15348", q15348), ("15355", q15355),
    ("15413", q15413), ("15421", q15421), ("15431", q15431), ("15456", q15456),
    ("15457", q15457), ("15488", q15488), ("15507", q15507), ("15530", q15530),
    ("15537", q15537), ("15557", q15557), ("15660", q15660), ("15662", q15662),
    ("15679", q15679), ("15682", q15682), ("15697", q15697), ("15739", q15739),
    ("15755", q15755), ("15772", q15772), ("15815", q15815), ("15819", q15819),
    ("15884", q15884), ("15908", q15908), ("16004", q16004), ("16097", q16097),
    ("16105", q16105), ("16112", q16112), ("16156", q16156), ("16222", q16222),
    ("16258", q16258), ("16272", q16272), ("16288", q16288), ("16337", q16337),
    ("16348", q16348), ("16365", q16365), ("16378", q16378), ("16554", q16554),
    ("16576", q16576), ("16679", q16679), ("16685", q16685), ("16693", q16693),
    ("16739", q16739), ("16746", q16746), ("16843", q16843), ("16847", q16847),
    ("17047", q17047), ("17288", q17288), ("17305", q17305), ("17480", q17480),
    ("17502", q17502), ("17528", q17528), ("17593", q17593), ("17602", q17602),
    ("17613", q17613), ("17635", q17635), ("17658", q17658), ("17679", q17679),
    ("18191", q18191), ("18204", q18204), ("18231", q18231), ("18287", q18287),
    ("18304", q18304), ("18333", q18333), ("18510", q18510), ("18541", q18541),
    ("18561", q18561), ("18620", q18620), ("18697", q18697), ("18715", q18715),
    ("18718", q18718), ("18756", q18756), ("18834", q18834), ("18845", q18845),
    ("18873", q18873), ("18886", q18886), ("18914", q18914), ("19044", q19044),
    ("19073", q19073), ("19097", q19097), ("19213", q19213), ("19383", q19383),
    ("19438", q19438), ("19445", q19445), ("19456", q19456), ("19533", q19533),
    ("19594", q19594), ("19631", q19631), ("19797", q19797), ("19907", q19907),
];
