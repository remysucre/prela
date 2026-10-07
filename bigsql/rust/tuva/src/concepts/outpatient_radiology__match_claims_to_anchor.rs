use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::outpatient_radiology__anchor_events::*;
use crate::concepts::outpatient_radiology__generate_encounter_id::*;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;
type G = OutpatientRadiologyGenerateEncounterId;

#[derive(Clone, Copy)]
pub struct OutpatientRadiologyMatchClaimsToAnchor {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Date,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub hcpcs_code: Str,
    pub old_encounter_id: Str,
}

pub fn outpatient_radiology__match_claims_to_anchor(_db: &'static Db, stg: &[M], enc: &[G]) -> Vec<OutpatientRadiologyMatchClaimsToAnchor> {
    let m = rel(stg.to_vec());
    let g = rel(enc.to_vec());
    let gx: HashIdx<(Str, Date, Str), usize> = (&g)
        .flat_map(|x: G| x.start_date.zip(x.hcpcs_code).map(|(d, h)| (x.patient_data_source_id, d, h)))
        .inv()
        .collect();
    let j = (&m)
        .and(
            (&m).flat_map(|x: M| x.start_date.zip(x.hcpcs_code).map(|(d, h)| (x.patient_data_source_id, d, h)))
                .select(&gx)
                .select(&g),
        )
        .map(|(x, u): (M, G)| {
            (
                x.patient_data_source_id,
                x.data_source,
                x.start_date.unwrap(),
                x.claim_id,
                x.claim_line_number,
                x.hcpcs_code.unwrap(),
                u.old_encounter_id,
            )
        })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&j)
        .into_iter()
        .map(
            |((patient_data_source_id, data_source, start_date, claim_id, claim_line_number, hcpcs_code, old_encounter_id), _)| {
                OutpatientRadiologyMatchClaimsToAnchor {
                    patient_data_source_id,
                    data_source,
                    start_date,
                    claim_id,
                    claim_line_number,
                    hcpcs_code,
                    old_encounter_id,
                }
            },
        )
        .collect()
}

pub fn fmt(v: &OutpatientRadiologyMatchClaimsToAnchor) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        V::D(v.start_date),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.hcpcs_code),
        V::S(v.old_encounter_id),
    ])
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = outpatient_radiology__anchor_events(db, &s);
    let g = outpatient_radiology__generate_encounter_id(db, &a);
    rows(outpatient_radiology__match_claims_to_anchor(db, &s, &g).iter().map(fmt))
}
