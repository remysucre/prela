use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct VentilatorSetting {
    pub subject_id: i64,
    pub stay_id: i64,
    pub charttime: Ts,
    pub respiratory_rate_set: Option<f64>,
    pub respiratory_rate_total: Option<f64>,
    pub respiratory_rate_spontaneous: Option<f64>,
    pub minute_volume: Option<f64>,
    pub tidal_volume_set: Option<f64>,
    pub tidal_volume_observed: Option<f64>,
    pub tidal_volume_spontaneous: Option<f64>,
    pub plateau_pressure: Option<f64>,
    pub peep: Option<f64>,
    pub fio2: Option<f64>,
    pub flow_rate: Option<f64>,
    pub ventilator_mode: Option<Str>,
    pub ventilator_mode_hamilton: Option<Str>,
    pub ventilator_type: Option<Str>,
}

const ITEMS: [i64; 15] = [
    224688, 224689, 224690, 224687, 224685, 224684, 224686, 224696, 220339, 224700, 223835, 223849, 229314, 223848,
    224691,
];

const NUM: [i64; 11] = [224688, 224690, 224689, 224687, 224684, 224685, 224686, 224696, 220339, 223835, 224691];
const TXT: [i64; 3] = [223849, 229314, 223848];

#[derive(Clone, Copy)]
struct Acc {
    stay_id: i64,
    num: [Option<f64>; 11],
    txt: [Option<Str>; 3],
}

fn omax<T: PartialOrd + Copy>(a: Option<T>, x: Option<T>) -> Option<T> {
    match (a, x) {
        (Some(a), Some(x)) => Some(if x > a { x } else { a }),
        (a, None) => a,
        (None, x) => x,
    }
}

fn clean(item: i64, v: Option<f64>) -> Option<f64> {
    let v = v?;
    match item {
        223835 => {
            if (0.20..=1.0).contains(&v) {
                Some((v as f32 * 100f32) as f64)
            } else if v >= 20.0 && v <= 100.0 {
                Some(v)
            } else {
                None
            }
        }
        220339 | 224700 => (v >= 0.0 && v <= 100.0).then_some(v),
        _ => Some(v),
    }
}

pub fn ventilator_setting(db: &'static Db) -> Vec<VentilatorSetting> {
    let ce = &db.chart_event;
    let init = Acc { stay_id: i64::MIN, num: [None; 11], txt: [None; 3] };
    let per = (&ce.id)
        .with((&ce.itemid).is_in(ITEMS))
        .group_by((&ce.subject_id).and(&ce.charttime))
        .select((&ce.stay_id).and(&ce.itemid).and((&ce.valuenum).opt()).and(&ce.value))
        .fold(init, |mut a, (((stay_id, item), v), value)| {
            a.stay_id = a.stay_id.max(stay_id);
            let v = clean(item, v);
            let item = if item == 224700 { 220339 } else { item };
            if let Some(i) = NUM.iter().position(|&x| x == item) {
                a.num[i] = omax(a.num[i], v);
            }
            if let Some(i) = TXT.iter().position(|&x| x == item) {
                a.txt[i] = omax(a.txt[i], Some(value));
            }
            a
        });
    drain(&per)
        .into_iter()
        .map(|((subject_id, charttime), a)| VentilatorSetting {
            subject_id,
            stay_id: a.stay_id,
            charttime,
            respiratory_rate_set: a.num[0],
            respiratory_rate_total: a.num[1],
            respiratory_rate_spontaneous: a.num[2],
            minute_volume: a.num[3],
            tidal_volume_set: a.num[4],
            tidal_volume_observed: a.num[5],
            tidal_volume_spontaneous: a.num[6],
            plateau_pressure: a.num[7],
            peep: a.num[8],
            fio2: a.num[9],
            flow_rate: a.num[10],
            ventilator_mode: a.txt[0],
            ventilator_mode_hamilton: a.txt[1],
            ventilator_type: a.txt[2],
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(ventilator_setting(db).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.stay_id),
            V::T(v.charttime),
            ofloat(v.respiratory_rate_set),
            ofloat(v.respiratory_rate_total),
            ofloat(v.respiratory_rate_spontaneous),
            ofloat(v.minute_volume),
            ofloat(v.tidal_volume_set),
            ofloat(v.tidal_volume_observed),
            ofloat(v.tidal_volume_spontaneous),
            ofloat(v.plateau_pressure),
            ofloat(v.peep),
            ofloat(v.fio2),
            ofloat(v.flow_rate),
            ostr(v.ventilator_mode),
            ostr(v.ventilator_mode_hamilton),
            ostr(v.ventilator_type),
        ])
    }))
}
