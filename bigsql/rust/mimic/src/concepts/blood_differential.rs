use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct BloodDifferential {
    pub subject_id: i64,
    pub hadm_id: Option<i64>,
    pub charttime: Ts,
    pub specimen_id: i64,
    pub wbc: Option<f64>,
    pub basophils_abs: Option<f64>,
    pub eosinophils_abs: Option<f64>,
    pub lymphocytes_abs: Option<f64>,
    pub monocytes_abs: Option<f64>,
    pub neutrophils_abs: Option<f64>,
    pub basophils: Option<f64>,
    pub eosinophils: Option<f64>,
    pub lymphocytes: Option<f64>,
    pub monocytes: Option<f64>,
    pub neutrophils: Option<f64>,
    pub atypical_lymphocytes: Option<f64>,
    pub bands: Option<f64>,
    pub immature_granulocytes: Option<f64>,
    pub metamyelocytes: Option<f64>,
    pub nrbc: Option<f64>,
}

const ITEMS: [i64; 23] = [
    51146, 52069, 51199, 51200, 52073, 51244, 51245, 51133, 52769, 51253, 51254, 52074, 51256, 52075, 51143, 51144,
    51218, 52135, 51251, 51257, 51300, 51301, 51755,
];

const PCT: [i64; 6] = [51146, 51200, 51244, 51245, 51254, 51256];

fn omax<T: PartialOrd>(a: Option<T>, x: Option<T>) -> Option<T> {
    match (a, x) {
        (Some(a), Some(x)) => Some(if x > a { x } else { a }),
        (a, None) => a,
        (None, x) => x,
    }
}

fn col(item: i64, v: f64) -> [Option<f64>; 16] {
    let is = |ids: &[i64]| ids.contains(&item);
    let one = |id: i64| if item == id { Some(v) } else { None };
    let two = |id: i64, milli: i64| {
        if item == id {
            Some(v)
        } else if item == milli {
            Some(v / 1000.0)
        } else {
            None
        }
    };
    [
        if is(&[51300, 51301, 51755]) { Some(v) } else { None },
        one(52069),
        two(52073, 51199),
        two(51133, 52769),
        two(52074, 51253),
        one(52075),
        one(51146),
        one(51200),
        if is(&[51244, 51245]) { Some(v) } else { None },
        one(51254),
        one(51256),
        one(51143),
        one(51144),
        one(52135),
        one(51251),
        one(51257),
    ]
}

pub fn blood_differential(db: &'static Db) -> Vec<BloodDifferential> {
    let le = &db.lab_event;
    let init = (i64::MIN, None::<i64>, i64::MIN as Ts, [None::<f64>; 16], None::<(f64, f64)>);
    let per = (&le.id)
        .with((&le.itemid).is_in(ITEMS))
        .with((&le.valuenum).filt(|v| v >= 0.0))
        .group_by(&le.specimen_id)
        .select((&le.subject_id).and((&le.hadm_id).opt()).and(&le.charttime).and(&le.itemid).and(&le.valuenum))
        .fold(init, |mut a, ((((s, h), t), item), v)| {
            a.0 = a.0.max(s);
            a.1 = omax(a.1, h);
            a.2 = a.2.max(t);
            for (m, x) in a.3.iter_mut().zip(col(item, v)) {
                *m = omax(*m, x);
            }
            if PCT.contains(&item) {
                a.4 = Some(kahan(a.4.unwrap_or((0.0, 0.0)), v));
            }
            a
        });
    drain(&per)
        .into_iter()
        .map(|(specimen_id, (subject_id, hadm_id, charttime, m, pct))| {
            let impute = m[0].is_some_and(|w| w > 0.0) && pct.is_some_and(|p| p.0 > 0.0);
            let abs = |a: Option<f64>, p: Option<f64>| {
                match (a, p, m[0]) {
                    (None, Some(p), Some(w)) if impute => Some(p * w / 100.0),
                    _ => a,
                }
                .map(|x| round_dec(x, 4))
            };
            BloodDifferential {
                subject_id,
                hadm_id,
                charttime,
                specimen_id,
                wbc: m[0],
                basophils_abs: abs(m[1], m[6]),
                eosinophils_abs: abs(m[2], m[7]),
                lymphocytes_abs: abs(m[3], m[8]),
                monocytes_abs: abs(m[4], m[9]),
                neutrophils_abs: abs(m[5], m[10]),
                basophils: m[6],
                eosinophils: m[7],
                lymphocytes: m[8],
                monocytes: m[9],
                neutrophils: m[10],
                atypical_lymphocytes: m[11],
                bands: m[12],
                immature_granulocytes: m[13],
                metamyelocytes: m[14],
                nrbc: m[15],
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(blood_differential(db).into_iter().map(|c| {
        row(vec![
            V::I(c.subject_id),
            oint(c.hadm_id),
            V::T(c.charttime),
            V::I(c.specimen_id),
            ofloat(c.wbc),
            ofloat(c.basophils_abs),
            ofloat(c.eosinophils_abs),
            ofloat(c.lymphocytes_abs),
            ofloat(c.monocytes_abs),
            ofloat(c.neutrophils_abs),
            ofloat(c.basophils),
            ofloat(c.eosinophils),
            ofloat(c.lymphocytes),
            ofloat(c.monocytes),
            ofloat(c.neutrophils),
            ofloat(c.atypical_lymphocytes),
            ofloat(c.bands),
            ofloat(c.immature_granulocytes),
            ofloat(c.metamyelocytes),
            ofloat(c.nrbc),
        ])
    }))
}
