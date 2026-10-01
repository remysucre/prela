use harness::prelude::*;

/// One row per user, whether or not they posted — `Users LEFT JOIN Posts`
/// with the aggregate computed on the Posts side. The forward edge inverted
/// gives Id<User> -> Id<Post>; `dense_fold_outer` seeds every user, so the
/// users with no posts come back as zeroes instead of being absent.
struct Row {
    id: i64,
    display_name: Str,
    reputation: i64,
    n: i64,
    score_sum: i64,
    views_n: i64,
    views_sum: i64,
}

fn user_rows(db: &'static So) -> Vec<Row> {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let n = db.user.id.n;

    let sc = owner_user
        .inv()
        .select(score)
        .dense_fold_outer(n, (0i64, 0i64), |(c, s), x| (c + 1, s + x));
    let vc = owner_user
        .inv()
        .select(view_count)
        .dense_fold_outer(n, (0i64, 0i64), |(c, s), x| (c + 1, s + x));

    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(sc).and(vc))
        .drive(|_, ((((id, dn), rep), (n, score_sum)), (views_n, views_sum))| {
            v.push(Row { id, display_name: dn, reputation: rep, n, score_sum, views_n, views_sum })
        });
    v
}

fn q11884(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.n.cmp(&x.n));
    rows(v.iter().map(|r| {
        row(vec![V::I(r.id), V::S(r.display_name), V::I(r.reputation), V::I(r.n)])
    }))
}

fn id_name_count_avgscore(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.n.cmp(&x.n));
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.score_sum, r.n),
        ])
    }))
}

fn q10571(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.reputation.cmp(&x.reputation));
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::I(r.reputation),
            V::I(r.n),
            avg(r.views_sum, r.views_n),
        ])
    }))
}

fn by_rep(db: &'static So, take: usize) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.reputation.cmp(&x.reputation));
    rows(v.iter().take(take).map(|r| {
        row(vec![V::I(r.id), V::S(r.display_name), V::I(r.reputation), V::I(r.n)])
    }))
}

fn q10801(db: &'static So) -> String {
    by_rep(db, 10)
}

fn q10133(db: &'static So) -> String {
    by_rep(db, 100)
}

fn q10131(db: &'static So) -> String {
    let mut v = user_rows(db);
    v.sort_by(|x, y| y.n.cmp(&x.n));
    rows(v.iter().take(100).map(|r| {
        row(vec![V::I(r.id), V::S(r.display_name), V::I(r.reputation), V::I(r.n)])
    }))
}

/// `Users LEFT JOIN Posts GROUP BY u.DisplayName`: [posts, score sum].
fn name_groups(db: &'static So) -> Vec<(Str, [i64; 2])> {
    let g = db.user.group_by(&db.user.display_name).select(posts_of(db).select(&db.post.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    drain(&g)
}

// SELECT Users.DisplayName, COUNT(Posts.Id) AS PostCount, SUM(Posts.Score) AS TotalScore FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY Users.DisplayName ORDER BY PostCount DESC LIMIT 10;
fn q18645(db: &'static So) -> String {
    let v = top_n(name_groups(db), |&(_, a)| std::cmp::Reverse(a[0]), 10);
    rows(v.into_iter().map(|(dn, a)| row(vec![V::S(dn), V::I(a[0]), nullable(a[1], a[0])])))
}

// SELECT u.DisplayName, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AveragePostScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.DisplayName ORDER BY PostCount DESC, AveragePostScore DESC LIMIT 100;
fn q11283(db: &'static So) -> String {
    let avg_key = |a: [i64; 2]| if a[0] == 0 { None } else { Some(fkey(a[1] as f64 / a[0] as f64)) };
    let v = top_n(name_groups(db), |&(_, a)| (std::cmp::Reverse(a[0]), avg_key(a).is_none(), std::cmp::Reverse(avg_key(a))), 100);
    rows(v.into_iter().map(|(dn, a)| row(vec![V::S(dn), V::I(a[0]), avg(a[1], a[0])])))
}

fn comments_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.comment.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

// SELECT p.Title, u.DisplayName, COUNT(c.Id) AS CommentCount FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1 GROUP BY p.Title, u.DisplayName ORDER BY CommentCount DESC, p.Title, u.DisplayName LIMIT 10;
// (rewrites/16616.sql: the tiebreak on Title, DisplayName is added)
fn q16616(db: &'static So) -> String {
    let Post { post_type_id, title: pt, owner_user, .. } = &db.post;
    let g = db
        .post
        .with(post_type_id.eq(1))
        .group_by(pt.opt().and(owner_user.select(&db.user.display_name)))
        .select(comments_per_post(db))
        .fold(0i64, |a, x| a + x);
    let v = top_n(drain(&g), |&((ti, dn), n)| (std::cmp::Reverse(n), ti.is_none(), ti, dn), 10);
    rows(v.into_iter().map(|((ti, dn), n)| row(vec![ostr(ti), V::S(dn), V::I(n)])))
}

/// `Users JOIN Posts LEFT JOIN Comments GROUP BY u.DisplayName, p.Title, p.CreationDate`.
fn post_comment_groups(db: &'static So) -> Vec<((Str, Option<Str>, i64), i64)> {
    let Post { title: pt, creation_date, owner_user, .. } = &db.post;
    let g = db
        .post
        .group_by(owner_user.select(&db.user.display_name).and(pt.opt()).and(creation_date))
        .select(comments_per_post(db))
        .fold(0i64, |a, x| a + x);
    drain(&g).into_iter().map(|(((dn, ti), cd), n)| ((dn, ti, cd), n)).collect()
}

// SELECT u.DisplayName, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY u.DisplayName, p.Title, p.CreationDate ORDER BY COUNT(c.Id) DESC, u.DisplayName, p.Title, p.CreationDate LIMIT 10;
// (rewrites/18612.sql: the tiebreak after COUNT(c.Id) is added)
fn q18612(db: &'static So) -> String {
    let v = top_n(post_comment_groups(db), |&((dn, ti, cd), n)| (std::cmp::Reverse(n), dn, ti.is_none(), ti, cd), 10);
    rows(v.into_iter().map(|((dn, ti, cd), n)| row(vec![V::S(dn), ostr(ti), V::T(cd), V::I(n)])))
}

// SELECT p.Title, p.CreationDate, u.DisplayName, COUNT(c.Id) AS CommentCount FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY p.Title, p.CreationDate, u.DisplayName ORDER BY p.CreationDate DESC LIMIT 10;
fn q15616(db: &'static So) -> String {
    let v = top_n(post_comment_groups(db), |&((_, _, cd), _)| std::cmp::Reverse(cd), 10);
    rows(v.into_iter().map(|((dn, ti, cd), n)| row(vec![ostr(ti), V::T(cd), V::S(dn), V::I(n)])))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("11884", q11884),
    ("14852", id_name_count_avgscore),
    ("10459", id_name_count_avgscore),
    ("18645", q18645),
    ("16616", q16616),
    ("10801", q10801),
    ("18612", q18612),
    ("10571", q10571),
    ("10133", q10133),
    ("15616", q15616),
    ("10131", q10131),
    ("11283", q11283),
];
