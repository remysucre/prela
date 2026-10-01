use harness::prelude::*;
use crate::q::{type_id_aggs, by_created, by_views};

fn q11492(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.answers_sum, a.answers_n),
        ])
    }))
}

fn q12981(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.views_sum, a.views_n),
            V::I(a.score_sum),
        ])
    }))
}

fn count_avgscore_sumviews(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
        ])
    }))
}

fn q10296(db: &'static So) -> String {
    rows(type_id_aggs(db).into_iter().map(|(t, a)| row(vec![tname(db, t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2])])))
}

fn q10054(db: &'static So) -> String {
    let mut a = type_aggs(db);
    a.sort_by_key(|x| (x.views_n == 0, std::cmp::Reverse(x.views_sum)));
    rows(a.iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
        ])
    }))
}

fn q10820(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
        ])
    }))
}

fn q12559(db: &'static So) -> String {
    let mut a = type_aggs(db);
    a.sort_by(|x, y| {
        let av = |t: &TypeAgg| {
            if t.views_n == 0 { f64::MIN } else { t.views_sum as f64 / t.views_n as f64 }
        };
        av(y).partial_cmp(&av(x)).unwrap()
    });
    rows(a.iter().map(|a| {
        row(vec![
            V::S(a.name),
            avg(a.views_sum, a.views_n),
            avg(a.score_sum, a.n),
            V::I(a.n),
        ])
    }))
}

fn q16812(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let Vote { vote_type_id, post, .. } = &db.vote;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, Str, i64)> = Vec::new();
    db.vote
        .with(post.select(post_type_id).eq(1))
        .select(
            post.and(post.select(creation_date.and(owner_user.select(display_name))))
                .and(vote_type_id),
        )
        .drive(|_, ((p, (created, dn)), vt)| v.push((p, created, dn, vt)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(p, created, dn, vt)| {
        row(vec![V::S(dn), title(db, *p), V::T(*created), V::I(*vt)])
    }))
}

fn q19877(db: &'static So) -> String {
    rows(by_views(db).iter().map(|r| {
        row(vec![
            V::S(r.display_name),
            V::I(r.reputation),
            title(db, r.pid),
            V::T(r.created),
            oint(r.views),
        ])
    }))
}

fn q17002(db: &'static So) -> String {
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.body),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
        ])
    }))
}

fn q17351(db: &'static So) -> String {
    let last = &db.post.last_activity_date;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::I(r.score),
            V::S(r.display_name),
            V::T(r.created),
            V::T(last.get(r.pid).unwrap()),
        ])
    }))
}

// SELECT U.DisplayName, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AverageViewCount
// FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.DisplayName ORDER BY PostCount DESC LIMIT 10;
fn q18422(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let g = db.post.with(owner_user).group_by(owner_user.select(&db.user.display_name)).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let v = top_n(drain(&g), |&(_, a)| std::cmp::Reverse(a[0]), 10);
    rows(v.into_iter().map(|(dn, a)| row(vec![V::S(dn), V::I(a[0]), V::I(a[1]), avg(a[3], a[2])])))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("11492", q11492),
    ("12981", q12981),
    ("16812", q16812),
    ("19877", q19877),
    ("10186", count_avgscore_sumviews),
    ("10296", q10296),
    ("17002", q17002),
    ("17351", q17351),
    ("18422", q18422),
    ("10054", q10054),
    ("10820", q10820),
    ("12559", q12559),
];
