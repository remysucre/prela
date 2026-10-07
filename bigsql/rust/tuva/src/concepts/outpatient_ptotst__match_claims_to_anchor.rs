use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::outpatient_ptotst__anchor_events::*;
use crate::concepts::outpatient_ptotst__generate_encounter_id::*;
use crate::schema::Db;
use harness::prelude::*;

pub type OutpatientPtotstMatchClaimsToAnchor = ClaimMatch;

pub fn outpatient_ptotst__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    enc: &[OutpatientPtotstGenerateEncounterId],
) -> Vec<OutpatientPtotstMatchClaimsToAnchor> {
    match_claims_to_anchor(stg, enc, claim_enc_key, claim_enc_id, None)
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = outpatient_ptotst__anchor_events(db, &s);
    let g = outpatient_ptotst__generate_encounter_id(db, &s, &a);
    rows(outpatient_ptotst__match_claims_to_anchor(db, &s, &g).iter().map(fmt_match))
}
