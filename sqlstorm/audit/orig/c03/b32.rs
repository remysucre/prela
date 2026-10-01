use harness::prelude::*;

// ---- Users LEFT JOIN Posts, one row per user -----------------------------

struct UAgg {
    id: i64,
    display_name: Str,
    reputation: i64,
    n: i64,
    score_sum: i64,
    views_n: i64,
    views_sum: i64,
}

/// `Users LEFT JOIN Posts GROUP BY u.Id` — count, score sum, view sum. Two
/// folds because ViewCount is nullable and AVG(ViewCount) has its own
/// divisor.
fn user_aggs(db: &'static So) -> Vec<UAgg> {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let nu = db.user.id.n;

    let sc = owner_user
        .inv()
        .select(score)
        .dense_fold_outer(nu, (0i64, 0i64), |(c, s), x| (c + 1, s + x));
    let vc = owner_user
        .inv()
        .select(view_count)
        .dense_fold_outer(nu, (0i64, 0i64), |(c, s), x| (c + 1, s + x));

    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(sc).and(vc))
        .drive(
            |_, ((((id, dn), rep), (n, score_sum)), (views_n, views_sum))| {
                v.push(UAgg {
                    id,
                    display_name: dn,
                    reputation: rep,
                    n,
                    score_sum,
                    views_n,
                    views_sum,
                })
            },
        );
    v
}

// SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, AVG(P.Score) AS AveragePostScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName ORDER BY PostCount DESC;
fn q10984(db: &'static So) -> String {
    let g = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt())
        .fold([0i64; 2], |a, s| match s {
            Some(s) => [a[0] + 1, a[1] + s],
            None => a,
        });
    rows(drain(&g).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), avg(a[1], a[0])]);
        row(f)
    }))
}

fn q13818(db: &'static So) -> String {
    let v = user_aggs(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.reputation),
            V::I(r.n),
            avg(r.score_sum, r.n),
        ])
    }))
}

fn q12409(db: &'static So) -> String {
    let v = user_aggs(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.score_sum, r.n),
            nullable(r.views_sum, r.views_n),
        ])
    }))
}

fn q11529(db: &'static So) -> String {
    let v = user_aggs(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.score_sum, r.n),
            avg(r.views_sum, r.views_n),
        ])
    }))
}

// ---- ... LEFT JOIN Votes ON p.Id = v.PostId AND v.VoteTypeId IN (..) -----
//
// The second LEFT JOIN multiplies each post by its matching votes, or keeps
// it once with no vote, so COUNT(p.Id) counts `max(votes, 1)` per post.

fn q16455(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let User { display_name, .. } = &db.user;

    let per_user = owner_user
        .inv()
        .select(votes_of(db).with(vote_type_id.is_in([8i64, 9])).select(bounty_amount.opt()).opt())
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64), |(c, bn, bs), b| {
            let b = b.flatten();
            (c + 1, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        });

    let mut v: Vec<(Str, i64, i64, i64)> = Vec::new();
    db.user
        .group_by(display_name)
        .select(per_user)
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), (x, y, z)| (c + x, bn + y, bs + z))
        .drive(|dn, (c, bn, bs)| v.push((dn, c, bn, bs)));

    rows(v.iter().map(|(dn, c, bn, bs)| {
        row(vec![V::S(dn), V::I(*c), nullable(*bs, *bn)])
    }))
}

fn q16014(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let User { display_name, reputation, .. } = &db.user;

    let mut v: Vec<(Str, i64, i64, i64)> = Vec::new();
    db.post
        .with(owner_user.select(reputation).gt(1000))
        .group_by(owner_user.select(display_name))
        .select(votes_of(db).with(vote_type_id.eq(8)).select(bounty_amount.opt()).opt())
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), b| {
            let b = b.flatten();
            (c + 1, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        })
        .drive(|dn, (c, bn, bs)| v.push((dn, c, bn, bs)));

    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(dn, c, bn, bs)| {
        row(vec![V::S(dn), V::I(*c), nullable(*bs, *bn)])
    }))
}

