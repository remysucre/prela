use crate::concepts::{first_day_lab::*, first_day_rrt::*};
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Meld {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub stay_id: i64,
    pub meld_initial: f64,
    pub meld: f64,
    pub rrt: Option<i64>,
    pub creatinine_max: Option<f64>,
    pub bilirubin_total_max: Option<f64>,
    pub inr_max: Option<f64>,
    pub sodium_min: Option<f64>,
}

fn score(subject_id: i64, hadm_id: i64, stay_id: i64, l: Option<FirstDayLab>, r: Option<FirstDayRrt>) -> Meld {
    let creat = l.and_then(|l| l.creatinine_max);
    let bili = l.and_then(|l| l.bilirubin_total_max);
    let inr = l.and_then(|l| l.inr_max);
    let na = l.and_then(|l| l.sodium_min);
    let rrt = r.and_then(|r| r.dialysis_present);
    let ln1 = 1f64.ln();
    let sodium_score = match na {
        None => 0.0,
        Some(x) if x > 137.0 => 0.0,
        Some(x) if x < 125.0 => 12.0,
        Some(x) => 137.0 - x,
    };
    let creatinine_score = if rrt == Some(1) || creat.is_some_and(|c| c > 4.0) {
        0.957 * 4f64.ln()
    } else if creat.is_some_and(|c| c < 1.0) {
        0.957 * ln1
    } else {
        0.957 * creat.map_or(ln1, |c| c.ln())
    };
    let bilirubin_score = if bili.is_some_and(|b| b < 1.0) { 0.378 * ln1 } else { 0.378 * bili.map_or(ln1, |b| b.ln()) };
    let inr_score =
        if inr.is_some_and(|i| i < 1.0) { 1.120 * ln1 + 0.643 } else { 1.120 * inr.map_or(ln1, |i| i.ln()) + 0.643 };
    let tot = creatinine_score + bilirubin_score + inr_score;
    let meld_initial = if tot > 4.0 { 40.0 } else { round_dec(tot, 1) * 10.0 };
    let meld = if meld_initial > 11.0 {
        meld_initial + 1.32 * sodium_score - round_dec(0.033 * meld_initial, 9) * sodium_score
    } else {
        meld_initial
    };
    Meld {
        subject_id,
        hadm_id,
        stay_id,
        meld_initial,
        meld,
        rrt,
        creatinine_max: creat,
        bilirubin_total_max: bili,
        inr_max: inr,
        sodium_min: na,
    }
}

pub fn meld(db: &'static Db, lab: &[FirstDayLab], frrt: &[FirstDayRrt]) -> Vec<Meld> {
    let ie = &db.icu_stay;
    let lr = rel(lab.to_vec());
    let l_by: HashIdx<i64, usize> = (&lr).map(|x: FirstDayLab| x.stay_id).inv().collect();
    let rr = rel(frrt.to_vec());
    let r_by: HashIdx<i64, usize> = (&rr).map(|x: FirstDayRrt| x.stay_id).inv().collect();
    let cohort = (&ie.subject_id)
        .and(&ie.hadm_id)
        .and(&ie.stay_id)
        .and((&ie.stay_id).select((&l_by).select(&lr).opt()))
        .and((&ie.stay_id).select((&r_by).select(&rr).opt()));
    drain(&cohort)
        .into_iter()
        .map(|(_, ((((subject_id, hadm_id), stay_id), l), r))| score(subject_id, hadm_id, stay_id, l, r))
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let diff = crate::concepts::blood_differential::blood_differential(db);
    let chem = crate::concepts::chemistry::chemistry(db);
    let coag = crate::concepts::coagulation::coagulation(db);
    let cbc = crate::concepts::complete_blood_count::complete_blood_count(db);
    let enz = crate::concepts::enzyme::enzyme(db);
    let lab = crate::concepts::first_day_lab::first_day_lab(db, &cbc, &chem, &diff, &coag, &enz);
    let r = crate::concepts::rrt::rrt(db);
    let fr = crate::concepts::first_day_rrt::first_day_rrt(db, &r);
    rows(meld(db, &lab, &fr).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.hadm_id),
            V::I(v.stay_id),
            V::F(v.meld_initial),
            V::F(v.meld),
            oint(v.rrt),
            ofloat(v.creatinine_max),
            ofloat(v.bilirubin_total_max),
            ofloat(v.inr_max),
            ofloat(v.sodium_min),
        ])
    }))
}
