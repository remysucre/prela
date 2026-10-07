use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct CompleteBloodCount {
    pub subject_id: i64,
    pub hadm_id: Option<i64>,
    pub charttime: Ts,
    pub specimen_id: i64,
    pub hematocrit: Option<f64>,
    pub hemoglobin: Option<f64>,
    pub mch: Option<f64>,
    pub mchc: Option<f64>,
    pub mcv: Option<f64>,
    pub platelet: Option<f64>,
    pub rbc: Option<f64>,
    pub rdw: Option<f64>,
    pub rdwsd: Option<f64>,
    pub wbc: Option<f64>,
}

const ITEMS: [i64; 10] = [51221, 51222, 51248, 51249, 51250, 51265, 51279, 51277, 52159, 51301];

fn omax<T: PartialOrd>(a: Option<T>, x: Option<T>) -> Option<T> {
    match (a, x) {
        (Some(a), Some(x)) => Some(if x > a { x } else { a }),
        (a, None) => a,
        (None, x) => x,
    }
}

pub fn complete_blood_count(db: &'static Db) -> Vec<CompleteBloodCount> {
    let le = &db.lab_event;
    let init = (i64::MIN, None::<i64>, i64::MIN as Ts, [None::<f64>; 10]);
    let per = (&le.id)
        .with((&le.itemid).is_in(ITEMS))
        .with((&le.valuenum).filt(|v| v > 0.0))
        .group_by(&le.specimen_id)
        .select((&le.subject_id).and((&le.hadm_id).opt()).and(&le.charttime).and(&le.itemid).and(&le.valuenum))
        .fold(init, |mut a, ((((s, h), t), item), v)| {
            a.0 = a.0.max(s);
            a.1 = omax(a.1, h);
            a.2 = a.2.max(t);
            for (k, &id) in ITEMS.iter().enumerate() {
                if item == id {
                    a.3[k] = omax(a.3[k], Some(v));
                }
            }
            a
        });
    drain(&per)
        .into_iter()
        .map(|(specimen_id, (subject_id, hadm_id, charttime, m))| CompleteBloodCount {
            subject_id,
            hadm_id,
            charttime,
            specimen_id,
            hematocrit: m[0],
            hemoglobin: m[1],
            mch: m[2],
            mchc: m[3],
            mcv: m[4],
            platelet: m[5],
            rbc: m[6],
            rdw: m[7],
            rdwsd: m[8],
            wbc: m[9],
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(complete_blood_count(db).into_iter().map(|c| {
        row(vec![
            V::I(c.subject_id),
            oint(c.hadm_id),
            V::T(c.charttime),
            V::I(c.specimen_id),
            ofloat(c.hematocrit),
            ofloat(c.hemoglobin),
            ofloat(c.mch),
            ofloat(c.mchc),
            ofloat(c.mcv),
            ofloat(c.platelet),
            ofloat(c.rbc),
            ofloat(c.rdw),
            ofloat(c.rdwsd),
            ofloat(c.wbc),
        ])
    }))
}
