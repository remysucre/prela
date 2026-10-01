use harness::prelude::*;

// SELECT u.Id, u.DisplayName, COUNT(p.Id), AVG(p.Score), u.Reputation,
//        MAX(p.LastActivityDate)
// FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ORDER BY TotalPosts DESC, AveragePostScore DESC
fn q14154(db: &'static So) -> String {
    let Post { score, last_activity_date, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = owner_user.inv().select(score.and(last_activity_date)).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, i64::MIN),
        |(n, ss, mx), (s, la)| (n + 1, ss + s, mx.max(la)),
    );
    let mut out = Vec::new();
    db.user.select(origid.and(display_name).and(reputation).and(&agg)).drive(
        |_, (((id, dn), rep), (n, ss, mx))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                avg(ss, n),
                V::I(rep),
                if n == 0 { V::Null } else { V::T(mx) },
            ]))
        },
    );
    rows(out)
}

// SELECT U.DisplayName, P.Title, P.CreationDate, P.Score, P.ViewCount,
//        C.Text, C.CreationDate
// FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN Comments C ON P.Id = C.PostId
// WHERE P.PostTypeId = 1
// ORDER BY P.Score DESC, P.CreationDate DESC LIMIT 10
fn q15237(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(score.and(creation_date).and(comments_of(db).opt()))
        .drive(|p, ((s, cd), c)| v.push((s, cd, p, c)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().take(10).map(|&(_, _, p, c)| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "score", "views"]);
        f.extend([
            ostr(c.map(|c| db.comment.text.get(c).unwrap())),
            ots(c.map(|c| db.comment.creation_date.get(c).unwrap())),
        ]);
        row(f)
    }))
}

// SELECT pt.Name, COUNT(p.Id), AVG(p.Score), AVG(p.ViewCount), AVG(u.Reputation)
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// GROUP BY pt.Name ORDER BY TotalPosts DESC
fn q12497(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            avg(a.rep_sum, a.rep_n),
        ])
    }))
}

// SELECT p.Title, p.CreationDate, u.DisplayName, COUNT(ans.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Posts ans ON ans.ParentId = p.Id
// WHERE p.PostTypeId = 1
// GROUP BY p.Title, p.CreationDate, u.DisplayName
// ORDER BY p.CreationDate DESC LIMIT 10
fn q18283(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = title.opt().and(creation_date).and(owner_user.select(&db.user.display_name));
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(children_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((t, cd), dn), n| v.push((cd, t, dn, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, t, dn, n)| {
        row(vec![ostr(t), V::T(cd), V::S(dn), V::I(n)])
    }))
}

// SELECT p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, COUNT(c.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score
// ORDER BY p.CreationDate DESC LIMIT 10
fn q17589(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post.with(owner_user).select(creation_date.and(&cc)).drive(|p, (cd, c)| v.push((cd, p, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score"]);
        f.push(V::I(c));
        row(f)
    }))
}

// SELECT p.Title, u.DisplayName, p.CreationDate, p.Score, COUNT(c.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1
// GROUP BY p.Title, u.DisplayName, p.CreationDate, p.Score
// ORDER BY p.Score DESC LIMIT 10
fn q16955(db: &'static So) -> String {
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
        .drive(|(((dn, t), cd), s), n| v.push((s, dn, t, cd, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(s, dn, t, cd, n)| {
        row(vec![ostr(t), V::S(dn), V::T(cd), V::I(s), V::I(n)])
    }))
}

// SELECT U.DisplayName, P.Title, P.CreationDate, COUNT(C.Id)
// FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN Comments C ON P.Id = C.PostId
// WHERE P.PostTypeId = 1
// GROUP BY U.DisplayName, P.Title, P.CreationDate
// ORDER BY PostDate DESC LIMIT 10
fn q15253(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((dn, t), cd), n| v.push((cd, dn, t, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, dn, t, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), V::I(n)])
    }))
}

// (DisplayName, Title, CreationDate, ViewCount) grouped, comment count folded,
// newest first — 15097 and 15877 differ only in projection and LIMIT.
fn views_groups(db: &'static So) -> Vec<(Str, Option<Str>, i64, Option<i64>, i64)> {
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
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|(((dn, t), cd), vc), n| v.push((dn, t, cd, vc, n)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    v
}

// SELECT p.Title, p.CreationDate, p.ViewCount, u.DisplayName, COUNT(c.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1
// GROUP BY p.Title, p.CreationDate, p.ViewCount, u.DisplayName
// ORDER BY p.CreationDate DESC LIMIT 10
fn q15097(db: &'static So) -> String {
    rows(views_groups(db).iter().take(10).map(|&(dn, t, cd, vc, n)| {
        row(vec![ostr(t), V::T(cd), oint(vc), V::S(dn), V::I(n)])
    }))
}

// SELECT U.DisplayName, P.Title, P.CreationDate, P.ViewCount, COUNT(C.Id)
// FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN Comments C ON P.Id = C.PostId
// WHERE P.PostTypeId = 1
// GROUP BY U.DisplayName, P.Title, P.CreationDate, P.ViewCount
// ORDER BY P.CreationDate DESC LIMIT 100
fn q15877(db: &'static So) -> String {
    rows(views_groups(db).iter().take(100).map(|&(dn, t, cd, vc, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), oint(vc), V::I(n)])
    }))
}

// SELECT p.Id, p.Title, p.CreationDate, u.DisplayName, vt.Name
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// WHERE p.PostTypeId = 1
// ORDER BY p.CreationDate DESC LIMIT 10
//
// No aggregate: a post with five votes is five rows.
fn q15729(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let name = (&db.vote.vote_type).select(&db.vote_type.name);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(creation_date.and(votes_of(db).select(name.opt()).opt()))
        .drive(|p, (cd, vt)| v.push((cd, p, vt.flatten())));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, vt)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(ostr(vt));
        row(f)
    }))
}

// SELECT u.Id, u.DisplayName, p.Title, p.CreationDate, COUNT(c.Id)
// FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1
// GROUP BY u.Id, u.DisplayName, p.Title, p.CreationDate
// ORDER BY p.CreationDate DESC LIMIT 10
fn q16578(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.origid)
        .and(owner_user.select(&db.user.display_name))
        .and(title.opt())
        .and(creation_date);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|(((id, dn), t), cd), n| v.push((cd, id, dn, t, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, id, dn, t, n)| {
        row(vec![V::I(id), V::S(dn), ostr(t), V::T(cd), V::I(n)])
    }))
}

// SELECT p.Id, p.Title, u.DisplayName, p.CreationDate, COUNT(c.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1
// GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate
// ORDER BY p.CreationDate DESC
fn q17505(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let mut out = Vec::new();
    db.post.with(post_type_id.eq(1)).with(owner_user).select(&cc).drive(|p, c| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.push(V::I(c));
        out.push(row(f));
    });
    rows(out)
}

pub static ENTRIES: &[harness::Entry] = &[
    ("12497", q12497),
    ("14154", q14154),
    ("15097", q15097),
    ("15237", q15237),
    ("15253", q15253),
    ("15729", q15729),
    ("15877", q15877),
    ("16578", q16578),
    ("16955", q16955),
    ("17505", q17505),
    ("17589", q17589),
    ("18283", q18283),
];
