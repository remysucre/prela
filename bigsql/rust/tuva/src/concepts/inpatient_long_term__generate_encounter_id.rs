use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::inpatient_common::*;
use crate::schema::Db;
use harness::prelude::*;

pub type InpatientLongTermGenerateEncounterId = GenerateEncounterId;

pub fn inpatient_long_term__generate_encounter_id(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<InpatientLongTermGenerateEncounterId> {
    generate_encounter_id(stg, "inpatient long term acute care", "inpatient long term acute care")
}

pub fn fmt(v: &InpatientLongTermGenerateEncounterId) -> String {
    fmt_gen(v)
}

pub fn q(db: &'static Db) -> String {
    let st = stg(db);
    rows(inpatient_long_term__generate_encounter_id(db, &st).iter().map(fmt))
}
