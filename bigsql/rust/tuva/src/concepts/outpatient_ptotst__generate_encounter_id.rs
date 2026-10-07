use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::outpatient_ptotst__anchor_events::*;
use crate::schema::Db;
use harness::prelude::*;

pub type OutpatientPtotstGenerateEncounterId = ClaimEncounter;

pub fn outpatient_ptotst__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[OutpatientPtotstAnchorEvents],
) -> Vec<OutpatientPtotstGenerateEncounterId> {
    claim_generate_encounter_id(stg, anchors, "outpatient pt/ot/st")
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = outpatient_ptotst__anchor_events(db, &s);
    rows(outpatient_ptotst__generate_encounter_id(db, &s, &a).iter().map(fmt_encounter))
}
