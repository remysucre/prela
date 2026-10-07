use crate::concepts::{crrt::*, kdigo_creatinine::*, kdigo_uo::*};
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct KdigoStages {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub stay_id: i64,
    pub charttime: Option<Ts>,
    pub creat_low_past_7day: Option<f64>,
    pub creat_low_past_48hr: Option<f64>,
    pub creat: Option<f64>,
    pub aki_stage_creat: Option<i64>,
    pub uo_rt_6hr: Option<f64>,
    pub uo_rt_12hr: Option<f64>,
    pub uo_rt_24hr: Option<f64>,
    pub aki_stage_uo: Option<i64>,
    pub aki_stage_crrt: Option<i64>,
    pub aki_stage: Option<i64>,
    pub aki_stage_smoothed: Option<i64>,
}

#[derive(Clone, Copy)]
struct CrStg {
    stay_id: i64,
    charttime: Option<Ts>,
    low7: Option<f64>,
    low48: Option<f64>,
    creat: Option<f64>,
    stage: i64,
}

#[derive(Clone, Copy)]
struct UoStg {
    stay_id: i64,
    charttime: Ts,
    rt: [Option<f64>; 3],
    stage: Option<i64>,
}

#[derive(Clone, Copy)]
struct CrrtStg {
    stay_id: i64,
    charttime: Ts,
    stage: Option<i64>,
}

type Base = ((((i64, i64), i64), Ts), Option<Ts>);

fn ge(a: Option<f64>, b: Option<f64>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if a >= b)
}

fn cr_stage(c: &KdigoCreatinine) -> i64 {
    let (cr, l7, l48) = (c.creat, c.creat_low_past_7day, c.creat_low_past_48hr);
    let m7 = |f: f64| l7.map(|x| x * f);
    let p48 = l48.map(|x| x + 0.3);
    if ge(cr, m7(3.0)) {
        3
    } else if ge(cr, Some(4.0)) && (ge(cr, p48) || ge(cr, l7.map(|x| 1.5 * x))) {
        3
    } else if ge(cr, m7(2.0)) {
        2
    } else if ge(cr, p48) || ge(cr, m7(1.5)) {
        1
    } else {
        0
    }
}

fn uo_stage(u: &KdigoUo, intime: Ts) -> Option<i64> {
    let lt = |x: Option<f64>, c: f64| x.is_some_and(|x| x < c);
    if u.uo_rt_6hr.is_none() {
        None
    } else if u.charttime <= intime + 6 * 3600 * US {
        Some(0)
    } else if u.uo_tm_24hr >= 24.0 && lt(u.uo_rt_24hr, 0.3) {
        Some(3)
    } else if u.uo_tm_12hr >= 12.0 && u.uo_rt_12hr == Some(0.0) {
        Some(3)
    } else if u.uo_tm_12hr >= 12.0 && lt(u.uo_rt_12hr, 0.5) {
        Some(2)
    } else if u.uo_tm_6hr >= 6.0 && lt(u.uo_rt_6hr, 0.5) {
        Some(1)
    } else {
        Some(0)
    }
}

fn secs(t: Ts) -> i64 {
    t.div_euclid(US)
}

fn smooth(g: &[(Option<i64>, KdigoStages)], out: &mut Vec<Option<i64>>) {
    for &(k, _) in g {
        let mut m: Option<i64> = None;
        for &(kj, r) in g {
            let inside = match (k, kj) {
                (None, None) => true,
                (Some(k), Some(kj)) => kj >= k - 21600 && kj <= k,
                _ => false,
            };
            if inside {
                if let Some(a) = r.aki_stage {
                    m = Some(m.map_or(a, |x| x.max(a)));
                }
            }
        }
        out.push(m);
    }
}

