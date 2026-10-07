use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct KdigoCreatinine {
    pub hadm_id: i64,
    pub stay_id: i64,
    pub charttime: Option<Ts>,
    pub creat: Option<f64>,
    pub creat_low_past_48hr: Option<f64>,
    pub creat_low_past_7day: Option<f64>,
}

#[derive(Clone, Copy)]
struct Cr {
    stay: Id<IcuStay>,
    hadm_id: i64,
    stay_id: i64,
    charttime: Option<Ts>,
    creat: Option<f64>,
}

fn low(cr: &VecRel<usize, Cr>, by_stay: &HashIdx<Id<IcuStay>, usize>, span: i64) -> Fold<usize, f64> {
    cr.and(cr.map(|c: Cr| c.stay).select(by_stay).select(cr))
        .filt(move |(a, b): (Cr, Cr)| match (a.charttime, b.charttime) {
            (Some(t), Some(u)) => u < t && u >= t - span,
            _ => false,
        })
        .flat_map(|(_, b): (Cr, Cr)| b.creat)
        .fold(f64::INFINITY, f64::min)
}

pub fn kdigo_creatinine(db: &'static Db) -> Vec<KdigoCreatinine> {
    let ie = &db.icu_stay;
    let le = &db.lab_event;
    let by_subj: HashIdx<Id<Patient>, Id<IcuStay>> = (&ie.subject).inv().collect();
    let stay = Ident::<IcuStay>::new().and(&ie.hadm_id).and(&ie.stay_id).and(&ie.intime).and(&ie.outtime);
    let key = (&le.charttime)
        .and((&le.subject).select(&by_subj).select(stay))
        .filt(|(t, ((_, i), o))| t >= i - 7 * DAY_US && t <= o)
        .map(|(t, ((s, _), _))| (s, t));
    let per = (&le.id)
        .with((&le.itemid).eq(50912))
        .with((&le.valuenum).filt(|v| v <= 150.0))
        .group_by(key)
        .select(&le.valuenum)
        .fold(((0.0, 0.0), 0i64), |(s, n), v| (kahan(s, v), n + 1));
    let found = rel(drain(&per)
        .into_iter()
        .map(|((((stay, hadm_id), stay_id), t), (s, n))| Cr {
            stay,
            hadm_id,
            stay_id,
            charttime: Some(t),
            creat: Some(s.0 / n as f64),
        })
        .collect());
    let seen: HashIdx<Id<IcuStay>, usize> = (&found).map(|c: Cr| c.stay).inv().collect();
    let mut all: Vec<Cr> = drain(&found).into_iter().map(|(_, c)| c).collect();
    all.extend(drain((&ie.hadm_id).and(&ie.stay_id).minus(&seen)).into_iter().map(|(stay, (hadm_id, stay_id))| Cr {
        stay,
        hadm_id,
        stay_id,
        charttime: None,
        creat: None,
    }));
    let cr = rel(all);
    let by_stay: HashIdx<Id<IcuStay>, usize> = (&cr).map(|c: Cr| c.stay).inv().collect();
    let low48 = low(&cr, &by_stay, 48 * 3600 * US);
    let low7 = low(&cr, &by_stay, 7 * DAY_US);
    drain((&cr).and((&low48).opt()).and((&low7).opt()))
        .into_iter()
        .map(|(_, ((c, l48), l7))| KdigoCreatinine {
            hadm_id: c.hadm_id,
            stay_id: c.stay_id,
            charttime: c.charttime,
            creat: c.creat,
            creat_low_past_48hr: l48,
            creat_low_past_7day: l7,
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(kdigo_creatinine(db).into_iter().map(|k| {
        row(vec![
            V::I(k.hadm_id),
            V::I(k.stay_id),
            ots(k.charttime),
            ofloat(k.creat),
            ofloat(k.creat_low_past_48hr),
            ofloat(k.creat_low_past_7day),
        ])
    }))
}
