use harness::prelude::*;

// Mechanical spellings of a handful of queries. Each `fn` carries its select
// list and its FROM/WHERE/ORDER BY in the comment above it; the shared shapes
// are:
//
//   T1  SELECT <post/owner cols> FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//       WHERE p.PostTypeId = 1 ORDER BY <CreationDate|Score|ViewCount> DESC LIMIT n
//   T2  SELECT <post/owner cols, pt.Name> FROM Posts p JOIN Users u ... JOIN PostTypes pt ...
//       WHERE <one predicate> ORDER BY p.CreationDate DESC LIMIT n
//   T3  SELECT pt.Name, <aggregates> FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id
//       GROUP BY pt.Name ORDER BY <the count> DESC
//
// Every cut was checked: rows n and n+1 differ in the sort column, so none of
// these needs a rewrite. T3 has no LIMIT and the harness compares rows as a
// sorted multiset, so its ORDER BY does not matter either.
fn take_n(db: &'static So, mut v: Vec<Question>, n: usize, cols: &[&str]) -> String {
    v.truncate(n);
    rows(v.iter().map(|q| row(post_fields(db, q.pid, cols))))
}

fn newest(db: &'static So, n: usize, cols: &[&str]) -> String {
    let mut v = questions(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    take_n(db, v, n, cols)
}

fn best(db: &'static So, n: usize, cols: &[&str]) -> String {
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score));
    take_n(db, v, n, cols)
}

fn most_viewed(db: &'static So, n: usize, cols: &[&str]) -> String {
    let mut v = questions(db);
    v.sort_by(|a, b| b.views.cmp(&a.views));
    take_n(db, v, n, cols)
}

// Posts with an owner, newest first, under one restriction. PostTypes is an
// inner join on a NOT NULL column, so it only supplies pt.Name.
fn newest_posts<Q: Probe<D = Id<Post>>>(
    db: &'static So,
    keep: Q,
    n: usize,
    cols: &[&str],
) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post.with(owner_user).with(keep).select(creation_date).drive(|p, cd| v.push((cd, p)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(n).map(|&(_, p)| row(post_fields(db, p, cols))))
}

fn since2023(db: &'static So, n: usize, cols: &[&str]) -> String {
    newest_posts(db, (&db.post.creation_date).ge(date(2023, 1, 1)), n, cols)
}

fn scored(db: &'static So, n: usize, cols: &[&str]) -> String {
    newest_posts(db, (&db.post.score).gt(0), n, cols)
}

fn viewed(db: &'static So, n: usize, cols: &[&str]) -> String {
    newest_posts(db, (&db.post.view_count).gt(100), n, cols)
}

fn question(db: &'static So, n: usize, cols: &[&str]) -> String {
    newest_posts(db, (&db.post.post_type_id).eq(1), n, cols)
}

fn types(db: &'static So, cols: &[&str]) -> String {
    rows(by_count(db).iter().map(|a| row(type_fields(a, cols))))
}

