use crate::concepts::{
    bg::*, first_day_gcs::*, first_day_lab::*, first_day_urine_output::*, first_day_vitalsign::*, ventilation::*,
};
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Lods {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub stay_id: i64,
    pub lods: i64,
    pub neurologic: Option<i64>,
    pub cardiovascular: Option<i64>,
    pub renal: Option<i64>,
    pub pulmonary: Option<i64>,
    pub hematologic: Option<i64>,
    pub hepatic: Option<i64>,
}

const H: i64 = 3600 * US;

type Pafi = (i64, (Ts, Option<f64>));
type Cpap = (i64, (Ts, Ts));

fn omin(a: Option<f64>, x: Option<f64>) -> Option<f64> {
    match (a, x) {
        (Some(a), Some(x)) => Some(a.min(x)),
        (a, None) => a,
        (None, x) => x,
    }
}

fn lt(x: Option<f64>, c: f64) -> bool {
    x.is_some_and(|x| x < c)
}

fn ge(x: Option<f64>, c: f64) -> bool {
    x.is_some_and(|x| x >= c)
}

fn gt(x: Option<f64>, c: f64) -> bool {
    x.is_some_and(|x| x > c)
}

fn is_cpap(v: Str) -> bool {
    let l = v.to_lowercase();
    like(&l, "%cpap%") || like(&l, "%bipap mask%")
}

