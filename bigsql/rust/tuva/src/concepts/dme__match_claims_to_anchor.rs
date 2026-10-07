use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::dme__anchor_events::*;
use crate::concepts::dme__generate_encounter_id::*;
use crate::schema::Db;
use harness::prelude::*;

pub type DmeMatchClaimsToAnchor = ClaimMatch;

pub fn dme__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    enc: &[DmeGenerateEncounterId],
) -> Vec<DmeMatchClaimsToAnchor> {
    match_claims_to_anchor(stg, enc, claim_enc_key, claim_enc_id, Some("durable medical equipment"))
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = dme__anchor_events(db, &s);
    let g = dme__generate_encounter_id(db, &s, &a);
    rows(dme__match_claims_to_anchor(db, &s, &g).iter().map(fmt_match))
}
