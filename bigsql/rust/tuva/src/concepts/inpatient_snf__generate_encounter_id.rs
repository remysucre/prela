use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::inpatient_common::*;
use crate::schema::Db;
use harness::prelude::*;

pub type InpatientSnfGenerateEncounterId = GenerateEncounterId;

pub fn inpatient_snf__generate_encounter_id(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<InpatientSnfGenerateEncounterId> {
    generate_encounter_id(stg, "skilled nursing", "inpatient skilled nursing")
}

pub fn fmt(v: &InpatientSnfGenerateEncounterId) -> String {
    fmt_gen(v)
}

pub fn q(db: &'static Db) -> String {
    let st = stg(db);
    rows(inpatient_snf__generate_encounter_id(db, &st).iter().map(fmt))
}
