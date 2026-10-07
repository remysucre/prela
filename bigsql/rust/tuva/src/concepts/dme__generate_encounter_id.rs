use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::dme__anchor_events::*;
use crate::schema::Db;
use harness::prelude::*;

pub type DmeGenerateEncounterId = ClaimEncounter;

pub fn dme__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[DmeAnchorEvents],
) -> Vec<DmeGenerateEncounterId> {
    claim_generate_encounter_id(stg, anchors, "dme")
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = dme__anchor_events(db, &s);
    rows(dme__generate_encounter_id(db, &s, &a).iter().map(fmt_encounter))
}
