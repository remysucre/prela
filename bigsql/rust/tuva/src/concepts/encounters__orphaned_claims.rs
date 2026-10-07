use crate::concepts as c;
use crate::concepts::anchor_kit::surrogate_key;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::encounters__combined_claim_line_crosswalk::EncountersCombinedClaimLineCrosswalk;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;
type X = EncountersCombinedClaimLineCrosswalk;
type K3 = (Str, i64, Str);

#[derive(Clone, Copy)]
pub struct EncountersOrphanedClaims {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub encounter_id: Str,
    pub encounter_type: Str,
    pub encounter_group: Str,
}

pub fn encounters__orphaned_claims(_db: &'static Db, stg: &[M], xw: &[X]) -> Vec<EncountersOrphanedClaims> {
    let m = rel(stg.to_vec());
    let x = rel(xw.to_vec());
    let enc: HashIdx<K3, usize> = (&x).map(|e: X| (e.claim_id, e.claim_line_number, e.data_source)).inv().collect();
    let o = (&m)
        .map(|s: M| (s.claim_id, s.claim_line_number, s.data_source))
        .inv()
        .minus(&enc)
        .select(&m);
    drain(&o)
        .into_iter()
        .map(|(_, s): (K3, M)| EncountersOrphanedClaims {
            claim_id: s.claim_id,
            claim_line_number: s.claim_line_number,
            data_source: s.data_source,
            encounter_id: surrogate_key(&[Some("orphaned claim"), Some(s.patient_data_source_id), Some(s.claim_id)]),
            encounter_type: "orphaned claim",
            encounter_group: "other",
        })
        .collect()
}

pub fn fmt(v: &EncountersOrphanedClaims) -> String {
    row(vec![
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.data_source),
        V::S(v.encounter_id),
        V::S(v.encounter_type),
        V::S(v.encounter_group),
    ])
}

pub fn q(db: &'static Db) -> String {
    let stg = c::anchor_kit::stg(db);
    let xw = crate::concepts::encounters__combined_claim_line_crosswalk::chain(db, &stg);
    rows(encounters__orphaned_claims(db, &stg, &xw).iter().map(fmt))
}
