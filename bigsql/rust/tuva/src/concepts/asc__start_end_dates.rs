use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::asc__generate_encounter_id::AscGenerateEncounterId;
use crate::schema::Db;
use harness::prelude::*;

type G = AscGenerateEncounterId;

#[derive(Clone, Copy)]
pub struct AscStartEndDates {
    pub patient_data_source_id: Str,
    pub old_encounter_id: Str,
    pub encounter_start_date: Option<Date>,
    pub encounter_end_date: Option<Date>,
}

fn pick(a: Option<Date>, b: Option<Date>, f: fn(Date, Date) -> Date) -> Option<Date> {
    match (a, b) {
        (Some(x), Some(y)) => Some(f(x, y)),
        (x, y) => x.or(y),
    }
}

pub fn asc__start_end_dates(_db: &'static Db, gen_: &[G]) -> Vec<AscStartEndDates> {
    let g = rel(gen_.to_vec());
    let f = (&g)
        .map(|x: G| (x.patient_data_source_id, x.old_encounter_id))
        .inv()
        .select(&g)
        .fold((None, None), |(s, e), x: G| (pick(s, x.start_date, std::cmp::min), pick(e, x.end_date, std::cmp::max)));
    drain(&f)
        .into_iter()
        .map(|((p, o), (s, e))| AscStartEndDates {
            patient_data_source_id: p,
            old_encounter_id: o,
            encounter_start_date: s,
            encounter_end_date: e,
        })
        .collect()
}

pub fn fmt(v: &AscStartEndDates) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.old_encounter_id),
        odate(v.encounter_start_date),
        odate(v.encounter_end_date),
    ])
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::asc__anchor_events::asc__anchor_events(db, &s);
    let g = crate::concepts::asc__generate_encounter_id::asc__generate_encounter_id(db, &s, &a);
    rows(asc__start_end_dates(db, &g).iter().map(fmt))
}
