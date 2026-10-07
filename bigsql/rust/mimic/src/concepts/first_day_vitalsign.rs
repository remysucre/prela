use crate::concepts::vitalsign::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct FirstDayVitalsign {
    pub subject_id: i64,
    pub stay_id: i64,
    pub heart_rate_min: Option<f64>,
    pub heart_rate_max: Option<f64>,
    pub heart_rate_mean: Option<f64>,
    pub sbp_min: Option<f64>,
    pub sbp_max: Option<f64>,
    pub sbp_mean: Option<f64>,
    pub dbp_min: Option<f64>,
    pub dbp_max: Option<f64>,
    pub dbp_mean: Option<f64>,
    pub mbp_min: Option<f64>,
    pub mbp_max: Option<f64>,
    pub mbp_mean: Option<f64>,
    pub resp_rate_min: Option<f64>,
    pub resp_rate_max: Option<f64>,
    pub resp_rate_mean: Option<f64>,
    pub temperature_min: Option<f64>,
    pub temperature_max: Option<f64>,
    pub temperature_mean: Option<f64>,
    pub spo2_min: Option<f64>,
    pub spo2_max: Option<f64>,
    pub spo2_mean: Option<f64>,
    pub glucose_min: Option<f64>,
    pub glucose_max: Option<f64>,
    pub glucose_mean: Option<f64>,
}

#[derive(Clone, Copy)]
struct Acc {
    min: f64,
    max: f64,
    sum: (f64, f64),
    n: i64,
}

type Stat = (Option<f64>, Option<f64>, Option<f64>);

fn stat(a: Acc) -> Stat {
    if a.n == 0 {
        (None, None, None)
    } else {
        (Some(a.min), Some(a.max), Some(a.sum.0 / a.n as f64))
    }
}

pub fn first_day_vitalsign(db: &'static Db, vs: &[Vitalsign]) -> Vec<FirstDayVitalsign> {
    let icu = &db.icu_stay;
    let r = rel(vs.to_vec());
    let by: HashIdx<i64, usize> = (&r).map(|x: Vitalsign| x.stay_id).inv().collect();
    let init = [Acc {
        min: f64::INFINITY,
        max: f64::NEG_INFINITY,
        sum: (0.0, 0.0),
        n: 0,
    }; 8];
    let per = (&icu.intime)
        .and((&icu.stay_id).select(&by).select(&r))
        .filt(|(i, x): (Ts, Vitalsign)| {
            x.charttime >= i - 6 * 3600 * US && x.charttime <= i + DAY_US
        })
        .fold(init, |mut a, (_, x): (Ts, Vitalsign)| {
            let vals = [
                x.heart_rate,
                x.sbp,
                x.dbp,
                x.mbp,
                x.resp_rate,
                x.temperature,
                x.spo2,
                x.glucose,
            ];
            for (s, c) in a.iter_mut().zip(vals) {
                if let Some(c) = c {
                    s.min = s.min.min(c);
                    s.max = s.max.max(c);
                    s.sum = kahan(s.sum, c);
                    s.n += 1;
                }
            }
            a
        });
    let all = (&icu.subject_id).and(&icu.stay_id).and((&per).opt());
    drain(&all)
        .into_iter()
        .map(|(_, ((subject_id, stay_id), a))| {
            let s: Vec<Stat> = a.unwrap_or(init).iter().map(|&x| stat(x)).collect();
            FirstDayVitalsign {
                subject_id,
                stay_id,
                heart_rate_min: s[0].0,
                heart_rate_max: s[0].1,
                heart_rate_mean: s[0].2,
                sbp_min: s[1].0,
                sbp_max: s[1].1,
                sbp_mean: s[1].2,
                dbp_min: s[2].0,
                dbp_max: s[2].1,
                dbp_mean: s[2].2,
                mbp_min: s[3].0,
                mbp_max: s[3].1,
                mbp_mean: s[3].2,
                resp_rate_min: s[4].0,
                resp_rate_max: s[4].1,
                resp_rate_mean: s[4].2,
                temperature_min: s[5].0,
                temperature_max: s[5].1,
                temperature_mean: s[5].2,
                spo2_min: s[6].0,
                spo2_max: s[6].1,
                spo2_mean: s[6].2,
                glucose_min: s[7].0,
                glucose_max: s[7].1,
                glucose_mean: s[7].2,
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let vs = crate::concepts::vitalsign::vitalsign(db);
    rows(first_day_vitalsign(db, &vs).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.stay_id),
            ofloat(v.heart_rate_min),
            ofloat(v.heart_rate_max),
            ofloat(v.heart_rate_mean),
            ofloat(v.sbp_min),
            ofloat(v.sbp_max),
            ofloat(v.sbp_mean),
            ofloat(v.dbp_min),
            ofloat(v.dbp_max),
            ofloat(v.dbp_mean),
            ofloat(v.mbp_min),
            ofloat(v.mbp_max),
            ofloat(v.mbp_mean),
            ofloat(v.resp_rate_min),
            ofloat(v.resp_rate_max),
            ofloat(v.resp_rate_mean),
            ofloat(v.temperature_min),
            ofloat(v.temperature_max),
            ofloat(v.temperature_mean),
            ofloat(v.spo2_min),
            ofloat(v.spo2_max),
            ofloat(v.spo2_mean),
            ofloat(v.glucose_min),
            ofloat(v.glucose_max),
            ofloat(v.glucose_mean),
        ])
    }))
}
