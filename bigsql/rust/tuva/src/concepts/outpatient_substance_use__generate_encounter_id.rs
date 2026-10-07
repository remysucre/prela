use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::outpatient_substance_use__anchor_events::*;
use crate::schema::Db;
use harness::prelude::*;

pub type OutpatientSubstanceUseGenerateEncounterId = ClaimEncounter;

pub fn outpatient_substance_use__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[OutpatientSubstanceUseAnchorEvents],
) -> Vec<OutpatientSubstanceUseGenerateEncounterId> {
    claim_generate_encounter_id(stg, anchors, "outpatient substance use")
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = outpatient_substance_use__anchor_events(db, &s);
    rows(outpatient_substance_use__generate_encounter_id(db, &s, &a).iter().map(fmt_encounter))
}
