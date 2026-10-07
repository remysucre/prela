use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type AscAnchorEvents = k::AnchorEvent;

pub fn asc__anchor_events(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<AscAnchorEvents> {
    k::anchor_events(stg, "ambulatory surgery center", true)
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    harness::prelude::rows(asc__anchor_events(db, &s).iter().map(k::fmt_anchor))
}
