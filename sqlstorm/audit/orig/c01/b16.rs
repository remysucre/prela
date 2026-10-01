use harness::prelude::*;

struct TRow {
    pid: Id<Post>,
    id: i64,
    created: i64,
    display_name: Str,
    type_name: Str,
}

fn typed(db: &'static So, min_score: Option<i64>, min_views: Option<i64>) -> Vec<TRow> {
    let Post { origid, creation_date, owner_user, post_type, score, view_count, .. } =
        &db.post;
    let User { display_name, .. } = &db.user;
    let PostType { name, .. } = &db.post_type;

    let proj = origid
        .and(creation_date)
        .and(owner_user.select(display_name))
        .and(post_type.select(name));

    let mut v: Vec<TRow> = Vec::new();
    let mut push = |pid, (((id, created), display_name), type_name)| {
        v.push(TRow { pid, id, created, display_name, type_name })
    };
    match (min_score, min_views) {
        (Some(s), None) => db.post.with(score.ge(s)).select(proj).drive(&mut push),
        (None, Some(n)) => db.post.with(view_count.gt(n)).select(proj).drive(&mut push),
        _ => unreachable!(),
    }
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

fn q16823(db: &'static So) -> String {
    rows(typed(db, None, Some(100)).iter().map(|r| {
        row(vec![
            title(db, r.pid),
            V::T(r.created),
            V::S(r.display_name),
            V::S(r.type_name),
        ])
    }))
}

fn q17261(db: &'static So) -> String {
    rows(typed(db, Some(0), None).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::S(r.display_name),
            V::S(r.type_name),
        ])
    }))
}

fn excerpt_tag_posts(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let Tag { excerpt_post, tag_name, .. } = &db.tag;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, i64, Str, Str)> = Vec::new();
    db.tag
        .with(excerpt_post.select(post_type_id).eq(1))
        .select(
            excerpt_post
                .and(excerpt_post.select(
                    creation_date
                        .and(score)
                        .and(view_count)
                        .and(owner_user.select(display_name)),
                ))
                .and(tag_name),
        )
        .drive(|_, ((p, (((created, sc), views), dn)), tn)| {
            v.push((p, created, sc, views, dn, tn))
        });
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(p, created, sc, views, dn, tn)| {
        row(vec![
            title(db, *p),
            V::S(dn),
            V::T(*created),
            V::I(*sc),
            V::I(*views),
            V::S(tn),
        ])
    }))
}

fn q16000(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let Tag { excerpt_post, tag_name, .. } = &db.tag;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, Str, Str)> = Vec::new();
    db.tag
        .with(excerpt_post.select(post_type_id).eq(1))
        .select(
            excerpt_post
                .and(excerpt_post.select(
                    creation_date.and(score).and(owner_user.select(display_name)),
                ))
                .and(tag_name),
        )
        .drive(|_, ((p, ((created, sc), dn)), tn)| v.push((p, created, sc, dn, tn)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(p, created, sc, dn, tn)| {
        row(vec![title(db, *p), V::I(*sc), V::S(dn), V::T(*created), V::S(tn)])
    }))
}

fn q18983(db: &'static So) -> String {
    let cc = &db.post.comment_count;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.score),
            V::I(r.views),
            V::I(cc.get(r.pid).unwrap()),
        ])
    }))
}

fn q19323(db: &'static So) -> String {
    let ac = &db.post.answer_count;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.body),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            V::I(r.views),
            oint(ac.get(r.pid)),
        ])
    }))
}

// SELECT pt.Name AS PostType, AVG(p.Score) AS AvgScore, AVG(p.ViewCount) AS AvgViewCount, AVG(p.AnswerCount) AS AvgAnswerCount
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.PostTypeId = 1 GROUP BY pt.Name ORDER BY AvgScore DESC;
fn q11453(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, answer_count, .. } = &db.post;
    let g = db.post.with(post_type_id.eq(1)).group_by(ptype_name(db)).select(score.and(view_count.opt()).and(answer_count.opt())).fold([0i64; 6], |a, ((s, w), n)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + n.is_some() as i64, a[5] + n.unwrap_or(0)]
    });
    rows(drain(&g).into_iter().map(|(k, a)| row(vec![V::S(k), avg(a[1], a[0]), avg(a[3], a[2]), avg(a[5], a[4])])))
}

fn q12137(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::I(a.type_id),
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
        ])
    }))
}

fn q11783(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            avg(a.comment_sum, a.n),
        ])
    }))
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, p.Score, p.ViewCount FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// WHERE p.PostTypeId = 1 AND p.Score > 0 ORDER BY p.CreationDate DESC LIMIT 10;
fn q15916(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(creation_date)), |&(p, d)| (std::cmp::Reverse(d), p), 10);
    rows(v.into_iter().map(|(p, _)| row(post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]))))
}

fn q10182(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let User { origid: uid, display_name, reputation, .. } = &db.user;

    let mut all: Vec<(Id<User>, i64, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user)
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x))
        .drive(|u, (n, s)| all.push((u, n, s)));
    all.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    rows(all.iter().take(10).map(|(u, n, s)| {
        row(vec![
            V::I(uid.get(*u).unwrap()),
            V::S(display_name.get(*u).unwrap()),
            V::I(reputation.get(*u).unwrap()),
            V::I(*n),
            V::I(*s),
        ])
    }))
}

fn q15013(db: &'static So) -> String {
    let Post { answer_count, comment_count, .. } = &db.post;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            V::I(r.views),
            oint(answer_count.get(r.pid)),
            V::I(comment_count.get(r.pid).unwrap()),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("16823", q16823),
    ("16000", q16000),
    ("18983", q18983),
    ("19323", q19323),
    ("11453", q11453),
    ("12137", q12137),
    ("15916", q15916),
    ("16253", excerpt_tag_posts),
    ("10182", q10182),
    ("15013", q15013),
    ("17261", q17261),
    ("11783", q11783),
];
