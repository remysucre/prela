use harness::prelude::*;

// A hundred spellings of one query:
//
//   SELECT <columns>, COUNT(c.Id),
//          SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END),
//          SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END)
//   FROM Posts p [LEFT] JOIN Users u ON p.OwnerUserId = u.Id
//        LEFT JOIN Comments c ON p.Id = c.PostId
//        LEFT JOIN Votes v ON p.Id = v.PostId
//   [WHERE p.PostTypeId = 1] GROUP BY <them> ORDER BY <one> DESC [LIMIT n]
//
// Comments and Votes both joined, so the fan-outs cross: COUNT(c.Id) counts a
// comment once per vote (`#cv`) and each vote-type sum counts a vote once per
// comment (`#up`, `#down`). Four join Votes alone, and there nothing
// multiplies the sums — `#v`, `#up0`, `#down0`. Two join Votes and the answers
// self-join instead.

// --- the group is one post, questions only ---------------------------------

fn q15261(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16958(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17936(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q18630(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q18433(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16145(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16240(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16516(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15271(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15425(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16531(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17860(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19300(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15057(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q16626(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q18682(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q15020(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q16352(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q18209(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q18590(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q15979(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q16862(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q17181(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19894(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "body", "owner", "created", "#cx", "#up", "#down"]) }
fn q16625(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "score", "#cx", "#up", "#down"]) }
fn q15730(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "score", "owner", "#cx", "#up", "#down"]) }
fn q17608(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "created", "views", "owner", "#cx", "#up", "#down"]) }
fn q15725(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "score", "#cx", "#up", "#down"]) }
fn q18205(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "owner", "created", "score", "#cx", "#up", "#down"]) }
fn q18493(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "score", "owner", "created", "#cx", "#up", "#down"]) }
fn q17185(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19087(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19557(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q13942(db: &'static So) -> String { post_rows(db, true, "cv", "#vx,score", 100, &["id", "title", "created", "score", "owner", "rep", "#c", "#vx"]) }

// --- the group is one post, every post type --------------------------------

fn q15025(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15318(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15398(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15409(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15773(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16456(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16696(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17469(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q18735(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19652(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19678(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16450(db: &'static So) -> String { post_rows(db, false, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q18575(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q18338(db: &'static So) -> String { post_rows(db, false, "cv", "created", 50, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15030(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "views", "owner", "#cx", "#up", "#down"]) }
fn q15991(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "views", "owner", "#cx", "#up", "#down"]) }
fn q11575(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#up", "#down"]) }
fn q19767(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "owner", "created", "score", "views", "#cx", "#up", "#down"]) }
fn q15090(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q15148(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }
fn q17422(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "owner", "created", "#cx", "#up", "#down"]) }

// --- LEFT JOIN Users, so the ownerless posts keep a NULL-name row ----------

fn q15196(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15447(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15689(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15756(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16011(db: &'static So) -> String { post_rows_outer(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15083(db: &'static So) -> String { post_rows_outer(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17574(db: &'static So) -> String { post_rows_outer(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q14708(db: &'static So) -> String { post_rows_outer(db, false, "cv", "created", 100, &["id", "title", "created", "owner", "#vx", "#up", "#down", "#cx"]) }
fn q19876(db: &'static So) -> String { tuple_rows(by_name_title_date_outer(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }

// --- Votes alone, so nothing multiplies the sums ---------------------------

fn q14252(db: &'static So) -> String { post_rows(db, false, "v", "created", 1000, &["id", "title", "created", "score", "views", "owner_id", "owner", "rep", "#v", "#up", "#down"]) }
fn q12562(db: &'static So) -> String { post_rows(db, false, "v", "created", 100, &["id", "title", "created", "score", "views", "owner_id", "owner", "#v", "#up", "#down"]) }
fn q18451(db: &'static So) -> String { post_rows(db, true, "v", "created", 10, &["id", "title", "owner", "created", "score", "#up", "#down"]) }
fn q11360(db: &'static So) -> String { post_rows(db, true, "v", "score", 100, &["id", "title", "owner", "score", "created", "views", "#v", "#up", "#down"]) }

// --- Votes and the answers self-join ---------------------------------------

fn q17711(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "va", PostWhere::All), "created", 10, &["title", "created", "owner", "#ax", "#up", "#down"]) }
fn q18676(db: &'static So) -> String { post_rows_outer(db, true, "va", "created", 10, &["title", "created", "owner", "#ax", "#up", "#down"]) }

// --- the group is a value tuple --------------------------------------------

fn q15116(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 0, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q15252(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q15812(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q16569(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q16698(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q17225(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q17560(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q17561(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q18171(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q19584(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q19899(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q17017(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q18438(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q18854(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q15236(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q17369(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q18597(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q17524(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 100, &["owner", "title", "created", "#cx", "#up", "#down"]) }
fn q15038(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 0, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15362(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 0, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17029(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 0, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19147(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 0, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q14503(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 100, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15273(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17310(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "owner", "created", "#cx", "#up", "#down"]) }
fn q19038(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "owner", "created", "#cx", "#up", "#down"]) }
fn q19053(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "owner", "created", "#cx", "#up", "#down"]) }
fn q18780(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#cx", "#up", "#down"]) }
fn q15294(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "cv", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "views", "#cx", "#up", "#down"]) }
fn q15497(db: &'static So) -> String { tuple_rows(by_name_title_date_body(db, true, "cv", PostWhere::All), "created", 10, &["title", "body", "owner", "#up", "#down", "#cx", "created"]) }

pub static ENTRIES: &[harness::Entry] = &[
    ("11360", q11360), ("11575", q11575), ("12562", q12562), ("13942", q13942),
    ("14252", q14252), ("14503", q14503), ("14708", q14708), ("15020", q15020),
    ("15025", q15025), ("15030", q15030), ("15038", q15038), ("15057", q15057),
    ("15083", q15083), ("15090", q15090), ("15116", q15116), ("15148", q15148),
    ("15196", q15196), ("15236", q15236), ("15252", q15252), ("15261", q15261),
    ("15271", q15271), ("15273", q15273), ("15294", q15294), ("15318", q15318),
    ("15362", q15362), ("15398", q15398), ("15409", q15409), ("15425", q15425),
    ("15447", q15447), ("15497", q15497), ("15689", q15689), ("15725", q15725),
    ("15730", q15730), ("15756", q15756), ("15773", q15773), ("15812", q15812),
    ("15979", q15979), ("15991", q15991), ("16011", q16011), ("16145", q16145),
    ("16240", q16240), ("16352", q16352), ("16450", q16450), ("16456", q16456),
    ("16516", q16516), ("16531", q16531), ("16569", q16569), ("16625", q16625),
    ("16626", q16626), ("16696", q16696), ("16698", q16698), ("16862", q16862),
    ("16958", q16958), ("17017", q17017), ("17029", q17029), ("17181", q17181),
    ("17185", q17185), ("17225", q17225), ("17310", q17310), ("17369", q17369),
    ("17422", q17422), ("17469", q17469), ("17524", q17524), ("17560", q17560),
    ("17561", q17561), ("17574", q17574), ("17608", q17608), ("17711", q17711),
    ("17860", q17860), ("17936", q17936), ("18171", q18171), ("18205", q18205),
    ("18209", q18209), ("18338", q18338), ("18433", q18433), ("18438", q18438),
    ("18451", q18451), ("18493", q18493), ("18575", q18575), ("18590", q18590),
    ("18597", q18597), ("18630", q18630), ("18676", q18676), ("18682", q18682),
    ("18735", q18735), ("18780", q18780), ("18854", q18854), ("19038", q19038),
    ("19053", q19053), ("19087", q19087), ("19147", q19147), ("19300", q19300),
    ("19557", q19557), ("19584", q19584), ("19652", q19652), ("19678", q19678),
    ("19767", q19767), ("19876", q19876), ("19894", q19894), ("19899", q19899),
];
