use harness::prelude::*;

// SELECT U.Id, U.DisplayName, U.Reputation, COUNT(P.Id), AVG(P.Score), COUNT(C.Id)
// FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ORDER BY U.Reputation DESC
//
// The comments join multiplies, so COUNT(P.Id) and AVG(P.Score) count a post
// once per comment. See notes/translation-failures.md 3.
fn q14731(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = owner_user.inv().select(score.and(comments_of(db).opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64),
        |(n, ss, cc), (s, c)| (n + 1, ss + s, cc + c.is_some() as i64),
    );
    let mut out = Vec::new();
    db.user.select(origid.and(display_name).and(reputation).and(&agg)).drive(
        |_, (((id, dn), rep), (n, ss, cc))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(rep),
                V::I(n),
                avg(ss, n),
                V::I(cc),
            ]))
        },
    );
    rows(out)
}

// SELECT p.Title, p.CreationDate, u.DisplayName, COUNT(a.Id)
// FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Posts a ON p.Id = a.ParentId
// WHERE p.PostTypeId = 1
// GROUP BY p.Title, p.CreationDate, u.DisplayName
// ORDER BY p.CreationDate DESC LIMIT 10
//
// LEFT JOIN Users, so the ownerless questions stay in a NULL-name group.
fn q15127(db: &'static So) -> String {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let key = title
        .opt()
        .and(creation_date)
        .and(owner_user.select(&db.user.display_name).opt());
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(children_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|((t, cd), dn), n| v.push((cd, t, dn, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, t, dn, n)| {
        row(vec![ostr(t), V::T(cd), ostr(dn), V::I(n)])
    }))
}

// SELECT p.Id, p.Title, p.Score, u.DisplayName, COUNT(v.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.PostTypeId IN (1, 2)
// GROUP BY p.Id, p.Title, p.Score, u.DisplayName
// ORDER BY p.Score DESC LIMIT 10
fn q11650(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let vc = votes_per_post(db);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.is_in([1, 2]))
        .with(owner_user)
        .select(score.and(&vc))
        .drive(|p, (s, n)| v.push((s, p, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner"]);
        f.push(V::I(n));
        row(f)
    }))
}

// (DisplayName, Title, CreationDate, Score) grouped over questions, comment
// count folded — 15547, 17365 and 16479 differ only in projection and sort.
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

// SELECT p.Title, p.CreationDate, u.DisplayName, p.Score, COUNT(c.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1
// GROUP BY p.Title, p.CreationDate, u.DisplayName, p.Score
// ORDER BY p.CreationDate DESC LIMIT 10
fn q15547(db: &'static So) -> String {
    let mut v = score_groups(db);
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|&(dn, t, cd, s, n)| {
        row(vec![ostr(t), V::T(cd), V::S(dn), V::I(s), V::I(n)])
    }))
}

// SELECT p.Title, p.Score, u.DisplayName, p.CreationDate, COUNT(c.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1
// GROUP BY p.Title, p.Score, u.DisplayName, p.CreationDate
// ORDER BY p.CreationDate DESC LIMIT 10
fn q17365(db: &'static So) -> String {
    let mut v = score_groups(db);
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|&(dn, t, cd, s, n)| {
        row(vec![ostr(t), V::I(s), V::S(dn), V::T(cd), V::I(n)])
    }))
}

// SELECT u.DisplayName, p.Title, p.CreationDate, p.Score, COUNT(c.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1
// GROUP BY u.DisplayName, p.Title, p.CreationDate, p.Score
// ORDER BY p.Score DESC, p.CreationDate DESC LIMIT 10
fn q16479(db: &'static So) -> String {
    let mut v = score_groups(db);
    v.sort_by(|a, b| (b.3, b.2).cmp(&(a.3, a.2)));
    rows(v.iter().take(10).map(|&(dn, t, cd, s, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), V::I(s), V::I(n)])
    }))
}

// SELECT p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score, COUNT(c.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score
// ORDER BY p.Score DESC LIMIT 10
fn q16691(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post.with(owner_user).select(score.and(&cc)).drive(|p, (s, c)| v.push((s, p, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.push(V::I(c));
        row(f)
    }))
}

