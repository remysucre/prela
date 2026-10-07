use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Chemistry {
    pub subject_id: i64,
    pub hadm_id: Option<i64>,
    pub charttime: Ts,
    pub specimen_id: i64,
    pub albumin: Option<f64>,
    pub globulin: Option<f64>,
    pub total_protein: Option<f64>,
    pub aniongap: Option<f64>,
    pub bicarbonate: Option<f64>,
    pub bun: Option<f64>,
    pub calcium: Option<f64>,
    pub chloride: Option<f64>,
    pub creatinine: Option<f64>,
    pub glucose: Option<f64>,
    pub sodium: Option<f64>,
    pub potassium: Option<f64>,
}

const ITEMS: [(i64, f64); 12] = [
    (50862, 10.0),
    (50930, 10.0),
    (50976, 20.0),
    (50868, 10000.0),
    (50882, 10000.0),
    (51006, 300.0),
    (50893, 10000.0),
    (50902, 10000.0),
    (50912, 150.0),
    (50931, 10000.0),
    (50983, 200.0),
    (50971, 30.0),
];

fn omax<T: PartialOrd>(a: Option<T>, x: Option<T>) -> Option<T> {
    match (a, x) {
        (Some(a), Some(x)) => Some(if x > a { x } else { a }),
        (a, None) => a,
        (None, x) => x,
    }
}

pub fn chemistry(db: &'static Db) -> Vec<Chemistry> {
    let le = &db.lab_event;
    let init = (i64::MIN, None::<i64>, i64::MIN as Ts, [None::<f64>; 12]);
    let per = (&le.id)
        .with((&le.itemid).is_in(ITEMS.map(|x| x.0)))
        .with((&le.itemid).and(&le.valuenum).filt(|(i, v)| v > 0.0 || i == 50868))
        .group_by(&le.specimen_id)
        .select((&le.subject_id).and((&le.hadm_id).opt()).and(&le.charttime).and(&le.itemid).and(&le.valuenum))
        .fold(init, |mut a, ((((s, h), t), item), v)| {
            a.0 = a.0.max(s);
            a.1 = omax(a.1, h);
            a.2 = a.2.max(t);
            for (k, &(id, hi)) in ITEMS.iter().enumerate() {
                if item == id && v <= hi {
                    a.3[k] = omax(a.3[k], Some(v));
                }
            }
            a
        });
    drain(&per)
        .into_iter()
        .map(|(specimen_id, (subject_id, hadm_id, charttime, m))| Chemistry {
            subject_id,
            hadm_id,
            charttime,
            specimen_id,
            albumin: m[0],
            globulin: m[1],
            total_protein: m[2],
            aniongap: m[3],
            bicarbonate: m[4],
            bun: m[5],
            calcium: m[6],
            chloride: m[7],
            creatinine: m[8],
            glucose: m[9],
            sodium: m[10],
            potassium: m[11],
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(chemistry(db).into_iter().map(|c| {
        row(vec![
            V::I(c.subject_id),
            oint(c.hadm_id),
            V::T(c.charttime),
            V::I(c.specimen_id),
            ofloat(c.albumin),
            ofloat(c.globulin),
            ofloat(c.total_protein),
            ofloat(c.aniongap),
            ofloat(c.bicarbonate),
            ofloat(c.bun),
            ofloat(c.calcium),
            ofloat(c.chloride),
            ofloat(c.creatinine),
            ofloat(c.glucose),
            ofloat(c.sodium),
            ofloat(c.potassium),
        ])
    }))
}
