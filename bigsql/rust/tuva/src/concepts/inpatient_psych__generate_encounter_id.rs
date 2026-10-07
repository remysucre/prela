use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::inpatient_common::*;
use crate::schema::Db;
use harness::prelude::*;

pub type InpatientPsychGenerateEncounterId = GenerateEncounterId;

pub fn inpatient_psych__generate_encounter_id(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<InpatientPsychGenerateEncounterId> {
    generate_encounter_id(stg, "inpatient psychiatric", "inpatient psych")
}

pub fn fmt(v: &InpatientPsychGenerateEncounterId) -> String {
    fmt_gen(v)
}

pub fn q(db: &'static Db) -> String {
    let st = stg(db);
    rows(inpatient_psych__generate_encounter_id(db, &st).iter().map(fmt))
}
