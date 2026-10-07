use crate::concepts::{
    bg::*, dobutamine::*, dopamine::*, epinephrine::*, first_day_gcs::*, first_day_lab::*, first_day_urine_output::*,
    first_day_vitalsign::*, norepinephrine::*, ventilation::*,
};
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct FirstDaySofa {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub stay_id: i64,
    pub sofa: i64,
    pub respiration: Option<i64>,
    pub coagulation: Option<i64>,
    pub liver: Option<i64>,
    pub cardiovascular: Option<i64>,
    pub cns: Option<i64>,
    pub renal: Option<i64>,
}

const H: i64 = 3600 * US;

type Stg = (i64, (usize, (Ts, Option<f64>)));
type Pafi = (i64, (Ts, Option<f64>));

fn omax(a: Option<f64>, x: Option<f64>) -> Option<f64> {
    match (a, x) {
        (Some(a), Some(x)) => Some(a.max(x)),
        (a, None) => a,
        (None, x) => x,
    }
}

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

fn fgt(x: Option<f64>, c: f32) -> bool {
    x.is_some_and(|x| x as f32 > c)
}

fn fle(x: Option<f64>, c: f32) -> bool {
    x.is_some_and(|x| x as f32 <= c)
}

#[allow(clippy::too_many_arguments)]
pub fn first_day_sofa(
    db: &'static Db,
    bg: &[Bg],
    ne: &[Norepinephrine],
    ep: &[Epinephrine],
    dob: &[Dobutamine],
    dop: &[Dopamine],
    vent: &[Ventilation],
    fdv: &[FirstDayVitalsign],
    fdl: &[FirstDayLab],
    fdu: &[FirstDayUrineOutput],
    fdg: &[FirstDayGcs],
) -> Vec<FirstDaySofa> {
    let icu = &db.icu_stay;

    let ner = rel(ne.to_vec());
    let epr = rel(ep.to_vec());
    let dobr = rel(dob.to_vec());
    let dopr = rel(dop.to_vec());
    let stg_u = (&ner)
        .map(|m: Norepinephrine| -> Stg { (m.stay_id, (0, (m.starttime, m.vaso_rate))) })
        .union((&epr).map(|m: Epinephrine| -> Stg { (m.stay_id, (1, (m.starttime, m.vaso_rate))) }))
        .union((&dobr).map(|m: Dobutamine| -> Stg { (m.stay_id, (3, (m.starttime, m.vaso_rate))) }))
        .union((&dopr).map(|m: Dopamine| -> Stg { (m.stay_id, (2, (m.starttime, m.vaso_rate))) }));
    let sr = rel(drain(&stg_u).into_iter().map(|(_, s)| s).collect());
    let sby: HashIdx<i64, usize> = (&sr).map(|s: Stg| s.0).inv().collect();
    let vaso = (&icu.intime)
        .and((&icu.stay_id).select(&sby).select(&sr))
        .filt(|(i, (_, (_, (t, _)))): (Ts, Stg)| t >= i - 6 * H && t <= i + DAY_US)
        .fold([None::<f64>; 4], |mut a, (_, (_, (k, (_, r)))): (Ts, Stg)| {
            a[k] = omax(a[k], r);
            a
        });

    let br = rel(bg.to_vec());
    let bby: HashIdx<i64, usize> = (&br).map(|b: Bg| b.subject_id).inv().collect();
    let pairs = (&icu.intime)
        .and(&icu.stay_id)
        .and((&icu.subject_id).select(&bby).select(&br))
        .filt(|((i, _), b): ((Ts, i64), Bg)| {
            b.charttime >= i - 6 * H && b.charttime <= i + DAY_US && b.specimen == Some("ART.")
        })
        .map(|((_, s), b): ((Ts, i64), Bg)| -> Pafi { (s, (b.charttime, b.pao2fio2ratio)) });
    let pr = rel(drain(&pairs).into_iter().map(|(_, p)| p).collect());
    let vr = rel(vent.to_vec());
    let vby: HashIdx<i64, usize> = (&vr)
        .filt(|v: Ventilation| v.ventilation_status == "InvasiveVent")
        .map(|v: Ventilation| v.stay_id)
        .inv()
        .collect();
    let hit: HashIdx<usize, ()> = (&pr)
        .and((&pr).map(|p: Pafi| p.0).select(&vby).select(&vr))
        .filt(|((_, (t, _)), v): (Pafi, Ventilation)| t >= v.starttime && t <= v.endtime)
        .map(|_| ())
        .collect();
    let p1 = (&pr).and((&hit).opt());
    let pby: HashIdx<i64, usize> = (&pr).map(|p: Pafi| p.0).inv().collect();
    let pafi = (&icu.stay_id)
        .select(&pby)
        .select(&p1)
        .fold((None::<f64>, None::<f64>), |(nv, v), ((_, (_, r)), h): (Pafi, Option<()>)| {
            if h.is_some() { (nv, omin(v, r)) } else { (omin(nv, r), v) }
        });

    let fvr = rel(fdv.to_vec());
    let fvby: HashIdx<i64, usize> = (&fvr).map(|x: FirstDayVitalsign| x.stay_id).inv().collect();
    let flr = rel(fdl.to_vec());
    let flby: HashIdx<i64, usize> = (&flr).map(|x: FirstDayLab| x.stay_id).inv().collect();
    let fur = rel(fdu.to_vec());
    let fuby: HashIdx<i64, usize> = (&fur).map(|x: FirstDayUrineOutput| x.stay_id).inv().collect();
    let fgr = rel(fdg.to_vec());
    let fgby: HashIdx<i64, usize> = (&fgr).map(|x: FirstDayGcs| x.stay_id).inv().collect();

    let comp = (&icu.subject_id)
        .and(&icu.hadm_id)
        .and(&icu.stay_id)
        .and((&vaso).opt())
        .and((&pafi).opt())
        .and((&icu.stay_id).select(&fvby).select(&fvr).opt())
        .and((&icu.stay_id).select(&flby).select(&flr).opt())
        .and((&icu.stay_id).select(&fuby).select(&fur).opt())
        .and((&icu.stay_id).select(&fgby).select(&fgr).opt());
    drain(&comp)
        .into_iter()
        .map(|(_, ((((((((subject_id, hadm_id), stay_id), mv), pf), v), l), uo), g))| {
            let [ne, ep, dop, dob] = mv.unwrap_or([None; 4]);
            let (novent, vent) = pf.unwrap_or((None, None));
            let mbp_min = v.and_then(|v: FirstDayVitalsign| v.mbp_min);
            let creat = l.and_then(|l: FirstDayLab| l.creatinine_max);
            let bili = l.and_then(|l: FirstDayLab| l.bilirubin_total_max);
            let plt = l.and_then(|l: FirstDayLab| l.platelets_min);
            let uo = uo.and_then(|u: FirstDayUrineOutput| u.urineoutput);
            let gcs = g.and_then(|g: FirstDayGcs| g.gcs_min);

            let respiration = if lt(vent, 100.0) {
                Some(4)
            } else if lt(vent, 200.0) {
                Some(3)
            } else if lt(novent, 300.0) {
                Some(2)
            } else if lt(novent, 400.0) {
                Some(1)
            } else if vent.or(novent).is_none() {
                None
            } else {
                Some(0)
            };
            let coagulation = if lt(plt, 20.0) {
                Some(4)
            } else if lt(plt, 50.0) {
                Some(3)
            } else if lt(plt, 100.0) {
                Some(2)
            } else if lt(plt, 150.0) {
                Some(1)
            } else {
                plt.map(|_| 0)
            };
            let liver = if ge(bili, 12.0) {
                Some(4)
            } else if ge(bili, 6.0) {
                Some(3)
            } else if ge(bili, 2.0) {
                Some(2)
            } else if ge(bili, 1.2) {
                Some(1)
            } else {
                bili.map(|_| 0)
            };
            let cardiovascular = if fgt(dop, 15.0) || fgt(ep, 0.1) || fgt(ne, 0.1) {
                Some(4)
            } else if fgt(dop, 5.0) || fle(ep, 0.1) || fle(ne, 0.1) {
                Some(3)
            } else if fgt(dop, 0.0) || fgt(dob, 0.0) {
                Some(2)
            } else if lt(mbp_min, 70.0) {
                Some(1)
            } else if mbp_min.or(dop).or(dob).or(ep).or(ne).is_none() {
                None
            } else {
                Some(0)
            };
            let cns = match gcs {
                Some(x) if (13.0..=14.0).contains(&x) => Some(1),
                Some(x) if (10.0..=12.0).contains(&x) => Some(2),
                Some(x) if (6.0..=9.0).contains(&x) => Some(3),
                Some(x) if x < 6.0 => Some(4),
                None => None,
                _ => Some(0),
            };
            let renal = if ge(creat, 5.0) || lt(uo, 200.0) {
                Some(4)
            } else if (ge(creat, 3.5) && lt(creat, 5.0)) || lt(uo, 500.0) {
                Some(3)
            } else if ge(creat, 2.0) && lt(creat, 3.5) {
                Some(2)
            } else if ge(creat, 1.2) && lt(creat, 2.0) {
                Some(1)
            } else if uo.or(creat).is_none() {
                None
            } else {
                Some(0)
            };
            let sofa = [respiration, coagulation, liver, cardiovascular, cns, renal]
                .iter()
                .map(|x| x.unwrap_or(0))
                .sum();
            FirstDaySofa { subject_id, hadm_id, stay_id, sofa, respiration, coagulation, liver, cardiovascular, cns, renal }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let bg = crate::concepts::bg::bg(db);
    let dob = crate::concepts::dobutamine::dobutamine(db);
    let dop = crate::concepts::dopamine::dopamine(db);
    let ep = crate::concepts::epinephrine::epinephrine(db);
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
    let ne = crate::concepts::norepinephrine::norepinephrine(db);
    let od = crate::concepts::oxygen_delivery::oxygen_delivery(db);
    let vset = crate::concepts::ventilator_setting::ventilator_setting(db);
    let vent = crate::concepts::ventilation::ventilation(db, &vset, &od);
    rows(first_day_sofa(db, &bg, &ne, &ep, &dob, &dop, &vent, &fdv, &fdl, &fdu, &fdg).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.hadm_id),
            V::I(v.stay_id),
            V::I(v.sofa),
            oint(v.respiration),
            oint(v.coagulation),
            oint(v.liver),
            oint(v.cardiovascular),
            oint(v.cns),
            oint(v.renal),
        ])
    }))
}
