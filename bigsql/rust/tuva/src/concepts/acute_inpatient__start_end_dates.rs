use crate::concepts::acute_inpatient__generate_encounter_id::AcuteInpatientGenerateEncounterId;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct AcuteInpatientStartEndDates {
    pub encounter_id: Str,
    pub anchor_claim_id: Option<Str>,
    pub encounter_start_date: Option<Date>,
    pub encounter_end_date: Option<Date>,
}

type G = AcuteInpatientGenerateEncounterId;

fn min_opt<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (x, None) => x,
        (None, y) => y,
    }
}

fn max_opt<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (x, None) => x,
        (None, y) => y,
    }
}

pub fn acute_inpatient__start_end_dates(_db: &'static Db, genc: &[G]) -> Vec<AcuteInpatientStartEndDates> {
    let g = rel(genc.to_vec());
    let f = (&g).map(|x: G| x.encounter_id).inv().select(&g).fold(
        (None, None, None),
        |(a, s, e): (Option<Str>, Option<Date>, Option<Date>), x: G| {
            (min_opt(a, x.anchor_claim_id), min_opt(s, x.start_date), max_opt(e, x.end_date))
        },
    );
    drain(&f)
        .into_iter()
        .map(|(encounter_id, (a, s, e))| AcuteInpatientStartEndDates {
            encounter_id,
            anchor_claim_id: a,
            encounter_start_date: s,
            encounter_end_date: e,
        })
        .collect()
}

pub fn fmt(v: &AcuteInpatientStartEndDates) -> String {
    row(vec![V::S(v.encounter_id), ostr(v.anchor_claim_id), odate(v.encounter_start_date), odate(v.encounter_end_date)])
}

pub fn q(db: &'static Db) -> String {
    let stg = crate::concepts::acute_inpatient__generate_encounter_id::stg_medical_claim(db);
    let genc = crate::concepts::acute_inpatient__generate_encounter_id::acute_inpatient__generate_encounter_id(db, &stg);
    rows(acute_inpatient__start_end_dates(db, &genc).iter().map(fmt))
}
