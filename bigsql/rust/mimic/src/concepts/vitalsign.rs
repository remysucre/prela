use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Vitalsign {
    pub subject_id: i64,
    pub stay_id: i64,
    pub charttime: Ts,
    pub heart_rate: Option<f64>,
    pub sbp: Option<f64>,
    pub dbp: Option<f64>,
    pub mbp: Option<f64>,
    pub sbp_ni: Option<f64>,
    pub dbp_ni: Option<f64>,
    pub mbp_ni: Option<f64>,
    pub resp_rate: Option<f64>,
    pub temperature: Option<f64>,
    pub temperature_site: Option<Str>,
    pub spo2: Option<f64>,
    pub glucose: Option<f64>,
}

const ITEMS: [i64; 19] = [
    220045, 225309, 225310, 225312, 220050, 220051, 220052, 220179, 220180, 220181, 220210, 224690, 220277, 225664,
    220621, 226537, 223762, 223761, 224642,
];

#[derive(Clone, Copy)]
struct Acc {
    avg: [((f64, f64), i64); 11],
    site: Option<Str>,
}

fn add(a: ((f64, f64), i64), x: Option<f64>) -> ((f64, f64), i64) {
    match x {
        Some(x) => (kahan(a.0, x), a.1 + 1),
        None => a,
    }
}

fn mean(a: ((f64, f64), i64)) -> Option<f64> {
    (a.1 > 0).then(|| a.0.0 / a.1 as f64)
}

pub fn vitalsign(db: &'static Db) -> Vec<Vitalsign> {
    let ce = &db.chart_event;
    let init = Acc { avg: [((0.0, 0.0), 0); 11], site: None };
    let per = (&ce.id)
        .with(&ce.stay_id)
        .with((&ce.itemid).is_in(ITEMS))
        .group_by((&ce.subject_id).and(&ce.stay_id).and(&ce.charttime))
        .select((&ce.itemid).and((&ce.valuenum).opt()).and(&ce.value))
        .fold(init, |mut a, ((item, v), value)| {
            let r = |lo: f64, hi: f64| v.filter(|&x| x > lo && x < hi);
            let hit = |items: &[i64]| items.contains(&item);
            let cols: [Option<f64>; 11] = [
                if item == 220045 { r(0.0, 300.0) } else { None },
                if hit(&[220179, 220050, 225309]) { r(0.0, 400.0) } else { None },
                if hit(&[220180, 220051, 225310]) { r(0.0, 300.0) } else { None },
                if hit(&[220052, 220181, 225312]) { r(0.0, 300.0) } else { None },
                if item == 220179 { r(0.0, 400.0) } else { None },
                if item == 220180 { r(0.0, 300.0) } else { None },
                if item == 220181 { r(0.0, 300.0) } else { None },
                if hit(&[220210, 224690]) { r(0.0, 70.0) } else { None },
                if item == 223761 {
                    r(70.0, 120.0).map(|x| ((x as f32 - 32f32) / 1.8f32) as f64)
                } else if item == 223762 {
                    r(10.0, 50.0)
                } else {
                    None
                },
                if item == 220277 { v.filter(|&x| x > 0.0 && x <= 100.0) } else { None },
                if hit(&[225664, 220621, 226537]) { v.filter(|&x| x > 0.0) } else { None },
            ];
            for (s, c) in a.avg.iter_mut().zip(cols) {
                *s = add(*s, c);
            }
            if item == 224642 {
                a.site = Some(a.site.map_or(value, |s| s.max(value)));
            }
            a
        });
    drain(&per)
        .into_iter()
        .map(|(((subject_id, stay_id), charttime), a)| {
            let m: Vec<Option<f64>> = a.avg.iter().map(|&x| mean(x)).collect();
            Vitalsign {
                subject_id,
                stay_id,
                charttime,
                heart_rate: m[0],
                sbp: m[1],
                dbp: m[2],
                mbp: m[3],
                sbp_ni: m[4],
                dbp_ni: m[5],
                mbp_ni: m[6],
                resp_rate: m[7],
                temperature: m[8].map(|x| round_dec(x, 2)),
                temperature_site: a.site,
                spo2: m[9],
                glucose: m[10],
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(vitalsign(db).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.stay_id),
            V::T(v.charttime),
            ofloat(v.heart_rate),
            ofloat(v.sbp),
            ofloat(v.dbp),
            ofloat(v.mbp),
            ofloat(v.sbp_ni),
            ofloat(v.dbp_ni),
            ofloat(v.mbp_ni),
            ofloat(v.resp_rate),
            ofloat(v.temperature),
            ostr(v.temperature_site),
            ofloat(v.spo2),
            ofloat(v.glucose),
        ])
    }))
}
