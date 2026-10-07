use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IcustayTimes {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub stay_id: i64,
    pub intime_hr: Option<Ts>,
    pub outtime_hr: Option<Ts>,
}

pub fn icustay_times(db: &'static Db) -> Vec<IcustayTimes> {
    let ce = &db.chart_event;
    let ie = &db.icu_stay;
    let t1 = (&ce.id)
        .with((&ce.itemid).eq(220045))
        .group_by(&ce.stay)
        .select(&ce.charttime)
        .fold((i64::MAX, i64::MIN), |(lo, hi), t| (lo.min(t), hi.max(t)));
    let q = (&ie.subject_id).and(&ie.hadm_id).and(&ie.stay_id).and((&t1).opt());
    drain(q)
        .into_iter()
        .map(|(_, (((subject_id, hadm_id), stay_id), t))| IcustayTimes {
            subject_id,
            hadm_id,
            stay_id,
            intime_hr: t.map(|t| t.0),
            outtime_hr: t.map(|t| t.1),
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(icustay_times(db).into_iter().map(|r| {
        row(vec![V::I(r.subject_id), V::I(r.hadm_id), V::I(r.stay_id), ots(r.intime_hr), ots(r.outtime_hr)])
    }))
}
