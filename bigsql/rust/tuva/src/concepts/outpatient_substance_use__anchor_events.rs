use crate::concepts::anchor_kit::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

pub type OutpatientSubstanceUseAnchorEvents = ClaimAnchor;

pub fn outpatient_substance_use__anchor_events(_db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<OutpatientSubstanceUseAnchorEvents> {
    claim_anchor_events(stg, |x: EncountersStgMedicalClaim| x.service_category_2 == "outpatient substance use" && x.claim_type == "institutional")
}

pub fn q(db: &'static Db) -> String {
    let s = stg(db);
    rows(outpatient_substance_use__anchor_events(db, &s).iter().map(fmt_anchor))
}
