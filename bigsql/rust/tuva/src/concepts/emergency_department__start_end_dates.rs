use crate::concepts::emergency_department__generate_encounter_id::EmergencyDepartmentGenerateEncounterId;
use crate::concepts::emergency_department__generate_encounter_id_pre_sort::{max_opt, min_opt};
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct EmergencyDepartmentStartEndDates {
    pub encounter_id: Str,
    pub encounter_start_date: Option<Date>,
    pub encounter_end_date: Option<Date>,
}

type G = EmergencyDepartmentGenerateEncounterId;

pub fn emergency_department__start_end_dates(_db: &'static Db, genc: &[G]) -> Vec<EmergencyDepartmentStartEndDates> {
    let g = rel(genc.to_vec());
    let f = (&g)
        .map(|x: G| x.encounter_id)
        .inv()
        .select(&g)
        .fold((None, None), |(s, e): (Option<Date>, Option<Date>), x: G| (min_opt(s, x.start_date), max_opt(e, x.end_date)));
    drain(&f)
        .into_iter()
        .map(|(encounter_id, (s, e))| EmergencyDepartmentStartEndDates {
            encounter_id,
            encounter_start_date: s,
            encounter_end_date: e,
        })
        .collect()
}

pub fn fmt(v: &EmergencyDepartmentStartEndDates) -> String {
    row(vec![V::S(v.encounter_id), odate(v.encounter_start_date), odate(v.encounter_end_date)])
}

pub fn q(db: &'static Db) -> String {
    let stg = crate::concepts::emergency_department__generate_encounter_id_pre_sort::stg_medical_claim(db);
    let pre = crate::concepts::emergency_department__generate_encounter_id_pre_sort::emergency_department__generate_encounter_id_pre_sort(db, &stg);
    let genc = crate::concepts::emergency_department__generate_encounter_id::emergency_department__generate_encounter_id(db, &pre);
    rows(emergency_department__start_end_dates(db, &genc).iter().map(fmt))
}
