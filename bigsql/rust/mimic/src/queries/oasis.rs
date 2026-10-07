use crate::concepts::age::*;
use crate::concepts::first_day_gcs::*;
use crate::concepts::first_day_urine_output::*;
use crate::concepts::first_day_vitalsign::*;
use crate::concepts::ventilation::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Oasis {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub stay_id: i64,
    pub oasis: i64,
    pub oasis_prob: f64,
    pub age: Option<i64>,
    pub age_score: Option<i64>,
    pub preiculos: i64,
    pub preiculos_score: i64,
    pub gcs: Option<f64>,
    pub gcs_score: Option<i64>,
    pub heartrate: Option<f64>,
    pub heart_rate_score: Option<i64>,
    pub meanbp: Option<f64>,
    pub mbp_score: Option<i64>,
    pub resprate: Option<f64>,
    pub resp_rate_score: Option<i64>,
    pub temp: Option<f64>,
    pub temp_score: Option<i64>,
    pub urineoutput: Option<f64>,
    pub urineoutput_score: Option<i64>,
    pub mechvent: i64,
    pub mechvent_score: i64,
    pub electivesurgery: i64,
    pub electivesurgery_score: i64,
}

fn by_stay<R: Copy + 'static>(db: &'static Db, v: &[R], key: fn(R) -> i64) -> HashIdx<Id<IcuStay>, R> {
    let r = rel(v.to_vec());
    let by: HashIdx<i64, usize> = (&r).map(key).inv().collect();
    (&db.icu_stay.stay_id).select(&by).select(&r).collect()
}

fn minute(t: Ts) -> i64 {
    t.div_euclid(60 * US)
}

