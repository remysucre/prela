use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Norepinephrine {
    pub stay_id: i64,
    pub linkorderid: i64,
    pub vaso_rate: Option<f64>,
    pub vaso_amount: f64,
    pub starttime: Ts,
    pub endtime: Ts,
}

pub fn norepinephrine(db: &'static Db) -> Vec<Norepinephrine> {
    let ie = &db.input_event;
    let r = (&ie.id)
        .with((&ie.itemid).filt(|i| i == 221906))
        .select(
            (&ie.stay_id)
                .and(&ie.linkorderid)
                .and((&ie.rate).opt())
                .and((&ie.rateuom).opt())
                .and(&ie.patientweight)
                .and(&ie.amount)
                .and(&ie.starttime)
                .and(&ie.endtime),
        );
    drain(&r)
        .into_iter()
        .map(|(_, (((((((stay_id, linkorderid), rate), rateuom), weight), vaso_amount), starttime), endtime))| Norepinephrine {
            stay_id,
            linkorderid,
            vaso_rate: match rateuom {
                Some("mg/kg/min") if weight == 1.0 => rate,
                Some("mg/kg/min") => rate.map(|r| (r as f32 * 1000f32) as f64),
                _ => rate,
            },
            vaso_amount,
            starttime,
            endtime,
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(norepinephrine(db).into_iter().map(|v| {
        row(vec![
            V::I(v.stay_id),
            V::I(v.linkorderid),
            ofloat(v.vaso_rate),
            V::F(v.vaso_amount),
            V::T(v.starttime),
            V::T(v.endtime),
        ])
    }))
}
