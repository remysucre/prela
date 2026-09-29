use harness::prelude::*;

// Eighty-seven queries that group *users* and aggregate their posts:
//
//   SELECT u.<columns>, COUNT(p.Id), SUM(CASE WHEN p.PostTypeId = 1 ...), ...
//   FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//        [LEFT JOIN Comments c ON p.Id = c.PostId]
//        [LEFT JOIN Votes v ON p.Id = v.PostId]
//        [LEFT JOIN Badges b ON u.Id = b.UserId]
//   GROUP BY u.Id, u.DisplayName[, ...] ORDER BY <one or more> DESC [LIMIT n]
//
// Posts is the optional side, so every user keeps a row: that is the
// `dense_fold_outer` inside `users_with_counts`, which gives a user with no
// posts the fold's initial value rather than dropping them.
//
// The FROM is a product of two independent sides — the user's posts (with
// their own comment and vote fan-outs) and the user's badges — so an aggregate
// over one is multiplied by the size of the other. `AVG(p.Score)` needs no
// correction at all, because the badge factor cancels between its sum and its
// count. See notes/limitations.md, "No optimiser".
//
// Three at the end are the other way round — `FROM Posts p JOIN Users u` — and
// group by the post.

fn u(db: &'static So, joins: &str, by: &str, n: usize, cols: &[&str]) -> String {
    user_rows(db, joins, false, by, n, cols)
}

/// `WHERE u.Reputation > n`, which is on a grouped column and so can be
/// applied to the grouped rows.
fn rep_over(db: &'static So, joins: &str, min: i64, by: &str, n: usize, cols: &[&str]) -> String {
    render_users(db, users_where(db, joins, false, UserWhere::RepGt(min)), joins, by, n, cols)
}

/// `WHERE u.CreationDate >= '2020-01-01'`, likewise.
fn joined_since(db: &'static So, joins: &str, cut: i64, by: &str, n: usize, cols: &[&str]) -> String {
    render_users(db, users_where(db, joins, false, UserWhere::CreatedGe(cut)), joins, by, n, cols)
}

// --- Posts alone -----------------------------------------------------------

