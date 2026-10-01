use harness::prelude::*;

// The same question as b81 — "how many comments does each post have?" — in a
// hundred more spellings:
//
//   SELECT <some columns>, COUNT(c.Id) [, COUNT(v.Id)]
//   FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//        [JOIN PostTypes pt ON p.PostTypeId = pt.Id]
//        LEFT JOIN Comments c ON p.Id = c.PostId
//        [LEFT JOIN Votes v ON p.Id = v.PostId]
//   [WHERE p.PostTypeId = 1]
//   GROUP BY <those columns> ORDER BY <one of them> DESC [LIMIT n]
//
// `post_rows` is the case where the GROUP BY names p.Id and the group is one
// post; the `by_*` views are the four-to-six column value tuples, where two
// posts agreeing on every column merge. Thirteen of these join Votes as well,
// and then the two LEFT JOINs cross: COUNT(c.Id) counts a comment once per
// vote (`#cv`) and COUNT(v.Id) a vote once per comment (`#vc`).

// --- GROUP BY names p.Id ---------------------------------------------------

// p.Id, p.Title, p.CreationDate, u.DisplayName and some of Score/ViewCount
fn q15009(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "views", "#c"]) }
fn q17856(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "views", "#c"]) }
fn q15059(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q15140(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q16210(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q15912(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "created", "owner", "score", "views", "#c"]) }
fn q15002(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "views", "owner", "#c"]) }
fn q15105(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "views", "owner", "#c"]) }
fn q15946(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "views", "owner", "#c"]) }
fn q16497(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "views", "owner", "#c"]) }
fn q15255(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "views", "#c"]) }
fn q18373(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "#c", "views"]) }
fn q15905(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "views", "owner", "created", "#c"]) }
fn q16149(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "score", "created", "#c"]) }
fn q16310(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "score", "created", "#c"]) }
fn q19356(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "score", "created", "owner", "#c"]) }
fn q16732(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "created", "score", "owner", "#c"]) }
fn q17843(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "score", "created", "owner", "#c"]) }
fn q17444(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "score", "owner", "created", "#c"]) }

// ... with both Score and ViewCount, in each order they appear
fn q15010(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q15071(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q15476(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q15606(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q16016(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q16151(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q17360(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q19426(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q15165(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q15402(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q15869(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q15048(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "views", "score", "#c"]) }
fn q15296(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "views", "score", "#c"]) }
fn q15369(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "views", "score", "#c"]) }
fn q15401(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "views", "score", "#c"]) }
fn q18464(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "views", "score", "#c"]) }
fn q15910(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }
fn q19961(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "score", "views", "#c"]) }

