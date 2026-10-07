use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::ambulance__anchor_events::*;
use crate::concepts::ambulance__generate_encounter_id::*;
use crate::schema::Db;
use harness::prelude::*;

pub type AmbulanceMatchClaimsToAnchor = ClaimMatch;

pub fn ambulance__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    enc: &[AmbulanceGenerateEncounterId],
) -> Vec<AmbulanceMatchClaimsToAnchor> {
    match_claims_to_anchor(stg, enc, claim_enc_key, claim_enc_id, Some("ambulance"))
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = ambulance__anchor_events(db, &s);
    let g = ambulance__generate_encounter_id(db, &s, &a);
    rows(ambulance__match_claims_to_anchor(db, &s, &g).iter().map(fmt_match))
}
