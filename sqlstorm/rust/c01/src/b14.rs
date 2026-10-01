use harness::prelude::*;
use crate::q::{questions, by_created, by_score};

fn q17466(db: &'static So) -> String {
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score).then(b.views.cmp(&a.views)));
    v.truncate(10);
    rows(v.iter().map(|r| {
        row(vec![
            title(db, r.pid),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.score),
            oint(r.views),
        ])
    }))
}

fn q15651(db: &'static So) -> String {
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::S(r.body),
            V::S(r.display_name),
            V::I(r.score),
            oint(r.views),
        ])
    }))
}

fn count_avgscore_avgviews(db: &'static So, n: usize) -> String {
    let mut v = by_count(db);
    v.truncate(n);
    rows(v.iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
        ])
    }))
}

fn q12022(db: &'static So) -> String {
    count_avgscore_avgviews(db, 100)
}

fn q12376(db: &'static So) -> String {
    count_avgscore_avgviews(db, 10)
}

fn q10620(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            V::T(a.last_activity_max),
        ])
    }))
}

fn q11592(db: &'static So) -> String {
    let mut a = type_aggs(db);
    a.sort_by(|x, y| {
        let s = |t: &TypeAgg| t.score_sum as f64 / t.n as f64;
        s(y).partial_cmp(&s(x)).unwrap()
    });
    rows(a.iter().map(|a| {
        row(vec![
            V::S(a.name),
            avg(a.score_sum, a.n),
            avg(a.answers_sum, a.answers_n),
            V::I(a.n),
        ])
    }))
}

fn q16030(db: &'static So) -> String {
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            oint(r.views),
            V::S(r.display_name),
            V::I(r.reputation),
        ])
    }))
}

fn q19503(db: &'static So) -> String {
    let ac = &db.post.answer_count;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::S(r.display_name),
            oint(r.views),
            oint(ac.get(r.pid)),
        ])
    }))
}

fn q16251(db: &'static So) -> String {
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::I(r.score),
            V::S(r.display_name),
            V::I(r.reputation),
        ])
    }))
}

fn q17354(db: &'static So) -> String {
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.body),
            V::S(r.display_name),
            V::T(r.created),
            oint(r.views),
        ])
    }))
}

fn q18681(db: &'static So) -> String {
    let ac = &db.post.answer_count;
    rows(by_score(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::I(r.score),
            V::S(r.display_name),
            V::T(r.created),
            oint(r.views),
            oint(ac.get(r.pid)),
        ])
    }))
}

fn q15139(db: &'static So) -> String {
    let ac = &db.post.answer_count;
    rows(by_created(db).iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.score),
            oint(r.views),
            oint(ac.get(r.pid)),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("17466", q17466),
    ("15651", q15651),
    ("12022", q12022),
    ("10620", q10620),
    ("12376", q12376),
    ("16030", q16030),
    ("11592", q11592),
    ("19503", q19503),
    ("16251", q16251),
    ("17354", q17354),
    ("18681", q18681),
    ("15139", q15139),
];
