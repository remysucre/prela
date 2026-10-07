use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;

pub type UrgentCareGenerateEncounterId = k::GenEnc;

pub fn urgent_care__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[crate::concepts::urgent_care__anchor_events::UrgentCareAnchorEvents],
) -> Vec<UrgentCareGenerateEncounterId> {
    k::generate_encounter_id(stg, &anchors.iter().map(|x| (x.claim_id, x.data_source)).collect::<Vec<_>>(), "urgent care")
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::urgent_care__anchor_events::urgent_care__anchor_events(db, &s);
    harness::prelude::rows(urgent_care__generate_encounter_id(db, &s, &a).iter().map(k::fmt_gen))
}
