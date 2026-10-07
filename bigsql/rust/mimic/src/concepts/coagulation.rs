use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Coagulation {
    pub subject_id: i64,
    pub hadm_id: Option<i64>,
    pub charttime: Ts,
    pub specimen_id: i64,
    pub d_dimer: Option<f64>,
    pub fibrinogen: Option<f64>,
    pub thrombin: Option<f64>,
    pub inr: Option<f64>,
    pub pt: Option<f64>,
    pub ptt: Option<f64>,
}

const ITEMS: [i64; 6] = [51196, 51214, 51297, 51237, 51274, 51275];

fn omax<T: PartialOrd>(a: Option<T>, x: Option<T>) -> Option<T> {
    match (a, x) {
        (Some(a), Some(x)) => Some(if x > a { x } else { a }),
        (a, None) => a,
        (None, x) => x,
    }
}

pub fn coagulation(db: &'static Db) -> Vec<Coagulation> {
    let le = &db.lab_event;
    let init = (i64::MIN, None::<i64>, i64::MIN as Ts, [None::<f64>; 6]);
    let per = (&le.id)
        .with((&le.itemid).is_in(ITEMS))
        .with(&le.valuenum)
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
        .map(|(specimen_id, (subject_id, hadm_id, charttime, m))| Coagulation {
            subject_id,
            hadm_id,
            charttime,
            specimen_id,
            d_dimer: m[0],
            fibrinogen: m[1],
            thrombin: m[2],
            inr: m[3],
            pt: m[4],
            ptt: m[5],
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(coagulation(db).into_iter().map(|c| {
        row(vec![
            V::I(c.subject_id),
            oint(c.hadm_id),
            V::T(c.charttime),
            V::I(c.specimen_id),
            ofloat(c.d_dimer),
            ofloat(c.fibrinogen),
            ofloat(c.thrombin),
            ofloat(c.inr),
            ofloat(c.pt),
            ofloat(c.ptt),
        ])
    }))
}
