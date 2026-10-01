use harness::prelude::*;

struct QRow {
    pid: Id<Post>,
    score: i64,
    views: Option<i64>,
    created: i64,
    body: Str,
    display_name: Str,
}

fn questions(db: &'static So) -> Vec<QRow> {
    let Post { post_type_id, score, view_count, creation_date, body, owner_user, .. } =
        &db.post;
    let User { display_name, .. } = &db.user;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(
            score
                .and(view_count.opt())
                .and(creation_date)
                .and(body)
                .and(owner_user.select(display_name)),
        )
        .drive(|pid, ((((score, views), created), body), display_name)| {
            v.push(QRow { pid, score, views, created, body, display_name })
        });
    v
}

fn by_created(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

fn by_score(db: &'static So) -> Vec<QRow> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    v
}

fn q15242(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::S(r.body),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

// SELECT CAST(CreationDate AS DATE) AS PostDate, COUNT(*) AS TotalPosts, AVG(Score) AS AverageScore, AVG(ViewCount) AS AverageViewCount
// FROM Posts GROUP BY CAST(CreationDate AS DATE) ORDER BY PostDate;
fn q13131(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let g = db.post.group_by(creation_date.map(trunc_day)).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    rows(drain(&g).into_iter().map(|(d, a)| row(vec![V::D(d), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2])])))
}

fn q16233(db: &'static So) -> String {
    let Post { title, answer_count, .. } = &db.post;
    rows(by_score(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            oint(r.views),
            oint(answer_count.get(r.pid)),
            V::I(r.score),
        ])
    }))
}

struct TRow {
    pid: Id<Post>,
    id: i64,
    created: i64,
    display_name: Str,
    type_name: Str,
}

fn since_2023(db: &'static So) -> Vec<TRow> {
    let Post { origid, creation_date, owner_user, post_type, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let PostType { name, .. } = &db.post_type;

    let mut v: Vec<TRow> = Vec::new();
    db.post
        .with(creation_date.ge(date(2023, 1, 1)))
        .select(
            origid
                .and(creation_date)
                .and(owner_user.select(display_name))
                .and(post_type.select(name)),
        )
        .drive(|pid, (((id, created), display_name), type_name)| {
            v.push(TRow { pid, id, created, display_name, type_name })
        });
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

fn q15039(db: &'static So) -> String {
    let title = &db.post.title;
    rows(since_2023(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::S(r.type_name),
        ])
    }))
}

fn q16649(db: &'static So) -> String {
    let title = &db.post.title;
    rows(since_2023(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::S(r.type_name),
        ])
    }))
}

fn q17151(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut all: Vec<(Str, i64, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user.select(display_name))
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x))
        .drive(|k, (n, s)| all.push((k, n, s)));
    all.sort_by(|a, b| b.2.cmp(&a.2));
    rows(all.iter().take(10).map(|(dn, n, s)| {
        row(vec![V::S(dn), V::I(*n), V::I(*s)])
    }))
}

fn q15585(db: &'static So) -> String {
    let Post { title, answer_count, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            oint(r.views),
            oint(answer_count.get(r.pid)),
        ])
    }))
}

fn q15060(db: &'static So) -> String {
    let Post { title, answer_count, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.score),
            oint(r.views),
            oint(answer_count.get(r.pid)),
        ])
    }))
}

fn q15263(db: &'static So) -> String {
    let Post { title, answer_count, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.score),
            oint(answer_count.get(r.pid)),
        ])
    }))
}

fn type_aggs(db: &'static So) -> Vec<(Str, i64, i64, i64, i64)> {
    let Post { post_type, score, view_count, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut out = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (x, v)| {
            (n + 1, s + x, vn + v.is_some() as i64, vs + v.unwrap_or(0))
        })
        .drive(|k, (n, s, vn, vs)| out.push((k, n, s, vn, vs)));
    out
}

fn q12979(db: &'static So) -> String {
    let mut a = type_aggs(db);
    a.sort_by_key(|r| r.0);
    rows(a.iter().map(|(k, n, s, vn, vs)| {
        let total = if *vn == 0 { V::Null } else { V::I(*vs) };
        row(vec![V::S(k), V::F(*s as f64 / *n as f64), total, V::I(*n)])
    }))
}

fn q10018(db: &'static So) -> String {
    let mut a = type_aggs(db);
    a.sort_by(|x, y| y.1.cmp(&x.1));
    rows(a.iter().map(|(k, n, s, vn, vs)| {
        let total = if *vn == 0 { V::Null } else { V::I(*vs) };
        row(vec![V::S(k), V::I(*n), V::F(*s as f64 / *n as f64), total])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("15242", q15242),
    ("13131", q13131),
    ("16233", q16233),
    ("15039", q15039),
    ("17151", q17151),
    ("15585", q15585),
    ("12979", q12979),
    ("15060", q15060),
    ("16649", q16649),
    ("15263", q15263),
    ("10018", q10018),
];
