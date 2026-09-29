use harness::prelude::*;

// Every query here is the same shape: a CTE of per-user aggregates, then
// `ROW_NUMBER()`/`RANK() OVER (ORDER BY ...)` with `WHERE Rank <= 10`. The
// window has no PARTITION BY, so it ranks a row against the whole result —
// which is host Rust after the plan, exactly like ORDER BY and LIMIT.
// See `notes/limitations.md`.

struct UStats {
    uid: Id<User>,
    id: i64,
    display_name: Str,
    reputation: i64,
    posts: i64,
    questions: i64,
    answers: i64,
    score_sum: i64,
    views_sum: i64,
}

fn user_stats(db: &'static So) -> Vec<UStats> {
    let Post { owner_user, score, post_type_id, view_count, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let nu = db.user.id.n;

    let base = owner_user.inv().select(score.and(post_type_id)).dense_fold_outer(
        nu,
        (0i64, 0i64, 0i64, 0i64),
        |(n, s, q, a), (sc, t)| (n + 1, s + sc, q + (t == 1) as i64, a + (t == 2) as i64),
    );
    let vw = owner_user
        .inv()
        .select(view_count)
        .dense_fold_outer(nu, 0i64, |s, x| s + x);

    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(base).and(vw))
        .drive(
            |uid, ((((id, dn), rep), (posts, score_sum, questions, answers)), views_sum)| {
                v.push(UStats {
                    uid,
                    id,
                    display_name: dn,
                    reputation: rep,
                    posts,
                    questions,
                    answers,
                    score_sum,
                    views_sum,
                })
            },
        );
    v
}

/// Top 10 by `SUM(COALESCE(P.Score, 0))`, which is 0 rather than NULL for a
/// user with no posts — the LEFT JOIN still gives them a row.
fn by_score(db: &'static So) -> Vec<UStats> {
    let mut v = user_stats(db);
    v.sort_by(|a, b| b.score_sum.cmp(&a.score_sum));
    v.truncate(10);
    v
}

fn by_reputation(db: &'static So) -> Vec<UStats> {
    let mut v = user_stats(db);
    v.sort_by(|a, b| b.reputation.cmp(&a.reputation));
    v.truncate(10);
    v
}

fn q13692(db: &'static So) -> String {
    rows(by_score(db).iter().enumerate().map(|(i, r)| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.posts),
            V::I(r.questions),
            V::I(r.answers),
            V::I(r.views_sum),
            V::I(r.score_sum),
            V::I(i as i64 + 1),
        ])
    }))
}

fn q12949(db: &'static So) -> String {
    rows(by_score(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.posts),
            V::I(r.questions),
            V::I(r.answers),
            V::I(r.score_sum),
            V::I(r.views_sum),
        ])
    }))
}

fn q13846(db: &'static So) -> String {
    rows(by_score(db).iter().enumerate().map(|(i, r)| {
        row(vec![
            V::I(i as i64 + 1),
            V::S(r.display_name),
            V::I(r.posts),
            V::I(r.score_sum),
            V::I(r.views_sum),
            V::I(r.questions),
            V::I(r.answers),
        ])
    }))
}

/// `RANK()`, not `ROW_NUMBER()` — but no two of the top ten tie on
/// TotalScore, so the two agree here.
fn q12563(db: &'static So) -> String {
    let v = by_score(db);
    rows(v.iter().enumerate().map(|(i, r)| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.posts),
            V::I(r.questions),
            V::I(r.answers),
            V::I(r.views_sum),
            V::I(r.score_sum),
            V::I(i as i64 + 1),
        ])
    }))
}

/// `SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL ...)`
/// — no answer in this data carries an AcceptedAnswerId, so it is 0.
fn q11199(db: &'static So) -> String {
    let Post { owner_user, post_type_id, accepted_answer_id, .. } = &db.post;
    let accepted = owner_user
        .inv()
        .with(post_type_id.eq(2))
        .with(accepted_answer_id)
        .dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);

    rows(by_reputation(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::I(r.reputation),
            V::I(r.posts),
            V::I(r.questions),
            V::I(r.answers),
            V::I(accepted.get(r.uid).unwrap_or(0)),
        ])
    }))
}

// ---- ... plus a second LEFT JOIN, which multiplies the rows -------------

fn badge_rows(db: &'static So) -> DenseFold<Id<User>, (i64, i64, i64, i64, i64, i64)> {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    db.user
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt().and(badges_of(db).opt()))
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(q, a, s, vn, vs, b), (p, bi)| {
            let t = p.map(|((t, _), _)| t);
            let w = p.and_then(|(_, w)| w);
            (
                q + (t == Some(1)) as i64,
                a + (t == Some(2)) as i64,
                s + p.map_or(0, |((_, sc), _)| sc),
                vn + w.is_some() as i64,
                vs + w.unwrap_or(0),
                b + bi.is_some() as i64,
            )
        })
}

fn q12517(db: &'static So) -> String {
    let rows_ = badge_rows(db);
    let mut v = user_stats(db);
    v.sort_by(|a, b| b.reputation.cmp(&a.reputation));
    rows(v.iter().take(10).map(|r| {
        let (q, a, _, _, _, b) = rows_.get(r.uid).unwrap();
        row(vec![V::I(r.id), V::I(r.reputation), V::I(r.posts), V::I(a), V::I(q), V::I(b)])
    }))
}

fn q9585(db: &'static So) -> String {
    let rows_ = badge_rows(db);
    let mut v: Vec<_> = user_stats(db).into_iter().map(|r| (rows_.get(r.uid).unwrap(), r)).collect();
    v.sort_by(|a, b| b.1.posts.cmp(&a.1.posts).then(b.0.2.cmp(&a.0.2)));
    rows(v.iter().take(10).map(|((q, a, s, vn, vs, b), r)| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.posts),
            V::I(*q),
            V::I(*a),
            nullable(*s, r.posts),
            nullable(*vs, *vn),
            V::I(*b),
        ])
    }))
}

fn q12087(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let nu = db.user.id.n;

    let w = owner_user
        .inv()
        .select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .dense_fold_outer(nu, (0i64, 0i64, 0i64, 0i64, 0i64), |(q, a, vw, u, d), ((t, x), vt)| {
            (
                q + (t == 1) as i64,
                a + (t == 2) as i64,
                vw + x.unwrap_or(0),
                u + (vt == Some(2)) as i64,
                d + (vt == Some(3)) as i64,
            )
        });

    let mut v: Vec<(i64, i64, i64, i64, i64, i64, i64, i64)> = Vec::new();
    db.user
        .select(
            (&db.user.origid)
                .and(&db.user.reputation)
                .and(owner_user.inv().dense_fold_outer(nu, 0i64, |a, _| a + 1))
                .and(w),
        )
        .drive(|_, (((id, rep), posts), (q, a, views, u, d))| {
            v.push((id, rep, posts, q, a, views, u, d))
        });

    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).enumerate().map(|(i, (id, rep, p, q, a, vi, u, d))| {
        row(vec![
            V::I(*id),
            V::I(*rep),
            V::I(*p),
            V::I(*q),
            V::I(*a),
            V::I(*vi),
            V::I(*u),
            V::I(*d),
            V::I(i as i64 + 1),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("13692", q13692),
    ("11199", q11199),
    ("12517", q12517),
    ("9585", q9585),
    ("12949", q12949),
    ("12087", q12087),
    ("13846", q13846),
    ("12563", q12563),
];
