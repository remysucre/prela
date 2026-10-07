use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::inpatient_common::*;
use crate::schema::Db;
use harness::prelude::*;

pub type InpatientHospiceGenerateEncounterId = GenerateEncounterId;

pub fn inpatient_hospice__generate_encounter_id(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<InpatientHospiceGenerateEncounterId> {
    generate_encounter_id(stg, "inpatient hospice", "inpatient hospice")
}

pub fn fmt(v: &InpatientHospiceGenerateEncounterId) -> String {
    fmt_gen(v)
}

pub fn q(db: &'static Db) -> String {
    let st = stg(db);
    rows(inpatient_hospice__generate_encounter_id(db, &st).iter().map(fmt))
}
