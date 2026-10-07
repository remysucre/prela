use crate::concepts::bg::*;
use crate::concepts::first_day_gcs::*;
use crate::concepts::first_day_lab::*;
use crate::concepts::first_day_urine_output::*;
use crate::concepts::first_day_vitalsign::*;
use crate::concepts::ventilation::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Apsiii {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub stay_id: i64,
    pub apsiii: Option<i64>,
    pub apsiii_prob: Option<f64>,
    pub hr_score: Option<i64>,
    pub mbp_score: Option<i64>,
    pub temp_score: Option<i64>,
    pub resp_rate_score: Option<i64>,
    pub pao2_aado2_score: Option<i64>,
    pub hematocrit_score: Option<i64>,
    pub wbc_score: Option<i64>,
    pub creatinine_score: Option<i64>,
    pub uo_score: Option<i64>,
    pub bun_score: Option<i64>,
    pub sodium_score: Option<i64>,
    pub albumin_score: Option<i64>,
    pub bilirubin_score: Option<i64>,
    pub glucose_score: Option<i64>,
    pub acidbase_score: Option<i64>,
    pub gcs_score: Option<i64>,
}

#[derive(Clone, Copy)]
struct Cohort {
    stay_id: i64,
    heart_rate_min: Option<f64>,
    heart_rate_max: Option<f64>,
    mbp_min: Option<f64>,
    mbp_max: Option<f64>,
    temperature_min: Option<f64>,
    temperature_max: Option<f64>,
    resp_rate_min: Option<f64>,
    resp_rate_max: Option<f64>,
    pao2: Option<f64>,
    aado2: Option<f64>,
    acidbase_score: Option<i64>,
    hematocrit_min: Option<f64>,
    hematocrit_max: Option<f64>,
    wbc_min: Option<f64>,
    wbc_max: Option<f64>,
    creatinine_min: Option<f64>,
    creatinine_max: Option<f64>,
    bun_min: Option<f64>,
    bun_max: Option<f64>,
    sodium_min: Option<f64>,
    sodium_max: Option<f64>,
    albumin_min: Option<f64>,
    albumin_max: Option<f64>,
    bilirubin_min: Option<f64>,
    bilirubin_max: Option<f64>,
    glucose_max: Option<f64>,
    glucose_min: Option<f64>,
    vent: Option<i64>,
    urineoutput: Option<f64>,
    gcs_motor: Option<f64>,
    gcs_verbal: Option<f64>,
    gcs_eyes: Option<f64>,
    gcs_unable: Option<i64>,
    arf: Option<i64>,
}

#[derive(Clone, Copy)]
struct Sc {
    hr: Option<i64>,
    mbp: Option<i64>,
    temp: Option<i64>,
    resp: Option<i64>,
    hct: Option<i64>,
    wbc: Option<i64>,
    creat: Option<i64>,
    bun: Option<i64>,
    sodium: Option<i64>,
    albumin: Option<i64>,
    bili: Option<i64>,
    glucose: Option<i64>,
}

type Pair = (Id<IcuStay>, Bg);

fn bins(x: Option<f64>, cuts: &[(f64, i64)], last: i64) -> Option<i64> {
    let x = x?;
    Some(cuts.iter().find(|&&(c, _)| x < c).map_or(last, |&(_, s)| s))
}

fn acidbase_score(ph: f64, pco2: f64) -> i64 {
    if ph < 7.20 {
        if pco2 < 50.0 { 12 } else { 4 }
    } else if ph < 7.30 {
        if pco2 < 30.0 {
            9
        } else if pco2 < 40.0 {
            6
        } else if pco2 < 50.0 {
            3
        } else {
            2
        }
    } else if ph < 7.35 {
        if pco2 < 30.0 {
            9
        } else if pco2 < 45.0 {
            0
        } else {
            1
        }
    } else if ph < 7.45 {
        if pco2 < 30.0 {
            5
        } else if pco2 < 45.0 {
            0
        } else {
            1
        }
    } else if ph < 7.50 {
        if pco2 < 30.0 {
            5
        } else if pco2 < 35.0 {
            0
        } else if pco2 < 45.0 {
            2
        } else {
            12
        }
    } else if ph < 7.60 {
        if pco2 < 40.0 { 3 } else { 12 }
    } else if pco2 < 25.0 {
        0
    } else if pco2 < 40.0 {
        3
    } else {
        12
    }
}

