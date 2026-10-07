use crate::concepts::emergency_department__generate_encounter_id_pre_sort::{
    EmergencyDepartmentGenerateEncounterIdPreSort, cmp3, cmp3_desc, surrogate,
};
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct EmergencyDepartmentGenerateEncounterId {
    pub patient_data_source_id: Str,
    pub claim_id: Str,
    pub start_date: Option<Date>,
    pub end_date: Option<Date>,
    pub discharge_disposition_code: Option<Str>,
    pub facility_npi: Option<Str>,
    pub encounter_claim_number: i64,
    pub encounter_claim_number_desc: i64,
    pub close_flag: i64,
    pub min_closing_row: Option<i64>,
    pub encounter_id: Str,
    pub original_anchor_claim: Option<Str>,
}

type P = EmergencyDepartmentGenerateEncounterIdPreSort;

pub fn emergency_department__generate_encounter_id(_db: &'static Db, pre: &[P]) -> Vec<EmergencyDepartmentGenerateEncounterId> {
    let p = rel(pre.to_vec());
    let key = |x: P| (x.start_date, x.end_date, x.claim_id);
    let w = (&p)
        .map(|x: P| (x.patient_data_source_id, x.encounter_id))
        .inv()
        .select(&p)
        .window(row_number, key, cmp3)
        .window(row_number, move |(x, _): (P, i64)| key(x), cmp3_desc);
    drain(&w)
        .into_iter()
        .map(|(_, ((x, n), nd))| EmergencyDepartmentGenerateEncounterId {
            patient_data_source_id: x.patient_data_source_id,
            claim_id: x.claim_id,
            start_date: x.start_date,
            end_date: x.end_date,
            discharge_disposition_code: x.discharge_disposition_code,
            facility_npi: x.facility_npi,
            encounter_claim_number: n,
            encounter_claim_number_desc: nd,
            close_flag: x.close_flag,
            min_closing_row: x.min_closing_row,
            encounter_id: surrogate("emergency department", x.patient_data_source_id, x.encounter_id),
            original_anchor_claim: x.encounter_id,
        })
        .collect()
}

pub fn fmt(v: &EmergencyDepartmentGenerateEncounterId) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.claim_id),
        odate(v.start_date),
        odate(v.end_date),
        ostr(v.discharge_disposition_code),
        ostr(v.facility_npi),
        V::I(v.encounter_claim_number),
        V::I(v.encounter_claim_number_desc),
        V::I(v.close_flag),
        oint(v.min_closing_row),
        V::S(v.encounter_id),
        ostr(v.original_anchor_claim),
    ])
}

pub fn q(db: &'static Db) -> String {
    let stg = crate::concepts::emergency_department__generate_encounter_id_pre_sort::stg_medical_claim(db);
    let pre = crate::concepts::emergency_department__generate_encounter_id_pre_sort::emergency_department__generate_encounter_id_pre_sort(db, &stg);
    rows(emergency_department__generate_encounter_id(db, &pre).iter().map(fmt))
}