// SELECT p.Title, p.CreationDate, u.DisplayName, COUNT(c.Id), AVG(v.BountyAmount)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY p.Title, p.CreationDate, u.DisplayName
// ORDER BY p.CreationDate DESC LIMIT 10
//
// Two LEFT JOINs off the same post cross-multiply: a post with 3 comments and
// 5 votes is 15 rows, so COUNT(c.Id) is 15 and each bounty is averaged in
// three times. `Option<Option<i64>>` is "was there a vote" then "did it carry
// a bounty".
fn q18714(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let key = title.opt().and(creation_date).and(owner_user.select(&db.user.display_name));
    let bounty = votes_of(db).select((&db.vote.bounty_amount).opt());
    let mut v = Vec::new();
    db.post
        .group_by(key)
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold((0i64, 0i64, 0i64), |(n, bs, bn), (c, b)| {
            let b = b.flatten();
            (n + c.is_some() as i64, bs + b.unwrap_or(0), bn + b.is_some() as i64)
        })
        .drive(|((t, cd), dn), (n, bs, bn)| v.push((cd, t, dn, n, bs, bn)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, t, dn, n, bs, bn)| {
        row(vec![ostr(t), V::T(cd), V::S(dn), V::I(n), avg(bs, bn)])
    }))
}

// SELECT u.DisplayName, p.Title, p.CreationDate, p.ViewCount, COUNT(c.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1
// GROUP BY u.DisplayName, p.Title, p.CreationDate, p.ViewCount
// ORDER BY p.CreationDate DESC LIMIT 10
fn q15596(db: &'static So) -> String {
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
        .drive(|(((dn, t), cd), vc), n| v.push((cd, dn, t, vc, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, dn, t, vc, n)| {
        row(vec![V::S(dn), ostr(t), V::T(cd), oint(vc), V::I(n)])
    }))
}

// SELECT p.Id, p.Title, p.CreationDate, u.DisplayName, COUNT(a.Id)
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Posts a ON p.Id = a.ParentId
// WHERE p.PostTypeId = 1
// GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY p.CreationDate DESC LIMIT 10
fn q18316(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, parent, .. } = &db.post;
    let ac = parent.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(creation_date.and(&ac))
        .drive(|p, (cd, n)| v.push((cd, p, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(V::I(n));
        row(f)
    }))
}

// SELECT u.DisplayName, COUNT(p.Id), SUM(v.BountyAmount), AVG(u.Reputation)
// FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate >= '2023-01-01'
// GROUP BY u.DisplayName, u.Id
// ORDER BY PostCount DESC
//
// u.Id is in the GROUP BY, so the group is the user; Reputation rides in the
// key because it is fixed per user and AVG over the group is just itself.
fn q19081(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let key = owner_user
        .select(display_name)
        .and(owner_user.select(origid))
        .and(owner_user.select(reputation));
    let bounty = votes_of(db).select((&db.vote.bounty_amount).opt());
    let mut out = Vec::new();
    db.post
        .with(creation_date.ge(date(2023, 1, 1)))
        .group_by(key)
        .select(bounty.opt())
        .fold((0i64, 0i64, 0i64), |(n, bs, bn), b| {
            let b = b.flatten();
            (n + 1, bs + b.unwrap_or(0), bn + b.is_some() as i64)
        })
        .drive(|((dn, _), rep), (n, bs, bn)| {
            out.push(row(vec![V::S(dn), V::I(n), nullable(bs, bn), V::F(rep as f64)]))
        });
    rows(out)
}

// SELECT u.Id, u.DisplayName, u.Reputation, COUNT(p.Id), SUM(p.Score),
//        AVG(p.ViewCount), MAX(p.CreationDate)
// FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ORDER BY TotalScore DESC, PostCount DESC
fn q13418(db: &'static So) -> String {
    let Post { score, view_count, creation_date, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = owner_user
        .inv()
        .select(score.and(view_count.opt()).and(creation_date))
        .dense_fold_outer(
            db.user.id.n,
            (0i64, 0i64, 0i64, 0i64, i64::MIN),
            |(n, ss, vn, vs, mx), ((s, v), cd)| {
                (n + 1, ss + s, vn + v.is_some() as i64, vs + v.unwrap_or(0), mx.max(cd))
            },
        );
    let mut out = Vec::new();
    db.user.select(origid.and(display_name).and(reputation).and(&agg)).drive(
        |_, (((id, dn), rep), (n, ss, vn, vs, mx))| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(rep),
                V::I(n),
                nullable(ss, n),
                avg(vs, vn),
                if n == 0 { V::Null } else { V::T(mx) },
            ]))
        },
    );
    rows(out)
}

pub static ENTRIES: &[harness::Entry] = &[
    ("11650", q11650),
    ("13418", q13418),
    ("14731", q14731),
    ("15127", q15127),
    ("15547", q15547),
    ("15596", q15596),
    ("16479", q16479),
    ("16691", q16691),
    ("17365", q17365),
    ("18316", q18316),
    ("18714", q18714),
    ("19081", q19081),
];
