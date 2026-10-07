use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::inpatient_common::*;
use crate::schema::Db;
use harness::prelude::*;

pub type InpatientRehabGenerateEncounterId = GenerateEncounterId;

pub fn inpatient_rehab__generate_encounter_id(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<InpatientRehabGenerateEncounterId> {
    generate_encounter_id(stg, "inpatient rehabilitation", "inpatient rehabilitation")
}

pub fn fmt(v: &InpatientRehabGenerateEncounterId) -> String {
    fmt_gen(v)
}

pub fn q(db: &'static Db) -> String {
    let st = stg(db);
    rows(inpatient_rehab__generate_encounter_id(db, &st).iter().map(fmt))
}
