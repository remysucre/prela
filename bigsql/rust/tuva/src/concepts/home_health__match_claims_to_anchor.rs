use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type HomeHealthMatchClaimsToAnchor = k::MatchClaim;

pub fn home_health__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    encs: &[crate::concepts::home_health__generate_encounter_id::HomeHealthGenerateEncounterId],
) -> Vec<HomeHealthMatchClaimsToAnchor> {
    k::match_claims_to_anchor(stg, encs)
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::home_health__anchor_events::home_health__anchor_events(db, &s);
    let g = crate::concepts::home_health__generate_encounter_id::home_health__generate_encounter_id(db, &s, &a);
    harness::prelude::rows(home_health__match_claims_to_anchor(db, &s, &g).iter().map(k::fmt_match))
}
