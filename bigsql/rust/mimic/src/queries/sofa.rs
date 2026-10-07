use crate::concepts::bg::*;
use crate::concepts::chemistry::*;
use crate::concepts::complete_blood_count::*;
use crate::concepts::dobutamine::*;
use crate::concepts::dopamine::*;
use crate::concepts::enzyme::*;
use crate::concepts::epinephrine::*;
use crate::concepts::gcs::*;
use crate::concepts::icustay_hourly::*;
use crate::concepts::norepinephrine::*;
use crate::concepts::urine_output_rate::*;
use crate::concepts::ventilation::*;
use crate::concepts::vitalsign::*;
use crate::schema::*;
use harness::prelude::*;
use std::hash::Hash;

#[derive(Clone, Copy)]
pub struct Sofa {
    pub stay_id: i64,
    pub hr: i64,
    pub starttime: Ts,
    pub endtime: Ts,
    pub pao2fio2ratio_novent: Option<f64>,
    pub pao2fio2ratio_vent: Option<f64>,
    pub rate_epinephrine: Option<f64>,
    pub rate_norepinephrine: Option<f64>,
    pub rate_dopamine: Option<f64>,
    pub rate_dobutamine: Option<f64>,
    pub meanbp_min: Option<f64>,
    pub gcs_min: Option<f64>,
    pub uo_24hr: Option<f64>,
    pub bilirubin_max: Option<f64>,
    pub creatinine_max: Option<f64>,
    pub platelet_min: Option<f64>,
    pub respiration: Option<i64>,
    pub coagulation: Option<i64>,
    pub liver: Option<i64>,
    pub cardiovascular: Option<i64>,
    pub cns: Option<i64>,
    pub renal: Option<i64>,
    pub respiration_24hours: i64,
    pub coagulation_24hours: i64,
    pub liver_24hours: i64,
    pub cardiovascular_24hours: i64,
    pub cns_24hours: i64,
    pub renal_24hours: i64,
    pub sofa_24hours: i64,
}

#[derive(Clone, Copy)]
struct Co {
    stay_id: i64,
    hadm_id: i64,
    hr: i64,
    starttime: Ts,
    endtime: Ts,
}

#[derive(Clone, Copy)]
struct Pafi {
    stay_id: i64,
    charttime: Ts,
    ratio: Option<f64>,
}

const HOUR_US: i64 = 3600 * US;

fn omin(a: Option<f64>, x: Option<f64>) -> Option<f64> {
    match (a, x) {
        (Some(a), Some(x)) => Some(a.min(x)),
        _ => a.or(x),
    }
}

fn omax(a: Option<f64>, x: Option<f64>) -> Option<f64> {
    match (a, x) {
        (Some(a), Some(x)) => Some(a.max(x)),
        _ => a.or(x),
    }
}

fn agg<T: Copy + 'static, K: Copy + Eq + Hash + 'static, A: Copy + 'static>(
    cor: &VecRel<usize, Co>,
    src: &[T],
    kc: impl Fn(Co) -> K + Copy,
    kt: impl Fn(T) -> K + Copy,
    hit: impl Fn(Co, T) -> bool + Copy,
    init: A,
    op: impl Fn(A, T) -> A + Copy,
) -> Fold<usize, A> {
    let tr = rel(src.to_vec());
    let by: HashIdx<K, usize> = (&tr).map(kt).inv().collect();
    cor.and(cor.map(kc).select(&by).select(&tr))
        .filt(move |(c, t): (Co, T)| hit(c, t))
        .map(|(_, t): (Co, T)| t)
        .fold(init, op)
}

fn hits<T: Copy + 'static>(
    cor: &VecRel<usize, Co>,
    src: &[T],
    kt: impl Fn(T) -> i64 + Copy,
    hit: impl Fn(Co, T) -> bool + Copy,
) -> HashIdx<usize, T> {
    let tr = rel(src.to_vec());
    let by: HashIdx<i64, usize> = (&tr).map(kt).inv().collect();
    cor.and(cor.map(|c: Co| c.stay_id).select(&by).select(&tr))
        .filt(move |(c, t): (Co, T)| hit(c, t))
        .map(|(_, t): (Co, T)| t)
        .collect()
}

fn during(c: Co, t: Ts) -> bool {
    c.starttime < t && c.endtime >= t
}

fn infusing(c: Co, s: Ts, e: Ts) -> bool {
    c.endtime > s && c.endtime <= e
}

