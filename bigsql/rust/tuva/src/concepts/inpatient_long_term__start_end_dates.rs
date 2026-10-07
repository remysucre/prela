use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::inpatient_common::*;
use crate::concepts::inpatient_long_term__generate_encounter_id::*;
use crate::schema::Db;
use harness::prelude::*;

pub type InpatientLongTermStartEndDates = StartEndDates;

pub fn inpatient_long_term__start_end_dates(_db: &'static Db, enc_ids: &[InpatientLongTermGenerateEncounterId]) -> Vec<InpatientLongTermStartEndDates> {
    start_end_dates(enc_ids)
}

pub fn fmt(v: &InpatientLongTermStartEndDates) -> String {
    fmt_sed(v)
}

pub fn q(db: &'static Db) -> String {
    let st: Vec<EncountersStgMedicalClaim> = stg(db);
    let enc_ids = inpatient_long_term__generate_encounter_id(db, &st);
    rows(inpatient_long_term__start_end_dates(db, &enc_ids).iter().map(fmt))
}
