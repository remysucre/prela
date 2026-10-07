use crate::concepts::eligibility::{Eligibility, fmt};
use crate::schema::Db;
use harness::prelude::*;

pub type InputLayerEligibility = Eligibility;

pub fn input_layer__eligibility(_db: &'static Db, el: &[Eligibility]) -> Vec<InputLayerEligibility> {
    el.to_vec()
}

pub fn q(db: &'static Db) -> String {
    let el = crate::concepts::eligibility::eligibility(db);
    rows(input_layer__eligibility(db, &el).iter().map(fmt))
}
