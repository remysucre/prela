use crate::concepts::int_normalized_input_drg_voting::{IntNormalizedInputDrgVoting, fmt};
use crate::schema::Db;
use harness::prelude::*;

pub type IntNormalizedInputDrgFinal = IntNormalizedInputDrgVoting;

pub fn int_normalized_input_drg_final(
    _db: &'static Db,
    voting: &[IntNormalizedInputDrgVoting],
) -> Vec<IntNormalizedInputDrgFinal> {
    let v = rel(voting.to_vec());
    let kept = (&v).filt(|x: IntNormalizedInputDrgVoting| {
        x.occurrence_row_count == 1 && x.occurrence_count > x.next_occurrence_count
    });
    drain(&kept).into_iter().map(|(_, x)| x).collect()
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let voting = crate::concepts::int_normalized_input_drg_voting::int_normalized_input_drg_voting(db, &stg);
    rows(int_normalized_input_drg_final(db, &voting).iter().map(fmt))
}
