use harness::prelude::*;

// Mechanical spellings of a handful of queries. Each `fn` carries its select
// list and its ORDER BY in the comment above it; the shared shape is
//
//   SELECT <post/owner cols> FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//   WHERE p.PostTypeId = 1 ORDER BY <CreationDate|Score> DESC LIMIT 10
fn top_questions(db: &'static So, key: &'static Col<Post, i64>, n: usize, cols: &[&str]) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(key)), |&(_, k)| std::cmp::Reverse(k), n);
    rows(v.iter().map(|&(p, _)| row(post_fields(db, p, cols))))
}

fn newest(db: &'static So, n: usize, cols: &[&str]) -> String {
    top_questions(db, &db.post.creation_date, n, cols)
}

fn best(db: &'static So, n: usize, cols: &[&str]) -> String {
    top_questions(db, &db.post.score, n, cols)
}

// id, title, created, owner, views, score | questions ORDER BY score DESC LIMIT 10
fn q17783(db: &'static So) -> String { best(db, 10, &["id", "title", "created", "owner", "views", "score"]) }
// id, title, owner, created, score, views | questions ORDER BY score DESC LIMIT 10
fn q17892(db: &'static So) -> String { best(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, created, owner, score, views | questions ORDER BY score DESC LIMIT 10
fn q18695(db: &'static So) -> String { best(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, owner, created, score, views | questions ORDER BY score DESC LIMIT 10
fn q19663(db: &'static So) -> String { best(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, owner, created, views, score | questions ORDER BY score DESC LIMIT 10
fn q19809(db: &'static So) -> String { best(db, 10, &["id", "title", "owner", "created", "views", "score"]) }
// title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q15049(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "score", "views"]) }
// owner, title, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15055(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "score", "views"]) }
// title, created, owner, views, score | questions ORDER BY created DESC LIMIT 10
fn q15075(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "views", "score"]) }
// title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15157(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views"]) }
// title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q15184(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "views", "score"]) }
// owner, title, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q15190(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "views", "score"]) }
// owner, title, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q15223(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "views", "score"]) }
// title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15245(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views"]) }
// title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q15264(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "score", "views"]) }
// title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15283(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views"]) }
// owner, title, views, created, score | questions ORDER BY created DESC LIMIT 10
fn q15359(db: &'static So) -> String { newest(db, 10, &["owner", "title", "views", "created", "score"]) }
// title, created, owner, views, score | questions ORDER BY created DESC LIMIT 10
fn q15393(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "views", "score"]) }
// title, created, views, score, owner | questions ORDER BY created DESC LIMIT 10
fn q15490(db: &'static So) -> String { newest(db, 10, &["title", "created", "views", "score", "owner"]) }
// title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15513(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views"]) }
// title, created, owner, views, score | questions ORDER BY created DESC LIMIT 10
fn q15580(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "views", "score"]) }
// title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15762(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views"]) }
// title, score, owner, created, views | questions ORDER BY created DESC LIMIT 10
fn q15763(db: &'static So) -> String { newest(db, 10, &["title", "score", "owner", "created", "views"]) }
// title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q15828(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "score", "views"]) }
// owner, title, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q15841(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "views", "score"]) }
// title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q15959(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "score", "views"]) }
// title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15978(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views"]) }
// title, created, owner, views, score | questions ORDER BY created DESC LIMIT 10
fn q16150(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "views", "score"]) }
// owner, title, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q16351(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "score", "views"]) }
// title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q16388(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "views", "score"]) }
// title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q16495(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "views", "score"]) }
// owner, title, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q16547(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "views", "score"]) }
// owner, title, score, views, created | questions ORDER BY created DESC LIMIT 10
fn q16713(db: &'static So) -> String { newest(db, 10, &["owner", "title", "score", "views", "created"]) }
// owner, title, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q16749(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "views", "score"]) }
// owner, title, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q16947(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "views", "score"]) }
// title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q17031(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "views", "score"]) }
// title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q17320(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views"]) }
// owner, title, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q17328(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "score", "views"]) }
// title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q17420(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "score", "views"]) }
// title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q17682(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "score", "views"]) }
// title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q17730(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views"]) }
// title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q18095(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "score", "views"]) }
// title, created, owner, views, score | questions ORDER BY created DESC LIMIT 10
fn q18110(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "views", "score"]) }
// owner, title, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q18119(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "score", "views"]) }
// owner, title, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q18145(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "score", "views"]) }
// owner, title, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q18317(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "views", "score"]) }
// title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q18574(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "score", "views"]) }
// title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q18583(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views"]) }
// owner, title, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q18698(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "views", "score"]) }
// title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q19037(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "views", "score"]) }
// owner, title, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q19084(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "score", "views"]) }
// title, score, views, owner, created | questions ORDER BY created DESC LIMIT 10
fn q19642(db: &'static So) -> String { newest(db, 10, &["title", "score", "views", "owner", "created"]) }
// owner, title, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q19866(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "score", "views"]) }
// owner_id, owner, title, created, score | questions ORDER BY created DESC LIMIT 10
fn q18628(db: &'static So) -> String { newest(db, 10, &["owner_id", "owner", "title", "created", "score"]) }
// id, title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q15033(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15054(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, score, views, owner, created | questions ORDER BY created DESC LIMIT 10
fn q15088(db: &'static So) -> String { newest(db, 10, &["id", "title", "score", "views", "owner", "created"]) }
// id, title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q15124(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score"]) }
// id, title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q15197(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score"]) }
// id, title, created, owner, views, score | questions ORDER BY created DESC LIMIT 10
fn q15265(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "views", "score"]) }
// id, title, views, score, owner, created | questions ORDER BY created DESC LIMIT 10
fn q15279(db: &'static So) -> String { newest(db, 10, &["id", "title", "views", "score", "owner", "created"]) }
// id, title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15329(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15358(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q15376(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, created, score, owner, views | questions ORDER BY created DESC LIMIT 10
fn q15378(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "score", "owner", "views"]) }
// id, title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q15383(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q15489(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15583(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q15621(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score"]) }
// id, title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15635(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q15636(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q15741(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score"]) }
// id, title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q15821(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, created, owner, views, score | questions ORDER BY created DESC LIMIT 10
fn q15824(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "views", "score"]) }
// id, title, owner, score, views, created | questions ORDER BY created DESC LIMIT 10
fn q15839(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "score", "views", "created"]) }
// id, title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q15902(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score"]) }
// id, title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q15922(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q16111(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score"]) }
// id, title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q16170(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, views, owner, created, score | questions ORDER BY created DESC LIMIT 10
fn q16254(db: &'static So) -> String { newest(db, 10, &["id", "title", "views", "owner", "created", "score"]) }
// id, title, created, owner, views, score | questions ORDER BY created DESC LIMIT 10
fn q16266(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "views", "score"]) }
// id, title, created, owner, views, score | questions ORDER BY created DESC LIMIT 10
fn q16360(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "views", "score"]) }
// owner_id, owner, title, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q16509(db: &'static So) -> String { newest(db, 10, &["owner_id", "owner", "title", "created", "views", "score"]) }
// id, title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q16605(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q16702(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score"]) }
// id, title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q16915(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, owner, score, views, created | questions ORDER BY created DESC LIMIT 10
fn q16949(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "score", "views", "created"]) }
// id, title, score, views, owner, created | questions ORDER BY created DESC LIMIT 10
fn q17073(db: &'static So) -> String { newest(db, 10, &["id", "title", "score", "views", "owner", "created"]) }
// id, title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q17532(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, score, owner, created, views | questions ORDER BY created DESC LIMIT 10
fn q17553(db: &'static So) -> String { newest(db, 10, &["id", "title", "score", "owner", "created", "views"]) }
// id, title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q18227(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, score, views, owner, created | questions ORDER BY created DESC LIMIT 10
fn q18272(db: &'static So) -> String { newest(db, 10, &["id", "title", "score", "views", "owner", "created"]) }
// id, title, created, owner, score, views | questions ORDER BY created DESC LIMIT 10
fn q18279(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views"]) }
// id, title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q18392(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score"]) }
// id, title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q18507(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, owner, score, views, created | questions ORDER BY created DESC LIMIT 10
fn q18678(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "score", "views", "created"]) }
// id, title, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q18951(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views"]) }
// id, title, owner, created, views, score | questions ORDER BY created DESC LIMIT 10
fn q19226(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score"]) }
// id, title, created, owner, views, score | questions ORDER BY created DESC LIMIT 10
fn q19504(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "views", "score"]) }
// title, created, owner, rep | questions ORDER BY created DESC LIMIT 10
fn q15632(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "rep"]) }
// title, created, owner, rep | questions ORDER BY created DESC LIMIT 10
fn q17227(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "rep"]) }

