use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

pub type LabAnchorEvents = ClaimAnchor;

pub fn lab__anchor_events(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<LabAnchorEvents> {
    claim_anchor_events(stg, |x: EncountersStgMedicalClaim| x.service_category_2 == "lab")
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    rows(lab__anchor_events(db, &s).iter().map(fmt_anchor))
}
