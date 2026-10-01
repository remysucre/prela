use harness::prelude::*;

struct Owned {
    name: Str,
    n: i64,
    score_sum: i64,
    views_n: i64,
    views_sum: i64,
    rep_sum: i64,
    users: i64,
    last_activity: i64,
}

// Posts JOIN PostTypes JOIN Users grouped by pt.Name: counts, sums, the latest activity, and COUNT(DISTINCT p.OwnerUserId) joined on the type name.
fn owned(db: &'static So) -> Vec<Owned> {
    let Post { score, view_count, owner_user, owner_user_id, last_activity_date, .. } = &db.post;
    let sc = db
        .post
        .with(owner_user)
        .group_by(ptype_name(db))
        .select(score.and(owner_user.select(&db.user.reputation)).and(last_activity_date).and(view_count.opt()))
        .fold([0, 0, 0, i64::MIN, 0, 0], |a, (((x, rep), la), v)| [a[0] + 1, a[1] + x, a[2] + rep, a[3].max(la), a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0)]);
    let du = db.post.with(owner_user).group_by(ptype_name(db)).select(owner_user_id).count_distinct();
    let mut out: Vec<Owned> = drain((&sc).and((&du).opt()))
        .into_iter()
        .map(|(k, (a, u))| Owned { name: k, n: a[0], score_sum: a[1], views_n: a[4], views_sum: a[5], rep_sum: a[2], users: u.unwrap_or(0), last_activity: a[3] })
        .collect();
    out.sort_by(|a, b| b.n.cmp(&a.n));
    out
}

// COUNT(DISTINCT p.OwnerUserId) per post type name.
fn distinct_owners(db: &'static So) -> Fold<Str, i64> {
    db.post.group_by(ptype_name(db)).select(&db.post.owner_user_id).count_distinct()
}


// SELECT pt.Name AS PostType, COUNT(p.Id) AS NumberOfPosts, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViewCount, COUNT(DISTINCT p.OwnerUserId) AS DistinctUsers
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name ORDER BY NumberOfPosts DESC;
fn q12318(db: &'static So) -> String {
    let v = drain((&stats_by_type(db)).and((&distinct_owners(db)).opt()));
    rows(v.into_iter().map(|(nm, (a, d))| row(vec![V::S(nm), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(d.unwrap_or(0))])))
}

// SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore, MAX(p.ViewCount) AS MaxViewCount, MIN(p.ViewCount) AS MinViewCount,
//        COUNT(DISTINCT p.OwnerUserId) AS AuthorCount
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name ORDER BY PostCount DESC;
fn q13618(db: &'static So) -> String {
    let g = db.post.group_by(ptype_name(db)).select((&db.post.score).and((&db.post.view_count).opt())).fold([0, 0, 0, i64::MIN, i64::MAX], |a, (s, w)| match w {
        Some(w) => [a[0] + 1, a[1] + s, a[2] + 1, a[3].max(w), a[4].min(w)],
        None => [a[0] + 1, a[1] + s, a[2], a[3], a[4]],
    });
    let v = drain((&g).and((&distinct_owners(db)).opt()));
    rows(v.into_iter().map(|(nm, (a, d))| row(vec![V::S(nm), V::I(a[0]), avg(a[1], a[0]), omax(a[3], a[2]), omax(a[4], a[2]), V::I(d.unwrap_or(0))])))
}

fn q12974(db: &'static So) -> String {
    rows(owned(db).iter().map(|t| {
        row(vec![
            V::S(t.name),
            V::I(t.n),
            avg(t.score_sum, t.n),
            nullable(t.views_sum, t.views_n),
            V::I(t.users),
        ])
    }))
}

fn q11994(db: &'static So) -> String {
    rows(owned(db).iter().map(|t| {
        row(vec![
            V::S(t.name),
            V::I(t.n),
            avg(t.score_sum, t.n),
            V::I(t.users),
            V::T(t.last_activity),
        ])
    }))
}

fn q11885(db: &'static So) -> String {
    rows(owned(db).iter().map(|t| {
        row(vec![
            V::S(t.name),
            V::I(t.n),
            V::I(t.score_sum),
            avg(t.score_sum, t.n),
            V::I(t.users),
            avg(t.rep_sum, t.n),
        ])
    }))
}

