use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::encounters__prof_and_lower_priority::EncountersProfAndLowerPriority;
use crate::concepts::inpatient_common::*;
use crate::concepts::inpatient_long_term__generate_encounter_id::*;
use crate::concepts::inpatient_long_term__start_end_dates::*;
use crate::schema::Db;
use harness::prelude::*;

pub type InpatientLongTermProfClaims = ProfClaims;

pub fn inpatient_long_term__prof_claims(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    plp: &[EncountersProfAndLowerPriority],
    enc_ids: &[InpatientLongTermGenerateEncounterId],
    sed: &[InpatientLongTermStartEndDates],
) -> Vec<InpatientLongTermProfClaims> {
    prof_claims(stg, plp, enc_ids, sed)
}

pub fn fmt(v: &InpatientLongTermProfClaims) -> String {
    fmt_prof(v)
}

pub fn q(db: &'static Db) -> String {
    let st = crate::concepts::inpatient_common::stg(db);
    let enc_ids = inpatient_long_term__generate_encounter_id(db, &st);
    let sed = inpatient_long_term__start_end_dates(db, &enc_ids);
    let pl = crate::concepts::inpatient_common::plp(db);
    rows(inpatient_long_term__prof_claims(db, &st, &pl, &enc_ids, &sed).iter().map(fmt))
}
