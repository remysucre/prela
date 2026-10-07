use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::outpatient_injections__anchor_events::*;
use crate::concepts::outpatient_injections__generate_encounter_id::*;
use crate::schema::Db;
use harness::prelude::*;

type G = OutpatientInjectionsGenerateEncounterId;

pub type OutpatientInjectionsMatchClaimsToAnchor = ClaimMatch;

pub fn outpatient_injections__match_claims_to_anchor(
    _db: &'static Db,
    stg: &[EncountersStgMedicalClaim],
    enc: &[G],
) -> Vec<OutpatientInjectionsMatchClaimsToAnchor> {
    match_claims_to_anchor(
        stg,
        enc,
        |x: G| x.start_date.map(|d| (x.patient_data_source_id, d)),
        |x: G| x.old_encounter_id,
        None,
    )
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    let op = crate::concepts::encounters__stg_outpatient_institutional::encounters__stg_outpatient_institutional(db);
    let a = outpatient_injections__anchor_events(db, &s, &op);
    let g = outpatient_injections__generate_encounter_id(db, &a);
    rows(outpatient_injections__match_claims_to_anchor(db, &s, &g).iter().map(fmt_match))
}
