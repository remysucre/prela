use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::outpatient_hospital_or_clinic__anchor_events::*;
use crate::schema::Db;
use harness::prelude::*;

pub type OutpatientHospitalOrClinicGenerateEncounterId = ClaimEncounter;

pub fn outpatient_hospital_or_clinic__generate_encounter_id(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    anchors: &[OutpatientHospitalOrClinicAnchorEvents],
) -> Vec<OutpatientHospitalOrClinicGenerateEncounterId> {
    claim_generate_encounter_id(stg, anchors, "outpatient hospital or clinic")
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let a = outpatient_hospital_or_clinic__anchor_events(db, &s);
    rows(outpatient_hospital_or_clinic__generate_encounter_id(db, &s, &a).iter().map(fmt_encounter))
}