// p.Id grouped but not printed
fn q19464(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q15897(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "created", "owner", "score", "views", "#c"]) }
fn q17208(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "created", "owner", "score", "views", "#c"]) }
fn q19501(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "created", "owner", "score", "views", "#c"]) }
fn q16162(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q18050(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q19013(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q19658(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q17103(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["title", "owner", "created", "views", "score", "#c"]) }

// ... printing p.Body, u.Reputation or p.Tags as well
fn q15871(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "body", "created", "owner", "#c"]) }
fn q18764(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "body", "owner", "created", "score", "#c"]) }
fn q16737(db: &'static So) -> String { post_rows(db, true, "c", "score", 10, &["id", "title", "body", "owner", "created", "score", "#c"]) }
fn q15941(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "rep", "#c"]) }
fn q18402(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "created", "owner", "rep", "#c"]) }
fn q17982(db: &'static So) -> String { post_rows(db, true, "c", "created", 10, &["id", "title", "owner", "created", "tags", "#c"]) }

// ... printing pt.Name, which the PostTypes join supplies
fn q17750(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "created", "owner", "#c", "type"]) }
fn q15883(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "created", "owner", "type", "#c"]) }
fn q16287(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "created", "owner", "type", "#c"]) }
fn q16674(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "created", "owner", "type", "#c"]) }
fn q18242(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "created", "owner", "type", "#c"]) }
fn q18796(db: &'static So) -> String { post_rows(db, false, "c", "created", 10, &["id", "title", "created", "owner", "type", "#c"]) }

// ... with Votes joined as well, so the counts cross
fn q15119(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q15302(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q16237(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q16789(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q19945(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q15469(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#vx"]) }
fn q17331(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#vx"]) }

// --- GROUP BY a value tuple ------------------------------------------------

fn q18990(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#vx"]) }
fn q15012(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"]) }
fn q15028(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"]) }
fn q15423(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"]) }
fn q16329(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"]) }
fn q17955(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"]) }

fn q15554(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q15851(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q15997(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"]) }
fn q18134(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "score", "created", "#c"]) }
fn q17397(db: &'static So) -> String { tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["title", "created", "score", "owner", "#c"]) }

fn q18253(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#c", "views"]) }

fn q15338(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "score", "views", "#c"]) }
fn q19229(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "score", "views", "#c"]) }
fn q16978(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "views", "score", "#c"]) }
fn q19478(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "views", "score", "#c"]) }
fn q16585(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "score,created", 10, &["title", "created", "owner", "views", "score", "#c"]) }
fn q19352(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "views", "score", "owner", "#c"]) }
fn q15031(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q15064(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q15313(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q16482(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q16901(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q19875(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "score", "views", "#c"]) }
fn q15079(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "views", "score", "#c"]) }
fn q16307(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "views", "score", "#c"]) }

fn q18193(db: &'static So) -> String { tuple_rows(by_name_title_date_body(db, true, "c", PostWhere::All), "created", 0, &["owner", "title", "created", "body", "#c"]) }
fn q17462(db: &'static So) -> String { tuple_rows(by_name_title_date_views_body(db, true, "c", PostWhere::All), "created", 10, &["title", "body", "owner", "created", "views", "#c"]) }

fn q17831(db: &'static So) -> String { tuple_rows(by_name_title_date_ptype(db, false, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "ptype", "#c"]) }
fn q15893(db: &'static So) -> String { tuple_rows(by_name_title_date_ptype(db, false, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "ptype", "#c"]) }
fn q18077(db: &'static So) -> String { tuple_rows(by_name_title_date_ptype(db, false, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "ptype", "#c"]) }
fn q18400(db: &'static So) -> String { tuple_rows(by_name_title_date_ptype(db, false, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "ptype", "#c"]) }
fn q18543(db: &'static So) -> String { tuple_rows(by_name_title_date_ptype(db, false, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "ptype", "#c"]) }
fn q19317(db: &'static So) -> String { tuple_rows(by_name_title_date_ptype(db, false, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "ptype", "#c"]) }
fn q17253(db: &'static So) -> String { tuple_rows(by_name_title_date_ptype(db, false, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "ptype", "#c"]) }

pub static ENTRIES: &[harness::Entry] = &[
    ("15002", q15002), ("15009", q15009), ("15010", q15010), ("15012", q15012),
    ("15028", q15028), ("15031", q15031), ("15048", q15048), ("15059", q15059),
    ("15064", q15064), ("15071", q15071), ("15079", q15079), ("15105", q15105),
    ("15119", q15119), ("15140", q15140), ("15165", q15165), ("15255", q15255),
    ("15296", q15296), ("15302", q15302), ("15313", q15313), ("15338", q15338),
    ("15369", q15369), ("15401", q15401), ("15402", q15402), ("15423", q15423),
    ("15469", q15469), ("15476", q15476), ("15554", q15554), ("15606", q15606),
    ("15851", q15851), ("15869", q15869), ("15871", q15871), ("15883", q15883),
    ("15893", q15893), ("15897", q15897), ("15905", q15905), ("15910", q15910),
    ("15912", q15912), ("15941", q15941), ("15946", q15946), ("15997", q15997),
    ("16016", q16016), ("16149", q16149), ("16151", q16151), ("16162", q16162),
    ("16210", q16210), ("16237", q16237), ("16287", q16287), ("16307", q16307),
    ("16310", q16310), ("16329", q16329), ("16482", q16482), ("16497", q16497),
    ("16585", q16585), ("16674", q16674), ("16732", q16732), ("16737", q16737),
    ("16789", q16789), ("16901", q16901), ("16978", q16978), ("17103", q17103),
    ("17208", q17208), ("17253", q17253), ("17331", q17331), ("17360", q17360),
    ("17397", q17397), ("17444", q17444), ("17462", q17462), ("17750", q17750),
    ("17831", q17831), ("17843", q17843), ("17856", q17856), ("17955", q17955),
    ("17982", q17982), ("18050", q18050), ("18077", q18077), ("18134", q18134),
    ("18193", q18193), ("18242", q18242), ("18253", q18253), ("18373", q18373),
    ("18400", q18400), ("18402", q18402), ("18464", q18464), ("18543", q18543),
    ("18764", q18764), ("18796", q18796), ("18990", q18990), ("19013", q19013),
    ("19229", q19229), ("19317", q19317), ("19352", q19352), ("19356", q19356),
    ("19426", q19426), ("19464", q19464), ("19478", q19478), ("19501", q19501),
    ("19658", q19658), ("19875", q19875), ("19945", q19945), ("19961", q19961),
];
