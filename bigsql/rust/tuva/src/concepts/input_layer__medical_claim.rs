use crate::concepts::medical_claim::{MedicalClaim, fmt};
use crate::schema::Db;
use harness::prelude::*;

pub type InputLayerMedicalClaim = MedicalClaim;

pub fn input_layer__medical_claim(_db: &'static Db, mc: &[MedicalClaim]) -> Vec<InputLayerMedicalClaim> {
    mc.to_vec()
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    rows(input_layer__medical_claim(db, &mc).iter().map(fmt))
}
