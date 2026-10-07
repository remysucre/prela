use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::outpatient_surgery__anchor_events::*;
use crate::schema::Db;
use harness::prelude::*;

pub type OutpatientSurgeryGenerateEncounterId = ClaimEncounter;

pub fn outpatient_surgery__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[OutpatientSurgeryAnchorEvents],
) -> Vec<OutpatientSurgeryGenerateEncounterId> {
    claim_generate_encounter_id(stg, anchors, "outpatient surgery")
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = outpatient_surgery__anchor_events(db, &s);
    rows(outpatient_surgery__generate_encounter_id(db, &s, &a).iter().map(fmt_encounter))
}
