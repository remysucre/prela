use harness::prelude::*;

struct TypeAgg {
    name: Str,
    n: i64,
    score_sum: i64,
    views_n: i64,
    views_sum: i64,
    views_max: i64,
}

fn type_aggs(db: &'static So) -> Vec<TypeAgg> {
    let Post { post_type, score, view_count, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut out = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(score.and(view_count.opt()))
        .fold(
            (0i64, 0i64, 0i64, 0i64, i64::MIN),
            |(n, s, vn, vs, m), (x, v)| {
                (
                    n + 1,
                    s + x,
                    vn + v.is_some() as i64,
                    vs + v.unwrap_or(0),
                    v.map_or(m, |y| m.max(y)),
                )
            },
        )
        .drive(|k, (n, s, views_n, views_sum, views_max)| {
            out.push(TypeAgg { name: k, n, score_sum: s, views_n, views_sum, views_max })
        });
    out
}

fn avg(sum: i64, n: i64) -> V {
    if n == 0 { V::Null } else { V::F(sum as f64 / n as f64) }
}

fn nullable(sum: i64, n: i64) -> V {
    if n == 0 { V::Null } else { V::I(sum) }
}

fn by_count(db: &'static So) -> Vec<TypeAgg> {
    let mut a = type_aggs(db);
    a.sort_by(|x, y| y.n.cmp(&x.n));
    a
}

fn count_avgscore_avgviews(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
        ])
    }))
}

fn q10028(db: &'static So) -> String {
    let mut a = type_aggs(db);
    a.sort_by_key(|x| (x.views_n == 0, std::cmp::Reverse(x.views_sum)));
    rows(a.iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
        ])
    }))
}

fn q10165(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
        ])
    }))
}

fn q11886(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_max, a.views_n),
        ])
    }))
}

fn q12007(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            V::I(a.score_sum),
            avg(a.views_sum, a.views_n),
        ])
    }))
}

// SELECT U.Reputation, COUNT(P.Id) AS TotalPosts, AVG(P.Score) AS AverageScore, AVG(P.ViewCount) AS AverageViewCount
// FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Reputation ORDER BY TotalPosts DESC;
fn q10688(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let g = db.post.with(owner_user).group_by(owner_user.select(&db.user.reputation)).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    rows(drain(&g).into_iter().map(|(r, a)| row(vec![V::I(r), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2])])))
}

// SELECT u.DisplayName, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
// WHERE u.Reputation > 100 GROUP BY u.DisplayName ORDER BY PostCount DESC LIMIT 10;
fn q15445(db: &'static So) -> String {
    let Post { owner_user, view_count, .. } = &db.post;
    let g = db.post.with(owner_user.select(&db.user.reputation).gt(100)).group_by(owner_user.select(&db.user.display_name)).select(view_count.opt()).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let v = top_n(drain(&g), |&(_, a)| std::cmp::Reverse(a[0]), 10);
    rows(v.into_iter().map(|(dn, a)| row(vec![V::S(dn), V::I(a[0]), nullable(a[2], a[1])])))
}

fn q18325(db: &'static So) -> String {
    let Post { owner_user, origid, .. } = &db.post;
    let User { origid: uid, display_name, reputation, .. } = &db.user;

    let mut all: Vec<(Id<User>, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user)
        .select(origid)
        .fold(0i64, |a, _| a + 1)
        .drive(|u, n| all.push((u, n)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(u, n)| {
        row(vec![
            V::I(uid.get(*u).unwrap()),
            V::S(display_name.get(*u).unwrap()),
            V::I(*n),
            V::F(reputation.get(*u).unwrap() as f64),
        ])
    }))
}

struct QRow {
    pid: Id<Post>,
    id: i64,
    score: i64,
    views: Option<i64>,
    created: i64,
    display_name: Str,
    reputation: i64,
}

fn questions(db: &'static So) -> Vec<QRow> {
    let Post { post_type_id, origid, score, view_count, creation_date, owner_user, .. } =
        &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(
            origid
                .and(score)
                .and(view_count.opt())
                .and(creation_date)
                .and(owner_user.select(display_name.and(reputation))),
        )
        .drive(|pid, ((((id, score), views), created), (display_name, reputation))| {
            v.push(QRow { pid, id, score, views, created, display_name, reputation })
        });
    v
}

fn q17090(db: &'static So) -> String {
    let title = &db.post.title;
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    rows(v.iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            V::I(r.reputation),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

fn q18847(db: &'static So) -> String {
    let Post { title, tags_str, .. } = &db.post;
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.score),
            ostr(tags_str.get(r.pid)),
        ])
    }))
}

fn q19756(db: &'static So) -> String {
    let Post { title, tags_str, .. } = &db.post;
    let mut v = questions(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    rows(v.iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            oint(r.views),
            ostr(tags_str.get(r.pid)),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("10176", count_avgscore_avgviews),
    ("10085", count_avgscore_avgviews),
    ("17090", q17090),
    ("10688", q10688),
    ("18847", q18847),
    ("10028", q10028),
    ("10165", q10165),
    ("15445", q15445),
    ("11886", q11886),
    ("12007", q12007),
    ("18325", q18325),
    ("19756", q19756),
];
