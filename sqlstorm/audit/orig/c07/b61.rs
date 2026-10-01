use crate::b60::{Ranked, counts_row, ranked_by, top_by_posts, user_posts};
use harness::prelude::*;

fn q13715(db: &'static So) -> String {
    let posts = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let comments = (&db.comment.user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let rk = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&db.user.reputation).and(&posts).and(&comments))
        .window(rank, |((((_, _), rep), _), _)| rep, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .drive(|_, (((((id, dn), rep), p), c), r)| out.push(row(vec![V::I(id), V::S(dn), V::I(rep), V::I(p), V::I(c), V::I(r)])));
    rows(out)
}

fn q12944(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, rank), &["id", "name", "n", "q", "a", "rank"])
}

fn q12065(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, rank), &["rank", "name", "n", "q", "a", "rep", "views_user"])
}

fn q10383(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, rank), &["rank", "id", "name", "n", "q", "a", "wiki"])
}

fn q14115(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, rank), &["id", "name", "n", "q", "a", "views_sum", "score_sum", "rank"])
}

fn by_score_sum(db: &'static So) -> Ranked {
    ranked_by(db, &user_posts(db), |(_, a)| (a.0 > 0).then_some(a.3), rank)
}

fn q13084(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, &["id", "name", "n", "q", "a", "score_sum"])
}

fn q12708(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, &["id", "name", "n", "q", "a", "score_sum", "rank"])
}

fn q14710(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, &["name", "n", "score_sum", "q", "a", "rank"])
}

fn q13429(db: &'static So) -> String {
    let rk = by_score_sum(db);
    counts_row(db, &rk, &["id", "name", "n", "q", "a", "score_sum", "rank"])
}

fn q13318(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let t = db
        .user
        .select(posts_of(db).select(score.and(view_count.opt())).opt())
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64), |(rows_, s, vs), p| {
            (rows_ + 1, s + p.map_or(0, |(sc, _)| sc), vs + p.and_then(|(_, v)| v).unwrap_or(0))
        });
    let rk = whole(&db.user.id).select(Ident::<User>::new().and(&t)).window(rank, |(_, a)| a.1, desc);
    let mut out = Vec::new();
    (&rk).filt(|(_, r)| r <= 10).drive(|_, ((u, (rows_, s, vs)), r)| {
        out.push(row(vec![
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(db.user.reputation.get(u).unwrap()),
            V::I(rows_),
            V::I(vs),
            V::I(s),
            V::I(r),
        ]))
    });
    rows(out)
}

fn q10677(db: &'static So) -> String {
    let rn = ranked_by(db, &user_posts(db), |((_, rep), _)| rep, row_number);
    counts_row(db, &rn, &["id", "name", "rep", "n", "q", "a"])
}

fn q14314(db: &'static So) -> String {
    let t = user_posts(db);
    let ranked = whole(&db.user.id)
        .select(Ident::<User>::new().and(&t))
        .window(rank, |(_, t)| t.3, desc)
        .window(rank, |((_, t), _)| t.5, desc);
    let mut out = Vec::new();
    ranked.filt(|((_, a), b)| a <= 10 || b <= 10).drive(|_, (((u, (n, _, _, s, _, vs, _)), a), b)| {
        out.push(row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(n),
            V::I(vs),
            V::I(s),
            V::I(a),
            V::I(b),
        ]))
    });
    rows(out)
}

fn q11859(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let latest = post.inv().select(hd).fold(i64::MIN, |m, d| m.max(d));
    let mut out = Vec::new();
    owned(db).with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(latest.opt()).drive(|p, h| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner", "rep"]);
        f.push(ots(h));
        f.extend(post_fields(db, p, &["score", "views", "answers", "comments", "favorites"]));
        out.push(row(f))
    });
    rows(out)
}

fn q7771(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, owner_user, .. } = &db.post;
    let badges = (&db.badge.user).inv().fold(0i64, |a, _| a + 1);
    let by_name: HashIdx<Str, i64> = db.user.select(&db.user.display_name).inv().select(&badges).collect();
    let base = owned(db).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rk = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), v)| (s, v), desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .map(|(((p, _), _), _)| p)
        .select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(by_name.opt())))
        .drive(|_, (p, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.push(oint(b));
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("13715", q13715),
    ("12944", q12944),
    ("12065", q12065),
    ("11859", q11859),
    ("13318", q13318),
    ("10677", q10677),
    ("13084", q13084),
    ("12708", q12708),
    ("14710", q14710),
    ("14314", q14314),
    ("10383", q10383),
    ("14115", q14115),
    ("13429", q13429),
    ("7771", q7771),
];
