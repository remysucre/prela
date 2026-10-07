use crate::concepts::int_normalized_input_admit_source_voting::{IntNormalizedInputAdmitSourceVoting, fmt};
use crate::schema::Db;
use harness::prelude::*;

pub type IntNormalizedInputAdmitSourceFinal = IntNormalizedInputAdmitSourceVoting;

pub fn int_normalized_input_admit_source_final(
    _db: &'static Db,
    voting: &[IntNormalizedInputAdmitSourceVoting],
) -> Vec<IntNormalizedInputAdmitSourceFinal> {
    let v = rel(voting.to_vec());
    let kept = (&v).filt(|x: IntNormalizedInputAdmitSourceVoting| {
        x.occurrence_row_count == 1 && x.occurrence_count > x.next_occurrence_count
    });
    drain(&kept).into_iter().map(|(_, x)| x).collect()
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let voting = crate::concepts::int_normalized_input_admit_source_voting::int_normalized_input_admit_source_voting(db, &stg);
    rows(int_normalized_input_admit_source_final(db, &voting).iter().map(fmt))
}
