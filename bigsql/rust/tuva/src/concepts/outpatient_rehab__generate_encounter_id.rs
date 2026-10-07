use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type OutpatientRehabGenerateEncounterId = k::GenEnc;

pub fn outpatient_rehab__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[crate::concepts::outpatient_rehab__anchor_events::OutpatientRehabAnchorEvents],
) -> Vec<OutpatientRehabGenerateEncounterId> {
    k::generate_encounter_id(stg, &k::anchor_pairs(anchors), "outpatient rehabilitation")
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::outpatient_rehab__anchor_events::outpatient_rehab__anchor_events(db, &s);
    harness::prelude::rows(outpatient_rehab__generate_encounter_id(db, &s, &a).iter().map(k::fmt_gen))
}
