use crate::concepts::age::*;
use crate::concepts::bg::*;
use crate::concepts::chemistry::*;
use crate::concepts::complete_blood_count::*;
use crate::concepts::enzyme::*;
use crate::concepts::gcs::*;
use crate::concepts::urine_output::*;
use crate::concepts::ventilation::*;
use crate::concepts::vitalsign::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Sapsii {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub stay_id: i64,
    pub starttime: Ts,
    pub endtime: Ts,
    pub sapsii: i64,
    pub sapsii_prob: f64,
    pub age_score: Option<i64>,
    pub hr_score: Option<i64>,
    pub sysbp_score: Option<i64>,
    pub temp_score: Option<i64>,
    pub pao2fio2_score: Option<i64>,
    pub uo_score: Option<i64>,
    pub bun_score: Option<i64>,
    pub wbc_score: Option<i64>,
    pub potassium_score: Option<i64>,
    pub sodium_score: Option<i64>,
    pub bicarbonate_score: Option<i64>,
    pub bilirubin_score: Option<i64>,
    pub gcs_score: Option<i64>,
    pub comorbidity_score: i64,
    pub admissiontype_score: i64,
}

#[derive(Clone, Copy)]
struct Cpap {
    subject_id: i64,
    starttime: Ts,
    endtime: Ts,
}

#[derive(Clone, Copy)]
struct P1 {
    stay_id: i64,
    subject_id: i64,
    charttime: Ts,
    pao2fio2: Option<f64>,
}

#[derive(Clone, Copy)]
struct Mm {
    min: Option<f64>,
    max: Option<f64>,
    sum: (f64, f64),
    n: i64,
}

const MM0: Mm = Mm { min: None, max: None, sum: (0.0, 0.0), n: 0 };
const HOUR_US: i64 = 3600 * US;

fn win<R: Copy + 'static, const N: usize>(
    db: &'static Db,
    v: &[R],
    key_col: &'static Col<IcuStay, i64>,
    key: fn(R) -> i64,
    t: fn(R) -> Ts,
    vals: fn(R) -> [Option<f64>; N],
) -> HashIdx<Id<IcuStay>, [Mm; N]> {
    let r = rel(v.to_vec());
    let by: HashIdx<i64, usize> = (&r).map(key).inv().collect();
    (&db.icu_stay.intime)
        .and(key_col.select(&by).select(&r))
        .filt(move |(i, x): (Ts, R)| {
            let c = t(x);
            c > i && c <= i + DAY_US
        })
        .fold([MM0; N], move |mut a, (_, x): (Ts, R)| {
            for (s, c) in a.iter_mut().zip(vals(x)) {
                if let Some(c) = c {
                    s.min = Some(s.min.map_or(c, |m| m.min(c)));
                    s.max = Some(s.max.map_or(c, |m| m.max(c)));
                    s.sum = kahan(s.sum, c);
                    s.n += 1;
                }
            }
            a
        })
        .collect()
}

fn sub(s: &str, n: usize) -> &str {
    &s[..n.min(s.len())]
}

fn between(s: &str, lo: &str, hi: &str) -> bool {
    s >= lo && s <= hi
}

fn comorb_flags(version: i64, code: &str) -> (i64, i64, i64) {
    let (s3, s4, s5) = (sub(code, 3), sub(code, 4), sub(code, 5));
    let aids = (version == 9 && between(s3, "042", "044"))
        || (version == 10 && between(s3, "B20", "B22"))
        || (version == 10 && s3 == "B24");
    let hem = if version == 9 {
        between(s5, "20000", "20238")
            || between(s5, "20240", "20248")
            || between(s5, "20250", "20302")
            || between(s5, "20310", "20312")
            || between(s5, "20302", "20382")
            || between(s5, "20400", "20522")
            || between(s5, "20580", "20702")
            || between(s5, "20720", "20892")
            || s4 == "2386"
            || s4 == "2733"
    } else {
        version == 10 && between(s3, "C81", "C96")
    };
    let mets = if version == 9 {
        between(s4, "1960", "1991") || between(s5, "20970", "20975") || s5 == "20979" || s5 == "78951"
    } else {
        (version == 10 && between(s3, "C77", "C79")) || (version == 10 && s4 == "C800")
    };
    (aids as i64, hem as i64, mets as i64)
}

