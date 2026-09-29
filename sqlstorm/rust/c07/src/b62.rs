use crate::b60::{Ranked, counts_row, ranked_by, top_by_posts, user_posts};
use harness::prelude::*;

fn by_score_sum(db: &'static So) -> Ranked {
    ranked_by(db, &user_posts(db), |(_, a)| (a.0 > 0).then_some(a.3), rank)
}

const TOTALS: &[&str] = &["id", "name", "n", "q", "a", "views_sum", "score_sum", "rank"];

fn q11838(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, rank), &["id", "name", "n", "q", "a", "avg_score", "views_sum", "rank"])
}

fn q10474(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, TOTALS)
}

fn q13364(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, TOTALS)
}

fn q10163(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, TOTALS)
}

fn q10152(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, TOTALS)
}

fn q12093(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, TOTALS)
}

fn q12204(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, &["name", "n", "q", "a", "views_sum", "score_sum"])
}

fn q13806(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, &["name", "n", "q", "a", "views_sum", "score_sum"])
}

fn q10831(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, &["id", "name", "n", "q", "a", "score_sum", "avg_views"])
}

fn q14255(db: &'static So) -> String {
    let rk = ranked_by(db, &user_posts(db), |(_, a)| (a.4 > 0).then_some(a.5), rank);
    counts_row(db, &rk, &["id", "name", "n", "q", "a", "avg_score", "views_sum", "rank"])
}

fn q10241(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, row_number), &["id", "name", "n", "views0", "score0", "avg_views", "avg_score"])
}

fn q12400(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, row_number), &["id", "name", "n", "q", "a", "avg_score", "views_sum"])
}

fn q12319(db: &'static So) -> String {
    let Post { owner_user, score, last_activity_date, .. } = &db.post;
    let t = owner_user.inv().select(score.and(last_activity_date)).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, i64::MIN),
        |(n, s, last), (sc, la)| (n + 1, s + sc, last.max(la)),
    );
    let mut out = Vec::new();
    db.user.select((&db.user.origid).and(&db.user.display_name).and(&db.user.reputation).and(&t)).drive(
        |_, (((id, dn), rep), (n, s, last))| {
            out.push(row(vec![V::I(id), V::S(dn), V::I(rep), V::I(n), V::I(s), if n == 0 { V::Null } else { V::T(last) }]))
        },
    );
    rows(out)
}

fn q14111(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let t = db
        .user
        .select(posts_of(db).select(score.and(view_count.opt()).and(comments_of(db).opt())).opt())
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |acc, p| {
            let (n, pos, neg, s, vn, vs, c) = acc;
            match p {
                Some(((sc, v), ci)) => (
                    n + 1,
                    pos + (sc > 0) as i64,
                    neg + (sc < 0) as i64,
                    s + sc,
                    vn + v.is_some() as i64,
                    vs + v.unwrap_or(0),
                    c + ci.is_some() as i64,
                ),
                None => acc,
            }
        });
    let rn = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t))
        .window(row_number, |(_, a)| a.0, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, r)| r <= 10).drive(
        |_, (((id, dn), (n, pos, neg, s, vn, vs, c)), r)| {
            out.push(row(vec![
                V::I(id),
                V::S(dn),
                V::I(n),
                V::I(pos),
                V::I(neg),
                avg(s, n),
                nullable(vs, vn),
                V::I(c),
                V::I(r),
            ]))
        },
    );
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("11838", q11838),
    ("12319", q12319),
    ("10474", q10474),
    ("13364", q13364),
    ("14255", q14255),
    ("10241", q10241),
    ("12204", q12204),
    ("10163", q10163),
    ("10831", q10831),
    ("12400", q12400),
    ("13806", q13806),
    ("14111", q14111),
    ("10152", q10152),
    ("12093", q12093),
];
