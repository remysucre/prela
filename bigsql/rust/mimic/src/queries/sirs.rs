use crate::concepts::{first_day_bg_art::*, first_day_lab::*, first_day_vitalsign::*};
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Sirs {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub stay_id: i64,
    pub sirs: i64,
    pub temp_score: Option<i64>,
    pub heart_rate_score: Option<i64>,
    pub resp_score: Option<i64>,
    pub wbc_score: Option<i64>,
}

type Calc = (i64, [Option<i64>; 4]);

fn lt(x: Option<f64>, c: f64) -> bool {
    x.is_some_and(|x| x < c)
}

fn gt(x: Option<f64>, c: f64) -> bool {
    x.is_some_and(|x| x > c)
}

fn calc(stay_id: i64, bg: Option<FirstDayBgArt>, v: Option<FirstDayVitalsign>, l: Option<FirstDayLab>) -> Calc {
    let tmin = v.and_then(|v| v.temperature_min);
    let tmax = v.and_then(|v| v.temperature_max);
    let hr = v.and_then(|v| v.heart_rate_max);
    let rr = v.and_then(|v| v.resp_rate_max);
    let pco2 = bg.and_then(|b| b.pco2_min);
    let wmin = l.and_then(|l| l.wbc_min);
    let wmax = l.and_then(|l| l.wbc_max);
    let bands = l.and_then(|l| l.bands_max);
    let temp = if lt(tmin, 36.0) || gt(tmax, 38.0) {
        Some(1)
    } else if tmin.is_none() {
        None
    } else {
        Some(0)
    };
    let heart = if gt(hr, 90.0) {
        Some(1)
    } else if hr.is_none() {
        None
    } else {
        Some(0)
    };
    let resp = if gt(rr, 20.0) || lt(pco2, 32.0) {
        Some(1)
    } else if rr.or(pco2).is_none() {
        None
    } else {
        Some(0)
    };
    let wbc = if lt(wmin, 4.0) || gt(wmax, 12.0) || gt(bands, 10.0) {
        Some(1)
    } else if wmin.or(bands).is_none() {
        None
    } else {
        Some(0)
    };
    (stay_id, [temp, heart, resp, wbc])
}

pub fn sirs(db: &'static Db, bg: &[FirstDayBgArt], vs: &[FirstDayVitalsign], lab: &[FirstDayLab]) -> Vec<Sirs> {
    let ie = &db.icu_stay;
    let bgr = rel(bg.to_vec());
    let bg_by: HashIdx<i64, usize> = (&bgr).map(|x: FirstDayBgArt| x.stay_id).inv().collect();
    let vr = rel(vs.to_vec());
    let v_by: HashIdx<i64, usize> = (&vr).map(|x: FirstDayVitalsign| x.stay_id).inv().collect();
    let lr = rel(lab.to_vec());
    let l_by: HashIdx<i64, usize> = (&lr).map(|x: FirstDayLab| x.stay_id).inv().collect();
    let comp = (&ie.stay_id)
        .and((&ie.stay_id).select((&bg_by).select(&bgr).opt()))
        .and((&ie.stay_id).select((&v_by).select(&vr).opt()))
        .and((&ie.stay_id).select((&l_by).select(&lr).opt()));
    let sc = rel(drain(&comp).into_iter().map(|(_, (((s, b), v), l))| calc(s, b, v, l)).collect::<Vec<Calc>>());
    let s_by: HashIdx<i64, usize> = (&sc).map(|c: Calc| c.0).inv().collect();
    let out = (&ie.subject_id).and(&ie.hadm_id).and(&ie.stay_id).and((&ie.stay_id).select((&s_by).select(&sc).opt()));
    drain(&out)
        .into_iter()
        .map(|(_, (((subject_id, hadm_id), stay_id), c))| {
            let s = c.map_or([None; 4], |c| c.1);
            Sirs {
                subject_id,
                hadm_id,
                stay_id,
                sirs: s.iter().map(|x| x.unwrap_or(0)).sum(),
                temp_score: s[0],
                heart_rate_score: s[1],
                resp_score: s[2],
                wbc_score: s[3],
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let bg = crate::concepts::bg::bg(db);
    let fbg = crate::concepts::first_day_bg_art::first_day_bg_art(db, &bg);
    let diff = crate::concepts::blood_differential::blood_differential(db);
    let chem = crate::concepts::chemistry::chemistry(db);
    let coag = crate::concepts::coagulation::coagulation(db);
    let cbc = crate::concepts::complete_blood_count::complete_blood_count(db);
    let enz = crate::concepts::enzyme::enzyme(db);
    let lab = crate::concepts::first_day_lab::first_day_lab(db, &cbc, &chem, &diff, &coag, &enz);
    let vs = crate::concepts::vitalsign::vitalsign(db);
    let fvs = crate::concepts::first_day_vitalsign::first_day_vitalsign(db, &vs);
    rows(sirs(db, &fbg, &fvs, &lab).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.hadm_id),
            V::I(v.stay_id),
            V::I(v.sirs),
            oint(v.temp_score),
            oint(v.heart_rate_score),
            oint(v.resp_score),
            oint(v.wbc_score),
        ])
    }))
}
