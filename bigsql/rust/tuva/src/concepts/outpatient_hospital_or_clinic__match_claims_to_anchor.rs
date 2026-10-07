use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::outpatient_hospital_or_clinic__anchor_events::*;
use crate::concepts::outpatient_hospital_or_clinic__generate_encounter_id::*;
use crate::schema::Db;
use harness::prelude::*;

pub type OutpatientHospitalOrClinicMatchClaimsToAnchor = ClaimMatch;

pub fn outpatient_hospital_or_clinic__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    enc: &[OutpatientHospitalOrClinicGenerateEncounterId],
) -> Vec<OutpatientHospitalOrClinicMatchClaimsToAnchor> {
    match_claims_to_anchor(stg, enc, claim_enc_key, claim_enc_id, None)
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = outpatient_hospital_or_clinic__anchor_events(db, &s);
    let g = outpatient_hospital_or_clinic__generate_encounter_id(db, &s, &a);
    rows(outpatient_hospital_or_clinic__match_claims_to_anchor(db, &s, &g).iter().map(fmt_match))
}