fn q10473(db: &'static So) -> String {
    rows(owned(db).iter().map(|t| {
        row(vec![
            V::S(t.name),
            avg(t.score_sum, t.n),
            avg(t.rep_sum, t.n),
            V::I(t.n),
        ])
    }))
}

fn q11662(db: &'static So) -> String {
    let Post { post_type, score, view_count, closed_date, accepted_answer_id, .. } =
        &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut sc: Vec<(Str, i64, i64, i64, i64)> = Vec::new();
    db.post
        .minus(closed_date)
        .with(accepted_answer_id)
        .group_by(post_type.select(name))
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (x, v)| {
            (n + 1, s + x, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        })
        .drive(|k, (n, s, vn, vs)| sc.push((k, n, s, vn, vs)));

    sc.sort_by(|a, b| b.1.cmp(&a.1));
    rows(sc.iter().map(|(k, n, s, vn, vs)| {
        row(vec![V::S(k), V::I(*n), avg(*vs, *vn), avg(*s, *n)])
    }))
}

fn q12039(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut all: Vec<((Str, i64), i64, i64)> = Vec::new();
    db.post
        .group_by(post_type.select(name).and(creation_date.map(trunc_month)))
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x))
        .drive(|k, (n, s)| all.push((k, n, s)));
    all.sort_by(|a, b| a.0.0.cmp(b.0.0).then(a.0.1.cmp(&b.0.1)));
    rows(all.iter().map(|((nm, m), n, s)| {
        row(vec![V::S(nm), V::T(*m), V::I(*n), avg(*s, *n)])
    }))
}

// SELECT pt.Name AS PostType, DATE_TRUNC('month', p.CreationDate) AS PostMonth, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= '2020-01-01' GROUP BY pt.Name, PostMonth ORDER BY PostMonth, pt.Name;
fn q14332(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let g = db
        .post
        .with(creation_date.ge(date(2020, 1, 1)))
        .group_by(ptype_name(db).and(creation_date.map(trunc_month)))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    rows(drain(&g).into_iter().map(|((nm, m), a)| row(vec![V::S(nm), V::T(m), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2])])))
}

const CUTOFF: fn() -> i64 = || ts(2023, 10, 1, 12, 34, 56);

fn q12837(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let User { reputation, .. } = &db.user;

    let mut all: Vec<(i64, i64, i64, i64)> = Vec::new();
    db.post
        .with(creation_date.ge(CUTOFF()))
        .with(owner_user)
        .group_by(post_type_id)
        .select(score.and(owner_user.select(reputation)))
        .fold((0i64, 0i64, 0i64), |(n, s, r), (x, rep)| (n + 1, s + x, r + rep))
        .drive(|k, (n, s, r)| all.push((k, n, s, r)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().map(|(k, n, s, r)| {
        row(vec![V::I(*k), V::I(*n), avg(*s, *n), V::I(*r)])
    }))
}

fn q11932(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut sc: Vec<(Str, i64, i64, i64, i64)> = Vec::new();
    db.post
        .with(creation_date.ge(CUTOFF()))
        .group_by(post_type.select(name))
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (x, v)| {
            (n + 1, s + x, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        })
        .drive(|k, (n, s, vn, vs)| sc.push((k, n, s, vn, vs)));

    sc.sort_by(|a, b| b.1.cmp(&a.1));
    rows(sc.iter().map(|(k, n, s, vn, vs)| {
        row(vec![V::S(k), V::I(*n), avg(*s, *n), avg(*vs, *vn)])
    }))
}

fn q15541(db: &'static So) -> String {
    let Post { view_count, answer_count, comment_count, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            oint(view_count.get(r.pid)),
            V::I(answer_count.get(r.pid).unwrap_or(0)),
            V::I(comment_count.get(r.pid).unwrap_or(0)),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("12318", q12318),
    ("12039", q12039),
    ("11662", q11662),
    ("10473", q10473),
    ("13618", q13618),
    ("12974", q12974),
    ("12837", q12837),
    ("15541", q15541),
    ("11932", q11932),
    ("11994", q11994),
    ("14332", q14332),
    ("11885", q11885),
];
