use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::inpatient_common::*;
use crate::schema::Db;
use harness::prelude::*;

pub type InpatientSubstanceUseGenerateEncounterId = GenerateEncounterId;

pub fn inpatient_substance_use__generate_encounter_id(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<InpatientSubstanceUseGenerateEncounterId> {
    generate_encounter_id(stg, "inpatient substance use", "inpatient substance use")
}

pub fn fmt(v: &InpatientSubstanceUseGenerateEncounterId) -> String {
    fmt_gen(v)
}

pub fn q(db: &'static Db) -> String {
    let st = stg(db);
    rows(inpatient_substance_use__generate_encounter_id(db, &st).iter().map(fmt))
}