pub static ENTRIES: &[harness::Entry] = &[
    ("15033", q15033),
    ("15049", q15049),
    ("15054", q15054),
    ("15055", q15055),
    ("15075", q15075),
    ("15088", q15088),
    ("15124", q15124),
    ("15157", q15157),
    ("15184", q15184),
    ("15190", q15190),
    ("15197", q15197),
    ("15223", q15223),
    ("15245", q15245),
    ("15264", q15264),
    ("15265", q15265),
    ("15279", q15279),
    ("15283", q15283),
    ("15329", q15329),
    ("15358", q15358),
    ("15359", q15359),
    ("15376", q15376),
    ("15378", q15378),
    ("15383", q15383),
    ("15393", q15393),
    ("15489", q15489),
    ("15490", q15490),
    ("15513", q15513),
    ("15580", q15580),
    ("15583", q15583),
    ("15621", q15621),
    ("15632", q15632),
    ("15635", q15635),
    ("15636", q15636),
    ("15741", q15741),
    ("15762", q15762),
    ("15763", q15763),
    ("15821", q15821),
    ("15824", q15824),
    ("15828", q15828),
    ("15839", q15839),
    ("15841", q15841),
    ("15902", q15902),
    ("15922", q15922),
    ("15959", q15959),
    ("15978", q15978),
    ("16111", q16111),
    ("16150", q16150),
    ("16170", q16170),
    ("16254", q16254),
    ("16266", q16266),
    ("16351", q16351),
    ("16360", q16360),
    ("16388", q16388),
    ("16495", q16495),
    ("16509", q16509),
    ("16547", q16547),
    ("16605", q16605),
    ("16702", q16702),
    ("16713", q16713),
    ("16749", q16749),
    ("16915", q16915),
    ("16947", q16947),
    ("16949", q16949),
    ("17031", q17031),
    ("17073", q17073),
    ("17227", q17227),
    ("17320", q17320),
    ("17328", q17328),
    ("17420", q17420),
    ("17532", q17532),
    ("17553", q17553),
    ("17682", q17682),
    ("17730", q17730),
    ("17783", q17783),
    ("17892", q17892),
    ("18095", q18095),
    ("18110", q18110),
    ("18119", q18119),
    ("18145", q18145),
    ("18227", q18227),
    ("18272", q18272),
    ("18279", q18279),
    ("18317", q18317),
    ("18392", q18392),
    ("18507", q18507),
    ("18574", q18574),
    ("18583", q18583),
    ("18628", q18628),
    ("18678", q18678),
    ("18695", q18695),
    ("18698", q18698),
    ("18951", q18951),
    ("19037", q19037),
    ("19084", q19084),
    ("19226", q19226),
    ("19504", q19504),
    ("19642", q19642),
    ("19663", q19663),
    ("19809", q19809),
    ("19866", q19866),
];
