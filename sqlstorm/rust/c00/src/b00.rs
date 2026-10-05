// Batch 00 — worklist positions 1..10.
//
// The SQL each port corresponds to is in
// $SQLSTORM_DATA/corpus/queries/<id>.sql; the one-line gloss above each
// function is there to make the shape of the port readable without opening
// it. ORDER BY / LIMIT are host-Rust work after the plan, since prela's
// job is the relational part.

use harness::prelude::*;

// 14889, 12558 — SELECT COUNT(*) FROM Posts
fn q14889(db: &'static So) -> String {
    row(vec![V::I((&db.post.id).fold_flat(0i64, |a, _| a + 1))])
}

// 12616 — SELECT COUNT(*) FROM Users
fn q12616(db: &'static So) -> String {
    row(vec![V::I((&db.user.id).fold_flat(0i64, |a, _| a + 1))])
}

// 11017 — SELECT 'Total Posts', COUNT(*) FROM Posts GROUP BY 'Total Posts'
fn q11017(db: &'static So) -> String {
    let n = (&db.post.id).fold_flat(0i64, |a, _| a + 1);
    row(vec![V::S("Total Posts"), V::I(n)])
}

// 14597 — SELECT PostTypeId, COUNT(*) FROM Posts GROUP BY PostTypeId
fn q14597(db: &'static So) -> String {
    let Post { post_type_id, origid, .. } = &db.post;

    let g = db
        .post
        .group_by(post_type_id)
        .select(origid)
        .fold(0i64, |a, _| a + 1);

    let mut out = Vec::new();
    g.drive(|k, n| out.push(row(vec![V::I(k), V::I(n)])));
    rows(out)
}

// 10576 — 14597 with ORDER BY PostCount DESC (rows compare as a set)
fn q10576(db: &'static So) -> String {
    q14597(db)
}

// 14374 — SELECT COUNT(*), AVG(Reputation) FROM Users
fn q14374(db: &'static So) -> String {
    let (sum, n) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, c), r| (s + r, c + 1));
    row(vec![V::I(n), avg(sum, n)])
}

// 10235 — Posts JOIN PostTypes, COUNT(p.Id) GROUP BY pt.Name
fn q10235(db: &'static So) -> String {
    let Post { post_type, origid, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let g = db
        .post
        .group_by(post_type.select(name))
        .select(origid)
        .fold(0i64, |a, _| a + 1);

    let mut out = Vec::new();
    g.drive(|k, n| out.push(row(vec![V::S(k), V::I(n)])));
    rows(out)
}

// 13637 — AVG(u.Reputation) over Users INNER JOIN Posts ON u.Id =
// p.OwnerUserId WHERE p.PostTypeId = 1. The average is over the JOIN, so
// one term per qualifying post; posts with no owner drop out on their own
// because `owner_user` is absent there.
fn q13637(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let User { reputation, .. } = &db.user;

    let (sum, n) = db
        .post
        .with(post_type_id.eq(1))
        .select(owner_user.select(reputation))
        .fold_flat((0i64, 0i64), |(s, c), r| (s + r, c + 1));
    row(vec![avg(sum, n)])
}

// 19206 — SELECT Id, DisplayName, Reputation, CreationDate FROM Users
//         ORDER BY Reputation DESC LIMIT 10
fn q19206(db: &'static So) -> String {
    let User { origid, display_name, reputation, creation_date, .. } = &db.user;

    let mut all: Vec<(i64, i64, Str, i64)> = Vec::new();
    db.user
        .select(reputation.and(origid).and(display_name).and(creation_date))
        .drive(|_, (((r, id), dn), cd)| all.push((r, id, dn, cd)));
    all.sort_by(|a, b| b.0.cmp(&a.0));
    rows(all.iter().take(10).map(|(r, id, dn, cd)| {
        row(vec![V::I(*id), V::S(dn), V::I(*r), V::T(*cd)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
        ("14889", q14889),
        ("12558", q14889),
        ("12616", q12616),
        ("11017", q11017),
        ("14597", q14597),
        ("10576", q10576),
        ("14374", q14374),
        ("10235", q10235),
        ("13637", q13637),
        ("19206", q19206),
];
