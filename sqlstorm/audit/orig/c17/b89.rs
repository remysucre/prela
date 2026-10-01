use harness::prelude::*;

// Twenty-nine more of the comment-count query, with three WHERE clauses this
// corpus uses besides the post-type filter:
//
//   WHERE p.CreationDate >= '2023-01-01' | > '2023-01-01' | u.Reputation >= 100
//
// Each is passed to the view as a `PostWhere`, which builds it into the relation the
// group and the join are taken over — so the excluded posts are never joined,
// let alone grouped.

// --- comments only, the group is one post ----------------------------------

fn q16207(db: &'static So) -> String {
    post_rows(
        db,
        true,
        "c",
        "created",
        10,
        &["id", "title", "created", "owner", "#c"],
    )
}
fn q16785(db: &'static So) -> String {
    post_rows(
        db,
        true,
        "c",
        "created",
        10,
        &["id", "title", "created", "owner", "#c"],
    )
}
fn q19420(db: &'static So) -> String {
    post_rows(
        db,
        true,
        "c",
        "created",
        10,
        &["id", "title", "created", "owner", "#c"],
    )
}
fn q16647(db: &'static So) -> String {
    post_rows(
        db,
        true,
        "c",
        "created",
        10,
        &["id", "title", "owner", "created", "#c"],
    )
}
fn q18257(db: &'static So) -> String {
    post_rows(
        db,
        true,
        "c",
        "created",
        10,
        &["id", "title", "owner", "created", "#c"],
    )
}
fn q15301(db: &'static So) -> String {
    post_rows(
        db,
        true,
        "c",
        "created",
        10,
        &["id", "title", "created", "owner", "rep", "#c"],
    )
}
fn q15615(db: &'static So) -> String {
    post_rows(
        db,
        true,
        "c",
        "created",
        10,
        &["id", "title", "created", "owner", "rep", "#c"],
    )
}
fn q15041(db: &'static So) -> String {
    post_rows(
        db,
        true,
        "c",
        "created",
        10,
        &["id", "title", "created", "owner", "score", "views", "#c"],
    )
}
fn q19401(db: &'static So) -> String {
    post_rows(
        db,
        true,
        "c",
        "created",
        10,
        &["id", "title", "owner", "rep", "created", "views", "#c"],
    )
}
fn q17956(db: &'static So) -> String {
    post_rows(
        db,
        false,
        "c",
        "created",
        10,
        &["id", "title", "created", "owner", "type", "#c"],
    )
}

// ... with a date filter
fn q19269(db: &'static So) -> String {
    let v = posts_where(
        db,
        false,
        false,
        "c",
        PostWhere::CreatedGt(date(2023, 1, 1)),
    );
    render_posts(
        db,
        v,
        "created",
        10,
        &["id", "title", "created", "owner", "type", "#c"],
    )
}

fn q15786(db: &'static So) -> String {
    let v = posts_where(
        db,
        false,
        false,
        "c",
        PostWhere::CreatedGe(date(2023, 1, 1)),
    );
    render_posts(
        db,
        v,
        "created",
        10,
        &["id", "title", "created", "owner", "type", "#c"],
    )
}

fn q16659(db: &'static So) -> String {
    let v = posts_where(
        db,
        false,
        false,
        "c",
        PostWhere::CreatedGe(date(2023, 1, 1)),
    );
    render_posts(
        db,
        v,
        "created",
        10,
        &["id", "title", "created", "owner", "type", "#c"],
    )
}

// --- comments only, the group is a value tuple -----------------------------

fn q16229(db: &'static So) -> String {
    tuple_rows(
        by_name_title_date(db, true, "c", PostWhere::All),
        "created",
        10,
        &["title", "created", "owner", "#c"],
    )
}
fn q16632(db: &'static So) -> String {
    tuple_rows(
        by_name_title_date(db, true, "c", PostWhere::All),
        "created",
        10,
        &["title", "created", "owner", "#c"],
    )
}
fn q16708(db: &'static So) -> String {
    tuple_rows(
        by_name_title_date(db, true, "c", PostWhere::All),
        "created",
        10,
        &["title", "created", "owner", "#c"],
    )
}
fn q19558(db: &'static So) -> String {
    tuple_rows(
        by_name_title_date(db, true, "c", PostWhere::All),
        "created",
        10,
        &["title", "created", "owner", "#c"],
    )
}
fn q16037(db: &'static So) -> String {
    tuple_rows(
        by_name_title_date_score(db, true, "c", PostWhere::All),
        "created",
        10,
        &["title", "owner", "created", "score", "#c"],
    )
}
fn q19745(db: &'static So) -> String {
    tuple_rows(
        by_name_title_date_score_views(db, true, "c", PostWhere::All),
        "created",
        10,
        &["title", "owner", "created", "score", "views", "#c"],
    )
}
fn q17848(db: &'static So) -> String {
    tuple_rows(
        by_name_title_date_score_views(db, true, "c", PostWhere::All),
        "created",
        10,
        &["title", "created", "owner", "views", "score", "#c"],
    )
}

