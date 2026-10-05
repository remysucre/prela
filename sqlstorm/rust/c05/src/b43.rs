use harness::prelude::*;

fn q13403(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let t = owner_user
        .inv()
        .select(post_type_id.and(view_count.opt()).and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .dense_fold_outer(
            db.user.id.n,
            (0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
            |(n, q, a, u, d, vn, vs, s), (((ty, w), sc), vt)| {
                (
                    n + 1,
                    q + (ty == 1) as i64,
                    a + (ty == 2) as i64,
                    u + (vt == Some(2)) as i64,
                    d + (vt == Some(3)) as i64,
                    vn + w.is_some() as i64,
                    vs + w.unwrap_or(0),
                    s + sc,
                )
            },
        );
    let rk = whole(&db.user.id).select((&db.user.display_name).and(&t)).window(rank, |(_, a)| a.0, desc);
    let mut v = Vec::new();
    (&rk).drive(|_, ((dn, a), r)| v.push((dn, a, r)));
    let avg_of = |a: &(i64, i64, i64, i64, i64, i64, i64, i64)| (a.0 > 0).then(|| a.7 as f64 / a.0 as f64);
    v.sort_by(|x, y| {
        y.1.0.cmp(&x.1.0).then(match (avg_of(&x.1), avg_of(&y.1)) {
            (Some(a), Some(b)) => b.partial_cmp(&a).unwrap(),
            (a, b) => b.is_some().cmp(&a.is_some()),
        })
    });
    rows(v.iter().take(10).map(|&(dn, (n, q, a, u, d, vn, vs, s), r)| {
        row(vec![V::S(dn), V::I(n), V::I(q), V::I(a), V::I(u), V::I(d), nullable(vs, vn), avg(s, n), V::I(r)])
    }))
}

fn answers_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.post.parent).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

fn q12213(db: &'static So) -> String {
    let Post { view_count, score, post_type_id, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let cc = comments_per_post(db);
    let ac = answers_per_post(db);
    let base = owned(db).with(post_type_id.eq(1));
    let ranked = whole(&base)
        .select(Ident::<Post>::new().and(view_count.opt()).and(score).and(&up).and(&down).and(&cc).and(&ac))
        .window(dense_rank, |((((((_, w), _), _), _), _), _)| w, desc)
        .window(dense_rank, |(((((((_, _), s), _), _), _), _), _)| s, desc);
    let mut v = Vec::new();
    ranked.drive(|_, ((((((((p, w), _), u), d), c), a), x), y)| v.push((p, (w, u, d, c, a, x, y))));
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    rows(v.iter().take(100).map(|&(p, (_, u, d, c, a, x, y))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(u), V::I(d), V::I(c), V::I(a), V::I(x), V::I(y)]);
        row(f)
    }))
}

fn q5989(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, creation_date, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let counts = db.post.with(post_type_id.eq(1)).group_by(tags_str.opt()).fold(0i64, |a, _| a + 1);
    let top: MatSet<Option<Str>> = whole(&counts)
        .select(Same::new().and(&counts))
        .window(
            row_number,
            |(t, n)| (n, t),
            |x: &(i64, Option<Str>), y: &(i64, Option<Str>)| y.0.cmp(&x.0).then(x.1.cmp(&y.1)),
        )
        .filt(|((t, _), r)| r <= 10 && t.is_some())
        .map(|((t, _), _)| t)
        .collect();
    let base = owned(db).with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n == 1)
        .and(&top)
        .map(|((((p, _), _), _), _)| p)
        .select(Ident::<Post>::new().and(&cc).and(&up))
        .drive(|_, ((p, c), u)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "answers", "owner", "tags"]);
        f.extend([V::I(c), V::I(u)]);
        out.push(row(f))
    });
    rows(out)
}

fn q8229(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let cc = comments_per_post(db);
    let base = owned(db).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rn = (&base)
        .group_by(post_type.select(&db.post_type.name))
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(&cc))
        .drive(|_, (p, c)| {
        let mut f =
            post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "type"]);
        f.push(V::I(c));
        out.push(row(f))
    });
    rows(out)
}

fn q12691(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let Vote { user, bounty_amount, .. } = &db.vote;
    let questions = db.post.with(post_type_id.eq(1));
    let rn = whole(&questions)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let latest: MatSet<Id<Post>> = owned(db)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .collect();
    let engagement = user
        .inv()
        .select(bounty_amount.opt())
        .fold((0i64, 0i64, 0i64), |(n, bn, bs), b| (n + 1, bn + b.is_some() as i64, bs + b.unwrap_or(0)));
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 100)
        .map(|((p, _), _)| p)
        .with(&latest)
        .select(Ident::<Post>::new().and(owner_user).and(owner_user.select(&engagement)))
        .drive(|_, ((p, u), (n, bn, bs))| {
            let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
            f.extend([
                V::I(db.user.origid.get(u).unwrap()),
                V::S(db.user.display_name.get(u).unwrap()),
                V::I(n),
                avg(bs, bn),
            ]);
            out.push(row(f))
        });
    rows(out)
}

