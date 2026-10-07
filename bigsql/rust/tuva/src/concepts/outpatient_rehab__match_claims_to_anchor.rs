use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type OutpatientRehabMatchClaimsToAnchor = k::MatchClaim;

pub fn outpatient_rehab__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    encs: &[crate::concepts::outpatient_rehab__generate_encounter_id::OutpatientRehabGenerateEncounterId],
) -> Vec<OutpatientRehabMatchClaimsToAnchor> {
    k::match_claims_to_anchor(stg, encs)
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::outpatient_rehab__anchor_events::outpatient_rehab__anchor_events(db, &s);
    let g = crate::concepts::outpatient_rehab__generate_encounter_id::outpatient_rehab__generate_encounter_id(db, &s, &a);
    harness::prelude::rows(outpatient_rehab__match_claims_to_anchor(db, &s, &g).iter().map(k::fmt_match))
}
