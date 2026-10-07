use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::encounters__prof_and_lower_priority::EncountersProfAndLowerPriority;
use crate::concepts::inpatient_common::*;
use crate::concepts::inpatient_hospice__generate_encounter_id::*;
use crate::concepts::inpatient_hospice__start_end_dates::*;
use crate::schema::Db;
use harness::prelude::*;

pub type InpatientHospiceProfClaims = ProfClaims;

pub fn inpatient_hospice__prof_claims(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    plp: &[EncountersProfAndLowerPriority],
    enc_ids: &[InpatientHospiceGenerateEncounterId],
    sed: &[InpatientHospiceStartEndDates],
) -> Vec<InpatientHospiceProfClaims> {
    prof_claims(stg, plp, enc_ids, sed)
}

pub fn fmt(v: &InpatientHospiceProfClaims) -> String {
    fmt_prof(v)
}

pub fn q(db: &'static Db) -> String {
    let st = crate::concepts::inpatient_common::stg(db);
    let enc_ids = inpatient_hospice__generate_encounter_id(db, &st);
    let sed = inpatient_hospice__start_end_dates(db, &enc_ids);
    let pl = crate::concepts::inpatient_common::plp(db);
    rows(inpatient_hospice__prof_claims(db, &st, &pl, &enc_ids, &sed).iter().map(fmt))
}
