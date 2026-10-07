use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type HomeHealthGenerateEncounterId = k::GenEnc;

pub fn home_health__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[crate::concepts::home_health__anchor_events::HomeHealthAnchorEvents],
) -> Vec<HomeHealthGenerateEncounterId> {
    k::generate_encounter_id(stg, &k::anchor_pairs(anchors), "home health")
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::home_health__anchor_events::home_health__anchor_events(db, &s);
    harness::prelude::rows(home_health__generate_encounter_id(db, &s, &a).iter().map(k::fmt_gen))
}
