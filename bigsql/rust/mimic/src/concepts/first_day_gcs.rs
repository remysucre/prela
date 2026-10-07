use crate::concepts::gcs::*;
use crate::schema::*;
use harness::prelude::*;
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub struct FirstDayGcs {
    pub subject_id: i64,
    pub stay_id: i64,
    pub gcs_min: Option<f64>,
    pub gcs_motor: Option<f64>,
    pub gcs_verbal: Option<f64>,
    pub gcs_eyes: Option<f64>,
    pub gcs_unable: Option<i64>,
}

fn order(a: &(Option<f64>, Ts), b: &(Option<f64>, Ts)) -> Ordering {
    let g = match (a.0, b.0) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    };
    g.then(b.1.cmp(&a.1))
}

pub fn first_day_gcs(db: &'static Db, gs: &[Gcs]) -> Vec<FirstDayGcs> {
    let icu = &db.icu_stay;
    let r = rel(gs.to_vec());
    let by: HashIdx<i64, usize> = (&r).map(|x: Gcs| x.stay_id).inv().collect();
    let ranked = (&icu.intime)
        .and((&icu.stay_id).select(&by).select(&r))
        .filt(|(i, g): (Ts, Gcs)| g.charttime >= i - 6 * 3600 * US && g.charttime <= i + DAY_US)
        .window(row_number, |(_, g): (Ts, Gcs)| (g.gcs, g.charttime), order);
    let first = (&ranked).filt(|(_, rn)| rn == 1);
    let all = (&icu.subject_id).and(&icu.stay_id).and((&first).opt());
    drain(&all)
        .into_iter()
        .map(|(_, ((subject_id, stay_id), m))| {
            let g = m.map(|((_, g), _): ((Ts, Gcs), i64)| g);
            FirstDayGcs {
                subject_id,
                stay_id,
                gcs_min: g.and_then(|g| g.gcs),
                gcs_motor: g.and_then(|g| g.gcs_motor),
                gcs_verbal: g.and_then(|g| g.gcs_verbal),
                gcs_eyes: g.and_then(|g| g.gcs_eyes),
                gcs_unable: g.map(|g| g.gcs_unable),
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let gs = crate::concepts::gcs::gcs(db);
    rows(first_day_gcs(db, &gs).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.stay_id),
            ofloat(v.gcs_min),
            ofloat(v.gcs_motor),
            ofloat(v.gcs_verbal),
            ofloat(v.gcs_eyes),
            oint(v.gcs_unable),
        ])
    }))
}
