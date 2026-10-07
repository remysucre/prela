use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type OutpatientRehabAnchorEvents = k::AnchorEvent;

pub fn outpatient_rehab__anchor_events(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<OutpatientRehabAnchorEvents> {
    k::anchor_events(stg, "outpatient rehabilitation", false)
}

pub fn q(db: &'static Db) -> String {
    k::q_anchor(db, "outpatient rehabilitation")
}
