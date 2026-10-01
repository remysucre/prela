use harness::prelude::*;

// (DisplayName, Title, CreationDate, Score) grouped over questions, comment
// count folded. 16084, 15670, 15161, 15939 and 16686 are this query with the
// columns permuted and a different sort.
fn score_groups(db: &'static So) -> Vec<(Str, Option<Str>, i64, i64, i64)> {
    let Post { post_type_id, title, creation_date, score, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(score);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|(((dn, t), cd), s), n| v.push((dn, t, cd, s, n)));
    v
}

fn by_score(db: &'static So) -> Vec<(Str, Option<Str>, i64, i64, i64)> {
    let mut v = score_groups(db);
    v.sort_by(|a, b| b.3.cmp(&a.3));
    v
}

fn by_date(db: &'static So) -> Vec<(Str, Option<Str>, i64, i64, i64)> {
    let mut v = score_groups(db);
    v.sort_by(|a, b| b.2.cmp(&a.2));
    v
}

// SELECT p.Title, p.CreationDate, u.DisplayName, p.Score, COUNT(c.Id)
// FROM Posts p JOIN Users u ... LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1
// GROUP BY p.Title, p.CreationDate, u.DisplayName, p.Score
// ORDER BY p.Score DESC LIMIT 10
fn q16084(db: &'static So) -> String {
    rows(by_score(db).iter().take(10).map(|&(dn, t, cd, s, n)| {
        row(vec![ostr(t), V::T(cd), V::S(dn), V::I(s), V::I(n)])
    }))
}

// Same group, ORDER BY p.CreationDate DESC LIMIT 10,
// projected Title, CreationDate, Score, OwnerDisplayName, CommentCount.
fn q15670(db: &'static So) -> String {
    rows(by_date(db).iter().take(10).map(|&(dn, t, cd, s, n)| {
        row(vec![ostr(t), V::T(cd), V::I(s), V::S(dn), V::I(n)])
    }))
}

// Same group, ORDER BY p.CreationDate DESC LIMIT 10,
// projected Title, OwnerDisplayName, CreationDate, Score, CommentCount.
fn q15161(db: &'static So) -> String {
    rows(by_date(db).iter().take(10).map(|&(dn, t, cd, s, n)| {
        row(vec![ostr(t), V::S(dn), V::T(cd), V::I(s), V::I(n)])
    }))
}

// Same group, ORDER BY p.Score DESC LIMIT 10,
// projected Title, Score, OwnerDisplayName, CreationDate, CommentCount.
fn q15939(db: &'static So) -> String {
    rows(by_score(db).iter().take(10).map(|&(dn, t, cd, s, n)| {
        row(vec![ostr(t), V::I(s), V::S(dn), V::T(cd), V::I(n)])
    }))
}

// Same group, ORDER BY p.Score DESC LIMIT 10,
// projected UserName, PostTitle, Score, CreationDate, CommentCount.
fn q16686(db: &'static So) -> String {
    rows(by_score(db).iter().take(10).map(|&(dn, t, cd, s, n)| {
        row(vec![V::S(dn), ostr(t), V::I(s), V::T(cd), V::I(n)])
    }))
}

// SELECT p.Id, p.Title, p.CreationDate, u.DisplayName, COUNT(c.Id)
// FROM Posts p JOIN Users u ... LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1
// GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY p.CreationDate DESC LIMIT 100
fn q17741(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(creation_date.and(&cc))
        .drive(|p, (cd, c)| v.push((cd, p, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(100).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(V::I(c));
        row(f)
    }))
}

// SELECT p.Title, u.DisplayName, p.CreationDate, p.ViewCount, COUNT(a.Id)
// FROM Posts p JOIN Users u ... LEFT JOIN Posts a ON p.Id = a.ParentId
// WHERE p.PostTypeId = 1
// GROUP BY p.Title, u.DisplayName, p.CreationDate, p.ViewCount
// ORDER BY p.CreationDate DESC LIMIT 10
fn q15154(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, view_count, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(view_count.opt());
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(children_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|(((dn, t), cd), vc), n| v.push((cd, dn, t, vc, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, dn, t, vc, n)| {
        row(vec![ostr(t), V::S(dn), V::T(cd), oint(vc), V::I(n)])
    }))
}

// Comment count per post over every post type, newest first — 18254, 19752 and
// 18266 differ only in which columns they project.
fn newest_posts(db: &'static So) -> Vec<(i64, Id<Post>, i64)> {
    let Post { creation_date, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post.with(owner_user).select(creation_date.and(&cc)).drive(|p, (cd, c)| v.push((cd, p, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v
}

// SELECT p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score, COUNT(c.Id)
// ... GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score
// ORDER BY p.CreationDate DESC LIMIT 10
fn q18254(db: &'static So) -> String {
    rows(newest_posts(db).iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.push(V::I(c));
        row(f)
    }))
}

// Same, projected Id, Title, OwnerDisplayName, Score, CreationDate, CommentCount.
fn q19752(db: &'static So) -> String {
    rows(newest_posts(db).iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "created"]);
        f.push(V::I(c));
        row(f)
    }))
}

// Same, projected Id, Title, CreationDate, ViewCount, OwnerName, CommentCount.
fn q18266(db: &'static So) -> String {
    rows(newest_posts(db).iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.push(V::I(c));
        row(f)
    }))
}

// SELECT pt.Name, COUNT(p.Id), AVG(p.Score), COUNT(c.Id), COUNT(v.Id)
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY pt.Name ORDER BY PostCount DESC
//
// Comments and Votes both hang off the post, so the two LEFT JOINs cross:
// every count here is over comments x votes, not over posts.
fn q11542(db: &'static So) -> String {
    let Post { post_type, score, .. } = &db.post;
    let mut out = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(comments_of(db).opt()).and(votes_of(db).opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, ss, cc, vv), ((s, c), v)| {
            (n + 1, ss + s, cc + c.is_some() as i64, vv + v.is_some() as i64)
        })
        .drive(|k, (n, ss, cc, vv)| {
            out.push(row(vec![V::S(k), V::I(n), avg(ss, n), V::I(cc), V::I(vv)]))
        });
    rows(out)
}

// SELECT u.DisplayName, p.Title, p.CreationDate, v.VoteTypeId, COUNT(v.Id)
// FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.DisplayName, p.Title, p.CreationDate, v.VoteTypeId
// ORDER BY VoteCount DESC LIMIT 10
//
// The group key reaches into the outer-joined side, so it is many-valued: a
// post with upvotes and downvotes lands in two groups, and one with no votes
// in a NULL-VoteTypeId group whose COUNT(v.Id) is 0, not 1.
fn q19286(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(votes_of(db).select(&db.vote.vote_type_id).opt());
    let mut v = Vec::new();
    db.post
        .with(owner_user)
        .group_by(key)
        .fold(0i64, |a, _| a + 1)
        .drive(|(((dn, t), cd), vt), n| v.push((if vt.is_some() { n } else { 0 }, dn, t, cd, vt)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(n, dn, t, cd, vt)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), oint(vt), V::I(n)])
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("11542", q11542),
    ("15154", q15154),
    ("15161", q15161),
    ("15670", q15670),
    ("15939", q15939),
    ("16084", q16084),
    ("16686", q16686),
    ("17741", q17741),
    ("18254", q18254),
    ("18266", q18266),
    ("19286", q19286),
    ("19752", q19752),
];
