use harness::prelude::*;
use crate::q::by_created;

// COUNT(DISTINCT p.OwnerUserId) per post type name.
fn distinct_owners(db: &'static So) -> Fold<Str, i64> {
    db.post.group_by(ptype_name(db)).select(&db.post.owner_user_id).count_distinct()
}


// SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.ViewCount) AS AvgViewCount, COUNT(DISTINCT p.Tags) AS UniqueTags
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name ORDER BY AvgViewCount DESC;
fn q11239(db: &'static So) -> String {
    let tags = db.post.group_by(ptype_name(db)).select(&db.post.tags_str).count_distinct();
    let v = drain((&stats_by_type(db)).and((&tags).opt()));
    rows(v.into_iter().map(|(nm, (a, d))| row(vec![V::S(nm), V::I(a[0]), avg(a[3], a[2]), V::I(d.unwrap_or(0))])))
}

// SELECT pt.Name AS PostTypeName, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore, COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name ORDER BY TotalPosts DESC;
fn q11911(db: &'static So) -> String {
    let v = drain((&stats_by_type(db)).and((&distinct_owners(db)).opt()));
    rows(v.into_iter().map(|(nm, (a, d))| row(vec![V::S(nm), V::I(a[0]), avg(a[1], a[0]), V::I(d.unwrap_or(0))])))
}

// SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore, COUNT(DISTINCT p.OwnerUserId) AS TotalUsers
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Id, pt.Name ORDER BY TotalPosts DESC;
fn q14915(db: &'static So) -> String {
    let Post { post_type, score, owner_user_id, .. } = &db.post;
    let sc = db.post.group_by(post_type).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let d = db.post.group_by(post_type).select(owner_user_id).count_distinct();
    let v = drain((&sc).and((&d).opt()));
    rows(v.into_iter().map(|(t, (a, d))| row(vec![tname(db, t), V::I(a[0]), avg(a[1], a[0]), V::I(d.unwrap_or(0))])))
}

// SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews, COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name ORDER BY TotalPosts DESC;
fn q12140(db: &'static So) -> String {
    let v = drain((&stats_by_type(db)).and((&distinct_owners(db)).opt()));
    rows(v.into_iter().map(|(nm, (a, d))| row(vec![V::S(nm), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(d.unwrap_or(0))])))
}

// 12771: SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViewCount, COUNT(DISTINCT p.OwnerUserId) AS ActiveUsers
//        FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name ORDER BY PostCount DESC;
// 11039: the same, with the columns named TotalPosts and TotalUsers.
fn count_avgscore_avgviews_distinctusers(db: &'static So) -> String {
    let v = drain((&stats_by_type(db)).and((&distinct_owners(db)).opt()));
    rows(v.into_iter().map(|(nm, (a, d))| row(vec![V::S(nm), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(d.unwrap_or(0))])))
}

// SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore, COUNT(DISTINCT p.OwnerUserId) AS TotalUsers
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= '2022-01-01' GROUP BY pt.Name ORDER BY TotalPosts DESC;
fn q14843(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(date(2022, 1, 1)));
    let sc = recent().group_by(ptype_name(db)).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let d = recent().group_by(ptype_name(db)).select(owner_user_id).count_distinct();
    let v = drain((&sc).and((&d).opt()));
    rows(v.into_iter().map(|(k, (a, d))| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(d.unwrap_or(0))])))
}

fn q10119(db: &'static So) -> String {
    let Post { post_type, creation_date, last_activity_date, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut v: Vec<(Str, i64, i128)> = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(last_activity_date.and(creation_date))
        .fold((0i64, 0i128), |(n, s), (la, cd)| (n + 1, s + (la - cd) as i128))
        .drive(|k, (n, s)| v.push((k, n, s)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().map(|(k, n, s)| {
        row(vec![V::S(k), V::I(*n), V::F(*s as f64 / 1e6 / *n as f64)])
    }))
}

fn totals(db: &'static So, from: Option<i64>) -> (i64, i64, i64, i64, i64, i64, i64) {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;

    let (t, q, a, ss) = match from {
        None => db.post.select(post_type_id.and(score)).fold_flat(
            (0i64, 0i64, 0i64, 0i64),
            |(t, q, a, s), (ty, sc)| {
                (t + 1, q + (ty == 1) as i64, a + (ty == 2) as i64, s + sc)
            },
        ),
        Some(d) => db
            .post
            .with(creation_date.ge(d))
            .select(post_type_id.and(score))
            .fold_flat((0i64, 0i64, 0i64, 0i64), |(t, q, a, s), (ty, sc)| {
                (t + 1, q + (ty == 1) as i64, a + (ty == 2) as i64, s + sc)
            }),
    };
    let (vn, vs) = match from {
        None => db
            .post
            .select(view_count)
            .fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x)),
        Some(d) => db
            .post
            .with(creation_date.ge(d))
            .select(view_count)
            .fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x)),
    };
    (t, q, a, ss, vn, vs, 0)
}

fn q17893(db: &'static So) -> String {
    let (t, q, a, _, vn, vs, _) = totals(db, Some(date(2023, 1, 1)));
    row(vec![V::I(t), nullable(q, t), nullable(a, t), avg(vs, vn)])
}

fn q12062(db: &'static So) -> String {
    let (t, q, a, ss, vn, vs, _) = totals(db, None);
    row(vec![V::I(t), nullable(q, t), nullable(a, t), avg(ss, t), avg(vs, vn)])
}

fn q18750(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut all: Vec<(Str, i64, i64, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user.select(display_name))
        .select(post_type_id)
        .fold((0i64, 0i64, 0i64), |(n, q, a), ty| {
            (n + 1, q + (ty == 1) as i64, a + (ty == 2) as i64)
        })
        .drive(|k, (n, q, a)| all.push((k, n, q, a)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(dn, n, q, a)| {
        row(vec![V::S(dn), V::I(*n), V::I(*q), V::I(*a)])
    }))
}

fn q15356(db: &'static So) -> String {
    let Post { view_count, answer_count, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            title(db, r.pid),
            V::T(r.created),
            V::S(r.display_name),
            V::I(view_count.get(r.pid).unwrap_or(0)),
            V::I(answer_count.get(r.pid).unwrap_or(0)),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("11239", q11239),
    ("17893", q17893),
    ("11911", q11911),
    ("10119", q10119),
    ("14915", q14915),
    ("12062", q12062),
    ("18750", q18750),
    ("12140", q12140),
    ("15356", q15356),
    ("12771", count_avgscore_avgviews_distinctusers),
    ("11039", count_avgscore_avgviews_distinctusers),
    ("14843", q14843),
];