pub fn kdigo_stages(db: &'static Db, kc: &[KdigoCreatinine], ku: &[KdigoUo], cr: &[Crrt]) -> Vec<KdigoStages> {
    let ie = &db.icu_stay;
    let cr_stg = rel(kc
        .iter()
        .map(|c| CrStg {
            stay_id: c.stay_id,
            charttime: c.charttime,
            low7: c.creat_low_past_7day,
            low48: c.creat_low_past_48hr,
            creat: c.creat,
            stage: cr_stage(c),
        })
        .collect::<Vec<_>>());
    let by_sid: HashIdx<i64, Id<IcuStay>> = (&ie.stay_id).inv().collect();
    let kur = rel(ku.to_vec());
    let uo_j = (&kur).and((&kur).map(|u: KdigoUo| u.stay_id).select(&by_sid).select(&ie.intime));
    let uo_stg = rel(drain(&uo_j)
        .into_iter()
        .map(|(_, (u, intime))| UoStg {
            stay_id: u.stay_id,
            charttime: u.charttime,
            rt: [u.uo_rt_6hr, u.uo_rt_12hr, u.uo_rt_24hr],
            stage: uo_stage(&u, intime),
        })
        .collect::<Vec<_>>());
    let crr = rel(cr.to_vec());
    let crrt_stg = rel(drain((&crr).filt(|c: Crrt| c.crrt_mode.is_some()))
        .into_iter()
        .map(|(_, c)| CrrtStg { stay_id: c.stay_id, charttime: c.charttime, stage: Some(3) })
        .collect::<Vec<_>>());

    let tm_u = (&cr_stg)
        .map(|c: CrStg| (c.stay_id, c.charttime))
        .inv()
        .map(|_| ())
        .union((&uo_stg).map(|u: UoStg| (u.stay_id, Some(u.charttime))).inv().map(|_| ()))
        .union((&crrt_stg).map(|c: CrrtStg| (c.stay_id, Some(c.charttime))).inv().map(|_| ()))
        .fold((), |a, _| a);
    let tm = rel(drain(&tm_u).into_iter().map(|(k, _)| k).collect::<Vec<(i64, Option<Ts>)>>());
    let tm_by: HashIdx<i64, usize> = (&tm).map(|(s, _): (i64, Option<Ts>)| s).inv().collect();

    let base = (&ie.subject_id)
        .and(&ie.hadm_id)
        .and(&ie.stay_id)
        .and(&ie.intime)
        .and((&ie.stay_id).select((&tm_by).select(&tm).opt()));
    let base = rel(drain(&base)
        .into_iter()
        .map(|(_, (k, t)): (_, ((((i64, i64), i64), Ts), Option<(i64, Option<Ts>)>))| -> Base { (k, t.and_then(|t| t.1)) })
        .collect::<Vec<_>>());

    let cr_by: HashIdx<(i64, Option<Ts>), usize> = (&cr_stg)
        .filt(|c: CrStg| c.charttime.is_some())
        .map(|c: CrStg| (c.stay_id, c.charttime))
        .inv()
        .collect();
    let uo_by: HashIdx<(i64, Option<Ts>), usize> =
        (&uo_stg).map(|u: UoStg| (u.stay_id, Some(u.charttime))).inv().collect();
    let crrt_by: HashIdx<(i64, Option<Ts>), usize> =
        (&crrt_stg).map(|c: CrrtStg| (c.stay_id, Some(c.charttime))).inv().collect();
    let key = |((((_, _), s), _), t): Base| (s, t);
    let joined = (&base)
        .and((&base).map(key).select((&cr_by).select(&cr_stg).opt()))
        .and((&base).map(key).select((&uo_by).select(&uo_stg).opt()))
        .and((&base).map(key).select((&crrt_by).select(&crrt_stg).opt()));
    let rows = rel(drain(&joined)
        .into_iter()
        .map(|(_, (((b, c), u), x))| {
            let ((((subject_id, hadm_id), stay_id), intime), charttime) = b;
            let sc = c.map(|c| c.stage);
            let su = u.and_then(|u| u.stage);
            let sx = x.and_then(|x| x.stage);
            let aki = Some(sc.unwrap_or(0).max(su.unwrap_or(0)).max(sx.unwrap_or(0)));
            (
                charttime.map(|t| secs(t) - secs(intime)),
                KdigoStages {
                    subject_id,
                    hadm_id,
                    stay_id,
                    charttime,
                    creat_low_past_7day: c.and_then(|c| c.low7),
                    creat_low_past_48hr: c.and_then(|c| c.low48),
                    creat: c.and_then(|c| c.creat),
                    aki_stage_creat: sc,
                    uo_rt_6hr: u.and_then(|u| u.rt[0]),
                    uo_rt_12hr: u.and_then(|u| u.rt[1]),
                    uo_rt_24hr: u.and_then(|u| u.rt[2]),
                    aki_stage_uo: su,
                    aki_stage_crrt: sx,
                    aki_stage: aki,
                    aki_stage_smoothed: None,
                },
            )
        })
        .collect::<Vec<(Option<i64>, KdigoStages)>>());
    let w = (&rows)
        .map(|(_, r): (Option<i64>, KdigoStages)| r.subject_id)
        .inv()
        .select(&rows)
        .window(
            |g: &[(Option<i64>, (Option<i64>, KdigoStages))], out: &mut Vec<Option<i64>>| {
                let g: Vec<(Option<i64>, KdigoStages)> = g.iter().map(|&(k, (_, r))| (k, r)).collect();
                smooth(&g, out)
            },
            |(k, _): (Option<i64>, KdigoStages)| k,
            |a: &Option<i64>, b: &Option<i64>| a.cmp(b),
        );
    drain(&w)
        .into_iter()
        .map(|(_, ((_, r), s))| KdigoStages { aki_stage_smoothed: s, ..r })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let crrt = crate::concepts::crrt::crrt(db);
    let kc = crate::concepts::kdigo_creatinine::kdigo_creatinine(db);
    let uo = crate::concepts::urine_output::urine_output(db);
    let wd = crate::concepts::weight_durations::weight_durations(db);
    let ku = crate::concepts::kdigo_uo::kdigo_uo(db, &uo, &wd);
    rows(kdigo_stages(db, &kc, &ku, &crrt).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.hadm_id),
            V::I(v.stay_id),
            ots(v.charttime),
            ofloat(v.creat_low_past_7day),
            ofloat(v.creat_low_past_48hr),
            ofloat(v.creat),
            oint(v.aki_stage_creat),
            ofloat(v.uo_rt_6hr),
            ofloat(v.uo_rt_12hr),
            ofloat(v.uo_rt_24hr),
            oint(v.aki_stage_uo),
            oint(v.aki_stage_crrt),
            oint(v.aki_stage),
            oint(v.aki_stage_smoothed),
        ])
    }))
}