fn score(c: &Cohort, side: fn(Option<f64>, Option<f64>) -> Option<f64>) -> Sc {
    let creat = {
        let x = side(c.creatinine_min, c.creatinine_max);
        match x {
            None => None,
            Some(v) if c.arf == Some(1) && v < 1.5 => Some(0),
            Some(v) if c.arf == Some(1) && v >= 1.5 => Some(10),
            _ => bins(x, &[(0.5, 3), (1.5, 0), (1.95, 4)], 7),
        }
    };
    let resp = {
        let x = side(c.resp_rate_min, c.resp_rate_max);
        match x {
            None => None,
            Some(v) if c.vent == Some(1) && v < 14.0 => Some(0),
            _ => bins(x, &[(6.0, 17), (12.0, 8), (14.0, 7), (25.0, 0), (35.0, 6), (40.0, 9), (50.0, 11)], 18),
        }
    };
    Sc {
        hr: bins(
            side(c.heart_rate_min, c.heart_rate_max),
            &[(40.0, 8), (50.0, 5), (100.0, 0), (110.0, 1), (120.0, 5), (140.0, 7), (155.0, 13)],
            17,
        ),
        mbp: bins(
            side(c.mbp_min, c.mbp_max),
            &[(40.0, 23), (60.0, 15), (70.0, 7), (80.0, 6), (100.0, 0), (120.0, 4), (130.0, 7), (140.0, 9)],
            10,
        ),
        temp: bins(
            side(c.temperature_min, c.temperature_max),
            &[(33.0, 20), (33.5, 16), (34.0, 13), (35.0, 8), (36.0, 2), (40.0, 0)],
            4,
        ),
        resp,
        hct: bins(side(c.hematocrit_min, c.hematocrit_max), &[(41.0, 3), (50.0, 0)], 3),
        wbc: bins(side(c.wbc_min, c.wbc_max), &[(1.0, 19), (3.0, 5), (20.0, 0), (25.0, 1)], 5),
        creat,
        bun: bins(side(c.bun_min, c.bun_max), &[(17.0, 0), (20.0, 2), (40.0, 7), (80.0, 11)], 12),
        sodium: bins(side(c.sodium_min, c.sodium_max), &[(120.0, 3), (135.0, 2), (155.0, 0)], 4),
        albumin: bins(side(c.albumin_min, c.albumin_max), &[(2.0, 11), (2.5, 6), (4.5, 0)], 4),
        bili: bins(side(c.bilirubin_min, c.bilirubin_max), &[(2.0, 0), (3.0, 5), (5.0, 6), (8.0, 8)], 16),
        glucose: bins(side(c.glucose_min, c.glucose_max), &[(40.0, 8), (60.0, 9), (200.0, 0), (350.0, 3)], 5),
    }
}

fn pick(mx: Option<f64>, mn: Option<f64>, mid: f64, smax: Option<i64>, smin: Option<i64>) -> Option<i64> {
    let mx = mx?;
    let mn = mn?;
    let (a, b) = ((mx - mid).abs(), (mn - mid).abs());
    if a > b {
        smax
    } else if a < b {
        smin
    } else {
        match (smax, smin) {
            (Some(x), Some(y)) => Some(if x >= y { x } else { y }),
            _ => None,
        }
    }
}

