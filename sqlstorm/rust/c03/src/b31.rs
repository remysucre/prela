use harness::prelude::*;

struct Row {
    id: i64,
    display_name: Str,
    reputation: i64,
    n: i64,
    score_sum: i64,
}

/// `Users LEFT JOIN Posts` — every user, posting or not.
fn user_rows(db: &'static So) -> Vec<Row> {
    let Post { owner_user, score, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;

    let sc = owner_user
        .inv()
        .select(score)
        .dense_fold_outer(db.user.id.n, (0i64, 0i64), |(c, s), x| (c + 1, s + x));

    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(sc))
        .drive(|_, (((id, dn), rep), (n, score_sum))| {
            v.push(Row { id, display_name: dn, reputation: rep, n, score_sum })
        });
    v
}

fn avg_score(r: &Row) -> f64 {
    if r.n == 0 { f64::NEG_INFINITY } else { r.score_sum as f64 / r.n as f64 }
}

fn q12530(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.n.cmp(&x.n).then(y.reputation.cmp(&x.reputation)));
    rows(v.iter().map(|r| {
        row(vec![V::I(r.id), V::S(r.display_name), V::I(r.reputation), V::I(r.n)])
    }))
}

fn count_then_avgscore(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| {
        y.n.cmp(&x.n).then(avg_score(y).partial_cmp(&avg_score(x)).unwrap())
    });
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.score_sum, r.n),
        ])
    }))
}

fn q12272(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.reputation.cmp(&x.reputation).then(y.n.cmp(&x.n)));
    rows(v.iter().map(|r| {
        row(vec![V::I(r.id), V::I(r.reputation), V::I(r.n), avg(r.score_sum, r.n)])
    }))
}

fn q10035(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.n.cmp(&x.n));
    rows(v.iter().take(100).map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            V::F(r.reputation as f64),
        ])
    }))
}

fn comments_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.comment.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

struct PostRow {
    pid: Id<Post>,
    id: i64,
    created: i64,
    display_name: Str,
    comments: i64,
}

fn posts_with_comments(db: &'static So) -> Vec<PostRow> {
    let Post { origid, creation_date, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post
        .with(owner_user)
        .select(origid.and(creation_date).and(owner_user.select(display_name)).and(cc))
        .drive(|pid, (((id, created), display_name), comments)| v.push(PostRow { pid, id, created, display_name, comments }));
    v
}

// SELECT u.DisplayName, p.Title, p.Score, COUNT(c.Id) AS CommentCount FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1 GROUP BY u.DisplayName, p.Title, p.Score ORDER BY p.Score DESC LIMIT 10;
fn q17350(db: &'static So) -> String {
    let Post { post_type_id, title: pt, score, owner_user, .. } = &db.post;
    let g = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user.select(&db.user.display_name).and(pt.opt()).and(score))
        .select(comments_per_post(db))
        .fold(0i64, |a, x| a + x);
    let v = top_n(drain(&g), |&((_, s), _)| std::cmp::Reverse(s), 10);
    rows(v.into_iter().map(|(((dn, ti), s), n)| row(vec![V::S(dn), ostr(ti), V::I(s), V::I(n)])))
}

fn q15673(db: &'static So) -> String {
    let pt = &db.post.title;
    let mut v = posts_with_comments(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    rows(v.iter().take(10).map(|r| {
        row(vec![
            V::I(r.id),
            ostr(pt.get(r.pid)),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.comments),
        ])
    }))
}

// SELECT p.Title, p.ViewCount, u.DisplayName, COUNT(c.Id) AS CommentCount FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1 GROUP BY p.Title, p.ViewCount, u.DisplayName ORDER BY p.ViewCount DESC LIMIT 10;
fn q17316(db: &'static So) -> String {
    let Post { post_type_id, view_count, title: pt, owner_user, .. } = &db.post;
    let g = db
        .post
        .with(post_type_id.eq(1))
        .group_by(pt.opt().and(view_count.opt()).and(owner_user.select(&db.user.display_name)))
        .select(comments_per_post(db))
        .fold(0i64, |a, x| a + x);
    let v = top_n(drain(&g), |&(((_, vc), _), _)| (vc.is_none(), std::cmp::Reverse(vc)), 10);
    rows(v.into_iter().map(|(((ti, vc), dn), n)| row(vec![ostr(ti), oint(vc), V::S(dn), V::I(n)])))
}

/// Questions JOIN Users LEFT JOIN Comments `GROUP BY u.DisplayName, p.Title, p.CreationDate`.
fn question_comment_groups(db: &'static So) -> Vec<((Str, Option<Str>, i64), i64)> {
    let Post { post_type_id, title: pt, creation_date, owner_user, .. } = &db.post;
    let g = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user.select(&db.user.display_name).and(pt.opt()).and(creation_date))
        .select(comments_per_post(db))
        .fold(0i64, |a, x| a + x);
    drain(&g).into_iter().map(|(((dn, ti), cd), n)| ((dn, ti, cd), n)).collect()
}

// SELECT u.DisplayName, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1 GROUP BY u.DisplayName, p.Title, p.CreationDate ORDER BY CommentCount DESC;
fn q17405(db: &'static So) -> String {
    rows(question_comment_groups(db).into_iter().map(|((dn, ti, cd), n)| row(vec![V::S(dn), ostr(ti), V::T(cd), V::I(n)])))
}

// SELECT p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1 GROUP BY p.Title, p.CreationDate, u.DisplayName ORDER BY p.CreationDate DESC;
fn q16304(db: &'static So) -> String {
    rows(question_comment_groups(db).into_iter().map(|((dn, ti, cd), n)| row(vec![ostr(ti), V::T(cd), V::S(dn), V::I(n)])))
}

fn q19392(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let Vote { bounty_amount, .. } = &db.vote;
    let User { display_name, reputation, .. } = &db.user;

    let mut v: Vec<(Str, i64, i64, i64)> = Vec::new();
    db.post
        .with(owner_user.select(reputation).gt(1000))
        .group_by(owner_user.select(display_name))
        .select(votes_of(db).select(bounty_amount.opt()).opt())
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), b| {
            let b = b.flatten();
            (c + 1, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        })
        .drive(|dn, (c, bn, bs)| v.push((dn, c, bn, bs)));

    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(dn, c, n, s)| {
        row(vec![V::S(dn), V::I(*c), nullable(*s, *n)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("17350", q17350),
    ("12530", q12530),
    ("19392", q19392),
    ("12426", count_then_avgscore),
    ("10035", q10035),
    ("12272", q12272),
    ("13997", count_then_avgscore),
    ("15673", q15673),
    ("17405", q17405),
    ("17316", q17316),
    ("16304", q16304),
];
