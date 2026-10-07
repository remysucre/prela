use crate::concepts::anchor_kit::*;
use crate::concepts::outpatient_radiology__anchor_events::*;
use crate::schema::Db;
use harness::prelude::*;

type A = OutpatientRadiologyAnchorEvents;

#[derive(Clone, Copy)]
pub struct OutpatientRadiologyGenerateEncounterId {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Option<Date>,
    pub hcpcs_code: Option<Str>,
    pub old_encounter_id: Str,
}

pub fn outpatient_radiology__generate_encounter_id(_db: &'static Db, anchors: &[A]) -> Vec<OutpatientRadiologyGenerateEncounterId> {
    let a = rel(anchors.to_vec());
    let g = (&a).map(|x: A| OutpatientRadiologyGenerateEncounterId {
        patient_data_source_id: x.patient_data_source_id,
        data_source: x.data_source,
        start_date: x.start_date,
        hcpcs_code: x.hcpcs_code,
        old_encounter_id: surrogate_key(&[
            Some("outpatient radiology"),
            Some(x.patient_data_source_id),
            date_text(x.start_date).as_deref(),
            x.hcpcs_code,
        ]),
    });
    drain(&g).into_iter().map(|(_, r)| r).collect()
}

pub fn fmt(v: &OutpatientRadiologyGenerateEncounterId) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        odate(v.start_date),
        ostr(v.hcpcs_code),
        V::S(v.old_encounter_id),
    ])
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = outpatient_radiology__anchor_events(db, &s);
    rows(outpatient_radiology__generate_encounter_id(db, &a).iter().map(fmt))
}
