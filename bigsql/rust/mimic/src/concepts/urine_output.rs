use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct UrineOutput {
    pub stay_id: i64,
    pub charttime: Ts,
    pub urineoutput: f64,
}

const ITEMS: [i64; 12] = [226559, 226560, 226561, 226584, 226563, 226564, 226565, 226567, 226557, 226558, 227488, 227489];

pub fn urine_output(db: &'static Db) -> Vec<UrineOutput> {
    let oe = &db.output_event;
    let per = (&oe.id)
        .with((&oe.itemid).is_in(ITEMS))
        .group_by((&oe.stay_id).and(&oe.charttime))
        .select((&oe.itemid).and(&oe.value))
        .fold((0.0, 0.0), |a, (item, v)| kahan(a, if item == 227488 && v > 0.0 { -v } else { v }));
    drain(&per)
        .into_iter()
        .map(|((stay_id, charttime), s)| UrineOutput { stay_id, charttime, urineoutput: s.0 })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(urine_output(db).into_iter().map(|r| row(vec![V::I(r.stay_id), V::T(r.charttime), V::F(r.urineoutput)])))
}