fn q15388(db: &'static So) -> String {
    let v = by_name_title_date_ptype(db, false, "c", PostWhere::CreatedGe(date(2023, 1, 1)));
    tuple_rows(
        v,
        "created",
        10,
        &["title", "created", "owner", "ptype", "#c"],
    )
}

fn q17458(db: &'static So) -> String {
    let v = by_name_title_date_ptype(db, false, "c", PostWhere::CreatedGe(date(2023, 1, 1)));
    tuple_rows(
        v,
        "created",
        0,
        &["title", "created", "owner", "ptype", "#c"],
    )
}

// --- Comments and Votes, which cross ---------------------------------------

fn q15653(db: &'static So) -> String {
    post_rows(
        db,
        true,
        "cv",
        "created",
        10,
        &["id", "title", "created", "owner", "#cx", "#vx"],
    )
}
fn q17315(db: &'static So) -> String {
    post_rows_outer(
        db,
        true,
        "cv",
        "created",
        10,
        &["id", "title", "created", "owner", "#cx", "#vx"],
    )
}
fn q15434(db: &'static So) -> String {
    tuple_rows(
        by_name_title_date(db, true, "cv", PostWhere::All),
        "created",
        10,
        &["title", "created", "owner", "#cx", "#vx"],
    )
}
fn q15761(db: &'static So) -> String {
    tuple_rows(
        by_name_title_date(db, false, "cv", PostWhere::All),
        "created",
        10,
        &["title", "created", "owner", "#cx", "#vx"],
    )
}

fn q11671(db: &'static So) -> String {
    let v = posts_where(
        db,
        false,
        false,
        "cv",
        PostWhere::CreatedGe(date(2023, 1, 1)),
    );
    render_posts(
        db,
        v,
        "created",
        100,
        &["id", "title", "created", "score", "owner", "#cx", "#vx"],
    )
}

// --- two that stand alone --------------------------------------------------

// SELECT u.Id, u.DisplayName, u.Reputation, p.Title, p.CreationDate, COUNT(v.Id)
// FROM Posts p JOIN Users u ... LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE u.Reputation >= 100 GROUP BY u.Id, u.DisplayName, u.Reputation, p.Id, p.Title, p.CreationDate
// ORDER BY u.Reputation DESC, VoteCount DESC
fn q10658(db: &'static So) -> String {
    // The WHERE is on the post's *owner*, not the post, so it is its own base
    // rather than a `PostWhere`: restricting `db.post` by the owner's reputation also
    // drops the ownerless posts, which is the inner join.
    let base = db.post.with(
        (&db.post.owner_user)
            .select(&db.user.reputation)
            .filt(|r| r >= 100),
    );
    let v = post_counts(db, base, "v");
    render_posts(
        db,
        v,
        "rep,#v",
        0,
        &["owner_id", "owner", "rep", "title", "created", "#v"],
    )
}

// SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Id,
//        u.DisplayName, u.Reputation, COUNT(b.Id)
// FROM Posts p JOIN Users u ... LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY p.Id, ..., u.Id, ... ORDER BY p.CreationDate DESC LIMIT 100
//
// Badges hang off the user, and the group is one post, so the count is just
// that user's badge count.
fn q10223(db: &'static So) -> String {
    let Post {
        origid,
        creation_date,
        score,
        view_count,
        owner_user,
        ..
    } = &db.post;
    let bc = badges_per_user(db);
    let mut v = Vec::new();
    db.post
        .with(owner_user)
        .select(
            origid
                .and(creation_date)
                .and(score)
                .and(view_count.opt())
                .and(
                    owner_user.select(
                        (&db.user.origid)
                            .and(&db.user.display_name)
                            .and(&db.user.reputation),
                    ),
                )
                .and(owner_user.select(&bc)),
        )
        .drive(|p, ((((((id, cd), s), w), ((uid, dn), rep)), nb))| {
            v.push((cd, id, p, s, w, uid, dn, rep, nb))
        });
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(
        v.iter()
            .take(100)
            .map(|&(cd, id, p, s, w, uid, dn, rep, nb)| {
                row(vec![
                    V::I(id),
                    title(db, p),
                    V::T(cd),
                    V::I(s),
                    oint(w),
                    V::I(uid),
                    V::S(dn),
                    V::I(rep),
                    V::I(nb),
                ])
            }),
    )
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10223", q10223),
    ("10658", q10658),
    ("11671", q11671),
    ("15041", q15041),
    ("15301", q15301),
    ("15388", q15388),
    ("15434", q15434),
    ("15615", q15615),
    ("15653", q15653),
    ("15761", q15761),
    ("15786", q15786),
    ("16037", q16037),
    ("16207", q16207),
    ("16229", q16229),
    ("16632", q16632),
    ("16647", q16647),
    ("16659", q16659),
    ("16708", q16708),
    ("16785", q16785),
    ("17315", q17315),
    ("17458", q17458),
    ("17848", q17848),
    ("17956", q17956),
    ("18257", q18257),
    ("19269", q19269),
    ("19401", q19401),
    ("19420", q19420),
    ("19558", q19558),
    ("19745", q19745),
];
