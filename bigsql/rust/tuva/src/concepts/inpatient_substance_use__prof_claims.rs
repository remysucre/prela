use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::encounters__prof_and_lower_priority::EncountersProfAndLowerPriority;
use crate::concepts::inpatient_common::*;
use crate::concepts::inpatient_substance_use__generate_encounter_id::*;
use crate::concepts::inpatient_substance_use__start_end_dates::*;
use crate::schema::Db;
use harness::prelude::*;

pub type InpatientSubstanceUseProfClaims = ProfClaims;

pub fn inpatient_substance_use__prof_claims(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    plp: &[EncountersProfAndLowerPriority],
    enc_ids: &[InpatientSubstanceUseGenerateEncounterId],
    sed: &[InpatientSubstanceUseStartEndDates],
) -> Vec<InpatientSubstanceUseProfClaims> {
    prof_claims(stg, plp, enc_ids, sed)
}

pub fn fmt(v: &InpatientSubstanceUseProfClaims) -> String {
    fmt_prof(v)
}

pub fn q(db: &'static Db) -> String {
    let st = crate::concepts::inpatient_common::stg(db);
    let enc_ids = inpatient_substance_use__generate_encounter_id(db, &st);
    let sed = inpatient_substance_use__start_end_dates(db, &enc_ids);
    let pl = crate::concepts::inpatient_common::plp(db);
    rows(inpatient_substance_use__prof_claims(db, &st, &pl, &enc_ids, &sed).iter().map(fmt))
}
