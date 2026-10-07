use crate::concepts::anchor_kit::*;
use crate::concepts::outpatient_injections__anchor_events::*;
use crate::schema::Db;
use harness::prelude::*;

type A = OutpatientInjectionsAnchorEvents;

#[derive(Clone, Copy)]
pub struct OutpatientInjectionsGenerateEncounterId {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Option<Date>,
    pub old_encounter_id: Str,
}

pub fn outpatient_injections__generate_encounter_id(_db: &'static Db, anchors: &[A]) -> Vec<OutpatientInjectionsGenerateEncounterId> {
    let a = rel(anchors.to_vec());
    let g = (&a).map(|x: A| OutpatientInjectionsGenerateEncounterId {
        patient_data_source_id: x.patient_data_source_id,
        data_source: x.data_source,
        start_date: x.start_date,
        old_encounter_id: surrogate_key(&[
            Some("outpatient injections"),
            Some(x.patient_data_source_id),
            date_text(x.start_date).as_deref(),
        ]),
    });
    drain(&g).into_iter().map(|(_, r)| r).collect()
}

pub fn fmt(v: &OutpatientInjectionsGenerateEncounterId) -> String {
    row(vec![V::S(v.patient_data_source_id), V::S(v.data_source), odate(v.start_date), V::S(v.old_encounter_id)])
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let op = crate::concepts::encounters__stg_outpatient_institutional::encounters__stg_outpatient_institutional(db);
    let a = outpatient_injections__anchor_events(db, &s, &op);
    rows(outpatient_injections__generate_encounter_id(db, &a).iter().map(fmt))
}