#[allow(clippy::too_many_arguments)]
pub fn lods(
    db: &'static Db,
    bg: &[Bg],
    vent: &[Ventilation],
    fdv: &[FirstDayVitalsign],
    fdl: &[FirstDayLab],
    fdu: &[FirstDayUrineOutput],
    fdg: &[FirstDayGcs],
) -> Vec<Lods> {
    let icu = &db.icu_stay;
    let ce = &db.chart_event;

    let cp = (&ce.id)
        .with((&ce.itemid).filt(|i| i == 226732))
        .with((&ce.value).filt(is_cpap))
        .with(
            (&ce.charttime)
                .and((&ce.stay).select(&icu.intime))
                .filt(|(t, i): (Ts, Ts)| t >= i && t <= i + DAY_US),
        )
        .group_by((&ce.stay).select(&icu.stay_id))
        .select(&ce.charttime)
        .fold((Ts::MAX, Ts::MIN), |(lo, hi), t| (lo.min(t), hi.max(t)));
    let cr = rel(drain(&cp).into_iter().map(|(s, (lo, hi))| -> Cpap { (s, (lo - H, hi + 4 * H)) }).collect());

    let br = rel(bg.to_vec());
    let bby: HashIdx<Option<i64>, usize> = (&br).map(|b: Bg| b.hadm_id).inv().collect();
    let pairs = (&icu.intime)
        .and(&icu.outtime)
        .and(&icu.stay_id)
        .and((&icu.hadm_id).map(Some).select(&bby).select(&br))
        .filt(|(((i, o), _), b): (((Ts, Ts), i64), Bg)| b.charttime >= i && b.charttime < o)
        .map(|((_, s), b): (((Ts, Ts), i64), Bg)| -> Pafi { (s, (b.charttime, b.pao2fio2ratio)) });
    let pr = rel(drain(&pairs).into_iter().map(|(_, p)| p).collect());

    let vr = rel(vent.to_vec());
    let vby: HashIdx<i64, usize> = (&vr)
        .filt(|v: Ventilation| v.ventilation_status == "InvasiveVent")
        .map(|v: Ventilation| v.stay_id)
        .inv()
        .collect();
    let vhit: HashIdx<usize, ()> = (&pr)
        .and((&pr).map(|p: Pafi| p.0).select(&vby).select(&vr))
        .filt(|((_, (t, _)), v): (Pafi, Ventilation)| t >= v.starttime && t <= v.endtime)
        .map(|_| ())
        .collect();
    let cby: HashIdx<i64, usize> = (&cr).map(|c: Cpap| c.0).inv().collect();
    let chit: HashIdx<usize, ()> = (&pr)
        .and((&pr).map(|p: Pafi| p.0).select(&cby).select(&cr))
        .filt(|((_, (t, _)), (_, (a, b))): (Pafi, Cpap)| t >= a && t <= b)
        .map(|_| ())
        .collect();
    let p1 = (&pr)
        .and((&vhit).opt())
        .and((&chit).opt())
        .filt(|((_, v), c): ((Pafi, Option<()>), Option<()>)| v.is_some() || c.is_some());
    let pby: HashIdx<i64, usize> = (&pr).map(|p: Pafi| p.0).inv().collect();
    let pafi = (&icu.stay_id)
        .select(&pby)
        .select(&p1)
        .fold(None::<f64>, |m, (((_, (_, r)), _), _): ((Pafi, Option<()>), Option<()>)| omin(m, r));

    let fvr = rel(fdv.to_vec());
    let fvby: HashIdx<i64, usize> = (&fvr).map(|x: FirstDayVitalsign| x.stay_id).inv().collect();
    let flr = rel(fdl.to_vec());
    let flby: HashIdx<i64, usize> = (&flr).map(|x: FirstDayLab| x.stay_id).inv().collect();
    let fur = rel(fdu.to_vec());
    let fuby: HashIdx<i64, usize> = (&fur).map(|x: FirstDayUrineOutput| x.stay_id).inv().collect();
    let fgr = rel(fdg.to_vec());
    let fgby: HashIdx<i64, usize> = (&fgr).map(|x: FirstDayGcs| x.stay_id).inv().collect();

    let cohort = (&icu.subject_id)
        .and(&icu.hadm_id)
        .and(&icu.stay_id)
        .and((&icu.hadm).select(&db.admission.hadm_id))
        .and((&icu.subject).select(&db.patient.subject_id))
        .and((&pafi).opt())
        .and((&icu.stay_id).select(&fgby).select(&fgr).opt())
        .and((&icu.stay_id).select(&fvby).select(&fvr).opt())
        .and((&icu.stay_id).select(&fuby).select(&fur).opt())
        .and((&icu.stay_id).select(&flby).select(&flr).opt());
    drain(&cohort)
        .into_iter()
        .map(|(_, (((((((((subject_id, hadm_id), stay_id), _), _), pf), g), v), uo), l))| {
            let pf = pf.flatten();
            let gcs = g.and_then(|g: FirstDayGcs| g.gcs_min);
            let hr_max = v.and_then(|v: FirstDayVitalsign| v.heart_rate_max);
            let hr_min = v.and_then(|v: FirstDayVitalsign| v.heart_rate_min);
            let sbp_max = v.and_then(|v: FirstDayVitalsign| v.sbp_max);
            let sbp_min = v.and_then(|v: FirstDayVitalsign| v.sbp_min);
            let uo = uo.and_then(|u: FirstDayUrineOutput| u.urineoutput);
            let lab = |f: fn(FirstDayLab) -> Option<f64>| l.and_then(f);
            let bun_max = lab(|l| l.bun_max);
            let wbc_max = lab(|l| l.wbc_max);
            let wbc_min = lab(|l| l.wbc_min);
            let bili = lab(|l| l.bilirubin_total_max);
            let creat = lab(|l| l.creatinine_max);
            let pt_min = lab(|l| l.pt_min);
            let pt_max = lab(|l| l.pt_max);
            let plt = lab(|l| l.platelets_min);

            let neurologic = match gcs {
                None => None,
                Some(x) if x < 3.0 => None,
                Some(x) if x <= 5.0 => Some(5),
                Some(x) if x <= 8.0 => Some(3),
                Some(x) if x <= 13.0 => Some(1),
                _ => Some(0),
            };
            let cardiovascular = if hr_max.is_none() && sbp_min.is_none() {
                None
            } else if lt(hr_min, 30.0) || lt(sbp_min, 40.0) {
                Some(5)
            } else if lt(sbp_min, 70.0) || ge(sbp_max, 270.0) {
                Some(3)
            } else if ge(hr_max, 140.0) || ge(sbp_max, 240.0) || lt(sbp_min, 90.0) {
                Some(1)
            } else {
                Some(0)
            };
            let renal = if bun_max.is_none() || uo.is_none() || creat.is_none() {
                None
            } else if lt(uo, 500.0) || ge(bun_max, 56.0) {
                Some(5)
            } else if ge(creat, 1.60) || lt(uo, 750.0) || ge(bun_max, 28.0) || ge(uo, 10000.0) {
                Some(3)
            } else if ge(creat, 1.20) || ge(bun_max, 17.0) || ge(bun_max, 7.50) {
                Some(1)
            } else {
                Some(0)
            };
            let pulmonary = match pf {
                None => Some(0),
                Some(x) if x >= 150.0 => Some(1),
                Some(x) if x < 150.0 => Some(3),
                _ => None,
            };
            let hematologic = if wbc_max.is_none() && plt.is_none() {
                None
            } else if lt(wbc_min, 1.0) {
                Some(3)
            } else if lt(wbc_min, 2.5) || lt(plt, 50.0) || ge(wbc_max, 50.0) {
                Some(1)
            } else {
                Some(0)
            };
            let hepatic = if pt_max.is_none() && bili.is_none() {
                None
            } else if ge(bili, 2.0) || gt(pt_max, 12.0 + 3.0) || lt(pt_min, 12.0 * 0.25) {
                Some(1)
            } else {
                Some(0)
            };
            let lods = [neurologic, cardiovascular, renal, pulmonary, hematologic, hepatic]
                .iter()
                .map(|x| x.unwrap_or(0))
                .sum();
            Lods { subject_id, hadm_id, stay_id, lods, neurologic, cardiovascular, renal, pulmonary, hematologic, hepatic }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let bg = crate::concepts::bg::bg(db);
    let gs = crate::concepts::gcs::gcs(db);
    let fdg = crate::concepts::first_day_gcs::first_day_gcs(db, &gs);
    let diff = crate::concepts::blood_differential::blood_differential(db);
    let chem = crate::concepts::chemistry::chemistry(db);
    let coag = crate::concepts::coagulation::coagulation(db);
    let cbc = crate::concepts::complete_blood_count::complete_blood_count(db);
    let enz = crate::concepts::enzyme::enzyme(db);
    let fdl = crate::concepts::first_day_lab::first_day_lab(db, &cbc, &chem, &diff, &coag, &enz);
    let uo = crate::concepts::urine_output::urine_output(db);
    let fdu = crate::concepts::first_day_urine_output::first_day_urine_output(db, &uo);
    let vs = crate::concepts::vitalsign::vitalsign(db);
    let fdv = crate::concepts::first_day_vitalsign::first_day_vitalsign(db, &vs);
    let od = crate::concepts::oxygen_delivery::oxygen_delivery(db);
    let vset = crate::concepts::ventilator_setting::ventilator_setting(db);
    let vent = crate::concepts::ventilation::ventilation(db, &vset, &od);
    rows(lods(db, &bg, &vent, &fdv, &fdl, &fdu, &fdg).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.hadm_id),
            V::I(v.stay_id),
            V::I(v.lods),
            oint(v.neurologic),
            oint(v.cardiovascular),
            oint(v.renal),
            oint(v.pulmonary),
            oint(v.hematologic),
            oint(v.hepatic),
        ])
    }))
}