fn gcs_score(c: &Cohort) -> Option<i64> {
    let is = |x: Option<f64>, vs: &[f64]| x.is_some_and(|x| vs.contains(&x));
    let (e, v, m) = (c.gcs_eyes, c.gcs_verbal, c.gcs_motor);
    if c.gcs_unable == Some(1) {
        Some(0)
    } else if is(e, &[1.0]) {
        if is(v, &[1.0]) && is(m, &[1.0, 2.0]) {
            Some(48)
        } else if is(v, &[1.0]) && is(m, &[3.0, 4.0]) {
            Some(33)
        } else if is(v, &[1.0]) && is(m, &[5.0, 6.0]) {
            Some(16)
        } else if is(v, &[2.0, 3.0]) && is(m, &[1.0, 2.0]) {
            Some(29)
        } else if is(v, &[2.0, 3.0]) && is(m, &[3.0, 4.0]) {
            Some(24)
        } else {
            None
        }
    } else if e.is_some_and(|e| e > 1.0) {
        if is(v, &[1.0]) && is(m, &[1.0, 2.0]) {
            Some(29)
        } else if is(v, &[1.0]) && is(m, &[3.0, 4.0]) {
            Some(24)
        } else if is(v, &[1.0]) && is(m, &[5.0, 6.0]) {
            Some(15)
        } else if is(v, &[2.0, 3.0]) && is(m, &[1.0, 2.0]) {
            Some(29)
        } else if is(v, &[2.0, 3.0]) && is(m, &[3.0, 4.0]) {
            Some(24)
        } else if is(v, &[2.0, 3.0]) && is(m, &[5.0]) {
            Some(13)
        } else if is(v, &[2.0, 3.0]) && is(m, &[6.0]) {
            Some(10)
        } else if is(v, &[4.0]) && is(m, &[1.0, 2.0, 3.0, 4.0]) {
            Some(13)
        } else if is(v, &[4.0]) && is(m, &[5.0]) {
            Some(8)
        } else if is(v, &[4.0]) && is(m, &[6.0]) {
            Some(3)
        } else if is(v, &[5.0]) && is(m, &[1.0, 2.0, 3.0, 4.0, 5.0]) {
            Some(3)
        } else if is(v, &[5.0]) && is(m, &[6.0]) {
            Some(0)
        } else {
            None
        }
    } else {
        None
    }
}

fn gmax(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if y > x { y } else { x }),
        (x, y) => x.or(y),
    }
}

fn gmin(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if y < x { y } else { x }),
        (x, y) => x.or(y),
    }
}

fn desc(a: &f64, b: &f64) -> std::cmp::Ordering {
    b.total_cmp(a)
}

fn desc_i(a: &i64, b: &i64) -> std::cmp::Ordering {
    b.cmp(a)
}

fn ckd(version: i64, code: Str) -> i64 {
    let p = &code[..code.len().min(4)];
    if (version == 9 && ["5854", "5855", "5856"].contains(&p)) || (version == 10 && ["N184", "N185", "N186"].contains(&p)) {
        1
    } else {
        0
    }
}

fn art_in_stay(db: &'static Db, br: &VecRel<usize, Bg>) -> VecRel<usize, Pair> {
    let icu = &db.icu_stay;
    let art_by_hadm: HashIdx<i64, usize> = br
        .filt(|b: Bg| b.specimen == Some("ART."))
        .flat_map(|b: Bg| b.hadm_id)
        .inv()
        .collect();
    let in_stay = (&icu.intime)
        .and(&icu.outtime)
        .and((&icu.hadm_id).select(&art_by_hadm).select(br))
        .filt(|((i, o), b): ((Ts, Ts), Bg)| b.charttime >= i && b.charttime < o)
        .map(|(_, b): ((Ts, Ts), Bg)| b);
    rel(drain(&in_stay))
}

fn invasive(vr: &VecRel<usize, Ventilation>) -> HashIdx<i64, usize> {
    vr.filt(|v: Ventilation| v.ventilation_status == "InvasiveVent").map(|v: Ventilation| v.stay_id).inv().collect()
}

