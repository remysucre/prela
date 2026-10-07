use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Enzyme {
    pub subject_id: i64,
    pub hadm_id: Option<i64>,
    pub charttime: Ts,
    pub specimen_id: i64,
    pub alt: Option<f64>,
    pub alp: Option<f64>,
    pub ast: Option<f64>,
    pub amylase: Option<f64>,
    pub bilirubin_total: Option<f64>,
    pub bilirubin_direct: Option<f64>,
    pub bilirubin_indirect: Option<f64>,
    pub ck_cpk: Option<f64>,
    pub ck_mb: Option<f64>,
    pub ggt: Option<f64>,
    pub ld_ldh: Option<f64>,
}

const ITEMS: [i64; 11] = [50861, 50863, 50878, 50867, 50885, 50883, 50884, 50910, 50911, 50927, 50954];

fn omax<T: PartialOrd>(a: Option<T>, x: Option<T>) -> Option<T> {
    match (a, x) {
        (Some(a), Some(x)) => Some(if x > a { x } else { a }),
        (a, None) => a,
        (None, x) => x,
    }
}

pub fn enzyme(db: &'static Db) -> Vec<Enzyme> {
    let le = &db.lab_event;
    let init = (i64::MIN, None::<i64>, i64::MIN as Ts, [None::<f64>; 11]);
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
        .map(|(specimen_id, (subject_id, hadm_id, charttime, m))| Enzyme {
            subject_id,
            hadm_id,
            charttime,
            specimen_id,
            alt: m[0],
            alp: m[1],
            ast: m[2],
            amylase: m[3],
            bilirubin_total: m[4],
            bilirubin_direct: m[5],
            bilirubin_indirect: m[6],
            ck_cpk: m[7],
            ck_mb: m[8],
            ggt: m[9],
            ld_ldh: m[10],
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(enzyme(db).into_iter().map(|c| {
        row(vec![
            V::I(c.subject_id),
            oint(c.hadm_id),
            V::T(c.charttime),
            V::I(c.specimen_id),
            ofloat(c.alt),
            ofloat(c.alp),
            ofloat(c.ast),
            ofloat(c.amylase),
            ofloat(c.bilirubin_total),
            ofloat(c.bilirubin_direct),
            ofloat(c.bilirubin_indirect),
            ofloat(c.ck_cpk),
            ofloat(c.ck_mb),
            ofloat(c.ggt),
            ofloat(c.ld_ldh),
        ])
    }))
}