fn q8952(db: &'static So) -> String {
    let Post { post_type_id, score, parent_id, creation_date, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let cc = comments_per_post(db);
    let (n, s) = db.post.select(score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let base = owned(db).with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base)
        .group_by(parent_id.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc);
    let mut v = Vec::new();
    (&rn)
        .filt(|(_, n)| n == 1)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(score.and(creation_date).and(&up).and(&down).and(&cc)))
        .drive(|_, (p, x)| v.push((p, x)));
    v.sort_by(|a, b| (b.1.0.0.0.0, b.1.0.0.0.1).cmp(&(a.1.0.0.0.0, a.1.0.0.0.1)));
    rows(v.iter().take(50).map(|&(p, ((((_, _), u), d), c))| {
        let mut f = post_fields(
            db,
            p,
            &["id", "title", "score", "views", "created", "answers", "comments", "activity", "owner"],
        );
        f.extend([V::I(u), V::I(d), avg(s, n), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.AnswerCount, p.CommentCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// PostStatistics AS (SELECT u.DisplayName, COUNT(rp.PostId) AS QuestionCount, SUM(rp.Score) AS TotalScore, SUM(rp.AnswerCount) AS TotalAnswers, AVG(rp.CommentCount) AS AvgComments
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.rn <= 5 GROUP BY u.DisplayName),
// TopUsers AS (SELECT DisplayName, QuestionCount, TotalScore, TotalAnswers, AvgComments, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM PostStatistics)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, tu.QuestionCount, tu.TotalScore, tu.TotalAnswers, tu.AvgComments, tu.ScoreRank
// FROM TopUsers tu JOIN Users u ON tu.DisplayName = u.DisplayName WHERE tu.ScoreRank <= 10 ORDER BY tu.ScoreRank;
fn q6959(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, answer_count, comment_count, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let stats = (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(answer_count.opt()).and(comment_count))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, an, asum, c), ((sc, a), cm)| {
            (n + 1, s + sc, an + a.is_some() as i64, asum + a.unwrap_or(0), c + cm)
        });
    let rk = whole(&stats).select(Same::new().and(&stats)).window(rank, |(_, a)| a.1, desc);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    type R = ((Str, (i64, i64, i64, i64, i64)), i64);
    let joined = (&rk).filt(|(_, r)| r <= 10).select(Same::<R>::new().and(Same::<R>::new().map(|((dn, _), _): R| dn).select(&by_name)));
    rows(drain(joined).into_iter().map(|(_, (((dn, (n, s, an, asum, c)), r), u))| {
        row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(dn), V::I(db.user.reputation.get(u).unwrap()), V::I(n), V::I(s), nullable(asum, an), avg(c, n), V::I(r)])
    }))
}

fn q6627(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, view_count, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1)).with(score.gt(0));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let us = (&base).select(owner_user).inv().select(score).fold((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let phs = (&db.post_history.post)
        .inv()
        .select(&db.post_history.creation_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 3)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.select(&us)).and(&phs))
        .drive(|_, ((p, (ts, pc)), (ec, last))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        f.extend([V::I(ts), V::I(pc), V::I(ec), V::T(last)]);
        out.push(row(f))
    });
    rows(out)
}

fn q12408(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let posts = owner_user.inv().fold(0i64, |a, _| a + 1);
    let t = owner_user
        .inv()
        .select(post_type_id.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(q, a, bn, bs), (ty, b)| {
            let b = b.flatten();
            (q + (ty == 1) as i64, a + (ty == 2) as i64, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        });
    let with_posts = db.user.with(&posts);
    let rk = whole(&with_posts)
        .select((&db.user.origid).and(&db.user.reputation).and(&posts).and(&t))
        .window(rank, |(((_, rep), _), _)| rep, desc);
    let mut v = Vec::new();
    (&rk).drive(|_, x| v.push(x));
    v.sort_by(|a, b| b.0.0.0.1.cmp(&a.0.0.0.1));
    rows(v.iter().take(10).map(|&((((id, rep), n), (q, a, bn, bs)), r)| {
        row(vec![V::I(id), V::I(rep), V::I(n), V::I(q), V::I(a), nullable(bs, bn), V::I(r)])
    }))
}

fn q14924(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, creation_date, .. } = &db.post;
    let t = owner_user.inv().select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64, 0i64, i64::MIN),
        |(n, q, a, s, vs, last), (((ty, sc), w), cd)| {
            (n + 1, q + (ty == 1) as i64, a + (ty == 2) as i64, s + sc, vs + w.unwrap_or(0), last.max(cd))
        },
    );
    let rn = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t))
        .window(row_number, |(_, a)| a.3, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(
        |_, (((id, dn), (n, q, a, s, vs, last)), r)| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                V::I(q),
                V::I(a),
                V::I(s),
                V::I(vs),
                if n == 0 { V::Null } else { V::T(last) },
                V::I(r),
            ]))
        },
    );
    rows(out)
}

fn q14655(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_years(current_date(), -1)));
    let rn = whole(&base).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    engagement(db, (&rn).filt(|(_, n)| n <= 100).map(|((p, _), _)| p)).drive(|p, (c, v, u, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(c), V::I(v), V::I(u), V::I(d)]);
        out.push(row(f))
    });
    rows(out)
}

fn q11084(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let base = db.post.with(post_type_id.eq(1));
    let rn = whole(&base).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(_, cd)| cd, desc);
    let counts = (&rn)
        .filt(|(_, n)| n <= 100)
        .map(|((p, _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).with(post_type_id.eq(2)).opt()))
        .fold((0i64, 0i64), |(c, a), (ci, ai)| (c + ci.is_some() as i64, a + ai.is_some() as i64));
    let mut out = Vec::new();
    (&counts).drive(|p, (c, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "rep"]);
        f.extend([V::I(c), V::I(a)]);
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("13403", q13403),
    ("12213", q12213),
    ("5989", q5989),
    ("8229", q8229),
    ("12691", q12691),
    ("8952", q8952),
    ("6959", q6959),
    ("6627", q6627),
    ("12408", q12408),
    ("14924", q14924),
    ("14655", q14655),
    ("11084", q11084),
];
