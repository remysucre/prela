use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::lab__anchor_events::*;
use crate::concepts::lab__generate_encounter_id::*;
use crate::schema::Db;
use harness::prelude::*;

pub type LabMatchClaimsToAnchor = ClaimMatch;

pub fn lab__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    enc: &[LabGenerateEncounterId],
) -> Vec<LabMatchClaimsToAnchor> {
    match_claims_to_anchor(stg, enc, claim_enc_key, claim_enc_id, Some("lab"))
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = lab__anchor_events(db, &s);
    let g = lab__generate_encounter_id(db, &s, &a);
    rows(lab__match_claims_to_anchor(db, &s, &g).iter().map(fmt_match))
}