fn q13438(db: &'static So) -> String {
    let Post { post_type, score, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut v: Vec<(Str, i64, i64, i64, i64)> = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(score.and(votes_of(db).select(&db.vote.origid).opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, ss, vc, vs), (sc, id)| {
            (n + 1, ss + sc, vc + id.is_some() as i64, vs + id.unwrap_or(0))
        })
        .drive(|k, (n, ss, vc, vs)| v.push((k, n, ss, vc, vs)));

    rows(v.iter().map(|(k, n, ss, vc, vs)| {
        row(vec![V::S(k), V::I(*n), avg(*ss, *n), nullable(*vs, *vc)])
    }))
}

fn comments_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.comment.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

/// `GROUP BY p.Title, p.CreationDate, u.DisplayName` over the questions.
fn title_date_owner_groups(db: &'static So) -> Vec<((Str, i64, Str), i64)> {
    let Post { post_type_id, title, creation_date, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let cc = comments_per_post(db);
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(title.and(creation_date).and(owner_user.select(display_name)))
        .select(cc)
        .fold(0i64, |a, x| a + x)
        .drive(|((ti, cd), dn), n| v.push(((ti, cd, dn), n)));
    v
}

fn q15008(db: &'static So) -> String {
    let mut v = title_date_owner_groups(db);
    v.sort_by(|a, b| b.0.1.cmp(&a.0.1));
    rows(v.iter().take(10).map(|((ti, cd, dn), n)| {
        row(vec![V::S(ti), V::T(*cd), V::S(dn), V::I(*n)])
    }))
}

fn q17809(db: &'static So) -> String {
    let mut v = title_date_owner_groups(db);
    v.sort_by(|a, b| b.0.1.cmp(&a.0.1));
    rows(v.iter().take(100).map(|((ti, cd, dn), n)| {
        row(vec![V::S(ti), V::T(*cd), V::S(dn), V::I(*n)])
    }))
}

fn q15073(db: &'static So) -> String {
    let Post { post_type_id, origid, title, view_count, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let cc = comments_per_post(db);
    let mut v: Vec<(Id<Post>, i64, Str, i64)> = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(origid.and(owner_user.select(display_name)).and(cc))
        .drive(|pid, ((id, dn), n)| v.push((pid, id, dn, n)));

    // DuckDB sorts NULLs first on DESC; ViewCount is never null on a question.
    let vcof = |p: Id<Post>| view_count.get(p).unwrap_or(i64::MAX);
    v.sort_by(|a, b| vcof(b.0).cmp(&vcof(a.0)));
    rows(v.iter().take(10).map(|(pid, id, dn, n)| {
        row(vec![
            V::I(*id),
            ostr(title.get(*pid)),
            oint(view_count.get(*pid)),
            V::S(dn),
            V::I(*n),
        ])
    }))
}

// ---- LEFT JOIN with no GROUP BY: the joined rows themselves --------------

// SELECT u.DisplayName, p.Title, p.CreationDate, p.Score, c.Text AS Comment FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 ORDER BY p.Score DESC LIMIT 10;
fn q17846(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let ci: HashIdx<Id<Post>, Str> = (&db.comment.post).inv().select(&db.comment.text).collect();
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(score.and((&ci).opt())));
    let v = top_n(v, |&(_, (s, _))| std::cmp::Reverse(s), 10);
    rows(v.into_iter().map(|(p, (_, c))| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "score"]);
        f.push(ostr(c));
        row(f)
    }))
}

// SELECT u.DisplayName, p.Title, p.CreationDate, p.Score, t.TagName FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Tags t ON t.ExcerptPostId = p.Id WHERE p.PostTypeId = 1 ORDER BY p.CreationDate DESC LIMIT 10;
fn q19602(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let ti_idx: HashIdx<Id<Post>, Str> = (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).collect();
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(creation_date.and((&ti_idx).opt())));
    let v = top_n(v, |&(_, (d, _))| std::cmp::Reverse(d), 10);
    rows(v.into_iter().map(|(p, (_, t))| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "score"]);
        f.push(ostr(t));
        row(f)
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("17846", q17846),
    ("19602", q19602),
    ("10984", q10984),
    ("16455", q16455),
    ("13818", q13818),
    ("16014", q16014),
    ("12409", q12409),
    ("13438", q13438),
    ("15008", q15008),
    ("15073", q15073),
    ("17809", q17809),
    ("11529", q11529),
];