// owner, rep, title, created | questions ORDER BY created DESC LIMIT 10
fn q17352(db: &'static So) -> String { newest(db, 10, &["owner", "rep", "title", "created"]) }
// name, n, score_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q12412(db: &'static So) -> String { types(db, &["name", "n", "score_avg"]) }
// owner, title, created, views, score | questions ORDER BY views DESC LIMIT 10
fn q19926(db: &'static So) -> String { most_viewed(db, 10, &["owner", "title", "created", "views", "score"]) }
// title, created, owner, type | posts+type WHERE scored ORDER BY created DESC LIMIT 10
fn q19398(db: &'static So) -> String { scored(db, 10, &["title", "created", "owner", "type"]) }
// owner, title, created, views, score | questions ORDER BY score DESC LIMIT 10
fn q18926(db: &'static So) -> String { best(db, 10, &["owner", "title", "created", "views", "score"]) }
// id, title, created, owner, type | posts+type WHERE viewed ORDER BY created DESC LIMIT 10
fn q19784(db: &'static So) -> String { viewed(db, 10, &["id", "title", "created", "owner", "type"]) }
// title, owner, created, score, views, answers | questions ORDER BY score DESC LIMIT 10
fn q17702(db: &'static So) -> String { best(db, 10, &["title", "owner", "created", "score", "views", "answers"]) }
// title, owner, created, score, views, answers | questions ORDER BY score DESC LIMIT 10
fn q18234(db: &'static So) -> String { best(db, 10, &["title", "owner", "created", "score", "views", "answers"]) }
// title, created, owner, type | posts+type WHERE since2023 ORDER BY created DESC LIMIT 10
fn q15099(db: &'static So) -> String { since2023(db, 10, &["title", "created", "owner", "type"]) }
// title, owner, created, type | posts+type WHERE since2023 ORDER BY created DESC LIMIT 10
fn q15403(db: &'static So) -> String { since2023(db, 10, &["title", "owner", "created", "type"]) }
// title, created, owner, type | posts+type WHERE since2023 ORDER BY created DESC LIMIT 10
fn q15911(db: &'static So) -> String { since2023(db, 10, &["title", "created", "owner", "type"]) }
// owner, title, created, type | posts+type WHERE since2023 ORDER BY created DESC LIMIT 10
fn q18483(db: &'static So) -> String { since2023(db, 10, &["owner", "title", "created", "type"]) }
// title, created, owner, views, answers | questions ORDER BY created DESC LIMIT 10
fn q15699(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "views", "answers"]) }
// title, created, owner, answers, views | questions ORDER BY created DESC LIMIT 10
fn q16121(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "answers", "views"]) }
// title, owner, created, views, answers | questions ORDER BY created DESC LIMIT 10
fn q18770(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "views", "answers"]) }
// title, owner, created, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q15747(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views", "answers"]) }
// title, owner, created, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q16093(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views", "answers"]) }
// title, owner, created, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q16356(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views", "answers"]) }
// title, created, owner, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q16529(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "score", "views", "answers"]) }
// title, owner, created, views, answers, score | questions ORDER BY created DESC LIMIT 10
fn q17481(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "views", "answers", "score"]) }
// title, owner, created, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q17530(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views", "answers"]) }
// owner, title, created, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q17818(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "score", "views", "answers"]) }
// owner, title, created, views, score, answers | questions ORDER BY created DESC LIMIT 10
fn q19128(db: &'static So) -> String { newest(db, 10, &["owner", "title", "created", "views", "score", "answers"]) }
// title, owner, created, views, score, answers | questions ORDER BY created DESC LIMIT 10
fn q19228(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "views", "score", "answers"]) }
// title, created, owner, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q19367(db: &'static So) -> String { newest(db, 10, &["title", "created", "owner", "score", "views", "answers"]) }
// id, title, created, owner, type | posts+type WHERE since2023 ORDER BY created DESC LIMIT 10
fn q16969(db: &'static So) -> String { since2023(db, 10, &["id", "title", "created", "owner", "type"]) }
// id, title, created, owner, type | posts+type WHERE since2023 ORDER BY created DESC LIMIT 10
fn q17129(db: &'static So) -> String { since2023(db, 10, &["id", "title", "created", "owner", "type"]) }
// id, title, created, owner, type | posts+type WHERE since2023 ORDER BY created DESC LIMIT 10
fn q18603(db: &'static So) -> String { since2023(db, 10, &["id", "title", "created", "owner", "type"]) }
// id, title, owner, created, type | posts+type WHERE since2023 ORDER BY created DESC LIMIT 10
fn q19772(db: &'static So) -> String { since2023(db, 10, &["id", "title", "owner", "created", "type"]) }
// title, owner, created, score, answers | questions ORDER BY created DESC LIMIT 10
fn q17451(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "answers"]) }
// name, n, score_avg, views_sum | GROUP BY PostTypes.Name ORDER BY count DESC
fn q10190(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_sum"]) }
// name, n, score_avg, views_sum | GROUP BY PostTypes.Name ORDER BY count DESC
fn q12015(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_sum"]) }
// name, n, score_avg, views_sum | GROUP BY PostTypes.Name ORDER BY count DESC
fn q12223(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_sum"]) }
// name, n, score_avg, views_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q10477(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_avg"]) }
// name, n, views_avg, score_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q11369(db: &'static So) -> String { types(db, &["name", "n", "views_avg", "score_avg"]) }
// name, n, score_avg, views_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q11387(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_avg"]) }
// name, n, views_avg, score_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q11604(db: &'static So) -> String { types(db, &["name", "n", "views_avg", "score_avg"]) }
// name, n, score_avg, views_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q13103(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_avg"]) }
// name, n, score_avg, views_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q12212(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_avg"]) }
// name, n, views_avg, score_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q12703(db: &'static So) -> String { types(db, &["name", "n", "views_avg", "score_avg"]) }
// name, n, score_avg, views_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q12914(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_avg"]) }
// name, n, views_avg, score_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q13400(db: &'static So) -> String { types(db, &["name", "n", "views_avg", "score_avg"]) }
// name, n, views_avg, score_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q14070(db: &'static So) -> String { types(db, &["name", "n", "views_avg", "score_avg"]) }
// name, n, views_avg, score_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q14600(db: &'static So) -> String { types(db, &["name", "n", "views_avg", "score_avg"]) }
// name, n, score_avg, views_sum | GROUP BY PostTypes.Name ORDER BY count DESC
fn q11540(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_sum"]) }
// name, n, score_avg, views_sum | GROUP BY PostTypes.Name ORDER BY count DESC
fn q13991(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_sum"]) }
// name, n, score_avg, views_sum | GROUP BY PostTypes.Name ORDER BY count DESC
fn q14975(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_sum"]) }
// name, n, score_avg, views_max | GROUP BY PostTypes.Name ORDER BY count DESC
fn q12156(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_max"]) }
// name, n, views_avg, score_sum | GROUP BY PostTypes.Name ORDER BY count DESC
fn q12998(db: &'static So) -> String { types(db, &["name", "n", "views_avg", "score_sum"]) }
// name, n, score_avg, views_sum | GROUP BY PostTypes.Name ORDER BY count DESC
fn q11164(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_sum"]) }
// name, n, score_avg, views_sum | GROUP BY PostTypes.Name ORDER BY count DESC
fn q11754(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_sum"]) }
// id, title, body, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q18026(db: &'static So) -> String { newest(db, 10, &["id", "title", "body", "owner", "created", "score", "views"]) }
// id, title, body, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q18879(db: &'static So) -> String { newest(db, 10, &["id", "title", "body", "owner", "created", "score", "views"]) }
// id, title, body, owner, created, score, views | questions ORDER BY created DESC LIMIT 10
fn q19091(db: &'static So) -> String { newest(db, 10, &["id", "title", "body", "owner", "created", "score", "views"]) }
// id, title, created, views, owner, rep | questions ORDER BY created DESC LIMIT 10
fn q18896(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "views", "owner", "rep"]) }
// id, title, created, owner, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q15262(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views", "answers"]) }
// id, title, owner, created, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q15286(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views", "answers"]) }
// id, title, created, owner, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q15875(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views", "answers"]) }
// id, title, owner, created, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q16668(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views", "answers"]) }
// id, title, owner, created, views, score, answers | questions ORDER BY created DESC LIMIT 10
fn q16690(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score", "answers"]) }
// id, title, owner, created, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q16743(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views", "answers"]) }
// id, title, created, owner, score, views, answers | questions ORDER BY created DESC LIMIT 10
fn q17289(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views", "answers"]) }
// id, title, created, owner, score, answers, views | questions ORDER BY created DESC LIMIT 10
fn q18213(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "answers", "views"]) }
// id, title, created, owner, score, answers, views | questions ORDER BY created DESC LIMIT 10
fn q18457(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "answers", "views"]) }
// title, owner, created, score, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q19378(db: &'static So) -> String { newest(db, 10, &["title", "owner", "created", "score", "views", "answers", "comments"]) }
// id, title, created, owner, score, views, tags | questions ORDER BY created DESC LIMIT 10
fn q16716(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views", "tags"]) }
// title, created, owner, type | posts+type WHERE viewed ORDER BY created DESC LIMIT 10
fn q19710(db: &'static So) -> String { viewed(db, 10, &["title", "created", "owner", "type"]) }
// id, title, owner, created, score, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q15076(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views", "answers", "comments"]) }
// id, title, owner, created, score, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q15317(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views", "answers", "comments"]) }
// id, title, created, owner, score, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q15745(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views", "answers", "comments"]) }
// id, title, created, score, owner, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q16299(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "score", "owner", "views", "answers", "comments"]) }
// id, title, owner, created, score, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q16336(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views", "answers", "comments"]) }
// id, owner, title, created, score, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q16780(db: &'static So) -> String { newest(db, 10, &["id", "owner", "title", "created", "score", "views", "answers", "comments"]) }
// id, title, created, owner, score, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q16868(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views", "answers", "comments"]) }
// id, title, created, owner, score, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q17169(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views", "answers", "comments"]) }
// id, title, created, owner, views, score, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q18586(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "views", "score", "answers", "comments"]) }
// id, title, created, score, owner, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q19077(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "score", "owner", "views", "answers", "comments"]) }
// id, title, owner, created, score, views, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q19191(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views", "answers", "comments"]) }
// id, title, owner, created, views, score, answers, comments | questions ORDER BY created DESC LIMIT 10
fn q19965(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "views", "score", "answers", "comments"]) }
// id, title, created, owner, score, views, answers, comments | questions ORDER BY score DESC LIMIT 10
fn q19215(db: &'static So) -> String { best(db, 10, &["id", "title", "created", "owner", "score", "views", "answers", "comments"]) }
// name, n, score_avg, views_avg, answers_avg | GROUP BY PostTypes.Name ORDER BY count DESC
fn q13829(db: &'static So) -> String { types(db, &["name", "n", "score_avg", "views_avg", "answers_avg"]) }
// id, title, created, owner, views, score, answers, comments, favorites | questions ORDER BY created DESC LIMIT 10
fn q16596(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "views", "score", "answers", "comments", "favorites"]) }
// id, title, owner, created, score, views, answers, comments, favorites | questions ORDER BY created DESC LIMIT 10
fn q16791(db: &'static So) -> String { newest(db, 10, &["id", "title", "owner", "created", "score", "views", "answers", "comments", "favorites"]) }
// id, title, created, owner, score, views, answers, comments, favorites | questions ORDER BY created DESC LIMIT 10
fn q16985(db: &'static So) -> String { newest(db, 10, &["id", "title", "created", "owner", "score", "views", "answers", "comments", "favorites"]) }

pub static ENTRIES: &[harness::Entry] = &[
    ("10190", q10190),
    ("10477", q10477),
    ("11164", q11164),
    ("11369", q11369),
    ("11387", q11387),
    ("11540", q11540),
    ("11604", q11604),
    ("11754", q11754),
    ("12015", q12015),
    ("12156", q12156),
    ("12212", q12212),
    ("12223", q12223),
    ("12412", q12412),
    ("12703", q12703),
    ("12914", q12914),
    ("12998", q12998),
    ("13103", q13103),
    ("13400", q13400),
    ("13829", q13829),
    ("13991", q13991),
    ("14070", q14070),
    ("14600", q14600),
    ("14975", q14975),
    ("15076", q15076),
    ("15099", q15099),
    ("15262", q15262),
    ("15286", q15286),
    ("15317", q15317),
    ("15403", q15403),
    ("15699", q15699),
    ("15745", q15745),
    ("15747", q15747),
    ("15875", q15875),
    ("15911", q15911),
    ("16093", q16093),
    ("16121", q16121),
    ("16299", q16299),
    ("16336", q16336),
    ("16356", q16356),
    ("16529", q16529),
    ("16596", q16596),
    ("16668", q16668),
    ("16690", q16690),
    ("16716", q16716),
    ("16743", q16743),
    ("16780", q16780),
    ("16791", q16791),
    ("16868", q16868),
    ("16969", q16969),
    ("16985", q16985),
    ("17129", q17129),
    ("17169", q17169),
    ("17289", q17289),
    ("17352", q17352),
    ("17451", q17451),
    ("17481", q17481),
    ("17530", q17530),
    ("17702", q17702),
    ("17818", q17818),
    ("18026", q18026),
    ("18213", q18213),
    ("18234", q18234),
    ("18457", q18457),
    ("18483", q18483),
    ("18586", q18586),
    ("18603", q18603),
    ("18770", q18770),
    ("18879", q18879),
    ("18896", q18896),
    ("18926", q18926),
    ("19077", q19077),
    ("19091", q19091),
    ("19128", q19128),
    ("19191", q19191),
    ("19215", q19215),
    ("19228", q19228),
    ("19367", q19367),
    ("19378", q19378),
    ("19398", q19398),
    ("19710", q19710),
    ("19772", q19772),
    ("19784", q19784),
    ("19926", q19926),
    ("19965", q19965),
];
