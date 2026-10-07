use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::lab__anchor_events::*;
use crate::schema::Db;
use harness::prelude::*;

pub type LabGenerateEncounterId = ClaimEncounter;

pub fn lab__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[LabAnchorEvents],
) -> Vec<LabGenerateEncounterId> {
    claim_generate_encounter_id(stg, anchors, "lab")
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = lab__anchor_events(db, &s);
    rows(lab__generate_encounter_id(db, &s, &a).iter().map(fmt_encounter))
}