pub fn oasis(
    db: &'static Db,
    ag: &[Age],
    fdg: &[FirstDayGcs],
    fduo: &[FirstDayUrineOutput],
    fdv: &[FirstDayVitalsign],
    vent: &[Ventilation],
) -> Vec<Oasis> {
    let icu = &db.icu_stay;
    let ad = &db.admission;
    let se = &db.service;
    let pa = &db.patient;

    let se_by: HashIdx<Id<Admission>, Id<Service>> = (&se.hadm).inv().collect();
    let surg = (&icu.intime)
        .and((&icu.hadm).select(&se_by).select((&se.transfertime).and(&se.curr_service)))
        .filt(|(i, (t, _)): (Ts, (Ts, Str))| t < i + DAY_US)
        .fold(0i64, |a, (_, (_, s)): (Ts, (Ts, Str))| {
            let f = if like(&s.to_lowercase(), "%surg%") || s == "ORTHO" { 1 } else { 0 };
            a.max(f)
        });

    let vr = rel(vent.to_vec());
    let v_by: HashIdx<i64, usize> = (&vr).map(|x: Ventilation| x.stay_id).inv().collect();
    let mv = (&icu.intime)
        .and((&icu.stay_id).select(&v_by).select(&vr))
        .filt(|(i, v): (Ts, Ventilation)| {
            let e = i + DAY_US;
            v.ventilation_status == "InvasiveVent"
                && ((v.starttime >= i && v.starttime <= e)
                    || (v.endtime >= i && v.endtime <= e)
                    || (v.starttime <= i && v.endtime >= e))
        })
        .fold(0i64, |_, _| 1);

    let ar = rel(ag.to_vec());
    let ag_by: HashIdx<i64, usize> = (&ar).map(|x: Age| x.hadm_id).inv().collect();
    let age_s: HashIdx<Id<IcuStay>, i64> = (&icu.hadm_id).select(&ag_by).select(&ar).map(|a: Age| a.age).collect();
    let gcs_s = by_stay(db, fdg, |x| x.stay_id);
    let uo_s = by_stay(db, fduo, |x| x.stay_id);
    let vs_s = by_stay(db, fdv, |x| x.stay_id);

    let cohort = (&icu.subject_id)
        .and(&icu.hadm_id)
        .and(&icu.stay_id)
        .and(&icu.intime)
        .and((&icu.hadm).select((&ad.admittime).and(&ad.admission_type)))
        .and((&icu.subject).select(&pa.subject_id))
        .and((&age_s).opt())
        .and((&surg).opt())
        .and((&gcs_s).opt())
        .and((&vs_s).opt())
        .and((&uo_s).opt())
        .and((&mv).opt());

    drain(&cohort)
        .into_iter()
        .map(
            |(_, (((((((((((subject_id, hadm_id), stay_id), intime), (admittime, atype)), _), age), surg), g), v), u), mv))| {
                let preiculos = minute(intime) - minute(admittime);
                let p = preiculos as f64;
                let preiculos_score = if p < 10.2 {
                    5
                } else if p < 297.0 {
                    3
                } else if p < 1440.0 {
                    0
                } else if p < 18708.0 {
                    2
                } else {
                    1
                };
                let age_score = age.map(|a| {
                    if a < 24 {
                        0
                    } else if a <= 53 {
                        3
                    } else if a <= 77 {
                        6
                    } else if a <= 89 {
                        9
                    } else if a >= 90 {
                        7
                    } else {
                        0
                    }
                });
                let gcs = g.and_then(|g| g.gcs_min);
                let gcs_score = gcs.map(|g| {
                    if g <= 7.0 {
                        10
                    } else if g < 14.0 {
                        4
                    } else if g == 14.0 {
                        3
                    } else {
                        0
                    }
                });
                let hmin = v.and_then(|v| v.heart_rate_min);
                let hmax = v.and_then(|v| v.heart_rate_max);
                let lt = |a: Option<f64>, x: f64| a.is_some_and(|a| a < x);
                let gt = |a: Option<f64>, x: f64| a.is_some_and(|a| a > x);
                let ge = |a: Option<f64>, x: f64| a.is_some_and(|a| a >= x);
                let le = |a: Option<f64>, x: f64| a.is_some_and(|a| a <= x);
                let avg = |a: Option<f64>, b: Option<f64>| match (a, b) {
                    (Some(a), Some(b)) => Some((a + b) / 2.0),
                    _ => None,
                };
                let (heart_rate_score, heartrate) = if hmax.is_none() {
                    (None, None)
                } else if gt(hmax, 125.0) {
                    (Some(6), hmax)
                } else if lt(hmin, 33.0) {
                    (Some(4), hmin)
                } else if ge(hmax, 107.0) && le(hmax, 125.0) {
                    (Some(3), hmax)
                } else if ge(hmax, 89.0) && le(hmax, 106.0) {
                    (Some(1), hmax)
                } else {
                    (Some(0), avg(hmin, hmax))
                };
                let mmin = v.and_then(|v| v.mbp_min);
                let mmax = v.and_then(|v| v.mbp_max);
                let (mbp_score, meanbp) = if mmin.is_none() {
                    (None, None)
                } else if lt(mmin, 20.65) {
                    (Some(4), mmin)
                } else if lt(mmin, 51.0) {
                    (Some(3), mmin)
                } else if gt(mmax, 143.44) {
                    (Some(3), mmax)
                } else if ge(mmin, 51.0) && lt(mmin, 61.33) {
                    (Some(2), mmin)
                } else {
                    (Some(0), avg(mmin, mmax))
                };
                let rmin = v.and_then(|v| v.resp_rate_min);
                let rmax = v.and_then(|v| v.resp_rate_max);
                let (resp_rate_score, resprate) = if rmin.is_none() {
                    (None, None)
                } else if lt(rmin, 6.0) {
                    (Some(10), rmin)
                } else if gt(rmax, 44.0) {
                    (Some(9), rmax)
                } else if gt(rmax, 30.0) {
                    (Some(6), rmax)
                } else if gt(rmax, 22.0) {
                    (Some(1), rmax)
                } else if lt(rmin, 13.0) {
                    (Some(1), rmin)
                } else {
                    (Some(0), avg(rmin, rmax))
                };
                let tmin = v.and_then(|v| v.temperature_min);
                let tmax = v.and_then(|v| v.temperature_max);
                let (temp_score, temp) = if tmax.is_none() {
                    (None, None)
                } else if gt(tmax, 39.88) {
                    (Some(6), tmax)
                } else if ge(tmin, 33.22) && le(tmin, 35.93) {
                    (Some(4), tmin)
                } else if ge(tmax, 33.22) && le(tmax, 35.93) {
                    (Some(4), tmax)
                } else if lt(tmin, 33.22) {
                    (Some(3), tmin)
                } else if gt(tmin, 35.93) && le(tmin, 36.39) {
                    (Some(2), tmin)
                } else if ge(tmax, 36.89) && le(tmax, 39.88) {
                    (Some(2), tmax)
                } else {
                    (Some(0), avg(tmin, tmax))
                };
                let urineoutput = u.and_then(|u| u.urineoutput);
                let urineoutput_score = urineoutput.map(|x| {
                    if x < 671.09 {
                        10
                    } else if x > 6896.80 {
                        8
                    } else if x >= 671.09 && x <= 1426.99 {
                        5
                    } else if x >= 1427.00 && x <= 2544.14 {
                        1
                    } else {
                        0
                    }
                });
                let mechvent = mv.unwrap_or(0);
                let mechvent_score = if mechvent == 1 { 9 } else { 0 };
                let electivesurgery = if atype == "ELECTIVE" && surg.unwrap_or(0) == 1 { 1 } else { 0 };
                let electivesurgery_score = if electivesurgery == 1 { 0 } else { 6 };
                let oasis = age_score.unwrap_or(0)
                    + preiculos_score
                    + gcs_score.unwrap_or(0)
                    + heart_rate_score.unwrap_or(0)
                    + mbp_score.unwrap_or(0)
                    + resp_rate_score.unwrap_or(0)
                    + temp_score.unwrap_or(0)
                    + urineoutput_score.unwrap_or(0)
                    + mechvent_score
                    + electivesurgery_score;
                let x = (-61746 + 1275 * oasis) as f64 / 10000.0;
                Oasis {
                    subject_id,
                    hadm_id,
                    stay_id,
                    oasis,
                    oasis_prob: 1.0 / (1.0 + (-x).exp()),
                    age,
                    age_score,
                    preiculos,
                    preiculos_score,
                    gcs,
                    gcs_score,
                    heartrate,
                    heart_rate_score,
                    meanbp,
                    mbp_score,
                    resprate,
                    resp_rate_score,
                    temp,
                    temp_score,
                    urineoutput,
                    urineoutput_score,
                    mechvent,
                    mechvent_score,
                    electivesurgery,
                    electivesurgery_score,
                }
            },
        )
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let ag = crate::concepts::age::age(db);
    let gs = crate::concepts::gcs::gcs(db);
    let fdg = crate::concepts::first_day_gcs::first_day_gcs(db, &gs);
    let uo = crate::concepts::urine_output::urine_output(db);
    let fduo = crate::concepts::first_day_urine_output::first_day_urine_output(db, &uo);
    let vs = crate::concepts::vitalsign::vitalsign(db);
    let fdv = crate::concepts::first_day_vitalsign::first_day_vitalsign(db, &vs);
    let od = crate::concepts::oxygen_delivery::oxygen_delivery(db);
    let vset = crate::concepts::ventilator_setting::ventilator_setting(db);
    let vent = crate::concepts::ventilation::ventilation(db, &vset, &od);
    rows(oasis(db, &ag, &fdg, &fduo, &fdv, &vent).into_iter().map(|o| {
        row(vec![
            V::I(o.subject_id),
            V::I(o.hadm_id),
            V::I(o.stay_id),
            V::I(o.oasis),
            V::F(o.oasis_prob),
            oint(o.age),
            oint(o.age_score),
            V::I(o.preiculos),
            V::I(o.preiculos_score),
            ofloat(o.gcs),
            oint(o.gcs_score),
            ofloat(o.heartrate),
            oint(o.heart_rate_score),
            ofloat(o.meanbp),
            oint(o.mbp_score),
            ofloat(o.resprate),
            oint(o.resp_rate_score),
            ofloat(o.temp),
            oint(o.temp_score),
            ofloat(o.urineoutput),
            oint(o.urineoutput_score),
            V::I(o.mechvent),
            V::I(o.mechvent_score),
            V::I(o.electivesurgery),
            V::I(o.electivesurgery_score),
        ])
    }))
}
