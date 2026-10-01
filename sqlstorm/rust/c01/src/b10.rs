use harness::prelude::*;

struct QRow {
    pid: Id<Post>,
    uid: Id<User>,
    score: i64,
    views: Option<i64>,
    created: i64,
    display_name: Str,
    reputation: i64,
}

fn questions(db: &'static So) -> Vec<QRow> {
    let Post { post_type_id, score, view_count, creation_date, owner_user, .. } =
        &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(
            score
                .and(view_count.opt())
                .and(creation_date)
                .and(owner_user)
                .and(owner_user.select(display_name.and(reputation))),
        )
        .drive(
            |pid,
             ((((score, views), created), uid), (display_name, reputation))| {
                v.push(QRow {
                    pid,
                    uid,
                    score,
                    views,
                    created,
                    display_name,
                    reputation,
                })
            },
        );
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

struct TRow {
    pid: Id<Post>,
    id: i64,
    created: i64,
    display_name: Str,
    type_name: Str,
}

fn typed_by_created(db: &'static So, filter: Pred) -> Vec<TRow> {
    let Post { origid, creation_date, owner_user, post_type, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let PostType { name, .. } = &db.post_type;

    let mut v: Vec<TRow> = Vec::new();
    let proj = origid
        .and(creation_date)
        .and(owner_user.select(display_name))
        .and(post_type.select(name));
    let push = |v: &mut Vec<TRow>, pid, (((id, created), display_name), type_name)| {
        v.push(TRow { pid, id, created, display_name, type_name })
    };
    match filter {
        Pred::ScorePos => db
            .post
            .with((&db.post.score).gt(0))
            .select(proj)
            .drive(|pid, r| push(&mut v, pid, r)),
        Pred::Views(n) => db
            .post
            .with((&db.post.view_count).gt(n))
            .select(proj)
            .drive(|pid, r| push(&mut v, pid, r)),
    }
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

enum Pred {
    ScorePos,
    Views(i64),
}

fn q18390(db: &'static So) -> String {
    let title = &db.post.title;
    rows(typed_by_created(db, Pred::ScorePos).iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::S(r.type_name),
        ])
    }))
}

fn q15004(db: &'static So) -> String {
    let title = &db.post.title;
    rows(typed_by_created(db, Pred::ScorePos).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::S(r.type_name),
        ])
    }))
}

fn q18848(db: &'static So) -> String {
    let title = &db.post.title;
    rows(typed_by_created(db, Pred::Views(1000)).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.type_name),
        ])
    }))
}

fn q15405(db: &'static So) -> String {
    let title = &db.post.title;
    rows(typed_by_created(db, Pred::Views(100)).iter().map(|r| {
        row(vec![
            V::I(r.id),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::S(r.type_name),
        ])
    }))
}

fn q13436(db: &'static So) -> String {
    let Post { title, answer_count, .. } = &db.post;
    let mut v = questions(db);
    v.sort_by(|a, b| b.views.cmp(&a.views));
    v.truncate(10);
    rows(v.iter().map(|r| {
        row(vec![
            ostr(title.get(r.pid)),
            oint(r.views),
            V::I(r.reputation),
            oint(answer_count.get(r.pid)),
            V::T(r.created),
        ])
    }))
}

fn q16098(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_score(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
            oint(r.views),
        ])
    }))
}

fn q18109(db: &'static So) -> String {
    let Post { title, .. } = &db.post;
    let User { origid: uid, .. } = &db.user;
    rows(by_score(db).iter().map(|r| {
        row(vec![
            V::I(uid.get(r.uid).unwrap()),
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

// SELECT pt.Name AS PostType, AVG(p.ViewCount) AS AverageViewCount, COUNT(p.Id) AS TotalPosts
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name ORDER BY AverageViewCount DESC;
fn q10162(db: &'static So) -> String {
    let g = db.post.group_by(ptype_name(db)).select((&db.post.view_count).opt()).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    rows(drain(&g).into_iter().map(|(nm, a)| row(vec![V::S(nm), avg(a[2], a[1]), V::I(a[0])])))
}

fn q16839(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
            oint(r.views),
        ])
    }))
}

fn q18536(db: &'static So) -> String {
    let title = &db.post.title;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            V::I(r.reputation),
            ostr(title.get(r.pid)),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("18390", q18390),
    ("15004", q15004),
    ("13436", q13436),
    ("18848", q18848),
    ("16098", q16098),
    ("18109", q18109),
    ("10162", q10162),
    ("15405", q15405),
    ("16839", q16839),
    ("18536", q18536),
];
