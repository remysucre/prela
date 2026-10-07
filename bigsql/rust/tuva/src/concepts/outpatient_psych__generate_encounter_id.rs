use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type OutpatientPsychGenerateEncounterId = k::GenEnc;

pub fn outpatient_psych__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[crate::concepts::outpatient_psych__anchor_events::OutpatientPsychAnchorEvents],
) -> Vec<OutpatientPsychGenerateEncounterId> {
    k::generate_encounter_id(stg, &k::anchor_pairs(anchors), "outpatient psych")
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::outpatient_psych__anchor_events::outpatient_psych__anchor_events(db, &s);
    harness::prelude::rows(outpatient_psych__generate_encounter_id(db, &s, &a).iter().map(k::fmt_gen))
}
