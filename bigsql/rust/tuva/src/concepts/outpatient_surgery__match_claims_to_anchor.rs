use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::outpatient_surgery__anchor_events::*;
use crate::concepts::outpatient_surgery__generate_encounter_id::*;
use crate::schema::Db;
use harness::prelude::*;

pub type OutpatientSurgeryMatchClaimsToAnchor = ClaimMatch;

pub fn outpatient_surgery__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    enc: &[OutpatientSurgeryGenerateEncounterId],
) -> Vec<OutpatientSurgeryMatchClaimsToAnchor> {
    match_claims_to_anchor(stg, enc, claim_enc_key, claim_enc_id, None)
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = outpatient_surgery__anchor_events(db, &s);
    let g = outpatient_surgery__generate_encounter_id(db, &s, &a);
    rows(outpatient_surgery__match_claims_to_anchor(db, &s, &g).iter().map(fmt_match))
}
