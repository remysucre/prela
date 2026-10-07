use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type OutpatientPsychAnchorEvents = k::AnchorEvent;

pub fn outpatient_psych__anchor_events(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<OutpatientPsychAnchorEvents> {
    k::anchor_events(stg, "outpatient psychiatric", false)
}

pub fn q(db: &'static Db) -> String {
    k::q_anchor(db, "outpatient psychiatric")
}
