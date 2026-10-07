use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type DialysisMatchClaimsToAnchor = k::MatchClaim;

pub fn dialysis__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    encs: &[crate::concepts::dialysis__generate_encounter_id::DialysisGenerateEncounterId],
) -> Vec<DialysisMatchClaimsToAnchor> {
    k::match_claims_to_anchor(stg, encs)
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::dialysis__anchor_events::dialysis__anchor_events(db, &s);
    let g = crate::concepts::dialysis__generate_encounter_id::dialysis__generate_encounter_id(db, &s, &a);
    harness::prelude::rows(dialysis__match_claims_to_anchor(db, &s, &g).iter().map(k::fmt_match))
}
