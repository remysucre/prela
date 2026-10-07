use crate::concepts::urine_output::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct FirstDayUrineOutput {
    pub subject_id: i64,
    pub stay_id: i64,
    pub urineoutput: Option<f64>,
}

pub fn first_day_urine_output(db: &'static Db, uo: &[UrineOutput]) -> Vec<FirstDayUrineOutput> {
    let icu = &db.icu_stay;
    let r = rel(uo.to_vec());
    let by: HashIdx<i64, usize> = (&r).map(|x: UrineOutput| x.stay_id).inv().collect();
    let per = (&icu.intime)
        .and((&icu.stay_id).select(&by).select(&r))
        .filt(|(i, u): (Ts, UrineOutput)| u.charttime >= i && u.charttime <= i + DAY_US)
        .fold(((0.0, 0.0), 0i64), |(s, n), (_, u): (Ts, UrineOutput)| (kahan(s, u.urineoutput), n + 1));
    let all = (&icu.subject_id).and(&icu.stay_id).and((&per).opt());
    drain(&all)
        .into_iter()
        .map(|(_, ((subject_id, stay_id), a))| FirstDayUrineOutput {
            subject_id,
            stay_id,
            urineoutput: a.filter(|a| a.1 > 0).map(|a| a.0.0),
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let uo = crate::concepts::urine_output::urine_output(db);
    rows(first_day_urine_output(db, &uo).into_iter().map(|v| {
        row(vec![V::I(v.subject_id), V::I(v.stay_id), ofloat(v.urineoutput)])
    }))
}
