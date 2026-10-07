use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type DialysisGenerateEncounterId = k::GenEnc;

pub fn dialysis__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[crate::concepts::dialysis__anchor_events::DialysisAnchorEvents],
) -> Vec<DialysisGenerateEncounterId> {
    k::generate_encounter_id(stg, &k::anchor_pairs(anchors), "dialysis")
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::dialysis__anchor_events::dialysis__anchor_events(db, &s);
    harness::prelude::rows(dialysis__generate_encounter_id(db, &s, &a).iter().map(k::fmt_gen))
}
