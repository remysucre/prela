use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;

#[derive(Clone, Copy)]
pub struct OutpatientRadiologyAnchorEvents {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Option<Date>,
    pub hcpcs_code: Option<Str>,
    pub tuva_last_run: Ts,
}

pub fn outpatient_radiology__anchor_events(_db: &'static Db, stg: &[M]) -> Vec<OutpatientRadiologyAnchorEvents> {
    let m = rel(stg.to_vec());
    let sc = (&m)
        .filt(|x: M| x.service_category_2 == "outpatient radiology")
        .map(|x: M| (x.patient_data_source_id, x.data_source, x.start_date, x.hcpcs_code))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&sc)
        .into_iter()
        .map(|((patient_data_source_id, data_source, start_date, hcpcs_code), _)| OutpatientRadiologyAnchorEvents {
            patient_data_source_id,
            data_source,
            start_date,
            hcpcs_code,
            tuva_last_run: run,
        })
        .collect()
}

pub fn fmt(v: &OutpatientRadiologyAnchorEvents) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        odate(v.start_date),
        ostr(v.hcpcs_code),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let s = crate::concepts::anchor_kit::stg(db);
    rows(outpatient_radiology__anchor_events(db, &s).iter().map(fmt))
}
