use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type UrgentCareMatchClaimsToAnchor = k::MatchClaim;

pub fn urgent_care__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    encs: &[crate::concepts::urgent_care__generate_encounter_id::UrgentCareGenerateEncounterId],
) -> Vec<UrgentCareMatchClaimsToAnchor> {
    k::match_claims_to_anchor(stg, encs)
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::urgent_care__anchor_events::urgent_care__anchor_events(db, &s);
    let g = crate::concepts::urgent_care__generate_encounter_id::urgent_care__generate_encounter_id(db, &s, &a);
    harness::prelude::rows(urgent_care__match_claims_to_anchor(db, &s, &g).iter().map(k::fmt_match))
}
