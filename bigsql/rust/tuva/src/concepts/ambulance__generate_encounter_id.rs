use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::ambulance__anchor_events::*;
use crate::schema::Db;
use harness::prelude::*;

pub type AmbulanceGenerateEncounterId = ClaimEncounter;

pub fn ambulance__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[AmbulanceAnchorEvents],
) -> Vec<AmbulanceGenerateEncounterId> {
    claim_generate_encounter_id(stg, anchors, "ambulance")
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = ambulance__anchor_events(db, &s);
    rows(ambulance__generate_encounter_id(db, &s, &a).iter().map(fmt_encounter))
}