fn on_vent(db: &'static Db, sbr: &VecRel<usize, Pair>, vr: &VecRel<usize, Ventilation>) -> HashIdx<usize, Ventilation> {
    let inv_by = invasive(vr);
    sbr.and(sbr.map(|(s, _): Pair| s).select(&db.icu_stay.stay_id).select(&inv_by).select(vr))
        .filt(|((_, b), v): (Pair, Ventilation)| b.charttime >= v.starttime && b.charttime <= v.endtime)
        .map(|(_, v): (Pair, Ventilation)| v)
        .collect()
}

pub fn apsiii(
    db: &'static Db,
    bg: &[Bg],
    vent: &[Ventilation],
    fdg: &[FirstDayGcs],
    fdl: &[FirstDayLab],
    fduo: &[FirstDayUrineOutput],
    fdv: &[FirstDayVitalsign],
) -> Vec<Apsiii> {
    let icu = &db.icu_stay;
    let adm = &db.admission;
    let pat = &db.patient;
    let dx = &db.diagnoses_icd;

    let br = rel(bg.to_vec());
    let vr = rel(vent.to_vec());

    let pa_bg = art_in_stay(db, &br);
    let pa_vent = on_vent(db, &pa_bg, &vr);
    let pa_by: HashIdx<Id<IcuStay>, usize> = (&pa_bg)
        .minus(&pa_vent)
        .filt(|(_, b): Pair| b.fio2.or(b.fio2_chartevents).unwrap_or(21.0) < 50.0 && b.po2.is_some())
        .map(|(s, _): Pair| s)
        .inv()
        .collect();
    let pa_w = (&pa_by).select(&pa_bg).window(row_number, |(_, b): Pair| b.po2.unwrap(), desc);
    let pa = (&pa_w).filt(|(_, rn): (Pair, i64)| rn == 1).map(|((_, b), _): (Pair, i64)| b.po2.unwrap());

    let aa_bg = art_in_stay(db, &br);
    let aa_vent = on_vent(db, &aa_bg, &vr);
    let aa_by: HashIdx<Id<IcuStay>, usize> = (&aa_bg)
        .and(&aa_vent)
        .filt(|((_, b), _): (Pair, Ventilation)| {
            b.fio2.or(b.fio2_chartevents).is_some_and(|f| f >= 50.0) && b.aado2.is_some()
        })
        .map(|((s, _), _): (Pair, Ventilation)| s)
        .inv()
        .collect();
    let aa_w = (&aa_by).select(&aa_bg).window(row_number, |(_, b): Pair| b.aado2.unwrap(), desc);
    let aa = (&aa_w).filt(|(_, rn): (Pair, i64)| rn == 1).map(|((_, b), _): (Pair, i64)| b.aado2.unwrap());

    let ab_bg = art_in_stay(db, &br);
    let ab_by: HashIdx<Id<IcuStay>, usize> = (&ab_bg)
        .filt(|(_, b): Pair| b.ph.is_some() && b.pco2.is_some())
        .map(|(s, _): Pair| s)
        .inv()
        .collect();
    let ab_w = (&ab_by)
        .select(&ab_bg)
        .map(|(_, b): Pair| (b.ph, b.pco2, acidbase_score(b.ph.unwrap(), b.pco2.unwrap())))
        .window(row_number, |(_, _, s): (Option<f64>, Option<f64>, i64)| s, desc_i);
    let ab = (&ab_w)
        .filt(|(_, rn): ((Option<f64>, Option<f64>, i64), i64)| rn == 1)
        .map(|((_, _, s), _): ((Option<f64>, Option<f64>, i64), i64)| s);

    let uor = rel(fduo.to_vec());
    let uo_by: HashIdx<i64, usize> = (&uor).map(|x: FirstDayUrineOutput| x.stay_id).inv().collect();
    let uo = (&icu.stay_id).select(&uo_by).select(&uor);
    let lr = rel(fdl.to_vec());
    let lab_by: HashIdx<i64, usize> = (&lr).map(|x: FirstDayLab| x.stay_id).inv().collect();
    let lab = (&icu.stay_id).select(&lab_by).select(&lr);
    let vsr = rel(fdv.to_vec());
    let vs_by: HashIdx<i64, usize> = (&vsr).map(|x: FirstDayVitalsign| x.stay_id).inv().collect();
    let vital = (&icu.stay_id).select(&vs_by).select(&vsr);
    let gr = rel(fdg.to_vec());
    let g_by: HashIdx<i64, usize> = (&gr).map(|x: FirstDayGcs| x.stay_id).inv().collect();
    let gcs = (&icu.stay_id).select(&g_by).select(&gr);

    let ckd_q = (&dx.id)
        .group_by(&dx.hadm_id)
        .select((&dx.icd_version).and(&dx.icd_code))
        .fold(0i64, |a, (v, c): (i64, Str)| a.max(ckd(v, c)));
    let arf = (&icu.id)
        .and((&uo).opt())
        .and((&lab).opt())
        .and((&icu.hadm_id).select(&ckd_q).opt())
        .map(
            |(((_, u), l), k): (((Id<IcuStay>, Option<FirstDayUrineOutput>), Option<FirstDayLab>), Option<i64>)| {
                let cr = l.and_then(|l| l.creatinine_max);
                let uo = u.and_then(|u| u.urineoutput);
                if cr.is_some_and(|c| c >= 1.5) && uo.is_some_and(|u| u < 410.0) && k == Some(0) { 1i64 } else { 0 }
            },
        );

    let inv_by = invasive(&vr);
    let vhit = (&icu.intime)
        .and((&icu.stay_id).select(&inv_by).select(&vr))
        .filt(|(i, v): (Ts, Ventilation)| {
            let e = i + DAY_US;
            (v.starttime >= i && v.starttime <= e)
                || (v.endtime >= i && v.endtime <= e)
                || (v.starttime <= i && v.endtime >= e)
        });
    let vent_q = (&icu.id)
        .and((&vhit).opt())
        .fold(0i64, |a, (_, m): (Id<IcuStay>, Option<(Ts, Ventilation)>)| a.max(m.is_some() as i64));

    let co = (&icu.stay_id)
        .and((&icu.hadm).select(&adm.hadm_id))
        .and((&icu.subject).select(&pat.subject_id))
        .and((&pa).opt())
        .and((&aa).opt())
        .and((&ab).opt())
        .and((&arf).opt())
        .and((&vent_q).opt())
        .and((&gcs).opt())
        .and((&vital).opt())
        .and((&uo).opt())
        .and((&lab).opt());
    let cohort: Vec<Cohort> = drain(&co)
        .into_iter()
        .map(|(_, (((((((((((stay_id, _), _), pao2), aado2), acidbase_score), arf), vent), g), v), u), l))| Cohort {
            stay_id,
            heart_rate_min: v.and_then(|v| v.heart_rate_min),
            heart_rate_max: v.and_then(|v| v.heart_rate_max),
            mbp_min: v.and_then(|v| v.mbp_min),
            mbp_max: v.and_then(|v| v.mbp_max),
            temperature_min: v.and_then(|v| v.temperature_min),
            temperature_max: v.and_then(|v| v.temperature_max),
            resp_rate_min: v.and_then(|v| v.resp_rate_min),
            resp_rate_max: v.and_then(|v| v.resp_rate_max),
            pao2,
            aado2,
            acidbase_score,
            hematocrit_min: l.and_then(|l| l.hematocrit_min),
            hematocrit_max: l.and_then(|l| l.hematocrit_max),
            wbc_min: l.and_then(|l| l.wbc_min),
            wbc_max: l.and_then(|l| l.wbc_max),
            creatinine_min: l.and_then(|l| l.creatinine_min),
            creatinine_max: l.and_then(|l| l.creatinine_max),
            bun_min: l.and_then(|l| l.bun_min),
            bun_max: l.and_then(|l| l.bun_max),
            sodium_min: l.and_then(|l| l.sodium_min),
            sodium_max: l.and_then(|l| l.sodium_max),
            albumin_min: l.and_then(|l| l.albumin_min),
            albumin_max: l.and_then(|l| l.albumin_max),
            bilirubin_min: l.and_then(|l| l.bilirubin_total_min),
            bilirubin_max: l.and_then(|l| l.bilirubin_total_max),
            glucose_max: gmax(l.and_then(|l| l.glucose_max), v.and_then(|v| v.glucose_max)),
            glucose_min: gmin(l.and_then(|l| l.glucose_min), v.and_then(|v| v.glucose_min)),
            vent,
            urineoutput: u.and_then(|u| u.urineoutput),
            gcs_motor: g.and_then(|g| g.gcs_motor),
            gcs_verbal: g.and_then(|g| g.gcs_verbal),
            gcs_eyes: g.and_then(|g| g.gcs_eyes),
            gcs_unable: g.and_then(|g| g.gcs_unable),
            arf,
        })
        .collect();

    let cr = rel(cohort);
    let smin = (&cr).map(|c: Cohort| score(&c, |mn, _| mn));
    let smin_by: HashIdx<i64, Sc> = (&cr).map(|c: Cohort| c.stay_id).inv().select(&smin).collect();
    let smax = (&cr).map(|c: Cohort| score(&c, |_, mx| mx));
    let smax_by: HashIdx<i64, Sc> = (&cr).map(|c: Cohort| c.stay_id).inv().select(&smax).collect();
    let comp = (&cr)
        .and((&cr).map(|c: Cohort| c.stay_id).select(&smin_by).opt())
        .and((&cr).map(|c: Cohort| c.stay_id).select(&smax_by).opt())
        .map(|((c, mn), mx): ((Cohort, Option<Sc>), Option<Sc>)| {
            let f = |g: fn(&Sc) -> Option<i64>| (mx.and_then(|s| g(&s)), mn.and_then(|s| g(&s)));
            let (a, b) = f(|s| s.hr);
            let hr = pick(c.heart_rate_max, c.heart_rate_min, 75.0, a, b);
            let (a, b) = f(|s| s.mbp);
            let mbp = pick(c.mbp_max, c.mbp_min, 90.0, a, b);
            let (a, b) = f(|s| s.temp);
            let temp = pick(c.temperature_max, c.temperature_min, 38.0, a, b);
            let (a, b) = f(|s| s.resp);
            let resp = pick(c.resp_rate_max, c.resp_rate_min, 19.0, a, b);
            let (a, b) = f(|s| s.hct);
            let hct = pick(c.hematocrit_max, c.hematocrit_min, 45.5, a, b);
            let (a, b) = f(|s| s.wbc);
            let wbc = pick(c.wbc_max, c.wbc_min, 11.5, a, b);
            let (a, b) = f(|s| s.creat);
            let creat = match c.creatinine_max {
                None => None,
                Some(_) if c.arf == Some(1) => a,
                _ => pick(c.creatinine_max, c.creatinine_min, 1.0, a, b),
            };
            let bun = c.bun_max.and(f(|s| s.bun).0);
            let (a, b) = f(|s| s.sodium);
            let sodium = pick(c.sodium_max, c.sodium_min, 145.5, a, b);
            let (a, b) = f(|s| s.albumin);
            let albumin = pick(c.albumin_max, c.albumin_min, 3.5, a, b);
            let bili = c.bilirubin_max.and(f(|s| s.bili).0);
            let (a, b) = f(|s| s.glucose);
            let glucose = pick(c.glucose_max, c.glucose_min, 130.0, a, b);
            let uo = bins(c.urineoutput, &[(400.0, 15), (600.0, 8), (900.0, 7), (1500.0, 5), (2000.0, 4), (4000.0, 0)], 1);
            let gcs = gcs_score(&c);
            let pao2 = match (c.pao2, c.aado2) {
                (None, None) => None,
                (Some(p), _) => bins(Some(p), &[(50.0, 15), (70.0, 5), (80.0, 2)], 0),
                (None, a) => bins(a, &[(100.0, 0), (250.0, 7), (350.0, 9), (500.0, 11)], 14),
            };
            let parts = [hr, mbp, temp, resp, pao2, hct, wbc, creat, uo, bun, sodium, albumin, bili, glucose, c.acidbase_score, gcs];
            let total: i64 = parts.iter().map(|p| p.unwrap_or(0)).sum();
            let x = (4726 * total - 443600) as f64 / 100000.0;
            (
                c.stay_id,
                Apsiii {
                    subject_id: 0,
                    hadm_id: 0,
                    stay_id: c.stay_id,
                    apsiii: Some(total),
                    apsiii_prob: Some(1.0 / (1.0 + (-x).exp())),
                    hr_score: hr,
                    mbp_score: mbp,
                    temp_score: temp,
                    resp_rate_score: resp,
                    pao2_aado2_score: pao2,
                    hematocrit_score: hct,
                    wbc_score: wbc,
                    creatinine_score: creat,
                    uo_score: uo,
                    bun_score: bun,
                    sodium_score: sodium,
                    albumin_score: albumin,
                    bilirubin_score: bili,
                    glucose_score: glucose,
                    acidbase_score: c.acidbase_score,
                    gcs_score: gcs,
                },
            )
        });
    let score_by: HashIdx<i64, Apsiii> = (&comp).map(|(s, _): (i64, Apsiii)| s).inv().select(&comp).map(|(_, a): (i64, Apsiii)| a).collect();
    let out = (&icu.subject_id).and(&icu.hadm_id).and(&icu.stay_id).and((&icu.stay_id).select(&score_by).opt());
    drain(&out)
        .into_iter()
        .map(|(_, (((subject_id, hadm_id), stay_id), s))| match s {
            Some(a) => Apsiii { subject_id, hadm_id, stay_id, ..a },
            None => Apsiii {
                subject_id,
                hadm_id,
                stay_id,
                apsiii: None,
                apsiii_prob: None,
                hr_score: None,
                mbp_score: None,
                temp_score: None,
                resp_rate_score: None,
                pao2_aado2_score: None,
                hematocrit_score: None,
                wbc_score: None,
                creatinine_score: None,
                uo_score: None,
                bun_score: None,
                sodium_score: None,
                albumin_score: None,
                bilirubin_score: None,
                glucose_score: None,
                acidbase_score: None,
                gcs_score: None,
            },
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
    let fduo = crate::concepts::first_day_urine_output::first_day_urine_output(db, &uo);
    let vs = crate::concepts::vitalsign::vitalsign(db);
    let fdv = crate::concepts::first_day_vitalsign::first_day_vitalsign(db, &vs);
    let od = crate::concepts::oxygen_delivery::oxygen_delivery(db);
    let vset = crate::concepts::ventilator_setting::ventilator_setting(db);
    let vent = crate::concepts::ventilation::ventilation(db, &vset, &od);
    rows(apsiii(db, &bg, &vent, &fdg, &fdl, &fduo, &fdv).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.hadm_id),
            V::I(v.stay_id),
            oint(v.apsiii),
            ofloat(v.apsiii_prob),
            oint(v.hr_score),
            oint(v.mbp_score),
            oint(v.temp_score),
            oint(v.resp_rate_score),
            oint(v.pao2_aado2_score),
            oint(v.hematocrit_score),
            oint(v.wbc_score),
            oint(v.creatinine_score),
            oint(v.uo_score),
            oint(v.bun_score),
            oint(v.sodium_score),
            oint(v.albumin_score),
            oint(v.bilirubin_score),
            oint(v.glucose_score),
            oint(v.acidbase_score),
            oint(v.gcs_score),
        ])
    }))
}