#[allow(clippy::too_many_arguments)]
pub fn sapsii(
    db: &'static Db,
    ag: &[Age],
    bg: &[Bg],
    chem: &[Chemistry],
    cbc: &[CompleteBloodCount],
    enz: &[Enzyme],
    gs: &[Gcs],
    uo: &[UrineOutput],
    vent: &[Ventilation],
    vs: &[Vitalsign],
) -> Vec<Sapsii> {
    let icu = &db.icu_stay;
    let ad = &db.admission;
    let se = &db.service;
    let ce = &db.chart_event;
    let dx = &db.diagnoses_icd;

    let ce_by: HashIdx<Id<IcuStay>, Id<ChartEvent>> = (&ce.id)
        .with((&ce.itemid).is_in([226732]))
        .with((&ce.value).filt(|v: Str| {
            let l = v.to_lowercase();
            l.contains("cpap mask") || l.contains("bipap")
        }))
        .select(&ce.stay)
        .inv()
        .collect();
    let cp = (&icu.intime)
        .and((&ce_by).select(&ce.charttime))
        .filt(|(i, t): (Ts, Ts)| t > i && t <= i + DAY_US)
        .fold((i64::MAX, i64::MIN), |(lo, hi), (_, t): (Ts, Ts)| (lo.min(t), hi.max(t)));
    let cpv: Vec<Cpap> = drain((&icu.subject_id).and(&icu.intime).and(&cp))
        .into_iter()
        .map(|(_, ((subject_id, i), (lo, hi)))| Cpap {
            subject_id,
            starttime: (lo - HOUR_US).max(i),
            endtime: (hi + 4 * HOUR_US).min(i + DAY_US),
        })
        .collect();

    let se_by: HashIdx<Id<Admission>, Id<Service>> = (&se.hadm).inv().collect();
    let ranked = (&se_by)
        .select((&se.transfertime).and(&se.curr_service))
        .window(row_number, |(t, _): (Ts, Str)| t, |a: &Ts, b: &Ts| a.cmp(b));
    let first = (&ranked).filt(|(_, rn)| rn == 1);
    let surg: HashIdx<Id<IcuStay>, i64> = (&icu.hadm)
        .select(&first)
        .map(|((_, s), _): ((Ts, Str), i64)| like(&s.to_lowercase(), "%surg%") as i64)
        .collect();

    let comorb = (&dx.id)
        .group_by(&dx.hadm_id)
        .select((&dx.icd_version).and(&dx.icd_code))
        .fold((0i64, 0i64, 0i64), |(a, h, m), (ver, code): (i64, Str)| {
            let (x, y, z) = comorb_flags(ver, code);
            (a.max(x), h.max(y), m.max(z))
        });
    let comorb_s: HashIdx<Id<IcuStay>, (i64, i64, i64)> = (&icu.hadm_id).select(&comorb).collect();

    let bgr = rel(bg.to_vec());
    let bg_by: HashIdx<i64, usize> = (&bgr).map(|x: Bg| x.subject_id).inv().collect();
    let p1: Vec<P1> = drain(
        (&icu.stay_id)
            .and(&icu.intime)
            .and((&icu.subject_id).select(&bg_by).select(&bgr))
            .filt(|((_, i), b): ((i64, Ts), Bg)| {
                b.specimen.is_some_and(|s| s == "ART.") && b.charttime > i && b.charttime <= i + DAY_US
            }),
    )
    .into_iter()
    .map(|(_, ((stay_id, _), b))| P1 { stay_id, subject_id: b.subject_id, charttime: b.charttime, pao2fio2: b.pao2fio2ratio })
    .collect();
    let n1 = p1.len();
    let p1r = rel(p1);
    let vr = rel(vent.to_vec());
    let v_by: HashIdx<i64, usize> = (&vr).map(|x: Ventilation| x.stay_id).inv().collect();
    let vhit = (&p1r)
        .and((&p1r).map(|p: P1| p.stay_id).select(&v_by).select(&vr))
        .filt(|(p, v): (P1, Ventilation)| {
            v.ventilation_status == "InvasiveVent" && p.charttime > v.starttime && p.charttime <= v.endtime
        })
        .fold((), |a, _| a);
    let cr = rel(cpv);
    let c_by: HashIdx<i64, usize> = (&cr).map(|x: Cpap| x.subject_id).inv().collect();
    let chit = (&p1r)
        .and((&p1r).map(|p: P1| p.subject_id).select(&c_by).select(&cr))
        .filt(|(p, c): (P1, Cpap)| p.charttime > c.starttime && p.charttime <= c.endtime)
        .fold((), |a, _| a);
    let pafi2 = Universe::new(n1)
        .with((&vhit).or(&chit))
        .group_by((&p1r).map(|p: P1| p.stay_id))
        .select(&p1r)
        .fold(None, |a: Option<f64>, p: P1| match (a, p.pao2fio2) {
            (Some(a), Some(x)) => Some(a.min(x)),
            (a, x) => a.or(x),
        });
    let pafi_s: HashIdx<Id<IcuStay>, Option<f64>> = (&icu.stay_id).select(&pafi2).collect();

    let gcs_s = win(db, gs, &icu.stay_id, |x: Gcs| x.stay_id, |x| x.charttime, |x| [x.gcs]);
    let vital_s = win(db, vs, &icu.subject_id, |x: Vitalsign| x.subject_id, |x| x.charttime, |x| {
        [x.heart_rate, x.sbp, x.temperature]
    });
    let uo_s = win(db, uo, &icu.stay_id, |x: UrineOutput| x.stay_id, |x| x.charttime, |x| [Some(x.urineoutput)]);
    let labs_s = win(db, chem, &icu.subject_id, |x: Chemistry| x.subject_id, |x| x.charttime, |x| {
        [x.bun, x.potassium, x.sodium, x.bicarbonate]
    });
    let cbc_s = win(db, cbc, &icu.subject_id, |x: CompleteBloodCount| x.subject_id, |x| x.charttime, |x| [x.wbc]);
    let enz_s = win(db, enz, &icu.subject_id, |x: Enzyme| x.subject_id, |x| x.charttime, |x| [x.bilirubin_total]);

    let ar = rel(ag.to_vec());
    let ag_by: HashIdx<i64, usize> = (&ar).map(|x: Age| x.hadm_id).inv().collect();
    let age_s: HashIdx<Id<IcuStay>, i64> = (&icu.hadm_id).select(&ag_by).select(&ar).map(|a: Age| a.age).collect();

    let cohort = (&icu.subject_id)
        .and(&icu.hadm_id)
        .and(&icu.stay_id)
        .and(&icu.intime)
        .and((&icu.hadm).select(&ad.admission_type))
        .and((&age_s).opt())
        .and((&pafi_s).opt())
        .and((&surg).opt())
        .and((&comorb_s).opt())
        .and((&gcs_s).opt())
        .and((&vital_s).opt())
        .and((&uo_s).opt())
        .and((&labs_s).opt())
        .and((&cbc_s).opt())
        .and((&enz_s).opt());

    drain(&cohort)
        .into_iter()
        .map(
            |(
                _,
                ((((((((((((((subject_id, hadm_id), stay_id), intime), atype), age), pf), surg), comorb), g), v), u), l), c), e),
            )| {
                let v = v.unwrap_or([MM0; 3]);
                let l = l.unwrap_or([MM0; 4]);
                let lt = |a: Option<f64>, x: f64| a.is_some_and(|a| a < x);
                let ge = |a: Option<f64>, x: f64| a.is_some_and(|a| a >= x);
                let rng = |a: Option<f64>, lo: f64, hi: f64| a.is_some_and(|a| a >= lo && a < hi);
                let age_score = age.map(|a| {
                    if a < 40 {
                        0
                    } else if a < 60 {
                        7
                    } else if a < 70 {
                        12
                    } else if a < 75 {
                        15
                    } else if a < 80 {
                        16
                    } else {
                        18
                    }
                });
                let (hmin, hmax) = (v[0].min, v[0].max);
                let hr_score = if hmax.is_none() {
                    None
                } else if lt(hmin, 40.0) {
                    Some(11)
                } else if ge(hmax, 160.0) {
                    Some(7)
                } else if ge(hmax, 120.0) {
                    Some(4)
                } else if lt(hmin, 70.0) {
                    Some(2)
                } else if rng(hmax, 70.0, 120.0) && rng(hmin, 70.0, 120.0) {
                    Some(0)
                } else {
                    None
                };
                let (smin, smax) = (v[1].min, v[1].max);
                let sysbp_score = if smin.is_none() {
                    None
                } else if lt(smin, 70.0) {
                    Some(13)
                } else if lt(smin, 100.0) {
                    Some(5)
                } else if ge(smax, 200.0) {
                    Some(2)
                } else if rng(smax, 100.0, 200.0) && rng(smin, 100.0, 200.0) {
                    Some(0)
                } else {
                    None
                };
                let (tmin, tmax) = (v[2].min, v[2].max);
                let temp_score = if tmax.is_none() {
                    None
                } else if ge(tmax, 39.0) {
                    Some(3)
                } else if lt(tmin, 39.0) {
                    Some(0)
                } else {
                    None
                };
                let pao2fio2_score = pf.flatten().map(|p| {
                    if p < 100.0 {
                        11
                    } else if p < 200.0 {
                        9
                    } else {
                        6
                    }
                });
                let urineoutput = u.and_then(|u| (u[0].n > 0).then_some(u[0].sum.0));
                let uo_score = urineoutput.map(|x| {
                    if x < 500.0 {
                        11
                    } else if x < 1000.0 {
                        4
                    } else {
                        0
                    }
                });
                let bun_score = l[0].max.map(|x| {
                    if x < 28.0 {
                        0
                    } else if x < 84.0 {
                        6
                    } else {
                        10
                    }
                });
                let cbc = c.unwrap_or([MM0; 1]);
                let (wmin, wmax) = (cbc[0].min, cbc[0].max);
                let wbc_score = if wmax.is_none() {
                    None
                } else if lt(wmin, 1.0) {
                    Some(12)
                } else if ge(wmax, 20.0) {
                    Some(3)
                } else if rng(wmax, 1.0, 20.0) && rng(wmin, 1.0, 20.0) {
                    Some(0)
                } else {
                    None
                };
                let (kmin, kmax) = (l[1].min, l[1].max);
                let potassium_score = if kmax.is_none() {
                    None
                } else if lt(kmin, 3.0) {
                    Some(3)
                } else if ge(kmax, 5.0) {
                    Some(3)
                } else if rng(kmax, 3.0, 5.0) && rng(kmin, 3.0, 5.0) {
                    Some(0)
                } else {
                    None
                };
                let (namin, namax) = (l[2].min, l[2].max);
                let sodium_score = if namax.is_none() {
                    None
                } else if lt(namin, 125.0) {
                    Some(5)
                } else if ge(namax, 145.0) {
                    Some(1)
                } else if rng(namax, 125.0, 145.0) && rng(namin, 125.0, 145.0) {
                    Some(0)
                } else {
                    None
                };
                let (bmin, bmax) = (l[3].min, l[3].max);
                let bicarbonate_score = if bmax.is_none() {
                    None
                } else if lt(bmin, 15.0) {
                    Some(6)
                } else if lt(bmin, 20.0) {
                    Some(3)
                } else if ge(bmax, 20.0) && ge(bmin, 20.0) {
                    Some(0)
                } else {
                    None
                };
                let bilirubin_score = e.and_then(|e| e[0].max).map(|x| {
                    if x < 4.0 {
                        0
                    } else if x < 6.0 {
                        4
                    } else {
                        9
                    }
                });
                let gcs_score = g.and_then(|g| g[0].min).and_then(|m| {
                    if m < 3.0 {
                        None
                    } else if m < 6.0 {
                        Some(26)
                    } else if m < 9.0 {
                        Some(13)
                    } else if m < 11.0 {
                        Some(7)
                    } else if m < 14.0 {
                        Some(5)
                    } else if m <= 15.0 {
                        Some(0)
                    } else {
                        None
                    }
                });
                let (aids, hem, mets) = comorb.map_or((None, None, None), |(a, h, m)| (Some(a), Some(h), Some(m)));
                let comorbidity_score = if aids == Some(1) {
                    17
                } else if hem == Some(1) {
                    10
                } else if mets == Some(1) {
                    9
                } else {
                    0
                };
                let surgical = surg.unwrap_or(0);
                let admissiontype_score = if atype == "ELECTIVE" && surgical == 1 {
                    0
                } else if atype != "ELECTIVE" && surgical == 1 {
                    8
                } else {
                    6
                };
                let sapsii = [
                    age_score,
                    hr_score,
                    sysbp_score,
                    temp_score,
                    pao2fio2_score,
                    uo_score,
                    bun_score,
                    wbc_score,
                    potassium_score,
                    sodium_score,
                    bicarbonate_score,
                    bilirubin_score,
                    gcs_score,
                ]
                .iter()
                .map(|x| x.unwrap_or(0))
                .sum::<i64>()
                    + comorbidity_score
                    + admissiontype_score;
                let x = (-77631 + 737 * sapsii) as f64 / 10000.0 + 0.9971 * ((sapsii + 1) as f64).ln();
                Sapsii {
                    subject_id,
                    hadm_id,
                    stay_id,
                    starttime: intime,
                    endtime: intime + DAY_US,
                    sapsii,
                    sapsii_prob: 1.0 / (1.0 + (-x).exp()),
                    age_score,
                    hr_score,
                    sysbp_score,
                    temp_score,
                    pao2fio2_score,
                    uo_score,
                    bun_score,
                    wbc_score,
                    potassium_score,
                    sodium_score,
                    bicarbonate_score,
                    bilirubin_score,
                    gcs_score,
                    comorbidity_score,
                    admissiontype_score,
                }
            },
        )
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let ag = crate::concepts::age::age(db);
    let bg = crate::concepts::bg::bg(db);
    let chem = crate::concepts::chemistry::chemistry(db);
    let cbc = crate::concepts::complete_blood_count::complete_blood_count(db);
    let enz = crate::concepts::enzyme::enzyme(db);
    let gs = crate::concepts::gcs::gcs(db);
    let uo = crate::concepts::urine_output::urine_output(db);
    let od = crate::concepts::oxygen_delivery::oxygen_delivery(db);
    let vset = crate::concepts::ventilator_setting::ventilator_setting(db);
    let vent = crate::concepts::ventilation::ventilation(db, &vset, &od);
    let vs = crate::concepts::vitalsign::vitalsign(db);
    rows(sapsii(db, &ag, &bg, &chem, &cbc, &enz, &gs, &uo, &vent, &vs).into_iter().map(|s| {
        row(vec![
            V::I(s.subject_id),
            V::I(s.hadm_id),
            V::I(s.stay_id),
            V::T(s.starttime),
            V::T(s.endtime),
            V::I(s.sapsii),
            V::F(s.sapsii_prob),
            oint(s.age_score),
            oint(s.hr_score),
            oint(s.sysbp_score),
            oint(s.temp_score),
            oint(s.pao2fio2_score),
            oint(s.uo_score),
            oint(s.bun_score),
            oint(s.wbc_score),
            oint(s.potassium_score),
            oint(s.sodium_score),
            oint(s.bicarbonate_score),
            oint(s.bilirubin_score),
            oint(s.gcs_score),
            V::I(s.comorbidity_score),
            V::I(s.admissiontype_score),
        ])
    }))
}
