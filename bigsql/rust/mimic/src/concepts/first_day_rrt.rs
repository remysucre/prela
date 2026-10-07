use crate::concepts::rrt::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct FirstDayRrt {
    pub subject_id: i64,
    pub stay_id: i64,
    pub dialysis_present: Option<i64>,
    pub dialysis_active: Option<i64>,
    pub dialysis_type: Option<Str>,
}

pub fn first_day_rrt(db: &'static Db, rrt: &[Rrt]) -> Vec<FirstDayRrt> {
    let ie = &db.icu_stay;
    let rr = rel(rrt.to_vec());
    let by: HashIdx<i64, usize> = (&rr).map(|r: Rrt| r.stay_id).inv().collect();
    let agg = (&ie.intime)
        .and((&ie.stay_id).select(&by).select(&rr))
        .filt(|(i, r): (Ts, Rrt)| r.charttime >= i - 6 * 3600 * US && r.charttime <= i + DAY_US)
        .map(|(_, r): (Ts, Rrt)| r)
        .buf_fold(|vs| {
            let p = vs.iter().map(|r| r.dialysis_present).max().unwrap();
            let a = vs.iter().map(|r| r.dialysis_active).max().unwrap();
            let mut ts: Vec<Str> = vs.iter().filter_map(|r| r.dialysis_type).collect();
            ts.sort_unstable();
            ts.dedup();
            let ty: Option<Str> = (!ts.is_empty()).then(|| &*Box::leak(ts.join(", ").into_boxed_str()));
            (p, a, ty)
        });
    drain((&ie.subject_id).and(&ie.stay_id).and((&agg).opt()))
        .into_iter()
        .map(|(_, ((subject_id, stay_id), g))| FirstDayRrt {
            subject_id,
            stay_id,
            dialysis_present: g.map(|g| g.0),
            dialysis_active: g.map(|g| g.1),
            dialysis_type: g.and_then(|g| g.2),
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let r = crate::concepts::rrt::rrt(db);
    rows(first_day_rrt(db, &r).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.stay_id),
            oint(v.dialysis_present),
            oint(v.dialysis_active),
            ostr(v.dialysis_type),
        ])
    }))
}