fn wmax(w: &[(i64, Sofa)], f: impl Fn(&Sofa) -> Option<i64>) -> i64 {
    w.iter().filter_map(|(_, s)| f(s)).max().unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
pub fn sofa(
    db: &'static Db,
    ih: &[IcustayHourly],
    bg: &[Bg],
    vent: &[Ventilation],
    vs: &[Vitalsign],
    gcs: &[Gcs],
    enz: &[Enzyme],
    chem: &[Chemistry],
    cbc: &[CompleteBloodCount],
    uor: &[UrineOutputRate],
    epi: &[Epinephrine],
    nor: &[Norepinephrine],
    dop: &[Dopamine],
    dob: &[Dobutamine],
) -> Vec<Sofa> {
    let ie = &db.icu_stay;
    let sby: HashIdx<i64, Id<IcuStay>> = (&ie.stay_id).inv().collect();
    let ihr = rel(ih.to_vec());
    let co = (&ihr).and((&ihr).map(|h: IcustayHourly| h.stay_id).select(&sby).select(&ie.hadm_id));
    let cor = rel(drain(&co)
        .into_iter()
        .map(|(_, (h, hadm_id))| Co {
            stay_id: h.stay_id,
            hadm_id,
            hr: h.hr,
            starttime: h.endtime - HOUR_US,
            endtime: h.endtime,
        })
        .collect::<Vec<Co>>());

    let bgr = rel(bg.to_vec());
    let bby: HashIdx<i64, usize> =
        (&bgr).filt(|b: Bg| b.specimen == Some("ART.")).map(|b: Bg| b.subject_id).inv().collect();
    let pairs = (&ie.stay_id).and((&ie.subject_id).select(&bby).select(&bgr));
    let pr = rel(drain(&pairs)
        .into_iter()
        .map(|(_, (stay_id, b))| Pafi { stay_id, charttime: b.charttime, ratio: b.pao2fio2ratio })
        .collect::<Vec<Pafi>>());
    let vr = rel(vent.to_vec());
    let vby: HashIdx<i64, usize> = (&vr)
        .filt(|v: Ventilation| v.ventilation_status == "InvasiveVent")
        .map(|v: Ventilation| v.stay_id)
        .inv()
        .collect();
    let onvent = (&pr)
        .and((&pr).map(|p: Pafi| p.stay_id).select(&vby).select(&vr))
        .filt(|(p, v): (Pafi, Ventilation)| p.charttime >= v.starttime && p.charttime <= v.endtime)
        .fold((), |_, _| ());
    let pafi: Vec<(Pafi, bool)> =
        drain((&pr).and((&onvent).opt())).into_iter().map(|(_, (p, v))| (p, v.is_some())).collect();

    let f_vs = agg(&cor, vs, |c| c.stay_id, |v: Vitalsign| v.stay_id, |c, v| during(c, v.charttime), None, |a, v| {
        omin(a, v.mbp)
    });
    let f_gcs = agg(&cor, gcs, |c| c.stay_id, |g: Gcs| g.stay_id, |c, g| during(c, g.charttime), None, |a, g| {
        omin(a, g.gcs)
    });
    let f_bili = agg(&cor, enz, |c| Some(c.hadm_id), |e: Enzyme| e.hadm_id, |c, e| during(c, e.charttime), None, |a, e| {
        omax(a, e.bilirubin_total)
    });
    let f_cr = agg(&cor, chem, |c| Some(c.hadm_id), |e: Chemistry| e.hadm_id, |c, e| during(c, e.charttime), None, |a, e| {
        omax(a, e.creatinine)
    });
    let f_plt = agg(
        &cor,
        cbc,
        |c| Some(c.hadm_id),
        |e: CompleteBloodCount| e.hadm_id,
        |c, e| during(c, e.charttime),
        None,
        |a, e| omin(a, e.platelet),
    );
    let f_pf = agg(
        &cor,
        &pafi,
        |c| c.stay_id,
        |(p, _): (Pafi, bool)| p.stay_id,
        |c, (p, _)| during(c, p.charttime),
        (None, None),
        |(nv, v), (p, on)| if on { (nv, omin(v, p.ratio)) } else { (omin(nv, p.ratio), v) },
    );
    let f_uo = agg(&cor, uor, |c| c.stay_id, |u: UrineOutputRate| u.stay_id, |c, u| during(c, u.charttime), None, |a, u| {
        let x = match (u.uo_tm_24hr, u.urineoutput_24hr) {
            (Some(t), Some(o)) if t >= 22.0 && t <= 30.0 => Some(o / t * 24.0),
            _ => None,
        };
        omax(a, x)
    });
    let h_epi = hits(&cor, epi, |v: Epinephrine| v.stay_id, |c, v| infusing(c, v.starttime, v.endtime));
    let h_nor = hits(&cor, nor, |v: Norepinephrine| v.stay_id, |c, v| infusing(c, v.starttime, v.endtime));
    let h_dop = hits(&cor, dop, |v: Dopamine| v.stay_id, |c, v| infusing(c, v.starttime, v.endtime));
    let h_dob = hits(&cor, dob, |v: Dobutamine| v.stay_id, |c, v| infusing(c, v.starttime, v.endtime));
    type Vx = ((((Co, Option<Epinephrine>), Option<Norepinephrine>), Option<Dopamine>), Option<Dobutamine>);
    type Vr = (Option<f64>, Option<f64>, Option<f64>, Option<f64>);
    let f_vaso = (&cor)
        .and((&h_epi).opt())
        .and((&h_nor).opt())
        .and((&h_dop).opt())
        .and((&h_dob).opt())
        .filt(|((((_, e), n), d), b): Vx| e.is_some() || n.is_some() || d.is_some() || b.is_some())
        .fold((None, None, None, None), |(a, b2, c, d2): Vr, ((((_, e), n), d), b): Vx| {
            (
                omax(a, e.and_then(|v| v.vaso_rate)),
                omax(b2, n.and_then(|v| v.vaso_rate)),
                omax(c, d.and_then(|v| v.vaso_rate)),
                omax(d2, b.and_then(|v| v.vaso_rate)),
            )
        });

    let comp = (&cor)
        .and((&f_vs).opt())
        .and((&f_gcs).opt())
        .and((&f_bili).opt())
        .and((&f_cr).opt())
        .and((&f_plt).opt())
        .and((&f_pf).opt())
        .and((&f_uo).opt())
        .and((&f_vaso).opt());
    let calc: Vec<Sofa> = drain(&comp)
        .into_iter()
        .map(|(_, ((((((((c, vs), gcs), bili), cr), plt), pf), uo), vaso))| {
            let (nv, v) = pf.unwrap_or((None, None));
            let f = |x: Option<Option<f64>>| x.flatten();
            let (meanbp_min, gcs_min, bilirubin_max, creatinine_max, platelet_min, uo_24hr) =
                (f(vs), f(gcs), f(bili), f(cr), f(plt), f(uo));
            let (epi, nor, dop, dob) = vaso.unwrap_or((None, None, None, None));
            let lt = |x: Option<f64>, k: f64| x.is_some_and(|x| x < k);
            let ge = |x: Option<f64>, k: f64| x.is_some_and(|x| x >= k);
            let fgt = |x: Option<f64>, k: f32| x.is_some_and(|x| x as f32 > k);
            let fle = |x: Option<f64>, k: f32| x.is_some_and(|x| x as f32 <= k);
            let respiration = if lt(v, 100.0) {
                Some(4)
            } else if lt(v, 200.0) {
                Some(3)
            } else if lt(nv, 300.0) || lt(v, 300.0) {
                Some(2)
            } else if lt(nv, 400.0) || lt(v, 400.0) {
                Some(1)
            } else if v.or(nv).is_none() {
                None
            } else {
                Some(0)
            };
            let coagulation = match platelet_min {
                Some(p) if p < 20.0 => Some(4),
                Some(p) if p < 50.0 => Some(3),
                Some(p) if p < 100.0 => Some(2),
                Some(p) if p < 150.0 => Some(1),
                None => None,
                _ => Some(0),
            };
            let liver = match bilirubin_max {
                Some(b) if b >= 12.0 => Some(4),
                Some(b) if b >= 6.0 => Some(3),
                Some(b) if b >= 2.0 => Some(2),
                Some(b) if b >= 1.2 => Some(1),
                None => None,
                _ => Some(0),
            };
            let cardiovascular = if fgt(dop, 15.0) || fgt(epi, 0.1) || fgt(nor, 0.1) {
                Some(4)
            } else if fgt(dop, 5.0) || fle(epi, 0.1) || fle(nor, 0.1) {
                Some(3)
            } else if fgt(dop, 0.0) || fgt(dob, 0.0) {
                Some(2)
            } else if lt(meanbp_min, 70.0) {
                Some(1)
            } else if meanbp_min.or(dop).or(dob).or(epi).or(nor).is_none() {
                None
            } else {
                Some(0)
            };
            let cns = match gcs_min {
                Some(g) if (13.0..=14.0).contains(&g) => Some(1),
                Some(g) if (10.0..=12.0).contains(&g) => Some(2),
                Some(g) if (6.0..=9.0).contains(&g) => Some(3),
                Some(g) if g < 6.0 => Some(4),
                None => None,
                _ => Some(0),
            };
            let renal = if ge(creatinine_max, 5.0) || lt(uo_24hr, 200.0) {
                Some(4)
            } else if (ge(creatinine_max, 3.5) && lt(creatinine_max, 5.0)) || lt(uo_24hr, 500.0) {
                Some(3)
            } else if ge(creatinine_max, 2.0) && lt(creatinine_max, 3.5) {
                Some(2)
            } else if ge(creatinine_max, 1.2) && lt(creatinine_max, 2.0) {
                Some(1)
            } else if uo_24hr.or(creatinine_max).is_none() {
                None
            } else {
                Some(0)
            };
            Sofa {
                stay_id: c.stay_id,
                hr: c.hr,
                starttime: c.starttime,
                endtime: c.endtime,
                pao2fio2ratio_novent: nv,
                pao2fio2ratio_vent: v,
                rate_epinephrine: epi,
                rate_norepinephrine: nor,
                rate_dopamine: dop,
                rate_dobutamine: dob,
                meanbp_min,
                gcs_min,
                uo_24hr,
                bilirubin_max,
                creatinine_max,
                platelet_min,
                respiration,
                coagulation,
                liver,
                cardiovascular,
                cns,
                renal,
                respiration_24hours: 0,
                coagulation_24hours: 0,
                liver_24hours: 0,
                cardiovascular_24hours: 0,
                cns_24hours: 0,
                renal_24hours: 0,
                sofa_24hours: 0,
            }
        })
        .collect();

    let sr = rel(calc);
    let w = (&sr).map(|s: Sofa| s.stay_id).inv().select(&sr).window(
        |s: &[(i64, Sofa)], out: &mut Vec<[i64; 6]>| {
            for i in 0..s.len() {
                let win = &s[i.saturating_sub(23)..=i];
                out.push([
                    wmax(win, |x| x.respiration),
                    wmax(win, |x| x.coagulation),
                    wmax(win, |x| x.liver),
                    wmax(win, |x| x.cardiovascular),
                    wmax(win, |x| x.cns),
                    wmax(win, |x| x.renal),
                ]);
            }
        },
        |s: Sofa| s.hr,
        |a: &i64, b: &i64| a.cmp(b),
    );
    let fin = (&w).filt(|(s, _): (Sofa, [i64; 6])| s.hr >= 0);
    drain(&fin)
        .into_iter()
        .map(|(_, (s, m))| Sofa {
            respiration_24hours: m[0],
            coagulation_24hours: m[1],
            liver_24hours: m[2],
            cardiovascular_24hours: m[3],
            cns_24hours: m[4],
            renal_24hours: m[5],
            sofa_24hours: m.iter().sum(),
            ..s
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    use crate::concepts;
    let bg = concepts::bg::bg(db);
    let chem = concepts::chemistry::chemistry(db);
    let cbc = concepts::complete_blood_count::complete_blood_count(db);
    let dob = concepts::dobutamine::dobutamine(db);
    let dop = concepts::dopamine::dopamine(db);
    let enz = concepts::enzyme::enzyme(db);
    let epi = concepts::epinephrine::epinephrine(db);
    let gcs = concepts::gcs::gcs(db);
    let it = concepts::icustay_times::icustay_times(db);
    let ih = concepts::icustay_hourly::icustay_hourly(db, &it);
    let nor = concepts::norepinephrine::norepinephrine(db);
    let uo = concepts::urine_output::urine_output(db);
    let wd = concepts::weight_durations::weight_durations(db);
    let uor = concepts::urine_output_rate::urine_output_rate(db, &uo, &wd);
    let od = concepts::oxygen_delivery::oxygen_delivery(db);
    let vset = concepts::ventilator_setting::ventilator_setting(db);
    let vent = concepts::ventilation::ventilation(db, &vset, &od);
    let vs = concepts::vitalsign::vitalsign(db);
    let s = sofa(db, &ih, &bg, &vent, &vs, &gcs, &enz, &chem, &cbc, &uor, &epi, &nor, &dop, &dob);
    rows(s.into_iter().map(|s| {
        row(vec![
            V::I(s.stay_id),
            V::I(s.hr),
            V::T(s.starttime),
            V::T(s.endtime),
            ofloat(s.pao2fio2ratio_novent),
            ofloat(s.pao2fio2ratio_vent),
            ofloat(s.rate_epinephrine),
            ofloat(s.rate_norepinephrine),
            ofloat(s.rate_dopamine),
            ofloat(s.rate_dobutamine),
            ofloat(s.meanbp_min),
            ofloat(s.gcs_min),
            ofloat(s.uo_24hr),
            ofloat(s.bilirubin_max),
            ofloat(s.creatinine_max),
            ofloat(s.platelet_min),
            oint(s.respiration),
            oint(s.coagulation),
            oint(s.liver),
            oint(s.cardiovascular),
            oint(s.cns),
            oint(s.renal),
            V::I(s.respiration_24hours),
            V::I(s.coagulation_24hours),
            V::I(s.liver_24hours),
            V::I(s.cardiovascular_24hours),
            V::I(s.cns_24hours),
            V::I(s.renal_24hours),
            V::I(s.sofa_24hours),
        ])
    }))
}
