use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type HomeHealthAnchorEvents = k::AnchorEvent;

pub fn home_health__anchor_events(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<HomeHealthAnchorEvents> {
    k::anchor_events(stg, "home health", false)
}

pub fn q(db: &'static Db) -> String {
    k::q_anchor(db, "home health")
}
