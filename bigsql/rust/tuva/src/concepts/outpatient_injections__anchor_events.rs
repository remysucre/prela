use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::encounters__stg_outpatient_institutional::EncountersStgOutpatientInstitutional;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;
type O = EncountersStgOutpatientInstitutional;

#[derive(Clone, Copy)]
pub struct OutpatientInjectionsAnchorEvents {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Option<Date>,
    pub tuva_last_run: Ts,
}

pub fn outpatient_injections__anchor_events(_db: &'static Db, stg: &[M], op: &[O]) -> Vec<OutpatientInjectionsAnchorEvents> {
    let m = rel(stg.to_vec());
    let o = rel(op.to_vec());
    let ox: HashIdx<(Str, Str), usize> = (&o).map(|x: O| (x.claim_id, x.data_source)).inv().collect();
    let ms = (&m)
        .filt(|x: M| x.hcpcs_code.is_some_and(|h| h.starts_with('J')))
        .and((&m).map(|x: M| (x.claim_id, x.data_source)).select(&ox))
        .map(|(x, _): (M, usize)| (x.patient_data_source_id, x.data_source, x.start_date))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&ms)
        .into_iter()
        .map(|((patient_data_source_id, data_source, start_date), _)| OutpatientInjectionsAnchorEvents {
            patient_data_source_id,
            data_source,
            start_date,
            tuva_last_run: run,
        })
        .collect()
}

pub fn fmt(v: &OutpatientInjectionsAnchorEvents) -> String {
    row(vec![V::S(v.patient_data_source_id), V::S(v.data_source), odate(v.start_date), V::T(v.tuva_last_run)])
}

pub fn q(db: &'static Db) -> String {
    let s = crate::concepts::anchor_kit::stg(db);
    let op = crate::concepts::encounters__stg_outpatient_institutional::encounters__stg_outpatient_institutional(db);
    rows(outpatient_injections__anchor_events(db, &s, &op).iter().map(fmt))
}
