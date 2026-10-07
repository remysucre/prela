use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::outpatient_substance_use__anchor_events::*;
use crate::concepts::outpatient_substance_use__generate_encounter_id::*;
use crate::schema::Db;
use harness::prelude::*;

pub type OutpatientSubstanceUseMatchClaimsToAnchor = ClaimMatch;

pub fn outpatient_substance_use__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    enc: &[OutpatientSubstanceUseGenerateEncounterId],
) -> Vec<OutpatientSubstanceUseMatchClaimsToAnchor> {
    match_claims_to_anchor(stg, enc, claim_enc_key, claim_enc_id, None)
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = outpatient_substance_use__anchor_events(db, &s);
    let g = outpatient_substance_use__generate_encounter_id(db, &s, &a);
    rows(outpatient_substance_use__match_claims_to_anchor(db, &s, &g).iter().map(fmt_match))
}