fn q10823(db: &'static So) -> String { u(db, "", "#n", 10, &["name", "#n", "#q", "#a", "score_avg", "views_sum", "created_max"]) }
fn q16986(db: &'static So) -> String { u(db, "", "#rows", 10, &["name", "#rows", "#q", "#a", "rep_avg"]) }
fn q19802(db: &'static So) -> String { u(db, "", "#rows", 10, &["name", "#rows", "#q", "#a", "rep_avg"]) }
fn q17865(db: &'static So) -> String { u(db, "", "#rows", 10, &["name", "#rows", "#q", "#a", "rep_avg"]) }
fn q15523(db: &'static So) -> String { u(db, "", "#rows", 10, &["name", "#rows", "#q", "#a", "rep_avg"]) }
fn q17442(db: &'static So) -> String { u(db, "", "#rows", 0, &["name", "#rows", "#q", "#a", "rep_avg"]) }
fn q17973(db: &'static So) -> String { u(db, "", "#rows", 10, &["name", "#rows", "#q", "#a", "score_avg"]) }
fn q11584(db: &'static So) -> String { u(db, "", "#rows", 10, &["name", "#rows", "#q", "#a", "score_sum", "views_avg", "created_max"]) }
fn q16315(db: &'static So) -> String { u(db, "", "#rows", 10, &["name", "#rows", "#q", "#a"]) }
fn q17964(db: &'static So) -> String { u(db, "", "#rows", 10, &["name", "#rows", "#q", "#a"]) }
fn q19380(db: &'static So) -> String { u(db, "", "#rows", 10, &["name", "#rows", "#q", "#a"]) }
fn q12266(db: &'static So) -> String { u(db, "", "#rows", 0, &["name", "#rows", "score_avg0", "rep"]) }
fn q17160(db: &'static So) -> String { u(db, "", "#n", 10, &["uid", "name", "#n", "#q", "#a"]) }
fn q19586(db: &'static So) -> String { u(db, "", "#rows", 10, &["uid", "name", "#rows", "#q", "#a"]) }
fn q14545(db: &'static So) -> String { u(db, "", "#rows", 0, &["uid", "name", "#rows", "#q", "#a", "score_avg", "rep"]) }
fn q14438(db: &'static So) -> String { u(db, "", "#rows,score_avg", 10, &["uid", "name", "#rows", "score_avg", "#q", "#a"]) }
fn q14166(db: &'static So) -> String { u(db, "", "#rows", 0, &["uid", "name", "#rows", "score_avg_all", "views_sum0"]) }
fn q14504(db: &'static So) -> String { u(db, "", "#rows", 0, &["uid", "name", "#rows", "score_avg", "views_avg"]) }
fn q13110(db: &'static So) -> String { u(db, "", "#rows", 0, &["uid", "name", "#rows", "score_avg0", "created_max"]) }
fn q13322(db: &'static So) -> String { u(db, "", "#rows", 0, &["uid", "name", "#rows", "score_avg0", "views_avg0"]) }
fn q15492(db: &'static So) -> String { u(db, "", "#rows", 10, &["uid", "name", "#rows", "views_sum0"]) }
fn q13057(db: &'static So) -> String { u(db, "", "rep", 0, &["uid", "name", "rep", "#rows", "#a", "#q", "score_avg", "created_max"]) }
fn q14149(db: &'static So) -> String { u(db, "", "rep", 10, &["uid", "name", "rep", "#rows", "#a", "score_sum0"]) }
fn q14275(db: &'static So) -> String { u(db, "", "rep", 0, &["uid", "name", "rep", "#rows", "#q", "#a", "score_sum0", "views_avg"]) }
fn q14829(db: &'static So) -> String { u(db, "", "rep", 0, &["uid", "name", "rep", "#rows", "#q", "#a", "views_sum", "score_sum", "score_avg", "created_max"]) }
fn q12582(db: &'static So) -> String { u(db, "", "rep,score_sum", 0, &["uid", "name", "rep", "#rows", "score_sum", "views_avg", "activity_max"]) }
fn q12996(db: &'static So) -> String { u(db, "", "rep", 10, &["uid", "name", "rep", "#rows"]) }

// --- plus the user's Badges ------------------------------------------------

fn q12357(db: &'static So) -> String { u(db, "b", "#rows", 0, &["uid", "name", "#rows", "score_avg", "views_sum", "#b"]) }
fn q12930(db: &'static So) -> String { u(db, "b", "#rows,score_sum", 0, &["uid", "name", "#rows", "views_avg", "#bx", "score_sum"]) }
fn q13863(db: &'static So) -> String { u(db, "b", "rep", 10, &["uid", "name", "rep", "#n", "#b"]) }

// --- plus the posts' Comments ----------------------------------------------

fn q12285(db: &'static So) -> String { u(db, "c", "#rows,score_avg", 0, &["uid", "name", "#rows", "score_avg_all", "#cx"]) }
fn q12507(db: &'static So) -> String { u(db, "c", "#rows,score_avg", 0, &["uid", "name", "rep", "uup", "udown", "#rows", "#cx", "score_avg", "activity_max"]) }
fn q14224(db: &'static So) -> String { u(db, "C", "rep", 0, &["uid", "rep", "#n", "#cu"]) }
fn q12359(db: &'static So) -> String { rep_over(db, "cb", 1000, "#n", 100, &["name", "#n", "#q", "#a", "score_avg", "#c", "#b"]) }
fn q13281(db: &'static So) -> String { rep_over(db, "cb", 1000, "#n", 100, &["uid", "name", "#n", "#q", "#a", "score_avg_all", "views_avg_all", "#c", "#b"]) }
fn q11394(db: &'static So) -> String { u(db, "cb", "#n,score_sum", 100, &["uid", "name", "#n", "#q", "#a", "score_sum", "views_avg", "created_max", "#c", "#b"]) }

// --- plus the posts' Votes -------------------------------------------------

fn q13920(db: &'static So) -> String { u(db, "v", "#rows", 10, &["name", "#rows", "#q", "#a", "#up", "#down", "views_avg_all", "score_avg", "created_max"]) }
fn q15714(db: &'static So) -> String { u(db, "v", "#rows", 10, &["name", "#rows", "#q", "#a", "#up", "#down"]) }
fn q17652(db: &'static So) -> String { u(db, "v", "#rows", 10, &["name", "#rows", "#q", "#a", "bounty_sum"]) }
fn q12308(db: &'static So) -> String { u(db, "v", "#rows", 100, &["name", "#rows", "#q", "#a", "rep_avg", "created_max"]) }
fn q19973(db: &'static So) -> String { u(db, "v", "#rows", 10, &["name", "#rows", "#up", "#down"]) }
fn q11005(db: &'static So) -> String { u(db, "v", "#n", 100, &["uid", "name", "#n", "#q", "#a", "rep_avg", "created_max"]) }
fn q11843(db: &'static So) -> String { u(db, "v", "+uid", 0, &["uid", "name", "#n", "#vx", "score_avg"]) }
fn q10544(db: &'static So) -> String { u(db, "v", "#rows", 100, &["uid", "name", "#rows", "#q", "#a", "#up", "#down", "score_avg", "created_max"]) }
fn q13855(db: &'static So) -> String { u(db, "v", "#rows", 100, &["uid", "name", "#rows", "#q", "#a", "#up", "#down", "score_avg", "created_max"]) }
fn q16671(db: &'static So) -> String { u(db, "v", "#rows", 10, &["uid", "name", "#rows", "#q", "#a", "bounty_sum"]) }
fn q16024(db: &'static So) -> String { u(db, "v", "#rows", 10, &["uid", "name", "#rows", "#up", "#down"]) }
fn q13556(db: &'static So) -> String { u(db, "v", "#rows", 0, &["uid", "name", "#rows", "score_avg", "#up", "#down"]) }
fn q10908(db: &'static So) -> String { u(db, "v", "#rows,score_avg", 0, &["uid", "name", "#rows", "score_avg0", "rep"]) }
fn q11083(db: &'static So) -> String { u(db, "v", "rep,#rows", 0, &["uid", "name", "rep", "#rows", "#up", "#down", "views_avg", "score_avg", "created_max"]) }
fn q10332(db: &'static So) -> String { u(db, "v", "rep,#rows", 100, &["uid", "name", "rep", "#rows", "#up", "#down"]) }
fn q12538(db: &'static So) -> String { u(db, "v", "rep", 0, &["uid", "name", "rep", "#rows", "score_avg", "#up", "#down"]) }
fn q14492(db: &'static So) -> String { u(db, "v", "#rows,score_sum", 100, &["uid", "name", "rep", "#rows", "score_sum", "#up", "#down", "activity_max"]) }
fn q17063(db: &'static So) -> String { user_rows(db, "v", true, "#rows", 10, &["name", "#rows", "#up", "#down"]) }

fn q12262(db: &'static So) -> String { u(db, "vb", "#rows,#up", 0, &["uid", "name", "#rows", "#up", "#down", "#bx"]) }
fn q13957(db: &'static So) -> String { u(db, "vb", "#rows", 0, &["uid", "name", "#rows", "#vx", "#bx", "score_avg", "bounty_avg"]) }
fn q14295(db: &'static So) -> String { rep_over(db, "vb", 1000, "#n,rep", 100, &["uid", "name", "rep", "#n", "#v", "#b", "bounty_avg"]) }
fn q13198(db: &'static So) -> String { u(db, "vb", "rep", 100, &["uid", "rep", "ucreated", "#n", "#q", "#a", "score_sum", "#v", "#b"]) }

// --- Comments and Votes both -----------------------------------------------

fn q13463(db: &'static So) -> String { u(db, "cv", "#n", 100, &["name", "#n", "#q", "#a", "#c", "#up", "#down", "created_max"]) }
fn q10007(db: &'static So) -> String { u(db, "cv", "#n", 0, &["uid", "name", "#n", "#c", "#up", "#down", "score_avg", "created_max", "created_min"]) }
fn q11009(db: &'static So) -> String { u(db, "cv", "#n,#c", 0, &["uid", "name", "#n", "#c", "#v", "score_avg_all"]) }
fn q10760(db: &'static So) -> String { u(db, "cV", "#n,score_sum", 100, &["uid", "name", "#n", "#c", "#vu", "views_sum0", "score_sum0"]) }
fn q13805(db: &'static So) -> String { u(db, "cv", "#n,#cx", 0, &["uid", "name", "#n", "#cx", "#vx", "#up", "#down"]) }
fn q12159(db: &'static So) -> String { u(db, "cv", "#rows", 100, &["uid", "name", "#rows", "#q", "#a", "#c", "#v", "#up", "#down", "score_avg", "views_avg", "created_max"]) }
fn q11767(db: &'static So) -> String { u(db, "cv", "#rows", 0, &["uid", "name", "#rows", "#q", "#a", "#up", "#down", "#cx", "views_sum", "score_avg"]) }
fn q12618(db: &'static So) -> String { u(db, "cv", "#rows,#vx,#cx", 0, &["uid", "name", "#rows", "#vx", "#cx", "rep_avg"]) }
fn q11198(db: &'static So) -> String { u(db, "cv", "#rows", 0, &["uid", "name", "#rows", "score_avg0", "#cx", "#vx"]) }
fn q11201(db: &'static So) -> String { u(db, "cv", "#n", 0, &["uid", "name", "rep", "#n", "#c", "#q", "#a", "#up", "#down", "score_avg", "views_avg", "created_max"]) }
fn q12756(db: &'static So) -> String { joined_since(db, "cv", date(2020, 1, 1), "#n,#up", 100, &["uid", "name", "rep", "#n", "#c", "#up", "#down", "#q", "#a"]) }
fn q10312(db: &'static So) -> String { u(db, "cv", "#n,rep", 0, &["uid", "name", "rep", "#n", "#c", "#up", "#down", "score_avg", "created_max"]) }
fn q12251(db: &'static So) -> String { u(db, "cv", "#n,#up", 0, &["uid", "name", "rep", "#n", "#c", "#up", "#down", "score_avg", "created_max"]) }
fn q14670(db: &'static So) -> String { u(db, "cv", "#n,#up", 100, &["uid", "name", "rep", "#n", "#c", "#up", "#down"]) }
fn q14291(db: &'static So) -> String { rep_over(db, "cv", 100, "#n,#up", 0, &["uid", "name", "rep", "#n", "#c", "#v", "#up", "#down", "created_max"]) }
fn q10387(db: &'static So) -> String { u(db, "cv", "rep", 0, &["uid", "name", "rep", "#n", "#c", "bounty_sum", "#up", "#down", "score_avg", "views_avg"]) }
fn q10546(db: &'static So) -> String { u(db, "cv", "#n", 0, &["uid", "name", "rep", "#n", "#q", "#a", "#up", "#down", "score_avg", "#c"]) }
fn q13260(db: &'static So) -> String { u(db, "cv", "#rows", 100, &["uid", "name", "rep", "#rows", "#q", "#a", "#cx", "#up", "#down", "created_max"]) }
fn q10951(db: &'static So) -> String { u(db, "cv", "rep", 100, &["uid", "name", "rep", "ucreated", "#n", "#c", "#v", "score_avg"]) }
fn q11277(db: &'static So) -> String { u(db, "cv", "rep", 0, &["uid", "name", "rep", "ucreated", "last_access", "#n", "#c", "#up", "#down", "score_sum", "created_max"]) }
fn q12474(db: &'static So) -> String { u(db, "cv", "rep", 100, &["uid", "name", "rep", "ucreated", "last_access", "#n", "#c", "bounty_avg"]) }
fn q11111(db: &'static So) -> String { u(db, "cv", "rep", 100, &["uid", "name", "rep", "ucreated", "last_access", "#n", "#c", "bounty_sum", "score_max", "views_avg"]) }
fn q10639(db: &'static So) -> String { u(db, "cv", "rep", 0, &["uid", "name", "rep", "ucreated", "uviews", "uup", "udown", "#n", "#c", "#q", "#a", "bounty_sum"]) }
fn q14845(db: &'static So) -> String { u(db, "cv", "rep", 0, &["uid", "rep", "#n", "#v", "#c"]) }
fn q14013(db: &'static So) -> String { u(db, "cvb", "views_sum,#n", 100, &["name", "rep", "#n", "views_sum0", "#up", "#down", "#c", "#b", "score_avg", "activity_max"]) }
fn q10894(db: &'static So) -> String { u(db, "cvb", "#n", 100, &["uid", "name", "#n", "#q", "#a", "score_avg", "#up", "#down", "#c", "#b"]) }

// --- the other way round: FROM Posts p JOIN Users u, grouped by the post ----

fn q10101(db: &'static So) -> String { post_rows(db, true, "cvb", "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down", "#b", "activity"]) }
fn q13102(db: &'static So) -> String { post_rows(db, false, "cvb", "created", 100, &["id", "title", "created", "views", "answers", "comments", "score", "owner_id", "owner", "rep", "#up", "#down", "#c", "#b"]) }

fn q12690(db: &'static So) -> String {
    let v = posts_where(db, false, false, "cvh", PostWhere::CreatedGe(date(2023, 1, 1)));
    render_posts(db, v, "created", 0, &["id", "title", "created", "views", "answers", "comments", "score", "owner", "rep", "#cx", "#up", "#down", "#h"])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10007", q10007), ("10101", q10101), ("10312", q10312), ("10332", q10332),
    ("10387", q10387), ("10544", q10544), ("10546", q10546), ("10639", q10639),
    ("10760", q10760), ("10823", q10823), ("10894", q10894), ("10908", q10908),
    ("10951", q10951), ("11005", q11005), ("11009", q11009), ("11083", q11083),
    ("11111", q11111), ("11198", q11198), ("11201", q11201), ("11277", q11277),
    ("11394", q11394), ("11584", q11584), ("11767", q11767), ("11843", q11843),
    ("12159", q12159), ("12251", q12251), ("12262", q12262), ("12266", q12266),
    ("12285", q12285), ("12308", q12308), ("12357", q12357), ("12359", q12359),
    ("12474", q12474), ("12507", q12507), ("12538", q12538), ("12582", q12582),
    ("12618", q12618), ("12690", q12690), ("12756", q12756), ("12930", q12930),
    ("12996", q12996), ("13057", q13057), ("13102", q13102), ("13110", q13110),
    ("13198", q13198), ("13260", q13260), ("13281", q13281), ("13322", q13322),
    ("13463", q13463), ("13556", q13556), ("13805", q13805), ("13855", q13855),
    ("13863", q13863), ("13920", q13920), ("13957", q13957), ("14013", q14013),
    ("14149", q14149), ("14166", q14166), ("14224", q14224), ("14275", q14275),
    ("14291", q14291), ("14295", q14295), ("14438", q14438), ("14492", q14492),
    ("14504", q14504), ("14545", q14545), ("14670", q14670), ("14829", q14829),
    ("14845", q14845), ("15492", q15492), ("15523", q15523), ("15714", q15714),
    ("16024", q16024), ("16315", q16315), ("16671", q16671), ("16986", q16986),
    ("17063", q17063), ("17160", q17160), ("17442", q17442), ("17652", q17652),
    ("17865", q17865), ("17964", q17964), ("17973", q17973), ("19380", q19380),
    ("19586", q19586), ("19802", q19802), ("19973", q19973),
];
