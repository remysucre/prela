use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type OutpatientHospiceMatchClaimsToAnchor = k::MatchClaim;

pub fn outpatient_hospice__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    encs: &[crate::concepts::outpatient_hospice__generate_encounter_id::OutpatientHospiceGenerateEncounterId],
) -> Vec<OutpatientHospiceMatchClaimsToAnchor> {
    k::match_claims_to_anchor(stg, encs)
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::outpatient_hospice__anchor_events::outpatient_hospice__anchor_events(db, &s);
    let g = crate::concepts::outpatient_hospice__generate_encounter_id::outpatient_hospice__generate_encounter_id(db, &s, &a);
    harness::prelude::rows(outpatient_hospice__match_claims_to_anchor(db, &s, &g).iter().map(k::fmt_match))
}
