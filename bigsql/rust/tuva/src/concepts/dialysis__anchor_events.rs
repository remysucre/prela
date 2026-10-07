use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type DialysisAnchorEvents = k::AnchorEvent;

pub fn dialysis__anchor_events(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<DialysisAnchorEvents> {
    k::anchor_events(stg, "dialysis", false)
}

pub fn q(db: &'static Db) -> String {
    k::q_anchor(db, "dialysis")
}
